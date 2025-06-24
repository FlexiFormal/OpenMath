use std::str::FromStr;

use const_format::concatcp;

use crate::{
    ArchiveUri, BaseUri, IsFtmlUri, IsNarrativeUri, Language, ModuleUri, PathUri, UriComponentKind,
    UriKind, UriName, UriPath, UriWithArchive, UriWithPath,
    aux::interning::NonEmptyInternedStr,
    errors::{SegmentParseError, UriParseError},
};

static NO_DOCUMENT: std::sync::LazyLock<DocumentUri> = std::sync::LazyLock::new(|| unsafe {
    "http://unknown.source?a=no/archive&d=unknown_document&l=en"
        .parse()
        .unwrap_unchecked()
});

/// A non-hierarchical name used for document.
///
/// [`SimpleUriName`] represents a [`UriName`](crate::UriName) that can *not* contain forward slashes as
/// separators. Names are interned for
/// efficient storage and fast equality comparisons.
/// Names cannot be empty.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let name = SimpleUriName::from_str("math").unwrap();
///
/// assert_eq!(name.as_ref(), "math");
///
/// assert!(SimpleUriName::from_str("math/algebra").is_err());
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct SimpleUriName(pub(crate) NonEmptyInternedStr<super::module::NameStore>);
crate::ts!(SimpleUriName);
crate::debugdisplay!(SimpleUriName);
impl AsRef<str> for SimpleUriName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl FromStr for SimpleUriName {
    type Err = SegmentParseError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('/') {
            return Err(SegmentParseError::IllegalChar('/'));
        }
        Ok(Self(NonEmptyInternedStr::new(s)?))
    }
}
impl std::fmt::Display for SimpleUriName {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A URI that identifies a specific document within an FTML archive.
///
/// A [`DocumentUri`] extends a [`PathUri`] with a document name and a [`Language`], creating a complete
/// reference to a document within an archive. [`DocumentUri`]s have the form:
/// `http://example.com?a=archive&p=path&m=module/name&d=doc&l=en`
///
/// Unlike [`UriName`]s, document names are *not* hierarchical and can *not* contain forward slashes.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let doc_uri = DocumentUri::from_str("http://example.com?a=math&d=algebra&l=en").unwrap();
///
/// assert_eq!(doc_uri.document_name().as_ref(), "algebra");
/// assert_eq!(doc_uri.archive_id().as_ref(), "math");
/// assert_eq!(doc_uri.base().as_str(), "http://example.com");
///
/// // Document URIs can have paths within the archive
/// let with_path = DocumentUri::from_str("http://example.com?a=math&p=textbooks&d=algebra&l=de").unwrap();
/// assert_eq!(with_path.path().unwrap().as_ref(), "textbooks");
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct DocumentUri {
    /// The name of the document.
    pub name: SimpleUriName,
    /// The path component specifying location within the archive.
    pub path: PathUri,
    /// The language of the document.
    pub language: Language,
}
crate::ts!(DocumentUri);
crate::debugdisplay!(DocumentUri);
impl crate::sealed::Sealed for DocumentUri {}

impl std::fmt::Display for DocumentUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}&{}={}&{}={}",
            self.path,
            Self::SEPARATOR,
            self.name,
            Language::SEPARATOR,
            self.language
        )
    }
}

impl DocumentUri {
    pub(crate) const SEPARATOR: char = 'd';

