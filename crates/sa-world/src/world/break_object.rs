//! WRITER-WORLD (batch `world5-1`, SA:MP-executed rows): `BreakObject_c`, the pieces of a broken
//! object (glass, crates): setup from the breakable plugin, per-frame motion and ground response,
//! and immediate-mode rendering. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the
//! bytes win: it differs in the face-only frame counts, the render early-out and the facing pick).
//!
//! Layouts: `BreakObject_c` (0x20): +0 just-faces, +1 active, +2 add-sparks, +3 draw-last,
//! +4 group count, +8 groups, +0xC frames active, +0x10 ground z, +0x14 ground normal.
//! `BreakGroup_t` (0x74): +0 `RwMatrix` (right/up/at/pos at 0/0x10/0x20/0x30), +0x40 velocity,
//! +0x4C stopped, +0x4E triangle count (u16), +0x50 render infos, +0x54 texture, +0x58 facing axis
//! (0 right, 1 up, 2 at), +0x5C bounding half size, +0x60 rotation speed, +0x64 rotation axis,
//! +0x70 frames to live. Render info (0x48): three positions, three UVs at +0x24, three RGBA at +0x3C.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use super::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `BreakObject_c::Exit()` (0x59DDD0)
    fn break_object_exit(o: GPtr<u8>) = thiscall 0x59DDD0 => unported;
    /// `BreakObject_c::DoCollisionResponse(BreakGroup_t*, float, const CVector*, float)` (0x59DE40)
    fn do_collision_response_call(o: GPtr<u8>, grp: GPtr<u8>, ts: f32, n: GPtr<u8>, z: f32) = thiscall 0x59DE40 => do_collision_response;
    /// `BreakObject_c::CalcGroupCenter(BreakGroup_t*)` (0x59D190)
    fn calc_group_center_call(o: GPtr<u8>, grp: GPtr<u8>) = thiscall 0x59D190 => calc_group_center;
    /// `BreakObject_c::SetBreakInfo(BreakInfo_t*, int32)` (0x59D7F0)
    fn set_break_info_call(o: GPtr<u8>, info: GPtr<u8>, just_faces: i32) = thiscall 0x59D7F0 => set_break_info;
    /// `BreakObject_c::SetGroupData(const RwMatrix*, const CVector*, float)` (0x59D570)
    fn set_group_data_call(o: GPtr<u8>, m: GPtr<u8>, vel: GPtr<u8>, rand: u32) = thiscall 0x59D570 => set_group_data;
    /// `RwV3dNormalize(out, in)` (0x7ED9B0): the length in ST0
    fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwV3dLength(const RwV3d*)` (0x7EDAC0): the length in ST0
    fn rw_v3d_length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RwV3dTransformVector(out, in, matrix)` (0x7EDDC0)
    fn rw_v3d_transform_vector(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDDC0 => unported;
    /// `RwMatrixRotate(m, axis, angle, op)` (0x7F1FD0; the angle as its raw dword)
    fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: u32, op: i32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// CRT `acos(double)` (0x822394), ST0
    fn crt_acos(v: f64) -> f64 = cdecl 0x822394 => unported;
    /// CRT `operator new(size)` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CPostEffects::IsVisionFXActive()` (0x7034F0)
    fn is_vision_fx_active() -> u8 = cdecl 0x7034F0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890): the result through the hidden pointer
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, float g, float b, float a, float size, float, float life)` (0x4AB290)
    fn fx_prt_mult_ctor(m: GPtr<u8>, r: u32, gg: u32, b: u32, a: u32, size: u32, inc: u32, life: u32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32, bright: u32, cam: u32, local: u32) = thiscall 0x4AA440 => unported;
    /// `Fx_c::AddSparks(RwV3d&, RwV3d&, float, int32, CVector, uint8, float, float)` (0x49F040) on `g_fx` (0xA9AE00)
    fn fx_add_sparks(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, force: u32, amount: i32, ax: u32, ay: u32, az: u32, ty: u32, spread: u32, life: u32) = thiscall 0x49F040 => unported;
    /// `RenderBegin(RwRaster*, RwMatrix*, uint32 flags)` (0x4A13B0)
    fn render_begin(raster: u32, ltm: u32, flags: u32) = cdecl 0x4A13B0 => unported;
    /// `RenderEnd()` (0x4A1600)
    fn render_end() = cdecl 0x4A1600 => unported;
    /// `RenderAddTri(x0..z2, u0..v2, r0, g0, b0, a0, r1.., r2..)` (0x4A1410)
    fn render_add_tri(x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32, u0: u32, v0: u32, u1: u32, v1: u32, u2: u32, v2: u32, r0: u32, g0: u32, b0: u32, a0: u32, r1: u32, g1: u32, b1: u32, a1: u32, r2: u32, g2: u32, b2: u32, a2: u32) = cdecl 0x4A1410 => unported;
}

/// `1 / 32767` (0x858C7C), the factor of every `rand()` range below.
const RAND_K: f32 = f32::from_bits(0x3800_0100);

/// `rand()` with the original return address `site`.
fn rnd(site: u32) -> i32 {
    at_site(site, rand)
}

/// `rand() * (1/32767) * 2 - 1` (`fild; fmul; fadd st, st; fsub 1.0`): [-1, 1].
fn rand_signed(site: u32) -> f32 {
    let t = xi(rnd(site)).fmul_m(RAND_K);
    t.fadd(t).fsub_m(1.0).to_f32()
}

/// `(int)(rand() & 0xFFFF) * (1/32768) * -32` truncated (`_ftol`): the 0..-31 offset of the frame
/// counters (0x858B14, 0x866E64).
fn rand_frames(site: u32) -> i32 {
    let r = rnd(site) & 0xFFFF;
    ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(-32.0))
}

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `BreakObject_c::DoCollisionResponse(BreakGroup_t* group, float ts, const CVector* n, float
/// groundZ)` (0x59DE40): reflect the group's velocity on the ground normal (`v - n * (v.n * 0.85 *
/// 2)`), add a random unit vector scaled by `ts * 0.05`, keep the reflected speed (normalise and
/// rescale) times 0.8, stop the spin, rest the piece on `groundZ + half size`. Below speed 0.05
/// the piece stops; face-only objects then get 32 + 0..31 frames to live (`32 - ftol(r * -32)`)
/// and return, as they do at any speed. Otherwise four `m_SmokeII3expand` puffs around the piece
/// (fresh position copy each, +-0.5 xy jitter, +-0.15 xy velocity) and, with sparks on, a spark
/// burst upwards of `|v| * 100` sparks.
#[hook(addr = 0x59DE40, conv = thiscall, name = "BreakObject_c::DoCollisionResponse", this = "BreakObject_c",
       expect = "83 EC 58 53 55 56",
       unverified = "world5-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x59DEAE/0x59DECD/0x59DEEC, 0x59E001, 0x59E07F/0x59E0A0/0x59E0C1/0x59E0E4; adds particles/sparks (fx pools); the original reuses its group argument slot for the doubled dot and the speed")]
