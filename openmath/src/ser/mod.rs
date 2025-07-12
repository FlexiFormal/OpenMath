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
use openmath::{OMSerializable, ser::{OMSerializer,Uri,AsOMS}};
pub struct Point {
    x: f64,
    y: f64,
}
impl Point {
    const URI: Uri<'static> = Uri {
        cdbase: Some("http://example.org"),
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
        serializer.oma(Self::URI.as_oms(), [self.x, self.y].iter())
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

use std::{borrow::Cow, fmt::Write};

#[cfg(feature = "serde")]
mod serde_impl;
pub(crate) mod xml;
pub use xml::XmlWriteError;

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
use openmath::{OMSerializable, ser::{OMSerializer,Uri,Error,AsOMS}};

pub struct Polynomial {
    pub coefficients: Vec<f64>,
}
impl Polynomial {
    const URI: Uri<'static> = Uri {
        cdbase: Some("http://example.org/algebra"),
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
        serializer.oma(Self::URI.as_oms(), self.coefficients.iter())
    }
}
```
**/
pub trait OMSerializable {
    #[inline]
    fn cdbase(&self) -> Option<&str> {
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
        OMDisplay(self, self.cdbase())
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
        serde_impl::SerdeSerializer(self, self.cdbase(), crate::OPENMATH_BASE_URI)
    }

    /// Returns something that [`Display`](std::fmt::Display)s
    /// as the OpenMath XML of this object.
    ///
    /// ### Errors
    /// if [as_openmath](OMSerializable::as_openmath) (or the underlying writer) does
    #[inline]
    fn xml(&self, pretty: bool) -> impl std::fmt::Display {
        xml::XmlDisplay { pretty, o: self }
    }

    /// returns this element as something that serializes into an OMOBJ; i.e. a "top-level"
    /// OpenMath object.
    #[inline]
    fn omobject(&self) -> OMObject<'_, Self> {
        OMObject(self)
    }
}

/// Blanket implementation for references to serializable types.
///
/// This allows `&T` to be serializable whenever `T` is serializable,
/// which is convenient for method chaining and generic contexts.
impl<T: OMSerializable + ?Sized> OMSerializable for &T {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        T::as_openmath(self, serializer)
    }
}
pub trait BindVar {
    fn name(&self) -> impl std::fmt::Display;
    #[inline]
    fn attrs(&self) -> impl ExactSizeIterator<Item: OMAttr> {
        std::iter::empty::<(&Uri<'static>, &str)>()
    }
}
impl<D: std::fmt::Display> BindVar for &D {
    #[inline]
    fn name(&self) -> impl std::fmt::Display {
        self
    }
}
pub trait OMAttr {
    fn symbol(&self) -> impl AsOMS;
    fn value(&self) -> impl OMOrForeign;
}
pub trait OMOrForeign {
    fn om_or_foreign(
        self,
    ) -> crate::either::Either<
        impl OMSerializable,
        (Option<impl std::fmt::Display>, impl std::fmt::Display),
    >;
}
impl<O: OMSerializable> OMOrForeign for O {
    fn om_or_foreign(
        self,
    ) -> crate::either::Either<
        impl OMSerializable,
        (Option<impl std::fmt::Display>, impl std::fmt::Display),
    > {
        crate::either::Either::Left::<Self, (Option<&&str>, &&str)>(self)
    }
}
impl<'a, O: ?Sized, S: AsOMS + ?Sized> OMAttr for (&'a S, &'a O)
where
    &'a O: OMOrForeign,
{
    #[inline]
    fn symbol(&self) -> impl AsOMS {
        self.0
    }
    #[inline]
    fn value(&self) -> impl OMOrForeign {
        self.1
    }
}
/*
pub struct OMAttrSerializable<
    'f,
    OM: OMSerializable = String,
    F: std::fmt::Display + ?Sized = String,
> {
    pub cdbase: Option<Cow<'f, str>>,
    pub cd: Cow<'f, str>,
    pub name: Cow<'f, str>,
    pub value: OMForeignSerializable<'f, OM, F>,
}

pub enum OMForeignSerializable<
    'f,
    OM: OMSerializable = String,
    F: std::fmt::Display + ?Sized = String,
> {
    OM(&'f OM),
    Foreign {
        encoding: Option<&'f str>,
        value: &'f F,
    },
}
 */

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

    fn current_cdbase(&self) -> &str;
    /// ### Errors
    fn with_cdbase<'ns>(self, cdbase: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
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
    fn omstr(self, string: impl std::fmt::Display) -> Result<Self::Ok, Self::Err>;

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
    fn omb(self, bytes: impl ExactSizeIterator<Item = u8>) -> Result<Self::Ok, Self::Err>;

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
    fn omv(self, name: impl std::fmt::Display) -> Result<Self::Ok, Self::Err>;

    #[allow(rustdoc::bare_urls)]
    /** Serialize an OpenMath symbol (OMS).

    OpenMath symbols are identified by their URI (e.g. `http://www.openmath.org/cd/arith1#plus`), which in all official serialization
    methods is split into three components:
    - The name of the symbol (`plus`)
    - The name of the content dictionary containing the symbol (`arith1`)
    - The base Url of the content dictionary (`http://www.openmath.org/cd`). This is
      provided using the [`with_cdbase`](OMSerializer::with_cdbase)-method


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
        cd_name: impl std::fmt::Display,
        name: impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath application (OMA).

    An OMA represent an application fo some OpenMath Object to a list of arguments, e.g. $2 + 2$
    would be represented as `OMA(OMS(plus),[OMI(2),OMI(2)])`.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    ```rust
    use openmath::{OMSerializable, ser::{OMSerializer,Uri,AsOMS}};
    struct Plus(u16,u16);
    impl Plus {
        const URI:Uri<'static> = Uri {
            cdbase:Some("http://www.openmath.org/cd"),
            cd:"arith1",
            name:"plus"
        };
    }
    impl OMSerializable for Plus {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.oma(Self::URI.as_oms(),[self.0,self.1].iter())
        }
    }
    ```
    */
    fn oma(
        self,
        head: impl OMSerializable,
        args: impl ExactSizeIterator<Item: OMSerializable>,
    ) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath attribution (OMATTR).

    `name` and `cd_name` are those of the URI of the error symbol.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).
    */
    fn omattr(
        self,
        attrs: impl ExactSizeIterator<Item: OMAttr>,
        atp: impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath error (OME).

    `name` and `cd_name` are those of the URI of the error symbol.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).
    */
    fn ome(
        self,
        error: impl AsOMS,
        args: impl ExactSizeIterator<Item: OMOrForeign>,
    ) -> Result<Self::Ok, Self::Err>;

    /** Serialize an OpenMath binding construct (OMBIND).

    OMBIND represents constructs that bind variables, such as
    quantifiers ($\forall x, \exists x$),
    lambda expressions ($\lambda x.f(x)$) etc.

    # Errors
    If either the [OMSerializer] erorrs, or this object can't be serialized
    after all (call [`Error::custom`] to return custom error messages).

    # Examples

    ```rust
    use openmath::{OMSerializable, ser::{OMSerializer,Uri,AsOMS}};
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
            cdbase:Some("http://www.openmath.org/cd"),
            cd:&"fns1",
            name:&"lambda"
        };
    }
    impl OMSerializable for Lambda<'_> {
        fn as_openmath<'s,S: OMSerializer<'s>>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Err> {
            serializer.ombind(Self::URI.as_oms(),[self.var].iter(),&self.body)
        }
    }
    ```
    */
    fn ombind(
        self,
        head: impl OMSerializable,
        vars: impl ExactSizeIterator<Item: BindVar>,
        body: impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err>;
}
/*
pub trait IntoVars<'a, St: std::fmt::Display + 'a> {
    fn vars(
        self,
    ) -> impl ExactSizeIterator<Item = (&'a St, &'a [&'a OMAttrSerializable<'a, String, St>])>;
}
impl<'a, St: std::fmt::Display + 'a, I: IntoIterator<Item = &'a St>> IntoVars<'a, St> for I
where
    I::IntoIter: ExactSizeIterator,
{
    fn vars(
        self,
    ) -> impl ExactSizeIterator<Item = (&'a St, &'a [&'a OMAttrSerializable<'a, String, St>])> {
        <Self as IntoIterator>::into_iter(self)
            .map::<(&'a St, &'a [&'a OMAttrSerializable<'a, String, St>]), _>(|s| (s, &[]))
    }
}
*/