    /// Returns a reference to the default anonymous "no document".
    #[inline]
    #[must_use]
    pub fn no_doc() -> &'static Self {
        &NO_DOCUMENT
    }

    /// Returns the URI of a document from and archive and its path relative to the archive.
    ///
    /// Attempts to extract the [`Language`] of the document from the filename, dropping the extension (assumed
    /// to be present); otherwise useing [`English`](Language::English) as a default. Then uses the filename
    /// without language id and extension as the name, and the rest of the relative filepath as the path component.
    ///
    /// #### Errors
    ///
    /// if the provided name is not a valid document name (e.g. contains illegal characters).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let archive_uri: ArchiveUri = "http://example.com?a=some/archive".parse().unwrap();
    /// let document_uri = DocumentUri::from_archive_relpath(archive_uri,"some/subfolder/foo.de.tex").unwrap();
    /// assert_eq!(document_uri.to_string(),"http://example.com?a=some/archive&p=some/subfolder&d=foo&l=de");
    /// ```
    pub fn from_archive_relpath(a: ArchiveUri, rel_path: &str) -> Result<Self, UriParseError> {
        #[cfg(windows)]
        let replaced = rel_path.replace('\\', "/");
        #[cfg(windows)]
        let rel_path = &replaced;
        let (path, mut name) = rel_path.rsplit_once('/').unwrap_or(("", rel_path));
        name = name.rsplit_once('.').map_or_else(|| name, |(name, _)| name);
        let lang = Language::from_rel_path(name);
        let path: Option<UriPath> = if path.is_empty() {
            None
        } else {
            Some(path.parse()?)
        };
        name = name.strip_suffix(&format!(".{lang}")).unwrap_or(name);
        Ok((a / path) & (name.parse()?, lang))
    }

    /// Returns the URI of a module within this document
    ///
    /// If the name's first segment is equal to the document's name, the module's [`PathUri`] and first name
    /// are those of the document. Otherwise, the document's name is appended to the path, and the module's name
    /// is the one provided.
    ///
    /// #### Errors
    ///
    /// if the provided name is not a valid module name (e.g. contains illegal characters or empty segments).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let doc_uri: DocumentUri = "http://example.com?a=archive&p=path&d=math&l=en".parse().unwrap();
    /// let module_uri = doc_uri.module_uri_from("math").unwrap();
    /// assert_eq!(module_uri.to_string(),"http://example.com?a=archive&p=path&m=math");
    /// let module_uri = doc_uri.module_uri_from("algebra").unwrap();
    /// assert_eq!(module_uri.to_string(),"http://example.com?a=archive&p=path/math&m=algebra");
    /// let module_uri = doc_uri.module_uri_from("math/algebra").unwrap();
    /// assert_eq!(module_uri.to_string(),"http://example.com?a=archive&p=path&m=math/algebra");
    /// let module_uri = doc_uri.module_uri_from("algebra/groups").unwrap();
    /// assert_eq!(module_uri.to_string(),"http://example.com?a=archive&p=path/math&m=algebra/groups");
    /// ```
    pub fn module_uri_from(&self, name: &str) -> Result<ModuleUri, UriParseError> {
        let (first_name, rest) = name
            .split_once('/')
            .map_or((name, None), |(n, r)| (n, Some(r)));
        if self.name.as_ref() == first_name {
            if rest.is_some() {
                Ok(self.path.clone() | name.parse()?)
            } else {
                Ok(self.path.clone() | UriName(self.name.0.clone()))
            }
        } else {
            Ok((self.path.clone() / UriPath(self.name.as_ref().parse()?)) | name.parse()?)
        }
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
                    part: UriComponentKind::d,
                });
            };
            m.strip_prefix(concatcp!(DocumentUri::SEPARATOR, "="))
                .map_or_else(
                    || {
                        Err(UriParseError::MissingPartFor {
                            uri_kind,
                            part: UriComponentKind::d,
                        })
                    },
                    |name| {
                        let Some(l) = split.next() else {
                            return Err(UriParseError::MissingPartFor {
                                uri_kind,
                                part: UriComponentKind::l,
                            });
                        };
                        l.strip_prefix(concatcp!(Language::SEPARATOR, "="))
                            .map_or_else(
                                || {
                                    Err(UriParseError::MissingPartFor {
                                        uri_kind,
                                        part: UriComponentKind::l,
                                    })
                                },
                                |lang| {
                                    let language = lang
                                        .parse()
                                        .map_or_else(|_| Err(UriParseError::InvalidLanguage), Ok)?;
                                    f(
                                        Self {
                                            path,
                                            name: name.parse()?,
                                            language,
                                        },
                                        split,
                                    )
                                },
                            )
                    },
                )
        })
    }
}
impl FromStr for DocumentUri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::pre_parse(s, UriKind::DocumentUri, |u, mut split| {
            if split.next().is_some() {
                return Err(UriParseError::TooManyPartsFor {
                    uri_kind: UriKind::DocumentUri,
                });
            }
            Ok(u)
        })
    }
}

