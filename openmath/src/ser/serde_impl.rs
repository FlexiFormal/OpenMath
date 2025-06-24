//! # Serde Integration for OpenMath Serialization
//!
//! This module provides serde integration for OpenMath serialization, allowing
//! OpenMath objects to be serialized to any format supported by serde (JSON,
//! XML, YAML, etc.).
//!
//! ## Usage
//!
//! ```rust
//! # #[cfg(feature = "serde")]
//! # {
//! use openmath::{OMSerializable, Int};
//!
//! let value = Int::from(42);
//! let json = serde_json::to_string(&value.openmath_serde()).unwrap();
//! println!("{}", json); // Outputs OpenMath JSON representation
//! # }
//! ```

use crate::{OMSerializable, ser::OMSerializer};
use serde::{
    Serializer,
    ser::{SerializeSeq, SerializeStructVariant},
};
impl<E: serde::ser::Error> super::Error for E {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn custom(err: impl std::fmt::Display) -> Self {
        serde::ser::Error::custom(err)
    }
}

/// Wrapper type that implements `serde::Serialize` for OpenMath objects.
///
/// This type wraps any `OMSerializable` type and provides a `serde::Serialize`
/// implementation that uses the OpenMath serialization format. It's created
/// automatically when you call [`OMSerializable::openmath_serde`].
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "serde")]
/// # {
/// # use crate::openmath::OMSerializable;
/// use openmath::Int;
///
/// let value = Int::from(123);
/// let serializer = value.openmath_serde();
/// let json = serde_json::to_string(&serializer).unwrap();
/// # }
/// ```
pub struct SerdeSerializer<'s, OM>(
    pub(crate) &'s OM,
    pub(crate) Option<&'s str>,
    pub(crate) Option<&'s str>,
)
where
    OM: crate::OMSerializable + ?Sized;

impl<OM: crate::OMSerializable + ?Sized> ::serde::Serialize for SerdeSerializer<'_, OM> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;
        let serializer = Serder {
            s: serializer,
            next_ns: self.1,
            current_ns: self.2,
        };
        self.0.as_openmath(serializer).map_err(S::Error::custom)
    }
}

/// Internal wrapper that adapts a serde `Serializer` to implement `OMSerializer`.
///
/// This type bridges the gap between serde's serialization model and OpenMath's
/// serialization model, allowing OpenMath objects to be serialized to any
/// serde-compatible format.
struct Serder<'s, S: ::serde::Serializer> {
    s: S,
    next_ns: Option<&'s str>,
    current_ns: Option<&'s str>,
}

impl<'s, S: ::serde::Serializer> OMSerializer<'s> for Serder<'s, S> {
    type Ok = S::Ok;
    type Err = S::Error;
    type SubSerializer<'ns>
        = Serder<'ns, S>
    where
        's: 'ns;

    #[inline]
    fn current_cd_base(&self) -> Option<&str> {
        self.next_ns.or(self.current_ns)
    }

    fn with_cd_base<'ns>(self, cd_base: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
    where
        's: 'ns,
    {
        if self.current_ns == Some(cd_base) {
            Ok(self)
        } else {
            Ok(Serder {
                s: self.s,
                next_ns: Some(cd_base),
                current_ns: self.current_ns,
            })
        }
    }

    #[inline]
    fn omi(self, value: &crate::Int) -> Result<Self::Ok, Self::Err> {
        self.s
            .serialize_newtype_variant("OMObject", 0, "OMI", &value)
    }
    fn omf(self, value: f64) -> Result<Self::Ok, Self::Err> {
        self.s
            .serialize_newtype_variant("OMObject", 1, "OMF", &value)
    }
    fn omstr(self, string: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        self.s
            .serialize_newtype_variant("OMObject", 2, "OMSTR", &DWrap(string))
    }
    fn omb<I: IntoIterator<Item = u8>>(self, bytes: I) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        struct Omb<I: IntoIterator<Item = u8>>(std::cell::Cell<Option<I>>)
        where
            I::IntoIter: ExactSizeIterator;
        impl<I: IntoIterator<Item = u8>> serde::Serialize for Omb<I>
        where
            I::IntoIter: ExactSizeIterator,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                use serde::ser::Error;
                let Some(i) = self.0.take() else {
                    return Err(S::Error::custom("error serializing iterator"));
                };
                let i = i.into_iter();
                let mut s = serializer.serialize_seq(Some(i.len()))?;
                for e in i {
                    s.serialize_element(&e)?;
                }
                s.end()
            }
        }
        self.s.serialize_newtype_variant(
            "OMObject",
            3,
            "OMB",
            &Omb(std::cell::Cell::new(Some(bytes))),
        )
    }
    fn omv(self, name: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        self.s
            .serialize_newtype_variant("OMObject", 4, "OMV", &DWrap(name))
    }
    fn oms(
        self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err> {
        let mut struc = self.s.serialize_struct_variant("OMObject", 5, "OMS", 3)?;
        if let Some(base) = self.next_ns {
            struc.serialize_field("cdbase", base)?;
        }
        struc.serialize_field("cd", &DWrap(cd_name))?;
        struc.serialize_field("name", &DWrap(name))?;
        struc.end()
    }

    fn oma<'a, T: OMSerializable + 'a, I: IntoIterator<Item = &'a T>>(
        self,
        head: &'a impl OMSerializable,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        self.s.serialize_newtype_variant(
            "OMObject",
            6,
            "OMA",
            &Oma {
                current_ns: self.current_ns,
                next_ns: self.next_ns,
                head,
                args: std::cell::Cell::new(Some(args.into_iter())),
            },
        )
    }

    fn ombind<'a, St: std::fmt::Display + 'a, I: IntoIterator<Item = &'a St>>(
        self,
        head: &'a impl OMSerializable,
        vars: I,
        body: &'a impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        //self.s.serialize_struct("OMObject", if self.next_ns.is_some() {4} else {3});
        //TODO cd_base;
        self.s.serialize_newtype_variant(
            "OMObject",
            7,
            "OMBIND",
            &Ombind {
                current_ns: self.current_ns,
                next_ns: self.next_ns,
                head,
                vars: std::cell::Cell::new(Some(vars.into_iter())),
                body,
            },
        )
    }
}

