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
//! let int_value = wrapper.into_inner();
//! # }
//! ```
#![allow(clippy::trait_duplication_in_bounds)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::type_complexity)]

use either::Either::{Left, Right};
use serde::{Deserialize, de::DeserializeSeed};
use serde_cow::{CowBytes, CowStr};

use super::OM;
use crate::{OMDeserializable, OMKind, de::OMForeign, either::Either};
use std::{borrow::Cow, marker::PhantomData};

impl<'de, O: OMDeserializable<'de> + 'de> serde::Deserialize<'de> for super::OMObject<'de, O> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, O: OMDeserializable<'de>>(PhantomData<&'de O>);
        impl<'de, O: OMDeserializable<'de>> serde::de::Visitor<'de> for Visitor<'de, O> {
            type Value = super::OMObject<'de, O>;
            #[inline]
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an OMOBJ struct")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error;
                let Some("OMOBJ") = seq.next_element()? else {
                    return Err(A::Error::custom("missing kind=\"OMOBJ\""));
                };
                let _ = seq.next_element::<serde::de::IgnoredAny>()?;
                let Some(o) = seq.next_element::<OMFromSerde<O>>()? else {
                    return Err(A::Error::custom("missing object"));
                };
                Ok(super::OMObject(o.into_inner(), PhantomData))
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de::Error;

                #[derive(serde::Deserialize)]
                #[allow(non_camel_case_types)]
                enum Fields {
                    kind,
                    openmath,
                    cdbase,
                    object,
                }
                let mut obj = None;
                let mut cdbase = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::kind => {
                            if map.next_value::<&str>()? != "OMOBJ" {
                                return Err(A::Error::custom("invalid kind"));
                            }
                        }
                        Fields::openmath => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                        Fields::cdbase => {
                            cdbase = Some(map.next_value()?);
                        }
                        Fields::object if cdbase.is_some() => {
                            let cdbase = unsafe { cdbase.take().unwrap_unchecked() };
                            obj = Some(
                                match map.next_value_seed(OMDeInner(cdbase, PhantomData))?.0 {
                                    Left(o) => o,
                                    Right(e) => {
                                        return Err(A::Error::custom(format!(
                                            "OpenMath object does not represent a valid instance of {}: {e:?}",
                                            std::any::type_name::<O>(),
                                        )));
                                    }
                                },
                            );
                        }
                        Fields::object => {
                            obj = Some(map.next_value::<OMFromSerde<_>>()?.0);
                        }
                    }
                }
                let Some(obj) = obj else {
                    return Err(A::Error::custom("missing object field"));
                };
                Ok(super::OMObject(obj, PhantomData))
            }
        }
        deserializer.deserialize_struct(
            "OMObject",
            &["kind", "openmath", "cdbase", "object"],
            Visitor(PhantomData),
        )
    }
}

/// Wrapper type for deserializing OpenMath objects via serde.
///
/// This type wraps any `OMDeserializable` type and provides a `serde::Deserialize`
/// implementation that can parse OpenMath objects from serde-compatible formats.
///
/// # Type Parameters
/// - `'de`: Lifetime of the deserialized data
/// - `OMD`: The target type that implements `OMDeserializable`
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
/// let int_value = wrapper.into_inner();
/// assert_eq!(int_value.is_i128(), Some(42));
/// # }
/// ```
pub struct OMFromSerde<OMD>(OMD);

impl<OMD> OMFromSerde<OMD> {
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
    /// let value = wrapper.into_inner();
    /// assert_eq!(value.is_i128(), Some(123));
    /// # }
    /// ```
    #[inline]
    pub fn into_inner(self) -> OMD {
        self.0
    }
}

impl<'de, OMD> serde::Deserialize<'de> for OMFromSerde<OMD>
where
    OMD: OMDeserializable<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        match OMDe::<'de, OMD>::deserialize(deserializer)?.0 {
            Left(o) => Ok(Self(o)),
            Right(e) => Err(D::Error::custom(format!(
                "OpenMath object does not represent a valid instance of {}: {e:?}",
                std::any::type_name::<OMD>(),
            ))),
        }
    }
}

struct OMDe<'de, OMD>(Either<OMD, super::OM<'de, OMD>>, PhantomData<&'de ()>)
where
    OMD: OMDeserializable<'de>;

