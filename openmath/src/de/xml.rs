#![allow(clippy::trait_duplication_in_bounds)]
#![allow(clippy::type_complexity)]
use std::{borrow::Cow, ops::ControlFlow};

use either::Either;
use quick_xml::events::{BytesStart, Event};

use crate::{OM, de::OMForeign};

#[derive(Debug, Clone, thiserror::Error)]
pub enum XmlReadError<E: std::fmt::Display> {
    #[error("{error} (at offset {position})")]
    Xml {
        error: quick_xml::errors::Error,
        position: u64,
    },
    #[error("invalid empty element at {0}")]
    Empty(u64),
    #[error("unknown OpenMath element at {0}")]
    UnexpectedTag(u64),
    #[error("missing OpenMath object")]
    NoObject,
    #[error("text node expected in xml element")]
    ExpectedText,
    #[error("invalid utf8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("invalid integer {0}")]
    InvalidInteger(String),
    #[error("invalid float {0}")]
    InvalidFloat(String),
    #[error("error converting OpenMath: {0}")]
    Conversion(E),
    #[error("OpenMath not fully convertible to target type")]
    NotFullyConvertible,
    #[error("attribute expected: {0}")]
    ExpectedAttribute(&'static str),
    #[error("error decoding base64 string: {0}")]
    Base64(#[from] crate::base64::Error),
    #[error("expected empty tag for {0} at {1}")]
    EmptyExpectedFor(&'static str, u64),
    #[error("expected non-empty tag for {0} at {1}")]
    NonEmptyExpectedFor(&'static str, u64),
    #[error("xml parsing requires string allocation (can't borrow) at {0}")]
    RequiresAllocating(u64),
    #[error("hexadecimal not yet implemented")]
    Hex,
    #[error("value for OMATP key-value-pair missing")]
    AttributeValue(u64),
}

pub(super) struct Ev<'e>(Event<'e>);
pub(super) struct NEv<'e>(Event<'e>);

pub(super) trait E<'e, 's: 'e>: AsRef<Event<'e>> {
    fn into_ref(self) -> Event<'e>;
    fn into_empty(self) -> BytesStart<'e>;

    fn as_empty(&self) -> &BytesStart<'e> {
        // SAFETY: private method; only gets called if known to be an Event::Empty!
        unsafe {
            let Event::Empty(s) = self.as_ref() else {
                std::hint::unreachable_unchecked()
            };
            s
        }
    }
    fn as_start(&self) -> &BytesStart<'e> {
        // SAFETY: private method; only gets called if known to be an Event::Start!
        unsafe {
            let Event::Start(s) = self.as_ref() else {
                std::hint::unreachable_unchecked()
            };
            s
        }
    }
    fn into_str<Err: std::fmt::Display>(self) -> Result<Cow<'s, [u8]>, XmlReadError<Err>>;
    fn get_attr_from_empty(&self, name: &str) -> Option<Cow<'s, [u8]>>;
    fn get_attr_from_start(&self, name: &str) -> Option<Cow<'s, [u8]>>;

