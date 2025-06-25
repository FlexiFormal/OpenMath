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

/// Enum representing all possible OᴘᴇɴMᴀᴛʜ objects.
///
/// This enum encompasses the complete OᴘᴇɴMᴀᴛʜ object model, providing variants
/// for each type of mathematical object that can be represented in OpenMath.
///
/// # Type Parameters
/// - `'de`: Lifetime of the deserialized data
/// - `I`: The type that implements [`OMDeserializable`] (your target type)
/// - `Arr`: Type for byte arrays (default: <code>[Cow]<'b, [u8]></code>)
/// - `Str`: Type for strings (default: `&'de str`)
///
///<div class="openmath">
/// OᴘᴇɴMᴀᴛʜ objects are built recursively as follows.
/// </div>
///
/// Note that we do not implement the `OMATTR` case; that's because
/// we use [`OpenMathObject`] instead of [`OpenMath`], which has a
/// (possibly empty) field [attrs](OpenMathObject::attrs) for attributions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum OpenMath<'de, Rec, Arg, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    /** <div class="openmath">
    Integers in the mathematical sense, with no predefined range.
    They are “infinite precision” integers (also called “bignums” in computer algebra).
    </div> */
    OMI(Int<'de>) = OpenMathKind::OMI as _,

    /** <div class="openmath">
    Double precision floating-point numbers following the IEEE 754-1985 standard.
    </div> */
    OMF(ordered_float::OrderedFloat<f64>) = OpenMathKind::OMF as _,

    /** <div class="openmath">
    A Unicode Character string. This also corresponds to “characters” in XML.
    </div> */
    OMSTR(Str) = OpenMathKind::OMSTR as _,

    /** <div class="openmath">
    A sequence of bytes.
    </div> */
    OMB(Arr) = OpenMathKind::OMB as _,

    ///<div class="openmath">
    ///
    /// A Variable must have a name which is a sequence of characters matching a regular
    /// expression, as described in [Section 2.3](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_names).
    ///
    ///</div>
    ///
    ///(Note: We do not enforce that names are valid XML names;)
    // */
    OMV(Str) = OpenMathKind::OMV as _,

    /** <div class="openmath">
    A Symbol encodes three fields of information, a symbol name, a Content Dictionary name,
    and (optionally) a Content Dictionary base URI, The name of a symbol is a sequence of
    characters matching the regular expression described in Section 2.3.
    The Content Dictionary is the location of the definition of the symbol, consisting of a
    name (a sequence of characters matching the regular expression described in Section 2.3)
    and, optionally, a unique prefix called a cdbase which is used to disambiguate multiple
    Content Dictionaries of the same name. There are other properties of the symbol that are
    not explicit in these fields but whose values may be obtained by inspecting the Content
    Dictionary specified. These include the symbol definition, formal properties and examples
    and, optionally, a role which is a restriction on where the symbol may appear in an
    OpenMath object. The possible roles are described in Section 2.1.4.
    </div> */
    OMS { cd: Str, name: Str } = OpenMathKind::OMS as _,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are OpenMath objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an OpenMath application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA { applicant: Rec, arguments: Vec<Arg> } = OpenMathKind::OMA as _,

    /** <div class="openmath">
    If $B$ and $C$ are OpenMath objects, and $v_1,...,v_n$\;(n\geq0)$
    are OpenMath variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an OpenMath binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND {
        binder: Rec,
        variables: Vec<BoundVariable<'de, Rec, Arg, Arr, Str>>,
        object: Rec,
    } = OpenMathKind::OMBIND as _,
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
    /** <div class="openmath">
    Integers in the mathematical sense, with no predefined range.
    They are “infinite precision” integers (also called “bignums” in computer algebra).
    </div> */
    OMI = 0,

    /** <div class="openmath">
    Double precision floating-point numbers following the IEEE 754-1985 standard.
    </div> */
    OMF = 1,

    /** <div class="openmath">
    A Unicode Character string. This also corresponds to “characters” in XML.
    </div> */
    OMSTR = 2,

    /** <div class="openmath">
    A sequence of bytes.
    </div> */
    OMB = 3,

    ///<div class="openmath">
    ///
    /// A Variable must have a name which is a sequence of characters matching a regular
    /// expression, as described in [Section 2.3](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_names).
    ///
    ///</div>
    ///
    ///(Note: We do not enforce that names are valid XML names;)
    OMV = 4,

    ///<div class="openmath">
    ///
    /// A Symbol encodes three fields of information, a symbol name, a Content Dictionary name,
    /// and (optionally) a Content Dictionary base URI, The name of a symbol is a sequence of
    /// characters matching the regular expression described in
    /// [Section 2.3](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_names).
    /// The Content Dictionary is the location of the definition of the symbol, consisting of a
    /// name (a sequence of characters matching the regular expression described in
    /// [Section 2.3](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_names))
    /// and, optionally, a unique prefix called a cdbase which is used to disambiguate multiple
    /// Content Dictionaries of the same name. There are other properties of the symbol that are
    /// not explicit in these fields but whose values may be obtained by inspecting the Content
    /// Dictionary specified. These include the symbol definition, formal properties and examples
    /// and, optionally, a role which is a restriction on where the symbol may appear in an
    /// OpenMath object. The possible roles are described in
    /// [Section 2.1.4](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_roles).
    ///
    ///</div>
    OMS = 5,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are OpenMath objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an OpenMath application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA = 6,

    /** <div class="openmath">
    If $B$ and $C$ are OpenMath objects, and $v_1,...,v_n\;(n\geq0)$
    are OpenMath variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an OpenMath binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND = 7,

    /** <div class="openmath">
    If $S$ is an OpenMath symbol and $A_1,...,A_n\;(n\geq0)$ are OpenMath objects or
    derived OpenMath objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an OpenMath error object.
    </div> */
    OME = 8,

    /** <div class="openmath">
    If $S_1,...,S_n$ are OpenMath symbols, and $A$ is an OpenMath object, and
    $A_1,...,A_n\;(n>0)$ are OpenMath objects or derived OpenMath objects, then
    $\mathrm{attribution}(A,S_1\;A_1,...,S_n\;A_n)$ is an OpenMath attribution object. We call
    $A$ the attributed object, the $S_i$ the keys, and the $A_i$ the attribute values.
    </div> */
    OMATTR = 9,

    /** <div class="openmath">
    If $A$ is not an OpenMath object, then $\mathrm{foreign}(A)$ is an OpenMath foreign object.
    An OpenMath foreign object may optionally have an encoding field which describes how its
    contents should be interpreted.
    </div> */
    OMFOREIGN = 10,

    /** <div class="openmath">
    OpenMath integers, symbols, variables, floating point numbers, character strings, bytearrays,
    applications, binding, attributions, error, and foreign objects can also be encoded as an empty
    OMR element with an href attribute whose value is the value of a URI referencing an id attribute of an
    OpenMath object of that type. The OpenMath element represented by this OMR reference is a copy of the
    OpenMath element referenced href attribute. Note that this copy is structurally equal, but not
    identical to the element referenced. These URI references will often be relative, in which case they
    are resolved using the base URI of the document containing the OpenMath.
    </div> */
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
impl<'de, Rec, Arg, Arr, Str> OMOrForeign<'de, Rec, Arg, Arr, Str>
where
    Arr: de::Bytes<'de>,
    Str: de::StringLike<'de>,
{
    pub fn kind(&self) -> OpenMathKind {
        match self {
            Self::OMForeign { .. } => OpenMathKind::OMFOREIGN,
            Self::Object(o) => o.kind(),
        }
    }
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
