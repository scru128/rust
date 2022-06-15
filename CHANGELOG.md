# Changelog

## v2.1.3 - 2022-06-15

### Added

- `const` qualifier to `Scru128Generator::with_rng()` to allow const
  initialization with e.g. `rand::rngs::OsRng`

### Changed

- Revered change to `scru128()` and `scru128_string()` in v2.1.2 (process fork
  protection) on systems other than Unix

## v2.1.2 - 2022-06-11

### Fixed

- `scru128()` and `scru128_string()` to reset state when process ID changes
- `generate_core()` to update `counter_hi` when `timestamp` passed < 1000

### Maintenance

- Updated `once_cell` to 1.12

## v2.1.1 - 2022-05-23

### Fixed

- `generate_core()` to reject zero as `timestamp` value

## v2.1.0 - 2022-05-22

### Added

- `generate_core()` and `last_status()` to `Scru128Generator`
- Experimental `no_std` support

## v2.0.4 - 2022-05-11

### Changed

- Textual representation: 26-digit Base32 -> 25-digit Base36
- Field structure: { `timestamp`: 44 bits, `counter`: 28 bits, `per_sec_random`:
  24 bits, `per_gen_random`: 32 bits } -> { `timestamp`: 48 bits, `counter_hi`:
  24 bits, `counter_lo`: 24 bits, `entropy`: 32 bits }
- Timestamp epoch: 2020-01-01 00:00:00.000 UTC -> 1970-01-01 00:00:00.000 UTC
- Counter overflow handling: stall generator -> increment timestamp
- Default RNG type: rand::rngs::StdRng -> newtype that wraps concrete RNG
- Rust edition: 2018 -> 2021

### Removed

- `log` feature as counter overflow is no longer likely to occur
- `TIMESTAMP_BIAS`
- `Scru128Id#counter()`, `Scru128Id#per_sec_random()`, `Scru128Id#per_gen_random()`
- `Scru128Id#as_u128()`

### Added

- `Scru128Id#counter_hi()`, `Scru128Id#counter_lo()`, `Scru128Id#entropy()`
- `Scru128Id#to_u128()`, `Scru128Id#to_bytes()`
- Compact binary serde format

## v1.0.0 - 2022-01-03

- Initial stable release
