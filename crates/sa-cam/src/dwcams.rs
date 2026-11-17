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

pub use crate::bindings::hand_shaker_process;
pub use sa_world::bindings::col_sphere_set;
pub use sa_core::bindings::cross_product;
pub use sa_core::bindings::matrix_mul_vec;
pub use sa_core::bindings::rw_camera_set_near_clip_plane;
pub use sa_core::bindings::vector_normalise;
pub use sa_world::bindings::placeable_allocate_matrix as allocate_matrix;
pub use sa_stream::bindings::simple_transform_update_matrix;
pub use sa_world::bindings::entity_get_bound_centre as get_bound_centre;
pub use sa_world::bindings::entity_get_col_model as get_col_model;
pub use sa_world::bindings::find_objects_in_range;
use sa_world::bindings::world_test_sphere_against_world;
use sa_world::bindings::vector_ctor_iterator;
pub use sa_world::bindings::multiply3x3_vm;
pub use sa_world::bindings::process_line_of_sight;
pub use sa_core::bindings::world_get_is_line_of_sight_clear as get_is_line_of_sight_clear;
orig! {
    /// `CCam::CacheLastSettingsDWCineyCam()` (0x50D7A0)
    pub fn cache_last_settings_dw_ciney_cam(cam: GPtr<u8>) = thiscall 0x50D7A0 => crate::cam::cache_last_settings_dw_ciney_cam;
    /// `IsLampPost(int32 modelId)` (0x509A30, AL)
    pub fn is_lamp_post(model: i32) -> u8 = cdecl 0x509A30 => unported;
}

use sa_core::field::ptr_of as vp;

use sa_core::field::copy3;

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
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(vp(&mut r)));
    let o = cross_product(vp(&mut t), vp(&mut r), front);
    copy3(up, o);
    let zero = x(k(K_ZERO));
    if !jp(x(rd::<f32>(front, 0)), zero, 0x44) && !jp(x(rd::<f32>(front, 4)), zero, 0x44) {
        wr::<u32>(front, 0, K_FRONT_EPS);
        wr::<u32>(front, 4, K_FRONT_EPS);
    }
    let o = cross_product(vp(&mut t), front, up);
    copy3(vp(&mut r), o);
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(vp(&mut r)));
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
    vector_normalise(front.cast::<[f32;3]>());
    orient(this, src, f32::from_bits(roll));
    wr::<u32>(this, 0xB4, fov);
    rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), f32::from_bits(K_NEAR_CLIP));
    cache_last_settings_dw_ciney_cam(this);
    DW_LAST_FRAME_PROCESSED.set(FRAME_COUNTER.get());
    hand_shaker_process(GPtr::from_addr(HAND_SHAKER), f32::from_bits(shake));
    let roll2 = f32_of(x(g::<f32>(HAND_SHAKER + 8).read()).fmul_m(f32::from_bits(shake)));
    let mut t: [f32; 3] = [0.0; 3];
    multiply3x3_vm(vp(&mut t), front, GPtr::from_addr(HAND_SHAKER + 0x3C));
    copy3(front, vp(&mut t));
    vector_normalise(front.cast::<[f32;3]>());
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
    col_sphere_set(sphere, f32::from_bits(radius).to_bits(), cp.cast::<u32>(), 0, 0, 0xFF);
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
        f32::from_bits(K_30),
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
            vector_normalise(GPtr::<u8>::cast::<[f32;3]>(vp(&mut dir)));
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

pub use crate::bindings::get_rough_distance_to_ground;
orig! {
    /// `CCam::GetCoreDataForDWCineyCamMode(...)` (0x517130)
    pub fn get_core_data_call(cam: GPtr<u8>, e: GPtr<u8>, v: GPtr<u8>, dest: GPtr<u8>, src: GPtr<u8>, up: GPtr<u8>, right: GPtr<u8>, fwd: GPtr<u8>, vel: GPtr<u8>, speed: GPtr<u8>, ang_vel: GPtr<u8>, ang_speed: GPtr<u8>, sphere: GPtr<u8>) = thiscall 0x517130 => get_core_data_for_dw_ciney_cam_mode;
    /// `CCam::IsTimeToExitThisDWCineyCamMode(...)` (0x517400, AL)
    pub fn is_time_to_exit_call(cam: GPtr<u8>, mode: i32, src: GPtr<u8>, dst: GPtr<u8>, t: u32, los: u32) -> u8 = thiscall 0x517400 => is_time_to_exit_this_dw_ciney_cam_mode;
    /// `CCam::Finalise_DW_CineyCams(...)` (0x50DD70)
    pub fn finalise_call(cam: GPtr<u8>, src: GPtr<u8>, dest: GPtr<u8>, roll: u32, fov: u32, near: u32, shake: u32) = thiscall 0x50DD70 => finalise_dw_ciney_cams;
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

use sa_core::field::rdf as m;
fn st(b: GPtr<u8>, off: isize, v: f32) {
    wr::<f32>(b, off, v)
}
use sa_core::field::rd32 as mu;
use sa_core::field::wr32 as stu;

/// `fsin` / `fcos` / `fidiv dword [m]` on the [`Fpu`] stack top.
fn fsin(f: &mut Fpu) {
    let v = f.pop();
    f.push(v.fsin());
}
use sa_pop::carctrl3::fidiv;
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
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(b.byte_add(0x64)));
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(b.byte_add(0x3C)));
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
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(b.byte_add(0x20)));
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(b.byte_add(0x2C)));
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

pub use sa_world::bindings::placeable_get_matrix as get_matrix;
pub use crate::bindings::hand_shaker_reset;
pub use sa_world::bindings::max_f;

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
            f32::from_bits(r),
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
                vector_normalise(GPtr::<u8>::cast::<[f32;3]>(b.byte_add(0x30)));
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
        vector_normalise(GPtr::<u8>::cast::<[f32;3]>(b.byte_add(0x30)));
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
            let r = max_f(f32::from_bits(mu(b, 0x10)), f32::from_bits(g::<u32>(0x8CCD2C).read())).to_f64();
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