    fn borrow_attr<'a>(&'a self, name: &str) -> Option<Cow<'a, [u8]>>
    where
        'e: 'a,
    {
        let es = self.as_empty();
        es.attributes().find_map(|a| {
            a.ok().and_then(|a| {
                if a.key.as_ref() == name.as_bytes() {
                    Some(a.value)
                } else {
                    None
                }
            })
        })
    }
}
impl<'e, 's: 'e> E<'e, 's> for Ev<'s> {
    #[inline]
    fn into_ref(self) -> Event<'e> {
        self.0
    }
    fn into_empty(self) -> BytesStart<'e> {
        // SAFETY: private method; only gets called if known to be an Event::Empty!
        unsafe {
            let Event::Empty(s) = self.0 else {
                std::hint::unreachable_unchecked()
            };
            s
        }
    }
    fn into_str<Err: std::fmt::Display>(self) -> Result<Cow<'s, [u8]>, XmlReadError<Err>> {
        let Event::Text(i) = self.0 else {
            return Err(XmlReadError::ExpectedText);
        };
        Ok(i.into_inner())
    }
    fn get_attr_from_empty(&self, name: &str) -> Option<Cow<'s, [u8]>> {
        let es = self.as_empty();
        es.attributes().find_map(|a| {
            a.ok().and_then(|a| {
                if a.key.as_ref() == name.as_bytes() {
                    // SAFETY: We know this is a slice of lifetime 's, but quick_xml doesn't
                    // return the most general applicable lifetime
                    Some(unsafe { std::mem::transmute::<Cow<'_, _>, Cow<'s, _>>(a.value) })
                } else {
                    None
                }
            })
        })
    }
    fn get_attr_from_start(&self, name: &str) -> Option<Cow<'s, [u8]>> {
        let es = self.as_start();
        es.attributes().find_map(|a| {
            a.ok().and_then(|a| {
                if a.key.as_ref() == name.as_bytes() {
                    // SAFETY: We know this is a slice of lifetime 's, but quick_xml doesn't
                    // return the most general applicable lifetime
                    Some(unsafe { std::mem::transmute::<Cow<'_, _>, Cow<'s, _>>(a.value) })
                } else {
                    None
                }
            })
        })
    }
}
impl<'e, 's: 'e> AsRef<Event<'e>> for Ev<'s> {
    fn as_ref(&self) -> &Event<'e> {
        &self.0
    }
}

impl<'e, 's: 'e> E<'e, 's> for NEv<'e> {
    #[inline]
    fn into_ref(self) -> Event<'e> {
        self.0
    }
    fn into_empty(self) -> BytesStart<'e> {
        // SAFETY: privae method; only gets called if known to be an Event::Empty!
        unsafe {
            let Event::Empty(s) = self.0 else {
                std::hint::unreachable_unchecked()
            };
            s
        }
    }

    fn into_str<Err: std::fmt::Display>(self) -> Result<Cow<'s, [u8]>, XmlReadError<Err>> {
        let Event::Text(i) = self.0 else {
            return Err(XmlReadError::ExpectedText);
        };
        Ok(Cow::Owned(i.into_inner().into_owned()))
    }
    fn get_attr_from_empty(&self, name: &str) -> Option<Cow<'s, [u8]>> {
        let es = self.as_empty();
        es.attributes().find_map(|a| {
            a.ok().and_then(|a| {
                if a.key.as_ref() == name.as_bytes() {
                    Some(Cow::Owned(a.value.into_owned()))
                } else {
                    None
                }
            })
        })
    }
    fn get_attr_from_start(&self, name: &str) -> Option<Cow<'s, [u8]>> {
        let es = self.as_start();
        es.attributes().find_map(|a| {
            a.ok().and_then(|a| {
                if a.key.as_ref() == name.as_bytes() {
                    Some(Cow::Owned(a.value.into_owned()))
                } else {
                    None
                }
            })
        })
    }
}
impl<'e> AsRef<Event<'e>> for NEv<'e> {
    fn as_ref(&self) -> &Event<'e> {
        &self.0
    }
}

