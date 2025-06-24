use std::str::FromStr;

use const_format::concatcp;

use crate::{
    ArchiveUri, BaseUri, IsDomainUri, IsFtmlUri, PathUri, SymbolUri, UriComponentKind, UriKind,
    UriWithArchive, UriWithPath,
    aux::interning::{InternMap, InternStore, NonEmptyInternedStr},
    errors::{SegmentParseError, UriParseError},
};

static NAMES: std::sync::LazyLock<InternMap> = std::sync::LazyLock::new(InternMap::default);

pub struct NameStore;
impl InternStore for NameStore {
    const LIMIT: usize = 1024;
    #[inline]
    fn get() -> &'static InternMap {
        &NAMES
    }
}

/// A hierarchical name used in FTML URIs for modules, symbols, and other named entities.
///
/// [`UriName`] represents a path-like name that can contain forward slashes as separators,
/// such as "math/algebra/groups" or "logic/propositional". Names are interned for
/// efficient storage and fast equality comparisons.
///
/// Names cannot be empty and cannot contain empty segments (no leading, trailing,
/// or consecutive forward slashes).
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let name = UriName::from_str("math/algebra/groups").unwrap();
///
/// assert_eq!(name.first(), "math");
/// assert_eq!(name.last(), "groups");
/// assert_eq!(name.steps().collect::<Vec<_>>(), vec!["math", "algebra", "groups"]);
/// assert!(!name.is_simple());
///
/// let top_name = name.top();
/// assert_eq!(top_name.to_string(), "math");
/// assert!(top_name.is_simple());
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct UriName(pub(crate) NonEmptyInternedStr<NameStore>);
crate::ts!(UriName);
crate::debugdisplay!(UriName);
impl AsRef<str> for UriName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl UriName {
    /// Returns the parent name by removing the last segment.
    ///
    /// Returns `None` if this is already a top-level name (no parent).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let name = UriName::from_str("math/algebra/groups").unwrap();
    /// let parent = name.up().unwrap();
    /// assert_eq!(parent.to_string(), "math/algebra");
    ///
    /// let top = UriName::from_str("math").unwrap();
    /// assert!(top.up().is_none());
    /// ```
    #[inline]
    pub fn up(&self) -> Option<Self> {
        self.0.up::<'/'>().map(Self)
    }

    /// Returns the first segment of the hierarchical name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let name = UriName::from_str("math/algebra/groups").unwrap();
    /// assert_eq!(name.first(), "math");
    ///
    /// let simple = UriName::from_str("logic").unwrap();
    /// assert_eq!(simple.first(), "logic");
    /// ```
    #[inline]
    #[must_use]
    pub fn first(&self) -> &str {
        self.0.first_of::<'/'>()
    }

    /// Returns the last segment of the hierarchical name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let name = UriName::from_str("math/algebra/groups").unwrap();
    /// assert_eq!(name.last(), "groups");
    ///
    /// let simple = UriName::from_str("logic").unwrap();
    /// assert_eq!(simple.last(), "logic");
    /// ```
    #[inline]
    #[must_use]
    pub fn last(&self) -> &str {
        self.0.last_of::<'/'>()
    }

    /// Returns an iterator over all segments in the hierarchical name.
    ///
    /// The iterator supports both forward and backward iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let name = UriName::from_str("math/algebra/groups").unwrap();
    /// let segments: Vec<&str> = name.steps().collect();
    /// assert_eq!(segments, vec!["math", "algebra", "groups"]);
    ///
    /// let reversed: Vec<&str> = name.steps().rev().collect();
    /// assert_eq!(reversed, vec!["groups", "algebra", "math"]);
    /// ```
    #[inline]
    #[must_use]
    pub fn steps(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.0.segmented::<'/'>()
    }

