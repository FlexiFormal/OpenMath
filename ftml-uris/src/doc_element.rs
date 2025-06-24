use std::str::FromStr;

use const_format::concatcp;

use crate::{
    ArchiveUri, BaseUri, DocumentUri, IsFtmlUri, IsNarrativeUri, PathUri, UriComponentKind,
    UriKind, UriName, UriWithArchive, UriWithPath, errors::UriParseError,
};

/// A URI that identifies some element of a document, like a section, paragraph, etc.
///
/// A [`DocumentElementUri`] extends a [`DocumentUri`] with a [`UriName`].
/// [`DocumentElementUri`]s have the form:
/// `http://example.com?a=archive&p=path&d=document&e=element/name`
///
/// The name is hierarchical and can contain forward slashes to represent
/// nested elements.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let element_uri = DocumentElementUri::from_str("http://example.com?a=math&d=algebra&l=en&e=groups/cyclic").unwrap();
///
/// assert_eq!(element_uri.name.as_ref(), "groups/cyclic");
/// assert_eq!(element_uri.document_name().as_ref(), "algebra");
/// assert_eq!(element_uri.archive_id().to_string(), "math");
/// assert_eq!(element_uri.base().as_str(), "http://example.com");
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct DocumentElementUri {
    /// The hierarchical name of the symbol.
    pub name: UriName,
    /// The module component.
    pub document: DocumentUri,
}
crate::ts!(DocumentElementUri);
crate::debugdisplay!(DocumentElementUri);
impl crate::sealed::Sealed for DocumentElementUri {}

impl std::fmt::Display for DocumentElementUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}&{}={}", self.document, Self::SEPARATOR, self.name)
    }
}

impl DocumentElementUri {
    pub(crate) const SEPARATOR: char = 'e';

    /// Returns the name of this element.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let element_uri = DocumentElementUri::from_str("http://example.com?a=archive&p=path&d=doc&l=en&e=element/name").unwrap();
    /// assert_eq!(element_uri.name().as_ref(),"element/name");
    /// ````
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &UriName {
        &self.name
    }

    /// Returns the parent of this element, if one exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let element_uri = DocumentElementUri::from_str("http://example.com?a=archive&p=path&d=doc&l=en&e=element/name").unwrap();
    /// let parent = element_uri.parent().unwrap();
    /// assert_eq!(parent.to_string(),"http://example.com?a=archive&p=path&d=doc&l=en&e=element");
    /// assert!(parent.parent().is_none());
    /// ````
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.name.is_simple() {
            return None;
        }
        // SAFETY: !is_simple() entails name contains at least one `/` and has no illegal characters or illegal segments.
        let name = unsafe {
            self.name
                .as_ref()
                .rsplit_once('/')
                .unwrap_unchecked()
                .0
                .parse()
                .unwrap_unchecked()
        };
        Some(Self {
            document: self.document.clone(),
            name,
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
        DocumentUri::pre_parse(s, uri_kind, |document, mut split| {
            let Some(s) = split.next() else {
                return Err(UriParseError::MissingPartFor {
                    uri_kind,
                    part: UriComponentKind::e,
                });
            };
            s.strip_prefix(concatcp!(DocumentElementUri::SEPARATOR, "="))
                .map_or_else(
                    || {
                        Err(UriParseError::MissingPartFor {
                            uri_kind,
                            part: UriComponentKind::e,
                        })
                    },
                    |name| {
                        f(
                            Self {
                                document,
                                name: name.parse()?,
                            },
                            split,
                        )
                    },
                )
        })
    }
}

impl FromStr for DocumentElementUri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::pre_parse(s, UriKind::DocumentElementUri, |u, mut split| {
            if split.next().is_some() {
                return Err(UriParseError::TooManyPartsFor {
                    uri_kind: UriKind::DocumentElementUri,
                });
            }
            Ok(u)
        })
    }
}

impl From<DocumentElementUri> for DocumentUri {
    #[inline]
    fn from(value: DocumentElementUri) -> Self {
        value.document
    }
}
impl From<DocumentElementUri> for PathUri {
    #[inline]
    fn from(value: DocumentElementUri) -> Self {
        value.document.path
    }
}
impl From<DocumentElementUri> for ArchiveUri {
    fn from(value: DocumentElementUri) -> Self {
        value.document.path.archive
    }
}
impl From<DocumentElementUri> for BaseUri {
    #[inline]
    fn from(value: DocumentElementUri) -> Self {
        value.document.path.archive.base
    }
}
impl IsFtmlUri for DocumentElementUri {
    #[inline]
    fn base(&self) -> &crate::BaseUri {
        &self.document.path.archive.base
    }
}
impl UriWithPath for DocumentElementUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        &self.document.path
    }
}
impl UriWithArchive for DocumentElementUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        &self.document.path.archive
    }
}

impl IsNarrativeUri for DocumentElementUri {
    #[inline]
    fn document_uri(&self) -> &DocumentUri {
        &self.document
    }
}

crate::tests! {
    doc_elem {
        tracing::info!("Size of DocumentElementUri: {}",std::mem::size_of::<DocumentElementUri>());
    };
    element_uri_parsing {
        use std::str::FromStr;
        // Valid element URIs
        let simple = DocumentElementUri::from_str("http://example.com?a=archive&d=document&l=sl&e=element").expect("works");
        assert_eq!(simple.name.as_ref(), "element");
        assert_eq!(simple.archive_id().as_ref(), "archive");
        assert_eq!(simple.document_name().as_ref(), "document");
        assert!(simple.path().is_none());

        let with_path = DocumentElementUri::from_str("http://example.com?a=archive&p=folder&d=math&l=ru&e=algebra/group").expect("works");
        assert_eq!(with_path.name.as_ref(), "algebra/group");
        assert_eq!(with_path.document_name().to_string(), "math");
        assert_eq!(with_path.path().expect("works").to_string(), "folder");

        // Invalid element URIs
        assert!(DocumentElementUri::from_str("http://example.com?a=archive").is_err());
        assert!(DocumentElementUri::from_str("http://example.com?a=archive&m=").is_err());
        assert!(DocumentElementUri::from_str("http://example.com?a=archive&d=a//b").is_err());
        assert!(DocumentElementUri::from_str("http://example.com?a=archive&d=foo&l=en&e=").is_err());
        assert!(DocumentElementUri::from_str("http://example.com?a=archive&d=foo&&l=de&e=a//b").is_err());
    };
    element_uri_parents {
        let with_path = DocumentElementUri::from_str("http://example.com?a=archive&p=folder&d=math&l=ru&e=algebra/group").expect("works");
        let parent_str = "http://example.com?a=archive&p=folder&d=math&l=ru&e=algebra";
        let parent = with_path.parent().expect("works");
        assert_eq!(parent.to_string(),parent_str);
        assert!(parent.parent().is_none());
    };
    element_uri_display {
        use std::str::FromStr;
        let symbol_uri = DocumentElementUri::from_str("http://example.com?a=archive&p=path&d=doc&l=en&e=elem").expect("works");
        let expected = "http://example.com?a=archive&p=path&d=doc&l=en&e=elem";
        assert_eq!(symbol_uri.to_string(), expected);

        let no_path = DocumentElementUri::from_str("http://example.com?a=archive&d=doc&l=de&e=elem").expect("works");
        let expected_no_path = "http://example.com?a=archive&d=doc&l=de&e=elem";
        assert_eq!(no_path.to_string(), expected_no_path);
    }
}
