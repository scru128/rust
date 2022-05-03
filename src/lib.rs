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
//! use scru128::{scru128, scru128_string};
//!
//! // generate a new identifier object
//! let x = scru128();
//! println!("{}", x); // e.g. "036Z951MHJIKZIK2GSL81GR7L"
//! println!("{}", x.as_u128()); // as a 128-bit unsigned integer
//!
//! // generate a textual representation directly
//! println!("{}", scru128_string()); // e.g. "036Z951MHZX67T63MQ9XE6Q0J"
//! ```
//!
//! See [SCRU128 Specification] for details.
//!
//! [uuid]: https://en.wikipedia.org/wiki/Universally_unique_identifier
//! [ulid]: https://github.com/ulid/spec
//! [ksuid]: https://github.com/segmentio/ksuid
//! [scru128 specification]: https://github.com/scru128/spec

mod default_gen;
mod generator;
mod identifier;
pub use default_gen::{scru128, scru128_string};
pub use generator::{default_rng::DefaultRng, Scru128Generator};
pub use identifier::{ParseError, Scru128Id};

#[cfg(test)]
mod tests {
    use crate::{Scru128Generator, Scru128Id};

    thread_local!(static SAMPLES: Vec<String> = {
        let mut g = Scru128Generator::new();
        (0..100_000).map(|_| g.generate().into()).collect()
    });

    /// Generates 25-digit canonical string
    #[test]
    fn it_generates_25_digit_canonical_string() {
        use regex::Regex;
        let re = Regex::new(r"^[0-9A-Z]{25}$").unwrap();
        SAMPLES.with(|samples| {
            for e in samples.iter() {
                assert!(re.is_match(e));
            }
        });
    }

    /// Generates 100k identifiers without collision
    #[test]
    fn it_generates_100k_identifiers_without_collision() {
        use std::collections::HashSet;
        SAMPLES.with(|samples| {
            let s: HashSet<String> = samples.iter().cloned().collect();
            assert_eq!(s.len(), samples.len());
        });
    }

    /// Generates sortable string representation by creation time
    #[test]
    fn it_generates_sortable_string_representation_by_creation_time() {
        SAMPLES.with(|samples| {
            for i in 1..samples.len() {
                assert!(samples[i - 1] < samples[i]);
            }
        });
    }

    /// Encodes up-to-date timestamp
    #[test]
    fn it_encodes_up_to_date_timestamp() {
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
    fn it_encodes_unique_sortable_tuple_of_timestamp_and_counters() {
        SAMPLES.with(|samples| {
            let mut prev = samples[0].parse::<Scru128Id>().unwrap();
            for i in 1..samples.len() {
                let curr = samples[i].parse::<Scru128Id>().unwrap();
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
        });
    }
}