use crate::bindings::dw_heli_chase_settings_init as heli_chase_settings_init_call;
orig! {
    /// `FUN_0050e090` (0x50E090): the heli-chase settings block's random scales
    fn heli_chase_settings_randomise_call(s: GPtr<u8>) = thiscall 0x50E090 => heli_chase_settings_randomise;
}

/// The heli-chase settings block `S` (0x9C bytes, `0xB6FEC0 - k * 0x9C` with `k` always 0).
const HELI_CHASE_SETTINGS: usize = 0xB6FEC0;
/// `1/32767` (0x858C7C), `2^-15` (0x858B14).
const K_RAND_UNIT: u32 = 0x3800_0100;
const K_RAND_15: u32 = 0x3800_0000;

/// `FUN_0050e090` (0x50E090, thiscall on the heli-chase settings block): five random scales, one
/// `rand()` each (sites 0x50E099 / 0x50E0BE / 0x50E0E3 / 0x50E108 / 0x50E12D): `+0x18 *= u * 1.4 +
/// 0.1`, `+0x1C / +0x20 / +0x24 *= u * 0.5 + 0.5`, `+0x28 *= u * 1.5 + 0.5` with `u = rand() / 32767`.
#[hook(addr = 0x50E090, conv = thiscall, name = "FUN_0050e090", expect = "51 56 8B F1 E8 85 3A 31 00",
       unverified = "frontier2-b20", route = "L2", hazards = "none")]
pub fn heli_chase_settings_randomise(this: GPtr<u8>) {
    for (site, off, mul, add) in [
        (0x50E099, 0x18, 0x3FB3_3333u32, 0x3DCC_CCCDu32),
        (0x50E0BE, 0x1C, K_HALF, K_HALF),
        (0x50E0E3, 0x20, K_HALF, K_HALF),
        (0x50E108, 0x24, K_HALF, K_HALF),
        (0x50E12D, 0x28, 0x3FC0_0000, K_HALF),
    ] {
        let r = sa_core::crt::rand_at(site);
        let mut f = Fpu::new();
        f.fild(r);
        f.fmul_m(k(K_RAND_UNIT));
        f.fmul_m(k(mul));
        f.fadd_m(k(add));
        f.fmul_m(m(this, off));
        st(this, off, f.fstp());
    }
}

/// The scene-reset part of the heli-chase start and failure paths (inlined twice): `+0x7C = +0x2C -
/// +0x30`, the flags +0x40 / +0x5C / +0x80 / +0x81 / +0x88 cleared, `+0x64 = +0x60`, `+0x44 = +0x48`.
fn heli_chase_reset(s: GPtr<u8>) {
    let mut f = Fpu::new();
    f.fld(m(s, 0x2C));
    f.fsub_m(m(s, 0x30));
    st(s, 0x7C, f.fstp());
    for o in [0x40, 0x5C, 0x80, 0x81] {
        wr::<u8>(s, o, 0);
    }
    stu(s, 0x64, mu(s, 0x60));
    stu(s, 0x44, mu(s, 0x48));
    wr::<u8>(s, 0x88, 0);
}

/// `CCam::Process_DW_HeliChaseCam(bool)` (0x51A740, thiscall, `ret 4`, AL; entry fragment 0x409B4B:
/// `sub esp, 0xEC`; mode 0x14, scene id 0x38): a chase helicopter. The settings block `S`
/// (0xB6FEC0) is reset (0x50E180) and randomised (0x50E090) at scene start; up to `S+0x58` tries put
/// the start `S+0xC` behind and the end `S+0x0` ahead of the target, both offset sideways by a random
/// sign, until the start sphere is free and the line to the target clear (none: `s_ExitCam[0x14]`).
/// Per frame the heli moves from start to end, keeps `S+0x74` away (2D), the FOV eases in and out,
/// the sphere lock 0xB70058 holds it near obstacles, the LOS counter `S+0x64` locks the target point
/// (`S+0x4C`), then `IsTimeToExit(0x14)` and `Finalise(src, dest, t * S+0x38, fov, S+0x3C, 1)`.
#[hook(addr = 0x51A740, conv = thiscall, name = "CCam::Process_DW_HeliChaseCam", expect = "90 E9 05 F4 EE FF",
       this = "CCam", unverified = "frontier2-b20", route = "L2", hazards = "x87-transcendental",
       note = "FSIN of the FOV ease curves")]
