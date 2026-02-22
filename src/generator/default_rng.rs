use std::error;

use rand09::{RngCore as _, rngs::OsRng, rngs::ReseedingRng};

use super::{DefaultRng, RandSource};

impl RandSource for DefaultRng {
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
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
        Self::try_new().expect("could not initialize DefaultRng")
    }
}

impl DefaultRng {
    pub(crate) fn try_new() -> Result<Self, impl error::Error> {
        ReseedingRng::new(1024 * 64, OsRng).map(|inner| Self {
            _private: (),
            inner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DefaultRng, RandSource};

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
