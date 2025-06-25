/*! Opinionated implementation of Base64 encoding/decoding
as adapters over [`Iterator`]s, rather than on pre-allocated
byte slices/[`Vec`]s/string types
 */

use std::num::NonZeroU8;

/** Encodes the underlying `u8`-[`Iterator`] as base64,
yielding chunks of <code>[[NonZeroU8];4]</code>.

Given a `u8`-[`Iterator`], use [`Base64Encodable::base64()`]
to get an instance.
Call [`.flatten()`](std::iter::Iterator::flatten) to get
a `u8`-[`Iterator`]

## Example
```
// import the trait
use openmath::base64::Base64Encodable;

let s = b"ThIs Is A tEsT!!";
// get a byte Iterator:
let mut iter = s.iter().copied();
// turn it into a base64-encoded byte Iterator:
let encoded = iter.base64();
// collect it into a string
let out = encoded.into_string();
assert_eq!(out,"VGhJcyBJcyBBIHRFc1QhIQ==");
````
 */
pub struct Base64Encoder<I: Iterator<Item = u8>>(Chunked<I>);
impl<I: Iterator<Item = u8>> Base64Encoder<I> {
    /// Converts into a [`char`]-[`Iterator`]
    pub fn chars(self) -> std::iter::Map<std::iter::Flatten<Self>, fn(NonZeroU8) -> char> {
        self.flatten().map(|u| u.get() as char)
    }
    /// Collects the base64 encoding into a [`String`]
    pub fn into_string(self) -> String {
        self.chars().collect()
    }
}
impl<I: ExactSizeIterator<Item = u8>> ExactSizeIterator for Base64Encoder<I> {}

/// Trait for [`Iterator`]s that can be base64-encoded.
/// Blanket implemented for all <code>I: [Iterator]<Item = u8></code>.
pub trait Base64Encodable: Iterator {
    type Inner: Iterator<Item = u8>;
    /// Encodes this [`Iterator`] as base64
    fn base64(self) -> Base64Encoder<Self::Inner>;
}
impl<I: Iterator<Item = u8>> Base64Encodable for I {
    type Inner = Self;
    fn base64(self) -> Base64Encoder<Self::Inner> {
        Base64Encoder(Chunked(self))
    }
}

/** Decodes the underlying base64-encoded `u8`-[`Iterator`] ,
yielding chunks of <code>[Result]<[u8; 3], [Error]></code>.

## Errors
If the underlying [`Iterator`] contains invalid base64.

Given a `u8`-[`Iterator`], use [`Base64Decodable::decode_base64()`]
to get an instance.
Call [`.flat()`](Base64Decoder::flat) to get
a <code>[Result]<u8, [Error]></code>-[`Iterator`].

## Example
```
// import the trait
use openmath::base64::Base64Decodable;

let s = b"RGlFcyBJc1QgZUluIFRlU3QhIQ==";
// get a byte Iterator:
let mut iter = s.iter().copied();
// turn it into a base64-decoded byte Iterator:
let decoded = iter.decode_base64();
// collect it into a string
let out = decoded.flat().map(|u| u.unwrap() as char).collect::<String>();
assert_eq!(out,"DiEs IsT eIn TeSt!!");
````
 */
pub struct Base64Decoder<I: Iterator<Item = u8>>(I);
impl<I: Iterator<Item = u8>> Base64Decoder<I> {
    /// Turns this into a
    pub fn flat(self) -> Flat<I> {
        self.flat_map(fltn as _).filter(flter as _)
    }
}
impl<I: ExactSizeIterator<Item = u8>> ExactSizeIterator for Base64Decoder<I> {}

/// Used in [`Base64Decoder::flat`].
pub type Flat<I> = std::iter::Filter<
    std::iter::FlatMap<
        Base64Decoder<I>,
        [Result<u8, Error>; 3],
        fn(Result<[u8; 3], Error>) -> [Result<u8, Error>; 3],
    >,
    fn(&Result<u8, Error>) -> bool,
>;

/// Trait for [`Iterator`]s that can be base64-decoded.
/// Blanket implemented for all <code>I: [Iterator]<Item = u8></code>.
pub trait Base64Decodable: Iterator {
    type Inner: Iterator<Item = u8>;
    fn decode_base64(self) -> Base64Decoder<Self::Inner>;
}
impl<I: Iterator<Item = u8>> Base64Decodable for I {
    type Inner = Self;
    fn decode_base64(self) -> Base64Decoder<Self::Inner> {
        Base64Decoder(self)
    }
}