pub fn process_dw_heli_chase_cam(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC70;
    const LOCK_POS: usize = 0xB70058;
    const LOCK_GUARD: usize = 0xB70064;
    const LOCK_COUNT: usize = 0x8CCD24;
    g::<u8>(CAMERA_B6F059).write(0);
    // k = _ftol2((rand() & 0xFFFF) * 2^-15 * 0.0): always 0, the rand() is still drawn
    let r = sa_core::crt::rand_at(0x51A75C) & 0xFFFF;
    let mut f = Fpu::new();
    f.fild(r);
    f.fmul_m(k(K_RAND_15));
    f.fmul_m(k(K_ZERO));
    let kk = sa_math::fpu::ftol32(f.pop());
    let s = GPtr::<u8>::from_addr(HELI_CHASE_SETTINGS.wrapping_sub((kk as usize).wrapping_mul(0x9C)));
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
            0x6C, 0x10, 0x1C, 0x38, 0x8C, 0x80, 0x28, 0x54, 0x14, 0x70, 0x44, 0xBC,
        ],
    );
    let fail = || {
        g::<u8>(EXIT).write(1);
        0
    };
    let now = TIME_MS.get();
    stu(b, 0x44, now);
    let ent = rd::<GPtr<u8>>(b, 0x6C);
    let bs = s.byte_add(0xC);
    if !scene_running(0x38) {
        let end = g::<u32>(DW_DURATIONS as usize).read().wrapping_add(now);
        g::<u8>(EXIT).write(0);
        g::<i32>(DW_LAST_MODE).write(0x38);
        g::<u32>(DW_SCENE_START_TIME).write(now);
        DW_SCENE_END_TIME.set(end);
        heli_chase_settings_init_call(s);
        heli_chase_reset(s);
        heli_chase_settings_randomise_call(s);
        hand_shaker_reset(GPtr::from_addr(HAND_SHAKER));
        let mut n: i32 = 0;
        let found = loop {
            if n >= g::<i32>(s.addr() + 0x58).read() {
                break false;
            }
            // Bs = dest - fwd * S.1C
            f.fld(m(s, 0x1C));
            f.fld(m(b, 0x28));
            f.fmul_st(1);
            f.fld(m(b, 0x2C));
            f.fmul_st(2);
            st(b, 0xA8, f.fstp());
            f.fld(m(b, 0x30));
            f.fmul_st(2);
            st(b, 0xAC, f.fstp());
            f.fld(m(b, 0x1C));
            f.fsub_st(1);
            st(b, 0x48, f.fstp());
            stu(bs, 0, mu(b, 0x48));
            f.fstp_st(0);
            f.fstp_st(0);
            f.fld(m(b, 0x20));
            f.fsub_m(m(b, 0xA8));
            st(b, 0x4C, f.fstp());
            f.fld(m(b, 0x24));
            stu(bs, 4, mu(b, 0x4C));
            f.fsub_m(m(b, 0xAC));
            st(b, 0x50, f.fstp());
            // A = fwd * S.18 + dest
            f.fld(m(s, 0x18));
            f.fld(m(b, 0x28));
            stu(bs, 8, mu(b, 0x50));
            f.fmul_st(1);
            f.fld(m(b, 0x2C));
            f.fmul_st(2);
            st(b, 0x9C, f.fstp());
            f.fld(m(b, 0x30));
            f.fmul_st(2);
            st(b, 0xA0, f.fstp());
            f.fadd_m(m(b, 0x1C));
            st(b, 0x60, f.fstp());
            stu(s, 0, mu(b, 0x60));
            f.fstp_st(0);
            f.fld(m(b, 0x9C));
            f.fadd_m(m(b, 0x20));
            st(b, 0x64, f.fstp());
            f.fld(m(b, 0xA0));
            stu(s, 4, mu(b, 0x64));
            f.fadd_m(m(b, 0x24));
            st(b, 0x68, f.fstp());
            stu(s, 8, mu(b, 0x68));
            // the heights
            f.fld(m(s, 0x14));
            f.fadd_m(m(s, 0x20));
            st(s, 0x14, f.fstp());
            f.fld(m(s, 0x20));
            f.fadd_m(m(s, 8));
            st(s, 8, f.fstp());
            // the random sides: sx in memory (B+0x10), sy on the stack
            let neg_x = sa_core::crt::rand_at(0x51A97D) < 0x3FFF;
            let neg_y = sa_core::crt::rand_at(0x51A98A) < 0x3FFF;
            f.fld(k(K_ONE));
            stu(b, 0x10, K_ONE);
            if neg_x {
                stu(b, 0x10, K_M1);
            }
            if neg_y {
                f.fstp_st(0);
                f.fld(k(K_M1));
            }
            // Bs += (right.x * S.24, right.y * S.24, S.24 * 0) * sx
            f.fld(m(s, 0x24));
            f.fld(m(b, 0x80));
            f.fmul_st(1);
            f.fld(m(b, 0x84));
            f.fmul_st(2);
            st(b, 0xB4, f.fstp());
            f.fxch(1);
            f.fmul_m(k(K_ZERO));
            st(b, 0xB8, f.fstp());
            f.fmul_m(m(b, 0x10));
            f.fld(m(b, 0xB4));
            f.fmul_m(m(b, 0x10));
            f.fld(m(b, 0xB8));
            f.fmul_m(m(b, 0x10));
            st(b, 0x78, f.fstp());
            f.fxch(1);
            f.fadd_m(m(bs, 0));
            st(bs, 0, f.fstp());
            f.fadd_m(m(bs, 4));
            st(bs, 4, f.fstp());
            f.fld(m(b, 0x78));
            f.fadd_m(m(bs, 8));
            st(bs, 8, f.fstp());
            // A += (right.x * S.24, right.y * S.24, S.24 * 0) * sy
            f.fld(m(s, 0x24));
            f.fld(m(b, 0x80));
            f.fmul_st(1);
            f.fld(m(b, 0x84));
            let sphere_c = sa_math::vector::CVector::new(m(bs, 0), m(bs, 4), m(bs, 8));
            f.fmul_st(2);
            st(b, 0x90, f.fstp());
            f.fxch(1);
            f.fmul_m(k(K_ZERO));
            st(b, 0x94, f.fstp());
            f.fmul_st(1);
            st(b, 0x54, f.fstp());
            f.fld(m(b, 0x90));
            f.fmul_st(1);
            st(b, 0x58, f.fstp());
            f.fmul_m(m(b, 0x94));
            f.fld(m(b, 0x54));
            f.fadd_m(m(s, 0));
            st(s, 0, f.fstp());
            f.fld(m(b, 0x58));
            f.fadd_m(m(s, 4));
            st(s, 4, f.fstp());
            f.fadd_m(m(s, 8));
            st(s, 8, f.fstp());
            let hit = world_test_sphere_against_world(sphere_c, m(s, 0x78), 0, 1, 1, 0, 0, 0, 0);
            if hit.is_null() {
                IGNORE_ENTITY.set(ent);
                let r = process_line_of_sight(
                    b.byte_add(0x1C),
                    bs,
                    b.byte_add(0xD0),
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
                wr::<u8>(b, 0x37, u8::from(r == 0));
                IGNORE_ENTITY.set(GPtr::NULL);
                if r == 0 {
                    break true;
                }
            }
            n += 1;
        };
        if !found {
            heli_chase_reset(s);
            return fail();
        }
        let a = sa_core::crt::rand_at(0x51AB5C) < 0x3FFF;
        wr::<u8>(s, 0x81, u8::from(a));
        let c = sa_core::crt::rand_at(0x51AB6F) < 0x3FFF;
        wr::<u8>(s, 0x88, u8::from(c));
    }
    if g::<u8>(EXIT).read() != 0 {
        return 0;
    }
    // f = elapsed / duration (B+0x10)
    scene_t(&mut f, now);
    st(b, 0x10, f.fstp());
    // P = (A - Bs) * f + Bs, also the source Q (B+0x38)
    for (i, d) in [(0isize, 0x48isize), (4, 0x4C), (8, 0x50)] {
        f.fld(m(s, i));
        f.fsub_m(m(s, 0xC + i));
        f.fmul_m(m(b, 0x10));
        f.fadd_m(m(s, 0xC + i));
        st(b, d, f.fstp());
        stu(b, 0x38 + i, mu(b, d));
    }
    // dest += fwd * speed * S.34 + fwd
    f.fld(m(b, 0x28));
    f.fmul_m(m(b, 0x14));
    f.fld(m(b, 0x2C));
    f.fmul_m(m(b, 0x14));
    f.fld(m(b, 0x30));
    f.fmul_m(m(b, 0x14));
    stu(b, 0x14, mu(s, 0x34));
    st(b, 0x78, f.fstp());
    f.fxch(1);
    f.fmul_m(m(b, 0x14));
    st(b, 0x54, f.fstp());
    f.fmul_m(m(b, 0x14));
    f.fld(m(b, 0x78));
    f.fmul_m(m(b, 0x14));
    f.fld(m(b, 0x54));
    f.fadd_m(m(b, 0x28));
    st(b, 0x54, f.fstp());
    f.fxch(1);
    f.fadd_m(m(b, 0x2C));
    st(b, 0x58, f.fstp());
    f.fadd_m(m(b, 0x30));
    f.fld(m(b, 0x54));
    f.fadd_m(m(b, 0x1C));
    st(b, 0x1C, f.fstp());
    f.fld(m(b, 0x58));
    f.fadd_m(m(b, 0x20));
    st(b, 0x20, f.fstp());
    f.fadd_m(m(b, 0x24));
    st(b, 0x24, f.fstp());
    // keep S.74 away (2D)
    f.fld(m(b, 0x1C));
    f.fsub_m(m(b, 0x48));
    st(b, 0x28, f.fstp());
    f.fld(m(b, 0x20));
    f.fsub_m(m(b, 0x4C));
    st(b, 0x2C, f.fst());
    f.fmul_m(m(b, 0x2C));
    f.fld(m(b, 0x28));
    f.fmul_m(m(b, 0x28));
    f.faddp(1);
    f.fsqrt();
    f.fld(k(K_ONE));
    f.fdiv_st(1);
    f.fld(m(b, 0x28));
    f.fmul_st(1);
    st(b, 0x28, f.fstp());
    f.fmul_m(m(b, 0x2C));
    st(b, 0x2C, f.fstp());
    f.fcomp_m(m(s, 0x74));
    if !f.jp(0x05) {
        f.fld(m(b, 0x28));
        f.fmul_m(m(s, 0x74));
        f.fsubr_m(m(b, 0x1C));
        st(b, 0x38, f.fstp());
        f.fld(m(b, 0x2C));
        f.fmul_m(m(s, 0x74));
        f.fsubr_m(m(b, 0x20));
        st(b, 0x3C, f.fstp());
    }
    // the FOV, eased in from S.2C over the first S.28 unless S.81
    f.fld(m(s, 0x30));
    f.fld(m(b, 0x10));
    f.fcomp_m(m(s, 0x28));
    if !f.jp(0x05) && rd::<u8>(s, 0x81) == 0 {
        f.fstp_st(0);
        f.fld(k(K_ONE));
        f.fdiv_m(m(s, 0x28));
        f.fmul_m(m(b, 0x10));
        f.fmul_m(k(0x4334_0000));
        f.fsubr_m(k(0x4387_0000));
        f.fmul_m(k(K_DEG2RAD));
        fsin(&mut f);
        f.fadd_m(k(K_ONE));
        f.fmul_m(k(K_HALF));
        f.fld(m(s, 0x30));
        f.fsub_m(m(s, 0x2C));
        f.fmulp(1);
        f.fadd_m(m(s, 0x2C));
    }
    // the zoom with the distance beyond S.68
    f.fld(k(K_ZERO));
    f.fld(m(b, 0x38));
    f.fsub_m(m(b, 0x1C));
    f.fld(m(b, 0x3C));
    f.fsub_m(m(b, 0x20));
    f.fld(m(b, 0x40));
    f.fsub_m(m(b, 0x24));
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
    st(b, 0x18, f.fst());
    f.fcomp_m(m(s, 0x68));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(m(b, 0x18));
        f.fsub_m(m(s, 0x68));
        f.fld(m(s, 0x6C));
        f.fsub_m(m(s, 0x68));
        f.fdivp(1);
        ease(&mut f);
        f.fmul_m(m(s, 0x70));
    }
    f.fxch(1);
    f.fsub_st(1);
    st(b, 0x14, f.fstp());
    f.fstp_st(0);
    f.fld(m(b, 0x10));
    f.fmul_m(m(s, 0x38));
    st(b, 0x18, f.fstp());
    // the sphere lock
    let mut lock = rd::<u8>(s, 0x5C) != 0;
    if !lock {
        let c = sa_math::vector::CVector::new(m(b, 0x38), m(b, 0x3C), m(b, 0x40));
        let hit = world_test_sphere_against_world(c, g::<f32>(0x8CCD28).read(), 0, 1, 1, 0, 0, 0, 0);
        lock = !hit.is_null();
        if lock {
            if g::<u8>(LOCK_GUARD).read() & 1 == 0 {
                g::<u32>(LOCK_GUARD).write(g::<u32>(LOCK_GUARD).read() | 1);
            }
            for (i, o) in [0x38isize, 0x3C, 0x40].into_iter().enumerate() {
                g::<u32>(LOCK_POS + 4 * i).write(mu(b, o));
            }
            wr::<u8>(s, 0x5C, 1);
            g::<i32>(LOCK_COUNT).write(100 - 1);
        }
    } else {
        if g::<u8>(LOCK_GUARD).read() & 1 == 0 {
            g::<u32>(LOCK_GUARD).write(g::<u32>(LOCK_GUARD).read() | 1);
        }
        let c = g::<i32>(LOCK_COUNT).read();
        g::<i32>(LOCK_COUNT).write(c.wrapping_sub(1));
        if c < 0 {
            return fail();
        }
    }
    if lock {
        let lp = GPtr::<u8>::from_addr(LOCK_POS);
        f.fld(m(b, 0x38));
        f.fsub_m(m(lp, 0));
        f.fld(m(b, 0x3C));
        f.fsub_m(m(lp, 4));
        f.fld(m(b, 0x40));
        f.fsub_m(m(lp, 8));
        st(b, 0x78, f.fstp());
        f.fxch(1);
        f.fmul_m(g::<f32>(0x8CCD20).read());
        st(b, 0x54, f.fstp());
        f.fmul_m(g::<f32>(0x8CCD20).read());
        f.fld(m(b, 0x78));
        f.fmul_m(g::<f32>(0x8CCD20).read());
        f.fld(m(b, 0x54));
        f.fadd_m(m(lp, 0));
        st(b, 0x60, f.fstp());
        stu(b, 0x38, mu(b, 0x60));
        f.fxch(1);
        f.fadd_m(m(lp, 4));
        st(b, 0x64, f.fstp());
        stu(b, 0x3C, mu(b, 0x64));
        f.fadd_m(m(lp, 8));
        st(b, 0x68, f.fstp());
        stu(b, 0x40, mu(b, 0x68));
    }
    // the target point: tracked while the line is clear, then locked (S.4C) for S.44 frames
    if rd::<u8>(s, 0x40) == 0 {
        IGNORE_ENTITY.set(ent);
        let r = process_line_of_sight(
            b.byte_add(0x1C),
            b.byte_add(0x38),
            b.byte_add(0xD0),
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
        wr::<u8>(b, 0x37, u8::from(r == 0));
        IGNORE_ENTITY.set(GPtr::NULL);
        if r == 0 {
            wr::<u8>(s, 0x80, 1);
            if rd::<u8>(s, 0x88) == 0 && rd::<i32>(s, 0x64) < rd::<i32>(s, 0x60) / 4 {
                stu(s, 0x84, mu(b, 0x14));
                wr::<u8>(s, 0x88, 1);
                stu(s, 0x8C, now);
                stu(s, 0x90, mu(s, 0x98).wrapping_add(now));
            }
            let c = rd::<i32>(s, 0x64);
            wr::<i32>(s, 0x64, c.wrapping_sub(1));
            if c == 0 {
                for o in [0isize, 4, 8] {
                    stu(s, 0x4C + o, mu(b, 0x1C + o));
                }
                wr::<u8>(s, 0x40, 1);
            }
        } else {
            let c = rd::<i32>(s, 0x64).wrapping_add(1);
            wr::<i32>(s, 0x64, c);
            if c > rd::<i32>(s, 0x60) {
                stu(s, 0x64, mu(s, 0x60));
            }
        }
    } else {
        for o in [0isize, 4, 8] {
            stu(b, 0x1C + o, mu(s, 0x4C + o));
        }
        let c = rd::<i32>(s, 0x44);
        wr::<i32>(s, 0x44, c.wrapping_sub(1));
        if c == 0 {
            return fail();
        }
    }
    // the zoom out after S.94 of the scene
    if rd::<u8>(s, 0x88) == 0 {
        f.fld(m(b, 0x10));
        f.fcomp_m(m(s, 0x94));
        if !f.jne(0x01) {
            stu(s, 0x84, mu(b, 0x14));
            wr::<u8>(s, 0x88, 1);
            stu(s, 0x8C, now);
            stu(s, 0x90, mu(s, 0x98).wrapping_add(now));
        }
    }
    if rd::<u8>(s, 0x88) != 0 {
        f.fild(rd::<i32>(s, 0x8C));
        f.fild(now as i32);
        f.fsub_st(1);
        f.fild(rd::<i32>(s, 0x90));
        f.fsub_st(2);
        f.fdivp(1);
        f.fstp_st(1);
        ease(&mut f);
        f.fld(m(s, 0x2C));
        f.fsub_m(m(s, 0x84));
        f.fmulp(1);
        f.fadd_m(m(s, 0x84));
        st(b, 0x14, f.fstp());
    }
    if is_time_to_exit_call(this, 0x14, b.byte_add(0x38), b.byte_add(0x1C), mu(b, 0x10), 0) != 0 {
        return fail();
    }
    finalise_call(
        this,
        b.byte_add(0x38),
        b.byte_add(0x1C),
        mu(b, 0x18),
        mu(b, 0x14),
        mu(s, 0x3C),
        K_ONE,
    );
    1
}

