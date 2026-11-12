//! PORT-VEH-4 (batch `world4-2`, SA:MP-executed rows): `CEntryExit::TransitionStarted` and
//! `CEntryExit::TransitionFinished`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win: it uses `Magnitude`, not the squared
//! one, and goes through a door found near the entrance).
//!
//! `CEntryExit`: +8 entrance rectangle (left, bottom, right, top), +0x18 entrance z, +0x20 exit position,
//! +0x30 flags (u16), +0x34/+0x35 time on/off, +0x38 link.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{f32_of, g, rd, wr, x};

orig! {
    /// `CClock::GetIsTimeInRange(uint8 from, uint8 to)` (0x52CEE0, AL)
    fn clock_get_is_time_in_range(from: u32, to: u32) -> u8 = cdecl 0x52CEE0 => unported;
    /// `CPlayerPed::CanPlayerStartMission()` (0x609590, AL)
    fn can_player_start_mission(p: GPtr<u8>) -> u8 = thiscall 0x609590 => unported;
    /// `CVector::Magnitude()` (0x4082C0): ST0
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CEntryExitManager::FindNearestDoor(const CEntryExit&, float)` (0x43F630)
    fn find_nearest_door(enex: GPtr<u8>, range: f32) -> GPtr<u8> = cdecl 0x43F630 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexGotoDoorAndOpen::CTaskComplexGotoDoorAndOpen(CObject* door)` (0x66BB20)
    fn task_goto_door_ctor_door(t: GPtr<u8>, door: GPtr<u8>) -> GPtr<u8> = thiscall 0x66BB20 => unported;
    /// `CTaskComplexGotoDoorAndOpen::CTaskComplexGotoDoorAndOpen(const CVector& start, const CVector& end)` (0x66BBA0)
    fn task_goto_door_ctor_points(t: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x66BBA0 => unported;
    /// `CTaskSimpleUninterruptable::CTaskSimpleUninterruptable()` (0x43E2E0)
    fn task_uninterruptable_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x43E2E0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, the result through `out`)
    fn vec_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, the result through `out`)
    fn vec_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, the result through `out`)
    fn vec_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CCamera::SetCamPositionForFixedMode(const CVector&, const CVector&)` (0x50BEC0) on `TheCamera`
    fn set_cam_position_for_fixed_mode(cam: GPtr<u8>, pos: GPtr<u8>, up: GPtr<u8>) = thiscall 0x50BEC0 => unported;
    /// `CCamera::TakeControlNoEntity(const CVector&, eSwitchType, int32)` (0x50C8B0) on `TheCamera`
    fn take_control_no_entity(cam: GPtr<u8>, pos: GPtr<u8>, switch: u32, who: i32) = thiscall 0x50C8B0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPed::GetEntityThatThisPedIsHolding()` (0x5E02E0)
    fn ped_get_held_entity(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E02E0 => unported;
    /// `CColStore::AddCollisionNeededAtPosn(const CVector&)` (0x4103A0)
    fn col_store_add_collision_needed(p: GPtr<u8>) = cdecl 0x4103A0 => unported;
    /// `CIplStore::AddIplsNeededAtPosn(const CVector&)` (0x4045B0)
    fn ipl_store_add_ipls_needed(p: GPtr<u8>) = cdecl 0x4045B0 => unported;
    /// `CStreaming::AddModelsToRequestList(const CVector&, int32 flags)` (0x40D3F0)
    fn streaming_add_models_to_request_list(p: GPtr<u8>, flags: u32) = cdecl 0x40D3F0 => unported;
    /// `CEntryExit::RequestObjectsInFrustum()` (0x43E690)
    fn entry_exit_request_objects_in_frustum(e: GPtr<u8>) = thiscall 0x43E690 => unported;
    /// `CPathFind::SetPathsNeededAtPosition(const CVector&)` (0x44DCD0) on `ThePaths`
    fn paths_set_needed_at_position(paths: GPtr<u8>, p: GPtr<u8>) = thiscall 0x44DCD0 => unported;
    /// `CCamera::GetFading()` (0x50ADE0, AL)
    fn camera_get_fading(cam: GPtr<u8>) -> u8 = thiscall 0x50ADE0 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00)
    fn camera_set_fade_colour(cam: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16 direction)` (0x50AC20)
    fn camera_fade(cam: GPtr<u8>, t: f32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CCamera::RestoreWithJumpCut()` (0x50BAB0)
    fn camera_restore_with_jump_cut(cam: GPtr<u8>) = thiscall 0x50BAB0 => unported;
    /// `CEntryExitManager::SetAreaCodeForVisibleObjects()` (0x43ECF0)
    fn set_area_code_for_visible_objects() = cdecl 0x43ECF0 => unported;
    /// `CEntryExitManager::ResetAreaCodeForVisibleObjects()` (0x43ED80)
    fn reset_area_code_for_visible_objects() = cdecl 0x43ED80 => unported;
    /// `CEntryExitManager::AddEntryExitToStack(CEntryExit*)` (0x43E410)
    fn add_entry_exit_to_stack(e: GPtr<u8>) = cdecl 0x43E410 => unported;
    /// `CEntryExitManager::DeleteOne(int32)` (0x43FD50)
    fn entry_exit_manager_delete_one(i: i32) = cdecl 0x43FD50 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CEvent::operator new(uint32)` (0x4B5620)
    fn event_new(size: u32) -> GPtr<u8> = cdecl 0x4B5620 => unported;
    /// `CEventLeaderEntryExit::CEventLeaderEntryExit(CPed*)` (0x43E1C0)
    fn event_leader_entry_exit_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x43E1C0 => unported;
    /// `CEventAreaCodes::CEventAreaCodes(CPed*)` (0x4B2190)
    fn event_area_codes_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B2190 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0)
    fn event_group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent()` (0x4AE070)
    fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470)
    fn group_intelligence_add_event(gi: GPtr<u8>, ev: GPtr<u8>) = thiscall 0x5F7470 => unported;
    /// `CClothes::RebuildPlayerIfNeeded(CPlayerPed*)` (0x5A8390)
    fn clothes_rebuild_player_if_needed(p: GPtr<u8>) = cdecl 0x5A8390 => unported;
    /// `CAudioZones::Update(bool, CVector)` (0x5083C0)
    fn audio_zones_update(force: u32, px: u32, py: u32, pz: u32) = cdecl 0x5083C0 => unported;
    /// `CWaterLevel::FindNearestWaterAndItsFlow()` (0x6E9D70)
    fn water_find_nearest_water_and_flow() = cdecl 0x6E9D70 => unported;
    /// `CGarages::CloseHideOutGaragesBeforeSave()` (0x44A170)
    fn garages_close_hideouts_before_save() = cdecl 0x44A170 => unported;
    /// `InteriorManager_c::SetEntryExitPtr(CEntryExit*)` (0x598180) on `g_interiorMan` (0xBAF670)
    fn interior_manager_set_entry_exit_ptr(m: GPtr<u8>, e: GPtr<u8>) = thiscall 0x598180 => unported;
    /// `InteriorManager_c::Update()` (0x598F50, AL)
    fn interior_manager_update(m: GPtr<u8>) -> u8 = thiscall 0x598F50 => unported;
    /// `CPopulation::RemoveAllRandomPeds()` (0x6122C0)
    fn population_remove_all_random_peds() = cdecl 0x6122C0 => unported;
    /// `CEntryExit::RequestAmbientPeds()` (0x43E6D0)
    fn entry_exit_request_ambient_peds(e: GPtr<u8>) = thiscall 0x43E6D0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn streaming_load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CTimeCycle::FindFarClipForCoors(CVector)` (0x5616E0): ST0
    fn find_far_clip_for_coors(x: u32, y: u32, z: u32) -> f64 = cdecl 0x5616E0 => unported;
    /// `RwCameraSetFarClipPlane(RwCamera*, float)` (0x7EE2A0)
    fn rw_camera_set_far_clip_plane(cam: GPtr<u8>, far: f32) -> GPtr<u8> = cdecl 0x7EE2A0 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn streaming_load_scene(p: GPtr<u8>) = cdecl 0x40EB70 => unported;
    /// `CEntryExit::GenerateAmbientPeds(const CVector&)` (0x43E8B0)
    fn entry_exit_generate_ambient_peds(e: GPtr<u8>, p: GPtr<u8>) = thiscall 0x43E8B0 => unported;
    /// `FUN_00407a30(bool)` (0x407A30, CStreaming: the load-scene vehicle flag)
    fn streaming_set_flag_407a30(b: u32) = cdecl 0x407A30 => unported;
    /// `CStreaming::ClearFlagForAll(uint32)` (0x407A40)
    fn streaming_clear_flag_for_all(f: u32) = cdecl 0x407A40 => unported;
    /// `CTimeCycle::StartExtraColour(int32, bool)` (0x55FEC0)
    fn time_cycle_start_extra_colour(c: i32, b: u32) = cdecl 0x55FEC0 => unported;
    /// `CTimeCycle::StopExtraColour(bool)` (0x55FF20)
    fn time_cycle_stop_extra_colour(b: u32) = cdecl 0x55FF20 => unported;
    /// `FUN_007204c0(bool)` (0x7204C0)
    fn fun_7204c0(b: u32) = cdecl 0x7204C0 => unported;
    /// virtual `CEntity::GetHeightAboveRoad()` (vtable +0xD4): ST0
    fn v_get_height_above_road(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// virtual `CPlaceable::Teleport(CVector, bool)` (vtable +0x38)
    fn v_teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, clear: u32) = thiscall vslot 0x38 => unported;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, angle: f32) = thiscall 0x59B020 => unported;
    /// `CEntryExit::FindValidTeleportPoint(CVector*)` (0x43EAF0)
    fn entry_exit_find_valid_teleport_point(e: GPtr<u8>, p: GPtr<u8>) = thiscall 0x43EAF0 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(p: GPtr<u8>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CShopping::RemoveLoadedShop()` (0x49AE30)
    fn shopping_remove_loaded_shop() = cdecl 0x49AE30 => unported;
    /// `CShopping::LoadShop(CEntryExit*)` (0x49BBE0)
    fn shopping_load_shop(e: GPtr<u8>) = cdecl 0x49BBE0 => unported;
    /// `CPopulation::ManageAllPopulation()` (0x6160A0)
    fn population_manage_all() = cdecl 0x6160A0 => unported;
    /// `CTheScripts::Process()` (0x46A000)
    fn the_scripts_process() = cdecl 0x46A000 => unported;
    /// `CEntryExit::WarpGangWithPlayer(CPed*)` (0x43F1F0)
    fn entry_exit_warp_gang_with_player(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x43F1F0 => unported;
    /// `CEntryExit::ProcessStealableObjects(CPed*)` (0x43E990)
    fn entry_exit_process_stealable_objects(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x43E990 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    fn task_manager_get_task_secondary(tm: GPtr<u8>, slot: u32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskComplexFacial::StopAll()` (0x691250)
    fn task_facial_stop_all(t: GPtr<u8>) = thiscall 0x691250 => unported;
    /// `CGame::TidyUpMemory(bool, bool)` (0x53C500)
    fn game_tidy_up_memory(a: u32, b: u32) = cdecl 0x53C500 => unported;
    /// `CEntryExit::GetEntryExitToDisplayNameOf()` (0x43E650)
    fn entry_exit_get_display_name_of(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x43E650 => unported;
    /// `CText::Get(const char*)` (0x6A0050) on `TheText` (0xC1B340)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetZoneName(const char*, bool)` (0x588BB0)
    fn hud_set_zone_name(name: GPtr<u8>, force: u32) = cdecl 0x588BB0 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `CEntryExit::ms_spawnPoint` (0x96A7C0).
fn spawn_point() -> GPtr<GPtr<u8>> {
    g::<GPtr<u8>>(0x96_A7C0)
}

/// `CEntryExit::ms_bWarping` (0x96A7B8).
fn warping() -> GPtr<u8> {
    g::<u8>(0x96_A7B8)
}

/// Adds `CEventScriptCommand(3, task, false)` to the ped's event group (intelligence +0x68) through a
/// local event, then destroys the local.
fn add_script_command(ped: GPtr<u8>, task: GPtr<u8>) {
    let mut ev = [0u32; 8];
    let ep = buf(&mut ev);
    event_script_command_ctor(ep, 3, task, 0);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    event_script_command_dtor(ep);
}

/// `CEntryExit::TransitionStarted(CPed* ped)` (0x43FFD0).
///
/// False unless access is enabled (flag 0x4000), no transition is running (0x96A7CC), the clock is in
/// the open hours and, in a vehicle, it is an automobile (flag 0x20) or a bike (type 9, flag 0x40) and
/// not a big one (handling flag 0x8 at +0xCC); on foot the exit must not be disabled (flag 0x800).
/// The spawn point is the link (which links back) or this; `d = exit - (rect centre, entrance z)`; false
/// when the player may not start a mission. Then entered-without-exit is set, the door cleared, the ped's
/// can-exit-car bit (+0x474 bit 26) cleared. With the pairing/food-date flags (0x202) or in a vehicle,
/// warping is set when `|d| > 10`. Otherwise warping = `|d| > 10`, `d` is normalised and a door within 10
/// units gives a go-to-door-and-open task (`d` then points from the centre to the door, flattened and
/// normalised); without a door a warp returns true at once (with flag 0x2 set), else the task walks
/// from the centre to `centre + d * 4`. Both set a fixed camera at `centre - d * k1` raised by `k3`,
/// looking at `centre + d * k2` (0x8A5E40/0x8A5E38/0x8A5E3C). Finally pad 0's controls are disabled
/// (+0x10E bit 3) and, on foot, the ped gets an uninterruptable task. True.
#[hook(addr = 0x43FFD0, conv = thiscall, name = "CEntryExit::TransitionStarted", this = "CEntryExit",
       expect = "6A FF 64 A1 00 00 00 00",
       unverified = "world4-2", route = "L2", hazards = "heap-writes",
       note = "allocates tasks (task pool) and adds script-command events (built on the Rust stack); writes TheCamera, pad 0, ms_spawnPoint/ms_pDoor/ms_bWarping; the SEH frame is not reproduced")]
pub fn entry_exit_transition_started(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x31) & 0x40 == 0
        || g::<u32>(0x96_A7CC).read() != 0
        || clock_get_is_time_in_range(u32::from(rd::<u8>(this, 0x34)), u32::from(rd::<u8>(this, 0x35))) == 0
    {
        return 0;
    }
    let in_vehicle = || rd::<u32>(ped, 0x46C) & 0x100 != 0;
    if in_vehicle() {
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        let vt = rd::<u32>(veh, 0x590);
        if vt != 0 && vt != 9 {
            return 0;
        }
        if rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xCC) & 8 != 0 {
            return 0;
        }
        let fl = rd::<u8>(this, 0x30);
        if fl & 0x20 == 0 && vt == 0 {
            return 0;
        }
        if fl & 0x40 == 0 && vt == 9 {
            return 0;
        }
    } else if rd::<u8>(this, 0x31) & 8 != 0 {
        return 0;
    }
    let link = rd::<GPtr<u8>>(this, 0x38);
    let spawn = if link.is_null() {
        spawn_point().write(this);
        this
    } else {
        spawn_point().write(link);
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x38, this);
        spawn_point().read()
    };
    let half = 0.5f32;
    let cy = f32_of(x(rd::<f32>(this, 0x14)).fadd_m(rd::<f32>(this, 0xC)).fmul_m(half));
    let cx = f32_of(x(rd::<f32>(this, 0x10)).fadd_m(rd::<f32>(this, 8)).fmul_m(half));
    let z = rd::<f32>(this, 0x18);
    let mut centre = [cx.to_bits(), cy.to_bits(), z.to_bits()];
    let dz = x(z).fsubr_m(rd::<f32>(spawn, 0x28));
    let dy = x(rd::<f32>(spawn, 0x24)).fsub_m(cy);
    let dx = x(rd::<f32>(spawn, 0x20)).fsub_m(cx);
    let mut d = [f32_of(dx).to_bits(), f32_of(dy).to_bits(), f32_of(dz).to_bits()];
    if can_player_start_mission(ped) == 0 {
        return 0;
    }
    wr::<u8>(this, 0x31, rd::<u8>(this, 0x31) | 0x20);
    g::<u32>(0x96_A7BC).write(0);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFBFF_FFFF);
    let ten = x(10.0);
    if rd::<u16>(this, 0x30) & 0x202 != 0 || in_vehicle() {
        let m = X87::from_f64_exact(vector_magnitude(buf(&mut d)));
        if !m.fcom(ten).test_any(0x41) {
            warping().write(1);
        }
    } else {
        let m = X87::from_f64_exact(vector_magnitude(buf(&mut d)));
        warping().write(1);
        if m.fcom(ten).test_any(0x41) {
            warping().write(0);
        }
        vector_normalise(buf(&mut d));
        let door = find_nearest_door(this, 10.0);
        if !door.is_null() {
            let mem = task_new(0x38);
            let task = if mem.is_null() { mem } else { task_goto_door_ctor_door(mem, door) };
            add_script_command_keep(ped, task, |ep| {
                g::<GPtr<u8>>(0x96_A7BC).write(door);
                let mtx = rd::<GPtr<u8>>(door, 0x14);
                let dp = if mtx.is_null() { door.byte_add(4) } else { mtx.byte_add(0x30) };
                let mut out = [0u32; 3];
                let r = vec_sub(buf(&mut out), dp, buf(&mut centre));
                d = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
                d[2] = 0;
                vector_normalise(buf(&mut d));
                ep
            });
        } else {
            if warping().read() != 0 {
                wr::<u8>(this, 0x30, rd::<u8>(this, 0x30) | 2);
                return 1;
            }
            let mem = task_new(0x38);
            let task = if mem.is_null() {
                mem
            } else {
                let four = f32::from_bits(0x4080_0000);
                let (fx, fy, fz) = (f32::from_bits(d[0]), f32::from_bits(d[1]), f32::from_bits(d[2]));
                let sy = f32_of(x(fy).fmul_m(four));
                let sx = f32_of(x(fx).fmul_m(four));
                let ez = x(fz).fmul_m(four).fadd_m(z);
                let ey = x(cy).fadd_m(sy);
                let ex = x(sx).fadd_m(cx);
                let mut end = [f32_of(ex).to_bits(), f32_of(ey).to_bits(), f32_of(ez).to_bits()];
                task_goto_door_ctor_points(mem, buf(&mut centre), buf(&mut end))
            };
            add_script_command(ped, task);
        }
        let mut dk1 = [0u32; 3];
        let r1 = vec_mul_scalar(buf(&mut dk1), buf(&mut d), g::<f32>(0x8A_5E40).read());
        let mut tmp = [0u32; 3];
        let r2 = vec_sub(buf(&mut tmp), buf(&mut centre), r1);
        let mut cam = [rd::<u32>(r2, 0), rd::<u32>(r2, 4), rd::<u32>(r2, 8)];
        cam[2] = f32_of(x(f32::from_bits(cam[2])).fadd_m(g::<f32>(0x8A_5E38).read())).to_bits();
        let mut dk2 = [0u32; 3];
        let r3 = vec_mul_scalar(buf(&mut dk2), buf(&mut d), g::<f32>(0x8A_5E3C).read());
        let mut tmp2 = [0u32; 3];
        let r4 = vec_add(buf(&mut tmp2), buf(&mut centre), r3);
        let mut look = [rd::<u32>(r4, 0), rd::<u32>(r4, 4), rd::<u32>(r4, 8)];
        let mut zero = [0u32; 3];
        let camera = GPtr::<u8>::from_addr(0xB6_F028);
        set_cam_position_for_fixed_mode(camera, buf(&mut cam), buf(&mut zero));
        take_control_no_entity(camera, buf(&mut look), 2, 1);
    }
    let pad = pad_get_pad(0);
    wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) | 8);
    if !in_vehicle() {
        let mem = task_new(8);
        let task = if mem.is_null() { mem } else { task_uninterruptable_ctor(mem) };
        add_script_command(ped, task);
    }
    1
}

