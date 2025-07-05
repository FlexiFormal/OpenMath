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
#![allow(clippy::type_complexity)]

use either::Either::{Left, Right};
use serde::{Deserialize, de::DeserializeSeed};

use super::OpenMath;
use crate::{
    OMDeserializable, OpenMathKind,
    de::{OMForeign, StringLike},
    either::Either,
};
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
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
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
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        OMDeInner(Right(crate::OPENMATH_BASE_URI.as_str()), PhantomData).deserialize(deserializer)
    }
}

#[impl_tools::autoimpl(Clone)]
struct OMDeInner<'de, 's, OMD, Arr, Str>(
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
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    type Value = OMDe<'de, OMD, Arr, Str>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_struct(
                "OMObject",
                &ALL_FIELDS,
                OMVisitor::<Arr, Str, OMD, false>(self.0, PhantomData),
            )
            .map(|r| OMDe(r, PhantomData))
    }
}

// -------------------------------------------------------------------------------------

#[allow(non_camel_case_types)]
#[derive(strum::Display)]
enum AllFields {
    kind,
    id,
    cdbase,
    integer,
    decimal,
    hexadecimal,
    float,
    string,
    bytes,
    base64,
    name,
    cd,
    encoding,
    foreign,
    error,
    arguments,
    applicant,
    binder,
    variables,
    object,
    __ignore,
}
static ALL_FIELDS: [&str; 20] = [
    "kind",
    "id",
    "cdbase",
    "integer",
    "decimal",
    "hexadecimal",
    "float",
    "string",
    "bytes",
    "base64",
    "name",
    "cd",
    "encoding",
    "foreign",
    "error",
    "arguments",
    "applicant",
    "binder",
    "variables",
    "object",
];
#[impl_tools::autoimpl(Default)]
struct FieldState<'de, Arr: super::Bytes<'de>, Str: super::StringLike<'de>> {
    id: Option<&'de str>,
    integer: Option<i64>,
    decimal: Option<&'de str>,
    hexadecimal: Option<&'de str>,
    float: Option<f64>,
    string: Option<Str>,
    bytes: Option<Arr>,
    base64: Option<&'de str>,
    name: Option<Str>,
    cdbase: Option<&'de str>,
    cd: Option<Str>,
    encoding: Option<Str>,
    foreign: Option<Str>,
    error: Option<serde::__private::de::Content<'de>>,
    arguments: Option<serde::__private::de::Content<'de>>,
    applicant: Option<serde::__private::de::Content<'de>>,
    binder: Option<serde::__private::de::Content<'de>>,
    variables: Option<Vec<Str>>,
    object: Option<serde::__private::de::Content<'de>>,
}

struct OMVisitor<
    'de,
    's,
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
    const ALLOW_FOREIGN: bool,
>(Either<Str, &'s str>, PhantomData<&'de (OMD, Arr)>);
impl<
    'de,
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>,
    const ALLOW_FOREIGN: bool,
