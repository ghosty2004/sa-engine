//! PORT-CAM-3 (helper B): the "DW" cinematic vehicle cameras (`CCam::Process_DW_*`) and their helpers
//! (`Finalise_DW_CineyCams`, `GetCoreDataForDWCineyCamMode`, `IsTimeToExitThisDWCineyCamMode`,
//! `GetLookFromLampPostPos`). Ported from the 1.0 US bytes; gta-reversed (where reversed) and
//! `docs/re/specs/CAM/CCam.md` as reading aids. Several entries are `nop; jmp` into a `.text`
//! protection fragment that does part of the prologue (`sub esp, N`, a register load) and jumps back;
//! the ports inline them.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::{FRAME_COUNTER, TIME_MS};
use sa_core::{hook, orig};

use sa_world::world::cworld::IGNORE_ENTITY;

use sa_math::x87::X87;

use crate::common::{Fpu, f32_of, g, jne, jp, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `gLastFrameProcessedDWCineyCam`: the frame counter of the last `Finalise_DW_CineyCams`
    pub static DW_LAST_FRAME_PROCESSED @ 0x8CCB9C: u32 = 0xFFFF_FFFF;
    /// `s_DWCineyCamSceneEndTime` (ms)
    pub static DW_SCENE_END_TIME @ 0x8CCBA4: u32 = 0xFFFF_FFFF;
    /// `IsTimeToExitThisDWCineyCamMode`'s minimum distances, modes 0x14..0x1C
    pub static DW_MIN_DIST @ 0x8CCBCC: [f32; 9] = [3.0, 3.0, 1.0, 3.0, 5.0, 3.0, 3.0, 3.0, 3.0];
    /// ... and maximum distances
    pub static DW_MAX_DIST @ 0x8CCBF0: [f32; 9] = [185.0, 100.0, 100.0, 100.0, 30.0, 30.0, 100.0, 100.0, 100.0];
    /// `GetLookFromLampPostPos`' preferred lamp distance (17.0)
    pub static LAMP_POST_BEST_DIST @ 0x8CC8D8: f32 = 17.0;
}

/// `s_ExitCam[mode]` (bool array, indexed by the camera mode)
const DW_EXIT_CAM: u32 = 0xB6EC5C;
/// `Scene.m_pRwCamera`
const SCENE_CAMERA: usize = 0xC1703C;
/// `gHandShaker[0]` and its result matrix (+0x3C) / roll factor (+8)
const HAND_SHAKER: usize = 0xB6ECA0;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;

/// 0.0 (0x858B50), 0.0001 (stored by the zero test), 0.4 (near clip), 0.9 (0x858C20), 5.0 (0x858C80),
/// 30.0, 10000.0.
const K_ZERO: u32 = 0;
const K_FRONT_EPS: u32 = 0x38D1_B717;
const K_NEAR_CLIP: u32 = 0x3ECC_CCCD;
const K_0_9: u32 = 0x3F66_6666;
const K_5: u32 = 0x40A0_0000;
const K_30: u32 = 0x41F0_0000;
const K_10000: u32 = 0x461C_4000;

orig! {
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `RwCameraSetNearClipPlane(RwCamera*, RwReal)` (0x7EE1D0)
    pub fn rw_camera_set_near_clip_plane(cam: GPtr<u8>, near: u32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `CCam::CacheLastSettingsDWCineyCam()` (0x50D7A0)
    pub fn cache_last_settings_dw_ciney_cam(cam: GPtr<u8>) = thiscall 0x50D7A0 => crate::cam::cache_last_settings_dw_ciney_cam;
    /// `CHandShaker::Process(float intensity)` (0x50D930), the intensity as the dword pushed
    pub fn hand_shaker_process(hs: GPtr<u8>, intensity: u32) = thiscall 0x50D930 => unported;
    /// `Multiply3x3(CVector* out, const CVector* v, const CMatrix* m)` (0x59C810, cdecl), returns `out`
    pub fn multiply3x3_vm(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `CEntity::GetBoundCentre(CVector* out)` (0x534250), returns `out`
    pub fn get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x534250 => unported;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 flags, uint8 lighting)`
    /// (0x40FD10); the radius as its dword
    pub fn col_sphere_set(sph: GPtr<u8>, r: u32, c: GPtr<u8>, material: u32, flags: u32, lighting: u32) = thiscall 0x40FD10 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool x8)`
    /// (0x56BA00, AL)
    pub fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, hit: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, cam: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool ignoreZ, int16* count, int16 max,
    /// CEntity** out, bool buildings, bool vehicles, bool peds, bool objects, bool dummies)` (0x564A20)
    pub fn find_objects_in_range(c: GPtr<u8>, r: u32, ignore_z: u32, count: GPtr<u8>, max: u32, out: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32) = cdecl 0x564A20 => unported;
    /// `IsLampPost(int32 modelId)` (0x509A30, AL)
    pub fn is_lamp_post(model: i32) -> u8 = cdecl 0x509A30 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890, cdecl), returns `out`
    pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490, AL)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
}

/// A frame-local CVector and its game pointer.
fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::<f32>::from_raw(v.as_mut_ptr()).cast::<u8>()
}

/// Dword copy of a 3-word vector.
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// The inlined `GetMatrix()`: allocates the matrix from the placement when missing.
fn ensure_matrix(e: GPtr<u8>) {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
}

/// The two orientation passes of `Finalise_DW_CineyCams`: `source = src`; `up = (sin roll, 0, cos
/// roll)` (f32, written into `m_vecUp` itself); `R = normalise(front x up)`; `up = R x front`; a front
/// with x and y both `== 0.0` gets 0.0001 in both; `R = normalise(front x up)`; `up = R x front`.
fn orient(this: GPtr<u8>, src: GPtr<u8>, roll: f32) {
    let front = this.byte_add(0x190);
    let up = this.byte_add(0x1B4);
    let s = f32_of(x(roll).fsin());
    copy3(this.byte_add(0x19C), src);
    let c = f32_of(x(roll).fcos());
    wr::<f32>(up, 0, s);
    wr::<u32>(up, 4, 0);
    wr::<f32>(up, 8, c);
    let mut r: [f32; 3] = [0.0; 3];
    let mut t: [f32; 3] = [0.0; 3];
    let o = cross_product(vp(&mut t), front, up);
    copy3(vp(&mut r), o);
    vector_normalise(vp(&mut r));
    let o = cross_product(vp(&mut t), vp(&mut r), front);
    copy3(up, o);
    let zero = x(k(K_ZERO));
    if !jp(x(rd::<f32>(front, 0)), zero, 0x44) && !jp(x(rd::<f32>(front, 4)), zero, 0x44) {
        wr::<u32>(front, 0, K_FRONT_EPS);
        wr::<u32>(front, 4, K_FRONT_EPS);
    }
    let o = cross_product(vp(&mut t), front, up);
    copy3(vp(&mut r), o);
    vector_normalise(vp(&mut r));
    let o = cross_product(vp(&mut t), vp(&mut r), front);
    copy3(up, o);
}

/// `CCam::Finalise_DW_CineyCams(const CVector* src, const CVector* dest, float roll, float fov, float
/// nearClip, float shake)` (0x50DD70, thiscall, `ret 0x18`). `front = normalise(dest - src)` (f32
/// differences), the orientation pass, `FOV = fov`, near clip 0.4 (the `nearClip` argument is never
/// read), `CacheLastSettingsDWCineyCam`, `gLastFrameProcessedDWCineyCam = frame counter`,
/// `gHandShaker[0].Process(shake)`, then — unlike gta-reversed, which keeps `roll` — the roll argument
/// becomes `gHandShaker[0] +8 * shake` (f32), `front = normalise(Multiply3x3(front, resultMat))`, and
/// the orientation pass runs again with the new roll (source copied again).
#[hook(addr = 0x50DD70, conv = thiscall, name = "CCam::Finalise_DW_CineyCams", expect = "83 EC 24 8B 44 24 2C", this = "CCam")]
pub fn finalise_dw_ciney_cams(
    this: GPtr<u8>,
    src: GPtr<u8>,
    dest: GPtr<u8>,
    roll: u32,
    fov: u32,
    _near_clip: u32,
    shake: u32,
) {
    let front = this.byte_add(0x190);
    let d = [0isize, 4, 8].map(|o| f32_of(x(rd::<f32>(dest, o)).fsub_m(rd::<f32>(src, o))));
    for (i, v) in d.into_iter().enumerate() {
        wr::<f32>(front, 4 * i as isize, v);
    }
    vector_normalise(front);
    orient(this, src, f32::from_bits(roll));
    wr::<u32>(this, 0xB4, fov);
    rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), K_NEAR_CLIP);
    cache_last_settings_dw_ciney_cam(this);
    DW_LAST_FRAME_PROCESSED.set(FRAME_COUNTER.get());
    hand_shaker_process(GPtr::from_addr(HAND_SHAKER), shake);
    let roll2 = f32_of(x(g::<f32>(HAND_SHAKER + 8).read()).fmul_m(f32::from_bits(shake)));
    let mut t: [f32; 3] = [0.0; 3];
    multiply3x3_vm(vp(&mut t), front, GPtr::from_addr(HAND_SHAKER + 0x3C));
    copy3(front, vp(&mut t));
    vector_normalise(front);
    orient(this, src, roll2);
}

