macro_rules! tri {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => return Err(err),
        }
    };
}

pub struct TaggedContentVisitor;
impl<'de> serde::de::Visitor<'de> for TaggedContentVisitor {
    type Value = (super::OpenMathKind, Content<'de>);
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an internally tagged OMObject enum")
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let tag = match tri!(seq.next_element()) {
            Some(tag) => tag,
            None => {
                return Err(S::Error::missing_field("kind"));
            }
        };
        let rest = SeqAccessDeserializer { seq };
        Ok((
            tag,
            tri!(<Content as serde::de::Deserialize>::deserialize(rest)),
        ))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut tag = None;
        let mut vec = Vec::<(Content, Content)>::with_capacity(cautious::<(Content, Content)>(
            map.size_hint(),
        ));
        while let Some(k) = tri!(map.next_key_seed(TagOrContentVisitor(PhantomData))) {
            match k {
                TagOrContent::Tag => {
                    if tag.is_some() {
                        return Err(de::Error::duplicate_field("kind"));
                    }
                    tag = Some(tri!(map.next_value()));
                }
                TagOrContent::Content(k) => {
                    let v = tri!(map.next_value());
                    vec.push((k, v));
                }
            }
        }
        match tag {
            None => Err(de::Error::missing_field("kind")),
            Some(tag) => Ok((tag, Content::Map(vec))),
        }
    }
}

#[derive(Clone, Debug)]
struct SeqAccessDeserializer<A> {
    seq: A,
}

impl<'de, A> serde::de::Deserializer<'de> for SeqAccessDeserializer<A>
where
    A: serde::de::SeqAccess<'de>,
{
    type Error = A::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(self.seq)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

enum TagOrContent<'de> {
    Tag,
    Content(Content<'de>),
}
struct TagOrContentVisitor<'de>(PhantomData<TagOrContent<'de>>);

