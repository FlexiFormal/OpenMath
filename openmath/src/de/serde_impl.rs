//! # Serde Integration for OpenMath Deserialization
//!
//! This module provides serde integration for OpenMath deserialization, allowing
//! OpenMath objects to be deserialized from any format supported by serde (JSON,
//! XML, YAML, etc.).
//!
//! ## Usage
//!
//! ```rust
//! # #[cfg(feature = "serde")]
//! # {
//! use openmath::{de::OMFromSerde, Int};
//! # use serde_json;
//!
//! // Deserialize from JSON
//! let json = r#"{ "OMI": 42}"#;
//! let wrapper: OMFromSerde<Int<'static>> = serde_json::from_str(json).unwrap();
//! let int_value = wrapper.take();
//! # }
//! ```

use either::Either::{Left, Right};

use super::OpenMath;
use crate::{OMDeserializable, either::Either};
use std::marker::PhantomData;

/// Wrapper type for deserializing OpenMath objects via serde.
///
/// This type wraps any `OMDeserializable` type and provides a `serde::Deserialize`
/// implementation that can parse OpenMath objects from serde-compatible formats.
///
/// # Type Parameters
/// - `'de`: Lifetime of the deserialized data
/// - `OMD`: The target type that implements `OMDeserializable`
/// - `Arr`: Type for byte arrays (default: `&'de [u8]`)
/// - `Str`: Type for strings (default: `&'de str`)
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "serde")]
/// # {
/// use openmath::{de::OMFromSerde, Int};
/// # use serde_json;
///
/// // Deserialize an integer from JSON
/// let json = r#"{ "OMI": 42 }"#;
/// let wrapper: OMFromSerde<Int<'static>> = serde_json::from_str(json).unwrap();
/// let int_value = wrapper.take();
/// assert_eq!(int_value.is_i128(), Some(42));
/// # }
/// ```
pub struct OMFromSerde<'de, OMD, Arr = &'de [u8], Str = &'de str>(
    OMD,
    PhantomData<(&'de (), Arr, Str)>,
)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, OMD, Arr, Str> OMFromSerde<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    /// Extract the deserialized value from the wrapper.
    ///
    /// This consumes the wrapper and returns the underlying deserialized value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "serde")]
    /// # {
    /// use openmath::{de::OMFromSerde, Int};
    /// # use serde_json;
    ///
    /// let json = r#"{ "OMI": 123 }"#;
    /// let wrapper: OMFromSerde<Int<'static>> = serde_json::from_str(json).unwrap();
    /// let value = wrapper.take();
    /// assert_eq!(value.is_i128(), Some(123));
    /// # }
    /// ```
    #[inline]
    pub fn take(self) -> OMD {
        self.0
    }
}
/// Type alias for owned OpenMath deserialization via serde.
///
/// This is a convenience type alias for `OMFromSerde` that uses owned types
/// (`String` and `Vec<u8>`) instead of borrowed ones, allowing the deserialized
/// data to outlive the source.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "serde")]
/// # {
/// use openmath::{de::OMFromSerdeOwned, Int};
/// # use serde_json;
///
/// let json = r#"{ "OMI": "12345678901234567890123456789012345678901234567890" }"#;
/// let wrapper: OMFromSerdeOwned<Int<'static>> = serde_json::from_str(json).unwrap();
/// let big_int = wrapper.take();
/// assert!(big_int.is_big().is_some());
/// # }
/// ```
pub type OMFromSerdeOwned<OMD> = OMFromSerde<'static, OMD, Vec<u8>, String>;

impl<'de, OMD, Arr, Str> serde::Deserialize<'de> for OMFromSerde<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        match OMDe::<'de, OMD, Arr, Str>::deserialize(deserializer)?.0 {
            Left(o) => Ok(Self(o, PhantomData)),
            Right(e) => Err(D::Error::custom(format!(
                "OpenMath object does not represent a valid instance of {}: {e:?}",
                std::any::type_name::<OMD>(),
            ))),
        }
    }
}

/// Internal wrapper for the deserialization result.
///
/// This type holds either a successfully deserialized value or the original
/// OpenMath object if deserialization failed. It's used internally by the
/// serde deserialization process.
struct OMDe<'de, OMD, Arr, Str>(Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, Arr, Str, OMD> serde::de::Deserialize<'de> for OMDe<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /*deserializer.deserialize_struct(
            "OMObject",
            &[
                "kind",
                "id",
                "name",
                "openmath",
                "object",
                "cdbase",
                "cd",
                "integer",
                "decimal",
                "hexadecimal",
                "float",
                "bytes",
                "base64",
                "string",
                "applicant",
                "arguments",
                "attributes",
                "binder",
                "variables",
                "error",
                "href",
                "encoding",
                "foreign",
            ],
            visitor,
        );*/
        deserializer.deserialize_enum(
            "OMObject",
            &["OMI", "OMF", "OMSTR", "OMB", "OMV", "OMS", "OMA", "OMBIND"],
            OMVisitor::<'de, D, OMD, Arr, Str>(PhantomData),
        )
    }
}

