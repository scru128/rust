//! SCRU128 generator and related items.

use crate::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};

#[cfg(feature = "std")]
pub use default_rng::DefaultRng;

/// Default random number generator used by [`Scru128Generator`].
///
/// No default random number generator is available in `no_std` mode.
#[cfg(not(feature = "std"))]
#[derive(Clone, Debug)]
pub struct DefaultRng(());

/// Represents a SCRU128 ID generator that encapsulates the monotonic counters and other internal
/// states.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "std")]
/// # {
/// use scru128::Scru128Generator;
///
/// let mut g = Scru128Generator::new();
/// println!("{}", g.generate());
/// println!("{}", g.generate().to_u128());
/// # }
/// ```
///
/// Each generator instance generates monotonically ordered IDs, but multiple generators called
/// concurrently may produce unordered results unless explicitly synchronized. Use Rust's
/// synchronization mechanisms to control the scope of guaranteed monotonicity:
///
/// ```rust
/// # #[cfg(feature = "std")]
/// # {
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
///             println!("Thread-local generator {i}: {}", g_local.generate());
///         }
///     }));
/// }
///
/// for h in hs {
///     let _ = h.join();
/// }
/// # }
/// ```
///
/// # Generator functions
///
/// The generator offers four different methods to generate a SCRU128 ID:
///
/// | Flavor                      | Timestamp | On big clock rewind |
/// | --------------------------- | --------- | ------------------- |
/// | [`generate`]                | Now       | Rewinds state       |
/// | [`generate_no_rewind`]      | Now       | Returns `None`      |
/// | [`generate_core`]           | Argument  | Rewinds state       |
/// | [`generate_core_no_rewind`] | Argument  | Returns `None`      |
///
/// Each method returns monotonically increasing IDs unless a `timestamp` provided is significantly
/// (by ten seconds or more) smaller than the one embedded in the immediately preceding ID. If such
/// a significant clock rollback is detected, the standard `generate` rewinds the generator state
/// and returns a new ID based on the current `timestamp`, whereas `no_rewind` variants keep the
/// state untouched and return `None`. `core` functions offer low-level primitives.
///
/// [`generate`]: Scru128Generator::generate
/// [`generate_no_rewind`]: Scru128Generator::generate_no_rewind
/// [`generate_core`]: Scru128Generator::generate_core
/// [`generate_core_no_rewind`]: Scru128Generator::generate_core_no_rewind
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

impl<R: rand::RngCore> Scru128Generator<R> {
    /// Creates a generator object with a specified random number generator. The specified random
    /// number generator should be cryptographically strong and securely seeded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "std")]
    /// # {
    /// use scru128::Scru128Generator;
    ///
    /// let mut g = Scru128Generator::with_rng(rand::rngs::OsRng);
    /// println!("{}", g.generate());
    /// # }
    /// ```
    pub const fn with_rng(rng: R) -> Self {
        Self {
            timestamp: 0,
            counter_hi: 0,
            counter_lo: 0,
            ts_counter_hi: 0,
            last_status: Status::NotExecuted,
            rng,
        }
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// # Panics
    ///
    /// Panics if the argument is not a 48-bit positive integer.
    pub fn generate_core(&mut self, timestamp: u64) -> Scru128Id {
        if let Some(value) = self.generate_core_no_rewind(timestamp) {
            value
        } else {
            // reset state and resume
            self.timestamp = 0;
            self.ts_counter_hi = 0;
            let value = self.generate_core_no_rewind(timestamp).unwrap();
            self.last_status = Status::ClockRollback;
            value
        }
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed, guaranteeing the monotonic
    /// order of generated IDs despite a significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// # Panics
    ///
    /// Panics if the argument is not a 48-bit positive integer.
    pub fn generate_core_no_rewind(&mut self, timestamp: u64) -> Option<Scru128Id> {
        const ROLLBACK_ALLOWANCE: u64 = 10_000; // 10 seconds

        if timestamp == 0 || timestamp > MAX_TIMESTAMP {
            panic!("`timestamp` must be a 48-bit positive integer");
        }

        if timestamp > self.timestamp {
            self.timestamp = timestamp;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
            self.last_status = Status::NewTimestamp;
        } else if timestamp + ROLLBACK_ALLOWANCE > self.timestamp {
            // go on with previous timestamp if new one is not much smaller
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
            // abort if clock moves back to unbearable extent
            return None;
        }

        if self.timestamp - self.ts_counter_hi >= 1_000 || self.ts_counter_hi == 0 {
            self.ts_counter_hi = self.timestamp;
            self.counter_hi = self.rng.next_u32() & MAX_COUNTER_HI;
        }

        Some(Scru128Id::from_fields(
            self.timestamp,
            self.counter_hi,
            self.counter_lo,
            self.rng.next_u32(),
        ))
    }

    /// Returns a [`Status`] code that indicates the internal state involved in the last generation
    /// of ID.
    ///
    /// Note that the generator object should be protected from concurrent accesses during the
    /// sequential calls to a generation method and this method to avoid race conditions.
    #[deprecated(
        since = "2.6.0",
        note = "use `generate_no_rewind()` to guarantee monotonicity"
    )]
    pub const fn last_status(&self) -> Status {
        self.last_status
    }
}

