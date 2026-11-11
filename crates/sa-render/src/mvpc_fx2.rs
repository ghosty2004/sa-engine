//! MVP-C writer batch `mvpc-fx-2`: FX emitter / particle runtime (`FxEmitter_c::AddParticle`). Ported from
//! the 1.0 US bytes; gta-reversed as a reading aid. Every hook is unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, rd, wr};

orig! {
    /// `FxInfoManager_c::ProcessEmissionInfo(currTime, timeStep, totalTime, useConst, out)` (0x4A4960)
    pub fn process_emission_info(this: GPtr<u8>, curr: u32, step: u32, total: u32, use_const: u32, out: GPtr<u8>) = thiscall 0x4A4960 => crate::mvpc_fx1::info_manager_process_emission_info;
    /// `FxManager_c::FxRwMatrixCreate()` (0x4A9440)
    pub fn fx_rw_matrix_create(this: GPtr<u8>) -> u32 = thiscall 0x4A9440 => crate::fx::fx_manager_fx_rw_matrix_create;
    /// `FxManager_c::FxRwMatrixDestroy(RwMatrix*)` (0x4A9460)
    pub fn fx_rw_matrix_destroy(this: GPtr<u8>, m: u32) = thiscall 0x4A9460 => crate::fx::fx_manager_fx_rw_matrix_destroy;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    pub fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwMatrixMultiply(out, a, b)` (0x7F18B0)
    pub fn rw_matrix_multiply(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => unported;
    /// `FxPrimBP_c::GetRWMatrix(RwMatrix* out)` (0x4A9DC0)
    pub fn prim_bp_get_rw_matrix(this: GPtr<u8>, out: GPtr<u8>) = thiscall 0x4A9DC0 => unported;
    /// `FxEmitter_c::CreateParticle(EmissionInfo_t*, RwMatrix*, a2, a3, a4, a6, a7)` (0x4A2580)
    pub fn create_particle(this: GPtr<u8>, ei: GPtr<u8>, m: GPtr<u8>, a2: u32, a3: u32, a4: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x4A2580 => unported;
}

/// `g_fxMan` (FxManager_c, 0xA9AE80).
const FX_MAN: usize = 0xA9AE80;

fn fx_man() -> GPtr<u8> {
    GPtr::from_addr(FX_MAN)
}

fn mcreate() -> GPtr<u8> {
    GPtr::from_addr(fx_rw_matrix_create(fx_man()) as usize)
}

fn mdestroy(m: GPtr<u8>) {
    fx_rw_matrix_destroy(fx_man(), m.addr() as u32);
}

/// `FxEmitter_c::AddParticle(RwV3d* pos, a2, a3, a4, float brightness, a6, a7)` (0x4A3EA0, thiscall, `ret
/// 0x1C`; the matrix overload): `ProcessEmissionInfo(0, 0, bp.length, sys.byte52, &ei)` of the prim's
/// info manager (prim +4 → +0x2C); three pooled matrices m1, m2, m3; m3 = identity at `*pos` (flags +0xC
/// `|= 0x20003`), `RwMatrixUpdate`; m2 = m3 (16 dwords) without a parent matrix (sys +0xC), else
/// `RwMatrixMultiply(m2, m3, parent)`; a fourth pooled m4 = `prim->GetRWMatrix`, `RwMatrixMultiply(m1,
/// m4, m2)`, m4 returned. `p = CreateParticle(&ei, m1, a2, a3, a4, a6, a7)`; non-null and brightness not
/// below 0 (`fcomp`, `test ah, 1`: NaN counts as below) → byte +0x36 = `_ftol(brightness * 0.5)`.
/// Returns m3, m2, m1 to the pool.
#[hook(addr = 0x4A3EA0, conv = thiscall, name = "FxEmitter_c::AddParticle_Reversed-matrix", expect = "83 EC 70 53 55",
       unverified = "mvpc-fx-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "uses four pooled FX matrices; creates a particle")]
#[allow(clippy::too_many_arguments)]
pub fn emitter_add_particle(this: GPtr<u8>, pos: GPtr<u8>, a2: u32, a3: u32, a4: u32, brightness: u32, a6: u32, a7: u32) {
    let mut ei = [0u32; 0x20];
    let eip = GPtr::from_raw(ei.as_mut_ptr()).cast::<u8>();
    let sys = rd::<GPtr<u8>>(this, 8);
    let bp = rd::<GPtr<u8>>(sys, 8);
    let prim = rd::<GPtr<u8>>(this, 4);
    process_emission_info(prim.byte_add(0x2C), 0, 0, rd::<u32>(bp, 0xC), u32::from(rd::<u8>(sys, 0x52)), eip);
    let m1 = mcreate();
    let m2 = mcreate();
    let m3 = mcreate();
    let flags = rd::<u32>(m3, 0xC);
    for o in [0x28, 0x14, 0] {
        wr::<u32>(m3, o, 0x3F80_0000);
    }
    for o in [0x38, 0x34, 0x30, 0x10, 8, 4, 0x24, 0x20, 0x18] {
        wr::<u32>(m3, o, 0);
    }
    wr::<u32>(m3, 0xC, flags | 0x20003);
    for o in [0, 4, 8] {
        wr::<u32>(m3, 0x30 + o, rd::<u32>(pos, o));
    }
    rw_matrix_update(m3);
    let parent = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0xC);
    if parent.is_null() {
        for o in 0..16isize {
            wr::<u32>(m2, 4 * o, rd::<u32>(m3, 4 * o));
        }
    } else {
        rw_matrix_multiply(m2, m3, parent);
    }
    let m4 = mcreate();
    prim_bp_get_rw_matrix(rd::<GPtr<u8>>(this, 4), m4);
    rw_matrix_multiply(m1, m4, m2);
    mdestroy(m4);
    let p = create_particle(this, eip, m1, a2, a3, a4, a6, a7);
    if !p.is_null() {
        let b = X87::from_f32(f32::from_bits(brightness));
        if !b.fcom(X87::zero()).test_any(1) {
            wr::<u8>(p, 0x36, ftol32(b.fmul_m(f32::from_bits(0x3F00_0000))) as u8);
        }
    }
    mdestroy(m3);
    mdestroy(m2);
    mdestroy(m1);
}