impl<'de, OMD> serde::Deserialize<'de> for OMDe<'de, OMD>
where
    OMD: OMDeserializable<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        OMDeInner(
            Cow::Borrowed(crate::OPENMATH_BASE_URI.as_str()),
            PhantomData,
        )
        .deserialize(deserializer)
    }
}

#[impl_tools::autoimpl(Clone)]
struct OMDeInner<'de, 's, OMD>(Cow<'s, str>, PhantomData<(&'de (), OMD)>)
where
    OMD: OMDeserializable<'de>;

impl<'de, OMD> serde::de::DeserializeSeed<'de> for OMDeInner<'de, '_, OMD>
where
    OMD: OMDeserializable<'de>,
{
    type Value = OMDe<'de, OMD>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_struct(
                "OMObject",
                &ALL_FIELDS,
                OMVisitor::<OMD, false>(self.0, PhantomData),
            )
            .map(|r| OMDe(r, PhantomData))
    }
}

// -------------------------------------------------------------------------------------

macro_rules! all_fields {
    ($($name:ident),* $(,)?) => {
        #[allow(non_camel_case_types)]
        enum AllFields {
            $($name),*,__ignore
        }
        impl std::fmt::Display for AllFields {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$name => f.write_str(stringify!($name))),*,
                    Self::__ignore => f.write_str("__ignore")
                }
            }
        }
        static ALL_FIELDS: [&str;20] = [$(stringify!($name)),*];
    }
}

all_fields! {
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
    object
}

#[impl_tools::autoimpl(Default)]
struct FieldState<'de> {
    id: Option<CowStr<'de>>,
    integer: Option<i64>,
    decimal: Option<CowStr<'de>>,
    hexadecimal: Option<CowStr<'de>>,
    float: Option<f64>,
    string: Option<CowStr<'de>>,
    bytes: Option<CowBytes<'de>>,
    base64: Option<CowStr<'de>>,
    name: Option<CowStr<'de>>,
    cdbase: Option<CowStr<'de>>,
    cd: Option<CowStr<'de>>,
    encoding: Option<CowStr<'de>>,
    foreign: Option<CowStr<'de>>,
    error: Option<serde::__private::de::Content<'de>>,
    arguments: Option<serde::__private::de::Content<'de>>,
    applicant: Option<serde::__private::de::Content<'de>>,
    binder: Option<serde::__private::de::Content<'de>>,
    variables: Option<Vec<CowStr<'de>>>,
    object: Option<serde::__private::de::Content<'de>>,
}