/// `CCam::GetCoreDataForDWCineyCamMode(CEntity** e, CVehicle** v, CVector* dest, CVector* src,
/// CVector* up, CVector* right, CVector* fwd, CVector* vel, float* speed, CVector* angVel, float*
/// angSpeed, CColSphere* sphere)` (0x517130, thiscall, `ret 0x30`; the entry fragment 0x407E04 loads
/// `ECX = this->m_pCamTargetEntity`). `*e = *v = target`; then everything through `*v` (re-read):
/// `dest` = position, `src` = `DWCineyCamLastPos`, up / right / forward from the matrix, or without
/// one `(0,0,1)`, `(cos h, sin h, 0)`, `(-sin h, cos h, 0)` of the placement heading; `vel` = move
/// speed, `speed = |vel|` (`(x^2 + y^2) + z^2`), `angVel` = turn speed, `angSpeed = |angVel|`; the sphere
/// = (model col model bound radius, `GetBoundCentre`, material 0, flags 0, lighting 0xFF).
#[hook(addr = 0x517130, conv = thiscall, name = "CCam::GetCoreDataForDWCineyCamMode", expect = "90 E9 CE 0C EF FF", this = "CCam")]
#[allow(clippy::too_many_arguments)]
pub fn get_core_data_for_dw_ciney_cam_mode(
    this: GPtr<u8>,
    e_out: GPtr<u8>,
    v_out: GPtr<u8>,
    dest: GPtr<u8>,
    src: GPtr<u8>,
    up: GPtr<u8>,
    right: GPtr<u8>,
    fwd: GPtr<u8>,
    vel: GPtr<u8>,
    speed: GPtr<u8>,
    ang_vel: GPtr<u8>,
    ang_speed: GPtr<u8>,
    sphere: GPtr<u8>,
) {
    let t = rd::<GPtr<u8>>(this, 0x21C);
    wr::<GPtr<u8>>(e_out, 0, t);
    wr::<GPtr<u8>>(v_out, 0, t);
    let ent = || rd::<GPtr<u8>>(v_out, 0);
    let m = rd::<GPtr<u8>>(t, 0x14);
    copy3(
        dest,
        if m.is_null() {
            t.byte_add(4)
        } else {
            m.byte_add(0x30)
        },
    );
    copy3(src, GPtr::from_addr(0xB6FE8C));
    let m = rd::<GPtr<u8>>(ent(), 0x14);
    if m.is_null() {
        wr::<u32>(up, 0, 0);
        wr::<u32>(up, 4, 0);
        wr::<u32>(up, 8, 0x3F80_0000);
    } else {
        copy3(up, m.byte_add(0x20));
    }
    let e = ent();
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        let c = f32_of(x(rd::<f32>(e, 0x10)).fcos());
        let s = f32_of(x(rd::<f32>(e, 0x10)).fsin());
        wr::<f32>(right, 0, c);
        wr::<f32>(right, 4, s);
        wr::<u32>(right, 8, 0);
    } else {
        copy3(right, m);
    }
    let e = ent();
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        let s = f32_of(x(rd::<f32>(e, 0x10)).fsin().fchs());
        let c = f32_of(x(rd::<f32>(e, 0x10)).fcos());
        wr::<f32>(fwd, 0, s);
        wr::<f32>(fwd, 4, c);
        wr::<u32>(fwd, 8, 0);
    } else {
        copy3(fwd, m.byte_add(0x10));
    }
    let mag = |v: GPtr<u8>| {
        let (vx, vy, vz) = (x(rd::<f32>(v, 0)), x(rd::<f32>(v, 4)), x(rd::<f32>(v, 8)));
        f32_of(vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt())
    };
    copy3(vel, ent().byte_add(0x44));
    wr::<f32>(speed, 0, mag(vel));
    copy3(ang_vel, ent().byte_add(0x50));
    wr::<f32>(ang_speed, 0, mag(ang_vel));
    let e = ent();
    let mi = g::<GPtr<u8>>(
        MODEL_INFO_PTRS.wrapping_add((i32::from(rd::<i16>(e, 0x22)) as u32).wrapping_mul(4)) as usize,
    )
    .read();
    let radius = rd::<u32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
    let mut c: [f32; 3] = [0.0; 3];
    let cp = get_bound_centre(e, vp(&mut c));
    col_sphere_set(sphere, radius, cp, 0, 0, 0xFF);
}

/// `CCam::IsTimeToExitThisDWCineyCamMode(int32 mode, const CVector* src, const CVector* dst, float t,
/// bool lineOfSight)` (0x517400, thiscall, `ret 0x14`, AL; `t` is never read). `s_ExitCam[mode]` -> 1.
/// The bytes compute the band test before the mode range check (so the distance tables are indexed
/// with any mode): `in = !(d < min[mode - 0x14]) && d <= max[mode - 0x14]` with `d = |dst - src|`
/// (`(z^2 + x^2) + y^2`); with `lineOfSight`: `clear = !ProcessLineOfSight(dst, src, .., buildings,
/// vehicles)` with the target ignored. A mode outside 0x14..0x1C -> 0; else 1 unless in band, clear
/// and `TimeInMs <= s_DWCineyCamSceneEndTime`.
#[hook(addr = 0x517400, conv = thiscall, name = "CCam::IsTimeToExitThisDWCineyCamMode", expect = "83 EC 2C 57 8B 7C 24 34", this = "CCam")]
pub fn is_time_to_exit_this_dw_ciney_cam_mode(
    this: GPtr<u8>,
    mode: i32,
    src: GPtr<u8>,
    dst: GPtr<u8>,
    _t: u32,
    line_of_sight: u32,
) -> u8 {
    if g::<u8>(DW_EXIT_CAM.wrapping_add(mode as u32) as usize).read() != 0 {
        return 1;
    }
    let i = (mode as u32).wrapping_sub(0x14).wrapping_mul(4);
    let dx = x(rd::<f32>(dst, 0)).fsub_m(rd::<f32>(src, 0));
    let dy = x(rd::<f32>(dst, 4)).fsub_m(rd::<f32>(src, 4));
    let dz = x(rd::<f32>(dst, 8)).fsub_m(rd::<f32>(src, 8));
    let dist = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt();
    let in_band = if jne(
        dist,
        x(g::<f32>(0x8CCBCCu32.wrapping_add(i) as usize).read()),
        0x01,
    ) {
        false
    } else {
        !jp(
            dist,
            x(g::<f32>(0x8CCBF0u32.wrapping_add(i) as usize).read()),
            0x41,
        )
    };
    let mut clear = true;
    if line_of_sight as u8 != 0 {
        IGNORE_ENTITY.set(rd::<GPtr<u8>>(this, 0x21C));
        let mut cp = [0u32; 11];
        let mut hit: u32 = 0;
        let r = process_line_of_sight(
            dst,
            src,
            GPtr::<u32>::from_raw(cp.as_mut_ptr()).cast::<u8>(),
            GPtr::<u32>::from_raw(&mut hit).cast::<u8>(),
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
        );
        clear = r == 0;
        IGNORE_ENTITY.set(GPtr::NULL);
    }
    if !(0x14..=0x1C).contains(&mode) {
        return 0;
    }
    u8::from(!(in_band && clear && TIME_MS.get() <= DW_SCENE_END_TIME.get()))
}

/// `CCam::GetLookFromLampPostPos(CEntity* target, CPed* cop, const CVector& targetPos, CVector& out)`
/// (0x5161A0, thiscall, `ret 0x10`, AL). Up to 15 objects / dummies within 30 (2D) of `targetPos`; for
/// each static (or waiting-for-collision) one whose up.z > 0.9, a lamp post (`IsLampPost`), more than
/// 5.0 away in 2D and with `|17.0 - dist|` below the best so far (10000 at first): `top = matrix *
/// colModel.bbox.max`; when `GetIsLineOfSightClear(top, normalise(top - targetPos) + targetPos,
/// buildings, seeThrough, ignoreSomeObjects)`, it wins (`out = top`). Returns whether one won.
#[hook(addr = 0x5161A0, conv = thiscall, name = "CCam::GetLookFromLampPostPos", expect = "81 EC 80 00 00 00 55 56", this = "CCam")]
pub fn get_look_from_lamp_post_pos(
    _this: GPtr<u8>,
    _target: GPtr<u8>,
    _cop: GPtr<u8>,
    tp: GPtr<u8>,
    out: GPtr<u8>,
) -> u8 {
    let mut count: [i16; 2] = [0; 2];
    let mut ents = [0u32; 16];
    find_objects_in_range(
        tp,
        K_30,
        1,
        GPtr::<i16>::from_raw(count.as_mut_ptr()).cast::<u8>(),
        0xF,
        GPtr::<u32>::from_raw(ents.as_mut_ptr()).cast::<u8>(),
        0,
        0,
        0,
        1,
        1,
    );
    let mut winner: u32 = 0;
    let mut closest = k(K_10000);
    if count[0] <= 0 {
        return 0;
    }
    let mut i: i16 = 0;
    loop {
        let slot = ents[i as usize];
        let e = GPtr::<u8>::from_addr(slot as usize);
        'next: {
            let f = rd::<u32>(e, 0x1C);
            if f & 4 == 0 && f & 0x40000 == 0 {
                break 'next;
            }
            ensure_matrix(e);
            if jne(x(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 0x28)), x(k(K_0_9)), 0x41) {
                break 'next;
            }
            if is_lamp_post(i32::from(rd::<i16>(e, 0x22))) == 0 {
                break 'next;
            }
            let m = rd::<GPtr<u8>>(e, 0x14);
            let p = if m.is_null() {
                e.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(tp, 0));
            let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(tp, 4));
            let dv = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
            let dist = f32_of(dv);
            if jne(dv, x(k(K_5)), 0x41) {
                break 'next;
            }
            if jp(x(LAMP_POST_BEST_DIST.get()).fsub_m(dist).fabs(), x(closest), 0x05) {
                break 'next;
            }
            let cm = get_col_model(e);
            let mut top: [f32; 3] = [0.0; 3];
            copy3(vp(&mut top), cm.byte_add(0xC));
            ensure_matrix(e);
            let mut t: [f32; 3] = [0.0; 3];
            let r = matrix_mul_vec(vp(&mut t), rd::<GPtr<u8>>(e, 0x14), vp(&mut top));
            copy3(vp(&mut top), r);
            let mut dir: [f32; 3] = [0.0; 3];
            for (j, v) in dir.iter_mut().enumerate() {
                *v = f32_of(x(top[j]).fsub_m(rd::<f32>(tp, 4 * j as isize)));
            }
            vector_normalise(vp(&mut dir));
            for (j, v) in dir.iter_mut().enumerate() {
                *v = f32_of(x(*v).fadd_m(rd::<f32>(tp, 4 * j as isize)));
            }
            if get_is_line_of_sight_clear(vp(&mut top), vp(&mut dir), 1, 0, 0, 0, 0, 1, 1) == 0 {
                break 'next;
            }
            let best = x(LAMP_POST_BEST_DIST.get()).fsub_m(dist);
            copy3(out, vp(&mut top));
            winner = slot;
            closest = f32_of(best.fabs());
        }
        i = i.wrapping_add(1);
        if i >= count[0] {
            break;
        }
    }
    u8::from(winner != 0)
}

