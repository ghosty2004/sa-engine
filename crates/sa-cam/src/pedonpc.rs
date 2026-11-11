//! PORT-CAM-3 (helper A): `CCam::Process_1rstPersonPedOnPC` (0x50EB70), the on-foot first-person
//! camera (also the photo camera, mode 0x27). Ported from the 1.0 US bytes (gta-reversed: not
//! reversed). Same technique as [`crate::followped`]: a frame array at the original's `B`-relative
//! offsets (B = ESP after the four register pushes; the orientation argument slot `B+0x68` is scratch)
//! and the [`Fpu`] machine. FPATAN / FSIN / FCOS use sa-math's f64 models; `CPlaceable::GetHeading`'s
//! ST0 (an FPATAN result) comes back through `orig!` as an f64.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::{hook, orig};

use crate::camcol::{allocate_matrix, set_near_clip_raw, simple_transform_update_matrix};
use crate::cammodes::ped_get_bone_position;
use crate::camproc::{get_pad, get_vectors_ready_for_rw, vector_normalise};
use crate::common::{Fpu, g, k, rd, wr};
use crate::followped::{clip_beta, copy3, d, fcos, fpatan, fsin, ped_is_player, push_st0};
use crate::rocket::{matrix_update_rw_matrix, simple_transform_update_rw_matrix};

orig! {
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    pub fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `ConvertPedNode2BoneTag(int32)` (0x4D58A0)
    pub fn convert_ped_node_2_bone_tag_call(node: i32) -> i32 = cdecl 0x4D58A0 => crate::camproc::convert_ped_node_2_bone_tag;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    pub fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    pub fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RwMatrixScale(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F22C0)
    pub fn rw_matrix_scale(m: GPtr<u8>, v: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7F22C0 => unported;
    /// `CMatrix * CVector` (0x59C890, cdecl(out, m, v)), returns out
    pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0, ST0)
    pub fn placeable_get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020); the float as a dword
    pub fn matrix_set_rotate_z_only(m: GPtr<u8>, h: u32) = thiscall 0x59B020 => unported;
    /// `CPad::SniperZoomOut()` (0x540B80, AL) / `SniperZoomIn()` (0x540B30, AL)
    pub fn pad_sniper_zoom_out(pad: GPtr<u8>) -> u8 = thiscall 0x540B80 => unported;
    pub fn pad_sniper_zoom_in(pad: GPtr<u8>) -> u8 = thiscall 0x540B30 => unported;
    /// `FUN_00540e80` / `FUN_00540f80` (CPad, AX): the first-person look left/right and up/down sticks
    pub fn pad_540e80(pad: GPtr<u8>) -> i16 = thiscall 0x540E80 => unported;
    pub fn pad_540f80(pad: GPtr<u8>) -> i16 = thiscall 0x540F80 => unported;
}

/// The guard (0xB6FFE0), `TheCamera` +0x3E (0xB6F066) and +0x3F (0xB6F067, head offset lerp on), the
/// lerp factor (0xB6FC48), the statics reset with the camera (u8 0xB6FFDC, 0xB6FFD8, 0xB6FFD4, 0xB6FFD0)
/// and the remembered head offset (0xB6FFC4..CC), `TheCamera` +0x14C (0xB6F174, alpha copy).
const GUARD: usize = 0xB6FFE0;
const CAMERA_3E: usize = 0xB6F066;
const CAMERA_3F: usize = 0xB6F067;
const HEAD_LERP: usize = 0xB6FC48;
const HEAD_OFFSET: usize = 0x8CCC54;
const S_HEAD: usize = 0xB6FFC4;
const CAMERA_ALPHA_COPY: usize = 0xB6F174;
const CAMERA_TARGET: usize = 0xB6F980;
const MOUSE_ACCEL_VERTICAL: usize = 0xB6EC18;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;
const TIME_STEP: usize = 0xB7CB5C;