/// Serde visitor for OpenMath objects.
///
/// This visitor handles the deserialization of OpenMath objects from serde's
/// enum representation. It recognizes all OpenMath object types and attempts
/// to deserialize them into the target type.
struct OMVisitor<'de, D: serde::Deserializer<'de>, OMD, Arr, Str>(
    PhantomData<(&'de (), D, OMD, Arr, Str)>,
)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

macro_rules! handle {
    ($e:expr) => {
        OMD::from_openmath($e).map(OMDe).map_err(A::Error::custom)
    };
}

impl<'de, D: serde::Deserializer<'de>, OMD, Arr, Str> serde::de::Visitor<'de>
    for OMVisitor<'de, D, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    type Value = OMDe<'de, OMD, Arr, Str>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an OMObject")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        use serde::de::Error;
        use serde::de::VariantAccess;
        let (v, var) = data.variant()?;
        match v {
            "OMI" => handle!(OpenMath::OMI(var.newtype_variant()?)),
            "OMF" => handle!(OpenMath::OMF(var.newtype_variant()?)),
            "OMSTR" => handle!(OpenMath::OMSTR(var.newtype_variant()?)),
            "OMB" => handle!(OpenMath::OMB(var.newtype_variant()?)),
            "OMV" => handle!(OpenMath::OMV(var.newtype_variant()?)),
            "OMS" => {
                let s: Str = var.newtype_variant()?;
                let Some((cd_base, cd_name, name)) = s.split_uri() else {
                    return Err(A::Error::custom("Invalid URI"));
                };
                handle!(OpenMath::OMS {
                    cd_base,
                    cd_name,
                    name
                })
            }
            "OMA" => {
                let OMAWrap(t) = var.newtype_variant()?;
                Ok(OMDe(t))
            }
            "OMBIND" => {
                let OMBindWrap(t) = var.newtype_variant()?;
                Ok(OMDe(t))
            }
            o => Err(A::Error::custom(format!("Unkown OpenMath variant: {o}"))),
        }
    }
}

/// Wrapper for OpenMath applications (OMA) during deserialization.
///
/// This type handles the deserialization of OMA objects, which are represented
/// as sequences with the first element being the head and subsequent elements
/// being the arguments.
struct OMAWrap<'de, OMD, Arr, Str>(Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;
impl<'de, OMD, Arr, Str> serde::Deserialize<'de> for OMAWrap<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(OMAVisitor::<'de, OMD, Arr, Str>(PhantomData))
    }
}

/// Serde visitor for OpenMath applications (OMA).
///
/// This visitor handles the deserialization of OMA objects from their sequence
/// representation. It expects the first element to be the head (function) and
/// the remaining elements to be arguments.
struct OMAVisitor<'de, OMD, Arr, Str>(PhantomData<(&'de (), OMD, Arr, Str)>)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;
impl<'de, OMD, Arr, Str> serde::de::Visitor<'de> for OMAVisitor<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    type Value = OMAWrap<'de, OMD, Arr, Str>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a head term and argument list")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(head) = seq.next_element::<OMDe<'de, OMD, Arr, Str>>()? else {
            return Err(A::Error::custom("empty Object sequence in OMA"));
        };
        let mut args = Vec::new();
        while let Some(arg) = seq.next_element::<OMDe<'de, OMD, Arr, Str>>()? {
            args.push(arg.0);
        }
        if args.is_empty() {
            return Ok(OMAWrap(head.0));
        }
        let head = head.0.map_right(Box::new);
        OMD::from_openmath(OpenMath::OMA { head, args })
            .map(OMAWrap)
            .map_err(A::Error::custom)
    }
}

/// Wrapper for OpenMath applications (OMA) during deserialization.
///
/// This type handles the deserialization of OMA objects, which are represented
/// as sequences with the first element being the head and subsequent elements
/// being the arguments.
struct OMBindWrap<'de, OMD, Arr, Str>(Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;
impl<'de, OMD, Arr, Str> serde::Deserialize<'de> for OMBindWrap<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(OMBindVisitor::<'de, OMD, Arr, Str>(PhantomData))
    }
}

