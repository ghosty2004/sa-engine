//! PORT-CAM-2: `CCam` mode processors (`this` = CCam 0x238 bytes): `ProcessPedsDeadBaby`,
//! `Process_SpecialFixedForSyphon`, `Process_AttachedCam`. Ported from the 1.0 US bytes; `docs/re/specs/CAM/CCam.md` as reading
//! aid (gta-reversed has only `NOTSA_UNREACHABLE()` stubs for them).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::{TIME_MS, TIME_STEP};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use sa_world::world::cworld::IGNORE_ENTITY;

use crate::common::{THE_CAMERA, ftol32, g, jp, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `ProcessPedsDeadBaby`'s start time `(float)(uint32)TimeInMilliseconds`, written on reset only
    pub static DEAD_BABY_START_TIME @ 0xB70054: f32 = 0.0;
    /// a u32 `ProcessPedsDeadBaby` zeroes on reset (never read there)
    pub static DEAD_BABY_B70050 @ 0xB70050: u32 = 0;
    /// `.data` tweak values of `ProcessPedsDeadBaby`: start height above the target (2.0)
    pub static DEAD_BABY_HEIGHT @ 0x8CC804: f32 = 2.0;
    /// rise per time step when the sphere test is clear (0.04)
    pub static DEAD_BABY_RISE @ 0x8CC808: f32 = 0.04;
    /// sideways swing scale (2.0)
    pub static DEAD_BABY_SWING @ 0x8CC80C: f32 = 2.0;
    /// swing period divisor in ms (600.0)
    pub static DEAD_BABY_PERIOD @ 0x8CC810: f32 = 600.0;
    /// look tilt along the right vector (-0.35)
    pub static DEAD_BABY_TILT @ 0x8CC814: f32 = -0.35;
}

/// `TheCamera.m_pTargetEntity` (+0x958), `m_nTimeLastChange` (+0x6C), `m_bMoveCamToAvoidGeom` (+0x2D).
const CAMERA_TARGET_ENTITY: usize = 0xB6F980;
const CAMERA_TIME_LAST_CHANGE: usize = 0xB6F094;
const CAMERA_MOVE_CAM_TO_AVOID_GEOM: usize = 0xB6F055;

/// 1.5 (0x858CE8), 0.2 (0x858CC4), 1000.0 (0x858C4C), 0.001 (0x858CDC), 2000.0 (0x8631F0), 0.0005
/// (0x859978), 2^32 (0x858C54); 0.3 (the pushed sphere radius).
const K_1_5: u32 = 0x3FC0_0000;
const K_0_2: u32 = 0x3E4C_CCCD;
const K_1000: u32 = 0x447A_0000;
const K_0_001: u32 = 0x3A83_126F;
const K_2000: u32 = 0x44FA_0000;
const K_0_0005: u32 = 0x3A03_126F;
const K_2P32: u32 = 0x4F80_0000;
const SPHERE_RADIUS: u32 = 0x3E99_999A;

orig! {
    /// `CPed::GetBonePosition(RwV3d& out, uint32 bone, bool updateSkin)` (0x5E4280)
    pub fn ped_get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: u32, update: u32) = thiscall 0x5E4280 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(float x, float y, float z, float* out, float*, float*)` (0x6E8580, AL)
    pub fn get_water_level_no_waves(x: u32, y: u32, z: u32, out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x6E8580 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector centre (by value), float r, CEntity* ignore, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies, bool camIgnore)` (0x569E20)
    pub fn test_sphere_against_world(cx: u32, cy: u32, cz: u32, r: u32, ignore: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, c: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CCamera::AvoidTheGeometry(const CVector* from, const CVector* target, CVector* out, float fov)` (0x514030)
    pub fn avoid_the_geometry(cam: GPtr<u8>, from: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>, fov: u32) = thiscall 0x514030 => unported;
    /// `CCam::GetVectorsReadyForRW()` (0x509CE0)
    pub fn get_vectors_ready_for_rw_call(cam: GPtr<u8>) = thiscall 0x509CE0 => crate::cam::get_vectors_ready_for_rw;
    /// `CPed::GetWeaponSkill()` (0x5E6580; tail call, the whole EAX is passed on)
    pub fn ped_get_weapon_skill(p: GPtr<u8>) -> u32 = thiscall 0x5E6580 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): `out = a - b`
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0)
    pub fn placeable_set_heading(e: GPtr<u8>, h: u32) = thiscall 0x43E0C0 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    pub fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790), returns `out`
    pub fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* out, bool waves, CVector* normal)`
    /// (0x6EB690, AL); the coordinates are pushed as dwords
    pub fn get_water_level(x: u32, y: u32, z: u32, out: GPtr<u8>, waves: u32, normal: GPtr<u8>) -> u8 = cdecl 0x6EB690 => unported;
}