const K_1: u32 = 0x3F80_0000; // [0x858624]
const K_0: u32 = 0; // [0x858B50]
const K_M1: u32 = 0xBF80_0000; // [0x858C1C]
const K_3: u32 = 0x4040_0000; // [0x858B3C]
const K_4: u32 = 0x4080_0000; // [0x858B90]
const K_15: u32 = 0x4170_0000; // [0x858B48]
const K_70: u32 = 0x428C_0000; // [0x858CE0]
const K_PI: u32 = 0x4049_0FDB; // [0x858CB8]
const K_2PI: u32 = 0x40C9_0FDB; // [0x858CBC]
const K_MPI: u32 = 0xC049_0FDB; // [0x858CC0]
const K_0_4: u32 = 0x3ECC_CCCD; // [0x858EE8]
const K_0_0001: u32 = 0x38D1_B717; // [0x858FC4]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_M3: u32 = 0xC040_0000; // [0x859024]
const K_10000: u32 = 0x461C_4000; // [0x859AA4]
const K_255: u32 = 0x437F_0000; // [0x859AAC]
const K_1_14: u32 = 0x3D92_4925; // [0x859B50]
const K_0_0125: u32 = 0x3C4C_CCCD; // [0x85A5C4]
const K_ALPHA_MIN: u32 = 0xBFC7_F1E7; // [0x8630F4]
const K_ALPHA_MAX: u32 = 0x3F86_0A92; // [0x8630F8]
const K_UD_SCALE: u32 = 0x383A_69DC; // [0x8630FC]
const K_LR_SCALE: u32 = 0x3D6A_0EA1; // [0x863100]
const K_0_59: u32 = 0x3F17_0A3D; // [0x863104]
const K_1_23: u32 = 0x3F9D_70A4; // [0x863108]

