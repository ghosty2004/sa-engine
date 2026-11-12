//! The `FxInfo*_c::GetValue` functions SA:MP executes that were still unported (batch `samp-render-1`):
//! `FxInfoAttractPt_c`, `FxInfoSpriteRect_c`, `FxInfoFlat_c`, `FxInfoColourBright_c`,
//! `FxInfoGroundCollide_c`, and the key evaluator `FxInterpInfo255_c::GetVal`. Ported from the 1.0 US bytes
//! (`FxInfoTrail_c::GetValue` 0x4A62B0 is in `fx_info_trail_value`.) (gta-reversed's `Fx/Blueprints` as a reading aid; where it differs, the bytes win and the difference is named).
//!
//! `GetValue(float currTime, float mult, float timeStep, float totalTime, bool useConst, void* out)` (thiscall,
//! `ret 0x18`): the key time is `mult` when the info's byte +6 is set, else `(float)(currTime / totalTime)`; the
//! interp member at +8 fills a 16-float stack buffer. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, k, rd, wr};

orig! {
    /// `FxInterpInfo32_c::GetVal(float* out, float t)` (0x4A89C0)
    fn interp_32_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A89C0 => crate::mvpc_fx1::interp_32_get_val;
    /// `FxInterpInfoU255_c::GetVal(float* out, float t)` (0x4A8800)
    fn interp_u255_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8800 => crate::mvpc_fx1::interp_u255_get_val;
    /// `FxInterpInfo255_c::GetVal(float* out, float t)` (0x4A8B80)
    fn interp_255_get_val_call(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8B80 => interp_255_get_val;
    /// `FxInterpInfoFloat_c::GetVal(float* out, float t)` (0x4A8470)
    fn interp_float_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8470 => unported;
    /// `RwV3dLength(const RwV3d*)` (0x7EDAC0): the length in ST0
    fn rw_v3d_length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
    /// `RwV3dNormalize(RwV3d* out, const RwV3d* in)` (0x7ED9B0): the length in ST0 (popped by every caller here)
    fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector& origin, float distance, CColPoint&, CEntity*&, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0, cdecl)
    fn world_process_vertical_line(origin: GPtr<u8>, dist: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32,
        o: u32, d: u32, see: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

fn xf(v: f32) -> X87 {
    X87::from_f32(v)
}

/// The key time: `mult` with byte +6 set, else `(float)(currTime / totalTime)`.
fn key_time(this: GPtr<u8>, curr: u32, mult: u32, total: u32) -> u32 {
    if rd::<u8>(this, 6) != 0 {
        return mult;
    }
    xf(f32::from_bits(curr)).fdiv_m(f32::from_bits(total)).to_f32().to_bits()
}

/// A 16-float buffer filled by the interp member through `get`.
fn eval(this: GPtr<u8>, t: u32, get: fn(GPtr<u8>, GPtr<u8>, u32)) -> [f32; 16] {
    let mut b = [0f32; 16];
    get(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    b
}

/// `FxInfoAttractPt_c::GetValue` (0x4A56E0): `key = FxInterpInfo32_c::GetVal(t)`; `v = key[0..3] - out.pos`
/// (f32 each); `len = RwV3dLength(&v)` (stored as f32 into the `mult` slot); when the unrounded length is > 0
/// (ordered) `RwV3dNormalize(&v, &v)`; when the stored `len > 0.1f` (0x858B1C): `s = key[0] * timeStep` (x87,
/// the original uses `key[0]`, not `key[3]`, as gta-reversed also has it), `out.vel.x += v.x * s` (unrounded
/// product), `out.vel.y += f32(v.y * s)`, `out.vel.z += f32(v.z * s)`.
#[hook(addr = 0x4A56E0, conv = thiscall, name = "FxInfoAttractPt_c::GetValue", expect = "8A 41 06 83 EC 4C",
       unverified = "samp-render-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "writes the particle movement info")]
pub fn fx_info_attract_pt_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval(this, t, interp_32_get_val);
    let mut v = [0f32; 3];
    v[0] = xf(b[0]).fsub_m(rd::<f32>(out, 0)).to_f32();
    v[1] = xf(b[1]).fsub_m(rd::<f32>(out, 4)).to_f32();
    v[2] = xf(b[2]).fsub_m(rd::<f32>(out, 8)).to_f32();
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    let len = X87::from_f64_exact(rw_v3d_length(vp));
    let len32 = len.to_f32();
    if !len.fcom(xf(0.0)).test_any(0x41) {
        rw_v3d_normalize(vp, vp);
    }
    if !xf(len32).fcom(xf(k(0x3DCC_CCCD))).test_any(0x41) {
        let s = xf(b[0]).fmul_m(f32::from_bits(step));
        let px = xf(v[0]).fmul(s);
        let py = s.fmul_m(v[1]).to_f32();
        let pz = s.fmul_m(v[2]).to_f32();
        wr::<f32>(out, 0xC, px.fadd_m(rd::<f32>(out, 0xC)).to_f32());
        wr::<f32>(out, 0x10, xf(py).fadd_m(rd::<f32>(out, 0x10)).to_f32());
        wr::<f32>(out, 0x14, xf(pz).fadd_m(rd::<f32>(out, 0x14)).to_f32());
    }
}

/// `FxInfoSpriteRect_c::GetValue` (0x4A6140): `key = FxInterpInfo255_c::GetVal(t)`; render info +0x1C = key[0]
/// (`fld`/`fstp`), +0x20, +0x24, +0x28 = key[1..3] (dword copies).
#[hook(addr = 0x4A6140, conv = thiscall, name = "FxInfoSpriteRect_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "samp-render-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "writes the particle render info")]
pub fn fx_info_sprite_rect_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval(this, t, interp_255_get_val_call);
    wr::<f32>(out, 0x1C, xf(b[0]).to_f32());
    for (i, v) in b.iter().enumerate().take(4).skip(1) {
        wr::<u32>(out, 0x1C + 4 * i as isize, v.to_bits());
    }
}

/// `FxInfoFlat_c::GetValue` (0x4A63B0): `key = FxInterpInfoFloat_c::GetVal(t)`; the render info's matrix at +0x38:
/// right (+0x38..+0x40) = key[0..2], up (+0x48..+0x50) = key[3..5], at (+0x58..+0x60) = key[6..8] (dword copies),
/// flag byte +0x78 = 1; then `RwMatrixUpdate(&out->matrix)`.
#[hook(addr = 0x4A63B0, conv = thiscall, name = "FxInfoFlat_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "samp-render-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "writes the particle render info")]
pub fn fx_info_flat_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval(this, t, interp_float_get_val);
    for (i, o) in [0x38, 0x3C, 0x40, 0x48, 0x4C, 0x50, 0x58, 0x5C].into_iter().enumerate() {
        wr::<u32>(out, o, b[i].to_bits());
    }
    wr::<u8>(out, 0x78, 1);
    wr::<u32>(out, 0x60, b[8].to_bits());
    rw_matrix_update(out.byte_add(0x38));
}

