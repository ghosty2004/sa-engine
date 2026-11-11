//! PORT-CAM: `CCamera` control hand-over, cut-scene helpers, mirror set-up, sphere visibility and the
//! vehicle-mode collision flags (thiscall, `this` = `TheCamera` 0xB6F028), plus `CMBlur::MotionBlurOpen`.
//! Ported from the 1.0 US bytes; gta-reversed `Camera.cpp` and `docs/re/specs/CAM/CCamera.md` as
//! reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GBool, GPtr};
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_pop::common::get_atan_of_xy;
use sa_world::collision::misc::CAM_COLLIDE_WITH_PEDS;
use sa_world::world::common::{find_player_ped, find_player_vehicle};
use sa_world::world::cworld::{PLAYER_IN_FOCUS, PLAYER_INFO_SIZE, PLAYERS};

use crate::camera::{CAM_COLLIDE_WITH_OBJECTS, CAM_COLLIDE_WITH_VEHICLES};
use crate::common::{Fpu, active_cam, cam_of, g, k, rd, the_camera, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CalculateGroundHeight`'s static "lowest" height (0xB70028, returned for type 0)
    pub static GROUND_HEIGHT_LOWEST @ 0xB70028: f32 = 0.0;
    /// its static "top of the ground entity's bounding box" height (type 2)
    pub static GROUND_HEIGHT_TOP @ 0xB7002C: f32 = 0.0;
    /// its static colpoint height (type 1)
    pub static GROUND_HEIGHT_ROUGH @ 0xB70030: f32 = 0.0;
    /// its static last camera position (initialised once to 0 under the guard bit 0xB70040 & 1)
    pub static GROUND_HEIGHT_OLD_POS @ 0xB70034: [u32; 3] = [0; 3];
    /// the guard word of the static initialisation
    pub static GROUND_HEIGHT_GUARD @ 0xB70040: u32 = 0;
    /// `CopyCameraMatrixToRWCam`'s static previous rows: right 0xB6FF90, up 0xB6FF9C, at 0xB6FFA8, pos
    /// 0xB6FFB4 (each set to -99999.0 once, under the guard bits 1/2/4/8 of 0xB6FFC0, pos first)
    pub static PREV_RW_ROWS @ 0xB6FF90: [u32; 12] = [0; 12];
    /// their guard word
    pub static PREV_RW_ROWS_GUARD @ 0xB6FFC0: u32 = 0;
}

/// `CDraw::ms_fNearClipZ` (0xC3EFA0, owner: RENDER)
const NEAR_CLIP_Z: usize = 0xC3EFA0;
/// `CDraw::ms_fFarClipZ` (0xC3EF9C, owner: RENDER)
const FAR_CLIP_Z: usize = 0xC3EF9C;
/// `CCollision::relVelCamCollisionVehiclesSqr` (0x8A5B18, owner: COLL)
const REL_VEL_CAM_COLLISION_VEHICLES_SQR: usize = 0x8A5B18;
/// The speed (0x8CCC88, 0.2) `CameraVehicleModeSpecialCases` squares (a `.data` tweak value).
const VEHICLE_SPEED_FOR_CAM_COLLISION: usize = 0x8CCC88;
/// `preMirrorMat` (CMatrix, 0xB6FE40)
const PRE_MIRROR_MAT: usize = 0xB6FE40;
/// `RsGlobal.maximumWidth` / `maximumHeight` (0xC17044 / 0xC17048, owner: sa-core platform)
const SCREEN_WIDTH: usize = 0xC17044;
const SCREEN_HEIGHT: usize = 0xC17048;
/// `CDraw::ms_fFOV` (0x8D5038, owner: sa-core `DRAW_FOV`)
const DRAW_FOV: usize = 0x8D5038;
/// `CTimer::m_FrameCounter` (0xB7CB4C, owner: sa-core `FRAME_COUNTER`)
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `FrontEndMenuManager` byte 0xBA67A5 (+0x5D, tested by `CCamera::Init`)
const MENU_REINIT: usize = 0xBA67A5;
/// `.data` snap thresholds of `CopyCameraMatrixToRWCam` (0x8CCC7C position, 0x8CCC78 axes; 0.00001)
const CAM_POS_SNAP: usize = 0x8CCC7C;
const CAM_AXIS_SNAP: usize = 0x8CCC78;

orig! {
    /// `CCamera::IsSphereVisible(const CVector&, float, RwMatrix*)` (0x420C40)
    pub fn is_sphere_visible_m(this: GPtr<u8>, c: GPtr<u8>, r: f32, m: GPtr<u8>) -> u8 = thiscall 0x420C40 => is_sphere_visible_with_matrix;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix* m)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<f32>, inp: GPtr<f32>, n: i32, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDD90 => unported;
    /// `CCamera::SetPercentAlongCutScene(float)` (0x50C070)
    pub fn set_percent_along_cut_scene_call(this: GPtr<u8>, pct: f32) = thiscall 0x50C070 => set_percent_along_cut_scene;
    /// `CCamera::CalculateGroundHeight(eGroundHeightType)` (0x514B80); returns `fld dword` of a global
    pub fn calculate_ground_height(this: GPtr<u8>, kind: i32) -> f64 = thiscall 0x514B80 => calculate_ground_height_st0;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings, bool vehicles,
    /// bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0)
    pub fn process_vertical_line(p: GPtr<u8>, dist: f32, cp: GPtr<u8>, e: GPtr<u8>, b: u8, v: u8, pe: u8, o: u8, d: u8, st: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5674E0 => unported;
    /// `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    pub fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u8) = thiscall 0x59BD10 => unported;
    /// `CMBlur::MotionBlurOpen(RwCamera*)` (0x71D710)
    pub fn motion_blur_open_call(cam: GPtr<u8>) -> i32 = cdecl 0x71D710 => motion_blur_open;
    /// `CMBlur::MotionBlurRender(RwCamera*, uint32 r, g, b, a, int32 type, uint32 alpha)` (0x71D700, a lone `ret`)
    pub fn motion_blur_render(cam: GPtr<u8>, r: u32, gr: u32, b: u32, a: u32, kind: u32, alpha: u32) = cdecl 0x71D700 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    pub fn placeable_set_matrix(p: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CCamera::CopyCameraMatrixToRWCam(bool)` (0x50AFA0)
    pub fn copy_camera_matrix_to_rw_cam(this: GPtr<u8>, update: u8) = thiscall 0x50AFA0 => copy_camera_matrix_to_rw_cam_port;
    /// `CCamera::CalculateDerivedValues(bool, bool)` (0x5150E0)
    pub fn calculate_derived_values(this: GPtr<u8>, a: u8, b: u8) = thiscall 0x5150E0 => calculate_derived_values_port;
    /// `CMatrix::Invert(const CMatrix&)` returning by value (0x59BDD0, `Invert-2`: hidden return pointer first)
    pub fn matrix_invert(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BDD0 => unported;
    /// `CPlaceable::CPlaceable()` (0x54F1E0)
    pub fn placeable_ctor(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x54F1E0 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `CCamera::InitialiseScriptableComponents()` (0x50D2D0)
    pub fn initialise_scriptable_components(this: GPtr<u8>) = thiscall 0x50D2D0 => crate::camfx::initialise_scriptable_components;
    /// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20)
    pub fn matrix_update_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD20 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    pub fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    pub fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `RwFrameOrthoNormalize(RwFrame*)` (0x7F1170)
    pub fn rw_frame_ortho_normalize(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1170 => unported;
    /// `CCam::Init()` (0x50E490)
    pub fn cam_init(cam: GPtr<u8>) = thiscall 0x50E490 => crate::cam::cam_init;
    /// `CCamera::ClearPlayerWeaponMode()` (0x50AB10)
    pub fn clear_player_weapon_mode(this: GPtr<u8>) = thiscall 0x50AB10 => crate::camera::clear_player_weapon_mode;
    /// `CMatrix::SetScale(float)` (0x59AED0)
    pub fn matrix_set_scale(m: GPtr<u8>, s: f32) = thiscall 0x59AED0 => unported;
    /// `CCamera::SetMotionBlur(int32 r, int32 g, int32 b, int32 a, int32 type)` (0x50BF40)
    pub fn set_motion_blur_call(this: GPtr<u8>, r: i32, gr: i32, b: i32, a: i32, kind: i32) = thiscall 0x50BF40 => crate::camera::set_motion_blur;
    /// `operator!=(const CVector&, const CVector&)` (0x509760, cdecl, AL)
    pub fn vector_ne(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x509760 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CCamera::CalculateFrustumPlanes(bool)` (0x514D60)
    pub fn calculate_frustum_planes(this: GPtr<u8>, for_mirror: u8) = thiscall 0x514D60 => calculate_frustum_planes_port;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10)
    pub fn col_sphere_set(sph: GPtr<u8>, r: f32, c: GPtr<u8>, material: u8, flags: u8, lighting: u8) = thiscall 0x40FD10 => unported;
    /// `CCollision::CameraConeCastVsWorldCollision(CColSphere&, CColSphere&, float& dist, float angle)` (0x41B000)
    pub fn camera_cone_cast_vs_world_collision(s1: GPtr<u8>, s2: GPtr<u8>, dist: GPtr<u8>, angle: f32) -> u8 = cdecl 0x41B000 => unported;
    /// `TransformVectors(RwV3d* out, int32 n, const CMatrix& m, const RwV3d* in)` (0x54EEA0)
    pub fn transform_vectors(out: GPtr<u8>, n: i32, m: GPtr<u8>, inp: GPtr<u8>) = cdecl 0x54EEA0 => unported;
    /// `CCamera::ProcessVectorMoveLinear()` (0x5164A0)
    pub fn process_vector_move_linear(this: GPtr<u8>) = thiscall 0x5164A0 => unported;
    /// `CCamera::ProcessVectorTrackLinear()` (0x516440)
    pub fn process_vector_track_linear(this: GPtr<u8>) = thiscall 0x516440 => unported;
    /// `CCamera::ProcessFOVLerp()` (0x516500)
    pub fn process_fov_lerp(this: GPtr<u8>) = thiscall 0x516500 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CCamera::SetZoomValueFollowPedScript(int16)` (0x50C160)
    pub fn set_zoom_value_follow_ped_script_call(this: GPtr<u8>, zoom: u16) = thiscall 0x50C160 => crate::camera::set_zoom_value_follow_ped_script;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    pub fn get_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCamera::GetArrPosForVehicleType(eVehicleType, int32&)` (0x50AF00)
    pub fn get_arr_pos_for_vehicle_type_call(this: GPtr<u8>, kind: i32, out: GPtr<i32>) -> u8 = thiscall 0x50AF00 => get_arr_pos_for_vehicle_type;
}

/// The three-dword CVector copy (`mov edi,[esi]; mov [eax],edi; ...`).
fn copy_vec(dst: GPtr<u8>, off: isize, src: GPtr<u8>) {
    for i in 0..3 {
        let d = rd::<u32>(src, 4 * i);
        wr::<u32>(dst, off + 4 * i, d);
    }
}