orig! {
    /// `FxManager_c::GetParticle(bool)` (0x4A93C0)
    pub fn get_particle(this: GPtr<u8>, flag: u32) -> GPtr<u8> = thiscall 0x4A93C0 => crate::fx::fx_manager_get_particle;
    /// `FxManager_c::FreeUpParticle()` (0x4A9400)
    pub fn free_up_particle(this: GPtr<u8>) = thiscall 0x4A9400 => unported;
    /// `RwV3dNormalize(RwV3d* out, const RwV3d* in)` (0x7ED9B0): the length in ST0
    pub fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwV3dTransformVectors(out, in, n, m)` (0x7EDDF0)
    pub fn rw_v3d_transform_vectors(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDDF0 => unported;
    /// `Fx_c::RotateVecIntoVec(out, in, dir)` (0x4A1660)
    pub fn rotate_vec_into_vec(out: GPtr<u8>, inp: GPtr<u8>, dir: GPtr<u8>) = cdecl 0x4A1660 => crate::mvpc_fx1::fx_rotate_vec_into_vec;
    /// `FxEmitterBP_c::UpdateParticle(float time, FxEmitterPrt_c*)` (0x4A21D0)
    pub fn update_particle(this: GPtr<u8>, t: u32, prt: GPtr<u8>) -> u8 = thiscall 0x4A21D0 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    pub fn list_add_item(this: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
}

fn xf(v: f32) -> X87 {
    X87::from_f32(v)
}

fn kf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `rand() % 10000` (`cdq; idiv`), the call attributed to its original return address.
fn r10k(ret: u32) -> i32 {
    sa_core::orig::at_site(ret, sa_core::crt::rand).wrapping_rem(10000)
}

/// `(rand() % 10000) * 0.0002f - 1.0f` (x87).
fn rand_sym(ret: u32) -> X87 {
    X87::from_i32(r10k(ret)).fmul_m(kf(0x3951_B717)).fsub_m(kf(0x3F80_0000))
}

/// `(rand() % 10000) * 0.0001f` (x87).
fn rand_unit(ret: u32) -> X87 {
    X87::from_i32(r10k(ret)).fmul_m(kf(0x38D1_B717))
}

/// `sinTable[_ftol(v) & 0xFF]` (0xBB3E00).
fn sin_tab(v: X87) -> f32 {
    GPtr::<f32>::from_addr(0xBB3E00).wrapping_index(ftol32(v) & 0xFF).read()
}

/// `FxEmitter_c::CreateParticle(EmissionInfo_t* ei, RwMatrix* m, RwV3d* vel, float time, FxPrtMult_c*
/// mult, float brightness, bool local)` (0x4A2580, thiscall, `ret 0x1C`). `p = g_fxMan.GetParticle(0)`;
/// none: when the system allows it (sys +0x62 & 0x20) `FreeUpParticle` and retry, else null. Life +8 =
/// `(sym * ei[0x40] + ei[0x3C]) * mult[6]` (sym = `rand%10000 * 0.0002 - 1`), +0xC = 0, +0x28 = sys;
/// colour bytes +0x2C..+0x31 = `_ftol(mult[i] * 255)`, +0x37 = local, +0x32..+0x34 = `_ftol(rand%10000
/// * 0.0001 * 255)`, +0x35 = `_ftol(brightness * 100)`, +0x38 = `u * (ei[0x54] - ei[0x50]) + ei[0x50]`,
/// +0x36 = 0xFF; local → `prim->GetRWMatrix(m)`. Offset: with `|ei[4]| >= 0.001` (NaN: the per-axis path) a random
/// direction (x, y, z f32) scaled by `1/len` (x87 `fsqrt`, `fdivr` 1.0) times — for `ei[4] >= 0` — a
/// random `u` (the scale stored f32 first), times `ei[4]`; else per axis `u * (max - min) + min` from
/// ei +8..+0x1C. The local position `(offset + ei[0x44..0x4C])` is transformed by m into +0x10..+0x18.
/// Velocity +0x1C..+0x24: `*vel` when given; else a random cone direction (angles from the sine table at
/// 0xBB3E00, `ei[0x34..0x38]` degrees) rotated into ei[0x28..0x30] (normalised; > 10 in x: the
/// particle position) transformed by m, times `sym * ei[0x24] + ei[0x20]`; plus sys +0x68..+0x70.
/// `prim->UpdateParticle(time, p)`, `List_c::AddItem(prim + 0x20, p)`; returns p. The original keeps
/// several temporaries in its own argument slots (+4, +8, +0x14, +0x1C).
#[hook(addr = 0x4A2580, conv = thiscall, name = "FxEmitter_c::CreateParticle", expect = "83 EC 34 55 56 8B E9",
       unverified = "mvpc-fx-2", route = "L2", hazards = "x87-transcendental,stack-residue,heap-writes", recs = "grove-walk",
       note = "up to 11 rand() calls; fsqrt; scratch in its own argument slots (no footprint recorded)")]
