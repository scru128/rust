//! # SCRU128: Sortable, Clock and Random number-based Unique identifier
//!
//! SCRU128 ID is yet another attempt to supersede [UUID] for the users who need
//! decentralized, globally unique time-ordered identifiers. SCRU128 is inspired by
//! [ULID] and [KSUID] and has the following features:
//!
//! - 128-bit unsigned integer type
//! - Sortable by generation time (as integer and as text)
//! - 25-digit case-insensitive textual representation (Base36)
//! - 48-bit millisecond Unix timestamp that ensures useful life until year 10889
//! - Up to 281 trillion time-ordered but unpredictable unique IDs per millisecond
//! - 80-bit three-layer randomness for global uniqueness
//!
//! ```rust
//! # #[cfg(feature = "global_gen")]
//! # {
//! // generate a new identifier object
//! let x = scru128::new();
//! println!("{}", x); // e.g., "036z951mhjikzik2gsl81gr7l"
//! println!("{}", x.to_u128()); // as a 128-bit unsigned integer
//!
//! // generate a textual representation directly
//! println!("{}", scru128::new_string()); // e.g., "036z951mhzx67t63mq9xe6q0j"
//! # }
//! ```
//!
//! See [SCRU128 Specification] for details.
//!
//! [UUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier
//! [ULID]: https://github.com/ulid/spec
//! [KSUID]: https://github.com/segmentio/ksuid
//! [SCRU128 Specification]: https://github.com/scru128/spec
//!
//! ## Crate features
//!
//! Default features:
//!
//! - `std` configures [`Scru128Generator`] with the system clock. Without `std`, this
//!   crate provides basic SCRU128 primitives available under `no_std` environments.
//! - `rand` enables a blanket implementation for [`rand::RngCore`] to use `rand` and
//!   any other conforming random number generators with [`Scru128Generator`].
//! - `default_rng` (implies `std` and `rand`) provides the default random number
//!   generator for [`Scru128Generator`] and enables the [`Scru128Generator::new()`]
//!   constructor.
//! - `global_gen` (implies `default_rng`) provides the process-wide default SCRU128
//!   generator and enables the [`new()`] and [`new_string()`] functions.
//!
//! Optional features:
//!
//! - `serde` enables serialization/deserialization of [`Scru128Id`] via serde.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod global_gen;
#[cfg(feature = "global_gen")]
pub use global_gen::{new, new_string};

mod id;
pub use id::{ParseError, Scru128Id};

pub mod gen;
#[doc(hidden)]
pub use gen as generator;
pub use gen::Scru128Generator;

/// The maximum value of 48-bit `timestamp` field.
const MAX_TIMESTAMP: u64 = 0xffff_ffff_ffff;

/// The maximum value of 24-bit `counter_hi` field.
const MAX_COUNTER_HI: u32 = 0xff_ffff;

/// The maximum value of 24-bit `counter_lo` field.
const MAX_COUNTER_LO: u32 = 0xff_ffff;

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use crate::{Scru128Generator, Scru128Id};

    fn samples() -> &'static [String] {
        use std::sync::OnceLock;
        static SAMPLES: OnceLock<Vec<String>> = OnceLock::new();
        SAMPLES.get_or_init(|| {
            Scru128Generator::new()
                .map(String::from)
                .take(100_000)
                .collect()
        })
    }

    /// Generates 25-digit canonical string
    #[test]
    fn generates_25_digit_canonical_string() {
        let re = regex::Regex::new(r"^[0-9a-z]{25}$").unwrap();
        for e in samples() {
            assert!(re.is_match(e));
        }
    }

    /// Generates 100k identifiers without collision
    #[test]
    fn generates_100k_identifiers_without_collision() {
        use std::collections::HashSet;
        let s: HashSet<&String> = samples().iter().collect();
        assert_eq!(s.len(), samples().len());
    }

    /// Generates sortable string representation by creation time
    #[test]
    fn generates_sortable_string_representation_by_creation_time() {
        let samples = samples();
        for i in 1..samples.len() {
            assert!(samples[i - 1] < samples[i]);
        }
    }

    /// Encodes up-to-date timestamp
    #[test]
    fn encodes_up_to_date_timestamp() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut g = Scru128Generator::new();
        for _ in 0..10_000 {
            let ts_now = (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock may have gone backwards")
                .as_millis()) as i64;
            let timestamp = g.generate().timestamp() as i64;
            assert!((ts_now - timestamp).abs() < 16);
        }
    }

    /// Encodes unique sortable tuple of timestamp and counters
    #[test]
    fn encodes_unique_sortable_tuple_of_timestamp_and_counters() {
        let samples = samples();
        let mut prev = samples[0].parse::<Scru128Id>().unwrap();
        for e in &samples[1..] {
            let curr = e.parse::<Scru128Id>().unwrap();
            assert!(
                prev.timestamp() < curr.timestamp()
                    || (prev.timestamp() == curr.timestamp()
                        && prev.counter_hi() < curr.counter_hi())
                    || (prev.timestamp() == curr.timestamp()
                        && prev.counter_hi() == curr.counter_hi()
                        && prev.counter_lo() < curr.counter_lo())
            );
            prev = curr;
        }
    }
}
