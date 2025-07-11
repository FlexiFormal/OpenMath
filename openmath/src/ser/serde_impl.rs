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
use crate::{
    OMSerializable,
    ser::{AsOMS, OMOrForeign, OMSerializer},
};
use either::Either;
use serde::{
    Serializer,
    ser::{SerializeSeq, SerializeStruct, SerializeTuple},
};
impl<E: serde::ser::Error> super::Error for E {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn custom(err: impl std::fmt::Display) -> Self {
        serde::ser::Error::custom(err)
    }
}

impl<O: OMSerializable + ?Sized> serde::Serialize for super::OMObject<'_, O> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let cd_base = self.0.cd_base();
        let mut s =
            serializer.serialize_struct("OMObject", if cd_base.is_some() { 4 } else { 3 })?;
        s.serialize_field("kind", "OMOBJ")?;
        s.serialize_field("openmath", "2.0")?;
        if let Some(b) = self.0.cd_base() {
            s.serialize_field("cdbase", b)?;
        } else {
            s.skip_field("cdbase")?;
        }
        s.serialize_field("object", &self.0.openmath_serde())?;
        s.end()
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
    pub(crate) OM,
    pub(crate) Option<&'s str>,
    pub(crate) &'s str,
)
where
    OM: crate::OMSerializable;

impl<OM: crate::OMSerializable> ::serde::Serialize for SerdeSerializer<'_, OM> {
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
    current_ns: &'s str,
}

