use std::str::FromStr;

use const_format::concatcp;

use crate::{
    ArchiveUri, BaseUri, IsDomainUri, IsFtmlUri, ModuleUri, PathUri, UriComponentKind, UriKind,
    UriName, UriWithArchive, UriWithPath, errors::UriParseError,
};

/// A URI that identifies a specific concept.
///
/// A [`SymbolUri`] extends a [`ModuleUri`] with a symbol [`UriName`], creating a complete
/// reference to a symbol within a module. [`SymbolUri`]s have the form:
/// `http://example.com?a=archive&p=path&m=module/name&s=symbol/name`
///
/// The symbol name is hierarchical and can contain forward slashes to represent
/// nested symbols.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let symbol_uri = SymbolUri::from_str("http://example.com?a=math&m=algebra/groups&s=cyclic").unwrap();
///
/// assert_eq!(symbol_uri.name.to_string(), "cyclic");
/// assert_eq!(symbol_uri.module_name().to_string(), "algebra/groups");
/// assert_eq!(symbol_uri.archive_id().to_string(), "math");
/// assert_eq!(symbol_uri.base().as_str(), "http://example.com");
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub struct SymbolUri {
    /// The hierarchical name of the symbol.
    pub name: UriName,
    /// The module component.
    pub module: ModuleUri,
}
crate::ts!(SymbolUri);
crate::debugdisplay!(SymbolUri);
impl crate::sealed::Sealed for SymbolUri {}

impl std::fmt::Display for SymbolUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}&{}={}", self.module, Self::SEPARATOR, self.name)
    }
}

impl SymbolUri {
    pub(crate) const SEPARATOR: char = 's';

    /// Returns the name of this symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let symbol_uri = SymbolUri::from_str("http://example.com?a=archive&p=path&m=some/module&s=symbol/name").unwrap();
    /// assert_eq!(symbol_uri.name().as_ref(),"symbol/name");
    /// ````
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &UriName {
        &self.name
    }

    /// Converts this module into a [`SymbolUri`].
    ///
    /// Concatenates the name of this symbol to the name of the containing module, generating
    /// a [`ModuleUri`] for a nested module with the name of this symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ftml_uris::prelude::*;
    /// # use std::str::FromStr;
    /// let symbol_uri = SymbolUri::from_str("http://example.com?a=archive&p=path&m=some/module&s=symbol/name").unwrap();
    /// let as_module = symbol_uri.into_module();
    /// assert_eq!(as_module.to_string(),"http://example.com?a=archive&p=path&m=some/module/symbol/name");
    /// ````
    #[must_use]
    #[inline]
    pub fn into_module(self) -> ModuleUri {
        self.module / &self.name
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
        ModuleUri::pre_parse(s, uri_kind, |module, mut split| {
            let Some(s) = split.next() else {
                return Err(UriParseError::MissingPartFor {
                    uri_kind,
                    part: UriComponentKind::s,
                });
            };
            s.strip_prefix(concatcp!(SymbolUri::SEPARATOR, "="))
                .map_or_else(
                    || {
                        Err(UriParseError::MissingPartFor {
                            uri_kind,
                            part: UriComponentKind::s,
                        })
                    },
                    |name| {
                        f(
                            Self {
                                module,
                                name: name.parse()?,
                            },
                            split,
                        )
                    },
                )
        })
    }
}

impl FromStr for SymbolUri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::pre_parse(s, UriKind::SymbolUri, |u, mut split| {
            if split.next().is_some() {
                return Err(UriParseError::TooManyPartsFor {
                    uri_kind: UriKind::SymbolUri,
                });
            }
            Ok(u)
        })
    }
}

