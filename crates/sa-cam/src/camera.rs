//! PORT-CAM: `CCamera` getters, setters and small state changes (thiscall, `this` = `TheCamera`
//! 0xB6F028). Ported from the 1.0 US bytes; gta-reversed `Camera.cpp` and
//! `docs/re/specs/CAM/CCamera.md` as reading aids. Fields are reached through `this` (sa-core and
//! sa-world declare a few cells of `TheCamera` as globals already).

use sa_core::abi::HookReg;
use sa_core::mem::{GBool, GPtr};
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::collision::misc::CAM_COLLIDE_WITH_PEDS;

use crate::common::{active_cam, cam_of, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CCamera::bDidWeProcessAnyCinemaCam`
    pub static DID_WE_PROCESS_ANY_CINEMA_CAM @ 0xB6EC2D: GBool = GBool::FALSE;
    /// `CGameLogic::n2PlayerPedInFocus` (eFocusedPlayer, default NONE = 2)
    pub static TWO_PLAYER_PED_IN_FOCUS @ 0x8A5E50: i32 = 2;
    /// `CCollision::bCamCollideWithVehicles`
    pub static CAM_COLLIDE_WITH_VEHICLES @ 0x8A5B14: GBool = GBool::TRUE;
    /// `CCollision::bCamCollideWithObjects`
    pub static CAM_COLLIDE_WITH_OBJECTS @ 0x8A5B15: GBool = GBool::TRUE;
}

orig! {
    /// `CCamera::StartTransition(eCamMode)` (0x515200)
    pub fn start_transition(this: GPtr<u8>, mode: u16) = thiscall 0x515200 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    pub fn get_task_hold(intel: GPtr<u8>, ignore_check: u8) -> GPtr<u8> = thiscall 0x600FF0 => unported;
}

/// `CCam::m_nMode` (int16, cam +0xC).
const CAM_MODE: isize = 0xC;
/// `CCam::m_nDirectionWasLooking` (int32, cam +0x28).
const CAM_DIRECTION: isize = 0x28;

/// `CCamera::Enable1rstPersonCamCntrlsScript()` (0x50AC00).
#[hook(addr = 0x50AC00, conv = thiscall, name = "CCamera::Enable1rstPersonCamCntrlsScript", expect = "C6 41 41 01 C3", this = "CCamera")]
pub fn enable_1rst_person_cam_cntrls_script(this: GPtr<u8>) {
    wr::<u8>(this, 0x41, 1);
}

/// `CCamera::Enable1rstPersonWeaponsCamera()` (0x50AC10).
#[hook(addr = 0x50AC10, conv = thiscall, name = "CCamera::Enable1rstPersonWeaponsCamera", expect = "C6 41 42 01 C3", this = "CCamera")]
pub fn enable_1rst_person_weapons_camera(this: GPtr<u8>) {
    wr::<u8>(this, 0x42, 1);
}

/// `CCamera::GetGameCamPosition()` (0x50AE50): `&m_vecGameCamPos` (+0x908).
#[hook(addr = 0x50AE50, conv = thiscall, name = "CCamera::GetGameCamPosition", expect = "8D 81 08 09 00 00", this = "CCamera")]
pub fn get_game_cam_position(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x908)
}

/// `CCamera::GetPositionAlongSpline()` (0x50AF80): `fld m_fPositionAlongSpline` (+0x140).
#[hook(addr = 0x50AF80, conv = thiscall, name = "CCamera::GetPositionAlongSpline", expect = "D9 81 40 01 00 00", this = "CCamera")]
pub fn get_position_along_spline(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x140))
}

/// `CCamera::DontProcessObbeCinemaCamera()` (0x50AB40).
#[hook(addr = 0x50AB40, conv = thiscall, name = "CCamera::DontProcessObbeCinemaCamera", expect = "C6 05 2D EC B6 00 00", this = "CCamera")]
pub fn dont_process_obbe_cinema_camera(_this: GPtr<u8>) {
    DID_WE_PROCESS_ANY_CINEMA_CAM.set(GBool::FALSE);
}

/// `CCamera::SetWideScreenOn()` (0x50C140).
#[hook(addr = 0x50C140, conv = thiscall, name = "CCamera::SetWideScreenOn", expect = "C6 41 3D 01 C6 41 3C 00", this = "CCamera")]
pub fn set_wide_screen_on(this: GPtr<u8>) {
    wr::<u8>(this, 0x3D, 1);
    wr::<u8>(this, 0x3C, 0);
}

/// `CCamera::SetWideScreenOff()` (0x50C150): `+0x3C = (+0x3D == 1)`.
#[hook(addr = 0x50C150, conv = thiscall, name = "CCamera::SetWideScreenOff", expect = "80 79 3D 01 0F 94 C0", this = "CCamera")]
pub fn set_wide_screen_off(this: GPtr<u8>) {
    let v = u8::from(rd::<u8>(this, 0x3D) == 1);
    wr::<u8>(this, 0x3C, v);
}

/// `CCamera::StartTransitionWhenNotFinishedInter(eCamMode)` (0x515BC0): `+0x4F = 1`, then a tail jump
/// to `StartTransition` with the same argument.
#[hook(addr = 0x515BC0, conv = thiscall, name = "CCamera::StartTransitionWhenNotFinishedInter", expect = "C6 41 4F 01 E9 37 F6 FF FF", this = "CCamera")]
pub fn start_transition_when_not_finished_inter(this: GPtr<u8>, mode: u16) {
    wr::<u8>(this, 0x4F, 1);
    start_transition(this, mode);
}

/// `CCamera::AllowShootingWith2PlayersInCar(bool)` (0x50C280).
#[hook(addr = 0x50C280, conv = thiscall, name = "CCamera::AllowShootingWith2PlayersInCar", expect = "8A 44 24 04 88 41 44", this = "CCamera")]
pub fn allow_shooting_with_2_players_in_car(this: GPtr<u8>, allow: u8) {
    wr::<u8>(this, 0x44, allow);
}

/// `CCamera::SetMotionBlurAlpha(int32)` (0x50BF80).
#[hook(addr = 0x50BF80, conv = thiscall, name = "CCamera::SetMotionBlurAlpha", expect = "8B 44 24 04 89 81 A4 00 00 00", this = "CCamera")]
pub fn set_motion_blur_alpha(this: GPtr<u8>, alpha: i32) {
    wr::<i32>(this, 0xA4, alpha);
}

/// `CCamera::StartCooperativeCamMode()` (0x50C260).
#[hook(addr = 0x50C260, conv = thiscall, name = "CCamera::StartCooperativeCamMode", expect = "C6 41 43 01 C7 05 50 5E 8A 00 02 00 00 00", this = "CCamera")]
pub fn start_cooperative_cam_mode(this: GPtr<u8>) {
    wr::<u8>(this, 0x43, 1);
    TWO_PLAYER_PED_IN_FOCUS.set(2);
}

/// `CCamera::StopCooperativeCamMode()` (0x50C270).
#[hook(addr = 0x50C270, conv = thiscall, name = "CCamera::StopCooperativeCamMode", expect = "C6 41 43 00 C7 05 50 5E 8A 00 02 00 00 00", this = "CCamera")]
pub fn stop_cooperative_cam_mode(this: GPtr<u8>) {
    wr::<u8>(this, 0x43, 0);
    TWO_PLAYER_PED_IN_FOCUS.set(2);
}

/// `CCamera::SetNearClipScript(float)` (0x50BF90): the float is copied as a dword.
#[hook(addr = 0x50BF90, conv = thiscall, name = "CCamera::SetNearClipScript", expect = "8B 44 24 04 89 81 38 01 00 00", this = "CCamera")]
pub fn set_near_clip_script(this: GPtr<u8>, clip: u32) {
    wr::<u32>(this, 0x138, clip);
    wr::<u8>(this, 0x31, 1);
}

/// `CCamera::FindCamFOV()` (0x50AD20): the active cam's `m_fFOV` (cam +0xB4).
#[hook(addr = 0x50AD20, conv = thiscall, name = "CCamera::FindCamFOV", expect = "0F B6 41 59 69 C0 38 02 00 00", this = "CCamera")]
pub fn find_cam_fov(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(active_cam(this), 0xB4))
}

/// `CCamera::CameraPedModeSpecialCases()` (0x50CD80, `ret 4`: one unused stack argument): the camera
/// collides with vehicles, objects and peds.
#[hook(addr = 0x50CD80, conv = thiscall, name = "CCamera::CameraPedModeSpecialCases", expect = "B0 01 A2 14 5B 8A 00", this = "CCamera")]
pub fn camera_ped_mode_special_cases(_this: GPtr<u8>, _unused: u32) {
    CAM_COLLIDE_WITH_VEHICLES.set(GBool::TRUE);
    CAM_COLLIDE_WITH_OBJECTS.set(GBool::TRUE);
    CAM_COLLIDE_WITH_PEDS.set(GBool::TRUE);
}

/// `CCamera::GetFadingDirection()` (0x50ADF0): 2 when not fading, else `m_nFadeInOutFlag == 1`.
#[hook(addr = 0x50ADF0, conv = thiscall, name = "CCamera::GetFadingDirection", expect = "8A 41 51 84 C0", this = "CCamera", preserve = "edx")]
pub fn get_fading_direction(this: GPtr<u8>) -> i32 {
    if rd::<u8>(this, 0x51) == 0 {
        return 2;
    }
    i32::from(rd::<u16>(this, 0xC30) == 1)
}

/// `CCamera::SetCamCutSceneOffSet(const CVector&)` (0x50BD20): three dwords to +0x884.
#[hook(addr = 0x50BD20, conv = thiscall, name = "CCamera::SetCamCutSceneOffSet", expect = "8B 44 24 04 8B 10", this = "CCamera")]
pub fn set_cam_cut_scene_offset(this: GPtr<u8>, v: GPtr<u8>) {
    copy_vec(this, 0x884, v);
}

/// `CCamera::UpdateAimingCoors(const CVector&)` (0x50CB10): three dwords to +0x854.
#[hook(addr = 0x50CB10, conv = thiscall, name = "CCamera::UpdateAimingCoors", expect = "8B 44 24 04 8B 10", this = "CCamera")]
pub fn update_aiming_coors(this: GPtr<u8>, v: GPtr<u8>) {
    copy_vec(this, 0x854, v);
}

/// The `mov edx,[eax]; mov [ecx],edx; ...` copy of a CVector as three dwords (x, y, z order).
fn copy_vec(dst: GPtr<u8>, off: isize, src: GPtr<u8>) {
    for i in 0..3 {
        let d = rd::<u32>(src, 4 * i);
        wr::<u32>(dst, off + 4 * i, d);
    }
}

/// `CCamera::ClearPlayerWeaponMode()` (0x50AB10).
#[hook(addr = 0x50AB10, conv = thiscall, name = "CCamera::ClearPlayerWeaponMode", expect = "33 C0 66 89 81 30 08 00 00", this = "CCamera")]
pub fn clear_player_weapon_mode(this: GPtr<u8>) {
    wr::<u16>(this, 0x830, 0);
    wr::<u16>(this, 0x83A, 1);
    wr::<u16>(this, 0x838, 0xFFFF);
    wr::<u32>(this, 0x834, 0);
}

/// `CCamera::SetNewPlayerWeaponMode(eCamMode, int16, int16)` (0x50BFB0).
#[hook(addr = 0x50BFB0, conv = thiscall, name = "CCamera::SetNewPlayerWeaponMode", expect = "66 8B 44 24 04", this = "CCamera")]
pub fn set_new_player_weapon_mode(this: GPtr<u8>, mode: u16, max_zoom: u16, min_zoom: u16) {
    wr::<u16>(this, 0x830, mode);
    wr::<u16>(this, 0x83A, min_zoom);
    wr::<u16>(this, 0x838, max_zoom);
    wr::<u32>(this, 0x834, 0);
}

/// `CCam::m_nMode` and `m_nDirectionWasLooking` of the active cam.
fn active_mode_dir(this: GPtr<u8>) -> (u16, i32) {
    let c = active_cam(this);
    (rd::<u16>(c, CAM_MODE), rd::<i32>(c, CAM_DIRECTION))
}

/// `CCamera::GetLookingLRBFirstPerson()` (0x50AE60): mode 16 (1st person) and not looking forward (3).
#[hook(addr = 0x50AE60, conv = thiscall, name = "CCamera::GetLookingLRBFirstPerson", expect = "0F B6 41 59 69 C0 38 02 00 00", this = "CCamera")]
pub fn get_looking_lrb_first_person(this: GPtr<u8>) -> u8 {
    let (mode, dir) = active_mode_dir(this);
    u8::from(mode == 0x10 && dir != 3)
}

/// `CCamera::GetLookingForwardFirstPerson()` (0x50AED0): mode 16 and looking forward (3).
#[hook(addr = 0x50AED0, conv = thiscall, name = "CCamera::GetLookingForwardFirstPerson", expect = "0F B6 41 59 69 C0 38 02 00 00", this = "CCamera")]
pub fn get_looking_forward_first_person(this: GPtr<u8>) -> u8 {
    let (mode, dir) = active_mode_dir(this);
    u8::from(mode == 0x10 && dir == 3)
}

/// `CCamera::GetLookDirection()` (0x50AE90): the active cam's direction in modes 0x12, 0x10, 0x16 and 4
/// unless it is 3; 3 otherwise.
#[hook(addr = 0x50AE90, conv = thiscall, name = "CCamera::GetLookDirection", expect = "0F B6 41 59 69 C0 38 02 00 00", this = "CCamera")]
pub fn get_look_direction(this: GPtr<u8>) -> i32 {
    let (mode, dir) = active_mode_dir(this);
    if matches!(mode, 0x12 | 0x10 | 0x16 | 4) && dir != 3 {
        dir
    } else {
        3
    }
}

/// `CCamera::TakeControlWithSpline(eSwitchType)` (0x50CAE0).
#[hook(addr = 0x50CAE0, conv = thiscall, name = "CCamera::TakeControlWithSpline", expect = "32 C0 88 41 2B", this = "CCamera")]
pub fn take_control_with_spline(this: GPtr<u8>, switch_type: u16) {
    wr::<u8>(this, 0x2B, 0);
    wr::<u8>(this, 0x2C, 0);
    wr::<u8>(this, 0x1D, 0);
    wr::<u16>(this, 0xC38, 0x11);
    wr::<u16>(this, 0xC3C, switch_type);
    wr::<u8>(this, 0x32, 1);
}

/// `CCamera::GetScreenFadeStatus()` (0x50AE20): 0 when `m_fFadeAlpha == 0.0` (`fcomp` with 0x858B50,
/// `test ah,0x44; jp`: -0.0 also counts as 0, NaN does not), 2 when its bits are 255.0, else 1.
#[hook(addr = 0x50AE20, conv = thiscall, name = "CCamera::GetScreenFadeStatus", expect = "D9 81 FC 0B 00 00", this = "CCamera")]
pub fn get_screen_fade_status(this: GPtr<u8>) -> i32 {
    let a = rd::<f32>(this, 0xBFC);
    if !x(a).fcom(x(k(0x0000_0000))).test_parity(0x44) {
        return 0;
    }
    if a.to_bits() == 0x437F_0000 { 2 } else { 1 }
}

/// `CCamera::IsExtraEntityToIgnore(CEntity*)` (0x50CE80): the entity is among the first
/// `m_nExtraEntitiesCount` (+0xC48) of `m_pExtraEntity[]` (+0xC4C).
#[hook(addr = 0x50CE80, conv = thiscall, name = "CCamera::IsExtraEntityToIgnore", expect = "8B 91 48 0C 00 00", this = "CCamera")]
pub fn is_extra_entity_to_ignore(this: GPtr<u8>, e: GPtr<u8>) -> u8 {
    let n = rd::<i32>(this, 0xC48);
    let mut i = 0;
    while i < n {
        if rd::<GPtr<u8>>(this, 0xC4C + 4 * i as isize) == e {
            return 1;
        }
        i += 1;
    }
    0
}

/// `CCamera::SetParametersForScriptInterpolation(float, float, int32)` (0x50C030): the two percentages
/// times 0.01 (0x858C58).
#[hook(addr = 0x50C030, conv = thiscall, name = "CCamera::SetParametersForScriptInterpolation", expect = "D9 44 24 04 8B 44 24 0C", this = "CCamera")]
pub fn set_parameters_for_script_interpolation(this: GPtr<u8>, stop_moving: f32, catch_up: f32, time: u32) {
    let a = x(stop_moving).fmul_m(k(0x3C23_D70A));
    wr::<u32>(this, 0xC2C, time);
    wr::<u8>(this, 0x50, 1);
    wr::<f32>(this, 0xC24, a.to_f32());
    let b = x(catch_up).fmul_m(k(0x3C23_D70A));
    wr::<f32>(this, 0xC28, b.to_f32());
}

/// `CCamera::SetMotionBlur(int32 r, int32 g, int32 b, int32 a, int32 type)` (0x50BF40).
#[hook(addr = 0x50BF40, conv = thiscall, name = "CCamera::SetMotionBlur", expect = "8B 44 24 04 8B 54 24 08", this = "CCamera")]
pub fn set_motion_blur(this: GPtr<u8>, r: i32, g: i32, b: i32, a: i32, kind: i32) {
    wr::<i32>(this, 0x8C, r);
    wr::<i32>(this, 0x88, g);
    wr::<i32>(this, 0x84, b);
    wr::<i32>(this, 0x90, kind);
    wr::<i32>(this, 0xA0, a);
}

/// `CCamera::Using1stPersonWeaponMode()` (0x50BFF0): the player weapon mode (+0x830) is one of
/// 7, 0x22, 8, 0x33, 0x2D, 0x2E, 0x41.
#[hook(addr = 0x50BFF0, conv = thiscall, name = "CCamera::Using1stPersonWeaponMode", expect = "66 8B 81 30 08 00 00", this = "CCamera")]
pub fn using_1st_person_weapon_mode(this: GPtr<u8>) -> u8 {
    u8::from(matches!(
        rd::<u16>(this, 0x830),
        7 | 0x22 | 8 | 0x33 | 0x2D | 0x2E | 0x41
    ))
}

/// `this->m_pExtraEntity[this->m_nExtraEntitiesCount++] = e` (the `mov [esi+ecx*4+0xC4C]; inc` pairs).
fn add_extra_entity(this: GPtr<u8>, e: GPtr<u8>) {
    let n = rd::<i32>(this, 0xC48);
    wr::<GPtr<u8>>(this, 0xC4C + (n as isize) * 4, e);
    wr::<i32>(this, 0xC48, rd::<i32>(this, 0xC48).wrapping_add(1));
}

/// `CCamera::CameraPedAimModeSpecialCases(CPed*)` (0x50CDA0): collide with everything; a ped in a
/// vehicle (`+0x46C` bit 8) adds its vehicle (`+0x58C`) to the extra entities.
#[hook(addr = 0x50CDA0, conv = thiscall, name = "CCamera::CameraPedAimModeSpecialCases", expect = "B0 01 A2 14 5B 8A 00", this = "CCamera")]
pub fn camera_ped_aim_mode_special_cases(this: GPtr<u8>, ped: GPtr<u8>) {
    CAM_COLLIDE_WITH_VEHICLES.set(GBool::TRUE);
    CAM_COLLIDE_WITH_OBJECTS.set(GBool::TRUE);
    CAM_COLLIDE_WITH_PEDS.set(GBool::TRUE);
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        return;
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if veh.is_null() {
        return;
    }
    add_extra_entity(this, veh);
}

/// `CCamera::CameraGenericModeSpecialCases(CPed*)` (0x50CD30): no extra entities; with a ped whose
/// `GetTaskHold(false)` task holds an entity (task +8), that entity is the one extra entity.
#[hook(addr = 0x50CD30, conv = thiscall, name = "CCamera::CameraGenericModeSpecialCases", expect = "8B 44 24 04 85 C0", this = "CCamera")]
pub fn camera_generic_mode_special_cases(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<i32>(this, 0xC48, 0);
    if ped.is_null() {
        return;
    }
    let task = get_task_hold(rd::<GPtr<u8>>(ped, 0x47C), 0);
    if task.is_null() {
        return;
    }
    let held = rd::<GPtr<u8>>(task, 8);
    if held.is_null() {
        return;
    }
    add_extra_entity(this, held);
}

/// `CCamera::GetCutSceneFinishTime()` (0x50AD90): the finish time (cam +0x10) of the active cam when it
/// is in MODE_FLYBY (0x11), else of the other cam (`(active + 1) % 2`, signed) when that one is, else 0.
#[hook(addr = 0x50AD90, conv = thiscall, name = "CCamera::GetCutSceneFinishTime", expect = "0F B6 41 59 8B D0", this = "CCamera")]
pub fn get_cut_scene_finish_time(this: GPtr<u8>) -> u32 {
    let active = i32::from(rd::<u8>(this, 0x59));
    let c = cam_of(this, active);
    if rd::<u16>(c, CAM_MODE) == 0x11 {
        return rd::<u32>(c, 0x10);
    }
    let c = cam_of(this, (active + 1) % 2);
    if rd::<u16>(c, CAM_MODE) == 0x11 {
        rd::<u32>(c, 0x10)
    } else {
        0
    }
}

/// `CCamera::SetCamPositionForFixedMode(const CVector& pos, const CVector& up)` (0x50BEC0): pos to
/// +0x86C (the address computed in the protection chunk at 0x401D1C), up to +0x878, then
/// `m_bGarageFixedCamPositionSet` (+0x4E) = false.
#[hook(addr = 0x50BEC0, conv = thiscall, name = "CCamera::SetCamPositionForFixedMode", expect = "8B 54 24 04 56", this = "CCamera")]
pub fn set_cam_position_for_fixed_mode(this: GPtr<u8>, pos: GPtr<u8>, up: GPtr<u8>) {
    copy_vec(this, 0x86C, pos);
    copy_vec(this, 0x878, up);
    wr::<u8>(this, 0x4E, 0);
}

/// `CCamera::SetZoomValueFollowPedScript(int16)` (0x50C160): zoom 0 / 1 / 2 set +0xD8 to 0.25 / 1.5 /
/// 2.9; any other value leaves it; `m_bUseScriptZoomValuePed` (+0x38) = 1 in every case.
#[hook(addr = 0x50C160, conv = thiscall, name = "CCamera::SetZoomValueFollowPedScript", expect = "66 8B 44 24 04", this = "CCamera")]
pub fn set_zoom_value_follow_ped_script(this: GPtr<u8>, zoom: u16) {
    match zoom {
        0 => wr::<u32>(this, 0xD8, 0x3E80_0000),
        1 => wr::<u32>(this, 0xD8, 0x3FC0_0000),
        2 => wr::<u32>(this, 0xD8, 0x4039_999A),
        _ => {}
    }
    wr::<u8>(this, 0x38, 1);
}

/// `CCamera::ProcessWideScreenOn()` (0x50B890): with `m_bWantsToSwitchWidescreenOff` (+0x3C) clear,
/// `m_fScreenReductionPercentage` (+0x134) = active FOV * 0.3 (0x858C24), +0x16C = 1.0, +0x144 = 30.0;
/// otherwise both widescreen flags and the three floats are cleared.
#[hook(addr = 0x50B890, conv = thiscall, name = "CCamera::ProcessWideScreenOn", expect = "8A 51 3C 33 C0", this = "CCamera")]
pub fn process_wide_screen_on(this: GPtr<u8>) {
    if rd::<u8>(this, 0x3C) == 0 {
        let v = x(rd::<f32>(active_cam(this), 0xB4)).fmul_m(k(0x3E99_999A));
        wr::<u32>(this, 0x16C, 0x3F80_0000);
        wr::<u32>(this, 0x144, 0x41F0_0000);
        wr::<f32>(this, 0x134, v.to_f32());
    } else {
        wr::<u8>(this, 0x3C, 0);
        wr::<u8>(this, 0x3D, 0);
        wr::<u32>(this, 0x144, 0);
        wr::<u32>(this, 0x134, 0);
        wr::<u32>(this, 0x16C, 0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENABLE_1RST_PERSON_CAM_CNTRLS_SCRIPT,
    HOOK_ENABLE_1RST_PERSON_WEAPONS_CAMERA,
    HOOK_GET_GAME_CAM_POSITION,
    HOOK_GET_POSITION_ALONG_SPLINE,
    HOOK_DONT_PROCESS_OBBE_CINEMA_CAMERA,
    HOOK_SET_WIDE_SCREEN_ON,
    HOOK_SET_WIDE_SCREEN_OFF,
    HOOK_START_TRANSITION_WHEN_NOT_FINISHED_INTER,
    HOOK_ALLOW_SHOOTING_WITH_2_PLAYERS_IN_CAR,
    HOOK_SET_MOTION_BLUR_ALPHA,
    HOOK_START_COOPERATIVE_CAM_MODE,
    HOOK_STOP_COOPERATIVE_CAM_MODE,
    HOOK_SET_NEAR_CLIP_SCRIPT,
    HOOK_FIND_CAM_FOV,
    HOOK_CAMERA_PED_MODE_SPECIAL_CASES,
    HOOK_GET_FADING_DIRECTION,
    HOOK_SET_CAM_CUT_SCENE_OFFSET,
    HOOK_UPDATE_AIMING_COORS,
    HOOK_CLEAR_PLAYER_WEAPON_MODE,
    HOOK_SET_NEW_PLAYER_WEAPON_MODE,
    HOOK_GET_LOOKING_LRB_FIRST_PERSON,
    HOOK_GET_LOOKING_FORWARD_FIRST_PERSON,
    HOOK_GET_LOOK_DIRECTION,
    HOOK_TAKE_CONTROL_WITH_SPLINE,
    HOOK_GET_SCREEN_FADE_STATUS,
    HOOK_IS_EXTRA_ENTITY_TO_IGNORE,
    HOOK_SET_PARAMETERS_FOR_SCRIPT_INTERPOLATION,
    HOOK_SET_MOTION_BLUR,
    HOOK_USING_1ST_PERSON_WEAPON_MODE,
    HOOK_CAMERA_PED_AIM_MODE_SPECIAL_CASES,
    HOOK_CAMERA_GENERIC_MODE_SPECIAL_CASES,
    HOOK_GET_CUT_SCENE_FINISH_TIME,
    HOOK_SET_CAM_POSITION_FOR_FIXED_MODE,
    HOOK_SET_ZOOM_VALUE_FOLLOW_PED_SCRIPT,
    HOOK_PROCESS_WIDE_SCREEN_ON,
];