/// `CCam::Process_1rstPersonPedOnPC(const CVector&, float orientation, float, float)` (0x50EB70,
/// thiscall, `ret 0x10`). Order of calls, re-reads and x87 operations as in the bytes:
/// - FOV 70 unless mode 0x27; TheCamera +0x3E = 0; no RwObject: return; not a ped: near clip 0.05;
/// - the head offset (0.06, 0.05, 0) [0x8CCC54] transformed by the head bone matrix, then that bone
///   matrix scaled by (0, 0, 0) (pre-concatenated: the head disappears);
/// - reset: angles from the orientation (a ped: heading + pi/2), statics zeroed, the eye offset +0x184
///   and the remembered offset 0xB6FFC4 = head point;
/// - +0x188 = head.y; with TheCamera +0x3F the eye offset lerps (x, z) and the source is
///   `matrix * offset`, else the source is `pos + flatForward * |head.xy - remembered.xy| * 1.23` (z +0.59);
/// - mouse / pad look as in `Process_Rocket` (pad: `FUN_00540e80` / `FUN_00540f80`), alpha clamp;
/// - an attached player (+0xFC) clamps beta around the attachment's heading (+0x778 quadrant, +0x77C
///   limit);
/// - front = normalised `(source + dir*3) - source`, source += front * 0.4; TheCamera +0x14C = alpha; the
///   target's heading = `atan2(-front.x, front.y)` (matrix `SetRotateZOnly` or the placement), RW matrix;
/// - mode 0x27: sniper zoom in / out (`(ts * 255 + 10000) * 0.0001`), the camera's colour filter block,
///   FOV clamped to [15, 70]; then m_bResetStatics = 0 and near clip 0.05.
#[hook(addr = 0x50EB70, conv = thiscall, name = "CCam::Process_1rstPersonPedOnPC", expect = "A0 E0 FF B6 00", this = "CCam")]
pub fn process_1rst_person_ped_on_pc(
    this: GPtr<u8>,
    _target: GPtr<u8>,
    orientation: u32,
    _sv: u32,
    _svw: u32,
) {
    let mut fr = [0u32; 0x1C];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    wr::<u32>(b, 0x68, orientation);
    let mut fpu = Fpu::new();
    let f = &mut fpu;
    let guard = g::<u32>(GUARD);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
    }
    if rd::<i16>(this, 0xC) != 0x27 {
        wr::<u32>(this, 0xB4, K_70);
    }
    g::<u8>(CAMERA_3E).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u32>(t, 0x18) == 0 {
        return;
    }
    if rd::<u8>(t, 0x36) & 7 != 3 {
        finish(this);
        return;
    }
    copy3(b.byte_add(0x24), g::<u8>(HEAD_OFFSET));
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(t, 0x18));
    let tag = convert_ped_node_2_bone_tag_call(2);
    let idx = rp_hanim_id_get_index(h, tag);
    let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add((idx as isize) << 6);
    rw_v3d_transform_points(b.byte_add(0x24), b.byte_add(0x24), 1, m);
    wr::<u32>(b, 0x3C, 0);
    wr::<u32>(b, 0x40, 0);
    wr::<u32>(b, 0x44, 0);
    rw_matrix_scale(m, b.byte_add(0x3C), 1);
    if rd::<u8>(this, 0xA) != 0 {
        let o = rd::<u32>(b, 0x68);
        let t = rd::<GPtr<u8>>(this, 0x21C);
        wr::<u32>(this, 0xBC, o);
        wr::<u32>(this, 0xAC, 0);
        wr::<u32>(this, 0xA8, o);
        if rd::<u8>(t, 0x36) & 7 == 3 {
            f.fld(rd(t, 0x558));
            wr::<u32>(this, 0xAC, 0);
            f.fadd_m(k(K_HALF_PI));
            g::<u8>(0xB6FFDC).write(0);
            g::<u32>(0xB6FFD8).write(0);
            g::<u32>(0xB6FFD4).write(0);
            wr::<f32>(this, 0xBC, f.fstp());
            f.fld(rd(t, 0x558));
            wr::<u8>(this, 0x3, 1);
            f.fadd_m(k(K_HALF_PI));
            wr::<f32>(this, 0xA8, f.fstp());
        }
        copy3(this.byte_add(0x184), b.byte_add(0x24));
        g::<u32>(0xB6FFD0).write(0);
        copy3(g::<u8>(S_HEAD), b.byte_add(0x24));
    }
    wr::<u32>(this, 0x188, rd::<u32>(b, 0x28));
    let src = this.byte_add(0x19C);
    let (sx, sy);
    if g::<u8>(CAMERA_3F).read() != 0 {
        let eye = this.byte_add(0x184);
        let lerp = |f: &mut Fpu, o_b: isize, o_e: isize| {
            f.fld(k(K_1));
            f.fsub_m(d(HEAD_LERP));
            f.fmul_m(rd(b, o_b));
            f.fld(d(HEAD_LERP));
            f.fmul_m(rd(eye, o_e));
            f.faddp(1);
            wr::<f32>(eye, o_e, f.fstp());
        };
        lerp(f, 0x24, 0);
        lerp(f, 0x2C, 8);
        let e = rd::<GPtr<u8>>(this, 0x21C);
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let r = matrix_mul_vec(b.byte_add(0x48), rd::<GPtr<u8>>(e, 0x14), eye);
        sx = rd::<u32>(r, 0);
        wr::<u32>(b, 0x24, sx);
        sy = rd::<u32>(r, 4);
        wr::<u32>(b, 0x28, sy);
        wr::<u32>(b, 0x2C, rd::<u32>(r, 8));
    } else {
        f.fld(rd(b, 0x24));
        let e = rd::<GPtr<u8>>(this, 0x21C);
        f.fsub_m(d(S_HEAD));
        wr::<f32>(b, 0x30, f.fstp());
        f.fld(rd(b, 0x28));
        f.fsub_m(d(S_HEAD + 4));
        wr::<f32>(b, 0x34, f.fstp());
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        copy3(b.byte_add(0x18), rd::<GPtr<u8>>(e, 0x14).byte_add(0x10));
        wr::<u32>(b, 0x20, 0);
        vector_normalise(b.byte_add(0x18));
        f.fld(rd(b, 0x30));
        f.fmul_m(rd(b, 0x30));
        let t = rd::<GPtr<u8>>(this, 0x21C);
        f.fld(rd(b, 0x34));
        let m = rd::<GPtr<u8>>(t, 0x14);
        f.fmul_m(rd(b, 0x34));
        f.faddp(1);
        f.fsqrt();
        f.fld(rd(b, 0x18));
        f.fmul_st(1);
        f.fld(rd(b, 0x1C));
        f.fmul_st(2);
        wr::<f32>(b, 0x34, f.fstp());
        f.fxch(1);
        f.fmul_m(rd(b, 0x20));
        wr::<f32>(b, 0x38, f.fstp());
        f.fmul_m(k(K_1_23));
        f.fld(rd(b, 0x34));
        f.fmul_m(k(K_1_23));
        f.fld(rd(b, 0x38));
        f.fmul_m(k(K_1_23));
        wr::<f32>(b, 0x50, f.fstp());
        let pos = if m.is_null() {
            t.byte_add(4)
        } else {
            m.byte_add(0x30)
        };
        f.fxch(1);
        f.fadd_m(rd(pos, 0));
        wr::<f32>(b, 0x30, f.fstp());
        sx = rd::<u32>(b, 0x30);
        f.fadd_m(rd(pos, 4));
        wr::<f32>(b, 0x34, f.fstp());
        f.fld(rd(b, 0x50));
        sy = rd::<u32>(b, 0x34);
        f.fadd_m(rd(pos, 8));
        wr::<u32>(b, 0x24, sx);
        wr::<u32>(b, 0x28, sy);
        wr::<f32>(b, 0x38, f.fstp());
        f.fld(rd(b, 0x38));
        wr::<u32>(b, 0x2C, rd::<u32>(b, 0x38));
        f.fadd_m(k(K_0_59));
        wr::<f32>(b, 0x2C, f.fstp());
    }
    // 0x50EE8E
    wr::<u32>(src, 0, sx);
    wr::<u32>(src, 4, sy);
    wr::<u32>(src, 8, rd::<u32>(b, 0x2C));
    ped_get_bone_position(rd::<GPtr<u8>>(this, 0x21C), b.byte_add(0x54), 3, 1);
    look(this, b, f);
    attached_clamp(this, b, f);
    output(this, b, f);
}