> OMVisitor<'de, '_, Arr, Str, OMD, ALLOW_FOREIGN>
{
    fn visit_seq_omi<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<crate::Int<'de>>()? else {
            return Err(A::Error::custom("missing value in OMI"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OpenMath::OMI(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omf<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<f64>()? else {
            return Err(A::Error::custom("missing value in OMF"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OpenMath::OMF(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omstr<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<Str>()? else {
            return Err(A::Error::custom("missing value in OMSTR"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OpenMath::OMSTR(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omb<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<Arr>()? else {
            return Err(A::Error::custom("missing value in OMB"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OpenMath::OMB(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omv<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<Str>()? else {
            return Err(A::Error::custom("missing value in OMV"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OpenMath::OMV(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_oms<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing cd in OMS"));
        };
        let Some(cd) = seq.next_element::<Str>()? else {
            return Err(A::Error::custom("missing cd in OMS"));
        };
        let Some(name) = seq.next_element::<Str>()? else {
            return Err(A::Error::custom("missing name in OMS"));
        };
        let cdbase: &str = cdbase.unwrap_or(&self.0);
        //cdbase.as_ref().map_or::<&str, _>(&self.0, |s| s.as_ref());

        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OpenMath::OMS { cd_name: cd, name }, cdbase).map_err(A::Error::custom)
    }

    fn visit_seq_ome<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing error in OME"));
        };
        let cdbase_i = cdbase.unwrap_or(&self.0);

        let Some(OMS {
            cdbase,
            cd: cd_name,
            name,
            ..
        }) = seq.next_element()?
        else {
            return Err(A::Error::custom("missing error in OME"));
        };
        let args = seq
            .next_element_seed(OMForeignSeq(cdbase_i, PhantomData))?
            .unwrap_or_default();
        //cdbase.as_ref().map_or::<&str, _>(&self.0, |s| s.as_ref());

        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(
            OpenMath::OME {
                cd_base: cdbase,
                cd_name,
                name,
                args,
            },
            cdbase_i,
        )
        .map_err(A::Error::custom)
    }

    fn visit_seq_oma<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing applicant in OMA"));
        };
        let cdbase = cdbase.unwrap_or(&self.0);

        let Some(head) = seq.next_element_seed(OMDeInner::<'de, '_, OMD, Arr, Str>(
            Right(cdbase),
            PhantomData,
        ))?
        else {
            return Err(A::Error::custom("missing applicant in OMA"));
        };

        let args = seq
            .next_element_seed(OMSeq(cdbase, PhantomData))?
            .unwrap_or_default();

        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(
            OpenMath::OMA {
                head: head.0.map_right(Box::new),
                args,
            },
            cdbase,
        )
        .map_err(A::Error::custom)
    }

    fn visit_seq_ombind<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing applicant in OMA"));
        };
        let cdbase = cdbase.unwrap_or(&self.0);

        let Some(head) = seq.next_element_seed(OMDeInner::<'de, '_, OMD, Arr, Str>(
            Right(cdbase),
            PhantomData,
        ))?
        else {
            return Err(A::Error::custom("missing binder in OMBIND"));
        };

        let Some(context) = seq.next_element()? else {
            return Err(A::Error::custom("missing variables in OMBIND"));
        };

        let Some(body) = seq.next_element_seed(OMDeInner::<'de, '_, OMD, Arr, Str>(
            Right(cdbase),
            PhantomData,
        ))?
        else {
            return Err(A::Error::custom("missing object in OMBIND"));
        };

        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(
            OpenMath::OMBIND {
                head: head.0.map_right(Box::new),
                context,
                body: body.0.map_right(Box::new),
            },
            cdbase,
        )
        .map_err(A::Error::custom)
    }

    fn visit_seq_omforeign<A>(mut seq: A) -> Result<super::OMForeign<'de, OMD, Arr, Str>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let _id = seq.next_element::<Option<&'de str>>()?.unwrap_or_default();
        let Some(foreign) = seq.next_element::<Str>()? else {
            return Err(A::Error::custom("missing foreign in OMFOREIGN"));
        };
        let encoding = seq.next_element::<Option<Str>>()?.unwrap_or_default();
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        Ok(OMForeign::Foreign {
            encoding,
            value: foreign,
        })
    }

    // ---------------------------------------------------------------

    fn visit_map_omi<A>(
        self,
        _id: Option<&'de str>,
        mut integer: Option<i64>,
        mut decimal: Option<&'de str>,
        mut hexadecimal: Option<&'de str>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::integer => integer = Some(map.next_value()?),
                AllFields::decimal => decimal = Some(map.next_value()?),
                AllFields::hexadecimal => hexadecimal = Some(map.next_value()?),
                k => return Err(A::Error::custom(format_args!("Invalid keys for OMI: {k}"))),
            }
        }
        if let Some(i) = integer {
            if decimal.is_some() || hexadecimal.is_some() {
                return Err(A::Error::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            return OMD::from_openmath(OpenMath::OMI(i.into()), &self.0).map_err(A::Error::custom);
        }
        if let Some(d) = decimal {
            if hexadecimal.is_some() {
                return Err(A::Error::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            return OMD::from_openmath(
                OpenMath::OMI(
                    d.into_int()
                        .ok_or_else(|| A::Error::custom("invalid decimal number"))?,
                ),
                &self.0,
            )
            .map_err(A::Error::custom);
        }
        if let Some(h) = hexadecimal {
            return Err(A::Error::custom(format_args!(
                "Not yet implemented: hexadecimal in OMI: {h}"
            )));
        }
        Err(A::Error::custom("Missing value for OMI"))
    }

    fn visit_map_omf<A>(
        self,
        _id: Option<&'de str>,
        mut float: Option<f64>,
        mut decimal: Option<&'de str>,
        mut hexadecimal: Option<&'de str>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::float => float = Some(map.next_value()?),
                AllFields::decimal => decimal = Some(map.next_value()?),
                AllFields::hexadecimal => hexadecimal = Some(map.next_value()?),
                k => return Err(A::Error::custom(format_args!("Invalid keys for OMF: {k}"))),
            }
        }
        if let Some(i) = float {
            if decimal.is_some() || hexadecimal.is_some() {
                return Err(A::Error::custom(
                    "OMF can not have more than one of the fields `float`, `decimal`, `hexadecimal`",
                ));
            }
            return OMD::from_openmath(OpenMath::OMF(i), &self.0).map_err(A::Error::custom);
        }
        if let Some(d) = decimal {
            if hexadecimal.is_some() {
                return Err(A::Error::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            return OMD::from_openmath(
                OpenMath::OMF(
                    d.parse().map_err(|e| {
                        A::Error::custom(format_args!("invalid decimal number: {e}"))
                    })?,
                ),
                &self.0,
            )
            .map_err(A::Error::custom);
        }
        if let Some(h) = hexadecimal {
            return Err(A::Error::custom(format_args!(
                "Not yet implemented: hexadecimal in OMF: {h}"
            )));
        }
        Err(A::Error::custom("Missing value for OMF"))
    }

    fn visit_map_omstr<A>(
        self,
        _id: Option<&'de str>,
        mut string: Option<Str>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::string => string = Some(map.next_value()?),
                k => {
                    return Err(A::Error::custom(format_args!(
                        "Invalid keys for OMSTR: {k}"
                    )));
                }
            }
        }
        if let Some(s) = string {
            return OMD::from_openmath(OpenMath::OMSTR(s), &self.0).map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OMSTR"))
    }

    fn visit_map_omb<A>(
        self,
        _id: Option<&'de str>,
        mut bytes: Option<Arr>,
        mut base64: Option<&'de str>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use crate::base64::Base64Decodable;
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::bytes => bytes = Some(map.next_value()?),
                AllFields::base64 => base64 = Some(map.next_value()?),
                k => {
                    return Err(A::Error::custom(format_args!("Invalid keys for OMB: {k}")));
                }
            }
        }
        let arr = if let Some(bytes) = bytes {
            if base64.is_some() {
                return Err(A::Error::custom(
                    "OMB can not have more than one of the fields `bytes`, `base64`",
                ));
            }
            bytes
        } else if let Some(base64) = base64 {
            base64
                .as_bytes()
                .iter()
                .copied()
                .decode_base64()
                .flat()
                .collect::<Result<Vec<_>, _>>()
                .map_err(A::Error::custom)?
                .into()
        } else {
            return Err(A::Error::custom("Missing value for OMB"));
        };
        OMD::from_openmath(OpenMath::OMB(arr), &self.0).map_err(A::Error::custom)
    }

    fn visit_map_omv<A>(
        self,
        _id: Option<&'de str>,
        mut name: Option<Str>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::name => name = Some(map.next_value()?),
                k => {
                    return Err(A::Error::custom(format_args!("Invalid keys for OMV: {k}")));
                }
            }
        }
        if let Some(name) = name {
            return OMD::from_openmath(OpenMath::OMV(name), &self.0).map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OMV"))
    }

    fn visit_map_oms<A>(
        self,
        _id: Option<&'de str>,
        mut cdbase: Option<&'de str>,
        mut cd: Option<Str>,
        mut name: Option<Str>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::cdbase => cdbase = Some(map.next_value()?),
                AllFields::cd => cd = Some(map.next_value()?),
                AllFields::name => name = Some(map.next_value()?),
                k => {
                    return Err(A::Error::custom(format_args!("Invalid keys for OMS: {k}")));
                }
            }
        }
        let Some(cd) = cd else {
            return Err(A::Error::custom("Missing cd for OMS"));
        };
        let Some(name) = name else {
            return Err(A::Error::custom("Missing name for OMS"));
        };
        let cdbase = cdbase.unwrap_or(&self.0);
        OMD::from_openmath(OpenMath::OMS { cd_name: cd, name }, cdbase).map_err(A::Error::custom)
    }

    fn visit_map_ome<A>(
        self,
        _id: Option<&'de str>,
        mut cdbase: Option<&'de str>,
        error: Option<serde::__private::de::Content<'de>>,
        arguments: Option<serde::__private::de::Content<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut error = if let Some(error) = error {
            Some(OMS::deserialize(
                serde::__private::de::ContentDeserializer::new(error),
            )?)
        } else {
            None
        };
        let mut arguments = if let Some(arguments) = arguments {
            Some(
                OMForeignSeq(cdbase.unwrap_or(&self.0), PhantomData)
                    .deserialize(serde::__private::de::ContentDeserializer::new(arguments))?,
            )
        } else {
            None
        };
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::cdbase => cdbase = Some(map.next_value()?),
                AllFields::error => error = Some(map.next_value()?),
                AllFields::arguments => {
                    arguments = Some(
                        map.next_value_seed(OMForeignSeq(cdbase.unwrap_or(&self.0), PhantomData))?,
                    );
                }
                k => {
                    return Err(A::Error::custom(format_args!("Invalid keys for OME: {k}")));
                }
            }
        }
        if let Some(OMS {
            cdbase, cd, name, ..
        }) = error
        {
            return OMD::from_openmath(
                OpenMath::OME {
                    cd_base: cdbase,
                    cd_name: cd,
                    name,
                    args: arguments.unwrap_or_default(),
                },
                &self.0,
            )
            .map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OME"))
    }

    fn visit_map_oma<A>(
        self,
        _id: Option<&'de str>,
        mut cdbase: Option<&'de str>,
        applicant: Option<serde::__private::de::Content<'de>>,
        arguments: Option<serde::__private::de::Content<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut applicant = if let Some(applicant) = applicant {
            Some(
                OMDeInner(Right(cdbase.unwrap_or(&self.0)), PhantomData)
                    .deserialize(serde::__private::de::ContentDeserializer::new(applicant))?,
            )
        } else {
            None
        };
        let mut arguments = if let Some(arguments) = arguments {
            Some(
                OMSeq(cdbase.unwrap_or(&self.0), PhantomData)
                    .deserialize(serde::__private::de::ContentDeserializer::new(arguments))?,
            )
        } else {
            None
        };
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::cdbase => cdbase = Some(map.next_value()?),
                AllFields::applicant => {
                    applicant = Some(map.next_value_seed(OMDeInner(
                        Right(cdbase.unwrap_or(&self.0)),
                        PhantomData,
                    ))?);
                }
                AllFields::arguments => {
                    arguments =
                        Some(map.next_value_seed(OMSeq(cdbase.unwrap_or(&self.0), PhantomData))?);
                }
                k => {
                    return Err(A::Error::custom(format_args!("Invalid keys for OMA: {k}")));
                }
            }
        }
        if let Some(head) = applicant {
            return OMD::from_openmath(
                OpenMath::OMA {
                    head: head.0.map_right(Box::new),
                    args: arguments.unwrap_or_default(),
                },
                cdbase.unwrap_or(&self.0),
            )
            .map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OMA"))
    }

    fn visit_map_ombind<A>(
        self,
        _id: Option<&'de str>,
        mut cdbase: Option<&'de str>,
        binder: Option<serde::__private::de::Content<'de>>,
        mut variables: Option<Vec<Str>>,
        object: Option<serde::__private::de::Content<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut binder = if let Some(binder) = binder {
            Some(
                OMDeInner(Right(cdbase.unwrap_or(&self.0)), PhantomData)
                    .deserialize(serde::__private::de::ContentDeserializer::new(binder))?,
            )
        } else {
            None
        };
        let mut object = if let Some(object) = object {
            Some(
                OMDeInner(Right(cdbase.unwrap_or(&self.0)), PhantomData)
                    .deserialize(serde::__private::de::ContentDeserializer::new(object))?,
            )
        } else {
            None
        };
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::cdbase => cdbase = Some(map.next_value()?),
                AllFields::binder => {
                    binder = Some(map.next_value_seed(OMDeInner(
                        Right(cdbase.unwrap_or(&self.0)),
                        PhantomData,
                    ))?);
                }
                AllFields::object => {
                    object = Some(map.next_value_seed(OMDeInner(
                        Right(cdbase.unwrap_or(&self.0)),
                        PhantomData,
                    ))?);
                }
                AllFields::variables => variables = Some(map.next_value()?),
                k => {
                    return Err(A::Error::custom(format_args!(
                        "Invalid keys for OMBIND: {k}"
                    )));
                }
            }
        }
        let Some(binder) = binder else {
            return Err(A::Error::custom("Missing binder for OMBIND"));
        };
        let Some(object) = object else {
            return Err(A::Error::custom("Missing object for OMBIND"));
        };
        let Some(variables) = variables else {
            return Err(A::Error::custom("Missing variables for OMBIND"));
        };
        OMD::from_openmath(
            OpenMath::OMBIND {
                head: binder.0.map_right(Box::new),
                context: variables,
                body: object.0.map_right(Box::new),
            },
            cdbase.unwrap_or(&self.0),
        )
        .map_err(A::Error::custom)
    }

    fn visit_map_omforeign<A>(
        _id: Option<&'de str>,
        mut encoding: Option<Str>,
        mut foreign: Option<Str>,
        mut map: A,
    ) -> Result<super::OMForeign<'de, OMD, Arr, Str>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::encoding => encoding = Some(map.next_value()?),
                AllFields::foreign => foreign = Some(map.next_value()?),
                k => {
                    return Err(A::Error::custom(format_args!(
                        "Invalid keys for OMFOREIGN: {k}"
                    )));
                }
            }
        }
        if let Some(foreign) = foreign {
            return Ok(super::OMForeign::Foreign {
                encoding,
                value: foreign,
            });
        }
        Err(A::Error::custom("Missing value for OMFOREIGN"))
    }

    // ---------------------------------------

    fn seq_om<A>(
        self,
        mut seq: A,
        kind: OpenMathKind,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let id = seq.next_element::<Option<&'de str>>()?.unwrap_or_default();
        match kind {
            OpenMathKind::OMI => self.visit_seq_omi(id, seq),
            OpenMathKind::OMF => self.visit_seq_omf(id, seq),
            OpenMathKind::OMSTR => self.visit_seq_omstr(id, seq),
            OpenMathKind::OMB => self.visit_seq_omb(id, seq),
            OpenMathKind::OMV => self.visit_seq_omv(id, seq),
            OpenMathKind::OMS => self.visit_seq_oms(id, seq),
            OpenMathKind::OME => self.visit_seq_ome(id, seq),
            OpenMathKind::OMA => self.visit_seq_oma(id, seq),
            OpenMathKind::OMBIND => self.visit_seq_ombind(id, seq),
            OpenMathKind::OMFOREIGN => {
                Err(A::Error::custom("OMFOREIGN is not allowed as an OMObject"))
            }
            OpenMathKind::OMATTR => todo!(),
            OpenMathKind::OMR => todo!(),
        }
    }

    fn map_state<A>(map: &mut A) -> Result<(OpenMathKind, FieldState<'de, Arr, Str>), A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut state = FieldState::<'de, Arr, Str>::default();
        while let Some(key) = map.next_key()? {
            match key {
                AllFields::kind => return Ok((map.next_value()?, state)),
                AllFields::id => state.id = Some(map.next_value()?),
                AllFields::cdbase => state.cdbase = Some(map.next_value()?),
                AllFields::integer => state.integer = Some(map.next_value()?),
                AllFields::decimal => state.decimal = Some(map.next_value()?),
                AllFields::hexadecimal => state.hexadecimal = Some(map.next_value()?),
                AllFields::float => state.float = Some(map.next_value()?),
                AllFields::string => state.string = Some(map.next_value()?),
                AllFields::bytes => state.bytes = Some(map.next_value()?),
                AllFields::base64 => state.base64 = Some(map.next_value()?),
                AllFields::name => state.name = Some(map.next_value()?),
                AllFields::cd => state.cd = Some(map.next_value()?),
                AllFields::encoding => state.encoding = Some(map.next_value()?),
                AllFields::foreign => state.foreign = Some(map.next_value()?),
                AllFields::error => state.error = Some(map.next_value()?),
                AllFields::arguments => state.arguments = Some(map.next_value()?),
                AllFields::applicant => state.applicant = Some(map.next_value()?),
                AllFields::binder => state.binder = Some(map.next_value()?),
                AllFields::variables => state.variables = Some(map.next_value()?),
                AllFields::object => state.object = Some(map.next_value()?),
                AllFields::__ignore => {
                    map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
        }
        Err(A::Error::custom("missing field \"kind\" in OMObject"))
    }

    #[allow(clippy::cognitive_complexity)]
    #[allow(clippy::too_many_lines)]
    fn om_map<A>(
        self,
        kind: OpenMathKind,
        state: FieldState<'de, Arr, Str>,
        map: A,
    ) -> Result<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;

        macro_rules! ass {
                ($is:ident != $($id:ident),*) => {{
                    let mut invalid_fields = Vec::new();
                    $(
                        if state.$id.is_some() { invalid_fields.push(stringify!($id));}
                    )*
                    if !invalid_fields.is_empty() {
                        return Err(A::Error::custom(format_args!("Invalid keys for {}: {invalid_fields:?}",stringify!($is),)))
                    }
                }}
            }
        match kind {
            OpenMathKind::OMI => {
                ass!(
                    OMI != float,
                    string,
                    bytes,
                    base64,
                    name,
                    cd,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_omi(
                    state.id,
                    state.integer,
                    state.decimal,
                    state.hexadecimal,
                    map,
                )
            }
            OpenMathKind::OMF => {
                ass!(
                    OMF != integer,
                    string,
                    bytes,
                    base64,
                    name,
                    cd,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_omf(state.id, state.float, state.decimal, state.hexadecimal, map)
            }
            OpenMathKind::OMSTR => {
                ass!(
                    OMSTR != integer,
                    float,
                    decimal,
                    hexadecimal,
                    bytes,
                    base64,
                    name,
                    cd,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_omstr(state.id, state.string, map)
            }
            OpenMathKind::OMB => {
                ass!(
                    OMB != integer,
                    float,
                    string,
                    decimal,
                    hexadecimal,
                    name,
                    cd,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_omb(state.id, state.bytes, state.base64, map)
            }
            OpenMathKind::OMV => {
                ass!(
                    OMV != integer,
                    string,
                    float,
                    decimal,
                    hexadecimal,
                    bytes,
                    base64,
                    cd,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_omv(state.id, state.name, map)
            }
            OpenMathKind::OMS => {
                ass!(
                    OMS != integer,
                    float,
                    string,
                    decimal,
                    hexadecimal,
                    bytes,
                    base64,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_oms(state.id, state.cdbase, state.cd, state.name, map)
            }
            OpenMathKind::OME => {
                ass!(
                    OME != integer,
                    float,
                    string,
                    decimal,
                    hexadecimal,
                    bytes,
                    base64,
                    name,
                    cd,
                    encoding,
                    foreign,
                    applicant,
                    binder,
                    variables,
                    object
                );
                self.visit_map_ome(state.id, state.cdbase, state.error, state.arguments, map)
            }
            OpenMathKind::OMA => {
                ass!(
                    OMA != integer,
                    string,
                    float,
                    decimal,
                    hexadecimal,
                    bytes,
                    base64,
                    name,
                    cd,
                    encoding,
                    foreign,
                    error,
                    binder,
                    variables,
                    object
                );
                self.visit_map_oma(
                    state.id,
                    state.cdbase,
                    state.applicant,
                    state.arguments,
                    map,
                )
            }
            OpenMathKind::OMBIND => {
                ass!(
                    OMBIND != integer,
                    float,
                    string,
                    decimal,
                    hexadecimal,
                    bytes,
                    base64,
                    name,
                    cd,
                    encoding,
                    foreign,
                    error,
                    arguments,
                    applicant
                );
                self.visit_map_ombind(
                    state.id,
                    state.cdbase,
                    state.binder,
                    state.variables,
                    state.object,
                    map,
                )
            }
            OpenMathKind::OMFOREIGN => {
                Err(A::Error::custom("OMFOREIGN is not allowed as an OMObject"))
            }
            OpenMathKind::OMATTR => todo!(),
            OpenMathKind::OMR => todo!(),
        }
    }
}

impl<'de, Arr: super::Bytes<'de>, Str: super::StringLike<'de>, OMD: OMDeserializable<'de, Arr, Str>>
    serde::de::Visitor<'de> for OMVisitor<'de, '_, Arr, Str, OMD, false>
{
    type Value = Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct OMObject")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(kind) = seq.next_element::<OpenMathKind>()? else {
            return Err(A::Error::custom("missing kind in OpenMath object"));
        };
        self.seq_om(seq, kind)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let (kind, state) = Self::map_state(&mut map)?;
        self.om_map(kind, state, map)
    }
}

impl<'de, Arr: super::Bytes<'de>, Str: super::StringLike<'de>, OMD: OMDeserializable<'de, Arr, Str>>
    serde::de::Visitor<'de> for OMVisitor<'de, '_, Arr, Str, OMD, true>
{
    type Value = Either<OMD, super::OMForeign<'de, OMD, Arr, Str>>;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct OMObject")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(kind) = seq.next_element::<OpenMathKind>()? else {
            return Err(A::Error::custom("missing kind in OpenMath object"));
        };
        if kind == OpenMathKind::OMFOREIGN {
            return Self::visit_seq_omforeign(seq).map(Either::Right);
        }
        self.seq_om(seq, kind)
            .map(|e| e.map_right(super::OMForeign::OM))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let (kind, state) = Self::map_state(&mut map)?;
        if kind == OpenMathKind::OMFOREIGN {
            macro_rules! ass {
                    ($is:ident != $($id:ident),*) => {{
                        let mut invalid_fields = Vec::new();
                        $(
                            if state.$id.is_some() { invalid_fields.push(stringify!($id));}
                        )*
                        if !invalid_fields.is_empty() {
                            return Err(A::Error::custom(format_args!("Invalid keys for {}: {invalid_fields:?}",stringify!($is),)))
                        }
                    }}
                }
            ass!(
                OMFOREIGN != float,
                integer,
                decimal,
                hexadecimal,
                string,
                bytes,
                base64,
                name,
                cd,
                error,
                arguments,
                applicant,
                binder,
                variables,
                object
            );
            return Self::visit_map_omforeign(state.id, state.encoding, state.foreign, map)
                .map(Either::Right);
        }
        self.om_map(kind, state, map)
            .map(|e| e.map_right(super::OMForeign::OM))
    }
}

impl<'de> serde::Deserialize<'de> for AllFields {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(AllFieldsVisitor)
    }
}

struct AllFieldsVisitor;
impl serde::de::Visitor<'_> for AllFieldsVisitor {
    type Value = AllFields;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("field identifier")
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // only allowed, if 0
        if v == 0 {
            Ok(AllFields::kind)
        } else if v == 1 {
            Ok(AllFields::id)
        } else {
            Err(E::custom(
                "first numerical identifier must be `kind`==0 or `id`==1",
            ))
        }
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(match v {
            "kind" => AllFields::kind,
            "id" => AllFields::id,
            "cdbase" => AllFields::cdbase,
            "integer" => AllFields::integer,
            "decimal" => AllFields::decimal,
            "hexadecimal" => AllFields::hexadecimal,
            "float" => AllFields::float,
            "string" => AllFields::string,
            "bytes" => AllFields::bytes,
            "base64" => AllFields::base64,
            "name" => AllFields::name,
            "cd" => AllFields::cd,
            "encoding" => AllFields::encoding,
            "value" => AllFields::foreign,
            "error" => AllFields::error,
            "arguments" => AllFields::arguments,
            "applicant" => AllFields::applicant,
            "binder" => AllFields::binder,
            "variables" => AllFields::variables,
            "object" => AllFields::object,
            _ => AllFields::__ignore,
        })
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(match v {
            b"kind" => AllFields::kind,
            b"id" => AllFields::id,
            b"cdbase" => AllFields::cdbase,
            b"integer" => AllFields::integer,
            b"decimal" => AllFields::decimal,
            b"hexadecimal" => AllFields::hexadecimal,
            b"float" => AllFields::float,
            b"string" => AllFields::string,
            b"bytes" => AllFields::bytes,
            b"base64" => AllFields::base64,
            b"name" => AllFields::name,
            b"cd" => AllFields::cd,
            b"encoding" => AllFields::encoding,
            b"value" => AllFields::foreign,
            b"error" => AllFields::error,
            b"arguments" => AllFields::arguments,
            b"applicant" => AllFields::applicant,
            b"binder" => AllFields::binder,
            b"variables" => AllFields::variables,
            b"object" => AllFields::object,
            _ => AllFields::__ignore,
        })
    }
}

