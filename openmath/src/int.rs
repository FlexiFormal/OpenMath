use std::borrow::Cow;

/// An arbitrary precision integer optimized for small values.
///
/// The implementation optimizes for the common case where integers are small enough to fit
/// in native types, while gracefully handling arbitrarily large integers through string
/// representation: small integers that fit in `i128` are stored on the stack,
/// while larger integers are stored as validated decimal strings.
/// This avoids the overhead of big integer libraries for typical use cases.
///
/// # Examples
///
/// ```rust
/// use openmath::Int;
///
/// // Create from various integer types
/// let a = Int::from(42u8);
/// let b = Int::from(-123i64);
/// let c = Int::from(i128::MAX);
///
/// // Small integers are stored efficiently
/// let small = Int::from(42);
/// assert_eq!(small.is_i128(), Some(42));
///
/// // Create from strings for big integers
/// let big = Int::new("999999999999999999999999999999999999999999").expect("should be defined");
/// assert!(big.is_big().is_some());
///
/// // representation is chosen automatically:
/// let small = Int::new("-42").expect("should be defined");
/// assert_eq!(small.is_i128(), Some(-42));
///
///
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord, Hash)]
pub struct Int<'l>(pub(crate) I<'l>);
impl std::fmt::Display for Int<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            I::Stack(i) => i.fmt(f),
            I::Heap(s) => f.write_str(s),
        }
    }
}

/// Internal representation of an integer value.
///
/// This enum distinguishes between small integers (stored as `i128`) and
/// large integers (stored as decimal strings).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum I<'l> {
    /// Small integer that fits in `i128`
    Stack(i128),
    /// Large integer stored as a decimal string
    Heap(Cow<'l, str>),
}

#[cfg(feature = "serde")]
impl serde::Serialize for I<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            I::Stack(n) => serializer.serialize_i128(*n),
            I::Heap(s) => serializer.serialize_str(s),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for I<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Visitor};

        struct IntVisitor;

        impl<'de> Visitor<'de> for IntVisitor {
            type Value = I<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an integer or string")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(I::Stack(value.into()))
            }

            fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(I::Stack(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(I::Stack(value.into()))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // Try to parse as i128 first
                if let Ok(n) = value.parse::<i128>() {
                    Ok(I::Stack(n))
                } else {
                    // Validate it's a valid integer string
                    let mut chars = value.as_bytes();
                    if chars.is_empty() {
                        return Err(E::custom("empty string"));
                    }
                    if chars[0] == b'+' || chars[0] == b'-' {
                        chars = &chars[1..];
                    }
                    if chars.iter().all(u8::is_ascii_digit) {
                        Ok(I::Heap(std::borrow::Cow::Owned(value.to_string())))
                    } else {
                        Err(E::custom("invalid integer string"))
                    }
                }
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // Try to parse as i128 first
                if let Ok(n) = value.parse::<i128>() {
                    Ok(I::Stack(n))
                } else {
                    // Validate it's a valid integer string
                    let mut chars = value.as_bytes();
                    if chars.is_empty() {
                        return Err(E::custom("empty string"));
                    }
                    if chars[0] == b'+' || chars[0] == b'-' {
                        chars = &chars[1..];
                    }
                    if chars.iter().all(u8::is_ascii_digit) {
                        Ok(I::Heap(std::borrow::Cow::Owned(value)))
                    } else {
                        Err(E::custom("invalid integer string"))
                    }
                }
            }
        }

        deserializer.deserialize_any(IntVisitor)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Int<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        I::deserialize(deserializer).map(Int)
    }
}

macro_rules! into {
    ($($t:ty),*) => {
        $(
            impl<'l> From<$t> for Int<'l> {
                #[inline]
                fn from(value:$t) -> Int<'l> {
                    #[allow(clippy::cast_lossless)]
                    Int(I::Stack(value as _))
                }
            }
        )*
    }
}
into! {u8, i8, u16, i16, u32, i32, u64, i64, usize, isize, i128}