orig! {
    /// `CCam::GetCoreDataForDWCineyCamMode(...)` (0x517130)
    pub fn get_core_data_call(cam: GPtr<u8>, e: GPtr<u8>, v: GPtr<u8>, dest: GPtr<u8>, src: GPtr<u8>, up: GPtr<u8>, right: GPtr<u8>, fwd: GPtr<u8>, vel: GPtr<u8>, speed: GPtr<u8>, ang_vel: GPtr<u8>, ang_speed: GPtr<u8>, sphere: GPtr<u8>) = thiscall 0x517130 => get_core_data_for_dw_ciney_cam_mode;
    /// `CCam::IsTimeToExitThisDWCineyCamMode(...)` (0x517400, AL)
    pub fn is_time_to_exit_call(cam: GPtr<u8>, mode: i32, src: GPtr<u8>, dst: GPtr<u8>, t: u32, los: u32) -> u8 = thiscall 0x517400 => is_time_to_exit_this_dw_ciney_cam_mode;
    /// `CCam::Finalise_DW_CineyCams(...)` (0x50DD70)
    pub fn finalise_call(cam: GPtr<u8>, src: GPtr<u8>, dest: GPtr<u8>, roll: u32, fov: u32, near: u32, shake: u32) = thiscall 0x50DD70 => finalise_dw_ciney_cams;
    /// `CCamera::GetRoughDistanceToGround()` (0x516B00, ST0)
    pub fn get_rough_distance_to_ground(cam: GPtr<u8>) -> f64 = thiscall 0x516B00 => unported;
}

/// `TheCamera` +0x31 (cleared by every DW processor on entry).
const CAMERA_B6F059: usize = 0xB6F059;
/// `gDWLastModeForCineyCam`, `gDWCamMaxDurationMs[mode - 0x14]` (int32 x9), the scene start time.
const DW_LAST_MODE: usize = 0x8CC488;
const DW_DURATIONS: u32 = 0x8CCBA8;
const DW_SCENE_START_TIME: usize = 0x8CCBA0;
/// 0.5 (0x858B8C), -1.0 (0x858C1C), 360.0 (0x859E2C), pi/180 (0x8595EC); the Finalise arguments 70.0,
/// 5.0 and 1.0.
const K_HALF: u32 = 0x3F00_0000;
const K_M1: u32 = 0xBF80_0000;
const K_360: u32 = 0x43B4_0000;
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_FOV70: u32 = 0x428C_0000;
const K_NEAR5: u32 = 0x40A0_0000;
const K_ONE: u32 = 0x3F80_0000;

/// The DW processors' frame: their locals and the out-parameters of `GetCoreDataForDWCineyCamMode`
/// live at fixed offsets of one stack frame and are reused for other values; the ports keep the same
/// offsets (`B` = ESP after the prologue's pushes).
struct Frame([u32; 0x50]);

impl Frame {
    fn new() -> Frame {
        Frame([0; 0x50])
    }
    fn b(&mut self) -> GPtr<u8> {
        GPtr::<u32>::from_raw(self.0.as_mut_ptr()).cast::<u8>()
    }
}

/// `fld dword [B + off]` operand / store helpers.
fn m(b: GPtr<u8>, off: isize) -> f32 {
    rd::<f32>(b, off)
}
fn st(b: GPtr<u8>, off: isize, v: f32) {
    wr::<f32>(b, off, v)
}
fn mu(b: GPtr<u8>, off: isize) -> u32 {
    rd::<u32>(b, off)
}
fn stu(b: GPtr<u8>, off: isize, v: u32) {
    wr::<u32>(b, off, v)
}

/// `fsin` / `fcos` / `fidiv dword [m]` on the [`Fpu`] stack top.
fn fsin(f: &mut Fpu) {
    let v = f.pop();
    f.push(v.fsin());
}
fn fidiv(f: &mut Fpu, d: i32) {
    let v = f.pop();
    f.push(v.fdiv(X87::from_i32(d)));
}

/// `GetCoreDataForDWCineyCamMode` with its 12 out-parameters at these frame offsets (entity, vehicle,
/// dest, src, up, right, fwd, vel, speed, angVel, angSpeed, sphere).
fn core_data(this: GPtr<u8>, b: GPtr<u8>, o: [isize; 12]) {
    let p = |i: usize| b.byte_add(o[i]);
    get_core_data_call(
        this,
        p(0),
        p(1),
        p(2),
        p(3),
        p(4),
        p(5),
        p(6),
        p(7),
        p(8),
        p(9),
        p(10),
        p(11),
    );
}

/// The scene start every DW processor shares: `gDWLastModeForCineyCam = last_mode` (the processors use
/// their own ids, e.g. 0x3E / 0x40), the end time = now +
/// the mode's duration, `s_ExitCam[mode] = 0`, the start time = now; then a `ProcessLineOfSight(dest,
/// src, colPoint, hit, buildings, vehicles)` with the target ignored. Returns the LOS result.
#[allow(clippy::too_many_arguments)]
fn start_scene(
    last_mode: i32,
    mode: i32,
    now: u32,
    ent: GPtr<u8>,
    dest: GPtr<u8>,
    src: GPtr<u8>,
    cp: GPtr<u8>,
    hit: GPtr<u8>,
) -> u8 {
    let dur = g::<u32>(DW_DURATIONS.wrapping_add(((mode - 0x14) as u32) * 4) as usize).read();
    g::<i32>(DW_LAST_MODE).write(last_mode);
    DW_SCENE_END_TIME.set(dur.wrapping_add(now));
    g::<u8>(DW_EXIT_CAM.wrapping_add(mode as u32) as usize).write(0);
    g::<u32>(DW_SCENE_START_TIME).write(now);
    IGNORE_ENTITY.set(ent);
    let r = process_line_of_sight(dest, src, cp, hit, 1, 1, 0, 0, 0, 0, 0, 0);
    IGNORE_ENTITY.set(GPtr::NULL);
    r
}

/// The scene is already running: the same mode and processed on the previous frame
/// (`gLastFrameProcessedDWCineyCam >= frame - 1`, unsigned).
fn scene_running(mode: i32) -> bool {
    g::<i32>(DW_LAST_MODE).read() == mode
        && DW_LAST_FRAME_PROCESSED.get() >= FRAME_COUNTER.get().wrapping_sub(1)
}

/// The visibility hysteresis of the plane cams: a static counter (`guard` bit 1 marks it initialised
/// to `[max]`, only in a register on the first call) counts down while the LOS is blocked (exit when
/// it was 0) and up while clear (clamped to `[max]` when it was above). Returns false to exit.
fn los_counter(clear: bool, guard: usize, counter: usize, max_addr: usize) -> bool {
    let max = g::<i32>(max_addr).read();
    let c = if g::<u8>(guard).read() & 1 == 0 {
        g::<u32>(guard).write(g::<u32>(guard).read() | 1);
        max
    } else {
        g::<i32>(counter).read()
    };
    if !clear {
        g::<i32>(counter).write(c.wrapping_sub(1));
        c != 0
    } else {
        g::<i32>(counter).write(c.wrapping_add(1));
        if c > max {
            g::<i32>(counter).write(max);
        }
        true
    }
}

/// `elapsed / duration` of the scene: `fild (now - start); fidiv (end - start)`.
fn scene_t(f: &mut Fpu, now: u32) {
    let start = g::<u32>(DW_SCENE_START_TIME).read();
    f.fild(now.wrapping_sub(start) as i32);
    fidiv(f, DW_SCENE_END_TIME.get().wrapping_sub(start) as i32);
}

/// `CModelInfo::ms_modelInfoPtrs[e->m_nModelIndex]->m_pColModel`.
fn model_col_model(e: GPtr<u8>) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(
        MODEL_INFO_PTRS.wrapping_add((i32::from(rd::<i16>(e, 0x22)) as u32).wrapping_mul(4)) as usize,
    )
    .read();
    rd::<GPtr<u8>>(mi, 0x14)
}

