use std::path::Path;

/// Represents supported languages in [`DocumentUri`](crate::DocumentUri)s
///
/// This enum provides a ist of supported languages, their Unicode flag representations and SVG flag icons.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Default,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
    strum::EnumProperty,
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::DeserializeFromStr, serde_with::SerializeDisplay)
)]
#[non_exhaustive]
#[repr(u8)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
pub enum Language {
    /// English language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): en)
    ///
    /// Default language variant. Uses the United Kingdom flag representation.
    #[default]
    #[strum(
        to_string = "en",
        props(
            unicode = "🇬🇧",
            svg = r##"<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-gb" viewBox="0 0 640 480">
    <path fill="#012169" d="M0 0h640v480H0z"/>
    <path fill="#FFF" d="m75 0 244 181L562 0h78v62L400 241l240 178v61h-80L320 301 81 480H0v-60l239-178L0 64V0z"/>
    <path fill="#C8102E" d="m424 281 216 159v40L369 281zm-184 20 6 35L54 480H0zM640 0v3L391 191l2-44L590 0zM0 0l239 176h-60L0 42z"/>
    <path fill="#FFF" d="M241 0v480h160V0zM0 160v160h640V160z"/>
    <path fill="#C8102E" d="M0 193v96h640v-96zM273 0v480h96V0z"/>
    </svg>"##
        )
    )]
    English = 0,

    /// German language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): de)
    ///
    /// Uses the Germany flag representation.
    #[strum(
        to_string = "de",
        props(
            unicode = "🇩🇪",
            svg = r##"<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-de" viewBox="0 0 640 480">
<path fill="#fc0" d="M0 320h640v160H0z"/>
<path fill="#000001" d="M0 0h640v160H0z"/>
<path fill="red" d="M0 160h640v160H0z"/>
</svg>
"##
        )
    )]
    German = 1,

    /// French language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): fr)
    ///
    /// Uses the France flag representation.
    #[strum(
        to_string = "fr",
        props(
            unicode = "🇫🇷",
            svg = r##"
<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-fr" viewBox="0 0 640 480">
<path fill="#fff" d="M0 0h640v480H0z"/>
<path fill="#000091" d="M0 0h213.3v480H0z"/>
<path fill="#e1000f" d="M426.7 0H640v480H426.7z"/>
</svg>
"##
        )
    )]
    French = 2,

    /// Romanian language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): ro)
    ///
    /// Uses the Romania flag representation.
    #[strum(
        to_string = "ro",
        props(
            unicode = "🇷🇴",
            svg = r##"
<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-ro" viewBox="0 0 640 480">
<g fill-rule="evenodd" stroke-width="1pt">
<path fill="#00319c" d="M0 0h213.3v480H0z"/>
<path fill="#ffde00" d="M213.3 0h213.4v480H213.3z"/>
<path fill="#de2110" d="M426.7 0H640v480H426.7z"/>
</g>
</svg>
"##
        )
    )]
    Romanian = 3,

    /// Arabic language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): ar)
    ///
    /// Uses the United Arab Emirates flag representation.
    #[strum(
        to_string = "ar",
        props(
            unicode = "🇦🇪",
            svg = r##"
<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-ae" viewBox="0 0 640 480">
<path fill="#00732f" d="M0 0h640v160H0z"/>
<path fill="#fff" d="M0 160h640v160H0z"/>
<path fill="#000001" d="M0 320h640v160H0z"/>
<path fill="red" d="M0 0h220v480H0z"/>
</svg>
"##
        )
    )]
    Arabic = 4,

    /// Bulgarian language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): bg)
    ///
    /// Uses the Bulgaria flag representation.
    #[strum(
        to_string = "bg",
        props(
            unicode = "🇧🇬",
            svg = r##"<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-bg" viewBox="0 0 640 480">
<path fill="#fff" d="M0 0h640v160H0z"/>
<path fill="#00966e" d="M0 160h640v160H0z"/>
<path fill="#d62612" d="M0 320h640v160H0z"/>
</svg>"##
        )
    )]
    Bulgarian = 5,

    /// Russian language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): ru)
    ///
    /// Uses the Russia flag representation.
    #[strum(
        to_string = "ru",
        props(
            unicode = "🇷🇺",
            svg = r##"
<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-ru" viewBox="0 0 640 480">
<path fill="#fff" d="M0 0h640v160H0z"/>
<path fill="#0039a6" d="M0 160h640v160H0z"/>
<path fill="#d52b1e" d="M0 320h640v160H0z"/>
</svg>"##
        )
    )]
    Russian = 6,

    /// Finnish language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): fi)
    ///
    /// Uses the Finland flag representation.
    #[strum(
        to_string = "fi",
        props(
            unicode = "🇫🇮",
            svg = r##"<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-fi" viewBox="0 0 640 480">
<path fill="#fff" d="M0 0h640v480H0z"/>
<path fill="#002f6c" d="M0 174.5h640v131H0z"/>
<path fill="#002f6c" d="M175.5 0h130.9v480h-131z"/>
</svg>"##
        )
    )]
    Finnish = 7,

    /// Turkish language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): tr)
    ///
    /// Uses the Turkey flag representation.
    #[strum(
        to_string = "tr",
        props(
            unicode = "🇹🇷",
            svg = r##"
<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-tr" viewBox="0 0 640 480">
<g fill-rule="evenodd">
<path fill="#e30a17" d="M0 0h640v480H0z"/>
<path fill="#fff" d="M407 247.5c0 66.2-54.6 119.9-122 119.9s-122-53.7-122-120 54.6-119.8 122-119.8 122 53.7 122 119.9"/>
<path fill="#e30a17" d="M413 247.5c0 53-43.6 95.9-97.5 95.9s-97.6-43-97.6-96 43.7-95.8 97.6-95.8 97.6 42.9 97.6 95.9z"/>
<path fill="#fff" d="m430.7 191.5-1 44.3-41.3 11.2 40.8 14.5-1 40.7 26.5-31.8 40.2 14-23.2-34.1 28.3-33.9-43.5 12-25.8-37z"/>
</g>
</svg>
"##
        )
    )]
    Turkish = 8,

    /// Slovenian language ([ISO 639-1](https://en.wikipedia.org/wiki/ISO_639): sl)
    ///
    /// Uses the Slovenia flag representation.
    #[strum(
        to_string = "sl",
        props(
            unicode = "🇸🇮",
            svg = r##"
<svg width="1.5em" xmlns="http://www.w3.org/2000/svg" id="flag-icons-si" viewBox="0 0 640 480">
<defs>
<clipPath id="si-a">
<path fill-opacity=".7" d="M-15 0h682.6v512H-15.1z"/>
</clipPath>
</defs>
<g fill-rule="evenodd" stroke-width="1pt" clip-path="url(#si-a)" transform="translate(14.1)scale(.9375)">
<path fill="#fff" d="M-62 0H962v512H-62z"/>
<path fill="#d50000" d="M-62 341.3H962V512H-62z"/>
<path fill="#0000bf" d="M-62 170.7H962v170.6H-62z"/>
<path fill="#d50000" d="M228.4 93c-4 61.6-6.4 95.4-15.7 111-10.2 16.8-20 29.1-59.7 44-39.6-14.9-49.4-27.2-59.6-44-9.4-15.6-11.7-49.4-15.7-111l5.8-2c11.8-3.6 20.6-6.5 27.1-7.8 9.3-2 17.3-4.2 42.3-4.7 25 .4 33 2.8 42.3 4.8 6.4 1.4 15.6 4 27.3 7.7z"/>
<path fill="#0000bf" d="M222.6 91c-3.8 61.5-7 89.7-12 103.2-9.6 23.2-24.8 35.9-57.6 48-32.8-12.1-48-24.8-57.7-48-5-13.6-8-41.7-11.8-103.3 11.6-3.6 20.6-6.4 27.1-7.7 9.3-2 17.3-4.3 42.3-4.7 25 .4 33 2.7 42.3 4.7a284 284 0 0 1 27.4 7.7z"/>
<path fill="#ffdf00" d="m153 109.8 1.5 3.7 7 1-4.5 2.7 4.3 2.9-6.3 1-2 3.4-2-3.5-6-.8 4-3-4.2-2.7 6.7-1z"/>
<path fill="#fff" d="m208.3 179.6-3.9-3-2.7-4.6-5.4-4.7-2.9-4.7-5.4-4.9-2.6-4.7-3-2.3-1.8-1.9-5 4.3-2.6 4.7-3.3 3-3.7-2.9-2.7-4.8-10.3-18.3-10.3 18.3-2.7 4.8-3.7 2.9-3.3-3-2.7-4.7-4.9-4.3-1.9 1.8-2.9 2.4-2.6 4.7-5.4 4.9-2.9 4.7-5.4 4.7-2.7 4.6-3.9 3a65.8 65.8 0 0 0 18.6 36.3 107 107 0 0 0 36.6 20.5 104.1 104.1 0 0 0 36.8-20.5c5.8-6 16.6-19.3 18.6-36.3"/>
<path fill="#ffdf00" d="m169.4 83.9 1.6 3.7 7 1-4.6 2.7 4.4 2.9-6.3 1-2 3.4-2-3.5-6-.8 4-3-4.2-2.7 6.6-1zm-33 0 1.6 3.7 7 .9-4.5 2.7 4.3 2.9-6.3 1-2 3.4-2-3.4-6-.9 4-3-4.2-2.7 6.7-1z"/>
<path fill="#0000bf" d="M199.7 203h-7.4l-7-.5-8.3-4h-9.4l-8.1 4-6.5.6-6.4-.6-8.1-4H129l-8.4 4-6.9.6-7.6-.1-3.6-6.2.1-.2 11.2 1.9 6.9-.5 8.3-4.1h9.4l8.2 4 6.4.6 6.5-.6 8.1-4h9.4l8.4 4 6.9.6 10.8-2 .2.4zm-86.4 9.5 7.4-.5 8.3-4h9.4l8.2 4 6.4.5 6.4-.5 8.2-4h9.4l8.3 4 7.5.5 4.8-6h-.1l-5.2 1.4-6.9-.5-8.3-4h-9.4l-8.2 4-6.4.6-6.5-.6-8.1-4H129l-8.4 4-6.9.6-5-1.3v.2l4.5 5.6z"/>
</g>
</svg>
"##
        )
    )]
    Slovenian = 9,
}

