//! PORT-RENDER: `CPointLights` (per-frame dynamic lights and the ground-height cache) and `CBrightLights`.
//! Ported from the 1.0 US bytes; gta-reversed `PointLights.cpp` as a reading aid.
//!
//! `CPointLight` (0x30 bytes, `aLights[32]` at 0xC3F0E0): +0 position, +0xC direction, +0x18 range,
//! +0x1C r, +0x20 g, +0x24 b, +0x28 attached entity, +0x2C type (u8), +0x2D fog type, +0x2E
//! generate extra shadows.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::world::common::{CAM_ENTITY, CAM_VECTOR};

use crate::common::{Fpu, k, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CPointLights::aCachedMapReads[32]` (ground z of the cached points)
    pub static CACHED_MAP_READS @ 0xC3F050: [f32; 32] = [0.0; 32];
    /// `CPointLights::NumLights`
    pub static NUM_LIGHTS @ 0xC3F0D0: i32 = 0;
    /// `CPointLights::NextCachedValue`
    pub static NEXT_CACHED_VALUE @ 0xC3F0D4: i32 = 0;
    /// `CPointLights::aLights[32]` (CPointLight, 0x30 bytes)
    pub static LIGHTS @ 0xC3F0E0: [u8; 32 * 0x30] = [0; 32 * 0x30];
    /// `CPointLights::aCachedMapReadPoints[32]` (CVector)
    pub static CACHED_MAP_READ_POINTS @ 0xC3F6E0: [f32; 32 * 3] = [0.0; 32 * 3];
    /// `CBrightLights::NumBrightLights`
    pub static NUM_BRIGHT_LIGHTS @ 0xC7C6FC: u32 = 0;
}

const LIGHTS_BASE: usize = 0xC3F0E0;
const MAX_LIGHTS: i32 = 32;

/// 1.0 (0x858624), 0.0 (0x858B50), 1/3 (0x859040), 15.0 (0x858B48), 0.75 (0x858F34), 4.0 (0x858B90),
/// 20.0 (0x858BA4)
const ONE: u32 = 0x3F80_0000;
const ZERO: u32 = 0x0000_0000;
const THIRD: u32 = 0x3EAA_AAAB;
const FIFTEEN: u32 = 0x4170_0000;
const P075: u32 = 0x3F40_0000;
const FOUR: u32 = 0x4080_0000;
const TWENTY: u32 = 0x41A0_0000;

orig! {
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings, bool
    /// vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0)
    pub fn process_vertical_line(p: GPtr<f32>, z: f32, cp: GPtr<u8>, e: GPtr<GPtr<u8>>, b: u32, v: u32, pe: u32, o: u32, d: u32, st: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

fn light(i: i32) -> GPtr<u8> {
    GPtr::from_addr(LIGHTS_BASE).byte_add(i as isize * 0x30)
}

/// The camera position the original reads: `[0xB6F03C] + 0x30` when set, else TheCamera +0x4.
fn cam_pos() -> GPtr<f32> {
    let m = CAM_ENTITY.get();
    if m.is_null() {
        CAM_VECTOR.ptr().cast()
    } else {
        m.byte_add(0x30).cast()
    }
}

/// `CPointLights::Init()` (0x6FFB40): cached reads 0, `NextCachedValue = 0`, every cached point (0, 0, 0).
#[hook(addr = 0x6FFB40, conv = cdecl, name = "CPointLights::Init", expect = "83 EC 0C 56 57")]
pub fn init() {
    CACHED_MAP_READS.set([0.0; 32]);
    NEXT_CACHED_VALUE.set(0);
    CACHED_MAP_READ_POINTS.set([0.0; 32 * 3]);
}

/// `CPointLights::GetLightMultiplier(const CVector* p)` (0x6FFE70, ST0): over the lights of types other
/// than 3 / 4 whose box `|d| < range` (per axis, against `-range` stored as f32) and distance `< range`
/// contain `p`: type 2 multiplies `mult` by `dist / range`, the others add `(1 - dist / range) * (r, g,
/// b) * 1/3` to `sum` (the y / z box tests compare the unrounded difference); returns `mult + sum` (mult starts 1, sum 0; the loop index is 16-bit).
#[hook(addr = 0x6FFE70, conv = cdecl, name = "CPointLights::GetLightMultiplier", expect = "D9 05 24 86 85 00")]
pub fn get_light_multiplier(p: GPtr<f32>) -> X87 {
    let mut f = Fpu::new();
    f.fld(k(ONE));
    f.fld(k(ZERO));
    let n = NUM_LIGHTS.get();
    let mut i: i16 = 0;
    if n > 0 {
        loop {
            let l = light(i32::from(i));
            let ty = crate::common::rd::<u8>(l, 0x2C);
            'light: {
                if ty == 3 || ty == 4 {
                    break 'light;
                }
                let rd = |o: isize| crate::common::rd::<f32>(l, o);
                let range = rd(0x18);
                let nrange = crate::common::f32_of(crate::common::x(range).fchs());
                let mut d = [0f32; 3];
                for (a, slot) in d.iter_mut().enumerate() {
                    f.fld(rd(a as isize * 4));
                    f.fsub_m(p.wrapping_index(a as i32).read());
                    // x: `fstp` then `fld` of the stored value; y / z: `fst`, the compare sees ST0
                    if a == 0 {
                        *slot = f.fstp();
                        f.fld(*slot);
                    } else {
                        *slot = f.fst();
                    }
                    f.fcomp_m(nrange);
                    if f.jne(0x41) {
                        break 'light;
                    }
                    f.fld(*slot);
                    f.fcomp_m(range);
                    if f.jp(0x05) {
                        break 'light;
                    }
                }
                f.fld(d[2]);
                f.fmul_m(d[2]);
                f.fld(d[1]);
                f.fmul_m(d[1]);
                f.faddp(1);
                f.fld(d[0]);
                f.fmul_m(d[0]);
                f.faddp(1);
                f.fsqrt();
                f.fcom_m(range);
                if f.jp(0x05) {
                    f.fstp_st(0);
                    break 'light;
                }
                f.fdiv_m(range);
                if ty == 2 {
                    f.fmulp(2);
                    break 'light;
                }
                f.fsubr_m(k(ONE));
                for (o, last) in [(0x1C, false), (0x20, false), (0x24, true)] {
                    if !last {
                        f.fld_st(0);
                    }
                    f.fmul_m(rd(o));
                    f.fmul_m(k(THIRD));
                    f.faddp(if last { 1 } else { 2 });
                }
            }
            i = i.wrapping_add(1);
            if i32::from(i) >= n {
                break;
            }
        }
    }
    f.faddp(1);
    f.pop()
}

/// `CPointLights::ProcessVerticalLineUsingCache(CVector p, float* groundZ)` (0x6FFFF0): a cached point
/// equal to `p` (all three compares equal) gives its cached z; else `CWorld::ProcessVerticalLine(p, p.z -
/// 20, ...)` (buildings and dummies); on a hit the colpoint z is cached at `NextCachedValue` (then `+1
/// mod 32`, C remainder) and returned; a miss returns false.
#[hook(addr = 0x6FFFF0, conv = cdecl, name = "CPointLights::ProcessVerticalLineUsingCache", expect = "83 EC 30 33 D2")]
pub fn process_vertical_line_using_cache(px: f32, py: f32, pz: f32, out: GPtr<u32>) -> u8 {
    let pts: GPtr<f32> = CACHED_MAP_READ_POINTS.ptr().cast();
    // `fld cached; fcomp p; test ah, 0x44; jp` = mismatch: only an ordered equal passes
    let eq = |a: f32, b: f32| !crate::common::x(a).fcom(crate::common::x(b)).test_parity(0x44);
    let mut i = 0;
    while i < MAX_LIGHTS {
        let c = pts.wrapping_index(3 * i);
        if eq(c.read(), px) && eq(c.wrapping_index(1).read(), py) && eq(c.wrapping_index(2).read(), pz) {
            out.write(CACHED_MAP_READS.ptr().cast::<u32>().wrapping_index(i).read());
            return 1;
        }
        i += 1;
    }
    let mut p = [px, py, pz];
    let z = crate::common::f32_of(crate::common::x(pz).fsub_m(k(TWENTY)));
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = GPtr::<u8>::NULL;
    let hit = process_vertical_line(
        GPtr::from_raw(p.as_mut_ptr()),
        z,
        GPtr::from_raw(cp.as_mut_ptr().cast()),
        GPtr::from_raw(&mut ent),
        1,
        0,
        0,
        0,
        1,
        0,
        0,
    );
    if hit == 0 {
        return 0;
    }
    let gz = crate::common::f32_of(crate::common::x(f32::from_bits(cp[2])));
    let n = NEXT_CACHED_VALUE.get();
    CACHED_MAP_READS.ptr().cast::<f32>().wrapping_index(n).write(gz);
    let c = pts.cast::<u32>().wrapping_index(n.wrapping_mul(3));
    c.write(px.to_bits());
    c.wrapping_index(1).write(py.to_bits());
    c.wrapping_index(2).write(pz.to_bits());
    NEXT_CACHED_VALUE.set(n.wrapping_add(1).wrapping_rem(32));
    out.write(gz.to_bits());
    1
}

/// `CPointLights::AddLight(u8 type, CVector pos, CVector dir, float radius, float r, float g, float b, u8
/// fogType, bool extraShadows, CEntity* attached)` (0x7000E0): dropped unless within `radius + 15` of
/// the camera on x and y (strict, `-lim` stored f32), fewer than 32 lights, and the distance (z
/// difference computed twice, unrounded) is below `radius + 15`; colour kept when `lim * 0.75 > dist`,
/// else scaled by `1 - (dist / lim - 0.75) * 4`.
#[hook(addr = 0x7000E0, conv = cdecl, name = "CPointLights::AddLight", expect = "8B 0D 3C F0 B6 00")]
#[allow(clippy::too_many_arguments)]
pub fn add_light(
    ty: u32,
    px: u32,
    py: u32,
    pz: u32,
    dx: u32,
    dy: u32,
    dz: u32,
    radius: u32,
    r: u32,
    g: u32,
    b: u32,
    fog: u32,
    shadows: u32,
    attached: u32,
) {
    let fx = f32::from_bits;
    let cam = cam_pos();
    let mut f = Fpu::new();
    f.fld(fx(px));
    f.fsub_m(cam.read());
    let ddx = f.fstp();
    f.fld(fx(radius));
    f.fadd_m(k(FIFTEEN));
    let lim = f.fstp();
    f.fld(ddx);
    f.fcomp_m(lim);
    if f.jp(0x05) {
        return;
    }
    let nlim = crate::common::f32_of(crate::common::x(lim).fchs());
    f.fld(ddx);
    f.fcomp_m(nlim);
    if f.jne(0x41) {
        return;
    }
    let cam = cam_pos();
    f.fld(fx(py));
    f.fsub_m(cam.wrapping_index(1).read());
    let ddy = f.fst();
    f.fcomp_m(lim);
    if f.jp(0x05) {
        return;
    }
    f.fld(ddy);
    f.fcomp_m(nlim);
    if f.jne(0x41) {
        return;
    }
    let n = NUM_LIGHTS.get();
    if n >= MAX_LIGHTS {
        return;
    }
    let cam = cam_pos();
    f.fld(fx(pz));
    f.fsub_m(cam.wrapping_index(2).read());
    f.fld(fx(pz));
    f.fsub_m(cam.wrapping_index(2).read());
    f.fmulp(1);
    f.fld(ddy);
    f.fmul_m(ddy);
    f.faddp(1);
    f.fld(ddx);
    f.fmul_m(ddx);
    f.faddp(1);
    f.fsqrt();
    let dist = f.fst();
    f.fcomp_m(lim);
    if f.jp(0x05) {
        return;
    }
    f.fld(lim);
    f.fmul_m(k(P075));
    let l = light(n);
    wr::<u8>(l, 0x2C, ty as u8);
    f.fcomp_m(dist);
    wr::<u8>(l, 0x2D, fog as u8);
    for (o, v) in [
        (0, px),
        (4, py),
        (8, pz),
        (0xC, dx),
        (0x10, dy),
        (0x14, dz),
        (0x18, radius),
    ] {
        wr::<u32>(l, o, v);
    }
    wr::<u8>(l, 0x2E, shadows as u8);
    wr::<u32>(l, 0x28, attached);
    if !f.jne(0x41) {
        wr::<u32>(l, 0x1C, r);
        wr::<u32>(l, 0x20, g);
        wr::<u32>(l, 0x24, b);
        NUM_LIGHTS.set(n + 1);
        return;
    }
    f.fld(dist);
    f.fdiv_m(lim);
    NUM_LIGHTS.set(n + 1);
    f.fsub_m(k(P075));
    f.fmul_m(k(FOUR));
    f.fsubr_m(k(ONE));
    f.fld(fx(r));
    f.fmul_st(1);
    wr::<f32>(l, 0x1C, f.fstp());
    f.fld(fx(g));
    f.fmul_st(1);
    wr::<f32>(l, 0x20, f.fstp());
    f.fmul_m(fx(b));
    wr::<f32>(l, 0x24, f.fstp());
}

/// `CBrightLights::Init()` (0x722140): `NumBrightLights = 0`.
#[hook(addr = 0x722140, conv = cdecl, name = "CBrightLights::Init", expect = "C7 05 FC C6 C7 00 00 00 00 00")]
pub fn bright_lights_init() {
    NUM_BRIGHT_LIGHTS.set(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_BRIGHT_LIGHTS_INIT,
    HOOK_GET_LIGHT_MULTIPLIER,
    HOOK_PROCESS_VERTICAL_LINE_USING_CACHE,
    HOOK_ADD_LIGHT,
];
