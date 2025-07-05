/*! OpenMath Deserialization; [OMDeserializable] and related types
*/

//#[cfg(feature = "serde")]
//pub(crate) mod serde_aux;
#[cfg(feature = "serde")]
pub(crate) mod serde_impl;
#[cfg(feature = "xml")]
pub(crate) mod xml;
use std::borrow::Cow;

use crate::{OMKind, either::Either};
use either::Either::Left;
#[cfg(feature = "serde")]
pub use serde_impl::{OMFromSerde, OMFromSerdeOwned};

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
  and calling [`take()`](serde_impl::OMFromSerde::take) on the result
  to get the `MyType`. (requires the `serde`-feature)
  If the last call to [`from_openmath`](OMDeserializable::from_openmath) is
  not a full <code>[Ok](Result::Ok)(MyType)</code>, serde deserialization will return
  an error already.
  The implementation follows the official OpenMath JSON encoding[^1], so using
  [`serde_json`](https://docs.rs/serde_json) allows for deserializing specification-compliant
  JSON.

## Parameters
- `'de`: The lifetime of the deserialized data; tied to the e.g. string from which it gets
  serialized. If `Self` should be entirely owned, implement [`OMDeserializableOwned`]
  instead; which provides a blanket implementation for <code>[OMDeserializableOwned]<'static,[Vec]<[u8]>,[String]></code>
- `Arr`: The type used for byte arrays (default: `&'de [u8]`)
- `Str`: The type used for strings (default: `&'de str`)

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
impl<'d> OMDeserializable<'d, Cow<'d,[u8]>, &'d str> for SimplifiedInt {
    type Err = String;
    fn from_openmath(
        om: OM<'d, Self, Cow<'d,[u8]>, &'d str>,
        cd_base:&str
    ) -> Result<Either<Self, OM<'d, Self, Cow<'d,[u8]>, &'d str>>, Self::Err>
    where
        Self: Sized,
    {
        match om {
            // An integer
            OM::OMI(i) => {
                // ...which fits in an i128
                if let Some(i) = i.is_i128() {
                    Ok(Either::Left(Self(i)))
                } else {
                    Err(format!("Invalid int value: {i}"))
                }
            }
            // Addition or multiplication
            t @ OM::OMS {
                cd_name: "arith1",
                name: "plus" | "times",
            } if cd_base == openmath::OPENMATH_BASE_URI.as_str() => {
                // works, but without arguments, we can't do anything to it *yet*.
                // => We send it back, so we can take care of it later, if it
                // occurs as the head of an OMA expression
                Ok(either::Right(t))
            }
            // some operator application to two arguments
            OM::OMA {
                // still an open math expression:
                head: either::Right(op),
                mut args,
            } if args.iter().all(Either::is_left)
                && args.len() == 2
                && cd_base == openmath::OPENMATH_BASE_URI.as_str() => {
                // An OMA only ends up here, after both the head and all arguments
                // were fead into this method.
                // Since "plus" and "times" are the only values for
                // which we return `either::Right`, we know the following matches:
                let is_times = match *op {
                    OM::OMS {
                        cd_name: "arith1",
                        name: "plus",
                    } => false,
                    OM::OMS {
                        cd_name: "arith1",
                        name: "times",
                    } => false,
                    _ => unreachable!(),
                };
                let Some(Either::Left(arg2)) = args.pop() else {
                    unreachable!()
                };
                let Some(Either::Left(arg1)) = args.pop() else {
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
assert_eq!(r.take().0, 4);
# }
```

[^1]: <https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_json-the-json-encoding>
*/
pub trait OMDeserializable<'de, Arr = Cow<'de, [u8]>, Str = &'de str>: std::fmt::Debug
where
    Arr: Bytes<'de>,
    Str: StringLike<'de>,
{
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
        om: OM<'de, Self, Arr, Str>,
        cd_base: &str,
    ) -> Result<Either<Self, OM<'de, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized;
}
/// Trait for types that can be deserialized as owned values OpenMath objects.
///
/// This is a specialized version of [`OMDeserializable`] for cases where you
/// need owned data (`String` and `Vec<u8>`) rather than borrowed data. This
/// is useful when the deserialized object needs to outlive the source data.
///
/// Also provides blanket implementations for [`OMDeserializable`].
pub trait OMDeserializableOwned: std::fmt::Debug {
    /// The type of errors that can occur during deserialization.
    type Err: std::fmt::Display;

    /// Attempt to deserialize an owned OpenMath object into this type.
    ///
    /// Similar to [`OMDeserializable::from_openmath`] but works with owned
    /// data types ([`String`], <code>[Vec]<[u8]></code>) instead of borrowed ones.
    ///
    /// # Errors
    /// This method examines the provided OpenMath object and either:
    #[allow(rustdoc::redundant_explicit_links)]
    /// 1. Successfully converts it to the target type (returns <code>[Ok](Result::Ok)([Left](either::Either::Left)(T))</code>)
    /// 2. Determines it cannot be converted *yet*, but maybe later in an OMA or OMBIND, and returns the
    #[allow(rustdoc::redundant_explicit_links)]
    ///    original object (<code>[Ok](Result::Ok)([Right](either::Either::Right)(om))</code>)
    /// 3. Encounters an error during processing ([`Err`])
    #[allow(clippy::type_complexity)]
    fn from_openmath<'d>(
        om: OM<'d, Self, Vec<u8>, String>,
        cd_base: &str,
    ) -> Result<Either<Self, OM<'d, Self, Vec<u8>, String>>, Self::Err>
    where
        Self: Sized;
}

/// Blanket implementation to allow owned deserializable types to work with the borrowed trait.
///
/// This implementation allows any type that implements [`OMDeserializableOwned`]
/// to automatically work with the [`OMDeserializable`] trait when using owned
/// data types.
impl<'d, O: OMDeserializableOwned> OMDeserializable<'d, Vec<u8>, String> for O {
    type Err = <Self as OMDeserializableOwned>::Err;
    #[inline]
    fn from_openmath(
        om: OM<'d, Self, Vec<u8>, String>,
        cd_base: &str,
    ) -> Result<Either<Self, OM<'d, Self, Vec<u8>, String>>, Self::Err>
    where
        Self: Sized,
    {
        <Self as OMDeserializableOwned>::from_openmath(om, cd_base)
    }
}

/// Wrapper to deserialize an OMOBJ
pub struct OMObject<'de, O: OMDeserializable<'de, Arr, Str>, Arr = Cow<'de, [u8]>, Str = &'de str>(
    O,
    std::marker::PhantomData<&'de (Arr, Str)>,
)
where
    Arr: Bytes<'de>,
    Str: StringLike<'de>;
impl<'de, O: OMDeserializable<'de>> OMObject<'de, O> {
    #[inline]
    pub fn take(self) -> O {
        self.0
    }
}

/// Enum for deserializing from OpenMath. See
/// see [OMDeserializable] for documentation and an example
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum OM<'de, I, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: crate::de::Bytes<'de>,
    Str: crate::de::StringLike<'de>,
    I: OMDeserializable<'de, Arr, Str>,
{
    /** <div class="openmath">
    Integers in the mathematical sense, with no predefined range.
    They are “infinite precision” integers (also called “bignums” in computer algebra).
    </div> */
    OMI(crate::Int<'de>) = OMKind::OMI as _,

    /** <div class="openmath">
    Double precision floating-point numbers following the IEEE 754-1985 standard.
    </div> */
    OMF(f64) = OMKind::OMF as _,

    /** <div class="openmath">
    A Unicode Character string. This also corresponds to “characters” in XML.
    </div> */
    OMSTR(Str) = OMKind::OMSTR as _,

    /** <div class="openmath">
    A sequence of bytes.
    </div> */
    OMB(Arr) = OMKind::OMB as _,

    ///<div class="openmath">
    ///
    /// A Variable must have a name which is a sequence of characters matching a regular
    /// expression, as described in [Section 2.3](https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_names).
    ///
    ///</div>
    ///
    ///(Note: We do not enforce that names are valid XML names;)
    OMV(Str) = OMKind::OMV as _,

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
    OMS { cd_name: Str, name: Str } = OMKind::OMS as _,

    /** <div class="openmath">
    If $A_1,...,A_n\;(n>0)$ are OpenMath objects, then
    $\mathrm{application}(A_1,...,A_n)$ is an OpenMath application object.
    We call $A_1$ the function and $A_2$ to $A_n$ the arguments.
    </div> */
    OMA {
        head: Either<I, Box<Self>>,
        args: Vec<Either<I, Self>>,
    } = OMKind::OMA as _,

    /** <div class="openmath">
    If $B$ and $C$ are OpenMath objects, and $v_1,...,v_n\;(n\geq0)$
    are OpenMath variables or attributed variables, then
    $\mathrm{binding}(B,v_1,...,v_n,C)$ is an OpenMath binding object.
    $B$ is called the binder, $v_1,...,v_n$ are called variable bindings, and
    $C$ is called the body of the binding object above.
    </div> */
    OMBIND {
        head: Either<I, Box<Self>>,
        context: Vec<Str>,
        body: Either<I, Box<Self>>,
    } = OMKind::OMBIND as _,

    /** <div class="openmath">
    If $S$ is an OpenMath symbol and $A_1,...,A_n\;(n\geq0)$ are OpenMath objects or
    derived OpenMath objects, then $\mathrm{error}(S,A_1,...,A_n)$ is an OpenMath error object.
    </div> */
    OME {
        cd_base: Option<Str>,
        cd_name: Str,
        name: Str,
        args: Vec<Either<I, OMForeign<'de, I, Arr, Str>>>,
    } = OMKind::OME as _,
}

#[derive(Debug, Clone)]
pub enum OMForeign<'de, I, Arr = Cow<'de, [u8]>, Str = &'de str>
where
    Arr: crate::de::Bytes<'de>,
    Str: crate::de::StringLike<'de>,
    I: OMDeserializable<'de, Arr, Str>,
{
    OM(OM<'de, I, Arr, Str>),

    /** <div class="openmath">
    If $A$ is not an OpenMath object, then $\mathrm{foreign}(A)$ is an OpenMath foreign object.
    An OpenMath foreign object may optionally have an encoding field which describes how its
    contents should be interpreted.
    </div> */
    Foreign {
        encoding: Option<Str>,
        value: Str,
    },
}

/// Type alias for owned OpenMath objects.
///
/// This is a convenience type alias for [`OpenMath`] objects that own their
/// data (using `String` and `Vec<u8>` instead of borrowed slices).
///
/// ```
pub type OMDeserOwned<I> = OM<'static, I, Vec<u8>, String>;

mod hidden {
    use std::borrow::Cow;

    #[cfg(feature = "serde")]
    pub trait SealedStr<'s>:
        std::fmt::Debug + std::ops::Deref<Target = str> + serde::Deserialize<'s> + 's
    {
    }
    #[cfg(not(feature = "serde"))]
    pub trait SealedStr<'s>: std::fmt::Debug + std::ops::Deref<Target = str> + 's {}
    #[cfg(feature = "serde")]
    pub trait SealedB<'s>:
        std::fmt::Debug + std::ops::Deref<Target = [u8]> + serde::Deserialize<'s> + 's
    {
    }
    #[cfg(not(feature = "serde"))]
    pub trait SealedB<'s>: std::fmt::Debug + std::ops::Deref<Target = [u8]> + 's {}
    impl<'s> SealedStr<'s> for &'s str {}
    impl SealedStr<'_> for String {}
    impl<'s> SealedB<'s> for Cow<'s, [u8]> {}
    impl SealedB<'_> for Vec<u8> {}
}

/// Trait for string-like types that can be used in OpenMath deserialization.
///
/// This trait allows the library to work with both borrowed (`&str`) and owned
/// (`String`) string types in a uniform way. It's sealed to prevent external
/// implementation.
///
/// # Implementations
/// - `&str`: For zero-copy deserialization from borrowed data
/// - `String`: For owned deserialization when data needs to outlive the source
pub trait StringLike<'s>: hidden::SealedStr<'s> + std::fmt::Display + Clone {
    fn split_uri(self) -> Option<(Self, Self, Self)>;
    fn into_int(self) -> Option<crate::Int<'s>>;
}
impl<'s> StringLike<'s> for &'s str {
    fn split_uri(self) -> Option<(Self, Self, Self)> {
        let (bcd, name) = self.rsplit_once(['#', '/'])?;
        let (base, cd) = bcd.rsplit_once('/')?;
        Some((base, cd, name))
    }
    #[inline]
    fn into_int(self) -> Option<crate::Int<'s>> {
        crate::Int::new(self)
    }
}
impl StringLike<'_> for String {
    fn split_uri(mut self) -> Option<(Self, Self, Self)> {
        let i = self.rfind(['#', '/'])?;
        let name = self.split_off(i);
        let i = self.rfind('/')?;
        let cd = self.split_off(i);
        Some((self, cd, name))
    }
    #[inline]
    fn into_int(self) -> Option<crate::Int<'static>> {
        crate::Int::from_string(self)
    }
}

/// Trait for byte array types that can be used in OpenMath deserialization.
///
/// This trait allows the library to work with both borrowed (`&[u8]`) and owned
/// (`Vec<u8>`) byte array types in a uniform way. It's sealed to prevent external
/// implementation.
///
/// # Implementations
/// - `&[u8]`: For zero-copy deserialization from borrowed data
/// - `Vec<u8>`: For owned deserialization when data needs to outlive the source
pub trait Bytes<'b>: hidden::SealedB<'b> + From<Vec<u8>> {}
impl<'b> Bytes<'b> for Cow<'b, [u8]> {}
impl Bytes<'_> for Vec<u8> {}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for crate::Int<'d> {
    type Err = &'static str;
    fn from_openmath(
        om: OM<'d, Self, Arr, Str>,
        _: &str,
    ) -> Result<Either<Self, OM<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMI(i) = om {
            Ok(Left(i))
        } else {
            Err("Not an integer")
        }
    }
}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for f32 {
    type Err = &'static str;
    #[allow(clippy::cast_possible_truncation)]
    fn from_openmath(
        om: OM<'d, Self, Arr, Str>,
        _: &str,
    ) -> Result<Either<Self, OM<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMF(f) = om {
            Ok(Left(f as _))
        } else {
            Err("Not a float")
        }
    }
}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for f64 {
    type Err = &'static str;
    fn from_openmath(
        om: OM<'d, Self, Arr, Str>,
        _: &str,
    ) -> Result<Either<Self, OM<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMF(f) = om {
            Ok(Left(f))
        } else {
            Err("Not a float")
        }
    }
}

