/*! OpenMath Deserialization; [OMDeserializable] and related types
*/

//#[cfg(feature = "serde")]
//pub(crate) mod serde_aux;
#[cfg(feature = "serde")]
pub(crate) mod serde_impl;
#[cfg(feature = "xml")]
pub(crate) mod xml;
use std::borrow::Cow;

use crate::{OMKind, OMMaybeForeign, either::Either};
use either::Either::Left;
#[cfg(feature = "serde")]
pub use serde_impl::OMFromSerde;

pub type OMAttr<'o, I> = crate::Attr<'o, Either<I, crate::OMMaybeForeign<'o, OM<'o, I>>>>;

#[allow(rustdoc::redundant_explicit_links)]
/**  Trait for types that can be deserialized from OpenMath objects.

This trait defines how a Rust type can be constructed from an OpenMath
representation. The deserialization process either succeeds (returning the
target type) or fails gracefully (returning the original OpenMath object).

Deserialization is driven by the [`from_openmath`](OMDeserializable::from_openmath)-method
which gets an [`OpenMath`] and can return either
- a `Self`, if the OpenMath expression represent a Self, or
- the original expression if it can not get deserialized (*yet*), or
- an error.

During deserialization, The method is called "from the bottom up" starting with the leafs.
If e.g. the expression is `OMA( OMS(s1), OMA( OMS(s2), OMI(1) ), OMI(3) )`, then this method
gets called successively with `OMS(s1)`, `OMS(s2)`, `OMI(1)`, `OMA( OMS(s2), OMI(1) )`, `OMI(3)`,
and finally `OMA( OMS(s1), OMA( OMS(s2), OMI(1) ), OMI(3) )`.
(See below for an example.)

### Built-in Deserializers
- **Serde-based** Deserialize from any [serde](https://docs.rs/serde)-compatible format by deserializing
  an <code>[OMFromSerde](serde_impl::OMFromSerde)<'d,MyType></code> instead,
  and calling [`into_inner()`](serde_impl::OMFromSerde::into_inner) on the result
  to get the `MyType`. (requires the `serde`-feature)
  If the last call to [`from_openmath`](OMDeserializable::from_openmath) is
  not a full <code>[Ok](Result::Ok)(MyType)</code>, serde deserialization will return
  an error already.
  The implementation follows the official OpenMath JSON encoding[^1], so using
  [`serde_json`](https://docs.rs/serde_json) allows for deserializing specification-compliant
  JSON.

`'de` is the lifetime of the deserialized data; tied to the e.g. string from which it gets
serialized. If `Self` should be entirely owned, implement [`OMDeserializableOwned`]
instead; which provides a blanket implementation for <code>[OMDeserializableOwned]<'static,[Vec]<[u8]>,[String]></code>

# Examples

We can deserialize an OpenMath expression using addition and multiplication
to an `i128` directly; like so:
```rust
# #[cfg(feature="serde")]
# {
# use std::borrow::Cow;
use openmath::de::{OMDeserializable, OMFromSerde, OM};
use openmath::either::Either;

#[derive(Copy, Clone, Debug)]
struct SimplifiedInt(i128);
impl<'d> OMDeserializable<'d> for SimplifiedInt {
    type Err = String;
    fn from_openmath(
        om: OM<'d, Self>,
        cdbase:&str
    ) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        match om {
            // An integer
            OM::OMI{int,..} => {
                // ...which fits in an i128
                if let Some(i) = int.is_i128() {
                    Ok(Either::Left(Self(i)))
                } else {
                    Err(format!("Invalid int value: {int}"))
                }
            }
            // Addition or multiplication
            OM::OMS { cd, name, attrs } if
                cd == "arith1" &&
                (name == "plus" || name == "times") &&
                cdbase == openmath::OPENMATH_BASE_URI.as_str() => {
                // works, but without arguments, we can't do anything to it *yet*.
                // => We send it back, so we can take care of it later, if it
                // occurs as the head of an OMA expression
                Ok(either::Right(OM::OMS { cd, name, attrs }))
            }
            // some operator application to two arguments
            OM::OMA {
                // still an open math expression:
                applicant: either::Right(op),
                mut arguments,
                attrs
            } if arguments.iter().all(Either::is_left)
                && arguments.len() == 2
                && cdbase == openmath::OPENMATH_BASE_URI.as_str() => {
                // An OMA only ends up here, after both the head and all arguments
                // were fed into this method.
                // Since "plus" and "times" are the only values for
                // which we return `either::Right`, we know the following matches:
                let is_times = match *op {
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
            o => Err(format!("Not an arithmetic expression: {o:?}")),
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
# #[cfg(feature="xml")]
# {
// If the xml feature is active:
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
# }
```

[^1]: <https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_json-the-json-encoding>
*/
pub trait OMDeserializable<'de>: std::fmt::Debug {
    /// The type of errors that can occur during deserialization.
    type Err: std::fmt::Display;

    /// Attempt to deserialize an OpenMath object into this type.
    ///
    /// # Errors
    /// This method examines the provided OpenMath object and either:
    #[allow(rustdoc::redundant_explicit_links)]
    /// 1. Successfully converts it to the target type (returns <code>[Ok](Result::Ok)([Left](either::Either::Left)(T))</code>)
    /// 2. Determines it cannot be converted *yet*, but maybe later in an OMA or OMBIND, and returns the
    #[allow(rustdoc::redundant_explicit_links)]
    ///    original object (<code>[Ok](Result::Ok)([Right](either::Either::Right)(om))</code>)
    /// 3. Encounters an error during processing ([`Err`])
    ///
    /// # Examples
    /// See [trait documentation](OMDeserializable)
    #[allow(clippy::type_complexity)]
    fn from_openmath(
        om: OM<'de, Self>,
        cdbase: &str,
    ) -> Result<Either<Self, OM<'de, Self>>, Self::Err>
    where
        Self: Sized;

    #[cfg(feature = "xml")]
    /// Deserializes self from a string of OpenMath XML.
    ///
    /// # Errors
    /// iff the string provided is invalid XML, or invalid OpenMath, or [from_openmath](OMDeserializable::from_openmath)
    /// errors.
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
/// Trait for types that can be deserialized as owned values OpenMath objects.
///
/// This is a specialized version of [`OMDeserializable`] for cases where you
/// need owned data (`String` and `Vec<u8>`) rather than borrowed data. This
/// is useful when the deserialized object needs to outlive the source data.
///
/// Also provides blanket implementations for [`OMDeserializable`].
pub trait OMDeserializableOwned: for<'d> OMDeserializable<'d> {
    #[cfg(feature = "xml")]
    /// Deserializes self from any [Read](std::io::BufRead) of OpenMath XML.
    ///
    /// # Errors
    /// iff the by stream provided is invalid UTF8, XML, or OpenMath, or
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
///
/// This implementation allows any type that implements [`OMDeserializableOwned`]
/// to automatically work with the [`OMDeserializable`] trait when using owned
/// data types.
impl<O> OMDeserializableOwned for O where O: for<'de> OMDeserializable<'de> {}

/// Wrapper to deserialize an OMOBJ
pub struct OMObject<'de, O: OMDeserializable<'de> + 'de>(O, std::marker::PhantomData<&'de ()>);

