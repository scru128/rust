#![cfg(any(feature = "default_rng", test))]

#[cfg(feature = "default_rng")]
use rand09::rngs::{OsRng, ReseedingRng};

#[cfg(not(feature = "default_rng"))]
use rand09::{SeedableRng as _, rngs::StdRng};

use super::{Scru128Generator, Scru128Rng};

/// The default random number generator used by [`Scru128Generator`].
///
/// Currently, `DefaultRng` uses [`ChaCha12Core`] that is initially seeded and subsequently
/// reseeded by [`OsRng`] every 64 kiB of random data using the [`ReseedingRng`] wrapper. It is the
/// same strategy as that employed by [`ThreadRng`]; see the docs of `rand` crate for a detailed
/// discussion on the strategy.
///
/// [`ChaCha12Core`]: rand_chacha::ChaCha12Core
/// [`ThreadRng`]: rand09::rngs::ThreadRng
#[derive(Clone, Debug)]
pub struct DefaultRng {
    #[cfg(feature = "default_rng")]
    inner: ReseedingRng<rand_chacha::ChaCha12Core, OsRng>,

    #[cfg(not(feature = "default_rng"))]
    inner: StdRng,
}

impl Scru128Rng for DefaultRng {
    fn next_u32(&mut self) -> u32 {
        rand09::RngCore::next_u32(&mut self.inner)
    }
}

impl Default for DefaultRng {
    /// Creates an instance of the default random number generator.
    ///
    /// # Panics
    ///
    /// Panics in the highly unlikely event where the operating system's random number generator
    /// failed to provide secure entropy.
    fn default() -> Self {
        Self {
            #[cfg(feature = "default_rng")]
            inner: ReseedingRng::new(1024 * 64, OsRng)
                .expect("scru128: could not initialize DefaultRng"),

            #[cfg(all(test, not(feature = "default_rng")))]
            inner: {
                let local_var = 0u32;
                let addr_as_seed = (&local_var as *const u32) as u64;
                StdRng::seed_from_u64(addr_as_seed)
            },
        }
    }
}

impl Scru128Generator<DefaultRng> {
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

        const N_LOOPS: usize = 1_000_000;
        for _ in 0..N_LOOPS {
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
        let margin = 4.417173 * (0.5 * 0.5 / N_LOOPS as f64).sqrt();
        assert!(
            counts
                .iter()
                .all(|e| (*e as f64 / N_LOOPS as f64 - 0.5).abs() < margin)
        );
        assert!(
            counts_xor
                .iter()
                .all(|e| (*e as f64 / N_LOOPS as f64 - 0.5).abs() < margin)
        );
    }
}
