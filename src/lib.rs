//! SCRU128: Sortable, Clock and Random number-based Unique identifier

use std::sync::Mutex;

use once_cell::sync::Lazy;

mod generator;
mod identifier;
use generator::Generator;

static DEFAULT_GENERATOR: Lazy<Mutex<Generator>> = Lazy::new(|| Mutex::new(Generator::new()));

/// Generates a new SCRU128 ID encoded in a string.
///
/// # Returns
///
/// 26-digit canonical string representation.
pub fn scru128() -> String {
    DEFAULT_GENERATOR.lock().unwrap().generate().to_string()
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;

    use crate::identifier::Identifier;
    use crate::scru128;

    static SAMPLES: Lazy<Vec<String>> = Lazy::new(|| (0..100_000).map(|_| scru128()).collect());

    /// Generates 26-digit canonical string
    #[test]
    fn it_generates_26_digit_canonical_string() {
        use regex::Regex;
        let re = Regex::new(r"^[0-7][0-9A-V]{25}$").unwrap();
        for e in SAMPLES.iter() {
            assert!(re.is_match(e));
        }
    }

    /// Generates 100k identifiers without collision
    #[test]
    fn it_generates_100k_identifiers_without_collision() {
        use std::collections::HashSet;
        let s: HashSet<String> = SAMPLES.iter().cloned().collect();
        assert_eq!(s.len(), SAMPLES.len());
    }

    /// Generates sortable string representation by creation time
    #[test]
    fn it_generates_sortable_string_representation_by_creation_time() {
        for i in 1..SAMPLES.len() {
            assert!(SAMPLES[i - 1] < SAMPLES[i]);
        }
    }

    /// Encodes up-to-date timestamp
    #[test]
    fn it_encodes_up_to_date_timestamp() {
        use std::time::{SystemTime, UNIX_EPOCH};
        for _ in 0..10_000 {
            let ts_now = (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock may have gone backwards")
                .as_millis()
                - 1577836800000) as i64;
            let timestamp = scru128().parse::<Identifier>().unwrap().timestamp() as i64;
            assert!((ts_now - timestamp).abs() < 16);
        }
    }

    /// Encodes unique sortable pair of timestamp and counter
    #[test]
    fn it_encodes_unique_sortable_pair_of_timestamp_and_counter() {
        let mut prev = SAMPLES[0].parse::<Identifier>().unwrap();

        for i in 1..SAMPLES.len() {
            let curr = SAMPLES[i].parse::<Identifier>().unwrap();
            assert!(
                prev.timestamp() < curr.timestamp()
                    || (prev.timestamp() == curr.timestamp() && prev.counter() < curr.counter())
            );
            prev = curr;
        }
    }

    /// Generates no IDs sharing same timestamp and counter under multithreading
    #[test]
    fn it_generates_no_ids_sharing_same_timestamp_and_counter_under_multithreading() {
        use std::collections::HashSet;
        use std::sync::mpsc::channel;
        use std::thread;

        let (tx, rx) = channel();
        for _ in 0..4 {
            let tx = tx.clone();
            thread::spawn(move || {
                for _ in 0..10000 {
                    tx.send(scru128()).unwrap();
                }
            });
        }
        drop(tx);

        let mut s = HashSet::new();
        while let Ok(msg) = rx.recv() {
            let e: Identifier = msg.parse().unwrap();
            s.insert((e.timestamp(), e.counter()));
        }

        assert_eq!(s.len(), 4 * 10000);
    }
}
