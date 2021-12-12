# SCRU128: Sortable, Clock and Random number-based Unique identifier

[![Crates.io](https://img.shields.io/crates/v/scru128)](https://crates.io/crates/scru128)
[![License](https://img.shields.io/crates/l/scru128)](https://github.com/scru128/rust/blob/main/LICENSE)

SCRU128 ID is yet another attempt to supersede [UUID] in the use cases that need
decentralized, globally unique time-ordered identifiers. SCRU128 is inspired by
[ULID] and [KSUID] and has the following features:

- 128-bit unsigned integer type
- Sortable by generation time (as integer and as text)
- 26-digit case-insensitive portable textual representation
- 44-bit biased millisecond timestamp that ensures remaining life of 550 years
- Up to 268 million time-ordered but unpredictable unique IDs per millisecond
- 84-bit _layered_ randomness for collision resistance

```rust
use scru128::{scru128, scru128_string};

// generate a new identifier object
let x = scru128();
println!("{}", x); // e.g. "00S6GVKR1MH58KE72EJD87SDOO"
println!("{}", x.as_u128()); // as a 128-bit unsigned integer

// generate a textual representation directly
println!("{}", scru128_string()); // e.g. "00S6GVKR3F7R79I72EJF0J4RGC"
```

See [SCRU128 Specification] for details.

[uuid]: https://en.wikipedia.org/wiki/Universally_unique_identifier
[ulid]: https://github.com/ulid/spec
[ksuid]: https://github.com/segmentio/ksuid
[scru128 specification]: https://github.com/scru128/spec

## Optional features

- `log` - Enables logging via the [log](https://crates.io/crates/log) crate.
- `serde` - Enables serialization/deserialization via serde.

## License

Licensed under the Apache License, Version 2.0.

## See also

- [docs.rs/scru128](https://docs.rs/scru128)