/// Wrapper that produces an OMOBJ wrapper in serialization
pub struct OMObject<'s, O: OMSerializable + ?Sized>(pub &'s O);
impl<O: OMSerializable + ?Sized> OMObject<'_, O> {
    /// Returns something that `[Display]`(std::fmt::Display)s as the OpenMath XML
    /// of this object
    ///
    /// ### Errors
    /// if [as_openmath](OMSerializable::as_openmath) or the underlying writer does
    #[inline]
    #[must_use]
    pub fn xml(&self, pretty: bool, insert_namespace: bool) -> impl std::fmt::Display {
        xml::XmlObjDisplay {
            o: self.0,
            pretty,
            insert_namespace,
        }
    }
}
impl<O: OMSerializable + ?Sized> Clone for OMObject<'_, O> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<O: OMSerializable + ?Sized> Copy for OMObject<'_, O> {}
impl<O: OMSerializable> std::fmt::Display for OMObject<'_, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OMOBJ({})", self.0.openmath_display())
    }
}

/// trait for things that can be serialized as an [OMS](crate::OMKind::OMS); i.e. things
/// that have a URI.
pub trait AsOMS {
    /// The cdbase of this URI. `current_cdbase` is the current namespace during
    /// serialization. This allows to return `None` if the current cdbase is already
    /// this one anyway
    fn cdbase(&self, _current_cdbase: &str) -> Option<Cow<'_, str>> {
        None
    }
    /// The cd of this URI
    fn cd(&self) -> impl std::fmt::Display;
    /// The name of the symbol represented by this URI
    fn name(&self) -> impl std::fmt::Display;
    fn as_oms(&self) -> impl OMSerializable {
        struct AsOM<'a, A: AsOMS + ?Sized>(&'a A);
        impl<A: AsOMS + ?Sized> OMSerializable for AsOM<'_, A> {
            fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
                if let Some(b) = self.0.cdbase(serializer.current_cdbase()) {
                    serializer.with_cdbase(&b)?.oms(self.0.cd(), self.0.name())
                } else {
                    serializer.oms(self.0.cd(), self.0.name())
                }
            }
        }
        AsOM(self)
    }
}
impl<A: AsOMS + ?Sized> AsOMS for &A {
    #[inline]
    fn cdbase(&self, current_cdbase: &str) -> Option<Cow<'_, str>> {
        A::cdbase(self, current_cdbase)
    }
    #[inline]
    fn cd(&self) -> impl std::fmt::Display {
        A::cd(self)
    }
    #[inline]
    fn name(&self) -> impl std::fmt::Display {
        A::name(self)
    }
}

