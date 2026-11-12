//! PORT-VEH-4 (batch `world4-1`, SA:MP-executed rows): the breakable-object pieces `BreakObject_c`
//! (`Init`, `CalcGroupCenter`, `SetGroupData`, `SetBreakInfo`, `DoCollisionResponse`, `Update`,
//! `Render`). Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).
//!
//! Layout (0x20): +0 just-faces, +1 active, +2 add sparks, +3 draw last, +4 group count, +8 groups
//! (`BreakGroup_t`, 0x74 each), +0xC frames active, +0x10 ground z, +0x14 ground normal.
//! `BreakGroup_t`: +0 `RwMatrix` (right +0, up +0x10, at +0x20, pos +0x30), +0x40 velocity, +0x4C stopped,
//! +0x4E triangle count (u16), +0x50 render infos (0x48 each: 3 positions, 3 uv pairs at +0x24, 3 RGBA at
//! +0x3C), +0x54 texture, +0x58 type, +0x5C bounding size, +0x60 rotation speed, +0x64 rotation axis,
//! +0x70 frames to live. `BreakInfo_t`: +0 position rule, +8 vertex positions, +0xC uvs, +0x10 vertex
//! colours, +0x14 triangle count, +0x18 triangles (3 x u16), +0x1C triangle material indices, +0x20
//! material count, +0x24 textures, +0x30 material colour multipliers (3 floats).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::common::{ftol32, f32_of, g, rd, wr, x, xi};

orig! {
    /// CRT `operator new(uint32)` (0x82119A)
    fn crt_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, the result through `out`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0, AL)
    fn process_vertical_line(p: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<u8>, b0: u32, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `RwV3dTransformVector(out, in, matrix)` (0x7EDDC0)
    fn rw_v3d_transform_vector(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDDC0 => unported;
    /// `RwV3dNormalize(out, in)` (0x7ED9B0): the length in ST0
    fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwV3dLength(const RwV3d*)` (0x7EDAC0): ST0
    fn rw_v3d_length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
    /// `RwMatrixRotate(m, axis, angle, combine)` (0x7F1FD0)
    fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: f32, op: u32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// CRT `acos(double)` (0x822394, the cdecl entry): ST0
    fn crt_acos(v: f64) -> f64 = cdecl 0x822394 => unported;
    /// `CPostEffects::IsVisionFXActive()` (0x7034F0, AL)
    fn is_vision_fx_active() -> u8 = cdecl 0x7034F0 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, unk, life)` (0x4AB290), the floats as bits
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, unk: u32, life: u32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    fn fx_add_particle(sys: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32, bright: u32, cam: u32, local: u32) = thiscall 0x4AA440 => unported;
    /// `Fx_c::AddSparks(CVector&, CVector&, float, int32, CVector, eSparkType, float, float)` (0x49F040) on `g_fx` (0xA9AE00)
    fn fx_add_sparks(fx: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>, force: f32, amount: i32, across: CVector, kind: u32, spread: f32, life: f32) = thiscall 0x49F040 => unported;
    /// `BreakObject_c::Exit()` (0x59DDD0)
    fn break_object_exit(b: GPtr<u8>) = thiscall 0x59DDD0 => super::interior_group_update::break_object_exit;
    /// `BreakObject_c::CalcGroupCenter(BreakGroup_t*)` (0x59D190)
    fn calc_group_center(b: GPtr<u8>, grp: GPtr<u8>) = thiscall 0x59D190 => break_object_calc_group_center;
    /// `BreakObject_c::SetGroupData(const RwMatrix*, const CVector*, float)` (0x59D570)
    fn set_group_data(b: GPtr<u8>, m: GPtr<u8>, vel: GPtr<u8>, rnd: f32) = thiscall 0x59D570 => break_object_set_group_data;
    /// `BreakObject_c::SetBreakInfo(BreakInfo_t*, int32)` (0x59D7F0)
    fn set_break_info(b: GPtr<u8>, info: GPtr<u8>, just_faces: i32) = thiscall 0x59D7F0 => break_object_set_break_info;
    /// `BreakObject_c::DoCollisionResponse(BreakGroup_t*, float, const CVector*, float)` (0x59DE40)
    fn do_collision_response(b: GPtr<u8>, grp: GPtr<u8>, ts: f32, normal: GPtr<u8>, ground_z: f32) = thiscall 0x59DE40 => break_object_do_collision_response;
    /// `Fx_c::RenderBegin(RwRaster*, RwMatrix*, uint32 flags)` (0x4A13B0)
    fn fx_render_begin(raster: GPtr<u8>, m: GPtr<u8>, flags: u32) = cdecl 0x4A13B0 => unported;
    /// `Fx_c::RenderEnd()` (0x4A1600)
    fn fx_render_end() = cdecl 0x4A1600 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `Fx_c::RenderAddTri(x1, y1, z1, x2, y2, z2, x3, y3, z3, u1, v1, u2, v2, u3, v3, r1, g1, b1, a1,
    /// r2, g2, b2, a2, r3, g3, b3, a3)` (0x4A1410, cdecl; floats as bits, colour channels as ints)
    fn fx_render_add_tri(x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32, x3: u32, y3: u32, z3: u32, u1: u32, v1: u32, u2: u32, v2: u32, u3: u32, v3: u32, r1: u32, g1: u32, b1: u32, a1: u32, r2: u32, g2: u32, b2: u32, a2: u32, r3: u32, g3: u32, b3: u32, a3: u32) = cdecl 0x4A1410 => unported;
}

/// `rand()` as the original call returning to `site` calls it.
fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `fild` of a `rand()` result times 1/RAND_MAX (0x858C7C, 3.0518509e-05).
fn rand_unit(site: u32) -> X87 {
    xi(rand_at(site)).fmul_m(f32::from_bits(0x3800_0100))
}

/// `fcomp` + `fnstsw ax` + `test ah, mask` + `jp` taken.
fn jp(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_parity(mask)
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Group `i` of the array at +8 (re-read, as the original does on every access).
fn group(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74)
}

/// `BreakObject_c::CalcGroupCenter(BreakGroup_t* group)` (0x59D190): the bounding box of every vertex
/// of the group (min starts at 9999999, max at -9999999; the z maximum lives in ST0), the box centre
/// subtracted from every vertex, the centre transformed by the group matrix and added to its position;
/// the smallest box side (x, then y, then z on ties) decides the type (0/1/2) and the bounding size (half
/// that side). With a NaN side no type is set.
#[hook(addr = 0x59D190, conv = thiscall, name = "BreakObject_c::CalcGroupCenter", this = "BreakObject_c",
       expect = "83 EC 34 D9 05 60 6E 86 00",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "writes the group's render infos (heap) and stores the width into its own argument slot")]