/// Like [`add_script_command`], with `between` run after the event is added and before it is
/// destroyed (the door branch computes its direction there, as the original does).
fn add_script_command_keep(ped: GPtr<u8>, task: GPtr<u8>, between: impl FnOnce(GPtr<u8>) -> GPtr<u8>) {
    let mut ev = [0u32; 8];
    let ep = buf(&mut ev);
    event_script_command_ctor(ep, 3, task, 0);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    let ep = between(ep);
    event_script_command_dtor(ep);
}

/// `CEntryExitManager::ms_exitEnterState` (0x96A7CC).
fn state() -> GPtr<u32> {
    g::<u32>(0x96_A7CC)
}

/// `CGame::currArea` (0xB72914).
fn curr_area() -> GPtr<u32> {
    g::<u32>(0xB7_2914)
}

/// `TheCamera` (0xB6F028).
fn camera() -> GPtr<u8> {
    GPtr::from_addr(0xB6_F028)
}

/// Sends a `CEventGroupEvent(ped, ev)` to the group's intelligence (group +0x30) through a local.
fn group_event(group: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) {
    let mut ge = [0u32; 8];
    let gp = buf(&mut ge);
    event_group_event_ctor(gp, ped, ev);
    group_intelligence_add_event(group.byte_add(0x30), gp);
    event_group_event_dtor(gp);
}