#[allow(clippy::too_many_arguments)]
pub fn emitter_create_particle(this: GPtr<u8>, ei: GPtr<u8>, m: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, brightness: u32, local: u32) -> GPtr<u8> {
    let mut p = get_particle(fx_man(), 0);
    if p.is_null() {
        if rd::<u8>(rd::<GPtr<u8>>(this, 8), 0x62) & 0x20 == 0 {
            return GPtr::NULL;
        }
        free_up_particle(fx_man());
        p = get_particle(fx_man(), 0);
        if p.is_null() {
            return GPtr::NULL;
        }
    }
    let e = |o: isize| rd::<f32>(ei, o);
    let life = rand_sym(0x4A25CC).fmul_m(e(0x40)).fadd_m(e(0x3C)).fmul_m(rd::<f32>(mult, 0x18));
    wr::<u32>(p, 0xC, 0);
    wr::<f32>(p, 8, life.to_f32());
    wr::<GPtr<u8>>(p, 0x28, rd::<GPtr<u8>>(this, 8));
    for i in 0..6isize {
        wr::<u8>(p, 0x2C + i, ftol32(xf(rd::<f32>(mult, 4 * i)).fmul_m(kf(0x437F_0000))) as u8);
    }
    wr::<u8>(p, 0x37, local as u8);
    for (o, ret) in [(0x32isize, 0x4A267Au32), (0x33, 0x4A26A3), (0x34, 0x4A26CC)] {
        wr::<u8>(p, o, ftol32(rand_unit(ret).fmul_m(kf(0x437F_0000))) as u8);
    }
    wr::<u8>(p, 0x35, ftol32(xf(f32::from_bits(brightness)).fmul_m(kf(0x42C8_0000))) as u8);
    let u = rand_unit(0x4A2707);
    let v = xf(e(0x54)).fsub_m(e(0x50));
    wr::<u8>(p, 0x36, 0xFF);
    wr::<f32>(p, 0x38, u.fmul(v).fadd_m(e(0x50)).to_f32());
    if rd::<u8>(p, 0x37) != 0 {
        prim_bp_get_rw_matrix(rd::<GPtr<u8>>(this, 4), m);
    }
    let (x, y, z): (f32, f32, X87);
    let thr = X87::from_f64_exact(f64::from(kf(0x3A83_126F)));
    if !xf(e(4)).fabs().fcom(thr).test_any(1) {
        let rx = rand_sym(0x4A275E).to_f32();
        let ry = rand_sym(0x4A2783).to_f32();
        let rz = rand_sym(0x4A27A8).to_f32();
        // fcomp 0.0, test ah 5, jnp: ei[4] < 0 keeps the plain inverse length
        let randomize = xf(e(4)).fcom(X87::zero()).test_parity(5);
        let len = xf(rz).fmul_m(rz).fadd(xf(ry).fmul_m(ry)).fadd(xf(rx).fmul_m(rx)).fsqrt();
        let mut s = X87::one().fdiv(len);
        if randomize {
            let inv = s.to_f32();
            s = rand_unit(0x4A2805).fmul_m(inv);
        }
        let s = s.fmul_m(e(4));
        x = xf(rx).fmul(s).to_f32();
        y = xf(ry).fmul(s).to_f32();
        z = s.fmul_m(rz);
    } else {
        x = rand_unit(0x4A2841).fmul(xf(e(0x14)).fsub_m(e(8))).fadd_m(e(8)).to_f32();
        y = rand_unit(0x4A286B).fmul(xf(e(0x18)).fsub_m(e(0xC))).fadd_m(e(0xC)).to_f32();
        z = rand_unit(0x4A2895).fmul(xf(e(0x1C)).fsub_m(e(0x10))).fadd_m(e(0x10));
    }
    let lx = xf(x).fadd_m(e(0x44));
    let ly = xf(y).fadd_m(e(0x48));
    let lz = z.fadd_m(e(0x4C));
    let mm = |o: isize| rd::<f32>(m, o);
    for (j, (c0, c1, c2, c3)) in [(0x20isize, 0x10isize, 0isize, 0x30isize), (0x24, 0x14, 4, 0x34), (0x28, 0x18, 8, 0x38)].into_iter().enumerate() {
        let v = lz.fmul_m(mm(c0)).fadd(ly.fmul_m(mm(c1))).fadd(lx.fmul_m(mm(c2))).fadd_m(mm(c3));
        wr::<f32>(p, 0x10 + 4 * j as isize, v.to_f32());
    }
    if !vel.is_null() {
        for o in [0, 4, 8] {
            wr::<u32>(p, 0x1C + o, rd::<u32>(vel, o));
        }
    } else {
        let a = rand_unit(0x4A2936).fmul_m(kf(0x40C9_0FD0)).to_f32();
        let b0 = xf(e(0x34)).fmul_m(kf(0x3C8E_FA2E)).to_f32();
        let ang = rand_unit(0x4A2968).fmul(xf(e(0x38)).fmul_m(kf(0x3C8E_FA2E)).fsub_m(b0)).fadd_m(b0);
        let av = ang.fmul_m(kf(0x4222_F983));
        let sin_a = sin_tab(av);
        let bv = xf(a).fmul_m(kf(0x4222_F983));
        let b32 = bv.to_f32();
        let mut dirv = [0f32; 3];
        dirv[0] = xf(sin_tab(bv.fadd_m(kf(0x4280_0000)))).fmul_m(sin_a).to_f32();
        dirv[1] = sin_tab(av.fadd_m(kf(0x4280_0000)));
        dirv[2] = xf(sin_tab(xf(b32))).fmul_m(sin_a).to_f32();
        let mut d = [0u32; 3];
        if !xf(e(0x28)).fcom(xf(kf(0x4120_0000))).test_any(0x41) {
            for (j, c) in d.iter_mut().enumerate() {
                *c = rd::<u32>(p, 0x10 + 4 * j as isize);
            }
        } else {
            for (j, c) in d.iter_mut().enumerate() {
                *c = rd::<u32>(ei, 0x28 + 4 * j as isize);
            }
            let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<u8>();
            rw_v3d_normalize(dp, dp);
        }
        let mut t = [0u32; 3];
        let mut r = [0f32; 3];
        let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<u8>();
        let tp = GPtr::from_raw(t.as_mut_ptr()).cast::<u8>();
        rw_v3d_transform_vectors(tp, dp, 1, m);
        rotate_vec_into_vec(GPtr::from_raw(r.as_mut_ptr()).cast(), GPtr::from_raw(dirv.as_mut_ptr()).cast(), tp);
        let speed = rand_sym(0x4A2A91).fmul_m(e(0x24)).fadd_m(e(0x20));
        for (j, c) in r.iter().enumerate() {
            wr::<f32>(p, 0x1C + 4 * j as isize, xf(*c).fmul(speed).to_f32());
        }
    }
    for (j, o) in [0x68isize, 0x6C, 0x70].into_iter().enumerate() {
        let po = 0x1C + 4 * j as isize;
        let s = rd::<GPtr<u8>>(this, 8);
        wr::<f32>(p, po, xf(rd::<f32>(s, o)).fadd_m(rd::<f32>(p, po)).to_f32());
    }
    update_particle(rd::<GPtr<u8>>(this, 4), time, p);
    list_add_item(rd::<GPtr<u8>>(this, 4).byte_add(0x20), p);
    p
}

