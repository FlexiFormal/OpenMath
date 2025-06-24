/*! Utilities for dealing with destructured [`Uri`] components individually.
 *
 * The primary use case is routing when treating a [`Uri`] like a url. In that case, chances are we
 * primarily have access to the individual components, for example in a
 * [leptos](https://docs.rs/leptos) server function, or some Map giving access to the url parameters;
 * e.g. when using [axum](https://docs.rs/axum).
 *
 * In short: The helpers in this module allow for converting between [`Uri`]s and
 * e.g. a tuple <code>a:[Option]<[ArchiveId]>,p:[Option]<[String]>,d:[Option]<[String]>,l:[Option]<[Language]>,m:[Option]<[String]>,...</code>
 *
 * The [`compfun`](crate::compfun!) macro can be used to conveniently declare a new *function* that takes such parameters,
 * [`UriComponentTuple`], [`DocumentUriComponentTuple`] and [`SymbolUriComponentTuple`] represent
 * such tuples in a structured way, and [`UriComponents`], [`DocumentUriComponents`] and [`SymbolUriComponents`]
 * represent them "checked" and enumed by coherence (e.g. `m` and `d` components are mutually exclusive).
 * The later all have [`parse`](UriComponents::parse) methods for actually converting them to a [`Uri`].
 *
 * See [`UriComponentTuple`] for an example.
 */

use std::str::FromStr;

use arrayvec::ArrayVec;
use strum::IntoDiscriminant;

use crate::{
    ArchiveId, ArchiveUri, DocumentElementUri, DocumentUri, Language, ModuleUri, SymbolUri, Uri,
    UriComponentKind, UriKind, UriPath,
    errors::{SegmentParseError, UriParseError},
};

/** Allows for conveniently declaring a new function that takes [`UriComponentTuple`]s, [`DocumentUriComponentTuple`]s
 * or [`SymbolUriComponentTuple`]s as individual arguments. This is especially useful for declaring e.g.
 * [leptos](https://docs.rs/leptos) server functions.
 *
 * Syntax:
 * - <code>[compfun!](crate::compfun!)(fn name(compname:[Uri],...) {...});</code>, or
 * - <code>[compfun!](crate::compfun!)(fn name(compname:[DocumentUri],...) {...});</code>, or
 * - <code>[compfun!](crate::compfun!)(fn name(compname:[SymbolUri],...) {...});</code>
 *
 * (...for arbitrary values of `name` and `compname`, with arbitrary additional arguments, return types,visibility
 * modifiers and metadata).
 *
 * In the function body, a variable `compname` will then be defined and have type
 * <code>[Result]<([UriComponents] or [DocumentUriComponents] or [SymbolUriComponents]),[ComponentError]></code>,
 * depending on whether [`Uri`] or [`DocumentUri`] or [`SymbolUri`] was used.
 *
 * # Examples
 *
 * ```
 * # use ftml_uris::prelude::*;
 * # use ftml_uris::components::*;
 * # use ftml_uris::compfun;
 * # use server_fn_macro_default::server;
 * # use std::str::FromStr;
 * # fn do_something(_:Uri) {}
 * # fn do_something_else(_:Uri) {}
 * # #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
 * # pub struct MyError;
 * # impl server_fn::error::FromServerFnError for MyError {
 * #     type Encoder = server_fn::codec::JsonEncoding;
 * #     fn from_server_fn_error(value: server_fn::error::ServerFnErrorErr) -> Self {
 * #         todo!()
 * #     }
 * # }
 * # impl From<ComponentError> for MyError {
 * #     fn from(_: ComponentError) -> Self {
 * #         Self
 * #     }
 * # }
 * fn get_archive_uri(id: &ArchiveId) -> Option<ArchiveUri> {
 *   // append this server's base URI:
 *   Some(BaseUri::from_str("http://this.server").unwrap() & id.clone())
 * }
 * compfun!{
 * #[server]
 * pub async fn get_any(uri:Uri,expensive:bool) -> Result<String,MyError> {
 *     let uri: UriComponents = uri?;
 *     let actual_uri: Uri = uri.parse(get_archive_uri)?;
 *     if expensive { do_something(actual_uri)} else {dom_something_else(actual_uri)}
 * }}
 * ```
 *
 *
 **/