struct OMVisitor<'de, 's, OMD: OMDeserializable<'de>, const ALLOW_FOREIGN: bool>(
    Cow<'s, str>,
    PhantomData<(&'de (), OMD)>,
);
impl<'de, OMD: OMDeserializable<'de>, const ALLOW_FOREIGN: bool>
    OMVisitor<'de, '_, OMD, ALLOW_FOREIGN>
{
    fn visit_seq_omi<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<crate::Int<'de>>()? else {
            return Err(A::Error::custom("missing value in OMI"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OM::OMI(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omf<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<f64>()? else {
            return Err(A::Error::custom("missing value in OMF"));
        };
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OM::OMF(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omstr<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<CowStr<'de>>()? else {
            return Err(A::Error::custom("missing value in OMSTR"));
        };
        let v = v.0;
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OM::OMSTR(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omb<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<CowBytes<'de>>()? else {
            return Err(A::Error::custom("missing value in OMB"));
        };
        let v = v.0;
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OM::OMB(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_omv<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(v) = seq.next_element::<CowStr<'de>>()? else {
            return Err(A::Error::custom("missing value in OMV"));
        };
        let v = v.0;
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OM::OMV(v), &self.0).map_err(A::Error::custom)
    }

    fn visit_seq_oms<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing cd in OMS"));
        };
        let Some(cd) = seq.next_element::<CowStr<'de>>()? else {
            return Err(A::Error::custom("missing cd in OMS"));
        };
        let cd = cd.0;
        let Some(name) = seq.next_element::<CowStr<'de>>()? else {
            return Err(A::Error::custom("missing name in OMS"));
        };
        let name = name.0;
        let cdbase: &str = cdbase.unwrap_or(&self.0);
        //cdbase.as_ref().map_or::<&str, _>(&self.0, |s| s.as_ref());

        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(OM::OMS { cd_name: cd, name }, cdbase).map_err(A::Error::custom)
    }

    fn visit_seq_ome<A>(
        self,
        _id: Option<&'de str>,
        mut seq: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            OM::OME {
                cd_base: cdbase.map(|e| e.0),
                cd_name: cd_name.0,
                name: name.0,
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
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing applicant in OMA"));
        };
        let cdbase = cdbase.unwrap_or(&self.0);

        let Some(head) = seq.next_element_seed(OMDeInner::<'de, '_, OMD>(
            Cow::Borrowed(cdbase),
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
            OM::OMA {
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
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        let Some(cdbase) = seq.next_element::<Option<&'de str>>()? else {
            return Err(A::Error::custom("missing applicant in OMA"));
        };
        let cdbase = cdbase.unwrap_or(&self.0);

        let Some(head) = seq.next_element_seed(OMDeInner::<'de, '_, OMD>(
            Cow::Borrowed(cdbase),
            PhantomData,
        ))?
        else {
            return Err(A::Error::custom("missing binder in OMBIND"));
        };

        let Some(context) = seq.next_element()? else {
            return Err(A::Error::custom("missing variables in OMBIND"));
        };

        let Some(body) = seq.next_element_seed(OMDeInner::<'de, '_, OMD>(
            Cow::Borrowed(cdbase),
            PhantomData,
        ))?
        else {
            return Err(A::Error::custom("missing object in OMBIND"));
        };

        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        OMD::from_openmath(
            OM::OMBIND {
                head: head.0.map_right(Box::new),
                context,
                body: body.0.map_right(Box::new),
            },
            cdbase,
        )
        .map_err(A::Error::custom)
    }

    fn visit_seq_omforeign<A>(mut seq: A) -> Result<super::OMForeign<'de, OMD>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let _id = seq.next_element::<Option<&'de str>>()?.unwrap_or_default();
        let Some(foreign) = seq.next_element::<CowStr<'de>>()? else {
            return Err(A::Error::custom("missing foreign in OMFOREIGN"));
        };
        let foreign = foreign.0;
        let encoding = seq
            .next_element::<Option<CowStr<'de>>>()?
            .unwrap_or_default()
            .map(|e| e.0);
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        Ok(OMForeign::Foreign {
            encoding,
            value: foreign,
        })
    }

    // ---------------------------------------------------------------

    fn visit_map_omi<A>(
        self,
        _id: Option<&str>,
        mut integer: Option<i64>,
        mut decimal: Option<CowStr<'de>>,
        mut hexadecimal: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            return OMD::from_openmath(OM::OMI(i.into()), &self.0).map_err(A::Error::custom);
        }
        if let Some(d) = decimal {
            if hexadecimal.is_some() {
                return Err(A::Error::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            return OMD::from_openmath(
                OM::OMI(
                    crate::Int::try_from(d.0)
                        .map_err(|()| A::Error::custom("invalid decimal number"))?,
                ),
                &self.0,
            )
            .map_err(A::Error::custom);
        }
        if let Some(h) = hexadecimal {
            return Err(A::Error::custom(format_args!(
                "Not yet implemented: hexadecimal in OMI: {}",
                h.0
            )));
        }
        Err(A::Error::custom("Missing value for OMI"))
    }

    fn visit_map_omf<A>(
        self,
        _id: Option<&str>,
        mut float: Option<f64>,
        mut decimal: Option<CowStr<'de>>,
        mut hexadecimal: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            return OMD::from_openmath(OM::OMF(i), &self.0).map_err(A::Error::custom);
        }
        if let Some(d) = decimal {
            if hexadecimal.is_some() {
                return Err(A::Error::custom(
                    "OMI can not have more than one of the fields `integer`, `decimal`, `hexadecimal`",
                ));
            }
            return OMD::from_openmath(
                OM::OMF(
                    d.0.parse().map_err(|e| {
                        A::Error::custom(format_args!("invalid decimal number: {e}"))
                    })?,
                ),
                &self.0,
            )
            .map_err(A::Error::custom);
        }
        if let Some(h) = hexadecimal {
            return Err(A::Error::custom(format_args!(
                "Not yet implemented: hexadecimal in OMF: {}",
                h.0
            )));
        }
        Err(A::Error::custom("Missing value for OMF"))
    }

    fn visit_map_omstr<A>(
        self,
        _id: Option<&str>,
        mut string: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            return OMD::from_openmath(OM::OMSTR(s.0), &self.0).map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OMSTR"))
    }

    fn visit_map_omb<A>(
        self,
        _id: Option<&str>,
        mut bytes: Option<CowBytes<'de>>,
        mut base64: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            bytes.0
        } else if let Some(base64) = base64 {
            base64
                .0
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
        OMD::from_openmath(OM::OMB(arr), &self.0).map_err(A::Error::custom)
    }

    fn visit_map_omv<A>(
        self,
        _id: Option<&str>,
        mut name: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            return OMD::from_openmath(OM::OMV(name.0), &self.0).map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OMV"))
    }

    fn visit_map_oms<A>(
        self,
        _id: Option<&str>,
        mut cdbase: Option<CowStr<'de>>,
        mut cd: Option<CowStr<'de>>,
        mut name: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
        let cdbase = cdbase.map(|e| e.0);
        let cdbase = cdbase.as_deref().unwrap_or(&self.0);
        OMD::from_openmath(
            OM::OMS {
                cd_name: cd.0,
                name: name.0,
            },
            cdbase,
        )
        .map_err(A::Error::custom)
    }

    fn visit_map_ome<A>(
        self,
        _id: Option<&str>,
        mut cdbase: Option<CowStr<'de>>,
        error: Option<serde::__private::de::Content<'de>>,
        arguments: Option<serde::__private::de::Content<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
                OMForeignSeq(cdbase.as_ref().map_or(&self.0, |e| &*e.0), PhantomData)
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
                    arguments = Some(map.next_value_seed(OMForeignSeq(
                        cdbase.as_ref().map_or(&self.0, |e| &*e.0),
                        PhantomData,
                    ))?);
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
                OM::OME {
                    cd_base: cdbase.map(|e| e.0),
                    cd_name: cd.0,
                    name: name.0,
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
        _id: Option<&str>,
        mut cdbase: Option<CowStr<'de>>,
        applicant: Option<serde::__private::de::Content<'de>>,
        arguments: Option<serde::__private::de::Content<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut applicant = if let Some(applicant) = applicant {
            Some(
                OMDeInner(
                    Cow::Borrowed(cdbase.as_ref().map_or(&self.0, |e| &*e.0)),
                    PhantomData,
                )
                .deserialize(serde::__private::de::ContentDeserializer::new(applicant))?,
            )
        } else {
            None
        };
        let mut arguments = if let Some(arguments) = arguments {
            Some(
                OMSeq(cdbase.as_ref().map_or(&self.0, |e| &*e.0), PhantomData)
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
                        Cow::Borrowed(cdbase.as_ref().map_or(&self.0, |e| &*e.0)),
                        PhantomData,
                    ))?);
                }
                AllFields::arguments => {
                    arguments = Some(map.next_value_seed(OMSeq(
                        cdbase.as_ref().map_or(&self.0, |e| &*e.0),
                        PhantomData,
                    ))?);
                }
                k => {
                    return Err(A::Error::custom(format_args!("Invalid keys for OMA: {k}")));
                }
            }
        }
        let cdbase = cdbase.map(|e| e.0);
        if let Some(head) = applicant {
            return OMD::from_openmath(
                OM::OMA {
                    head: head.0.map_right(Box::new),
                    args: arguments.unwrap_or_default(),
                },
                cdbase.as_deref().unwrap_or(&self.0),
            )
            .map_err(A::Error::custom);
        }
        Err(A::Error::custom("Missing value for OMA"))
    }

    fn visit_map_ombind<A>(
        self,
        _id: Option<&str>,
        mut cdbase: Option<CowStr<'de>>,
        binder: Option<serde::__private::de::Content<'de>>,
        mut variables: Option<Vec<CowStr<'de>>>,
        object: Option<serde::__private::de::Content<'de>>,
        mut map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut binder = if let Some(binder) = binder {
            Some(
                OMDeInner(
                    Cow::Borrowed(cdbase.as_ref().map_or(&self.0, |e| &*e.0)),
                    PhantomData,
                )
                .deserialize(serde::__private::de::ContentDeserializer::new(binder))?,
            )
        } else {
            None
        };
        let mut object = if let Some(object) = object {
            Some(
                OMDeInner(
                    Cow::Borrowed(cdbase.as_ref().map_or(&self.0, |e| &*e.0)),
                    PhantomData,
                )
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
                        Cow::Borrowed(cdbase.as_ref().map_or(&self.0, |e| &*e.0)),
                        PhantomData,
                    ))?);
                }
                AllFields::object => {
                    object = Some(map.next_value_seed(OMDeInner(
                        Cow::Borrowed(cdbase.as_ref().map_or(&self.0, |e| &*e.0)),
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
        let cdbase = cdbase.map(|e| e.0);
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
            OM::OMBIND {
                head: binder.0.map_right(Box::new),
                context: variables.into_iter().map(|v| v.0).collect(),
                body: object.0.map_right(Box::new),
            },
            cdbase.as_deref().unwrap_or(&self.0),
        )
        .map_err(A::Error::custom)
    }

    fn visit_map_omforeign<A>(
        _id: Option<&str>,
        mut encoding: Option<CowStr<'de>>,
        mut foreign: Option<CowStr<'de>>,
        mut map: A,
    ) -> Result<super::OMForeign<'de, OMD>, A::Error>
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
                encoding: encoding.map(|e| e.0),
                value: foreign.0,
            });
        }
        Err(A::Error::custom("Missing value for OMFOREIGN"))
    }

    // ---------------------------------------

    fn seq_om<A>(
        self,
        mut seq: A,
        kind: OMKind,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let id = seq.next_element::<Option<&'de str>>()?.unwrap_or_default();
        match kind {
            OMKind::OMI => self.visit_seq_omi(id, seq),
            OMKind::OMF => self.visit_seq_omf(id, seq),
            OMKind::OMSTR => self.visit_seq_omstr(id, seq),
            OMKind::OMB => self.visit_seq_omb(id, seq),
            OMKind::OMV => self.visit_seq_omv(id, seq),
            OMKind::OMS => self.visit_seq_oms(id, seq),
            OMKind::OME => self.visit_seq_ome(id, seq),
            OMKind::OMA => self.visit_seq_oma(id, seq),
            OMKind::OMBIND => self.visit_seq_ombind(id, seq),
            OMKind::OMFOREIGN => Err(A::Error::custom("OMFOREIGN is not allowed as an OMObject")),
            OMKind::OMATTR => todo!(),
            OMKind::OMR => todo!(),
        }
    }

    fn map_state<A>(map: &mut A) -> Result<(OMKind, FieldState<'de>), A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut state = FieldState::<'de>::default();
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
        kind: OMKind,
        state: FieldState<'de>,
        map: A,
    ) -> Result<Either<OMD, super::OM<'de, OMD>>, A::Error>
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
            OMKind::OMI => {
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
                    state.id.as_ref().map(|e| &*e.0),
                    state.integer,
                    state.decimal,
                    state.hexadecimal,
                    map,
                )
            }
            OMKind::OMF => {
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
                self.visit_map_omf(
                    state.id.as_ref().map(|e| &*e.0),
                    state.float,
                    state.decimal,
                    state.hexadecimal,
                    map,
                )
            }
            OMKind::OMSTR => {
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
                self.visit_map_omstr(state.id.as_ref().map(|e| &*e.0), state.string, map)
            }
            OMKind::OMB => {
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
                self.visit_map_omb(
                    state.id.as_ref().map(|e| &*e.0),
                    state.bytes,
                    state.base64,
                    map,
                )
            }
            OMKind::OMV => {
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
                self.visit_map_omv(state.id.as_ref().map(|e| &*e.0), state.name, map)
            }
            OMKind::OMS => {
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
                self.visit_map_oms(
                    state.id.as_ref().map(|e| &*e.0),
                    state.cdbase,
                    state.cd,
                    state.name,
                    map,
                )
            }
            OMKind::OME => {
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
                self.visit_map_ome(
                    state.id.as_ref().map(|e| &*e.0),
                    state.cdbase,
                    state.error,
                    state.arguments,
                    map,
                )
            }
            OMKind::OMA => {
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
                    state.id.as_ref().map(|e| &*e.0),
                    state.cdbase,
                    state.applicant,
                    state.arguments,
                    map,
                )
            }
            OMKind::OMBIND => {
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
                    state.id.as_ref().map(|e| &*e.0),
                    state.cdbase,
                    state.binder,
                    state.variables,
                    state.object,
                    map,
                )
            }
            OMKind::OMFOREIGN => Err(A::Error::custom("OMFOREIGN is not allowed as an OMObject")),
            OMKind::OMATTR => todo!(),
            OMKind::OMR => todo!(),
        }
    }
}

