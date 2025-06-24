#![allow(unexpected_cfgs)]
#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]
/*! ## Features */
#![cfg_attr(doc,doc = document_features::document_features!())]
pub mod ser;
pub use ser::OMSerializable;

pub mod de;
pub use de::OMDeserializable;

mod int;
/// reexported for convenience
pub use either;
pub use int::Int;

/// The base URI of official OᴘᴇɴMᴀᴛʜ dictionaries (`http://www.openmath.org/OpenMath`)
pub static OPENMATH_BASE_URI: std::sync::LazyLock<url::Url> = std::sync::LazyLock::new(||
    // SAFETY: Known to be a valid Url
    unsafe{
        url::Url::parse("http://www.openmath.org/OpenMath").unwrap_unchecked()
    });