/// `this->m_pExtraEntity[this->m_nExtraEntitiesCount++] = e`.
fn add_extra_entity(this: GPtr<u8>, e: GPtr<u8>) {
    let n = rd::<i32>(this, 0xC48);
    wr::<GPtr<u8>>(this, 0xC4C + (n as isize) * 4, e);
    wr::<i32>(this, 0xC48, rd::<i32>(this, 0xC48).wrapping_add(1));
}

/// `m_fPedOrientForBehindOrInFront` (+0x120) = `GetATanOfXY(m->up.x, m->up.y)` of the entity's matrix
/// (`[e+0x14]`, not null-checked). The raw FPATAN result reaches Rust as an f64 (the i686 ABI's
/// `fstp qword`) and is stored as f32 (`fstp dword`): a double rounding the original does not do
/// (64 -> 24 bits); it can differ only when the 53-bit value lies on an f32 rounding tie.
fn store_ped_orient(this: GPtr<u8>, e: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(e, 0x14);
    let a = get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14));
    wr::<f32>(this, 0x120, a as f32);
}

/// `CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString()` (0x50BD40).
#[hook(addr = 0x50BD40, conv = thiscall, name = "CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString", expect = "56 8B F1 6A FF", this = "CCamera")]
pub fn set_camera_directly_behind_for_follow_ped_cam_on_a_string(this: GPtr<u8>) {
    wr::<u8>(this, 0x1A, 1);
    let ped = find_player_ped(-1);
    if !ped.is_null() {
        store_ped_orient(this, ped);
    }
}

/// `CCamera::SetCameraDirectlyInFrontForFollowPed_CamOnAString()` (0x50BD70).
#[hook(addr = 0x50BD70, conv = thiscall, name = "CCamera::SetCameraDirectlyInFrontForFollowPed_CamOnAString", expect = "56 8B F1 6A FF", this = "CCamera")]
pub fn set_camera_directly_in_front_for_follow_ped_cam_on_a_string(this: GPtr<u8>) {
    wr::<u8>(this, 0x1B, 1);
    let ped = find_player_ped(-1);
    if !ped.is_null() {
        store_ped_orient(this, ped);
    }
}

/// The `_ForAPed_` variants: with a ped, set the flag, clear +0x2B, make the ped `TheCamera`'s target
/// entity (+0x958) and the active cam's `m_pCamTargetEntity` (cam +0x21C, through the absolute
/// `TheCamera` addresses 0xB6F081 / 0xB6F3B8 as the bytes do, with the reference clean-up and
/// registration), then the ped orientation.
fn for_a_ped(this: GPtr<u8>, ped: GPtr<u8>, flag: isize) {
    if ped.is_null() {
        return;
    }
    let tc = the_camera();
    wr::<u8>(this, flag, 1);
    wr::<u8>(this, 0x2B, 0);
    let slot = active_cam(tc).byte_add(0x21C);
    wr::<GPtr<u8>>(tc, 0x958, ped);
    let old = slot.cast::<GPtr<u8>>().read();
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    active_cam(tc).byte_add(0x21C).cast::<GPtr<u8>>().write(ped);
    let slot = active_cam(tc).byte_add(0x21C);
    register_reference(slot.cast::<GPtr<u8>>().read(), slot);
    store_ped_orient(this, ped);
}

/// `CCamera::SetCameraDirectlyBehindForFollowPed_ForAPed_CamOnAString(CPed*)` (0x50BDA0).
#[hook(addr = 0x50BDA0, conv = thiscall, name = "CCamera::SetCameraDirectlyBehindForFollowPed_ForAPed_CamOnAString", expect = "56 57 8B 7C 24 0C", this = "CCamera")]
pub fn set_camera_directly_behind_for_follow_ped_for_a_ped_cam_on_a_string(this: GPtr<u8>, ped: GPtr<u8>) {
    for_a_ped(this, ped, 0x1A);
}

/// `CCamera::SetCameraDirectlyInFrontForFollowPed_ForAPed_CamOnAString(CPed*)` (0x50BE30).
#[hook(addr = 0x50BE30, conv = thiscall, name = "CCamera::SetCameraDirectlyInFrontForFollowPed_ForAPed_CamOnAString", expect = "56 57 8B 7C 24 0C", this = "CCamera")]
pub fn set_camera_directly_in_front_for_follow_ped_for_a_ped_cam_on_a_string(this: GPtr<u8>, ped: GPtr<u8>) {
    for_a_ped(this, ped, 0x1B);
}

/// `CMBlur::MotionBlurOpen(RwCamera*)` (0x71D710): `mov eax, 1; ret`.
#[hook(addr = 0x71D710, conv = cdecl, name = "CMBlur::MotionBlurOpen", expect = "B8 01 00 00 00")]
pub fn motion_blur_open(_cam: GPtr<u8>) -> i32 {
    1
}

/// `CCamera::SetRwCamera(RwCamera*)` (0x50C100): stores it (+0x954), attaches `m_mViewMatrix`... the
/// CMatrix at +0xA04 to the RwMatrix at camera + 0x20 (not owned), then `MotionBlurOpen`.
#[hook(addr = 0x50C100, conv = thiscall, name = "CCamera::SetRwCamera", expect = "8B 44 24 04 56", this = "CCamera")]
pub fn set_rw_camera(this: GPtr<u8>, cam: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x954, cam);
    matrix_attach(this.byte_add(0xA04), cam.byte_add(0x20), 0);
    motion_blur_open_call(rd::<GPtr<u8>>(this, 0x954));
}

/// `CCamera::RenderMotionBlur()` (0x50B8F0): with a blur type (+0x90), `MotionBlurRender(m_pRwCamera,
/// r +0x8C, g +0x88, b +0x84, a +0xA0, type, alpha +0xA4)`.
#[hook(addr = 0x50B8F0, conv = thiscall, name = "CCamera::RenderMotionBlur", expect = "8B 81 90 00 00 00", this = "CCamera")]
pub fn render_motion_blur(this: GPtr<u8>) {
    let kind = rd::<u32>(this, 0x90);
    if kind == 0 {
        return;
    }
    motion_blur_render(
        rd::<GPtr<u8>>(this, 0x954),
        rd::<u32>(this, 0x8C),
        rd::<u32>(this, 0x88),
        rd::<u32>(this, 0x84),
        rd::<u32>(this, 0xA0),
        kind,
        rd::<u32>(this, 0xA4),
    );
}

/// `CCamera::IsSphereVisible(const CVector&, float, RwMatrix*)` (0x420C40, `ret 0xC`): the centre in
/// camera space (x copied as a dword, y and z through `fld`/`fstp`, so a signalling NaN is quieted),
/// then the near/far and four frustum-plane tests, each as the bytes order the products.
///
/// Ported but **unhookable**: protection code reads its entry bytes as data (`push 0x420C46` at
/// 0x14664CF; sa-hook `every_hook_installs_on_the_exe` rejects it). Its callers (incl. the Rust
/// `is_sphere_visible`) reach it through `orig!`.
pub fn is_sphere_visible_with_matrix(this: GPtr<u8>, c: GPtr<u8>, r: f32, m: GPtr<u8>) -> u8 {
    let mut p = [
        rd::<f32>(c, 0),
        x(rd::<f32>(c, 4)).to_f32(),
        x(rd::<f32>(c, 8)).to_f32(),
    ];
    let pp = GPtr::from_raw(p.as_mut_ptr());
    rw_v3d_transform_points(pp, pp, 1, m);
    let (px, py, pz) = (p[0], p[1], p[2]);
    let mut f = Fpu::new();
    // p.y + r < near: not visible (`test ah,5; jnp`)
    f.fld(py);
    f.fadd_m(r);
    f.fcomp_m(g::<f32>(NEAR_CLIP_Z).read());
    if !f.jp(0x05) {
        return 0;
    }
    // p.y - r > far (`test ah,0x41; je`)
    f.fld(py);
    f.fsub_m(r);
    f.fcomp_m(g::<f32>(FAR_CLIP_Z).read());
    if !f.jne(0x41) {
        return 0;
    }
    let plane = |f: &mut Fpu, a: f32, na: isize, b: f32, nb: isize| -> bool {
        f.fld(a);
        f.fmul_m(rd::<f32>(this, na));
        f.fld(b);
        f.fmul_m(rd::<f32>(this, nb));
        f.faddp(1);
        f.fcomp_m(r);
        f.jne(0x41)
    };
    if !plane(&mut f, px, 0xB24, py, 0xB28) {
        return 0;
    }
    if !plane(&mut f, px, 0xB30, py, 0xB34) {
        return 0;
    }
    if !plane(&mut f, pz, 0xB44, py, 0xB40) {
        return 0;
    }
    u8::from(plane(&mut f, pz, 0xB50, py, 0xB4C))
}

/// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40): against `m_mMatInverse` (+0xA4C), then,
/// with the mirror active (+0x970), against `m_mMatMirrorInverse` (+0xA94).
#[hook(addr = 0x420D40, conv = thiscall, name = "CCamera::IsSphereVisible", expect = "53 8B 5C 24 08", this = "CCamera")]
pub fn is_sphere_visible(this: GPtr<u8>, c: GPtr<u8>, r: f32) -> u8 {
    if is_sphere_visible_m(this, c, r, this.byte_add(0xA4C)) != 0 {
        return 1;
    }
    if rd::<u8>(this, 0x970) == 0 {
        return 0;
    }
    is_sphere_visible_m(this, c, r, this.byte_add(0xA94))
}

/// `CCamera::ProcessScriptedCommands()` (0x516AE0): `ProcessVectorMoveLinear`, `ProcessVectorTrackLinear`,
/// then (tail jump) `ProcessFOVLerp`; the original keeps `this` in EDX across the first two calls.
#[hook(addr = 0x516AE0, conv = thiscall, name = "CCamera::ProcessScriptedCommands", expect = "8B D1 E8 B9 F9 FF FF", this = "CCamera")]
pub fn process_scripted_commands(this: GPtr<u8>) {
    process_vector_move_linear(this);
    process_vector_track_linear(this);
    process_fov_lerp(this);
}

/// `CCamera::GetRoughDistanceToGround()` (0x516B00): the active cam's source z (cam +0x1A4) minus
/// `CalculateGroundHeight(0)` (`fsubr dword`).
#[hook(addr = 0x516B00, conv = thiscall, name = "CCamera::GetRoughDistanceToGround", expect = "56 6A 00 8B F1", this = "CCamera")]
pub fn get_rough_distance_to_ground(this: GPtr<u8>) -> X87 {
    let gh = X87::from_f64_exact(calculate_ground_height(this, 0));
    gh.fsubr_m(rd::<f32>(active_cam(this), 0x1A4))
}

