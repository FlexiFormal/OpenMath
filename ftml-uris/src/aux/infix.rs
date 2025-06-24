//! Infix operators for [FTML URI](crate::Uri) types.
//!
//! This module provides convenient infix operators for combining and manipulating
//! [URI](crate::Uri) components using operator overloading. The operators follow these patterns:
//!
//! - `&` (BitAnd): Combines [`BaseUri`]s with [`ArchiveId`]s, doument names and document element names,
//! - `/` (Div): Combines paths and names, adds paths to [URI](crate::Uri)s
//! - `|` (BitOr): Adds module names and symbol name to [URI](crate::Uri)s
//! - `!` (Not): Extracts top-level module name
//!
//! # Examples
//!
//! ```
//! # use ftml_uris::prelude::*;
//! # use std::str::FromStr;
//! let base = BaseUri::from_str("http://example.com").unwrap();
//! let archive_id = ArchiveId::from_str("my/archive").unwrap();
//! let path = UriPath::from_str("folder/file").unwrap();
//! let name = UriName::from_str("module").unwrap();
//!
//! // Combine base URI with archive ID
//! let archive_uri = base & archive_id;
//!
//! // Add path to archive URI
//! let path_uri = archive_uri / path;
//!
//! // Add module name to path URI
//! let module_uri = path_uri | name;
//! ```

use std::ops::{BitAnd, BitOr, Div, Not};

use crate::{
    ArchiveId, BaseUri, DocumentElementUri, DocumentUri, Language, ModuleUri, PathUri,
    SimpleUriName, SymbolUri, UriName, UriPath, archive::ArchiveUri,
    aux::interning::NonEmptyInternedStr,
};

/// Combines a [`BaseUri`] with an [`ArchiveId`] to create an [`ArchiveUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let base = BaseUri::from_str("http://example.com").unwrap();
/// let archive_id = ArchiveId::from_str("my/archive").unwrap();
/// let archive_uri = base & archive_id;
/// assert_eq!(archive_uri.to_string(), "http://example.com?a=my/archive");
/// ```
impl BitAnd<ArchiveId> for BaseUri {
    type Output = ArchiveUri;
    #[inline]
    fn bitand(self, rhs: ArchiveId) -> Self::Output {
        ArchiveUri {
            base: self,
            id: rhs,
        }
    }
}

/// Concatenates two [`UriPath`]s with a forward slash.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let path1 = UriPath::from_str("folder").unwrap();
/// let path2 = UriPath::from_str("file").unwrap();
/// let combined = &path1 / &path2;
/// assert_eq!(combined.as_ref(), "folder/file");
/// ```
impl<'a> Div<&'a UriPath> for &'a UriPath {
    type Output = UriPath;
    fn div(self, rhs: &'a UriPath) -> Self::Output {
        // SAFETY: since both sides are UriPaths, no empty segments and no
        // illegal characters
        unsafe {
            UriPath(NonEmptyInternedStr::new_from_nonempty(
                format!("{self}/{rhs}").parse().unwrap_unchecked(),
            ))
        }
    }
}

/// Adds a [`UriPath`] to an [`ArchiveUri`] to create a [`PathUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let archive = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
/// let path = UriPath::from_str("folder/file").unwrap();
/// let path_uri = archive / path;
/// assert_eq!(path_uri.to_string(), "http://example.com?a=archive&p=folder/file");
/// ```
impl Div<UriPath> for ArchiveUri {
    type Output = PathUri;
    #[inline]
    fn div(self, rhs: UriPath) -> Self::Output {
        PathUri {
            archive: self,
            path: Some(rhs),
        }
    }
}
/// Adds an optional [`UriPath`] to an [`ArchiveUri`] to create a [`PathUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let archive = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
/// let path = Some(UriPath::from_str("folder").unwrap());
/// let path_uri = archive.clone() / path;
/// assert_eq!(path_uri.path().unwrap().as_ref(), "folder");
///
/// let path_uri_none = archive / None;
/// assert!(path_uri_none.path().is_none());
/// ```
impl Div<Option<UriPath>> for ArchiveUri {
    type Output = PathUri;
    #[inline]
    fn div(self, rhs: Option<UriPath>) -> Self::Output {
        PathUri {
            archive: self,
            path: rhs,
        }
    }
}