/// 0x50EEB6: the mouse or the first-person look sticks (as `Process_Rocket`), `ClipBeta`, alpha clamp.
fn look(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let _ = get_pad(0);
    wr::<u32>(b, 0x18, g::<u32>(MOUSE_X).read());
    f.fld(rd(b, 0x18));
    f.fcomp_m(k(K_0));
    wr::<u32>(b, 0x1C, g::<u32>(MOUSE_Y).read());
    let mouse = f.jp(0x44) || {
        f.fld(rd(b, 0x1C));
        f.fcomp_m(k(K_0));
        f.jp(0x44)
    };
    if mouse {
        f.fld(rd(b, 0x18));
        f.fmul_m(k(K_M3));
        f.fld(rd(b, 0x1C));
        f.fmul_m(k(K_4));
        wr::<f32>(b, 0x68, f.fstp());
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        wr::<f32>(b, 0x10, f.fst());
        f.fmul_m(d(MOUSE_ACCEL_HORZNTL));
        f.fmulp(1);
        f.fld(rd(b, 0x10));
        f.fmul_m(d(MOUSE_ACCEL_VERTICAL));
        f.fmul_m(rd(b, 0x68));
    } else {
        let lr = i32::from(pad_540e80(get_pad(0))).wrapping_neg();
        wr::<i32>(b, 0x68, lr);
        f.fild(rd(b, 0x68));
        wr::<f32>(b, 0x14, f.fstp());
        let ud = i32::from(pad_540f80(get_pad(0)));
        wr::<i32>(b, 0x68, ud);
        wr::<u32>(b, 0x18, K_1);
        f.fild(rd(b, 0x68));
        wr::<f32>(b, 0x68, f.fstp());
        f.fld(k(K_1));
        f.fld(rd(b, 0x14));
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(k(K_M1));
        }
        f.fld(rd(b, 0x68));
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            wr::<u32>(b, 0x18, K_M1);
        }
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        wr::<f32>(b, 0x10, f.fstp());
        f.fld(rd(b, 0x14));
        f.fmul_m(rd(b, 0x14));
        f.fmul_m(k(K_0_0001));
        f.fld(rd(b, 0x10));
        f.fmul_m(k(K_LR_SCALE));
        f.fmulp(1);
        f.fmulp(1);
        f.fmul_m(d(TIME_STEP));
        f.fld(rd(b, 0x68));
        f.fmul_m(rd(b, 0x68));
        f.fmul_m(k(K_UD_SCALE));
        f.fld(rd(b, 0x10));
        f.fmul_m(k(K_1_14));
        f.fmulp(1);
        f.fmul_m(rd(b, 0x18));
        f.fmul_m(d(TIME_STEP));
    }
    f.fxch(1);
    f.fadd_m(rd(this, 0xBC));
    wr::<f32>(this, 0xBC, f.fstp());
    f.fadd_m(rd(this, 0xAC));
    wr::<f32>(this, 0xAC, f.fstp());
    clip_beta(this);
    f.fld(rd(this, 0xAC));
    f.fcomp_m(k(K_ALPHA_MAX));
    if !f.jne(0x41) {
        wr::<u32>(this, 0xAC, K_ALPHA_MAX);
    } else {
        f.fld(rd(this, 0xAC));
        f.fcomp_m(k(K_ALPHA_MIN));
        if !f.jp(0x5) {
            wr::<u32>(this, 0xAC, K_ALPHA_MIN);
        }
    }
}

