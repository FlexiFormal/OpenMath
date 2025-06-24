#![allow(clippy::derived_hash_with_manual_eq)]

use crate::errors::SegmentParseError;
use std::{
    borrow::Borrow, hash::Hasher, marker::PhantomData, num::NonZeroUsize, ops::Deref, str::FromStr,
};

pub type InternMap = (
    dashmap::DashSet<strumbra::SharedString, rustc_hash::FxBuildHasher>,
    parking_lot::Mutex<usize>,
);

pub trait InternStore {
    const LIMIT: usize;
    fn get() -> &'static InternMap;
}

#[impl_tools::autoimpl(Clone, PartialOrd, Ord, Hash)]
pub struct InternedStr<Store: InternStore>(strumbra::SharedString, PhantomData<Store>);
crate::debugdisplay!(InternedStr<Store: InternStore>);
impl<Store: InternStore> std::fmt::Display for InternedStr<Store> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<Store: InternStore> Borrow<str> for InternedStr<Store> {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0
    }
}
impl<Store: InternStore> std::ops::Deref for InternedStr<Store> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<Store: InternStore> PartialEq for InternedStr<Store> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // By construction, string are disambiguated in some store;
        // so equality is pointer equality
        self.ptr_eq(other)
    }
}
impl<Store: InternStore> Eq for InternedStr<Store> {}

impl<Store: InternStore> FromStr for InternedStr<Store> {
    type Err = SegmentParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
impl<Store: InternStore> InternedStr<Store> {
    #[inline]
    const fn morph(&self) -> &internals::UmbraStringImpl {
        // SAFETY: This transmute is safe because:
        // 1. We're transmuting a reference, not moving data
        // 2. internals::UmbraStringImpl has the same memory layout as strumbra::SharedString
        // 3. The lifetime of the returned reference is tied to self
        // WARNING: This relies on strumbra::SharedString's internal implementation
        unsafe { &*((&raw const self.0).cast()) }
    }

    #[inline]
    #[must_use]
    pub const fn on_stack(&self) -> bool {
        self.morph().len <= internals::INLINE_LEN
    }