impl<'de, O: OMDeserializable<'de>> OMObject<'de, O> {
    #[inline]
    pub fn take(self) -> O {
        self.0
    }

    /** Deserializes an OMDeserializable from an XML string that contains an `<OMOBJ>`
    # Errors
    iff the string provided is invalid XML, or invalid OpenMath, or [from_openmath](OMDeserializable::from_openmath)
    errors.

    # Examples
    ```
    use openmath::de::{OMDeserializable, OM,OMObject};
    use openmath::either::Either;
    #[derive(Debug)]
    struct Meh;
    impl OMDeserializable<'static> for Meh {
        type Err = &'static str;
        fn from_openmath(
            om: OM<'static, Self>,
            _cdbase: &str,
        ) -> Result<Either<Self, OM<'static, Self>>, Self::Err>
        where
            Self: Sized,
        {
            match om {
                OM::OMA { .. } => Ok(Either::Left(Self)),
                o => Ok(Either::Right(o)),
            }
        }
    }
    let s = r#"
    <OMOBJ cdbase="http://www.openmath.org/cd">
      <OMA>
        <OMS cd="arith1" name="plus"/>
        <OMI>2</OMI>
        <OMI>2</OMI>
      </OMA>
    </OMOBJ>"#;
    OMObject::<Meh>::from_openmath_xml(s).expect("is valid");
    ```
    */
    #[cfg(feature = "xml")]
    #[inline]
    pub fn from_openmath_xml(input: &'de str) -> Result<O, xml::XmlReadError<O::Err>>
    where
        O: Sized,
    {
        use xml::Readable;
        <xml::FromString as xml::Readable<'de, O>>::new(input).read_obj()
    }
}

