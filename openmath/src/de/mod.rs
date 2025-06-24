/*! OpenMath Deserialization; [OMDeserializable] and related types
*/

#[cfg(feature = "serde")]
pub(crate) mod serde_aux;
#[cfg(feature = "serde")]
pub(crate) mod serde_impl;
use crate::either::Either;
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
- **Serde-based** Deserialize from any serde-compatible format by deserializing
  an <code>[OMFromSerde](serde_impl::OMFromSerde)<'d,MyType></code> instead,
  and calling [`take()`](serde_impl::OMFromSerde::take) on the result
  to get the `MyType`. (requires the `serde`-feature)
  If the last call to [`from_openmath`](OMDeserializable::from_openmath) is
  not a full <code>[Ok](Result::Ok)(MyType)</code>, serde deserialization will return
  an error already.

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
use openmath::de::{OMDeserializable, OMFromSerde, OpenMath};
use openmath::either::Either;

#[derive(Copy, Clone, Debug)]
struct SimplifiedInt(i128);
impl<'d> OMDeserializable<'d, &'d [u8], &'d str> for SimplifiedInt {
    type Err = String;
    fn from_openmath(
        om: OpenMath<'d, Self, &'d [u8], &'d str>,
    ) -> Result<Either<Self, OpenMath<'d, Self, &'d [u8], &'d str>>, Self::Err>
    where
        Self: Sized,
    {
        match om {
            // An integer
            OpenMath::OMI(i) => {
                // ...which fits in an i128
                if let Some(i) = i.is_i128() {
                    Ok(Either::Left(Self(i)))
                } else {
                    Err(format!("Invalid int value: {i}"))
                }
            }
            // Addition or multiplication
            t @ OpenMath::OMS {
                cd_base: "http://openmath.org",
                cd_name: "arith1",
                name: "plus" | "times",
            } => {
                // works, but without arguments, we can't do anything to it *yet*.
                // => We send it back, so we can take care of it later, if it
                // occurs as the head of an OMA expression
                Ok(either::Right(t))
            }
            // some operator application to two arguments
            OpenMath::OMA {
                // still an open math expression:
                head: either::Right(op),
                mut args,
            } if args.iter().all(Either::is_left) && args.len() == 2 => {
                // An OMA only ends up here, after both the head and all arguments
                // were fead into this method.
                // Since "plus" and "times" are the only values for
                // which we return `either::Right`, we know the following matches:
                let is_times = match *op {
                    OpenMath::OMS {
                        cd_base: "http://openmath.org",
                        cd_name: "arith1",
                        name: "plus",
                    } => false,
                    OpenMath::OMS {
                        cd_base: "http://openmath.org",
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
let s = r#"{ "OMA": [
    { "OMS": "http://openmath.org/arith1#plus" },
    { "OMI":2 },
    { "OMI":2 }
] }"#;
let r = serde_json::from_str::<'_, OMFromSerde<SimplifiedInt>>(s)
    .expect("valid json, openmath, and arithmetic expression");
assert_eq!(r.take().0, 4);
# }
```
*/
pub trait OMDeserializable<'de, Arr = &'de [u8], Str = &'de str>: std::fmt::Debug
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
        om: OpenMath<'de, Self, Arr, Str>,
    ) -> Result<Either<Self, OpenMath<'de, Self, Arr, Str>>, Self::Err>
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
    fn from_openmath(
        om: OpenMath<'_, Self, Vec<u8>, String>,
    ) -> Result<Either<Self, OpenMath<'_, Self, Vec<u8>, String>>, Self::Err>
    where
        Self: Sized;
}

/// Blanket implementation to allow owned deserializable types to work with the borrowed trait.
///
/// This implementation allows any type that implements [`OMDeserializableOwned`]
/// to automatically work with the [`OMDeserializable`] trait when using owned
/// data types.
impl<O: OMDeserializableOwned> OMDeserializable<'_, Vec<u8>, String> for O {
    type Err = <Self as OMDeserializableOwned>::Err;
    #[inline]
    fn from_openmath(
        om: OpenMath<'_, Self, Vec<u8>, String>,
    ) -> Result<Either<Self, OpenMath<'_, Self, Vec<u8>, String>>, Self::Err>
    where
        Self: Sized,
    {
        <Self as OMDeserializableOwned>::from_openmath(om)
    }
}

/// Enum representing all possible OpenMath objects.
///
/// This enum encompasses the complete OpenMath object model, providing variants
/// for each type of mathematical object that can be represented in OpenMath.
///
/// # Type Parameters
/// - `'de`: Lifetime of the deserialized data
/// - `I`: The type that implements [`OMDeserializable`] (your target type)
/// - `Arr`: Type for byte arrays (default: `&'de [u8]`)
/// - `Str`: Type for strings (default: `&'de str`)
///
/// # Variants
///
/// ## Basic Objects
/// - [`OMI`](Self::OMI): Arbitrary precision integers
/// - [`OMF`](Self::OMF): IEEE 754 double precision floating point numbers
/// - [`OMSTR`](Self::OMSTR): String literals
/// - [`OMB`](Self::OMB): Binary data (byte arrays)
/// - [`OMV`](Self::OMV): Variables (identifiers)
/// - [`OMS`](Self::OMS): Symbols from Content Dictionaries
///
/// ## Compound Objects
/// - [`OMA`](Self::OMA): Applications (function calls)
/// - [`OMBIND`](Self::OMBIND): Binding constructs (quantifiers, lambda expressions)
///
/// # Examples
///
/// ## Pattern Matching
///
/// ```rust
/// use openmath::{de::OpenMath, Int};
///
/// # #[derive(Debug)]
/// # struct MyType;
/// # impl<'de> openmath::de::OMDeserializable<'de> for MyType {
/// #     type Err = u8;
/// #     fn from_openmath(om: OpenMath<'de, Self>) -> Result<openmath::either::Either<Self, OpenMath<'de, Self>>, Self::Err> {
/// #         Ok(openmath::either::Either::Right(om))
/// #     }
/// # }
/// fn process_openmath(om: OpenMath<MyType>) {
///     match om {
///         OpenMath::OMI(int_val) => {
///             println!("Integer: {:?}", int_val);
///         }
///         OpenMath::OMF(float_val) => {
///             println!("Float: {}", float_val);
///         }
///         OpenMath::OMSTR(string_val) => {
///             println!("String: {}", string_val);
///         }
///         OpenMath::OMS { cd_base, cd_name, name } => {
///             println!("Symbol: {}#{}.{}", cd_base, cd_name, name);
///         }
///         OpenMath::OMA { head, args } => {
///             println!("Application with {} arguments", args.len());
///         }
///         _ => {
///             println!("Other OpenMath object");
///         }
///     }
/// }
/// ```
///
/// ## Mathematical Expressions
///
/// The enum can represent complex mathematical expressions:
///
/// - **Arithmetic**: `2 + 3` becomes `OMA { head: plus_symbol, args: [OMI(2), OMI(3)] }`
/// - **Functions**: `sin(π)` becomes `OMA { head: sin_symbol, args: [OMS{pi_symbol}] }`
/// - **Quantification**: `∀x: P(x)` becomes `OMBIND { head: forall_symbol, context: ["x"], body: P_application }`
#[derive(Debug, Clone, strum::EnumDiscriminants)]
#[strum_discriminants(vis(pub))]
#[strum_discriminants(name(OpenMathKind))]
#[strum_discriminants(derive(strum::VariantNames))]
#[cfg_attr(feature = "serde", strum_discriminants(derive(serde::Deserialize)))]
pub enum OpenMath<'de, I, Arr = &'de [u8], Str = &'de str>
where
    Arr: crate::de::Bytes<'de>,
    Str: crate::de::StringLike<'de>,
    I: OMDeserializable<'de, Arr, Str>,
{
    /// OpenMath integer (arbitrary precision)
    OMI(crate::Int<'de>),

    /// OpenMath floating point number (IEEE 754 double precision)
    OMF(f64),

    /// OpenMath string literal
    OMSTR(Str),

    /// OpenMath byte array (binary data)
    OMB(Arr),

    /// OpenMath variable (identifier)
    OMV(Str),

    /// OpenMath symbol from a Content Dictionary
    ///
    /// Contains:
    /// - `base`: Base URI of the Content Dictionary
    /// - `cd`: Content Dictionary name
    /// - `name`: Symbol name within the CD
    OMS {
        cd_base: Str,
        cd_name: Str,
        name: Str,
    },

    /// OpenMath application (function call)
    ///
    /// Represents `head(arg1, arg2, ..., argN)` where:
    /// - `head`: The function being applied (either a deserialized value or nested OpenMath)
    /// - `args`: List of arguments (each either deserialized or raw OpenMath)
    OMA {
        head: Either<I, Box<Self>>,
        args: Vec<Either<I, Self>>,
    },

    /// OpenMath binding construct
    ///
    /// Represents constructs that bind variables like quantifiers, lambda expressions, etc.
    /// - `head`: The binding operator (∀, ∃, λ, etc.)
    /// - `context`: List of variable names being bound
    /// - `body`: The expression in which variables are bound
    OMBIND {
        head: Either<I, Box<Self>>,
        context: Vec<Str>,
        body: Either<I, Box<Self>>,
    },
}
/// Type alias for owned OpenMath objects.
///
/// This is a convenience type alias for [`OpenMath`] objects that own their
/// data (using `String` and `Vec<u8>` instead of borrowed slices).
///
/// # Examples
///
/// ```rust
/// use openmath::de::OMDeserOwned;
///
/// # #[derive(Debug)]
/// # struct MyType;
/// # impl openmath::de::OMDeserializableOwned for MyType {
/// #     type Err = u8;
/// #     fn from_openmath(om: openmath::de::OpenMath<'_, Self, Vec<u8>, String>) -> Result<openmath::either::Either<Self, openmath::de::OpenMath<'_, Self, Vec<u8>, String>>, Self::Err> {
/// #         Ok(openmath::either::Either::Right(om))
/// #     }
/// # }
/// fn process_owned_openmath(om: OMDeserOwned<MyType>) {
///     // Process OpenMath object that owns its data
/// }
/// ```
pub type OMDeserOwned<I> = OpenMath<'static, I, Vec<u8>, String>;

mod hidden {
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
    impl<'s> SealedB<'s> for &'s [u8] {}
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
pub trait StringLike<'s>: hidden::SealedStr<'s> + std::fmt::Display {
    fn split_uri(self) -> Option<(Self, Self, Self)>;
}
impl<'s> StringLike<'s> for &'s str {
    fn split_uri(self) -> Option<(Self, Self, Self)> {
        let (bcd, name) = self.rsplit_once(['#', '/'])?;
        let (base, cd) = bcd.rsplit_once('/')?;
        Some((base, cd, name))
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
pub trait Bytes<'b>: hidden::SealedB<'b> {}
impl<'b> Bytes<'b> for &'b [u8] {}
impl Bytes<'_> for Vec<u8> {}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for crate::Int<'d> {
    type Err = &'static str;
    fn from_openmath(
        om: OpenMath<'d, Self, Arr, Str>,
    ) -> Result<Either<Self, OpenMath<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OpenMath::OMI(i) = om {
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
        om: OpenMath<'d, Self, Arr, Str>,
    ) -> Result<Either<Self, OpenMath<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OpenMath::OMF(f) = om {
            Ok(Left(f as _))
        } else {
            Err("Not a float")
        }
    }
}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for f64 {
    type Err = &'static str;
    fn from_openmath(
        om: OpenMath<'d, Self, Arr, Str>,
    ) -> Result<Either<Self, OpenMath<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OpenMath::OMF(f) = om {
            Ok(Left(f))
        } else {
            Err("Not a float")
        }
    }
}

impl<'d, Arr: Bytes<'d>> OMDeserializable<'d, Arr, &'d str> for &'d str {
    type Err = &'static str;
    fn from_openmath(
        om: OpenMath<'d, Self, Arr, &'d str>,
    ) -> Result<Either<Self, OpenMath<'d, Self, Arr, &'d str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OpenMath::OMSTR(s) = om {
            Ok(Left(s))
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d, Arr: Bytes<'d>> OMDeserializable<'d, Arr, Self> for String {
    type Err = &'static str;
    fn from_openmath(
        om: OpenMath<'d, Self, Arr, Self>,
    ) -> Result<Either<Self, OpenMath<'d, Self, Arr, Self>>, Self::Err>
    where
        Self: Sized,
    {
        if let OpenMath::OMSTR(s) = om {
            Ok(Left(s))
        } else {
            Err("Not an OMSTR")
        }
    }
}

impl<'d, Arr: Bytes<'d>, Str: StringLike<'d>> OMDeserializable<'d, Arr, Str> for Arr {
    type Err = &'static str;
    fn from_openmath(
        om: OpenMath<'d, Self, Arr, Str>,
    ) -> Result<Either<Self, OpenMath<'d, Self, Arr, Str>>, Self::Err>
    where
        Self: Sized,
    {
        if let OpenMath::OMB(b) = om {
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
                    om: OpenMath<'d, Self, Arr, Str>,
                ) -> Result<Either<Self, OpenMath<'d, Self, Arr, Str>>, Self::Err>
                where
                    Self: Sized,
                {
                    if let OpenMath::OMI(i) = om {
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
            om: OpenMath<'de, Self>,
        ) -> Result<Either<Self, OpenMath<'de, Self>>, Self::Err> {
            match om {
                OpenMath::OMI(int_val) => {
                    if let Some(i) = int_val.is_i128() {
                        if i >= i64::MIN.into() && i <= i64::MAX.into() {
                            #[allow(clippy::cast_possible_truncation)]
                            Ok(Either::Left(Self(i as i64)))
                        } else {
                            // Return the original value instead of error for too large integers
                            Ok(Either::Right(OpenMath::OMI(int_val)))
                        }
                    } else {
                        // Big integer - can't fit in i64
                        Ok(Either::Right(OpenMath::OMI(int_val)))
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
            om: OpenMath<'de, Self>,
        ) -> Result<Either<Self, OpenMath<'de, Self>>, Self::Err> {
            match om {
                OpenMath::OMF(f) if f.is_finite() => Ok(Either::Left(Self(f))),
                OpenMath::OMF(f) => Err(format!("Non-finite float: {f}")),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestString(String);

    impl<'de> OMDeserializable<'de> for TestString {
        type Err = String;

        fn from_openmath(
            om: OpenMath<'de, Self>,
        ) -> Result<Either<Self, OpenMath<'de, Self>>, Self::Err> {
            match om {
                OpenMath::OMSTR(s) => Ok(Either::Left(Self(s.to_string()))),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct TestVariable(String);

    impl<'de> OMDeserializable<'de> for TestVariable {
        type Err = String;

        fn from_openmath(
            om: OpenMath<'de, Self>,
        ) -> Result<Either<Self, OpenMath<'de, Self>>, Self::Err> {
            match om {
                OpenMath::OMV(name) => Ok(Either::Left(Self(name.to_string()))),
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
            om: OpenMath<'de, Self>,
        ) -> Result<Either<Self, OpenMath<'de, Self>>, Self::Err> {
            match om {
                OpenMath::OMS {
                    cd_base: base,
                    cd_name: cd,
                    name,
                } => Ok(Either::Left(Self {
                    cd_base: base.to_string(),
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

        fn from_openmath(
            om: OpenMath<'_, Self, Vec<u8>, String>,
        ) -> Result<Either<Self, OpenMath<'_, Self, Vec<u8>, String>>, Self::Err> {
            match om {
                OpenMath::OMSTR(s) => Ok(Either::Left(Self(s))),
                other => Ok(Either::Right(other)),
            }
        }
    }

    #[test]
    fn test_omi_deserialization_success() {
        let int_val = Int::from(42);
        let om = OpenMath::<TestInt>::OMI(int_val);

        let result = TestInt::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(test_int) => assert_eq!(test_int, TestInt(42)),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omi_deserialization_too_large() {
        let big_int = Int::new("123456789012345678901234567890").expect("should be defined");
        let om: OpenMath<TestInt> = OpenMath::OMI(big_int.clone());

        let result = TestInt::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail"),
            Either::Right(OpenMath::OMI(returned_int)) => {
                assert_eq!(returned_int.is_big(), big_int.is_big());
            }
            Either::Right(_) => panic!("Expected OMI to be returned"),
        }
    }

    #[test]
    fn test_omi_deserialization_i128_max() {
        let int_val = Int::from(i128::MAX);
        let om: OpenMath<TestInt> = OpenMath::OMI(int_val.clone());

        let result = TestInt::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail for i128::MAX"),
            Either::Right(OpenMath::OMI(returned_int)) => {
                assert_eq!(returned_int.is_i128(), int_val.is_i128());
            }
            Either::Right(_) => panic!("Expected OMI to be returned"),
        }
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_omf_deserialization_success() {
        let om = OpenMath::<TestFloat>::OMF(3.14159);

        let result = TestFloat::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(test_float) => assert_eq!(test_float, TestFloat(3.14159)),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omf_deserialization_infinity() {
        let om = OpenMath::<TestFloat>::OMF(f64::INFINITY);

        let result = TestFloat::from_openmath(om);
        match result {
            Err(e) => assert!(e.contains("Non-finite")),
            Ok(_) => panic!("Expected error for infinity"),
        }
    }

    #[test]
    fn test_omstr_deserialization() {
        let om = OpenMath::<TestString>::OMSTR("hello world");

        let result = TestString::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(test_string) => {
                assert_eq!(test_string, TestString("hello world".to_string()));
            }
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_omv_deserialization() {
        let om = OpenMath::<TestVariable>::OMV("x");

        let result = TestVariable::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(test_var) => assert_eq!(test_var, TestVariable("x".to_string())),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_oms_deserialization() {
        let om: OpenMath<TestSymbol> = OpenMath::OMS {
            cd_base: "http://openmath.org",
            cd_name: "arith1",
            name: "plus",
        };

        let result = TestSymbol::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(test_symbol) => assert_eq!(
                test_symbol,
                TestSymbol {
                    cd_base: "http://openmath.org".to_string(),
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
        let om = OpenMath::<TestInt>::OMF(3.14);

        let result = TestInt::from_openmath(om).expect("should be defined");
        match result {
            Either::Left(_) => panic!("Expected deserialization to fail"),
            Either::Right(OpenMath::OMF(f)) => assert_eq!(f, 3.14f64),
            Either::Right(_) => panic!("Expected OMF to be returned"),
        }
    }

    #[test]
    fn test_owned_deserialization() {
        let om = OpenMath::<OwnedTestString, Vec<u8>, String>::OMSTR("owned string".to_string());

        let result = <OwnedTestString as OMDeserializableOwned>::from_openmath(om)
            .expect("should be defined");
        match result {
            Either::Left(owned_string) => {
                assert_eq!(owned_string, OwnedTestString("owned string".to_string()));
            }
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }

    #[test]
    fn test_oma_structure() {
        // Create an OMA with a symbol head and integer arguments
        let head = Either::Right(Box::new(OpenMath::OMS {
            cd_base: "http://openmath.org",
            cd_name: "arith1",
            name: "plus",
        }));
        let args = vec![
            Either::Right(OpenMath::OMI(Int::from(1))),
            Either::Right(OpenMath::OMI(Int::from(2))),
        ];
        let om: OpenMath<TestInt> = OpenMath::OMA { head, args };

        // Test structure
        match om {
            OpenMath::OMA {
                head: Either::Right(head_box),
                args,
            } => {
                match head_box.as_ref() {
                    OpenMath::OMS {
                        cd_base: base,
                        cd_name: cd,
                        name,
                    } => {
                        assert_eq!(*base, "http://openmath.org");
                        assert_eq!(*cd, "arith1");
                        assert_eq!(*name, "plus");
                    }
                    _ => panic!("Expected OMS head"),
                }
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected OMA"),
        }
    }

    #[test]
    fn test_ombind_structure() {
        // Create an OMBIND for a lambda expression
        let head = Either::Right(Box::new(OpenMath::OMS {
            cd_base: "http://openmath.org",
            cd_name: "fns1",
            name: "lambda",
        }));
        let context = vec!["x"];
        let body = Either::Right(Box::new(OpenMath::OMV("x")));
        let om: OpenMath<TestInt> = OpenMath::OMBIND {
            head,
            context,
            body,
        };

        // Test structure
        match om {
            OpenMath::OMBIND {
                head: Either::Right(head_box),
                context,
                body: Either::Right(body_box),
            } => {
                match head_box.as_ref() {
                    OpenMath::OMS { name, .. } => {
                        assert_eq!(*name, "lambda");
                    }
                    _ => panic!("Expected OMS head"),
                }
                assert_eq!(context, vec!["x"]);
                match body_box.as_ref() {
                    OpenMath::OMV(var_name) => {
                        assert_eq!(*var_name, "x");
                    }
                    _ => panic!("Expected OMV body"),
                }
            }
            _ => panic!("Expected OMBIND"),
        }
    }

    #[test]
    fn test_omb_structure() {
        let bytes = vec![1, 2, 3, 4, 5];
        let om: OpenMath<TestInt> = OpenMath::OMB(bytes.as_slice());

        match om {
            OpenMath::OMB(b) => {
                assert_eq!(b, &[1, 2, 3, 4, 5]);
            }
            _ => panic!("Expected OMB"),
        }
    }

    #[test]
    fn test_clone_openmath() {
        let om: OpenMath<TestInt> = OpenMath::OMI(Int::from(42));
        let cloned = om.clone();

        match (om, cloned) {
            (OpenMath::OMI(orig), OpenMath::OMI(clone)) => {
                assert_eq!(orig, clone);
            }
            _ => panic!("Expected both to be OMI"),
        }
    }

    #[test]
    fn test_blanket_impl_for_owned() {
        // Test that OMDeserializableOwned types work with the borrowed trait
        let om: OpenMath<OwnedTestString, Vec<u8>, String> = OpenMath::OMSTR("test".to_string());

        // This should work due to the blanket impl
        let result = <OwnedTestString as OMDeserializableOwned>::from_openmath(om)
            .expect("should be defined");
        match result {
            Either::Left(owned) => assert_eq!(owned.0, "test"),
            Either::Right(_) => panic!("Expected successful deserialization"),
        }
    }
}
