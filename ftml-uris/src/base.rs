use crate::{IsFtmlUri, errors::UriParseError};
use either::{Either, Either::Left, Either::Right};
use std::{hash::BuildHasher, str::FromStr};
use triomphe::Arc;

/// A base URI that serves as the foundation for all FTML URI types.
///
/// [`BaseUri`] represents a standard URL without query parameters or fragments.
/// It must be a valid base URL that can serve as the foundation for building
/// more complex FTML URIs like [`ArchiveUri`](crate::ArchiveUri)s, [`PathUri`](crate::PathUri)s,
/// and [`ModuleUri`](crate::ModuleUri)s.
///
/// [`BaseUri`] are interned for efficient memory usage and fast equality
/// comparisons - i.e. multiple [`BaseUri`] instances with the same URL will share
/// the same underlying memory.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let base_uri = BaseUri::from_str("http://example.com").unwrap();
/// assert_eq!(base_uri.as_str(), "http://example.com");
///
/// // Base URIs are normalized (trailing slashes removed)
/// let normalized = BaseUri::from_str("http://example.com/path/").unwrap();
/// assert_eq!(normalized.as_str(), "http://example.com/path");
///
/// // These will fail because they have query/fragment components
/// assert!(BaseUri::from_str("http://example.com?query=value").is_err());
/// assert!(BaseUri::from_str("http://example.com#fragment").is_err());
/// ```
#[derive(Clone, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct BaseUri(Arc<url::Url>);
crate::ts!(BaseUri);
crate::debugdisplay!(BaseUri);
impl crate::sealed::Sealed for BaseUri {}

impl IsFtmlUri for BaseUri {
    #[inline]
    fn base(&self) -> &BaseUri {
        self
    }
}

impl BaseUri {
    /// Returns a reference to the default "unknown source" [`BaseUri`], usable as a meaningful
    /// default for e.g. "anonymous" documents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// let unknown = BaseUri::unknown();
    /// assert_eq!(unknown.as_str(), "http://unknown.source");
    /// ```
    #[inline]
    #[must_use]
    pub fn unknown() -> &'static Self {
        &UNKNOWN_BASE
    }

    /// Returns a reference to the underlying [`url::Url`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let base_uri = BaseUri::from_str("http://example.com/path").unwrap();
    /// let url = base_uri.as_url();
    /// assert_eq!(url.host_str(), Some("example.com"));
    /// assert_eq!(url.path(), "/path");
    /// ```
    #[inline]
    #[must_use]
    pub fn as_url(&self) -> &url::Url {
        &self.0
    }

    /// Returns the URI as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let base_uri = BaseUri::from_str("http://example.com/path/").unwrap();
    /// assert_eq!(base_uri.as_str(), "http://example.com/path");
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str().trim_end_matches('/')
    }

    /// Creates a new base URI from a [`url::Url`].
    ///
    /// # Errors
    ///
    /// Returns an error if the URL:
    /// - Has query parameters or fragment components
    /// - Cannot be used as a base URL (e.g., data URLs)
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use url::Url;
    /// let url = Url::parse("http://example.com/path").unwrap();
    /// let base_uri = BaseUri::new(url).unwrap();
    /// assert_eq!(base_uri.as_str(), "http://example.com/path");
    ///
    /// // This will fail due to query parameter
    /// let bad_url = Url::parse("http://example.com?query=value").unwrap();
    /// assert!(BaseUri::new(bad_url).is_err());
    /// ```
    pub fn new(url: url::Url) -> Result<Self, UriParseError> {
        let mut base = BASE_URIS.lock();
        #[allow(clippy::map_unwrap_or)]
        base.iter()
            .rev()
            .find(|e| **e == url)
            .map(|e| Ok(Self(e.url.clone())))
            .unwrap_or_else(|| Self::make_new(url, &mut base))
    }

    fn make_new(url: url::Url, cache: &mut Vec<InternedBaseURI>) -> Result<Self, UriParseError> {
        if url.fragment().is_some() || url.query().is_some() {
            return Err(UriParseError::HasQueryOrFragment);
        }
        if url.cannot_be_a_base() {
            return Err(UriParseError::CannotBeABase);
        }
        clean(cache);
        let e = InternedBaseURI::from(url);
        let rf = e.url.clone();
        cache.push(e);
        Ok(Self(rf))
    }

    /// Internal parsing method used by URI parsing infrastructure.
    ///
    /// This method handles the common parsing logic for all FTML URI types,
    /// separating the base URI from query parameters.
    pub(super) fn pre_parse(
        s: &str,
    ) -> Result<Either<Self, (Self, std::str::Split<'_, char>)>, UriParseError> {
        let Some((base, rest)) = s.split_once('?') else {
            return s.parse().map(Left);
        };
        let base = base.parse()?;
        Ok(if rest.is_empty() {
            Left(base)
        } else {
            Right((base, rest.split('&')))
        })
    }
}

impl std::hash::Hash for BaseUri {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.as_ptr().cast::<()>() as usize).hash(state);
    }
}
impl PartialEq for BaseUri {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}
impl Eq for BaseUri {}

impl TryFrom<url::Url> for BaseUri {
    type Error = UriParseError;
    #[inline]
    fn try_from(value: url::Url) -> Result<Self, UriParseError> {
        Self::new(value)
    }
}
impl<'s> TryFrom<&'s str> for BaseUri {
    type Error = UriParseError;
    #[inline]
    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}