/// _Deprecated_. Status code returned by [`Scru128Generator::last_status()`] method.
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

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
mod std_ext {
    use super::{Scru128Generator, Scru128Id};
    use std::{iter, time};

    impl Scru128Generator {
        /// Creates a generator object with the default random number generator.
        pub fn new() -> Self {
            Default::default()
        }
    }

    /// Returns the current Unix timestamp in milliseconds.
    fn unix_ts_ms() -> u64 {
        time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("clock may have gone backward")
            .as_millis() as u64
    }

    impl<R: rand::RngCore> Scru128Generator<R> {
        /// Generates a new SCRU128 ID object from the current `timestamp`.
        ///
        /// See the [`Scru128Generator`] type documentation for the description.
        pub fn generate(&mut self) -> Scru128Id {
            self.generate_core(unix_ts_ms())
        }

        /// Generates a new SCRU128 ID object from the current `timestamp`, guaranteeing the
        /// monotonic order of generated IDs despite a significant timestamp rollback.
        ///
        /// See the [`Scru128Generator`] type documentation for the description.
        pub fn generate_no_rewind(&mut self) -> Option<Scru128Id> {
            self.generate_core_no_rewind(unix_ts_ms())
        }
    }

    /// `Scru128Generator` behaves as an infinite iterator that produces a new ID for each call of
    /// `next()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Generator;
    ///
    /// let g = Scru128Generator::new();
    /// for (i, e) in g.take(8).enumerate() {
    ///     println!("[{i}] {e}");
    /// }
    /// ```
    impl<R: rand::RngCore> Iterator for Scru128Generator<R> {
        type Item = Scru128Id;

        fn next(&mut self) -> Option<Self::Item> {
            Some(self.generate())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (usize::MAX, None)
        }
    }