/// Adds a [`UriPath`] segment to a [`PathUri`].
///
/// If the [`PathUri`] has no existing path, the new path becomes the path.
/// If it has an existing path, the new path is appended.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let archive = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
/// let path_uri: PathUri = archive.into();
/// let new_path = UriPath::from_str("folder").unwrap();
/// let result = path_uri / new_path;
/// assert_eq!(result.path().unwrap().as_ref(), "folder");
/// ```
impl Div<UriPath> for PathUri {
    type Output = Self;
    #[inline]
    fn div(self, rhs: UriPath) -> Self::Output {
        Self {
            archive: self.archive,
            path: match self.path {
                None => Some(rhs),
                Some(p) => Some(&p / &rhs),
            },
        }
    }
}
/// Adds a [`UriPath`] segment (by reference) to a [`PathUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let mut path_uri = PathUri::from_str("http://example.com?a=archive&p=base").unwrap();
/// let additional = UriPath::from_str("extra").unwrap();
/// path_uri = path_uri / &additional;
/// assert_eq!(path_uri.path().unwrap().as_ref(), "base/extra");
/// ```
impl Div<&UriPath> for PathUri {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &UriPath) -> Self::Output {
        Self {
            archive: self.archive,
            path: Some(self.path.map_or_else(|| rhs.clone(), |p| &p / rhs)),
        }
    }
}

/// Concatenates two [`UriName`]s with a forward slash.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let name1 = UriName::from_str("math").unwrap();
/// let name2 = UriName::from_str("algebra").unwrap();
/// let combined = &name1 / &name2;
/// assert_eq!(combined.as_ref(), "math/algebra");
/// ```
impl<'a> Div<&'a UriName> for &'a UriName {
    type Output = UriName;
    fn div(self, rhs: &'a UriName) -> Self::Output {
        // SAFETY: since both sides are UriNames, no empty segments and no
        // illegal characters
        unsafe {
            UriName(NonEmptyInternedStr::new_from_nonempty(
                format!("{self}/{rhs}").parse().unwrap_unchecked(),
            ))
        }
    }
}

/// Extracts the top-level module from a [`ModuleUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let module_uri = ModuleUri::from_str("http://example.com?a=archive&m=math/algebra/groups").unwrap();
/// let top_level = !module_uri; // = http://example.com?a=archive&m=math
/// assert_eq!(top_level.name.as_ref(), "math");
/// ```
impl Not for ModuleUri {
    type Output = Self;
    #[inline]
    fn not(self) -> Self::Output {
        Self {
            path: self.path,
            name: self.name.top(),
        }
    }
}

/// Adds a module [`UriName`] to an [`ArchiveUri`] to create a [`ModuleUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let archive = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
/// let name = UriName::from_str("math/algebra").unwrap();
/// let module_uri = archive | name;
/// assert_eq!(module_uri.to_string(), "http://example.com?a=archive&m=math/algebra");
/// ```
impl BitOr<UriName> for ArchiveUri {
    type Output = ModuleUri;
    #[inline]
    fn bitor(self, rhs: UriName) -> Self::Output {
        ModuleUri {
            path: self.into(),
            name: rhs,
        }
    }
}
/// Adds a module [`UriName`] to a [`PathUri`] to create a [`ModuleUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder").unwrap();
/// let name = UriName::from_str("module").unwrap();
/// let module_uri = path_uri | name;
/// assert_eq!(module_uri.to_string(), "http://example.com?a=archive&p=folder&m=module");
/// ```
impl BitOr<UriName> for PathUri {
    type Output = ModuleUri;
    #[inline]
    fn bitor(self, rhs: UriName) -> Self::Output {
        ModuleUri {
            path: self,
            name: rhs,
        }
    }
}

/// Extends a [`ModuleUri`]'s name with an additional [`UriName`] segment.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let module_uri = ModuleUri::from_str("http://example.com?a=archive&m=math").unwrap();
/// let additional = UriName::from_str("algebra").unwrap();
/// let extended = module_uri / &additional;
/// assert_eq!(extended.name.as_ref(), "math/algebra");
/// ```
impl Div<&UriName> for ModuleUri {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &UriName) -> Self::Output {
        Self {
            path: self.path,
            name: &self.name / rhs,
        }
    }
}

/// Adds a symbol [`UriName`] to a [`ModuleUri`] to create a [`SymbolUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let module_uri = ModuleUri::from_str("http://example.com?a=archive&p=folder&m=module").unwrap();
/// let name = UriName::from_str("symbol").unwrap();
/// let symbol_uri = module_uri | name;
/// assert_eq!(symbol_uri.to_string(), "http://example.com?a=archive&p=folder&m=module&s=symbol");
/// ```
impl BitOr<UriName> for ModuleUri {
    type Output = SymbolUri;
    #[inline]
    fn bitor(self, rhs: UriName) -> Self::Output {
        SymbolUri {
            module: self,
            name: rhs,
        }
    }
}

