//! SCRU128 generator and related items.
//!
//! This module is also exported as `scru128::gen` for backward compatibility.

#[cfg(not(feature = "std"))]
use core as std;
use std::iter;

use crate::{Scru128Id, MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};

/// A trait that defines the minimum random number generator interface for [`Scru128Generator`].
pub trait RandSource {
    /// Returns the next random `u32`.
    fn next_u32(&mut self) -> u32;
}

#[deprecated(since = "3.3.0", note = "use `RandSource` instead")]
pub use RandSource as Scru128Rng;

pub mod with_rand08;
pub mod with_rand09;

mod default_rng;
pub use default_rng::DefaultRng;

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
#[derive(Clone, Eq, PartialEq, Debug)]
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

impl<R> Scru128Generator<R> {
    /// Creates a generator object with a specified random number generator. The specified random
    /// number generator should be cryptographically strong and securely seeded.
    ///
    /// Use [`Scru128Generator::with_rand09()`] to create a generator with the random number
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
    /// Use [`Scru128Generator::with_rand09()`] to create a generator with the random number
    /// generators from `rand` crate.
    pub const fn with_rand_and_time_sources(rand_source: R, time_source: T) -> Self {
        Self {
            timestamp: 0,
            counter_hi: 0,
            counter_lo: 0,
            ts_counter_hi: 0,
            rand_source,
            time_source,
            rollback_allowance: 10_000, // 10 seconds
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
        self.generate_or_reset_core_inner(timestamp, self.rollback_allowance)
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
    #[deprecated(since = "3.3.0", note = "use `generate_or_reset_with_ts()` instead")]
    pub fn generate_or_reset_core(&mut self, timestamp: u64, rollback_allowance: u64) -> Scru128Id {
        self.generate_or_reset_core_inner(timestamp, rollback_allowance)
    }

    fn generate_or_reset_core_inner(
        &mut self,
        timestamp: u64,
        rollback_allowance: u64,
    ) -> Scru128Id {
        if let Some(value) = self.generate_or_abort_core_inner(timestamp, rollback_allowance) {
            value
        } else {
            // reset state and resume
            self.timestamp = 0;
            self.ts_counter_hi = 0;
            self.generate_or_abort_core_inner(timestamp, rollback_allowance)
                .unwrap()
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
        self.generate_or_abort_core_inner(timestamp, self.rollback_allowance)
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
    #[deprecated(since = "3.3.0", note = "use `generate_or_abort_with_ts()` instead")]
    pub fn generate_or_abort_core(
        &mut self,
        timestamp: u64,
        rollback_allowance: u64,
    ) -> Option<Scru128Id> {
        self.generate_or_abort_core_inner(timestamp, rollback_allowance)
    }

    fn generate_or_abort_core_inner(
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
            self.counter_lo = self.rand_source.next_u32() & MAX_COUNTER_LO;
        } else if timestamp + rollback_allowance >= self.timestamp {
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

impl<R: Default, T: Default> Default for Scru128Generator<R, T> {
    fn default() -> Self {
        Self::with_rand_and_time_sources(R::default(), T::default())
    }
}

/// The default time source that uses [`std::time::SystemTime`].
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

#[cfg(any(feature = "default_rng", test))]
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

#[cfg(test)]
mod tests_generate_or_reset {
    use super::Scru128Generator;

    /// Generates increasing IDs even with decreasing or constant timestamp
    #[test]
    fn generates_increasing_ids_even_with_decreasing_or_constant_timestamp() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();

        let mut prev = g.generate_or_reset_with_ts(ts);
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000u64 {
            let curr = g.generate_or_reset_with_ts(ts - i.min(9_999));
            assert!(prev < curr);
            prev = curr;
        }
        assert!(prev.timestamp() >= ts);
    }

    /// Breaks increasing order of IDs if timestamp goes backwards a lot
    #[test]
    fn breaks_increasing_order_of_ids_if_timestamp_goes_backwards_a_lot() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();

        let mut prev = g.generate_or_reset_with_ts(ts);
        assert_eq!(prev.timestamp(), ts);

        let mut curr = g.generate_or_reset_with_ts(ts - 10_000);
        assert!(prev < curr);

        prev = curr;
        curr = g.generate_or_reset_with_ts(ts - 10_001);
        assert!(prev > curr);
        assert_eq!(curr.timestamp(), ts - 10_001);

        prev = curr;
        curr = g.generate_or_reset_with_ts(ts - 10_002);
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
        let mut g = Scru128Generator::new();

        let mut prev = g.generate_or_abort_with_ts(ts).unwrap();
        assert_eq!(prev.timestamp(), ts);

        for i in 0..100_000u64 {
            let curr = g.generate_or_abort_with_ts(ts - i.min(9_999)).unwrap();
            assert!(prev < curr);
            prev = curr;
        }
        assert!(prev.timestamp() >= ts);
    }

    /// Returns None if timestamp goes backwards a lot
    #[test]
    fn returns_none_if_timestamp_goes_backwards_a_lot() {
        let ts = 0x0123_4567_89abu64;
        let mut g = Scru128Generator::new();

        let prev = g.generate_or_abort_with_ts(ts).unwrap();
        assert_eq!(prev.timestamp(), ts);

        let mut curr = g.generate_or_abort_with_ts(ts - 10_000);
        assert!(prev < curr.unwrap());

        curr = g.generate_or_abort_with_ts(ts - 10_001);
        assert!(curr.is_none());

        curr = g.generate_or_abort_with_ts(ts - 10_002);
        assert!(curr.is_none());
    }
}

#[cfg(all(feature = "std", test))]
mod tests_iterator {
    /// Is iterable with for-in loop
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