pub fn do_collision_response(this: GPtr<u8>, group: GPtr<u8>, ts: f32, normal: GPtr<u8>, ground_z: f32) {
    let (vx, vy, vz) = (rd::<f32>(group, 0x40), rd::<f32>(group, 0x44), rd::<f32>(group, 0x48));
    let (nx, ny, nz) = (rd::<f32>(normal, 0), rd::<f32>(normal, 4), rd::<f32>(normal, 8));
    let dot = x(vx).fmul_m(nx).fadd(x(vy).fmul_m(ny)).fadd(x(vz).fmul_m(nz));
    let d = dot.fmul_m(f32::from_bits(0x3F59_999A));
    let d2 = d.fadd(d);
    let d2f = d2.to_f32();
    let mut nv = [
        d2.fmul_m(nx).fsubr(x(vx)).to_f32(),
        x(d2f).fmul_m(ny).fsubr(x(vy)).to_f32(),
        x(d2f).fmul_m(nz).fsubr_m(vz).to_f32(),
    ];
    let mut r = [rand_signed(0x59_DEAE), rand_signed(0x59_DECD), rand_signed(0x59_DEEC)];
    rw_v3d_normalize(p(&mut r), p(&mut r));
    let tsc = x(f32::from_bits(0x3D4C_CCCD)).fmul_m(ts);
    r[0] = x(r[0]).fmul(tsc).to_f32();
    r[1] = x(r[1]).fmul(tsc).to_f32();
    r[2] = tsc.fmul_m(r[2]).to_f32();
    let len = X87::from_f64_exact(rw_v3d_length(p(&mut nv))).to_f32();
    for k in 0..3 {
        nv[k] = x(r[k]).fadd_m(nv[k]).to_f32();
    }
    rw_v3d_normalize(p(&mut nv), p(&mut nv));
    nv[0] = x(len).fmul_m(nv[0]).to_f32();
    wr::<u32>(group, 0x60, 0);
    nv[1] = x(len).fmul_m(nv[1]).to_f32();
    nv[2] = x(len).fmul_m(nv[2]).to_f32();
    for (k, v) in nv.iter().enumerate() {
        wr::<f32>(group, 0x40 + 4 * k as isize, x(f32::from_bits(0x3F4C_CCCD)).fmul_m(*v).to_f32());
    }
    wr::<f32>(group, 0x38, x(ground_z).fadd_m(rd::<f32>(group, 0x5C)).to_f32());
    let just_faces = || rd::<u8>(this, 0) != 0;
    if x(len).fcom(x(f32::from_bits(0x3D4C_CCCD))).test_parity(0x05) {
        if just_faces() {
            return;
        }
    } else {
        wr::<u8>(group, 0x4C, 1);
        if just_faces() {
            wr::<i32>(group, 0x70, 32i32.wrapping_sub(rand_frames(0x59_E001)));
            return;
        }
    }
    let mut mult = [0u32; 8];
    fx_prt_mult_ctor(p(&mut mult), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3DCC_CCCD, 0x3E99_999A, 0, 0x3E19_999A);
    let half = f32::from_bits(0x3F00_0000);
    let (k3, k15) = (f32::from_bits(0x3E99_999A), f32::from_bits(0x3E19_999A));
    for _ in 0..4 {
        let mut pos = [rd::<f32>(group, 0x30), rd::<f32>(group, 0x34), rd::<f32>(group, 0x38)];
        pos[0] = xi(rnd(0x59_E07F)).fmul_m(RAND_K).fadd_m(pos[0]).fsub_m(half).to_f32();
        pos[1] = xi(rnd(0x59_E0A0)).fmul_m(RAND_K).fadd_m(pos[1]).fsub_m(half).to_f32();
        let mut vel = [0f32; 3];
        vel[0] = xi(rnd(0x59_E0C1)).fmul_m(RAND_K).fmul_m(k3).fsub_m(k15).to_f32();
        vel[1] = xi(rnd(0x59_E0E4)).fmul_m(RAND_K).fmul_m(k3).fsub_m(k15).to_f32();
        fx_add_particle(
            g::<GPtr<u8>>(0xA9_AE24).read(),
            p(&mut pos),
            p(&mut vel),
            0,
            p(&mut mult),
            0xBF80_0000,
            0x3F99_999A,
            0x3F19_999A,
            0,
        );
    }
    if rd::<u8>(this, 2) == 0 {
        return;
    }
    let speed = X87::from_f64_exact(rw_v3d_length(group.byte_add(0x40))).fmul_m(100.0);
    let mut pos = [rd::<u32>(group, 0x30), rd::<u32>(group, 0x34), rd::<u32>(group, 0x38)];
    let mut dir = [0u32, 0, 0x3F80_0000];
    let amount = ftol32(speed);
    fx_add_sparks(GPtr::from_addr(0xA9_AE00), p(&mut pos), p(&mut dir), 0x4000_0000, amount, 0, 0, 0, 1, 0x3ECC_CCCD, 0x3F80_0000);
}

