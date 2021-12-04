use crate::identifier::{Scru128Id, MAX_COUNTER, MAX_PER_SEC_RANDOM};

use std::time::{SystemTime, UNIX_EPOCH};

use rand::{rngs::StdRng, RngCore, SeedableRng};

/// Unix time in milliseconds at 2020-01-01 00:00:00+00:00.
pub const TIMESTAMP_BIAS: u64 = 1577836800000;

/// Represents a SCRU128 ID generator that encapsulates the monotonic counter and other internal
/// states.
///
/// # Examples
///
/// ```rust
/// use scru128::Scru128Generator;
///
/// let mut g = Scru128Generator::new();
/// println!("{}", g.generate());
/// println!("{}", g.generate().as_u128());
/// ```
///
/// Each generator instance generates monotonically ordered IDs, but multiple generators called
/// concurrently may produce unordered results unless explicitly synchronized. Use Rust's
/// synchronization mechanisms to control the scope of guaranteed monotonicity:
///
/// ```rust
/// use scru128::Scru128Generator;
/// use std::sync::{Arc, Mutex};
///
/// let g_shared = Arc::new(Mutex::new(Scru128Generator::new()));
///
/// let mut hs = Vec::new();
/// for i in 0..4 {
///     let g_shared = Arc::clone(&g_shared);
///     hs.push(std::thread::spawn(move || {
///         let mut g_local = Scru128Generator::new();
///         for _ in 0..4 {
///             println!("Shared generator: {}", g_shared.lock().unwrap().generate());
///             println!("Thread-local generator {}: {}", i, g_local.generate());
///         }
///     }));
/// }
///
/// for h in hs {
///     let _ = h.join();
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Scru128Generator<R = StdRng> {
    ts_last_gen: u64,
    counter: u32,
    ts_last_sec: u64,
    per_sec_random: u32,
    n_clock_check_max: usize,
    rng: R,
}

impl Default for Scru128Generator {
    fn default() -> Self {
        Self::with_rng(StdRng::from_entropy())
    }
}

impl Scru128Generator {
    /// Creates a generator object with the default random number generator.
    pub fn new() -> Self {
        Self::with_rng(StdRng::from_entropy())
    }
}

impl<R: RngCore> Scru128Generator<R> {
    /// Creates a generator object with a specified random number generator. The specified random
    /// number generator should be cryptographically strong and securely seeded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Generator;
    ///
    /// let mut g = Scru128Generator::with_rng(rand::thread_rng());
    /// println!("{}", g.generate());
    /// ```
    pub fn with_rng(rng: R) -> Self {
        Self {
            ts_last_gen: 0,
            counter: 0,
            ts_last_sec: 0,
            per_sec_random: 0,
            n_clock_check_max: 1_000_000,
            rng,
        }
    }

    /// Generates a new SCRU128 ID object.
    pub fn generate(&mut self) -> Scru128Id {
        // update timestamp and counter
        let mut ts_now = get_msec_unixts();
        if ts_now > self.ts_last_gen {
            self.ts_last_gen = ts_now;
            self.counter = self.rng.next_u32() & MAX_COUNTER;
        } else {
            self.counter += 1;
            if self.counter > MAX_COUNTER {
                #[cfg(feature = "log")]
                log::info!("counter limit reached; will wait until clock goes forward");
                let mut n_clock_check = 0;
                while ts_now <= self.ts_last_gen {
                    ts_now = get_msec_unixts();
                    n_clock_check += 1;
                    if n_clock_check > self.n_clock_check_max {
                        #[cfg(feature = "log")]
                        log::warn!("reset state as clock did not go forward");
                        self.ts_last_sec = 0;
                        break;
                    }
                }
                self.ts_last_gen = ts_now;
                self.counter = self.rng.next_u32() & MAX_COUNTER;
            }
        }

        // update per_sec_random
        if self.ts_last_gen - self.ts_last_sec > 1000 {
            self.ts_last_sec = self.ts_last_gen;
            self.per_sec_random = self.rng.next_u32() & MAX_PER_SEC_RANDOM;
        }

        Scru128Id::from_fields(
            self.ts_last_gen - TIMESTAMP_BIAS,
            self.counter,
            self.per_sec_random,
            self.rng.next_u32(),
        )
    }
}

/// Returns the current unix time in milliseconds.
fn get_msec_unixts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock may have gone backwards")
        .as_millis() as u64
}