struct Oma<
    's,
    'a: 's,
    T: OMSerializable + 'a,
    H: OMSerializable + 's,
    I: ExactSizeIterator<Item = &'a T>,
> {
    head: &'s H,
    args: std::cell::Cell<Option<I>>,
    current_ns: Option<&'s str>,
    next_ns: Option<&'s str>,
}
impl<'s, 'a: 's, T: OMSerializable + 's, H: OMSerializable + 's, I: ExactSizeIterator<Item = &'a T>>
    serde::Serialize for Oma<'s, 'a, T, H, I>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let Some(args) = self.args.take() else {
            return Err(S::Error::custom("Error iterating over arguments"));
        };
        if let Some(s) = self.next_ns {
            todo!() //WAAAH
        }
        let mut r = serializer.serialize_seq(Some(args.len() + 1))?;
        r.serialize_element(&SerdeSerializer(&self.head, None, self.current_ns))?;
        for s in args {
            r.serialize_element(&SerdeSerializer(s, None, self.current_ns))?;
        }
        r.end()
    }
}

struct DWrap<'d, D: std::fmt::Display>(&'d D);
impl<D: std::fmt::Display> serde::Serialize for DWrap<'_, D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self.0)
    }
}

struct Ombind<
    's,
    'a: 's,
    B: OMSerializable + 's,
    H: OMSerializable + 's,
    St: std::fmt::Display + 'a,
    I: ExactSizeIterator<Item = &'a St>,
> {
    head: &'s H,
    vars: std::cell::Cell<Option<I>>,
    body: &'s B,
    current_ns: Option<&'s str>,
    next_ns: Option<&'s str>,
}
impl<
    's,
    'a: 's,
    B: OMSerializable + 's,
    H: OMSerializable + 's,
    St: std::fmt::Display + 'a,
    I: ExactSizeIterator<Item = &'a St>,