// ------------------------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct OMS<'s, Str: super::StringLike<'s>> {
    #[serde(skip)]
    __phantom: PhantomData<&'s ()>,

    #[serde(default = "Option::default")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>, 'de:'s, 's:'de"))]
    #[allow(dead_code)]
    id: Option<Str>,

    #[serde(default = "Option::default")]
    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cdbase: Option<Str>,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    cd: Str,

    #[serde(bound(deserialize = "Str: super::StringLike<'s>"))]
    name: Str,
}

#[impl_tools::autoimpl(Clone, Copy)]
struct OMSeq<'de, 's, OMD, Arr, Str>(&'s str, PhantomData<(&'de (), Arr, Str, OMD)>)
//()
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;
impl<'de, OMD, Arr, Str> serde::de::DeserializeSeed<'de> for OMSeq<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    type Value = Vec<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>>;
    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}
impl<'de, OMD, Arr, Str> serde::de::Visitor<'de> for OMSeq<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    type Value = Vec<Either<OMD, super::OpenMath<'de, OMD, Arr, Str>>>;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an optional argument list")
    }
    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Vec::new())
    }
    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(e) = seq.next_element_seed(OMDeInner(Right(self.0), PhantomData))? {
            vec.push(e.0);
        }
        Ok(vec)
    }
}

