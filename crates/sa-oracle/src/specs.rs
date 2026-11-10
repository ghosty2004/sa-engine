//! L1 function specs: how to call the original, how to generate inputs, what the Rust port says, and the
//! explicit route (L1 under Unicorn, or L2 because Unicorn's x87 is not faithful for the instructions the
//! function uses; see `docs/re/l1-harness.md`).

use crate::call::*;
use crate::generate::{self, Rng};
use sa_math::x87::X87;
use std::sync::Arc;

/// Raw argument words (f32 bits, flags). Their meaning is per spec.
pub type Inputs = Vec<u32>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetKind {
    /// float in ST0 (x87 stack depth 1 after return)
    St0,
    /// bool in AL (x87 stack empty)
    Al,
    /// 32-bit integer or pointer in EAX (x87 stack empty)
    Eax,
    /// nothing (x87 stack empty)
    Void,
}

/// Where a function's parity is established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// L1 under Unicorn is authoritative.
    L1,
    /// Unicorn is not faithful for this function (reason); its verdict does not count. Parity comes from
    /// L2 (in-process, INF-8) and, until then, from the native backend (real CPU under Wine).
    L2 { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpRet {
    St0 { f32: u32, ext: [u8; 10] },
    Al(u8),
    Eax(u32),
    Void,
}

/// What the Rust port says the original does. (The scratch image is kept inline: tests construct it
/// directly from `*scratch`.)
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    Returns {
        ret: ExpRet,
        scratch: [u8; SCRATCH_LEN],
    },
    /// The original never returns (it spins); the oracle must exhaust its budget.
    Hangs,
}

pub type RustFn = Arc<dyn Fn(&Inputs, &[u8; SCRATCH_LEN]) -> Expect + Send + Sync>;

#[derive(Clone)]
pub struct Spec {
    /// CLI name
    pub key: &'static str,
    pub name: &'static str,
    pub addr: u32,
    pub ret: RetKind,
    /// Also compare ST0's full 80 bits (else only the f32 a caller's `fstp dword` stores).
    pub st0_exact: bool,
    pub route: Route,
    /// Callees the function may call (by catalogued start).
    pub callees: &'static [u32],
    pub budget: u64,
    /// Number of leading inputs that are f32 (shrunk as floats).
    pub floats: usize,
    pub generate: GenFn,
    pub edges: EdgesFn,
    pub call: CallFn,
    pub rust: RustFn,
    /// Scratch byte ranges (offset, length) not compared: outputs the original fills from uninitialised
    /// stack memory (documented per spec, ledger "original UB").
    pub ignore: Vec<(usize, usize)>,
    /// Byte patches applied to the image for this spec (both backends), e.g. a `rand()` stub that
    /// returns a value from scratch memory so that the random input becomes an L1 input.
    pub patches: Vec<(u32, Vec<u8>)>,
    /// Writable image ranges [lo, hi) the function may read (Unicorn guard rail exception), with the
    /// values the patches give them.
    pub reads: Vec<(u32, u32)>,
    /// Caller-saved registers the original returns unchanged and some original caller relies on
    /// (`sa_core::abi::PRESERVE_ECX | PRESERVE_EDX`; VER-2, verifier finding V-9). The runner loads
    /// each with a per-case value that is not an argument, requires the original to return it
    /// unchanged and the port's hook registration to declare it (`#[hook(preserve = ...)]`).
    pub preserve: u8,
}

pub type GenFn = Arc<dyn Fn(&mut Rng) -> Inputs + Send + Sync>;
pub type EdgesFn = Arc<dyn Fn() -> Vec<Inputs> + Send + Sync>;
pub type CallFn = Arc<dyn Fn(&Inputs) -> Call + Send + Sync>;

fn f(x: u32) -> f32 {
    f32::from_bits(x)
}

fn bits(v: Vec<f32>) -> Inputs {
    v.into_iter().map(f32::to_bits).collect()
}