macro_rules! impl_from {
    ($value:ident => $cow:expr;$dropped:expr) => {{
        if let Ok(i) = <i128 as std::str::FromStr>::from_str(&$value) {
            return Ok(Int(I::Stack(i)));
        }
        let mut chars = $value.as_bytes();
        let Some(v) = chars.first().copied() else {
            return Err(());
        };
        let drop_one = (v == b'+' && {
            chars = &chars[1..];
            true
        }) || (v == b'-' && {
            chars = &chars[1..];
            false
        });
        if chars.iter().all(|c| c.is_ascii_digit()) {
            Ok(Int(I::Heap(if drop_one { $dropped } else { $cow })))
        } else {
            Err(())
        }
    }};
}

impl<'l> TryFrom<&'l str> for Int<'l> {
    type Error = ();
    fn try_from(value: &'l str) -> Result<Self, Self::Error> {
        impl_from!(value => Cow::Borrowed(value);Cow::Borrowed(&value[1..]))
    }
}
impl TryFrom<String> for Int<'_> {
    type Error = ();
    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        impl_from!(value => Cow::Owned(value);Cow::Owned({value.remove(0);value}))
    }
}
impl<'l> TryFrom<Cow<'l, str>> for Int<'l> {
    type Error = ();
    fn try_from(value: Cow<'l, str>) -> Result<Self, Self::Error> {
        impl_from!(value => value;match value {
            Cow::Borrowed(v) => Cow::Borrowed(&v[1..]),
            Cow::Owned(mut v) => Cow::Owned({v.remove(0);v})
        })
    }
}