impl Language {
    pub(crate) const SEPARATOR: char = 'l';

    /// Returns the Unicode flag emoji for this language.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ftml_uris::prelude::*;
    /// assert_eq!(Language::French.flag_unicode(), "🇫🇷");
    /// assert_eq!(Language::German.flag_unicode(), "🇩🇪");
    /// ```
    #[inline]
    #[must_use]
    pub fn flag_unicode(self) -> &'static str {
        use strum::EnumProperty;
        // safe, because unicode property is defined on all cases
        unsafe { self.get_str("unicode").unwrap_unchecked() }
    }

    /// Returns the SVG flag representation for this language, suitable for embedding in web pages or
    /// other SVG-compatible contexts.
    #[inline]
    #[must_use]
    pub fn flag_svg(self) -> &'static str {
        use strum::EnumProperty;
        // safe, because svg property is defined on all cases
        unsafe { self.get_str("svg").unwrap_unchecked() }
    }

    /// Extracts language from a relative file path.
    ///
    /// This method parses file paths to extract language codes, particularly
    /// for files with `.tex` or `.html` extensions. It removes known file
    /// extensions before attempting to parse the language code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ftml_uris::prelude::*;
    /// assert_eq!(Language::from_rel_path("document.fr.tex"), Language::French);
    /// assert_eq!(Language::from_rel_path("page.de.html"), Language::German);
    /// // No language code => returns English as default
    /// assert_eq!(Language::from_rel_path("file.txt"), Language::English);
    /// ```
    #[must_use]
    pub fn from_rel_path(mut s: &str) -> Self {
        const FILENAMES: [&str; 2] = [".tex", ".html"];
        s = FILENAMES
            .iter()
            .find_map(|e| s.strip_suffix(e))
            .unwrap_or(s);
        Self::check(s)
    }

    #[inline]
    fn check(s: &str) -> Self {
        if s.len() < 3 {
            return Self::default();
        }
        if s.as_bytes().get(s.len() - 3).copied() != Some(b'.') {
            return Self::default();
        }
        let Some(s) = s.get(s.len() - 2..) else {
            return Self::default();
        };
        s.try_into().unwrap_or_default()
    }
}