    unsafe fn arc_count(str: &internals::UmbraStringImpl) -> NonZeroUsize {
        // SAFETY: Caller must ensure:
        // 1. str is not on stack (i.e., str.len > INLINE_LEN)
        // 2. str.trailing.ptr is valid and points to a live Arc
        // 3. The Arc has a non-zero reference count
        unsafe {
            let inner = str.trailing.ptr.ptr.as_ptr().as_ref().unwrap_unchecked();
            let count = inner.count.load(std::sync::atomic::Ordering::Relaxed);
            // SAFETY: Count is guaranteed to be non-zero because we hold a reference
            NonZeroUsize::new_unchecked(count)
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn get_arc_count(&self) -> Option<NonZeroUsize> {
        if self.on_stack() {
            return None;
        }
        let morphed = self.morph();
        // SAFETY: We checked that !self.on_stack(), so:
        // 1. slf.trailing contains ptr, not buf
        // 2. self is a strong reference, so the Arc is valid and count > 0
        Some(unsafe { Self::arc_count(morphed) })
    }

    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        let morphed = self.morph();
        let other = other.morph();
        morphed.len == other.len && morphed.prefix == other.prefix && {
            if self.on_stack() {
                // SAFETY: on_stack() guarantees that trailing contains buf, not ptr
                unsafe { morphed.trailing.buf == other.trailing.buf }
            } else {
                // SAFETY: !on_stack() guarantees that trailing contains ptr, not buf
                // Both strings are interned from the same store, so pointer equality
                // is sufficient to determine string equality
                unsafe {
                    std::ptr::eq(
                        morphed.trailing.ptr.ptr.as_ptr(),
                        other.trailing.ptr.ptr.as_ptr(),
                    )
                }
            }
        }
    }

    fn new(s: &str) -> Result<Self, SegmentParseError> {
        if s.len() <= internals::INLINE_LEN as usize {
            if let Some(i) = s.find(super::errors::ILLEGAL_CHARS) {
                // SAFETY: i is defined, so s[i..].chars().next() is defined
                return unsafe {
                    Err(SegmentParseError::IllegalChar(
                        s[i..].chars().next().unwrap_unchecked(),
                    ))
                };
            }
            Ok(Self(strumbra::SharedString::try_from(s)?, PhantomData))
        } else {
            let (store, len) = Store::get();
            if let Some(s) = store.get(s) {
                Ok(Self(s.clone(), PhantomData))
            } else {
                let mut len = len.lock();
                if let Some(i) = s.find(super::errors::ILLEGAL_CHARS) {
                    // SAFETY: i is defined, so s[i..].chars().next() is defined
                    return unsafe {
                        Err(SegmentParseError::IllegalChar(
                            s[i..].chars().next().unwrap_unchecked(),
                        ))
                    };
                }
                let s = strumbra::SharedString::try_from(s)?;
                store.insert(s.clone());
                *len += 1;
                if *len > Store::LIMIT {
                    // SAFETY: store only contains heap-allocated strings (len > INLINE_LEN)
                    // so arc_count preconditions are satisfied
                    unsafe {
                        store.retain(|e| {
                            let impl_ref: &internals::UmbraStringImpl =
                                &*(std::ptr::from_ref(e).cast());
                            Self::arc_count(impl_ref).get() > 1
                        });
                    };
                    *len = store.len();
                }
                drop(len);
                Ok(Self(s, PhantomData))
            }
        }
    }
}

type Inner = (std::num::NonZeroU32, u32, usize); //std::num::NonZeroU128; //(u64, std::num::NonZeroUsize); //std::num::NonZeroU128; //(std::num::NonZeroU64,usize);

// transmute-fuckery to get a niche for optimisations; e.g.
// `size_of::<Option<NonEmptyInternedStr>>()==size_of::<NonEmptyInternedStr>()`
#[impl_tools::autoimpl(PartialEq, Eq)]
pub struct NonEmptyInternedStr<Store: InternStore>(Inner, PhantomData<Store>);
crate::debugdisplay!(NonEmptyInternedStr<Store: InternStore>);
impl<Store: InternStore> std::ops::Deref for NonEmptyInternedStr<Store> {
    type Target = InternedStr<Store>;
    #[inline]
    #[allow(clippy::transmute_undefined_repr)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: NonEmptyInternedStr and InternedStr have the same memory layout
        // (both contain a single field of the same type)
        unsafe { &*(&raw const self.0).cast() }
    }
}
impl<Store: InternStore> FromStr for NonEmptyInternedStr<Store> {
    type Err = SegmentParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
impl<Store: InternStore> std::fmt::Display for NonEmptyInternedStr<Store> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<Store: InternStore> From<NonEmptyInternedStr<Store>> for InternedStr<Store> {
    #[allow(clippy::transmute_undefined_repr)]
    fn from(value: NonEmptyInternedStr<Store>) -> Self {
        // SAFETY: NonZeroU128 and strumbra::SharedString have the same size and alignment
        // This is a bijection that preserves the bit pattern
        unsafe { std::mem::transmute(value.0) }
    }
}
impl<Store: InternStore> Clone for NonEmptyInternedStr<Store> {
    #[inline]
    #[allow(clippy::transmute_undefined_repr)]
    fn clone(&self) -> Self {
        // SAFETY: We clone the underlying InternedStr and transmute it back
        // The clone operation ensures proper reference counting
        unsafe {
            Self(
                std::mem::transmute::<InternedStr<Store>, Inner>(self.deref().clone()),
                PhantomData,
            )
        }
    }
}
impl<Store: InternStore> Drop for NonEmptyInternedStr<Store> {
    #[allow(clippy::transmute_undefined_repr)]
    fn drop(&mut self) {
        // SAFETY: We transmute NonZeroU128 back to InternedStr to properly
        // decrement reference counts and clean up resources
        let morphed: InternedStr<Store> = unsafe { std::mem::transmute(self.0) };
        std::mem::drop(morphed);
    }
}
impl<Store: InternStore> std::hash::Hash for NonEmptyInternedStr<Store> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}
impl<Store: InternStore> Ord for NonEmptyInternedStr<Store> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deref().cmp(&**other)
    }
}
impl<Store: InternStore> PartialOrd for NonEmptyInternedStr<Store> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<Store: InternStore> PartialEq<InternedStr<Store>> for NonEmptyInternedStr<Store> {
    #[inline]
    fn eq(&self, other: &InternedStr<Store>) -> bool {
        self.deref().eq(other)
    }
}
impl<Store: InternStore> PartialOrd<InternedStr<Store>> for NonEmptyInternedStr<Store> {
    #[inline]
    fn partial_cmp(&self, other: &InternedStr<Store>) -> Option<std::cmp::Ordering> {
        Some(self.deref().cmp(other))
    }
}
impl<Store: InternStore> PartialEq<NonEmptyInternedStr<Store>> for InternedStr<Store> {
    #[inline]
    fn eq(&self, other: &NonEmptyInternedStr<Store>) -> bool {
        self.eq(&**other)
    }
}
impl<Store: InternStore> PartialOrd<NonEmptyInternedStr<Store>> for InternedStr<Store> {
    #[inline]
    fn partial_cmp(&self, other: &NonEmptyInternedStr<Store>) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&**other))
    }
}