/// `BreakObject_c::DoCollision(BreakGroup_t* group, float ts)` (0x59E1F0): the ground response when
/// the piece's bottom (`pos.z - half size`) is below the ground z.
#[hook(addr = 0x59E1F0, conv = thiscall, name = "BreakObject_c::DoCollision", this = "BreakObject_c",
       expect = "8B 54 24 04 D9 42 38",
       unverified = "world5-1", route = "L2", hazards = "heap-writes")]
pub fn do_collision(this: GPtr<u8>, group: GPtr<u8>, ts: f32) {
    let below = x(rd::<f32>(group, 0x38)).fsub_m(rd::<f32>(group, 0x5C)).fcom(x(rd::<f32>(this, 0x10)));
    if !below.test_parity(0x05) {
        do_collision_response_call(this, group, ts, this.byte_add(0x14), rd::<f32>(this, 0x10));
    }
}

/// `BreakObject_c::Update(float ts)` (0x59E220): every moving piece falls (`v.z -= ts * 0.008`) and
/// moves by `v * ts`; for the first five frames it spins about its random axis (`RwMatrixRotate`
/// pre-concat, `ts * speed` degrees), later it turns its facing axis towards the ground normal
/// (post-concat about `facing x normal`, `acos(normal . facing)` in degrees `* ts / 20`, only above
/// 0.01 rad; the position is kept), then the ground response (DoCollision, inlined). Every piece
/// counts its frames down to 0; when none is left (or there are no groups) the object exits. The
/// frame counter always advances. Original quirk kept: a facing type above 2 reuses the previous
/// piece's axis pointer (null for the first).
#[hook(addr = 0x59E220, conv = thiscall, name = "BreakObject_c::Update", this = "BreakObject_c",
       expect = "55 8B EC 83 E4 F8",
       unverified = "world5-1", route = "L2", hazards = "heap-writes",
       note = "CRT acos (0x822394) through orig; frees the pieces on exit (heap)")]
