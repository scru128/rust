//! SCRU128 generator and related items.

use crate::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};
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

    /// Status code reported at the last generation.
    last_status: Status,

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
            last_status: Default::default(),
            rng,
        }
    }

    /// Generates a new SCRU128 ID object.
    pub fn generate(&mut self) -> Scru128Id {
        self.generate_core(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock may have gone backward")
                .as_millis() as u64,
        )
    }

    /// Generates a new SCRU128 ID object with the `timestamp` passed.
    ///
    /// # Panics
    ///
    /// Panics if the argument is not a 48-bit unsigned integer.
    pub fn generate_core(&mut self, timestamp: u64) -> Scru128Id {
        if timestamp > MAX_TIMESTAMP {
            panic!("`timestamp` must be a 48-bit unsigned integer");
        }

        self.last_status = Status::NewTimestamp;
        if timestamp > self.timestamp {
            self.timestamp = timestamp;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
        } else if timestamp + 10_000 > self.timestamp {
            self.counter_lo += 1;
            self.last_status = Status::CounterLoInc;
            if self.counter_lo > MAX_COUNTER_LO {
                self.counter_lo = 0;
                self.counter_hi += 1;
                self.last_status = Status::CounterHiInc;
                if self.counter_hi > MAX_COUNTER_HI {
                    self.counter_hi = 0;
                    // increment timestamp at counter overflow
                    self.timestamp += 1;
                    self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
                    self.last_status = Status::TimestampInc;
                }
            }
        } else {
            // reset state if clock moves back by ten seconds or more
            self.ts_counter_hi = 0;
            self.timestamp = timestamp;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
            self.last_status = Status::ClockRollback;
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

    /// Returns a [`Status`] code that indicates the internal state involved in the last generation
    /// of ID.
    ///
    /// Note that the generator object should be protected from concurrent accesses during the
    /// sequential calls to a generation method and this method to avoid race conditions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::{generator::Status, Scru128Generator};
    ///
    /// let mut g = Scru128Generator::new();
    /// let x = g.generate();
    /// let y = g.generate();
    /// if g.last_status() == Status::ClockRollback {
    ///     panic!("clock moved backward");
    /// } else {
    ///     assert!(x < y);
    /// }
    /// ```
    pub fn last_status(&self) -> Status {
        self.last_status
    }
}

/// Status code returned by [`Scru128Generator::last_status()`] method.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Status {
    /// Indicates that the generator has yet to generate an ID.
    NotExecuted,

    /// Indicates that the latest `timestamp` was used because it was greater than the previous
    /// one.
    NewTimestamp,

    /// Indicates that `counter_lo` was incremented because the latest `timestamp` was no greater
    /// than the previous one.
    CounterLoInc,

    /// Indicates that `counter_hi` was incremented because `counter_lo` reached its maximum value.
    CounterHiInc,

    /// Indicates that the previous `timestamp` was incremented because `counter_hi` reached its
    /// maximum value.
    TimestampInc,

    /// Indicates that the monotonic order of generated IDs was broken because the latest
    /// `timestamp` was less than the previous one by ten seconds or more.
    ClockRollback,
}

impl Default for Status {
    fn default() -> Self {
        Status::NotExecuted
    }
}

#[cfg(test)]
mod tests {
    use super::{Scru128Generator, Status};

    /// Generates increasing IDs even with decreasing or constant timestamp
    #[test]
    fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();
        assert_eq!(g.last_status(), Status::NotExecuted);

        let mut prev = g.generate_core(ts);
        assert_eq!(g.last_status(), Status::NewTimestamp);
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000 as u64 {
            let curr = g.generate_core(ts - i.min(9_998));
            assert!(
                g.last_status() == Status::CounterLoInc
                    || g.last_status() == Status::CounterHiInc
                    || g.last_status() == Status::TimestampInc
            );
            assert!(prev < curr);
            prev = curr;
        }
        assert!(prev.timestamp() >= ts);
    }

    /// Breaks increasing order of IDs if timestamp moves backward a lot
    #[test]
    fn breaks_increasing_order_of_ids_if_timestamp_moves_backward_a_lot() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();
        assert_eq!(g.last_status(), Status::NotExecuted);

        let prev = g.generate_core(ts);
        assert_eq!(g.last_status(), Status::NewTimestamp);
        assert_eq!(prev.timestamp(), ts);

        let curr = g.generate_core(ts - 10_000);
        assert_eq!(g.last_status(), Status::ClockRollback);
        assert!(prev > curr);
        assert_eq!(curr.timestamp(), ts - 10_000);
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
