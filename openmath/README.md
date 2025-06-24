A comprehensive Rust library for handling **<span style="font-variant:small-caps;">OpenMath</span>** objects, providing robust (de)serialization
capabilities for various OpenMath formats including specification-compliant XML, JSON, and arbitrary
formats via [serde](https://docs.rs/serde).

## What is <span style="font-variant:small-caps;">OpenMath</span>?

OpenMath is a standard for representing mathematical expressions in a machine-readable, uniform way as abstract syntax tree with binding.


### Serialization & Deserialization
- [`OMSerializable`] trait for converting Rust types to <span style="font-variant:small-caps;">OpenMath</span>
- [`OMDeserializable`] trait for parsing <span style="font-variant:small-caps;">OpenMath</span> into Rust types
- Built-in support for serde-based formats (JSON, XML, etc.)

## Quick Start

```rust
use openmath::{OMSerializable, Int};

// TODO
```

## Features

- **Zero-copy deserialization** where possible
- **Arbitrary precision integers** with automatic small/big integer optimization
- **Flexible serialization** to multiple formats

## TODO

- structure sharing via OMR
- attributions via OMATTR
- omforeign, omerror

[1]: https://openmath.org/standard/om20-2019-07-01/omstd20.html
[2]: https://openmath.org/cd/
