//! PORT-CAM-2: camera collision reaction (`CCamera::ImproveNearClip`, `CCamera::CameraColDetAndReact`).
//! Ported from the 1.0 US bytes; `docs/re/specs/CAM/CCamera.md` and gta-reversed `Camera.cpp` as
//! reading aids (both are `plugin::Call*` there).
//!
//! Values the original copies with integer moves (`mov eax, [mem]` / `push`) are carried as raw
//! dwords, so a signalling NaN in a `.data` tunable or a `gCamColVars` row reaches the callee unchanged.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::pools::MODEL_INFO_PTRS;
use sa_core::timer::TIME_STEP;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::camctl::calculate_ground_height;
use crate::camfx::{CAM_COL_VARS_PTR, CUR_CAM_COL_VARS, CUR_DIST_FOR_CAM};
use crate::common::{active_cam, ftol32, g, jne, jp, k, rd, the_camera, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `gLastRadiusUsedInCollisionPreventionOfCamera` (written by `CameraColDetAndReact`, read by
    /// `ImproveNearClip`)
    pub static LAST_RADIUS_USED_IN_COLLISION_PREVENTION @ 0xB6EC6C: f32 = 0.0;
    /// `CameraColDetAndReact`'s static previous source (set to 0 once, under bit 0 of 0xB700E8)
    pub static COL_DET_PREV_SOURCE @ 0xB700DC: [u32; 3] = [0; 3];
    /// its guard word
    pub static COL_DET_PREV_SOURCE_GUARD @ 0xB700E8: u32 = 0;
    /// `CameraColDetAndReact`'s static lowest sphere bottom (`centre.z - radius`) of the cached model
    pub static COL_DET_MIN_SPHERE_BOTTOM @ 0xB700EC: f32 = 0.0;
    /// the model index that bottom belongs to
    pub static COL_DET_CACHED_MODEL @ 0xB700F0: i32 = 0;
    // `.data` tunables of `ImproveNearClip` (variables, read at each use)
    /// near clip = this * `gCurDistForCam` when the camera is far from its target (1.0)
    pub static INC_FAR_NEAR_SCALE @ 0x8CCD04: f32 = 1.0;
    /// "far from the target" distance (10.0)
    pub static INC_FAR_DIST @ 0x8CCD08: f32 = 10.0;
    /// aircraft: minimum `gCurDistForCam` for the height rule (0.3)
    pub static INC_AIR_MIN_CUR_DIST @ 0x8CCD00: f32 = 0.3;
    /// aircraft / jet pack / parachute: height above the ground (10.0)
    pub static INC_HIGH_ABOVE_GROUND @ 0x8CCCFC: f32 = 10.0;
    /// aircraft: near clip = this * `gCurDistForCam` (5.0)
    pub static INC_AIR_CUR_DIST_SCALE @ 0x8CCCF8: f32 = 5.0;
    /// aircraft: near clip <= this * |source - target| (0.1)
    pub static INC_AIR_DIST_SCALE @ 0x8CCCF4: f32 = 0.1;
    /// helicopter close-in near clip (0.1)
    pub static INC_HELI_NEAR @ 0x8CCCF0: f32 = 0.1;
    /// swimming: |water level - source.z| below this sets the near clip (0.3)
    pub static INC_SWIM_SURFACE_DIST @ 0x8CCCEC: f32 = 0.3;
    /// swimming at the surface: near clip (0.1)
    pub static INC_SWIM_SURFACE_NEAR @ 0x8CCCE8: f32 = 0.1;
    /// swimming under water, zoom 1: near clip (0.1)
    pub static INC_SWIM_DEEP_NEAR @ 0x8CCCE4: f32 = 0.1;
    /// jet pack / parachute: near clip <= this * `gCurDistForCam` (2.0)
    pub static INC_JETPACK_CUR_DIST_SCALE @ 0x8CCCE0: f32 = 2.0;
    /// jet pack / parachute: near clip <= this * |source - target| (0.3)
    pub static INC_JETPACK_DIST_SCALE @ 0x8CCCDC: f32 = 0.3;
    /// standing ped: radius factor subtracted again for spheres of piece 9 (head) (1.0)
    pub static INC_HEAD_RADIUS_SCALE @ 0x8CCCD8: f32 = 1.0;
    /// standing ped: minimum near clip (0.02)
    pub static INC_PED_MIN_NEAR @ 0x8CCCD4: f32 = 0.02;
    /// standing ped: maximum near clip (0.3)
    pub static INC_PED_MAX_NEAR @ 0x8CCCD0: f32 = 0.3;
    // `.data` tunables of `CameraColDetAndReact`
    /// radius = |source - target| * cv[0] * this (0.2939)
    pub static CCDR_RADIUS_SCALE @ 0x8CCB90: f32 = 0.2939;
    /// minimum radius (0.65)
    pub static CCDR_MIN_RADIUS @ 0x8CCE18: f32 = 0.65;
    /// cone angle numerator for col-var sets 4..9 (0.3)
    pub static CCDR_ANGLE_FAR @ 0x8CCE14: f32 = 0.3;
    /// cone angle numerator for col-var sets 0..3 (0.18)
    pub static CCDR_ANGLE_NEAR @ 0x8CCE10: f32 = 0.18;
    /// cone angle on a bike (0.05)
    pub static CCDR_BIKE_ANGLE @ 0x8CCE0C: f32 = 0.05;
    /// "source moved" threshold, squared at use (0.01)
    pub static CCDR_MOVE_EPS @ 0x8CCE08: f32 = 0.01;
    /// maximum per-frame increase of `gCurDistForCam` (0.05)
    pub static CCDR_MAX_DIST_STEP @ 0x8CCE04: f32 = 0.05;
    /// bike: `gCurDistForCam` below this sets the bike near clip (0.5)
    pub static CCDR_BIKE_CLOSE_DIST @ 0x8CCE00: f32 = 0.5;
    /// bike close-in near clip (0.05)
    pub static CCDR_BIKE_NEAR @ 0x8CCDFC: f32 = 0.05;
}

/// `CWorld::pIgnoreEntity` (0xB7CD68, owner: WORLD)
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `RwCamera::nearPlane` offset.
const RW_CAM_NEAR: isize = 0x80;
/// `CCamera::m_nPedZoom` (+0xC8; `ImproveNearClip` reads it at the absolute 0xB6F0F0).
const PED_ZOOM: isize = 0xC8;

orig! {
    /// `RwCameraSetNearClipPlane(RwCamera*, RwReal)` (0x7EE1D0), the value passed as the dword the
    /// caller pushes
    pub fn set_near_clip_raw(cam: GPtr<u8>, near: u32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `CCamera::GetRoughDistanceToGround()` (0x516B00, ST0)
    pub fn get_rough_distance_to_ground(this: GPtr<u8>) -> f64 = thiscall 0x516B00 => get_rough_distance_to_ground_st0;
    /// `CPedIntelligence::GetUsingParachute()` (0x6011B0, AL)
    pub fn get_using_parachute(intel: GPtr<u8>) -> u8 = thiscall 0x6011B0 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    pub fn get_task_swim(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    pub fn get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool waves, CVector* normal)`
    /// (0x6EB690, cdecl, AL); the coordinates are pushed as dwords
    pub fn get_water_level_raw(x: u32, y: u32, z: u32, level: GPtr<f32>, waves: u32, normal: GPtr<u8>) -> u8 = cdecl 0x6EB690 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b, returns out
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    pub fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `std::min<float>(a, b)` instance (0x404330, ST0): `b` unless `a < b`
    pub fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => min_f_port;
    /// `CPedModelInfo::AnimatePedColModelSkinnedWorld(RpClump*)` (0x4C7170)
    pub fn animate_ped_col_model_skinned_world(mi: GPtr<u8>, clump: GPtr<u8>) = thiscall 0x4C7170 => unported;
    /// `CCollision::CheckPeds(const CVector& pos, const CVector& dir, float& t)` (0x4154A0, AL)
    pub fn check_peds(pos: GPtr<f32>, dir: GPtr<f32>, t: GPtr<f32>) -> u8 = cdecl 0x4154A0 => sa_world::collision::misc::check_peds;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, thiscall on the placement)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CCamera::ConeCastCollisionResolve(CVector* source, CVector* target, CVector* out, float radius,
    /// float angle, float* dist)` (0x51A5D0), radius and angle pushed as dwords
    pub fn cone_cast_collision_resolve_raw(this: GPtr<u8>, source: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>, radius: u32, angle: u32, dist: GPtr<u8>) -> u8 = thiscall 0x51A5D0 => cone_cast_collision_resolve_bits;
}

/// Phase B path of `get_rough_distance_to_ground` (ST0 as f64).
pub fn get_rough_distance_to_ground_st0(this: GPtr<u8>) -> f64 {
    crate::camctl::get_rough_distance_to_ground(this).to_f64()
}

/// Phase B path of `min_f` (0x404330): `fld a; fcomp b; test ah,5; jp` -> `b`, else `a`.
pub fn min_f_port(a: f32, b: f32) -> f64 {
    if jp(x(a), x(b), 0x05) {
        f64::from(b)
    } else {
        f64::from(a)
    }
}

/// Phase B path of `cone_cast_collision_resolve_raw`.
pub fn cone_cast_collision_resolve_bits(
    this: GPtr<u8>,
    source: GPtr<u8>,
    target: GPtr<u8>,
    out: GPtr<u8>,
    radius: u32,
    angle: u32,
    dist: GPtr<u8>,
) -> u8 {
    crate::camctl::cone_cast_collision_resolve(
        this,
        source,
        target,
        out,
        f32::from_bits(radius),
        f32::from_bits(angle),
        dist,
    )
}

/// The dword of an f32 global (an integer move in the original).
fn bits(gl: &sa_core::mem::Global<f32>) -> u32 {
    gl.ptr().cast::<u32>().read()
}

/// `fld dword [m]` of the f32 whose bits are `b`.
fn xb(b: u32) -> X87 {
    x(f32::from_bits(b))
}

/// `|*a - *b|` as the camera functions inline it: `d = a - b` (memory operands, kept on the stack),
/// `fld st0; fmul st,st1; fld st2; fmul st,st3; faddp; fld st3; fmul st,st4; faddp; fsqrt`, i.e.
/// `sqrt((dz*dz + dy*dy) + dx*dx)`.
fn dist_x87(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
    let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `CCamera::ImproveNearClip(CVehicle* veh, CPed* ped, CVector* source, CVector* target)` (0x516B20,
/// thiscall, `ret 0x10`). `near` = `Scene.camera->nearPlane`, read afresh at each use; `SetNear` =
/// `RwCameraSetNearClipPlane(Scene.camera, v)`.
/// 1. `|source - target| > INC_FAR_DIST` (`test ah,0x41`): `L = INC_FAR_NEAR_SCALE * gCurDistForCam`,
///    `SetNear(L)` when `L > near` (compared unrounded, passed as the f32 store).
/// 2. `veh` with `+0x594` 4 (plane) or 3 (heli): `gCurDistForCam > 0.3` -> when the active cam's
///    source z minus `CalculateGroundHeight(0)` is > 10: `L = (f32)(5 * gCurDistForCam)`, replaced by
///    `(f32)(|s - t| * 0.1)` when that is smaller; `SetNear(L)` when `L > near` (near read before the
///    distance). Otherwise (`<= 0.3` or NaN) a heli sets `SetNear(0.1)`. Other vehicles: nothing.
/// 3. `ped` (when `veh` is null), not standing (`+0x46C` bit 0 clear): `GetUsingParachute`, then with a
///    swim task: `GetWaterLevel(source, &lvl = 0, false, null)`; found and `|lvl - source.z| < 0.3`
///    (`fabs`, NaN no) -> `SetNear(0.1)`, else swim state (`task +0xA`, u16) 4 and `m_nPedZoom` == 1 ->
///    `SetNear(0.1)`. Without one, a parachute or a jet-pack task: when `GetRoughDistanceToGround() > 10`,
///    `L = min((f32)(Magnitude(s - t) * 0.3), (f32)(2 * gCurDistForCam))`, `SetNear(L)` when `L > near`.
///    Standing: `a = (f32)(sin((90 - fov * 0.5) * 0.017453292) * gLastRadius...)` (FSIN: sa-math's f64
///    model); `AnimatePedColModelSkinnedWorld`; over the first 12 spheres of the ped col model
///    `v = (c . F) - (f32)(F . S) - r` (`- 1.0 * r` again for piece 9), minimum kept on the x87 stack
///    (replaced by the f32 store of `v` when `v < min`, start 999999.0); then `min > a` -> `a`, else
///    `min < 0.02` -> 0.02; `min > 0.3` -> 0.3; `SetNear((f32)(_ftol(min * 100) * 0.01))`.
/// 4. Always: `CheckPeds(source, &activeCam.front, &t = 999999.0)`, result unused.
///
/// The FOV index comes from the absolute `TheCamera.m_nActiveCam` (0xB6F081) and the sphere loop's
/// camera from `this`'s index applied to the absolute `TheCamera` (0xB6F19C), as in the bytes.
#[hook(addr = 0x516B20, conv = thiscall, name = "CCamera::ImproveNearClip", expect = "83 EC 10 53 55 8B 6C 24 24", this = "CCamera")]
pub fn improve_near_clip(this: GPtr<u8>, veh: GPtr<u8>, ped: GPtr<u8>, source: GPtr<u8>, target: GPtr<u8>) {
    // 1. far from the target
    if !jne(dist_x87(source, target), x(INC_FAR_DIST.get()), 0x41) {
        let l = x(INC_FAR_NEAR_SCALE.get()).fmul_m(CUR_DIST_FOR_CAM.get());
        let l32 = l.to_f32(); // fst [esp+0x28]
        let cam = SCENE_CAMERA.get();
        let near = x(rd::<f32>(cam, RW_CAM_NEAR));
        if !jne(l, near, 0x41) {
            set_near_clip_raw(cam, l32.to_bits());
        }
    }
    if !veh.is_null() {
        let kind = rd::<u32>(veh, 0x594);
        if kind == 4 || kind == 3 {
            if !jne(x(CUR_DIST_FOR_CAM.get()), x(INC_AIR_MIN_CUR_DIST.get()), 0x41) {
                let gh = X87::from_f64_exact(calculate_ground_height(this, 0));
                let above = x(rd::<f32>(active_cam(this), 0x1A4)).fsub(gh);
                if !jne(above, x(INC_HIGH_ABOVE_GROUND.get()), 0x41) {
                    let cam = SCENE_CAMERA.get();
                    let near = x(rd::<f32>(cam, RW_CAM_NEAR));
                    let mut l = x(INC_AIR_CUR_DIST_SCALE.get())
                        .fmul_m(CUR_DIST_FOR_CAM.get())
                        .to_f32();
                    let m = dist_x87(source, target).fmul_m(INC_AIR_DIST_SCALE.get());
                    if !jp(m, x(l), 0x05) {
                        l = m.to_f32();
                    }
                    if !jne(x(l), near, 0x41) {
                        set_near_clip_raw(cam, l.to_bits());
                    }
                }
            } else if kind == 3 {
                let v = bits(&INC_HELI_NEAR);
                set_near_clip_raw(SCENE_CAMERA.get(), v);
            }
        }
    } else if !ped.is_null() {
        if rd::<u8>(ped, 0x46C) & 1 == 0 {
            ped_not_standing(this, ped, source, target);
        } else {
            ped_standing(this, ped);
        }
    }
    // 4. CheckPeds, result discarded (its distance lives in the source argument slot)
    let mut t = [999_999.0f32];
    let front = active_cam(this).byte_add(0x190);
    check_peds(source.cast(), front.cast(), GPtr::from_raw(t.as_mut_ptr()));
}

/// `ImproveNearClip`'s ped-in-the-air / in-water branch (0x516CC6..0x516DF8).
fn ped_not_standing(this: GPtr<u8>, ped: GPtr<u8>, source: GPtr<u8>, target: GPtr<u8>) {
    let para = get_using_parachute(rd::<GPtr<u8>>(ped, 0x47C));
    let swim = get_task_swim(rd::<GPtr<u8>>(ped, 0x47C));
    if !swim.is_null() {
        let deep = rd::<u16>(swim, 0xA) == 4;
        let mut lvl = [0.0f32];
        let found = get_water_level_raw(
            rd::<u32>(source, 0),
            rd::<u32>(source, 4),
            rd::<u32>(source, 8),
            GPtr::from_raw(lvl.as_mut_ptr()),
            0,
            GPtr::NULL,
        );
        let mut f = sa_world::collision::fpu::Fpu::new();
        f.fld(lvl[0]);
        f.fsub_m(rd::<f32>(source, 8));
        f.fabs();
        f.fcomp_m(INC_SWIM_SURFACE_DIST.get());
        let close = !f.jp(0x05);
        if found != 0 && close {
            let v = bits(&INC_SWIM_SURFACE_NEAR);
            set_near_clip_raw(SCENE_CAMERA.get(), v);
        } else if deep && rd::<i32>(the_camera(), PED_ZOOM) == 1 {
            let v = bits(&INC_SWIM_DEEP_NEAR);
            set_near_clip_raw(SCENE_CAMERA.get(), v);
        }
        return;
    }
    if para == 0 && get_task_jet_pack(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        return;
    }
    let h = X87::from_f64_exact(get_rough_distance_to_ground(this));
    if jne(h, x(INC_HIGH_ABOVE_GROUND.get()), 0x41) {
        return;
    }
    let b = x(INC_JETPACK_CUR_DIST_SCALE.get())
        .fmul_m(CUR_DIST_FOR_CAM.get())
        .to_f32();
    let mut tmp = [0u32; 3];
    let d = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), source, target);
    let a = X87::from_f64_exact(vector_magnitude(d))
        .fmul_m(INC_JETPACK_DIST_SCALE.get())
        .to_f32();
    let l = X87::from_f64_exact(min_f(a, b));
    let l32 = l.to_f32(); // fst [esp+0x34]
    let cam = SCENE_CAMERA.get();
    let near = x(rd::<f32>(cam, RW_CAM_NEAR));
    if !jne(l, near, 0x41) {
        set_near_clip_raw(cam, l32.to_bits());
    }
}

/// `ImproveNearClip`'s standing-ped branch (0x516DFD..0x5170ED).
fn ped_standing(this: GPtr<u8>, ped: GPtr<u8>) {
    let tc = the_camera();
    let ac = isize::from(rd::<u8>(tc, 0x59)); // absolute 0xB6F081
    let model = i32::from(rd::<i16>(ped, 0x22));
    let fov = x(rd::<f32>(tc, 0x174 + ac * 0x238 + 0xB4)); // 0xB6F250 + i*0x238
    let mi = MODEL_INFO_PTRS
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(model)
        .read();
    let clump = rd::<GPtr<u8>>(ped, 0x18);
    let a = fov
        .fmul_m(k(0x3F00_0000)) // 0.5 [0x858B8C]
        .fsubr_m(k(0x42B4_0000)) // 90.0 [0x85991C]
        .fmul_m(k(0x3C8E_FA35)) // 0.017453292 [0x8595EC]
        .fsin()
        .fmul_m(LAST_RADIUS_USED_IN_COLLISION_PREVENTION.get())
        .to_f32();
    animate_ped_col_model_skinned_world(mi, clump);

    let ai = isize::from(rd::<u8>(this, 0x59));
    let cam = tc.byte_add(0x174 + ai * 0x238); // absolute 0xB6F19C + i*0x238
    let f = |o: isize| rd::<f32>(cam, 0x190 + o);
    let s = |o: isize| rd::<f32>(cam, 0x19C + o);
    let spheres = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(mi, 0x34), 0x2C), 8);
    let d0 = x(f(8))
        .fmul_m(s(8))
        .fadd(x(f(4)).fmul_m(s(4)))
        .fadd(x(s(0)).fmul_m(f(0)))
        .to_f32();
    let mut mn = x(f32::from_bits(0x4974_23F0)); // 999999.0, through the stack slot
    for i in 0..12isize {
        let sp = spheres.byte_add(i * 0x14);
        let c = |o: isize| rd::<f32>(sp, o);
        let r = c(0xC);
        let piece = rd::<u8>(sp, 0x11);
        // operand order of the unrolled loop: sphere 0 (mod 6) x, z, y; sphere 1 z, x, then
        // `fld F.y; fmul c.y`; spheres 2..5 z, x, y
        let mut v = match i % 6 {
            0 => x(c(0))
                .fmul_m(f(0))
                .fadd(x(c(8)).fmul_m(f(8)))
                .fadd(x(c(4)).fmul_m(f(4))),
            1 => x(c(8))
                .fmul_m(f(8))
                .fadd(x(c(0)).fmul_m(f(0)))
                .fadd(x(f(4)).fmul_m(c(4))),
            _ => x(c(8))
                .fmul_m(f(8))
                .fadd(x(c(0)).fmul_m(f(0)))
                .fadd(x(c(4)).fmul_m(f(4))),
        };
        v = v.fsub_m(d0).fsub_m(r);
        let mut slot = v.to_f32();
        if piece == 9 {
            v = v.fsub(x(INC_HEAD_RADIUS_SCALE.get()).fmul_m(r));
            slot = v.to_f32();
        }
        if !jp(v, mn, 0x05) {
            mn = x(slot);
        }
    }
    if !jne(mn, x(a), 0x41) {
        mn = x(a);
    } else if !jp(mn, x(INC_PED_MIN_NEAR.get()), 0x05) {
        mn = x(INC_PED_MIN_NEAR.get());
    }
    if !jne(mn, x(INC_PED_MAX_NEAR.get()), 0x41) {
        mn = x(INC_PED_MAX_NEAR.get());
    }
    let cm = ftol32(mn.fmul_m(k(0x42C8_0000))); // 100.0 [0x858628]
    let v = xi(cm).fmul_m(k(0x3C23_D70A)).to_f32(); // 0.01 [0x858C58]
    set_near_clip_raw(SCENE_CAMERA.get(), v.to_bits());
}