impl<'de, OMD: OMDeserializable<'de>> serde::de::Visitor<'de> for OMVisitor<'de, '_, OMD, false> {
    type Value = Either<OMD, super::OM<'de, OMD>>;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct OMObject")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(kind) = seq.next_element::<OMKind>()? else {
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

impl<'de, OMD: OMDeserializable<'de>> serde::de::Visitor<'de> for OMVisitor<'de, '_, OMD, true> {
    type Value = Either<OMD, super::OMForeign<'de, OMD>>;
    #[inline]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct OMObject")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let Some(kind) = seq.next_element::<OMKind>()? else {
            return Err(A::Error::custom("missing kind in OpenMath object"));
        };
        if kind == OMKind::OMFOREIGN {
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
        if kind == OMKind::OMFOREIGN {
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
            return Self::visit_map_omforeign(
                state.id.as_ref().map(|e| &*e.0),
                state.encoding,
                state.foreign,
                map,
            )
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
#[serde(bound = "'s: 'de,'de:'s")]
struct OMS<'s> {
    #[serde(default)]
    #[allow(dead_code)]
    id: Option<CowStr<'s>>,

    #[serde(default)]
    cdbase: Option<CowStr<'s>>,

    cd: CowStr<'s>,

    name: CowStr<'s>,
}

#[impl_tools::autoimpl(Clone, Copy)]
struct OMSeq<'de, 's, OMD>(&'s str, PhantomData<(&'de (), OMD)>)
//()
where
    OMD: OMDeserializable<'de>;