pub fn break_object_calc_group_center(_this: GPtr<u8>, grp: GPtr<u8>) {
    let big = f32::from_bits(0x4B18_967F);
    let (mut minx, mut miny, mut minz) = (big, big, big);
    let (mut maxx, mut maxy) = (-big, -big);
    let mut maxz = f32::from_bits(0xCB18_967F);
    let n = i32::from(rd::<u16>(grp, 0x4E));
    let ri = rd::<GPtr<u8>>(grp, 0x50);
    for t in 0..n as isize {
        for k in 0..3isize {
            let p = ri.byte_add(t * 0x48 + k * 12);
            let (vx, vy, vz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
            if vx < minx {
                minx = vx;
            }
            if vx > maxx {
                maxx = vx;
            }
            if vy < miny {
                miny = vy;
            }
            if vy > maxy {
                maxy = vy;
            }
            if vz < minz {
                minz = vz;
            }
            if maxz < vz {
                maxz = vz;
            }
        }
    }
    let half = 0.5f32;
    let cx = f32_of(x(maxx).fadd_m(minx).fmul_m(half));
    let cy = f32_of(x(maxy).fadd_m(miny).fmul_m(half));
    let cz = f32_of(x(minz).fadd(x(maxz)).fmul_m(half));
    if n > 0 {
        for t in 0..i32::from(rd::<u16>(grp, 0x4E)) as isize {
            let ri = rd::<GPtr<u8>>(grp, 0x50).byte_add(t * 0x48);
            for k in 0..3isize {
                for (o, c) in [(0, cx), (4, cy), (8, cz)] {
                    let off = k * 12 + o;
                    wr::<f32>(ri, off, f32_of(x(rd::<f32>(ri, off)).fsub_m(c)));
                }
            }
        }
    }
    let minx2 = f32_of(x(minx).fsub_m(cx));
    let miny2 = f32_of(x(miny).fsub_m(cy));
    let minz2 = f32_of(x(minz).fsub_m(cz));
    let maxx2 = f32_of(x(maxx).fsub_m(cx));
    let maxy2 = f32_of(x(maxy).fsub_m(cy));
    let maxz2 = f32_of(x(maxz).fsub_m(cz));
    let mut centre = [cx.to_bits(), cy.to_bits(), cz.to_bits()];
    let mut out = [0u32; 3];
    rw_v3d_transform_vector(buf(&mut out), buf(&mut centre), grp);
    for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
        wr::<f32>(grp, o, f32_of(x(f32::from_bits(out[k])).fadd_m(rd::<f32>(grp, o))));
    }
    let w = f32_of(x(maxx2).fsub_m(minx2));
    let l = x(maxy2).fsub_m(miny2);
    let h = f32_of(x(maxz2).fsub_m(minz2));
    if !jp(x(w), l, 0x41) && !jp(x(w), x(h), 0x41) {
        wr::<u8>(grp, 0x58, 0);
        wr::<f32>(grp, 0x5C, f32_of(x(w).fmul_m(half)));
    } else if !jp(l, x(w), 0x41) && !jp(l, x(h), 0x41) {
        wr::<u8>(grp, 0x58, 1);
        wr::<f32>(grp, 0x5C, f32_of(l.fmul_m(half)));
    } else if !jp(x(h), l, 0x41) && !jp(x(h), x(w), 0x41) {
        wr::<u8>(grp, 0x58, 2);
        wr::<f32>(grp, 0x5C, f32_of(x(h).fmul_m(half)));
    }
}