/// A local 3-dword vector and its game pointer.
struct V3([u32; 3]);

impl V3 {
    fn p(&mut self) -> GPtr<u8> {
        GPtr::<u32>::from_raw(self.0.as_mut_ptr()).cast::<u8>()
    }
    fn f(&self, i: usize) -> f32 {
        f32::from_bits(self.0[i])
    }
}

/// Dword copy of 3 words.
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// The inlined "right vector of the target": `R = cross(M(t)->forward, (0,0,1))` (the matrix allocated and
/// updated from the placement first when missing), `R.z = 0`, `R.Normalise()`.
fn target_right(r: &mut V3) {
    let t = g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read();
    let mut z_up = V3([0, 0, 0x3F80_0000]);
    let mut tmp = V3([0; 3]);
    if rd::<GPtr<u8>>(t, 0x14).is_null() {
        allocate_matrix(t);
        simple_transform_update_matrix(t.byte_add(4), rd::<GPtr<u8>>(t, 0x14));
    }
    let fwd = rd::<GPtr<u8>>(t, 0x14).byte_add(0x10);
    let o = cross_product(tmp.p(), fwd, z_up.p());
    copy3(r.p(), o);
    r.0[2] = 0;
    vector_normalise(r.p());
}

/// `up = cross(R, front)` (into a stack temporary), then `up.Normalise()`.
fn up_from(this: GPtr<u8>, r: &mut V3) {
    let mut tmp = V3([0; 3]);
    let o = cross_product(tmp.p(), r.p(), this.byte_add(0x190));
    copy3(this.byte_add(0x1B4), o);
    vector_normalise(this.byte_add(0x1B4));
}

/// `(float)(uint32)v` (`fild` + 2^32 when negative), stored as f32.
fn u32_to_f32(v: u32) -> f32 {
    let mut f = X87::from_i32(v as i32);
    if (v as i32) < 0 {
        f = f.fadd_m(k(K_2P32));
    }
    f.to_f32()
}

