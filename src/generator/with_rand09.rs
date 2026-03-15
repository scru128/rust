//! Integration with `rand` (v0.9) crate.
//!
//! `rand09` feature is deprecated and removed from documentation and thus may be removed in the
//! future by a SemVer minor update.

#![cfg(feature = "rand09")]
#![deprecated(since = "3.6.0", note = "use a newer version of `rand` crate")]
#![doc(hidden)]

use super::{Generator, RandSource, StdSystemTime};
use rand_core09::RngCore;

/// An adapter that implements [`RandSource`] for [`RngCore`] types.
#[derive(Clone, Debug, Default)]
pub struct Adapter<T>(/** The wrapped [`RngCore`] type. */ pub T);

impl<T: RngCore> RandSource for Adapter<T> {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
}

#[doc(hidden)]
impl<T: RngCore> Generator<Adapter<T>> {
    /// Creates a generator object with a specified random number generator that implements
    /// [`RngCore`] from `rand` (v0.9) crate. The specified random number generator should be
    /// cryptographically strong and securely seeded.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut g = scru128::Generator::with_rand09(rand::rng());
    /// println!("{}", g.generate());
    /// ```
    pub const fn with_rand09(rng: T) -> Self {
        Self::with_rand_and_time_sources(Adapter(rng), StdSystemTime)
    }
}