    /// Returns `true` if this is a top-level name (contains no forward slashes).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let top = UriName::from_str("math").unwrap();
    /// assert!(top.is_simple());
    ///
    /// let nested = UriName::from_str("math/algebra").unwrap();
    /// assert!(!nested.is_simple());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_simple(&self) -> bool {
        !self.as_ref().contains('/')
    }

    /// Returns the top-level name (first segment only).
    ///
    /// If this is already a top-level name, returns itself.
    /// Otherwise, returns a new `UriName` containing only the first segment.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let name = UriName::from_str("math/algebra/groups").unwrap();
    /// let top = name.top();
    /// assert_eq!(top.to_string(), "math");
    /// assert!(top.is_simple());
    ///
    /// let already_top = UriName::from_str("logic").unwrap();
    /// let still_top = already_top.clone().top();
    /// assert_eq!(already_top, still_top);
    /// ```
    #[inline]
    #[must_use]
    pub fn top(self) -> Self {
        if self.is_simple() {
            self
        } else {
            // SAFETY: safe by construction of Self
            Self(unsafe { self.first().parse().unwrap_unchecked() })
        }
    }
}

impl FromStr for UriName {
    type Err = SegmentParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(NonEmptyInternedStr::new_with_sep::<'/'>(s)?))
    }
}
impl std::fmt::Display for UriName {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A URI that identifies a specific module within an FTML archive.
///
/// A [`ModuleUri`] extends a [`PathUri`] with a module [`UriName`], creating a complete
/// reference to a module within an archive. Module URIs have the form:
/// `http://example.com?a=archive&p=path&m=module/name`
///
/// The module name is hierarchical and can contain forward slashes to represent
/// nested modules, similar to filesystem paths or programming language namespaces.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let module_uri = ModuleUri::from_str("http://example.com?a=math&m=algebra/groups").unwrap();
///
/// assert_eq!(module_uri.name.as_ref(), "algebra/groups");
/// assert_eq!(module_uri.archive_id().as_ref(), "math");
/// assert_eq!(module_uri.base().as_str(), "http://example.com");
///
/// // Module URIs can have paths within the archive
/// let with_path = ModuleUri::from_str("http://example.com?a=math&p=textbooks&m=algebra").unwrap();
/// assert_eq!(with_path.path().unwrap().as_ref(), "textbooks");
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct ModuleUri {
    /// The hierarchical name of the module.
    pub name: UriName,
    /// The path component specifying location within the archive.
    pub path: PathUri,
}
crate::ts!(ModuleUri);
crate::debugdisplay!(ModuleUri);
impl crate::sealed::Sealed for ModuleUri {}

impl std::fmt::Display for ModuleUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}&{}={}", self.path, Self::SEPARATOR, self.name)
    }
}

impl ModuleUri {
    pub(crate) const SEPARATOR: char = 'm';

    /// Converts this module URI into a symbol URI by treating the last segment
    /// of the module name as a symbol name.
    ///
    /// Returns `None` if the module name has only one segment (is top-level).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let module_uri = ModuleUri::from_str("http://example.com?a=math&m=algebra/groups/theorem").unwrap();
    /// let symbol_uri = module_uri.into_symbol().unwrap();
    ///
    /// assert_eq!(symbol_uri.module.name.to_string(), "algebra/groups");
    /// assert_eq!(symbol_uri.name.to_string(), "theorem");
    /// ```
    #[must_use]
    pub fn into_symbol(self) -> Option<SymbolUri> {
        if self.name.is_simple() {
            return None;
        }
        // SAFETY: by construction segments are non-empty and have no illegal
        // characters. Since !is_top(), up() is Some()
        let (last, name) = unsafe {
            (
                self.name.last().parse().unwrap_unchecked(),
                self.name.up().unwrap_unchecked(),
            )
        };

        Some(SymbolUri {
            module: Self {
                path: self.path,
                name,
            },
            name: last,
        })
    }