impl<'de> serde::de::DeserializeSeed<'de> for TagOrContentVisitor<'de> {
    type Value = TagOrContent<'de>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Internally tagged enums are only supported in self-describing
        // formats.
        deserializer.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for TagOrContentVisitor<'de> {
    type Value = TagOrContent<'de>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("a type tag `kind` or any other value")
    }

    fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_bool(value)
            .map(TagOrContent::Content)
    }

    fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_i8(value)
            .map(TagOrContent::Content)
    }

    fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_i16(value)
            .map(TagOrContent::Content)
    }

    fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_i32(value)
            .map(TagOrContent::Content)
    }

    fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_i64(value)
            .map(TagOrContent::Content)
    }

    fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_u8(value)
            .map(TagOrContent::Content)
    }

    fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_u16(value)
            .map(TagOrContent::Content)
    }

    fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_u32(value)
            .map(TagOrContent::Content)
    }

    fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_u64(value)
            .map(TagOrContent::Content)
    }

    fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_f32(value)
            .map(TagOrContent::Content)
    }

    fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_f64(value)
            .map(TagOrContent::Content)
    }

    fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_char(value)
            .map(TagOrContent::Content)
    }

    fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == "kind" {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor(PhantomData)
                .visit_str(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == "kind" {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor(PhantomData)
                .visit_borrowed_str(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == "kind" {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor(PhantomData)
                .visit_string(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == b"kind" {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor(PhantomData)
                .visit_bytes(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_borrowed_bytes<F>(self, value: &'de [u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == b"kind" {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor(PhantomData)
                .visit_borrowed_bytes(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == b"kind" {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor(PhantomData)
                .visit_byte_buf(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_unit<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_unit()
            .map(TagOrContent::Content)
    }

    fn visit_none<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor(PhantomData)
            .visit_none()
            .map(TagOrContent::Content)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        ContentVisitor(PhantomData)
            .visit_some(deserializer)
            .map(TagOrContent::Content)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        ContentVisitor(PhantomData)
            .visit_newtype_struct(deserializer)
            .map(TagOrContent::Content)
    }

    fn visit_seq<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        ContentVisitor(PhantomData)
            .visit_seq(visitor)
            .map(TagOrContent::Content)
    }

    fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::MapAccess<'de>,
    {
        ContentVisitor(PhantomData)
            .visit_map(visitor)
            .map(TagOrContent::Content)
    }

    fn visit_enum<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::EnumAccess<'de>,
    {
        ContentVisitor(PhantomData)
            .visit_enum(visitor)
            .map(TagOrContent::Content)
    }
}

#[derive(Debug, Clone)]
pub enum Content<'de> {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),
    Str(&'de str),
    ByteBuf(Vec<u8>),
    Bytes(&'de [u8]),

    None,
    Some(Box<Content<'de>>),

    Unit,
    Newtype(Box<Content<'de>>),
    Seq(Vec<Content<'de>>),
    Map(Vec<(Content<'de>, Content<'de>)>),
}
use serde::de::Unexpected;
impl<'de> Content<'de> {
    fn unexpected(&self) -> Unexpected {
        match *self {
            Content::Bool(b) => Unexpected::Bool(b),
            Content::U8(n) => Unexpected::Unsigned(n as u64),
            Content::U16(n) => Unexpected::Unsigned(n as u64),
            Content::U32(n) => Unexpected::Unsigned(n as u64),
            Content::U64(n) => Unexpected::Unsigned(n),
            Content::I8(n) => Unexpected::Signed(n as i64),
            Content::I16(n) => Unexpected::Signed(n as i64),
            Content::I32(n) => Unexpected::Signed(n as i64),
            Content::I64(n) => Unexpected::Signed(n),
            Content::F32(f) => Unexpected::Float(f as f64),
            Content::F64(f) => Unexpected::Float(f),
            Content::Char(c) => Unexpected::Char(c),
            Content::String(ref s) => Unexpected::Str(s),
            Content::Str(s) => Unexpected::Str(s),
            Content::ByteBuf(ref b) => Unexpected::Bytes(b),
            Content::Bytes(b) => Unexpected::Bytes(b),
            Content::None | Content::Some(_) => Unexpected::Option,
            Content::Unit => Unexpected::Unit,
            Content::Newtype(_) => Unexpected::NewtypeStruct,
            Content::Seq(_) => Unexpected::Seq,
            Content::Map(_) => Unexpected::Map,
        }
    }
}

impl<'de, E> de::IntoDeserializer<'de, E> for Content<'de>
where
    E: de::Error,
{
    type Deserializer = ContentDeserializer<'de, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        ContentDeserializer(self, PhantomData)
    }
}

impl<'de> serde::de::Deserialize<'de> for Content<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Untagged and internally tagged enums are only supported in
        // self-describing formats.
        let visitor = ContentVisitor(PhantomData);
        deserializer.deserialize_any(visitor)
    }
}
struct ContentVisitor<'de>(PhantomData<Content<'de>>);

use serde::de;
use std::{fmt, marker::PhantomData};

impl<'de> serde::de::Visitor<'de> for ContentVisitor<'de> {
    type Value = Content<'de>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("any value")
    }

    fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Bool(value))
    }

    fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I8(value))
    }

    fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I16(value))
    }

    fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I32(value))
    }

    fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I64(value))
    }

    fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U8(value))
    }

    fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U16(value))
    }

    fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U32(value))
    }

    fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U64(value))
    }

    fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::F32(value))
    }

    fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::F64(value))
    }

    fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Char(value))
    }

    fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::String(value.into()))
    }

    fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Str(value))
    }

    fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::String(value))
    }

    fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::ByteBuf(value.into()))
    }

    fn visit_borrowed_bytes<F>(self, value: &'de [u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Bytes(value))
    }

    fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::ByteBuf(value))
    }

    fn visit_unit<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Unit)
    }

    fn visit_none<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = tri!(serde::de::Deserialize::deserialize(deserializer));
        Ok(Content::Some(Box::new(v)))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = tri!(serde::de::Deserialize::deserialize(deserializer));
        Ok(Content::Newtype(Box::new(v)))
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let mut vec = Vec::<Content>::with_capacity(cautious::<Content>(visitor.size_hint()));
        while let Some(e) = tri!(visitor.next_element()) {
            vec.push(e);
        }
        Ok(Content::Seq(vec))
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::MapAccess<'de>,
    {
        let mut vec = Vec::<(Content, Content)>::with_capacity(cautious::<(Content, Content)>(
            visitor.size_hint(),
        ));
        while let Some(kv) = tri!(visitor.next_entry()) {
            vec.push(kv);
        }
        Ok(Content::Map(vec))
    }

    fn visit_enum<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::EnumAccess<'de>,
    {
        Err(de::Error::custom(
            "untagged and internally tagged enums do not support enum input",
        ))
    }
}
fn cautious<Element>(hint: Option<usize>) -> usize {
    const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

    if std::mem::size_of::<Element>() == 0 {
        0
    } else {
        std::cmp::min(
            hint.unwrap_or(0),
            MAX_PREALLOC_BYTES / std::mem::size_of::<Element>(),
        )
    }
}

