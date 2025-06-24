#![allow(unexpected_cfgs)]
#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

/*! # FTML URIs
 *
 * [FTML](https://mathhub.info/?a=Papers%2F25-CICM-FLAMS&d=paper&l=en) URIs are intended to serve as hierarchical,
 * globally unique identifiers, are used as keys for retrieving identified content elements, and occur in large
 * numbers in FTML documents. As such, it is important that they are fast to parse, clone, equality-check,
 * (de)serialize, and ideally are parsimonious with respect to memory usage.
 *
 * Naturally, these desiderata are contradictory. Hence, as a tradeoff, we
 * - intern [Uri]s and Uri *components* for deduplication,
 * - use [strumbra](strumbra::SharedString) strings to keep allocations infrequent,
 * - use [Arc](triomphe::Arc)s where heap is unavoidable
 * - use pointer-equality (thanks to interning) for fast equality checks
 *
 * ## Grammar
 *
 * | Type  |     | Cases/Def | Trait |
 * |----------- |---- | -----|-------|
 * | [`Uri`]      | ::= | [`BaseUri`]⏐[`ArchiveUri`]⏐[`PathUri`]⏐[`ModuleUri`]⏐[`SymbolUri`]⏐[`DocumentUri`]⏐[`DocumentElementUri`] | [`IsFtmlUri`] |
 * | [`BaseUri`]  | ::= | (URL with no query/fragment) | - |
 * | [`ArchiveUri`] | ::= | <code>[BaseUri]?a=[ArchiveId]</code> | [`UriWithArchive`] |
 * | [`PathUri`]  | ::= | <code>[ArchiveUri][&p=[UriPath]]</code> | [`UriWithPath`] |
 * | [`DomainUri`] | ::= | [`ModuleUri`]⏐[`SymbolUri`]   | [`IsDomainUri`] |
 * | [`ModuleUri`] | ::= | <code>[PathUri]&m=[UriName]&l=[Language]</code> | - |
 * | [`SymbolUri`] | ::= | <code>[ModuleUri]&s=[UriName]</code> | - |
 * | [`NarrativeUri`] | ::= | [`DocumentUri`]⏐[`DocumentElementUri`] | [`IsNarrativeUri`] |
 * | [`DocumentUri`] | ::= | <code>[PathUri]&d=[SimpleUriName]&l=[Language]</code> | - |
 * | [`DocumentElementUri`] | ::= | <code>[DocumentUri]&e=[UriName]</code> | - |
 *
 * ## Feature flags
 **/
#![cfg_attr(doc,doc = document_features::document_features!())]

mod archive;
mod aux;
mod base;
mod doc_element;
mod document;
mod language;
mod module;
mod paths;
mod symbol;

/// parsing and related errors
pub mod errors {
    pub use crate::aux::errors::*;
}
#[cfg(feature = "components")]
pub mod components;
use std::str::FromStr;

pub(crate) use aux::macros::{debugdisplay, tests, ts};

/// exports all Uri types and associated traits
pub mod prelude {
    pub use super::archive::{ArchiveId, ArchiveUri};
    pub use super::base::BaseUri;
    pub use super::language::Language;
    pub use super::paths::{PathUri, UriPath};
    pub use super::symbol::SymbolUri;
    pub use super::{
        DomainUri, IsDomainUri, IsFtmlUri, IsNarrativeUri, NarrativeUri, Uri, UriWithArchive,
        UriWithPath,
    };
    pub use crate::doc_element::DocumentElementUri;
    pub use crate::document::{DocumentUri, SimpleUriName};
    pub use crate::module::{ModuleUri, UriName};
}
use const_format::concatcp;
use either::Either::{Left, Right};
pub use prelude::*;

use crate::errors::UriParseError;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Core trait for all FTML URI types.
///
/// This trait provides the fundamental interface that all implement. They only
/// common component of all URI types is that they have (or are) a [`BaseUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let base = BaseUri::from_str("http://example.com").unwrap();
/// let archive_uri = ArchiveUri::from_str("http://example.com?a=my/archive").unwrap();
///
/// // Both types implement IsFtmlUri
/// assert_eq!(base.base().as_str(), "http://example.com");
/// assert_eq!(archive_uri.base().as_str(), "http://example.com");
/// ```
pub trait IsFtmlUri: Into<BaseUri> + Into<Uri> + sealed::Sealed {
    /// Returns a reference to the [`BaseUri`] component.
    fn base(&self) -> &BaseUri;
}

