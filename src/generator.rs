//! SCRU128 generator and related items.
//!
//! This module is also exported as `scru128::gen` for backward compatibility.

#[cfg(not(feature = "std"))]
use core as std;
use std::{fmt, iter};

use crate::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};

/// A trait that defines the minimum random number generator interface for [`Scru128Generator`].
pub trait RandSource {
    /// Returns the next random `u32`.
    fn next_u32(&mut self) -> u32;
}

#[deprecated(since = "3.3.0", note = "use `RandSource` instead")]
pub use RandSource as Scru128Rng;

pub mod with_rand010;
pub mod with_rand08;
pub mod with_rand09;

/// A trait that defines the minimum system clock interface for [`Scru128Generator`].
pub trait TimeSource {
    /// Returns the current Unix timestamp in milliseconds.
    fn unix_ts_ms(&mut self) -> u64;
}

/// Represents a SCRU128 ID generator that encapsulates the monotonic counters and other internal
/// states.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "default_rng")]
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
/// # #[cfg(feature = "default_rng")]
/// # {
/// use scru128::Scru128Generator;
/// use std::sync::{Arc, Mutex};
///
/// let g_shared = Arc::new(Mutex::new(Scru128Generator::new()));
///
/// std::thread::scope(|s| {
///     for i in 0..4 {
///         let g_shared = Arc::clone(&g_shared);
///         s.spawn(move || {
///             let mut g_local = Scru128Generator::new();
///             for _ in 0..4 {
///                 println!("Shared generator: {}", g_shared.lock().unwrap().generate());
///                 println!("Thread-local generator {}: {}", i, g_local.generate());
///             }
///         });
///     }
/// });
/// # }
/// ```
///
/// # Generator functions
///
/// The generator comes with four different methods that generate a SCRU128 ID:
///
/// | Flavor                        | Timestamp | On big clock rewind |
/// | ----------------------------- | --------- | ------------------- |
/// | [`generate`]                  | Now       | Resets generator    |
/// | [`generate_or_abort`]         | Now       | Returns `None`      |
/// | [`generate_or_reset_with_ts`] | Argument  | Resets generator    |
/// | [`generate_or_abort_with_ts`] | Argument  | Returns `None`      |
///
/// All of the four return a monotonically increasing ID by reusing the previous `timestamp` even
/// if the one provided is smaller than the immediately preceding ID's. However, when such a clock
/// rollback is considered significant (by default, more than ten seconds):
///
/// 1.  `generate` (or_reset) methods reset the generator and return a new ID based on the given
///     `timestamp`, breaking the increasing order of IDs.
/// 2.  `or_abort` variants abort and return `None` immediately.
///
/// The `with_ts` functions accepts the `timestamp` as an argument.
///
/// [`generate`]: Scru128Generator::generate
/// [`generate_or_abort`]: Scru128Generator::generate_or_abort
/// [`generate_or_reset_with_ts`]: Scru128Generator::generate_or_reset_with_ts
/// [`generate_or_abort_with_ts`]: Scru128Generator::generate_or_abort_with_ts
#[derive(Clone, Eq, PartialEq)]
pub struct Scru128Generator<R = DefaultRng, T = StdSystemTime> {
    timestamp: u64,
    counter_hi: u32,
    counter_lo: u32,

    /// The timestamp at the last renewal of `counter_hi` field.
    ts_counter_hi: u64,

    /// The random number generator used by the generator.
    rand_source: R,

    /// The system clock used by the generator.
    time_source: T,

    /// The amount of `timestamp` rollback that is considered significant (in milliseconds).
    rollback_allowance: u64,
}

#[cfg(feature = "default_rng")]
impl Scru128Generator {
    /// Creates a generator object with the default random number generator.
    ///
    /// # Panics
    ///
    /// Panics in the highly unlikely event where [`DefaultRng`] could not be initialized.
    pub fn new() -> Self {
        Default::default()
    }
}

impl<R> Scru128Generator<R> {
    /// Creates a generator object with a specified random number generator. The specified random
    /// number generator should be cryptographically strong and securely seeded.
    ///
    /// Use [`Scru128Generator::with_rand010()`] to create a generator with the random number
    /// generators from `rand` crate. Although this constructor accepts `rand::RngCore` (v0.8)
    /// types for historical reasons, such behavior is deprecated and will be removed in the
    /// future.
    #[deprecated(
        since = "3.3.0",
        note = "use `with_rand_and_time_sources()` with `StdSystemTime` instead"
    )]
    pub const fn with_rng(rng: R) -> Self {
        Self::with_rand_and_time_sources(rng, StdSystemTime)
    }
}

