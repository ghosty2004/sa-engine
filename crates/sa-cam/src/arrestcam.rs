//! PORT-CAM-3 (helper B): the unnamed helpers of `CCam::ProcessArrestCamOne` (0x518500): the cop's
//! head view 0x512EF0 and the two source-position helpers 0x515D80 / 0x516010. Ported from the 1.0 US
//! bytes; `docs/re/specs/CAM/_unattributed-B.md` as reading aid (gta-reversed has none of them).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, g, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// 0x512EF0's init-guard bits (1: 0xB70010, 2: the source offset 0xB70004)
    pub static ARREST_HEAD_GUARD @ 0xB7001C: u32 = 0;
    /// 0x512EF0's zero vector (written once, never read)
    pub static ARREST_HEAD_ZERO @ 0xB70010: [u32; 3] = [0; 3];
    /// 0x512EF0's source offset (0, 0, -0.5)
    pub static ARREST_HEAD_OFFSET @ 0xB70004: [u32; 3] = [0; 3];
    /// 0x512EF0's toggle byte (written, never read there)
    pub static ARREST_HEAD_TOGGLE @ 0xB70020: u8 = 0;
    /// 0x512EF0's time offset (ms, f32)
    pub static ARREST_HEAD_T0 @ 0xB70024: f32 = 0.0;
    /// `.data` tunables of 0x512EF0: head drop per t (0.5), head z offset (-0.06), duration (1000)
    pub static ARREST_HEAD_K @ 0x8CCCA8: [u32; 3] = [0x3F00_0000, 0xBD75_C28F, 0x447A_0000];
    /// `.data` tunables of 0x515D80: back distance (5.0), side offset (3.0), (1.4, unused here),
    /// minimum length (8.0)
    pub static ARREST_SRC_K @ 0x8CC8C8: [u32; 4] = [0x40A0_0000, 0x4040_0000, 0x3FB3_3333, 0x4100_0000];
    /// `.data` tunables of 0x516010: ahead (5.0), side (10.0), height above ground (0.7)
    pub static ARREST_SIDE_K @ 0x8CC7F0: [u32; 3] = [0x40A0_0000, 0x4120_0000, 0x3F33_3333];
}

/// `TheCamera.m_pTargetEntity`; the start time 0xB6EC60 (f32, catalogue "s_ExitCam+4").
const CAMERA_TARGET_ENTITY: usize = 0xB6F980;
const ARREST_START_TIME: usize = 0xB6EC60;
/// 1/32767 (0x858C7C), 0.65 (0x858F50), 1.0, 0.0, 2^32 (0x858C54), -0.7071 (0x8631E8), 1.4142271
/// (0x8631E4), 5.0 (0x858C80).
const K_RAND: u32 = 0x3800_0100;
const K_0_65: u32 = 0x3F26_6666;
const K_ONE: u32 = 0x3F80_0000;
const K_ZERO: u32 = 0;
const K_2P32: u32 = 0x4F80_0000;
const K_M0_7071: u32 = 0xBF35_0481;
const K_1_4142: u32 = 0x3FB5_0565;
const K_5: u32 = 0x40A0_0000;

orig! {
    /// `RwHelper::GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    pub fn get_anim_hierarchy_from_skin_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    pub fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    pub fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490, AL)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** ent)` (0x5696C0, ST0)
    pub fn find_ground_z_for_3d_coord(px: u32, py: u32, pz: u32, found: GPtr<u8>, ent: u32) -> f64 = cdecl 0x5696C0 => unported;
}

/// A frame of `n` dwords (the functions' locals keep the original offsets) and its pointer.
struct Frame([u32; 0x20]);

impl Frame {
    fn b(&mut self) -> GPtr<u8> {
        GPtr::<u32>::from_raw(self.0.as_mut_ptr()).cast::<u8>()
    }
}

fn m(b: GPtr<u8>, o: isize) -> f32 {
    rd::<f32>(b, o)
}
fn st(b: GPtr<u8>, o: isize, v: f32) {
    wr::<f32>(b, o, v)
}
fn mu(b: GPtr<u8>, o: isize) -> u32 {
    rd::<u32>(b, o)
}
fn stu(b: GPtr<u8>, o: isize, v: u32) {
    wr::<u32>(b, o, v)
}

