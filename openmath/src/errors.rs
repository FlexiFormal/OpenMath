/*
use super::{AsOpenMath, MaybeForeign, OMObjectRef, OPENMATH_BASE_URI, URIRef};
use std::borrow::Cow;

pub const CD_NAME: &'static str = "error";
lazy_static! {
    pub static ref UNHANDLED_SYMBOL: URIRef<'static> = URIRef {
        base_uri: Cow::Borrowed(&OPENMATH_BASE_URI),
        cd_name: Cow::Borrowed(CD_NAME),
        name: Cow::Borrowed("unhandled_symbol")
    };
}
pub struct OMError<'l, T: AsOpenMath> {
    pub err: URIRef<'l>,
    pub args: Vec<MaybeForeign<'l, T>>,
}

impl<'l, T: AsOpenMath> OMError<'l, T> {
    pub fn unhandled_symbol(uri: URIRef<'l>) -> Self {
        Self {
            err: UNHANDLED_SYMBOL.copy(),
            args: vec![MaybeForeign::OM(OMObjectRef::OMS(uri))],
        }
    }
}
*/
