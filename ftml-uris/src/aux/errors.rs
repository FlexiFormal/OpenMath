use thiserror::Error;

/// Characters universally disallowed in [`Uri`](crate::Uri)s: `\`,`{`,`}`
pub const ILLEGAL_CHARS: [char; 3] = ['\\', '{', '}'];

/// Errors that can occur during parsing / deserializing of [`Uri`](crate::Uri)s
#[derive(Debug, Clone, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UriParseError {
    /// Base Url is invalid
    #[error("url parse error: {0}")]
    URL(
        #[from]
        #[serde(with = "urlerr")]
        url::ParseError,
    ),
    /// Error occuring when trying to parse a Uri segment
    #[error("{0}")]
    Name(#[from] SegmentParseError),
    /// Base Url has a query or fragment component
    #[error("base url has query or fragment component")]
    HasQueryOrFragment,
    /// Base url can not be a base
    #[error("base url can not be a bse")]
    CannotBeABase,
    /// Missing query parameter for some particular [`UriKind`](crate::UriKind);
    /// e.g. missing <code>&[l](crate::UriComponentKind::l)=</code>-[`Language`](crate::Language) parameter for a
    /// [`DocumentUri`](crate::DocumentUri).
    #[error("missing query parameter ({part}) for {uri_kind}")]
    MissingPartFor {
        /// The kind of Uri that is implied by the present components
        uri_kind: crate::UriKind,
        /// The required component that is missing
        part: crate::UriComponentKind,
    },
    /// Unexpected component that shouldn't be there, e.g.
    /// after a <code>&[d](crate::UriComponentKind::d)=</code>-component implies a
    /// [`DocumentUri`](crate::DocumentUri), a <code>&[m](crate::UriComponentKind::m)=</code>-component.
    #[error("too many parts for {uri_kind}")]
    TooManyPartsFor {
        /// The kind of Uri that is implied by the present components
        uri_kind: crate::UriKind,
    },
    /// Some unknown query parameter
    #[error("unknown URL parameter")]
    UnknownParameter,
    /// Invalid langauge abbreviation in the
    /// <code>&[l](crate::UriComponentKind::l)=</code>-[`Language`](crate::Language) parameter
    #[error("invalid language parameter")]
    InvalidLanguage,
}

/// Error occuring when trying to parse a Uri segment
#[derive(Debug, Clone, Copy, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SegmentParseError {
    /// String is longer than [`u32::MAX`].
    #[error("string too long")]
    TooLong,
    /// One of the illegal characters (see [`ILLEGAL_CHARS`]) occurred,
    /// or a `/` in a [`SimpleUriName`](crate::SimpleUriName)
    #[error("character {0:?} not allowed in URI segments")]
    IllegalChar(char),
    /// Empty Uri component
    #[error("string is empty")]
    Empty,
}
impl From<strumbra::Error> for SegmentParseError {
    #[inline]
    fn from(_: strumbra::Error) -> Self {
        Self::TooLong
    }
}

#[cfg(feature = "serde")]
mod urlerr {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: serde::Serializer>(
        url: &url::ParseError,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        #[allow(clippy::enum_glob_use)]
        use url::ParseError::*;
        serializer.serialize_u8(match url {
            EmptyHost => 0,
            IdnaError => 1,
            InvalidPort => 2,
            InvalidIpv4Address => 3,
            InvalidIpv6Address => 4,
            InvalidDomainCharacter => 5,
            RelativeUrlWithoutBase => 6,
            RelativeUrlWithCannotBeABaseBase => 7,
            SetHostOnCannotBeABaseUrl => 8,
            Overflow => 9,
            e => {
                return Err(S::Error::custom(format!(
                    "unsupported url::ParseError: {e}"
                )));
            }
        })
    }
    pub fn deserialize<'de, D: serde::de::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<url::ParseError, D::Error> {
        struct De;
        impl serde::de::Visitor<'_> for De {
            type Value = url::ParseError;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte")
            }
            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                #[allow(clippy::enum_glob_use)]
                use url::ParseError::*;
                Ok(match v {
                    0 => EmptyHost,
                    1 => IdnaError,
                    2 => InvalidPort,
                    3 => InvalidIpv4Address,
                    4 => InvalidIpv6Address,
                    5 => InvalidDomainCharacter,
                    6 => RelativeUrlWithoutBase,
                    7 => RelativeUrlWithCannotBeABaseBase,
                    8 => SetHostOnCannotBeABaseUrl,
                    9 => Overflow,
                    i => {
                        return Err(E::custom(format!(
                            "unexpected code {i} for url::ParseError"
                        )));
                    }
                })
            }
        }
        deserializer.deserialize_u8(De)
    }
}