/// Enum representing any type of FTML URI.
///
/// This enum provides a unified type that can hold any FTML URI variant,
/// from simple base URIs to complex module URIs. It implements the core
/// [`IsFtmlUri`] trait, allowing uniform access to the base URI component.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let base_uri = BaseUri::from_str("http://example.com").unwrap();
/// let archive_uri = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
///
/// let uris: Vec<Uri> = vec![
///     Uri::BaseUri(base_uri),
///     Uri::ArchiveUri(archive_uri),
/// ];
///
/// for uri in &uris {
///     println!("Base: {}", uri.base());
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Hash, strum::EnumDiscriminants)]
#[strum_discriminants(vis(pub))]
#[strum_discriminants(name(UriKind))]
#[strum_discriminants(derive(strum::Display))]
#[cfg_attr(
    feature = "serde",
    strum_discriminants(derive(serde::Serialize, serde::Deserialize))
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub enum Uri {
    /// A base URI with no additional components.
    BaseUri(BaseUri),
    /// An archive URI identifying a specific archive.
    ArchiveUri(ArchiveUri),
    /// A path URI identifying a location within an archive.
    PathUri(PathUri),
    /// A module URI identifying a specific module.
    ModuleUri(ModuleUri),
    /// A symbol URI identifying a specific concept.
    SymbolUri(SymbolUri),
    /// A document URI identifying a document in some archive.
    DocumentUri(DocumentUri),
    /// A document element URI identifying a named part in a document (section, paragraph, etc.).
    DocumentElementUri(DocumentElementUri),
}
impl crate::sealed::Sealed for Uri {}
crate::ts!(Uri);
crate::debugdisplay!(Uri);

impl std::fmt::Display for Uri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BaseUri(b) => b.fmt(f),
            Self::ArchiveUri(a) => a.fmt(f),
            Self::PathUri(p) => p.fmt(f),
            Self::ModuleUri(m) => m.fmt(f),
            Self::SymbolUri(s) => s.fmt(f),
            Self::DocumentUri(d) => d.fmt(f),
            Self::DocumentElementUri(e) => e.fmt(f),
        }
    }
}
impl IsFtmlUri for Uri {
    fn base(&self) -> &BaseUri {
        match self {
            Self::BaseUri(b) => b,
            Self::ArchiveUri(a) => a.base(),
            Self::PathUri(p) => p.base(),
            Self::ModuleUri(m) => m.base(),
            Self::SymbolUri(s) => s.base(),
            Self::DocumentUri(d) => d.base(),
            Self::DocumentElementUri(e) => e.base(),
        }
    }
}
impl From<Uri> for BaseUri {
    #[inline]
    fn from(value: Uri) -> Self {
        match value {
            Uri::BaseUri(b) => b,
            Uri::ArchiveUri(a) => a.into(),
            Uri::PathUri(p) => p.into(),
            Uri::ModuleUri(m) => m.into(),
            Uri::SymbolUri(s) => s.into(),
            Uri::DocumentUri(d) => d.into(),
            Uri::DocumentElementUri(e) => e.into(),
        }
    }
}
impl From<BaseUri> for Uri {
    #[inline]
    fn from(value: BaseUri) -> Self {
        Self::BaseUri(value)
    }
}
impl From<ArchiveUri> for Uri {
    #[inline]
    fn from(value: ArchiveUri) -> Self {
        Self::ArchiveUri(value)
    }
}
impl From<PathUri> for Uri {
    #[inline]
    fn from(value: PathUri) -> Self {
        Self::PathUri(value)
    }
}
impl From<ModuleUri> for Uri {
    #[inline]
    fn from(value: ModuleUri) -> Self {
        Self::ModuleUri(value)
    }
}
impl From<SymbolUri> for Uri {
    #[inline]
    fn from(value: SymbolUri) -> Self {
        Self::SymbolUri(value)
    }
}
impl From<DocumentUri> for Uri {
    #[inline]
    fn from(value: DocumentUri) -> Self {
        Self::DocumentUri(value)
    }
}
impl From<DocumentElementUri> for Uri {
    #[inline]
    fn from(value: DocumentElementUri) -> Self {
        Self::DocumentElementUri(value)
    }
}
impl From<DomainUri> for Uri {
    #[inline]
    fn from(value: DomainUri) -> Self {
        match value {
            DomainUri::Module(m) => Self::ModuleUri(m),
            DomainUri::Symbol(s) => Self::SymbolUri(s),
        }
    }
}
impl From<NarrativeUri> for Uri {
    #[inline]
    fn from(value: NarrativeUri) -> Self {
        match value {
            NarrativeUri::Document(d) => Self::DocumentUri(d),
            NarrativeUri::Element(e) => Self::DocumentElementUri(e),
        }
    }
}