fn cowfrombytes(cow: Cow<'_, [u8]>) -> Result<Cow<'_, str>, std::str::Utf8Error> {
    match cow {
        Cow::Borrowed(s) => Ok(Cow::Borrowed(std::str::from_utf8(s)?)),
        Cow::Owned(s) => Ok(Cow::Owned(
            String::from_utf8(s).map_err(|e| e.utf8_error())?,
        )),
    }
}

fn tryfrombytes<E: std::fmt::Display>(cow: Cow<'_, [u8]>) -> Result<Cow<'_, str>, XmlReadError<E>> {
    Ok(match cow {
        Cow::Borrowed(s) => Cow::Borrowed(std::str::from_utf8(s)?),
        Cow::Owned(s) => Cow::Owned(String::from_utf8(s).map_err(|e| e.utf8_error())?),
    })
}

pub(super) trait Readable<'s, O: super::OMDeserializable<'s>> {
    type Input;
    type E<'e>: E<'e, 's>
    where
        's: 'e,
        Self: 'e;
    //fn clear(&mut self);
    fn now(&self) -> u64;
    fn new(input: Self::Input) -> Self;
    fn next(&mut self) -> Result<Self::E<'_>, XmlReadError<O::Err>>;
    fn until(&mut self, tag: quick_xml::name::QName)
    -> Result<Cow<'s, [u8]>, XmlReadError<O::Err>>;

    #[allow(clippy::too_many_lines)]
    fn next_omforeign(
        &mut self,
        cd_base: &str,
    ) -> Result<ControlFlow<Either<O, super::OMForeign<'s, O>>, bool>, XmlReadError<O::Err>> {
        let now = self.now();
        let n = self.next()?;
        match n.as_ref() {
            Event::Empty(e) => match e.local_name().as_ref() {
                b"OMF" => Ok(ControlFlow::Break(
                    Self::omf(n.into_empty(), cd_base, Vec::new())?.map_right(OMForeign::OM),
                )), //next!(@ret Self::omf($event, &$cd_base)?),
                b"OMV" => Ok(ControlFlow::Break(
                    Self::omv(n, cd_base, Vec::new())?.map_right(OMForeign::OM),
                )),
                b"OMS" => Ok(ControlFlow::Break(
                    Self::oms(n, cd_base, Vec::new())?.map_right(OMForeign::OM),
                )),
                b"OMATTR" => Err(XmlReadError::NonEmptyExpectedFor("OMATTR", now)),
                b"OME" => Err(XmlReadError::NonEmptyExpectedFor("OME", now)),
                b"OMA" => Err(XmlReadError::NonEmptyExpectedFor("OMA", now)),
                b"OMBIND" => Err(XmlReadError::NonEmptyExpectedFor("OMBIND", now)),
                b"OMSTR" => Err(XmlReadError::NonEmptyExpectedFor("OMSTR", now)),
                b"OMI" => Err(XmlReadError::NonEmptyExpectedFor("OMI", now)),
                b"OMB" => Err(XmlReadError::NonEmptyExpectedFor("OMB", now)),
                b"OMFOREIGN" => Err(XmlReadError::NonEmptyExpectedFor("OMFOREIGN", now)),
                _ => Err(XmlReadError::UnexpectedTag(now)),
            },
            Event::Start(e) => match e.local_name().as_ref() {
                b"OMFOREIGN" => {
                    let encoding = n
                        .get_attr_from_start("encoding")
                        .map(tryfrombytes)
                        .transpose()?;
                    let name: smallvec::SmallVec<u8, 12> = e.name().0.into();
                    drop(n);
                    let end = quick_xml::name::QName(&name);
                    let value = tryfrombytes(self.until(end)?)?;
                    if !matches!(self.next()?.as_ref(), Event::End(_)) {
                        return Err(XmlReadError::UnexpectedTag(self.now()));
                    }
                    Ok(ControlFlow::Break(Either::Right(OMForeign::Foreign {
                        encoding,
                        value,
                    })))
                }
                b"OMI" => {
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omi(cd_base, Vec::new())?.map_right(OMForeign::OM),
                    ))
                }
                b"OMB" => {
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omb(cd_base, Vec::new())?.map_right(OMForeign::OM),
                    ))
                }
                b"OMSTR" => {
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omstr(cd_base, Vec::new())?.map_right(OMForeign::OM),
                    ))
                }
                b"OMA" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.oma(&cd_base, now, Vec::new())?
                            .map_right(OMForeign::OM),
                    ))
                }
                b"OMBIND" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.ombind(&cd_base, now, Vec::new())?
                            .map_right(OMForeign::OM),
                    ))
                }
                b"OME" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.ome(&cd_base, now, Vec::new())?
                            .map_right(OMForeign::OM),
                    ))
                }
                b"OMATTR" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omattr(&cd_base, Vec::new())?.map_right(OMForeign::OM),
                    ))
                }
                b"OMS" => Err(XmlReadError::EmptyExpectedFor("OMS", now)),
                b"OMF" => Err(XmlReadError::EmptyExpectedFor("OMF", now)),
                b"OMV" => Err(XmlReadError::EmptyExpectedFor("OMV", now)),
                _ => Err(XmlReadError::UnexpectedTag(now)),
            },
            Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                drop(n);
                self.next_omforeign(cd_base)
            }
            Event::Eof => Err(XmlReadError::NoObject),
            Event::End(_) => Ok(ControlFlow::Continue(true)),
            _ => Ok(ControlFlow::Continue(false)),
        }
    }

    fn handle_next(
        &mut self,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<ControlFlow<Either<O, OM<'s, O>>, bool>, XmlReadError<O::Err>> {
        let now = self.now();
        let n = self.next()?;
        match n.as_ref() {
            Event::Empty(e) => match e.local_name().as_ref() {
                b"OMF" => Ok(ControlFlow::Break(Self::omf(
                    n.into_empty(),
                    cd_base,
                    attrs,
                )?)), //next!(@ret Self::omf($event, &$cd_base)?),
                b"OMV" => Ok(ControlFlow::Break(Self::omv(n, cd_base, attrs)?)),
                b"OMS" => Ok(ControlFlow::Break(Self::oms(n, cd_base, attrs)?)),
                b"OME" => Err(XmlReadError::NonEmptyExpectedFor("OME", now)),
                b"OMA" => Err(XmlReadError::NonEmptyExpectedFor("OMA", now)),
                b"OMBIND" => Err(XmlReadError::NonEmptyExpectedFor("OMBIND", now)),
                b"OMSTR" => Err(XmlReadError::NonEmptyExpectedFor("OMSTR", now)),
                b"OMI" => Err(XmlReadError::NonEmptyExpectedFor("OMI", now)),
                b"OMB" => Err(XmlReadError::NonEmptyExpectedFor("OMB", now)),
                b"OMATTR" => Err(XmlReadError::NonEmptyExpectedFor("OMATTR", now)),
                _ => Err(XmlReadError::UnexpectedTag(now)),
            },
            Event::Start(e) => match e.local_name().as_ref() {
                b"OMI" => {
                    drop(n);
                    Ok(ControlFlow::Break(self.omi(cd_base, attrs)?))
                }
                b"OMB" => {
                    drop(n);
                    Ok(ControlFlow::Break(self.omb(cd_base, attrs)?))
                }
                b"OMSTR" => {
                    drop(n);
                    Ok(ControlFlow::Break(self.omstr(cd_base, attrs)?))
                }
                b"OMA" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(self.oma(&cd_base, now, attrs)?))
                }
                b"OMBIND" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(self.ombind(&cd_base, now, attrs)?))
                }
                b"OME" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(self.ome(&cd_base, now, attrs)?))
                }
                b"OMATTR" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    Ok(ControlFlow::Break(self.omattr(&cd_base, attrs)?))
                }
                b"OMS" => Err(XmlReadError::EmptyExpectedFor("OMS", now)),
                b"OMF" => Err(XmlReadError::EmptyExpectedFor("OMF", now)),
                b"OMV" => Err(XmlReadError::EmptyExpectedFor("OMV", now)),
                _ => Err(XmlReadError::UnexpectedTag(now)),
            },
            Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                drop(n);
                self.handle_next(cd_base, attrs)
            }
            Event::Eof => Err(XmlReadError::NoObject),
            Event::End(_) => Ok(ControlFlow::Continue(true)),
            _ => Ok(ControlFlow::Continue(false)),
        }
    }

    fn read_obj(mut self) -> Result<O, XmlReadError<O::Err>>
    where
        Self: Sized,
    {
        let cd_base = crate::OPENMATH_BASE_URI.as_str();
        loop {
            let now = self.now();
            let n = self.next()?;
            match n.as_ref() {
                Event::Start(s) if s.name().0 == b"OMOBJ" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cd_base = a.unwrap_or(Cow::Borrowed(cd_base));
                    drop(n);
                    return self.read(Some(&*cd_base));
                }
                Event::Text(t) if !t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                    return Err(XmlReadError::UnexpectedTag(now));
                }
                Event::Eof => return Err(XmlReadError::NoObject),
                Event::End(_) | Event::Empty(_) => return Err(XmlReadError::UnexpectedTag(now)),
                _ => (),
            }
        }
    }

    fn read(mut self, cd_base: Option<&str>) -> Result<O, XmlReadError<O::Err>>
    where
        Self: Sized,
    {
        let cd_base = cd_base.unwrap_or(crate::OPENMATH_BASE_URI.as_str());
        loop {
            if let ControlFlow::Break(b) = self.handle_next(cd_base, Vec::new())? {
                return match b {
                    Either::Left(e) => Ok(e),
                    Either::Right(_) => Err(XmlReadError::NotFullyConvertible),
                };
            }
        }
    }

    fn omi(
        &mut self,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let Event::Text(i) = self.next()?.into_ref() else {
            return Err(XmlReadError::ExpectedText);
        };
        let s = std::str::from_utf8(&i)?;
        if s.starts_with('x') || s.starts_with("-x") {
            return Err(XmlReadError::Hex);
        }
        let int = crate::Int::try_from(s)
            .map_err(|()| XmlReadError::InvalidInteger(s.to_string()))?
            .into_owned();
        if !matches!(self.next()?.as_ref(), Event::End(_)) {
            return Err(XmlReadError::UnexpectedTag(self.now()));
        }
        O::from_openmath(OM::OMI { int, attrs }, cd_base).map_err(XmlReadError::Conversion)
    }

    fn omb(
        &mut self,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        use crate::base64::Base64Decodable;
        let Event::Text(i) = self.next()?.into_ref() else {
            return Err(XmlReadError::ExpectedText);
        };
        let b: Result<Vec<u8>, _> = i.as_ref().iter().copied().decode_base64().flat().collect();
        let b = b?;
        if !matches!(self.next()?.as_ref(), Event::End(_)) {
            return Err(XmlReadError::UnexpectedTag(self.now()));
        }
        O::from_openmath(
            OM::OMB {
                bytes: b.into(),
                attrs,
            },
            cd_base,
        )
        .map_err(XmlReadError::Conversion)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn omf(
        event: BytesStart<'_>,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let Some(v) = event.attributes().find_map(|a| {
            a.ok().and_then(|a| {
                if a.key.as_ref() == b"hex" {
                    Some(None)
                } else if a.key.as_ref() == b"dec" {
                    Some(Some(a))
                } else {
                    None
                }
            })
        }) else {
            return Err(XmlReadError::ExpectedAttribute("dec"));
        };
        let Some(v) = v else {
            return Err(XmlReadError::Hex);
        };
        let s = std::str::from_utf8(&v.value)?;
        let float: f64 = s
            .parse()
            .map_err(|_| XmlReadError::InvalidFloat(s.to_string()))?;
        O::from_openmath(OM::OMF { float, attrs }, cd_base).map_err(XmlReadError::Conversion)
    }

    fn omstr(
        &mut self,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let cow = self.next()?.into_str()?;
        let string = tryfrombytes(cow)?;
        if !matches!(self.next()?.as_ref(), Event::End(_)) {
            return Err(XmlReadError::UnexpectedTag(self.now()));
        }
        O::from_openmath(OM::OMSTR { string, attrs }, cd_base).map_err(XmlReadError::Conversion)
    }

    fn omv(
        event: Self::E<'_>,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let Some(cow) = event.get_attr_from_empty("name") else {
            return Err(XmlReadError::ExpectedAttribute("name"));
        };
        let name = tryfrombytes(cow)?;
        O::from_openmath(OM::OMV { name, attrs }, cd_base).map_err(XmlReadError::Conversion)
    }

    fn oms(
        event: Self::E<'_>,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let Some(name) = event.get_attr_from_empty("name") else {
            return Err(XmlReadError::ExpectedAttribute("name"));
        };
        let name = tryfrombytes(name)?;

        let Some(cd_name) = event.get_attr_from_empty("cd") else {
            return Err(XmlReadError::ExpectedAttribute("cd"));
        };
        let cd_name = tryfrombytes(cd_name)?;

        if let Some(s) = event.borrow_attr("cdbase") {
            let s = std::str::from_utf8(s.as_ref())?;
            O::from_openmath(
                OM::OMS {
                    cd_name,
                    name,
                    attrs,
                },
                s,
            )
            .map_err(XmlReadError::Conversion)
        } else {
            O::from_openmath(
                OM::OMS {
                    cd_name,
                    name,
                    attrs,
                },
                cd_base,
            )
            .map_err(XmlReadError::Conversion)
        }
    }

    fn oma(
        &mut self,
        cd_base: &str,
        off: u64,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let ControlFlow::Break(head) = self.handle_next(cd_base, Vec::new())? else {
            return Err(XmlReadError::NonEmptyExpectedFor("OMA Applicant", off));
        };
        let mut args = Vec::with_capacity(2);
        loop {
            match self.handle_next(cd_base, Vec::new())? {
                ControlFlow::Break(a) => args.push(a),
                ControlFlow::Continue(true) => break,
                ControlFlow::Continue(false) => return Err(XmlReadError::UnexpectedTag(off)),
            }
        }
        O::from_openmath(
            OM::OMA {
                head: head.map_right(Box::new),
                args,
                attrs,
            },
            cd_base,
        )
        .map_err(XmlReadError::Conversion)
    }

    fn ome(
        &mut self,
        cd_base: &str,
        now: u64,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let (ocd_base, cd_name, name) = {
            let event = self.next()?;
            match event.as_ref() {
                Event::Empty(e) if e.local_name().as_ref() == b"OMS" => {
                    let Some(name) = event.get_attr_from_empty("name") else {
                        return Err(XmlReadError::ExpectedAttribute("name"));
                    };
                    let name = tryfrombytes(name)?;
                    let Some(cd_name) = event.get_attr_from_empty("cd") else {
                        return Err(XmlReadError::ExpectedAttribute("cd"));
                    };
                    let cd_name = tryfrombytes(cd_name)?;
                    let cd_base = event
                        .get_attr_from_empty("cdbase")
                        .map(tryfrombytes)
                        .transpose()?;
                    (cd_base, cd_name, name)
                }
                _ => return Err(XmlReadError::UnexpectedTag(now)),
            }
        };

        let mut args = Vec::with_capacity(2);
        loop {
            match self.next_omforeign(cd_base)? {
                ControlFlow::Break(a) => args.push(a),
                ControlFlow::Continue(true) => break,
                ControlFlow::Continue(false) => return Err(XmlReadError::UnexpectedTag(now)),
            }
        }

        O::from_openmath(
            OM::OME {
                cd_base: ocd_base,
                cd_name,
                name,
                args,
                attrs,
            },
            cd_base,
        )
        .map_err(XmlReadError::Conversion)
    }

    fn omattr_pairs(
        &mut self,
        cd_base: &str,
        attrs: &mut Vec<super::Attr<'s, O>>,
    ) -> Result<(), XmlReadError<O::Err>> {
        loop {
            let now = self.now();
            let next = self.next()?;
            match next.as_ref() {
                Event::End(_) => {
                    drop(next);
                    return Ok(());
                }
                Event::Empty(event) if event.local_name().as_ref() == b"OMS" => {
                    let Some(name) = next.get_attr_from_empty("name") else {
                        return Err(XmlReadError::ExpectedAttribute("name"));
                    };
                    let name = tryfrombytes(name)?;
                    let Some(cd_name) = next.get_attr_from_empty("cd") else {
                        return Err(XmlReadError::ExpectedAttribute("cd"));
                    };
                    let cd_name = tryfrombytes(cd_name)?;
                    let cd_base_o = next
                        .get_attr_from_empty("cdbase")
                        .map(tryfrombytes)
                        .transpose()?;
                    drop(next);
                    let now = self.now();
                    match self.next_omforeign(cd_base)? {
                        ControlFlow::Continue(true) => {
                            return Err(XmlReadError::AttributeValue(now));
                        }
                        ControlFlow::Continue(false) => {
                            return Err(XmlReadError::UnexpectedTag(now));
                        }
                        ControlFlow::Break(value) => attrs.push(super::Attr {
                            cd_base: cd_base_o,
                            cd_name,
                            name,
                            value,
                        }),
                    }
                }
                _ => return Err(XmlReadError::UnexpectedTag(now)),
            }
        }
    }

    fn omattr_i<R>(
        &mut self,
        cd_base: &str,
        mut attrs: Vec<super::Attr<'s, O>>,
        cont: impl FnOnce(&mut Self, Vec<super::Attr<'s, O>>) -> Result<R, XmlReadError<O::Err>>,
    ) -> Result<R, XmlReadError<O::Err>> {
        let now = self.now();
        let next = self.next()?;
        match next.as_ref() {
            Event::Empty(e) if e.local_name().as_ref() == b"OMATP" => {
                drop(next);
            }
            Event::Start(e) if e.local_name().as_ref() == b"OMATP" => {
                drop(next);
                self.omattr_pairs(cd_base, &mut attrs)?;
            }
            _ => return Err(XmlReadError::UnexpectedTag(now)),
        }
        let r = cont(self, attrs)?;
        if !matches!(self.next()?.as_ref(), Event::End(_)) {
            return Err(XmlReadError::UnexpectedTag(self.now()));
        }
        Ok(r)
    }

    #[inline]
    fn omattr(
        &mut self,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        self.omattr_i(cd_base, attrs, |nslf, attrs| {
            let now = nslf.now();
            let ControlFlow::Break(object) = nslf.handle_next(cd_base, attrs)? else {
                return Err(XmlReadError::NonEmptyExpectedFor("OMATTR", now));
            };
            Ok(object)
        })
    }

    fn omattr_or_var(
        &mut self,
        cd_base: &str,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Option<(Cow<'s, str>, Vec<super::Attr<'s, O>>)>, XmlReadError<O::Err>> {
        let now = self.now();
        let next = self.next()?;
        match next.as_ref() {
            Event::End(_) => {
                drop(next);
                Ok(None)
            }
            Event::Start(e) if e.local_name().as_ref() == b"OMATTR" => {
                let a = next
                    .get_attr_from_start("cdbase")
                    .map(cowfrombytes)
                    .transpose()?;
                let cd_base = a.as_deref().unwrap_or(cd_base);
                drop(next);
                self.omattr_i(cd_base, attrs, |nslf, attrs| {
                    nslf.omattr_or_var(cd_base, attrs)
                })
            }
            Event::Empty(e) if e.local_name().as_ref() == b"OMV" => {
                let Some(cow) = next.get_attr_from_empty("name") else {
                    return Err(XmlReadError::ExpectedAttribute("name"));
                };
                let s = tryfrombytes(cow)?;
                Ok(Some((s, attrs)))
            }
            _ => Err(XmlReadError::UnexpectedTag(now)),
        }
    }

    fn ombind(
        &mut self,
        cd_base: &str,
        off: u64,
        attrs: Vec<super::Attr<'s, O>>,
    ) -> Result<Either<O, OM<'s, O>>, XmlReadError<O::Err>> {
        let ControlFlow::Break(head) = self.handle_next(cd_base, Vec::new())? else {
            return Err(XmlReadError::NonEmptyExpectedFor("OMBIND", off));
        };

        let mut context = Vec::with_capacity(2);
        let now = self.now();
        let next = self.next()?;
        match next.as_ref() {
            Event::Empty(e) if e.local_name().as_ref() == b"OMBVAR" => {
                drop(next);
            }
            Event::Start(e) if e.local_name().as_ref() == b"OMBVAR" => {
                drop(next);
                while let Some(e) = self.omattr_or_var(cd_base, Vec::new())? {
                    context.push(e);
                }
            }
            _ => return Err(XmlReadError::UnexpectedTag(now)),
        }

        let ControlFlow::Break(body) = self.handle_next(cd_base, Vec::new())? else {
            return Err(XmlReadError::NonEmptyExpectedFor("OMBIND", now));
        };
        if !matches!(self.next()?.as_ref(), Event::End(_)) {
            return Err(XmlReadError::UnexpectedTag(self.now()));
        }

        O::from_openmath(
            OM::OMBIND {
                head: head.map_right(Box::new),
                context,
                body: body.map_right(Box::new),
                attrs,
            },
            cd_base,
        )
        .map_err(XmlReadError::Conversion)
    }
}