impl<Store: InternStore> NonEmptyInternedStr<Store> {
    #[inline]
    #[allow(clippy::transmute_undefined_repr)]
    pub(crate) unsafe fn new_from_nonempty(s: InternedStr<Store>) -> Self {
        // SAFETY: Caller must ensure s is non-empty
        // We transmute InternedStr to NonZeroU128 for niche optimization
        Self(
            unsafe { std::mem::transmute::<InternedStr<Store>, Inner>(s) },
            PhantomData,
        )
    }

    #[inline]
    #[allow(dead_code)]
    pub fn new_from_interned(s: InternedStr<Store>) -> Option<Self> {
        if s.is_empty() {
            None
        } else {
            // SAFETY: s is non-empty
            Some(unsafe { Self::new_from_nonempty(s) })
        }
    }

    pub fn new(s: &str) -> Result<Self, SegmentParseError> {
        if s.is_empty() {
            Err(SegmentParseError::Empty)
        } else {
            // SAFETY: s is non-empty
            Ok(unsafe { Self::new_from_nonempty(InternedStr::new(s)?) })
        }
    }

    #[allow(clippy::transmute_undefined_repr)]
    #[allow(dead_code)]
    pub fn from_interned_with_sep<const SEP: char>(s: InternedStr<Store>) -> Option<Self> {
        if s.is_empty() || s.split(SEP).any(str::is_empty) {
            return None;
        }
        // SAFETY: We've verified that s is non-empty and has no empty segments
        Some(Self(
            unsafe { std::mem::transmute::<InternedStr<Store>, Inner>(s) },
            PhantomData,
        ))
    }

    pub fn new_with_sep<const SEP: char>(s: &str) -> Result<Self, SegmentParseError> {
        if s.is_empty() || s.split(SEP).any(str::is_empty) {
            return Err(SegmentParseError::Empty);
        }
        // SAFETY: s is non-empty
        Ok(unsafe { Self::new_from_nonempty(InternedStr::new(s)?) })
    }

    #[inline]
    pub fn segmented<const SEP: char>(&self) -> std::str::Split<'_, char> {
        self.split(SEP)
    }

    #[inline]
    pub fn first_of<const SEP: char>(&self) -> &str {
        // SAFETY: NonEmptyInternedStr guarantees the string is non-empty,
        // so split() will always yield at least one element
        unsafe { self.split(SEP).next().unwrap_unchecked() }
    }

    #[inline]
    pub fn last_of<const SEP: char>(&self) -> &str {
        // SAFETY: NonEmptyInternedStr guarantees the string is non-empty,
        // so split() will always yield at least one element
        unsafe { self.split(SEP).next_back().unwrap_unchecked() }
    }

    pub fn up<const SEP: char>(&self) -> Option<Self> {
        if let Some((s, _)) = self.rsplit_once(SEP) {
            // SAFETY: rsplit_once with a non-empty string that was validated
            // to have no empty segments guarantees s is non-empty
            Some(unsafe { Self::new_from_nonempty(InternedStr::new(s).unwrap_unchecked()) })
        } else {
            None
        }
    }
}

mod internals {
    use std::{marker::PhantomData, mem::ManuallyDrop, ptr::NonNull, sync::atomic::AtomicUsize};

    #[allow(clippy::cast_possible_truncation)]
    pub const INLINE_LEN: u32 = 4 + std::mem::size_of::<usize>() as u32;

    #[repr(C)]
    #[allow(missing_debug_implementations)]
    pub struct ArcDynBytes {
        pub(super) ptr: NonNull<ArcDynBytesInner<[u8; 0]>>,
        phantom: PhantomData<ArcDynBytesInner<[u8]>>,
    }

    #[repr(C)]
    pub(super) struct ArcDynBytesInner<T: ?Sized> {
        pub(super) count: AtomicUsize,
        data: T,
    }

    #[repr(C)]
    pub union Trailing {
        pub buf: [u8; std::mem::size_of::<usize>()],
        pub ptr: ManuallyDrop<ArcDynBytes>,
    }

    #[repr(C)]
    pub struct UmbraStringImpl {
        pub len: u32,
        pub prefix: [u8; 4],
        pub trailing: Trailing,
    }
}

#[cfg(test)]
static TEST_PATHS: std::sync::LazyLock<InternMap> = std::sync::LazyLock::new(InternMap::default);