/// Convenience structure for producing OMS triples in [as_openmath](OMSerializable::as_openmath)
///
/// # Examples
///
/// ```rust
/// use openmath::ser::Uri;
/// const URI:Uri<'static> = Uri {
///     cdbase:Some("http://www.openmath.org/cd"),
///     cd:&"fns1",
///     name:&"lambda"
/// };
/// ```
#[derive(Debug)]
pub struct Uri<'s, CD = &'s str, Name = &'s str>
where
    CD: std::fmt::Display,
    Name: std::fmt::Display,
{
    /// The content dictionary base
    pub cdbase: Option<&'s str>,
    /// The name of the content dictionary
    pub cd: CD,
    /// The name of the symbol
    pub name: Name,
}

impl<CD, Name> AsOMS for Uri<'_, CD, Name>
where
    CD: std::fmt::Display,
    Name: std::fmt::Display,
{
    fn cdbase(&self, current_cdbase: &str) -> Option<Cow<'_, str>> {
        self.cdbase
            .map(Cow::Borrowed)
            .and_then(|s| if s == current_cdbase { None } else { Some(s) })
    }
    #[inline]
    fn cd(&self) -> impl std::fmt::Display {
        &self.cd
    }
    #[inline]
    fn name(&self) -> impl std::fmt::Display {
        &self.name
    }
}