/// `CCam::Process_DW_PlaneCam3(bool)` (0x51D100, thiscall, `ret 4`, AL; mode 0x1C): a camera ahead of
/// the plane (`dest + fwd * (bbox length)` + up * [0x8CCDE4]) looking back at it, while the target is
/// above [0x8CCDEC]. The scene starts (LOS from dest to the target's last DW position must be clear)
/// unless it runs already; the LOS hysteresis counter 0xB700D4; then `IsTimeToExit` and
/// `Finalise_DW_CineyCams(src, dest, 0, 70, 5, 1)`. Exits set `s_ExitCam[0x1C]` and return 0.
#[hook(addr = 0x51D100, conv = thiscall, name = "CCam::Process_DW_PlaneCam3", expect = "81 EC D4 00 00 00 53 55", this = "CCam")]
pub fn process_dw_plane_cam3(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC78;
    g::<u8>(CAMERA_B6F059).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
        return 0;
    }
    let mut fr = Frame::new();
    let b = fr.b();
    core_data(
        this,
        b,
        [
            0x10, 0x38, 0x2C, 0x14, 0x48, 0x6C, 0x20, 0x60, 0x58, 0x3C, 0x54, 0x78,
        ],
    );
    let exit = || {
        g::<u8>(EXIT).write(1);
        0
    };
    if !jp(x(m(b, 0x34)), x(g::<f32>(0x8CCDEC).read()), 0x05) {
        return exit();
    }
    let now = TIME_MS.get();
    let ent = rd::<GPtr<u8>>(b, 0x10);
    if !scene_running(0x40)
        && start_scene(
            0x40,
            0x1C,
            now,
            ent,
            b.byte_add(0x2C),
            b.byte_add(0x14),
            b.byte_add(0x8C),
            b.byte_add(0x38),
        ) != 0
    {
        return exit();
    }
    let mut f = Fpu::new();
    scene_t(&mut f, now);
    let cm = model_col_model(ent);
    st(b, 0x10, f.fstp());
    f.fld(rd::<f32>(cm, 0x10));
    f.fsub_m(rd::<f32>(cm, 4));
    IGNORE_ENTITY.set(ent);
    f.fmul_m(k(K_HALF));
    f.fadd_st(0);
    g::<f32>(0x8CCDE8).write(f.fstp());
    let len = || g::<f32>(0x8CCDE8).read();
    f.fld(m(b, 0x20));
    f.fmul_m(len());
    f.fld(m(b, 0x24));
    f.fmul_m(len());
    f.fld(m(b, 0x28));
    f.fmul_m(len());
    st(b, 0x44, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x2C));
    st(b, 0x20, f.fstp());
    f.fld(m(b, 0x30));
    stu(b, 0x14, mu(b, 0x20));
    f.fadd_st(1);
    st(b, 0x24, f.fstp());
    f.fstp_st(0);
    stu(b, 0x18, mu(b, 0x24));
    f.fld(m(b, 0x44));
    f.fadd_m(m(b, 0x34));
    st(b, 0x28, f.fstp());
    let h = || g::<f32>(0x8CCDE4).read();
    f.fld(m(b, 0x48));
    f.fmul_m(h());
    stu(b, 0x1C, mu(b, 0x28));
    f.fld(m(b, 0x4C));
    f.fmul_m(h());
    f.fld(m(b, 0x50));
    f.fmul_m(h());
    st(b, 0x44, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x20));
    st(b, 0x14, f.fstp());
    f.fld(m(b, 0x18));
    f.fadd_st(1);
    st(b, 0x18, f.fstp());
    f.fstp_st(0);
    f.fld(m(b, 0x1C));
    f.fadd_m(m(b, 0x44));
    st(b, 0x1C, f.fstp());
    let r = process_line_of_sight(
        b.byte_add(0x2C),
        b.byte_add(0x14),
        b.byte_add(0xB8),
        b.byte_add(0x5C),
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    IGNORE_ENTITY.set(GPtr::NULL);
    if !los_counter(r == 0, 0xB700D8, 0xB700D4, 0x8CCDE0) {
        return exit();
    }
    if is_time_to_exit_call(this, 0x1C, b.byte_add(0x14), b.byte_add(0x2C), mu(b, 0x10), 0) != 0 {
        return exit();
    }
    finalise_call(
        this,
        b.byte_add(0x14),
        b.byte_add(0x2C),
        0,
        K_FOV70,
        K_NEAR5,
        K_ONE,
    );
    1
}

/// `CCam::Process_DW_PlaneCam1(bool)` (0x51C760, thiscall, `ret 4`, AL; mode 0x1A): a camera beside the
/// plane: `dest + F * [0x8CCDB0] + R * [0x8CCDAC] + up * [0x8CCDA8] + up * [0x8CCDA4] * (t - 0.5) *
/// side + up * [0x8CCD9C] * sin(t * [0x8CCDA0] * 360 * pi/180)` (F, R the normalised forward / right,
/// R scaled by the side sign [0x8CCDB4]; the vertical side sign 0xB700C0 and [0x8CCDB4] are set from
/// two `rand()` (sites 0x51C8BB / 0x51C8D9) when the scene starts). Runs while the target is above
/// [0x8CCDBC] and, during a running scene, `GetRoughDistanceToGround() >= [0x8CCDB8]`. LOS hysteresis
/// 0xB700B8, `IsTimeToExit(0x1A)`, `Finalise(src, dest, 0, 70, 5, 1)`; exits set `s_ExitCam[0x1A]`.
#[hook(addr = 0x51C760, conv = thiscall, name = "CCam::Process_DW_PlaneCam1", expect = "81 EC EC 00 00 00 53 55", this = "CCam")]
pub fn process_dw_plane_cam1(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC76;
    const SIDE: usize = 0xB700C0;
    g::<u8>(CAMERA_B6F059).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
        return 0;
    }
    let mut fr = Frame::new();
    let b = fr.b();
    core_data(
        this,
        b,
        [
            0x10, 0x70, 0x54, 0x14, 0x48, 0x20, 0x2C, 0x84, 0x38, 0x78, 0x60, 0x90,
        ],
    );
    let exit = || {
        g::<u8>(EXIT).write(1);
        0
    };
    let below = !jp(x(m(b, 0x5C)), x(g::<f32>(0x8CCDBC).read()), 0x05);
    let now = TIME_MS.get();
    stu(b, 0x60, now);
    if below {
        return exit();
    }
    let ent = rd::<GPtr<u8>>(b, 0x10);
    if scene_running(0x3E) {
        let gd = X87::from_f64_exact(get_rough_distance_to_ground(GPtr::from_addr(0xB6F028)));
        if !jp(gd, x(g::<f32>(0x8CCDB8).read()), 0x05) {
            return exit();
        }
    } else {
        if start_scene(
            0x3E,
            0x1A,
            now,
            ent,
            b.byte_add(0x54),
            b.byte_add(0x14),
            b.byte_add(0xA4),
            b.byte_add(0x70),
        ) != 0
        {
            return exit();
        }
        g::<u32>(SIDE).write(K_ONE);
        if sa_core::orig::at_site(0x51C8BB, sa_core::crt::rand) < 0x3FFF {
            g::<f32>(SIDE).write(f32_of(x(g::<f32>(SIDE).read()).fmul_m(k(K_M1))));
        }
        if sa_core::orig::at_site(0x51C8D9, sa_core::crt::rand) < 0x3FFF {
            g::<u32>(0x8CCDB4).write(K_M1);
        }
    }
    let mut f = Fpu::new();
    scene_t(&mut f, now);
    for (dst, srco) in [
        (0x6C, 0x34),
        (0x64, 0x2C),
        (0x68, 0x30),
        (0x44, 0x28),
        (0x3C, 0x20),
        (0x40, 0x24),
    ] {
        stu(b, dst, mu(b, srco));
    }
    st(b, 0x10, f.fstp());
    vector_normalise(b.byte_add(0x64));
    vector_normalise(b.byte_add(0x3C));
    for o in [0x3C, 0x40, 0x44] {
        f.fld(m(b, o));
        f.fmul_m(g::<f32>(0x8CCDB4).read());
        st(b, o, f.fstp());
    }
    let kd = |a: usize| g::<f32>(a).read();
    f.fld(m(b, 0x64));
    f.fmul_m(kd(0x8CCDB0));
    f.fld(m(b, 0x68));
    f.fmul_m(kd(0x8CCDB0));
    f.fld(m(b, 0x6C));
    f.fmul_m(kd(0x8CCDB0));
    st(b, 0x28, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x54));
    st(b, 0x2C, f.fstp());
    f.fld(m(b, 0x58));
    stu(b, 0x14, mu(b, 0x2C));
    f.fadd_st(1);
    st(b, 0x30, f.fstp());
    f.fstp_st(0);
    stu(b, 0x18, mu(b, 0x30));
    f.fld(m(b, 0x28));
    f.fadd_m(m(b, 0x5C));
    st(b, 0x34, f.fstp());
    f.fld(m(b, 0x3C));
    f.fmul_m(kd(0x8CCDAC));
    stu(b, 0x1C, mu(b, 0x34));
    f.fld(m(b, 0x40));
    f.fmul_m(kd(0x8CCDAC));
    f.fld(m(b, 0x44));
    f.fmul_m(kd(0x8CCDAC));
    st(b, 0x28, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x2C));
    st(b, 0x14, f.fstp());
    f.fadd_m(m(b, 0x18));
    f.fld(m(b, 0x1C));
    f.fadd_m(m(b, 0x28));
    f.fld(m(b, 0x48));
    f.fmul_m(kd(0x8CCDA8));
    f.fld(m(b, 0x4C));
    f.fmul_m(kd(0x8CCDA8));
    f.fld(m(b, 0x50));
    f.fmul_m(kd(0x8CCDA8));
    st(b, 0x28, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x14));
    st(b, 0x14, f.fstp());
    f.fadd_st(2);
    st(b, 0x18, f.fstp());
    f.fld(m(b, 0x28));
    f.fadd_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    f.fld(m(b, 0x48));
    f.fmul_m(kd(0x8CCDA4));
    f.fld(m(b, 0x4C));
    f.fmul_m(kd(0x8CCDA4));
    f.fld(m(b, 0x50));
    f.fmul_m(kd(0x8CCDA4));
    st(b, 0x28, f.fstp());
    f.fld(m(b, 0x10));
    f.fsub_m(k(K_HALF));
    st(b, 0x38, f.fstp());
    f.fxch(1);
    f.fmul_m(m(b, 0x38));
    st(b, 0x2C, f.fstp());
    f.fmul_m(m(b, 0x38));
    f.fld(m(b, 0x28));
    f.fmul_m(m(b, 0x38));
    f.fld(m(b, 0x2C));
    f.fmul_m(g::<f32>(SIDE).read());
    st(b, 0x20, f.fstp());
    f.fxch(1);
    f.fmul_m(g::<f32>(SIDE).read());
    st(b, 0x24, f.fstp());
    f.fmul_m(g::<f32>(SIDE).read());
    f.fld(m(b, 0x20));
    f.fadd_m(m(b, 0x14));
    st(b, 0x14, f.fstp());
    f.fld(m(b, 0x24));
    f.fadd_m(m(b, 0x18));
    st(b, 0x18, f.fstp());
    f.faddp(1);
    f.fild(g::<i32>(DW_SCENE_START_TIME).read());
    f.fild(mu(b, 0x60) as i32);
    f.fsub_st(1);
    f.fild(DW_SCENE_END_TIME.get() as i32);
    f.fsub_st(2);
    f.fdivp(1);
    f.fild(g::<i32>(0x8CCDA0).read());
    f.fmulp(1);
    f.fmul_m(k(K_360));
    f.fmul_m(k(K_DEG2RAD));
    fsin(&mut f);
    f.fstp_st(1);
    f.fld(m(b, 0x48));
    f.fmul_m(kd(0x8CCD9C));
    f.fld(m(b, 0x4C));
    f.fmul_m(kd(0x8CCD9C));
    f.fld(m(b, 0x50));
    f.fmul_m(kd(0x8CCD9C));
    st(b, 0x34, f.fstp());
    f.fxch(1);
    f.fmul_st(2);
    st(b, 0x20, f.fstp());
    f.fmul_st(1);
    f.fld(m(b, 0x34));
    f.fmul_st(2);
    st(b, 0x28, f.fstp());
    f.fld(m(b, 0x20));
    f.fadd_m(m(b, 0x14));
    st(b, 0x14, f.fstp());
    f.fadd_m(m(b, 0x18));
    st(b, 0x18, f.fstp());
    IGNORE_ENTITY.set(ent);
    f.fstp_st(0);
    f.fld(m(b, 0x28));
    f.fadd_st(1);
    st(b, 0x1C, f.fstp());
    f.fstp_st(0);
    let r = process_line_of_sight(
        b.byte_add(0x54),
        b.byte_add(0x14),
        b.byte_add(0xD0),
        b.byte_add(0x74),
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    IGNORE_ENTITY.set(GPtr::NULL);
    if !los_counter(r == 0, 0xB700BC, 0xB700B8, 0x8CCD98) {
        return exit();
    }
    if is_time_to_exit_call(this, 0x1A, b.byte_add(0x14), b.byte_add(0x54), mu(b, 0x10), 0) != 0 {
        return exit();
    }
    finalise_call(
        this,
        b.byte_add(0x14),
        b.byte_add(0x54),
        0,
        K_FOV70,
        K_NEAR5,
        K_ONE,
    );
    1
}