/// 0x50F06F: a player attached to something (+0xFC) keeps beta within +0x77C of the attachment's
/// heading turned by the +0x778 quadrant (0: +pi/2, 1: +pi, 2: -pi/2, 3: +0; else `B+0x68`).
fn attached_clamp(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    if ped_is_player(rd::<GPtr<u8>>(this, 0x21C)) == 0 {
        return;
    }
    let ped = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<GPtr<u8>>(ped, 0xFC).is_null() {
        return;
    }
    let q = rd::<u16>(ped, 0x778);
    if q <= 3 {
        push_st0(f, placeable_get_heading(rd::<GPtr<u8>>(ped, 0xFC)));
        match q {
            0 => f.fadd_m(k(K_HALF_PI)),
            1 => f.fadd_m(k(K_PI)),
            2 => f.fsub_m(k(K_HALF_PI)),
            _ => {}
        }
    } else {
        f.fld(rd(b, 0x68));
    }
    f.fld(rd(this, 0xBC));
    f.fsub_st(1);
    f.fcom_m(k(K_PI));
    if !f.jne(0x41) {
        f.fsub_m(k(K_2PI));
    } else {
        f.fcom_m(k(K_MPI));
        if !f.jp(0x5) {
            f.fadd_m(k(K_2PI));
        }
    }
    f.fcom_m(rd(ped, 0x77C));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(rd(ped, 0x77C));
    } else {
        f.fld(rd(ped, 0x77C));
        f.fchs();
        wr::<f32>(b, 0x68, f.fstp());
        f.fcom_m(rd(b, 0x68));
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(rd(b, 0x68));
        }
    }
    f.fadd_st(1);
    wr::<f32>(this, 0xBC, f.fstp());
    f.fstp_st(0);
}

