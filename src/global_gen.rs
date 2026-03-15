//! Default generator and entry point functions.

#![cfg(feature = "global_gen")]

use crate::{Generator, Id};

/// Generates a new SCRU128 ID object using the global generator.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. On Unix, this function resets the generator
/// state when the process ID changes (i.e., upon forks) to avoid collisions across processes.
pub fn new() -> Id {
    use std::sync::{LazyLock, Mutex};
    static G: LazyLock<Mutex<GlobalGenInner>> = LazyLock::new(|| {
        Mutex::new(GlobalGenInner {
            #[cfg(unix)]
            pid: std::process::id(),
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
/// state when the process ID changes (i.e., upon forks) to avoid collisions across processes.
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

/// A thin wrapper to reset the state when the process ID changes (i.e., upon Unix forks).
#[derive(Debug)]
struct GlobalGenInner {
    #[cfg(unix)]
    pid: u32,
    generator: Generator<GlobalGenRng>,
}

impl GlobalGenInner {
    /// Returns a mutable reference to the inner [`Generator`] instance, reseting the generator
    /// state on Unix if the process ID has changed.
    fn get_mut(&mut self) -> &mut Generator<GlobalGenRng> {
        #[cfg(unix)]
        if self.pid != std::process::id() {
            self.pid = std::process::id();
            self.generator.reset_state();
            if let Ok(rng) = GlobalGenRng::try_new() {
                *self.generator.rand_source_mut() = rng;
            }
        }
        &mut self.generator
    }
}

mod global_gen_rng {
    use rand::{Rng as _, SeedableRng as _, rngs::StdRng, rngs::SysRng};

    use crate::generator::RandSource;

    /// The new type for the random number generator of the global generator.
    ///
    /// The global generator currently employs [`StdRng`] and reseeds it after every 64KiB
    /// consumption, emulating the strategy used by `ThreadRng`.
    #[derive(Debug)]
    pub struct GlobalGenRng {
        counter: usize,
        inner: StdRng,
    }

    const RESEED_THRESHOLD: usize = 64 * 1024;

    impl RandSource for GlobalGenRng {
        fn next_u32(&mut self) -> u32 {
            if self.counter >= RESEED_THRESHOLD {
                self.try_to_reseed();
            }
            self.counter += 32 / 8;
            self.inner.next_u32()
        }
    }

    impl GlobalGenRng {
        pub fn try_new() -> Result<Self, impl std::error::Error> {
            StdRng::try_from_rng(&mut SysRng).map(|inner| Self { counter: 0, inner })
        }

        #[cold]
        fn try_to_reseed(&mut self) {
            if let Ok(rng) = Self::try_new() {
                *self = rng;
            }
        }
    }

    #[cfg(test)]
    #[test]
    fn reseeded_after_64kib() {
        let seed = rand::TryRng::try_next_u64(&mut SysRng).unwrap();
        let mut g1 = StdRng::seed_from_u64(seed);
        let mut g2 = GlobalGenRng {
            counter: 0,
            inner: StdRng::seed_from_u64(seed),
        };

        for _ in 0..(64 * 1024 / (32 / 8)) {
            assert_eq!(g1.next_u32(), g2.next_u32());
        }

        assert_ne!(g1.next_u32(), g2.next_u32());
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