/// Enum ranging over all url parameters occurring in [`Uri`]s; used for error messaging etc.
#[derive(
    Copy, Clone, PartialEq, Eq, Debug, strum::Display, strum::IntoStaticStr, strum::EnumString,
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
#[allow(non_camel_case_types)]
pub enum UriComponentKind {
    uri,
    rp,
    a,
    p,
    m,
    d,
    l,
    s,
    e,
}

/// Enum ranging over all [`IsDomainUri`] types ([`ModuleUri`] and [`SymbolUri`]).
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let module_uri = ModuleUri::from_str("http://example.com?a=archive&m=module").unwrap();
/// let domain_uri: DomainUri = module_uri.into();
///
/// match domain_uri {
///     DomainUri::Module(m) => {
///         assert_eq!(m.name.to_string(), "module");
///     }
///     DomainUri::Symbol(m) => unreachable!()
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub enum DomainUri {
    /// A module URI identifying a specific module within an archive.
    Module(ModuleUri),
    /// A symbol URI identifying a specific concept.
    Symbol(SymbolUri),
}
crate::ts!(DomainUri);
crate::debugdisplay!(DomainUri);
impl crate::sealed::Sealed for DomainUri {}

impl std::fmt::Display for DomainUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Module(m) => m.fmt(f),
            Self::Symbol(s) => s.fmt(f),
        }
    }
}
impl IsFtmlUri for DomainUri {
    #[inline]
    fn base(&self) -> &BaseUri {
        match self {
            Self::Module(m) => m.base(),
            Self::Symbol(s) => s.base(),
        }
    }
}
impl IsDomainUri for DomainUri {
    #[inline]
    fn module_uri(&self) -> &ModuleUri {
        match self {
            Self::Module(m) => m,
            Self::Symbol(s) => s.module_uri(),
        }
    }
}
impl From<DomainUri> for BaseUri {
    #[inline]
    fn from(value: DomainUri) -> Self {
        match value {
            DomainUri::Module(m) => m.into(),
            DomainUri::Symbol(s) => s.into(),
        }
    }
}
impl UriWithArchive for DomainUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        match self {
            Self::Module(m) => m.archive_uri(),
            Self::Symbol(s) => s.archive_uri(),
        }
    }
}
impl From<DomainUri> for ArchiveUri {
    #[inline]
    fn from(value: DomainUri) -> Self {
        match value {
            DomainUri::Module(m) => m.into(),
            DomainUri::Symbol(s) => s.into(),
        }
    }
}
impl UriWithPath for DomainUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        match self {
            Self::Module(m) => m.path_uri(),
            Self::Symbol(s) => s.path_uri(),
        }
    }
}
impl From<DomainUri> for PathUri {
    #[inline]
    fn from(value: DomainUri) -> Self {
        match value {
            DomainUri::Module(m) => m.into(),
            DomainUri::Symbol(s) => s.into(),
        }
    }
}
impl From<DomainUri> for ModuleUri {
    #[inline]
    fn from(value: DomainUri) -> Self {
        match value {
            DomainUri::Module(m) => m,
            DomainUri::Symbol(s) => s.into(),
        }
    }
}
impl From<ModuleUri> for DomainUri {
    #[inline]
    fn from(value: ModuleUri) -> Self {
        Self::Module(value)
    }
}
impl From<SymbolUri> for DomainUri {
    #[inline]
    fn from(value: SymbolUri) -> Self {
        Self::Symbol(value)
    }
}