/// `addr = 1.0`, then `addr = f32(addr * -1.0)` when the `rand()` returning to `site` is below 0x3FFF.
fn random_sign(site: u32, addr: usize) {
    g::<u32>(addr).write(K_ONE);
    if sa_core::orig::at_site(site, sa_core::crt::rand) < 0x3FFF {
        g::<f32>(addr).write(f32_of(x(g::<f32>(addr).read()).fmul_m(k(K_M1))));
    }
}

/// The sine sway of the plane cams: `sin(((now - start) / (end - start)) * [n] * 360 * pi/180)` with the
/// stack layout of the bytes (start pushed first, replaced by the result: `fstp st(1)`); `now` is the
/// frame copy of the time.
fn sway(f: &mut Fpu, now: u32, n_addr: usize) {
    f.fild(g::<i32>(DW_SCENE_START_TIME).read());
    f.fild(now as i32);
    f.fsub_st(1);
    f.fild(DW_SCENE_END_TIME.get() as i32);
    f.fsub_st(2);
    f.fdivp(1);
    f.fild(g::<i32>(n_addr).read());
    f.fmulp(1);
    f.fmul_m(k(K_360));
    f.fmul_m(k(K_DEG2RAD));
    fsin(f);
    f.fstp_st(1);
}

/// `CCam::Process_DW_PlaneCam2(bool)` (0x51CC30, thiscall, `ret 4`, AL; the entry fragment 0x40801D
/// does `sub esp, 0xEC`; mode 0x1B, scene id 0x3F): a camera passing the plane: with F / R the
/// normalised forward / right, `R *= (1 - t) * [0x8CCDD8]`, `F *= (1 - 2t) * 0xB700CC` (signs from three
/// `rand()` at scene start, sites 0x51CD63 / 0x51CD8B / 0x51CDA9), `src = dest + F * [0x8CCDD4] + R *
/// [0x8CCDD0] + up * [0x8CCDCC] + up * [0x8CCDC4] * sin(t * [0x8CCDC8] * 2 pi)`. Runs while the target
/// is above [0x8CCDDC]; LOS hysteresis 0xB700C4; `IsTimeToExit(0x1B)`; `Finalise(src, dest, 0, 70, 5,
/// 1)`; exits set `s_ExitCam[0x1B]`.
#[hook(addr = 0x51CC30, conv = thiscall, name = "CCam::Process_DW_PlaneCam2", expect = "90 E9 E7 B3 EE FF", this = "CCam")]
pub fn process_dw_plane_cam2(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC77;
    const S1: usize = 0xB700CC;
    const S2: usize = 0x8CCDD8;
    g::<u8>(CAMERA_B6F059).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
        return 0;
    }
    let mut fr = Frame::new();
    let b = fr.b();
    core_data(
        this,
        b,
        [
            0x10, 0x6C, 0x50, 0x14, 0x60, 0x38, 0x44, 0x78, 0x70, 0x84, 0x5C, 0x90,
        ],
    );
    let exit = || {
        g::<u8>(EXIT).write(1);
        0
    };
    let below = !jp(x(m(b, 0x58)), x(g::<f32>(0x8CCDDC).read()), 0x05);
    let now = TIME_MS.get();
    stu(b, 0x5C, now);
    if below {
        return exit();
    }
    let ent = rd::<GPtr<u8>>(b, 0x10);
    if !scene_running(0x3F) {
        if start_scene(
            0x3F,
            0x1B,
            now,
            ent,
            b.byte_add(0x50),
            b.byte_add(0x14),
            b.byte_add(0xA4),
            b.byte_add(0x6C),
        ) != 0
        {
            return exit();
        }
        random_sign(0x51CD63, 0xB700D0);
        random_sign(0x51CD8B, S1);
        if sa_core::orig::at_site(0x51CDA9, sa_core::crt::rand) < 0x3FFF {
            g::<u32>(S2).write(K_M1);
        }
    }
    let kd = |a: usize| g::<f32>(a).read();
    let mut f = Fpu::new();
    scene_t(&mut f, now);
    for (dst, srco) in [
        (0x28, 0x4C),
        (0x20, 0x44),
        (0x24, 0x48),
        (0x34, 0x40),
        (0x2C, 0x38),
        (0x30, 0x3C),
    ] {
        stu(b, dst, mu(b, srco));
    }
    st(b, 0x10, f.fstp());
    vector_normalise(b.byte_add(0x20));
    vector_normalise(b.byte_add(0x2C));
    f.fld(k(K_ONE));
    f.fsub_m(m(b, 0x10));
    f.fld(m(b, 0x2C));
    f.fmul_st(1);
    f.fld(m(b, 0x30));
    f.fmul_st(2);
    st(b, 0x30, f.fstp());
    f.fld(m(b, 0x34));
    f.fmul_st(2);
    st(b, 0x34, f.fstp());
    f.fld(m(b, 0x10));
    f.fadd_st(0);
    f.fsubr_m(k(K_ONE));
    f.fld(m(b, 0x20));
    f.fmul_st(1);
    f.fld(m(b, 0x24));
    f.fmul_st(2);
    st(b, 0x24, f.fstp());
    f.fld(m(b, 0x28));
    f.fmul_st(2);
    st(b, 0x28, f.fstp());
    f.fmul_m(kd(S1));
    st(b, 0x20, f.fstp());
    f.fstp_st(0);
    for o in [0x24, 0x28] {
        f.fld(m(b, o));
        f.fmul_m(kd(S1));
        st(b, o, f.fstp());
    }
    f.fmul_m(kd(S2));
    st(b, 0x2C, f.fstp());
    f.fstp_st(0);
    for o in [0x30, 0x34] {
        f.fld(m(b, o));
        f.fmul_m(kd(S2));
        st(b, o, f.fstp());
    }
    f.fld(m(b, 0x20));
    f.fmul_m(kd(0x8CCDD4));
    f.fld(m(b, 0x24));
    f.fmul_m(kd(0x8CCDD4));
    f.fld(m(b, 0x28));
    f.fmul_m(kd(0x8CCDD4));
    st(b, 0x40, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x50));
    st(b, 0x44, f.fstp());
    f.fld(m(b, 0x54));
    f.fadd_st(1);
    st(b, 0x48, f.fstp());
    f.fstp_st(0);
    f.fld(m(b, 0x40));
    f.fadd_m(m(b, 0x58));
    stu(b, 0x14, mu(b, 0x44));
    stu(b, 0x18, mu(b, 0x48));
    st(b, 0x4C, f.fstp());
    f.fld(m(b, 0x2C));
    f.fmul_m(kd(0x8CCDD0));
    stu(b, 0x1C, mu(b, 0x4C));
    f.fld(m(b, 0x30));
    f.fmul_m(kd(0x8CCDD0));
    f.fld(m(b, 0x34));
    f.fmul_m(kd(0x8CCDD0));
    st(b, 0x40, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x44));
    st(b, 0x14, f.fstp());
    f.fadd_m(m(b, 0x18));
    f.fld(m(b, 0x1C));
    f.fadd_m(m(b, 0x40));
    f.fld(m(b, 0x60));
    f.fmul_m(kd(0x8CCDCC));
    f.fld(m(b, 0x64));
    f.fmul_m(kd(0x8CCDCC));
    f.fld(m(b, 0x68));
    f.fmul_m(kd(0x8CCDCC));
    st(b, 0x40, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x14));
    st(b, 0x14, f.fstp());
    f.fadd_st(2);
    st(b, 0x18, f.fstp());
    f.fld(m(b, 0x40));
    f.fadd_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    sway(&mut f, mu(b, 0x5C), 0x8CCDC8);
    f.fld(m(b, 0x60));
    f.fmul_m(kd(0x8CCDC4));
    f.fld(m(b, 0x64));
    f.fmul_m(kd(0x8CCDC4));
    f.fld(m(b, 0x68));
    f.fmul_m(kd(0x8CCDC4));
    st(b, 0x68, f.fstp());
    f.fxch(1);
    f.fmul_st(2);
    st(b, 0x38, f.fstp());
    f.fmul_st(1);
    f.fld(m(b, 0x68));
    f.fmul_st(2);
    st(b, 0x40, f.fstp());
    f.fld(m(b, 0x38));
    f.fadd_m(m(b, 0x14));
    st(b, 0x14, f.fstp());
    f.fadd_m(m(b, 0x18));
    st(b, 0x18, f.fstp());
    f.fstp_st(0);
    f.fld(m(b, 0x40));
    f.fadd_st(1);
    IGNORE_ENTITY.set(ent);
    st(b, 0x1C, f.fstp());
    f.fstp_st(0);
    let r = process_line_of_sight(
        b.byte_add(0x50),
        b.byte_add(0x14),
        b.byte_add(0xD0),
        b.byte_add(0x74),
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    IGNORE_ENTITY.set(GPtr::NULL);
    if !los_counter(r == 0, 0xB700C8, 0xB700C4, 0x8CCDC0) {
        return exit();
    }
    if is_time_to_exit_call(this, 0x1B, b.byte_add(0x14), b.byte_add(0x50), mu(b, 0x10), 0) != 0 {
        return exit();
    }
    finalise_call(
        this,
        b.byte_add(0x14),
        b.byte_add(0x50),
        0,
        K_FOV70,
        K_NEAR5,
        K_ONE,
    );
    1
}

