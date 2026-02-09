//! Integration with `rand` (v0.8) crate.

#![cfg(feature = "rand08")]

use super::{RandSource, Scru128Generator, StdSystemTime};
use rand_core06::RngCore;

/// An adapter that implements [`RandSource`] for [`RngCore`] types.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Adapter<T>(/** The wrapped [`RngCore`] type. */ pub T);

impl<T: RngCore> RandSource for Adapter<T> {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
}

impl<T: RngCore> Scru128Generator<Adapter<T>> {
    /// Creates a generator object with a specified random number generator that implements
    /// [`RngCore`] from `rand` (v0.8) crate. The specified random number generator should be
    /// cryptographically strong and securely seeded.
    pub const fn with_rand08(rng: T) -> Self {
        Self::with_rand_and_time_sources(Adapter(rng), StdSystemTime)
    }
}