/// Enum for deserializing from OpenMath. See
/// see [OMDeserializable] for documentation and an example.
///
/// Note that there is no case for [OMATTR](crate::OMKind::OMATTR) - instead,
/// every case has a <code>[Vec]<[Attr]<'de, I>></code>, which is usually empty.
/// Otherwise, we'd have to either deal with two separate types, or have the
/// nonsensical case `OMATTR(OMATTR(OMATTR(...),...),...)`, which would also
/// require a [`Box`]-indirection (hence allocation), etc. since OMATTRS is mostly used
/// for metadata which the recipient might not even care about, or only care secondarily
/// (compared to the *actual* OM-kind), having OMATTR be a separate case seems
/// like bad API design.
/// Also, empty Vecs are cheap.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum OM<'de, I: OMDeserializable<'de>> {
    /** <div class="openmath">
    Integers in the mathematical sense, with no predefined range.
    They are “infinite precision” integers (also called “bignums” in computer algebra).
    </div> */
    OMI {
        int: crate::Int<'de>,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OMI as _,

    /** <div class="openmath">
    Double precision floating-point numbers following the IEEE 754-1985 standard.
    </div> */
    OMF {
        float: f64,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OMF as _,

    /** <div class="openmath">
    A Unicode Character string. This also corresponds to “characters” in XML.
    </div> */
    OMSTR {
        string: Cow<'de, str>,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OMSTR as _,

    /** <div class="openmath">
    A sequence of bytes.
    </div> */
    OMB {
        bytes: Cow<'de, [u8]>,
        attrs: Vec<OMAttr<'de, I>>,
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
        attrs: Vec<OMAttr<'de, I>>,
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
    /// OpenMath object. The possible roles are described in
    /// [Section 2.1.4](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_roles).
    ///
    ///</div>
    OMS {
        cd: Cow<'de, str>,
        name: Cow<'de, str>,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OMS as _,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are OpenMath objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an OpenMath application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA {
        applicant: Either<I, Box<Self>>,
        arguments: Vec<Either<I, Self>>,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OMA as _,

    /** <div class="openmath">
    If $B$ and $C$ are OpenMath objects, and $v_1,...,v_n\;(n\geq0)$
    are OpenMath variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an OpenMath binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND {
        binder: Either<I, Box<Self>>,
        variables: Vec<(Cow<'de, str>, Vec<OMAttr<'de, I>>)>,
        object: Either<I, Box<Self>>,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OMBIND as _,

    /** <div class="openmath">
    If $S$ is an OpenMath symbol and $A_1,...,A_n\;(n\geq0)$ are OpenMath objects or
    derived OpenMath objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an OpenMath error object.
    </div> */
    OME {
        cdbase: Option<Cow<'de, str>>,
        cd: Cow<'de, str>,
        name: Cow<'de, str>,
        arguments: Vec<Either<I, OMMaybeForeign<'de, Self>>>,
        attrs: Vec<OMAttr<'de, I>>,
    } = OMKind::OME as _,
}

impl<'d> OMDeserializable<'d> for crate::Int<'d> {
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMI { int, .. } = om {
            Ok(Left(int))
        } else {
            Err("Not an integer")
        }
    }
}

impl<'d> OMDeserializable<'d> for f32 {
    type Err = &'static str;
    #[allow(clippy::cast_possible_truncation)]
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMF { float, .. } = om {
            Ok(Left(float as _))
        } else {
            Err("Not a float")
        }
    }
}

impl<'d> OMDeserializable<'d> for f64 {
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMF { float, .. } = om {
            Ok(Left(float))
        } else {
            Err("Not a float")
        }
    }
}

impl<'d> OMDeserializable<'d> for Cow<'d, str> {
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMSTR { string, .. } = om {
            Ok(Left(string))
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d> OMDeserializable<'d> for String {
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMSTR { string, .. } = om {
            Ok(Left(string.into_owned()))
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d> OMDeserializable<'d> for Cow<'d, [u8]> {
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMB { bytes, .. } = om {
            Ok(Left(bytes))
        } else {
            Err("Not an OMB")
        }
    }
}
impl<'d> OMDeserializable<'d> for Vec<u8> {
    type Err = &'static str;
    fn from_openmath(om: OM<'d, Self>, _: &str) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMB { bytes, .. } = om {
            Ok(Left(bytes.into_owned()))
        } else {
            Err("Not an OMB")
        }
    }
}