    /// Internal parsing method used by URI parsing infrastructure.
    ///
    /// This method handles the common parsing logic for module URIs and
    /// URI types that extend module URIs (like symbol URIs).
    pub(super) fn pre_parse<R>(
        s: &str,
        uri_kind: UriKind,
        f: impl FnOnce(Self, std::str::Split<char>) -> Result<R, UriParseError>,
    ) -> Result<R, UriParseError> {
        PathUri::pre_parse(s, uri_kind, |path, next, mut split| {
            let Some(m) = next.or_else(|| split.next()) else {
                return Err(UriParseError::MissingPartFor {
                    uri_kind,
                    part: UriComponentKind::m,
                });
            };
            m.strip_prefix(concatcp!(ModuleUri::SEPARATOR, "="))
                .map_or_else(
                    || {
                        Err(UriParseError::MissingPartFor {
                            uri_kind,
                            part: UriComponentKind::m,
                        })
                    },
                    |name| {
                        f(
                            Self {
                                path,
                                name: name.parse()?,
                            },
                            split,
                        )
                    },
                )
        })
    }
}
impl FromStr for ModuleUri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::pre_parse(s, UriKind::ModuleUri, |u, mut split| {
            if split.next().is_some() {
                return Err(UriParseError::TooManyPartsFor {
                    uri_kind: UriKind::ModuleUri,
                });
            }
            Ok(u)
        })
    }
}

impl From<ModuleUri> for PathUri {
    #[inline]
    fn from(value: ModuleUri) -> Self {
        value.path
    }
}
impl From<ModuleUri> for ArchiveUri {
    fn from(value: ModuleUri) -> Self {
        value.path.archive
    }
}
impl From<ModuleUri> for BaseUri {
    #[inline]
    fn from(value: ModuleUri) -> Self {
        value.path.archive.base
    }
}
impl IsFtmlUri for ModuleUri {
    #[inline]
    fn base(&self) -> &crate::BaseUri {
        &self.path.archive.base
    }
}
impl UriWithPath for ModuleUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        &self.path
    }
}
impl UriWithArchive for ModuleUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        &self.path.archive
    }
}

impl IsDomainUri for ModuleUri {
    #[inline]
    fn module_uri(&self) -> &ModuleUri {
        self
    }
}