pub(super) struct FromString<'s> {
    orig: &'s [u8],
    inner: quick_xml::Reader<&'s [u8]>,
    position: u64,
}

impl<'s, O> Readable<'s, O> for FromString<'s>
where
    O: super::OMDeserializable<'s>,
{
    type Input = &'s str;
    type E<'e>
        = Ev<'s>
    where
        's: 'e;

    #[allow(clippy::cast_possible_truncation)]
    fn until(
        &mut self,
        tag: quick_xml::name::QName,
    ) -> Result<Cow<'s, [u8]>, XmlReadError<O::Err>> {
        let e = self.inner.read_to_end(tag).map_err(|e| XmlReadError::Xml {
            error: e,
            position: self.position,
        })?;
        Ok(Cow::Borrowed(&self.orig[e.start as usize..e.end as usize]))
    }

    #[inline]
    fn next(&mut self) -> Result<Self::E<'_>, XmlReadError<O::Err>> {
        self.position = self.inner.buffer_position();
        self.inner
            .read_event()
            .map_err(|e| XmlReadError::Xml {
                error: e,
                position: self.inner.error_position(),
            })
            .map(Ev)
    }

    /*#[inline]
    fn clear(&mut self) {}
    */

    #[inline]
    fn now(&self) -> u64 {
        self.position
    }
    #[inline]
    fn new(input: Self::Input) -> Self {
        Self {
            orig: input.as_bytes(),
            inner: quick_xml::Reader::from_str(input),
            position: 0,
        }
    }
}