/// `CCamera::FinishCutscene()` (0x514950).
#[hook(addr = 0x514950, conv = thiscall, name = "CCamera::FinishCutscene", expect = "56 68 00 00 C8 42", this = "CCamera")]
pub fn finish_cutscene(this: GPtr<u8>) {
    set_percent_along_cut_scene_call(this, 100.0);
    wr::<u32>(this, 0x140, 0x3F80_0000);
    wr::<u8>(this, 0x1D, 1);
}

/// `CCamera::DeleteCutSceneCamDataMemory()` (0x5B24A0): frees the four spline buffers (+0x960..) that
/// are set and clears them.
#[hook(addr = 0x5B24A0, conv = thiscall, name = "CCamera::DeleteCutSceneCamDataMemory", expect = "56 57 8D B1 60 09 00 00", this = "CCamera")]
pub fn delete_cut_scene_cam_data_memory(this: GPtr<u8>) {
    for i in 0..4 {
        let p = rd::<GPtr<u8>>(this, 0x960 + 4 * i);
        if !p.is_null() {
            crt_free(p);
            wr::<GPtr<u8>>(this, 0x960 + 4 * i, GPtr::NULL);
        }
    }
}

/// `CCamera::TakeControlNoEntity(const CVector&, eSwitchType, int32 whoIsInControl)` (0x50C8B0): ignored
/// when the script (2) asks while the player (1) has control (+0xB0).
#[hook(addr = 0x50C8B0, conv = thiscall, name = "CCamera::TakeControlNoEntity", expect = "8B 44 24 0C 83 F8 02", this = "CCamera")]
pub fn take_control_no_entity(this: GPtr<u8>, pos: GPtr<u8>, switch_type: u16, who: i32) {
    if who == 2 && rd::<i32>(this, 0xB0) == 1 {
        return;
    }
    wr::<i32>(this, 0xB0, who);
    wr::<u8>(this, 0x2C, 1);
    wr::<u16>(this, 0xC38, 0xF);
    wr::<u8>(this, 0x2B, 0);
    copy_vec(this, 0x860, pos);
    wr::<u16>(this, 0xC3C, switch_type);
    wr::<u8>(this, 0x32, 1);
}

/// `CCamera::GetArrPosForVehicleType(eVehicleType, int32&)` (0x50AF00): types 1..5 give index type-1.
#[hook(addr = 0x50AF00, conv = thiscall, name = "CCamera::GetArrPosForVehicleType", expect = "8B 44 24 04 48", this = "CCamera")]
pub fn get_arr_pos_for_vehicle_type(_this: GPtr<u8>, kind: i32, out: GPtr<i32>) -> u8 {
    let i = kind.wrapping_sub(1);
    if (i as u32) > 4 {
        return 0;
    }
    out.write(i);
    1
}

/// The `fld pct; fmul 0.01; fild time; [fadd 2^32]; fmulp; fstp cam+0x80` of `SetPercentAlongCutScene`.
fn set_cut_scene_cam_time(cam: GPtr<u8>, pct: f32) {
    let t = rd::<i32>(cam, 0x10);
    let mut f = Fpu::new();
    f.fld(pct);
    f.fmul_m(k(0x3C23_D70A)); // 0x858C58: 0.01
    f.fild(t);
    if t < 0 {
        f.fadd_m(k(0x4F80_0000)); // 0x858C54: 2^32 (uint32 conversion)
    }
    f.fmulp(1);
    wr::<f32>(cam, 0x90, f.fstp());
}

/// `CCamera::SetPercentAlongCutScene(float)` (0x50C070): on the active cam when it is in MODE_FLYBY
/// (0x11), else on the other cam when that one is: `m_fTimeElapsedFloat` (cam +0x90) = pct * 0.01 *
/// `m_nFinishTime` (cam +0x10, uint32).
#[hook(addr = 0x50C070, conv = thiscall, name = "CCamera::SetPercentAlongCutScene", expect = "0F B6 41 59 8B D0", this = "CCamera")]
pub fn set_percent_along_cut_scene(this: GPtr<u8>, pct: f32) {
    let active = i32::from(rd::<u8>(this, 0x59));
    let c = cam_of(this, active);
    if rd::<u16>(c, 0xC) == 0x11 {
        set_cut_scene_cam_time(c, pct);
        return;
    }
    let c = cam_of(this, (active + 1) % 2);
    if rd::<u16>(c, 0xC) == 0x11 {
        set_cut_scene_cam_time(c, pct);
    }
}

/// `CCamera::CameraVehicleModeSpecialCases(CVehicle*)` (0x50CDE0): objects never collide; when 0.2^2
/// (the `.data` value 0x8CCC88 squared) is below the vehicle's squared speed (`x*x + y*y + z*z` of
/// +0x44.., x87 sum), vehicles collide at relative speed^2 1.0 and peds not; otherwise (also NaN) all
/// collide at 0.01. A vehicle's `+0x4C8` entity (when set) is added to the extra entities.
#[hook(addr = 0x50CDE0, conv = thiscall, name = "CCamera::CameraVehicleModeSpecialCases", expect = "8B 54 24 04 C6 05 15 5B 8A 00 00", this = "CCamera")]
pub fn camera_vehicle_mode_special_cases(this: GPtr<u8>, veh: GPtr<u8>) {
    CAM_COLLIDE_WITH_OBJECTS.set(GBool::FALSE);
    let mut f = Fpu::new();
    f.fld(rd::<f32>(veh, 0x4C));
    f.fld(rd::<f32>(veh, 0x48));
    f.fld(rd::<f32>(veh, 0x44));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    let s = g::<f32>(VEHICLE_SPEED_FOR_CAM_COLLISION).read();
    f.fld(s);
    f.fmul_m(s);
    f.fcompp();
    let fast = !f.jp(0x05);
    if fast {
        g::<u32>(REL_VEL_CAM_COLLISION_VEHICLES_SQR).write(0x3F80_0000);
        CAM_COLLIDE_WITH_VEHICLES.set(GBool::TRUE);
        CAM_COLLIDE_WITH_PEDS.set(GBool::FALSE);
        CAM_COLLIDE_WITH_OBJECTS.set(GBool::FALSE);
    } else {
        g::<u32>(REL_VEL_CAM_COLLISION_VEHICLES_SQR).write(0x3C23_D70B);
        CAM_COLLIDE_WITH_VEHICLES.set(GBool::TRUE);
        CAM_COLLIDE_WITH_PEDS.set(GBool::TRUE);
        CAM_COLLIDE_WITH_OBJECTS.set(GBool::TRUE);
    }
    let e = rd::<GPtr<u8>>(veh, 0x4C8);
    if !e.is_null() {
        add_extra_entity(this, e);
    }
}

/// `CCamera::SetCameraUpForMirror()` (0x51A560): `preMirrorMat = m_mCameraMatrix` (+0x974),
/// `m_mCameraMatrix = m_mMatMirror` (+0xADC), `CopyCameraMatrixToRWCam(true)`,
/// `CalculateDerivedValues(true, false)`.
#[hook(addr = 0x51A560, conv = thiscall, name = "CCamera::SetCameraUpForMirror", expect = "56 8B F1 57 8D BE 74 09 00 00", this = "CCamera")]
pub fn set_camera_up_for_mirror(this: GPtr<u8>) {
    let cam_m = this.byte_add(0x974);
    matrix_assign(g::<u8>(PRE_MIRROR_MAT), cam_m);
    matrix_assign(cam_m, this.byte_add(0xADC));
    copy_camera_matrix_to_rw_cam(this, 1);
    calculate_derived_values(this, 1, 0);
}

/// `CCamera::RestoreCameraAfterMirror()` (0x51A5A0): `SetMatrix(preMirrorMat)`,
/// `CopyCameraMatrixToRWCam(true)`, `CalculateDerivedValues(false, false)`.
#[hook(addr = 0x51A5A0, conv = thiscall, name = "CCamera::RestoreCameraAfterMirror", expect = "56 68 40 FE B6 00", this = "CCamera")]
pub fn restore_camera_after_mirror(this: GPtr<u8>) {
    placeable_set_matrix(this, g::<u8>(PRE_MIRROR_MAT));
    copy_camera_matrix_to_rw_cam(this, 1);
    calculate_derived_values(this, 0, 0);
}

/// `CCamera::GetScreenRect(CRect&)` (0x50AB50): left 0, right = width; without widescreen (+0x3D) the
/// full height, with it the borders `(H/2) * m_fScreenReductionPercentage (+0x144) * 0.01` less
/// `H * (1/448) * 22` (top, +4) and `H - that` less `H * (1/448) * 14` (bottom, +0xC), as the x87
/// sequence computes them (`H/2` by `cdq; sub; sar`).
#[hook(addr = 0x50AB50, conv = thiscall, name = "CCamera::GetScreenRect", expect = "56 8B 74 24 08", this = "CCamera", preserve = "ecx")]
pub fn get_screen_rect(this: GPtr<u8>, rect: GPtr<u8>) {
    wr::<u32>(rect, 0, 0);
    let mut f = Fpu::new();
    f.fild(g::<i32>(SCREEN_WIDTH).read());
    wr::<f32>(rect, 8, f.fstp());
    if rd::<u8>(this, 0x3D) == 0 {
        wr::<u32>(rect, 4, 0);
        f.fild(g::<i32>(SCREEN_HEIGHT).read());
        wr::<f32>(rect, 0xC, f.fstp());
        return;
    }
    let h = g::<i32>(SCREEN_HEIGHT).read();
    f.fild(h / 2);
    f.fmul_m(rd::<f32>(this, 0x144));
    f.fmul_m(k(0x3C23_D70A)); // 0x858C58: 0.01
    f.fild(h);
    f.fmul_m(k(0x3B12_4925)); // 0x859524: 1/448
    f.fmul_m(k(0x41B0_0000)); // 0x858F8C: 22.0
    f.fsubp(1);
    wr::<f32>(rect, 4, f.fstp());
    let h = g::<i32>(SCREEN_HEIGHT).read();
    f.fild(h);
    f.fild(h / 2);
    f.fmul_m(rd::<f32>(this, 0x144));
    f.fmul_m(k(0x3C23_D70A));
    f.fsubr_st(1);
    f.fxch(1);
    f.fmul_m(k(0x3B12_4925));
    f.fmul_m(k(0x4160_0000)); // 0x859030: 14.0
    f.fsubp(1);
    wr::<f32>(rect, 0xC, f.fstp());
}

