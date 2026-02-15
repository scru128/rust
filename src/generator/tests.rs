use super::*;

mod generate_or_abort;
mod generate_or_reset;

#[cfg(not(feature = "default_rng"))]
impl Scru128Generator {
    pub(crate) fn new() -> Scru128Generator<impl RandSource, impl TimeSource> {
        Scru128Generator::with_rand_and_time_sources(new_rand_source(), new_time_source())
    }
}

fn new_rand_source() -> impl RandSource {
    #[cfg(feature = "default_rng")]
    return DefaultRng::default();

    #[cfg(not(feature = "default_rng"))]
    {
        use rand09::{rngs::StdRng, RngCore as _, SeedableRng as _};

        struct MockRandSource(StdRng);
        impl RandSource for MockRandSource {
            fn next_u32(&mut self) -> u32 {
                self.0.next_u32()
            }
        }

        let local_var = 0u32;
        let mut addr_as_seed = (&local_var as *const u32) as u64;
        addr_as_seed ^= new_time_source().unix_ts_ms();
        MockRandSource(StdRng::seed_from_u64(addr_as_seed))
    }
}

fn new_time_source() -> impl TimeSource {
    #[cfg(feature = "std")]
    return StdSystemTime;

    #[cfg(not(feature = "std"))]
    {
        struct MockTimeSource(u64);
        impl TimeSource for MockTimeSource {
            fn unix_ts_ms(&mut self) -> u64 {
                self.0 += 8;
                self.0
            }
        }
        MockTimeSource(0x0123_4567_89abu64)
    }
}

/// Reads timestamp from time source
#[test]
fn reads_timestamp_from_time_source() {
    use std::cell;
    struct PeekableTimeSource<'a, T>(&'a cell::Cell<u64>, T);
    impl<T: TimeSource> TimeSource for PeekableTimeSource<'_, T> {
        fn unix_ts_ms(&mut self) -> u64 {
            self.0.set(self.1.unix_ts_ms());
            self.0.get()
        }
    }

    let ts = cell::Cell::default();
    let time_source = PeekableTimeSource(&ts, new_time_source());
    let mut g = Scru128Generator::with_rand_and_time_sources(new_rand_source(), time_source);

    assert_eq!(g.generate().timestamp(), ts.get());
    assert_eq!(g.generate().timestamp(), ts.get());
    assert_eq!(g.generate_or_abort().unwrap().timestamp(), ts.get());
    assert_eq!(g.generate_or_abort().unwrap().timestamp(), ts.get());
    assert_eq!(g.generate().timestamp(), ts.get());
    assert_eq!(g.generate_or_abort().unwrap().timestamp(), ts.get());
    assert_eq!(g.generate().timestamp(), ts.get());
    assert_eq!(g.generate_or_abort().unwrap().timestamp(), ts.get());
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