#[impl_tools::autoimpl(Clone, Copy)]
struct OMForeignSeq<'de, 's, OMD, Arr, Str>(&'s str, PhantomData<(&'de (), Arr, Str, OMD)>)
//()
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;
impl<'de, OMD, Arr, Str> serde::de::DeserializeSeed<'de> for OMForeignSeq<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    type Value = Vec<Either<OMD, super::OMForeign<'de, OMD, Arr, Str>>>;
    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}
impl<'de, OMD, Arr, Str> serde::de::Visitor<'de> for OMForeignSeq<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    type Value = Vec<Either<OMD, super::OMForeign<'de, OMD, Arr, Str>>>;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an optional argument list")
    }
    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Vec::new())
    }
    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(e) = seq.next_element_seed(OMDeForeign(self.0, PhantomData))? {
            vec.push(e);
        }
        Ok(vec)
    }
}

#[impl_tools::autoimpl(Clone)]
struct OMDeForeign<'de, 's, OMD, Arr, Str>(&'s str, PhantomData<(&'de (), Arr, Str, OMD)>)
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str>;

impl<'de, OMD, Arr, Str> serde::de::DeserializeSeed<'de> for OMDeForeign<'de, '_, OMD, Arr, Str>
where
    Arr: super::Bytes<'de>,
    Str: super::StringLike<'de>,
    OMD: OMDeserializable<'de, Arr, Str> + 'de,
{
    type Value = Either<OMD, super::OMForeign<'de, OMD, Arr, Str>>; //e<'de, OMD, Arr, Str>, (Option<Str>, Str)>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "OMObject",
            &ALL_FIELDS,
            OMVisitor::<Arr, Str, OMD, true>(Right(self.0), PhantomData),
        )
    }
}