/// Enum ranging over all [`IsNarrativeUri`] types ([`DocumentUri`] and [`DocumentElementUri`]).
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let document_uri = DocumentUri::from_str("http://example.com?a=archive&d=document&l=en").unwrap();
/// let narrative_uri: NarrativeUri = document_uri.into();
///
/// match narrative_uri {
///     NarrativeUri::Document(d) => {
///         assert_eq!(d.document_name().as_ref(), "document");
///     }
///     NarrativeUri::Element(e) => unreachable!()
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
pub enum NarrativeUri {
    /// A document URI identifying a specific document within an archive.
    Document(DocumentUri),
    /// A document element URI identifying a named part of a document.
    Element(DocumentElementUri),
}
crate::ts!(NarrativeUri);
crate::debugdisplay!(NarrativeUri);
impl crate::sealed::Sealed for NarrativeUri {}

impl std::fmt::Display for NarrativeUri {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Document(d) => d.fmt(f),
            Self::Element(e) => e.fmt(f),
        }
    }
}
impl IsFtmlUri for NarrativeUri {
    #[inline]
    fn base(&self) -> &BaseUri {
        match self {
            Self::Document(d) => d.base(),
            Self::Element(e) => e.base(),
        }
    }
}
impl IsNarrativeUri for NarrativeUri {
    #[inline]
    fn document_uri(&self) -> &DocumentUri {
        match self {
            Self::Document(d) => d,
            Self::Element(e) => e.document_uri(),
        }
    }
}
impl From<NarrativeUri> for BaseUri {
    #[inline]
    fn from(value: NarrativeUri) -> Self {
        match value {
            NarrativeUri::Document(d) => d.into(),
            NarrativeUri::Element(e) => e.into(),
        }
    }
}
impl UriWithArchive for NarrativeUri {
    #[inline]
    fn archive_uri(&self) -> &ArchiveUri {
        match self {
            Self::Document(d) => d.archive_uri(),
            Self::Element(e) => e.archive_uri(),
        }
    }
}
impl From<NarrativeUri> for ArchiveUri {
    #[inline]
    fn from(value: NarrativeUri) -> Self {
        match value {
            NarrativeUri::Document(d) => d.into(),
            NarrativeUri::Element(e) => e.into(),
        }
    }
}
impl UriWithPath for NarrativeUri {
    #[inline]
    fn path_uri(&self) -> &PathUri {
        match self {
            Self::Document(d) => d.path_uri(),
            Self::Element(e) => e.path_uri(),
        }
    }
}
impl From<NarrativeUri> for PathUri {
    #[inline]
    fn from(value: NarrativeUri) -> Self {
        match value {
            NarrativeUri::Document(d) => d.into(),
            NarrativeUri::Element(e) => e.into(),
        }
    }
}
impl From<NarrativeUri> for DocumentUri {
    #[inline]
    fn from(value: NarrativeUri) -> Self {
        match value {
            NarrativeUri::Document(d) => d,
            NarrativeUri::Element(e) => e.into(),
        }
    }
}
impl From<DocumentUri> for NarrativeUri {
    #[inline]
    fn from(value: DocumentUri) -> Self {
        Self::Document(value)
    }
}
impl From<DocumentElementUri> for NarrativeUri {
    #[inline]
    fn from(value: DocumentElementUri) -> Self {
        Self::Element(value)
    }
}

/// Trait for URI types that have an associated archive; i.e. have an [`ArchiveUri`] component.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let path_uri = PathUri::from_str("http://example.com?a=my/archive&p=some/path").unwrap();
///
/// assert_eq!(path_uri.archive_id().to_string(), "my/archive");
/// assert_eq!(path_uri.archive_uri().to_string(), "http://example.com?a=my/archive");
/// ```
pub trait UriWithArchive: Into<ArchiveUri> + IsFtmlUri {
    /// Returns a reference to the [`ArchiveUri`] component.
    fn archive_uri(&self) -> &ArchiveUri;

    /// Returns a reference to the [`ArchiveId`] of this URI.
    #[inline]
    fn archive_id(&self) -> &ArchiveId {
        &self.archive_uri().id
    }
}