/// `FxInfoColourBright_c::GetValue` (0x4A6860): `key = FxInterpInfoU255_c::GetVal(t)`; render info bytes 0..3 =
/// `_ftol(key[0..3])`, byte 7 = `_ftol(key[4])` (low bytes), dword +8 = 2.
#[hook(addr = 0x4A6860, conv = thiscall, name = "FxInfoColourBright_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "samp-render-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "writes the particle render info")]
pub fn fx_info_colour_bright_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval(this, t, interp_u255_get_val);
    for (i, o) in [0, 1, 2, 3, 7].into_iter().enumerate() {
        wr::<u8>(out, o, ftol32(xf(b[i])) as u8);
    }
    wr::<u32>(out, 8, 2);
}

/// A random component: `(f32)((rand() % 10000) * 0.0002f - 1.0f)`, the `rand` call attributed to its original
/// return address.
fn rand_comp(ret: u32) -> f32 {
    let r = sa_core::orig::at_site(ret, sa_core::crt::rand);
    X87::from_i32(r.wrapping_rem(10000)).fmul_m(k(0x3951_B717)).fsub_m(k(0x3F80_0000)).to_f32()
}

/// `FxInfoGroundCollide_c::GetValue` (0x4A7100): `CWorld::ProcessVerticalLine(pos copy, pos.z - vel.z, cp, ent,
/// buildings only)`; nothing more without a hit, when `cp.z > pos.z` (copy), or unless `out.pos.z < cp.z`
/// (ordered); the two tests exclude each other for ordered values, so the rest does not run in practice. The rest
/// (kept as the bytes have it): `key = FxInterpInfo32_c::GetVal(t)`; `d = f32(2 * k0 * dot(normal, vel))` (the
/// dot product of the hit normal with the velocity, where gta-reversed has the normal's squared length);
/// `o = vel - normal * d` (f32 each); `r` = three [`rand_comp`] normalised and scaled by `key[2] * timeStep * 5`
/// (f32 each); `len = f32(RwV3dLength(&o))`; `o += r`; `RwV3dNormalize(&o, &o)`; `out.pos.z = cp.z`;
/// `out.vel.x = k1 * (len * o.x)`, `.y = k1 * (len * o.y)` (unrounded products), `.z = k1 * f32(len * o.z)`.
#[hook(addr = 0x4A7100, conv = thiscall, name = "FxInfoGroundCollide_c::GetValue", expect = "81 EC AC 00 00 00",
       unverified = "samp-render-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "writes the particle movement info; the body after the hit tests is dead for ordered values")]