orig! {
    /// `FxPrtMult_c::FxPrtMult_c()` (0x4AB270)
    pub fn prt_mult_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AB270 => unported;
    /// `FxEmitter_c::CreateParticle(...)` (0x4A2580)
    pub fn create_particle_call(this: GPtr<u8>, ei: GPtr<u8>, m: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, brightness: u32, local: u32) -> GPtr<u8> = thiscall 0x4A2580 => emitter_create_particle;
}

/// `FxEmitter_c::CreateParticles(float currTime, float timeStep)` (0x4A41E0, thiscall, `ret 8`):
/// `ProcessEmissionInfo(currTime, timeStep, bp.length, sys.byte52, &ei)`. LOD factor from the system's
/// camera distance (sys +0x58) against the prim's LOD start / end words (+0x38, +0x3A, `* 1/64`, f32):
/// below start 1, beyond end 0, else `1 - (d - lo) / (hi - lo)`; the accumulator +0x10 += `ei.rate * f *
/// (sys word +0x5E * 0.001)` (f32). Nothing more unless `[0xC812F0]` lies in [ei+0x58, ei+0x5C],
/// `[0xC81324]` in [ei+0x60, ei+0x64] (x87 compares, NaN passes) and the accumulator is >= 1 (or NaN).
/// Then m2 = the system matrix (+0x10, `RwMatrixUpdate` first; times the parent +0xC when set), m1 =
/// `prim->GetRWMatrix * m2` (a pooled scratch), and while `i < _ftol(acc)` (re-evaluated):
/// `FxPrtMult_c` defaults, `CreateParticle(&ei, m1, null, (f32)(i / acc * timeStep), &mult, 1.2,
/// (sys +0x62 >> 1) & 1)`. Finally `acc -= _ftol(acc)`; the two matrices go back to the pool.
#[hook(addr = 0x4A41E0, conv = thiscall, name = "FxEmitter_c::CreateParticles", expect = "81 EC 8C 00 00 00",
       unverified = "mvpc-fx-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "creates particles (CreateParticle calls rand); uses pooled FX matrices")]