/// Extends a [`SymbolUri`]'s name with an additional [`UriName`] segment.
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let symbol_uri = SymbolUri::from_str("http://example.com?a=archive&m=math&s=algebra").unwrap();
/// let additional = UriName::from_str("group-theory").unwrap();
/// let extended = symbol_uri / &additional;
/// assert_eq!(extended.name.as_ref(), "algebra/group-theory");
/// ```
impl Div<&UriName> for SymbolUri {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &UriName) -> Self::Output {
        Self {
            module: self.module,
            name: &self.name / rhs,
        }
    }
}

/// Adds a document [`SimpleUriName`] to an [`ArchiveUri`] to create a [`DocumentUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let archive = ArchiveUri::from_str("http://example.com?a=archive").unwrap();
/// let name = SimpleUriName::from_str("math").unwrap();
/// let document_uri = archive & (name,Language::English);
/// assert_eq!(document_uri.to_string(), "http://example.com?a=archive&d=math&l=en");
/// ```
impl BitAnd<(SimpleUriName, Language)> for ArchiveUri {
    type Output = DocumentUri;
    #[inline]
    fn bitand(self, rhs: (SimpleUriName, Language)) -> Self::Output {
        DocumentUri {
            path: self.into(),
            name: rhs.0,
            language: rhs.1,
        }
    }
}
/// Adds a document [`SimpleUriName`] to a [`PathUri`] to create a [`DocumentUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder").unwrap();
/// let name = SimpleUriName::from_str("document").unwrap();
/// let document_uri = path_uri & (name,Language::German);
/// assert_eq!(document_uri.to_string(), "http://example.com?a=archive&p=folder&d=document&l=de");
/// ```
impl BitAnd<(SimpleUriName, Language)> for PathUri {
    type Output = DocumentUri;
    #[inline]
    fn bitand(self, rhs: (SimpleUriName, Language)) -> Self::Output {
        DocumentUri {
            path: self,
            name: rhs.0,
            language: rhs.1,
        }
    }
}

/// Adds an element name [`UriName`] to a [`DocumentUri`] to create a [`DocumentElementUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let document_uri = DocumentUri::from_str("http://example.com?a=archive&p=folder&d=foo&l=fr").unwrap();
/// let name = UriName::from_str("bar").unwrap();
/// let element_uri = document_uri & name;
/// assert_eq!(element_uri.to_string(), "http://example.com?a=archive&p=folder&d=foo&l=fr&e=bar");
/// ```
impl BitAnd<UriName> for DocumentUri {
    type Output = DocumentElementUri;
    #[inline]
    fn bitand(self, rhs: UriName) -> Self::Output {
        DocumentElementUri {
            document: self,
            name: rhs,
        }
    }
}

/// Adds a name step [`UriName`] to a [`DocumentElementUri`].
///
/// # Examples
///
/// ```
/// # use ftml_uris::prelude::*;
/// # use std::str::FromStr;
/// let first_uri = DocumentElementUri::from_str("http://example.com?a=archive&p=folder&d=foo&l=fr&e=bar").unwrap();
/// let name = UriName::from_str("baz/buz").unwrap();
/// let element_uri = first_uri / &name;
/// assert_eq!(element_uri.to_string(), "http://example.com?a=archive&p=folder&d=foo&l=fr&e=bar/baz/buz");
/// ```
impl Div<&UriName> for DocumentElementUri {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &UriName) -> Self::Output {
        Self {
            document: self.document,
            name: &self.name / rhs,
        }
    }
}

