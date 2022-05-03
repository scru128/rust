//! SCRU128 generator and related items.

use crate::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO};
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

pub use default_rng::DefaultRng;

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
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Scru128Generator<R = DefaultRng> {
    timestamp: u64,
    counter_hi: u32,
    counter_lo: u32,

    /// Timestamp at the last renewal of `counter_hi` field.
    ts_counter_hi: u64,

    /// Random number generator used by the generator.
    rng: R,
}

impl Scru128Generator {
    /// Creates a generator object with the default random number generator.
    pub fn new() -> Self {
        Default::default()
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
            timestamp: Default::default(),
            counter_hi: Default::default(),
            counter_lo: Default::default(),
            ts_counter_hi: Default::default(),
            rng,
        }
    }

    /// Generates a new SCRU128 ID object.
    pub fn generate(&mut self) -> Scru128Id {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock may have gone backwards")
            .as_millis() as u64;
        if ts > self.timestamp {
            self.timestamp = ts;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
        } else if ts + 10_000 > self.timestamp {
            self.counter_lo += 1;
            if self.counter_lo > MAX_COUNTER_LO {
                self.counter_lo = 0;
                self.counter_hi += 1;
                if self.counter_hi > MAX_COUNTER_HI {
                    self.counter_hi = 0;
                    // increment timestamp at counter overflow
                    self.timestamp += 1;
                    self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
                }
            }
        } else {
            // reset state if clock moves back more than ten seconds
            self.ts_counter_hi = 0;
            self.timestamp = ts;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
        }

        if self.timestamp - self.ts_counter_hi >= 1_000 {
            self.ts_counter_hi = self.timestamp;
            self.counter_hi = self.rng.next_u32() & MAX_COUNTER_HI;
        }

        Scru128Id::from_fields(
            self.timestamp,
            self.counter_hi,
            self.counter_lo,
            self.rng.next_u32(),
        )
    }
}

mod default_rng {
    use rand::{rngs::StdRng, Error, RngCore, SeedableRng};

    /// Default random number generator used by [`Scru128Generator`].
    ///
    /// Currently, this type wraps an instance of [`rand::rngs::StdRng`] and delegates all the jobs
    /// to it.
    ///
    /// [`Scru128Generator`]: super::Scru128Generator
    #[derive(Clone, Debug)]
    pub struct DefaultRng(StdRng);

    impl Default for DefaultRng {
        fn default() -> Self {
            Self(StdRng::from_entropy())
        }
    }

    impl RngCore for DefaultRng {
        fn next_u32(&mut self) -> u32 {
            self.0.next_u32()
        }

        fn next_u64(&mut self) -> u64 {
            self.0.next_u64()
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            self.0.fill_bytes(dest)
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
            self.0.try_fill_bytes(dest)
        }
    }
}