/// Trait for URI types that have an associated path within an archive.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder/file").unwrap();
/// let archive_uri = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
/// let path_uri_no_path: PathUri = archive_uri.into();
///
/// assert_eq!(path_uri.path().unwrap().to_string(), "folder/file");
/// assert!(path_uri_no_path.path().is_none());
/// ```
pub trait UriWithPath: UriWithArchive + Into<PathUri> + IsFtmlUri {
    /// Returns a reference to the [`PathUri`] component.
    fn path_uri(&self) -> &PathUri;

    /// Returns the [`UriPath`] component, if present.
    #[inline]
    fn path(&self) -> Option<&UriPath> {
        self.path_uri().path.as_ref()
    }
}

/// Trait for URI types that represent domain knowledge.
///
/// This trait is implemented by URI types that identify specific domain content,
/// i.e. modules or symbols. All domain URIs have (or are) an associated [`ModuleUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let module_uri = ModuleUri::from_str("http://example.com?a=archive&m=my/module").unwrap();
///
/// assert_eq!(module_uri.module_name().to_string(), "my/module");
/// ```
pub trait IsDomainUri: UriWithPath + Into<ModuleUri> + Into<DomainUri> {
    /// Returns a reference to the [`ModuleUri`] component.
    fn module_uri(&self) -> &ModuleUri;

    /// Returns the module name.
    #[inline]
    fn module_name(&self) -> &UriName {
        &self.module_uri().name
    }
}

/// Trait for URI types that represent narration.
///
/// This trait is implemented by URI types that identify narraitve, human-oriented content,
/// i.e. documents, paragraphs, etc. All narrative URIs have (or are) an associated
/// [`DocumentUri`] and [`Language`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let document_uri = DocumentUri::from_str("http://example.com?a=archive&d=document&l=en").unwrap();
///
/// assert_eq!(document_uri.document_name().as_ref(), "document");
/// assert_eq!(document_uri.language(), Language::English);
/// ```
pub trait IsNarrativeUri: UriWithPath + Into<DocumentUri> + Into<NarrativeUri> {
    /// Returns a reference to the [`DocumentUri`] component.
    fn document_uri(&self) -> &DocumentUri;

    /// Returns the document's name.
    #[inline]
    fn document_name(&self) -> &SimpleUriName {
        &self.document_uri().name
    }

    /// Returns the language of the (containing) document.
    #[inline]
    fn language(&self) -> Language {
        self.document_uri().language
    }
}

fn parse_domain(
    module: &str,
    path: impl FnOnce() -> Result<PathUri, UriParseError>,
    mut split: std::str::Split<char>,
) -> Result<DomainUri, UriParseError> {
    let name = move || module.parse();
    let module = move || {
        Ok::<_, UriParseError>(ModuleUri {
            path: path()?,
            name: name()?,
        })
    };
    let Some(next) = split.next() else {
        return Ok(DomainUri::Module(module()?));
    };
    next.strip_prefix(concatcp!(SymbolUri::SEPARATOR, "="))
        .map_or_else(
            || Err(UriParseError::UnknownParameter),
            |symbol| {
                if split.next().is_some() {
                    Err(UriParseError::TooManyPartsFor {
                        uri_kind: UriKind::SymbolUri,
                    })
                } else {
                    Ok(DomainUri::Symbol(SymbolUri {
                        module: module()?,
                        name: symbol.parse()?,
                    }))
                }
            },
        )
}

fn parse_narrative(
    document: &str,
    (language, next): (Language, Option<&str>),
    path: impl FnOnce() -> Result<PathUri, UriParseError>,
    mut split: std::str::Split<char>,
) -> Result<NarrativeUri, UriParseError> {
    let name = move || document.parse();
    let document = move || {
        Ok::<_, UriParseError>(DocumentUri {
            path: path()?,
            name: name()?,
            language,
        })
    };
    let Some(next) = next else {
        return Ok(NarrativeUri::Document(document()?));
    };
    next.strip_prefix(concatcp!(DocumentElementUri::SEPARATOR, "="))
        .map_or_else(
            || Err(UriParseError::UnknownParameter),
            |element| {
                if split.next().is_some() {
                    Err(UriParseError::TooManyPartsFor {
                        uri_kind: UriKind::DocumentElementUri,
                    })
                } else {
                    Ok(NarrativeUri::Element(DocumentElementUri {
                        document: document()?,
                        name: element.parse()?,
                    }))
                }
            },
        )
}

