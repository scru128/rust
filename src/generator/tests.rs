use super::*;
use std::cell;

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
        use rand09::{RngCore as _, SeedableRng as _, rngs::StdRng};

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

/// Generator methods handle clock rollback according to their specifications
#[test]
fn handle_clock_rollback() {
    const DEFAULT_ROLLBACK_ALLOWANCE: u64 = 10_000;

    struct CellTimeSource<'a>(&'a cell::Cell<u64>);

    impl TimeSource for CellTimeSource<'_> {
        fn unix_ts_ms(&mut self) -> u64 {
            self.0.get()
        }
    }

    for rollback_allowance in [DEFAULT_ROLLBACK_ALLOWANCE, 5_000, 20_000] {
        let ts = cell::Cell::new(0);
        let [mut g0, mut g1, mut g2, mut g3, mut g4, mut g5] = [
            Scru128Generator::with_rand_and_time_sources(new_rand_source(), CellTimeSource(&ts)),
            Scru128Generator::with_rand_and_time_sources(new_rand_source(), CellTimeSource(&ts)),
            Scru128Generator::with_rand_and_time_sources(new_rand_source(), CellTimeSource(&ts)),
            Scru128Generator::with_rand_and_time_sources(new_rand_source(), CellTimeSource(&ts)),
            Scru128Generator::with_rand_and_time_sources(new_rand_source(), CellTimeSource(&ts)),
            Scru128Generator::with_rand_and_time_sources(new_rand_source(), CellTimeSource(&ts)),
        ];

        if rollback_allowance != DEFAULT_ROLLBACK_ALLOWANCE {
            g0.set_rollback_allowance(rollback_allowance);
            g1.set_rollback_allowance(rollback_allowance);
            g2.set_rollback_allowance(rollback_allowance);
            g3.set_rollback_allowance(rollback_allowance);
        }

        #[allow(deprecated)]
        let methods: [(&mut dyn FnMut() -> Option<Scru128Id>, bool); 6] = [
            (&mut || Some(g0.generate()), true),
            (&mut || g1.generate_or_abort(), false),
            (&mut || Some(g2.generate_or_reset_with_ts(ts.get())), true),
            (&mut || g3.generate_or_abort_with_ts(ts.get()), false),
            (
                &mut || Some(g4.generate_or_reset_core(ts.get(), rollback_allowance)),
                true,
            ),
            (
                &mut || g5.generate_or_abort_core(ts.get(), rollback_allowance),
                false,
            ),
        ];

        for (generate, is_reset) in methods {
            let mut ts_base = new_time_source().unix_ts_ms();

            ts.set(ts_base);
            let mut prev = generate().unwrap();
            assert_eq!(prev.timestamp(), ts_base);

            // generates increasing IDs with constant timestamp
            for _ in 0..50 {
                let curr = generate().unwrap();
                assert!(prev < curr);
                assert!(curr.timestamp() >= ts_base);
                prev = curr;
            }

            // generates increasing IDs with decreasing timestamp
            for i in 0..50_000u64 {
                ts.set(ts_base - i.min(rollback_allowance - 1));
                let curr = generate().unwrap();
                assert!(prev < curr);
                assert!(curr.timestamp() >= ts_base);
                prev = curr;
            }

            // reset generator state
            ts_base += rollback_allowance * 4;
            ts.set(ts_base);
            prev = generate().unwrap();
            assert_eq!(prev.timestamp(), ts_base);

            ts.set(ts_base - rollback_allowance);
            let mut curr = generate();
            assert!(prev < curr.unwrap());
            assert!(curr.unwrap().timestamp() >= ts_base);

            if is_reset {
                // breaks increasing order if timestamp goes backwards a lot
                prev = curr.unwrap();
                ts.set(ts_base - rollback_allowance - 1);
                curr = generate();
                assert!(prev > curr.unwrap());
                assert_eq!(curr.unwrap().timestamp(), ts_base - rollback_allowance - 1);

                prev = curr.unwrap();
                ts.set(ts_base - rollback_allowance - 2);
                curr = generate();
                assert!(prev < curr.unwrap());
                assert!(curr.unwrap().timestamp() >= ts_base - rollback_allowance - 1);
            } else {
                // returns None if timestamp goes backwards a lot
                ts.set(ts_base - rollback_allowance - 1);
                curr = generate();
                assert!(curr.is_none());

                ts.set(ts_base - rollback_allowance - 2);
                curr = generate();
                assert!(curr.is_none());
            }
        }
    }
}

/// _core methods do not change generator-level rollback allowance
#[test]
#[allow(deprecated)]
fn core_fns_do_not_change_rollback_allowance() {
    let ts = new_time_source().unix_ts_ms();

    let mut g = Scru128Generator::new();
    g.set_rollback_allowance(100);
    assert_eq!(g.rollback_allowance, 100);

    g.generate_or_reset_core(ts, 1_000);
    assert_eq!(g.rollback_allowance, 100);

    g.generate_or_abort_core(ts, 1_000);
    assert_eq!(g.rollback_allowance, 100);
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
