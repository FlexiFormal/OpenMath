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
pub use de::{OM, OMDeserializable};
pub mod base64;
mod int;
/// reexported for convenience
pub use either;
pub use int::Int;

use crate::ser::AsOMS;

/// The base URI of official OᴘᴇɴMᴀᴛʜ dictionaries (`http://www.openmath.org/cd`)
pub static OPENMATH_BASE_URI: std::sync::LazyLock<url::Url> = std::sync::LazyLock::new(||
    // SAFETY: Known to be a valid Url
    unsafe{
        url::Url::parse("http://www.openmath.org/cd").unwrap_unchecked()
    });

/// XML namespace for OpenMath elements
pub const XML_NAMESPACE: &str = "http://www.openmath.org/OpenMath";

macro_rules! omkinds {
    ($( $(#[$meta:meta])* $id:ident = $v:literal ),* $(,)?) => {
        /// All OpenMath tags/kinds
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        pub enum OMKind {
            $(
                $(#[$meta])*
                $id = $v
            ),*
        }
        impl OMKind {
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {$(
                    Self::$id => stringify!($id)
                ),*}
            }
        }
        impl std::fmt::Display for OMKind {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

omkinds! {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OMExpr<'om> {
    /** <div class="openmath">
    Integers in the mathematical sense, with no predefined range.
    They are “infinite precision” integers (also called “bignums” in computer algebra).
    </div> */
    OMI {
        int: Int<'om>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMI as _,

    /** <div class="openmath">
    Double precision floating-point numbers following the IEEE 754-1985 standard.
    </div> */
    OMF {
        float: ordered_float::OrderedFloat<f64>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMF as _,

    /** <div class="openmath">
    A Unicode Character string. This also corresponds to “characters” in XML.
    </div> */
    OMSTR {
        string: Cow<'om, str>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMSTR as _,

    /** <div class="openmath">
    A sequence of bytes.
    </div> */
    OMB {
        bytes: Cow<'om, [u8]>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMB as _,

    ///<div class="openmath">
    ///
    /// A Variable must have a name which is a sequence of characters matching a regular
    /// expression, as described in [Section 2.3](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_names).
    ///
    ///</div>
    ///
    ///(Note: We do not enforce that names are valid XML names;)
    OMV {
        name: Cow<'om, str>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMV as _,

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
    OMS {
        cd: Cow<'om, str>,
        name: Cow<'om, str>,
        cd_base: Option<Cow<'om, str>>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMS as _,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are OpenMath objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an OpenMath application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA {
        applicant: Box<Self>,
        arguments: Vec<Self>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMA as _,

    /** <div class="openmath">
    If $S$ is an OpenMath symbol and $A_1,...,A_n\;(n\geq0)$ are OpenMath objects or
    derived OpenMath objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an OpenMath error object.
    </div> */
    OME {
        cd: Cow<'om, str>,
        name: Cow<'om, str>,
        cdbase: Option<Cow<'om, str>>,
        arguments: Vec<OMMaybeForeign<'om, Self>>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OME as _,

    /** <div class="openmath">
    If $B$ and $C$ are OpenMath objects, and $v_1,...,v_n$\;(n\geq0)$
    are OpenMath variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an OpenMath binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND {
        binder: Box<Self>,
        variables: Vec<BoundVariable<'om>>,
        object: Box<Self>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMBIND as _,
}

/// A bound variable in an [`OMBIND`](OMExpr::OMBIND)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundVariable<'om> {
    name: Cow<'om, str>,
    attributes: Vec<Attr<'om, OMMaybeForeign<'om, OMExpr<'om>>>>,
}
impl ser::BindVar for &BoundVariable<'_> {
    #[inline]
    fn attrs(&self) -> impl ExactSizeIterator<Item: ser::OMAttr> {
        self.attributes.iter()
    }
    #[inline]
    fn name(&self) -> impl std::fmt::Display {
        &*self.name
    }
}

/// An attribute in an [`OMATTR`](OMKind::OMATTR)
///
/// Generic, so it can be reused in [OM](de::OM)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attr<'o, I> {
    pub cdbase: Option<Cow<'o, str>>,
    pub cd: Cow<'o, str>,
    pub name: Cow<'o, str>,
    pub value: I,
}
impl<I> ser::OMAttr for &Attr<'_, I>
where
    for<'a> &'a I: ser::OMOrForeign,
{
    #[inline]
    fn symbol(&self) -> impl AsOMS {
        ser::Uri {
            cdbase: self.cdbase.as_deref(),
            cd: &self.cd,
            name: &self.name,
        }
    }
    fn value(&self) -> impl ser::OMOrForeign {
        &self.value
    }
}

/// Either an [OpenMath Expression](OMExpr) or an [`OMFOREIGN`](OMKind::OMFOREIGN).
///
/// Generic, so it can be reused in [OM](de::OM)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OMMaybeForeign<'o, I> {
    // An OMExpr
    OM(I),

    /** <div class="openmath">
    If $A$ is not an OpenMath object, then $\mathrm{foreign}(A)$ is an OpenMath foreign object.
    An OpenMath foreign object may optionally have an encoding field which describes how its
    contents should be interpreted.
    </div> */
    Foreign {
        encoding: Option<Cow<'o, str>>,
        value: Cow<'o, str>,
    },
}
impl<I: ser::OMSerializable> ser::OMOrForeign for &OMMaybeForeign<'_, I> {
    fn om_or_foreign(
        self,
    ) -> crate::either::Either<
        impl OMSerializable,
        (Option<impl std::fmt::Display>, impl std::fmt::Display),
    > {
        match self {
            OMMaybeForeign::OM(i) => either::Either::Left(i),
            OMMaybeForeign::Foreign { encoding, value } => {
                either::Either::Right((encoding.as_deref(), &**value))
            }
        }
    }
}

impl ser::OMSerializable for OMExpr<'_> {
    fn as_openmath<'s, S: ser::OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        //struct NoAttrs<'s,'o>(&'s OMExpr<'o>);
        match self {
            Self::OMI { int, attributes } if attributes.is_empty() => int.as_openmath(serializer),
            Self::OMF { float, attributes } if attributes.is_empty() => {
                float.0.as_openmath(serializer)
            }
            Self::OMSTR { string, attributes } if attributes.is_empty() => {
                string.as_openmath(serializer)
            }
            Self::OMB { bytes, attributes } if attributes.is_empty() => {
                bytes.as_openmath(serializer)
            }
            Self::OMV { name, attributes } if attributes.is_empty() => {
                ser::Omv(name).as_openmath(serializer)
            }
            Self::OMS {
                cd,
                name,
                cd_base,
                attributes,
            } if attributes.is_empty() => ser::Uri {
                cdbase: cd_base.as_deref(),
                name,
                cd,
            }
            .as_oms()
            .as_openmath(serializer),
            Self::OMA {
                applicant,
                arguments,
                attributes,
            } if attributes.is_empty() => serializer.oma(&**applicant, arguments.iter()),
            Self::OME {
                cd,
                name,
                cdbase,
                arguments,
                attributes,
            } if attributes.is_empty() => serializer.ome(
                &ser::Uri {
                    cdbase: cdbase.as_deref(),
                    cd,
                    name,
                },
                arguments.iter(),
            ),
            Self::OMBIND {
                binder,
                variables,
                object,
                attributes,
            } if attributes.is_empty() => serializer.ombind(&**binder, variables.iter(), &**object),
            _ => todo!(),
        }
    }
}

//impl<'o> de::OMDeserializable<'o> for OMExpr<'o> {}
