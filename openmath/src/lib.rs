#![allow(unexpected_cfgs)]
#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]
/*! ## Features */
#![cfg_attr(doc,doc = document_features::document_features!())]
pub mod ser;

use std::{borrow::Cow, convert::Infallible};

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
pub const CD_BASE: &str = "http://www.openmath.org/cd";

/// XML namespace for OpenMath elements
pub const XML_NS: &str = "http://www.openmath.org/OpenMath";

macro_rules! omkinds {
    ($( $(#[$meta:meta])* $id:ident = $v:literal ),* $(,)?) => {
        /// All <span style="font-variant:small-caps;">OpenMath</span> tags/kinds
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
            /// as static string
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {$(
                    Self::$id => stringify!($id)
                ),*}
            }
            /// convert from a byte
            #[must_use]
            pub const fn from_u8(u:u8) -> Option<Self> {
                match u {
                    $( $v => Some(Self::$id) ),*,
                    _ => None
                }
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
    /// <span style="font-variant:small-caps;">OpenMath</span> object. The possible roles are described in
    /// [Section 2.1.4](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_roles).
    ///
    ///</div>
    OMS = 5,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA = 6,

    /** <div class="openmath">
    If $B$ and $C$ are <span style="font-variant:small-caps;">OpenMath</span> objects, and $v_1,...,v_n\;(n\geq0)$
    are <span style="font-variant:small-caps;">OpenMath</span> variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an <span style="font-variant:small-caps;">OpenMath</span> binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND = 7,

    /** <div class="openmath">
    If $S$ is an <span style="font-variant:small-caps;">OpenMath</span> symbol and $A_1,...,A_n\;(n\geq0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects or
    derived <span style="font-variant:small-caps;">OpenMath</span> objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> error object.
    </div> */
    OME = 8,

    /** <div class="openmath">
    If $S_1,...,S_n$ are <span style="font-variant:small-caps;">OpenMath</span> symbols, and $A$ is an <span style="font-variant:small-caps;">OpenMath</span> object, and
    $A_1,...,A_n\;(n>0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects or derived <span style="font-variant:small-caps;">OpenMath</span> objects, then
    $\mathrm{attribution}(A,S_1\;A_1,...,S_n\;A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> attribution object. We call
    $A$ the attributed object, the $S_i$ the keys, and the $A_i$ the attribute values.
    </div> */
    OMATTR = 9,

    /** <div class="openmath">
    If $A$ is not an <span style="font-variant:small-caps;">OpenMath</span> object, then $\mathrm{foreign}(A)$ is an <span style="font-variant:small-caps;">OpenMath</span> foreign object.
    An <span style="font-variant:small-caps;">OpenMath</span> foreign object may optionally have an encoding field which describes how its
    contents should be interpreted.
    </div> */
    OMFOREIGN = 10,

    /** <div class="openmath">
    <span style="font-variant:small-caps;">OpenMath</span> integers, symbols, variables, floating point numbers, character strings, bytearrays,
    applications, binding, attributions, error, and foreign objects can also be encoded as an empty
    OMR element with an href attribute whose value is the value of a URI referencing an id attribute of an
    <span style="font-variant:small-caps;">OpenMath</span> object of that type. The <span style="font-variant:small-caps;">OpenMath</span> element represented by this OMR reference is a copy of the
    <span style="font-variant:small-caps;">OpenMath</span> element referenced href attribute. Note that this copy is structurally equal, but not
    identical to the element referenced. These URI references will often be relative, in which case they
    are resolved using the base URI of the document containing the <span style="font-variant:small-caps;">OpenMath</span>.
    </div> */
    OMR = 11,
}

/// Enum representing all possible OᴘᴇɴMᴀᴛʜ objects.
///
/// This enum encompasses the complete OᴘᴇɴMᴀᴛʜ object model, providing variants
/// for each type of mathematical object that can be represented in <span style="font-variant:small-caps;">OpenMath</span>.
///
/// Note that we add `attributes` to each variant rather than having a separate
/// [`OMATTR`](OMKind::OMATTR) case; that is to avoid having to deal with nested
/// `OMATTR(OMATTR(OMATTR(...` terms or having to make the grammar significantly
/// more complicated.
///
///<div class="openmath">
/// OᴘᴇɴMᴀᴛʜ objects are built recursively as follows.
/// </div>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpenMath<'om> {
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
    <span style="font-variant:small-caps;">OpenMath</span> object. The possible roles are described in Section 2.1.4.
    </div> */
    OMS {
        cd: Cow<'om, str>,
        name: Cow<'om, str>,
        cdbase: Option<Cow<'om, str>>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMS as _,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA {
        applicant: Box<Self>,
        arguments: Vec<Self>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OMA as _,

    /** <div class="openmath">
    If $S$ is an <span style="font-variant:small-caps;">OpenMath</span> symbol and $A_1,...,A_n\;(n\geq0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects or
    derived <span style="font-variant:small-caps;">OpenMath</span> objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> error object.
    </div> */
    OME {
        cd: Cow<'om, str>,
        name: Cow<'om, str>,
        cdbase: Option<Cow<'om, str>>,
        arguments: Vec<OMMaybeForeign<'om, Self>>,
        attributes: Vec<Attr<'om, OMMaybeForeign<'om, Self>>>,
    } = OMKind::OME as _,

    /** <div class="openmath">
    If $B$ and $C$ are <span style="font-variant:small-caps;">OpenMath</span> objects, and $v_1,...,v_n\;(n\geq0)$
    are <span style="font-variant:small-caps;">OpenMath</span> variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an <span style="font-variant:small-caps;">OpenMath</span> binding object.
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

/// A bound variable in an [`OMBIND`](OpenMath::OMBIND)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundVariable<'om> {
    /// the name of the variable
    pub name: Cow<'om, str>,
    /// (optional) attributes of the variable;
    /// this Vec being non-empty represents the case `OMATTR(...,OMV(name))`
    pub attributes: Vec<Attr<'om, OMMaybeForeign<'om, OpenMath<'om>>>>,
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
/// Generic over the attribute value, so it can be used in [OpenMath] and [OM]
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

/// Either an [OpenMath Expression](OpenMath) or an [`OMFOREIGN`](OMKind::OMFOREIGN).
///
/// Generic over the non-OMFOREIGN-case, so it can be used in both [OpenMath] and [OM]
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
    /// converts this into an `Either`(crate::either::Either)
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

impl ser::OMSerializable for OpenMath<'_> {
    fn as_openmath<'s, S: ser::OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        struct NoAttrs<'s, 'o>(&'s OpenMath<'o>);
        impl ser::OMSerializable for NoAttrs<'_, '_> {
            fn as_openmath<'s, S: ser::OMSerializer<'s>>(
                &self,
                serializer: S,
            ) -> Result<S::Ok, S::Err> {
                match self.0 {
                    OpenMath::OMI { int, .. } => int.as_openmath(serializer),
                    OpenMath::OMF { float, .. } => float.0.as_openmath(serializer),
                    OpenMath::OMSTR { string, .. } => string.as_openmath(serializer),
                    OpenMath::OMB { bytes, .. } => bytes.as_openmath(serializer),
                    OpenMath::OMV { name, .. } => ser::Omv(name).as_openmath(serializer),
                    OpenMath::OMS {
                        cd, name, cdbase, ..
                    } => ser::Uri {
                        cdbase: cdbase.as_deref(),
                        name,
                        cd,
                    }
                    .as_oms()
                    .as_openmath(serializer),
                    OpenMath::OMA {
                        applicant,
                        arguments,
                        ..
                    } => serializer.oma(&**applicant, arguments.iter()),
                    OpenMath::OME {
                        cd,
                        name,
                        cdbase,
                        arguments,
                        ..
                    } => serializer.ome(
                        &ser::Uri {
                            cdbase: cdbase.as_deref(),
                            cd,
                            name,
                        },
                        arguments.iter(),
                    ),
                    OpenMath::OMBIND {
                        binder,
                        variables,
                        object,
                        ..
                    } => serializer.ombind(&**binder, variables.iter(), &**object),
                }
            }
        }
        match self {
            Self::OMI { attributes, .. }
            | Self::OMF { attributes, .. }
            | Self::OMSTR { attributes, .. }
            | Self::OMB { attributes, .. }
            | Self::OMV { attributes, .. }
            | Self::OMS { attributes, .. }
            | Self::OMA { attributes, .. }
            | Self::OME { attributes, .. }
            | Self::OMBIND { attributes, .. }
                if !attributes.is_empty() =>
            {
                serializer.omattr(attributes.iter(), NoAttrs(self))
            }
            _ => NoAttrs(self).as_openmath(serializer),
        }
    }
}

impl<'o> de::OMDeserializable<'o> for OpenMath<'o> {
    type Ret = Self;
    type Err = Infallible;
    #[allow(clippy::too_many_lines)]
    fn from_openmath(om: OM<'o, Self>, cdbase: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        /*fn do_attrs<'o>(
            attrs: Vec<de::OMAttr<'o, OpenMath<'o>>>,
        ) -> Vec<Attr<'o, OMMaybeForeign<'o, OpenMath<'o>>>> {
            attrs
                .into_iter()
                .map(|a| Attr {
                    cdbase: a.cdbase,
                    cd: a.cd,
                    name: a.name,
                    value: match a.value {
                        either::Either::Left(a) => OMMaybeForeign::OM(a),
                        either::Either::Right(OMMaybeForeign::Foreign { encoding, value }) => {
                            OMMaybeForeign::Foreign { encoding, value }
                        }
                        either::Either::Right(OMMaybeForeign::OM(_)) => {
                            unreachable!("by construction")
                        }
                    },
                })
                .collect()
        }*/
        Ok(match om {
            OM::OMI { int, attrs } => Self::OMI {
                int,
                attributes: attrs,
            },
            OM::OMF { float, attrs } => Self::OMF {
                float: float.into(),
                attributes: attrs,
            },
            OM::OMSTR { string, attrs } => Self::OMSTR {
                string,
                attributes: attrs,
            },
            OM::OMB { bytes, attrs } => Self::OMB {
                bytes,
                attributes: attrs,
            },
            OM::OMV { name, attrs } => Self::OMV {
                name,
                attributes: attrs,
            },
            OM::OMS { cd, name, attrs } => Self::OMS {
                cd,
                name,
                cdbase: Some(Cow::Owned(cdbase.to_string())),
                attributes: attrs,
            },
            OM::OMA {
                applicant,
                arguments,
                attrs,
            } => Self::OMA {
                applicant: Box::new(applicant),
                arguments: arguments.into_iter().collect(),
                attributes: attrs,
            },
            OM::OMBIND {
                binder,
                variables,
                object,
                attrs,
            } => Self::OMBIND {
                binder: Box::new(binder),
                variables: variables
                    .into_iter()
                    .map(|(name, a)| BoundVariable {
                        name,
                        attributes: a,
                    })
                    .collect(),
                object: Box::new(object),
                attributes: attrs,
            },
            OM::OME {
                cdbase,
                cd,
                name,
                arguments,
                attrs,
            } => Self::OME {
                cd,
                name,
                cdbase,
                arguments,
                attributes: attrs,
            },
        })
    }
}

#[cfg(all(test, feature = "xml", feature = "serde"))]
#[test]
#[allow(clippy::too_many_lines)]
fn roundtrip() {
    use OpenMath::*;
    const XML: &str = r#"<OMOBJ version="2.0" xmlns="http://www.openmath.org/OpenMath">
      <OMBIND>
        <OMS cdbase="http://openmath.org/cd" cd="fns1" name="lambda"/>
        <OMBVAR>
          <OMV name="x"/>
          <OMATTR>
            <OMATP>
              <OMS cdbase="http://openmath.org/cd" cd="nope" name="type"/>
              <OMS cdbase="http://openmath.org/cd" cd="arith1" name="real"/>
            </OMATP>
          <OMV name="y"/>
          </OMATTR>
        </OMBVAR>
        <OMA>
          <OMS cdbase="http://my.namespace" cd="utils" name="either"/>
          <OMA>
            <OMS cdbase="http://openmath.org/cd" cd="arith1" name="plus"/>
            <OMI>128</OMI>
            <OMATTR>
              <OMATP>
                <OMS cdbase="http://openmath.org/cd" cd="nope" name="type"/>
                <OMFOREIGN>
                  <MOOT>this is an opaque OMFOREIGN</MOOT>
                </OMFOREIGN>
              </OMATP>
            <OMI>-1234567898765432123456789</OMI>
            </OMATTR>
            <OMF dec="3.88988"/>
            <OMSTR>some number</OMSTR>
            <OMV name="x"/>
          </OMA>
          <OME>
            <OMS cdbase="http://openmath.org" cd="error" name="unhandled_arithmetics"/>
            <OMFOREIGN encoding="application/nonsense">
              ERROAR CODE MOO
            </OMFOREIGN>
          </OME>
        </OMA>
      </OMBIND>
    </OMOBJ>"#;
    const JSON: &str = r#"{
      "kind": "OMOBJ",
      "openmath": "2.0",
      "object": {
        "kind": "OMBIND",
        "binder": {
          "kind": "OMS",
          "cdbase": "http://openmath.org/cd",
          "cd": "fns1",
          "name": "lambda"
        },
        "variables": [
          {
            "kind": "OMV",
            "name": "x"
          },
          {
            "kind": "OMATTR",
            "attributes": [
              [
                {
                  "kind": "OMS",
                  "cdbase": "http://openmath.org/cd",
                  "cd": "nope",
                  "name": "type"
                },
                {
                  "kind": "OMS",
                  "cdbase": "http://openmath.org/cd",
                  "cd": "arith1",
                  "name": "real"
                }
              ]
            ],
            "object": {
              "kind": "OMV",
              "name": "y"
            }
          }
        ],
        "object": {
          "kind": "OMA",
          "applicant": {
            "kind": "OMS",
            "cdbase": "http://my.namespace",
            "cd": "utils",
            "name": "either"
          },
          "arguments": [
            {
              "kind": "OMA",
              "applicant": {
                "kind": "OMS",
                "cdbase": "http://openmath.org/cd",
                "cd": "arith1",
                "name": "plus"
              },
              "arguments": [
                {
                  "kind": "OMI",
                  "integer": 128
                },
                {
                  "kind": "OMATTR",
                  "attributes": [
                    [
                      {
                        "kind": "OMS",
                        "cdbase": "http://openmath.org/cd",
                        "cd": "nope",
                        "name": "type"
                      },
                      {
                        "kind": "OMFOREIGN",
                        "foreign": "<MOOT>this is an opaque OMFOREIGN</MOOT>"
                      }
                    ]
                  ],
                  "object": {
                    "kind": "OMI",
                    "integer": -1234567898765432123456789
                  }
                },
                {
                  "kind": "OMF",
                  "float": 3.88988
                },
                {
                  "kind": "OMSTR",
                  "string": "some number"
                },
                {
                  "kind": "OMV",
                  "name": "x"
                }
              ]
            },
            {
              "kind": "OME",
              "error": {
                "kind": "OMS",
                "cdbase": "http://openmath.org",
                "cd": "error",
                "name": "unhandled_arithmetics"
              },
              "arguments": [
                {
                  "kind": "OMFOREIGN",
                  "foreign": "ERROAR CODE MOO",
                  "encoding": "application/nonsense"
                }
              ]
            }
          ]
        }
      }
    }"#;

    let om = OMBIND {
        binder: Box::new(OMS {
            cdbase: Some(Cow::Borrowed("http://openmath.org/cd")),
            cd: Cow::Borrowed("fns1"),
            name: Cow::Borrowed("lambda"),
            attributes: Vec::new(),
        }),
        variables: vec![
            BoundVariable {
                name: Cow::Borrowed("x"),
                attributes: Vec::new(),
            },
            BoundVariable {
                name: Cow::Borrowed("y"),
                attributes: vec![Attr {
                    cdbase: Some(Cow::Borrowed("http://openmath.org/cd")),
                    cd: Cow::Borrowed("nope"),
                    name: Cow::Borrowed("type"),
                    value: OMMaybeForeign::OM(OMS {
                        cdbase: Some(Cow::Borrowed("http://openmath.org/cd")),
                        cd: Cow::Borrowed("arith1"),
                        name: Cow::Borrowed("real"),
                        attributes: Vec::new(),
                    }),
                }],
            },
        ],
        object: Box::new(OMA {
            applicant: Box::new(OMS {
                cd: Cow::Borrowed("utils"),
                name: Cow::Borrowed("either"),
                cdbase: Some(Cow::Borrowed("http://my.namespace")),
                attributes: Vec::new(),
            }),
            arguments: vec![
                OMA {
                    applicant: Box::new(OMS {
                        cdbase: Some(Cow::Borrowed("http://openmath.org/cd")),
                        cd: Cow::Borrowed("arith1"),
                        name: Cow::Borrowed("plus"),
                        attributes: Vec::new(),
                    }),
                    arguments: vec![
                        OMI {
                            int: 128.into(),
                            attributes: Vec::new(),
                        },
                        OMI {
                            int: Int::new("-1234567898765432123456789").expect("works"),
                            attributes: vec![Attr {
                                cdbase: Some(Cow::Borrowed("http://openmath.org/cd")),
                                cd: Cow::Borrowed("nope"),
                                name: Cow::Borrowed("type"),
                                value: OMMaybeForeign::Foreign {
                                    encoding: None,
                                    value: Cow::Borrowed(
                                        "<MOOT>this is an opaque OMFOREIGN</MOOT>",
                                    ),
                                },
                            }],
                        },
                        OMF {
                            float: 3.88988.into(),
                            attributes: Vec::new(),
                        },
                        OMSTR {
                            string: Cow::Borrowed("some number"),
                            attributes: Vec::new(),
                        },
                        OMV {
                            name: Cow::Borrowed("x"),
                            attributes: Vec::new(),
                        },
                    ],
                    attributes: Vec::new(),
                },
                OME {
                    cdbase: Some(Cow::Borrowed("http://openmath.org")),
                    cd: Cow::Borrowed("error"),
                    name: Cow::Borrowed("unhandled_arithmetics"),
                    arguments: vec![OMMaybeForeign::Foreign {
                        encoding: Some(Cow::Borrowed("application/nonsense")),
                        value: Cow::Borrowed("ERROAR CODE MOO"),
                    }],
                    attributes: Vec::new(),
                },
            ],
            attributes: Vec::new(),
        }),
        attributes: Vec::new(),
    };

    let json = serde_json::to_string_pretty(&ser::OMObject(&om)).expect("works");
    assert_eq!(
        json.replace(|c: char| c.is_ascii_whitespace(), ""),
        JSON.replace(|c: char| c.is_ascii_whitespace(), "")
    );
    let nom = serde_json::from_str::<'_, de::OMObject<OpenMath<'_>>>(&json)
        .expect("works")
        .into_inner();
    assert_eq!(om, nom);
    let xml = ser::OMObject(&nom).xml(true, true).to_string();
    assert_eq!(
        xml.replace(|c: char| c.is_ascii_whitespace(), ""),
        XML.replace(|c: char| c.is_ascii_whitespace(), "")
    );
    let nom = de::OMObject::<OpenMath<'_>>::from_openmath_xml(&xml).expect("works");
    assert_eq!(om, nom);
}
