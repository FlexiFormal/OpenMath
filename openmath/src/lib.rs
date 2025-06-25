#![allow(unexpected_cfgs)]
#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]
/*! ## Features */
#![cfg_attr(doc,doc = document_features::document_features!())]
pub mod ser;
use std::borrow::Cow;

pub use ser::OMSerializable;
pub mod de;
pub use de::OMDeserializable;
pub mod base64;
mod int;
/// reexported for convenience
pub use either;
pub use int::Int;

/// The base URI of official OᴘᴇɴMᴀᴛʜ dictionaries (`http://www.openmath.org/cd`)
pub static OPENMATH_BASE_URI: std::sync::LazyLock<url::Url> = std::sync::LazyLock::new(||
    // SAFETY: Known to be a valid Url
    unsafe{
        url::Url::parse("http://www.openmath.org/cd").unwrap_unchecked()
    });

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum BasicOpenMath<'de, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    /// OpenMath integer (arbitrary precision)
    OMI(Int<'de>) = 0,

    /// OpenMath floating point number (IEEE 754 double precision)
    OMF(ordered_float::OrderedFloat<f64>) = 1,

    /// OpenMath string literal
    OMSTR(Str) = 2,

    /// OpenMath byte array (binary data)
    OMB(Arr) = 3,

    /// OpenMath variable (identifier)
    OMV(Str) = 4,

    /// OpenMath symbol from a Content Dictionary
    ///
    /// - `cd_name`: Content Dictionary name
    /// - `name`: Symbol name within the CD
    OMS { cd: Str, name: Str } = 5,
}

impl<'de, Arr, Str> BasicOpenMath<'de, Arr, Str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    /// The [`OpenMathKind`] associated with this object
    ///
    /// ### Examples
    ///```
    /// use openmath::*;
    /// let obj: BasicOpenMath<'static> = BasicOpenMath::OMI(42.into());
    /// assert_eq!(obj.kind(),OpenMathKind::OMI);
    ///```
    pub fn kind(&self) -> OpenMathKind {
        // SAFETY: Both types have #[repr(u8)] and all of Self's discriminant
        // values are OpenMathKind values.
        unsafe {
            let b = *<*const _>::from(self).cast::<u8>();
            std::mem::transmute(b)
        }
    }
}

/// Enum representing all possible OpenMath objects.
///
/// This enum encompasses the complete OpenMath object model, providing variants
/// for each type of mathematical object that can be represented in OpenMath.
///
/// # Type Parameters
/// - `'de`: Lifetime of the deserialized data
/// - `I`: The type that implements [`OMDeserializable`] (your target type)
/// - `Arr`: Type for byte arrays (default: <code>[Cow]<'b, [u8]></code>)
/// - `Str`: Type for strings (default: `&'de str`)
///
/// # Variants
///
/// ## Basic Objects
/// - [`OMI`](Self::OMI): Arbitrary precision integers
/// - [`OMF`](Self::OMF): IEEE 754 double precision floating point numbers
/// - [`OMSTR`](Self::OMSTR): String literals
/// - [`OMB`](Self::OMB): Binary data (byte arrays)
/// - [`OMV`](Self::OMV): Variables (identifiers)
/// - [`OMS`](Self::OMS): Symbols from Content Dictionaries
///
/// ## Compound Objects
/// - [`OMA`](Self::OMA): Applications (function calls)
/// - [`OMBIND`](Self::OMBIND): Binding constructs (quantifiers, lambda expressions)
///
/// see [OMDeserializable] for a complex example
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum OpenMath<'de, Rec, Arg, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    /// OpenMath integer (arbitrary precision)
    OMI(Int<'de>) = 0,

    /// OpenMath floating point number (IEEE 754 double precision)
    OMF(ordered_float::OrderedFloat<f64>) = 1,

    /// OpenMath string literal
    OMSTR(Str) = 2,

    /// OpenMath byte array (binary data)
    OMB(Arr) = 3,

    /// OpenMath variable (identifier)
    OMV(Str) = 4,

    /// OpenMath symbol from a Content Dictionary
    ///
    /// - `cd_name`: Content Dictionary name
    /// - `name`: Symbol name within the CD
    OMS { cd: Str, name: Str } = 5,

    /// OpenMath application (function call)
    ///
    /// Represents `head(arg1, arg2, ..., argN)` where:
    /// - `head`: The function being applied (either a deserialized value or nested OpenMath)
    /// - `args`: List of arguments (each either deserialized or raw OpenMath)
    OMA { applicant: Rec, arguments: Vec<Arg> } = 7,

    /// OpenMath binding construct
    ///
    /// Represents constructs that bind variables like quantifiers, lambda expressions, etc.
    /// - `head`: The binding operator (∀, ∃, λ, etc.)
    /// - `context`: List of variable names being bound
    /// - `body`: The expression in which variables are bound
    OMBIND {
        binder: Rec,
        variables: Vec<BoundVariable<'de, Rec, Arg, Arr, Str>>,
        object: Rec,
    } = 8,
}