impl<R, T> Scru128Generator<R, T> {
    /// Creates a generator object with specified random number generator and system clock.
    ///
    /// Use [`with_rand010::Adapter`] to pass a random number generator from `rand` crate. Although
    /// this constructor accepts `rand::RngCore` (v0.8) types for historical reasons, such behavior
    /// is deprecated and will be removed in the future.
    pub const fn with_rand_and_time_sources(rand_source: R, time_source: T) -> Self {
        Self {
            timestamp: 0,
            counter_hi: 0,
            counter_lo: 0,
            ts_counter_hi: 0,
            rand_source,
            time_source,
            rollback_allowance: 10_000, // 10 seconds in milliseconds
        }
    }

    /// Sets the `rollback_allowance` parameter of the generator.
    ///
    /// The `rollback_allowance` parameter specifies the amount of `timestamp` rollback that is
    /// considered significant. The default value is `10_000` (milliseconds). See the
    /// [`Scru128Generator`] type documentation for the treatment of the significant rollback.
    pub fn set_rollback_allowance(&mut self, rollback_allowance: u64) {
        if rollback_allowance > MAX_TIMESTAMP {
            panic!("`rollback_allowance` out of reasonable range");
        }
        self.rollback_allowance = rollback_allowance;
    }

    /// Resets the internal state of the generator.
    fn reset_state(&mut self) {
        self.timestamp = 0;
        self.counter_hi = 0;
        self.counter_lo = 0;
        self.ts_counter_hi = 0;
    }
}

impl<R: RandSource, T: TimeSource> Scru128Generator<R, T> {
    /// Generates a new SCRU128 ID object from the current `timestamp`, or resets the generator
    /// upon significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    pub fn generate(&mut self) -> Scru128Id {
        let timestamp = self.time_source.unix_ts_ms();
        self.generate_or_reset_with_ts(timestamp)
    }

    /// Generates a new SCRU128 ID object from the current `timestamp`, or returns `None` upon
    /// significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "default_rng")]
    /// # {
    /// use scru128::Scru128Generator;
    ///
    /// let mut g = Scru128Generator::new();
    /// let x = g.generate_or_abort().unwrap();
    /// let y = g
    ///     .generate_or_abort()
    ///     .expect("The clock went backwards by ten seconds!");
    /// assert!(x < y);
    /// # }
    /// ```
    pub fn generate_or_abort(&mut self) -> Option<Scru128Id> {
        let timestamp = self.time_source.unix_ts_ms();
        self.generate_or_abort_with_ts(timestamp)
    }
}

impl<R: RandSource, T> Scru128Generator<R, T> {
    /// Generates a new SCRU128 ID object from the `timestamp` passed, or resets the generator upon
    /// significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    pub fn generate_or_reset_with_ts(&mut self, timestamp: u64) -> Scru128Id {
        if let Some(value) = self.generate_or_abort_with_ts(timestamp) {
            value
        } else {
            // reset state and resume
            self.reset_state();
            self.generate_or_abort_with_ts(timestamp).unwrap()
        }
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed, or returns `None` upon
    /// significant timestamp rollback.
    ///
    /// See the [`Scru128Generator`] type documentation for the description.
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    pub fn generate_or_abort_with_ts(&mut self, timestamp: u64) -> Option<Scru128Id> {
        if timestamp == 0 || timestamp > MAX_TIMESTAMP {
            panic!("`timestamp` must be a 48-bit positive integer");
        }

        if timestamp > self.timestamp {
            self.timestamp = timestamp;
            self.counter_lo = self.rand_source.next_u32() & MAX_COUNTER_LO;
        } else if timestamp + self.rollback_allowance >= self.timestamp {
            // go on with previous timestamp if new one is not much smaller
            self.counter_lo += 1;
            if self.counter_lo > MAX_COUNTER_LO {
                self.counter_lo = 0;
                self.counter_hi += 1;
                if self.counter_hi > MAX_COUNTER_HI {
                    self.counter_hi = 0;
                    // increment timestamp at counter overflow
                    self.timestamp += 1;
                    self.counter_lo = self.rand_source.next_u32() & MAX_COUNTER_LO;
                }
            }
        } else {
            // abort if clock went backwards to unbearable extent
            return None;
        }

        if self.timestamp - self.ts_counter_hi >= 1_000 || self.ts_counter_hi == 0 {
            self.ts_counter_hi = self.timestamp;
            self.counter_hi = self.rand_source.next_u32() & MAX_COUNTER_HI;
        }

        Some(Scru128Id::from_fields(
            self.timestamp,
            self.counter_hi,
            self.counter_lo,
            self.rand_source.next_u32(),
        ))
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed, or resets the generator upon
    /// significant timestamp rollback.
    ///
    /// This method is a deprecated version of `generate_or_reset_with_ts()` that accepts the
    /// `rollback_allowance` parameter as an argument, rather than using [the generator-level
    /// parameter](Self::set_rollback_allowance).
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    #[deprecated(since = "3.3.0", note = "use `generate_or_reset_with_ts()` instead")]
    pub fn generate_or_reset_core(&mut self, timestamp: u64, rollback_allowance: u64) -> Scru128Id {
        #[allow(deprecated)]
        if let Some(value) = self.generate_or_abort_core(timestamp, rollback_allowance) {
            value
        } else {
            // reset state and resume
            self.reset_state();
            self.generate_or_abort_core(timestamp, rollback_allowance)
                .unwrap()
        }
    }

    /// Generates a new SCRU128 ID object from the `timestamp` passed, or returns `None` upon
    /// significant timestamp rollback.
    ///
    /// This method is a deprecated version of `generate_or_abort_with_ts()` that accepts the
    /// `rollback_allowance` parameter as an argument, rather than using [the generator-level
    /// parameter](Self::set_rollback_allowance).
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    #[deprecated(since = "3.3.0", note = "use `generate_or_abort_with_ts()` instead")]
    pub fn generate_or_abort_core(
        &mut self,
        timestamp: u64,
        rollback_allowance: u64,
    ) -> Option<Scru128Id> {
        struct PanicGuard<'a, R, T> {
            orig_rollback_allowance: u64,
            inner: &'a mut Scru128Generator<R, T>,
        }
        impl<R, T> Drop for PanicGuard<'_, R, T> {
            fn drop(&mut self) {
                self.inner.rollback_allowance = self.orig_rollback_allowance;
            }
        }