pub fn emitter_create_particles(this: GPtr<u8>, curr: u32, step: u32) {
    let mut ei = [0u32; 0x1A];
    let eip = GPtr::from_raw(ei.as_mut_ptr()).cast::<u8>();
    let sys = rd::<GPtr<u8>>(this, 8);
    let bp = rd::<GPtr<u8>>(sys, 8);
    process_emission_info(rd::<GPtr<u8>>(this, 4).byte_add(0x2C), curr, step, rd::<u32>(bp, 0xC), u32::from(rd::<u8>(sys, 0x52)), eip);
    let sys = rd::<GPtr<u8>>(this, 8);
    let prim = rd::<GPtr<u8>>(this, 4);
    let c = X87::from_i32(i32::from(rd::<u16>(sys, 0x5E))).fmul_m(kf(0x3A83_126F));
    let lo = X87::from_i32(i32::from(rd::<u16>(prim, 0x38))).fmul_m(kf(0x3C80_0000)).to_f32();
    let hi = X87::from_i32(i32::from(rd::<u16>(prim, 0x3A))).fmul_m(kf(0x3C80_0000)).to_f32();
    let d = rd::<f32>(sys, 0x58);
    let f = if !crate::common::jp(xf(d), xf(lo), 5) {
        xf(kf(0x3F80_0000))
    } else if !crate::common::jne(xf(d), xf(hi), 0x41) {
        X87::zero()
    } else {
        xf(d).fsub_m(lo).fdiv(xf(hi).fsub_m(lo)).fsubr_m(kf(0x3F80_0000))
    };
    let acc = xf(f32::from_bits(ei[0])).fmul(f).fmul(c).fadd_m(rd::<f32>(this, 0x10)).to_f32();
    wr::<f32>(this, 0x10, acc);
    let e = |o: usize| f32::from_bits(ei[o / 4]);
    let g1 = xf(GPtr::<f32>::from_addr(0xC812F0).read());
    let g2 = xf(GPtr::<f32>::from_addr(0xC81324).read());
    let below = |a: X87, b: f32| !a.fcom(xf(b)).test_parity(5);
    let above = |a: X87, b: f32| !a.fcom(xf(b)).test_any(0x41);
    if below(g1, e(0x58)) || above(g1, e(0x5C)) || below(g2, e(0x60)) || above(g2, e(0x64)) || below(xf(acc), kf(0x3F80_0000)) {
        return;
    }
    let m1 = mcreate();
    let m2 = mcreate();
    let sm = rd::<GPtr<u8>>(this, 8).byte_add(0x10);
    rw_matrix_update(sm);
    let sys = rd::<GPtr<u8>>(this, 8);
    let parent = rd::<GPtr<u8>>(sys, 0xC);
    if parent.is_null() {
        for o in 0..16isize {
            wr::<u32>(m2, 4 * o, rd::<u32>(sys, 0x10 + 4 * o));
        }
    } else {
        rw_matrix_multiply(m2, sys.byte_add(0x10), parent);
    }
    let m3 = mcreate();
    prim_bp_get_rw_matrix(rd::<GPtr<u8>>(this, 4), m3);
    rw_matrix_multiply(m1, m3, m2);
    mdestroy(m3);
    let mut i = 0i32;
    while i < ftol32(xf(rd::<f32>(this, 0x10))) {
        let mut mult = [0u32; 7];
        let mp = GPtr::from_raw(mult.as_mut_ptr()).cast::<u8>();
        prt_mult_ctor(mp);
        let t = X87::from_i32(i).fdiv_m(rd::<f32>(this, 0x10)).fmul_m(f32::from_bits(step)).to_f32();
        let local = u32::from((rd::<u8>(rd::<GPtr<u8>>(this, 8), 0x62) >> 1) & 1);
        create_particle_call(this, eip, m1, GPtr::NULL, t.to_bits(), mp, 0x3F99_999A, local);
        i += 1;
    }
    let a = rd::<f32>(this, 0x10);
    let n = ftol32(xf(a));
    wr::<f32>(this, 0x10, X87::from_i32(n).fsubr_m(a).to_f32());
    mdestroy(m2);
    mdestroy(m1);
}

