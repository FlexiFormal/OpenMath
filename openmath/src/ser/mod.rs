/*! # OpenMath Serialization

This module provides traits and implementations for serializing Rust types
as OpenMath. The core trait [`OMSerializable`] allows any type to
define how it should be represented as an OpenMath object.
Serialization uses a [serde](https://docs.rs/serde)-style architecture to avoid allocations
and cloning wherever possible.

### Built-in Serializers
- [openmath_display()](OMSerializable::openmath_display) implements
  [Debug](std::fmt::Debug) and [Display](std::fmt::Display) using the OpenMath XML tags
  as prefix (see below for an example)
- **Serde-based**: Serialize to any serde-compatible format by using <code>self.[openmath_serde()](OMSerializable::openmath_serde())</code>
  instead of `self` (requires the `serde` feature).
  The implementation follows the official OpenMath JSON encoding[^1], so using
  [`serde_json`](https://docs.rs/serde_json) allows for serializing to specification-compliant
  JSON.

## Examples

```rust
use openmath::{OMSerializable, ser::{OMSerializer,Uri}};
pub struct Point {
    x: f64,
    y: f64,
}
impl Point {
    const URI: Uri<'_> = Uri {
        cd_base: "http://example.org",
        cd: "geometry1",
        name: "point",
    };
}
impl OMSerializable for Point {
    fn as_openmath<'s,S: OMSerializer<'s>>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Err> {
        // Represent as OMA: point(x, y)
        serializer.oma(&Self::URI, &[&self.x, &self.y])
    }
}
fn test() {
    let point = Point { x:1.4, y:7.8 };
    assert_eq!(
        point.openmath_display().to_string(),
        "OMA(OMS(http://example.org/geometry1#point),OMF(1.4),OMF(7.8))"
    )
}
#[cfg(feature="serde")]
fn serde_test() {
    let point = Point { x:1.4, y:7.8 };
    let json = serde_json::to_string(&point.openmath_serde()).expect("should be defined");
    println!("{}", json); // Outputs OpenMath JSON representation
}
```
[^1]: <https://openmath.org/standard/om20-2019-07-01/omstd20.html#sec_json-the-json-encoding>
*/

use std::fmt::Write;

#[cfg(feature = "serde")]
mod serde_impl;

/// Trait for [`OMSerializer`]-Errors;
pub trait Error {
    /// call this in [`OMSerializable::as_openmath`]-implementations
    /// to return custom errors.
    fn custom(err: impl std::fmt::Display) -> Self;
}

/** Trait for types that can be serialized to OpenMath format.

This trait defines how a Rust type should be represented as an OpenMath object.
The serialization process is delegated to an [`OMSerializer`] implementation,
allowing the same type to be serialized to different output formats.

# Examples

## Simple Value Types

```rust
use openmath::{OMSerializable, ser::OMSerializer};

struct Temperature(f64);
impl OMSerializable for Temperature {
    fn as_openmath<'s,S: OMSerializer<'s>>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Err> {
        // Serialize as a floating point number
        serializer.omf(self.0)
    }
}
```

## Complex Structures

```rust
use openmath::{OMSerializable, ser::{OMSerializer,Uri,Error}};

pub struct Polynomial {
    pub coefficients: Vec<f64>,
}
impl Polynomial {
    const URI: Uri<'_> = Uri {
        cd_base: "http://example.org/algebra",
        cd: "linera_algebra",
        name: "polynomial",
    };
}
impl OMSerializable for Polynomial {
    fn as_openmath<'s,S: OMSerializer<'s>>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Err> {
        if self.coefficients.is_empty() {
            return Err(S::Err::custom("Empty polynomial"));
        }

        // Represent as polynomial(coeff1, coeff2, ...)
        serializer.oma(&Self::URI, &self.coefficients)
    }
}
```
**/
pub trait OMSerializable {
    #[inline]
    fn cd_base(&self) -> Option<&str> {
        None
    }

    /// Serialize this value using the provided serializer.
    ///
    /// This method should convert the Rust value into appropriate OpenMath
    /// representation using the serializer's methods.
    ///
    ///
    /// # Errors
    /// If either the [OMSerializer] erorrs, or this object can't be serialized
    /// after all (call [`Error::custom`] to return custom error messages).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::{OMSerializable, ser::OMSerializer};
    ///
    /// struct MyInt(i32);
    ///
    /// impl OMSerializable for MyInt {
    ///     fn as_openmath<'s,S: OMSerializer<'s>>(
    ///         &self,
    ///         serializer: S,
    ///     ) -> Result<S::Ok, S::Err> {
    ///         serializer.omi(&self.0.into())
    ///     }
    /// }
    /// ```
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err>;