/// The entity position: `&matrix->pos` or `&placement.pos`.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `FUN_00512ef0(CPed* ped, bool check)` (0x512EF0, thiscall on a CCam, `ret 8`, AL): the arrest
/// camera looking from the target ped's position at the cop's head (bone 5) lowered by `0.5 * t`.
/// With `check`, a random `rand() / 32767 > 0.65` (site 0x512F04) fails first. The target must be a
/// ped and `ped` set. `t = (((float)(uint32)now - [0xB6EC60]) - [0xB70024]) / [0x8CCCB0]` clamped to
/// [0, 1] with the toggle byte 0xB70020 (1 below 1.0, flipped at 1.0 and above, 1 below 0). Source =
/// target position + (0, 0, -0.5), FOV 100, front / up from the source to the head point; with
/// `check` the line of sight must be clear (buildings, vehicles, objects, camera-ignore) or it fails
/// with everything already written, and a blocked second test (buildings, objects) restores the old
/// source. Clears the target's visible flag (+0x1C bit 0x80); returns 1.
#[hook(addr = 0x512EF0, conv = thiscall, name = "FUN_00512ef0", expect = "8A 44 24 08 83 EC 34", this = "CCam")]
pub fn arrest_cam_head_view(this: GPtr<u8>, ped: GPtr<u8>, check: u32) -> u8 {
    let check = check as u8 != 0;
    if check {
        let r = sa_core::orig::at_site(0x512F04, sa_core::crt::rand);
        let v = X87::from_i32(r).fmul_m(k(K_RAND));
        if !v.fcom(x(k(K_0_65))).test_any(0x41) {
            return 0;
        }
    }
    let tgt = g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read();
    if rd::<u8>(tgt, 0x36) & 7 != 3 || ped.is_null() {
        return 0;
    }
    let mut f = Fpu::new();
    let now = TIME_MS.get();
    f.fild(now as i32);
    if (now as i32) < 0 {
        f.fadd_m(k(K_2P32));
    }
    f.fsub_m(g::<f32>(ARREST_START_TIME).read());
    f.fsub_m(ARREST_HEAD_T0.get());
    f.fdiv_m(g::<f32>(0x8CCCB0).read());
    let mut t = f.fst();
    f.fcomp_m(k(K_ONE));
    let toggle = if !f.jne(0x41) {
        t = k(K_ONE);
        true
    } else {
        f.fld(t);
        f.fcomp_m(k(K_ZERO));
        if !f.jp(0x05) {
            t = 0.0;
            false
        } else {
            f.fld(t);
            f.fcomp_m(k(K_ONE));
            !f.jne(0x01)
        }
    };
    if toggle {
        let was = ARREST_HEAD_TOGGLE.get();
        ARREST_HEAD_TOGGLE.set(0);
        if was == 0 {
            ARREST_HEAD_TOGGLE.set(1);
        }
    } else {
        ARREST_HEAD_TOGGLE.set(1);
    }
    let mut guard = ARREST_HEAD_GUARD.get();
    if guard & 1 == 0 {
        guard |= 1;
        ARREST_HEAD_GUARD.set(guard);
        ARREST_HEAD_ZERO.set([0; 3]);
    }
    if guard & 2 == 0 {
        ARREST_HEAD_GUARD.set(guard | 2);
        ARREST_HEAD_OFFSET.set([0, 0, 0xBF00_0000]);
    }
    let mut fr = Frame([0; 0x20]);
    let b = fr.b();
    let src = this.byte_add(0x19C);
    stu(b, 0x2C, rd::<u32>(src, 0));
    stu(b, 0x34, rd::<u32>(src, 8));
    stu(b, 0x30, rd::<u32>(src, 4));
    let p = pos_of(tgt);
    for o in [0, 4, 8] {
        wr::<u32>(src, o, rd::<u32>(p, o));
    }
    wr::<u32>(this, 0xB4, 0x42C8_0000);
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
    let idx = rp_hanim_id_get_index(h, 5);
    let head = rp_hanim_hierarchy_get_matrix_array(h)
        .byte_add(((idx as u32) << 6).wrapping_add(0x30) as i32 as isize);
    f.fld(g::<f32>(0x8CCCAC).read());
    stu(b, 0x14, rd::<u32>(head, 0));
    stu(b, 0x18, rd::<u32>(head, 4));
    stu(b, 0x1C, rd::<u32>(head, 8));
    f.fadd_m(m(b, 0x1C));
    f.fld(g::<f32>(0x8CCCA8).read());
    f.fmul_m(t);
    f.fsubp(1);
    st(b, 0x1C, f.fstp());
    let off = ARREST_HEAD_OFFSET.ptr().cast::<u8>();
    for o in [0, 4, 8] {
        f.fld(rd::<f32>(off, o));
        f.fadd_m(rd::<f32>(src, o));
        wr::<f32>(src, o, f.fstp());
    }
    let front = this.byte_add(0x190);
    for (i, o) in [0x20, 0x24, 0x28].into_iter().enumerate() {
        f.fld(m(b, 0x14 + 4 * i as isize));
        f.fsub_m(rd::<f32>(src, 4 * i as isize));
        st(b, o, f.fstp());
    }
    for (i, o) in [0x20, 0x24, 0x28].into_iter().enumerate() {
        wr::<u32>(front, 4 * i as isize, mu(b, o));
    }
    vector_normalise(front);
    let up = this.byte_add(0x1B4);
    stu(b, 0x20, 0);
    wr::<u32>(up, 0, 0);
    stu(b, 0x24, 0);
    wr::<u32>(up, 4, 0);
    stu(b, 0x28, K_ONE);
    wr::<u32>(up, 8, K_ONE);
    cross_product(b.byte_add(0x38), front, up);
    vector_normalise(b.byte_add(0x38));
    let o = cross_product(b.byte_add(0x20), b.byte_add(0x38), front);
    for i in [0, 4, 8] {
        wr::<u32>(up, i, rd::<u32>(o, i));
    }
    if check {
        if get_is_line_of_sight_clear(src, b.byte_add(0x14), 1, 1, 0, 1, 0, 0, 1) == 0 {
            return 0;
        }
        if get_is_line_of_sight_clear(src, b.byte_add(0x14), 1, 0, 0, 1, 0, 0, 0) == 0 {
            for (i, o) in [0x2C, 0x30, 0x34].into_iter().enumerate() {
                wr::<u32>(src, 4 * i as isize, mu(b, o));
            }
        }
    }
    let tgt = g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read();
    wr::<u32>(tgt, 0x1C, rd::<u32>(tgt, 0x1C) & 0xFFFF_FF7F);
    1
}