/// Scratch fill pattern (makes stray or missing writes visible).
pub const FILL: u8 = 0xA5;

fn st0(v: X87) -> ExpRet {
    ExpRet::St0 {
        f32: v.to_f32().to_bits(),
        ext: v.to_ext(),
    }
}

fn ret_st0(v: Option<X87>, s: &[u8; SCRATCH_LEN]) -> Expect {
    match v {
        Some(v) => Expect::Returns {
            ret: st0(v),
            scratch: *s,
        },
        None => Expect::Hangs,
    }
}

fn cdecl_f32(addr: u32, inp: &Inputs, n: usize) -> Call {
    let args: Vec<Arg> = inp[..n].iter().map(|&x| Arg::F32(f(x))).collect();
    let mut c = Call::new(addr, Conv::Cdecl, &args);
    c.scratch = [FILL; SCRATCH_LEN];
    c
}

fn edges_n(n: usize) -> Vec<Inputs> {
    generate::edge_tuples(n, 250_000).into_iter().map(bits).collect()
}

// ---------------------------------------------------------------- CGeneral::LimitAngle ----------

/// Values that take more than ~50k loop iterations in the original (|x| > 2^24) are not generated, only
/// the fixed points (hangs) among the edge values; see the l1-harness doc.
fn limit_angle_gen(r: &mut Rng) -> Inputs {
    let mut x = generate::f32_any(r);
    if x.is_finite() && x.abs() > 16_777_216.0 {
        let range = r.pick(&[1e3, 1e4, 1e5]);
        x = r.uniform(range);
    }
    vec![x.to_bits()]
}

/// Edge values within the generated domain plus the fixed points (where the original hangs).
fn limit_angle_edges() -> Vec<Inputs> {
    generate::edge_f32()
        .into_iter()
        .filter(|x| {
            !x.is_finite() || x.abs() <= 16_777_216.0 || sa_math::general::limit_angle_checked(*x).is_none()
        })
        .map(|x| vec![x.to_bits()])
        .collect()
}

pub fn limit_angle() -> Spec {
    Spec {
        preserve: 0,
        key: "LimitAngle",
        name: "CGeneral::LimitAngle",
        addr: 0x53CB00,
        ret: RetKind::St0,
        st0_exact: true,
        route: Route::L1,
        callees: &[],
        budget: 1_000_000,
        floats: 1,
        generate: Arc::new(limit_angle_gen),
        edges: Arc::new(limit_angle_edges),
        call: Arc::new(|i| cdecl_f32(0x53CB00, i, 1)),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| ret_st0(sa_math::general::limit_angle_checked(f(i[0])), s)),
    }
}

// ---------------------------------------------------------------- CGeneral::LimitRadianAngle ----

pub fn limit_radian_angle() -> Spec {
    Spec {
        preserve: 0,
        key: "LimitRadianAngle",
        name: "CGeneral::LimitRadianAngle",
        addr: 0x53CB50,
        ret: RetKind::St0,
        st0_exact: true,
        route: Route::L1,
        callees: &[],
        budget: 100_000,
        floats: 1,
        generate: Arc::new(|r| bits(generate::f32_tuple(r, 1))),
        edges: Arc::new(|| edges_n(1)),
        call: Arc::new(|i| cdecl_f32(0x53CB50, i, 1)),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| ret_st0(Some(sa_math::general::limit_radian_angle_x87(f(i[0]))), s)),
    }
}

// ---------------------------------------------------------------- CGeneral::GetATanOfXY ---------

pub const FPATAN_REASON: &str = "FPATAN: Unicorn (QEMU 5.0.1 helper_fpatan) computes it with the host's double atan2, \
     53-bit instead of the CPU's 64-bit result (measured, docs/re/l1-harness.md)";