    /// OpenMath-style [Debug](std::fmt::Debug) and [Display](std::fmt::Display) implementations
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::{Int,ser::OMSerializable};
    ///
    /// let value = Int::from(42);
    /// assert_eq!(value.openmath_display().to_string(),"OMI(42)");
    /// ```
    #[inline]
    fn openmath_display(&self) -> impl std::fmt::Display + std::fmt::Debug + use<'_, Self> {
        OMDisplay(self, self.cd_base())
    }

    /// Create a serde-compatible serializer wrapper.
    ///
    /// This method returns a wrapper that implements [`serde::Serialize`],
    /// allowing OpenMath objects to be serialized using any serde-compatible
    /// format (JSON, XML, YAML, etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "serde")]
    /// # {
    /// use openmath::{Int,ser::OMSerializable};
    ///
    /// let value = Int::from(42);
    /// let json = serde_json::to_string(&value.openmath_serde()).expect("should be defined");
    /// println!("{}", json); // Outputs OpenMath JSON representation
    /// # }
    /// ```
    #[cfg(feature = "serde")]
    #[inline]
    fn openmath_serde(&self) -> impl ::serde::Serialize + use<'_, Self> {
        serde_impl::SerdeSerializer(self, self.cd_base(), crate::OPENMATH_BASE_URI.as_str())
    }

    /// returns this element as something that serializes into an OMOBJ; i.e. a "top-level"
    /// OpenMath object.
    #[inline]
    fn omobject(&self) -> OMObject<'_, Self> {
        OMObject(self)
    }
}

impl<A: OMSerializable, B: OMSerializable> OMSerializable for either::Either<A, B> {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        match self {
            Self::Left(a) => a.as_openmath(serializer),
            Self::Right(a) => a.as_openmath(serializer),
        }
    }
}
pub enum OMForeignSerializable<'f, OM: OMSerializable = String, F: std::fmt::Display = String> {
    OM(&'f OM),
    Foreign {
        encoding: Option<&'f str>,
        value: &'f F,
    },
}

/// Blanket implementation for references to serializable types.
///
/// This allows `&T` to be serializable whenever `T` is serializable,
/// which is convenient for method chaining and generic contexts.
impl<T: OMSerializable> OMSerializable for &T {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        T::as_openmath(self, serializer)
    }
}