pub fn update(this: GPtr<u8>, ts: f32) {
    if rd::<i32>(this, 4) > 0 {
        let mut remove = true;
        let mut facing = GPtr::<u8>::NULL;
        let mut i = 0i32;
        while i < rd::<i32>(this, 4) {
            let grp = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
            if rd::<u8>(grp, 0x4C) == 0 {
                let vz = x(ts).fmul_m(f32::from_bits(0x3C03_126F)).fsubr_m(rd::<f32>(grp, 0x48));
                wr::<f32>(grp, 0x48, vz.to_f32());
                for k in 0..3isize {
                    let v = x(ts).fmul_m(rd::<f32>(grp, 0x40 + 4 * k)).fadd_m(rd::<f32>(grp, 0x30 + 4 * k));
                    wr::<f32>(grp, 0x30 + 4 * k, v.to_f32());
                }
                if rd::<i32>(this, 0xC) < 5 {
                    let angle = x(ts).fmul_m(rd::<f32>(grp, 0x60)).to_f32();
                    rw_matrix_rotate(grp, grp.byte_add(0x64), angle.to_bits(), 1);
                } else {
                    match rd::<u8>(grp, 0x58) {
                        0 => facing = grp,
                        1 => facing = grp.byte_add(0x10),
                        2 => facing = grp.byte_add(0x20),
                        _ => {}
                    }
                    let (nx, ny, nz) = (rd::<f32>(this, 0x14), rd::<f32>(this, 0x18), rd::<f32>(this, 0x1C));
                    let (fx, fy, fz) = (rd::<f32>(facing, 0), rd::<f32>(facing, 4), rd::<f32>(facing, 8));
                    let dot = x(nx).fmul_m(fx).fadd(x(ny).fmul_m(fy)).fadd(x(nz).fmul_m(fz));
                    let a = X87::from_f64_exact(crt_acos(dot.to_f64()));
                    let a32 = a.to_f32();
                    if !a.fabs().fcom(x(f32::from_bits(0x3C23_D70A))).test_any(0x41) {
                        let mut axis = [
                            x(nz).fmul_m(fy).fsub(x(ny).fmul_m(fz)).to_f32(),
                            x(nx).fmul_m(fz).fsub(x(nz).fmul_m(fx)).to_f32(),
                            x(ny).fmul_m(fx).fsub(x(nx).fmul_m(fy)).to_f32(),
                        ];
                        rw_v3d_normalize(p(&mut axis), p(&mut axis));
                        let deg = x(a32)
                            .fmul_m(f32::from_bits(0x4265_2EE0))
                            .fmul_m(ts)
                            .fmul_m(f32::from_bits(0x3D4C_CCCD))
                            .to_f32();
                        let saved = [rd::<u32>(grp, 0x30), rd::<u32>(grp, 0x34), rd::<u32>(grp, 0x38)];
                        rw_matrix_rotate(grp, p(&mut axis), deg.to_bits(), 2);
                        let grp2 = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
                        for (k, v) in saved.iter().enumerate() {
                            wr::<u32>(grp2, 0x30 + 4 * k as isize, *v);
                        }
                    }
                }
                let grp = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
                let c = x(rd::<f32>(grp, 0x38)).fsub_m(rd::<f32>(grp, 0x5C)).fcom(x(rd::<f32>(this, 0x10)));
                if !c.test_parity(0x05) {
                    do_collision_response_call(this, grp, ts, this.byte_add(0x14), rd::<f32>(this, 0x10));
                }
            }
            let grp = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
            let left = rd::<i32>(grp, 0x70).wrapping_sub(1);
            wr::<i32>(grp, 0x70, left);
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
    } else {
        break_object_exit(this);
    }
    wr::<i32>(this, 0xC, rd::<i32>(this, 0xC).wrapping_add(1));
}

/// `BreakObject_c::Render(bool drawLast)` (0x59E480): only in the pass matching the object's
/// draw-last flag, and only with at least one group (no `RenderEnd` otherwise). Per group: restart
/// the immediate-mode batch when the texture's raster changes (and again for an untextured group
/// after an untextured one), then one triangle per render info, transformed by the group matrix,
/// alpha `frames to live * 8` (face-only) or `* 2`, capped at 255 (`or cl, 0xFF`), RGB from the
/// info. `RenderEnd` at the end.
#[hook(addr = 0x59E480, conv = thiscall, name = "BreakObject_c::Render", this = "BreakObject_c",
       expect = "8A 44 24 04 83 EC 2C",
       unverified = "world5-1", route = "L2", hazards = "heap-writes",
       note = "fills the g_fx immediate-mode vertex buffer (heap); render pass only")]
pub fn render(this: GPtr<u8>, draw_last: u8) {
    if draw_last != rd::<u8>(this, 3) || rd::<i32>(this, 4) <= 0 {
        return;
    }
    let mut last = 0u32;
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        let grp = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
        let tex = rd::<GPtr<u8>>(grp, 0x54);
        let raster = if tex.is_null() { 0 } else { rd::<u32>(tex, 0) };
        if last != raster {
            render_end();
            render_begin(raster, 0, 1);
            last = raster;
        }
        if raster == 0 && last == 0 {
            render_end();
            render_begin(0, 0, 1);
            last = 0;
        }
        let grp = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
        let n = u32::from(rd::<u16>(grp, 0x4E));
        for k in 0..n as usize {
            let ri = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74), 0x50).byte_add(k as isize * 0x48);
            let mut v = [0u32; 9];
            rw_v3d_transform_points(p(&mut v), ri, 3, grp);
            let g2 = rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
            let ftl = rd::<i32>(g2, 0x70);
            let a = if rd::<u8>(this, 0) != 0 { ftl.wrapping_shl(3) } else { ftl.wrapping_shl(1) };
            let a = if a > 0xFF { 0xFF } else { a as u32 & 0xFF };
            let ri = rd::<GPtr<u8>>(g2, 0x50).byte_add(k as isize * 0x48);
            let c = |o: isize| u32::from(rd::<u8>(ri, o));
            let u = |o: isize| rd::<u32>(ri, o);
            render_add_tri(
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                u(0x24), u(0x28), u(0x2C), u(0x30), u(0x34), u(0x38),
                c(0x3C), c(0x3D), c(0x3E), a,
                c(0x40), c(0x41), c(0x42), a,
                c(0x44), c(0x45), c(0x46), a,
            );
        }
        i += 1;
    }
    render_end();
}

