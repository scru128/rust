# Changelog

## v2.0.2 - unreleased

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

## v1.0.0 - 2022-01-03

- Initial stable release