/// Convenience structure for producing OMVs in [as_openmath](OMSerializable::as_openmath)
///
/// # Examples
///
/// ```rust
/// use openmath::ser::{Omv,OMSerializable};
/// const V:Omv<&'static str> = Omv("x");
/// assert_eq!(V.xml(true).to_string(),"<OMV name=\"x\"/>");
/// ```
pub struct Omv<D: std::fmt::Display>(pub D);
impl<D: std::fmt::Display> OMSerializable for Omv<D> {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        serializer.omv(&self.0)
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
        serializer.omstr(self)
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

impl<A: OMSerializable, B: OMSerializable> OMSerializable for either::Either<A, B> {
    #[inline]
    fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
        match self {
            Self::Left(a) => a.as_openmath(serializer),
            Self::Right(a) => a.as_openmath(serializer),
        }
    }
}

/// Simple [OMSerializer] that simply implements [Display](std::fmt::Display) and
/// [Debug](std::fmt::Debug)
pub struct OMDisplay<'o, O: OMSerializable + ?Sized>(&'o O, Option<&'o str>);
impl<O: OMSerializable + ?Sized> Clone for OMDisplay<'_, O> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<O: OMSerializable + ?Sized> Copy for OMDisplay<'_, O> {}
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
                current_ns: crate::OPENMATH_BASE_URI,
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
    fn rec(&mut self, o: impl OMSerializable) -> Result<(), DisplayErr> {
        let s = if let Some(next) = o.cdbase() {
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
                    current_ns: crate::OPENMATH_BASE_URI,
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
    fn foreign(&mut self, o: impl OMOrForeign) -> Result<(), DisplayErr> {
        match o.om_or_foreign() {
            either::Either::Left(o) => self.rec(o),
            either::Either::Right((Some(enc), value)) => {
                Ok(write!(self.f, "OMF(encoding:{enc},{value})")?)
            }
            either::Either::Right((None, value)) => Ok(write!(self.f, "OMF({value})")?),
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
    fn current_cdbase(&self) -> &str {
        self.next_ns.unwrap_or(self.current_ns)
    }

    fn with_cdbase<'ns>(self, cdbase: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
    where
        'f1: 'ns,
    {
        if self.current_ns == cdbase {
            Ok(self)
        } else {
            Ok(DisplaySerializer {
                f: self.f,
                next_ns: Some(cdbase),
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
    fn omstr(self, string: impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        write!(self.f, "OMSTR(\"{string}\")").map_err(Into::into)
    }
    #[inline]
    fn omb(self, bytes: impl ExactSizeIterator<Item = u8>) -> Result<Self::Ok, Self::Err> {
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
    fn omv(self, name: impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        write!(self.f, "OMV({name})").map_err(Into::into)
    }
    #[inline]
    fn oms(
        self,
        cd_name: impl std::fmt::Display,
        name: impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err> {
        let (s, t) = self.next_ns.map_or(("", ""), |s| (s, "/"));
        write!(self.f, "OMS({s}{t}{cd_name}#{name})").map_err(Into::into)
    }

    fn oma(
        mut self,
        head: impl OMSerializable,
        args: impl ExactSizeIterator<Item: OMSerializable>,
    ) -> Result<Self::Ok, Self::Err> {
        let (a, b) = if let Some(s) = self.next_ns {
            self.current_ns = s;
            self.next_ns = None;
            ("@", s)
        } else {
            ("", "")
        };
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

    fn ome(
        mut self,
        error: impl AsOMS,
        mut args: impl ExactSizeIterator<Item: OMOrForeign>,
    ) -> Result<Self::Ok, Self::Err> {
        let (s, t) = self.next_ns.map_or(("", ""), |s| (s, "/"));
        write!(self.f, "OME{s}{t}{}#{}(", error.cd(), error.name())?;
        if let Some(next) = args.next() {
            self.foreign(next)?;
            for a in args {
                self.f.write_char(',')?;
                self.foreign(a)?;
            }
        }
        self.f.write_char(')').map_err(Into::into)
    }

    fn omattr(
        mut self,
        attrs: impl ExactSizeIterator<Item: OMAttr>,
        atp: impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err> {
        let (a, b) = if let Some(s) = self.next_ns {
            self.current_ns = s;
            self.next_ns = None;
            ("@", s)
        } else {
            ("", "")
        };
        write!(self.f, "OMATTR{a}{b}(")?;
        self.rec(atp)?;
        self.f.write_char(',')?;
        self.f.write_char('[')?;
        let mut first = true;
        for a in attrs {
            if !first {
                self.f.write_str(", ")?;
            }
            first = false;
            self.rec(a.symbol().as_oms())?;
            self.f.write_str(" = ")?;
            self.foreign(a.value())?;
        }
        self.f.write_str("])").map_err(Into::into)
    }

    fn ombind(
        mut self,
        head: impl OMSerializable,
        vars: impl ExactSizeIterator<Item: BindVar>,
        body: impl OMSerializable,
    ) -> Result<Self::Ok, Self::Err> {
        let (a, b) = if let Some(s) = self.next_ns {
            self.current_ns = s;
            self.next_ns = None;
            ("@", s)
        } else {
            ("", "")
        };
        write!(self.f, "OMBIND{a}{b}(")?;
        self.rec(head)?;
        self.f.write_char(',')?;
        self.f.write_char('[')?;
        let mut first = true;
        for v in vars {
            let a = v.attrs();
            if a.len() == 0 {
                write!(self.f, "{}{}", if first { "" } else { ", " }, v.name())?;
            } else {
                if first {
                    self.f.write_str(", ")?;
                }
                DisplaySerializer {
                    f: self.f,
                    next_ns: None,
                    current_ns: self.current_ns,
                }
                .omattr(a, Omv(v.name()))?;
            }
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
            cdbase: Some("http://example.org"),
            cd: "geometry1",
            name: "point",
        };
    }
    impl OMSerializable for Point {
        fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
            // Represent as OMA: point(x, y)
            serializer.oma(&Self::URI.as_oms(), [&self.x, &self.y].into_iter())
        }
    }

    pub struct Lambda<'s, const LEN: usize, O> {
        pub vars: [&'s str; LEN],
        pub body: O,
    }
    impl<const LEN: usize, O> Lambda<'_, LEN, O> {
        pub const URI: Uri<'static> = Uri {
            cdbase: Some("http://openmath.org"),
            cd: "fns1",
            name: "lambda",
        };
    }
    impl<const LEN: usize, O: OMSerializable> OMSerializable for Lambda<'_, LEN, O> {
        fn cdbase(&self) -> Option<&str> {
            Self::URI.cdbase
        }
        fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
            serializer.ombind(&Self::URI.as_oms(), self.vars.iter(), &self.body)
        }
    }

    // Test types
    pub struct TestSymbol(pub &'static str);
    impl OMSerializable for TestSymbol {
        fn as_openmath<'s, S: OMSerializer<'s>>(&self, serializer: S) -> Result<S::Ok, S::Err> {
            serializer
                .with_cdbase("http://test.org")?
                .oms("test", self.0)
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
    fn test_omi_serialization_xml() {
        let result = Int::from(42).xml(true).to_string();
        assert_eq!(result, "<OMI>42</OMI>");

        let result = Int::new("123456789012345678901234567890")
            .expect("should be defined")
            .xml(true)
            .to_string();
        assert_eq!(result, "<OMI>123456789012345678901234567890</OMI>");
    }

    #[test]
    fn test_omf_serialization() {
        #[allow(clippy::approx_constant)]
        let result = (3.14159f32).openmath_display().to_string();
        assert!(result.starts_with("OMF(3.14159"));
    }

    #[test]
    fn test_omf_serialization_xml() {
        #[allow(clippy::approx_constant)]
        let result = (3.14159f32).xml(true).to_string();
        assert!(result.starts_with("<OMF dec=\"3.14159"));
    }

    #[test]
    fn test_omstr_serialization() {
        let result = "42".openmath_display().to_string();
        assert_eq!(result, "OMSTR(\"42\")");
    }

    #[test]
    fn test_omstr_serialization_xml() {
        let result = "42".xml(true).to_string();
        assert_eq!(result, "<OMSTR>42</OMSTR>");
    }

    #[test]
    fn test_omb_serialization() {
        let result = vec![1u8, 2, 3, 4, 5].openmath_display().to_string();
        assert_eq!(result, "OMB(1,2,3,4,5)");
    }

    #[test]
    fn test_omb_serialization_xml() {
        let result = b"foo bar".xml(true).to_string();
        assert_eq!(result, "<OMB>Zm9vIGJhcg==</OMB>");
    }

    #[test]
    fn test_omv_serialization() {
        let result = Omv("variable").openmath_display().to_string();
        assert_eq!(result, "OMV(variable)");
    }

    #[test]
    fn test_omv_serialization_xml() {
        let result = Omv("variable").xml(true).to_string();
        assert_eq!(result, "<OMV name=\"variable\"/>");
    }

    #[test]
    fn test_oms_serialization() {
        let result = Uri {
            cdbase: Some("http://test.org"),
            cd: "test",
            name: "symbol",
        }
        .as_oms()
        .openmath_display()
        .to_string();
        assert_eq!(result, "OMS(http://test.org/test#symbol)");
    }

    #[test]
    fn test_oms_serialization_xml() {
        let result = Uri {
            cdbase: Some("http://test.org"),
            cd: "test",
            name: "symbol",
        }
        .as_oms()
        .xml(true)
        .to_string();
        assert_eq!(
            result,
            "<OMS cdbase=\"http://test.org\" cd=\"test\" name=\"symbol\"/>"
        );
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
    fn test_oma_serialization_xml() {
        let result = Point { x: 13.1, y: 17.4 }.xml(true).to_string();
        assert_eq!(
            result,
            "<OMA>\n  <OMS cdbase=\"http://example.org\" cd=\"geometry1\" name=\"point\"/>\n  <OMF dec=\"13.1\"/>\n  <OMF dec=\"17.4\"/>\n</OMA>"
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
            "OMBIND@http://openmath.org(OMS(fns1#lambda),[x, y],OMSTR(\"x + y\"))"
        );
    }

    #[test]
    fn test_ombind_serialization_xml() {
        let result = Lambda {
            vars: ["x", "y"],
            body: "x + y",
        }
        .xml(true)
        .to_string();
        assert_eq!(
            result,
            "<OMBIND cdbase=\"http://openmath.org\">\n  <OMS cd=\"fns1\" name=\"lambda\"/>\n  <OMBVAR>\n    <OMV name=\"x\"/>\n    <OMV name=\"y\"/>\n  </OMBVAR>\n  <OMSTR>x + y</OMSTR>\n</OMBIND>"
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

    #[test]
    fn test_empty_ombind_xml() {
        let result = Lambda {
            vars: [],
            body: "true",
        }
        .xml(true)
        .to_string();
        assert_eq!(
            result,
            "<OMBIND cdbase=\"http://openmath.org\">\n  <OMS cd=\"fns1\" name=\"lambda\"/>\n  <OMBVAR/>\n  <OMSTR>true</OMSTR>\n</OMBIND>"
        );
    }
}
