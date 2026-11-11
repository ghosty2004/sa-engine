//! WRITER-RENDER-BIG-2: `CClouds::VolumetricClouds_Create` (0x715F40). Ported from the 1.0 US bytes;
//! gta-reversed `Clouds.cpp` as reading aid. x87 arithmetic follows the original's instruction order on
//! the `sa_math::x87::X87` model; every `rand()` goes through `at_site` with the original's return
//! address.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, rd, x, xi};

/// `CClouds::m_VolumetricCloudsUsedNum` (slot count), `m_bVolumetricCloudsUsed[]`,
/// `m_bVolumetricCloudsInsideFrustum[]` (bytes), `m_fVolumetricCloudDensity` (0xC6AA58),
/// `m_VolumetricCloudsPosition[]` (CVector), `m_VolumetricCloudsSize[]` (CVector),
/// `m_VolumetricCloudsMaxAlpha[]` (int).
const USED_NUM: usize = 0xc6_aa5c;
const USED: usize = 0xc6_aab0;
const IN_FRUSTUM: usize = 0xc6_ab64;
const DENSITY_RANGE: usize = 0xc6_aa58;
const POSITION: usize = 0xc6_ac18;
const SIZE: usize = 0xc6_b488;
const MAX_ALPHA: usize = 0xc6_bcf8;
/// `TheCamera.m_matrix` (pointer) and `TheCamera.m_placement` position.
const CAMERA_MATRIX: usize = 0xb6_f03c;
const CAMERA_PLACEMENT_POS: usize = 0xb6_f02c;

fn rand_at(ret: u32) -> i32 {
    sa_core::orig::at_site(ret, sa_core::crt::rand)
}

/// `rand() * (1 / 32767)` (0x858C7C) as the x87 value.
fn rand_unit(ret: u32) -> X87 {
    xi(rand_at(ret)).fmul_m(k(0x3800_0100))
}

/// `(rand() & 0xFFFF) * 2^-15` (0x858B14) as the x87 value.
fn rand_low(ret: u32) -> X87 {
    xi(rand_at(ret) & 0xffff).fmul_m(k(0x3800_0000))
}

/// `36 - ftol((rand() & 0xFFFF) * 2^-15 * -92)`.
fn max_alpha(ret: u32) -> u32 {
    // -92 (0x872880)
    0x24u32.wrapping_sub(ftol32(rand_low(ret).fmul_m(k(0xc2b8_0000))) as u32)
}

/// `CClouds::VolumetricClouds_Create(CVector* pos)` (0x715F40); returns -1 on every path. `R = rand() /
/// 32767 * 4 + 1`; size ranges: x, y, z from `20R` to `100R`, `100R`, `40R`.
///
/// With `pos`: up to 5 clouds, each in the first unused slot (none: return at once); used and
/// in-frustum set, max alpha `36 - ftol(rand16 / 32768 * -92)`, size `rand / 32767 * (max - min) + min`
/// per axis, position `prev + ftol(rand16 / 32768 * 2s) - s` per axis with `s = ftol(3R)` (the first
/// relative to `pos`, each next one relative to the previous).
///
/// Without `pos` (`R` dropped): all 180 slots are filled around the camera with `d` = the density
/// range (0xC6AA58): size as above (with `(40R - 20R)` for z), position camera `+ rand / 32767 * (d -
/// -d) + -d` in x and y and `+ rand / 32767 * (d/4 - -d/4) + -d/4` in z.
///
/// The original reuses its own argument slot as the `fild` temporary (stack only).
#[hook(addr = 0x715f40, conv = cdecl, name = "CClouds::VolumetricClouds_Create", expect = "83 EC 3C 53 55 56 57",
       route = "L2", hazards = "stack-residue",
       recs = "grove-walk",
       note = "rand() at 15 sites; writes its own argument slot (the fild temporary)")]
