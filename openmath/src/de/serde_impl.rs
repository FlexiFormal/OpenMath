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
//! let json = r#"{ "kind": "OMI", "integer": 42}"#;
//! let wrapper: OMFromSerde<Int<'static>> = serde_json::from_str(json).unwrap();
//! let int_value = wrapper.take();
//! # }
//! ```
#![allow(clippy::trait_duplication_in_bounds)]
#![allow(clippy::upper_case_acronyms)]

use either::Either::{Left, Right};
use serde::de::DeserializeSeed;

use super::OpenMath;
use crate::{OMDeserializable, either::Either};
use std::{borrow::Cow, marker::PhantomData};

/// Wrapper type for deserializing OpenMath objects via serde.
///
/// This type wraps any `OMDeserializable` type and provides a `serde::Deserialize`
/// implementation that can parse OpenMath objects from serde-compatible formats.
///
/// # Type Parameters
/// - `'de`: Lifetime of the deserialized data
/// - `OMD`: The target type that implements `OMDeserializable`
/// - `Arr`: Type for byte arrays (default: <code>[Cow]<'de, [u8]></code>)
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
/// let json = r#"{ "kind": "OMI", "integer": 42 }"#;
/// let wrapper: OMFromSerde<Int<'static>> = serde_json::from_str(json).unwrap();
/// let int_value = wrapper.take();
/// assert_eq!(int_value.is_i128(), Some(42));
/// # }
/// ```
pub struct OMFromSerde<'de, OMD, Arr = Cow<'de, [u8]>, Str = &'de str>(
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
    /// let json = r#"{ "kind": "OMI", "integer": 123 }"#;
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
/// let json = r#"{ "kind":"OMI", "decimal": "12345678901234567890123456789012345678901234567890" }"#;
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

struct OMDe<'de, OMD, Arr = Cow<'de, [u8]>, Str = &'de str>(
    Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>,
    PhantomData<(&'de (), Arr, Str)>,
)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, OMD, Arr, Str> serde::Deserialize<'de> for OMDe<'de, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        OMDeInner(Right(crate::OPENMATH_BASE_URI.as_str()), PhantomData).deserialize(deserializer)
    }
}

#[impl_tools::autoimpl(Clone)]
struct OMDeInner<'de, 's, OMD, Arr = Cow<'de, [u8]>, Str = &'de str>(
    Either<Str, &'s str>,
    PhantomData<(&'de (), Arr, Str, OMD)>,
)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, OMD, Arr, Str> serde::de::DeserializeSeed<'de> for OMDeInner<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    type Value = OMDe<'de, OMD, Arr, Str>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, content) =
            deserializer.deserialize_any(super::serde_aux::TaggedContentVisitor)?;
        let deserializer = super::serde_aux::ContentDeserializer::<D::Error>(content, PhantomData);
        match tag {
            super::OpenMathKind::OMI => self.omi(deserializer),
            super::OpenMathKind::OMF => self.omf(deserializer),
            super::OpenMathKind::OMSTR => self.omstr(deserializer),
            super::OpenMathKind::OMB => self.omb(deserializer),
            super::OpenMathKind::OMV => self.omv(deserializer),
            super::OpenMathKind::OMS => self.oms(deserializer),
            super::OpenMathKind::OMA => self.oma(deserializer),
            super::OpenMathKind::OMBIND => self.ombind(deserializer),
        }
    }
}