/// `BreakObject_c::SetGroupData(const RwMatrix* matrix, const CVector* velocity, float velRand)`
/// (0x59D570): per group the matrix is copied and `CalcGroupCenter` run; the velocity is the given one
/// plus, unless `velRand == 0.0` (a NaN randomises), a random offset in `[-r, r]` per axis
/// (`(r - (-r)) * u + v + (-r)`), or without a velocity the random offset alone; rotation speed
/// `u * 3 + 3`, a random axis `u * 2 - 1` normalised, not stopped.
#[hook(addr = 0x59D570, conv = thiscall, name = "BreakObject_c::SetGroupData", this = "BreakObject_c",
       expect = "83 EC 20 53 55 8B E9",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x59D5CC/0x59D5FE/0x59D630 (no velocity), 0x59D693/0x59D6C9/0x59D6FF (velocity), 0x59D72D, 0x59D753/0x59D775/0x59D797")]
pub fn break_object_set_group_data(this: GPtr<u8>, matrix: GPtr<u8>, vel: GPtr<u8>, vel_rand: f32) {
    let mut i = 0;
    while i < rd::<i32>(this, 4) {
        let grp = group(this, i);
        for k in 0..16isize {
            wr::<u32>(grp, 4 * k, rd::<u32>(matrix, 4 * k));
        }
        calc_group_center(this, group(this, i));
        let nr = f32_of(x(vel_rand).fchs());
        if vel.is_null() {
            for (o, site) in [(0x40isize, 0x59_D5CCu32), (0x44, 0x59_D5FE), (0x48, 0x59_D630)] {
                let u = rand_unit(site);
                let v = x(vel_rand).fsub_m(nr).fmul(u).fadd_m(nr);
                wr::<f32>(group(this, i), o, f32_of(v));
            }
        } else {
            let grp = group(this, i);
            for o in [0isize, 4, 8] {
                wr::<u32>(grp, 0x40 + o, rd::<u32>(vel, o));
            }
            if x(vel_rand).fcom(x(0.0)) != sa_math::x87::Cmp::Equal {
                for (o, site) in [(0x40isize, 0x59_D693u32), (0x44, 0x59_D6C9), (0x48, 0x59_D6FF)] {
                    let u = rand_unit(site);
                    let g2 = group(this, i);
                    let v = x(vel_rand).fsub_m(nr).fmul(u).fadd_m(rd::<f32>(g2, o)).fadd_m(nr);
                    wr::<f32>(g2, o, f32_of(v));
                }
            }
        }
        let speed = rand_unit(0x59_D72D).fmul_m(3.0).fadd_m(3.0);
        wr::<f32>(group(this, i), 0x60, f32_of(speed));
        for (o, site) in [(0x64isize, 0x59_D753u32), (0x68, 0x59_D775), (0x6C, 0x59_D797)] {
            let a = rand_unit(site).fdouble().fsub_m(1.0);
            wr::<f32>(group(this, i), o, f32_of(a));
        }
        let axis = group(this, i).byte_add(0x64);
        rw_v3d_normalize(axis, axis);
        wr::<u8>(group(this, i), 0x4C, 0);
        i += 1;
    }
}

/// `BreakObject_c::SetBreakInfo(BreakInfo_t* info, int32 justFaces)` (0x59D7F0): one group per
/// triangle (`justFaces == 1`, compared with 1) or per material; per group a render-info array sized by
/// its triangle count and frames to live `256 - ftol(u * -32)` (u from `rand() & 0xFFFF`); then every
/// triangle's positions, uvs and colours (vertex colour times the material multiplier plus the ambient
/// colour, capped at 255, `_ftol`ed to a byte) go into its group, whose texture is the triangle's; the
/// ambient colour statics (0x8D0A0C/08/04) are refreshed from `AmbientLightColourForFrame * 255` per
/// triangle unless the vision FX are active; finally each group's texture gets a reference.
#[hook(addr = 0x59D7F0, conv = thiscall, name = "BreakObject_c::SetBreakInfo", this = "BreakObject_c",
       expect = "8B 44 24 08 83 EC 20",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "allocates the group and render-info arrays (operator new); rand at_site 0x59D89D; writes the ambient colour statics 0x8D0A04..0x8D0A0C and the textures' ref counts")]
