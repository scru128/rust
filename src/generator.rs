//! SCRU128 generator and related items.

#[cfg(not(feature = "std"))]
use core as std;
use std::{fmt, iter};

use crate::{Id, MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};

/// A trait that defines the minimum random number generator interface for [`Generator`].
pub trait RandSource {
    /// Returns the next random `u32`.
    fn next_u32(&mut self) -> u32;
}

pub mod with_rand010;
pub mod with_rand09;

/// A trait that defines the minimum system clock interface for [`Generator`].
pub trait TimeSource {
    /// Returns the current Unix timestamp in milliseconds.
    fn unix_ts_ms(&mut self) -> u64;
}

/// Represents a SCRU128 ID generator that encapsulates the counters and other internal state and
/// guarantees the monotonic order of IDs generated within the same millisecond.
///
/// This type provides the interface to customize the random number generator, system clock, and
/// clock rollback handling of a SCRU128 ID generator.
///
/// # Examples
///
/// This structure is typically instantiated with a random number generator from `rand` crate via
/// the adapters enabled by the corresponding cargo features.
///
/// ```rust
/// # #[cfg(all(feature = "std", feature = "rand010"))]
/// # {
/// let mut g = scru128::Generator::with_rand010(rand::rng());
/// println!("{}", g.generate());
/// if let Some(value) = g.generate_or_abort() {
///     println!("{}", value);
/// }
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
/// [`generate`]: Generator::generate
/// [`generate_or_abort`]: Generator::generate_or_abort
/// [`generate_or_reset_with_ts`]: Generator::generate_or_reset_with_ts
/// [`generate_or_abort_with_ts`]: Generator::generate_or_abort_with_ts
#[derive(Clone, Eq, PartialEq)]
pub struct Generator<R, T = StdSystemTime> {
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

impl<R, T> Generator<R, T> {
    /// Creates a generator object with specified random number generator and system clock.
    ///
    /// Use [`with_rand010::Adapter`] to pass a random number generator from `rand` crate.
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
    /// considered significant. The default value is `10_000` (milliseconds). See the [`Generator`]
    /// type documentation for the treatment of the significant rollback.
    pub fn set_rollback_allowance(&mut self, rollback_allowance: u64) {
        if rollback_allowance > MAX_TIMESTAMP {
            panic!("`rollback_allowance` out of reasonable range");
        }
        self.rollback_allowance = rollback_allowance;
    }

    /// Resets the internal state of the generator.
    pub(crate) fn reset_state(&mut self) {
        self.timestamp = 0;
        self.counter_hi = 0;
        self.counter_lo = 0;
        self.ts_counter_hi = 0;
    }

    /// Returns a mutable reference to the inner random number source.
    #[cfg(all(unix, feature = "global_gen"))]
    pub(crate) fn rand_source_mut(&mut self) -> &mut R {
        &mut self.rand_source
    }
}

impl<R: RandSource, T: TimeSource> Generator<R, T> {
    /// Generates a new SCRU128 ID object from the current `timestamp`, or resets the generator
    /// upon significant timestamp rollback.
    ///
    /// See the [`Generator`] type documentation for the description.
    pub fn generate(&mut self) -> Id {
        let timestamp = self.time_source.unix_ts_ms();
        self.generate_or_reset_with_ts(timestamp)
    }

    /// Generates a new SCRU128 ID object from the current `timestamp`, or returns `None` upon
    /// significant timestamp rollback.
    ///
    /// See the [`Generator`] type documentation for the description.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(all(feature = "std", feature = "rand010"))]
    /// # {
    /// let mut g = scru128::Generator::with_rand010(rand::rng());
    /// let x = g.generate_or_abort().unwrap();
    /// let y = g
    ///     .generate_or_abort()
    ///     .expect("The clock went backwards by ten seconds!");
    /// assert!(x < y);
    /// # }
    /// ```
    pub fn generate_or_abort(&mut self) -> Option<Id> {
        let timestamp = self.time_source.unix_ts_ms();
        self.generate_or_abort_with_ts(timestamp)
    }

    /// Returns an infinite iterator that produces a new ID for each call of `next()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(all(feature = "std", feature = "rand010"))]
    /// # {
    /// let mut g = scru128::Generator::with_rand010(rand::rng());
    /// for (i, e) in g.iter().take(8).enumerate() {
    ///     println!("[{}] {}", i, e);
    /// }
    /// # }
    /// ```
    pub fn iter(&mut self) -> impl Iterator<Item = Id> {
        iter::from_fn(|| Some(self.generate()))
    }
}

impl<R: RandSource, T> Generator<R, T> {
    /// Generates a new SCRU128 ID object from the `timestamp` passed, or resets the generator upon
    /// significant timestamp rollback.
    ///
    /// See the [`Generator`] type documentation for the description.
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    pub fn generate_or_reset_with_ts(&mut self, timestamp: u64) -> Id {
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
    /// See the [`Generator`] type documentation for the description.
    ///
    /// # Panics
    ///
    /// Panics if `timestamp` is not a 48-bit positive integer.
    pub fn generate_or_abort_with_ts(&mut self, timestamp: u64) -> Option<Id> {
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

        Some(
            Id::try_from_fields(
                self.timestamp,
                self.counter_hi,
                self.counter_lo,
                self.rand_source.next_u32(),
            )
            .unwrap(),
        )
    }
}

impl<R: Default, T: Default> Default for Generator<R, T> {
    fn default() -> Self {
        Self::with_rand_and_time_sources(R::default(), T::default())
    }
}

impl<R: fmt::Debug, T: fmt::Debug> fmt::Debug for Generator<R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("Generator")
            .field("rand_source", &self.rand_source)
            .field("time_source", &self.time_source)
            .field("rollback_allowance", &self.rollback_allowance)
            .finish_non_exhaustive()
    }
}

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