crate::tests! {
    str {
        tracing::info!("Size of InternedStr: {}",std::mem::size_of::<InternedStr<crate::archive::IdStore>>());
        tracing::info!("Size of Option<InternedStr>: {}",std::mem::size_of::<Option<InternedStr<crate::archive::IdStore>>>());
        tracing::info!("Size of Option<NonEmptyInternedStr>: {}",std::mem::size_of::<Option<NonEmptyInternedStr<crate::archive::IdStore>>>());
    };
    thread_safety {
        use std::sync::Barrier;
        use crate::UriPath;
        use triomphe::Arc;
        const NUM_THREADS: usize = 100;
        const ITERATIONS: usize = 1000;

        let barrier = Arc::new(Barrier::new(NUM_THREADS));
        let mut handles = vec![];

        for i in 0..NUM_THREADS {
            let barrier = Arc::clone(&barrier);
            let handle = std::thread::spawn(move || {
                barrier.wait();

                for j in 0..ITERATIONS {
                    // Create strings that might collide
                    let s1 = format!("thread_{i}/iteration_{j}");
                    let s2 = format!("thread_{i}/iteration_{j}");

                    let path1 = UriPath::from_str(&s1).expect("works");
                    let path2 = UriPath::from_str(&s2).expect("works");

                    // Should be pointer-equal due to interning
                    assert_eq!(path1, path2);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("works");
        }
    };
    empty_segments {
        use crate::UriPath;
        assert!(UriPath::from_str("/").is_err());
        assert!(UriPath::from_str("a//b").is_err());
        assert!(UriPath::from_str("a/").is_err());
        assert!(UriPath::from_str("/a").is_err());
        assert!(UriPath::from_str("").is_err());
    };
    illegal_characters {
        use crate::UriPath;
        for ch in &crate::aux::errors::ILLEGAL_CHARS {
            let s = format!("test{ch}string");
            assert!(UriPath::from_str(&s).is_err());

            // Test at different positions
            let s = format!("{ch}test");
            assert!(UriPath::from_str(&s).is_err());

            let s = format!("test{ch}");
            assert!(UriPath::from_str(&s).is_err());
        }
    };
    long_strings {
        use crate::UriPath;
        // Test strings longer than INLINE_LEN (12 bytes)
        let short_str = "a".repeat(10);
        let medium_str = "a".repeat(20);
        let long_str = "a".repeat(1000);

        // All should succeed if they don't exceed strumbra limits
        let short = UriPath::from_str(&short_str).expect("works");
        let medium = UriPath::from_str(&medium_str).expect("works");
        let long = UriPath::from_str(&long_str).expect("works");

        // Test that interning works correctly
        let short2 = UriPath::from_str(&short_str).expect("works");
        let medium2 = UriPath::from_str(&medium_str).expect("works");
        let long2 = UriPath::from_str(&long_str).expect("works");

        assert_eq!(short, short2);
        assert_eq!(medium, medium2);
        assert_eq!(long, long2);
    };
    moving_up {
        use crate::UriPath;
        let path = UriPath::from_str("a/b/c/d").expect("works");

        let up1 = path.up().expect("works");
        assert_eq!(up1.to_string(), "a/b/c");

        let up2 = up1.up().expect("works");
        assert_eq!(up2.to_string(), "a/b");

        let up3 = up2.up().expect("works");
        assert_eq!(up3.to_string(), "a");

        // Should return None when can't go up further
        assert!(up3.up().is_none());
    };
    unicode {
        use crate::UriPath;
        // Test Unicode in paths
        let unicode_path = "路径/测试/文件";
        let path = UriPath::from_str(unicode_path).expect("works");
        assert_eq!(path.to_string(), unicode_path);

        let unicode2 = "アーカイブ/識別子";
        let path2 = UriPath::from_str(unicode2).expect("works");
        assert_eq!(path2.to_string(), unicode2);
    };
    memory_limits {
        struct PathStore;
        impl InternStore for PathStore {
            const LIMIT:usize = 1024;
            #[inline]
            fn get() -> &'static InternMap { &TEST_PATHS }
        }
        type UriPath = NonEmptyInternedStr<PathStore>;
        // Test that the interning system respects its limits
        // Create more strings than the LIMIT for PathStore (1024)
        let mut paths = Vec::new();
        for i in 0..2000 {
            let path = UriPath::new_with_sep::<'/'>(&format!("unique/path/number/{i}")).expect("works");
            paths.push(path);
        }

        // System should still work correctly even after exceeding limits
        let test_path = UriPath::new_with_sep::<'/'>("test/after/limit").expect("works");
        let test_path2 = UriPath::new_with_sep::<'/'>("test/after/limit").expect("works");
        assert_eq!(test_path, test_path2);
        // check store
        let (store,len) = PathStore::get();
        assert_eq!(store.len(),*len.lock());
        assert!(store.len()>2000);
        // force cleanup
        paths.clear();
        drop(paths);
        let test_path =UriPath::new_with_sep::<'/'>("this/one/is/new").expect("works");
        let test_path = UriPath::new_with_sep::<'/'>("this/also/is/new").expect("works");
        assert!(store.len()<100);
    }
}
