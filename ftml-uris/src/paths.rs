use std::str::FromStr;

use const_format::concatcp;

use crate::{
    BaseUri, IsFtmlUri, UriKind, UriWithArchive, UriWithPath,
    archive::ArchiveUri,
    aux::interning::{InternMap, InternStore, NonEmptyInternedStr},
    errors::{SegmentParseError, UriParseError},
};

static PATHS: std::sync::LazyLock<InternMap> = std::sync::LazyLock::new(InternMap::default);

/// Storage configuration for interned URI paths.
pub struct PathStore;
impl InternStore for PathStore {
    const LIMIT: usize = 1024;
    #[inline]
    fn get() -> &'static InternMap {
        &PATHS
    }
}

/// A path within an FTML archive.
///
/// [`UriPath`] represents a forward-slash separated path that can be used to
/// navigate within an archive, such as "folder/subfolder"
/// or "documents/papers". Paths are interned for efficient storage and fast
/// equality comparisons.
///
/// Paths cannot be empty and cannot contain empty segments (no leading,
/// trailing, or consecutive forward slashes).
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let path = UriPath::from_str("folder/subfolder/subsub").unwrap();
/// assert_eq!(path.as_ref(), "folder/subfolder/subsub");
///
/// // Navigate up the path hierarchy
/// let parent = path.up().unwrap();
/// assert_eq!(parent.as_ref(), "folder/subfolder");
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct UriPath(pub(crate) NonEmptyInternedStr<PathStore>);
crate::ts!(UriPath);
crate::debugdisplay!(UriPath);
impl AsRef<str> for UriPath {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl UriPath {
    /// Returns the parent path by removing the last segment.
    ///
    /// Returns `None` if this is already a top-level path (no parent).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let path = UriPath::from_str("folder/subfolder/subsub").unwrap();
    /// let parent = path.up().unwrap();
    /// assert_eq!(parent.as_ref(), "folder/subfolder");
    ///
    /// let top_level = UriPath::from_str("folder").unwrap();
    /// assert!(top_level.up().is_none());
    ///
    /// let grandparent = parent.up().unwrap();
    /// assert_eq!(grandparent.as_ref(), "folder");
    ///
    /// // Top-level path has no parent
    /// assert!(grandparent.up().is_none());
    /// ```
    #[inline]
    pub fn up(&self) -> Option<Self> {
        self.0.up::<'/'>().map(Self)
    }
}

impl FromStr for UriPath {
    type Err = SegmentParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(NonEmptyInternedStr::new_with_sep::<'/'>(s)?))
    }
}
impl std::fmt::Display for UriPath {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A URI that identifies a location within an FTML archive.
///
/// A [`PathUri`] extends an [`ArchiveUri`] with an optional path component,
/// e.g. referencing a particular directory within an archive on a file system.
/// [`PathUri`]s have the form: `http://example.com?a=archive[&p=path/to/location]`
///
/// A `PathUri` without a path component
/// simply references the root of the archive. Hence, [`ArchiveUri`] implements <code>[Into]<[PathUri]></code>.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// // Path URI with a path component
/// let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder/file").unwrap();
/// assert_eq!(path_uri.archive.id.as_ref(), "archive");
/// assert_eq!(path_uri.path.as_ref().unwrap().as_ref(), "folder/file");
///
/// // Path URI without a path component (archive root)
/// let root_uri = PathUri::from_str("http://example.com?a=archive").unwrap();
/// assert!(root_uri.path.is_none());
///
/// // Navigation within the path
/// let parent = path_uri.up();
/// assert_eq!(parent.path.as_ref().unwrap().to_string(), "folder");
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct PathUri {
    /// The optional path component within the archive.
    pub path: Option<UriPath>,
    /// The archive component.
    pub archive: ArchiveUri,
}
crate::ts!(PathUri);
crate::debugdisplay!(PathUri);
impl crate::sealed::Sealed for PathUri {}

impl PathUri {
    pub(crate) const SEPARATOR: char = 'p';