impl FromStr for Uri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, mut split) = match BaseUri::pre_parse(s)? {
            Left(base) => return Ok(Self::BaseUri(base)),
            Right(c) => c,
        };
        let Some(next) = split.next() else {
            unreachable!()
        };
        next.strip_prefix(concatcp!(ArchiveUri::SEPARATOR, "="))
            .map_or_else(
                || Err(UriParseError::UnknownParameter),
                |archive| {
                    let archive = move || -> Result<_, UriParseError> {
                        Ok(ArchiveUri {
                            base,
                            id: archive.parse()?,
                        })
                    };
                    let Some(next) = split.next() else {
                        return Ok(Self::ArchiveUri(archive()?));
                    };
                    let (path, next) =
                        if let Some(path) = next.strip_prefix(concatcp!(PathUri::SEPARATOR, "=")) {
                            (
                                Left(|| {
                                    Ok(PathUri {
                                        archive: archive()?,
                                        path: Some(path.parse()?),
                                    })
                                }),
                                split.next(),
                            )
                        } else {
                            (
                                Right(|| -> Result<_, UriParseError> {
                                    Ok(PathUri {
                                        archive: archive()?,
                                        path: None,
                                    })
                                }),
                                Some(next),
                            )
                        };
                    let path = move || match path {
                        Left(p) => p(),
                        Right(p) => Ok(p()?),
                    };
                    let Some(next) = next else {
                        return Ok(Self::PathUri(path()?));
                    };
                    let mut language = || {
                        split.next().map_or_else(
                            || Ok((Language::default(), None)),
                            |n| {
                                n.strip_prefix(concatcp!(Language::SEPARATOR, "="))
                                    .map_or_else(
                                        || Ok((Language::default(), Some(n))),
                                        |l| {
                                            l.parse()
                                                .map_err(|_| UriParseError::InvalidLanguage)
                                                .map(|l| (l, split.next()))
                                        },
                                    )
                            },
                        )
                    };
                    if let Some(module) = next.strip_prefix(concatcp!(ModuleUri::SEPARATOR, "=")) {
                        Ok(parse_domain(module, path, split)?.into())
                    } else if let Some(document) =
                        next.strip_prefix(concatcp!(DocumentUri::SEPARATOR, "="))
                    {
                        Ok(parse_narrative(document, language()?, path, split)?.into())
                    } else {
                        Err(UriParseError::UnknownParameter)
                    }
                },
            )
    }
}
impl FromStr for DomainUri {
    type Err = errors::UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ModuleUri::pre_parse(s, UriKind::ModuleUri, |module, mut split| {
            let Some(c) = split.next() else {
                return Ok(Self::Module(module));
            };
            c.strip_prefix(concatcp!(SymbolUri::SEPARATOR, "="))
                .map_or_else(
                    || {
                        Err(UriParseError::TooManyPartsFor {
                            uri_kind: UriKind::SymbolUri,
                        })
                    },
                    |name| {
                        Ok(Self::Symbol(SymbolUri {
                            module,
                            name: name.parse()?,
                        }))
                    },
                )
        })
    }
}

impl FromStr for NarrativeUri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DocumentUri::pre_parse(s, UriKind::DocumentUri, |document, mut split| {
            let Some(c) = split.next() else {
                return Ok(Self::Document(document));
            };
            c.strip_prefix(concatcp!(DocumentElementUri::SEPARATOR, "="))
                .map_or_else(
                    || {
                        Err(UriParseError::TooManyPartsFor {
                            uri_kind: UriKind::DocumentElementUri,
                        })
                    },
                    |name| {
                        Ok(Self::Element(DocumentElementUri {
                            document,
                            name: name.parse()?,
                        }))
                    },
                )
        })
    }
}

#[cfg(test)]
#[rstest::fixture]
fn trace() {
    let _ = tracing_subscriber::fmt().try_init();
}