/// The birdy cam's lamp filter `(flags & 4 || flags & 0x40000) && matrix.up.z > 0.9 &&
/// IsLampPost(model)` (the matrix allocated from the placement when missing).
fn birdy_lamp_ok(e: GPtr<u8>) -> bool {
    let fl = rd::<u32>(e, 0x1C);
    if fl & 4 == 0 && fl & 0x40000 == 0 {
        return false;
    }
    ensure_matrix(e);
    if jne(x(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 0x28)), x(k(K_0_9)), 0x41) {
        return false;
    }
    is_lamp_post(i32::from(rd::<i16>(e, 0x22))) != 0
}

/// The birdy cam's lamp top at frame offset `t` (`matrix * colModel.bbox.max` via the frame slot
/// `tmp`), lowered by `(h - 1) * u + 1` with `h = max.z - min.z * 0.5` (stored at `h_off`) and `u =
/// rand() / 32767` (the raw value stored at `r_off`, `rand()` at `site`).
fn birdy_lamp_top(e: GPtr<u8>, b: GPtr<u8>, t: isize, tmp: isize, h_off: isize, r_off: isize, site: u32) {
    let cm = get_col_model(e);
    for o in [0isize, 4, 8] {
        stu(b, t + o, rd::<u32>(cm, 0xC + o));
    }
    ensure_matrix(e);
    let o = matrix_mul_vec(b.byte_add(tmp), rd::<GPtr<u8>>(e, 0x14), b.byte_add(t));
    let (ox, oy, oz) = (rd::<u32>(o, 0), rd::<u32>(o, 4), rd::<u32>(o, 8));
    stu(b, t, ox);
    stu(b, t + 4, oy);
    stu(b, t + 8, oz);
    let cm_a = get_col_model(e);
    let cm_b = get_col_model(e);
    let mut f = Fpu::new();
    f.fld(rd::<f32>(cm_b, 8));
    f.fmul_m(k(K_HALF));
    f.fsubr_m(rd::<f32>(cm_a, 0x14));
    st(b, h_off, f.fstp());
    let r = sa_core::crt::rand_at(site);
    wr::<i32>(b, r_off, r);
    f.fild(r);
    f.fmul_m(k(K_RAND_UNIT));
    f.fld(m(b, h_off));
    f.fsub_m(k(K_ONE));
    f.fmul_st(1);
    f.fadd_m(k(K_ONE));
    f.fsubr_m(m(b, t + 8));
    st(b, t + 8, f.fstp());
}

