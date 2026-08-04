//! .NET-compatible `System.Random` — time-seeded, three independent streams
//! for the battle engine (§6.0) plus fresh instances for quests/daily.
//!
//! The .NET `Random` PRNG uses a 56-element int array with a specific
//! seeding and generation algorithm. We replicate its exact output sequence
//! so that the Rust port is RNG-parity with the C# server.

/// A .NET-compatible `System.Random` implementation.
///
/// Replicates the exact PRNG algorithm from .NET Framework / .NET Core
/// (Knuth subtractive generator with a 56-element state array).
#[derive(Debug, Clone)]
pub struct DotNetRandom {
    seed_array: [i32; 56],
    inext: i32,
    inextp: i32,
}

impl DotNetRandom {
    /// Create a new RNG with the given seed (mirrors `new Random(seed)`).
    pub fn new(seed: i32) -> Self {
        let mut r = DotNetRandom {
            seed_array: [0i32; 56],
            inext: 0,
            inextp: 21,
        };
        // .NET seed algorithm
        let subtraction = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut mj = 161803398i32.wrapping_sub(subtraction);
        r.seed_array[55] = mj;
        let mut mk = 1i32;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            r.seed_array[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(i32::MAX);
            }
            mj = r.seed_array[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                r.seed_array[i] = r.seed_array[i].wrapping_sub(r.seed_array[1 + (i + 30) % 55]);
                if r.seed_array[i] < 0 {
                    r.seed_array[i] = r.seed_array[i].wrapping_add(i32::MAX);
                }
            }
        }
        r
    }

    /// Create a time-seeded RNG (mirrors `new Random()` — uses system ticks).
    pub fn time_seeded() -> Self {
        // .NET uses Environment.TickCount as seed for the default constructor.
        // We use the current time in milliseconds modulo i32::MAX.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let ticks = (now.as_millis() % (i32::MAX as u128)) as i32;
        Self::new(ticks)
    }

    /// Internal sample: returns a value in [0, i32::MAX).
    fn internal_sample(&mut self) -> i32 {
        let mut inext = self.inext;
        let mut inextp = self.inextp;
        inext += 1;
        if inext >= 56 {
            inext = 1;
        }
        inextp += 1;
        if inextp >= 56 {
            inextp = 1;
        }
        let mut ret =
            self.seed_array[inext as usize].wrapping_sub(self.seed_array[inextp as usize]);
        if ret == i32::MAX {
            ret -= 1;
        }
        if ret < 0 {
            ret = ret.wrapping_add(i32::MAX);
        }
        self.seed_array[inext as usize] = ret;
        self.inext = inext;
        self.inextp = inextp;
        ret
    }

    /// `Random.Next()` — returns a non-negative random integer.
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// `Random.Next(maxValue)` — returns a value in [0, maxValue).
    pub fn next_max(&mut self, max_value: i32) -> i32 {
        (self.internal_sample() as f64 * (1.0 / i32::MAX as f64) * max_value as f64) as i32
    }

    /// `Random.Next(minValue, maxValue)` — returns a value in [minValue, maxValue).
    pub fn next_range(&mut self, min_value: i32, max_value: i32) -> i32 {
        let range = (max_value as i64).wrapping_sub(min_value as i64);
        if range <= i32::MAX as i64 {
            ((self.internal_sample() as f64 * (1.0 / i32::MAX as f64)) * range as f64) as i32
                + min_value
        } else {
            let sample = self.internal_sample();
            let large = (sample as f64 * (1.0 / i32::MAX as f64)) * range as f64 + min_value as f64;
            large as i32
        }
    }
}

/// The three battle RNG streams (§6.0).
#[derive(Debug, Clone)]
pub struct BattleRng {
    /// Stream 0: drop rolls, skill pick, RandomizeArrayWithPercent.
    pub random_0: DotNetRandom,
    /// Stream 1: per-turn _Random tie-breaker, damage jitter Next(0,2).
    pub random_1: DotNetRandom,
    /// Stream 2: npc respawn coordinates.
    pub random_2: DotNetRandom,
}

impl BattleRng {
    /// Create three independent time-seeded streams.
    pub fn new() -> Self {
        Self {
            random_0: DotNetRandom::time_seeded(),
            random_1: DotNetRandom::time_seeded(),
            random_2: DotNetRandom::time_seeded(),
        }
    }

    /// Create with explicit seeds (for deterministic testing).
    pub fn with_seeds(s0: i32, s1: i32, s2: i32) -> Self {
        Self {
            random_0: DotNetRandom::new(s0),
            random_1: DotNetRandom::new(s1),
            random_2: DotNetRandom::new(s2),
        }
    }
}

/// `RandomizeArrayWithPercent(v1, v2, percent)` — §5.2 / §5.8.
/// Uses the given RNG stream. Returns v1 if the roll passes, v2 otherwise.
/// `percent` is clamped to [0, 100].
pub fn randomize_with_percent(rng: &mut DotNetRandom, v1: i32, v2: i32, percent: i32) -> i32 {
    let p = percent.clamp(0, 100);
    let roll = rng.next_range(1, 1000);
    if roll <= p * 10 {
        v1
    } else {
        v2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_seed() {
        let mut r1 = DotNetRandom::new(42);
        let mut r2 = DotNetRandom::new(42);
        for _ in 0..100 {
            assert_eq!(r1.next(), r2.next());
        }
    }

    #[test]
    fn next_range_bounds() {
        let mut r = DotNetRandom::new(123);
        for _ in 0..1000 {
            let v = r.next_range(5, 10);
            assert!(v >= 5 && v < 10, "value {} out of [5,10)", v);
        }
    }

    #[test]
    fn next_max_bounds() {
        let mut r = DotNetRandom::new(456);
        for _ in 0..1000 {
            let v = r.next_max(7);
            assert!(v >= 0 && v < 7, "value {} out of [0,7)", v);
        }
    }

    #[test]
    fn randomize_with_percent_always_passes_at_100() {
        let mut r = DotNetRandom::new(789);
        for _ in 0..100 {
            assert_eq!(randomize_with_percent(&mut r, 1, 0, 100), 1);
        }
    }

    #[test]
    fn randomize_with_percent_never_passes_at_0() {
        let mut r = DotNetRandom::new(321);
        for _ in 0..100 {
            assert_eq!(randomize_with_percent(&mut r, 1, 0, 0), 0);
        }
    }

    #[test]
    fn three_streams_independent() {
        let rng = BattleRng::with_seeds(1, 2, 3);
        // Each stream should produce different sequences
        let mut r0 = rng.random_0.clone();
        let mut r1 = rng.random_1.clone();
        let mut r2 = rng.random_2.clone();
        let v0 = r0.next();
        let v1 = r1.next();
        let v2 = r2.next();
        // Different seeds should give different first values
        assert_ne!(v0, v1);
        assert_ne!(v1, v2);
    }
}