impl From<DocumentUri> for PathUri {
    #[inline]
    fn from(value: DocumentUri) -> Self {
        value.path
    }
}
impl From<DocumentUri> for ArchiveUri {
    fn from(value: DocumentUri) -> Self {
        value.path.archive
    }
}
impl From<DocumentUri> for BaseUri {
    #[inline]
    fn from(value: DocumentUri) -> Self {
        value.path.archive.base
    }
}
impl IsFtmlUri for DocumentUri {
    #[inline]
    fn base(&self) -> &crate::BaseUri {
        &self.path.archive.base
    }
}
impl UriWithPath for DocumentUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        &self.path
    }
}
impl UriWithArchive for DocumentUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        &self.path.archive
    }
}
impl IsNarrativeUri for DocumentUri {
    #[inline]
    fn document_uri(&self) -> &DocumentUri {
        self
    }
}

crate::tests! {
    document {
        tracing::info!("Size of SimpleUriName: {}",std::mem::size_of::<SimpleUriName>());
        tracing::info!("Size of DocumentUri: {}",std::mem::size_of::<DocumentUri>());
    };
    uri_simple_name_parsing {
        use std::str::FromStr;
        // Valid names
        let simple = SimpleUriName::from_str("module").expect("works");
        assert_eq!(simple.to_string(), "module");

        assert!(SimpleUriName::from_str("math/algebra/groups").is_err());

        // Invalid names
        assert!(SimpleUriName::from_str("").is_err());
        assert!(SimpleUriName::from_str("/").is_err());
        assert!(SimpleUriName::from_str("a/").is_err());
        assert!(SimpleUriName::from_str("/a").is_err());
        assert!(SimpleUriName::from_str("a//b").is_err());
    };
    document_uri_parsing {
        use std::str::FromStr;
        // Valid module URIs
        let simple = DocumentUri::from_str("http://example.com?a=archive&d=document&l=ru").expect("works");
        assert_eq!(simple.name.as_ref(), "document");
        assert_eq!(simple.archive_id().as_ref(), "archive");
        assert!(simple.path().is_none());

        let with_path = DocumentUri::from_str("http://example.com?a=archive&p=folder&d=math&l=en").expect("works");
        assert_eq!(with_path.name.as_ref(), "math");
        assert_eq!(with_path.path().expect("works").as_ref(), "folder");

        // Invalid module URIs
        assert!(DocumentUri::from_str("http://example.com?a=archive").is_err());
        assert!(DocumentUri::from_str("http://example.com?a=archive&d=").is_err());
        assert!(DocumentUri::from_str("http://example.com?a=archive&d=document").is_err());
        assert!(DocumentUri::from_str("http://example.com?a=archive&d=a/b&l=de").is_err());
    };
    document_uri_display {
        use std::str::FromStr;
        let document_uri = DocumentUri::from_str("http://example.com?a=archive&p=path&d=document&l=fr").expect("works");
        let expected = "http://example.com?a=archive&p=path&d=document&l=fr";
        assert_eq!(document_uri.to_string(), expected);

        let no_path = ArchiveUri::from_str("http://example.com?a=archive").expect("works");
        let no_path = no_path & ("doc".parse().expect("works"),Language::Bulgarian);
        let expected_no_path = "http://example.com?a=archive&d=doc&l=bg";
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
    }
}
