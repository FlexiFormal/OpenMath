#![allow(clippy::trait_duplication_in_bounds)]
#![allow(clippy::type_complexity)]
use std::{borrow::Cow, ops::ControlFlow};

use quick_xml::events::{BytesStart, Event};

use crate::{
    OM, OMDeserializable,
    de::{Args, Attrs, Vars},
};
type Attr<'s, O> = crate::Attr<'s, crate::OMMaybeForeign<'s, <O as OMDeserializable<'s>>::Ret>>;

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

    fn need_end(&mut self) -> Result<(), XmlReadError<O::Err>> {
        self.with_next(|e: Self::E<'_>, now| {
            if matches!(e.as_ref(), Event::End(_)) {
                Ok(())
            } else {
                Err(XmlReadError::UnexpectedTag(now))
            }
        })
    }

    fn with_next<R>(
        &mut self,
        f: impl FnOnce(Self::E<'_>, u64) -> Result<R, XmlReadError<O::Err>>,
    ) -> Result<R, XmlReadError<O::Err>> {
        let now = self.now();
        let n = self.next()?;
        match n.as_ref() {
            Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                drop(n);
                self.with_next(f)
            }
            _ => f(n, now),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn next_omforeign(
        &mut self,
        cdbase: &str,
    ) -> Result<ControlFlow<crate::OMMaybeForeign<'s, O::Ret>, bool>, XmlReadError<O::Err>> {
        let now = self.now();
        let n = self.next()?;
        match n.as_ref() {
            Event::Empty(e) => match e.local_name().as_ref() {
                b"OMF" => Ok(ControlFlow::Break(
                    Self::omf(n.into_empty(), cdbase, Attrs::new())
                        .map(crate::OMMaybeForeign::OM)?,
                )), //next!(@ret Self::omf($event, &$cdbase)?),
                b"OMV" => Ok(ControlFlow::Break(
                    Self::omv(n, cdbase, Attrs::new()).map(crate::OMMaybeForeign::OM)?,
                )),
                b"OMS" => Ok(ControlFlow::Break(
                    Self::oms(n, cdbase, Attrs::new()).map(crate::OMMaybeForeign::OM)?,
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
                    Ok(ControlFlow::Break(crate::OMMaybeForeign::Foreign {
                        encoding,
                        value,
                    }))
                }
                b"OMI" => {
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omi(cdbase, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OMB" => {
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omb(cdbase, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OMSTR" => {
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omstr(cdbase, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OMA" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.oma(&cdbase, now, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OMBIND" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.ombind(&cdbase, now, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OME" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.ome(&cdbase, now, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OMATTR" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(
                        self.omattr(&cdbase, Attrs::new())
                            .map(crate::OMMaybeForeign::OM)?,
                    ))
                }
                b"OMS" => Err(XmlReadError::EmptyExpectedFor("OMS", now)),
                b"OMF" => Err(XmlReadError::EmptyExpectedFor("OMF", now)),
                b"OMV" => Err(XmlReadError::EmptyExpectedFor("OMV", now)),
                _ => Err(XmlReadError::UnexpectedTag(now)),
            },
            Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                drop(n);
                self.next_omforeign(cdbase)
            }
            Event::Eof => Err(XmlReadError::NoObject),
            Event::End(_) => Ok(ControlFlow::Continue(true)),
            _ => Ok(ControlFlow::Continue(false)),
        }
    }

    fn handle_next(
        &mut self,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<ControlFlow<O::Ret, bool>, XmlReadError<O::Err>> {
        let now = self.now();
        let n = self.next()?;
        match n.as_ref() {
            Event::Empty(e) => match e.local_name().as_ref() {
                b"OMF" => Ok(ControlFlow::Break(Self::omf(
                    n.into_empty(),
                    cdbase,
                    attrs,
                )?)), //next!(@ret Self::omf($event, &$cdbase)?),
                b"OMV" => Ok(ControlFlow::Break(Self::omv(n, cdbase, attrs)?)),
                b"OMS" => Ok(ControlFlow::Break(Self::oms(n, cdbase, attrs)?)),
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
                    Ok(ControlFlow::Break(self.omi(cdbase, attrs)?))
                }
                b"OMB" => {
                    drop(n);
                    Ok(ControlFlow::Break(self.omb(cdbase, attrs)?))
                }
                b"OMSTR" => {
                    drop(n);
                    Ok(ControlFlow::Break(self.omstr(cdbase, attrs)?))
                }
                b"OMA" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(self.oma(&cdbase, now, attrs)?))
                }
                b"OMBIND" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(self.ombind(&cdbase, now, attrs)?))
                }
                b"OME" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(self.ome(&cdbase, now, attrs)?))
                }
                b"OMATTR" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    Ok(ControlFlow::Break(self.omattr(&cdbase, attrs)?))
                }
                b"OMS" => Err(XmlReadError::EmptyExpectedFor("OMS", now)),
                b"OMF" => Err(XmlReadError::EmptyExpectedFor("OMF", now)),
                b"OMV" => Err(XmlReadError::EmptyExpectedFor("OMV", now)),
                _ => Err(XmlReadError::UnexpectedTag(now)),
            },
            Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                drop(n);
                self.handle_next(cdbase, attrs)
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
        let cdbase = crate::CD_BASE;
        loop {
            let now = self.now();
            let n = self.next()?;
            match n.as_ref() {
                Event::Start(s) if s.name().0 == b"OMOBJ" => {
                    let a = n
                        .get_attr_from_start("cdbase")
                        .map(cowfrombytes)
                        .transpose()?;
                    let cdbase = a.unwrap_or(Cow::Borrowed(cdbase));
                    drop(n);
                    return self.read(Some(&*cdbase));
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

    fn read(mut self, cdbase: Option<&str>) -> Result<O, XmlReadError<O::Err>>
    where
        Self: Sized,
    {
        let cdbase = cdbase.unwrap_or(crate::CD_BASE);
        loop {
            if let ControlFlow::Break(b) = self.handle_next(cdbase, Attrs::new())? {
                return b.try_into().map_err(|_| XmlReadError::NotFullyConvertible);
            }
        }
    }

    fn omi(
        &mut self,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        let int = self.with_next(|e: Self::E<'_>, _| {
            let Event::Text(i) = e.into_ref() else {
                return Err(XmlReadError::ExpectedText);
            };
            let s = std::str::from_utf8(&i)?;
            if s.starts_with('x') || s.starts_with("-x") {
                return Err(XmlReadError::Hex);
            }
            let int = crate::Int::try_from(s)
                .map_err(|()| XmlReadError::InvalidInteger(s.to_string()))?
                .into_owned();
            Ok(int)
        })?;
        self.need_end()?;

        O::from_openmath(OM::OMI { int, attrs }, cdbase).map_err(XmlReadError::Conversion)
    }

    fn omb(
        &mut self,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        use crate::base64::Base64Decodable;
        let bytes = self.with_next(|e: Self::E<'_>, _| {
            let Event::Text(i) = e.into_ref() else {
                return Err(XmlReadError::ExpectedText);
            };
            let b: Result<Vec<u8>, _> = i.as_ref().iter().copied().decode_base64().flat().collect();
            Ok(b?)
        })?;
        self.need_end()?;
        O::from_openmath(
            OM::OMB {
                bytes: bytes.into(),
                attrs,
            },
            cdbase,
        )
        .map_err(XmlReadError::Conversion)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn omf(
        event: BytesStart<'_>,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
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
        O::from_openmath(OM::OMF { float, attrs }, cdbase).map_err(XmlReadError::Conversion)
    }

    fn omstr(
        &mut self,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        let cow = self.next()?.into_str()?;
        let string = tryfrombytes(cow)?;
        self.need_end()?;
        O::from_openmath(OM::OMSTR { string, attrs }, cdbase).map_err(XmlReadError::Conversion)
    }

    fn omv(
        event: Self::E<'_>,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        let Some(cow) = event.get_attr_from_empty("name") else {
            return Err(XmlReadError::ExpectedAttribute("name"));
        };
        let name = tryfrombytes(cow)?;
        O::from_openmath(OM::OMV { name, attrs }, cdbase).map_err(XmlReadError::Conversion)
    }

    fn oms(
        event: Self::E<'_>,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
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
                    cd: cd_name,
                    name,
                    attrs,
                },
                s,
            )
            .map_err(XmlReadError::Conversion)
        } else {
            O::from_openmath(
                OM::OMS {
                    cd: cd_name,
                    name,
                    attrs,
                },
                cdbase,
            )
            .map_err(XmlReadError::Conversion)
        }
    }

    fn oma(
        &mut self,
        cdbase: &str,
        off: u64,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        let ControlFlow::Break(head) = self.handle_next(cdbase, Attrs::new())? else {
            return Err(XmlReadError::NonEmptyExpectedFor("OMA Applicant", off));
        };

        let mut args = Args::new();
        loop {
            match self.handle_next(cdbase, Attrs::new())? {
                ControlFlow::Break(a) => args.push(a),
                ControlFlow::Continue(true) => break,
                ControlFlow::Continue(false) => {
                    return Err(XmlReadError::UnexpectedTag(off));
                }
            }
        }

        O::from_openmath(
            OM::OMA {
                applicant: head,
                arguments: args,
                attrs,
            },
            cdbase,
        )
        .map_err(XmlReadError::Conversion)
    }

    fn ome(
        &mut self,
        cdbase: &str,
        now: u64,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        let (ocdbase, cd, name) = self.with_next(|event: Self::E<'_>, _| match event.as_ref() {
            Event::Empty(e) if e.local_name().as_ref() == b"OMS" => {
                let Some(name) = event.get_attr_from_empty("name") else {
                    return Err(XmlReadError::ExpectedAttribute("name"));
                };
                let name = tryfrombytes(name)?;
                let Some(cd_name) = event.get_attr_from_empty("cd") else {
                    return Err(XmlReadError::ExpectedAttribute("cd"));
                };
                let cd_name = tryfrombytes(cd_name)?;
                let cdbase = event
                    .get_attr_from_empty("cdbase")
                    .map(tryfrombytes)
                    .transpose()?;
                Ok((cdbase, cd_name, name))
            }
            _ => Err(XmlReadError::UnexpectedTag(now)),
        })?;

        let mut arguments = Vec::with_capacity(2);
        loop {
            match self.next_omforeign(cdbase)? {
                ControlFlow::Break(a) => arguments.push(a),
                ControlFlow::Continue(true) => break,
                ControlFlow::Continue(false) => return Err(XmlReadError::UnexpectedTag(now)),
            }
        }

        O::from_openmath(
            OM::OME {
                cdbase: ocdbase,
                cd,
                name,
                arguments,
                attrs,
            },
            cdbase,
        )
        .map_err(XmlReadError::Conversion)
    }

    fn omattr_pairs(
        &mut self,
        cdbase: &str,
        attrs: &mut Attrs<Attr<'s, O>>,
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
                    let cdbase_o = next
                        .get_attr_from_empty("cdbase")
                        .map(tryfrombytes)
                        .transpose()?;
                    drop(next);
                    let now = self.now();
                    match self.next_omforeign(cdbase)? {
                        ControlFlow::Continue(true) => {
                            return Err(XmlReadError::AttributeValue(now));
                        }
                        ControlFlow::Continue(false) => {
                            return Err(XmlReadError::UnexpectedTag(now));
                        }
                        ControlFlow::Break(value) => {
                            attrs.push(Attr::<O> {
                                cdbase: cdbase_o,
                                cd: cd_name,
                                name,
                                value,
                            });
                        }
                    }
                }
                Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => drop(next),
                _ => return Err(XmlReadError::UnexpectedTag(now)),
            }
        }
    }

    fn omattr_i<R>(
        &mut self,
        cdbase: &str,
        mut attrs: Attrs<Attr<'s, O>>,
        cont: impl FnOnce(&mut Self, Attrs<Attr<'s, O>>) -> Result<R, XmlReadError<O::Err>>,
    ) -> Result<R, XmlReadError<O::Err>> {
        let do_pairs = self.with_next(|n: Self::E<'_>, now| match n.as_ref() {
            Event::Empty(e) if e.local_name().as_ref() == b"OMATP" => {
                drop(n);
                Ok(false)
            }
            Event::Start(e) if e.local_name().as_ref() == b"OMATP" => {
                drop(n);
                Ok(true)
            }
            _ => Err(XmlReadError::UnexpectedTag(now)),
        })?;
        if do_pairs {
            self.omattr_pairs(cdbase, &mut attrs)?;
        }
        let r = cont(self, attrs)?;
        Ok(r)
    }

    #[inline]
    fn omattr(
        &mut self,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        self.omattr_i(cdbase, attrs, |nslf, attrs| {
            let now = nslf.now();
            let ControlFlow::Break(object) = nslf.handle_next(cdbase, attrs)? else {
                return Err(XmlReadError::NonEmptyExpectedFor("OMATTR", now));
            };
            nslf.need_end()?;
            Ok(object)
        })
    }

    fn omattr_or_var(
        &mut self,
        cdbase: &str,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<Option<(Cow<'s, str>, Attrs<Attr<'s, O>>)>, XmlReadError<O::Err>> {
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
                let cdbase = a.as_deref().unwrap_or(cdbase);
                drop(next);
                self.omattr_i(cdbase, attrs, |nslf, attrs| {
                    let r = nslf.omattr_or_var(cdbase, attrs)?;
                    nslf.need_end()?;
                    Ok(r)
                })
            }
            Event::Empty(e) if e.local_name().as_ref() == b"OMV" => {
                let Some(cow) = next.get_attr_from_empty("name") else {
                    return Err(XmlReadError::ExpectedAttribute("name"));
                };
                let s = tryfrombytes(cow)?;
                Ok(Some((s, attrs)))
            }
            Event::Text(t) if t.as_ref().iter().all(u8::is_ascii_whitespace) => {
                drop(next);
                self.omattr_or_var(cdbase, attrs)
            }
            _ => Err(XmlReadError::UnexpectedTag(now)),
        }
    }

    fn ombind(
        &mut self,
        cdbase: &str,
        off: u64,
        attrs: Attrs<Attr<'s, O>>,
    ) -> Result<O::Ret, XmlReadError<O::Err>> {
        let ControlFlow::Break(head) = self.handle_next(cdbase, Attrs::new())? else {
            return Err(XmlReadError::NonEmptyExpectedFor("OMBIND", off));
        };

        let mut context = Vars::new();
        let ombvar = self.with_next(|n: Self::E<'_>, now| match n.as_ref() {
            Event::Empty(e) if e.local_name().as_ref() == b"OMBVAR" => {
                drop(n);
                Ok(false)
            }
            Event::Start(e) if e.local_name().as_ref() == b"OMBVAR" => {
                drop(n);
                Ok(true)
            }
            _ => Err(XmlReadError::UnexpectedTag(now)),
        })?;
        if ombvar {
            while let Some(e) = self.omattr_or_var(cdbase, Attrs::new())? {
                context.push(e);
            }
        }

        let now = self.now();
        let ControlFlow::Break(body) = self.handle_next(cdbase, Attrs::new())? else {
            return Err(XmlReadError::NonEmptyExpectedFor("OMBIND", now));
        };
        self.need_end()?;

        O::from_openmath(
            OM::OMBIND {
                binder: head,
                variables: context,
                object: body,
                attrs,
            },
            cdbase,
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
        Ok(Cow::Borrowed(
            self.orig[e.start as usize..e.end as usize].trim_ascii(),
        ))
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
    //cdbase: Cow<'static, str>,
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
        self.buf = self
            .buf
            .drain(
                self.buf.len() - self.buf.trim_ascii_start().len()..self.buf.trim_ascii_end().len(),
            )
            .collect();
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