crate::tests! {
    uri_enum {
        use std::str::FromStr;

        let Uri::BaseUri(base_uri) = Uri::from_str("http://example.com").expect("works") else { panic!("Didn't work!")};
        let Uri::ArchiveUri(archive_uri) = Uri::from_str("http://example.com?a=archive").expect("works") else { panic!("Didn't work!")};
        let Uri::PathUri(path_uri) = Uri::from_str("http://example.com?a=archive&p=path").expect("works") else { panic!("Didn't work!")};
        let Uri::ModuleUri(module_uri) = Uri::from_str("http://example.com?a=archive&m=module").expect("works") else { panic!("Didn't work!")};
        let Uri::SymbolUri(symbol_uri) = Uri::from_str("http://example.com?a=archive&m=module&s=symbol").expect("works") else { panic!("Didn't work!")};
        let Uri::DocumentUri(document_uri) = Uri::from_str("http://example.com?a=archive&d=document&l=en").expect("works") else { panic!("Didn't work!")};
        let Uri::DocumentElementUri(element_uri) = Uri::from_str("http://example.com?a=archive&d=document&l=fr&e=foo/bar/baz").expect("works") else { panic!("Didn't work!")};

        // Test URI enum conversions
        let uri_base: Uri = base_uri.clone().into();
        let uri_archive: Uri = archive_uri.into();
        let uri_path: Uri = path_uri.into();
        let uri_module: Uri = module_uri.into();
        let uri_symbol: Uri = symbol_uri.into();
        let uri_document: Uri = document_uri.into();
        let uri_element: Uri = element_uri.into();

        // Test IsFtmlUri implementation
        assert_eq!(uri_base.base().as_str(), "http://example.com");
        assert_eq!(uri_archive.base().as_str(), "http://example.com");
        assert_eq!(uri_path.base().as_str(), "http://example.com");
        assert_eq!(uri_module.base().as_str(), "http://example.com");
        assert_eq!(uri_symbol.base().as_str(), "http://example.com");

        // Test Display implementation
        assert_eq!(uri_base.to_string(), "http://example.com");
        assert_eq!(uri_archive.to_string(), "http://example.com?a=archive");
        assert_eq!(uri_path.to_string(), "http://example.com?a=archive&p=path");
        assert_eq!(uri_module.to_string(), "http://example.com?a=archive&m=module");
        assert_eq!(uri_symbol.to_string(), "http://example.com?a=archive&m=module&s=symbol");

        // Test conversion back to BaseUri
        let base_from_uri: BaseUri = uri_base.into();
        assert_eq!(base_from_uri, base_uri);
    };
    domain_uri {
        use std::str::FromStr;

        let DomainUri::Module(module_uri) = DomainUri::from_str("http://example.com?a=archive&p=path&m=module").expect("works") else {
            panic!("Didn't work!")
        };
        let DomainUri::Symbol(symbol_uri) = DomainUri::from_str("http://example.com?a=archive&p=path&m=module&s=symbol").expect("works") else {
            panic!("Didn't work!")
        };
        let domain_uri: DomainUri = module_uri.clone().into();
        let domain_uri2: DomainUri = symbol_uri.into();

        // Test ContentUri traits
        assert_eq!(domain_uri.base().as_str(), "http://example.com");
        assert_eq!(domain_uri.archive_id().to_string(), "archive");
        assert_eq!(domain_uri.path().expect("works").to_string(), "path");
        assert_eq!(domain_uri.module_name().to_string(), "module");
        assert_eq!(domain_uri2.base().as_str(), "http://example.com");
        assert_eq!(domain_uri2.archive_id().to_string(), "archive");
        assert_eq!(domain_uri2.path().expect("works").to_string(), "path");
        assert_eq!(domain_uri2.module_name().to_string(), "module");

        // Test Display
        assert_eq!(domain_uri.to_string(), "http://example.com?a=archive&p=path&m=module");
        assert_eq!(domain_uri2.to_string(), "http://example.com?a=archive&p=path&m=module&s=symbol");

        // Test conversions
        let base_from_domain: BaseUri = domain_uri.clone().into();
        let archive_from_domain: ArchiveUri = domain_uri.clone().into();
        let path_from_domain: PathUri = domain_uri.clone().into();
        let module_from_domain: ModuleUri = domain_uri.into();

        assert_eq!(base_from_domain.as_str(), "http://example.com");
        assert_eq!(archive_from_domain.id.to_string(), "archive");
        assert_eq!(path_from_domain.path().expect("works").to_string(), "path");
        assert_eq!(module_from_domain.name.to_string(), "module");

        // Test Uri conversion
        let uri_from_content: Uri = DomainUri::Module(module_uri).into();
        assert_eq!(uri_from_content.to_string(), "http://example.com?a=archive&p=path&m=module");
    };
    narrative_uri {
        use std::str::FromStr;

        let NarrativeUri::Document(document_uri) = NarrativeUri::from_str("http://example.com?a=archive&p=path&d=document&l=de").expect("works") else {
            panic!("Didn't work!")
        };
        let NarrativeUri::Element(element_uri) = NarrativeUri::from_str("http://example.com?a=archive&p=path&d=doc&l=de&e=elem").expect("works") else {
            panic!("Didn't work!")
        };
        let narr_uri: NarrativeUri = document_uri.clone().into();
        let narr_uri2: NarrativeUri = element_uri.into();

        // Test NarrativeUri traits
        assert_eq!(narr_uri.base().as_str(), "http://example.com");
        assert_eq!(narr_uri.archive_id().to_string(), "archive");
        assert_eq!(narr_uri.path().expect("works").to_string(), "path");
        assert_eq!(narr_uri.document_name().to_string(), "document");
        assert_eq!(narr_uri.language(), Language::German);
        assert_eq!(narr_uri2.base().as_str(), "http://example.com");
        assert_eq!(narr_uri2.archive_id().to_string(), "archive");
        assert_eq!(narr_uri2.path().expect("works").to_string(), "path");
        assert_eq!(narr_uri2.document_name().to_string(), "doc");
        assert_eq!(narr_uri2.language(), Language::German);

        // Test Display
        assert_eq!(narr_uri.to_string(), "http://example.com?a=archive&p=path&d=document&l=de");
        assert_eq!(narr_uri2.to_string(), "http://example.com?a=archive&p=path&d=doc&l=de&e=elem");

        // Test conversions
        let base_from_domain: BaseUri = narr_uri.clone().into();
        let archive_from_domain: ArchiveUri = narr_uri.clone().into();
        let path_from_domain: PathUri = narr_uri.clone().into();
        let document_from_domain: DocumentUri = narr_uri.into();

        assert_eq!(base_from_domain.as_str(), "http://example.com");
        assert_eq!(archive_from_domain.id.to_string(), "archive");
        assert_eq!(path_from_domain.path().expect("works").to_string(), "path");
        assert_eq!(document_from_domain.name.to_string(), "document");

        // Test Uri conversion
        let uri_from_narrative: Uri = NarrativeUri::Document(document_uri).into();
        assert_eq!(uri_from_narrative.to_string(),  "http://example.com?a=archive&p=path&d=document&l=de");
    };
    trait_implementations {
        use std::str::FromStr;

        let module_uri = ModuleUri::from_str("http://example.com?a=archive&p=path&m=math/algebra").expect("works");

        // Test all trait implementations
        assert_eq!(module_uri.base().as_str(), "http://example.com");
        assert_eq!(module_uri.archive_id().to_string(), "archive");
        assert_eq!(module_uri.path().expect("works").to_string(), "path");
        assert_eq!(module_uri.module_name().to_string(), "math/algebra");

        // Test trait method access
        assert_eq!(module_uri.base().as_str(), "http://example.com");
        assert_eq!(module_uri.archive_id().to_string(), "archive");
        assert_eq!(module_uri.path().expect("works").to_string(), "path");
        assert_eq!(module_uri.module_name().to_string(), "math/algebra");
    };
    uri_sizes {
        tracing::info!("Size of Uri: {}", std::mem::size_of::<Uri>());
        tracing::info!("Size of DomainUri: {}", std::mem::size_of::<DomainUri>());
        tracing::info!("Size of Option<Uri>: {}", std::mem::size_of::<Option<Uri>>());
        tracing::info!("Size of Option<DomainUri>: {}", std::mem::size_of::<Option<DomainUri>>());
    }
}
