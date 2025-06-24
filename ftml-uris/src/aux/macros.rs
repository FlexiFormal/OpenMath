macro_rules! ts {
    ($name:ident$(<$($a:ident$(:$guard:ident)?),+>)? = $c:ident) => {
        #[cfg(feature = "typescript")]
        #[::wasm_bindgen::prelude::wasm_bindgen(typescript_custom_section)]
        const $c: &str = ::const_format::concatcp!("export type ", stringify!($name), " = string;");
        #[cfg(feature = "typescript")]
        impl$(<$($a$(:$guard)?),+>)? ::wasm_bindgen::convert::FromWasmAbi for $name$(<$($a),+>)? {
            type Abi = <String as ::wasm_bindgen::convert::FromWasmAbi>::Abi;
            unsafe fn from_abi(js: Self::Abi) -> Self {
                unsafe {
                    use ::wasm_bindgen::UnwrapThrowExt;
                    let s = <String as ::wasm_bindgen::convert::FromWasmAbi>::from_abi(js);
                    s.parse().unwrap_throw()
                }
            }
        }
        #[cfg(feature = "typescript")]
        impl$(<$($a$(:$guard)?),+>)? ::wasm_bindgen::convert::IntoWasmAbi for $name$(<$($a),+>)? {
            type Abi = <String as ::wasm_bindgen::convert::IntoWasmAbi>::Abi;
            fn into_abi(self) -> Self::Abi {
                <String as ::wasm_bindgen::convert::IntoWasmAbi>::into_abi(self.to_string())
            }
        }
        #[cfg(feature = "typescript")]
        impl<'ts$(,$($a$(:$guard)?),+)?> ::wasm_bindgen::convert::IntoWasmAbi for &'ts $name$(<$($a),+>)? {
            type Abi = <String as ::wasm_bindgen::convert::IntoWasmAbi>::Abi;
            fn into_abi(self) -> Self::Abi {
                <String as ::wasm_bindgen::convert::IntoWasmAbi>::into_abi(self.to_string())
            }
        }

        #[cfg(feature = "typescript")]
        impl$(<$($a$(:$guard)?),+>)? ::wasm_bindgen::describe::WasmDescribe for $name$(<$($a),+>)? {
            fn describe() {
                <String as ::wasm_bindgen::describe::WasmDescribe>::describe();
            }
        }
    };
    ($name:ident$(<$($a:ident$(:$guard:ident)?),+>)?) => {
        $crate::ts!($name$(<$($a$(:$guard)?),+>)? = TS);
    };
}
macro_rules! tests {
    ($($(#[$meta:meta])* $name:ident $(($argn:ident:$argtp:ty))? $code:block);*) => {
        #[cfg(test)]
        mod tests {
            #![allow(unused_imports)]
            use super::*;
            use crate::trace;
            use rstest::{rstest,fixture};
            $(
                #[rstest]
                #[allow(unused_variables)]
                $(#[$meta])*
                fn $name(trace:()$($(,$argn:$argtp)*)?) $code
            )*

        }
    };
}

macro_rules! debugdisplay {
    ($s:ident$(<$($a:ident$(:$guard:ident)?),+>)?) => {
        impl$(<$($a$(:$guard)?),+>)? std::fmt::Debug for $s$(<$($a),+>)? {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(self, f)
            }
        }
    };
}
pub(crate) use {debugdisplay, tests, ts};