/// The display-name tail (0x440B54): the zone name of the entry-exit to name, if any; false.
fn show_name(this: GPtr<u8>) -> u8 {
    let e = entry_exit_get_display_name_of(this);
    if !e.is_null() {
        let s = text_get(GPtr::from_addr(0xC1_B340), e);
        hud_set_zone_name(s, 1);
    }
    0
}

/// `CEntryExit::TransitionFinished(CPed* ped)` (0x4404A0): the state machine (0x96A7CC) that completes
/// a transition to the spawn point's (`ms_spawnPoint`) exit.
///
/// The held entity takes the spawn point's area. On foot without the pairing flags (0x202): the
/// collision and IPLs at the exit are requested (and, when warping, the models, the frustum objects and
/// the paths); state 0 sets the visible objects' area, goes to 1, sets the current area and sends the
/// ped's group a leader-entry-exit event (false); state 1 fades out and goes to 2 unless the ped's
/// primary task has type 0x3A4 (false, the name shown either way); state 2 waits for the fade, then 3.
/// In a vehicle or with the pairing flags: state 0 fades out over 0.5 s, goes to 2 and sends the group a
/// leader-entry-exit event (built as an area-codes event with the leader vtable 0x859540) (false, name
/// shown); state 2 waits for the fade then goes to 3 (false); other states set the area and clear the
/// visible entities, then finish. The finish (also state 3+ on foot): areas of the ped (and its
/// vehicle), the ped's +0x78C entry-exit (this outside area 0), the stack, can-exit-car, pad 0's
/// controls back, clothes; with flag 0x200 on this it returns true at once (entered-without-exit
/// cleared on the spawn point). Otherwise the camera jumps to the exit; audio zones, water, garages,
/// visible objects, the interior manager, random and ambient peds and the scene are reloaded with the
/// timer suspended (far clip from the time cycle in area 0); the extra colour is started or stopped;
/// the vehicle (`z - 1 + height above road`) or the ped (at a valid teleport point) is teleported and
/// turned to the exit angle; the camera fades in, space is cleared, the shop loaded or removed,
/// population and scripts run, the gang warped and stealable objects processed, the spawn point deleted
/// when flagged (0x8000), facial tasks stopped and memory tidied. True.
#[hook(addr = 0x4404A0, conv = thiscall, name = "CEntryExit::TransitionFinished", this = "CEntryExit",
       expect = "6A FF 68 06 78 83 00",
       unverified = "world4-2", route = "L3-only", hazards = "heap-writes",
       note = "drives streaming (LoadScene, LoadAllRequestedModels), the timer, the camera, events and population once per transition; group events built on the Rust stack; the original stores the exit heading into its own ped-argument slot; the SEH frame is not reproduced")]