pub fn get_atan_of_xy() -> Spec {
    Spec {
        preserve: 0,
        key: "GetATanOfXY",
        name: "CGeneral::GetATanOfXY",
        addr: 0x53CC70,
        ret: RetKind::St0,
        // One path returns the raw FPATAN value (64-bit significand); the port has 53 bits there.
        st0_exact: false,
        route: Route::L2 {
            reason: FPATAN_REASON,
        },
        callees: &[],
        budget: 10_000,
        floats: 2,
        generate: Arc::new(|r| bits(generate::f32_tuple(r, 2))),
        edges: Arc::new(|| edges_n(2)),
        call: Arc::new(|i| cdecl_f32(0x53CC70, i, 2)),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| ret_st0(Some(sa_math::general::get_atan_of_xy_x87(f(i[0]), f(i[1]))), s)),
    }
}

// ---------------------------------------------------------------- GetRadianAngleBetweenPoints ---

pub fn get_radian_angle_between_points() -> Spec {
    Spec {
        preserve: 0,
        key: "GetRadianAngleBetweenPoints",
        name: "CGeneral::GetRadianAngleBetweenPoints",
        addr: 0x53CBE0,
        ret: RetKind::St0,
        // Every path ends with a PC=24 op, so ST0 has 24 bits.
        st0_exact: true,
        route: Route::L2 {
            reason: FPATAN_REASON,
        },
        callees: &[],
        budget: 10_000,
        floats: 4,
        generate: Arc::new(|r| bits(generate::f32_tuple(r, 4))),
        edges: Arc::new(|| edges_n(4)),
        call: Arc::new(|i| cdecl_f32(0x53CBE0, i, 4)),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| {
            let v = sa_math::general::get_radian_angle_between_points_x87(f(i[0]), f(i[1]), f(i[2]), f(i[3]));
            ret_st0(Some(v), s)
        }),
    }
}

// ---------------------------------------------------------------- CGeneral::SolveQuadratic ------

/// `&x1` and `&x2` in scratch; input 3 = 1 makes them alias.
pub const QX1: u32 = SCRATCH + 0x10;
pub const QX2: u32 = SCRATCH + 0x20;

fn quadratic_call(i: &Inputs) -> Call {
    let x2 = if i[3] != 0 { QX1 } else { QX2 };
    let args = [
        Arg::F32(f(i[0])),
        Arg::F32(f(i[1])),
        Arg::F32(f(i[2])),
        Arg::Ptr(QX1),
        Arg::Ptr(x2),
    ];
    let mut c = Call::new(0x53CE30, Conv::Cdecl, &args);
    c.scratch = [FILL; SCRATCH_LEN];
    c
}

/// Expected outcome of SolveQuadratic from a port's `(x1, x2)` (shared with the mutation smoke test).
pub fn quadratic_expect(i: &Inputs, s: &[u8; SCRATCH_LEN], roots: Option<(f32, f32)>) -> Expect {
    let mut s = *s;
    let al = match roots {
        None => 0,
        Some((x1, x2)) => {
            let p2 = (if i[3] != 0 { QX1 } else { QX2 } - SCRATCH) as usize;
            let p1 = (QX1 - SCRATCH) as usize;
            // the original stores *x2 first, then *x1
            s[p2..p2 + 4].copy_from_slice(&x2.to_le_bytes());
            s[p1..p1 + 4].copy_from_slice(&x1.to_le_bytes());
            1
        }
    };
    Expect::Returns {
        ret: ExpRet::Al(al),
        scratch: s,
    }
}

fn quadratic_gen(r: &mut Rng) -> Inputs {
    let mut v = bits(generate::f32_tuple(r, 3));
    // Discriminant exactly zero is a branch boundary: b^2 = 4ac with small integers.
    if r.chance(5) {
        let b = (r.below(64) as f32) * if r.chance(50) { -1.0 } else { 1.0 };
        let a = r.pick(&[1.0f32, 0.25, 4.0, -1.0, 0.5]);
        v = bits(vec![a, b, b * b / (4.0 * a)]);
    }
    v.push(r.chance(10) as u32);
    v
}