/// Trait for serializers that can produce OpenMath output.
///
/// This trait defines the interface for converting OpenMath constructs into
/// various output formats. Implementors provide methods for each OpenMath
/// object type (OMI, OMF, OMSTR, etc.).
///
/// # Design Pattern
///
/// The serializer uses a "builder" pattern where each method consumes `self`
/// and returns the final result. For complex structures like OMA and OMBIND,
/// additional iterator-based methods are provided for incremental construction.
///
/// See [OMDisplay] for a relatively simple example implementation.
pub trait OMSerializer<'s>: Sized {
    /// The type of successful serialization results.
    type Ok;
    /// The type of serialization errors.
    type Err: Error;

    type SubSerializer<'ns>: OMSerializer<'ns, Ok = Self::Ok, Err = Self::Err>
    where
        's: 'ns;

    fn current_cd_base(&self) -> &str;
    /// ### Errors
    fn with_cd_base<'ns>(self, cd_base: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
    where
        's: 'ns;

    /** Serialize an OpenMath integer (OMI).

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    Usage:
    ```rust
    use openmath::{OMSerializable, ser::OMSerializer};
    struct MyInt(u16);
    impl OMSerializable for MyInt {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.omi(&self.0.into())
        }
    }
    ```
    */
    fn omi(self, value: &crate::Int) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath floating point number (OMF).

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    Usage:
    ```rust
    use openmath::{OMSerializable, ser::OMSerializer};
    struct MyFloat(f32);
    impl OMSerializable for MyFloat {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.omf(self.0.into())
        }
    }
    ```
    */
    fn omf(self, value: f64) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath string (OMSTR).

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    Usage:
    ```rust
    use openmath::{OMSerializable, ser::OMSerializer};
    struct MyName<'s>(&'s str);
    impl OMSerializable for MyName<'_> {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.omstr(&self.0)
        }
    }
    ```
    */
    fn omstr(self, string: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath byte array (OMB).

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    Usage:
    ```rust
    use openmath::{OMSerializable, ser::OMSerializer};
    struct Blob(Vec<u8>);
    impl OMSerializable for Blob {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.omb(self.0.iter().copied())
        }
    }
    ```
    */
    fn omb<I: IntoIterator<Item = u8>>(self, bytes: I) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator;

    /** Serialize an OpenMath variable (OMV).

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    Usage:
    ```rust
    use openmath::{OMSerializable, ser::OMSerializer};
    struct Var{ name: &'static str }
    impl OMSerializable for Var {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.omv(&self.name)
        }
    }
    ```
    */
    fn omv(self, name: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err>;

    #[allow(rustdoc::bare_urls)]
    /** Serialize an OpenMath symbol (OMS).

    OpenMath symbols are identified by their URI (e.g. `http://www.openmath.org/cd/arith1#plus`), which in all official serialization
    methods is split into three components:
    - The name of the symbol (`plus`)
    - The name of the content dictionary containing the symbol (`arith1`)
    - The base Url of the content dictionary (`http://www.openmath.org/cd`). This is
      provided using the [`with_cd_base`](OMSerializer::with_cd_base)-method


    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    ```rust
    use openmath::{OMSerializable, ser::OMSerializer};
    struct PlusSymbol;
    impl OMSerializable for PlusSymbol {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.oms(
                //&"http://www.openmath.org/cd",
                &"arith1",
                &"plus"
            )
        }
    }
    ```
    */
    fn oms(
        self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath application (OMA).

    An OMA represent an application fo some OpenMath Object to a list of arguments, e.g. $2 + 2$
    would be represented as `OMA(OMS(plus),[OMI(2),OMI(2)])`.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    ```rust
    use openmath::{OMSerializable, ser::{OMSerializer,Uri}};
    struct Plus(u16,u16);
    impl Plus {
        const URI:Uri<'_> = Uri {
            cd_base:"http://www.openmath.org/cd",
            cd:"arith1",
            name:"plus"
        };
    }
    impl OMSerializable for Plus {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.oma(&Self::URI,[&self.0,&self.1])
        }
    }
    ```
    */
    fn oma<'a, T: OMSerializable + 'a, I: IntoIterator<Item = &'a T>>(
        self,
        head: &'a impl OMSerializable,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator;

    /** Serialize an OpenMath error (OME).

    `name` and `cd_name` are those of the URI of the error symbol.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).
    */
    fn ome<
        'a,
        T: OMSerializable + 'a,
        D: std::fmt::Display + 'a,
        I: IntoIterator<Item = OMForeignSerializable<'a, T, D>>,
    >(
        self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator;

    /** Serialize an OpenMath binding construct (OMBIND).

    OMBIND represents constructs that bind variables, such as
    quantifiers ($\forall x, \exists x$),
    lambda expressions ($\lambda x.f(x)$) etc.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    ```rust
    use openmath::{OMSerializable, ser::{OMSerializer,Uri}};
    # struct Term;
    # impl OMSerializable for Term {
    # fn as_openmath<'s,S: OMSerializer<'s>>(
    #    &self,
    #    serializer: S,
    # ) -> Result<S::Ok, S::Err> {
    #  todo!()
    # }}
    struct Lambda<'a>{var:&'a str,body:Term};
    impl Lambda<'_> {
        const URI:Uri<'static> = Uri {
            cd_base:"http://www.openmath.org/cd",
            cd:&"fns1",
            name:&"lambda"
        };
    }
    impl OMSerializable for Lambda<'_> {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.ombind(&Self::URI,[&self.var],&self.body)
        }
    }
    ```
    */
    fn ombind<'a, St: std::fmt::Display + 'a, I: IntoIterator<Item = &'a St>>(
        self,
        head: &'a impl OMSerializable,
        vars: I,
        body: &'a impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator;
}

/// Wrapper that produces an OMOBJ wrapper in serialization
#[impl_tools::autoimpl(Copy, Clone)]
pub struct OMObject<'s, O: OMSerializable + ?Sized>(pub &'s O);
impl<O: OMSerializable> std::fmt::Display for OMObject<'_, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OMOBJ({})", self.0.openmath_display())
    }
}

