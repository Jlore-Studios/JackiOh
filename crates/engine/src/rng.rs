//! The only source of randomness in the engine (§10.7, §9.3). The cursor lives in GameState, so
//! (seed, cursor) reproduces a sequence exactly, in this process or any other.
//!
//! A bit-for-bit port of `packages/engine/src/rng.ts` (SURFACE §6.3): xmur3 over the seed's UTF-16
//! code units seeds mulberry32, and draw `n` is mulberry32 evaluated at `(seed_int, n)`, so a
//! generator is nothing but its seed and its cursor. `Math.imul` is `wrapping_mul` on `u32`, `>>>`
//! is `>>` on `u32`, and the `a + Math.imul(…)` sums are `wrapping_add` (their `>>> 0` / `^=`
//! reduce them mod 2^32, which is exactly what wrapping does to the same bit patterns).

/// xmur3: string seed to a 32-bit integer stream, used once to seed mulberry32.
fn seed_to_int(seed: &str) -> u32 {
    let units: Vec<u16> = seed.encode_utf16().collect();
    // JS `seed.length` counts UTF-16 code units.
    let mut h: u32 = 1_779_033_703_u32 ^ (units.len() as u32);
    for unit in units {
        h = (h ^ u32::from(unit)).wrapping_mul(3_432_918_353);
        // `(h << 13) | (h >>> 19)`
        h = h.rotate_left(13);
    }
    h = (h ^ (h >> 16)).wrapping_mul(2_246_822_507);
    h = (h ^ (h >> 13)).wrapping_mul(3_266_489_909);
    h ^ (h >> 16)
}

/// mulberry32, stepped `n` times from the seed.
fn value_at(seed_int: u32, n: u32) -> f64 {
    let mut a: u32 = seed_int.wrapping_add(n.wrapping_mul(0x6d2b_79f5));
    a = (a ^ (a >> 15)).wrapping_mul(a | 1);
    a ^= a.wrapping_add((a ^ (a >> 7)).wrapping_mul(a | 61));
    f64::from(a ^ (a >> 14)) / 4_294_967_296.0
}

/// A seeded generator: TS's `Rng`. It holds no state but its seed and its cursor, so cloning it
/// forks the sequence and `Rng::new(seed, rng.cursor())` resumes it anywhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    seed_int: u32,
    cursor: u32,
}

impl Rng {
    /// TS `createRng(seed, cursor)`.
    pub fn new(seed: &str, cursor: u32) -> Rng {
        Rng { seed_int: seed_to_int(seed), cursor }
    }

    /// Draws taken so far; store this in state and resume from it.
    pub fn cursor(&self) -> u32 {
        self.cursor
    }

    /// A float in [0, 1).
    pub fn next(&mut self) -> f64 {
        let value = value_at(self.seed_int, self.cursor);
        self.cursor += 1;
        value
    }

    /// An integer in [0, n); `n <= 0` is 0 and draws nothing.
    pub fn int(&mut self, n: i32) -> i32 {
        if n <= 0 {
            return 0;
        }
        (self.next() * f64::from(n)).floor() as i32 % n
    }

    /// One element, uniformly; an empty list is `None` and draws nothing.
    pub fn pick<'a, T>(&mut self, list: &'a [T]) -> Option<&'a T> {
        if list.is_empty() {
            return None;
        }
        let at = self.int(list.len() as i32);
        list.get(at as usize)
    }

    /// Fisher-Yates, top down, so the result depends only on (seed, cursor).
    pub fn shuffle<T: Clone>(&mut self, list: &[T]) -> Vec<T> {
        let mut out = list.to_vec();
        for i in (1..out.len()).rev() {
            let j = self.int(i as i32 + 1) as usize;
            out.swap(i, j);
        }
        out
    }

    pub fn coin(&mut self) -> bool {
        self.next() < 0.5
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }

    /// Lucky X (§6.1): roll X extra times and keep the best per `better`. `roll` is handed the
    /// generator, so a roll that draws needs no second borrow of it.
    pub fn lucky<T>(&mut self, x: i32, mut roll: impl FnMut(&mut Rng) -> T, better: impl Fn(T, T) -> T) -> T {
        let mut best = roll(self);
        for _ in 0..x.max(0) {
            let next = roll(self);
            best = better(best, next);
        }
        best
    }
}