pub fn volumetric_clouds_create(pos: GPtr<f32>) -> i32 {
    // 4 (0x858B90), 20 (0x858BA4), 100 (0x858628), 40 (0x858A10), 3 (0x858B3C)
    let r = rand_unit(0x71_5f4c).fmul_m(k(0x4080_0000)).fadd_m(1.0);
    let a = x(k(0x41a0_0000)).fmul(r).to_f32();
    let b = r.fmul_m(k(0x42c8_0000)).to_f32();
    let c = x(k(0x4220_0000)).fmul(r).to_f32();
    let (mins, maxs) = ([a, a, a], [b, b, c]);
    if !pos.is_null() {
        let p = pos.cast::<u8>();
        let mut prev = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
        let s = ftol32(r.fmul_m(k(0x4040_0000)));
        for _ in 0..5 {
            let n = g::<i32>(USED_NUM).read();
            let mut slot = 0;
            loop {
                if slot >= n {
                    return -1;
                }
                if g::<u8>(USED + slot as usize).read() == 0 {
                    break;
                }
                slot += 1;
            }
            let i = slot as usize;
            g::<u8>(USED + i).write(1);
            g::<u8>(IN_FRUSTUM + i).write(1);
            g::<u32>(MAX_ALPHA + 4 * i).write(max_alpha(0x71_600c));
            for (ax, ret) in [0x71_6043u32, 0x71_606a, 0x71_6091].into_iter().enumerate() {
                let v = rand_unit(ret).fmul(x(maxs[ax]).fsub_m(mins[ax])).fadd_m(mins[ax]);
                g::<f32>(SIZE + 12 * i + 4 * ax).write(v.to_f32());
            }
            let neg = s.wrapping_neg();
            for (ax, ret) in [0x71_60bcu32, 0x71_60f9, 0x71_6136].into_iter().enumerate() {
                let off = ftol32(rand_low(ret).fmul(xi(s.wrapping_sub(neg)))).wrapping_add(neg);
                let v = xi(off).fadd_m(prev[ax]).to_f32();
                g::<f32>(POSITION + 12 * i + 4 * ax).write(v);
            }
            prev = [
                g::<f32>(POSITION + 12 * i).read(),
                g::<f32>(POSITION + 12 * i + 4).read(),
                g::<f32>(POSITION + 12 * i + 8).read(),
            ];
        }
        return -1;
    }
    let d = g::<f32>(DENSITY_RANGE).read();
    // 0.25 (0x858C84)
    let q = x(d).fmul_m(k(0x3e80_0000)).to_f32();
    let m = g::<GPtr<u8>>(CAMERA_MATRIX).read();
    let cp = if m.is_null() { GPtr::<u8>::from_addr(CAMERA_PLACEMENT_POS) } else { m.byte_add(0x30) };
    let cam = [rd::<f32>(cp, 0), rd::<f32>(cp, 4), rd::<f32>(cp, 8)];
    let span = [
        x(b).fsub_m(a).to_f32(),
        x(b).fsub_m(a).to_f32(),
        x(c).fsub_m(a).to_f32(),
    ];
    let nd = x(d).fchs().to_f32();
    let nq = x(q).fchs().to_f32();
    let pmin = [nd, nd, nq];
    let pspan = [x(d).fsub_m(nd).to_f32(), x(d).fsub_m(nd).to_f32(), x(q).fsub_m(nq).to_f32()];
    for i in 0..0xb4usize {
        g::<u8>(USED + i).write(1);
        g::<u8>(IN_FRUSTUM + i).write(1);
        g::<u32>(MAX_ALPHA + 4 * i).write(max_alpha(0x71_6263));
        for (ax, ret) in [0x71_6294u32, 0x71_62b2, 0x71_62cf].into_iter().enumerate() {
            let v = rand_unit(ret).fmul_m(span[ax]).fadd_m(a);
            g::<f32>(SIZE + 12 * i + 4 * ax).write(v.to_f32());
        }
        for (ax, ret) in [0x71_62edu32, 0x71_6312, 0x71_6337].into_iter().enumerate() {
            let v = rand_unit(ret).fmul_m(pspan[ax]).fadd_m(pmin[ax]).fadd_m(cam[ax]);
            g::<f32>(POSITION + 12 * i + 4 * ax).write(v.to_f32());
        }
    }
    -1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VOLUMETRIC_CLOUDS_CREATE];