// Implement for integer types by converting to Int
macro_rules! impl_int_deserializable {
    ($($t:ty=$err:literal),*) => {
        $(
            impl<'d> OMDeserializable<'d> for $t {
                type Err = &'static str;
                fn from_openmath(
                    om: OM<'d, Self>,
                    _: &str
                ) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
                where
                    Self: Sized,
                {
                    if let OM::OMI{int,..} = om {
                        int.is_i128().map_or(Err($err), |i| {
                            i.try_into().map(Left).map_err(|_| $err)
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
    use crate::Int;

    // Test types for deserialization
    #[derive(Debug, PartialEq, Clone)]
    struct TestInt(i64);

    impl<'de> OMDeserializable<'de> for TestInt {
        type Err = String;

        fn from_openmath(
            om: OM<'de, Self>,
            _: &str,
        ) -> Result<Either<Self, OM<'de, Self>>, Self::Err> {
            match om {
                OM::OMI { int, attrs } => {
                    if let Some(i) = int.is_i128() {
                        if i >= i64::MIN.into() && i <= i64::MAX.into() {
                            #[allow(clippy::cast_possible_truncation)]
                            Ok(Either::Left(Self(i as i64)))
                        } else {
                            // Return the original value instead of error for too large integers
                            Ok(Either::Right(OM::OMI { int, attrs }))
                        }
                    } else {
                        // Big integer - can't fit in i64
                        Ok(Either::Right(OM::OMI { int, attrs }))
                    }
                }
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestFloat(f64);

    impl<'de> OMDeserializable<'de> for TestFloat {
        type Err = String;

        fn from_openmath(
            om: OM<'de, Self>,
            _: &str,
        ) -> Result<Either<Self, OM<'de, Self>>, Self::Err> {
            match om {
                OM::OMF { float, .. } if float.is_finite() => Ok(Either::Left(Self(float))),
                OM::OMF { float, .. } => Err(format!("Non-finite float: {float}")),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestString(String);

    impl<'de> OMDeserializable<'de> for TestString {
        type Err = String;

        fn from_openmath(
            om: OM<'de, Self>,
            _: &str,
        ) -> Result<Either<Self, OM<'de, Self>>, Self::Err> {
            match om {
                OM::OMSTR { string, .. } => Ok(Either::Left(Self(string.to_string()))),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestVariable(String);

    impl<'de> OMDeserializable<'de> for TestVariable {
        type Err = String;

        fn from_openmath(
            om: OM<'de, Self>,
            _: &str,
        ) -> Result<Either<Self, OM<'de, Self>>, Self::Err> {
            match om {
                OM::OMV { name, .. } => Ok(Either::Left(Self(name.to_string()))),
                other => Ok(Either::Right(other)),
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
        type Err = String;

        fn from_openmath(
            om: OM<'de, Self>,
            cdbase: &str,
        ) -> Result<Either<Self, OM<'de, Self>>, Self::Err> {
            match om {
                OM::OMS { cd, name, .. } => Ok(Either::Left(Self {
                    cdbase: cdbase.to_string(),
                    cd: cd.to_string(),
                    name: name.to_string(),
                })),
                other => Ok(Either::Right(other)),
            }
        }
    }

    // Test for owned deserialization
    #[derive(Debug, PartialEq, Clone)]
    struct OwnedTestString(String);

    impl<'d> OMDeserializable<'d> for OwnedTestString {
        type Err = String;

        fn from_openmath(
            om: OM<'d, Self>,
            _: &str,
        ) -> Result<Either<Self, OM<'d, Self>>, Self::Err> {
            match om {
                OM::OMSTR { string, .. } => Ok(Either::Left(Self(string.into_owned()))),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[test]
    fn test_omi_deserialization_success() {
        let int = Int::from(42);
        let om = OM::OMI::<'static, TestInt> {
            int,
            attrs: Vec::new(),
        };

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_int) => assert_eq!(test_int, TestInt(42)),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omi_deserialization_too_large() {
        let big_int = Int::new("123456789012345678901234567890").expect("should be defined");
        let om: OM<TestInt> = OM::OMI {
            int: big_int.clone(),
            attrs: Vec::new(),
        };

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail"),
            Either::Right(OM::OMI {
                int: returned_int, ..
            }) => {
                assert_eq!(returned_int.is_big(), big_int.is_big());
            }
            Either::Right(_) => panic!("Expected OMI to be returned"),
        }
    }

    #[test]
    fn test_omi_deserialization_i128_max() {
        let int_val = Int::from(i128::MAX);
        let om: OM<TestInt> = OM::OMI {
            int: int_val.clone(),
            attrs: Vec::new(),
        };

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail for i128::MAX"),
            Either::Right(OM::OMI {
                int: returned_int, ..
            }) => {
                assert_eq!(returned_int.is_i128(), int_val.is_i128());
            }
            Either::Right(_) => panic!("Expected OMI to be returned"),
        }
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_omf_deserialization_success() {
        let om = OM::OMF::<'_, TestFloat> {
            float: 3.14159,
            attrs: Vec::new(),
        };

        let result = TestFloat::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_float) => assert_eq!(test_float, TestFloat(3.14159)),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omf_deserialization_infinity() {
        let om = OM::OMF::<'_, TestFloat> {
            float: f64::INFINITY,
            attrs: Vec::new(),
        };

        let result = TestFloat::from_openmath(om, crate::OPENMATH_BASE_URI.as_str());
        match result {
            Err(e) => assert!(e.contains("Non-finite")),
            Ok(_) => panic!("Expected error for infinity"),
        }
    }

    #[test]
    fn test_omstr_deserialization() {
        let om = OM::OMSTR::<'_, TestString> {
            string: Cow::Borrowed("hello world"),
            attrs: Vec::new(),
        };

        let result = TestString::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_string) => {
                assert_eq!(test_string, TestString("hello world".to_string()));
            }
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omv_deserialization() {
        let om = OM::OMV::<TestVariable> {
            name: Cow::Borrowed("x"),
            attrs: Vec::new(),
        };

        let result = TestVariable::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_var) => assert_eq!(test_var, TestVariable("x".to_string())),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_oms_deserialization() {
        let om: OM<TestSymbol> = OM::OMS {
            cd: Cow::Borrowed("arith1"),
            name: Cow::Borrowed("plus"),
            attrs: Vec::new(),
        };

        let result = TestSymbol::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_symbol) => assert_eq!(
                test_symbol,
                TestSymbol {
                    cdbase: "http://www.openmath.org/cd".to_string(),
                    cd: "arith1".to_string(),
                    name: "plus".to_string(),
                }
            ),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    #[allow(clippy::approx_constant)]
    #[allow(clippy::float_cmp)]
    fn test_wrong_type_deserialization() {
        // Try to deserialize a float as an integer
        let om = OM::OMF::<'_, TestInt> {
            float: 3.14,
            attrs: Vec::new(),
        };

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail"),
            Either::Right(OM::OMF { float, .. }) => assert_eq!(float, 3.14f64),
            Either::Right(_) => panic!("Expected OMF to be returned"),
        }
    }

    #[test]
    fn test_owned_deserialization() {
        let om = OM::OMSTR::<'_, OwnedTestString> {
            string: Cow::Owned("owned string".to_string()),
            attrs: Vec::new(),
        };

        let result = <OwnedTestString as OMDeserializable<'static>>::from_openmath(
            om,
            crate::OPENMATH_BASE_URI.as_str(),
        )
        .expect("should be defined");
        match result {
            Either::Left(owned_string) => {
                assert_eq!(owned_string, OwnedTestString("owned string".to_string()));
            }
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_oma_deserialization() {
        #[derive(Copy, Clone, Debug)]
        struct Oma;
        impl<'d> OMDeserializable<'d> for Oma {
            type Err = String;
            fn from_openmath(
                om: OM<'d, Self>,
                _cdbase: &str,
            ) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
            where
                Self: Sized,
            {
                match om {
                    OM::OMA { .. } => Ok(Either::Left(Self)),
                    o => Ok(Either::Right(o)),
                }
            }
        }
        let _ = tracing_subscriber::fmt().try_init();
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

    #[cfg(feature = "serde")]
    #[test]
    fn test_oma_deserialization_borrowed() {
        #[derive(Copy, Clone, Debug)]
        struct Oma;
        impl<'d> OMDeserializable<'d> for Oma {
            type Err = String;
            fn from_openmath(
                om: OM<'d, Self>,
                _cdbase: &str,
            ) -> Result<Either<Self, OM<'d, Self>>, Self::Err>
            where
                Self: Sized,
            {
                match om {
                    OM::OMA { .. } => Ok(Either::Left(Self)),
                    o => Ok(Either::Right(o)),
                }
            }
        }
        let _ = tracing_subscriber::fmt().try_init();
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
    }
}