impl Int<'_> {
    /// Returns the value as an `i128` if it fits, otherwise `None`.
    ///
    /// This method allows you to check if the integer is small enough to be
    /// represented as a native `i128` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// let small = Int::from(42);
    /// assert_eq!(small.is_i128(), Some(42));
    ///
    /// let big = Int::new("999999999999999999999999999999999999999999").expect("should be defined");
    /// assert_eq!(big.is_i128(), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_i128(&self) -> Option<i128> {
        if let I::Stack(v) = &self.0 {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns the value as a string slice if it's a big integer, otherwise `None`.
    ///
    /// This method allows you to access the string representation of large integers
    /// that don't fit in `i128`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// let small = Int::from(42);
    /// assert_eq!(small.is_big(), None);
    ///
    /// let big = Int::new("999999999999999999999999999999999999999999").expect("should be defined");
    /// assert_eq!(big.is_big(), Some("999999999999999999999999999999999999999999"));
    /// ```
    #[must_use]
    pub fn is_big(&self) -> Option<&'_ str> {
        if let I::Heap(s) = &self.0 {
            Some(&**s)
        } else {
            None
        }
    }

    /// Creates a new `Int` from a string slice.
    ///
    /// The string must represent a valid decimal integer, optionally with a leading
    /// sign (`+` or `-`). Returns `None` if the string is not a valid integer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// assert!(Int::new("42").is_some());
    /// assert!(Int::new("-123").is_some());
    /// assert!(Int::new("+456").is_some());
    /// assert!(Int::new("999999999999999999999999999999999999999999").is_some());
    ///
    /// // Invalid formats
    /// assert!(Int::new("12.34").is_none());
    /// assert!(Int::new("abc").is_none());
    /// assert!(Int::new("").is_none());
    /// ```
    #[inline]
    #[must_use]
    pub fn new(num: &str) -> Option<Int<'_>> {
        num.try_into().ok()
    }

    /// Creates a new `Int` from an owned string.
    ///
    /// Similar to [`new`](Self::new), but takes ownership of the string for cases
    /// where you want a `'static` lifetime integer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// let big_num = "12345678901234567890123456789012345678901234567890".to_string();
    /// let int = Int::from_string(big_num).unwrap();
    /// assert!(int.is_big().is_some());
    /// ```
    #[must_use]
    pub fn from_string(num: String) -> Option<Int<'static>> {
        num.try_into().ok()
    }

    /// Returns `true` if this integer represents zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// assert!(Int::from(0).is_zero());
    /// assert!(Int::new("0").expect("should be defined").is_zero());
    /// assert!(Int::new("+0").expect("should be defined").is_zero());
    /// assert!(Int::new("-0").expect("should be defined").is_zero());
    /// assert!(!Int::from(1).is_zero());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_zero(&self) -> bool {
        matches!(self.0, I::Stack(0))
    }

    /// Returns `true` if this integer is positive (> 0).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// assert!(Int::from(1).is_positive());
    /// assert!(Int::new("999999999999999999999999999999999999999999").expect("should be defined").is_positive());
    /// assert!(!Int::from(0).is_positive());
    /// assert!(!Int::from(-1).is_positive());
    /// ```
    #[must_use]
    pub fn is_positive(&self) -> bool {
        match &self.0 {
            I::Stack(v) => *v > 0,
            I::Heap(s) => s.as_ref().as_bytes()[0] != b'-',
        }
    }

    /// Returns `true` if this integer is negative (< 0).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openmath::Int;
    ///
    /// assert!(Int::from(-1).is_negative());
    /// assert!(Int::new("-999999999999999999999999999999999999999999").expect("should be defined").is_negative());
    /// assert!(!Int::from(0).is_negative());
    /// assert!(!Int::from(1).is_negative());
    /// ```
    #[must_use]
    pub fn is_negative(&self) -> bool {
        match &self.0 {
            I::Stack(v) => *v < 0,
            I::Heap(s) => s.as_ref().as_bytes()[0] == b'-',
        }
    }

    #[must_use]
    pub fn into_owned(self) -> Int<'static> {
        match self.0 {
            I::Stack(i) => Int(I::Stack(i)),
            I::Heap(Cow::Owned(s)) => Int(I::Heap(Cow::Owned(s))),
            I::Heap(b) => Int(I::Heap(Cow::Owned(b.into_owned()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_integers() {
        let int = Int::from(42);
        assert_eq!(int.is_i128(), Some(42));
        assert_eq!(int.is_big(), None);
        assert!(!int.is_zero());
        assert!(int.is_positive());
        assert!(!int.is_negative());
    }

    #[test]
    fn test_negative_integers() {
        let int = Int::from(-42);
        assert_eq!(int.is_i128(), Some(-42));
        assert_eq!(int.is_big(), None);
        assert!(!int.is_zero());
        assert!(!int.is_positive());
        assert!(int.is_negative());
    }

    #[test]
    fn test_zero() {
        let int = Int::from(0);
        assert_eq!(int.is_i128(), Some(0));
        assert_eq!(int.is_big(), None);
        assert!(int.is_zero());
        assert!(!int.is_positive());
        assert!(!int.is_negative());
    }

    #[test]
    fn test_max_i128() {
        let int = Int::from(i128::MAX);
        assert_eq!(int.is_i128(), Some(i128::MAX));
        assert_eq!(int.is_big(), None);
        assert!(int.is_positive());
    }

    #[test]
    fn test_min_i128() {
        let int = Int::from(i128::MIN);
        assert_eq!(int.is_i128(), Some(i128::MIN));
        assert_eq!(int.is_big(), None);
        assert!(int.is_negative());
    }

    #[test]
    fn test_big_integer_from_string() {
        let big_str =
            "12345678901234567890123456789012345678901234567890123456789012345678901234567890";
        let int = Int::new(big_str).expect("works");
        assert_eq!(int.is_i128(), None);
        assert_eq!(int.is_big(), Some(big_str));
        assert!(int.is_positive());
        assert!(!int.is_zero());
        assert!(!int.is_negative());
    }

    #[test]
    fn test_big_negative_integer() {
        let big_str =
            "-12345678901234567890123456789012345678901234567890123456789012345678901234567890";
        let int = Int::new(big_str).expect("should be defined");
        assert_eq!(int.is_i128(), None);
        assert_eq!(int.is_big(), Some(big_str));
        assert!(!int.is_positive());
        assert!(!int.is_zero());
        assert!(int.is_negative());
    }

    #[test]
    fn test_zero_variants() {
        for zero_str in ["0", "+0", "-0"] {
            let int = Int::new(zero_str).expect("should be defined");
            assert!(int.is_zero(), "Failed for '{zero_str}'");
            assert!(!int.is_positive(), "Failed for '{zero_str}'");
            assert!(!int.is_negative(), "Failed for '{zero_str}'");
        }
    }

    #[test]
    fn test_invalid_strings() {
        let invalid = ["", "abc", "12.34", "12e5", "12.0", "+-123", "12-34"];
        for s in invalid {
            assert!(Int::new(s).is_none(), "Should reject '{s}'");
        }
    }

    #[test]
    fn test_valid_string_formats() {
        let valid = ["123", "+123", "-123", "0", "+0", "-0"];
        for s in valid {
            assert!(Int::new(s).is_some(), "Should accept '{s}'");
        }
    }

    #[test]
    fn test_from_different_types() {
        assert_eq!(Int::from(42u8).is_i128(), Some(42));
        assert_eq!(Int::from(42i8).is_i128(), Some(42));
        assert_eq!(Int::from(42u16).is_i128(), Some(42));
        assert_eq!(Int::from(42i16).is_i128(), Some(42));
        assert_eq!(Int::from(42u32).is_i128(), Some(42));
        assert_eq!(Int::from(42i32).is_i128(), Some(42));
        assert_eq!(Int::from(42u64).is_i128(), Some(42));
        assert_eq!(Int::from(42i64).is_i128(), Some(42));
        assert_eq!(Int::from(42usize).is_i128(), Some(42));
        assert_eq!(Int::from(42isize).is_i128(), Some(42));
        assert_eq!(Int::from(42i128).is_i128(), Some(42));
    }

    #[test]
    fn test_ordering() {
        let small = Int::from(42);
        let big = Int::new("123456789012345678901234567890").expect("should be defined");
        let bigger = Int::new("999999999999999999999999999999").expect("should be defined");

        assert!(small < big);
        assert!(big < bigger);
        assert!(small < bigger);
    }

    #[test]
    fn test_equality() {
        let a = Int::from(42);
        let b = Int::from(42);
        let c = Int::from(43);

        assert_eq!(a, b);
        assert_ne!(a, c);

        let big1 = Int::new("12345678901234567890123456789012345678901234567890")
            .expect("should be defined");
        let big2 = Int::new("12345678901234567890123456789012345678901234567890")
            .expect("should be defined");
        let big3 = Int::new("12345678901234567890123456789012345678901234567891")
            .expect("should be defined");

        assert_eq!(big1, big2);
        assert_ne!(big1, big3);
    }

    #[test]
    fn test_clone() {
        let original = Int::new("12345678901234567890123456789012345678901234567890")
            .expect("should be defined");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_from_string_owned() {
        let s = "12345678901234567890123456789012345678901234567890".to_string();
        let int = Int::from_string(s).expect("should be defined");
        assert!(int.is_big().is_some());
    }

    #[test]
    fn test_boundary_conditions() {
        // Test the boundary where we switch from i128 to string
        let max_plus_one = "170141183460469231731687303715884105728"; // i128::MAX + 1
        let int = Int::new(max_plus_one).expect("should be defined");
        assert!(int.is_big().is_some());
        assert_eq!(int.is_big().expect("should be defined"), max_plus_one);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_serialization() {
        let small = Int::from(42);
        let json = serde_json::to_string(&small).expect("should be defined");
        let deserialized: Int = serde_json::from_str(&json).expect("should be defined");
        assert_eq!(small, deserialized);

        let big = Int::new("12345678901234567890123456789012345678901234567890")
            .expect("should be defined");
        let json = serde_json::to_string(&big).expect("should be defined");
        let deserialized: Int = serde_json::from_str(&json).expect("should be defined");
        assert_eq!(big, deserialized);
    }
}