/// Convenience structure for producing OMS triples in [as_openmath](OMSerializable::as_openmath)
///
/// # Examples
///
/// ```rust
/// use openmath::ser::Uri;
/// const URI:Uri<'static> = Uri {
///     cd_base:"http://www.openmath.org/cd",
///     cd:&"fns1",
///     name:&"lambda"
/// };
/// ```
#[derive(Debug)]
pub struct Uri<'s, CD = str, Name = str>
where
    CD: std::fmt::Display + ?Sized,
    Name: std::fmt::Display + ?Sized,
{
    /// The content dictionary base
    pub cd_base: &'s str,
    /// The name of the content dictionary
    pub cd: &'s CD,
    /// The name of the symbol
    pub name: &'s Name,
}

impl<CD, Name> OMSerializable for Uri<'_, CD, Name>
where
    CD: std::fmt::Display + ?Sized,
    Name: std::fmt::Display + ?Sized,
{
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer
            .with_cd_base(self.cd_base)?
            .oms(&self.cd, &self.name)
    }
}

// Implement OMSerializable for basic types
impl OMSerializable for crate::Int<'_> {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omi(self)
    }
}

impl OMSerializable for f32 {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omf((*self).into())
    }
}

impl OMSerializable for f64 {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omf(*self)
    }
}

impl OMSerializable for str {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omstr(&self)
    }
}

impl OMSerializable for String {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omstr(self)
    }
}

impl OMSerializable for [u8] {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omb(self.iter().copied())
    }
}

impl OMSerializable for Vec<u8> {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omb(self.iter().copied())
    }
}

impl OMSerializable for &str {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omstr(self)
    }
}

// Implement for integer types by converting to Int
macro_rules! impl_int_serializable {
    ($($t:ty),*) => {
        $(
            impl OMSerializable for $t {
                #[inline]
                fn as_openmath<'s,S: OMSerializer<'s>>(
                    &self,
                    serializer: S,
                ) -> Result<S::Ok, S::Err> {
                    serializer.omi(&crate::Int::from(*self))
                }
            }
        )*
    };
}
impl_int_serializable! {i8, u8, i16, u16, u32, i32, i64, u64, i128, isize, usize}

/// Simple [OMSerializer] that simply implements [Display](std::fmt::Display) and
/// [Debug](std::fmt::Debug)
#[impl_tools::autoimpl(Copy, Clone)]
pub struct OMDisplay<'o, O: OMSerializable + ?Sized>(&'o O, Option<&'o str>);
impl<O: OMSerializable + ?Sized> std::fmt::Debug for OMDisplay<'_, O> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}
impl<O: OMSerializable + ?Sized> std::fmt::Display for OMDisplay<'_, O> {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0
            .as_openmath(DisplaySerializer {
                f,
                next_ns: self.1,
                current_ns: crate::OPENMATH_BASE_URI.as_ref(),
            })
            .map_err(Into::into)
    }
}