impl<'d, Arr: Bytes<'d>> OMDeserializable<'d, Arr, &'d str> for &'d str {
    type Err = &'static str;
    fn from_openmath(
        om: OM<'d, Self, Arr, &'d str>,
        _: &str,
    ) -> Result<Either<Self, OM<'d, Self, Arr, &'d str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMSTR(s) = om {
            Ok(Left(s))
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d, Arr: Bytes<'d>> OMDeserializable<'d, Arr, Self> for String {
    type Err = &'static str;
    fn from_openmath(
        om: OM<'d, Self, Arr, Self>,
        _: &str,
    ) -> Result<Either<Self, OM<'d, Self, Arr, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMSTR(s) = om {
            Ok(Left(s))
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for Arr {
    type Err = &'static str;
    fn from_openmath(
        om: OM<'d, Self, Arr, Str>,
        _: &str,
    ) -> Result<Either<Self, OM<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OM::OMB(b) = om {
            Ok(Left(b))
        } else {
            Err("Not an OMB")
        }
    }
}

// Implement for integer types by converting to Int
macro_rules! impl_int_deserializable {
    ($($t:ty=$err:literal),*) => {
        $(
            impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for $t {
                type Err = &'static str;
                fn from_openmath(
                    om: OM<'d, Self, Arr, Str>,
                    _: &str,
                ) -> Result<Either<Self, OM<'d, Self, Arr, Str>>, Self::Err>
                where
                    Self: Sized,
                {
                    if let OM::OMI(i) = om {
                        i.is_i128().map_or(Err($err), |i| {
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
                OM::OMI(int_val) => {
                    if let Some(i) = int_val.is_i128() {
                        if i >= i64::MIN.into() && i <= i64::MAX.into() {
                            #[allow(clippy::cast_possible_truncation)]
                            Ok(Either::Left(Self(i as i64)))
                        } else {
                            // Return the original value instead of error for too large integers
                            Ok(Either::Right(OM::OMI(int_val)))
                        }
                    } else {
                        // Big integer - can't fit in i64
                        Ok(Either::Right(OM::OMI(int_val)))
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
                OM::OMF(f) if f.is_finite() => Ok(Either::Left(Self(f))),
                OM::OMF(f) => Err(format!("Non-finite float: {f}")),
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
                OM::OMSTR(s) => Ok(Either::Left(Self(s.to_string()))),
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
                OM::OMV(name) => Ok(Either::Left(Self(name.to_string()))),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestSymbol {
        cd_base: String,
        cd: String,
        name: String,
    }

    impl<'de> OMDeserializable<'de> for TestSymbol {
        type Err = String;

        fn from_openmath(
            om: OM<'de, Self>,
            cd_base: &str,
        ) -> Result<Either<Self, OM<'de, Self>>, Self::Err> {
            match om {
                OM::OMS { cd_name: cd, name } => Ok(Either::Left(Self {
                    cd_base: cd_base.to_string(),
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

    impl OMDeserializableOwned for OwnedTestString {
        type Err = String;

        fn from_openmath<'d>(
            om: OM<'d, Self, Vec<u8>, String>,
            _: &str,
        ) -> Result<Either<Self, OM<'d, Self, Vec<u8>, String>>, Self::Err> {
            match om {
                OM::OMSTR(s) => Ok(Either::Left(Self(s))),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[test]
    fn test_omi_deserialization_success() {
        let int_val = Int::from(42);
        let om = OM::<TestInt>::OMI(int_val);

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
        let om: OM<TestInt> = OM::OMI(big_int.clone());

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail"),
            Either::Right(OM::OMI(returned_int)) => {
                assert_eq!(returned_int.is_big(), big_int.is_big());
            }
            Either::Right(_) => panic!("Expected OMI to be returned"),
        }
    }

    #[test]
    fn test_omi_deserialization_i128_max() {
        let int_val = Int::from(i128::MAX);
        let om: OM<TestInt> = OM::OMI(int_val.clone());

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail for i128::MAX"),
            Either::Right(OM::OMI(returned_int)) => {
                assert_eq!(returned_int.is_i128(), int_val.is_i128());
            }
            Either::Right(_) => panic!("Expected OMI to be returned"),
        }
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_omf_deserialization_success() {
        let om = OM::<TestFloat>::OMF(3.14159);

        let result = TestFloat::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_float) => assert_eq!(test_float, TestFloat(3.14159)),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omf_deserialization_infinity() {
        let om = OM::<TestFloat>::OMF(f64::INFINITY);

        let result = TestFloat::from_openmath(om, crate::OPENMATH_BASE_URI.as_str());
        match result {
            Err(e) => assert!(e.contains("Non-finite")),
            Ok(_) => panic!("Expected error for infinity"),
        }
    }

    #[test]
    fn test_omstr_deserialization() {
        let om = OM::<TestString>::OMSTR("hello world");

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
        let om = OM::<TestVariable>::OMV("x");

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
            cd_name: "arith1",
            name: "plus",
        };

        let result = TestSymbol::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(test_symbol) => assert_eq!(
                test_symbol,
                TestSymbol {
                    cd_base: "http://www.openmath.org/cd".to_string(),
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
        let om = OM::<TestInt>::OMF(3.14);

        let result = TestInt::from_openmath(om, crate::OPENMATH_BASE_URI.as_str())
            .expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail"),
            Either::Right(OM::OMF(f)) => assert_eq!(f, 3.14f64),
            Either::Right(_) => panic!("Expected OMF to be returned"),
        }
    }

    #[test]
    fn test_owned_deserialization() {
        let om = OM::<OwnedTestString, Vec<u8>, String>::OMSTR("owned string".to_string());

        let result = <OwnedTestString as OMDeserializableOwned>::from_openmath(
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

    #[test]
    fn test_blanket_impl_for_owned() {
        // Test that OMDeserializableOwned types work with the borrowed trait
        let om: OM<OwnedTestString, Vec<u8>, String> = OM::OMSTR("test".to_string());

        // This should work due to the blanket impl
        let result = <OwnedTestString as OMDeserializableOwned>::from_openmath(
            om,
            crate::OPENMATH_BASE_URI.as_str(),
        )
        .expect("should be defined");
        match result {
            Either::Left(owned) => assert_eq!(owned.0, "test"),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }
}