pub fn break_object_set_break_info(this: GPtr<u8>, info: GPtr<u8>, just_faces: i32) {
    let faces = just_faces == 1;
    let num = if faces { rd::<u16>(info, 0x14) } else { rd::<u16>(info, 0x20) };
    wr::<u32>(this, 4, u32::from(num));
    let groups = crt_new(rd::<u32>(this, 4).wrapping_mul(0x74));
    wr::<GPtr<u8>>(this, 8, groups);
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        wr::<u16>(group(this, i), 0x4E, 0);
        wr::<u32>(group(this, i), 0x54, 0);
        let mut count = 0u32;
        if faces {
            count = 1;
        } else {
            let n = rd::<u16>(info, 0x14);
            let idx = rd::<GPtr<u8>>(info, 0x1C);
            for t in 0..n as isize {
                if i32::from(rd::<u16>(idx, 2 * t)) == i {
                    count += 1;
                }
            }
        }
        let ri = crt_new(count.wrapping_mul(0x48));
        wr::<GPtr<u8>>(group(this, i), 0x50, ri);
        let r = rand_at(0x59_D89D) & 0xFFFF;
        let f = xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(-32.0);
        wr::<i32>(group(this, i), 0x70, 0x100i32.wrapping_sub(ftol32(f)));
        i += 1;
    }
    let mut t = 0i32;
    while t < i32::from(rd::<u16>(info, 0x14)) {
        let gi = if faces { t } else { i32::from(rd::<u16>(rd::<GPtr<u8>>(info, 0x1C), 2 * t as isize)) };
        let grp = group(this, gi);
        let nt = rd::<u16>(grp, 0x4E);
        let ri = rd::<GPtr<u8>>(grp, 0x50).byte_add(nt as isize * 0x48);
        let tri = rd::<GPtr<u8>>(info, 0x18).byte_add(6 * t as isize);
        let v = [rd::<u16>(tri, 0) as isize, rd::<u16>(tri, 2) as isize, rd::<u16>(tri, 4) as isize];
        for (k, &vi) in v.iter().enumerate() {
            let src = rd::<GPtr<u8>>(info, 8).byte_add(vi * 12);
            for o in [0isize, 4, 8] {
                wr::<u32>(ri, k as isize * 12 + o, rd::<u32>(src, o));
            }
        }
        let tex_idx = if faces { i32::from(rd::<u16>(rd::<GPtr<u8>>(info, 0x1C), 2 * t as isize)) } else { gi };
        wr::<u32>(grp, 0x54, rd::<u32>(rd::<GPtr<u8>>(info, 0x24), 4 * tex_idx as isize));
        let mp = rd::<GPtr<u8>>(info, 0x30).byte_add(12 * tex_idx as isize);
        let mult = [rd::<f32>(mp, 0), rd::<f32>(mp, 4), rd::<f32>(mp, 8)];
        if is_vision_fx_active() == 0 {
            for (dst, src) in [(0x8D_0A0Cusize, 0xC8_86D4usize), (0x8D_0A08, 0xC8_86D8), (0x8D_0A04, 0xC8_86DC)] {
                g::<f32>(dst).write(f32_of(x(g::<f32>(src).read()).fmul_m(255.0)));
            }
        }
        let ambient = [0x8D_0A0Cusize, 0x8D_0A08, 0x8D_0A04];
        for (k, &vi) in v.iter().enumerate() {
            for c in 0..3usize {
                let col = rd::<u8>(rd::<GPtr<u8>>(info, 0x10), vi * 4 + c as isize);
                let mut val = xi(i32::from(col)).fmul_m(mult[c]).fadd_m(g::<f32>(ambient[c]).read());
                if !jp(x(255.0), val, 0x05) {
                    val = x(255.0);
                }
                wr::<u8>(ri, 0x3C + 4 * k as isize + c as isize, ftol32(val) as u8);
            }
        }
        for (k, &vi) in v.iter().enumerate() {
            let uv = rd::<GPtr<u8>>(info, 0xC).byte_add(vi * 8);
            wr::<u32>(ri, 0x24 + 8 * k as isize, rd::<u32>(uv, 0));
            wr::<u32>(ri, 0x28 + 8 * k as isize, rd::<u32>(uv, 4));
        }
        wr::<u16>(grp, 0x4E, rd::<u16>(grp, 0x4E).wrapping_add(1));
        t += 1;
    }
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        let tex = rd::<GPtr<u8>>(group(this, i), 0x54);
        if !tex.is_null() {
            wr::<u32>(tex, 0x54, rd::<u32>(tex, 0x54).wrapping_add(1));
        }
        i += 1;
    }
}