crate::tests! {
    infix_base {
        use std::str::FromStr;
        use crate::{IsFtmlUri, UriWithArchive};
        let base = BaseUri::from_str("http://example.com").expect("works");
        let archive_id = ArchiveId::from_str("my/archive").expect("works");

        // Test BaseUri & ArchiveId
        let archive_uri = base.clone() & archive_id;
        assert_eq!(archive_uri.base, base);
        assert_eq!(archive_uri.id.to_string(), "my/archive");

        let result = base & "another/archive".parse().expect("works");
        assert_eq!(result.id.to_string(), "another/archive");
    };
    paths {
        use std::str::FromStr;
        use crate::{UriWithArchive, UriWithPath};
        let path = UriPath::from_str("foo/bar/baz").expect("works");
        let path2 = UriPath::from_str("hurtz").expect("works");
        let p3: UriPath = &path / &path2;
        assert_eq!(p3.as_ref(),"foo/bar/baz/hurtz");

        let archive : ArchiveUri = "http://example.com?a=some/archive".parse().expect("works");
        let path_uri = archive.clone() / path.clone();
        let path_uri = archive.clone() / Some(path.clone());
        let path_uri = archive.clone() / None;
        assert_eq!(Into::<ArchiveUri>::into(path_uri),archive);
        let path_uri = archive / path.clone();
        let path_uri = path_uri / &path2;
        let path_uri = path_uri / path;
    };
    names {
        use std::str::FromStr;
        let name1 = UriName::from_str("math").expect("works");
        let name2 = UriName::from_str("algebra").expect("works");
        let name3 = UriName::from_str("groups").expect("works");

        // Test name concatenation
        let combined = &name1 / &name2;
        assert_eq!(combined.as_ref(), "math/algebra");

        let longer = &combined / &name3;
        assert_eq!(longer.as_ref(), "math/algebra/groups");
    };
    module_operations {
        use std::str::FromStr;
        use crate::UriWithPath;
        let archive = ArchiveUri::from_str("http://example.com?a=archive").expect("works");
        let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder").expect("works");
        let name = UriName::from_str("module").expect("works");

        // Test BitOr operations (adding module names)
        let module_uri1 = archive | name.clone();
        assert_eq!(module_uri1.name.as_ref(), "module");
        assert!(module_uri1.path().is_none());

        let module_uri2 = path_uri | name;
        assert_eq!(module_uri2.name.as_ref(), "module");
        assert_eq!(module_uri2.path().expect("works").as_ref(), "folder");

        // Test extending module names
        let additional = UriName::from_str("submodule").expect("works");
        let extended = module_uri1 / &additional;
        assert_eq!(extended.name.as_ref(), "module/submodule");
    };
    document_operations {
        use std::str::FromStr;
        use crate::{UriWithPath,IsNarrativeUri};
        let archive = ArchiveUri::from_str("http://example.com?a=archive").expect("works");
        let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder").expect("works");
        let name = SimpleUriName::from_str("doc").expect("works");

        // Test BitOr operations (adding module names)
        let document_uri1 = archive & (name.clone(),Language::French);
        assert_eq!(document_uri1.name.as_ref(), "doc");
        assert!(document_uri1.path().is_none());
        assert_eq!(document_uri1.language(),Language::French);

        let document_uri2 = path_uri & (name,Language::German);
        assert_eq!(document_uri2.name.as_ref(), "doc");
        assert_eq!(document_uri2.path().expect("works").as_ref(), "folder");
    };
    module_not_operation {
        use std::str::FromStr;
        let module_uri = ModuleUri::from_str("http://example.com?a=archive&m=math/algebra/groups").expect("works");

        // Test Not operation (extract top-level name)
        let top_level = !module_uri;
        assert_eq!(top_level.name.as_ref(), "math");
        assert!(top_level.name.is_simple());
    };
    complex_combinations {
        use std::str::FromStr;
        use crate::{IsFtmlUri, UriWithArchive, UriWithPath};

        // Test complex operator chaining
        let base = BaseUri::from_str("http://example.com").expect("works");
        let archive_id = ArchiveId::from_str("math/archive").expect("works");
        let path = UriPath::from_str("textbooks").expect("works");
        let module_name = UriName::from_str("algebra/groups").expect("works");

        // Create a complex URI using operators
        #[allow(clippy::precedence)]
        let module_uri = (base & archive_id) / path | module_name;

        assert_eq!(module_uri.base().as_str(), "http://example.com");
        assert_eq!(module_uri.archive_id().to_string(), "math/archive");
        assert_eq!(module_uri.path().expect("works").as_ref(), "textbooks");
        assert_eq!(module_uri.name.as_ref(), "algebra/groups");

        // Extract top-level module
        let top_level = !module_uri;
        assert_eq!(top_level.name.as_ref(), "algebra");
    };
    operator_precedence {
        use std::str::FromStr;
        use crate::UriWithPath;

        // Test that operators work correctly with Rust's precedence rules
        let base = BaseUri::from_str("http://example.com").expect("works");
        let archive_id = ArchiveId::from_str("archive").expect("works");
        let path1 = UriPath::from_str("folder").expect("works");
        let path2 = UriPath::from_str("subfolder").expect("works");
        let name = UriName::from_str("module").expect("works");

        // Test that / has higher precedence than |
        let archive_uri = base & archive_id;
        let path_uri = archive_uri / path1 / &path2;
        let result = path_uri | name;

        assert_eq!(result.path().expect("works").as_ref(), "folder/subfolder");
        assert_eq!(result.name.as_ref(), "module");
    };
    edge_cases {
        use std::str::FromStr;
        use crate::UriWithPath;

        // Test with empty path cases
        let archive = ArchiveUri::from_str("http://example.com?a=archive").expect("works");
        let path_uri: PathUri = archive.clone().into();
        let new_path = UriPath::from_str("newpath").expect("works");

        // Adding path to empty path URI
        let result = path_uri / new_path;
        assert_eq!(result.path().expect("works").as_ref(), "newpath");

        // Test with None option
        let empty_path_uri = archive / None;
        assert!(empty_path_uri.path().is_none());
    }
}