struct DisplayErr;
impl From<std::fmt::Error> for DisplayErr {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn from(_: std::fmt::Error) -> Self {
        Self
    }
}
impl From<DisplayErr> for std::fmt::Error {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn from(_: DisplayErr) -> Self {
        Self
    }
}
impl Error for DisplayErr {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn custom(_: impl std::fmt::Display) -> Self {
        Self
    }
}
struct DisplaySerializer<'f1, 'f2> {
    f: &'f1 mut std::fmt::Formatter<'f2>,
    next_ns: Option<&'f1 str>,
    current_ns: &'f1 str,
}
impl DisplaySerializer<'_, '_> {
    fn rec(&mut self, o: &impl OMSerializable) -> Result<(), DisplayErr> {
        let s = if let Some(next) = o.cd_base() {
            if self.current_ns == next {
                DisplaySerializer {
                    f: self.f,
                    next_ns: self.next_ns,
                    current_ns: self.current_ns,
                }
            } else {
                DisplaySerializer {
                    f: self.f,
                    next_ns: Some(next),
                    current_ns: crate::OPENMATH_BASE_URI.as_ref(),
                }
            }
        } else {
            DisplaySerializer {
                f: self.f,
                next_ns: self.next_ns,
                current_ns: self.current_ns,
            }
        };
        o.as_openmath(s)
    }
    fn foreign<O: OMSerializable, D: std::fmt::Display>(
        &mut self,
        o: &OMForeignSerializable<'_, O, D>,
    ) -> Result<(), DisplayErr> {
        match o {
            OMForeignSerializable::OM(o) => self.rec(o),
            OMForeignSerializable::Foreign {
                encoding: Some(enc),
                value,
            } => Ok(write!(self.f, "OMF(encoding:{enc},{value})")?),
            OMForeignSerializable::Foreign { value, .. } => Ok(write!(self.f, "OMF({value})")?),
        }
    }
}
impl<'f1, 'f2> OMSerializer<'f1> for DisplaySerializer<'f1, 'f2> {
    type Err = DisplayErr;
    type Ok = ();
    type SubSerializer<'ns>
        = DisplaySerializer<'ns, 'f2>
    where
        'f1: 'ns;
    #[inline]
    fn current_cd_base(&self) -> &str {
        self.next_ns.unwrap_or(self.current_ns)
    }

    fn with_cd_base<'ns>(self, cd_base: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
    where
        'f1: 'ns,
    {
        if self.current_ns == cd_base {
            Ok(self)
        } else {
            Ok(DisplaySerializer {
                f: self.f,
                next_ns: Some(cd_base),
                current_ns: self.current_ns,
            })
        }
    }
    #[inline]
    fn omi(self, value: &crate::Int) -> Result<Self::Ok, Self::Err> {
        write!(self.f, "OMI({value})").map_err(Into::into)
    }
    #[inline]
    fn omf(self, value: f64) -> Result<Self::Ok, Self::Err> {
        write!(self.f, "OMF({value})").map_err(Into::into)
    }
    #[inline]
    fn omstr(self, string: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        write!(self.f, "OMSTR(\"{string}\")").map_err(Into::into)
    }
    #[inline]
    fn omb<I: IntoIterator<Item = u8>>(self, bytes: I) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        let f = self.f;
        f.write_str("OMB(")?;
        let mut first = true;
        for b in bytes {
            if !first {
                f.write_char(',')?;
            }
            std::fmt::Display::fmt(&b, f)?;
            first = false;
        }
        f.write_char(')').map_err(Into::into)
    }
    #[inline]
    fn omv(self, name: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        write!(self.f, "OMV({name})").map_err(Into::into)
    }
    #[inline]
    fn oms(
        self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err> {
        let (s, t) = self.next_ns.map_or(("", ""), |s| (s, "/"));
        write!(self.f, "OMS({s}{t}{cd_name}#{name})").map_err(Into::into)
    }

    fn oma<'s, T: OMSerializable + 's, I: IntoIterator<Item = &'s T>>(
        mut self,
        head: &'s impl OMSerializable,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        let (a, b) = if let Some(s) = self.next_ns {
            self.current_ns = s;
            self.next_ns = None;
            ("@", s)
        } else {
            ("", "")
        };
        let args = args.into_iter();
        if args.len() == 0 {
            return self.rec(head);
        }
        write!(self.f, "OMA{a}{b}(")?;
        self.rec(head)?;
        for a in args {
            self.f.write_char(',')?;
            self.rec(a)?;
        }
        self.f.write_char(')').map_err(Into::into)
    }

    fn ome<
        'a,
        T: OMSerializable + 'a,
        D: std::fmt::Display + 'a,
        I: IntoIterator<Item = OMForeignSerializable<'a, T, D>>,
    >(
        mut self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        let (s, t) = self.next_ns.map_or(("", ""), |s| (s, "/"));
        let mut args = args.into_iter();
        write!(self.f, "OME{s}{t}{cd_name}#{name}(")?;
        if let Some(next) = args.next() {
            self.foreign(&next)?;
            for a in args {
                self.f.write_char(',')?;
                self.foreign(&a)?;
            }
        }
        self.f.write_char(')').map_err(Into::into)
    }

    fn ombind<'s, St: std::fmt::Display + 's, I: IntoIterator<Item = &'s St>>(
        mut self,
        head: &'s impl OMSerializable,
        vars: I,
        body: &'s impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        let (a, b) = if let Some(s) = self.next_ns {
            self.current_ns = s;
            self.next_ns = None;
            ("@", s)
        } else {
            ("", "")
        };
        let vars = vars.into_iter();
        //write!(self.f, "OMBIND{a}{b}({},[", head.openmath_display())?;
        write!(self.f, "OMBIND{a}{b}(")?;
        self.rec(head)?;
        self.f.write_char(',')?;
        self.f.write_char('[')?;
        let mut first = true;
        for v in vars {
            write!(self.f, "{}{v}", if first { "" } else { "," })?;
            first = false;
        }
        self.f.write_char(']')?;
        self.f.write_char(',')?;
        self.rec(body)?;
        self.f.write_char(')').map_err(Into::into)
    }
}

