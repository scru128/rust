//! SCRU128 generator and related items.

use crate::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};

/// A trait that defines the minimum random number generator interface for [`Scru128Generator`].
pub trait Scru128Rng {
    /// Return the next random `u32`.
    fn next_u32(&mut self) -> u32;
}

impl<T: rand::RngCore> Scru128Rng for T {
    fn next_u32(&mut self) -> u32 {
        self.next_u32()
    }
}

#[cfg(feature = "std")]
pub use default_rng::DefaultRng;

/// The default random number generator used by [`Scru128Generator`].
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
/// | Flavor                     | Timestamp | On big clock rewind |
/// | -------------------------- | --------- | ------------------- |
/// | [`generate`]               | Now       | Resets generator    |
/// | [`generate_or_abort`]      | Now       | Returns `None`      |
/// | [`generate_or_reset_core`] | Argument  | Resets generator    |
/// | [`generate_or_abort_core`] | Argument  | Returns `None`      |
///
/// All of these methods return monotonically increasing IDs unless a `timestamp` provided is
/// significantly (by default, ten seconds or more) smaller than the one embedded in the
/// immediately preceding ID. If such a significant clock rollback is detected, the `generate`
/// (or_reset) method resets the generator and returns a new ID based on the given `timestamp`,
/// while the `or_abort` variants abort and return `None`. The `core` functions offer low-level
/// primitives.
///
/// [`generate`]: Scru128Generator::generate
/// [`generate_or_abort`]: Scru128Generator::generate_or_abort
/// [`generate_or_reset_core`]: Scru128Generator::generate_or_reset_core
/// [`generate_or_abort_core`]: Scru128Generator::generate_or_abort_core
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Scru128Generator<R = DefaultRng> {
    timestamp: u64,
    counter_hi: u32,
    counter_lo: u32,

    /// The timestamp at the last renewal of `counter_hi` field.
    ts_counter_hi: u64,

    /// The random number generator used by the generator.
    rng: R,
}