/// `FUN_00515d80(void* guard, CEntity* ent, const CVector* target, CVector* out)` (0x515D80, `ret
/// 0x10`, AL; ECX is never read). 0 when `guard` or `ent` is null. With P = the entity position and
/// T = *target: `D = T - P`; `Q = P + normalise(D x (0,0,1)) * [0x8CC8CC]`; `D` normalised, its z
/// limited to -0.7071 (x / y rescaled by `1 / (|D.xy| * 1.4142271)` when that is > 0) or raised to 0
/// from above, normalised again in those two cases; `F = T - (Q - D * [0x8CC8C8])` (y and z from the
/// unstored differences), stretched to `[0x8CC8D4]` when shorter but non-zero; `*out = T - F`; 1.
#[hook(addr = 0x515D80, conv = thiscall, name = "FUN_00515d80", expect = "8B 44 24 04 83 EC 30")]
pub fn arrest_cam_source_from(
    _this: GPtr<u8>,
    guard: GPtr<u8>,
    ent: GPtr<u8>,
    target: GPtr<u8>,
    out: GPtr<u8>,
) -> u8 {
    if guard.is_null() || ent.is_null() {
        return 0;
    }
    let mut fr = Frame([0; 0x20]);
    let b = fr.b();
    let kd = |a: usize| g::<f32>(a).read();
    let p = pos_of(ent);
    let (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
    let mut f = Fpu::new();
    f.fld(rd::<f32>(target, 0));
    stu(b, 0x10, px);
    f.fsub_m(m(b, 0x10));
    stu(b, 0x14, py);
    stu(b, 0x18, pz);
    st(b, 4, f.fstp());
    f.fld(rd::<f32>(target, 4));
    f.fsub_m(m(b, 0x14));
    st(b, 8, f.fstp());
    f.fld(rd::<f32>(target, 8));
    stu(b, 0x28, 0);
    f.fsub_m(m(b, 0x18));
    stu(b, 0x2C, 0);
    stu(b, 0x30, K_ONE);
    st(b, 0xC, f.fstp());
    cross_product(b.byte_add(0x1C), b.byte_add(4), b.byte_add(0x28));
    vector_normalise(b.byte_add(0x1C));
    f.fld(m(b, 0x1C));
    f.fmul_m(kd(0x8CC8CC));
    f.fld(m(b, 0x20));
    f.fmul_m(kd(0x8CC8CC));
    f.fld(m(b, 0x24));
    f.fmul_m(kd(0x8CC8CC));
    st(b, 0x30, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x10));
    st(b, 0x10, f.fstp());
    f.fadd_m(m(b, 0x14));
    st(b, 0x14, f.fstp());
    f.fld(m(b, 0x30));
    f.fadd_m(m(b, 0x18));
    st(b, 0x18, f.fstp());
    vector_normalise(b.byte_add(4));
    f.fld(m(b, 0xC));
    f.fcomp_m(k(K_M0_7071));
    let renorm = if !f.jp(0x05) {
        f.fld(m(b, 8));
        stu(b, 0xC, K_M0_7071);
        f.fmul_m(m(b, 8));
        f.fld(m(b, 4));
        f.fmul_m(m(b, 4));
        f.faddp(1);
        f.fsqrt();
        f.fmul_m(k(K_1_4142));
        f.fcom_m(k(K_ZERO));
        if !f.jne(0x41) {
            f.fdivr_m(k(K_ONE));
            f.fld(m(b, 4));
            f.fmul_st(1);
            st(b, 4, f.fstp());
            f.fmul_m(m(b, 8));
            st(b, 8, f.fstp());
        } else {
            f.fstp_st(0);
        }
        true
    } else {
        f.fld(m(b, 0xC));
        f.fcomp_m(k(K_ZERO));
        if !f.jne(0x41) {
            stu(b, 0xC, 0);
            true
        } else {
            false
        }
    };
    if renorm {
        vector_normalise(b.byte_add(4));
    }
    f.fld(kd(0x8CC8C8));
    f.fmul_m(m(b, 4));
    f.fld(kd(0x8CC8C8));
    f.fmul_m(m(b, 8));
    f.fld(kd(0x8CC8C8));
    f.fmul_m(m(b, 0xC));
    st(b, 0x30, f.fstp());
    f.fld(m(b, 0x10));
    f.fsub_st(2);
    st(b, 0x10, f.fstp());
    f.fld(m(b, 0x14));
    f.fsub_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    f.fld(m(b, 0x18));
    f.fsub_m(m(b, 0x30));
    f.fld(rd::<f32>(target, 0));
    f.fsub_m(m(b, 0x10));
    st(b, 0x10, f.fstp());
    f.fld(rd::<f32>(target, 4));
    stu(b, 4, mu(b, 0x10));
    f.fsub_st(2);
    st(b, 0x14, f.fstp());
    f.fld(rd::<f32>(target, 8));
    f.fsub_st(1);
    stu(b, 8, mu(b, 0x14));
    st(b, 0x18, f.fstp());
    f.fstp_st(0);
    stu(b, 0xC, mu(b, 0x18));
    f.fstp_st(0);
    f.fld(m(b, 0x18));
    f.fmul_m(m(b, 0x18));
    f.fld(m(b, 0x14));
    f.fmul_m(m(b, 0x14));
    f.faddp(1);
    f.fld(m(b, 0x10));
    f.fmul_m(m(b, 0x10));
    f.faddp(1);
    f.fsqrt();
    let len = f.fst();
    f.fcomp_m(kd(0x8CC8D4));
    let mut stretched = false;
    if !f.jp(0x05) {
        f.fld(len);
        f.fcomp_m(k(K_ZERO));
        if !f.jne(0x41) {
            f.fld(kd(0x8CC8D4));
            f.fdiv_m(len);
            f.fld(m(b, 0x10));
            f.fmul_st(1);
            st(b, 4, f.fstp());
            f.fld(m(b, 8));
            f.fmul_st(1);
            st(b, 8, f.fstp());
            f.fmul_m(m(b, 0xC));
            stretched = true;
        }
    }
    if !stretched {
        f.fld(m(b, 0xC));
    }
    f.fld(rd::<f32>(target, 0));
    f.fsub_m(m(b, 4));
    st(b, 0x10, f.fstp());
    f.fld(rd::<f32>(target, 4));
    f.fsub_m(m(b, 8));
    st(b, 0x14, f.fstp());
    f.fld(rd::<f32>(target, 8));
    f.fsub_st(1);
    wr::<u32>(out, 0, mu(b, 0x10));
    wr::<u32>(out, 4, mu(b, 0x14));
    st(b, 0x18, f.fstp());
    f.fstp_st(0);
    wr::<u32>(out, 8, mu(b, 0x18));
    1
}