/// `CCamera::SetZoomValueCamStringScript(int16)` (0x50C1B0): when cam 0's target entity (+0x390) is not
/// a vehicle (type bits of +0x36 != 2), `SetZoomValueFollowPedScript`; else the zoom of the vehicle
/// appearance's row (`GetArrPosForVehicleType(GetVehicleAppearance())`, index 0 when that fails) from
/// the tables 0x8CC3E0 / 0x8CC3F4 / 0x8CC408 (zoom 0 / 1 / 2) into +0xC4; `m_bUseScriptZoomValueCar`
/// (+0x39) = 1.
#[hook(addr = 0x50C1B0, conv = thiscall, name = "CCamera::SetZoomValueCamStringScript", expect = "51 56 8B F1 8B 8E 90 03 00 00", this = "CCamera")]
pub fn set_zoom_value_cam_string_script(this: GPtr<u8>, zoom: u16) {
    let e = rd::<GPtr<u8>>(this, 0x390);
    if rd::<u8>(e, 0x36) & 7 != 2 {
        set_zoom_value_follow_ped_script_call(this, zoom);
        return;
    }
    let mut pos = [0i32; 1];
    let pp = GPtr::from_raw(pos.as_mut_ptr());
    let app = get_vehicle_appearance(e);
    get_arr_pos_for_vehicle_type_call(this, app, pp);
    let table = match zoom {
        0 => Some(0x8CC3E0),
        1 => Some(0x8CC3F4),
        2 => Some(0x8CC408),
        _ => None,
    };
    if let Some(t) = table {
        let v = g::<u32>(t + 4 * pos[0] as usize).read();
        wr::<u32>(this, 0xC4, v);
    }
    wr::<u8>(this, 0x39, 1);
}

/// `CCamera::TakeControl(CEntity*, eCamMode, eSwitchType, int32 whoIsInControl)` (0x50C7C0).
/// The script (2) cannot take control from the player (1) (`+0xB0`, compared in the protection chunk at
/// 0x408005) unless `+0xCF0` is set. A null target becomes the player's vehicle (`FindPlayerVehicle(-1,
/// false)`, called twice as the bytes do) or else the focused player's ped. With a target and mode 0,
/// the mode comes from the **current** target's (+0x958) entity type (3 = ped: 4, 2 = vehicle: 0x12;
/// the original reads the old target, not the new one). The target reference is moved to the new
/// entity, then mode, switch type, flags; the trailing `FindPlayerPed(-1)` result is unused.
#[hook(addr = 0x50C7C0, conv = thiscall, name = "CCamera::TakeControl", expect = "56 8B F1 8B 4C 24 14", this = "CCamera")]
pub fn take_control(this: GPtr<u8>, target: GPtr<u8>, mode: u16, switch_type: u16, who: i32) {
    let allowed = !(who == 2 && rd::<i32>(this, 0xB0) == 1);
    if rd::<u8>(this, 0xCF0) == 0 && !allowed {
        return;
    }
    wr::<i32>(this, 0xB0, who);
    let mut mode = mode;
    let mut target = target;
    if target.is_null() {
        if !find_player_vehicle(-1, 0).is_null() {
            target = find_player_vehicle(-1, 0);
        } else {
            let focus = usize::from(PLAYER_IN_FOCUS.get());
            target = g::<GPtr<u8>>(PLAYERS + focus * PLAYER_INFO_SIZE).read();
        }
    } else if mode == 0 {
        match rd::<u8>(rd::<GPtr<u8>>(this, 0x958), 0x36) & 7 {
            3 => mode = 4,
            2 => mode = 0x12,
            _ => {}
        }
    }
    let slot = this.byte_add(0x958);
    let old = rd::<GPtr<u8>>(this, 0x958);
    wr::<u8>(this, 0x2C, 0);
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(this, 0x958, target);
    register_reference(target, slot);
    wr::<u16>(this, 0xC38, mode);
    wr::<u16>(this, 0xC3C, switch_type);
    wr::<u8>(this, 0x2B, 0);
    wr::<u8>(this, 0x32, 1);
    find_player_ped(-1);
}

/// `CCamera::StoreValuesDuringInterPol(CVector& source, CVector& target, CVector& up, float& fov)`
/// (0x50C290): the four values to +0x914 / +0x920 / +0x92C / +0xEC (dword copies), then with
/// `d = source - stored target` (f32 locals): `+0x104 = GetATanOfXY(d.x, d.y)` and
/// `+0x100 = GetATanOfXY(sqrt(d.y*d.y + d.x*d.x), d.z)` (the sum and root on the x87, stored as f32
/// into the argument slot). The FPATAN results are stored through f64 as in `store_ped_orient`.
#[hook(addr = 0x50C290, conv = thiscall, name = "CCamera::StoreValuesDuringInterPol", expect = "83 EC 0C 53 56", this = "CCamera")]
pub fn store_values_during_inter_pol(
    this: GPtr<u8>,
    source: GPtr<u8>,
    target: GPtr<u8>,
    up: GPtr<u8>,
    fov: GPtr<u8>,
) {
    copy_vec(this, 0x914, source);
    copy_vec(this, 0x920, target);
    copy_vec(this, 0x92C, up);
    wr::<u32>(this, 0xEC, rd::<u32>(fov, 0));
    let d = |i: isize| {
        x(rd::<f32>(source, 4 * i))
            .fsub_m(rd::<f32>(this, 0x920 + 4 * i))
            .to_f32()
    };
    let (dx, dy, dz) = (d(0), d(1), d(2));
    let a = get_atan_of_xy(dx, dy);
    wr::<f32>(this, 0x104, a as f32);
    let mut f = Fpu::new();
    f.fld(dy);
    f.fmul_m(dy);
    f.fld(dx);
    f.fmul_m(dx);
    f.faddp(1);
    f.fsqrt();
    let r = f.fstp();
    let b = get_atan_of_xy(r, dz);
    wr::<f32>(this, 0x100, b as f32);
}

/// `CCamera::CalculateDerivedValues(bool bForMirror, bool bOriented)` (0x5150E0):
/// `m_mMatInverse` (+0xA4C) = `Invert(m_mCameraMatrix)` (through a temporary CMatrix and its
/// destructor, as the bytes), `CalculateFrustumPlanes(bForMirror)`, then from the camera matrix's
/// up.x/up.y (+0x984 / +0x988): both 0.0 (`test ah,0x44`: -0.0 too, NaN not) sets up.x to 1e-4 (and
/// skips the orientation); else with `bOriented`, `m_fOrientation` (+0x150) = FPATAN(up.x, up.y)
/// (f64 model, stored as f32). Finally `m_fCamFrontXNorm/YNorm` (+0xDC/+0xE0) = up.x/up.y normalised
/// on the x87 (1.0 / 0 path when the length is 0: only +0xDC is set to 1.0 after the plain copies).
///
/// Ported but **not hooked**: `Invert-2` builds its result in a stack local whose +0xC pad (the
/// RwMatrix `flags` word, which `RwV3dTransformPoints` tests for the identity flag) it never writes, so
/// `m_mMatInverse.flags` is uninitialised stack data of the caller's frame. The original's value comes
/// from its own stack history (L2 pause-restart: orig 4, Rust run another value, 1000/1000 cases
/// differ only there). Reproducing it needs a stack-faithful call or a normaliser decision.
pub fn calculate_derived_values_port(this: GPtr<u8>, for_mirror: u8, oriented: u8) {
    let mut tmp = [0u32; 18];
    let t = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    let r = matrix_invert(t, this.byte_add(0x974));
    matrix_assign(this.byte_add(0xA4C), r);
    matrix_dtor(t);
    calculate_frustum_planes(this, for_mirror);
    let zero = x(k(0x0000_0000)); // 0x858B50
    let ux = x(rd::<f32>(this, 0x984));
    let uy = x(rd::<f32>(this, 0x988));
    if !ux.fcom(zero).test_parity(0x44) && !uy.fcom(zero).test_parity(0x44) {
        wr::<u32>(this, 0x984, 0x38D1_B717);
    } else if oriented != 0 {
        let a = x(rd::<f32>(this, 0x984)).fpatan(x(rd::<f32>(this, 0x988)));
        wr::<f32>(this, 0x150, a.to_f32());
    }
    let mut f = Fpu::new();
    f.fld(rd::<f32>(this, 0x984));
    wr::<f32>(this, 0xDC, f.fst());
    f.fld(rd::<f32>(this, 0x988));
    wr::<f32>(this, 0xE0, f.fst());
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    f.fcom_m(k(0x0000_0000));
    if !f.jp(0x44) {
        f.fstp_st(0);
        wr::<u32>(this, 0xDC, 0x3F80_0000);
    } else {
        f.fdivr_m(k(0x3F80_0000)); // 0x858624: 1.0
        f.fld_st(0);
        f.fmul_st(3);
        wr::<f32>(this, 0xDC, f.fstp());
        f.fmul_st(1);
        wr::<f32>(this, 0xE0, f.fstp());
    }
}