impl<'a> From<&'a Path> for Language {
    /// Converts a file path to a `Language` by extracting the language code.
    ///
    /// This implementation examines the file stem (filename without extension)
    /// to determine the language. It expects files to follow the naming convention
    /// "filename.XX.extension" where XX is a two-character [ISO 639-1](https://en.wikipedia.org/wiki/ISO_639)
    /// language code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::path::Path;
    /// # use ftml_uris::prelude::*;
    ///
    /// let path = Path::new("/home/me/Documents/document.fr.tex");
    /// assert_eq!(Language::from(path), Language::French);
    ///
    /// let path = Path::new("page.de.html");
    /// assert_eq!(Language::from(path), Language::German);
    ///
    /// let path = Path::new("/usr/bin/file.txt");
    /// // No language code => returns English as default
    /// assert_eq!(Language::from(path), Language::English);
    /// ```
    fn from(value: &'a Path) -> Self {
        value
            .file_stem()
            .and_then(|s| s.to_str())
            .map_or_else(Self::default, Self::check)
    }
}

crate::tests! {
    language_parsing {
        use std::path::Path;

        // Test from file paths
        assert_eq!(Language::from(Path::new("file.en.tex")), Language::English);
        assert_eq!(Language::from(Path::new("file.de.html")), Language::German);
        assert_eq!(Language::from(Path::new("file.fr.tex")), Language::French);

        // Test edge cases
        assert_eq!(Language::from(Path::new("file.xx.tex")), Language::English); // Unknown -> default
        assert_eq!(Language::from(Path::new("file.tex")), Language::English); // No language -> default
        assert_eq!(Language::from(Path::new("file")), Language::English); // No extension -> default
        assert_eq!(Language::from(Path::new("")), Language::English); // Empty -> default
    }
}