/// The ease curve on the stack top: clamp to [0, 1] (`fcom 0.0; test ah,5; jp` / `fcom 1.0; test
/// ah,0x41; jne`), then `(sin((270 - v * 180) * pi/180) + 1) * 0.5`.
fn ease(f: &mut Fpu) {
    f.fcom_m(k(K_ZERO));
    if !f.jp(0x05) {
        f.fstp_st(0);
        f.fld(k(K_ZERO));
    } else {
        f.fcom_m(k(K_ONE));
        if !f.jne(0x41) {
            f.fstp_st(0);
            f.fld(k(K_ONE));
        }
    }
    f.fmul_m(k(0x4334_0000));
    f.fsubr_m(k(0x4387_0000));
    f.fmul_m(k(K_DEG2RAD));
    fsin(f);
    f.fadd_m(k(K_ONE));
    f.fmul_m(k(K_HALF));
}

/// `CCam::Process_DW_PlaneSpotterCam(bool)` (0x51C250, thiscall, `ret 4`, AL; entry fragment 0x401DC0:
/// `sub esp, 0xDC`; mode 0x17, scene id 0x3B). At scene start it tries up to [0x8CCD94] random spots
/// around the target (`dest.xy + [0x8CCD8C] * (0.5 + u * 0.5)` per axis with `u = rand() / 32767`, sites
/// 0x51C394 / 0x51C3D9; z = dest.z - [0x8CCD90]) until the line from the target down to the spot hits
/// something: the hit point + 2.0 up becomes the spotter position (0xB700A8); no hit at all exits; a
/// third `rand()` (site 0x51C485) picks zooming (0xB700A4). The spotter must stay 5.0 away (2D); the
/// zoom FOV eases from [0x8CCD80] towards [0x8CCD84] with the distance / [0x8CCD88], blended in over
/// the first [0x8CCD7C] of the scene from the FOV latched in 0xB700A0. LOS hysteresis 0xB7009C;
/// `IsTimeToExit(0x17)`; `Finalise(spot, dest, 0, fov, 10 - fov / 70 * 9.7, 1)`.
#[hook(addr = 0x51C250, conv = thiscall, name = "CCam::Process_DW_PlaneSpotterCam", expect = "90 E9 6A 5B EE FF", this = "CCam")]
pub fn process_dw_plane_spotter_cam(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC73;
    const GUARD: usize = 0xB700B4;
    const SPOT: usize = 0xB700A8;
    const ZOOM: usize = 0xB700A4;
    const FOV_LATCH: usize = 0xB700A0;
    g::<u8>(CAMERA_B6F059).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
        return 0;
    }
    let mut fr = Frame::new();
    let b = fr.b();
    core_data(
        this,
        b,
        [
            0x10, 0x14, 0x28, 0x1C, 0x7C, 0xA0, 0x88, 0x70, 0x34, 0x94, 0x18, 0xAC,
        ],
    );
    let exit = || {
        g::<u8>(EXIT).write(1);
        0
    };
    if g::<u8>(GUARD).read() & 1 == 0 {
        g::<u32>(GUARD).write(g::<u32>(GUARD).read() | 1);
    }
    let now = TIME_MS.get();
    let ent = rd::<GPtr<u8>>(b, 0x10);
    let kd = |a: usize| g::<f32>(a).read();
    let mut f = Fpu::new();
    if !scene_running(0x3B) {
        let dur = g::<u32>(0x8CCBB4).read();
        g::<i32>(DW_LAST_MODE).write(0x3B);
        let tries = g::<i32>(0x8CCD94).read();
        g::<u8>(EXIT).write(0);
        g::<u32>(DW_SCENE_START_TIME).write(now);
        DW_SCENE_END_TIME.set(dur.wrapping_add(now));
        if tries > 0 {
            let mut i = 0;
            let clear = loop {
                f.fld(m(b, 0x30));
                f.fsub_m(kd(0x8CCD90));
                stu(b, 0x24, mu(b, 0x30));
                st(b, 0x24, f.fstp());
                stu(b, 0x1C, mu(b, 0x28));
                f.fld(kd(0x8CCD8C));
                f.fmul_m(k(K_HALF));
                stu(b, 0x20, mu(b, 0x2C));
                stu(b, 0x34, g::<u32>(0x8CCD8C).read());
                st(b, 0x10, f.fstp());
                stu(
                    b,
                    0x14,
                    sa_core::orig::at_site(0x51C394, sa_core::crt::rand) as u32,
                );
                f.fild(mu(b, 0x14) as i32);
                stu(b, 0x18, g::<u32>(0x8CCD8C).read());
                f.fmul_m(k(0x3800_0100));
                f.fld(m(b, 0x34));
                f.fsub_m(m(b, 0x10));
                f.fmul_st(1);
                f.fadd_m(m(b, 0x10));
                f.fadd_m(m(b, 0x1C));
                st(b, 0x1C, f.fstp());
                f.fstp_st(0);
                f.fld(kd(0x8CCD8C));
                f.fmul_m(k(K_HALF));
                st(b, 0x10, f.fstp());
                stu(
                    b,
                    0x14,
                    sa_core::orig::at_site(0x51C3D9, sa_core::crt::rand) as u32,
                );
                f.fild(mu(b, 0x14) as i32);
                f.fmul_m(k(0x3800_0100));
                f.fld(m(b, 0x18));
                f.fsub_m(m(b, 0x10));
                f.fmul_st(1);
                f.fadd_m(m(b, 0x10));
                f.fadd_m(m(b, 0x20));
                st(b, 0x20, f.fstp());
                IGNORE_ENTITY.set(ent);
                f.fstp_st(0);
                let r = process_line_of_sight(
                    b.byte_add(0x28),
                    b.byte_add(0x1C),
                    b.byte_add(0x44),
                    b.byte_add(0x40),
                    1,
                    1,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                );
                IGNORE_ENTITY.set(GPtr::NULL);
                if r != 0 {
                    g::<u32>(SPOT).write(mu(b, 0x44));
                    g::<u32>(SPOT + 8).write(mu(b, 0x4C));
                    f.fld(m(b, 0x4C));
                    f.fadd_m(k(0x4000_0000));
                    g::<f32>(SPOT + 8).write(f.fstp());
                    g::<u32>(SPOT + 4).write(mu(b, 0x48));
                    break false;
                }
                i += 1;
                if i >= g::<i32>(0x8CCD94).read() {
                    break true;
                }
            };
            if clear {
                return exit();
            }
        }
        let z = sa_core::orig::at_site(0x51C485, sa_core::crt::rand) < 0x3FFF;
        g::<u8>(ZOOM).write(u8::from(z));
    }
    scene_t(&mut f, now);
    for (i, o) in [0x1C, 0x20, 0x24].into_iter().enumerate() {
        stu(b, o, g::<u32>(SPOT + 4 * i).read());
    }
    stu(b, 0x14, K_FOV70);
    st(b, 0x10, f.fstp());
    f.fld(kd(SPOT));
    f.fsub_m(m(b, 0x28));
    f.fld(kd(SPOT + 4));
    f.fsub_m(m(b, 0x2C));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(2);
    f.fstp_st(0);
    f.fcomp_m(k(K_5));
    if !f.jp(0x05) {
        return exit();
    }
    if g::<u8>(ZOOM).read() != 0 {
        f.fld(m(b, 0x28));
        f.fsub_m(kd(SPOT));
        f.fld(m(b, 0x2C));
        f.fsub_m(kd(SPOT + 4));
        f.fld(m(b, 0x30));
        f.fsub_m(kd(SPOT + 8));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fsqrt();
        f.fstp_st(3);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fdiv_m(kd(0x8CCD88));
        ease(&mut f);
        f.fld(kd(0x8CCD84));
        f.fsub_m(kd(0x8CCD80));
        f.fmulp(1);
        f.fadd_m(kd(0x8CCD80));
        st(b, 0x14, f.fstp());
        f.fld(m(b, 0x10));
        f.fcomp_m(kd(0x8CCD7C));
        if !f.jp(0x05) {
            if g::<u8>(GUARD).read() & 2 == 0 {
                g::<u32>(GUARD).write(g::<u32>(GUARD).read() | 2);
                g::<u32>(FOV_LATCH).write(g::<u32>(0x8CCD80).read());
            }
            f.fld(m(b, 0x10));
            f.fdiv_m(kd(0x8CCD7C));
            ease(&mut f);
            f.fld(m(b, 0x14));
            f.fsub_m(kd(FOV_LATCH));
            f.fmulp(1);
            f.fadd_m(kd(FOV_LATCH));
            st(b, 0x14, f.fstp());
        }
    }
    IGNORE_ENTITY.set(ent);
    let r = process_line_of_sight(
        b.byte_add(0x28),
        b.byte_add(0x1C),
        b.byte_add(0xC0),
        b.byte_add(0x3C),
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    IGNORE_ENTITY.set(GPtr::NULL);
    // the counter's initialised bit is 4 in the shared guard word
    let max = g::<i32>(0x8CCD78).read();
    let c = if g::<u8>(GUARD).read() & 4 == 0 {
        g::<u32>(GUARD).write(g::<u32>(GUARD).read() | 4);
        max
    } else {
        g::<i32>(0xB7009C).read()
    };
    if r != 0 {
        g::<i32>(0xB7009C).write(c.wrapping_sub(1));
        if c == 0 {
            return exit();
        }
    } else {
        g::<i32>(0xB7009C).write(c.wrapping_add(1));
        if c > max {
            g::<i32>(0xB7009C).write(max);
        }
    }
    if is_time_to_exit_call(this, 0x17, b.byte_add(0x1C), b.byte_add(0x28), mu(b, 0x10), 0) != 0 {
        return exit();
    }
    f.fld(m(b, 0x14));
    f.fmul_m(k(0x3C6A_0EA1));
    f.fmul_m(k(0x411B_3333));
    f.fsubr_m(k(0x4120_0000));
    let near = f.fstp();
    finalise_call(
        this,
        b.byte_add(0x1C),
        b.byte_add(0x28),
        0,
        mu(b, 0x14),
        near.to_bits(),
        K_ONE,
    );
    1
}