pub struct ContentDeserializer<'de, E>(pub Content<'de>, pub PhantomData<E>);

impl<'de, E> ContentDeserializer<'de, E>
where
    E: de::Error,
{
    #[cold]
    fn invalid_type(self, exp: &impl serde::de::Expected) -> E {
        de::Error::invalid_type(self.0.unexpected(), exp)
    }

    fn deserialize_integer<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_float<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::F32(v) => visitor.visit_f32(v),
            Content::F64(v) => visitor.visit_f64(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
}

fn visit_content_seq<'de, V, E>(content: Vec<Content<'de>>, visitor: V) -> Result<V::Value, E>
where
    V: serde::de::Visitor<'de>,
    E: de::Error,
{
    let mut seq_visitor = serde::de::value::SeqDeserializer::new(content.into_iter());
    let value = tri!(visitor.visit_seq(&mut seq_visitor));
    tri!(seq_visitor.end());
    Ok(value)
}

fn visit_content_map<'de, V, E>(
    content: Vec<(Content<'de>, Content<'de>)>,
    visitor: V,
) -> Result<V::Value, E>
where
    V: serde::de::Visitor<'de>,
    E: de::Error,
{
    let mut map_visitor = serde::de::value::MapDeserializer::new(content.into_iter());
    let value = tri!(visitor.visit_map(&mut map_visitor));
    tri!(map_visitor.end());
    Ok(value)
}