#[cfg(any(test, doc))]
#[doc(hidden)]
pub mod testdoc {
    use super::*;

    pub struct Point {
        pub x: f64,
        pub y: f64,
    }
    impl Point {
        const URI: Uri<'_> = Uri {
            cd_base: "http://example.org",
            cd: "geometry1",
            name: "point",
        };
    }
    impl OMSerializable for Point {
        fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
            // Represent as OMA: point(x, y)
            serializer.oma(&Self::URI, [&self.x, &self.y])
        }
    }

    pub struct Lambda<'s, const LEN: usize, O> {
        pub vars: [&'s str; LEN],
        pub body: O,
    }
    impl<const LEN: usize, O> Lambda<'_, LEN, O> {
        const URI: Uri<'static> = Uri {
            cd_base: "http://openmath.org",
            cd: "fns1",
            name: "lambda",
        };
    }
    impl<const LEN: usize, O: OMSerializable> OMSerializable for Lambda<'_, LEN, O> {
        fn cd_base(&self) -> Option<&str> {
            Some(Self::URI.cd_base)
        }
        fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
            serializer.ombind(&Self::URI, &self.vars, &self.body)
        }
    }

    // Test types
    pub struct TestSymbol(pub &'static str);
    impl OMSerializable for TestSymbol {
        fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
            serializer
                .with_cd_base("http://test.org")?
                .oms(&"test", &self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testdoc::*;
    use super::*;
    use crate::Int;

    #[test]
    fn test_omi_serialization() {
        let result = Int::from(42).openmath_display().to_string();
        assert_eq!(result, "OMI(42)");

        let result = Int::new("123456789012345678901234567890")
            .expect("should be defined")
            .openmath_display()
            .to_string();
        assert_eq!(result, "OMI(123456789012345678901234567890)");
    }

    #[test]
    fn test_omf_serialization() {
        #[allow(clippy::approx_constant)]
        let result = (3.14159f32).openmath_display().to_string();
        assert!(result.starts_with("OMF(3.14159"));
    }

    #[test]
    fn test_omstr_serialization() {
        let result = "42".openmath_display().to_string();
        assert_eq!(result, "OMSTR(\"42\")");
    }

    #[test]
    fn test_omb_serialization() {
        let result = vec![1u8, 2, 3, 4, 5].openmath_display().to_string();
        assert_eq!(result, "OMB(1,2,3,4,5)");
    }

    #[test]
    fn test_omv_serialization() {
        struct Omv(&'static str);
        impl OMSerializable for Omv {
            fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
                serializer.omv(&self.0)
            }
        }
        let result = Omv("variable").openmath_display().to_string();
        assert_eq!(result, "OMV(variable)");
    }

    #[test]
    fn test_oms_serialization() {
        let result = Uri {
            cd_base: "http://test.org",
            cd: "test",
            name: "symbol",
        }
        .openmath_display()
        .to_string();
        assert_eq!(result, "OMS(http://test.org/test#symbol)");
    }

    #[test]
    fn test_oma_serialization() {
        let result = Point { x: 13.1, y: 17.4 }.openmath_display().to_string();
        assert_eq!(
            result,
            "OMA(OMS(http://example.org/geometry1#point),OMF(13.1),OMF(17.4))"
        );
    }

    #[test]
    fn test_ombind_serialization() {
        let result = Lambda {
            vars: ["x", "y"],
            body: "x + y",
        }
        .openmath_display()
        .to_string();
        assert_eq!(
            result,
            "OMBIND@http://openmath.org(OMS(fns1#lambda),[x,y],OMSTR(\"x + y\"))"
        );
    }

    #[test]
    fn test_empty_ombind() {
        let result = Lambda {
            vars: [],
            body: "true",
        }
        .openmath_display()
        .to_string();
        assert_eq!(
            result,
            "OMBIND@http://openmath.org(OMS(fns1#lambda),[],OMSTR(\"true\"))"
        );
    }
}