impl<'de, Rec, Arg, Arr, Str> OpenMath<'de, Rec, Arg, Arr, Str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    /// The [`OpenMathKind`] associated with this object
    ///
    /// ### Examples
    ///```
    /// use openmath::*;
    /// let obj = OMObject(OpenMath::OMI(42.into()).into());
    /// assert_eq!(obj.0.kind(),OpenMathKind::OMI);
    ///```
    pub fn kind(&self) -> OpenMathKind {
        // SAFETY: Both types have #[repr(u8)] and all of Self's discriminant
        // values are OpenMathKind values.
        unsafe {
            let b = *<*const _>::from(self).cast::<u8>();
            std::mem::transmute(b)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OpenMathKind {
    OMI = 0,
    OMF = 1,
    OMSTR = 2,
    OMB = 3,
    OMV = 4,
    OMS = 5,
    OMA = 6,
    OMBIND = 7,
    OME = 8,
    OMATTR = 9,
    OMFOREIGN = 10,
    OMR = 11,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenMathObject<'de, Rec, Arg, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    pub object: OpenMath<'de, Rec, Arg, Arr, Str>,
    pub attrs: Vec<(Uri<'de>, OMOrForeign<'de, Rec, Arg, Arr, Str>)>,
}
impl<'de, Rec, Arg, Arr, Str> From<OpenMath<'de, Rec, Arg, Arr, Str>>
    for OpenMathObject<'de, Rec, Arg, Arr, Str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    #[inline]
    fn from(object: OpenMath<'de, Rec, Arg, Arr, Str>) -> Self {
        Self {
            object,
            attrs: Vec::new(),
        }
    }
}
impl<'de, Rec, Arg, Arr, Str> std::ops::Deref for OpenMathObject<'de, Rec, Arg, Arr, Str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    type Target = OpenMath<'de, Rec, Arg, Arr, Str>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl<'de, Rec, Arg, Arr, Str> OpenMathObject<'de, Rec, Arg, Arr, Str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    pub fn kind(&self) -> OpenMathKind {
        if self.attrs.is_empty() {
            self.object.kind()
        } else {
            OpenMathKind::OMATTR
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundVariable<'de, Rec, Arg, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    name: Str,
    attributions: Vec<(Uri<'de>, OMOrForeign<'de, Rec, Arg, Arr, Str>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri<'de, Str = &'de str>
where
    Str: de::StringLike<'de>,
{
    pub cdbase: Str,
    pub cd: Str,
    pub name: Str,
    pub phantom: &'de (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OMOrForeign<'de, Rec, Arg, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    OMForeign { encoding: Option<Str>, foreign: Str },
    Object(OpenMathObject<'de, Rec, Arg, Arr, Str>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OMObject(pub OpenMathObject<'static, Box<Self>, Self, Vec<u8>, String>);
impl std::ops::Deref for OMObject {
    type Target = OpenMathObject<'static, Box<Self>, Self, Vec<u8>, String>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