/// 0x50F15F to the end.
fn output(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let src = this.byte_add(0x19C);
    let front = this.byte_add(0x190);
    f.fld(rd(this, 0xAC));
    fcos(f);
    f.fld(rd(this, 0xBC));
    fcos(f);
    f.fmul_st(1);
    f.fld(rd(this, 0xBC));
    fsin(f);
    f.fmul_st(2);
    wr::<f32>(b, 0x34, f.fstp());
    f.fld(rd(this, 0xAC));
    fsin(f);
    wr::<f32>(b, 0x38, f.fstp());
    f.fmul_m(k(K_3));
    f.fstp_st(1);
    f.fld(rd(b, 0x34));
    f.fmul_m(k(K_3));
    f.fld(rd(b, 0x38));
    f.fmul_m(k(K_3));
    wr::<f32>(b, 0x50, f.fstp());
    f.fxch(1);
    f.fadd_m(rd(src, 0));
    wr::<f32>(b, 0x30, f.fstp());
    f.fadd_m(rd(src, 4));
    f.fld(rd(b, 0x50));
    f.fadd_m(rd(src, 8));
    f.fld(rd(b, 0x30));
    f.fsub_m(rd(src, 0));
    wr::<f32>(b, 0x30, f.fstp());
    f.fxch(1);
    f.fsub_m(rd(src, 4));
    wr::<f32>(b, 0x34, f.fstp());
    f.fsub_m(rd(src, 8));
    wr::<u32>(front, 0, rd::<u32>(b, 0x30));
    wr::<u32>(front, 4, rd::<u32>(b, 0x34));
    wr::<f32>(b, 0x38, f.fstp());
    wr::<u32>(front, 8, rd::<u32>(b, 0x38));
    vector_normalise(front);
    f.fld(rd(front, 0));
    f.fmul_m(k(K_0_4));
    f.fld(rd(front, 4));
    f.fmul_m(k(K_0_4));
    f.fld(rd(front, 8));
    f.fmul_m(k(K_0_4));
    wr::<f32>(b, 0x50, f.fstp());
    f.fxch(1);
    f.fadd_m(rd(src, 0));
    wr::<f32>(src, 0, f.fstp());
    f.fadd_m(rd(src, 4));
    wr::<f32>(src, 4, f.fstp());
    f.fld(rd(b, 0x50));
    f.fadd_m(rd(src, 8));
    wr::<f32>(src, 8, f.fstp());
    g::<u32>(CAMERA_ALPHA_COPY).write(rd::<u32>(this, 0xAC));
    get_vectors_ready_for_rw(this);
    f.fld(rd(front, 0));
    f.fchs();
    f.fld(rd(this, 0x194));
    fpatan(f);
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    wr::<f32>(b, 0x68, f.fst());
    wr::<f32>(te, 0x558, f.fstp());
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    f.fld(rd(b, 0x68));
    wr::<f32>(te, 0x55C, f.fstp());
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    let m = rd::<GPtr<u8>>(te, 0x14);
    if !m.is_null() {
        matrix_set_rotate_z_only(m, rd::<u32>(b, 0x68));
    } else {
        wr::<u32>(te, 0x10, rd::<u32>(b, 0x68));
    }
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    let rw = rd::<GPtr<u8>>(te, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(te, 0x14);
        if !m.is_null() {
            matrix_update_rw_matrix(m, rwm);
        } else {
            simple_transform_update_rw_matrix(te.byte_add(4), rwm);
        }
    }
    if rd::<i16>(this, 0xC) == 0x27 {
        photo_zoom(this, f);
    }
    finish(this);
}

/// 0x50F2C2: the photo camera's zoom and colour filter, FOV clamped to [15, 70].
fn photo_zoom(this: GPtr<u8>, f: &mut Fpu) {
    if pad_sniper_zoom_out(get_pad(0)) != 0 || pad_sniper_zoom_in(get_pad(0)) != 0 {
        let step = |f: &mut Fpu| {
            f.fld(d(TIME_STEP));
            f.fmul_m(k(K_255));
            f.fadd_m(k(K_10000));
        };
        if pad_sniper_zoom_out(get_pad(0)) != 0 {
            step(f);
            f.fmul_m(rd(this, 0xB4));
            f.fmul_m(k(K_0_0001));
            wr::<f32>(this, 0xB4, f.fstp());
        } else if pad_sniper_zoom_in(get_pad(0)) != 0 {
            step(f);
            f.fmul_m(k(K_0_0001));
            f.fdivr_m(rd(this, 0xB4));
            wr::<f32>(this, 0xB4, f.fstp());
        }
    }
    g::<u32>(0xB6F0B4).write(0xB4);
    g::<u32>(0xB6F0AC).write(0xB4);
    g::<u32>(0xB6F0B0).write(0xFF);
    g::<u32>(0xB6F0B8).write(1);
    g::<u32>(0xB6F0C8).write(0x78);
    f.fld(rd(this, 0xB4));
    f.fcomp_m(k(K_70));
    if !f.jne(0x41) {
        wr::<u32>(this, 0xB4, K_70);
    }
    f.fld(rd(this, 0xB4));
    f.fcomp_m(k(K_15));
    if !f.jp(0x5) {
        wr::<u32>(this, 0xB4, K_15);
    }
}

/// 0x50F3BD: `m_bResetStatics = 0`, near clip 0.05.
fn finish(this: GPtr<u8>) {
    wr::<u8>(this, 0xA, 0);
    set_near_clip_raw(SCENE_CAMERA.get(), 0x3D4C_CCCD);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_1RST_PERSON_PED_ON_PC];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