/// The camera position the original reads: `m_matrix ? m_matrix->pos : placement.pos` (+0x14 / +0x4).
fn cam_pos(this: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        this.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CCamera::CalculateFrustumPlanes(bool bForMirror)` (0x514D60).
/// Half angle `a = CDraw::ms_fFOV * 0.0087263891 (0x8631D8)` (x87), `c = (f32)fcos(a)`,
/// `s = (f32)fsin(a)`, `r = fild(H) / fild(W)` (`fidiv`, not stored before the products):
/// normals (+0xB24..) N0 = (c, -s, 0), N1 = (-c, -s, 0), N2 = (0, -(f32 r * s), -(r * c)),
/// N3 = (0, -(f32 r * s), r * c), transformed by the camera matrix (`TransformVectors` 0x54EEA0, 4
/// vectors) into +0xB54 (+0xB84 for the mirror); then each plane's distance
/// `(n.y*p.y + n.z*p.z) + n.x*p.x` against the camera position into +0xBB4.. (+0xBC4.. for the mirror).
/// FSIN/FCOS use sa-math's f64 model (53 bits, then stored as f32).
#[hook(addr = 0x514D60, conv = thiscall, name = "CCamera::CalculateFrustumPlanes", expect = "83 EC 18 D9 05 38 50 8D 00", this = "CCamera")]
pub fn calculate_frustum_planes_port(this: GPtr<u8>, for_mirror: u8) {
    let a = x(g::<f32>(DRAW_FOV).read()).fmul_m(k(0x3C0E_F921));
    let c = a.fcos().to_f32();
    let s = a.fsin().to_f32();
    let n = |i: isize, v: [f32; 3]| {
        for (j, e) in v.iter().enumerate() {
            wr::<f32>(this, 0xB24 + 0xC * i + 4 * j as isize, *e);
        }
    };
    n(0, [c, x(s).fchs().to_f32(), 0.0]);
    n(1, [x(c).fchs().to_f32(), x(s).fchs().to_f32(), 0.0]);
    let r = xi(g::<i32>(SCREEN_HEIGHT).read()).fdiv(xi(g::<i32>(SCREEN_WIDTH).read()));
    let rc = r.fmul_m(c);
    let rs = x(r.to_f32()).fmul_m(s).fchs().to_f32();
    n(2, [0.0, rs, rc.fchs().to_f32()]);
    n(3, [0.0, rs, rc.to_f32()]);
    let (out, d) = if for_mirror == 0 {
        (0xB54, 0xBB4)
    } else {
        (0xB84, 0xBC4)
    };
    transform_vectors(this.byte_add(out), 4, this.byte_add(0x974), this.byte_add(0xB24));
    for i in 0..4 {
        let v = |j: isize| rd::<f32>(this, out + 0xC * i + 4 * j);
        let p = cam_pos(this);
        let mut f = Fpu::new();
        f.fld(v(1));
        f.fmul_m(rd::<f32>(p, 4));
        f.fld(v(2));
        f.fmul_m(rd::<f32>(p, 8));
        f.faddp(1);
        f.fld(v(0));
        f.fmul_m(rd::<f32>(p, 0));
        f.faddp(1);
        wr::<f32>(this, d + 4 * i, f.fstp());
    }
}

/// Phase B path of the `calculate_ground_height` call (ST0 as f64).
pub fn calculate_ground_height_st0(this: GPtr<u8>, kind: i32) -> f64 {
    calculate_ground_height_port(this, kind).to_f64()
}

/// `CCamera::CalculateGroundHeight(eGroundHeightType)` (0x514B80, ST0). When the camera moved more than
/// 20.0 (0x858BA4) on an axis since the last probe (`|old - p| > 20`, x, y, z in turn; NaN does not
/// count), a vertical line from `(p.x, p.y, 1000.0)` down by 1000 (buildings and dummies) gives: type 1
/// = the colpoint z; type 2 = the hit entity's z + its col model's max.z; type 0 = the colpoint z when
/// the bounding box is wider than 120.0 (0x858BB0) in x or y, else the entity's z + min.z, clamped to
/// >= 0.0 (`test ah,5; jp`: NaN kept). The old position is updated whenever a probe ran.
#[hook(addr = 0x514B80, conv = thiscall, name = "CCamera::CalculateGroundHeight", expect = "A0 40 00 B7 00", this = "CCamera")]
pub fn calculate_ground_height_port(this: GPtr<u8>, kind: i32) -> X87 {
    if GROUND_HEIGHT_GUARD.get() & 1 == 0 {
        GROUND_HEIGHT_GUARD.set(GROUND_HEIGHT_GUARD.get() | 1);
        GROUND_HEIGHT_OLD_POS.set([0; 3]);
    }
    let pos = cam_pos(this);
    let mut p = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
    let old = GROUND_HEIGHT_OLD_POS.get();
    let far = |i: usize| {
        let mut f = Fpu::new();
        f.fld(f32::from_bits(old[i]));
        f.fsub_m(f32::from_bits(p[i]));
        f.fabs();
        jne_greater(f.pop(), k(0x41A0_0000))
    };
    if far(0) || far(1) || far(2) {
        let mut cp = [0u32; 0x2C / 4];
        let mut ent = [GPtr::<u8>::NULL; 1];
        p[2] = 0x447A_0000; // 1000.0
        let hit = process_vertical_line(
            GPtr::from_raw(p.as_mut_ptr()).cast(),
            k(0xC47A_0000),
            GPtr::from_raw(cp.as_mut_ptr()).cast(),
            GPtr::from_raw(ent.as_mut_ptr()).cast(),
            1,
            0,
            0,
            0,
            1,
            0,
            GPtr::NULL,
        );
        if hit != 0 {
            GROUND_HEIGHT_ROUGH.ptr().cast::<u32>().write(cp[2]);
            let e = ent[0];
            let epos = cam_pos(e);
            let mi = sa_core::pools::MODEL_INFO_PTRS
                .ptr()
                .cast::<GPtr<u8>>()
                .wrapping_index(i32::from(rd::<i16>(e, 0x22)))
                .read();
            let cm = rd::<GPtr<u8>>(mi, 0x14);
            GROUND_HEIGHT_TOP.set(x(rd::<f32>(cm, 0x14)).fadd_m(rd::<f32>(epos, 8)).to_f32());
            let wide = |hi: isize, lo: isize| {
                jne_greater(x(rd::<f32>(cm, hi)).fsub_m(rd::<f32>(cm, lo)), k(0x42F0_0000))
            };
            if wide(0xC, 0) || wide(0x10, 4) {
                GROUND_HEIGHT_LOWEST.ptr().cast::<u32>().write(cp[2]);
            } else {
                let epos = cam_pos(e);
                GROUND_HEIGHT_LOWEST.set(x(rd::<f32>(epos, 8)).fadd_m(rd::<f32>(cm, 8)).to_f32());
            }
            if !x(GROUND_HEIGHT_LOWEST.get()).fcom(x(k(0))).test_parity(0x05) {
                GROUND_HEIGHT_LOWEST.set(0.0);
            }
        }
        let pos = cam_pos(this);
        GROUND_HEIGHT_OLD_POS.set([rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)]);
    }
    match kind {
        2 => x(GROUND_HEIGHT_TOP.get()),
        1 => x(GROUND_HEIGHT_ROUGH.get()),
        _ => x(GROUND_HEIGHT_LOWEST.get()),
    }
}

/// `fcomp m; test ah, 0x41; je` taken: `a > m` (not equal, not less, not unordered).
fn jne_greater(a: X87, m: f32) -> bool {
    !a.fcom(x(m)).test_any(0x41)
}

/// `CCamera::ConeCastCollisionResolve(const CVector& source, const CVector& target, CVector& out,
/// float radius, float angle, float& dist)` (0x51A5D0, AL): equal source and target (`test ah,0x44`,
/// per axis; NaN counts as different) give 0 with nothing written. Otherwise two spheres of `radius`
/// (at target, then at source) go to `CameraConeCastVsWorldCollision(s_target, s_source, dist, angle)`;
/// a hit gives `out = target + (source - target) * dist` in the original's x87 order (dz and dx*t stored
/// as f32 first) and its AL; a miss gives `out = source` (dword copies), `dist = 1.0` and 0.
#[hook(addr = 0x51A5D0, conv = thiscall, name = "CCamera::ConeCastCollisionResolve", expect = "83 EC 28 56 8B 74 24 34", this = "CCamera")]
pub fn cone_cast_collision_resolve(
    _this: GPtr<u8>,
    source: GPtr<u8>,
    target: GPtr<u8>,
    out: GPtr<u8>,
    radius: f32,
    angle: f32,
    dist: GPtr<u8>,
) -> u8 {
    let v = |p: GPtr<u8>, i: isize| rd::<f32>(p, 4 * i);
    if (0..3).all(|i| !x(v(source, i)).fcom(x(v(target, i))).test_parity(0x44)) {
        return 0;
    }
    let mut s1 = [0u32; 5];
    let mut s2 = [0u32; 5];
    let p1 = GPtr::from_raw(s1.as_mut_ptr()).cast::<u8>();
    let p2 = GPtr::from_raw(s2.as_mut_ptr()).cast::<u8>();
    col_sphere_set(p1, radius, target, 0, 0, 0xFF);
    col_sphere_set(p2, radius, source, 0, 0, 0xFF);
    let hit = camera_cone_cast_vs_world_collision(p1, p2, dist, angle);
    if hit == 0 {
        copy_vec(out, 0, source);
        wr::<u32>(dist, 0, 0x3F80_0000);
        return 0;
    }
    let t = rd::<f32>(dist, 0);
    let mut f = Fpu::new();
    f.fld(v(source, 0));
    f.fsub_m(v(target, 0));
    f.fld(v(source, 1));
    f.fsub_m(v(target, 1));
    f.fld(v(source, 2));
    f.fsub_m(v(target, 2));
    let dz = f.fstp();
    f.fxch(1);
    f.fmul_m(t);
    let dxt = f.fstp();
    f.fmul_m(t);
    f.fld(dz);
    f.fmul_m(t);
    f.fld(dxt);
    f.fadd_m(v(target, 0));
    let ox = f.fstp();
    f.fxch(1);
    f.fadd_m(v(target, 1));
    let oy = f.fstp();
    f.fadd_m(v(target, 2));
    wr::<f32>(out, 0, ox);
    wr::<f32>(out, 4, oy);
    let oz = f.fstp();
    wr::<f32>(out, 8, oz);
    hit
}

/// The focused player's ped (`CWorld::Players[CWorld::PlayerInFocus].m_pPed`).
fn focus_ped() -> GPtr<u8> {
    let focus = usize::from(PLAYER_IN_FOCUS.get());
    g::<GPtr<u8>>(PLAYERS + focus * PLAYER_INFO_SIZE).read()
}

/// The target hand-back `Restore` and `RestoreWithJumpCut` share (0x50B969.. / 0x50BAF6..).
fn restore_target(this: GPtr<u8>) {
    let slot = this.byte_add(0x958);
    let veh = find_player_vehicle(-1, 0);
    let old = rd::<GPtr<u8>>(this, 0x958);
    let target = if veh.is_null() {
        wr::<u16>(this, 0xC38, 4);
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        let p = focus_ped();
        wr::<GPtr<u8>>(this, 0x958, p);
        p
    } else {
        wr::<u16>(this, 0xC38, 0x12);
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        let v = find_player_vehicle(-1, 0);
        wr::<GPtr<u8>>(this, 0x958, v);
        v
    };
    register_reference(target, slot);
    if rd::<u32>(focus_ped(), 0x530) == 0x3B {
        wr::<u16>(this, 0xC38, 0x12);
    }
    let st = rd::<u32>(focus_ped(), 0x530);
    if st == 0x39 || st == 0x35 {
        wr::<u16>(this, 0xC38, 0x12);
    }
    if rd::<u32>(focus_ped(), 0x530) == 0x3D {
        let old = rd::<GPtr<u8>>(this, 0x958);
        wr::<u16>(this, 0xC38, 4);
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        let p = focus_ped();
        wr::<GPtr<u8>>(this, 0x958, p);
        register_reference(p, slot);
    }
}

/// `CCamera::Restore()` (0x50B930): control back to the player. Target = the player's vehicle in
/// MODE_CAM_ON_A_STRING (0x12), else the focused ped in MODE_FOLLOWPED (4) (references moved); the
/// focused ped's state (+0x530) 0x3B, 0x39 or 0x35 forces mode 0x12, state 0x3D forces mode 4 with the
/// ped as target. The script's target (+0x95C) reference is dropped; the flags as the bytes set them.
#[hook(addr = 0x50B930, conv = thiscall, name = "CCamera::Restore", expect = "53 33 DB 56 8B F1", this = "CCamera")]
pub fn restore(this: GPtr<u8>) {
    wr::<u8>(this, 0x2B, 1);
    wr::<u8>(this, 0x2C, 0);
    wr::<u16>(this, 0xC3C, 1);
    wr::<u8>(this, 0x31, 0);
    wr::<u32>(this, 0xC34, 0x1E);
    wr::<u32>(this, 0x140, 0);
    wr::<u8>(this, 0x33, 0);
    wr::<u8>(this, 0x50, 0);
    wr::<u32>(this, 0xB0, 0);
    restore_target(this);
    let sslot = this.byte_add(0x95C);
    let st = rd::<GPtr<u8>>(this, 0x95C);
    if !st.is_null() {
        clean_up_old_reference(st, sslot);
        wr::<GPtr<u8>>(this, 0x95C, GPtr::NULL);
    }
    wr::<u8>(this, 0x41, 0);
    wr::<u8>(this, 0x42, 0);
    wr::<u8>(this, 0x38, 0);
    wr::<u8>(this, 0x39, 0);
    wr::<u32>(this, 0x164, 0);
    wr::<u8>(this, 0x32, 1);
    wr::<u8>(this, 0x1C, 1);
}

/// `CCamera::RestoreWithJumpCut()` (0x50BAB0): as `Restore` with a jump cut (switch type 2, +0x30 set);
/// in cooperative mode (+0x43) with both players' peds: both in the same vehicle -> mode +0x48 (shooting
/// allowed, +0x44) or +0x4A; different vehicles -> +0x46; otherwise +0x4C with player 0's ped as the
/// target (player 0's vehicle in the vehicle cases).
#[hook(addr = 0x50BAB0, conv = thiscall, name = "CCamera::RestoreWithJumpCut", expect = "53 33 DB 56 8B F1", this = "CCamera")]
pub fn restore_with_jump_cut(this: GPtr<u8>) {
    wr::<u8>(this, 0x30, 1);
    wr::<u8>(this, 0x2B, 1);
    wr::<u8>(this, 0x2C, 0);
    wr::<u16>(this, 0xC3C, 2);
    wr::<u32>(this, 0xB0, 0);
    wr::<u32>(this, 0x140, 0);
    wr::<u8>(this, 0x33, 0);
    wr::<u8>(this, 0x31, 0);
    wr::<u32>(this, 0xC34, 0x1E);
    wr::<u8>(this, 0x50, 0);
    wr::<u8>(this, 0x1C, 1);
    wr::<u8>(this, 0x41, 0);
    wr::<u8>(this, 0x42, 0);
    restore_target(this);
    let slot = this.byte_add(0x958);
    let p0 = || g::<GPtr<u8>>(PLAYERS).read();
    let p1 = || g::<GPtr<u8>>(PLAYERS + PLAYER_INFO_SIZE).read();
    if rd::<u8>(this, 0x43) != 0 && !p0().is_null() && !p1().is_null() {
        let old = rd::<GPtr<u8>>(this, 0x958);
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        let (a, b) = (p0(), p1());
        let (va, vb) = (rd::<GPtr<u8>>(a, 0x58C), rd::<GPtr<u8>>(b, 0x58C));
        let both_in = rd::<u32>(a, 0x46C) & 0x100 != 0 && rd::<u32>(b, 0x46C) & 0x100 != 0;
        let target = if both_in && !va.is_null() && !vb.is_null() {
            let mode = if va != vb {
                rd::<u16>(this, 0x46)
            } else if rd::<u8>(this, 0x44) != 0 {
                rd::<u16>(this, 0x48)
            } else {
                rd::<u16>(this, 0x4A)
            };
            wr::<u16>(this, 0xC38, mode);
            rd::<GPtr<u8>>(p0(), 0x58C)
        } else {
            wr::<u16>(this, 0xC38, rd::<u16>(this, 0x4C));
            p0()
        };
        wr::<GPtr<u8>>(this, 0x958, target);
        register_reference(target, slot);
    }
    wr::<u8>(this, 0x38, 0);
    wr::<u8>(this, 0x39, 0);
}

/// `CCamera::UpdateSoundDistances()` (0x515BD0). A point ahead of the camera (`pos + fwd * k`, fwd =
/// the matrix row at +0x984, k = 0.5 in the 1st-person / aiming modes with a ped target, else 5.0;
/// stored as f32); every 12th frame (`CTimer::m_FrameCounter % 12 == 0`) the previous ground distance
/// (+0x15C) moves to +0x160 and a vertical line (buildings and dummies) to `z + 20` gives the new one
/// (`colpoint.z - z`, 20.0 when nothing is hit). +0x158 = `(1 - t) * prev + t * new` with
/// `t = (frame % 12 + 1) * 0.16666667` (0x85F0A0; reaches 2.0, as the bytes).
#[hook(addr = 0x515BD0, conv = thiscall, name = "CCamera::UpdateSoundDistances", expect = "83 EC 48 56 8B F1", this = "CCamera")]
pub fn update_sound_distances(this: GPtr<u8>) {
    let mode = rd::<u16>(active_cam(this), 0xC);
    let aiming = matches!(
        mode,
        0x10 | 7 | 0x27 | 0x28 | 0x34 | 0x2A | 0x2B | 0x29 | 0x2D | 0x2E | 0x22 | 8 | 0x33
    );
    let ped_target = aiming && rd::<u8>(rd::<GPtr<u8>>(this, 0x958), 0x36) & 7 == 3;
    let kf = if ped_target {
        k(0x3F00_0000)
    } else {
        k(0x40A0_0000)
    }; // 0x858B8C 0.5 / 0x858C80 5.0
    let mut f = Fpu::new();
    f.fld(rd::<f32>(this, 0x984));
    f.fmul_m(kf);
    f.fld(rd::<f32>(this, 0x988));
    f.fmul_m(kf);
    f.fld(rd::<f32>(this, 0x98C));
    f.fmul_m(kf);
    let pos = cam_pos(this);
    let fz = f.fstp();
    f.fxch(1);
    f.fadd_m(rd::<f32>(pos, 0));
    let px = f.fstp();
    f.fadd_m(rd::<f32>(pos, 4));
    let py = f.fstp();
    f.fld(fz);
    f.fadd_m(rd::<f32>(pos, 8));
    let pz = f.fstp();
    let mut v = [px, py, pz];
    let n = (g::<u32>(FRAME_COUNTER).read() % 12) as u16;
    if n == 0 {
        let top = x(pz).fadd_m(k(0x41A0_0000)).to_f32(); // 0x858BA4 20.0
        wr::<u32>(this, 0x160, rd::<u32>(this, 0x15C));
        let mut cp = [0u32; 0x2C / 4];
        let mut ent = [GPtr::<u8>::NULL; 1];
        let hit = process_vertical_line(
            GPtr::from_raw(v.as_mut_ptr()).cast(),
            top,
            GPtr::from_raw(cp.as_mut_ptr()).cast(),
            GPtr::from_raw(ent.as_mut_ptr()).cast(),
            1,
            0,
            0,
            0,
            1,
            0,
            GPtr::NULL,
        );
        if hit != 0 {
            wr::<f32>(this, 0x15C, x(f32::from_bits(cp[2])).fsub_m(pz).to_f32());
        } else {
            wr::<u32>(this, 0x15C, 0x41A0_0000);
        }
    }
    let mut f = Fpu::new();
    f.fild(i32::from(n as i16) + 1);
    f.fmul_m(k(0x3E2A_AAAB));
    f.fld(k(0x3F80_0000));
    f.fsub_st(1);
    f.fmul_m(rd::<f32>(this, 0x160));
    f.fxch(1);
    f.fmul_m(rd::<f32>(this, 0x15C));
    f.faddp(1);
    wr::<f32>(this, 0x158, f.fstp());
}

/// `CCamera::CCamera()` (0x51A450, catalogue `CCamera::Constructor`; returns `this`): CPlaceable base,
/// the CCamera vtable (0x8630E8), the three CCams (eh vector ctor: CCam ctor 0x517730 / dtor 0x5097D0),
/// the four CCamPathSplines (0x5097B0 / 0x5097C0), the six CMatrix attach fields (+0x974.. +0x40/+0x44,
/// stride 0x48) zeroed, the five CQueuedMode-ish 0x10 entries at +0xCF4 (0x50E470 / 0x509800), then
/// +0xCA0 = 1, +0x53 = 0, `m_matrix` (+0x14) = &m_mCameraMatrix (+0x974), +0xC54 = +0xC58 = 0 and
/// `InitialiseScriptableComponents()`. (The SEH frame of the original only covers the vector ctors.)
#[hook(addr = 0x51A450, conv = thiscall, name = "CCamera::Constructor", expect = "6A FF 68 57 C8 83 00", this = "CCamera")]
pub fn camera_constructor(this: GPtr<u8>) -> GPtr<u8> {
    placeable_ctor(this);
    wr::<u32>(this, 0, 0x0086_30E8);
    eh_vector_ctor(this.byte_add(0x174), 0x238, 3, 0x0051_7730, 0x0050_97D0);
    eh_vector_ctor(this.byte_add(0x960), 4, 4, 0x0050_97B0, 0x0050_97C0);
    for off in [
        0x974 + 0x40,
        0x974 + 0x44,
        0x9FC,
        0xA00,
        0xA44,
        0xA48,
        0xA8C,
        0xA90,
        0xAD4,
        0xAD8,
        0xB1C,
        0xB20,
    ] {
        wr::<u32>(this, off, 0);
    }
    eh_vector_ctor(this.byte_add(0xCF4), 0x10, 5, 0x0050_E470, 0x0050_9800);
    wr::<u32>(this, 0xCA0, 1);
    wr::<u8>(this, 0x53, 0);
    wr::<GPtr<u8>>(this, 0x14, this.byte_add(0x974));
    wr::<u32>(this, 0xC54, 0);
    wr::<u32>(this, 0xC58, 0);
    initialise_scriptable_components(this);
    this
}

/// `|prev - cur|^2` of a row as the x87 sums it: `(dz*dz + dy*dy) + dx*dx` (d = prev - cur, memory
/// operands), left on the stack of `f`.
fn row_dist2(f: &mut Fpu, prev: [u32; 3], cur: GPtr<u8>) {
    for (i, p) in prev.iter().enumerate() {
        f.fld(f32::from_bits(*p));
        f.fsub_m(rd::<f32>(cur, 4 * i as isize));
    }
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
}

/// `CCamera::CopyCameraMatrixToRWCam(bool bUpdateMatrix)` (0x50AFA0): the RwFrame matrix of
/// `m_pRwCamera` gets (unless the argument is set, after `m_mViewMatrix.UpdateMatrix(rw)`... the
/// CMatrix at +0x9BC) the camera matrix rows (right +0x974 -> right, forward +0x984 -> at, up +0x994 ->
/// up, pos +0x9A4 -> pos; dword copies). Each row that moved less than the threshold since the last call
/// is snapped back to the previous one: the position when `0.00001^2 (0x8CCC7C) > |d|^2` (`test
/// ah,0x41`), the axes when `|d|^2 < (f32)(0.00001^2) (0x8CCC78)` (`test ah,5`), d = prev - cur. The
/// (snapped) rows become the new previous rows (statics 0xB6FF90..), then `RwMatrixUpdate`,
/// `RwFrameUpdateObjects`, `RwFrameOrthoNormalize`; with `+0x971` set and no argument, +0x9BC's
/// `UpdateMatrix(rw)` again and +0x971 cleared.
#[hook(addr = 0x50AFA0, conv = thiscall, name = "CCamera::CopyCameraMatrixToRWCam", expect = "83 EC 14 56 57", this = "CCamera")]
pub fn copy_camera_matrix_to_rw_cam_port(this: GPtr<u8>, update: u8) {
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x954), 4);
    let m = frame.byte_add(0x10);
    if update == 0 {
        matrix_update_matrix(this.byte_add(0x9BC), m);
    }
    copy_vec(m, 0x30, this.byte_add(0x9A4));
    copy_vec(m, 0x20, this.byte_add(0x984));
    copy_vec(m, 0x10, this.byte_add(0x994));
    copy_vec(m, 0x00, this.byte_add(0x974));
    let mut guard = PREV_RW_ROWS_GUARD.get();
    let mut prev = PREV_RW_ROWS.get();
    // init order: pos (bit 1), at (2), up (4), right (8)
    for (bit, base) in [(1u32, 9usize), (2, 6), (4, 3), (8, 0)] {
        if guard & bit == 0 {
            guard |= bit;
            PREV_RW_ROWS_GUARD.set(guard);
            for j in 0..3 {
                prev[base + j] = 0xC7C3_4F80;
                PREV_RW_ROWS
                    .ptr()
                    .cast::<u32>()
                    .wrapping_index((base + j) as i32)
                    .write(0xC7C3_4F80);
            }
        }
    }
    let row = |i: usize| [prev[i], prev[i + 1], prev[i + 2]];
    // position: k^2 > |d|^2 -> snap
    let mut f = Fpu::new();
    row_dist2(&mut f, row(9), m.byte_add(0x30));
    let kp = g::<f32>(CAM_POS_SNAP).read();
    f.fld(kp);
    f.fmul_m(kp);
    f.fcompp();
    let snap = !f.jne(0x41);
    if snap {
        for j in 0..3 {
            wr::<u32>(m, 0x30 + 4 * j as isize, prev[9 + j]);
        }
    }
    let ka = g::<f32>(CAM_AXIS_SNAP).read();
    let t2 = x(ka).fmul_m(ka).to_f32();
    for (base, off) in [(6usize, 0x20isize), (3, 0x10), (0, 0)] {
        let mut f = Fpu::new();
        row_dist2(&mut f, row(base), m.byte_add(off));
        f.fcomp_m(t2);
        if !f.jp(0x05) {
            for j in 0..3 {
                wr::<u32>(m, off + 4 * j as isize, prev[base + j]);
            }
        }
    }
    let mut now = [0u32; 12];
    for (base, off) in [(9usize, 0x30isize), (6, 0x20), (3, 0x10), (0, 0)] {
        for j in 0..3 {
            now[base + j] = rd::<u32>(m, off + 4 * j as isize);
        }
    }
    PREV_RW_ROWS.set(now);
    rw_matrix_update(m);
    rw_frame_update_objects(frame);
    rw_frame_ortho_normalize(frame);
    if rd::<u8>(this, 0x971) != 0 && update == 0 {
        matrix_update_matrix(this.byte_add(0x9BC), m);
        wr::<u8>(this, 0x971, 0);
    }
}