pub fn fx_info_ground_collide_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let mut origin = [rd::<u32>(out, 0), rd::<u32>(out, 4), rd::<u32>(out, 8)];
    let dist = xf(f32::from_bits(origin[2])).fsub_m(rd::<f32>(out, 0x14)).to_f32();
    let mut cp = [0u32; 11];
    let mut ent = 0u32;
    let hit = world_process_vertical_line(
        GPtr::from_raw(origin.as_mut_ptr()).cast(),
        dist.to_bits(),
        GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&raw mut ent).cast(),
        1, 0, 0, 0, 0, 0, 0,
    );
    if hit == 0 {
        return;
    }
    let cz = f32::from_bits(cp[2]);
    let (nx, ny, nz) = (f32::from_bits(cp[4]), f32::from_bits(cp[5]), f32::from_bits(cp[6]));
    if !xf(cz).fcom(xf(f32::from_bits(origin[2]))).test_any(0x41) {
        return;
    }
    if xf(rd::<f32>(out, 8)).fcom(xf(cz)).test_parity(5) {
        return;
    }
    let b = eval(this, t, interp_32_get_val);
    let (vx, vy, vz) = (rd::<f32>(out, 0xC), rd::<f32>(out, 0x10), rd::<f32>(out, 0x14));
    let k1 = b[1];
    let dot = xf(ny).fmul_m(vy).fadd(xf(nx).fmul_m(vx)).fadd(xf(nz).fmul_m(vz));
    let d = dot.fmul_m(b[0]).fdouble().to_f32();
    let mut o = [
        xf(vx).fsub(xf(nx).fmul_m(d)).to_f32(),
        xf(vy).fsub(xf(ny).fmul_m(d)).to_f32(),
        xf(vz).fsub(xf(nz).fmul_m(d)).to_f32(),
    ];
    let mut r = [rand_comp(0x4A_7246), rand_comp(0x4A_726B), rand_comp(0x4A_7290)];
    let rp = GPtr::from_raw(r.as_mut_ptr()).cast::<u8>();
    rw_v3d_normalize(rp, rp);
    let s = xf(b[2]).fmul_m(f32::from_bits(step)).fmul_m(k(0x40A0_0000));
    r[0] = xf(r[0]).fmul(s).to_f32();
    r[1] = xf(r[1]).fmul(s).to_f32();
    r[2] = s.fmul_m(r[2]).to_f32();
    let op = GPtr::from_raw(o.as_mut_ptr()).cast::<u8>();
    let len = X87::from_f64_exact(rw_v3d_length(op)).to_f32();
    o[0] = xf(r[0]).fadd_m(o[0]).to_f32();
    o[1] = xf(r[1]).fadd_m(o[1]).to_f32();
    o[2] = xf(r[2]).fadd_m(o[2]).to_f32();
    let op = GPtr::from_raw(o.as_mut_ptr()).cast::<u8>();
    rw_v3d_normalize(op, op);
    let lx = xf(len).fmul_m(o[0]);
    wr::<f32>(out, 8, cz);
    let ly = xf(len).fmul_m(o[1]);
    let lz = xf(len).fmul_m(o[2]).to_f32();
    wr::<f32>(out, 0xC, xf(k1).fmul(lx).to_f32());
    wr::<f32>(out, 0x10, xf(k1).fmul(ly).to_f32());
    wr::<f32>(out, 0x14, xf(k1).fmul_m(lz).to_f32());
}

/// `FxInterpInfo255_c::GetVal(float* out, float t)` (0x4A8B80, thiscall, `ret 8`): byte for byte
/// `FxInterpInfo32_c::GetVal` (0x4A89C0) with the key scale `1/128` (0x858B88) instead of 0.001: signed short keys,
/// key times `u16 / 256`; the shared body is `mvpc_fx1`'s `interp_short_get_val`.
#[hook(addr = 0x4A8B80, conv = thiscall, name = "FxInterpInfo255_c::GetVal", expect = "51 53 56 8B F1",
       unverified = "samp-render-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "weapon-combat",
       note = "writes the caller's key buffer; uses its own argument slot t (+8) and the pushed-ECX slot as scratch (no footprint recorded)")]
pub fn interp_255_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) {
    crate::mvpc_fx1::interp_short_get_val(this, out, t, true, 0x3C00_0000);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FX_INFO_ATTRACT_PT_GET_VALUE,
    HOOK_FX_INFO_SPRITE_RECT_GET_VALUE,
    HOOK_FX_INFO_FLAT_GET_VALUE,
    HOOK_FX_INFO_COLOUR_BRIGHT_GET_VALUE,
    HOOK_FX_INFO_GROUND_COLLIDE_GET_VALUE,
    HOOK_INTERP_255_GET_VAL,
];