/// `BreakObject_c::Init(CObject* obj, const CVector* vel, float velRand, int32 justFaces)`
/// (0x59E750): false unless the object's RW object is an atomic whose geometry carries breakable
/// plugin data (geometry + `[0xBB4238]`). Sets the break info and group data (from the atomic
/// frame's LTM), the flags (just-faces = the argument's low byte, active, sparks from the object
/// info +0x4C, draw-last from object flag bit 14), then probes the ground straight down (-1000,
/// buildings only) from either the LTM position lifted by the collision box's min z + 0.25
/// (position rule 0) or the object matrix times the box centre lifted by 0.25: ground z and
/// normal on a hit (a normal with every component below 0.01 becomes (0, 0, 1)), -1000 and
/// (0, 0, 1) otherwise. True.
#[hook(addr = 0x59E750, conv = thiscall, name = "BreakObject_c::Init", this = "BreakObject_c",
       expect = "83 EC 50 53 55 56",
       unverified = "world5-1", route = "split", hazards = "heap-writes",
       note = "allocates the groups and render infos (operator new); rand through SetGroupData/SetBreakInfo; the ground probe writes the hit entity into the original's object argument slot")]
pub fn init(this: GPtr<u8>, obj: GPtr<u8>, vel: GPtr<u8>, vel_rand: u32, just_faces: i32) -> u8 {
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if rw.is_null() || rd::<u8>(rw, 0) != 1 {
        return 0;
    }
    let slot = rd::<GPtr<u8>>(rw, 0x18).byte_add(g::<u32>(0xBB_4238).read() as i32 as isize);
    let info = rd::<GPtr<u8>>(slot, 0);
    if info.is_null() {
        return 0;
    }
    set_break_info_call(this, info, just_faces);
    let ltm = rw_frame_get_ltm(rd::<GPtr<u8>>(rw, 4));
    set_group_data_call(this, ltm, vel, vel_rand);
    wr::<u8>(this, 0, just_faces as u8);
    wr::<u8>(this, 1, 1);
    wr::<i32>(this, 0xC, 0);
    wr::<u8>(this, 2, rd::<u8>(rd::<GPtr<u8>>(obj, 0x160), 0x4C));
    wr::<u8>(this, 3, ((rd::<u32>(obj, 0x1C) >> 14) & 1) as u8);
    let info = rd::<GPtr<u8>>(slot, 0);
    let mi = i32::from(rd::<i16>(obj, 0x22));
    let col = || rd::<GPtr<u8>>(g::<GPtr<u8>>((0xA9_B0C8 + mi * 4) as u32 as usize).read(), 0x14);
    let quarter = f32::from_bits(0x3E80_0000);
    let mut pos = if rd::<u32>(info, 0) == 0 {
        let z = x(rd::<f32>(ltm, 0x38)).fadd_m(rd::<f32>(col(), 8)).fadd_m(quarter).to_f32();
        [rd::<f32>(ltm, 0x30), rd::<f32>(ltm, 0x34), z]
    } else {
        let c = col();
        let mn = [rd::<f32>(c, 0), rd::<f32>(c, 4), rd::<f32>(c, 8)];
        let mx = [rd::<f32>(c, 0xC), rd::<f32>(c, 0x10), rd::<f32>(c, 0x14)];
        let mut centre = [0f32; 3];
        for k in 0..3 {
            centre[k] = x(mx[k]).fsub_m(mn[k]).fmul_m(0.5).fadd_m(mn[k]).to_f32();
        }
        centre[2] = x(mx[2]).fsub_m(mn[2]).fmul_m(0.5).fadd_m(mn[2]).fadd_m(quarter).to_f32();
        let mut out = [0f32; 3];
        let r = matrix_mul_vec(p(&mut out), rd::<GPtr<u8>>(obj, 0x14), p(&mut centre));
        [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)]
    };
    let mut cp = [0u32; 11];
    let mut ent = 0u32;
    if process_vertical_line(p(&mut pos), 0xC47A_0000, p(&mut cp), p(&mut ent), 1, 0, 0, 0, 0, 0, 0) != 0 {
        wr::<u32>(this, 0x10, cp[2]);
        wr::<u32>(this, 0x14, cp[4]);
        wr::<u32>(this, 0x18, cp[5]);
        wr::<u32>(this, 0x1C, cp[6]);
        let small = |o: isize| x(rd::<f32>(this, o)).fabs().fcom(x(f32::from_bits(0x3C23_D70A))) == Cmp::Less;
        if small(0x14) && small(0x18) && small(0x1C) {
            wr::<u32>(this, 0x18, 0);
            wr::<u32>(this, 0x1C, 0x3F80_0000);
            wr::<u32>(this, 0x14, 0);
        }
    } else {
        wr::<u32>(this, 0x10, 0xC47A_0000);
        wr::<u32>(this, 0x14, 0);
        wr::<u32>(this, 0x18, 0);
        wr::<u32>(this, 0x1C, 0x3F80_0000);
    }
    1
}