orig! {
    /// `FxInfoManager_c::ProcessMovementInfo(a1, a2, a3, a4, a5, MovementInfo_t*)` (0x4A4A10)
    pub fn process_movement_info(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, out: GPtr<u8>) = thiscall 0x4A4A10 => crate::fx::fx_info_manager_process_movement_info;
    /// `CWaterLevel::GetWaterLevel(x, y, z, float* level, bool touchingWater, bool)` (0x6EB690)
    pub fn get_water_level(x: u32, y: u32, z: u32, level: GPtr<u8>, a: u32, b: u32) -> u8 = cdecl 0x6EB690 => unported;
}

/// `FxEmitterBP_c::UpdateParticle(float time, FxEmitterPrt_c* p)` (0x4A21D0, thiscall, `ret 8`, AL = 1
/// when the particle dies). `dt = sys word +0x60 * 0.001 * time` (x87; stored f32 into the original's
/// `time` slot, the age sum uses the unrounded value): `age = dt + p+0xC` (stored f32); not below life
/// (+8; NaN counts as below) → 1. Otherwise pos +0x10.. += dt32 * vel (+0x1C..); `MovementInfo_t`
/// (pos, vel, 4 rotation floats, 2 flags) from `ProcessMovementInfo(sys+0x54, (f32)(age / life), dt32,
/// bp.length, 0, &mi)` (prim info manager +0x2C), pos / vel copied back. A set flag (+0x28 / +0x29):
/// `GetWaterLevel(pos, &level, 1, 0)`; flag 0 with water: pos.z below level → level; flag 1: no water
/// or pos.z above level → 1. Rotation +0x38 by the four floats w0..w3 (+0x18..+0x24) and the particle
/// bytes b31 (speed), b32 (bias): only w0/w1 positive: `+= ((b32 * (1/255)) * (w1 - w0) + w0) * b31 *
/// dt * (1/255)`; only w2/w3: `-= ((b32 / 255) * (w3 - w2) + w2) * b31 * dt / 255`; both: b32 < 128 →
/// the first with `b32 * (1/128)`, else the second with `(b32 - 128) * (1/128)`. Returns 0.
#[hook(addr = 0x4A21D0, conv = thiscall, name = "FxEmitterBP_c::UpdateParticle", expect = "83 EC 2C 56 8B 74 24 38",
       unverified = "mvpc-fx-2", route = "L2", hazards = "stack-residue,heap-writes,ax-al-return", recs = "grove-walk",
       note = "writes dt / the water level / byte values into its own argument slots (no footprint recorded)")]
