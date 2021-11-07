use crate::Scru128Generator;

use std::sync::Mutex;

use once_cell::sync::Lazy;

static DEFAULT_GENERATOR: Lazy<Mutex<Scru128Generator>> = Lazy::new(|| {
    #[cfg(feature = "log")]
    log::debug!("initialized global generator");
    Mutex::new(Scru128Generator::new())
});

/// Generates a new SCRU128 ID encoded in the 26-digit canonical string representation.
///
/// Use this function to quickly get a new SCRU128 ID as a string. Use [Scru128Generator] to do
/// more.
///
/// This function is thread safe in that it generates monotonically ordered IDs using a shared
/// state when called concurrently from multiple threads.
///
/// # Examples
///
/// ```rust
/// use scru128::scru128;
/// let x = scru128(); // e.g. "00Q1BPRUE21T9VN8I9JR18TO9T"
///
/// assert!(regex::Regex::new(r"^[0-7][0-9A-V]{25}$").unwrap().is_match(&x));
/// ```
pub fn scru128() -> String {
    DEFAULT_GENERATOR.lock().unwrap().generate().into()
}

#[cfg(test)]
mod tests {
    use super::scru128;
    use crate::Scru128Id;

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
            let e: Scru128Id = msg.parse().unwrap();
            s.insert((e.timestamp(), e.counter()));
        }

        assert_eq!(s.len(), 4 * 10000);
    }
}