pub(super) struct Reader<R: std::io::BufRead> {
    buf: Vec<u8>,
    inner: quick_xml::Reader<R>,
    position: u64,
    //cd_base: Cow<'static, str>,
}
impl<O, R: std::io::BufRead> Readable<'static, O> for Reader<R>
where
    O: super::OMDeserializable<'static>,
{
    type Input = R;
    type E<'e>
        = NEv<'e>
    where
        Self: 'e;

    fn until(
        &mut self,
        tag: quick_xml::name::QName,
    ) -> Result<Cow<'static, [u8]>, XmlReadError<O::Err>> {
        self.buf.clear();
        self.inner
            .read_to_end_into(tag, &mut self.buf)
            .map_err(|e| XmlReadError::Xml {
                error: e,
                position: self.position,
            })?;
        Ok(Cow::Owned(std::mem::take(&mut self.buf)))
    }

    #[inline]
    fn next(&mut self) -> Result<Self::E<'_>, XmlReadError<O::Err>> {
        self.buf.clear();
        self.position = self.inner.buffer_position();
        self.inner
            .read_event_into(&mut self.buf)
            .map_err(|e| XmlReadError::Xml {
                error: e,
                position: self.inner.error_position(),
            })
            .map(NEv)
    }

    /*#[inline]
    fn clear(&mut self) {
        self.buf.clear();
    }
    */

    #[inline]
    fn now(&self) -> u64 {
        self.position
    }
    #[inline]
    fn new(input: Self::Input) -> Self {
        Self {
            inner: quick_xml::Reader::from_reader(input),
            position: 0,
            buf: Vec::with_capacity(256),
        }
    }
}