/// `BreakObject_c::DoCollisionResponse(BreakGroup_t* group, float timeStep, const CVector* normal,
/// float groundZ)` (0x59DE40): the velocity is reflected (`v - n * (v.n * 0.85 * 2)`), nudged by a
/// random unit vector times `timeStep * 0.05`, renormalised to its pre-nudge length and scaled by 0.8;
/// rotation stops and the group sits on the ground (`z = groundZ + boundingSize`). Below a speed of
/// 0.05 it stops moving (just-faces pieces then live `32 - ftol(u * -32)` frames). Unless just faces:
/// four `SmokeII3expand` puffs around it, and sparks (`|v| * 100` of them) when the object makes sparks.
#[hook(addr = 0x59DE40, conv = thiscall, name = "BreakObject_c::DoCollisionResponse", this = "BreakObject_c",
       expect = "83 EC 58 53 55 56",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x59DEAE/0x59DECD/0x59DEEC, 0x59E001, 0x59E07F/0x59E0A0/0x59E0C1/0x59E0E4 (per puff); adds particles and sparks (fx heap); stores the reflection factor and the speed into its own group-argument slot")]
pub fn break_object_do_collision_response(this: GPtr<u8>, grp: GPtr<u8>, ts: f32, normal: GPtr<u8>, ground_z: f32) {
    let (nx, ny, nz) = (rd::<f32>(normal, 0), rd::<f32>(normal, 4), rd::<f32>(normal, 8));
    let vx = x(rd::<f32>(grp, 0x40));
    let vy = x(rd::<f32>(grp, 0x44));
    let vz = rd::<f32>(grp, 0x48);
    let k = vx
        .fmul_m(nx)
        .fadd(vy.fmul_m(ny))
        .fadd(x(vz).fmul_m(nz))
        .fmul_m(g::<f32>(0x8D_0A18).read())
        .fdouble();
    let kf = f32_of(k);
    let mut new = [0u32; 3];
    new[0] = f32_of(vx.fsub(k.fmul_m(nx))).to_bits();
    new[1] = f32_of(vy.fsub(x(kf).fmul_m(ny))).to_bits();
    new[2] = f32_of(x(vz).fsub(x(kf).fmul_m(nz))).to_bits();
    let mut rnd = [0u32; 3];
    for (j, site) in [0x59_DEAEu32, 0x59_DECD, 0x59_DEEC].into_iter().enumerate() {
        rnd[j] = f32_of(rand_unit(site).fdouble().fsub_m(1.0)).to_bits();
    }
    rw_v3d_normalize(buf(&mut rnd), buf(&mut rnd));
    let sc = x(g::<f32>(0x8D_0A14).read()).fmul_m(ts);
    for r in rnd.iter_mut() {
        *r = f32_of(x(f32::from_bits(*r)).fmul(sc)).to_bits();
    }
    let mag = f32_of(X87::from_f64_exact(rw_v3d_length(buf(&mut new))));
    for j in 0..3 {
        new[j] = f32_of(x(f32::from_bits(rnd[j])).fadd_m(f32::from_bits(new[j]))).to_bits();
    }
    rw_v3d_normalize(buf(&mut new), buf(&mut new));
    for n in new.iter_mut() {
        *n = f32_of(x(mag).fmul_m(f32::from_bits(*n))).to_bits();
    }
    wr::<u32>(grp, 0x60, 0);
    for (j, o) in [(0usize, 0x40isize), (1, 0x44), (2, 0x48)] {
        wr::<f32>(grp, o, f32_of(x(g::<f32>(0x8D_0A10).read()).fmul_m(f32::from_bits(new[j]))));
    }
    wr::<f32>(grp, 0x38, f32_of(x(ground_z).fadd_m(rd::<f32>(grp, 0x5C))));
    let just_faces = rd::<u8>(this, 0) != 0;
    if !jp(x(mag), x(f32::from_bits(0x3D4C_CCCD)), 0x05) {
        wr::<u8>(grp, 0x4C, 1);
        if just_faces {
            let r = rand_at(0x59_E001) & 0xFFFF;
            let f = xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(-32.0);
            wr::<i32>(grp, 0x70, 0x20i32.wrapping_sub(ftol32(f)));
        }
    }
    if just_faces {
        return;
    }
    let mut mult = [0u32; 8];
    let mp = buf(&mut mult);
    fx_prt_mult_ctor(mp, 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3DCC_CCCD, 0x3E99_999A, 0, 0x3E19_999A);
    let pos = grp.byte_add(0x30);
    for sites in [[0x59_E07Fu32, 0x59_E0A0, 0x59_E0C1, 0x59_E0E4]; 4] {
        let mut p = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
        p[0] = f32_of(rand_unit(sites[0]).fadd_m(f32::from_bits(p[0])).fsub_m(0.5)).to_bits();
        p[1] = f32_of(rand_unit(sites[1]).fadd_m(f32::from_bits(p[1])).fsub_m(0.5)).to_bits();
        let mut v = [0u32; 3];
        v[0] = f32_of(rand_unit(sites[2]).fmul_m(f32::from_bits(0x3E99_999A)).fsub_m(f32::from_bits(0x3E19_999A))).to_bits();
        v[1] = f32_of(rand_unit(sites[3]).fmul_m(f32::from_bits(0x3E99_999A)).fsub_m(f32::from_bits(0x3E19_999A))).to_bits();
        let sys = g::<GPtr<u8>>(0xA9_AE24).read();
        fx_add_particle(sys, buf(&mut p), buf(&mut v), 0, mp, 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
    }
    if rd::<u8>(this, 2) != 0 {
        let len = X87::from_f64_exact(rw_v3d_length(grp.byte_add(0x40)));
        let amount = ftol32(len.fmul_m(100.0));
        let mut origin = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
        let mut dir = [0u32, 0, 0x3F80_0000];
        fx_add_sparks(
            GPtr::from_addr(0xA9_AE00),
            buf(&mut origin),
            buf(&mut dir),
            2.0,
            amount,
            CVector { x: 0.0, y: 0.0, z: 0.0 },
            1,
            f32::from_bits(0x3ECC_CCCD),
            1.0,
        );
    }
}

/// `BreakObject_c::Update(float timeStep)` (0x59E220): without groups `Exit`; else every moving group
/// falls (`v.z -= timeStep * 0.008`) and moves by `v * timeStep`; during the first five frames it spins
/// about its random axis (`timeStep * speed` degrees, pre-concatenated), later its facing axis (by type:
/// right/up/at) turns toward the ground normal (`acos(n.f)` when above 0.01 in magnitude, about `f x n`,
/// `angle * 57.29578 * timeStep * 0.05` degrees, post-concatenated, position kept); a group below the
/// ground collides. Every group's frames to live drop (clamped at 0); when none is left `Exit`. Frames
/// active +1.
///
/// Kept as in the original: a type other than 0..2 reuses the previous group's facing axis (the first
/// group's would be a null pointer).
#[hook(addr = 0x59E220, conv = thiscall, name = "BreakObject_c::Update", this = "BreakObject_c",
       expect = "55 8B EC 83 E4 F8",
       unverified = "world4-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "CRT acos (0x822394) of the facing dot product")]
pub fn break_object_update(this: GPtr<u8>, ts: f32) {
    if rd::<i32>(this, 4) <= 0 {
        break_object_exit(this);
        wr::<i32>(this, 0xC, rd::<i32>(this, 0xC).wrapping_add(1));
        return;
    }
    let mut remove = true;
    let mut facing = GPtr::<u8>::NULL;
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        if rd::<u8>(group(this, i), 0x4C) == 0 {
            let grp = group(this, i);
            let vz = x(ts).fmul_m(f32::from_bits(0x3C03_126F)).fsubr_m(rd::<f32>(grp, 0x48));
            wr::<f32>(grp, 0x48, f32_of(vz));
            for (p, v) in [(0x30isize, 0x40isize), (0x34, 0x44), (0x38, 0x48)] {
                let grp = group(this, i);
                wr::<f32>(grp, p, f32_of(x(ts).fmul_m(rd::<f32>(grp, v)).fadd_m(rd::<f32>(grp, p))));
            }
            if rd::<i32>(this, 0xC) < 5 {
                let grp = group(this, i);
                let angle = f32_of(x(ts).fmul_m(rd::<f32>(grp, 0x60)));
                rw_matrix_rotate(grp, grp.byte_add(0x64), angle, 1);
            } else {
                let grp = group(this, i);
                match rd::<u8>(grp, 0x58) {
                    0 => facing = grp,
                    1 => facing = grp.byte_add(0x10),
                    2 => facing = grp.byte_add(0x20),
                    _ => {}
                }
                let n = [rd::<f32>(this, 0x14), rd::<f32>(this, 0x18), rd::<f32>(this, 0x1C)];
                let (fx, fy, fz) = (rd::<f32>(facing, 0), rd::<f32>(facing, 4), rd::<f32>(facing, 8));
                let dot = x(n[0]).fmul_m(fx).fadd(x(n[1]).fmul_m(fy)).fadd(x(n[2]).fmul_m(fz));
                let a = X87::from_f64_exact(crt_acos(dot.to_f64()));
                let af = f32_of(a);
                if !a.fabs().fcom(x(f32::from_bits(0x3C23_D70A))).test_any(0x41) {
                    let mut axis = [0u32; 3];
                    axis[0] = f32_of(x(n[2]).fmul_m(fy).fsub(x(n[1]).fmul_m(fz))).to_bits();
                    axis[1] = f32_of(x(n[0]).fmul_m(fz).fsub(x(n[2]).fmul_m(fx))).to_bits();
                    axis[2] = f32_of(x(n[1]).fmul_m(fx).fsub(x(n[0]).fmul_m(fy))).to_bits();
                    rw_v3d_normalize(buf(&mut axis), buf(&mut axis));
                    let deg = x(af)
                        .fmul_m(f32::from_bits(0x4265_2EE0))
                        .fmul_m(ts)
                        .fmul_m(f32::from_bits(0x3D4C_CCCD));
                    let grp = group(this, i);
                    let saved = [rd::<u32>(grp, 0x30), rd::<u32>(grp, 0x34), rd::<u32>(grp, 0x38)];
                    rw_matrix_rotate(grp, buf(&mut axis), f32_of(deg), 2);
                    let grp = group(this, i);
                    for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
                        wr::<u32>(grp, o, saved[k]);
                    }
                }
            }
            let grp = group(this, i);
            let below = x(rd::<f32>(grp, 0x38)).fsub_m(rd::<f32>(grp, 0x5C));
            if !jp(below, x(rd::<f32>(this, 0x10)), 0x05) {
                do_collision_response(this, grp, ts, this.byte_add(0x14), rd::<f32>(this, 0x10));
            }
        }
        let grp = group(this, i);
        wr::<i32>(grp, 0x70, rd::<i32>(grp, 0x70).wrapping_sub(1));
        let grp = group(this, i);
        if rd::<i32>(grp, 0x70) > 0 {
            remove = false;
        } else {
            wr::<i32>(grp, 0x70, 0);
        }
        i += 1;
    }
    if remove {
        break_object_exit(this);
    }
    wr::<i32>(this, 0xC, rd::<i32>(this, 0xC).wrapping_add(1));
}

