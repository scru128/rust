use crate::identifier::{Identifier, MAX_COUNTER, MAX_PER_SEC_RANDOM};

use std::time::{SystemTime, UNIX_EPOCH};

use rand::prelude::*;

/// Unix time in milliseconds as at 2020-01-01 00:00:00+00:00.
const TIMESTAMP_EPOCH: u64 = 1577836800000;

/// Represents a SCRU128 ID generator.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Generator {
    ts_last_gen: u64,
    counter: u32,
    ts_last_sec: u64,
    per_sec_random: u32,
    rng: StdRng,
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator {
    pub fn new() -> Self {
        Self {
            ts_last_gen: 0,
            counter: 0,
            ts_last_sec: 0,
            per_sec_random: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// Generates a new SCRU128 ID object.
    pub fn generate(&mut self) -> Identifier {
        let mut ts_now = get_msec_unixts();

        // update timestamp and counter
        if ts_now > self.ts_last_gen {
            self.ts_last_gen = ts_now;
            self.counter = self.rng.gen::<u32>() & MAX_COUNTER;
        } else {
            self.counter += 1;
            if self.counter > MAX_COUNTER {
                #[cfg(feature = "log")]
                log::info!("counter limit reached; will wait until clock goes forward");
                let mut n_trials = 0;
                while ts_now >= self.ts_last_gen {
                    ts_now = get_msec_unixts();
                    n_trials += 1;
                    if n_trials > 1_000_000 {
                        #[cfg(feature = "log")]
                        log::warn!("reset state as clock did not go forward");
                        self.ts_last_sec = 0;
                        break;
                    }
                }
                self.ts_last_gen = ts_now;
                self.counter = self.rng.gen::<u32>() & MAX_COUNTER;
            }
        }

        // update per_sec_random
        if self.ts_last_gen - self.ts_last_sec > 1000 {
            self.ts_last_sec = self.ts_last_gen;
            self.per_sec_random = self.rng.gen::<u32>() & MAX_PER_SEC_RANDOM;
        }

        Identifier::from_field_values(
            self.ts_last_gen - TIMESTAMP_EPOCH,
            self.counter,
            self.per_sec_random,
            self.rng.gen(),
        )
    }
}

/// Returns the current unix time in milliseconds.
fn get_msec_unixts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock may have gone backwards")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::Generator;

    #[test]
    fn basic_examples() {
        let mut g = Generator::new();
        for _ in 0..4 {
            println!("{}", g.generate().to_string());
        }
    }

    /// Describes how to use Generator globally and thread-locally.
    #[test]
    fn thread_examples() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let g_shared = Arc::new(Mutex::new(Generator::new()));

        let mut hs = Vec::new();
        for i in 0..4 {
            let g_shared = Arc::clone(&g_shared);
            hs.push(thread::spawn(move || {
                let mut g_local = Generator::new();
                for _ in 0..4 {
                    println!(
                        "Shared generator: {}",
                        g_shared.lock().unwrap().generate().to_string()
                    );
                    println!(
                        "Thread-local generator {}: {}",
                        i,
                        g_local.generate().to_string(),
                    );
                }
            }));
        }

        for h in hs {
            let _ = h.join();
        }
    }
}