/// Errors that can occur during base64 decoding
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A valid base64 string's length must be divisible by 4 (with padding)
    #[error("length not divisible by 4")]
    IllegalLength,
    /// padding character (`=`) may only occur at the end of the string
    #[error("base64 string has characters after padding")]
    NonsensicalPadding,
    /// Only alpha-numeric ASCII characters, `+`, and `/` are allowed (and `=` for padding)
    #[error("base64 string contains illegal characters")]
    IllegalChar(u8),
}

// -------------------------------------------------------------------------------------

const PAD: NonZeroU8 = NonZeroU8::new(b'=').unwrap();
macro_rules! table{
    ($($c:literal),*) => {
        // SAFETY: all values are != 0
        const TABLE: [NonZeroU8; 64] = unsafe{[
            $(NonZeroU8::new_unchecked($c)),*
        ]};
    }
}
table![
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/'
];
#[allow(clippy::cast_possible_truncation)]
const INVERSE_TABLE: [u8; 256] = {
    let mut ret = [255u8; 256];
    let mut i = 0;
    while i < 64 {
        ret[TABLE[i].get() as usize] = i as u8;
        i += 1;
    }
    ret
};

struct Chunked<I: Iterator<Item = u8>>(I);
impl<I: Iterator<Item = u8>> Iterator for Chunked<I> {
    type Item = Chunk;
    fn next(&mut self) -> Option<Self::Item> {
        let a = self.0.next()?;
        let Some(b) = self.0.next() else {
            return Some(Chunk::One(a));
        };
        Some(
            self.0
                .next()
                .map_or(Chunk::Two(a, b), |c| Chunk::Three(a, b, c)),
        )
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.0.size_hint();
        (lower / 3, upper.map(|u| u / 3))
    }
}
impl<I: ExactSizeIterator<Item = u8>> ExactSizeIterator for Chunked<I> {}

enum Chunk {
    One(u8),
    Two(u8, u8),
    Three(u8, u8, u8),
}

impl<I: Iterator<Item = u8>> Iterator for Base64Encoder<I> {
    type Item = [NonZeroU8; 4];
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.0.next()?;
        Some(match chunk {
            Chunk::One(a) => [
                TABLE[(a >> 2) as usize],
                TABLE[((a << 4) & 0x3F) as usize],
                PAD,
                PAD,
            ],
            Chunk::Two(a, b) => [
                TABLE[(a >> 2) as usize],
                TABLE[((a << 4 | b >> 4) & 0x3F) as usize],
                TABLE[((b << 2) & 0x3F) as usize],
                PAD,
            ],
            Chunk::Three(a, b, c) => [
                TABLE[(a >> 2) as usize],
                TABLE[((a << 4 | b >> 4) & 0x3F) as usize],
                TABLE[((b << 2 | c >> 6) & 0x3F) as usize],
                TABLE[(c & 0x3F) as usize],
            ],
        })
    }
}

const fn fltn(r: Result<[u8; 3], Error>) -> [Result<u8, Error>; 3] {
    match r {
        Ok([a, b, c]) => [Ok(a), Ok(b), Ok(c)],
        Err(e) => [Err(e), Ok(0), Ok(0)],
    }
}
const fn flter(r: &Result<u8, Error>) -> bool {
    !matches!(r, Ok(0))
}

impl<I: Iterator<Item = u8>> Iterator for Base64Decoder<I> {
    type Item = Result<[u8; 3], Error>;
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.0.size_hint();
        (lower / 4, upper.map(|u| u / 4))
    }

    #[allow(unused_assignments)]
    fn next(&mut self) -> Option<Self::Item> {
        let mut in_pad = false;
        macro_rules! get {
            () => {{
                let Some(n) = self.0.next() else {
                    return Some(Err(Error::IllegalLength));
                };
                get!(n)
            }};
            ($e:ident) => {{
                if in_pad && $e != b'=' {
                    return Some(Err(Error::NonsensicalPadding))
                }
                if $e == b'=' {
                    in_pad = true;
                    0u32
                } else {
                    let n = INVERSE_TABLE[$e as usize];
                    if n == 255 {
                        return Some(Err(Error::IllegalChar($e)));
                    }
                    n.into()
                }
            }}
        }
        let a = self.0.next()?;
        let mut r = get!(a) << 26;
        r |= get!() << 20;
        r |= get!() << 14;
        r |= get!() << 8;
        let [a, b, c, _] = r.to_be_bytes();
        Some(Ok([a, b, c]))
    }
}