/// The `mov [esi+off], imm/reg` field stores of `CCamera::Init`, in byte order: (offset, size, value).
type Store = (isize, u8, u32);

fn apply(this: GPtr<u8>, stores: &[Store]) {
    for &(off, size, v) in stores {
        match size {
            1 => wr::<u8>(this, off, v as u8),
            2 => wr::<u16>(this, off, v as u16),
            _ => wr::<u32>(this, off, v),
        }
    }
}

/// `CCamera::Init()` (0x5BC520). Zeroes `this + 0x18 .. + 0xD78` (`rep stos`, 0x358 dwords; the count
/// is loaded in the protection chunk at 0x406E9A), `m_pRwCamera` = null, then (through `TheCamera`
/// 0xB6F028, not `this`) `InitialiseScriptableComponents`, the three `CCam::Init`s, and the defaults in
/// the original's store order: cam 0/1 MODE_FOLLOWPED, cam 0 +0x100 = 2.0837801 - 1.85 (0x86A46C /
/// 0x86A468, x87), the follow-car tweak table, `ClearPlayerWeaponMode`, the target entity (player vehicle
/// or the focused ped, reference registered), the fade state unless the menu's "reinit" byte 0xBA67A5
/// is set (gr's FIX_BUGS mouse-accel line is not in the bytes: `m_fMouseAccelVertical` 0xB6EC18 =
/// 0.0015 as stored), `m_mCameraMatrix.SetScale(1.0)`, `SetMotionBlur(0xFF, 0xFF, 0xFF, 0, 0)`, the
/// 3rd-person cross-hair multipliers 0xB6EC14 / 0xB6EC10 and 0x8CC380 = 1, +0x971 = 1.
#[hook(addr = 0x5BC520, conv = thiscall, name = "CCamera::Init", expect = "83 EC 0C 53 55", this = "CCamera")]
pub fn camera_init(this: GPtr<u8>) {
    for i in 0..0x358 {
        wr::<u32>(this, 0x18 + 4 * i, 0);
    }
    wr::<u32>(this, 0x954, 0);
    let tc = the_camera();
    initialise_scriptable_components(tc);
    apply(this, &[(0x27, 1, 0), (0x3E, 1, 0), (0x140, 4, 0), (0x1C, 1, 0)]);
    cam_init(this.byte_add(0x174));
    cam_init(this.byte_add(0x3AC));
    cam_init(this.byte_add(0x5E4));
    let d = x(k(0x4005_5CA7)).fsub_m(k(0x3FEC_CCCD)); // 0x86A46C - 0x86A468
    wr::<u16>(this, 0x180, 4);
    wr::<u16>(this, 0x3B8, 4);
    wr::<f32>(this, 0x274, d.to_f32());
    apply(
        this,
        &[
            (0x41, 1, 0),
            (0x42, 1, 0),
            (0x43, 1, 0),
            (0x44, 1, 1),
            (0x40, 1, 0),
            (0x270, 4, 0x3FEC_CCCD),
            (0x250, 4, 0xBF0C_CCCD),
            (0x254, 4, 0x3FC0_0000),
            (0x258, 4, 0x4066_6666),
            (0x25C, 4, 0x3D75_C28F),
            (0x260, 4, 0xBDCC_CCCD),
            (0x264, 4, 0),
            (0x268, 4, 0xBD8F_5C29),
            (0x26C, 4, 0x3F66_8113),
            (0x2D, 1, 0),
        ],
    );
    clear_player_weapon_mode(this);
    apply(
        this,
        &[
            (0x22, 1, 0),
            (0x390, 4, 0),
            (0x5C8, 4, 0),
            (0x800, 4, 0),
            (0x1C8, 4, 0),
            (0x1CC, 4, 0),
            (0x400, 4, 0),
            (0x404, 4, 0),
            (0x176, 1, 0),
            (0x3AE, 1, 0),
            (0x5E6, 1, 0),
            (0x39C, 4, 0),
            (0x5D4, 4, 0),
            (0x80C, 4, 0),
            (0x3F, 1, 0),
            (0xC10, 4, 0x3E80_0000),
            (0xC14, 4, 0x3F40_0000),
            (0xC20, 4, 0x3F59_999A),
            (0x50, 1, 0),
            (0x5C, 4, 0),
            (0x12C, 4, 0),
            (0xC34, 4, 0x1E),
            (0x25, 1, 0),
            (0x3A, 1, 0),
            (0x81C, 4, 0),
            (0x820, 4, 0),
            (0x26, 1, 0),
            (0x28, 1, 0),
            (0x138, 4, 0x3F66_6666),
            (0x31, 1, 0),
            (0x4F, 1, 0),
            (0x18, 1, 0),
            (0x19, 1, 0),
            (0x46, 2, 0x32),
            (0x48, 2, 0x31),
            (0x4A, 2, 3),
            (0x4C, 2, 0x30),
            (0x3D, 1, 0),
            (0x134, 4, 0),
            (0x30, 1, 0),
            (0xB4, 4, 2),
            (0xC8, 4, 2),
            (0xC0, 4, 0),
            (0xBC, 4, 0),
            (0xB8, 4, 0),
            (0xD4, 4, 0),
            (0xD0, 4, 0),
            (0xCC, 4, 0),
            (0x958, 4, 0),
        ],
    );
    let target = if find_player_vehicle(-1, 0).is_null() {
        focus_ped()
    } else {
        find_player_vehicle(-1, 0)
    };
    wr::<GPtr<u8>>(this, 0x958, target);
    if !target.is_null() {
        register_reference(target, this.byte_add(0x958));
    }
    apply(
        this,
        &[
            (0x23, 1, 0),
            (0x144, 4, 0),
            (0x148, 4, 0),
            (0x3D, 1, 0),
            (0x3C, 1, 0),
            (0x56, 1, 0),
            (0x154, 4, 0x3F80_0000),
            (0x120, 4, 0),
        ],
    );
    if g::<u8>(MENU_REINIT).read() == 0 {
        wr::<u8>(this, 0x51, 0);
        g::<u8>(0xC3EFAB).write(0);
        wr::<u32>(this, 0xBFC, 0);
        wr::<u8>(this, 0x52, 0);
        wr::<u32>(this, 0xC08, 0);
        wr::<u32>(this, 0xC00, 0);
        g::<u32>(0xB6EC18).write(0x3AC4_9BA6);
    }
    if g::<u8>(MENU_REINIT).read() == 1 {
        wr::<u32>(this, 0xC08, 0);
    }
    apply(
        this,
        &[
            (0x33, 1, 0),
            (0xC3C, 2, 1),
            (0x38, 1, 0),
            (0x39, 1, 0),
            (0xD8, 4, 0),
            (0xC4, 4, 0),
            (0x36, 1, 0),
            (0x130, 4, 0x428C_0000),
            (0xC38, 2, 4),
            (0x2A, 1, 0),
            (0x37, 1, 0),
        ],
    );
    matrix_set_scale(this.byte_add(0x974), 1.0);
    apply(
        this,
        &[
            (0x34, 1, 0),
            (0x24, 1, 0),
            (0x64, 4, 0x1388),
            (0x6C, 4, 0),
            (0x21, 1, 0),
            (0x70, 4, 0),
            (0x74, 4, 0),
            (0xF0, 4, 0x3F80_0000),
            (0x1A, 1, 0),
            (0x1B, 1, 0),
            (0xA0, 4, 0),
            (0x4E, 1, 0),
        ],
    );
    set_motion_blur_call(this, 0xFF, 0xFF, 0xFF, 0, 0);
    apply(
        this,
        &[
            (0x1E, 1, 0),
            (0x54, 1, 0),
            (0xA8, 4, 6),
            (0xAC, 4, 0),
            (0x124, 4, 0),
            (0x128, 4, 0),
            (0x83C, 4, 0),
            (0x840, 4, 0),
            (0x844, 4, 0),
            (0x94, 4, 4),
            (0x98, 4, 0),
            (0x29, 1, 1),
            (0x58, 1, 0),
            (0x78, 4, 0),
            (0x2B, 1, 1),
            (0x2C, 1, 0),
        ],
    );
    g::<u32>(0xB6EC14).write(0x3F07_AE14);
    g::<u32>(0xB6EC10).write(0x3ECC_CCCD);
    wr::<u32>(this, 0x164, 0);
    wr::<u16>(this, 0x168, 0);
    g::<u8>(0x8CC380).write(1);
    wr::<u8>(this, 0x971, 1);
}