pub fn emitter_bp_update_particle(this: GPtr<u8>, time: u32, p: GPtr<u8>) -> u8 {
    let sys = rd::<GPtr<u8>>(p, 0x28);
    let dtx = X87::from_i32(i32::from(rd::<u16>(sys, 0x60))).fmul_m(kf(0x3A83_126F)).fmul_m(f32::from_bits(time));
    let dt = dtx.to_f32();
    let age = dtx.fadd_m(rd::<f32>(p, 0xC));
    let alive = age.fcom(xf(rd::<f32>(p, 8))).test_any(1);
    wr::<f32>(p, 0xC, age.to_f32());
    if !alive {
        return 1;
    }
    for j in 0..3isize {
        let v = xf(dt).fmul_m(rd::<f32>(p, 0x1C + 4 * j)).fadd_m(rd::<f32>(p, 0x10 + 4 * j));
        wr::<f32>(p, 0x10 + 4 * j, v.to_f32());
    }
    let ratio = age.fdiv_m(rd::<f32>(p, 8)).to_f32();
    let mut mi = [0u32; 11];
    for (j, m) in mi.iter_mut().take(6).enumerate() {
        *m = rd::<u32>(p, 0x10 + 4 * j as isize);
    }
    let mip = GPtr::from_raw(mi.as_mut_ptr()).cast::<u8>();
    let len = rd::<u32>(rd::<GPtr<u8>>(sys, 8), 0xC);
    process_movement_info(this.byte_add(0x2C), rd::<u32>(sys, 0x54), ratio.to_bits(), dt.to_bits(), len, 0, mip);
    for (j, v) in mi.iter().take(6).enumerate() {
        wr::<u32>(p, 0x10 + 4 * j as isize, *v);
    }
    let flags = mi[10];
    let (f0, f1) = (flags as u8, (flags >> 8) as u8);
    if f0 != 0 || f1 != 0 {
        let mut level = 0f32;
        let water = get_water_level(rd::<u32>(p, 0x10), rd::<u32>(p, 0x14), rd::<u32>(p, 0x18), GPtr::from_raw(&mut level as *mut f32).cast(), 1, 0);
        if f0 != 0 && water != 0 && !crate::common::jp(xf(rd::<f32>(p, 0x18)), xf(level), 5) {
            wr::<f32>(p, 0x18, level);
        }
        if f1 != 0 && (water == 0 || !crate::common::jne(xf(rd::<f32>(p, 0x18)), xf(level), 0x41)) {
            return 1;
        }
    }
    let w = |j: usize| f32::from_bits(mi[6 + j]);
    let pos = |v: f32| !xf(v).fcom(X87::zero()).test_any(0x41);
    let first = pos(w(0)) || pos(w(1));
    let second = pos(w(2)) || pos(w(3));
    let b31 = i32::from(rd::<u8>(p, 0x31));
    let b32 = i32::from(rd::<u8>(p, 0x32));
    let k255 = kf(0x3B80_8081);
    let k128 = kf(0x3C00_0000);
    let rot = rd::<f32>(p, 0x38);
    let add = |bias: X87, lo: f32, hi: f32| {
        bias.fmul(xf(hi).fsub_m(lo)).fadd_m(lo).fmul(X87::from_i32(b31)).fmul_m(dt).fmul_m(k255)
    };
    if first && !second {
        let v = add(X87::from_i32(b32).fmul_m(k255), w(0), w(1));
        wr::<f32>(p, 0x38, v.fadd_m(rot).to_f32());
    } else if !first && second {
        let v = add(X87::from_i32(b32).fmul_m(k255), w(2), w(3));
        wr::<f32>(p, 0x38, v.fsubr_m(rot).to_f32());
    } else if first && second {
        if b32 < 0x80 {
            let v = add(X87::from_i32(b32).fmul_m(k128), w(0), w(1));
            wr::<f32>(p, 0x38, v.fadd_m(rot).to_f32());
        } else {
            let v = add(X87::from_i32(b32).fsub_m(kf(0x4300_0000)).fmul_m(k128), w(2), w(3));
            wr::<f32>(p, 0x38, v.fsubr_m(rot).to_f32());
        }
    }
    0
}

