#![cfg(feature = "std")]

use crate::{Scru128Generator, Scru128Id};
use once_cell::sync::Lazy;
use std::process;
use std::sync::Mutex;

static DEFAULT_GENERATOR: Lazy<Mutex<ProcessWideGenerator>> = Lazy::new(Default::default);

/// Thin wrapper to reset the state when the process ID changes (i.e. upon fork).
#[derive(Debug)]
struct ProcessWideGenerator {
    gen: Scru128Generator,
    pid: u32,
}

impl Default for ProcessWideGenerator {
    fn default() -> Self {
        Self {
            gen: Default::default(),
            pid: process::id(),
        }
    }
}

impl ProcessWideGenerator {
    fn generate(&mut self) -> Scru128Id {
        let pid = process::id();
        if pid != self.pid {
            self.gen = Default::default();
            self.pid = pid;
        }
        self.gen.generate()
    }
}

/// Generates a new SCRU128 ID object.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. This function resets the generator state when
/// the process ID changes.
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn scru128() -> Scru128Id {
    DEFAULT_GENERATOR
        .lock()
        .unwrap_or_else(|err| panic!("could not lock default generator: {}", err))
        .generate()
}

/// Generates a new SCRU128 ID encoded in the 25-digit canonical string representation.
///
/// Use this to quickly get a new SCRU128 ID as a string.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. This function resets the generator state when
/// the process ID changes.
///
/// # Examples
///
/// ```rust
/// use scru128::scru128_string;
/// let x = scru128_string(); // e.g. "036Z951MHJIKZIK2GSL81GR7L"
///
/// assert!(regex::Regex::new(r"^[0-9A-Z]{25}$").unwrap().is_match(&x));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn scru128_string() -> String {
    scru128().into()
}

#[cfg(test)]
mod tests {
    use super::scru128;

    /// Generates no IDs sharing same timestamp and counters under multithreading
    #[test]
    fn generates_no_ids_sharing_same_timestamp_and_counters_under_multithreading() {
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
        while let Ok(e) = rx.recv() {
            s.insert((e.timestamp(), e.counter_hi(), e.counter_lo()));
        }

        assert_eq!(s.len(), 4 * 10000);
    }
}