/// `BreakObject_c::CalcGroupCenter(BreakGroup_t* group)` (0x59D190): the bounding box of every
/// triangle vertex (min/max starting at +-9999999; the max z lives in ST0 the whole time), the
/// box centre (`(max + min) * 0.5`) subtracted from every vertex and the box, the centre turned by
/// the group matrix (`RwV3dTransformVector`) and added to its position, then the facing axis is
/// the box's thinnest side (width if <= length and height, else length if <= width and height,
/// else height if <= length and width; none on unordered sizes) with half of it as the bounding
/// size.
#[hook(addr = 0x59D190, conv = thiscall, name = "BreakObject_c::CalcGroupCenter", this = "BreakObject_c",
       expect = "83 EC 34 D9 05 60 6E 86 00",
       unverified = "world5-1", route = "L2", hazards = "heap-writes",
       note = "writes the group's render infos (heap)")]
pub fn calc_group_center(_this: GPtr<u8>, grp: GPtr<u8>) {
    let big = f32::from_bits(0x4B18_967F);
    let nbig = f32::from_bits(0xCB18_967F);
    let mut mn = [big; 3];
    let mut mx = [nbig; 2];
    let mut mz = x(nbig);
    let n = usize::from(rd::<u16>(grp, 0x4E));
    let ri = rd::<GPtr<u8>>(grp, 0x50);
    for t in 0..n {
        for v in 0..3 {
            let base = (t * 0x48 + v * 0xC) as isize;
            for k in 0..2 {
                let c = rd::<f32>(ri, base + 4 * k as isize);
                if !x(c).fcom(x(mn[k])).test_parity(0x05) {
                    mn[k] = c;
                }
                if !x(c).fcom(x(mx[k])).test_any(0x41) {
                    mx[k] = c;
                }
            }
            let z = rd::<f32>(ri, base + 8);
            if !x(z).fcom(x(mn[2])).test_parity(0x05) {
                mn[2] = z;
            }
            if !mz.fcom(x(z)).test_parity(0x05) {
                mz = x(z);
            }
        }
    }
    let half = f32::from_bits(0x3F00_0000);
    let mut c = [
        x(mx[0]).fadd_m(mn[0]).fmul_m(half).to_f32(),
        x(mx[1]).fadd_m(mn[1]).fmul_m(half).to_f32(),
        x(mn[2]).fadd(mz).fmul_m(half).to_f32(),
    ];
    let mut t = 0usize;
    while t < usize::from(rd::<u16>(grp, 0x4E)) {
        for k in 0..9 {
            let o = (t * 0x48 + k * 4) as isize;
            let ri = rd::<GPtr<u8>>(grp, 0x50);
            wr::<f32>(ri, o, x(rd::<f32>(ri, o)).fsub_m(c[k % 3]).to_f32());
        }
        t += 1;
    }
    for k in 0..3 {
        mn[k] = x(mn[k]).fsub_m(c[k]).to_f32();
    }
    for k in 0..2 {
        mx[k] = x(mx[k]).fsub_m(c[k]).to_f32();
    }
    let mxz = mz.fsub_m(c[2]).to_f32();
    let mut out = [0f32; 3];
    rw_v3d_transform_vector(p(&mut out), p(&mut c), grp);
    for (k, o) in out.iter().enumerate() {
        let off = 0x30 + 4 * k as isize;
        wr::<f32>(grp, off, x(*o).fadd_m(rd::<f32>(grp, off)).to_f32());
    }
    let w = x(mx[0]).fsub_m(mn[0]).to_f32();
    let l = x(mx[1]).fsub_m(mn[1]);
    let h = x(mxz).fsub_m(mn[2]).to_f32();
    let le = |a: X87, b: X87| !a.fcom(b).test_parity(0x41);
    if le(x(w), l) && le(x(w), x(h)) {
        wr::<u8>(grp, 0x58, 0);
        wr::<f32>(grp, 0x5C, x(w).fmul_m(half).to_f32());
    } else if le(l, x(w)) && le(l, x(h)) {
        wr::<u8>(grp, 0x58, 1);
        wr::<f32>(grp, 0x5C, l.fmul_m(half).to_f32());
    } else if le(x(h), l) && le(x(h), x(w)) {
        wr::<u8>(grp, 0x58, 2);
        wr::<f32>(grp, 0x5C, x(h).fmul_m(half).to_f32());
    }
}