orig! {
    /// `RwIm3DTransform(verts, n, ltm, flags)` (0x7EF450)
    pub fn rw_im3d_transform(v: GPtr<u8>, n: i32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderPrimitive(type)` (0x7EF6B0)
    pub fn rw_im3d_render_primitive(ty: i32) -> i32 = cdecl 0x7EF6B0 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
}

/// The FX triangle batch: vertex cursor 0xA9AE6C into the 0x24-byte `RwIm3DVertex` buffer 0xC4D958,
/// vertex count 0xA9AE58, triangle count 0xA9AE5C, use-UV flag 0xA9AE64, LTM 0xA9AE68, flags 0xA9AE60.
fn gp<T: Copy>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

/// `Fx_c::RenderAddTri_(x0, y0, z0, x1, y1, z1, x2, y2, z2, u0, v0, u1, v1, u2, v2, r0, g0, b0, a0,
/// r1, g1, b1, a1, r2, g2, b2, a2)` (0x4A1410, cdecl): appends three vertices at the cursor (position
/// +0/+4/+8, colour +0x18 = `((a << 8 | r) << 8 | g) << 8 | b` of the full int arguments). Floats go
/// through `fld`/`fstp` except x1, x2 and u0, which are moved as dwords (as the original does). UVs
/// (+0x1C/+0x20) only with the UV flag. Triangle count + 1; at 0x7FD vertices or 0xFFF triangles the
/// batch is flushed: `RwIm3DTransform(buf, 3 * tris, ltm, flags)`, when it succeeds
/// `RwIm3DRenderPrimitive(3)` + `RwIm3DEnd`; counts 0, cursor back to the buffer.
#[hook(addr = 0x4A1410, conv = cdecl, name = "Fx_c::RenderAddTri_", expect = "A1 6C AE A9 00",
       unverified = "mvpc-fx-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "writes the FX Im3D vertex buffer 0xC4D958 (global)")]
#[allow(clippy::too_many_arguments)]
pub fn fx_render_add_tri(
    x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32, u0: u32, v0: u32, u1: u32,
    v1: u32, u2: u32, v2: u32, r0: u32, g0: u32, b0: u32, a0: u32, r1: u32, g1: u32, b1: u32, a1: u32, r2: u32,
    g2: u32, b2: u32, a2: u32,
) {
    let f = |v: u32| xf(f32::from_bits(v)).to_f32().to_bits();
    let col = |r: u32, g: u32, b: u32, a: u32| ((((a << 8) | r) << 8 | g) << 8) | b;
    let cur = gp::<GPtr<u8>>(0xA9AE6C);
    let cnt = gp::<i32>(0xA9AE58);
    let verts = [
        (f(x0), f(y0), f(z0), col(r0, g0, b0, a0)),
        (x1, f(y1), f(z1), col(r1, g1, b1, a1)),
        (x2, f(y2), f(z2), col(r2, g2, b2, a2)),
    ];
    for (x, y, z, c) in verts {
        let v = cur.read();
        wr::<u32>(v, 0, x);
        wr::<u32>(v, 4, y);
        wr::<u32>(v, 8, z);
        wr::<u32>(v, 0x18, c);
        cur.write(v.byte_add(0x24));
        cnt.write(cnt.read().wrapping_add(1));
    }
    if gp::<u32>(0xA9AE64).read() != 0 {
        let e = cur.read();
        wr::<u32>(e, -0x50, u0);
        wr::<u32>(e, -0x4C, f(v0));
        wr::<u32>(e, -0x2C, f(u1));
        wr::<u32>(e, -0x28, f(v1));
        wr::<u32>(e, -8, f(u2));
        wr::<u32>(e, -4, f(v2));
    }
    let tris = gp::<i32>(0xA9AE5C);
    tris.write(tris.read().wrapping_add(1));
    if cnt.read() >= 0x7FD || tris.read() >= 0xFFF {
        let n = tris.read().wrapping_mul(3);
        if !rw_im3d_transform(gp(0xC4D958), n, gp::<u32>(0xA9AE68).read(), gp::<u32>(0xA9AE60).read()).is_null() {
            rw_im3d_render_primitive(3);
            rw_im3d_end();
        }
        tris.write(0);
        cnt.write(0);
        cur.write(gp(0xC4D958));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FX_RENDER_ADD_TRI,
    HOOK_EMITTER_ADD_PARTICLE,
    HOOK_EMITTER_CREATE_PARTICLE,
    HOOK_EMITTER_CREATE_PARTICLES,
    HOOK_EMITTER_BP_UPDATE_PARTICLE,
];