/// `BreakObject_c::Render(bool drawLast)` (0x59E480): only for the matching draw-last flag (+3); per
/// group the texture's raster starts a new `Fx_c` render batch when it changes (a null raster after a
/// null one restarts it too), then every triangle is transformed by the group matrix and added with the
/// group's alpha (`framesToLive * 8` for just faces, else `* 2`, 255 above 255, else the low byte);
/// the batch is ended after the last group. With no groups nothing is begun or ended.
#[hook(addr = 0x59E480, conv = thiscall, name = "BreakObject_c::Render", this = "BreakObject_c",
       expect = "8A 44 24 04 83 EC 2C",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "Fx_c render batch (RenderBegin/AddTri/End: the fx vertex buffer)")]
pub fn break_object_render(this: GPtr<u8>, draw_last: u8) {
    if draw_last != rd::<u8>(this, 3) {
        return;
    }
    if rd::<i32>(this, 4) <= 0 {
        return;
    }
    let mut last = GPtr::<u8>::NULL;
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        let tex = rd::<GPtr<u8>>(group(this, i), 0x54);
        let cur = if tex.is_null() { GPtr::<u8>::NULL } else { rd::<GPtr<u8>>(tex, 0) };
        if last != cur {
            fx_render_end();
            fx_render_begin(cur, GPtr::NULL, 1);
            last = cur;
        }
        if cur.is_null() && last.is_null() {
            fx_render_end();
            fx_render_begin(GPtr::NULL, GPtr::NULL, 1);
            last = GPtr::NULL;
        }
        let grp = group(this, i);
        let n = rd::<u16>(grp, 0x4E);
        for t in 0..n as isize {
            let mut verts = [0u32; 9];
            let ri = rd::<GPtr<u8>>(group(this, i), 0x50).byte_add(t * 0x48);
            rw_v3d_transform_points(buf(&mut verts), ri, 3, grp);
            let frames = rd::<i32>(group(this, i), 0x70);
            let a = if rd::<u8>(this, 0) != 0 { frames.wrapping_shl(3) } else { frames.wrapping_shl(1) };
            let alpha = u32::from(if a > 0xFF { 0xFF } else { a as u8 });
            let ri = rd::<GPtr<u8>>(group(this, i), 0x50).byte_add(t * 0x48);
            let c = |o: isize| u32::from(rd::<u8>(ri, o));
            let u = |o: isize| rd::<u32>(ri, o);
            fx_render_add_tri(
                verts[0], verts[1], verts[2], verts[3], verts[4], verts[5], verts[6], verts[7], verts[8],
                u(0x24), u(0x28), u(0x2C), u(0x30), u(0x34), u(0x38),
                c(0x3C), c(0x3D), c(0x3E), alpha,
                c(0x40), c(0x41), c(0x42), alpha,
                c(0x44), c(0x45), c(0x46), alpha,
            );
        }
        i += 1;
    }
    fx_render_end();
}