/// `CCamera::TakeControlAttachToEntity(CEntity* target, CEntity* attached, CVector& offset,
/// CVector& lookAt, float tilt, eSwitchType, int32 whoIsInControl)` (0x50C910): ignored when the script
/// (2) asks while the player (1) has control. A null `attached` becomes the player's vehicle or the
/// focused ped. With a target: the target reference moves to it, +0x2C = 0; without: +0x2C = 1 and
/// `m_vecAttachedCamLookAt` (+0x944) = lookAt when `lookAt != offset` (`operator!=` 0x509760, called
/// as the bytes do), else zero. `m_vecAttachedCamOffset` (+0x938) = offset unless it is (0, 0, 0)
/// (`test ah,0x44`; NaN counts as non-zero), then (0, 0, 2.0). Tilt to +0x950 (dword), the attached
/// entity reference (+0x95C), MODE 0x2F, the switch type, flags, and the unused `FindPlayerPed(-1)`.
#[hook(addr = 0x50C910, conv = thiscall, name = "CCamera::TakeControlAttachToEntity", expect = "8B 44 24 1C 83 EC 0C", this = "CCamera")]
#[allow(clippy::too_many_arguments)]
pub fn take_control_attach_to_entity(
    this: GPtr<u8>,
    target: GPtr<u8>,
    attached: GPtr<u8>,
    offset: GPtr<u8>,
    look_at: GPtr<u8>,
    tilt: u32,
    switch_type: u16,
    who: i32,
) {
    if who == 2 && rd::<i32>(this, 0xB0) == 1 {
        return;
    }
    wr::<i32>(this, 0xB0, who);
    let mut attached = attached;
    if attached.is_null() {
        attached = if find_player_vehicle(-1, 0).is_null() {
            focus_ped()
        } else {
            find_player_vehicle(-1, 0)
        };
    }
    if !target.is_null() {
        let slot = this.byte_add(0x958);
        let old = rd::<GPtr<u8>>(this, 0x958);
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        wr::<GPtr<u8>>(this, 0x958, target);
        register_reference(target, slot);
        wr::<u8>(this, 0x2C, 0);
    } else {
        wr::<u8>(this, 0x2C, 1);
        if vector_ne(look_at, offset) != 0 {
            copy_vec(this, 0x944, look_at);
        } else {
            for i in 0..3 {
                wr::<u32>(this, 0x944 + 4 * i, 0);
            }
        }
    }
    let zero = x(k(0));
    let nonzero = (0..3).any(|i| x(rd::<f32>(offset, 4 * i)).fcom(zero).test_parity(0x44));
    if nonzero {
        copy_vec(this, 0x938, offset);
    } else {
        wr::<u32>(this, 0x938, 0);
        wr::<u32>(this, 0x93C, 0);
        wr::<u32>(this, 0x940, 0x4000_0000);
    }
    wr::<u32>(this, 0x950, tilt);
    let slot = this.byte_add(0x95C);
    let old = rd::<GPtr<u8>>(this, 0x95C);
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(this, 0x95C, attached);
    register_reference(attached, slot);
    wr::<u16>(this, 0xC38, 0x2F);
    wr::<u16>(this, 0xC3C, switch_type);
    wr::<u8>(this, 0x2B, 0);
    wr::<u8>(this, 0x32, 1);
    find_player_ped(-1);
}