impl<'de, OMD> serde::de::DeserializeSeed<'de> for OMSeq<'de, '_, OMD>
where
    OMD: OMDeserializable<'de>,
{
    type Value = Vec<Either<OMD, super::OM<'de, OMD>>>;
    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}
impl<'de, OMD> serde::de::Visitor<'de> for OMSeq<'de, '_, OMD>
where
    OMD: OMDeserializable<'de>,
{
    type Value = Vec<Either<OMD, super::OM<'de, OMD>>>;
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
        while let Some(e) = seq.next_element_seed(OMDeInner(Cow::Borrowed(self.0), PhantomData))? {
            vec.push(e.0);
        }
        Ok(vec)
    }
}

#[impl_tools::autoimpl(Clone, Copy)]
struct OMForeignSeq<'de, 's, OMD>(&'s str, PhantomData<(&'de (), OMD)>)
//()
where
    OMD: OMDeserializable<'de>;
impl<'de, OMD> serde::de::DeserializeSeed<'de> for OMForeignSeq<'de, '_, OMD>
where
    OMD: OMDeserializable<'de>,
{
    type Value = Vec<Either<OMD, super::OMForeign<'de, OMD>>>;
    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(self)
    }
}
impl<'de, OMD> serde::de::Visitor<'de> for OMForeignSeq<'de, '_, OMD>
where
    OMD: OMDeserializable<'de>,
{
    type Value = Vec<Either<OMD, super::OMForeign<'de, OMD>>>;
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
struct OMDeForeign<'de, 's, OMD>(&'s str, PhantomData<(&'de (), OMD)>)
where
    OMD: OMDeserializable<'de>;

impl<'de, OMD> serde::de::DeserializeSeed<'de> for OMDeForeign<'de, '_, OMD>
where
    OMD: OMDeserializable<'de>,
{
    type Value = Either<OMD, super::OMForeign<'de, OMD>>; //e<'de, OMD, Arr, Str>, (Option<Str>, Str)>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "OMObject",
            &ALL_FIELDS,
            OMVisitor::<OMD, true>(Cow::Borrowed(self.0), PhantomData),
        )
    }
}
