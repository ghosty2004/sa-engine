//! Deterministic input generation. Case `i` of a run with seed `s` depends only on `(s, i)`, so any case
//! can be regenerated alone (`--seed s --case i`).

/// SplitMix64.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    /// The generator for case `i` of a run with `seed`.
    pub fn for_case(seed: u64, i: u64) -> Rng {
        let mut r = Rng(seed ^ i.wrapping_mul(0xD1B5_4A32_D192_ED03));
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in [0, n).
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u32() as u64 * n as u64) >> 32) as u32
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }

    /// Uniform in [-r, r] (f32 rounding of an f64 draw).
    pub fn uniform(&mut self, r: f32) -> f32 {
        let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        ((u * 2.0 - 1.0) * r as f64) as f32
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
}

/// Edge values every float argument is tested with: signed zeros, NaN payloads (quiet and signalling,
/// both signs), subnormals, infinities, extremes, and the angle boundaries the game's math uses.
pub fn edge_f32() -> Vec<f32> {
    let pi = std::f32::consts::PI;
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        f32::from_bits(1), // min subnormal
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF), // max subnormal
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_0000), // default QNaN
        f32::from_bits(0xFFC0_0000), // indefinite
        f32::from_bits(0x7FC0_1234), // QNaN with payload
        f32::from_bits(0xFFFF_FFFF),
        f32::from_bits(0x7F80_0001), // SNaN
        f32::from_bits(0xFFA0_0000), // SNaN, negative
        1e-30,
        -1e-30,
        1e30,
        -1e30,
        1e10,
        16_777_216.0,
        8_589_934_592.0, // 2^33
        pi,
        -pi,
        2.0 * pi,
        -2.0 * pi,
        pi / 2.0,
        -pi / 2.0,
        1.5 * pi,
        25.0,
        -25.0,
        180.0,
        -180.0,
        360.0,
        -360.0,
        0.0001,
    ];
    // Neighbours of the boundaries.
    for b in [pi, -pi, 2.0 * pi, 25.0, -25.0, 180.0, -180.0] {
        v.push(f32::from_bits(b.to_bits() + 1));
        v.push(f32::from_bits(b.to_bits() - 1));
    }
    v
}

/// A float argument: a mix of uniform bit patterns, realistic ranges, tiny values, edge values and
/// near-boundary values.
pub fn f32_any(r: &mut Rng) -> f32 {
    match r.below(100) {
        0..=24 => f32::from_bits(r.next_u32()),
        25..=59 => {
            let range = r.pick(&[1.0f32, 4.0, 10.0, 100.0, 1000.0, 1e4, 1e6]);
            r.uniform(range)
        }
        60..=69 => {
            // small magnitudes down into the subnormals
            let e = r.below(160) as i32 - 150;
            let s = if r.chance(50) { -1.0 } else { 1.0 };
            (s * (1.0 + r.uniform(1.0).abs() as f64) * 2f64.powi(e)) as f32
        }
        70..=84 => r.pick(&edge_f32()),
        _ => {
            let b = r.pick(&edge_f32());
            if b.is_nan() || b.is_infinite() {
                return b;
            }
            let d = r.below(9) as i32 - 4;
            f32::from_bits((b.to_bits() as i32).wrapping_add(d) as u32)
        }
    }
}

/// `n` float arguments, sometimes correlated (equal, negated, or a few ulps apart), which hits the
/// branches on `x == y`, `|x| == |y|` and zero differences.
pub fn f32_tuple(r: &mut Rng, n: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..n).map(|_| f32_any(r)).collect();
    if n >= 2 && r.chance(20) {
        let i = r.below(n as u32) as usize;
        let j = r.below(n as u32) as usize;
        v[j] = match r.below(3) {
            0 => v[i],
            1 => -v[i],
            _ => f32::from_bits(v[i].to_bits().wrapping_add(r.below(5).wrapping_sub(2))),
        };
    }
    v
}

/// The edge cases for `n` float arguments: the full cartesian product of [`edge_f32`] when it has at most
/// `cap` elements, else every edge value in every position with the other arguments drawn from the
/// edge set by a fixed generator.
pub fn edge_tuples(n: usize, cap: usize) -> Vec<Vec<f32>> {
    let e = edge_f32();
    let total = e.len().checked_pow(n as u32).unwrap_or(usize::MAX);
    if total <= cap {
        let mut out = vec![vec![]];
        for _ in 0..n {
            out = out
                .into_iter()
                .flat_map(|p| e.iter().map(move |x| [p.clone(), vec![*x]].concat()))
                .collect();
        }
        return out;
    }
    let mut r = Rng::new(0xED6E);
    let mut out = Vec::new();
    while out.len() < cap {
        for pos in 0..n {
            for &x in &e {
                let mut t: Vec<f32> = (0..n).map(|_| r.pick(&e)).collect();
                t[pos] = x;
                out.push(t);
            }
        }
    }
    out.truncate(cap);
    out
}