pub fn entry_exit_transition_finished(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let sp = spawn_point().read();
    let mut pos = [rd::<u32>(sp, 0x20), rd::<u32>(sp, 0x24), rd::<u32>(sp, 0x28)];
    let held = ped_get_held_entity(ped);
    if !held.is_null() {
        wr::<u8>(held, 0x2F, rd::<u8>(spawn_point().read(), 0x32));
    }
    let in_vehicle = || rd::<u32>(ped, 0x46C) & 0x100 != 0;
    if rd::<u16>(this, 0x30) & 0x202 != 0 || in_vehicle() {
        match state().read() {
            0 => {
                camera_set_fade_colour(camera(), 0, 0, 0);
                camera_fade(camera(), 0.5, 0);
                state().write(2);
                let group = get_peds_group(ped);
                if !group.is_null() {
                    let mem = event_new(0x10);
                    let ev = if mem.is_null() {
                        mem
                    } else {
                        event_area_codes_ctor(mem, ped);
                        wr::<u32>(mem, 0, 0x0085_9540);
                        mem
                    };
                    group_event(group, ped, ev);
                }
                return show_name(this);
            }
            2 => {
                if camera_get_fading(camera()) == 0 {
                    state().write(3);
                }
                return 0;
            }
            _ => {
                curr_area().write(u32::from(rd::<u8>(spawn_point().read(), 0x32)));
                g::<u32>(0x96_A7DC).write(0);
            }
        }
    } else {
        col_store_add_collision_needed(buf(&mut pos));
        ipl_store_add_ipls_needed(buf(&mut pos));
        if warping().read() != 0 {
            g::<u8>(0xB7_6850).write(0);
            streaming_add_models_to_request_list(buf(&mut pos), 0x20);
            entry_exit_request_objects_in_frustum(spawn_point().read());
            paths_set_needed_at_position(GPtr::from_addr(0x96_F050), buf(&mut pos));
        }
        match state().read() {
            0 => {
                set_area_code_for_visible_objects();
                state().write(1);
                curr_area().write(u32::from(rd::<u8>(spawn_point().read(), 0x32)));
                let group = get_peds_group(ped);
                if group.is_null() {
                    return 0;
                }
                let mem = event_new(0x10);
                let ev = if mem.is_null() { mem } else { event_leader_entry_exit_ctor(mem, ped) };
                group_event(group, ped, ev);
                return 0;
            }
            1 => {
                let task = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x47C), 0x10);
                if task.is_null() || v_task_get_type(task) != 0x3A4 {
                    camera_set_fade_colour(camera(), 0, 0, 0);
                    camera_fade(camera(), 1.0, 0);
                    state().write(2);
                }
                return show_name(this);
            }
            2 => {
                if camera_get_fading(camera()) != 0 {
                    return 0;
                }
                state().write(3);
            }
            _ => {}
        }
    }
    transition_finish(this, ped, &mut pos)
}