/// `CCamera::CameraColDetAndReact(CVector* source, const CVector* target)` (0x520190, `ret 8`, AL).
/// ECX is never read (the Process_AimWeapon call site 0x52102D does not even load it): a stdcall
/// function. `cv` = `gpCamColVars`, `ccv` = `gCurCamColVars` (u8), `ign` = `CWorld::pIgnoreEntity`.
/// 1. `D = source - target` (each stored f32; z also kept unrounded for its square),
///    `R = (f32)(sqrt((dz*Dz + Dy*Dy) + Dx*Dx) * cv[0] * 0.2939)`.
/// 2. `ccv >= 10` and `ign`: an automobile (`(+0x36 & 7) == 2`, `+0x594 == 0`) caches the lowest sphere
///    bottom of its col model per model index (statics 0xB700EC / 0xB700F0), ensures its matrix and
///    takes `h = ((target - pos) . at) - bottom`, at least 0.2 (NaN kept); anything else takes the
///    smallest half extent of its model's bounding box. `h > cv[1]` -> `R = cv[1]` when `cv[1] < R`;
///    else `R = (f32)h` when `h < R`.
/// 3. `R > cv[1]` -> `cv[1]`; `R < 0.65` -> 0.65. Angle `A = cv[2]`; `ccv <= 9`: `A = (ccv > 3 ? 0.3 :
///    0.18) / |source - target|`; `ccv >= 10` and `ign` a bike (`+0x590 == 9`): `A = 0.05`, bike flag.
/// 4. `gLastRadius... = R`; `TheCamera.ConeCastCollisionResolve(source, target, &tmp, R, A, &dist)`:
///    hit -> result 1, `dist <= cv[3]` (`test ah,0x41; jp`) sets the near clip to `cv[4]`.
/// 5. `dist < gCurDistForCam` -> `gCurDistForCam = dist`; else, when the source moved
///    (`0.01*0.01 < |source - prev|^2`): `gCurDistForCam += min(ts * cv[5] * (dist - cur), 0.05)`, and
///    `prev = source` either way. Then at most 1.0; `source = target + D * gCurDistForCam` (z product
///    stored f32 first); bike and `gCurDistForCam < 0.5` -> near clip 0.05.
///
/// Ported but **not hooked**: when `*source == *target` exactly, `ConeCastCollisionResolve` returns 0
/// without writing `dist`, and step 5 reads the uninitialised stack local `[esp+0x10]` (the caller's
/// stack history) into `gCurDistForCam` and `source`. The port uses 1.0 (the cone cast's own miss
/// value) there. Hooking it needs a decision on that case (ledger) or a stack-faithful call. The
/// registration, once decided (checked by the sa-hook tests: entry bytes, callers' ECX/EDX):
/// `#[hook(addr = 0x520190, conv = stdcall, name = "CCamera::CameraColDetAndReact",
/// expect = "83 EC 24 53 55 8B 6C 24 30")]`.
pub fn camera_col_det_and_react(source: GPtr<u8>, target: GPtr<u8>) -> u8 {
    let mut cv = CAM_COL_VARS_PTR.get();
    let mut ccv = CUR_CAM_COL_VARS.get();
    let sv = |p: GPtr<u8>, i: isize| rd::<f32>(p, 4 * i);
    let ddx = x(sv(source, 0)).fsub_m(sv(target, 0)).to_f32();
    let ddy = x(sv(source, 1)).fsub_m(sv(target, 1)).to_f32();
    let dzv = x(sv(source, 2)).fsub_m(sv(target, 2));
    let ddz = dzv.to_f32();
    let mut r = dzv
        .fmul_m(ddz)
        .fadd(x(ddy).fmul_m(ddy))
        .fadd(x(ddx).fmul_m(ddx))
        .fsqrt()
        .fmul_m(rd::<f32>(cv, 0))
        .fmul_m(CCDR_RADIUS_SCALE.get())
        .to_f32()
        .to_bits();

    if ccv >= 10 {
        let ign = g::<GPtr<u8>>(IGNORE_ENTITY).read();
        if !ign.is_null() {
            let h = if rd::<u8>(ign, 0x36) & 7 == 2 && rd::<u32>(ign, 0x594) == 0 {
                if i32::from(rd::<i16>(ign, 0x22)) != COL_DET_CACHED_MODEL.get() {
                    let cd = rd::<GPtr<u8>>(get_col_model(ign), 0x2C);
                    COL_DET_MIN_SPHERE_BOTTOM.ptr().cast::<u32>().write(0x42C8_0000); // 100.0
                    if !cd.is_null() {
                        let n = i32::from(rd::<i16>(cd, 0));
                        let mut sp = rd::<GPtr<u8>>(cd, 8);
                        for _ in 0..n.max(0) {
                            let v = x(rd::<f32>(sp, 8)).fsub_m(rd::<f32>(sp, 0xC));
                            if !jp(v, x(COL_DET_MIN_SPHERE_BOTTOM.get()), 0x05) {
                                COL_DET_MIN_SPHERE_BOTTOM.set(v.to_f32());
                            }
                            sp = sp.byte_add(0x14);
                        }
                    }
                    COL_DET_CACHED_MODEL.set(i32::from(rd::<i16>(ign, 0x22)));
                    ccv = CUR_CAM_COL_VARS.get();
                    cv = CAM_COL_VARS_PTR.get();
                }
                if rd::<GPtr<u8>>(ign, 0x14).is_null() {
                    allocate_matrix(ign);
                    simple_transform_update_matrix(ign.byte_add(4), rd::<GPtr<u8>>(ign, 0x14));
                    cv = CAM_COL_VARS_PTR.get();
                    ccv = CUR_CAM_COL_VARS.get();
                }
                let m = rd::<GPtr<u8>>(ign, 0x14);
                let pos = if m.is_null() {
                    ign.byte_add(4)
                } else {
                    m.byte_add(0x30)
                };
                let px = x(sv(target, 0)).fsub_m(sv(pos, 0));
                let py = x(sv(target, 1)).fsub_m(sv(pos, 1));
                let pz = x(sv(target, 2)).fsub_m(sv(pos, 2));
                let mut h = pz
                    .fmul_m(rd::<f32>(m, 0x28))
                    .fadd(py.fmul_m(rd::<f32>(m, 0x24)))
                    .fadd(px.fmul_m(rd::<f32>(m, 0x20)))
                    .fsub_m(COL_DET_MIN_SPHERE_BOTTOM.get());
                let lo = x(k(0x3E4C_CCCD)); // 0.2 [0x858CC4]
                if !jne(lo, h, 0x41) {
                    h = lo;
                }
                h
            } else {
                let model = i32::from(rd::<i16>(ign, 0x22));
                let mi = MODEL_INFO_PTRS
                    .ptr()
                    .cast::<GPtr<u8>>()
                    .wrapping_index(model)
                    .read();
                let bb = rd::<GPtr<u8>>(mi, 0x14);
                let half = k(0x3F00_0000); // 0.5 [0x858B8C]
                let ex = x(sv(bb, 3)).fsub_m(sv(bb, 0));
                let ey = x(sv(bb, 4)).fsub_m(sv(bb, 1));
                let ez = x(sv(bb, 5)).fsub_m(sv(bb, 2)).to_f32();
                let hx = ex.fmul_m(half).to_f32();
                let hy = ey.fmul_m(half).to_f32();
                let hz = x(ez).fmul_m(half);
                if !jp(x(hx), x(hy), 0x05) {
                    if !jp(x(hx), hz, 0x05) { x(hx) } else { hz }
                } else if !jp(x(hy), hz, 0x05) {
                    x(hy)
                } else {
                    hz
                }
            };
            let cv1 = rd::<f32>(cv, 4);
            if !jne(h, x(cv1), 0x41) {
                if !jp(x(cv1), xb(r), 0x05) {
                    r = rd::<u32>(cv, 4);
                }
            } else if !jp(h, xb(r), 0x05) {
                r = h.to_f32().to_bits();
            }
        }
    }
    if !jne(xb(r), x(rd::<f32>(cv, 4)), 0x41) {
        r = rd::<u32>(cv, 4);
    }
    if !jp(xb(r), x(CCDR_MIN_RADIUS.get()), 0x05) {
        r = bits(&CCDR_MIN_RADIUS);
    }
    let mut angle = rd::<u32>(cv, 8);
    if ccv <= 9 {
        let num = if ccv > 3 {
            x(CCDR_ANGLE_FAR.get())
        } else {
            x(CCDR_ANGLE_NEAR.get())
        };
        angle = dist_x87(source, target).fdivr(num).to_f32().to_bits();
    }
    let mut bike = false;
    if ccv >= 10 {
        let ign = g::<GPtr<u8>>(IGNORE_ENTITY).read();
        if !ign.is_null() && rd::<u8>(ign, 0x36) & 7 == 2 && rd::<u32>(ign, 0x590) == 9 {
            angle = bits(&CCDR_BIKE_ANGLE);
            bike = true;
        }
    }

    // `dist` ([esp+0x10]) is not written by the cone cast when source == target (see above)
    let mut dist = [0x3F80_0000u32];
    let mut tmp = [0u32; 3];
    let dist_p = GPtr::from_raw(dist.as_mut_ptr()).cast::<u8>();
    LAST_RADIUS_USED_IN_COLLISION_PREVENTION
        .ptr()
        .cast::<u32>()
        .write(r);
    let hit = cone_cast_collision_resolve_raw(
        the_camera(),
        source,
        target,
        GPtr::from_raw(tmp.as_mut_ptr()).cast(),
        r,
        angle,
        dist_p,
    );
    let ret = if hit != 0 {
        let cvn = CAM_COL_VARS_PTR.get();
        if !jp(xb(dist[0]), x(rd::<f32>(cvn, 0xC)), 0x41) {
            set_near_clip_raw(SCENE_CAMERA.get(), rd::<u32>(cvn, 0x10));
        }
        1
    } else {
        0
    };
    let d = dist[0];

    let cur = CUR_DIST_FOR_CAM.ptr().cast::<u32>();
    if !jp(xb(d), x(CUR_DIST_FOR_CAM.get()), 0x05) {
        cur.write(d);
    } else {
        if COL_DET_PREV_SOURCE_GUARD.get() & 1 == 0 {
            COL_DET_PREV_SOURCE_GUARD.set(COL_DET_PREV_SOURCE_GUARD.get() | 1);
            COL_DET_PREV_SOURCE.set([0; 3]);
        }
        let prev = COL_DET_PREV_SOURCE.ptr().cast::<u8>();
        let mx = x(sv(source, 0)).fsub_m(sv(prev, 0));
        let my = x(sv(source, 1)).fsub_m(sv(prev, 1));
        let mz = x(sv(source, 2)).fsub_m(sv(prev, 2));
        let moved = mz.fmul(mz).fadd(my.fmul(my)).fadd(mx.fmul(mx));
        let eps = x(CCDR_MOVE_EPS.get()).fmul_m(CCDR_MOVE_EPS.get());
        if !jp(eps, moved, 0x05) {
            let cvn = CAM_COL_VARS_PTR.get();
            let t = x(TIME_STEP.get()).fmul_m(rd::<f32>(cvn, 0x14));
            let mut inc = t.fmul(xb(d).fsub_m(CUR_DIST_FOR_CAM.get()));
            if !jne(inc, x(CCDR_MAX_DIST_STEP.get()), 0x41) {
                inc = x(CCDR_MAX_DIST_STEP.get());
            }
            CUR_DIST_FOR_CAM.set(inc.fadd_m(CUR_DIST_FOR_CAM.get()).to_f32());
        }
        for i in 0..3 {
            wr::<u32>(prev, 4 * i, rd::<u32>(source, 4 * i));
        }
    }
    if !jne(x(CUR_DIST_FOR_CAM.get()), x(k(0x3F80_0000)), 0x41) {
        // 1.0 [0x858624]
        cur.write(0x3F80_0000);
    }
    let px = x(ddx).fmul_m(CUR_DIST_FOR_CAM.get());
    let py = x(ddy).fmul_m(CUR_DIST_FOR_CAM.get());
    let pz = x(ddz).fmul_m(CUR_DIST_FOR_CAM.get()).to_f32();
    let nx = px.fadd_m(sv(target, 0)).to_f32();
    let ny = py.fadd_m(sv(target, 1)).to_f32();
    let nz = x(pz).fadd_m(sv(target, 2)).to_f32();
    wr::<u32>(source, 0, nx.to_bits());
    wr::<u32>(source, 4, ny.to_bits());
    wr::<u32>(source, 8, nz.to_bits());
    if bike && !jp(x(CUR_DIST_FOR_CAM.get()), x(CCDR_BIKE_CLOSE_DIST.get()), 0x05) {
        let v = bits(&CCDR_BIKE_NEAR);
        set_near_clip_raw(SCENE_CAMERA.get(), v);
    }
    ret
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IMPROVE_NEAR_CLIP];