/// `BreakObject_c::SetGroupData(const RwMatrix* m, const CVector* vel, float velRand)` (0x59D570):
/// per group: copy the matrix, centre it (CalcGroupCenter), velocity = `vel` plus (when velRand is
/// not 0) or else only a random value in `[-velRand, velRand)` per axis (`(r - -r) * rand / 32767 +
/// -r`), rotation speed 3 + 0..3, a random normalised rotation axis, moving.
#[hook(addr = 0x59D570, conv = thiscall, name = "BreakObject_c::SetGroupData", this = "BreakObject_c",
       expect = "83 EC 20 53 55",
       unverified = "world5-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x59D5CC/0x59D5FE/0x59D630 (no velocity), 0x59D693/0x59D6C9/0x59D6FF (velocity, velRand != 0), 0x59D72D, 0x59D753/0x59D775/0x59D797")]
pub fn set_group_data(this: GPtr<u8>, m: GPtr<u8>, vel: GPtr<u8>, vel_rand: u32) {
    let r = f32::from_bits(vel_rand);
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        let grp = || rd::<GPtr<u8>>(this, 8).byte_add(i as isize * 0x74);
        for k in 0..16isize {
            wr::<u32>(grp(), 4 * k, rd::<u32>(m, 4 * k));
        }
        calc_group_center_call(this, grp());
        if vel.is_null() {
            let nr = x(r).fchs().to_f32();
            for (o, site) in [(0x40, 0x59_D5CC), (0x44, 0x59_D5FE), (0x48, 0x59_D630)] {
                let t = xi(rnd(site)).fmul_m(RAND_K);
                wr::<f32>(grp(), o, x(r).fsub_m(nr).fmul(t).fadd_m(nr).to_f32());
            }
        } else {
            let c = x(r).fcom(x(0.0));
            for k in 0..3isize {
                wr::<u32>(grp(), 0x40 + 4 * k, rd::<u32>(vel, 4 * k));
            }
            if c.test_parity(0x44) {
                let nr = x(r).fchs().to_f32();
                for (o, site) in [(0x40, 0x59_D693), (0x44, 0x59_D6C9), (0x48, 0x59_D6FF)] {
                    let t = xi(rnd(site)).fmul_m(RAND_K);
                    let v = x(r).fsub_m(nr).fmul(t).fadd_m(rd::<f32>(grp(), o)).fadd_m(nr);
                    wr::<f32>(grp(), o, v.to_f32());
                }
            }
        }
        let rot = xi(rnd(0x59_D72D)).fmul_m(RAND_K).fmul_m(3.0).fadd_m(3.0);
        wr::<f32>(grp(), 0x60, rot.to_f32());
        wr::<f32>(grp(), 0x64, rand_signed(0x59_D753));
        wr::<f32>(grp(), 0x68, rand_signed(0x59_D775));
        wr::<f32>(grp(), 0x6C, rand_signed(0x59_D797));
        let axis = grp().byte_add(0x64);
        rw_v3d_normalize(axis, axis);
        wr::<u8>(grp(), 0x4C, 0);
        i += 1;
    }
}

/// One colour channel: `min(255, vertexColour * materialColour + ambient)` truncated (`_ftol`).
fn channel(vc: u8, mat: f32, ambient: usize) -> u8 {
    let v = xi(i32::from(vc)).fmul_m(mat).fadd_m(g::<f32>(ambient).read());
    let v = if x(255.0).fcom(v) == Cmp::Less { x(255.0) } else { v };
    ftol32(v) as u8
}

/// `BreakObject_c::SetBreakInfo(BreakInfo_t* info, int32 justFaces)` (0x59D7F0): one group per
/// triangle (justFaces == 1 exactly) or per material, allocated with `operator new`; each group gets
/// its triangle count's render infos (1, or the triangles of that material) and 256 + 0..31 frames
/// to live (`256 - ftol(r * -32)`). Then every triangle is appended to its group: the three vertex
/// positions, the texture of its material (the face's material with justFaces, else the group
/// index), vertex colour times material colour plus the frame's ambient light * 255 (refreshed into
/// the statics 0x8D0A0C/08/04 unless a vision effect is on) clamped to 255, and the UVs. Finally
/// every group texture's reference count is raised.
#[hook(addr = 0x59D7F0, conv = thiscall, name = "BreakObject_c::SetBreakInfo", this = "BreakObject_c",
       expect = "8B 44 24 08 83 EC 20",
       unverified = "world5-1", route = "split", hazards = "heap-writes",
       note = "operator new for the groups and render infos; rand at_site 0x59D89D; raises texture refcounts")]