/// Serde visitor for OpenMath applications (OMA).
///
/// This visitor handles the deserialization of OMA objects from their sequence
/// representation. It expects the first element to be the head (function) and
/// the remaining elements to be arguments.
struct OMBindVisitor<'de, OMD, Arr, Str>(PhantomData<(&'de (), OMD, Arr, Str)>)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;
impl<'de, OMD, Arr, Str> serde::de::Visitor<'de> for OMBindVisitor<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    type Value = OMBindWrap<'de, OMD, Arr, Str>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a head term and argument list")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        let Some(head) = seq.next_element::<OMDe<'de, OMD, Arr, Str>>()? else {
            return Err(A::Error::custom("empty Object sequence in OMA"));
        };
        let mut context = Vec::<Str>::new();
        while let Some(arg) = get_var(&mut seq)? {
            match arg {
                Left(a) => context.push(a),
                Right(body) => {
                    let head = head.0.map_right(Box::new);
                    let body = body.0.map_right(Box::new);
                    return OMD::from_openmath(OpenMath::OMBIND {
                        head,
                        context,
                        body,
                    })
                    .map(OMBindWrap)
                    .map_err(A::Error::custom);
                }
            }
        }
        Err(A::Error::custom("Unexpected end of OMBIND"))
    }
}

#[allow(clippy::type_complexity)]
fn get_var<'de, OMD, Arr, Str, A>(
    seq: &mut A,
) -> Result<Option<Either<Str, OMDe<'de, OMD, Arr, Str>>>, A::Error>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
    A: serde::de::SeqAccess<'de>,
{
    if let Ok(Some(a)) = seq.next_element() {
        return Ok(Some(Left(a)));
    }
    seq.next_element().map(|e| e.map(Right))
}

// -------------------------------------------------------------------------------------------------------
//
// -------------------------------------------------------------------------------------------------------
//
// -------------------------------------------------------------------------------------------------------

pub struct OMDe2<'de, OMD, Arr = &'de [u8], Str = &'de str>(
    Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>,
    PhantomData<(&'de (), Arr, Str)>,
)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, OMD, Arr, Str> serde::Deserialize<'de> for OMDe2<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, content) =
            deserializer.deserialize_any(super::serde_aux::TaggedContentVisitor)?;
        let deserializer = super::serde_aux::ContentDeserializer::<D::Error>(content, PhantomData);
        match tag {
            super::OpenMathKind::OMS => Self::oms(deserializer),
            _ => todo!(),
        }
    }
}

pub struct OMDe2Inner<'de, OMD, Arr = &'de [u8], Str = &'de str>(
    Option<Str>,
    PhantomData<(&'de (), Arr, Str, OMD)>,
)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, OMD, Arr, Str> OMDe2<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn oms<E>(deserializer: super::serde_aux::ContentDeserializer<'de, E>) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        let OMS {
            cdbase, cd, name, ..
        } = serde::Deserialize::deserialize(deserializer)?;
        let r = OMD::from_openmath(OpenMath::OMS {
            cd_base: cdbase.unwrap(),
            cd_name: cd,
            name,
        })
        .map_err(E::custom)?;
        Ok(Self(r, PhantomData))
    }
}

#[derive(serde::Deserialize)]
struct OMS<'s, Str: super::StringLike<'s>> {
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    cd: Str,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    name: Str,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cdbase: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    id: Option<Str>,

    #[serde(default)]
    __phantom: PhantomData<&'s ()>,
}

const fn none<T>() -> Option<T> {
    None
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct TestSymbol<'s> {
        cd_base: Option<&'s str>,
        cd_name: &'s str,
        name: &'s str,
    }
    impl<'de> OMDeserializable<'de> for TestSymbol<'de> {
        type Err = &'static str;
        fn from_openmath(
            om: OpenMath<'de, Self, &'de [u8], &'de str>,
        ) -> Result<Either<Self, OpenMath<'de, Self, &'de [u8], &'de str>>, Self::Err>
        where
            Self: Sized,
        {
            let OpenMath::OMS {
                cd_base,
                cd_name,
                name,
            } = om
            else {
                return Err("Failed");
            };
            Ok(Left(Self {
                cd_base,
                cd_name,
                name,
            }))
        }
    }

    fn oms() {
        let str1 =
            r#"{ "kind":"OMS", "id":"foo", "cd":"foo1", "name":"narf" "cdname":"http://foo.com" }"#;
        let str2 = r#"{ "kind":"OMS", "cd":"foo1", "name":"narf" }"#;
        let o:OMDe2<'static,> serde_json::from_str(s)
    }
}
 */
