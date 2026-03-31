//! Default generator and entry point functions.

#![cfg(feature = "global_gen")]

use crate::{Generator, Id};

/// Generates a new SCRU128 ID object using the global generator.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. On Unix, this function resets the generator
/// state when a process fork is detected to avoid collisions across processes.
pub fn new() -> Id {
    use std::sync::{LazyLock, Mutex};
    static G: LazyLock<Mutex<GlobalGenInner>> = LazyLock::new(|| {
        Mutex::new(GlobalGenInner {
            guard: forkguard::new(),
            generator: Generator::with_rand_and_time_sources(
                GlobalGenRng::try_new().expect("scru128: could not initialize global generator"),
                Default::default(),
            ),
        })
    });
    G.lock()
        .expect("scru128: could not lock global generator")
        .get_mut()
        .generate()
}

/// Generates a new SCRU128 ID encoded in the 25-digit canonical string representation using the
/// global generator.
///
/// Use this to quickly get a new SCRU128 ID as a string.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. On Unix, this function resets the generator
/// state when a process fork is detected to avoid collisions across processes.
///
/// # Examples
///
/// ```rust
/// let x = scru128::new_string(); // e.g., "036z951mhjikzik2gsl81gr7l"
///
/// assert!(regex::Regex::new(r"^[0-9a-z]{25}$").unwrap().is_match(&x));
/// ```
pub fn new_string() -> String {
    new().into()
}

use global_gen_rng::GlobalGenRng;

/// A thin wrapper to reset the state when a process fork is detected.
#[derive(Debug)]
struct GlobalGenInner {
    guard: forkguard::Guard,
    generator: Generator<GlobalGenRng>,
}

impl GlobalGenInner {
    /// Returns a mutable reference to the inner [`Generator`] instance, reseting the generator
    /// state on Unix if a process fork is detected.
    fn get_mut(&mut self) -> &mut Generator<GlobalGenRng> {
        if self.guard.detected_fork() {
            self.generator.reset_state();
            let _ = self.generator.rand_source_mut().try_reseed();
        }
        &mut self.generator
    }
}

mod global_gen_rng {
    use rand::{Rng as _, rngs::StdRng, rngs::SysRng};
    use reseeding_rng::ReseedingRng;

    use crate::generator::RandSource;

    #[derive(Debug)]
    pub struct GlobalGenRng(ReseedingRng<StdRng, SysRng>);

    impl RandSource for GlobalGenRng {
        fn next_u32(&mut self) -> u32 {
            self.0.next_u32()
        }
    }

    impl GlobalGenRng {
        pub fn try_new() -> Result<Self, impl std::error::Error> {
            ReseedingRng::try_new(1024 * 64, SysRng).map(Self)
        }

        #[cold]
        pub fn try_reseed(&mut self) -> Result<(), impl std::error::Error> {
            self.0.try_reseed()
        }
    }
}

#[cfg(test)]
mod tests {
    /// Generates no IDs sharing same timestamp and counters under multithreading
    #[test]
    fn generates_no_ids_sharing_same_timestamp_and_counters_under_multithreading()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::{collections::HashSet, sync::mpsc, thread};

        let (tx, rx) = mpsc::channel();
        for _ in 0..4 {
            let tx = tx.clone();
            thread::Builder::new()
                .spawn(move || {
                    for _ in 0..10_000 {
                        tx.send(super::new()).unwrap();
                    }
                })
                .map_err(|err| format!("failed to spawn thread: {:?}", err))?;
        }
        drop(tx);

        let mut s = HashSet::new();
        while let Ok(e) = rx.recv() {
            s.insert((e.timestamp(), e.counter_hi(), e.counter_lo()));
        }

        assert_eq!(s.len(), 4 * 10_000);
        Ok(())
    }
}