pub fn set_break_info(this: GPtr<u8>, info: GPtr<u8>, just_faces: i32) {
    let jf1 = just_faces == 1;
    let num = i32::from(if jf1 { rd::<u16>(info, 0x14) } else { rd::<u16>(info, 0x20) });
    wr::<i32>(this, 4, num);
    let groups = operator_new(rd::<i32>(this, 4).wrapping_mul(0x74) as u32);
    wr::<GPtr<u8>>(this, 8, groups);
    let grp = |i: usize| rd::<GPtr<u8>>(this, 8).byte_add((i * 0x74) as isize);
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        wr::<u16>(grp(i as usize), 0x4E, 0);
        wr::<u32>(grp(i as usize), 0x54, 0);
        let mut cnt = 0i32;
        if jf1 {
            cnt = 1;
        } else {
            let n = usize::from(rd::<u16>(info, 0x14));
            let idx = rd::<GPtr<u8>>(info, 0x1C);
            for k in 0..n {
                if i32::from(rd::<u16>(idx, 2 * k as isize)) == i {
                    cnt += 1;
                }
            }
        }
        let ri = operator_new(cnt.wrapping_mul(0x48) as u32);
        wr::<GPtr<u8>>(grp(i as usize), 0x50, ri);
        let f = 0x100i32.wrapping_sub(rand_frames(0x59_D89D));
        wr::<i32>(grp(i as usize), 0x70, f);
        i += 1;
    }
    let mut t = 0usize;
    while t < usize::from(rd::<u16>(info, 0x14)) {
        let gi = if jf1 { t } else { usize::from(rd::<u16>(rd::<GPtr<u8>>(info, 0x1C), 2 * t as isize)) };
        let nt = usize::from(rd::<u16>(grp(gi), 0x4E));
        let tri = rd::<GPtr<u8>>(info, 0x18).byte_add((t * 6) as isize);
        let vi = [usize::from(rd::<u16>(tri, 0)), usize::from(rd::<u16>(tri, 2)), usize::from(rd::<u16>(tri, 4))];
        let ri = || rd::<GPtr<u8>>(grp(gi), 0x50).byte_add((nt * 0x48) as isize);
        for (j, v) in vi.iter().enumerate() {
            for k in 0..3usize {
                let val = rd::<u32>(rd::<GPtr<u8>>(info, 8), (v * 12 + k * 4) as isize);
                wr::<u32>(ri(), (j * 12 + k * 4) as isize, val);
            }
        }
        let tex = if jf1 { usize::from(rd::<u16>(rd::<GPtr<u8>>(info, 0x1C), 2 * t as isize)) } else { gi };
        wr::<u32>(grp(gi), 0x54, rd::<u32>(rd::<GPtr<u8>>(info, 0x24), (tex * 4) as isize));
        let mat = rd::<GPtr<u8>>(info, 0x30).byte_add((tex * 12) as isize);
        let mc = [rd::<f32>(mat, 0), rd::<f32>(mat, 4), rd::<f32>(mat, 8)];
        if is_vision_fx_active() == 0 {
            let k255 = 255.0f32;
            g::<f32>(0x8D_0A0C).write(x(g::<f32>(0xC8_86D4).read()).fmul_m(k255).to_f32());
            g::<f32>(0x8D_0A08).write(x(g::<f32>(0xC8_86D8).read()).fmul_m(k255).to_f32());
            g::<f32>(0x8D_0A04).write(x(g::<f32>(0xC8_86DC).read()).fmul_m(k255).to_f32());
        }
        let amb = [0x8D_0A0C, 0x8D_0A08, 0x8D_0A04];
        for (j, v) in vi.iter().enumerate() {
            for k in 0..3usize {
                let vc = rd::<u8>(rd::<GPtr<u8>>(info, 0x10), (v * 4 + k) as isize);
                wr::<u8>(ri(), (0x3C + j * 4 + k) as isize, channel(vc, mc[k], amb[k]));
            }
        }
        for (j, v) in vi.iter().enumerate() {
            for k in 0..2usize {
                let uv = rd::<u32>(rd::<GPtr<u8>>(info, 0xC), (v * 8 + k * 4) as isize);
                wr::<u32>(ri(), (0x24 + j * 8 + k * 4) as isize, uv);
            }
        }
        wr::<u16>(grp(gi), 0x4E, rd::<u16>(grp(gi), 0x4E).wrapping_add(1));
        t += 1;
    }
    let mut i = 0i32;
    while i < rd::<i32>(this, 4) {
        let tex = rd::<GPtr<u8>>(grp(i as usize), 0x54);
        if !tex.is_null() {
            wr::<i32>(tex, 0x54, rd::<i32>(tex, 0x54).wrapping_add(1));
        }
        i += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_DO_COLLISION_RESPONSE,
    HOOK_DO_COLLISION,
    HOOK_UPDATE,
    HOOK_RENDER,
    HOOK_INIT,
    HOOK_CALC_GROUP_CENTER,
    HOOK_SET_GROUP_DATA,
    HOOK_SET_BREAK_INFO,
];
