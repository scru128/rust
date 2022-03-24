use crate::identifier::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO};

use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::{rngs::StdRng, RngCore, SeedableRng};

/// Represents a SCRU128 ID generator that encapsulates the monotonic counters and other internal
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
    timestamp: u64,
    counter_hi: u32,
    counter_lo: u32,

    /// Timestamp at the last renewal of `counter_hi` field.
    ts_counter_hi: u64,

    /// Random number generator used by the generator.
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
            timestamp: 0,
            counter_hi: 0,
            counter_lo: 0,
            ts_counter_hi: 0,
            rng,
        }
    }

    /// Generates a new SCRU128 ID object.
    pub fn generate(&mut self) -> Scru128Id {
        let ts = get_msec_unixts();
        if ts > self.timestamp {
            self.timestamp = ts;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
            if ts - self.ts_counter_hi >= 1000 {
                self.ts_counter_hi = ts;
                self.counter_hi = self.rng.next_u32() & MAX_COUNTER_HI;
            }
        } else {
            self.counter_lo += 1;
            if self.counter_lo > MAX_COUNTER_LO {
                self.counter_lo = 0;
                self.counter_hi += 1;
                if self.counter_hi > MAX_COUNTER_HI {
                    self.counter_hi = 0;
                    self.handle_counter_overflow();
                    return self.generate();
                }
            }
        }

        Scru128Id::from_fields(
            self.timestamp,
            self.counter_hi,
            self.counter_lo,
            self.rng.next_u32(),
        )
    }

    /// Defines the behavior on counter overflow.
    ///
    /// Currently, this method waits for the next clock tick and, if the clock does not move
    /// forward for a while, reinitializes the generator state.
    fn handle_counter_overflow(&mut self) {
        #[cfg(feature = "log")]
        log::warn!("counter overflowing; will wait for next clock tick");
        self.ts_counter_hi = 0;
        for _ in 0..10_000 {
            sleep(Duration::from_micros(100));
            if get_msec_unixts() > self.timestamp {
                return;
            }
        }
        #[cfg(feature = "log")]
        log::warn!("reset state as clock did not move for a while");
        self.timestamp = 0;
    }
}

/// Returns the current unix time in milliseconds.
fn get_msec_unixts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock may have gone backwards")
        .as_millis() as u64
}