/// `CCam::Process_DW_BirdyCam(bool)` (0x51B850, thiscall, `ret 4`, AL; SEH frame, `sub esp, 0x8F0`;
/// mode 0x16, scene id 0x3A): a camera sliding between two lamp posts. At scene start (also
/// `CHandShaker::Reset`, the LOS counter 0xB7007C = [0x8CCD74]) it collects up to 127 objects within
/// [0x8CCD70] of a point 2 x [0x8CCD70] ahead and of a point [0x8CCD68] behind the target, keeps
/// the static lamp posts of each list, then looks for a pair (lamp i of the first list, lamp j >= i
/// of the second: the inner index starts at the outer one) whose tops (each lowered by a random
/// amount, one `rand()` each, sites 0x51BCF1 / 0x51BEA1) see each other and the second sees the
/// target; none exits. Per frame the source eases from one lamp to the other (each pulled 1.0 /
/// 2.0 towards the other) unless `s_ExitCam[0x16]` is set, `IsTimeToExit(0x16)`, the LOS hysteresis,
/// then `Finalise(src, dest, 0, 70, 0.3, 0)`.
#[hook(addr = 0x51B850, conv = thiscall, name = "CCam::Process_DW_BirdyCam", expect = "6A FF 68 7E C8 83 00",
       this = "CCam", unverified = "frontier2-b20", route = "L2", hazards = "x87-transcendental",
       note = "FSIN of the lamp-to-lamp ease; the SEH frame of the static sLamps construction is not modelled")]
