A comprehensive Rust library for handling [**<span style="font-variant:small-caps;">OpenMath</span>**](http://openmath.org/) objects, providing
- An [`OpenMath`] data structure and
- *almost* zero-copy (de)serialization for various <span style="font-variant:small-caps;">OpenMath</span> formats, including specification-compliant XML, JSON, and arbitrary other formats via [serde](https://docs.rs/serde).


### Serialization & Deserialization
- [`OMSerializable`] trait for converting Rust types to <span style="font-variant:small-caps;">OpenMath</span>
- [`OMDeserializable`] trait for parsing <span style="font-variant:small-caps;">OpenMath</span> into Rust types
- (with `serde` feature enabled:) support for serde-based formats (JSON, etc.) following the
  OpenMath JSON specification (see [`openmath_serde`](OMSerializable::openmath_serde) and [`OMFromSerde`](de::OMFromSerde)).
- specification-conform XML (de)serialization

## TODO

- structure sharing via OMR
- binary format
- official errors

[1]: https://openmath.org/standard/om20-2019-07-01/omstd20.html
[2]: https://openmath.org/cd/