impl<'s, S: ::serde::Serializer> OMSerializer<'s> for Serder<'s, S> {
    type Ok = S::Ok;
    type Err = S::Error;
    type SubSerializer<'ns>
        = Serder<'ns, S>
    where
        's: 'ns;

    #[inline]
    fn current_cd_base(&self) -> &str {
        self.next_ns.unwrap_or(self.current_ns)
    }

    fn with_cd_base<'ns>(self, cd_base: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
    where
        's: 'ns,
    {
        if self.current_ns == cd_base {
            Ok(self)
        } else {
            Ok(Serder {
                s: self.s,
                next_ns: Some(cd_base),
                current_ns: self.current_ns,
            })
        }
    }

    fn omi(self, value: &crate::Int) -> Result<Self::Ok, Self::Err> {
        let mut struc = self.s.serialize_struct("OMObject", 2)?;
        struc.serialize_field("kind", &crate::OMKind::OMI)?;
        struc.skip_field("id")?;
        if let Some(i) = value.is_i128() {
            struc.serialize_field("integer", &i)?;
        } else {
            struc.serialize_field("decimal", value)?;
        }
        struc.end()
    }

    fn omf(self, value: f64) -> Result<Self::Ok, Self::Err> {
        let mut struc = self.s.serialize_struct("OMObject", 2)?;
        struc.serialize_field("kind", &crate::OMKind::OMF)?;
        struc.skip_field("id")?;
        struc.serialize_field("float", &value)?;
        struc.end()
    }

    fn omstr(self, string: impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        let mut struc = self.s.serialize_struct("OMObject", 2)?;
        struc.serialize_field("kind", &crate::OMKind::OMSTR)?;
        struc.skip_field("id")?;
        struc.serialize_field("string", &DWrap(string))?;
        struc.end()
    }

    fn omb(self, bytes: impl ExactSizeIterator<Item = u8>) -> Result<Self::Ok, Self::Err> {
        use crate::base64::Base64Encodable;
        let mut struc = self.s.serialize_struct("OMObject", 2)?;
        struc.serialize_field("kind", &crate::OMKind::OMB)?;
        struc.skip_field("id")?;
        let s = bytes.into_iter().base64().into_string();
        struc.serialize_field("base64", &s)?;
        struc.end()
    }

    fn omv(self, name: impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        let mut struc = self.s.serialize_struct("OMObject", 2)?;
        struc.serialize_field("kind", &crate::OMKind::OMV)?;
        struc.skip_field("id")?;
        struc.serialize_field("name", &DWrap(name))?;
        struc.end()
    }

    fn oms(
        self,
        cd_name: impl std::fmt::Display,
        name: impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err> {
        let num_fields = if self.next_ns.is_some() { 4 } else { 3 };
        let mut struc = self.s.serialize_struct("OMObject", num_fields)?;
        struc.serialize_field("kind", &crate::OMKind::OMS)?;
        struc.skip_field("id")?;
        if let Some(ns) = self.next_ns {
            struc.serialize_field("cdbase", ns)?;
        } else {
            struc.skip_field("cdbase")?;
        }
        struc.serialize_field("cd", &DWrap(cd_name))?;
        struc.serialize_field("name", &DWrap(name))?;
        struc.end()
    }

    fn ome(
        mut self,
        error: impl AsOMS,
        args: impl ExactSizeIterator<Item: super::OMOrForeign>,
    ) -> Result<Self::Ok, Self::Err> {
        let mut num_fields = 2;
        if args.len() > 0 {
            num_fields += 1;
        }
        if self.next_ns.is_some() {
            num_fields += 1;
        }

        let mut struc = self.s.serialize_struct("OMObject", num_fields)?;
        struc.serialize_field("kind", &crate::OMKind::OME)?;
        struc.skip_field("id")?;
        if let Some(ns) = self.next_ns.take() {
            self.current_ns = ns;
            struc.serialize_field("cdbase", ns)?;
        } else {
            struc.skip_field("cdbase")?;
        }

        struc.serialize_field(
            "error",
            &SerdeSerializer(&error.as_oms(), None, self.current_ns),
        )?;
        if args.len() > 0 {
            struc.serialize_field(
                "arguments",
                &Iter(std::cell::Cell::new(Some(args.map(
                    |e| match e.om_or_foreign() {
                        Either::Left(e) => {
                            ForeignSerializer::O(SerdeSerializer(e, None, self.current_ns))
                        }
                        Either::Right((encoding, value)) => {
                            ForeignSerializer::F { encoding, value }
                        }
                    },
                )))),
            )?;
        } else {
            struc.skip_field("arguments")?;
        }
        struc.end()
    }

    fn oma(
        mut self,
        head: impl OMSerializable,
        args: impl ExactSizeIterator<Item: OMSerializable>,
    ) -> Result<Self::Ok, Self::Err> {
        let mut num_fields = 2;
        if args.len() != 0 {
            num_fields += 1;
        }
        if self.next_ns.is_some() {
            num_fields += 1;
        }
        let mut struc = self.s.serialize_struct("OMObject", num_fields)?;
        struc.serialize_field("kind", &crate::OMKind::OMA)?;
        struc.skip_field("id")?;
        if let Some(ns) = self.next_ns.take() {
            self.current_ns = ns;
            struc.serialize_field("cdbase", ns)?;
        } else {
            struc.skip_field("cdbase")?;
        }
        struc.serialize_field("applicant", &SerdeSerializer(head, None, self.current_ns))?;
        if args.len() != 0 {
            struc.serialize_field(
                "arguments",
                &Iter(std::cell::Cell::new(Some(
                    args.map(|e| SerdeSerializer(e, None, self.current_ns)),
                ))),
            )?;
        } else {
            struc.skip_field("arguments")?;
        }
        struc.end()
    }

    fn ombind(
        mut self,
        head: impl OMSerializable,
        vars: impl ExactSizeIterator<Item: super::BindVar>,
        body: impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err> {
        let vars = vars.into_iter();
        let mut num_fields = 4;
        if self.next_ns.is_some() {
            num_fields += 1;
        }
        let mut struc = self.s.serialize_struct("OMObject", num_fields)?;
        struc.serialize_field("kind", &crate::OMKind::OMBIND)?;
        struc.skip_field("id")?;
        if let Some(ns) = self.next_ns.take() {
            self.current_ns = ns;
            struc.serialize_field("cdbase", ns)?;
        } else {
            struc.skip_field("cdbase")?;
        }
        struc.serialize_field("binder", &SerdeSerializer(head, None, self.current_ns))?;
        struc.serialize_field(
            "variables",
            &Iter(std::cell::Cell::new(Some(vars.map(|v| VWrap {
                ns: self.current_ns,
                var: v,
            })))),
        )?;
        struc.serialize_field("object", &SerdeSerializer(body, None, self.current_ns))?;
        struc.end()
    }

    fn omattr(
        mut self,
        attrs: impl ExactSizeIterator<Item: super::OMAttr>,
        atp: impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err> {
        let i = attrs.into_iter();
        if i.len() == 0 {
            return atp.as_openmath(self);
        }

        let mut struc = self
            .s
            .serialize_struct("OMObject", if self.next_ns.is_some() { 4 } else { 3 })?;
        struc.serialize_field("kind", &crate::OMKind::OMATTR)?;
        struc.skip_field("id")?;
        if let Some(ns) = self.next_ns.take() {
            self.current_ns = ns;
            struc.serialize_field("cdbase", ns)?;
        } else {
            struc.skip_field("cdbase")?;
        }
        struc.serialize_field(
            "attributes",
            &Iter(std::cell::Cell::new(Some(i.map(|v| OMAttrW {
                ns: self.current_ns,
                attr: v,
            })))),
        )?;

        struc.serialize_field("object", &SerdeSerializer(atp, None, self.current_ns))?;
        struc.end()
    }
}

struct Iter<I: ExactSizeIterator>(std::cell::Cell<Option<I>>)
where
    I::Item: serde::Serialize;
impl<I: ExactSizeIterator> serde::Serialize for Iter<I>
where
    I::Item: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let Some(args) = self.0.take() else {
            return Err(S::Error::custom("Error iterating over arguments"));
        };
        let mut seq = serializer.serialize_seq(Some(args.len()))?;
        for s in args {
            seq.serialize_element(&s)?;
        }
        seq.end()
    }
}