orig! {
    /// `CHandShaker::Reset()` (0x50D860)
    pub fn hand_shaker_reset(hs: GPtr<u8>) = thiscall 0x50D860 => crate::handshaker::hand_shaker_reset;
    /// `CPlaceable::GetMatrix()` (0x411990)
    pub fn get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `a > b ? a : b` (0x420800, cdecl floats as dwords, ST0)
    pub fn max_f(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
}

/// The 3D distance `|a - b|` of two frame vectors as the bytes compute it (`dx*dx` first with
/// `fmulp st(3)`, then `+ dz*dz`, then `+ dy*dy`), left on the stack.
fn dist3_xzy(f: &mut Fpu, b: GPtr<u8>, a: isize, c: isize) {
    f.fld(m(b, a));
    f.fsub_m(m(b, c));
    f.fld(m(b, a + 4));
    f.fsub_m(m(b, c + 4));
    f.fld(m(b, a + 8));
    f.fsub_m(m(b, c + 8));
    f.fld_st(2);
    f.fmulp(3);
    f.fld_st(0);
    f.fmul_st(1);
    f.faddp(3);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(3);
    f.fxch(2);
    f.fsqrt();
    f.fstp_st(2);
    f.fstp_st(0);
}

/// `CCam::Process_DW_CamManCam(bool)` (0x51B120, thiscall, `ret 4`, AL; entry fragment 0x401279:
/// `sub esp, 0xFC`; mode 0x15, scene id 0x39): a "camera man" on a lamp post. At scene start (also
/// `CHandShaker::Reset`) it searches up to 15 objects within [0x8CCD54] of a point [0x8CCD54] ahead of
/// the vehicle for a static lamp post (up.z > 0.9) more than [0x8CCD50] away (2D) and nearer than the
/// best so far, with `top = matrix * bbox.max` lowered to `top.z - max.z + min.z / 2` and a clear line
/// from it towards the target; none exits. Then the source is `post + normalise(dest - post) *
/// [0x8CCD48]` (unless `s_ExitCam[0x15]` is set), the FOV eases between [0x8CCD3C] and [0x8CCD40] with
/// the distance / [0x8CCD44] (blended in from [0x8CCD34] over the first [0x8CCD38]), `IsTimeToExit(0x15)`,
/// the LOS hysteresis 0xB70074, and the shake `clamp(max(|vel| * 8, 0.2), 0, 1)` for
/// `Finalise(src, dest, 0, fov, 10 - fov / 70 * 9.7, shake)`.
#[hook(addr = 0x51B120, conv = thiscall, name = "CCam::Process_DW_CamManCam", expect = "90 E9 53 61 EE FF", this = "CCam")]
pub fn process_dw_cam_man_cam(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC71;
    const GUARD: usize = 0xB70078;
    const COUNTER: usize = 0xB70074;
    const POST: usize = 0xB70068;
    g::<u8>(CAMERA_B6F059).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
        return 0;
    }
    let mut fr = Frame::new();
    let b = fr.b();
    core_data(
        this,
        b,
        [
            0x78, 0x14, 0x18, 0x24, 0x3C, 0x30, 0x50, 0x60, 0x4C, 0x6C, 0x5C, 0x8C,
        ],
    );
    let exit = || {
        g::<u8>(EXIT).write(1);
        0
    };
    let kd = |a: usize| g::<f32>(a).read();
    let guard = g::<u32>(GUARD).read();
    let now = TIME_MS.get();
    stu(b, 0x5C, now);
    let mut gw = guard;
    if gw & 1 == 0 {
        gw |= 1;
        g::<u32>(GUARD).write(gw);
    }
    let max = g::<u32>(0x8CCD58).read();
    if gw & 2 == 0 {
        g::<u32>(GUARD).write(gw | 2);
        g::<u32>(COUNTER).write(max);
    }
    let mut f = Fpu::new();
    if !scene_running(0x39) {
        g::<i32>(DW_LAST_MODE).write(0x39);
        let end = g::<u32>(0x8CCBAC).read().wrapping_add(now);
        g::<u32>(COUNTER).write(max);
        g::<u8>(EXIT).write(0);
        g::<u32>(DW_SCENE_START_TIME).write(now);
        DW_SCENE_END_TIME.set(end);
        hand_shaker_reset(GPtr::from_addr(HAND_SHAKER));
        let r = g::<u32>(0x8CCD54).read();
        f.fld(m(b, 0x50));
        f.fmul_m(kd(0x8CCD54));
        f.fld(m(b, 0x54));
        f.fmul_m(kd(0x8CCD54));
        f.fld(m(b, 0x58));
        f.fmul_m(kd(0x8CCD54));
        st(b, 0x58, f.fstp());
        f.fxch(1);
        f.fadd_m(m(b, 0x18));
        st(b, 0x6C, f.fstp());
        f.fld(m(b, 0x1C));
        f.fadd_st(1);
        st(b, 0x70, f.fstp());
        f.fstp_st(0);
        f.fld(m(b, 0x20));
        f.fadd_m(m(b, 0x58));
        st(b, 0x74, f.fstp());
        find_objects_in_range(
            b.byte_add(0x6C),
            r,
            1,
            b.byte_add(0x10),
            0xF,
            b.byte_add(0xA0),
            0,
            0,
            0,
            1,
            1,
        );
        let mut winner: u32 = 0;
        stu(b, 0x4C, K_10000);
        let count = rd::<i16>(b, 0x10);
        if count <= 0 {
            return exit();
        }
        let mut i: i16 = 0;
        loop {
            let slot = b.byte_add(0xA0 + 4 * isize::from(i));
            let e = rd::<GPtr<u8>>(slot, 0);
            'next: {
                let fl = rd::<u32>(e, 0x1C);
                if fl & 4 == 0 && fl & 0x40000 == 0 {
                    break 'next;
                }
                if rd::<GPtr<u8>>(e, 0x14).is_null() {
                    allocate_matrix(e);
                    simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
                }
                if jne(x(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 0x28)), x(k(K_0_9)), 0x41) {
                    break 'next;
                }
                let e = rd::<GPtr<u8>>(slot, 0);
                if is_lamp_post(i32::from(rd::<i16>(e, 0x22))) == 0 {
                    break 'next;
                }
                let mm = rd::<GPtr<u8>>(e, 0x14);
                let p = if mm.is_null() {
                    e.byte_add(4)
                } else {
                    mm.byte_add(0x30)
                };
                f.fld(rd::<f32>(p, 0));
                f.fsub_m(m(b, 0x18));
                f.fld(rd::<f32>(p, 4));
                f.fsub_m(m(b, 0x1C));
                f.fld_st(0);
                f.fmul_st(1);
                f.fld_st(2);
                f.fmul_st(3);
                f.faddp(1);
                f.fsqrt();
                f.fstp_st(2);
                f.fstp_st(0);
                st(b, 0x14, f.fst());
                f.fcomp_m(m(b, 0x4C));
                if f.jp(0x05) {
                    break 'next;
                }
                f.fld(m(b, 0x14));
                f.fcomp_m(kd(0x8CCD50));
                if f.jne(0x41) {
                    break 'next;
                }
                let cm = get_col_model(e);
                stu(b, 0x3C, rd::<u32>(cm, 0xC));
                stu(b, 0x40, rd::<u32>(cm, 0x10));
                stu(b, 0x44, rd::<u32>(cm, 0x14));
                let mx = get_matrix(rd::<GPtr<u8>>(slot, 0));
                let o = matrix_mul_vec(b.byte_add(0x80), mx, b.byte_add(0x3C));
                let (ox, oy, oz) = (rd::<u32>(o, 0), rd::<u32>(o, 4), rd::<u32>(o, 8));
                stu(b, 0x3C, ox);
                stu(b, 0x40, oy);
                stu(b, 0x44, oz);
                let cm = get_col_model(rd::<GPtr<u8>>(slot, 0));
                f.fld(m(b, 0x44));
                f.fsub_m(rd::<f32>(cm, 0x14));
                st(b, 0x44, f.fstp());
                let cm = get_col_model(rd::<GPtr<u8>>(slot, 0));
                f.fld(rd::<f32>(cm, 8));
                f.fmul_m(k(K_HALF));
                f.fadd_m(m(b, 0x44));
                st(b, 0x44, f.fstp());
                for (d, s, t2) in [(0x50, 0x3C, 0x18), (0x54, 0x40, 0x1C), (0x58, 0x44, 0x20)] {
                    f.fld(m(b, s));
                    f.fsub_m(m(b, t2));
                    st(b, d, f.fstp());
                    stu(b, d - 0x20, mu(b, d));
                }
                vector_normalise(b.byte_add(0x30));
                f.fld(m(b, 0x30));
                f.fadd_m(m(b, 0x18));
                st(b, 0x30, f.fstp());
                f.fld(m(b, 0x34));
                f.fadd_m(m(b, 0x1C));
                st(b, 0x34, f.fstp());
                f.fld(m(b, 0x20));
                f.fadd_m(m(b, 0x38));
                st(b, 0x38, f.fstp());
                // fld qword [0x859EF8] (0.0); fabs; fcomp [0x8CCD4C]
                f.push(X87::from_f64_exact(0.0));
                f.fabs();
                f.fcomp_m(kd(0x8CCD4C));
                if f.jp(0x05) {
                    break 'next;
                }
                if get_is_line_of_sight_clear(b.byte_add(0x3C), b.byte_add(0x30), 1, 0, 0, 0, 0, 1, 1) == 0 {
                    break 'next;
                }
                stu(b, 0x4C, mu(b, 0x14));
                winner = rd::<u32>(slot, 0);
                for (j, o) in [0x3C, 0x40, 0x44].into_iter().enumerate() {
                    stu(b, 0x24 + 4 * j as isize, mu(b, o));
                    g::<u32>(POST + 4 * j).write(mu(b, o));
                }
            }
            i = i.wrapping_add(1);
            if i >= rd::<i16>(b, 0x10) {
                break;
            }
        }
        if winner == 0 {
            return exit();
        }
    }
    let start = g::<u32>(DW_SCENE_START_TIME).read();
    f.fild(now.wrapping_sub(start) as i32);
    let skip_approach = g::<u8>(EXIT).read() != 0;
    fidiv(&mut f, DW_SCENE_END_TIME.get().wrapping_sub(start) as i32);
    st(b, 0x10, f.fstp());
    if !skip_approach {
        f.fld(m(b, 0x18));
        f.fsub_m(kd(POST));
        stu(b, 0x2C, g::<u32>(POST + 8).read());
        st(b, 0x30, f.fstp());
        f.fld(m(b, 0x1C));
        stu(b, 0x24, g::<u32>(POST).read());
        f.fsub_m(kd(POST + 4));
        stu(b, 0x28, g::<u32>(POST + 4).read());
        st(b, 0x34, f.fstp());
        f.fld(m(b, 0x20));
        f.fsub_m(kd(POST + 8));
        st(b, 0x38, f.fstp());
        vector_normalise(b.byte_add(0x30));
        f.fld(m(b, 0x30));
        f.fmul_m(kd(0x8CCD48));
        f.fld(m(b, 0x34));
        f.fmul_m(kd(0x8CCD48));
        f.fld(m(b, 0x38));
        f.fmul_m(kd(0x8CCD48));
        st(b, 0x74, f.fstp());
        f.fxch(1);
        f.fadd_m(m(b, 0x24));
        st(b, 0x24, f.fstp());
        f.fadd_m(m(b, 0x28));
        st(b, 0x28, f.fstp());
        f.fld(m(b, 0x74));
        f.fadd_m(m(b, 0x2C));
        st(b, 0x2C, f.fstp());
    }
    dist3_xzy(&mut f, b, 0x18, 0x24);
    f.fdiv_m(kd(0x8CCD44));
    ease(&mut f);
    f.fld(kd(0x8CCD40));
    f.fsub_m(kd(0x8CCD3C));
    f.fmulp(1);
    f.fadd_m(kd(0x8CCD3C));
    st(b, 0x14, f.fstp());
    f.fld(m(b, 0x10));
    f.fcomp_m(kd(0x8CCD38));
    if !f.jp(0x05) {
        f.fld(m(b, 0x10));
        f.fdiv_m(kd(0x8CCD38));
        ease(&mut f);
        f.fld(m(b, 0x14));
        f.fsub_m(kd(0x8CCD34));
        f.fmulp(1);
        f.fadd_m(kd(0x8CCD34));
        st(b, 0x14, f.fstp());
    }
    if is_time_to_exit_call(this, 0x15, b.byte_add(0x24), b.byte_add(0x18), mu(b, 0x10), 0) != 0 {
        return exit();
    }
    IGNORE_ENTITY.set(rd::<GPtr<u8>>(b, 0x78));
    let r = process_line_of_sight(
        b.byte_add(0x18),
        b.byte_add(0x24),
        b.byte_add(0xE0),
        b.byte_add(0x7C),
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    let clear = r == 0;
    wr::<u8>(b, 0x10, u8::from(clear));
    let c = g::<i32>(COUNTER).read();
    IGNORE_ENTITY.set(GPtr::NULL);
    if clear {
        g::<i32>(COUNTER).write(c.wrapping_add(1));
        let mx = g::<i32>(0x8CCD58).read();
        if c > mx {
            g::<i32>(COUNTER).write(mx);
        }
    } else {
        g::<i32>(COUNTER).write(c.wrapping_sub(1));
        if c == 0 {
            return exit();
        }
    }
    f.fld(m(b, 0x60));
    f.fmul_m(m(b, 0x60));
    f.fld(m(b, 0x68));
    f.fmul_m(m(b, 0x68));
    f.faddp(1);
    f.fld(m(b, 0x64));
    f.fmul_m(m(b, 0x64));
    f.faddp(1);
    f.fsqrt();
    f.fmul_m(kd(0x8CCD30));
    f.fcom_m(kd(0x8CCD2C));
    st(b, 0x10, f.fst());
    if f.jne(0x41) {
        f.fstp_st(0);
        f.fld(kd(0x8CCD2C));
    }
    f.fcomp_m(k(K_ZERO));
    if !f.jp(0x05) {
        stu(b, 0x10, 0);
    } else {
        f.fld(m(b, 0x10));
        f.fcom_m(kd(0x8CCD2C));
        if f.jne(0x41) {
            f.fstp_st(0);
            f.fld(kd(0x8CCD2C));
        }
        f.fcomp_m(k(K_ONE));
        if !f.jne(0x41) {
            stu(b, 0x10, K_ONE);
        } else {
            let r = max_f(mu(b, 0x10), g::<u32>(0x8CCD2C).read());
            st(b, 0x10, f32_of(X87::from_f64_exact(r)));
        }
    }
    f.fld(m(b, 0x14));
    f.fmul_m(k(0x3C6A_0EA1));
    f.fmul_m(k(0x411B_3333));
    f.fsubr_m(k(0x4120_0000));
    let near = f.fstp();
    finalise_call(
        this,
        b.byte_add(0x24),
        b.byte_add(0x18),
        0,
        mu(b, 0x14),
        near.to_bits(),
        mu(b, 0x10),
    );
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_DW_CAM_MAN_CAM,
    HOOK_PROCESS_DW_PLANE_SPOTTER_CAM,
    HOOK_PROCESS_DW_PLANE_CAM3,
    HOOK_PROCESS_DW_PLANE_CAM1,
    HOOK_PROCESS_DW_PLANE_CAM2,
    HOOK_FINALISE_DW_CINEY_CAMS,
    HOOK_GET_CORE_DATA_FOR_DW_CINEY_CAM_MODE,
    HOOK_IS_TIME_TO_EXIT_THIS_DW_CINEY_CAM_MODE,
    HOOK_GET_LOOK_FROM_LAMP_POST_POS,
];
