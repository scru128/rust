use super::*;

mod generate_or_abort;
mod generate_or_reset;

#[cfg(not(feature = "default_rng"))]
impl Scru128Generator {
    pub(crate) fn new() -> Scru128Generator<impl RandSource, impl TimeSource> {
        let rand_source = {
            use rand09::{rngs::StdRng, RngCore as _, SeedableRng as _};

            struct MockRandSource(StdRng);
            impl RandSource for MockRandSource {
                fn next_u32(&mut self) -> u32 {
                    self.0.next_u32()
                }
            }

            let local_var = 0u32;
            let addr_as_seed = (&local_var as *const u32) as u64;
            #[cfg(feature = "std")]
            let addr_as_seed = addr_as_seed ^ StdSystemTime.unix_ts_ms();
            MockRandSource(StdRng::seed_from_u64(addr_as_seed))
        };

        #[cfg(feature = "std")]
        let time_source = StdSystemTime;

        #[cfg(not(feature = "std"))]
        let time_source = {
            struct MockTimeSource(u64);
            impl TimeSource for MockTimeSource {
                fn unix_ts_ms(&mut self) -> u64 {
                    self.0 += 1;
                    self.0
                }
            }
            MockTimeSource(0x0123_4567_89abu64)
        };

        Scru128Generator::with_rand_and_time_sources(rand_source, time_source)
    }
}

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
