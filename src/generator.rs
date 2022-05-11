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
/// println!("{}", g.generate().to_u128());
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
    /// let mut g = Scru128Generator::with_rng(rand::rngs::OsRng);
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
    use rand::{rngs::adapter::ReseedingRng, rngs::OsRng, Error, RngCore, SeedableRng};
    use rand_chacha::ChaCha12Core;

    /// Default random number generator used by [`Scru128Generator`].
    ///
    /// Currently, `DefaultRng` uses [`ChaCha12Core`] that is initially seeded and subsequently
    /// reseeded by [`OsRng`] every 64 kiB of random data using the [`ReseedingRng`] wrapper. It is
    /// the same strategy as that employed by [`ThreadRng`]; see the docs for a detailed discussion
    /// on the strategy.
    ///
    /// [`Scru128Generator`]: super::Scru128Generator
    /// [`ChaCha12Core`]: rand_chacha::ChaCha12Core
    /// [`OsRng`]: rand::rngs::OsRng
    /// [`ReseedingRng`]: rand::rngs::adapter::ReseedingRng
    /// [`ThreadRng`]: rand::rngs::ThreadRng
    #[derive(Clone, Debug)]
    pub struct DefaultRng(ReseedingRng<ChaCha12Core, OsRng>);

    impl Default for DefaultRng {
        fn default() -> Self {
            let rng = ChaCha12Core::from_rng(OsRng)
                .unwrap_or_else(|err| panic!("could not initialize DefaultRng: {}", err));
            Self(ReseedingRng::new(rng, 1024 * 64, OsRng))
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

    #[cfg(test)]
    mod tests {
        use super::DefaultRng;
        use rand::RngCore;

        /// Generates unbiased random numbers
        ///
        /// This test may fail at a very low probability.
        #[test]
        fn generates_unbiased_random_numbers() {
            let mut rng = DefaultRng::default();

            // test if random bits are set to 1 at ~50% probability
            let mut counts = [0u32; 32];

            // test if XOR of two consecutive outputs is also random
            let mut prev = rng.next_u32();
            let mut counts_xor = [0u32; 32];

            const N: usize = 1_000_000;
            for _ in 0..N {
                let num = rng.next_u32();

                let mut x = num;
                for e in counts.iter_mut().rev() {
                    *e += x & 1;
                    x >>= 1;
                }

                let mut x = prev ^ num;
                for e in counts_xor.iter_mut().rev() {
                    *e += x & 1;
                    x >>= 1;
                }
                prev = num;
            }

            // set margin based on binom dist 99.999% confidence interval
            let margin = 4.417173 * (0.5 * 0.5 / N as f64).sqrt();
            assert!(counts
                .iter()
                .all(|e| (*e as f64 / N as f64 - 0.5).abs() < margin));
            assert!(counts_xor
                .iter()
                .all(|e| (*e as f64 / N as f64 - 0.5).abs() < margin));
        }
    }
}
