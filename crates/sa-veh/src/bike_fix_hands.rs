//! `CBike::FixHandsToBars(CPed* rider)` (0x6B7F90, batch `std12-veh-2`): moves the rider's hand bones onto the
//! handlebars after the animation update (reached from the ped's after-pre-render step whenever someone rides a
//! bike). Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, model_info, rd, wr, x};

orig! {
    /// `CMatrix::Attach(RwMatrix*, bool owns)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    fn matrix_mul_assign(m: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A80 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool temporary)` (0x59C050)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, temp: u32) = thiscall 0x59C050 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790) and `(out, const CVector*, const
    /// CMatrix*)` (0x59C810, the transposed rotation)
    fn multiply3x3_mv(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    fn multiply3x3_vm(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890), hidden result pointer first
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0), `RpHAnimHierarchyGetMatrixArray` (0x7C5120)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
}

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// A local `CMatrix` (0x48 bytes) with its attach pointer and owner flag zeroed (as the original's constructor-less
/// stack matrices start).
struct Mat([u32; 0x12]);

impl Mat {
    fn new() -> Mat {
        Mat([0; 0x12])
    }
    fn p(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// The bone's matrix position (`&RpHAnimHierarchyGetMatrixArray(h)[RpHAnimIDGetIndex(h, id)].pos`).
fn bone_pos(h: GPtr<u8>, id: i32) -> GPtr<u8> {
    let i = rp_hanim_id_get_index(h, id);
    rp_hanim_hierarchy_get_matrix_array(h).byte_add((i << 6) as isize + 0x30)
}

/// `pos += (dx, dy, dz)` with each sum stored as f32 (`dz` may be an unrounded register value).
fn shift(p: GPtr<u8>, dx: f32, dy: f32, dz: X87) {
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let nz = dz.fadd_m(pz);
    let ny = x(py).fadd_m(dy);
    let nx = x(dx).fadd_m(px);
    wr::<f32>(p, 0, nx.to_f32());
    wr::<f32>(p, 4, ny.to_f32());
    wr::<f32>(p, 8, nz.to_f32());
}

/// One hand: `W = bars * P` (P the handlebar point), the hand bone `h` moved onto W by `d = W - pos(h)` (f32), then
/// the two finger bones `f1`, `f2` by `d`, the forearm `a1` by `d * k1` and the upper arm `a2` by `d * k2` (x and y
/// products stored as f32, z at register precision), and for the player the extra bone `extra` by `d * k2` (all
/// f32).
#[allow(clippy::too_many_arguments)]
fn hand(bars: GPtr<u8>, p: &mut [f32; 3], hier: GPtr<u8>, ped: GPtr<u8>, ids: [i32; 6], k1: f32, k2: f32) {
    let mut out = [0f32; 3];
    let w = matrix_mul_vec(vp(&mut out), bars, vp(p));
    let w = [rd::<f32>(w, 0), rd::<f32>(w, 4), rd::<f32>(w, 8)];
    let b = bone_pos(hier, ids[0]);
    let (bx, by, bz) = (rd::<f32>(b, 0), rd::<f32>(b, 4), rd::<f32>(b, 8));
    let dz = x(w[2]).fsub_m(bz).to_f32();
    let dy = x(w[1]).fsub_m(by).to_f32();
    let dx = x(w[0]).fsub_m(bx).to_f32();
    shift(b, dx, dy, x(dz));
    for &id in &ids[1..3] {
        shift(bone_pos(hier, id), dx, dy, x(dz));
    }
    let b = bone_pos(hier, ids[3]);
    let z1 = x(dz).fmul_m(k1);
    let y1 = x(dy).fmul_m(k1).to_f32();
    let x1 = x(dx).fmul_m(k1).to_f32();
    shift(b, x1, y1, z1);
    let b = bone_pos(hier, ids[4]);
    let z2 = x(dz).fmul_m(k2);
    let y2 = x(dy).fmul_m(k2).to_f32();
    let x2 = x(dx).fmul_m(k2).to_f32();
    shift(b, x2, y2, z2);
    if ped_is_player(ped) != 0 {
        shift(bone_pos(hier, ids[5]), x2, y2, x(z2.to_f32()));
    }
}

/// `CBike::FixHandsToBars(CPed* rider)` (0x6B7F90, thiscall, `ret 4`). Only with the right (+0x7B5) or left (+0x7B4)
/// hand flag. Without the handlebar frame (+0x5A4) nothing else happens. Otherwise `bars = bike matrix *
/// handlebar frame matrix`; the grip point P is the vehicle structure's dummy +0x78, or when that is (0, 0, 0) the
/// model's default (model 0x1FE: [0x8D3250], 0x1FD: [0x8D325C], else [0x8D3244]) plus `bars^T * (rightHand matrix *
/// [0x8D3274])` (the rotation parts only). Right hand: bones 24 (hand), 25, 26, forearm 23 (×0.667), upper arm 22
/// (×0.333), player bone 301. Left hand: P.x negated (stored), bones 34, 35, 36, 33 (×0.75), 32 (×0.4), player bone
/// 302. Both flags are cleared afterwards.
#[hook(addr = 0x6B7F90, conv = thiscall, name = "CBike::FixHandsToBars", expect = "64 A1 00 00 00 00 6A FF 68 CE 82 84 00",
       unverified = "std12-veh-2", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving")]
pub fn fix_hands_to_bars(bike: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(bike, 0x7B5) == 0 && rd::<u8>(bike, 0x7B4) == 0 {
        return;
    }
    let mut bars = Mat::new();
    let mut frame_m = Mat::new();
    let frame = rd::<GPtr<u8>>(bike, 0x5A4);
    if frame.is_null() {
        matrix_dtor(frame_m.p());
        matrix_dtor(bars.p());
        return;
    }
    matrix_attach(frame_m.p(), frame.byte_add(0x10), 0);
    matrix_assign(bars.p(), rd::<GPtr<u8>>(bike, 0x14));
    matrix_mul_assign(bars.p(), frame_m.p());
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
    let model = rd::<i16>(bike, 0x22);
    let vs = rd::<GPtr<u8>>(model_info(i32::from(model)), 0x5C);
    let mut p = [rd::<f32>(vs, 0x78), rd::<f32>(vs, 0x7C), rd::<f32>(vs, 0x80)];
    let zero = x(k(0x858B50));
    // `fcomp 0.0; test ah, 0x44; jp`: taken for a non-zero (or NaN) component
    let nz = |v: f32| x(v).fcom(zero).test_parity(0x44);
    if nz(p[0]) || nz(p[1]) || nz(p[2]) {
        let i = rp_hanim_id_get_index(hier, 0x18);
        let m = rp_hanim_hierarchy_get_matrix_array(hier).byte_add((i << 6) as isize);
        let mut q = [k(0x8D3274), k(0x8D3278), k(0x8D327C)];
        let mut bone = Mat::new();
        matrix_ctor_rw(bone.p(), m, 0);
        let mut out = [0f32; 3];
        let r = multiply3x3_mv(vp(&mut out), bone.p(), vp(&mut q));
        q = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        let r = multiply3x3_vm(vp(&mut out), vp(&mut q), bars.p());
        let o = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        for i in 0..3 {
            p[i] = x(o[i]).fadd_m(p[i]).to_f32();
        }
        matrix_dtor(bone.p());
    } else {
        let base = match model as u16 {
            0x1FE => 0x8D3250,
            0x1FD => 0x8D325C,
            _ => 0x8D3244,
        };
        p = [k(base), k(base + 4), k(base + 8)];
    }
    if rd::<u8>(bike, 0x7B5) != 0 {
        hand(bars.p(), &mut p, hier, ped, [0x18, 0x19, 0x1A, 0x17, 0x16, 0x12D], k(0x871488), k(0x864E30));
    }
    if rd::<u8>(bike, 0x7B4) != 0 {
        p[0] = x(p[0]).fmul_m(k(0x858C1C)).to_f32();
        hand(bars.p(), &mut p, hier, ped, [0x22, 0x23, 0x24, 0x21, 0x20, 0x12E], k(0x858F34), k(0x858EE8));
    }
    wr::<u8>(bike, 0x7B4, 0);
    wr::<u8>(bike, 0x7B5, 0);
    matrix_dtor(frame_m.p());
    matrix_dtor(bars.p());
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_FIX_HANDS_TO_BARS];