crate::tests! {
    module {
        tracing::info!("Size of UriName: {}",std::mem::size_of::<UriName>());
        tracing::info!("Size of ModuleUri: {}",std::mem::size_of::<ModuleUri>());
    };
    uri_name_parsing {
        use std::str::FromStr;
        // Valid names
        let simple = UriName::from_str("module").expect("works");
        assert_eq!(simple.to_string(), "module");
        assert!(simple.is_simple());
        assert_eq!(simple.first(), "module");
        assert_eq!(simple.last(), "module");

        let nested = UriName::from_str("math/algebra/groups").expect("works");
        assert_eq!(nested.to_string(), "math/algebra/groups");
        assert!(!nested.is_simple());
        assert_eq!(nested.first(), "math");
        assert_eq!(nested.last(), "groups");

        // Invalid names
        assert!(UriName::from_str("").is_err());
        assert!(UriName::from_str("/").is_err());
        assert!(UriName::from_str("a/").is_err());
        assert!(UriName::from_str("/a").is_err());
        assert!(UriName::from_str("a//b").is_err());
    };
    uri_name_navigation {
        let name = UriName::from_str("math/algebra/groups/theory").expect("works");

        // Test steps iteration
        let steps: Vec<&str> = name.steps().collect();
        assert_eq!(steps, vec!["math", "algebra", "groups", "theory"]);

        let rev_steps: Vec<&str> = name.steps().rev().collect();
        assert_eq!(rev_steps, vec!["theory", "groups", "algebra", "math"]);

        // Test up navigation
        let up1 = name.up().expect("works");
        assert_eq!(up1.to_string(), "math/algebra/groups");

        let up2 = up1.up().expect("works");
        assert_eq!(up2.to_string(), "math/algebra");

        let up3 = up2.up().expect("works");
        assert_eq!(up3.to_string(), "math");
        assert!(up3.is_simple());

        assert!(up3.up().is_none());

        // Test top
        let top = name.clone().top();
        assert_eq!(top.to_string(), "math");
        assert!(top.is_simple());

        let already_top = up3.clone().top();
        assert_eq!(already_top, up3);
    };
    uri_name_interning {
        // Test that names are properly interned
        let name1 = UriName::from_str("math/algebra").expect("works");
        let name2 = UriName::from_str("math/algebra").expect("works");

        // Should be equal and efficiently comparable
        assert_eq!(name1, name2);

        // Test different names
        let name3 = UriName::from_str("logic/propositional").expect("works");
        assert_ne!(name1, name3);
    };
    module_uri_parsing {
        use std::str::FromStr;
        // Valid module URIs
        let simple = ModuleUri::from_str("http://example.com?a=archive&m=module").expect("works");
        assert_eq!(simple.name.to_string(), "module");
        assert_eq!(simple.archive_id().to_string(), "archive");
        assert!(simple.path().is_none());

        let with_path = ModuleUri::from_str("http://example.com?a=archive&p=folder&m=math/algebra").expect("works");
        assert_eq!(with_path.name.to_string(), "math/algebra");
        assert_eq!(with_path.path().expect("works").to_string(), "folder");

        // Invalid module URIs
        assert!(ModuleUri::from_str("http://example.com?a=archive").is_err());
        assert!(ModuleUri::from_str("http://example.com?a=archive&m=").is_err());
        assert!(ModuleUri::from_str("http://example.com?a=archive&m=a//b").is_err());
    };
    module_uri_traits {
        use std::str::FromStr;
        use crate::{IsFtmlUri, UriWithArchive, UriWithPath, IsDomainUri};

        let module_uri = ModuleUri::from_str("http://example.com?a=math&p=textbooks&m=algebra/groups").expect("works");

        // Test IsFtmlUri
        assert_eq!(module_uri.base().as_str(), "http://example.com");

        // Test UriWithArchive
        assert_eq!(module_uri.archive_id().to_string(), "math");
        assert_eq!(module_uri.archive_uri().to_string(), "http://example.com?a=math");

        // Test UriWithPath
        assert_eq!(module_uri.path().expect("works").to_string(), "textbooks");

        // Test IsContentUri
        assert_eq!(module_uri.module_name().to_string(), "algebra/groups");

        // Test conversions
        let path_uri: PathUri = module_uri.clone().into();
        assert_eq!(path_uri.path().expect("works").to_string(), "textbooks");

        let archive_uri: ArchiveUri = module_uri.clone().into();
        assert_eq!(archive_uri.id.to_string(), "math");

        let base_uri: BaseUri = module_uri.into();
        assert_eq!(base_uri.as_str(), "http://example.com");
    };
    module_uri_display {
        use std::str::FromStr;
        let module_uri = ModuleUri::from_str("http://example.com?a=archive&p=path&m=module").expect("works");
        let expected = "http://example.com?a=archive&p=path&m=module";
        assert_eq!(module_uri.to_string(), expected);

        let no_path = ModuleUri::from_str("http://example.com?a=archive&m=module").expect("works");
        let expected_no_path = "http://example.com?a=archive&m=module";
        assert_eq!(no_path.to_string(), expected_no_path);
    };
    unicode_names {
        use std::str::FromStr;
        // Test Unicode in module names
        let unicode_name = UriName::from_str("数学/代数/群论").expect("works");
        assert_eq!(unicode_name.first(), "数学");
        assert_eq!(unicode_name.last(), "群论");

        let unicode_module = ModuleUri::from_str("http://example.com?a=archive&m=логика/предикаты").expect("works");
        assert_eq!(unicode_module.name.to_string(), "логика/предикаты");
    };
    concurrent_name_creation {
        use std::sync::{Arc, Barrier};

        const NUM_THREADS: usize = 10;
        const NUM_NAMES: usize = 100;

        let barrier = Arc::new(Barrier::new(NUM_THREADS));
        let mut handles = vec![];

        for i in 0..NUM_THREADS {
            let barrier = Arc::clone(&barrier);
            let handle = std::thread::spawn(move || {
                barrier.wait();

                let mut names = Vec::new();
                for j in 0..NUM_NAMES {
                    let name = UriName::from_str(&format!("thread{i}/name{j}")).expect("works");
                    names.push(name);
                }

                // Verify all names are valid
                for name in &names {
                    assert!(!name.as_ref().is_empty());
                    assert!(name.steps().count() == 2);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("works");
        }
    }
}