    /// Navigates up to the parent path.
    ///
    /// If the current path has a parent, returns a new `PathUri`
    /// pointing to that parent. If the current path is at the root of an
    /// archive, returns a `PathUri` with no path component (archive root).
    /// If already at the archive root, returns self unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder/subfolder/file").unwrap();
    ///
    /// let parent = path_uri.up();
    /// assert_eq!(parent.path.as_ref().unwrap().to_string(), "folder/subfolder");
    ///
    /// let grandparent = parent.up();
    /// assert_eq!(grandparent.path.as_ref().unwrap().to_string(), "folder");
    ///
    /// let root = grandparent.up();
    /// assert!(root.path.is_none()); // Now at archive root
    ///
    /// let still_root = root.up();
    /// assert!(still_root.path.is_none()); // Still at archive root
    /// ```
    #[must_use]
    pub fn up(self) -> Self {
        if let Some(path) = self.path {
            if let Some(npath) = path.up() {
                Self {
                    archive: self.archive,
                    path: Some(npath),
                }
            } else {
                Self {
                    archive: self.archive,
                    path: None,
                }
            }
        } else {
            self
        }
    }

    /// Internal parsing method used by URI parsing infrastructure.
    ///
    /// This method handles the common parsing logic for path URIs and
    /// URI types that extend path URIs (like module URIs).
    pub(super) fn pre_parse<R>(
        s: &str,
        uri_kind: UriKind,
        f: impl FnOnce(Self, Option<&str>, std::str::Split<char>) -> Result<R, UriParseError>,
    ) -> Result<R, UriParseError> {
        ArchiveUri::pre_parse(s, uri_kind, |archive, mut split| {
            let (p, n) = if let Some(p) = split.next() {
                if let Some(p) = p.strip_prefix(concatcp!(PathUri::SEPARATOR, "=")) {
                    (
                        Self {
                            archive,
                            path: Some(p.parse()?),
                        },
                        None,
                    )
                } else {
                    (
                        Self {
                            archive,
                            path: None,
                        },
                        Some(p),
                    )
                }
            } else {
                (
                    Self {
                        archive,
                        path: None,
                    },
                    None,
                )
            };
            f(p, n, split)
        })
    }
}
impl std::fmt::Display for PathUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{}&{}={}", self.archive, Self::SEPARATOR, path)
        } else {
            std::fmt::Display::fmt(&self.archive, f)
        }
    }
}
impl FromStr for PathUri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::pre_parse(s, UriKind::PathUri, |u, next, mut split| {
            if next.is_some() || split.next().is_some() {
                return Err(UriParseError::TooManyPartsFor {
                    uri_kind: UriKind::PathUri,
                });
            }
            Ok(u)
        })
    }
}

impl From<ArchiveUri> for PathUri {
    #[inline]
    fn from(value: ArchiveUri) -> Self {
        Self {
            archive: value,
            path: None,
        }
    }
}
impl From<PathUri> for ArchiveUri {
    fn from(value: PathUri) -> Self {
        value.archive
    }
}
impl From<PathUri> for BaseUri {
    #[inline]
    fn from(value: PathUri) -> Self {
        value.archive.base
    }
}
impl IsFtmlUri for PathUri {
    #[inline]
    fn base(&self) -> &crate::BaseUri {
        &self.archive.base
    }
}
impl UriWithPath for PathUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        self
    }
}
impl UriWithArchive for PathUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        &self.archive
    }
}

crate::tests! {
    paths {
        tracing::info!("Size of UriPath: {}",std::mem::size_of::<UriPath>());
        tracing::info!("Size of PathUri: {}",std::mem::size_of::<PathUri>());
    };
    path_uri_parsing {
        // Valid path URIs
        let uri = PathUri::from_str("http://example.com?a=archive&p=some/path").expect("works");
        assert_eq!(uri.archive.base.to_string(), "http://example.com");
        assert_eq!(uri.archive.id.to_string(), "archive");
        assert_eq!(uri.path.as_ref().expect("works").to_string(), "some/path");

        // Without path
        let uri = PathUri::from_str("http://example.com?a=archive").expect("works");
        assert!(uri.path.is_none());

        // Invalid path URIs
        assert!(PathUri::from_str("http://example.com?a=archive&p=").is_err());
        assert!(PathUri::from_str("http://example.com?a=archive&p=a//b").is_err());
    }
}