/// `BreakObject_c::Init(CObject* object, const CVector* velocity, float velRand, int32 justFaces)`
/// (0x59E750): false without an atomic (RwObject type 1) or without breakable-plugin info on its geometry
/// (plugin offset 0xBB4238); else `SetBreakInfo`, `SetGroupData` from the frame's LTM, the flags (just
/// faces, active, frames 0, sparks from the object info +0x4C, draw-last from entity flag bit 14), and
/// the ground under the object: from the LTM position raised by the col-model box min z + 0.25 (position
/// rule 0) or the box centre (+0.25 z) through the object matrix; `ProcessVerticalLine` down to -1000
/// gives ground z and normal (a normal with every component below 0.01 in magnitude becomes (0, 0, 1)),
/// a miss gives -1000 and (0, 0, 1). True.
#[hook(addr = 0x59E750, conv = thiscall, name = "BreakObject_c::Init", this = "BreakObject_c",
       expect = "83 EC 50 53 55 56 57",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "the CEntity* out-parameter of ProcessVerticalLine is the original's object-argument slot (a local here)")]
pub fn break_object_init(this: GPtr<u8>, obj: GPtr<u8>, vel: GPtr<u8>, vel_rand: f32, just_faces: i32) -> u8 {
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if rw.is_null() || rd::<u8>(rw, 0) != 1 {
        return 0;
    }
    let slot = rd::<GPtr<u8>>(rw, 0x18).byte_add(g::<i32>(0xBB_4238).read() as isize);
    let info = rd::<GPtr<u8>>(slot, 0);
    if info.is_null() {
        return 0;
    }
    set_break_info(this, info, just_faces);
    let ltm = frame_get_ltm(rd::<GPtr<u8>>(rw, 4));
    set_group_data(this, ltm, vel, vel_rand);
    wr::<u8>(this, 0, just_faces as u8);
    wr::<u8>(this, 1, 1);
    wr::<u32>(this, 0xC, 0);
    wr::<u8>(this, 2, rd::<u8>(rd::<GPtr<u8>>(obj, 0x160), 0x4C));
    wr::<u8>(this, 3, ((rd::<u32>(obj, 0x1C) >> 14) & 1) as u8);
    let mi = i32::from(rd::<i16>(obj, 0x22));
    let col = || {
        let m = g::<GPtr<u8>>(0xA9_B0C8 + 4 * mi as usize).read();
        rd::<GPtr<u8>>(m, 0x14)
    };
    let mut origin = [0u32; 3];
    if rd::<u32>(rd::<GPtr<u8>>(slot, 0), 0) == 0 {
        let p = ltm.byte_add(0x30);
        origin[0] = rd::<u32>(p, 0);
        origin[1] = rd::<u32>(p, 4);
        let z = rd::<f32>(p, 8);
        origin[2] = f32_of(x(z).fadd_m(rd::<f32>(col(), 8)).fadd_m(0.25)).to_bits();
    } else {
        let c = col();
        let (mn, mx) = (c, c.byte_add(0xC));
        let mut centre = [0u32; 3];
        for (k, o) in [(0usize, 0isize), (1, 4), (2, 8)] {
            let lo = rd::<f32>(mn, o);
            let mut v = x(rd::<f32>(mx, o)).fsub_m(lo).fmul_m(0.5).fadd_m(lo);
            if k == 2 {
                v = v.fadd_m(0.25);
            }
            centre[k] = f32_of(v).to_bits();
        }
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(buf(&mut out), rd::<GPtr<u8>>(obj, 0x14), buf(&mut centre));
        for (k, o) in [(0usize, 0isize), (1, 4), (2, 8)] {
            origin[k] = rd::<u32>(r, o);
        }
    }
    let mut cp = [0u32; 11];
    let mut ent = [0u32; 1];
    let hit = process_vertical_line(buf(&mut origin), -1000.0, buf(&mut cp), buf(&mut ent), 1, 0, 0, 0, 0, 0, 0);
    if hit == 0 {
        wr::<u32>(this, 0x10, 0xC47A_0000);
        wr::<u32>(this, 0x14, 0);
        wr::<u32>(this, 0x18, 0);
        wr::<u32>(this, 0x1C, 0x3F80_0000);
        return 1;
    }
    wr::<u32>(this, 0x10, cp[2]);
    wr::<u32>(this, 0x14, cp[4]);
    wr::<u32>(this, 0x18, cp[5]);
    wr::<u32>(this, 0x1C, cp[6]);
    let small = |o: isize| !jp(x(rd::<f32>(this, o)).fabs(), x(f32::from_bits(0x3C23_D70A)), 0x05);
    if small(0x14) && small(0x18) && small(0x1C) {
        wr::<u32>(this, 0x18, 0);
        wr::<u32>(this, 0x1C, 0x3F80_0000);
        wr::<u32>(this, 0x14, 0);
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_BREAK_OBJECT_CALC_GROUP_CENTER,
    HOOK_BREAK_OBJECT_SET_GROUP_DATA,
    HOOK_BREAK_OBJECT_SET_BREAK_INFO,
    HOOK_BREAK_OBJECT_DO_COLLISION_RESPONSE,
    HOOK_BREAK_OBJECT_UPDATE,
    HOOK_BREAK_OBJECT_RENDER,
    HOOK_BREAK_OBJECT_INIT,
];