    impl<R: rand::RngCore> iter::FusedIterator for Scru128Generator<R> {}
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests_generate_core {
    use super::{Scru128Generator, Status};

    /// Generates increasing IDs even with decreasing or constant timestamp
    #[test]
    fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();
        assert_eq!(g.last_status, Status::NotExecuted);

        let mut prev = g.generate_core(ts);
        assert_eq!(g.last_status, Status::NewTimestamp);
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000u64 {
            let curr = g.generate_core(ts - i.min(9_998));
            assert!(
                g.last_status == Status::CounterLoInc
                    || g.last_status == Status::CounterHiInc
                    || g.last_status == Status::TimestampInc
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
        assert_eq!(g.last_status, Status::NotExecuted);

        let mut prev = g.generate_core(ts);
        assert_eq!(g.last_status, Status::NewTimestamp);
        assert_eq!(prev.timestamp(), ts);

        let mut curr = g.generate_core(ts - 10_000);
        assert_eq!(g.last_status, Status::ClockRollback);
        assert!(prev > curr);
        assert_eq!(curr.timestamp(), ts - 10_000);

        prev = curr;
        curr = g.generate_core(ts - 10_001);
        assert!(
            g.last_status == Status::CounterLoInc
                || g.last_status == Status::CounterHiInc
                || g.last_status == Status::TimestampInc
        );
        assert!(prev < curr);
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests_generate_core_no_rewind {
    use super::{Scru128Generator, Status};

    /// Generates increasing IDs even with decreasing or constant timestamp
    #[test]
    fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();
        assert_eq!(g.last_status, Status::NotExecuted);

        let mut prev = g.generate_core_no_rewind(ts).unwrap();
        assert_eq!(g.last_status, Status::NewTimestamp);
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000u64 {
            let curr = g.generate_core_no_rewind(ts - i.min(9_998)).unwrap();
            assert!(
                g.last_status == Status::CounterLoInc
                    || g.last_status == Status::CounterHiInc
                    || g.last_status == Status::TimestampInc
            );
            assert!(prev < curr);
            prev = curr;
        }
        assert!(prev.timestamp() >= ts);
    }

    /// Returns None if timestamp moves backward a lot
    #[test]
    fn returns_none_if_timestamp_moves_backward_a_lot() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();
        assert_eq!(g.last_status, Status::NotExecuted);

        let prev = g.generate_core_no_rewind(ts).unwrap();
        assert_eq!(g.last_status, Status::NewTimestamp);
        assert_eq!(prev.timestamp(), ts);

        let mut curr = g.generate_core_no_rewind(ts - 10_000);
        assert!(curr.is_none());
        assert_eq!(g.last_status, Status::NewTimestamp);

        curr = g.generate_core_no_rewind(ts - 10_001);
        assert!(curr.is_none());
        assert_eq!(g.last_status, Status::NewTimestamp);
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use super::Scru128Generator;

    /// Is iterable with for-in loop
    #[test]
    fn is_iterable_with_for_in_loop() {
        let mut i = 0;
        for e in Scru128Generator::new() {
            assert!(e.timestamp() > 0);
            i += 1;
            if i > 100 {
                break;
            }
        }
        assert_eq!(i, 101);
    }
}

#[cfg(feature = "std")]
mod default_rng {
    use rand::{rngs::adapter::ReseedingRng, rngs::OsRng, SeedableRng};
    use rand_chacha::ChaCha12Core;

    /// Default random number generator used by [`Scru128Generator`].
    ///
    /// Currently, `DefaultRng` uses [`ChaCha12Core`] that is initially seeded and subsequently
    /// reseeded by [`OsRng`] every 64 kiB of random data using the [`ReseedingRng`] wrapper. It is
    /// the same strategy as that employed by [`ThreadRng`]; see the docs of `rand` crate for a
    /// detailed discussion on the strategy.
    ///
    /// [`Scru128Generator`]: super::Scru128Generator
    /// [`ChaCha12Core`]: rand_chacha::ChaCha12Core
    /// [`OsRng`]: rand::rngs::OsRng
    /// [`ReseedingRng`]: rand::rngs::adapter::ReseedingRng
    /// [`ThreadRng`]: https://docs.rs/rand/0.8/rand/rngs/struct.ThreadRng.html
    #[derive(Clone, Debug)]
    pub struct DefaultRng(ReseedingRng<ChaCha12Core, OsRng>);

    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    impl Default for DefaultRng {
        fn default() -> Self {
            let rng = ChaCha12Core::from_rng(OsRng)
                .unwrap_or_else(|err| panic!("could not initialize DefaultRng: {err}"));
            Self(ReseedingRng::new(rng, 1024 * 64, OsRng))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    impl rand::RngCore for DefaultRng {
        fn next_u32(&mut self) -> u32 {
            self.0.next_u32()
        }

        fn next_u64(&mut self) -> u64 {
            self.0.next_u64()
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            self.0.fill_bytes(dest)
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            self.0.try_fill_bytes(dest)
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    impl rand::CryptoRng for DefaultRng {}

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