impl<R: Scru128Rng> Scru128Generator<R> {
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
            rng,
        }
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed, or resets the generator upon
    /// significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// The `rollback_allowance` parameter specifies the amount of `timestamp` rollback that is
    /// considered significant. A suggested value is `10_000` (milliseconds).
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    pub fn generate_or_reset_core(&mut self, timestamp: u64, rollback_allowance: u64) -> Scru128Id {
        if let Some(value) = self.generate_or_abort_core(timestamp, rollback_allowance) {
            value
        } else {
            // reset state and resume
            self.timestamp = 0;
            self.ts_counter_hi = 0;
            self.generate_or_abort_core(timestamp, rollback_allowance)
                .unwrap()
        }
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed, or returns `None` upon
    /// significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// The `rollback_allowance` parameter specifies the amount of `timestamp` rollback that is
    /// considered significant. A suggested value is `10_000` (milliseconds).
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    pub fn generate_or_abort_core(
        &mut self,
        timestamp: u64,
        rollback_allowance: u64,
    ) -> Option<Scru128Id> {
        if timestamp == 0 || timestamp > MAX_TIMESTAMP {
            panic!("`timestamp` must be a 48-bit positive integer");
        } else if rollback_allowance > MAX_TIMESTAMP {
            panic!("`rollback_allowance` out of reasonable range");
        }

        if timestamp > self.timestamp {
            self.timestamp = timestamp;
            self.counter_lo = self.rng.next_u32() & MAX_COUNTER_LO;
        } else if timestamp + rollback_allowance > self.timestamp {
            // go on with previous timestamp if new one is not much smaller
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
            // abort if clock went backwards to unbearable extent
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
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
mod std_ext {
    use super::{Scru128Generator, Scru128Id, Scru128Rng};
    use std::{iter, time};

    /// The default timestamp rollback allowance.
    const DEFAULT_ROLLBACK_ALLOWANCE: u64 = 10_000; // 10 seconds

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
            .expect("clock may have gone backwards")
            .as_millis() as u64
    }

    impl<R: Scru128Rng> Scru128Generator<R> {
        /// Generates a new SCRU128 ID object from the current `timestamp`, or resets the generator
        /// upon significant timestamp rollback.
        ///
        /// See the [`Scru128Generator`] type documentation for the description.
        pub fn generate(&mut self) -> Scru128Id {
            self.generate_or_reset_core(unix_ts_ms(), DEFAULT_ROLLBACK_ALLOWANCE)
        }

        /// Generates a new SCRU128 ID object from the current `timestamp`, or returns `None` upon
        /// significant timestamp rollback.
        ///
        /// See the [`Scru128Generator`] type documentation for the description.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use scru128::Scru128Generator;
        ///
        /// let mut g = Scru128Generator::new();
        /// let x = g.generate_or_abort().unwrap();
        /// let y = g
        ///     .generate_or_abort()
        ///     .expect("The clock went backwards by ten seconds!");
        /// assert!(x < y);
        /// ```
        pub fn generate_or_abort(&mut self) -> Option<Scru128Id> {
            self.generate_or_abort_core(unix_ts_ms(), DEFAULT_ROLLBACK_ALLOWANCE)
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
    impl<R: Scru128Rng> Iterator for Scru128Generator<R> {
        type Item = Scru128Id;

        fn next(&mut self) -> Option<Self::Item> {
            Some(self.generate())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (usize::MAX, None)
        }
    }

    impl<R: Scru128Rng> iter::FusedIterator for Scru128Generator<R> {}
}

#[cfg(test)]
mod tests_generate_or_reset {
    use super::Scru128Generator;

    /// Generates increasing IDs even with decreasing or constant timestamp
    #[test]
    fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
        let ts = 0x0123_4567_89abu64;
        #[cfg(feature = "std")]
        let mut g = Scru128Generator::new();
        #[cfg(not(feature = "std"))]
        let mut g = Scru128Generator::with_rng(super::tests::no_std_rng());

        let mut prev = g.generate_or_reset_core(ts, 10_000);
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000u64 {
            let curr = g.generate_or_reset_core(ts - i.min(9_998), 10_000);
            assert!(prev < curr);
            prev = curr;
        }
        assert!(prev.timestamp() >= ts);
    }

    /// Breaks increasing order of IDs if timestamp goes backwards a lot
    #[test]
    fn breaks_increasing_order_of_ids_if_timestamp_goes_backwards_a_lot() {
        let ts = 0x0123_4567_89abu64;
        #[cfg(feature = "std")]
        let mut g = Scru128Generator::new();
        #[cfg(not(feature = "std"))]
        let mut g = Scru128Generator::with_rng(super::tests::no_std_rng());

        let mut prev = g.generate_or_reset_core(ts, 10_000);
        assert_eq!(prev.timestamp(), ts);

        let mut curr = g.generate_or_reset_core(ts - 10_000, 10_000);
        assert!(prev > curr);
        assert_eq!(curr.timestamp(), ts - 10_000);

        prev = curr;
        curr = g.generate_or_reset_core(ts - 10_001, 10_000);
        assert!(prev < curr);
    }
}

#[cfg(test)]
mod tests_generate_or_abort {
    use super::Scru128Generator;

    /// Generates increasing IDs even with decreasing or constant timestamp
    #[test]
    fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
        let ts = 0x0123_4567_89abu64;
        #[cfg(feature = "std")]
        let mut g = Scru128Generator::new();
        #[cfg(not(feature = "std"))]
        let mut g = Scru128Generator::with_rng(super::tests::no_std_rng());

        let mut prev = g.generate_or_abort_core(ts, 10_000).unwrap();
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000u64 {
            let curr = g.generate_or_abort_core(ts - i.min(9_998), 10_000).unwrap();
            assert!(prev < curr);
            prev = curr;
        }
        assert!(prev.timestamp() >= ts);
    }

    /// Returns None if timestamp goes backwards a lot
    #[test]
    fn returns_none_if_timestamp_goes_backwards_a_lot() {
        let ts = 0x0123_4567_89abu64;
        #[cfg(feature = "std")]
        let mut g = Scru128Generator::new();
        #[cfg(not(feature = "std"))]
        let mut g = Scru128Generator::with_rng(super::tests::no_std_rng());

        let prev = g.generate_or_abort_core(ts, 10_000).unwrap();
        assert_eq!(prev.timestamp(), ts);

        let mut curr = g.generate_or_abort_core(ts - 10_000, 10_000);
        assert!(curr.is_none());

        curr = g.generate_or_abort_core(ts - 10_001, 10_000);
        assert!(curr.is_none());
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    pub fn no_std_rng() -> impl rand::RngCore {
        use rand::SeedableRng as _;
        let local_var = 0u32;
        let addr_as_seed = (&local_var as *const u32) as u64;
        rand_chacha::ChaCha12Rng::seed_from_u64(addr_as_seed)
    }

    /// Is iterable with for-in loop
    #[cfg(feature = "std")]
    #[test]
    fn is_iterable_with_for_in_loop() {
        use super::Scru128Generator;

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
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
mod default_rng {
    use super::Scru128Rng;
    use rand::{rngs::adapter::ReseedingRng, rngs::OsRng, SeedableRng};
    use rand_chacha::ChaCha12Core;

    /// The default random number generator used by [`Scru128Generator`].
    ///
    /// Currently, `DefaultRng` uses [`ChaCha12Core`] that is initially seeded and subsequently
    /// reseeded by [`OsRng`] every 64 kiB of random data using the [`ReseedingRng`] wrapper. It is
    /// the same strategy as that employed by [`ThreadRng`]; see the docs of `rand` crate for a
    /// detailed discussion on the strategy.
    ///
    /// This structure does exist under the `no_std` environment but is not able to be instantiated
    /// or used as a random number generator.
    ///
    /// [`Scru128Generator`]: super::Scru128Generator
    /// [`ChaCha12Core`]: rand_chacha::ChaCha12Core
    /// [`OsRng`]: rand::rngs::OsRng
    /// [`ReseedingRng`]: rand::rngs::adapter::ReseedingRng
    /// [`ThreadRng`]: https://docs.rs/rand/0.8/rand/rngs/struct.ThreadRng.html
    #[derive(Clone, Debug)]
    pub struct DefaultRng(ReseedingRng<ChaCha12Core, OsRng>);

    impl Default for DefaultRng {
        fn default() -> Self {
            let rng = ChaCha12Core::from_rng(OsRng).expect("could not initialize DefaultRng");
            Self(ReseedingRng::new(rng, 1024 * 64, OsRng))
        }
    }

    impl Scru128Rng for DefaultRng {
        fn next_u32(&mut self) -> u32 {
            self.0.next_u32()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{DefaultRng, Scru128Rng};

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