/// `FUN_00516010(void* guard, CEntity* ent, const CVector* target, CVector* out)` (0x516010, `ret
/// 0x10`, AL; ECX is never read). 0 when `guard` or `ent` is null. `D = normalise(T.xy - P.xy, 0)`;
/// `out = T + D * [0x8CC7F0]` (x as `k * D.x`, y as `D.y * k`), `+ (D x (0,0,1)) * [0x8CC7F4]`;
/// `out.z = T.z + 5.0`; when `FindGroundZFor3DCoord(out)` finds ground, `out.z = [0x8CC7F8] + ground`.
#[hook(addr = 0x516010, conv = thiscall, name = "FUN_00516010", expect = "8B 44 24 04 83 EC 24")]
pub fn arrest_cam_side_pos(
    _this: GPtr<u8>,
    guard: GPtr<u8>,
    ent: GPtr<u8>,
    target: GPtr<u8>,
    out: GPtr<u8>,
) -> u8 {
    if guard.is_null() || ent.is_null() {
        return 0;
    }
    let mut fr = Frame([0; 0x20]);
    let b = fr.b();
    let kd = |a: usize| g::<f32>(a).read();
    let p = pos_of(ent);
    let (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
    let mut f = Fpu::new();
    f.fld(rd::<f32>(target, 0));
    stu(b, 0x14, px);
    f.fsub_m(m(b, 0x14));
    stu(b, 0x18, py);
    stu(b, 0x1C, pz);
    st(b, 8, f.fstp());
    stu(b, 0x10, 0);
    f.fld(rd::<f32>(target, 4));
    f.fsub_m(m(b, 0x18));
    st(b, 0xC, f.fstp());
    vector_normalise(b.byte_add(8));
    f.fld(kd(0x8CC7F0));
    f.fmul_m(m(b, 8));
    f.fld(m(b, 0xC));
    f.fmul_m(kd(0x8CC7F0));
    f.fld(kd(0x8CC7F0));
    f.fmul_m(m(b, 0x10));
    st(b, 0x28, f.fstp());
    f.fxch(1);
    f.fadd_m(rd::<f32>(target, 0));
    st(b, 0x14, f.fstp());
    let ox = mu(b, 0x14);
    stu(b, 0x14, 0);
    f.fadd_m(rd::<f32>(target, 4));
    st(b, 0x18, f.fstp());
    f.fld(m(b, 0x28));
    f.fadd_m(rd::<f32>(target, 8));
    wr::<u32>(out, 0, ox);
    wr::<u32>(out, 4, mu(b, 0x18));
    st(b, 0x1C, f.fstp());
    wr::<u32>(out, 8, mu(b, 0x1C));
    stu(b, 0x18, 0);
    stu(b, 0x1C, K_ONE);
    let c = cross_product(b.byte_add(0x20), b.byte_add(8), b.byte_add(0x14));
    f.fld(kd(0x8CC7F4));
    f.fmul_m(rd::<f32>(c, 0));
    f.fld(kd(0x8CC7F4));
    f.fmul_m(rd::<f32>(c, 4));
    f.fld(kd(0x8CC7F4));
    f.fmul_m(rd::<f32>(c, 8));
    let mut found: [u32; 1] = [0];
    st(b, 0x28, f.fstp());
    f.fxch(1);
    f.fadd_m(rd::<f32>(out, 0));
    let qx = f.fst();
    wr::<f32>(out, 0, f.fstp());
    f.fadd_m(rd::<f32>(out, 4));
    let qy = f.fst();
    wr::<f32>(out, 4, f.fstp());
    f.fld(m(b, 0x28));
    f.fadd_m(rd::<f32>(out, 8));
    wr::<f32>(out, 8, f.fstp());
    f.fld(rd::<f32>(target, 8));
    f.fadd_m(k(K_5));
    let qz = f.fst();
    wr::<f32>(out, 8, f.fstp());
    let gz = find_ground_z_for_3d_coord(
        qx.to_bits(),
        qy.to_bits(),
        qz.to_bits(),
        GPtr::<u32>::from_raw(found.as_mut_ptr()).cast::<u8>(),
        0,
    );
    f.push(X87::from_f64_exact(gz));
    if found[0] as u8 != 0 {
        f.fld(kd(0x8CC7F8));
        f.fadd_st(1);
        wr::<f32>(out, 8, f.fstp());
    }
    f.fstp_st(0);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ARREST_CAM_HEAD_VIEW,
    HOOK_ARREST_CAM_SOURCE_FROM,
    HOOK_ARREST_CAM_SIDE_POS,
];