pub fn process_dw_birdy_cam(this: GPtr<u8>, _arg: u32) -> u8 {
    const EXIT: usize = 0xB6EC72;
    const GUARD: usize = 0xB70098;
    const COUNTER: usize = 0xB7007C;
    const LAMPS: usize = 0xB70080;
    g::<u8>(CAMERA_B6F059).write(0);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
        return 0;
    }
    // the frame holds the object and lamp lists up to B+0x8FF
    let mut frame = [0u32; 0x240];
    let b = GPtr::<u32>::from_raw(frame.as_mut_ptr()).cast::<u8>();
    core_data(
        this,
        b,
        [
            0xAC, 0x50, 0x44, 0x74, 0x24, 0x90, 0x30, 0x68, 0x80, 0x84, 0x1C, 0xC0,
        ],
    );
    let fail = || {
        g::<u8>(EXIT).write(1);
        0
    };
    let now = TIME_MS.get();
    stu(b, 0x1C, now);
    if g::<u8>(GUARD).read() & 1 == 0 {
        g::<u32>(GUARD).write(g::<u32>(GUARD).read() | 1);
        // the static CVector sLamps[2] (CVector::CVector 0x404350)
        vector_ctor_iterator(GPtr::from_addr(LAMPS), 0xC, 2, 0x404350);
    }
    let max = g::<u32>(0x8CCD74).read();
    if g::<u8>(GUARD).read() & 2 == 0 {
        g::<u32>(GUARD).write(g::<u32>(GUARD).read() | 2);
        g::<u32>(COUNTER).write(max);
    }
    let mut f = Fpu::new();
    if !scene_running(0x3A) {
        g::<i32>(DW_LAST_MODE).write(0x3A);
        DW_SCENE_END_TIME.set(g::<u32>(0x8CCBB0).read().wrapping_add(now));
        g::<u8>(EXIT).write(0);
        g::<u32>(DW_SCENE_START_TIME).write(now);
        g::<u32>(COUNTER).write(max);
        hand_shaker_reset(GPtr::from_addr(HAND_SHAKER));
        let kd = |a: usize| g::<f32>(a).read();
        // the point ahead: dest + fwd * 50 * 2
        f.fld(m(b, 0x30));
        f.fmul_m(kd(0x8CCD70));
        f.fld(m(b, 0x34));
        f.fmul_m(kd(0x8CCD70));
        f.fld(m(b, 0x38));
        f.fmul_m(kd(0x8CCD70));
        st(b, 0x70, f.fstp());
        f.fxch(1);
        f.fmul_m(kd(0x8CCD6C));
        st(b, 0x90, f.fstp());
        f.fmul_m(kd(0x8CCD6C));
        f.fld(m(b, 0x70));
        f.fmul_m(kd(0x8CCD6C));
        f.fld(m(b, 0x90));
        f.fadd_m(m(b, 0x44));
        st(b, 0x90, f.fstp());
        f.fld(m(b, 0x48));
        f.fadd_st(2);
        st(b, 0x94, f.fstp());
        f.fld(m(b, 0x4C));
        f.fadd_st(1);
        st(b, 0x98, f.fstp());
        f.fstp_st(0);
        f.fstp_st(0);
        find_objects_in_range(
            b.byte_add(0x90),
            kd(0x8CCD70),
            1,
            b.byte_add(0x20),
            0x7F,
            b.byte_add(0x100),
            0,
            0,
            0,
            1,
            1,
        );
        // the point behind: dest - fwd * 50 * 1
        f.fld(m(b, 0x30));
        f.fmul_m(kd(0x8CCD68));
        f.fld(m(b, 0x34));
        f.fmul_m(kd(0x8CCD68));
        f.fld(m(b, 0x38));
        f.fmul_m(kd(0x8CCD68));
        st(b, 0x38, f.fstp());
        f.fxch(1);
        f.fmul_m(kd(0x8CCD64));
        st(b, 0x84, f.fstp());
        f.fmul_m(kd(0x8CCD64));
        f.fld(m(b, 0x38));
        f.fmul_m(kd(0x8CCD64));
        f.fld(m(b, 0x44));
        f.fsub_m(m(b, 0x84));
        st(b, 0x68, f.fstp());
        f.fld(m(b, 0x48));
        f.fsub_st(2);
        st(b, 0x6C, f.fstp());
        f.fld(m(b, 0x4C));
        f.fsub_st(1);
        st(b, 0x70, f.fstp());
        f.fstp_st(0);
        f.fstp_st(0);
        find_objects_in_range(
            b.byte_add(0x68),
            kd(0x8CCD68),
            1,
            b.byte_add(0x40),
            0x7F,
            b.byte_add(0x300),
            0,
            0,
            0,
            1,
            1,
        );
        // the lamp posts of each list
        let mut n1: i32 = 0;
        let mut i = 0;
        while i < i32::from(rd::<i16>(b, 0x20)) {
            let e = rd::<GPtr<u8>>(b, 0x100 + 4 * i as isize);
            if birdy_lamp_ok(e) {
                wr::<GPtr<u8>>(b, 0x500 + 4 * isize::from(n1 as i16), e);
                n1 += 1;
            }
            i += 1;
        }
        stu(b, 0x54, 0);
        let mut i = 0;
        while i < i32::from(rd::<i16>(b, 0x40)) {
            let e = rd::<GPtr<u8>>(b, 0x300 + 4 * i as isize);
            if birdy_lamp_ok(e) {
                let n2 = rd::<i32>(b, 0x54);
                wr::<GPtr<u8>>(b, 0x700 + 4 * isize::from(n2 as i16), e);
                wr::<i32>(b, 0x54, n2.wrapping_add(1));
            }
            i += 1;
        }
        // the lamp pair
        let n1 = i32::from(n1 as i16);
        let mut found = false;
        let mut i = 0;
        while i < n1 && !found {
            let e1 = rd::<GPtr<u8>>(b, 0x500 + 4 * i as isize);
            'outer: {
                if !birdy_lamp_ok(e1) {
                    break 'outer;
                }
                birdy_lamp_top(e1, b, 0x10, 0x84, 0x50, 0x64, 0x51BCF1);
                for (q, d) in [(0x24isize, 0x30isize), (0x28, 0x34), (0x2C, 0x38)] {
                    f.fld(m(b, q - 0x14));
                    f.fsub_m(m(b, 0x44 + (q - 0x24)));
                    st(b, d, f.fstp());
                    stu(b, q, mu(b, d));
                }
                vector_normalise(GPtr::<u8>::cast::<[f32; 3]>(b.byte_add(0x24)));
                f.fld(m(b, 0x24));
                f.fadd_m(m(b, 0x44));
                st(b, 0x24, f.fstp());
                f.fld(m(b, 0x28));
                f.fadd_m(m(b, 0x48));
                st(b, 0x28, f.fstp());
                f.fld(m(b, 0x4C));
                f.fadd_m(m(b, 0x2C));
                st(b, 0x2C, f.fstp());
                // fld qword [0x859EF8] (0.0); fabs; fcomp [0x8CCD60]
                f.push(X87::from_f64_exact(0.0));
                f.fabs();
                f.fcomp_m(kd(0x8CCD60));
                if f.jp(0x05) {
                    break 'outer;
                }
                let mut j = i;
                while j < i32::from(rd::<i16>(b, 0x54)) && !found {
                    let e2 = rd::<GPtr<u8>>(b, 0x700 + 4 * j as isize);
                    if birdy_lamp_ok(e2) {
                        birdy_lamp_top(e2, b, 0x58, 0xB4, 0x64, 0x50, 0x51BEA1);
                        if get_is_line_of_sight_clear(b.byte_add(0x10), b.byte_add(0x58), 1, 0, 0, 0, 0, 1, 1) != 0
                            && get_is_line_of_sight_clear(b.byte_add(0x58), b.byte_add(0x24), 1, 0, 0, 0, 0, 1, 1) != 0
                        {
                            for (n, o) in [0x10isize, 0x14, 0x18].into_iter().enumerate() {
                                stu(b, 0x74 + 4 * n as isize, mu(b, o));
                                g::<u32>(LAMPS + 4 * n).write(mu(b, o));
                            }
                            for (n, o) in [0x58isize, 0x5C, 0x60].into_iter().enumerate() {
                                g::<u32>(LAMPS + 0xC + 4 * n).write(mu(b, o));
                            }
                            found = true;
                        }
                    }
                    j += 1;
                }
            }
            i += 1;
        }
        if !found {
            return fail();
        }
    }
    // per frame: the source between the two lamps
    let start = g::<u32>(DW_SCENE_START_TIME).read();
    f.fild(now.wrapping_sub(start) as i32);
    fidiv(&mut f, DW_SCENE_END_TIME.get().wrapping_sub(start) as i32);
    let lamps = GPtr::<u8>::from_addr(LAMPS);
    for (n, o) in [0x10isize, 0x14, 0x18].into_iter().enumerate() {
        stu(b, o, mu(lamps, 0xC + 4 * n as isize));
        stu(b, 0x24 + 4 * n as isize, mu(lamps, 4 * n as isize));
    }
    st(b, 0x3C, f.fstp());
    for n in 0..3isize {
        f.fld(m(lamps, 0xC + 4 * n));
        f.fsub_m(m(lamps, 4 * n));
        st(b, 0x9C + 4 * n, f.fstp());
    }
    vector_normalise(GPtr::<u8>::cast::<[f32; 3]>(b.byte_add(0x9C)));
    f.fld(m(b, 0x9C));
    f.fmul_m(k(g::<u32>(0x8CCD5C).read()));
    let failed = g::<u8>(EXIT).read() != 0;
    st(b, 0x84, f.fst());
    f.fld(m(b, 0xA0));
    f.fmul_m(k(g::<u32>(0x8CCD5C).read()));
    f.fld_st(0);
    f.fld(m(b, 0xA4));
    f.fmul_m(k(g::<u32>(0x8CCD5C).read()));
    st(b, 0x1C, f.fst());
    f.fld(m(b, 0x10));
    f.fsub_m(m(b, 0x84));
    st(b, 0x10, f.fstp());
    f.fld(m(b, 0x14));
    f.fsub_st(2);
    st(b, 0x14, f.fstp());
    f.fld(m(b, 0x18));
    f.fsub_st(1);
    st(b, 0x18, f.fstp());
    f.fstp_st(0);
    f.fstp_st(0);
    f.fxch(1);
    st(b, 0x84, f.fstp());
    f.fld(m(b, 0x1C));
    f.fld(m(b, 0x84));
    f.fadd_st(0);
    st(b, 0x68, f.fstp());
    f.fxch(1);
    f.fadd_st(0);
    st(b, 0x6C, f.fstp());
    f.fadd_st(0);
    f.fld(m(b, 0x68));
    f.fadd_m(m(b, 0x24));
    st(b, 0x24, f.fstp());
    f.fld(m(b, 0x6C));
    f.fadd_m(m(b, 0x28));
    st(b, 0x28, f.fstp());
    f.fadd_m(m(b, 0x2C));
    if !failed {
        f.fld(m(b, 0x3C));
        f.fmul_m(k(0x4334_0000));
        f.fsubr_m(k(0x4387_0000));
        f.fmul_m(k(K_DEG2RAD));
        fsin(&mut f);
        f.fld(k(K_ONE));
        f.fadd_st(1);
        f.fmul_m(k(K_HALF));
        f.fld(m(b, 0x24));
        f.fsub_m(m(b, 0x10));
        f.fmul_st(1);
        f.fadd_m(m(b, 0x10));
        st(b, 0x30, f.fstp());
        stu(b, 0x74, mu(b, 0x30));
        f.fld(m(b, 0x28));
        f.fsub_m(m(b, 0x14));
        f.fmul_st(1);
        f.fadd_m(m(b, 0x14));
        st(b, 0x34, f.fstp());
        stu(b, 0x78, mu(b, 0x34));
        f.fstp_st(0);
        f.fadd_m(k(K_ONE));
        f.fmul_m(k(K_HALF));
        f.fxch(1);
        f.fsub_m(m(b, 0x18));
        f.fmulp(1);
        f.fadd_m(m(b, 0x18));
        st(b, 0x38, f.fstp());
        stu(b, 0x7C, mu(b, 0x38));
    } else {
        f.fstp_st(0);
    }
    if is_time_to_exit_call(this, 0x16, b.byte_add(0x74), b.byte_add(0x44), mu(b, 0x3C), 0) != 0 {
        return fail();
    }
    IGNORE_ENTITY.set(rd::<GPtr<u8>>(b, 0xAC));
    let r = process_line_of_sight(
        b.byte_add(0x44),
        b.byte_add(0x74),
        b.byte_add(0xD4),
        b.byte_add(0xB0),
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    wr::<u8>(b, 0x20, u8::from(r == 0));
    let c = g::<i32>(COUNTER).read();
    IGNORE_ENTITY.set(GPtr::NULL);
    if r != 0 {
        g::<i32>(COUNTER).write(c.wrapping_sub(1));
        if c == 0 {
            return fail();
        }
    } else {
        g::<i32>(COUNTER).write(c.wrapping_add(1));
        let mx = g::<i32>(0x8CCD74).read();
        if c > mx {
            g::<i32>(COUNTER).write(mx);
        }
    }
    finalise_call(this, b.byte_add(0x74), b.byte_add(0x44), 0, K_FOV70, 0x3E99_999A, 0);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_DW_BIRDY_CAM,
    HOOK_PROCESS_DW_HELI_CHASE_CAM,
    HOOK_HELI_CHASE_SETTINGS_RANDOMISE,
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
