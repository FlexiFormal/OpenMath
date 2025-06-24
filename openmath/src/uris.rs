/*
use std::borrow::Cow;

macro_rules! str {
    ($e:expr) => {
        unsafe { strumbra::UniqueString::try_from($e).unwrap_unchecked() }
    };
    (@ $e:expr) => {
        match $e {
            Cow::Owned(e) => str!(e),
            Cow::Borrowed(e) => str!(e),
        }
    };
}

pub trait URILike {
    fn base(&self) -> &impl std::fmt::Display;
    fn cd_name(&self) -> &impl std::fmt::Display;
    fn name(&self) -> &impl std::fmt::Display;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct URIRef<'l> {
    pub base_uri: Cow<'l, url::Url>,
    pub cd_name: Cow<'l, str>,
    pub name: Cow<'l, str>,
}

impl URIRef<'_> {
    pub fn into_owned(self) -> URI {
        URI {
            base_uri: Box::new(self.base_uri.into_owned()),
            cd_name: str!(@ self.cd_name),
            name: str!(@ self.name),
        }
    }
}
impl URIRef<'static> {
    pub(crate) fn copy(&self) -> Self {
        URIRef {
            base_uri: self.base_uri.clone(),
            cd_name: self.cd_name.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct URI {
    pub base_uri: Box<url::Url>,
    pub cd_name: strumbra::UniqueString,
    pub name: strumbra::UniqueString,
}
*/