impl FromStr for BaseUri {
    type Err = UriParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().trim_end_matches('/');
        let mut base = BASE_URIS.lock();
        #[allow(clippy::map_unwrap_or)]
        base.iter()
            .rev()
            .find(|e| **e == *s)
            .map(|e| Ok(Self(e.url.clone())))
            .unwrap_or_else(|| Self::make_new(url::Url::parse(s)?, &mut base))
    }
}

impl std::fmt::Display for BaseUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// Internal structure for interning base URIs.
///
/// This structure stores a hash of the URL string for fast lookups,
/// the string representation, and the shared URL instance.
struct InternedBaseURI {
    hash: u64,
    string: Box<str>,
    url: Arc<url::Url>,
}
impl From<url::Url> for InternedBaseURI {
    fn from(url: url::Url) -> Self {
        let string = url.as_str().trim_end_matches('/').into();
        let hash = rustc_hash::FxBuildHasher.hash_one(&string);
        Self {
            hash,
            string,
            url: url.into(),
        }
    }
}

impl PartialEq<str> for InternedBaseURI {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        let h = rustc_hash::FxBuildHasher.hash_one(other);
        h == self.hash && *other == *self.string
    }
}
impl PartialEq<url::Url> for InternedBaseURI {
    #[inline]
    fn eq(&self, other: &url::Url) -> bool {
        let h = rustc_hash::FxBuildHasher.hash_one(other.as_str().trim_end_matches('/'));
        h == self.hash && *other == *self.url
    }
}

static BASE_URIS: std::sync::LazyLock<parking_lot::Mutex<Vec<InternedBaseURI>>> =
    std::sync::LazyLock::new(|| parking_lot::Mutex::new(Vec::with_capacity(8)));
static UNKNOWN_BASE: std::sync::LazyLock<BaseUri> = std::sync::LazyLock::new(||
    // SAFETY: known to be well-formed Url
    unsafe{
        BaseUri::from_str("http://unknown.source").unwrap_unchecked()
    });

const MAX: usize = 8;

/// Cleans up the base URI cache by removing entries that are no longer referenced.
#[inline]
fn clean(v: &mut Vec<InternedBaseURI>) {
    fn actually_clean(v: &mut Vec<InternedBaseURI>) {
        v.retain(|e| Arc::strong_count(&e.url) > 1);
    }
    if v.len() > MAX {
        actually_clean(v);
    }
}

crate::tests! {
    base_uris {
        tracing::info!("Size of BaseURI: {}", std::mem::size_of::<BaseUri>());
        let s = BaseUri::unknown().as_str();
        assert_eq!(s, "http://unknown.source");
        let test = BaseUri::from_str("https://mathhub.info/foo/bar/").expect("works");
        assert_eq!(test.as_str(), "https://mathhub.info/foo/bar");
        let test = BaseUri::from_str("https://mathhub.info/foo/bar").expect("works");
        assert_eq!(test.as_str(), "https://mathhub.info/foo/bar");
    };
    base_uris_parsing {
        // Test various malformed URIs
        assert!(BaseUri::from_str("not a url").is_err());
        assert!(BaseUri::from_str("http://example.com#fragment").is_err());
        assert!(BaseUri::from_str("http://example.com?query=value").is_err());
        assert!(BaseUri::from_str("data:text/plain;base64,SGVsbG8=").is_err());

        // Test valid URIs
        assert!(BaseUri::from_str("http://example.com").is_ok());
        assert!(BaseUri::from_str("https://example.com/path").is_ok());
        assert!(BaseUri::from_str("http://example.com/").is_ok());
    };
    base_uri_interning {
        // Test that base URIs are properly interned
        let uri1 = BaseUri::from_str("http://example.com").expect("works");
        let uri2 = BaseUri::from_str("http://example.com").expect("works");
        let uri3 = BaseUri::from_str("http://example.com/").expect("works"); // Trailing slash should be normalized

        // Should be the same instance
        assert_eq!(uri1, uri2);
        assert_eq!(uri1, uri3);
    };
    url_edge_cases {
        // Test various special URL cases
        assert!(BaseUri::from_str("file:///path/to/file").is_ok());
        assert!(BaseUri::from_str("ftp://example.com").is_ok());
        assert!(BaseUri::from_str("http://localhost:8080").is_ok());
        assert!(BaseUri::from_str("https://user:pass@example.com").is_ok());

        // Test normalization
        let uri1 = BaseUri::from_str("http://example.com/path/").expect("works");
        let uri2 = BaseUri::from_str("http://example.com/path").expect("works");
        assert_eq!(uri1.as_str(), "http://example.com/path");
        assert_eq!(uri2.as_str(), "http://example.com/path");
    };
    concurrent_base_uris {
        use std::sync::{Arc,Barrier};
        // Test thread safety of BaseUri creation
        const NUM_THREADS: usize = 50;
        const NUM_URIS: usize = 20;

        let barrier = Arc::new(Barrier::new(NUM_THREADS));
        let mut handles = vec![];

        for i in 0..NUM_THREADS {
            let barrier = Arc::clone(&barrier);
            let handle = std::thread::spawn(move || {
                barrier.wait();

                let mut uris = Vec::new();
                for j in 0..NUM_URIS {
                    let uri = BaseUri::from_str(&format!("http://example{j}.com/path{i}")).expect("works");
                    uris.push(uri);
                }

                // Verify all URIs are valid
                for uri in &uris {
                    assert!(!uri.as_str().is_empty());
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("works");
        }
    }
}