#[macro_export]
macro_rules! compfun {
    ($(#[$meta:meta])* $vis:vis async fn $ident:ident($components:ident:Uri$(,$arg:ident:$argtp:ty)*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $(#[$meta])*
        #[allow(clippy::too_many_arguments)]
        $vis async fn $ident(
            uri:Option<$crate::Uri>,
            rp:Option<std::string::String>,
            a:Option<$crate::ArchiveId>,
            p:Option<std::string::String>,
            d:Option<std::string::String>,
            m:Option<std::string::String>,
            l:Option<Language>,
            e:Option<std::string::String>,
            s:Option<std::string::String>
            $(,$arg:$argtp)?
        ) $(-> $ret)? {
            let _comps = $crate::components::UriComponentTuple {
                uri,rp,a,p,d,m,l,e,s
            };
            let $components = $crate::components::UriComponents::<String>::try_from(_comps);
            $($body)*
        }
    };

    ($(#[$meta:meta])* $vis:vis fn $ident:ident($components:ident:Uri$(,$arg:ident:$argtp:ty)*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $(#[$meta])*
        #[allow(clippy::too_many_arguments)]
        $vis fn $ident(
            uri:Option<$crate::Uri>,
            rp:Option<std::string::String>,
            a:Option<$crate::ArchiveId>,
            p:Option<std::string::String>,
            d:Option<std::string::String>,
            m:Option<std::string::String>,
            l:Option<Language>,
            e:Option<std::string::String>,
            s:Option<std::string::String>
            $(,$arg:$argtp)?
        ) $(-> $ret)? {
            let _comps = $crate::components::UriComponentTuple {
                uri,rp,a,p,d,m,l,e,s
            };
            let $components = $crate::components::UriComponents::<String>::try_from(_comps);
            $($body)*
        }
    };

    ($(#[$meta:meta])* $vis:vis async fn $ident:ident($components:ident:DocumentUri $(,$arg:ident:$argtp:ty)*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $(#[$meta])*
        #[allow(clippy::too_many_arguments)]
        $vis async fn $ident(
            uri:Option<$crate::DocumentUri>,
            rp:Option<std::string::String>,
            a:Option<$crate::ArchiveId>,
            p:Option<std::string::String>,
            d:Option<std::string::String>,
            l:Option<Language>
            $(,$arg:$argtp)?
        ) $(-> $ret)? {
            let _comps = $crate::components::DocumentUriComponentTuple {
                uri,rp,a,p,d,l
            };
            let $components = $crate::components::DocumentUriComponents::<String>::try_from(_comps);
            $($body)*
        }
    };

    ($(#[$meta:meta])* $vis:vis fn $ident:ident($components:ident:DocumentUri $(,$arg:ident:$argtp:ty)*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $(#[$meta])*
        #[allow(clippy::too_many_arguments)]
        $vis fn $ident(
            uri:Option<$crate::DocumentUri>,
            rp:Option<std::string::String>,
            a:Option<$crate::ArchiveId>,
            p:Option<std::string::String>,
            d:Option<std::string::String>,
            l:Option<Language>
            $(,$arg:$argtp)?
        ) $(-> $ret)? {
            let _comps = $crate::components::DocumentUriComponentTuple {
                uri,rp,a,p,d,l
            };
            let $components = $crate::components::DocumentUriComponents::<String>::try_from(_comps);
            $($body)*
        }
    };

    ($(#[$meta:meta])* $vis:vis async fn $ident:ident($components:ident:SymbolUri $(,$arg:ident:$argtp:ty)*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $(#[$meta])*
        #[allow(clippy::too_many_arguments)]
        $vis async fn $ident(
            uri:Option<$crate::SymbolUri>,
            a:Option<$crate::ArchiveId>,
            p:Option<std::string::String>,
            m:Option<std::string::String>,
            s:Option<std::string::String>
            $(,$arg:$argtp)?
        ) $(-> $ret)? {
            let _comps = $crate::components::SymbolUriComponentTuple {
                uri,a,p,m,s
            };
            let $components = $crate::components::SymbolUriComponents::<String>::try_from(_comps);
            $($body)*
        }
    };

    ($(#[$meta:meta])* $vis:vis fn $ident:ident($components:ident:SymbolUri $(,$arg:ident:$argtp:ty)*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $(#[$meta])*
        #[allow(clippy::too_many_arguments)]
        $vis fn $ident(
            uri:Option<$crate::SymbolUri>,
            a:Option<$crate::ArchiveId>,
            p:Option<std::string::String>,
            m:Option<std::string::String>,
            s:Option<std::string::String>
            $(,$arg:$argtp)?
        ) $(-> $ret)? {
            let _comps = $crate::components::SymbolUriComponentTuple {
                uri,a,p,m,s
            };
            let $components = $crate::components::SymbolUriComponents::<String>::try_from(_comps);
            $($body)*
        }
    };
}

#[derive(Debug, Clone, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentError {
    #[error("no valid combination of components for a uri")]
    NoValidCombination,
    #[error("missing component {1} for {0}")]
    MissingComponents(UriKind, UriComponentKind),
    #[error("invalid components for {0}: {1:?}")]
    InvalidComponents(UriKind, ArrayVec<UriComponentKind, 8>),
    #[error("{0}")]
    Parse(#[from] UriParseError),
    #[error("No archive {0} known")]
    UnknownArchive(ArchiveId),
}

impl From<SegmentParseError> for ComponentError {
    #[inline]
    fn from(value: SegmentParseError) -> Self {
        Self::Parse(value.into())
    }
}
impl From<strum::ParseError> for ComponentError {
    #[inline]
    fn from(_: strum::ParseError) -> Self {
        Self::Parse(UriParseError::InvalidLanguage)
    }
}

pub type UriComponentFun<S, R> = fn(
    Option<Uri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<S>,
    Option<Language>,
    Option<S>,
    Option<S>,
) -> R;

pub type UriComponentFun1<S, T, R> = fn(
    Option<Uri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<S>,
    Option<Language>,
    Option<S>,
    Option<S>,
    T,
) -> R;

pub type UriComponentFun2<S, T1, T2, R> = fn(
    Option<Uri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<S>,
    Option<Language>,
    Option<S>,
    Option<S>,
    T1,
    T2,
) -> R;

pub type UriComponentFun3<S, T1, T2, T3, R> = fn(
    Option<Uri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<S>,
    Option<Language>,
    Option<S>,
    Option<S>,
    T1,
    T2,
    T3,
) -> R;

#[derive(Clone, Hash, PartialEq, Eq)]
#[impl_tools::autoimpl(Default)]
pub struct UriComponentTuple<S: AsRef<str> = String> {
    uri: Option<Uri>,
    rp: Option<S>,
    a: Option<ArchiveId>,
    p: Option<S>,
    m: Option<S>,
    d: Option<S>,
    l: Option<Language>,
    s: Option<S>,
    e: Option<S>,
}
impl<S: AsRef<str>> UriComponentTuple<S> {
    #[inline]
    pub fn apply<R>(self, f: UriComponentFun<S, R>) -> R {
        f(
            self.uri, self.rp, self.a, self.p, self.m, self.d, self.l, self.s, self.e,
        )
    }
    #[inline]
    pub fn apply1<T, R>(self, f: UriComponentFun1<S, T, R>, a: T) -> R {
        f(
            self.uri, self.rp, self.a, self.p, self.m, self.d, self.l, self.s, self.e, a,
        )
    }
    #[inline]
    pub fn apply2<T1, T2, R>(self, f: UriComponentFun2<S, T1, T2, R>, a: T1, b: T2) -> R {
        f(
            self.uri, self.rp, self.a, self.p, self.m, self.d, self.l, self.s, self.e, a, b,
        )
    }
    #[inline]
    pub fn apply3<T1, T2, T3, R>(
        self,
        f: UriComponentFun3<S, T1, T2, T3, R>,
        a: T1,
        b: T2,
        c: T3,
    ) -> R {
        f(
            self.uri, self.rp, self.a, self.p, self.m, self.d, self.l, self.s, self.e, a, b, c,
        )
    }
}

#[derive(Clone)]
pub enum UriComponents<S: AsRef<str> = String> {
    Full(Uri),
    RelPath {
        a: ArchiveId,
        rp: S,
    },
    ArchiveComponents {
        a: ArchiveId,
    },
    PathComponents {
        a: ArchiveId,
        p: S,
    },
    DocumentComponents {
        a: ArchiveId,
        p: Option<S>,
        l: Language,
        d: S,
    },
    ElementComponents {
        a: ArchiveId,
        p: Option<S>,
        l: Language,
        d: S,
        e: S,
    },
    ModuleComponents {
        a: ArchiveId,
        p: Option<S>,
        m: S,
    },
    SymbolComponents {
        a: ArchiveId,
        p: Option<S>,
        m: S,
        s: S,
    },
}
impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<UriComponents<S2>> for UriComponentTuple<S1> {
    fn from(value: UriComponents<S2>) -> Self {
        match value {
            UriComponents::Full(uri) => Self {
                uri: Some(uri),
                ..Self::default()
            },
            UriComponents::RelPath { a, rp } => Self {
                a: Some(a),
                rp: Some(rp.into()),
                ..Self::default()
            },
            UriComponents::ArchiveComponents { a } => Self {
                a: Some(a),
                ..Self::default()
            },
            UriComponents::PathComponents { a, p } => Self {
                a: Some(a),
                p: Some(p.into()),
                ..Self::default()
            },
            UriComponents::DocumentComponents { a, p, l, d } => Self {
                a: Some(a),
                p: p.map(Into::into),
                l: Some(l),
                d: Some(d.into()),
                ..Self::default()
            },
            UriComponents::ElementComponents { a, p, l, d, e } => Self {
                a: Some(a),
                p: p.map(Into::into),
                l: Some(l),
                d: Some(d.into()),
                e: Some(e.into()),
                ..Self::default()
            },
            UriComponents::ModuleComponents { a, p, m } => Self {
                a: Some(a),
                p: p.map(Into::into),
                m: Some(m.into()),
                ..Self::default()
            },
            UriComponents::SymbolComponents { a, p, m, s } => Self {
                a: Some(a),
                p: p.map(Into::into),
                m: Some(m.into()),
                s: Some(s.into()),
                ..Self::default()
            },
        }
    }
}

macro_rules! forbidden {
    ($value:ident => $kind:expr;$($f:ident),*) => {{
        let mut ret = ArrayVec::<UriComponentKind, 8>::new();
        $(
            if $value.$f.is_some() { ret.push(UriComponentKind::$f); }
        )?
    if !ret.is_empty() { return Err(ComponentError::InvalidComponents($kind,ret)) }
    }}
}
macro_rules! missing {
    ($name:ident:$r:ident) => {
        return Err(ComponentError::MissingComponents(
            UriKind::$name,
            UriComponentKind::$r,
        ))
    };
    (>$e:expr;$r:ident) => {
        return Err(ComponentError::MissingComponents($e, UriComponentKind::$r))
    };
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> TryFrom<UriComponentTuple<S2>>
    for UriComponents<S1>
{
    type Error = ComponentError;
    #[allow(clippy::cognitive_complexity)]
    fn try_from(value: UriComponentTuple<S2>) -> Result<Self, Self::Error> {
        if let Some(uri) = value.uri {
            forbidden!(value => uri.discriminant(); rp, a, p, d, l, e, m, s);
            return Ok(Self::Full(uri));
        }
        if let Some(rp) = value.rp {
            let Some(a) = value.a else {
                missing!(DocumentUri:a);
            };
            forbidden!(value => UriKind::DocumentUri;p,d,l,e,m,s);
            return Ok(Self::RelPath { a, rp: rp.into() });
        }
        if let Some(d) = value.d {
            let Some(l) = value.l else {
                let kind = if value.e.is_some() {
                    UriKind::DocumentElementUri
                } else {
                    UriKind::DocumentUri
                };
                missing!(>kind; l);
            };
            let Some(a) = value.a else {
                let kind = if value.e.is_some() {
                    UriKind::DocumentElementUri
                } else {
                    UriKind::DocumentUri
                };
                missing!(>kind; a);
            };
            if let Some(e) = value.e {
                forbidden!(value => UriKind::DocumentElementUri;m,s);
                return Ok(Self::ElementComponents {
                    a,
                    p: value.p.map(Into::into),
                    l,
                    d: d.into(),
                    e: e.into(),
                });
            }
            forbidden!(value => UriKind::DocumentUri;m,s);
            return Ok(Self::DocumentComponents {
                a,
                p: value.p.map(Into::into),
                l,
                d: d.into(),
            });
        } else if let Some(m) = value.m {
            let Some(a) = value.a else {
                let kind = if value.s.is_some() {
                    UriKind::SymbolUri
                } else {
                    UriKind::ModuleUri
                };
                missing!(>kind; a);
            };
            if let Some(s) = value.s {
                forbidden!(value => UriKind::SymbolUri;d,l,e);
                return Ok(Self::SymbolComponents {
                    a,
                    p: value.p.map(Into::into),
                    m: m.into(),
                    s: s.into(),
                });
            }
            forbidden!(value => UriKind::ModuleUri;d,l,e);
            return Ok(Self::ModuleComponents {
                a,
                p: value.p.map(Into::into),
                m: m.into(),
            });
        } else if let Some(a) = value.a {
            if let Some(p) = value.p {
                forbidden!(value => UriKind::PathUri;e,l,s);
                return Ok(Self::PathComponents { a, p: p.into() });
            }
            forbidden!(value => UriKind::ArchiveUri;e,l,s);
            return Ok(Self::ArchiveComponents { a });
        }
        Err(ComponentError::NoValidCombination)
    }
}
impl UriComponents {
    /// #### Errors
    pub fn parse(
        self,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<Uri, ComponentError> {
        match self {
            Self::Full(uri) => Ok(uri),
            Self::ArchiveComponents { a } => {
                get(&a).map_or(Err(ComponentError::UnknownArchive(a)), |a| Ok(Uri::from(a)))
            }
            Self::PathComponents { a, p } => Ok((get(&a)
                .ok_or(ComponentError::UnknownArchive(a))?
                / p.parse::<UriPath>()?)
            .into()),
            Self::RelPath { a, rp } => Self::from_archive_relpath(a, &rp, get).map(Uri::from),
            Self::DocumentComponents { a, p, l, d } => {
                Self::get_doc_uri(a, p.as_deref(), l, &d, get).map(Uri::from)
            }
            Self::ElementComponents { a, p, l, d, e } => {
                Self::get_elem_uri(a, p.as_deref(), l, &d, &e, get).map(Uri::from)
            }
            Self::ModuleComponents { a, p, m } => {
                Self::get_mod_uri(a, p.as_deref(), &m, get).map(Uri::from)
            }
            Self::SymbolComponents { a, p, m, s } => {
                Self::get_sym_uri(a, p.as_deref(), &m, &s, get).map(Uri::from)
            }
        }
    }

    #[inline]
    fn from_archive_relpath(
        a: ArchiveId,
        rp: &str,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<DocumentUri, ComponentError> {
        let Some(a) = get(&a) else {
            return Err(ComponentError::UnknownArchive(a));
        };
        DocumentUri::from_archive_relpath(a, rp).map_err(Into::into)
    }

    #[allow(clippy::many_single_char_names)]
    #[inline]
    fn get_sym_uri(
        a: ArchiveId,
        p: Option<&str>,
        m: &str,
        s: &str,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<SymbolUri, ComponentError> {
        Ok(Self::get_mod_uri(a, p, m, get)? | s.parse()?)
    }

    #[allow(clippy::many_single_char_names)]
    fn get_mod_uri(
        a: ArchiveId,
        p: Option<&str>,
        m: &str,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<ModuleUri, ComponentError> {
        let Some(a) = get(&a) else {
            return Err(ComponentError::UnknownArchive(a));
        };
        let p = if let Some(p) = p {
            a / UriPath::from_str(p)?
        } else {
            a.into()
        };
        Ok(p | m.parse()?)
    }

    #[allow(clippy::many_single_char_names)]
    #[inline]
    fn get_elem_uri(
        a: ArchiveId,
        p: Option<&str>,
        l: Language,
        d: &str,
        e: &str,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<DocumentElementUri, ComponentError> {
        Ok(Self::get_doc_uri(a, p, l, d, get)? & e.parse()?)
    }

    fn get_doc_uri(
        a: ArchiveId,
        p: Option<&str>,
        l: Language,
        d: &str,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<DocumentUri, ComponentError> {
        let Some(a) = get(&a) else {
            return Err(ComponentError::UnknownArchive(a));
        };
        let p = if let Some(p) = p {
            a / UriPath::from_str(p)?
        } else {
            a.into()
        };
        Ok(p & (d.parse()?, l))
    }
}

pub type DocumentUriComponentFun<S, R> = fn(
    Option<DocumentUri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<Language>,
) -> R;

pub type DocumentUriComponentFun1<S, T, R> = fn(
    Option<DocumentUri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<Language>,
    T,
) -> R;

pub type DocumentUriComponentFun2<S, T1, T2, R> = fn(
    Option<DocumentUri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<Language>,
    T1,
    T2,
) -> R;

pub type DocumentUriComponentFun3<S, T1, T2, T3, R> = fn(
    Option<DocumentUri>,
    Option<S>,
    Option<ArchiveId>,
    Option<S>,
    Option<S>,
    Option<Language>,
    T1,
    T2,
    T3,
) -> R;

#[derive(Clone, Hash, PartialEq, Eq)]
#[impl_tools::autoimpl(Default)]
pub struct DocumentUriComponentTuple<S: AsRef<str> = String> {
    uri: Option<DocumentUri>,
    rp: Option<S>,
    a: Option<ArchiveId>,
    p: Option<S>,
    d: Option<S>,
    l: Option<Language>,
}
impl<S: AsRef<str>> DocumentUriComponentTuple<S> {
    #[inline]
    pub fn apply<R>(self, f: DocumentUriComponentFun<S, R>) -> R {
        f(self.uri, self.rp, self.a, self.p, self.d, self.l)
    }
    #[inline]
    pub fn apply1<T, R>(self, f: DocumentUriComponentFun1<S, T, R>, a: T) -> R {
        f(self.uri, self.rp, self.a, self.p, self.d, self.l, a)
    }
    #[inline]
    pub fn apply2<T1, T2, R>(self, f: DocumentUriComponentFun2<S, T1, T2, R>, a: T1, b: T2) -> R {
        f(self.uri, self.rp, self.a, self.p, self.d, self.l, a, b)
    }
    #[inline]
    pub fn apply3<T1, T2, T3, R>(
        self,
        f: DocumentUriComponentFun3<S, T1, T2, T3, R>,
        a: T1,
        b: T2,
        c: T3,
    ) -> R {
        f(self.uri, self.rp, self.a, self.p, self.d, self.l, a, b, c)
    }
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<DocumentUriComponentTuple<S2>>
    for UriComponentTuple<S1>
{
    fn from(
        DocumentUriComponentTuple {
            uri,
            rp,
            a,
            p,
            d,
            l,
        }: DocumentUriComponentTuple<S2>,
    ) -> Self {
        Self {
            uri: uri.map(Uri::DocumentUri),
            rp: rp.map(Into::into),
            a,
            p: p.map(Into::into),
            d: d.map(Into::into),
            l,
            ..Self::default()
        }
    }
}

#[derive(Clone)]
pub enum DocumentUriComponents<S: AsRef<str> = String> {
    Full(DocumentUri),
    RelPath {
        a: ArchiveId,
        rp: S,
    },
    Components {
        a: ArchiveId,
        p: Option<S>,
        l: Language,
        d: S,
    },
}
impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<DocumentUriComponents<S2>>
    for UriComponents<S1>
{
    fn from(value: DocumentUriComponents<S2>) -> Self {
        match value {
            DocumentUriComponents::Full(uri) => Self::Full(uri.into()),
            DocumentUriComponents::RelPath { a, rp } => Self::RelPath { a, rp: rp.into() },
            DocumentUriComponents::Components { a, p, l, d } => Self::DocumentComponents {
                a,
                p: p.map(Into::into),
                l,
                d: d.into(),
            },
        }
    }
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<DocumentUriComponents<S2>>
    for DocumentUriComponentTuple<S1>
{
    fn from(value: DocumentUriComponents<S2>) -> Self {
        match value {
            DocumentUriComponents::Full(uri) => Self {
                uri: Some(uri),
                ..Self::default()
            },
            DocumentUriComponents::RelPath { a, rp } => Self {
                a: Some(a),
                rp: Some(rp.into()),
                ..Self::default()
            },
            DocumentUriComponents::Components { a, p, l, d } => Self {
                a: Some(a),
                p: p.map(Into::into),
                l: Some(l),
                d: Some(d.into()),
                ..Self::default()
            },
        }
    }
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> TryFrom<DocumentUriComponentTuple<S2>>
    for DocumentUriComponents<S1>
{
    type Error = ComponentError;
    fn try_from(value: DocumentUriComponentTuple<S2>) -> Result<Self, Self::Error> {
        if let Some(uri) = value.uri {
            forbidden!(value => UriKind::DocumentUri; rp, a, p, d, l);
            return Ok(Self::Full(uri));
        }
        let Some(a) = value.a else {
            missing!(DocumentUri: a);
        };
        if let Some(rp) = value.rp {
            forbidden!(value => UriKind::DocumentUri;p,d,l);
            return Ok(Self::RelPath { a, rp: rp.into() });
        }
        let Some(d) = value.d else {
            missing!(DocumentUri: d);
        };
        let Some(l) = value.l else {
            missing!(DocumentUri: l);
        };
        Ok(Self::Components {
            a,
            p: value.p.map(Into::into),
            l,
            d: d.into(),
        })
    }
}
impl<S: AsRef<str>> DocumentUriComponents<S> {
    /// #### Errors
    pub fn parse(
        self,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<DocumentUri, ComponentError> {
        match self {
            Self::Full(uri) => Ok(uri),
            Self::RelPath { a, rp } => UriComponents::from_archive_relpath(a, rp.as_ref(), get),
            Self::Components { a, p, l, d } => {
                UriComponents::get_doc_uri(a, p.as_ref().map(AsRef::as_ref), l, d.as_ref(), get)
            }
        }
    }
}

pub type SymbolUriComponentFun<S, R> =
    fn(Option<SymbolUri>, Option<ArchiveId>, Option<S>, Option<S>, Option<S>) -> R;

pub type SymbolUriComponentFun1<S, T, R> =
    fn(Option<SymbolUri>, Option<ArchiveId>, Option<S>, Option<S>, Option<S>, T) -> R;

pub type SymbolUriComponentFun2<S, T1, T2, R> =
    fn(Option<SymbolUri>, Option<ArchiveId>, Option<S>, Option<S>, Option<S>, T1, T2) -> R;

pub type SymbolUriComponentFun3<S, T1, T2, T3, R> =
    fn(Option<SymbolUri>, Option<ArchiveId>, Option<S>, Option<S>, Option<S>, T1, T2, T3) -> R;

#[derive(Clone, Hash, PartialEq, Eq)]
#[impl_tools::autoimpl(Default)]
pub struct SymbolUriComponentTuple<S: AsRef<str> = String> {
    uri: Option<SymbolUri>,
    a: Option<ArchiveId>,
    p: Option<S>,
    m: Option<S>,
    s: Option<S>,
}
impl<S: AsRef<str>> SymbolUriComponentTuple<S> {
    #[inline]
    pub fn apply<R>(self, f: SymbolUriComponentFun<S, R>) -> R {
        f(self.uri, self.a, self.p, self.m, self.s)
    }
    #[inline]
    pub fn apply1<T, R>(self, f: SymbolUriComponentFun1<S, T, R>, a: T) -> R {
        f(self.uri, self.a, self.p, self.m, self.s, a)
    }
    #[inline]
    pub fn apply2<T1, T2, R>(self, f: SymbolUriComponentFun2<S, T1, T2, R>, a: T1, b: T2) -> R {
        f(self.uri, self.a, self.p, self.m, self.s, a, b)
    }
    #[inline]
    pub fn apply3<T1, T2, T3, R>(
        self,
        f: SymbolUriComponentFun3<S, T1, T2, T3, R>,
        a: T1,
        b: T2,
        c: T3,
    ) -> R {
        f(self.uri, self.a, self.p, self.m, self.s, a, b, c)
    }
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<SymbolUriComponentTuple<S2>>
    for UriComponentTuple<S1>
{
    fn from(SymbolUriComponentTuple { uri, a, p, m, s }: SymbolUriComponentTuple<S2>) -> Self {
        Self {
            uri: uri.map(Uri::SymbolUri),
            a,
            p: p.map(Into::into),
            m: m.map(Into::into),
            s: s.map(Into::into),
            ..Self::default()
        }
    }
}

#[derive(Clone)]
pub enum SymbolUriComponents<S: AsRef<str> = String> {
    Full(SymbolUri),
    Components {
        a: ArchiveId,
        p: Option<S>,
        m: S,
        s: S,
    },
}
impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<SymbolUriComponents<S2>>
    for UriComponents<S1>
{
    fn from(value: SymbolUriComponents<S2>) -> Self {
        match value {
            SymbolUriComponents::Full(uri) => Self::Full(uri.into()),
            SymbolUriComponents::Components { a, p, m, s } => Self::SymbolComponents {
                a,
                p: p.map(Into::into),
                m: m.into(),
                s: s.into(),
            },
        }
    }
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> From<SymbolUriComponents<S2>>
    for SymbolUriComponentTuple<S1>
{
    fn from(value: SymbolUriComponents<S2>) -> Self {
        match value {
            SymbolUriComponents::Full(uri) => Self {
                uri: Some(uri),
                ..Self::default()
            },
            SymbolUriComponents::Components { a, p, m, s } => Self {
                a: Some(a),
                p: p.map(Into::into),
                m: Some(m.into()),
                s: Some(s.into()),
                ..Self::default()
            },
        }
    }
}

impl<S1: AsRef<str>, S2: AsRef<str> + Into<S1>> TryFrom<SymbolUriComponentTuple<S2>>
    for SymbolUriComponents<S1>
{
    type Error = ComponentError;
    fn try_from(value: SymbolUriComponentTuple<S2>) -> Result<Self, Self::Error> {
        if let Some(uri) = value.uri {
            forbidden!(value => UriKind::SymbolUri; a,m,s);
            return Ok(Self::Full(uri));
        }
        let Some(a) = value.a else {
            missing!(SymbolUri: a);
        };
        let Some(m) = value.m else {
            missing!(SymbolUri: m);
        };
        let Some(s) = value.s else {
            missing!(SymbolUri: s);
        };
        Ok(Self::Components {
            a,
            p: value.p.map(Into::into),
            m: m.into(),
            s: s.into(),
        })
    }
}
impl<S: AsRef<str>> SymbolUriComponents<S> {
    /// #### Errors
    pub fn parse(
        self,
        get: impl FnOnce(&ArchiveId) -> Option<ArchiveUri>,
    ) -> Result<SymbolUri, ComponentError> {
        match self {
            Self::Full(uri) => Ok(uri),
            Self::Components { a, p, m, s } => UriComponents::get_sym_uri(
                a,
                p.as_ref().map(AsRef::as_ref),
                m.as_ref(),
                s.as_ref(),
                get,
            ),
        }
    }
}

// --------------------------------------------------------------------------------------------------------------
macro_rules! forbidden {
    ($value:ident => $kind:expr;$($f:ident),*) => {{
        let mut ret = ArrayVec::<UriComponentKind, 8>::new();
        $(
            if $value.get(stringify!($f)).is_some() { ret.push(UriComponentKind::$f); }
        )?
    if !ret.is_empty() { return Err(ComponentError::InvalidComponents($kind,ret).into()) }
    }}
}
macro_rules! need {
    ($value:ident[$t:ident] => $kind:ident;$f:ident) => {
        $value
            .get(stringify!($f))
            .map(|s| $t::from_str(s.as_ref()))
            .ok_or(ComponentError::MissingComponents(
                UriKind::$kind,
                UriComponentKind::$f,
            ))??
    };
    ($value:ident => $kind:ident;$f:ident) => {
        $value
            .get(stringify!($f))
            .ok_or(ComponentError::MissingComponents(
                UriKind::$kind,
                UriComponentKind::$f,
            ))?
    };
}

pub trait UriComponentsTrait {
    type S<'s>: AsRef<str> + 's
    where
        Self: 's;
    fn get<'slf>(&'slf self, key: &str) -> Option<Self::S<'slf>>;

    /// #### Errors
    fn as_document(&self) -> Result<DocumentUriComponents<Self::S<'_>>, ComponentError> {
        if let Some(uri) = self.get("uri") {
            return Ok(DocumentUriComponents::Full(DocumentUri::from_str(
                uri.as_ref(),
            )?));
        }
        let a = need!(self[ArchiveId] => DocumentUri;a);
        if let Some(rp) = self.get("rp") {
            forbidden!(self => UriKind::DocumentUri;p,d,l,e,m,s);
            Ok(DocumentUriComponents::RelPath { a, rp })
        } else {
            forbidden!(self => UriKind::DocumentUri;e,m,s);
            let p = self.get("p");
            let l = need!(self[Language] => DocumentUri;l);
            let d = need!(self => DocumentUri;d);
            Ok(DocumentUriComponents::Components { a, p, l, d })
        }
    }

    /// #### Errors
    fn as_comps(&self) -> Result<UriComponents<Self::S<'_>>, ComponentError> {
        if let Some(uri) = self.get("uri") {
            return Ok(UriComponents::Full(Uri::from_str(uri.as_ref())?));
        }
        if let Some(rp) = self.get("rp") {
            let a = need!(self[ArchiveId] => DocumentUri;a);
            forbidden!(self => UriKind::DocumentUri;p,d,l,e,m,s);
            return Ok(UriComponents::RelPath { a, rp });
        }
        let p = self.get("p");
        if let Some(e) = self.get("e") {
            let a = need!(self[ArchiveId] => DocumentElementUri;a);
            let l = need!(self[Language] => DocumentElementUri;l);
            let d = need!(self => DocumentElementUri;d);
            forbidden!(self => UriKind::DocumentElementUri;m,s);
            return Ok(UriComponents::ElementComponents { a, p, l, d, e });
        }
        if let Some(d) = self.get("d") {
            forbidden!(self => UriKind::DocumentUri;m,s);
            let a = need!(self[ArchiveId] => DocumentElementUri;a);
            let l = need!(self[Language] => DocumentElementUri;l);
            return Ok(UriComponents::DocumentComponents { a, p, l, d });
        }
        if let Some(s) = self.get("s") {
            forbidden!(self => UriKind::SymbolUri;d,l,e);
            let a = need!(self[ArchiveId] => SymbolUri;a);
            let m = need!(self => SymbolUri;m);
            return Ok(UriComponents::SymbolComponents { a, p, m, s });
        }
        if let Some(m) = self.get("m") {
            forbidden!(self => UriKind::SymbolUri;d,l,e);
            let a = need!(self[ArchiveId] => SymbolUri;a);
            return Ok(UriComponents::ModuleComponents { a, p, m });
        }
        Err(ComponentError::NoValidCombination)
    }
}

impl<H: std::hash::BuildHasher, K: std::borrow::Borrow<str> + Eq + std::hash::Hash, V: AsRef<str>>
    UriComponentsTrait for std::collections::HashMap<K, V, H>
{
    type S<'s>
        = &'s V
    where
        Self: 's;
    #[inline]
    fn get<'slf>(&'slf self, key: &str) -> Option<Self::S<'slf>> {
        Self::get(self, key)
    }
}

#[cfg(feature = "leptos")]
impl UriComponentsTrait for leptos_router::params::ParamsMap {
    type S<'s> = &'s str;
    #[inline]
    fn get(&self, key: &str) -> Option<&str> {
        self.get_str(key)
    }
}

crate::tests! {
    uri_components {
        use crate::BaseUri;
        crate::compfun!(
            fn foo(comps:Uri,get:impl FnOnce(&ArchiveId) -> Option<ArchiveUri>) -> Result<Uri,ComponentError> {
                let uri = comps?.parse(get)?;
                Ok(uri)
            }
        );
        let uricomps : UriComponentTuple<String> =
            UriComponents::<String>::Full(
                Uri::from_str("http://example.org?a=foo/bar&p=baz&d=doc&l=en").expect("works")
            ).into();
        uricomps.apply1(foo,|_:&ArchiveId| None).expect("works");

        let uricomps : UriComponentTuple<&str> = UriComponents::<&str>::RelPath {
            a:"foo/bar".parse().expect("works"),
            rp:"foo/bar/baz/doc.en.html"
        }.into();
        let uricomps : UriComponentTuple<String> = UriComponents::<String>::RelPath {
            a:"foo/bar".parse().expect("works"),
            rp:"foo/bar/baz/doc.en.html".to_string()
        }.into();
        uricomps.apply1(foo,|i:&ArchiveId| Some(BaseUri::from_str("http://example.org").expect("works") & i.clone())).expect("works");
    };
    doc_components {
        use crate::BaseUri;
        crate::compfun!(
            fn foo(comps:DocumentUri,get:impl FnOnce(&ArchiveId) -> Option<ArchiveUri>) -> Result<DocumentUri,ComponentError> {
                let uri = comps?.parse(get)?;
                Ok(uri)
            }
        );
        let uricomps : DocumentUriComponentTuple<String> =
            DocumentUriComponents::<String>::Full(
                DocumentUri::from_str("http://example.org?a=foo/bar&p=baz&d=doc&l=en").expect("works")
            ).into();
        uricomps.apply1(foo,|_:&ArchiveId| None).expect("works");

        let uricomps : DocumentUriComponentTuple<&str> = DocumentUriComponents::<&str>::RelPath {
            a:"foo/bar".parse().expect("works"),
            rp:"foo/bar/baz/doc.en.html"
        }.into();
        let uricomps : DocumentUriComponentTuple<String> = DocumentUriComponents::<String>::RelPath {
            a:"foo/bar".parse().expect("works"),
            rp:"foo/bar/baz/doc.en.html".to_string()
        }.into();
        uricomps.apply1(foo,|i:&ArchiveId| Some(BaseUri::from_str("http://example.org").expect("works") & i.clone())).expect("works");
    };
    sym_components {
        use crate::BaseUri;
        crate::compfun!(
            fn foo(comps:SymbolUri,get:impl FnOnce(&ArchiveId) -> Option<ArchiveUri>) -> Result<SymbolUri,ComponentError> {
                let uri = comps?.parse(get)?;
                Ok(uri)
            }
        );
        let uricomps : SymbolUriComponentTuple<String> =
            SymbolUriComponents::<String>::Full(
                SymbolUri::from_str("http://example.org?a=foo/bar&p=baz&m=mod&s=symbol").expect("works")
            ).into();
        uricomps.apply1(foo,|_:&ArchiveId| None).expect("works");

        let uricomps : SymbolUriComponentTuple<&str> = SymbolUriComponents::<&str>::Components {
            a:"foo/bar".parse().expect("works"),
            p:Some("baz"),m:"mod",s:"symbol"
        }.into();
        let uricomps : SymbolUriComponentTuple<String> = SymbolUriComponents::<String>::Components {
            a:"foo/bar".parse().expect("works"),
            p:Some("baz".to_string()),m:"mod".to_string(),s:"symbol".to_string()
        }.into();
        uricomps.apply1(foo,|i:&ArchiveId| Some(BaseUri::from_str("http://example.org").expect("works") & i.clone())).expect("works");
    }
}
