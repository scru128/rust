//! # SCRU128: Sortable, Clock and Random number-based Unique identifier
//!
//! SCRU128 ID is yet another attempt to supersede [UUID] in the use cases that need
//! decentralized, globally unique time-ordered identifiers. SCRU128 is inspired by
//! [ULID] and [KSUID] and has the following features:
//!
//! - 128-bit unsigned integer type
//! - Sortable by generation time (as integer and as text)
//! - 26-digit case-insensitive portable textual representation
//! - 44-bit biased millisecond timestamp that ensures remaining life of 550 years
//! - Up to 268 million time-ordered but unpredictable unique IDs per millisecond
//! - 84-bit _layered_ randomness for collision resistance
//!
//! ```rust
//! use scru128::{scru128, scru128_string};
//!
//! // generate a new identifier object
//! let x = scru128();
//! println!("{}", x); // e.g. "00S6GVKR1MH58KE72EJD87SDOO"
//! println!("{}", x.as_u128()); // as a 128-bit unsigned integer
//!
//! // generate a textual representation directly
//! println!("{}", scru128_string()); // e.g. "00S6GVKR3F7R79I72EJF0J4RGC"
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
pub use generator::{Scru128Generator, TIMESTAMP_BIAS};
pub use identifier::{ParseError, Scru128Id};

#[cfg(test)]
mod tests {
    use crate::{Scru128Generator, Scru128Id};

    thread_local!(static SAMPLES: Vec<String> = {
        let mut g = Scru128Generator::new();
        (0..100_000).map(|_| g.generate().into()).collect()
    });

    /// Generates 26-digit canonical string
    #[test]
    fn it_generates_26_digit_canonical_string() {
        use regex::Regex;
        let re = Regex::new(r"^[0-7][0-9A-V]{25}$").unwrap();
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
                .as_millis()
                - 1577836800000) as i64;
            let timestamp = g.generate().timestamp() as i64;
            assert!((ts_now - timestamp).abs() < 16);
        }
    }

    /// Encodes unique sortable pair of timestamp and counter
    #[test]
    fn it_encodes_unique_sortable_pair_of_timestamp_and_counter() {
        SAMPLES.with(|samples| {
            let mut prev = samples[0].parse::<Scru128Id>().unwrap();
            for i in 1..samples.len() {
                let curr = samples[i].parse::<Scru128Id>().unwrap();
                assert!(
                    prev.timestamp() < curr.timestamp()
                        || (prev.timestamp() == curr.timestamp()
                            && prev.counter() < curr.counter())
                );
                prev = curr;
            }
        });
    }
}