/// The common finish of `TransitionFinished` (0x4406AD).
fn transition_finish(this: GPtr<u8>, ped: GPtr<u8>, pos: &mut [u32; 3]) -> u8 {
    let in_vehicle = || rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    wr::<u8>(ped, 0x2F, curr_area().read() as u8);
    if !veh.is_null() && in_vehicle() {
        wr::<u8>(veh, 0x2F, curr_area().read() as u8);
    }
    let area_this = if curr_area().read() != 0 { this } else { GPtr::NULL };
    wr::<GPtr<u8>>(ped, 0x78C, area_this);
    add_entry_exit_to_stack(this);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x400_0000);
    let pad = pad_get_pad(0);
    wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) & 0xF7);
    clothes_rebuild_player_if_needed(ped);
    if rd::<u8>(this, 0x31) & 2 != 0 {
        let sp = spawn_point().read();
        wr::<u8>(sp, 0x31, rd::<u8>(sp, 0x31) & 0xDF);
        return 1;
    }
    let m = g::<GPtr<u8>>(0xB6_F03C).read();
    if m.is_null() {
        g::<u32>(0xB6_F02C).write(pos[0]);
        g::<u32>(0xB6_F030).write(pos[1]);
        g::<u32>(0xB6_F034).write(pos[2]);
    } else {
        wr::<u32>(m, 0x30, pos[0]);
        wr::<u32>(g::<GPtr<u8>>(0xB6_F03C).read(), 0x34, pos[1]);
        wr::<u32>(g::<GPtr<u8>>(0xB6_F03C).read(), 0x38, pos[2]);
    }
    camera_restore_with_jump_cut(camera());
    audio_zones_update(1, pos[0], pos[1], pos[2]);
    water_find_nearest_water_and_flow();
    garages_close_hideouts_before_save();
    reset_area_code_for_visible_objects();
    let interiors = GPtr::<u8>::from_addr(0xBA_F670);
    interior_manager_set_entry_exit_ptr(interiors, this);
    population_remove_all_random_peds();
    entry_exit_request_ambient_peds(this);
    streaming_load_all_requested_models(0);
    timer_suspend();
    if curr_area().read() == 0 {
        let far = f32_of(X87::from_f64_exact(find_far_clip_for_coors(pos[0], pos[1], pos[2])));
        rw_camera_set_far_clip_plane(g::<GPtr<u8>>(0xB6_F97C).read(), far);
    }
    entry_exit_request_objects_in_frustum(spawn_point().read());
    streaming_load_scene(buf(pos));
    streaming_load_all_requested_models(0);
    entry_exit_generate_ambient_peds(this, buf(pos));
    if interior_manager_update(interiors) != 0 {
        streaming_set_flag_407a30(0);
        streaming_load_scene(buf(pos));
        streaming_set_flag_407a30(1);
    }
    timer_resume();
    streaming_clear_flag_for_all(0x20);
    let extra = rd::<u8>(spawn_point().read(), 0x33);
    if extra != 0 {
        time_cycle_start_extra_colour(i32::from(extra) - 1, 0);
    } else {
        time_cycle_stop_extra_colour(0);
    }
    fun_7204c0(u32::from(rd::<u8>(spawn_point().read(), 0x30) & 1));
    let deg2rad = f32::from_bits(0x3C8E_FA35);
    if in_vehicle() {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        let (px, py, pz) = (pos[0], pos[1], f32::from_bits(pos[2]));
        let h = X87::from_f64_exact(v_get_height_above_road(v));
        let z = f32_of(x(pz).fsub_m(1.0).fadd(h));
        v_teleport(v, px, py, z.to_bits(), 0);
        let a = f32_of(x(rd::<f32>(spawn_point().read(), 0x2C)).fmul_m(deg2rad));
        let vm = rd::<GPtr<u8>>(v, 0x14);
        if vm.is_null() {
            wr::<f32>(v, 0x10, a);
        } else {
            matrix_set_rotate_z_only(vm, a);
        }
    } else {
        entry_exit_find_valid_teleport_point(spawn_point().read(), buf(pos));
        v_teleport(ped, pos[0], pos[1], pos[2], 0);
        let a = f32_of(x(rd::<f32>(spawn_point().read(), 0x2C)).fmul_m(deg2rad));
        wr::<f32>(ped, 0x558, a);
        wr::<f32>(ped, 0x55C, a);
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        if pm.is_null() {
            wr::<f32>(ped, 0x10, a);
        } else {
            matrix_set_rotate_z_only(pm, a);
        }
    }
    camera_fade(camera(), 1.0, 1);
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
    clear_space_for_mission_entity(pp, ped);
    let fl = rd::<u8>(spawn_point().read(), 0x30);
    if fl & 0x10 != 0 {
        shopping_remove_loaded_shop();
    } else if fl & 8 != 0 {
        shopping_load_shop(this);
        if rd::<u32>(this, 0x38) != 0 {
            wr::<u8>(this, 0x30, rd::<u8>(this, 0x30) | 0x10);
        }
    }
    population_manage_all();
    the_scripts_process();
    if rd::<u8>(spawn_point().read(), 0x31) & 1 != 0 {
        entry_exit_warp_gang_with_player(this, ped);
    }
    entry_exit_process_stealable_objects(this, ped);
    let sp = spawn_point().read();
    wr::<u8>(sp, 0x31, rd::<u8>(sp, 0x31) & 0xDF);
    let sp = spawn_point().read();
    if rd::<u8>(sp, 0x31) & 0x80 != 0 {
        let base = rd::<GPtr<u8>>(g::<GPtr<u8>>(0x96_A7D8).read(), 0);
        let diff = (sp.addr() as u32).wrapping_sub(base.addr() as u32) as i32;
        entry_exit_manager_delete_one(diff / 0x3C);
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let facial = task_manager_get_task_secondary(intel.byte_add(4), 3);
    task_facial_stop_all(facial);
    game_tidy_up_memory(1, 1);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ENTRY_EXIT_TRANSITION_STARTED, HOOK_ENTRY_EXIT_TRANSITION_FINISHED];