        let guard = PanicGuard {
            orig_rollback_allowance: self.rollback_allowance,
            inner: self,
        };
        guard.inner.set_rollback_allowance(rollback_allowance);
        guard.inner.generate_or_abort_with_ts(timestamp)
    }
}

impl<R: Default, T: Default> Default for Scru128Generator<R, T> {
    fn default() -> Self {
        Self::with_rand_and_time_sources(R::default(), T::default())
    }
}

impl<R: fmt::Debug, T: fmt::Debug> fmt::Debug for Scru128Generator<R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("Scru128Generator")
            .field("rand_source", &self.rand_source)
            .field("time_source", &self.time_source)
            .field("rollback_allowance", &self.rollback_allowance)
            .finish()
    }
}

/// `Scru128Generator` behaves as an infinite iterator that produces a new ID for each call of
/// `next()`.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "default_rng")]
/// # {
/// use scru128::Scru128Generator;
///
/// let g = Scru128Generator::new();
/// for (i, e) in g.take(8).enumerate() {
///     println!("[{}] {}", i, e);
/// }
/// # }
/// ```
impl<R: RandSource, T: TimeSource> Iterator for Scru128Generator<R, T> {
    type Item = Scru128Id;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generate())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

impl<R: RandSource, T: TimeSource> iter::FusedIterator for Scru128Generator<R, T> {}

/// The default random number generator used by [`Scru128Generator`].
///
/// Currently, `DefaultRng` uses [`ChaCha12Core`] that is initially seeded and subsequently
/// reseeded by [`OsRng`] every 64 kiB of random data using the [`ReseedingRng`] wrapper. It is the
/// same strategy as that employed by [`ThreadRng`]; see the docs of `rand` crate for a detailed
/// discussion on the strategy.
///
/// This structure does exist without the `default_rng` feature flag but is not able to be
/// instantiated or used as a random number generator.
///
/// [`ChaCha12Core`]: rand_chacha::ChaCha12Core
/// [`ReseedingRng`]: rand09::rngs::ReseedingRng
/// [`OsRng`]: rand09::rngs::OsRng
/// [`ThreadRng`]: rand09::rngs::ThreadRng
#[derive(Clone, Debug)]
pub struct DefaultRng {
    _private: (),

    #[cfg(feature = "default_rng")]
    inner: rand09::rngs::ReseedingRng<rand_chacha::ChaCha12Core, rand09::rngs::OsRng>,
}

#[cfg(feature = "default_rng")]
mod default_rng;

/// The default [`TimeSource`] that uses [`std::time::SystemTime`].
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct StdSystemTime;

#[cfg(feature = "std")]
impl TimeSource for StdSystemTime {
    fn unix_ts_ms(&mut self) -> u64 {
        use std::time;
        time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("clock may have gone backwards")
            .as_millis() as u64
    }
}

#[cfg(test)]
mod tests;
