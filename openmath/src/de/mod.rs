/*! <span style="font-variant:small-caps;">OpenMath</span> Deserialization; [OMDeserializable] and related types
*/

//#[cfg(feature = "serde")]
//pub(crate) mod serde_aux;
#[cfg(feature = "serde")]
pub(crate) mod serde_impl;
pub(crate) mod xml;
use std::borrow::Cow;

use crate::{OMKind, OMMaybeForeign};
#[cfg(feature = "serde")]
pub use serde_impl::OMFromSerde;

type Args<T> = smallvec::SmallVec<T, 2>;
type Vars<T> = smallvec::SmallVec<T, 2>;
type Attrs<T> = Vec<T>;

pub type OMAttr<'o, I> = crate::Attr<'o, crate::OMMaybeForeign<'o, I>>;

#[allow(rustdoc::redundant_explicit_links)]
/**  Trait for types that can be deserialized from
<span style="font-variant:small-caps;">OpenMath</span> objects.

Deserialization is driven by the [`from_openmath`](OMDeserializable::from_openmath)-method
which gets an [`OM`] and can return an arbitrary intermediate type [`Ret`](OMDeserializable::Ret) --
in the simplest case, <code>[Ret](OMDeserializable::Ret) == Self</code>.

During deserialization, The method is called "from the bottom up" starting with the leafs.
If e.g. the expression is `OMA( OMS(s1), OMA( OMS(s2), OMI(1) ), OMI(3) )`, then this method
gets called successively with `OMS(s1)`, `OMS(s2)`, `OMI(1)`, `OMA( OMS(s2), OMI(1) )`, `OMI(3)`,
and finally `OMA( OMS(s1), OMA( OMS(s2), OMI(1) ), OMI(3) )`
(See below for an example).
Deserialization is considered to have failed (and
will return an Error), if for the *final* instance `r` of [`Ret`](OMDeserializable::Ret),
<code>r.[try_into](TryInto::try_into)()</code> errors.

`'de` is the lifetime of the deserialized data; tied to the e.g. string from which it gets
serialized. [`OMDeserializableOwned`] indicates that *owned* values
can be deserialized, and is implemented for any <code>S where for<'a> S:[OMDeserializable]<'s></code>.

- With the `serde`-feature active, deserialize from any serde-compatible format
  by deserializing a [OMFromSerde](serde_impl::OMFromSerde) instead (see below for an example).
  The implementation follows the official <span style="font-variant:small-caps;">OpenMath</span> JSON
  encoding[^1], so using
  [`serde_json`](https://docs.rs/serde_json) allows for deserializing from specification-compliant
  JSON.
- deserialize <span style="font-variant:small-caps;">OpenMath</span> XML
  from a `&'de str` using [from_openmath_xml](OMDeserializable::from_openmath_xml).
  If `Self` can be deserialized into owned values (i.e. implements <code>for<'a> [OMDeserializable]<'a></code>),
  the [`OMDeserializableOwned`] trait also provides
  <code>[from_openmath_xml_reader](OMDeserializableOwned::from_openmath_xml_reader)<R: [BufRead](std::io::BufRead)></code>.

# Examples

We can deserialize an <span style="font-variant:small-caps;">OpenMath</span> expression using addition and multiplication
to an `i128` directly; like so:
```rust
# #[cfg(feature="serde")]
# {
use either::Either;
use openmath::de::{OM, OMDeserializable, OMFromSerde};

#[derive(Copy, Clone, Debug)]
struct SimplifiedInt(i128);
impl<'d> TryFrom<Either<Self, OM<'d, Box<Self>>>> for SimplifiedInt {
    type Error = &'static str;
    fn try_from(value: Either<Self, OM<'d, Box<Self>>>) -> Result<Self, Self::Error> {
        if let Either::Left(v) = value {
            Ok(v)
        } else {
            Err("nope")
        }
    }
}
impl<'d> OMDeserializable<'d> for SimplifiedInt {
    type Ret = Either<Self, OM<'d, Box<Self>>>;
    type Err = &'static str;
    fn from_openmath(
        om: OM<'d, Self::Ret>,
        cdbase: &str,
    ) -> Result<Either<Self, OM<'d, Box<Self>>>, Self::Err>
    where
        Self: Sized,
    {
        match om {
            // An integer
            OM::OMI { int, .. } => {
                // ...which fits in an i128
                int.is_i128()
                    .map_or(Err("Invalid int value"), |i| Ok(Either::Left(Self(i))))
            }
            // Addition or multiplication
            OM::OMS { cd, name, .. }
                if cd == "arith1"
                    && (name == "plus" || name == "times")
                    && cdbase == openmath::CD_BASE =>
            {
                // works, but without arguments, we can't do anything to it *yet*.
                // => We send it back, so we can take care of it later, if it
                // occurs as the head of an OMA expression
                Ok(either::Right(OM::OMS {
                    cd,
                    name,
                    attrs: Vec::new(),
                }))
            }
            // some operator application to two arguments
            OM::OMA {
                // still an open math expression:
                applicant: either::Right(op),
                mut arguments,
                ..
            } if arguments.iter().all(Either::is_left)
                && arguments.len() == 2
                && cdbase == openmath::CD_BASE =>
            {
                // An OMA only ends up here, after both the head and all arguments
                // were fed into this method.
                // Since "plus" and "times" are the only values for
                // which we return `either::Right`, we know the following matches:
                let is_times = match op {
                    OM::OMS { name, .. } => name == "times",
                    _ => unreachable!(),
                };
                let Some(Either::Left(arg2)) = arguments.pop() else {
                    unreachable!()
                };
                let Some(Either::Left(arg1)) = arguments.pop() else {
                    unreachable!()
                };
                let value = if is_times {
                    arg1.0 * arg2.0
                } else {
                    arg1.0 + arg2.0
                };
                Ok(Either::Left(Self(value)))
            }
            // everything else is illegal
            _ => Err("Not an arithmetic expression"),
        }
    }
}

// 2 + 2
let s = r#"{
    "cdbase":"http://www.openmath.org/cd",
    "kind": "OMA",
    "applicant": {
        "kind": "OMS",
        "cd": "arith1",
        "name": "plus"
    },
    "arguments": [
        { "kind":"OMI", "integer":2 },
        { "kind":"OMI", "integer":2 }
    ]
}"#;
// If the serde-feature is active:
let r = serde_json::from_str::<'_, OMFromSerde<SimplifiedInt>>(s)
    .expect("valid json, openmath, and arithmetic expression");
assert_eq!(r.into_inner().0, 4);
let s = r#"
<OMA cdbase="http://www.openmath.org/cd">
  <OMS cd="arith1" name="plus"/>
  <OMI>2</OMI>
  <OMI>2</OMI>
</OMA>"#;
    let r = SimplifiedInt::from_openmath_xml(s)
        .expect("valid xml, openmath, and arithmetic expression");
    assert_eq!(r.0, 4);
# }
```
[^1]: <https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_json-the-json-encoding>
*/
pub trait OMDeserializable<'de>: std::fmt::Debug {
    /// The type returned by [from_openmath](OMDeserializable::from_openmath);
    /// Can be `Self`, but can also be something more complex so that [OM]-values can be
    /// "deferred" until enough information is there to construct a `Self`; See
    /// above for an example.
    type Ret: TryInto<Self, Error: std::fmt::Debug>
    where
        Self: Sized;
    /// The type of errors that can occur during deserialization.
    type Err: std::fmt::Display;

    /// Attempt to deserialize an <span style="font-variant:small-caps;">OpenMath</span> object
    /// into this type.
    ///
    /// # Errors
    /// If the provided [OM] can not be converted into a [`Ret`](OMDeserializable::Ret),
    /// meaning: it does not represent an meaningful intermediate result
    /// on the way to constructing a `Self`
    ///
    /// # Examples
    /// See [trait documentation](OMDeserializable)
    #[allow(clippy::type_complexity)]
    fn from_openmath(om: OM<'de, Self::Ret>, cdbase: &str) -> Result<Self::Ret, Self::Err>
    where
        Self: Sized;

    /// Deserializes self from a string of <span style="font-variant:small-caps;">OpenMath</span> XML.
    ///
    /// # Errors
    /// iff the string provided is invalid XML, or invalid
    /// <span style="font-variant:small-caps;">OpenMath</span>, or
    /// [from_openmath](OMDeserializable::from_openmath) errors.
    ///
    /// # Examples
    /// See [trait documentation](OMDeserializable)
    fn from_openmath_xml(input: &'de str) -> Result<Self, xml::XmlReadError<Self::Err>>
    where
        Self: Sized,
    {
        use xml::Readable;
        <xml::FromString<'de> as Readable<'de, Self>>::new(input).read(None)
    }
}
/// Trait for types that can be deserialized as owned values from
/// <span style="font-variant:small-caps;">OpenMath</span> objects.
///
/// Implemented primarily (and automatically) for types that
/// implement <code>for<'a> [OMDeserializable]<'a></code>.
pub trait OMDeserializableOwned: for<'d> OMDeserializable<'d> {
    /// Deserializes self from any [Read](std::io::BufRead) of <span style="font-variant:small-caps;">OpenMath</span> XML.
    ///
    /// # Errors
    /// iff the by stream provided is invalid UTF8, XML, or
    /// <span style="font-variant:small-caps;">OpenMath</span>, or
    /// [from_openmath](OMDeserializable::from_openmath)
    /// errors.
    /// # Examples
    /// See [trait documentation](OMDeserializable)
    #[inline]
    fn from_openmath_xml_reader<R: std::io::BufRead>(
        reader: R,
    ) -> Result<Self, xml::XmlReadError<<Self as OMDeserializable<'static>>::Err>>
    where
        Self: Sized,
    {
        use xml::Readable;
        <xml::Reader<R> as Readable<'static, Self>>::new(reader).read(None)
    }
}

/// Blanket implementation to allow owned deserializable types to work with the borrowed trait.
impl<O> OMDeserializableOwned for O where O: for<'de> OMDeserializable<'de> {}

/// Wrapper to deserialize an OMOBJ value.
pub struct OMObject<'de, O: OMDeserializable<'de>>(O, std::marker::PhantomData<&'de ()>);
impl<'de, O: OMDeserializable<'de>> OMObject<'de, O> {
    /// Returns the deserialized value.
    #[inline]
    pub fn into_inner(self) -> O {
        self.0
    }

    /** Deserializes an [OMDeserializable] from an XML string starting with `<OMOBJ>`
     *
    # Errors
    iff the string provided is invalid XML, or invalid <span style="font-variant:small-caps;">OpenMath</span>, or [from_openmath](OMDeserializable::from_openmath)
    errors.

    # Examples
    ```
    use openmath::de::{OMDeserializable, OM, OMObject};

    let s = r#"<OMOBJ cdbase="http://www.openmath.org/cd">
      <OMI>2</OMI>
    </OMOBJ>"#;
    assert_eq!(OMObject::<i32>::from_openmath_xml(s).expect("is valid"),2);
    ```
    */
    #[inline]
    pub fn from_openmath_xml(input: &'de str) -> Result<O, xml::XmlReadError<O::Err>>
    where
        O: Sized,
    {
        use xml::Readable;
        <xml::FromString as xml::Readable<'de, O>>::new(input).read_obj()
    }
}

/// Enum for deserializing from <span style="font-variant:small-caps;">OpenMath</span>. See
/// see [OMDeserializable] for documentation and an example.
///
/// Note that there is no case for [OMATTR](crate::OMKind::OMATTR) - instead,
/// every case has a <code>[Vec]<[OMAttr]<'de, I>></code>, which is usually empty.
/// Otherwise, we'd have to either deal with two separate types, or have the
/// nonsensical case `OMATTR(OMATTR(OMATTR(...),...),...)`, which would also
/// require a [`Box`]-indirection (hence allocation), etc. since OMATTR is mostly used
/// for metadata which the recipient might not even care about, or only care secondarily
/// (compared to the *actual* [OM]-kind), having OMATTR be a separate case seems
/// like bad API design.
/// Also, empty Vecs are cheap.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum OM<'de, I> {
    /** <div class="openmath">
    Integers in the mathematical sense, with no predefined range.
    They are “infinite precision” integers (also called “bignums” in computer algebra).
    </div> */
    OMI {
        int: crate::Int<'de>,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMI as _,

    /** <div class="openmath">
    Double precision floating-point numbers following the IEEE 754-1985 standard.
    </div> */
    OMF {
        float: f64,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMF as _,

    /** <div class="openmath">
    A Unicode Character string. This also corresponds to “characters” in XML.
    </div> */
    OMSTR {
        string: Cow<'de, str>,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMSTR as _,

    /** <div class="openmath">
    A sequence of bytes.
    </div> */
    OMB {
        bytes: Cow<'de, [u8]>,
        attrs: Attrs<OMAttr<'de, I>>,
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
        name: Cow<'de, str>,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMV as _,

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
    OMS {
        cd: Cow<'de, str>,
        name: Cow<'de, str>,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMS as _,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA {
        applicant: I,
        arguments: Args<I>,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMA as _,

    /** <div class="openmath">
    If $B$ and $C$ are <span style="font-variant:small-caps;">OpenMath</span> objects, and $v_1,...,v_n\;(n\geq0)$
    are <span style="font-variant:small-caps;">OpenMath</span> variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an <span style="font-variant:small-caps;">OpenMath</span> binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND {
        binder: I,
        variables: Vars<(Cow<'de, str>, Attrs<OMAttr<'de, I>>)>,
        object: I,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OMBIND as _,

    /** <div class="openmath">
    If $S$ is an <span style="font-variant:small-caps;">OpenMath</span> symbol and $A_1,...,A_n\;(n\geq0)$ are <span style="font-variant:small-caps;">OpenMath</span> objects or
    derived <span style="font-variant:small-caps;">OpenMath</span> objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an <span style="font-variant:small-caps;">OpenMath</span> error object.
    </div> */
    OME {
        cdbase: Option<Cow<'de, str>>,
        cd: Cow<'de, str>,
        name: Cow<'de, str>,
        arguments: Vec<OMMaybeForeign<'de, I>>,
        attrs: Attrs<OMAttr<'de, I>>,
    } = OMKind::OME as _,
}
impl<I> OM<'_, I> {
    /// Returns the [OMKind] of this [`OM`], which of all practical purposes
    /// acts as a discriminant.
    pub fn kind(&self) -> crate::OMKind {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe {
            let u = *<*const _>::from(self).cast::<u8>();
            crate::OMKind::from_u8(u).unwrap_unchecked()
        }
    }
}

impl<'d> OMDeserializable<'d> for crate::Int<'d> {
    type Ret = Self;
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMI { int, .. } = om {
            Ok(int)
        } else {
            Err("Not an integer")
        }
    }
}

impl<'d> OMDeserializable<'d> for f32 {
    type Ret = Self;
    type Err = &'static str;
    #[allow(clippy::cast_possible_truncation)]
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMF { float, .. } = om {
            Ok(float as _)
        } else {
            Err("Not a float")
        }
    }
}

impl<'d> OMDeserializable<'d> for f64 {
    type Ret = Self;
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMF { float, .. } = om {
            Ok(float)
        } else {
            Err("Not a float")
        }
    }
}

impl<'d> OMDeserializable<'d> for Cow<'d, str> {
    type Ret = Self;
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMSTR { string, .. } = om {
            Ok(string)
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d> OMDeserializable<'d> for String {
    type Ret = Self;
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMSTR { string, .. } = om {
            Ok(string.into_owned())
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d> OMDeserializable<'d> for Cow<'d, [u8]> {
    type Ret = Self;
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMB { bytes, .. } = om {
            Ok(bytes)
        } else {
            Err("Not an OMB")
        }
    }
}
impl<'d> OMDeserializable<'d> for Vec<u8> {
    type Ret = Self;
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMB { bytes, .. } = om {
            Ok(bytes.into_owned())
        } else {
            Err("Not an OMB")
        }
    }
}

macro_rules! impl_int_deserializable {
    ($($t:ty=$err:literal),*) => {
        $(
            impl<'d> OMDeserializable<'d> for $t {
                type Ret = Self;
                type Err = &'static str;
                fn from_openmath(
                    om: OM<'d, Self>,
                    _: &str
                ) -> Result<Self, Self::Err>
                where
                    Self: Sized,
                {
                    if let OM::OMI{int,..} = om {
                        int.is_i128().map_or(Err($err), |i| {
                            i.try_into().map_err(|_| $err)
                        })
                    } else {
                        Err("Not an OMI")
                    }
                }
            }
        )*
    };
}
impl_int_deserializable! {
    i8 = "not an i8", u8 = "not a u8",
    i16 = "not an i16", u16 = "not a u16",
    u32 = "not a u32", i32 = "not an i32",
    i64 = "not an i64", u64 = "not a u64",
    i128 = "not an i128",
    isize = "not an isize", usize = "not a usize"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    struct TestVariable(String);

    impl<'de> OMDeserializable<'de> for TestVariable {
        type Ret = Self;
        type Err = String;

        fn from_openmath(om: OM<'de, Self>, _: &str) -> Result<Self, Self::Err> {
            match om {
                OM::OMV { name, .. } => Ok(Self(name.to_string())),
                _ => Err("wrong".to_string()),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestSymbol {
        cdbase: String,
        cd: String,
        name: String,
    }

    impl<'de> OMDeserializable<'de> for TestSymbol {
        type Ret = Self;
        type Err = &'static str;

        fn from_openmath(om: OM<'de, Self>, cdbase: &str) -> Result<Self, Self::Err> {
            match om {
                OM::OMS { cd, name, .. } => Ok(Self {
                    cdbase: cdbase.to_string(),
                    cd: cd.to_string(),
                    name: name.to_string(),
                }),
                _ => Err("nope"),
            }
        }
    }

    #[derive(Copy, Clone, Debug)]
    struct Oma;
    enum ArgOrOMA {
        Oms,
        Omi,
        Oma,
    }
    impl TryFrom<ArgOrOMA> for Oma {
        type Error = &'static str;
        fn try_from(value: ArgOrOMA) -> Result<Self, Self::Error> {
            if matches!(value, ArgOrOMA::Oma) {
                Ok(Self)
            } else {
                Err("nope")
            }
        }
    }
    impl<'d> OMDeserializable<'d> for Oma {
        type Ret = ArgOrOMA;
        type Err = &'static str;
        fn from_openmath(om: OM<'d, ArgOrOMA>, _cdbase: &str) -> Result<Self::Ret, Self::Err>
        where
            Self: Sized,
        {
            match om {
                OM::OMA {
                    applicant: ArgOrOMA::Oms,
                    arguments,
                    ..
                } if arguments.len() == 2
                    && arguments.iter().all(|a| matches!(a, ArgOrOMA::Omi)) =>
                {
                    Ok(ArgOrOMA::Oma)
                }
                OM::OMS { .. } => Ok(ArgOrOMA::Oms),
                OM::OMI { .. } => Ok(ArgOrOMA::Omi),
                _ => Err("nope"),
            }
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_oma_deserialization() {
        let s = r#"{
            "cdbase":"http://www.openmath.org/cd",
            "kind": "OMA",
            "applicant": {
                "kind": "OMS",
                "cd": "arith1",
                "name": "plus"
            },
            "arguments": [
                { "kind":"OMI", "integer":2 },
                { "kind":"OMI", "integer":2 }
            ]
        }"#;
        serde_json::from_str::<'_, OMFromSerde<Oma>>(s)
            .expect("valid json, openmath, and arithmetic expression");
        serde_json::from_reader::<_, OMFromSerde<Oma>>(s.as_bytes())
            .expect("valid json, openmath, and arithmetic expression");
    }

    #[test]
    fn test_oma_deserialization_xml() {
        let s = r#"<OMOBJ cdbase="http://www.openmath.org/cd">
          <OMA>
            <OMS cd="arith1" name="plus"/>
            <OMI>2</OMI>
            <OMI>2</OMI>
          </OMA>
        </OMOBJ>"#;
        OMObject::<Oma>::from_openmath_xml(s).expect("is valid");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn fancy() {
        use crate as openmath;
        use either::Either;
        use openmath::de::{OM, OMDeserializable, OMFromSerde};

        #[derive(Copy, Clone, Debug)]
        struct SimplifiedInt(i128);
        impl<'d> TryFrom<Either<Self, OM<'d, Box<Self>>>> for SimplifiedInt {
            type Error = &'static str;
            fn try_from(value: Either<Self, OM<'d, Box<Self>>>) -> Result<Self, Self::Error> {
                if let Either::Left(v) = value {
                    Ok(v)
                } else {
                    Err("nope")
                }
            }
        }
        impl<'d> OMDeserializable<'d> for SimplifiedInt {
            type Ret = Either<Self, OM<'d, Box<Self>>>;
            type Err = &'static str;
            fn from_openmath(
                om: OM<'d, Self::Ret>,
                cdbase: &str,
            ) -> Result<Either<Self, OM<'d, Box<Self>>>, Self::Err>
            where
                Self: Sized,
            {
                match om {
                    // An integer
                    OM::OMI { int, .. } => {
                        // ...which fits in an i128
                        int.is_i128()
                            .map_or(Err("Invalid int value"), |i| Ok(Either::Left(Self(i))))
                    }
                    // Addition or multiplication
                    OM::OMS { cd, name, .. }
                        if cd == "arith1"
                            && (name == "plus" || name == "times")
                            && cdbase == openmath::CD_BASE =>
                    {
                        // works, but without arguments, we can't do anything to it *yet*.
                        // => We send it back, so we can take care of it later, if it
                        // occurs as the head of an OMA expression
                        Ok(either::Right(OM::OMS {
                            cd,
                            name,
                            attrs: Attrs::new(),
                        }))
                    }
                    // some operator application to two arguments
                    OM::OMA {
                        // still an open math expression:
                        applicant: either::Right(op),
                        mut arguments,
                        ..
                    } if arguments.iter().all(Either::is_left)
                        && arguments.len() == 2
                        && cdbase == openmath::CD_BASE =>
                    {
                        // An OMA only ends up here, after both the head and all arguments
                        // were fed into this method.
                        // Since "plus" and "times" are the only values for
                        // which we return `either::Right`, we know the following matches:
                        let is_times = match op {
                            OM::OMS { name, .. } => name == "times",
                            _ => unreachable!(),
                        };
                        let Some(Either::Left(arg2)) = arguments.pop() else {
                            unreachable!()
                        };
                        let Some(Either::Left(arg1)) = arguments.pop() else {
                            unreachable!()
                        };
                        let value = if is_times {
                            arg1.0 * arg2.0
                        } else {
                            arg1.0 + arg2.0
                        };
                        Ok(Either::Left(Self(value)))
                    }
                    // everything else is illegal
                    _ => Err("Not an arithmetic expression"),
                }
            }
        }

        // 2 + 2
        let s = r#"{
            "cdbase":"http://www.openmath.org/cd",
            "kind": "OMA",
            "applicant": {
                "kind": "OMS",
                "cd": "arith1",
                "name": "plus"
            },
            "arguments": [
                { "kind":"OMI", "integer":2 },
                { "kind":"OMI", "integer":2 }
            ]
        }"#;
        let r = serde_json::from_str::<'_, OMFromSerde<SimplifiedInt>>(s)
            .expect("valid json, openmath, and arithmetic expression");
        assert_eq!(r.into_inner().0, 4);
        let s = r#"
        <OMA cdbase="http://www.openmath.org/cd">
          <OMS cd="arith1" name="plus"/>
          <OMI>2</OMI>
          <OMI>2</OMI>
        </OMA>"#;
        let r = SimplifiedInt::from_openmath_xml(s)
            .expect("valid xml, openmath, and arithmetic expression");
        assert_eq!(r.0, 4);
    }
}