/// `CCam::ProcessPedsDeadBaby()` (0x519250, thiscall): the camera over a dead player.
/// - Target point TP: a ped target's bone 3 (`GetBonePosition(&TP, 3, true)`), a vehicle's position with
///   `z + colModel->bbox.max.z`; other types return at once.
/// - Reset (+0xA): start time static = `(float)(u32)TimeMs`, `TheCamera.m_nTimeLastChange = TimeMs`,
///   the 0xB70050 static = 0; S = (TP.x, TP.y, 2.0 + TP.z); the water level (no waves) + 1.5 raises S.z
///   when greater; R = right of the target; front = normalised TP - S; up = normalised R x front; +0xA = 0.
/// - Otherwise S = source; S.z += TimeStep * 0.04 when a 0.3 sphere at S + (0,0,0.2) touches nothing;
///   with `dt = (float)(u32)(TimeMs - m_nTimeLastChange)` the swing point
///   Q = TP + R*2 * sin(dt/600) * min(1000, dt) * 0.001 (x on the stack, y through an f32) replaces S.xy when
///   a 0.3 sphere 0.2 along the normalised flat S->Q direction is clear; front = (0,0,-1) +
///   R*-0.35 * (f32)(min(2000, dt) * 0.0005) * (f32)cos(dt/600), normalised; up = normalised R x front.
/// - source = S; `TheCamera.AvoidTheGeometry(&S copy, &TP, &source, FOV)`; `m_bMoveCamToAvoidGeom` = 0.
#[hook(addr = 0x519250, conv = thiscall, name = "CCam::ProcessPedsDeadBaby", expect = "83 EC 5C 53 55 56 57", this = "CCam")]
pub fn process_peds_dead_baby(this: GPtr<u8>) {
    let t = g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read();
    let mut tp = V3([0; 3]);
    match rd::<u8>(t, 0x36) & 7 {
        3 => ped_get_bone_position(t, tp.p(), 3, 1),
        2 => {
            let m = rd::<GPtr<u8>>(t, 0x14);
            let pos = if m.is_null() {
                t.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            copy3(tp.p(), pos);
            let cm = get_col_model(t);
            tp.0[2] = x(tp.f(2)).fadd_m(rd::<f32>(cm, 0x14)).to_f32().to_bits();
        }
        _ => return,
    }
    let mut r = V3([0; 3]);
    // S = (sx, sy, sz): EBP, EBX and the local B+0x20
    let (mut sx, mut sy, mut sz);
    if rd::<u8>(this, 0xA) != 0 {
        let now = TIME_MS.get();
        DEAD_BABY_START_TIME.set(u32_to_f32(now));
        sz = x(DEAD_BABY_HEIGHT.get()).fadd_m(tp.f(2)).to_f32().to_bits();
        sy = tp.0[1];
        sx = tp.0[0];
        g::<u32>(CAMERA_TIME_LAST_CHANGE).write(now);
        DEAD_BABY_B70050.set(0);
        let mut wl: u32 = 0;
        let wlp = GPtr::<u32>::from_raw(&mut wl).cast::<u8>();
        if get_water_level_no_waves(sx, sy, sz, wlp, GPtr::NULL, GPtr::NULL) != 0 {
            let w = x(f32::from_bits(wlp.cast::<u32>().read())).fadd_m(k(K_1_5));
            if !w.fcom(x(f32::from_bits(sz))).test_any(0x41) {
                sz = w.to_f32().to_bits();
            }
        }
        target_right(&mut r);
        let front = this.byte_add(0x190);
        let d = [0, 1, 2].map(|i| x(tp.f(i)).fsub_m(f32::from_bits([sx, sy, sz][i])).to_f32());
        for (i, v) in d.iter().enumerate() {
            wr::<f32>(front, i as isize * 4, *v);
        }
        vector_normalise(front);
        up_from(this, &mut r);
        wr::<u8>(this, 0xA, 0);
    } else {
        let src = this.byte_add(0x19C);
        sx = rd::<u32>(src, 0);
        sy = rd::<u32>(src, 4);
        sz = rd::<u32>(src, 8);
        let target = g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read();
        // centre: x raw, y through fld/fstp, z + 0.2 stored as f32
        let cy = x(f32::from_bits(sy)).to_f32().to_bits();
        let cz = x(f32::from_bits(sz)).fadd_m(k(K_0_2)).to_f32().to_bits();
        if test_sphere_against_world(sx, cy, cz, SPHERE_RADIUS, target, 1, 1, 0, 1, 0, 1).is_null() {
            sz = x(TIME_STEP.get())
                .fmul_m(DEAD_BABY_RISE.get())
                .fadd_m(f32::from_bits(sz))
                .to_f32()
                .to_bits();
        }
        target_right(&mut r);
        let dt_i = TIME_MS
            .get()
            .wrapping_sub(g::<u32>(CAMERA_TIME_LAST_CHANGE).read());
        let dt = u32_to_f32(dt_i);
        let m = if jp(x(k(K_1000)), x(dt), 0x05) {
            x(dt)
        } else {
            x(k(K_1000))
        };
        let s = x(dt).fdiv_m(DEAD_BABY_PERIOD.get()).fsin();
        let kk = s.fmul(m.fmul_m(k(K_0_001)));
        let rx2 = x(r.f(0)).fmul_m(DEAD_BABY_SWING.get());
        let ry2 = x(r.f(1)).fmul_m(DEAD_BABY_SWING.get()).to_f32();
        let rx2k = rx2.fmul(kk);
        let ry2k = x(ry2).fmul(kk).to_f32();
        let qx = rx2k.fadd_m(tp.f(0)).to_f32();
        let qy = x(ry2k).fadd_m(tp.f(1)).to_f32();
        let sz_copy = sz;
        let mut d = V3([
            x(qx).fsub_m(f32::from_bits(sx)).to_f32().to_bits(),
            x(qy).fsub_m(f32::from_bits(sy)).to_f32().to_bits(),
            0,
        ]);
        vector_normalise(d.p());
        let dx2 = x(d.f(0)).fmul_m(k(K_0_2));
        let dy2 = x(d.f(1)).fmul_m(k(K_0_2));
        let dz2 = x(d.f(2)).fmul_m(k(K_0_2)).to_f32();
        let target = g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read();
        let cx = dx2.fadd_m(f32::from_bits(sx)).to_f32().to_bits();
        let cy = dy2.fadd_m(f32::from_bits(sy)).to_f32().to_bits();
        let cz = x(dz2).fadd_m(f32::from_bits(sz)).to_f32().to_bits();
        if test_sphere_against_world(cx, cy, cz, SPHERE_RADIUS, target, 1, 1, 0, 1, 0, 1).is_null() {
            sx = qx.to_bits();
            sy = qy.to_bits();
            sz = sz_copy;
        }
        let front = this.byte_add(0x190);
        wr::<u32>(front, 0, 0);
        wr::<u32>(front, 4, 0);
        wr::<u32>(front, 8, 0xBF80_0000);
        let m2 = if jp(x(k(K_2000)), x(dt), 0x05) {
            x(dt)
        } else {
            x(k(K_2000))
        };
        let tilt = DEAD_BABY_TILT.get();
        let rx = x(r.f(0)).fmul_m(tilt);
        let ry = x(r.f(1)).fmul_m(tilt).to_f32();
        let rz = x(tilt).fmul_m(r.f(2)).to_f32();
        let w = m2.fmul_m(k(K_0_0005)).to_f32();
        let rxw = rx.fmul_m(w);
        let ryw = x(ry).fmul_m(w);
        let rzw = x(rz).fmul_m(w).to_f32();
        let c = x(dt).fdiv_m(DEAD_BABY_PERIOD.get()).fcos().to_f32();
        let fx = rxw.fmul_m(c).to_f32();
        let rywc = ryw.fmul_m(c);
        let rzwc = x(rzw).fmul_m(c);
        wr::<f32>(front, 0, x(fx).fadd_m(rd::<f32>(front, 0)).to_f32());
        wr::<f32>(front, 4, rywc.fadd_m(rd::<f32>(front, 4)).to_f32());
        wr::<f32>(front, 8, rzwc.fadd_m(rd::<f32>(front, 8)).to_f32());
        vector_normalise(front);
        up_from(this, &mut r);
    }
    let src = this.byte_add(0x19C);
    wr::<u32>(src, 0, sx);
    wr::<u32>(src, 4, sy);
    wr::<u32>(src, 8, sz);
    let mut l = V3([sx, sy, sz]);
    avoid_the_geometry(
        GPtr::from_addr(THE_CAMERA),
        l.p(),
        tp.p(),
        src,
        rd::<u32>(this, 0xB4),
    );
    g::<u8>(CAMERA_MOVE_CAM_TO_AVOID_GEOM).write(0);
}

/// `&pos(e)`: the matrix position (+0x30) when the entity has a matrix (+0x14), else the placement (+4).
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CCam::Process_SpecialFixedForSyphon(const CVector& target, float, float, float)` (0x517500,
/// thiscall, `ret 0x10`; only `target` is read).
/// - source = fixed-mode source (+0x160); +0x148 = `*target` with `z = zo + z` (`zo` = +0x38 loaded
///   first, kept on the stack); front = `*target - source` (f32 per axis).
/// - `TheCamera.AvoidTheGeometry(&copy of +0x160, &+0x148, &source, FOV)`.
/// - `front.z = front.z + zo` (re-read); `GetVectorsReadyForRW()`; up += fixed-mode up offset (+0x16C;
///   x as `up + off`, y / z as `off + up`), normalised; `R = up x front`, normalised;
///   `front = R x up`, normalised; FOV = 70.0.
/// - When the cam's target (+0x21C) is a ped aiming at an entity (+0x71C) with its current weapon's info
///   (`GetWeaponInfo(type, GetWeaponSkill())`, the skill's whole EAX passed on) not requiring a duck it
///   lacks (`flags >> 1 & 1` with ped flag 0x4000000 clear returns) and a fire type (+0) not 0: with
///   `d = pos(aimed) - pos(ped)` (`VectorSub`), `h = (f32)atan2(-d.x, d.y)` (FPATAN, sa-math's f64
///   model) becomes the ped's +0x55C and +0x558, `SetHeading(h)`, `UpdateRwMatrix()`. The original
///   stores `h` in its own `target` argument slot first (the caller does not read it back).
#[hook(addr = 0x517500, conv = thiscall, name = "CCam::Process_SpecialFixedForSyphon", expect = "83 EC 18 53 55 56", this = "CCam")]
pub fn process_special_fixed_for_syphon(this: GPtr<u8>, target: GPtr<u8>, _a: f32, _b: f32, _c: f32) {
    let zo = x(rd::<f32>(this, 0x38));
    let fixed = this.byte_add(0x160);
    let src = this.byte_add(0x19C);
    copy3(src, fixed);
    let fudge = this.byte_add(0x148);
    copy3(fudge, target);
    wr::<f32>(fudge, 8, zo.fadd_m(rd::<f32>(fudge, 8)).to_f32());
    let front = this.byte_add(0x190);
    let d = [0, 4, 8].map(|o| x(rd::<f32>(target, o)).fsub_m(rd::<f32>(src, o)).to_f32());
    for (i, v) in d.iter().enumerate() {
        wr::<f32>(front, i as isize * 4, *v);
    }
    let mut l = V3([rd::<u32>(fixed, 0), rd::<u32>(fixed, 4), rd::<u32>(fixed, 8)]);
    avoid_the_geometry(
        GPtr::from_addr(THE_CAMERA),
        l.p(),
        fudge,
        src,
        rd::<u32>(this, 0xB4),
    );
    wr::<f32>(
        front,
        8,
        x(rd::<f32>(front, 8)).fadd_m(rd::<f32>(this, 0x38)).to_f32(),
    );
    get_vectors_ready_for_rw_call(this);
    let up = this.byte_add(0x1B4);
    wr::<f32>(up, 0, x(rd::<f32>(up, 0)).fadd_m(rd::<f32>(this, 0x16C)).to_f32());
    wr::<f32>(up, 4, x(rd::<f32>(this, 0x170)).fadd_m(rd::<f32>(up, 4)).to_f32());
    wr::<f32>(up, 8, x(rd::<f32>(this, 0x174)).fadd_m(rd::<f32>(up, 8)).to_f32());
    vector_normalise(up);
    let mut r = V3([0; 3]);
    cross_product(r.p(), up, front);
    vector_normalise(r.p());
    let o = cross_product(l.p(), r.p(), up);
    copy3(front, o);
    vector_normalise(front);
    wr::<u32>(this, 0xB4, 0x428C_0000);

    let p = rd::<GPtr<u8>>(this, 0x21C);
    if p.is_null() || rd::<u8>(p, 0x36) & 7 != 3 || rd::<GPtr<u8>>(p, 0x71C).is_null() {
        return;
    }
    let slot = isize::from(rd::<i8>(p, 0x718));
    let ty = rd::<u32>(p, slot * 0x1C + 0x5A0);
    let wi = get_weapon_info(ty, ped_get_weapon_skill(p));
    if wi.is_null() {
        return;
    }
    if (rd::<u32>(wi, 0x18) >> 1) & 1 != 0 && rd::<u32>(p, 0x46C) & 0x0400_0000 == 0 {
        return;
    }
    if rd::<u32>(wi, 0) == 0 {
        return;
    }
    let ped_pos = pos_of(p);
    let aim_pos = pos_of(rd::<GPtr<u8>>(p, 0x71C));
    let mut d = V3([0; 3]);
    vector_sub(d.p(), aim_pos, ped_pos);
    let h = x(d.f(0)).fchs().fpatan(x(d.f(1))).to_f32();
    wr::<f32>(p, 0x55C, h);
    wr::<f32>(p, 0x558, h);
    placeable_set_heading(p, h.to_bits());
    update_rw_matrix(p);
}

/// `TheCamera.m_vecAttachedCamOffset` / `m_vecAttachedCamLookAt` / `m_fAttachedCamAngle` /
/// `m_pAttachedEntity` / `m_bLookingAtVector`.
const CAMERA_ATTACHED_OFFSET: usize = 0xB6F960;
const CAMERA_ATTACHED_LOOK_AT: usize = 0xB6F96C;
const CAMERA_ATTACHED_ANGLE: usize = 0xB6F978;
const CAMERA_ATTACHED_ENTITY: usize = 0xB6F984;
const CAMERA_LOOKING_AT_VECTOR: usize = 0xB6F054;
/// `TheCamera.m_nBlurBlue` / `m_nBlurGreen` / `m_nBlurRed` / `m_nBlurType` / `m_nMotionBlur`.
const CAMERA_BLUR_BLUE: usize = 0xB6F0AC;
const CAMERA_BLUR_GREEN: usize = 0xB6F0B0;
const CAMERA_BLUR_RED: usize = 0xB6F0B4;
const CAMERA_BLUR_TYPE: usize = 0xB6F0B8;
const CAMERA_MOTION_BLUR: usize = 0xB6F0C8;
/// `CTimeCycle::m_CurrentColours` water red / green / blue (owner: timecycle).
const WATER_R: usize = 0xB7C508;
const WATER_G: usize = 0xB7C50C;
const WATER_B: usize = 0xB7C510;
/// `.data` tweak values (also read by camproc.rs): the colour magnitude cap 0x8CC7A8 (10.0), the motion
/// blur alpha 0x8CC7A4 (20).
const UW_COLOUR_CAP: usize = 0x8CC7A8;
const UW_ALPHA: usize = 0x8CC7A4;
/// 0.017453292 (0x8595EC), 0.3 (0x858C24)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_0_3: u32 = 0x3E99_999A;

/// The attached entity's matrix, allocated and updated from the placement first when missing.
fn attached_matrix() -> GPtr<u8> {
    let e = g::<GPtr<u8>>(CAMERA_ATTACHED_ENTITY).read();
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `v = Multiply3x3(M(attached), off) + pos(attached)` (x as `v + p`, y / z as `p + v`).
fn attached_point(v: GPtr<u8>, off: usize) {
    let mut tmp = V3([0; 3]);
    let o = multiply_3x3(tmp.p(), attached_matrix(), GPtr::from_addr(off));
    copy3(v, o);
    let p = pos_of(g::<GPtr<u8>>(CAMERA_ATTACHED_ENTITY).read());
    wr::<f32>(v, 0, x(rd::<f32>(v, 0)).fadd_m(rd::<f32>(p, 0)).to_f32());
    wr::<f32>(v, 4, x(rd::<f32>(p, 4)).fadd_m(rd::<f32>(v, 4)).to_f32());
    wr::<f32>(v, 8, x(rd::<f32>(p, 8)).fadd_m(rd::<f32>(v, 8)).to_f32());
}

/// `CCam::Process_AttachedCam()` (0x512B10, thiscall): FOV = 70.0; `ang = (f32)(angle * 0.017453292)`;
/// source = attached point of the offset; front = attached point of the look-at minus source (looking at a
/// vector) or `pos(TheCamera target) - source`, normalised; `R = front x (0,0,1)`, normalised;
/// `U = R x front`, normalised. Under water (`GetWaterLevel` with waves, `wl - 0.3 > source.z`) the blur
/// colour is the timecycle water colour (`_ftol2` each), scaled by `10 / |c|` when `|c| = sqrt((b*b + g*g)
/// + r*r) > 10`, alpha 20 (0x8CC7A4), type 2. Then `up = U*(f32)cos(ang) + R*sin(ang)` (x: the R term
/// unrounded on the stack; y / z through f32) and `CWorld::pIgnoreEntity = NULL`.
#[hook(addr = 0x512B10, conv = thiscall, name = "CCam::Process_AttachedCam", expect = "83 EC 38 53 8B D9", this = "CCam")]
pub fn process_attached_cam(this: GPtr<u8>) {
    wr::<u32>(this, 0xB4, 0x428C_0000);
    let ang = x(g::<f32>(CAMERA_ATTACHED_ANGLE).read())
        .fmul_m(k(K_DEG2RAD))
        .to_f32();
    let src = this.byte_add(0x19C);
    attached_point(src, CAMERA_ATTACHED_OFFSET);
    let front = this.byte_add(0x190);
    if g::<u8>(CAMERA_LOOKING_AT_VECTOR).read() != 0 {
        attached_point(front, CAMERA_ATTACHED_LOOK_AT);
        for o in [0, 4, 8] {
            wr::<f32>(
                front,
                o,
                x(rd::<f32>(front, o)).fsub_m(rd::<f32>(src, o)).to_f32(),
            );
        }
    } else {
        let p = pos_of(g::<GPtr<u8>>(CAMERA_TARGET_ENTITY).read());
        let d = [0, 4, 8].map(|o| x(rd::<f32>(p, o)).fsub_m(rd::<f32>(src, o)).to_f32());
        for (i, v) in d.iter().enumerate() {
            wr::<f32>(front, i as isize * 4, *v);
        }
    }
    vector_normalise(front);
    let mut u = V3([0, 0, 0x3F80_0000]);
    let mut r = V3([0; 3]);
    let mut tmp = V3([0; 3]);
    let o = cross_product(tmp.p(), front, u.p());
    copy3(r.p(), o);
    vector_normalise(r.p());
    let o = cross_product(tmp.p(), r.p(), front);
    copy3(u.p(), o);
    vector_normalise(u.p());

    let mut wl: u32 = 0;
    let wlp = GPtr::<u32>::from_raw(&mut wl).cast::<u8>();
    let (sx, sy, sz) = (rd::<u32>(src, 0), rd::<u32>(src, 4), rd::<u32>(src, 8));
    if get_water_level(sx, sy, sz, wlp, 1, GPtr::NULL) != 0
        && !x(f32::from_bits(wlp.cast::<u32>().read()))
            .fsub_m(k(K_0_3))
            .fcom(x(rd::<f32>(src, 8)))
            .test_any(0x41)
    {
        let (cr, cg, cb) = (
            g::<f32>(WATER_R).read(),
            g::<f32>(WATER_G).read(),
            g::<f32>(WATER_B).read(),
        );
        let m = x(cb)
            .fmul_m(cb)
            .fadd(x(cg).fmul_m(cg))
            .fadd(x(cr).fmul_m(cr))
            .fsqrt();
        let cap = g::<f32>(UW_COLOUR_CAP).read();
        let blue = if !m.fcom(x(cap)).test_any(0x41) {
            let s = m.fdivr_m(cap);
            g::<i32>(CAMERA_BLUR_RED).write(ftol32(x(g::<f32>(WATER_R).read()).fmul(s)));
            g::<i32>(CAMERA_BLUR_GREEN).write(ftol32(x(g::<f32>(WATER_G).read()).fmul(s)));
            ftol32(x(g::<f32>(WATER_B).read()).fmul(s))
        } else {
            g::<i32>(CAMERA_BLUR_RED).write(ftol32(x(g::<f32>(WATER_R).read())));
            g::<i32>(CAMERA_BLUR_GREEN).write(ftol32(x(g::<f32>(WATER_G).read())));
            ftol32(x(g::<f32>(WATER_B).read()))
        };
        g::<u32>(CAMERA_MOTION_BLUR).write(g::<u32>(UW_ALPHA).read());
        g::<i32>(CAMERA_BLUR_BLUE).write(blue);
        g::<u32>(CAMERA_BLUR_TYPE).write(2);
    }

    let sa = x(ang).fsin();
    let rxs = x(r.f(0)).fmul(sa);
    let rys = x(r.f(1)).fmul(sa).to_f32();
    let rzs = x(r.f(2)).fmul(sa).to_f32();
    let ca = x(ang).fcos();
    let uxc = x(u.f(0)).fmul(ca).to_f32();
    let uyc = x(u.f(1)).fmul(ca).to_f32();
    let uzc = x(u.f(2)).fmul(ca).to_f32();
    let up = this.byte_add(0x1B4);
    wr::<f32>(up, 0, x(uxc).fadd(rxs).to_f32());
    wr::<f32>(up, 4, x(uyc).fadd_m(rys).to_f32());
    wr::<f32>(up, 8, x(uzc).fadd_m(rzs).to_f32());
    IGNORE_ENTITY.set(GPtr::NULL);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_PEDS_DEAD_BABY,
    HOOK_PROCESS_SPECIAL_FIXED_FOR_SYPHON,
    HOOK_PROCESS_ATTACHED_CAM,
];