// ------------------------------------------------------------------------------------------------

#[test]
fn xml_wut() {
    let _ = tracing_subscriber::fmt().try_init();
    let s = r#"<xml xmlns="http://www.openmath.org/OpenMath" xmlns:om="http://www.openmath.org/OpenMath">\
        <test>foo</test><om:test om:foo="bar"/>\
        <OMA cdbase="http://www.openmath.org/cd">
          <OMS cd="arith1" name="plus"/>
          <OMI>2</OMI>
          <OMI>2</OMI>
        </OMA>
    </xml>"#;

    let mut reader = quick_xml::Reader::from_str(s);
    loop {
        match reader.read_event().expect("works") {
            Event::Eof => break,
            Event::Start(e) => {
                tracing::info!(
                    "{:?}={:?}={:?}",
                    e.name(),
                    e.local_name(),
                    e.name().prefix()
                );
                for a in e.attributes() {
                    let a = a.expect("wut");
                    tracing::info!("{:?}={:?}", a.key, a.key.as_namespace_binding());
                }
            }
            Event::Empty(e) => {
                tracing::info!(
                    "{:?}={:?}={:?}",
                    e.name(),
                    e.local_name(),
                    e.name().prefix()
                );
            }
            e => tracing::info!("Other: {e:?}"),
        }
    }
}