impl From<SymbolUri> for ModuleUri {
    #[inline]
    fn from(value: SymbolUri) -> Self {
        value.module
    }
}
impl From<SymbolUri> for PathUri {
    #[inline]
    fn from(value: SymbolUri) -> Self {
        value.module.path
    }
}
impl From<SymbolUri> for ArchiveUri {
    fn from(value: SymbolUri) -> Self {
        value.module.path.archive
    }
}
impl From<SymbolUri> for BaseUri {
    #[inline]
    fn from(value: SymbolUri) -> Self {
        value.module.path.archive.base
    }
}
impl IsFtmlUri for SymbolUri {
    #[inline]
    fn base(&self) -> &crate::BaseUri {
        &self.module.path.archive.base
    }
}
impl UriWithPath for SymbolUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        &self.module.path
    }
}
impl UriWithArchive for SymbolUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        &self.module.path.archive
    }
}

impl IsDomainUri for SymbolUri {
    #[inline]
    fn module_uri(&self) -> &ModuleUri {
        &self.module
    }
}

crate::tests! {
    symbol {
        tracing::info!("Size of SymbolUri: {}",std::mem::size_of::<SymbolUri>());
    };
    symbol_uri_parsing {
        use std::str::FromStr;
        // Valid module URIs
        let simple = SymbolUri::from_str("http://example.com?a=archive&m=module&s=symbol").expect("works");
        assert_eq!(simple.name.to_string(), "symbol");
        assert_eq!(simple.archive_id().to_string(), "archive");
        assert_eq!(simple.module_name().to_string(), "module");
        assert!(simple.path().is_none());

        let with_path = SymbolUri::from_str("http://example.com?a=archive&p=folder&m=math/algebra&s=group").expect("works");
        assert_eq!(with_path.name.to_string(), "group");
        assert_eq!(with_path.module_name().to_string(), "math/algebra");
        assert_eq!(with_path.path().expect("works").to_string(), "folder");

        // Invalid module URIs
        assert!(SymbolUri::from_str("http://example.com?a=archive").is_err());
        assert!(SymbolUri::from_str("http://example.com?a=archive&m=").is_err());
        assert!(SymbolUri::from_str("http://example.com?a=archive&m=a//b").is_err());
        assert!(SymbolUri::from_str("http://example.com?a=archive&m=foo&s=").is_err());
        assert!(SymbolUri::from_str("http://example.com?a=archive&m=foo&s=a//b").is_err());
    };
    symbol_uri_traits {
        use std::str::FromStr;
        use crate::{IsFtmlUri, UriWithArchive, UriWithPath, IsDomainUri};

        let symbol_uri = SymbolUri::from_str("http://example.com?a=math&p=textbooks&m=algebra/groups&s=cyclic").expect("works");

        // Test IsFtmlUri
        assert_eq!(symbol_uri.base().as_str(), "http://example.com");

        // Test UriWithArchive
        assert_eq!(symbol_uri.archive_id().to_string(), "math");
        assert_eq!(symbol_uri.archive_uri().to_string(), "http://example.com?a=math");

        // Test UriWithPath
        assert_eq!(symbol_uri.path().expect("works").to_string(), "textbooks");

        // Test IsContentUri
        assert_eq!(symbol_uri.module_name().to_string(), "algebra/groups");

        // Test conversions
        let path_uri: PathUri = symbol_uri.clone().into();
        assert_eq!(symbol_uri.path().expect("works").to_string(), "textbooks");

        let archive_uri: ArchiveUri = symbol_uri.clone().into();
        assert_eq!(archive_uri.id.to_string(), "math");

        let base_uri: BaseUri = symbol_uri.into();
        assert_eq!(base_uri.as_str(), "http://example.com");
    };
    symbol_uri_display {
        use std::str::FromStr;
        let symbol_uri = SymbolUri::from_str("http://example.com?a=archive&p=path&m=module&s=symbol").expect("works");
        let expected = "http://example.com?a=archive&p=path&m=module&s=symbol";
        assert_eq!(symbol_uri.to_string(), expected);

        let no_path = SymbolUri::from_str("http://example.com?a=archive&m=module&s=symbol").expect("works");
        let expected_no_path = "http://example.com?a=archive&m=module&s=symbol";
        assert_eq!(no_path.to_string(), expected_no_path);
    }
}