/// Used when deserializing an internally tagged enum because the content
/// will be used exactly once.
impl<'de, E> serde::Deserializer<'de> for ContentDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Bool(v) => visitor.visit_bool(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            Content::F32(v) => visitor.visit_f32(v),
            Content::F64(v) => visitor.visit_f64(v),
            Content::Char(v) => visitor.visit_char(v),
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::Unit => visitor.visit_unit(),
            Content::None => visitor.visit_none(),
            Content::Some(v) => visitor.visit_some(ContentDeserializer(*v, PhantomData)),
            Content::Newtype(v) => {
                visitor.visit_newtype_struct(ContentDeserializer(*v, PhantomData))
            }
            Content::Seq(v) => visit_content_seq(v, visitor),
            Content::Map(v) => visit_content_map(v, visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Bool(v) => visitor.visit_bool(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_float(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_float(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Char(v) => visitor.visit_char(v),
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::Seq(v) => visit_content_seq(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::None => visitor.visit_none(),
            Content::Some(v) => visitor.visit_some(ContentDeserializer(*v, PhantomData)),
            Content::Unit => visitor.visit_unit(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Unit => visitor.visit_unit(),

            // Allow deserializing newtype variant containing unit.
            //
            //     #[derive(Deserialize)]
            //     #[serde(tag = "result")]
            //     enum Response<T> {
            //         Success(T),
            //     }
            //
            // We want {"result":"Success"} to deserialize into Response<()>.
            Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            // As a special case, allow deserializing untagged newtype
            // variant containing unit struct.
            //
            //     #[derive(Deserialize)]
            //     struct Info;
            //
            //     #[derive(Deserialize)]
            //     #[serde(tag = "topic")]
            //     enum Message {
            //         Info(Info),
            //     }
            //
            // We want {"topic":"Info"} to deserialize even though
            // ordinarily unit structs do not deserialize from empty map/seq.
            Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
            Content::Seq(ref v) if v.is_empty() => visitor.visit_unit(),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Newtype(v) => {
                visitor.visit_newtype_struct(ContentDeserializer(*v, PhantomData))
            }
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Seq(v) => visit_content_seq(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Map(v) => visit_content_map(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::Seq(v) => visit_content_seq(v, visitor),
            Content::Map(v) => visit_content_map(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    #[allow(clippy::unnested_or_patterns)]
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let (variant, value) = match self.0 {
            Content::Map(value) => {
                let mut iter = value.into_iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Map,
                            &"map with a single key",
                        ));
                    }
                };
                // enums are encoded in json as maps with a single key:value pair
                if iter.next().is_some() {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Map,
                        &"map with a single key",
                    ));
                }
                (variant, Some(value))
            }
            s @ Content::String(_) | s @ Content::Str(_) => (s, None),
            other => {
                return Err(de::Error::invalid_type(
                    other.unexpected(),
                    &"string or map",
                ));
            }
        };

        visitor.visit_enum(EnumDeserializer::new(variant, value))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U64(v) => visitor.visit_u64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}

struct EnumDeserializer<'de, E>
where
    E: de::Error,
{
    variant: Content<'de>,
    value: Option<Content<'de>>,
    err: PhantomData<E>,
}
impl<'de, E> EnumDeserializer<'de, E>
where
    E: de::Error,
{
    pub fn new(variant: Content<'de>, value: Option<Content<'de>>) -> EnumDeserializer<'de, E> {
        EnumDeserializer {
            variant,
            value,
            err: PhantomData,
        }
    }
}

impl<'de, E> de::EnumAccess<'de> for EnumDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;
    type Variant = VariantDeserializer<'de, Self::Error>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), E>
    where
        V: de::DeserializeSeed<'de>,
    {
        let visitor = VariantDeserializer {
            value: self.value,
            err: PhantomData,
        };
        seed.deserialize(ContentDeserializer(self.variant, PhantomData))
            .map(|v| (v, visitor))
    }
}

struct VariantDeserializer<'de, E>
where
    E: de::Error,
{
    value: Option<Content<'de>>,
    err: PhantomData<E>,
}

impl<'de, E> de::VariantAccess<'de> for VariantDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn unit_variant(self) -> Result<(), E> {
        match self.value {
            Some(value) => de::Deserialize::deserialize(ContentDeserializer(value, PhantomData)),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, E>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(ContentDeserializer(value, PhantomData)),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Some(Content::Seq(v)) => de::Deserializer::deserialize_any(
                serde::de::value::SeqDeserializer::new(v.into_iter()),
                visitor,
            ),
            Some(other) => Err(de::Error::invalid_type(
                other.unexpected(),
                &"tuple variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Some(Content::Map(v)) => de::Deserializer::deserialize_any(
                serde::de::value::MapDeserializer::new(v.into_iter()),
                visitor,
            ),
            Some(Content::Seq(v)) => de::Deserializer::deserialize_any(
                serde::de::value::SeqDeserializer::new(v.into_iter()),
                visitor,
            ),
            Some(other) => Err(de::Error::invalid_type(
                other.unexpected(),
                &"struct variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}