pub fn solve_quadratic() -> Spec {
    Spec {
        preserve: 0,
        key: "SolveQuadratic",
        name: "CGeneral::SolveQuadratic",
        addr: 0x53CE30,
        ret: RetKind::Al,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 10_000,
        floats: 3,
        generate: Arc::new(quadratic_gen),
        edges: Arc::new(|| {
            let mut e = edges_n(3);
            for (k, v) in e.iter_mut().enumerate() {
                v.push((k % 7 == 0) as u32);
            }
            e
        }),
        call: Arc::new(quadratic_call),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| {
            quadratic_expect(i, s, sa_math::general::solve_quadratic(f(i[0]), f(i[1]), f(i[2])))
        }),
    }
}

// ---------------------------------------------------------------- CVector::Normalise ------------

/// `this` in scratch.
pub const VEC: u32 = SCRATCH + 0x10;

fn normalise_call(i: &Inputs) -> Call {
    let mut c = Call::new(0x59C910, Conv::Thiscall, &[Arg::Ptr(VEC)]);
    c.scratch = [FILL; SCRATCH_LEN];
    let o = (VEC - SCRATCH) as usize;
    for (k, v) in i.iter().take(3).enumerate() {
        c.scratch[o + 4 * k..o + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
    }
    c
}

pub fn normalise() -> Spec {
    Spec {
        preserve: 0,
        key: "CVectorNormalise",
        name: "CVector::Normalise",
        addr: 0x59C910,
        ret: RetKind::Void,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 10_000,
        floats: 3,
        generate: Arc::new(|r| bits(generate::f32_tuple(r, 3))),
        edges: Arc::new(|| edges_n(3)),
        call: Arc::new(normalise_call),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| {
            let v = sa_math::vector::normalise([f(i[0]), f(i[1]), f(i[2])]);
            let mut s = *s;
            let o = (VEC - SCRATCH) as usize;
            for (k, c) in v.iter().enumerate() {
                s[o + 4 * k..o + 4 * k + 4].copy_from_slice(&c.to_le_bytes());
            }
            Expect::Returns {
                ret: ExpRet::Void,
                scratch: s,
            }
        }),
    }
}

/// All L1 specs.
pub fn all() -> Vec<Spec> {
    vec![
        limit_angle(),
        limit_radian_angle(),
        get_atan_of_xy(),
        get_radian_angle_between_points(),
        solve_quadratic(),
        normalise(),
    ]
    .into_iter()
    .chain(crate::math_specs::all())
    .chain(crate::specs_core::all())
    .chain(crate::specs_core_util::all())
    .chain(crate::specs_stream_m::all())
    .chain(crate::specs_stream_a::all())
    .chain(crate::specs_stream_b::all())
    .chain(crate::specs_stream_c::all())
    .chain(crate::specs_coll_surface::all())
    .chain(crate::specs_coll_data::all())
    .chain(crate::specs_coll_prim::all())
    .chain(crate::specs_world_p1::all())
    .chain(crate::specs_phys::all())
    .chain(crate::specs_pop::all())
    .collect()
}

pub fn by_key(key: &str) -> Option<Spec> {
    all()
        .into_iter()
        .find(|s| s.key.eq_ignore_ascii_case(key) || s.name.eq_ignore_ascii_case(key))
}

/// The SolveQuadratic spec with another port in place of the real one (the INF-12 mutation smoke test
/// passes `sa_math::general::mutants::solve_quadratic`).
pub fn solve_quadratic_with(
    port: impl Fn(f32, f32, f32) -> Option<(f32, f32)> + Send + Sync + 'static,
) -> Spec {
    let mut s = solve_quadratic();
    s.rust = Arc::new(move |i, sc| quadratic_expect(i, sc, port(f(i[0]), f(i[1]), f(i[2]))));
    s
}
