# SCRU128: Sortable, Clock and Random number-based Unique identifier

[![Crates.io](https://img.shields.io/crates/v/scru128)](https://crates.io/crates/scru128)
[![License](https://img.shields.io/crates/l/scru128)](https://github.com/scru128/rust/blob/main/LICENSE)

SCRU128 ID is yet another attempt to supersede [UUID] for the users who need
decentralized, globally unique time-ordered identifiers. SCRU128 is inspired by
[ULID] and [KSUID] and has the following features:

- 128-bit unsigned integer type
- Sortable by generation time (as integer and as text)
- 25-digit case-insensitive textual representation (Base36)
- 48-bit millisecond Unix timestamp that ensures useful life until year 10889
- Up to 281 trillion time-ordered but unpredictable unique IDs per millisecond
- 80-bit three-layer randomness for global uniqueness

```rust
use scru128::{scru128, scru128_string};

// generate a new identifier object
let x = scru128();
println!("{}", x); // e.g. "036Z951MHJIKZIK2GSL81GR7L"
println!("{}", x.to_u128()); // as a 128-bit unsigned integer

// generate a textual representation directly
println!("{}", scru128_string()); // e.g. "036Z951MHZX67T63MQ9XE6Q0J"
```

See [SCRU128 Specification] for details.

[uuid]: https://en.wikipedia.org/wiki/Universally_unique_identifier
[ulid]: https://github.com/ulid/spec
[ksuid]: https://github.com/segmentio/ksuid
[scru128 specification]: https://github.com/scru128/spec

## Optional features

- `serde` - Enables serialization/deserialization via serde.

## License

Licensed under the Apache License, Version 2.0.

## See also

- [docs.rs/scru128](https://docs.rs/scru128)