impl<'de, OMD, Arr, Str> OMDeInner<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
{
    fn omi<E>(
        self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMI::<'_, Str> {
            integer,
            decimal,
            hexadecimal,
            ..
        } = serde::Deserialize::deserialize(deserializer)?;
        let r = if let Some(i) = integer {
            if decimal.is_some() || hexadecimal.is_some() {
                return Err(E::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            crate::Int::from(i)
        } else if let Some(s) = decimal {
            if hexadecimal.is_some() {
                return Err(E::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            s.into_int()
                .ok_or_else(|| E::custom("invalid decimal number"))?
        } else if let Some(s) = hexadecimal {
            return Err(E::custom(format_args!(
                "Not yet implemented: hexadecimal in OMI: {s}"
            )));
        } else {
            return Err(E::custom("Missing value for OMI"));
        };

        let r = OMD::from_openmath(OpenMath::OMI(r), &self.0).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn omf<E>(
        self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMF::<'_, Str> {
            float,
            decimal,
            hexadecimal,
            ..
        } = serde::Deserialize::deserialize(deserializer)?;
        let r = if let Some(f) = float {
            if decimal.is_some() || hexadecimal.is_some() {
                return Err(E::custom(
                    "OMF can not have more than one of the fields `float`, `decimal`, `hexadecimal`",
                ));
            }
            f
        } else if let Some(s) = decimal {
            if hexadecimal.is_some() {
                return Err(E::custom(
                    "OMF can not have more than one of the fields `float`, `decimal`, `hexadecimal`",
                ));
            }
            return Err(E::custom(format_args!(
                "Not yet implemented: decimal in OMF: {s}"
            )));
        } else if let Some(s) = hexadecimal {
            return Err(E::custom(format_args!(
                "Not yet implemented: hexadecimal in OMF: {s}"
            )));
        } else {
            return Err(E::custom("Missing value for OMF"));
        };

        let r = OMD::from_openmath(OpenMath::OMF(r), &self.0).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn omstr<E>(
        self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMSTR::<'_, Str> { string, .. } = serde::Deserialize::deserialize(deserializer)?;

        let r = OMD::from_openmath(OpenMath::OMSTR(string), &self.0).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn omb<E>(
        self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMB::<'_, Arr, Str> { bytes, base64, .. } =
            serde::Deserialize::deserialize(deserializer)?;
        let arr = if let Some(bytes) = bytes {
            if base64.is_some() {
                return Err(E::custom(
                    "OMB can not have more than one of the fields `bytes`, `base64`",
                ));
            }
            bytes
        } else if let Some(base64) = base64 {
            crate::base64::decode(base64.as_ref())
                .map_err(E::custom)?
                .into()
        } else {
            return Err(E::custom("Missing value for OMB"));
        };

        let r = OMD::from_openmath(OpenMath::OMB(arr), &self.0).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn omv<E>(
        self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMV::<'_, Str> { name, .. } = serde::Deserialize::deserialize(deserializer)?;

        let r = OMD::from_openmath(OpenMath::OMV(name), &self.0).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn oms<E>(
        self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMS::<'de, Str> {
            cdbase, cd, name, ..
        } = serde::Deserialize::deserialize(deserializer)?;
        let cdbase: &str = cdbase.as_ref().map_or::<&str, _>(&self.0, |s| s.as_ref());

        let r =
            OMD::from_openmath(OpenMath::OMS { cd_name: cd, name }, cdbase).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn oma<E>(
        mut self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMA::<'de, Str> {
            cdbase,
            applicant,
            arguments,
            ..
        } = serde::Deserialize::deserialize(deserializer)?;
        if let Some(cdbase) = cdbase {
            self.0 = Left(cdbase);
        }

        let sub = OMDeInner::<'de, '_, OMD, Arr, Str>(Right(&self.0), PhantomData);
        let head = sub
            .clone()
            .deserialize(super::serde_aux::ContentDeserializer::<'de, E>(
                applicant,
                PhantomData,
            ))?
            .0
            .map_right(Box::new);
        let args = arguments
            .into_iter()
            .map(|a| {
                sub.clone()
                    .deserialize(super::serde_aux::ContentDeserializer::<'de, E>(
                        a,
                        PhantomData,
                    ))
                    .map(|r| r.0)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let r = OMD::from_openmath(OpenMath::OMA { head, args }, &self.0).map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }

    fn ombind<E>(
        mut self,
        deserializer: super::serde_aux::ContentDeserializer<'de, E>,
    ) -> Result<OMDe<'de, OMD, Arr, Str>, E>
    where
        E: serde::de::Error,
    {
        let OMBIND::<'de, Str> {
            cdbase,
            binder,
            variables,
            object,
            ..
        } = serde::Deserialize::deserialize(deserializer)?;
        if let Some(cdbase) = cdbase {
            self.0 = Left(cdbase);
        }

        let sub = OMDeInner::<'de, '_, OMD, Arr, Str>(Right(&self.0), PhantomData);
        let head = sub
            .clone()
            .deserialize(super::serde_aux::ContentDeserializer::<'de, E>(
                binder,
                PhantomData,
            ))?
            .0
            .map_right(Box::new);
        let context = variables
            .into_iter()
            .map(|a| {
                Str::deserialize(super::serde_aux::ContentDeserializer::<'de, E>(
                    a,
                    PhantomData,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let body = sub
            .clone()
            .deserialize(super::serde_aux::ContentDeserializer::<'de, E>(
                object,
                PhantomData,
            ))?
            .0
            .map_right(Box::new);

        let r = OMD::from_openmath(
            OpenMath::OMBIND {
                head,
                context,
                body,
            },
            &self.0,
        )
        .map_err(E::custom)?;
        Ok(OMDe(r, PhantomData))
    }
}

#[derive(serde::Deserialize)]
struct OMI<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(default)]
    integer: Option<i64>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    decimal: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    hexadecimal: Option<Str>,
}

#[derive(serde::Deserialize)]
struct OMF<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(default)]
    float: Option<f64>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    decimal: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    hexadecimal: Option<Str>,
}

#[derive(serde::Deserialize)]
struct OMSTR<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    string: Str,
}

#[derive(serde::Deserialize)]
struct OMB<'s, Arr: super::Bytes<'s>, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Arr: super::Bytes<'s>"))]
    bytes: Option<Arr>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    base64: Option<Str>,
}

#[derive(serde::Deserialize)]
struct OMV<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    name: Str,
}

#[derive(serde::Deserialize)]
struct OMS<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cdbase: Option<Str>,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cd: Str,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    name: Str,
}

#[derive(serde::Deserialize)]
struct OMA<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cdbase: Option<Str>,

    applicant: super::serde_aux::Content<'s>,

    #[serde(default)]
    arguments: Vec<super::serde_aux::Content<'s>>,
}

#[derive(serde::Deserialize)]
struct OMBIND<'s, Str: super::StringLike<'s>> {
    #[serde(default)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    id: Option<Str>,

    #[serde(default = "none")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cdbase: Option<Str>,

    binder: super::serde_aux::Content<'s>,

    variables: Vec<super::serde_aux::Content<'s>>,

    object: super::serde_aux::Content<'s>,
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