/// `CCamera::CalculateMirroredMatrix(CVector n, float d, CMatrix* in, CMatrix* out)` (0x50B380, `ret
/// 0x18`; `this` unused): reflects `in` in the plane `n . p = d`, transcribed instruction by instruction
/// (which products are stored as f32 locals and which stay on the x87 stack): pos' = pos - n * 2(n.pos
/// - d), up' = up - n * 2(n.up), at' = at - n * 2(n.at) (the dot products summed z, y, x), then
/// right' = up' x at' in the original's operand order (`(at.y*up.z - at.z*up.y, at.z*up.x - at.x*up.z,
/// at.x*up.y - at.y*up.x)`, with up.x and at.x/at.y as stored f32 and up.y/up.z from the stack).
#[hook(addr = 0x50B380, conv = thiscall, name = "CCamera::CalculateMirroredMatrix", expect = "83 EC 24 8B 4C 24 38", this = "CCamera")]
pub fn calculate_mirrored_matrix(
    _this: GPtr<u8>,
    nx: f32,
    ny: f32,
    nz: f32,
    d: f32,
    inp: GPtr<u8>,
    out: GPtr<u8>,
) {
    let m = |o: isize| rd::<f32>(inp, o);
    let (px, py, pz) = (m(0x30), m(0x34), m(0x38));
    let mut f = Fpu::new();
    // position
    f.fld(nz);
    f.fmul_m(pz);
    f.fld(ny);
    f.fmul_m(py);
    f.faddp(1);
    f.fld(nx);
    f.fmul_m(px);
    f.faddp(1);
    f.fsub_m(d);
    f.fadd_st(0);
    f.fld(nx);
    f.fmul_st(1);
    f.fld(ny);
    f.fmul_st(2);
    let l10 = f.fstp();
    f.fld(nz);
    f.fmul_st(2);
    let l14 = f.fstp();
    f.fld(px);
    f.fsub_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    f.fld(py);
    f.fsub_m(l10);
    f.fld(pz);
    f.fsub_m(l14);
    f.fxch(2);
    wr::<f32>(out, 0x30, f.fstp());
    wr::<f32>(out, 0x34, f.fstp());
    wr::<f32>(out, 0x38, f.fstp());
    // up
    f.fld(m(0x10));
    f.fld(m(0x14));
    f.fld(m(0x18));
    let uz = f.fst();
    f.fmul_m(nz);
    f.fld_st(1);
    f.fmul_m(ny);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(nx);
    f.faddp(1);
    f.fadd_st(0);
    f.fld(nx);
    f.fmul_st(1);
    f.fld(ny);
    f.fmul_st(2);
    let l10 = f.fstp();
    f.fld(nz);
    f.fmul_st(2);
    let l14 = f.fstp();
    f.fxch(3);
    f.fsub_st(3);
    let rx = f.fstp();
    f.fstp_st(2);
    f.fstp_st(1);
    f.fsub_m(l10);
    f.fld(uz);
    f.fsub_m(l14);
    f.fld_st(1);
    wr::<f32>(out, 0x10, rx);
    wr::<f32>(out, 0x14, f.fstp());
    wr::<f32>(out, 0x18, f.fst());
    // at
    f.fld(m(0x20));
    f.fld(m(0x24));
    f.fld(m(0x28));
    let az = f.fst();
    f.fmul_m(nz);
    f.fld_st(1);
    f.fmul_m(ny);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(nx);
    f.faddp(1);
    f.fadd_st(0);
    f.fld(nx);
    f.fmul_st(1);
    f.fld(ny);
    f.fmul_st(2);
    let l10 = f.fstp();
    f.fld(nz);
    f.fmul_st(2);
    let l14 = f.fstp();
    f.fxch(3);
    f.fsub_st(3);
    let atx = f.fstp();
    wr::<f32>(out, 0x20, atx);
    f.fstp_st(2);
    f.fstp_st(1);
    f.fsub_m(l10);
    let aty = f.fstp();
    f.fld(az);
    f.fsub_m(l14);
    wr::<f32>(out, 0x24, aty);
    wr::<f32>(out, 0x28, f.fst());
    // right = up x at
    f.fld(aty);
    f.fmul_st(2);
    f.fld_st(1);
    f.fmul_st(4);
    f.fsubp(1);
    wr::<f32>(out, 0, f.fstp());
    f.fmul_m(rx);
    f.fld(atx);
    f.fmul_st(2);
    f.fsubp(1);
    wr::<f32>(out, 4, f.fstp());
    f.fstp_st(0);
    f.fld(atx);
    f.fmul_st(1);
    f.fld(aty);
    f.fmul_m(rx);
    f.fsubp(1);
    wr::<f32>(out, 8, f.fstp());
    f.fstp_st(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_CAMERA_DIRECTLY_BEHIND_FOR_FOLLOW_PED_CAM_ON_A_STRING,
    HOOK_SET_CAMERA_DIRECTLY_IN_FRONT_FOR_FOLLOW_PED_CAM_ON_A_STRING,
    HOOK_SET_CAMERA_DIRECTLY_BEHIND_FOR_FOLLOW_PED_FOR_A_PED_CAM_ON_A_STRING,
    HOOK_SET_CAMERA_DIRECTLY_IN_FRONT_FOR_FOLLOW_PED_FOR_A_PED_CAM_ON_A_STRING,
    HOOK_MOTION_BLUR_OPEN,
    HOOK_SET_RW_CAMERA,
    HOOK_RENDER_MOTION_BLUR,
    HOOK_IS_SPHERE_VISIBLE,
    HOOK_PROCESS_SCRIPTED_COMMANDS,
    HOOK_GET_ROUGH_DISTANCE_TO_GROUND,
    HOOK_FINISH_CUTSCENE,
    HOOK_DELETE_CUT_SCENE_CAM_DATA_MEMORY,
    HOOK_TAKE_CONTROL_NO_ENTITY,
    HOOK_GET_ARR_POS_FOR_VEHICLE_TYPE,
    HOOK_SET_PERCENT_ALONG_CUT_SCENE,
    HOOK_CAMERA_VEHICLE_MODE_SPECIAL_CASES,
    HOOK_SET_CAMERA_UP_FOR_MIRROR,
    HOOK_RESTORE_CAMERA_AFTER_MIRROR,
    HOOK_GET_SCREEN_RECT,
    HOOK_SET_ZOOM_VALUE_CAM_STRING_SCRIPT,
    HOOK_TAKE_CONTROL,
    HOOK_STORE_VALUES_DURING_INTER_POL,
    HOOK_CALCULATE_FRUSTUM_PLANES_PORT,
    HOOK_CALCULATE_GROUND_HEIGHT_PORT,
    HOOK_CONE_CAST_COLLISION_RESOLVE,
    HOOK_RESTORE,
    HOOK_RESTORE_WITH_JUMP_CUT,
    HOOK_UPDATE_SOUND_DISTANCES,
    HOOK_CAMERA_CONSTRUCTOR,
    HOOK_COPY_CAMERA_MATRIX_TO_RW_CAM_PORT,
    HOOK_CAMERA_INIT,
    HOOK_TAKE_CONTROL_ATTACH_TO_ENTITY,
    HOOK_CALCULATE_MIRRORED_MATRIX,
];