> serde::Serialize for Ombind<'s, 'a, B, H, St, I>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let Some(args) = self.vars.take() else {
            return Err(S::Error::custom("Error iterating over variables"));
        };
        if let Some(s) = self.next_ns {
            todo!() //WAAAH
        }
        let mut r = serializer.serialize_seq(Some(args.len() + 2))?;
        r.serialize_element(&SerdeSerializer(&self.head, None, self.current_ns))?;
        for s in args {
            r.serialize_element(&DWrap(s))?;
        }
        r.serialize_element(&SerdeSerializer(&self.body, None, self.current_ns))?;
        r.end()
    }
}
/*
enum Foo {
    OMS { value: String, foo: Option<String> },
    OMI(i64),
    OMV(String),
}

#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    impl<'de> _serde::Deserialize<'de> for Foo {
        fn deserialize<D>(__deserializer: D) -> _serde::__private::Result<Self, D::Error>
        where
            D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum Field {
                OMS,
                OMI,
                OMV,
            }
            #[doc(hidden)]
            struct FieldVisitor;

            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for FieldVisitor {
                type Value = Field;
                fn expecting(&self, __formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "variant identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(Field::OMS),
                        1u64 => _serde::__private::Ok(Field::OMI),
                        2u64 => _serde::__private::Ok(Field::OMV),
                        _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"variant index 0 <= i < 3",
                        )),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "OMS" => _serde::__private::Ok(Field::OMS),
                        "OMI" => _serde::__private::Ok(Field::OMI),
                        "OMV" => _serde::__private::Ok(Field::OMV),
                        _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                            __value, VARIANTS,
                        )),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"OMS" => _serde::__private::Ok(Field::OMS),
                        b"OMI" => _serde::__private::Ok(Field::OMI),
                        b"OMV" => _serde::__private::Ok(Field::OMV),
                        _ => {
                            let __value = &_serde::__private::from_utf8_lossy(__value);
                            _serde::__private::Err(_serde::de::Error::unknown_variant(
                                __value, VARIANTS,
                            ))
                        }
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, FieldVisitor)
                }
            }
            #[doc(hidden)]
            const VARIANTS: &'static [&'static str] = &["OMS", "OMI", "OMV"];
            let (__tag, __content) = _serde::Deserializer::deserialize_any(
                __deserializer,
                _serde::__private::de::TaggedContentVisitor::<Field>::new(
                    "kind",
                    "internally tagged enum Foo",
                ),
            )?;
            let __deserializer =
                _serde::__private::de::ContentDeserializer::<D::Error>::new(__content);
            match __tag {
                Field::OMS => {
                    #[allow(non_camel_case_types)]
                    #[doc(hidden)]
                    enum Field {
                        Value,
                        Foo,
                        __ignore,
                    }
                    #[doc(hidden)]
                    struct __FieldVisitor;

                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "field identifier")
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(Field::Value),
                                1u64 => _serde::__private::Ok(Field::Foo),
                                _ => _serde::__private::Ok(Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "value" => _serde::__private::Ok(Field::Value),
                                "foo" => _serde::__private::Ok(Field::Foo),
                                _ => _serde::__private::Ok(Field::__ignore),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"value" => _serde::__private::Ok(Field::Value),
                                b"foo" => _serde::__private::Ok(Field::Foo),
                                _ => _serde::__private::Ok(Field::__ignore),
                            }
                        }
                    }
                    #[automatically_derived]
                    impl<'de> _serde::Deserialize<'de> for Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    #[doc(hidden)]
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<Foo>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = Foo;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "struct variant Foo::OMS",
                            )
                        }
                        #[inline]
                        fn visit_seq<__A>(
                            self,
                            mut __seq: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::SeqAccess<'de>,
                        {
                            let __field0 =
                                match _serde::de::SeqAccess::next_element::<String>(&mut __seq)? {
                                    _serde::__private::Some(__value) => __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(
                                            _serde::de::Error::invalid_length(
                                                0usize,
                                                &"struct variant Foo::OMS with 2 elements",
                                            ),
                                        );
                                    }
                                };
                            let __field1 = match _serde::de::SeqAccess::next_element::<
                                Option<String>,
                            >(&mut __seq)?
                            {
                                _serde::__private::Some(__value) => __value,
                                _serde::__private::None => _serde::__private::Default::default(),
                            };
                            _serde::__private::Ok(Foo::OMS {
                                value: __field0,
                                foo: __field1,
                            })
                        }
                        #[inline]
                        fn visit_map<__A>(
                            self,
                            mut __map: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::MapAccess<'de>,
                        {
                            let mut __field0: _serde::__private::Option<String> =
                                _serde::__private::None;
                            let mut __field1: _serde::__private::Option<Option<String>> =
                                _serde::__private::None;
                            while let _serde::__private::Some(__key) =
                                _serde::de::MapAccess::next_key::<Field>(&mut __map)?
                            {
                                match __key {
                                    Field::Value => {
                                        if _serde::__private::Option::is_some(&__field0) {
                                            return _serde::__private::Err(
                                                <__A::Error as _serde::de::Error>::duplicate_field(
                                                    "value",
                                                ),
                                            );
                                        }
                                        __field0 = _serde::__private::Some(
                                            _serde::de::MapAccess::next_value::<String>(
                                                &mut __map,
                                            )?,
                                        );
                                    }
                                    Field::Foo => {
                                        if _serde::__private::Option::is_some(&__field1) {
                                            return _serde::__private::Err(
                                                <__A::Error as _serde::de::Error>::duplicate_field(
                                                    "foo",
                                                ),
                                            );
                                        }
                                        __field1 = _serde::__private::Some(
                                            _serde::de::MapAccess::next_value::<Option<String>>(
                                                &mut __map,
                                            )?,
                                        );
                                    }
                                    _ => {
                                        let _ = _serde::de::MapAccess::next_value::<
                                            _serde::de::IgnoredAny,
                                        >(
                                            &mut __map
                                        )?;
                                    }
                                }
                            }
                            let __field0 = match __field0 {
                                _serde::__private::Some(__field0) => __field0,
                                _serde::__private::None => {
                                    _serde::__private::de::missing_field("value")?
                                }
                            };
                            let __field1 = match __field1 {
                                _serde::__private::Some(__field1) => __field1,
                                _serde::__private::None => _serde::__private::Default::default(),
                            };
                            _serde::__private::Ok(Foo::OMS {
                                value: __field0,
                                foo: __field1,
                            })
                        }
                    }
                    #[doc(hidden)]
                    const FIELDS: &'static [&'static str] = &["value", "foo"];
                    _serde::Deserializer::deserialize_any(
                        __deserializer,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<Foo>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
                Field::OMI => _serde::__private::Result::map(
                    <i64 as _serde::Deserialize>::deserialize(__deserializer),
                    Foo::OMI,
                ),
                Field::OMV => _serde::__private::Result::map(
                    <String as _serde::Deserialize>::deserialize(__deserializer),
                    Foo::OMV,
                ),
            }
        }
    }
};
 */