struct DWrap<D: std::fmt::Display>(D);
impl<D: std::fmt::Display> serde::Serialize for DWrap<D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

struct VWrap<'d, V: super::BindVar> {
    ns: &'d str,
    var: V,
}
impl<V: super::BindVar> serde::Serialize for VWrap<'_, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let attrs = self.var.attrs();
        if attrs.len() == 0 {
            Serder {
                s: serializer,
                next_ns: None,
                current_ns: self.ns,
            }
            .omv(self.var.name())
        } else {
            Serder {
                s: serializer,
                next_ns: None,
                current_ns: self.ns,
            }
            .omattr(attrs, super::Omv(self.var.name()))
        }
    }
}

struct OMAttrW<'de, A: super::OMAttr> {
    ns: &'de str,
    attr: A,
}

impl<A: super::OMAttr> serde::Serialize for OMAttrW<'_, A> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(2)?;
        let symbol = self.attr.symbol();
        tup.serialize_element(&SerdeSerializer(&symbol.as_oms(), None, self.ns))?;
        let v = match self.attr.value().om_or_foreign() {
            Either::Left(e) => ForeignSerializer::O(SerdeSerializer(e, None, self.ns)),
            Either::Right((encoding, value)) => ForeignSerializer::F { encoding, value },
        };
        tup.serialize_element(&v)?;
        tup.end()
    }
}

enum ForeignSerializer<'s, OM, D: std::fmt::Display, E: std::fmt::Display>
where
    OM: crate::OMSerializable,
{
    O(SerdeSerializer<'s, OM>),
    F { encoding: Option<E>, value: D },
}
impl<OM: crate::OMSerializable, D: std::fmt::Display, E: std::fmt::Display> ::serde::Serialize
    for ForeignSerializer<'_, OM, D, E>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::O(o) => o.serialize(serializer),
            Self::F { encoding, value } => {
                let mut struc = serializer
                    .serialize_struct("OMObject", if encoding.is_some() { 3 } else { 2 })?;
                struc.serialize_field("kind", &crate::OMKind::OMFOREIGN)?;
                struc.skip_field("id")?;
                struc.serialize_field("foreign", &DWrap(value))?;
                if let Some(e) = encoding {
                    struc.serialize_field("encoding", &DWrap(e))?;
                } else {
                    struc.skip_field("encoding")?;
                }
                struc.end()
            }
        }
    }
}