/// TS `createRng(seed, cursor = 0)`, by its TS name (SURFACE §4.2): the same as `Rng::new`.
pub fn create_rng(seed: &str, cursor: u32) -> Rng {
    Rng::new(seed, cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SURFACE §6.3 (V3): the first ten draws TS prints for two seeds, pinned as exact `f64`s
    /// (`pnpm exec tsx -e '… createRng(s) … r.next() …'` at the plan's commit).
    #[test]
    fn first_ten_draws_match_typescript() {
        let golden = [
            0.6569062923081219,
            0.9212453747168183,
            0.48674044315703213,
            0.8353869873099029,
            0.1645236941985786,
            0.9309415211901069,
            0.44464601506479084,
            0.746172602288425,
            0.2840092333499342,
            0.3314517338294536,
        ];
        let fuzz = [
            0.14485335885547101,
            0.2014636874664575,
            0.7321288040839136,
            0.5006692679598927,
            0.01700649783015251,
            0.9128239972051233,
            0.10607194644398987,
            0.1391449123620987,
            0.9416085947304964,
            0.5141714666970074,
        ];
        let mut a = Rng::new("golden", 0);
        for expected in golden {
            assert_eq!(a.next(), expected);
        }
        assert_eq!(a.cursor(), 10);
        let mut b = Rng::new("jackioh-fuzz-1", 0);
        for expected in fuzz {
            assert_eq!(b.next(), expected);
        }
    }

    /// A generator resumed at a cursor continues the same sequence.
    #[test]
    fn a_cursor_resumes_the_sequence() {
        let mut from_start = Rng::new("golden", 0);
        for _ in 0..4 {
            from_start.next();
        }
        let mut resumed = Rng::new("golden", 4);
        assert_eq!(resumed.next(), from_start.next());
    }

    /// TS: `int(10), int(7), int(1), int(0), int(100)` on "golden" is [6, 6, 0, 0, 83] at cursor 4
    /// (`int(0)` draws nothing); `shuffle([1..8])` on "jackioh-fuzz-1" at cursor 5 is
    /// [2, 6, 4, 3, 5, 7, 1, 8] at cursor 12.
    #[test]
    fn int_and_shuffle_match_typescript() {
        let mut q = Rng::new("golden", 0);
        let ints = [q.int(10), q.int(7), q.int(1), q.int(0), q.int(100)];
        assert_eq!(ints, [6, 6, 0, 0, 83]);
        assert_eq!(q.cursor(), 4);
        let mut w = Rng::new("jackioh-fuzz-1", 5);
        assert_eq!(w.shuffle(&[1, 2, 3, 4, 5, 6, 7, 8]), vec![2, 6, 4, 3, 5, 7, 1, 8]);
        assert_eq!(w.cursor(), 12);
    }

    /// The seed is read as UTF-16 code units, as JS reads it: "xé♥😀" (the emoji is two units).
    #[test]
    fn seeds_hash_utf16_code_units() {
        let mut u = Rng::new("x\u{e9}\u{2665}\u{1f600}", 0);
        assert_eq!([u.next(), u.next()], [0.4544040390755981, 0.2282192155253142]);
    }

    #[test]
    fn empty_pick_draws_nothing_and_lucky_rolls_one_plus_x() {
        let mut r = Rng::new("golden", 0);
        let empty: [i32; 0] = [];
        assert_eq!(r.pick(&empty), None);
        assert_eq!(r.cursor(), 0);
        let best = r.lucky(2, |g| g.int(100), |a, b| a.max(b));
        assert_eq!(r.cursor(), 3);
        assert_eq!(best, 92);
    }
}
