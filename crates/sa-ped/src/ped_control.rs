//! `CPed` / `CCivilianPed` / `CCopPed` / `CEmergencyPed` / `CPlayerPed` process drivers (PORT-PED-3, agent):
//! `ProcessControl` of every ped class, `CCopPed::AddCriminalToKill`, and the per-frame helpers `ProcessControl`
//! calls (`CalculateNewVelocity`, `PlayFootSteps`, `DoFootLanded`). Ported from the 1.0 US bytes (instruction
//! addresses cited); specs `docs/re/specs/PED/CPed.md`, `CCopPed.md`, `CEmergencyPed.md`; gta-reversed was a
//! reading aid only (original bugs kept, deviations noted per function).
//!
//! `CPed` fields used here: +0x14 matrix, +0x18 RW clump, +0x1C entity flags, +0x20 random seed, +0x40 physical
//! flags, +0x44 move speed, +0x138 ped audio, +0x394 weapon audio, +0x46C/+0x470/+0x474/+0x478 ped flag dwords
//! (F1..F4), +0x47C intelligence, +0x480 player data, +0x484 created-by, +0x4D8 anim shift local, +0x4F4 weapon
//! object, +0x504.. gun flash alpha/progress, +0x530 ped state, +0x534 move state, +0x568 standing-on entity,
//! +0x588 ground-z probe, +0x58C vehicle, +0x598 ped type, +0x5A0 weapons (0x1C each), +0x718 active slot.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;
use sa_world::world::common::find_player_coors;

/// `CTimer::m_FrameCounter`
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CGame::currArea`
const CURR_AREA: usize = 0xB72914;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CWorld::bForceProcessControl`
const FORCE_PROCESS_CONTROL: usize = 0xB7CD6E;
/// `g_fxMan` (FxManager_c)
const G_FX_MAN: usize = 0xA9AE80;
/// `"molotov_flame"` (.rdata)
const MOLOTOV_FLAME: u32 = 0x85AA44;
/// `CCollision::ms_colPoints` style static array passed to `ProcessColModels` (0xC092A8)
const COL_POINTS: u32 = 0xC092A8;
/// `CPedGroups::ms_groups` membership of group 0 (`0xC09920 + 8`, 0x2D4 per group)
const GROUP_MEMBERSHIP_0: usize = 0xC09928;
/// `gpBloodPoolTex`
const BLOOD_POOL_TEX: usize = 0xC40400;
/// `TheCamera` (CCamera; +0x14 matrix, +4 placement position)
const THE_CAMERA: usize = 0xB6F028;
/// pi and 2pi as .rdata 0x858CB8 / 0x858CBC hold them
const PI: f32 = f32::from_bits(0x4049_0FDB);
const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
/// `g_surfaceInfos`
const SURFACE_INFOS: usize = 0xB79538;
/// `CTimer::bSlowMotionActive`
const SLOW_MOTION: usize = 0xB7CB88;
/// 99999.992f, the "no ground" value of +0x588
const NO_GROUND_Z: u32 = 0x47C3_4FFF;
/// `g_fx` (Fx_c) and its `m_Splash` (+0x30) / `m_SmokeII3expand` (+0x24) systems
const G_FX: usize = 0xA9AE00;
const FX_SPLASH: usize = 0xA9AE30;
const FX_SMOKE_II3_EXPAND: usize = 0xA9AE24;
/// byte 0xB72C70 (catalogue "gHorZ+0x4"): set = no foot dust particle
const NO_FOOT_DUST: usize = 0xB72C70;
/// `gpShadowPedTex`
const SHADOW_PED_TEX: usize = 0xC403E4;
/// `CWeather::Rain`
const WEATHER_RAIN: usize = 0xC81324;
/// .data floats of DoFootLanded's shallow-water splash: z offset 0.3f and top-direction factor -0.2f
const SHALLOW_SPLASH_Z: usize = 0x8D21EC;
const SHALLOW_SPLASH_K: usize = 0x8D21E8;
/// `CPedStats::ms_apPedStats` (pointer)
const PED_STATS_PTRS: usize = 0xC0BBEC;
/// `TheCamera.m_aCams` (0x238 each) and `TheCamera.m_nActiveCam` (byte)
const CAMERA_CAMS: usize = 0xB6F19C;
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
/// `AudioEngine` (CAudioEngine)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CPedGroups::ms_groups` (0x2D4 each; +4 m_bMembersEnterLeadersVehicle, +8 membership)
const PED_GROUPS: usize = 0xC09920;

/// PreRenderAfterTest globals: `CTimer::bSkipProcessThisFrame`, "minigun2", `CPedIK::XaxisIK`, the time cycle's
/// shadow strength word, `MAX_DISTANCE_PED_SHADOWS_SQR`, `g_realTimeShadowMan`, `CTimeCycle::m_CurrentStoredValue`,
/// "vest" / "torso", the breathing scale factors and the wobble factor (.data), `CWeather::Earthquake`,
/// `g_fx.m_WaterSplash` (+0x38).
const SKIP_PROCESS_THIS_FRAME: usize = 0xB7CB89;
const MINIGUN2: u32 = 0x86C34C;
const XAXIS_IK: usize = 0x8D232C;
const SHADOW_STRENGTH: usize = 0xB7C4E8;
const MAX_PED_SHADOW_DIST_SQR: usize = 0xC4B6B0;
const REALTIME_SHADOW_MAN: usize = 0xC40350;
const TIMECYC_CURRENT_STORED: usize = 0xB79FD0;
const STR_VEST: u32 = 0x86C344;
const STR_TORSO: u32 = 0x8590CC;
const BREATH_K1: usize = 0x8D1378;
const BREATH_K2: usize = 0x8D137C;
const BREATH_K3: usize = 0x8D1380;
const WOBBLE_FACTOR: usize = 0x8D21F0;
const WEATHER_EARTHQUAKE: usize = 0xC81340;
const FX_WATER_SPLASH: usize = 0xA9AE38;

/// CPlayerInfo::Process globals: `CReplay::Mode`, `CTimer::m_snPreviousTimeInMilliseconds`, `ThePaths`,
/// `CGameLogic::bPlayersCanBeInSeparateCars`, the chase-value statics (init flag 0xB9B9A0, saved coords 0xB9B994,
/// "moved" 0x8CDF21, "near node" 0x8CDF20).
const REPLAY_MODE: usize = 0xA43088;
const TIME_PREV_MS: usize = 0xB7CB78;
const THE_PATHS: usize = 0x96F050;
const PLAYERS_CAN_BE_IN_SEPARATE_CARS: usize = 0x96A8B3;
const CHASE_INIT: usize = 0xB9B9A0;
const CHASE_SAVED: usize = 0xB9B994;
const CHASE_MOVED: usize = 0x8CDF21;
const CHASE_NEAR_NODE: usize = 0x8CDF20;

orig! {
    /// `CStats::SetNewRecordStat(uint16, float)` (0x55C410)
    fn stats_set_new_record_stat(stat: u32, v: u32) = cdecl 0x55C410 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CPlayerInfo::WorkOutEnergyFromHunger()` (0x56E610)
    fn player_info_work_out_energy_from_hunger(p: GPtr<u8>) = thiscall 0x56E610 => unported;
    /// `CPlayerInfo::StreamParachuteWeapon(bool)` (0x56EB30)
    fn player_info_stream_parachute_weapon(p: GPtr<u8>, b: u32) = thiscall 0x56EB30 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390, AL; result unused here)
    fn game_logic_is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CPathFind::CalcRoadDensity(float x, float y)` (0x44EFC0, ST0)
    fn paths_calc_road_density(pf: GPtr<u8>, x: u32, y: u32) -> f64 = thiscall 0x44EFC0 => unported;
    /// `CPad::ExitVehicleJustDown()` (0x540120, AL)
    fn pad_exit_vehicle_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540120 => unported;
    /// `CPad::GetExitVehicle()` (0x5400D0, AL)
    fn pad_get_exit_vehicle(pad: GPtr<u8>) -> u8 = thiscall 0x5400D0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_mgr_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18, AL)
    fn task_make_abortable(t: GPtr<u8>, p: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_operator_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32, int32, bool, bool)` (0x63B8C0)
    fn task_leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0)
    fn task_mgr_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CVehicle::CanPedStepOutCar(bool)` (vtable +0xE4, AL)
    fn vehicle_can_ped_step_out_car(v: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0xE4 => unported;
    /// `CVehicle::CanPedJumpOutCar(CPed*)` (vtable +0xE8, AL)
    fn vehicle_can_ped_jump_out_car(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall vslot 0xE8 => unported;
    /// `CPlayerInfo::GivePlayerParachute()` (0x56EC40)
    fn player_info_give_player_parachute(p: GPtr<u8>) = thiscall 0x56EC40 => unported;
    /// `CBridge::ThisIsABridgeObjectMovingUp(int32)` (0x41BCB0, AL)
    fn bridge_this_is_a_bridge_object_moving_up(m: i32) -> u8 = cdecl 0x41BCB0 => unported;
    /// `CPlayerInfo::FindObjectToSteal(CPed*)` (0x56DBD0)
    fn player_info_find_object_to_steal(p: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x56DBD0 => unported;
    /// `CTaskComplexGoPickUpEntity::CTaskComplexGoPickUpEntity(CEntity*, int32)` (0x6919C0)
    fn task_go_pick_up_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32) -> GPtr<u8> = thiscall 0x6919C0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(ev: GPtr<u8>, slot: i32, t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(ev: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// 0x406CE0: `floor((double)f)` as a cdecl(float) helper (ST0)
    fn floor_f(v: f32) -> f64 = cdecl 0x406CE0 => unported;
    /// `CWorld::SetNextScanCode()` (0x4072E0)
    fn world_set_next_scan_code() = cdecl 0x4072E0 => unported;
    /// `CWorld::GetRepeatSector(int32, int32)` (0x4072A0)
    fn world_get_repeat_sector(x: i32, y: i32) -> GPtr<u8> = cdecl 0x4072A0 => unported;
    /// `CPlayerInfo::FindClosestCarSectorList(CPtrList&, CPed*, float x4, float*, CVehicle**)` (0x56F4E0)
    fn player_info_find_closest_car_sector_list(p: GPtr<u8>, list: GPtr<u8>, ped: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, dist: GPtr<u8>, veh: GPtr<u8>) = thiscall 0x56F4E0 => unported;
    /// `CVehicle::CanBeDriven()` (0x6D5400, AL)
    fn vehicle_can_be_driven(v: GPtr<u8>) -> u8 = thiscall 0x6D5400 => unported;
    /// `CTaskSimpleJetPack::DropJetPack(CPed*)` (0x67B660)
    fn jet_pack_drop_jet_pack(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x67B660 => unported;
    /// `CCarEnterExit::GetNearestCarDoor(CPed*, CVehicle*, CVector&, int32&)` (0x6528F0, AL)
    fn car_enter_exit_get_nearest_car_door(p: GPtr<u8>, v: GPtr<u8>, pos: GPtr<u8>, door: GPtr<u8>) -> u8 = cdecl 0x6528F0 => unported;
    /// `CTaskComplexEnterCarAsPassenger::CTaskComplexEnterCarAsPassenger(CVehicle*, int32, bool)` (0x640340)
    fn task_enter_car_as_passenger_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x640340 => unported;
    /// `CTaskComplexEnterCarAsDriver::CTaskComplexEnterCarAsDriver(CVehicle*)` (0x6402F0)
    fn task_enter_car_as_driver_ctor(t: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x6402F0 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00)
    fn camera_set_fade_colour(c: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16)` (0x50AC20)
    fn camera_fade(c: GPtr<u8>, t: u32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CCamera::RestoreWithJumpCut()` (0x50BAB0)
    fn camera_restore_with_jump_cut(c: GPtr<u8>) = thiscall 0x50BAB0 => unported;
    /// `CCamera::Process()` (0x52B730)
    fn camera_process(c: GPtr<u8>) = thiscall 0x52B730 => unported;
    /// `CTimer::Stop()` (0x561AA0)
    fn timer_stop() = cdecl 0x561AA0 => unported;
    /// `CRenderer::RequestObjectsInFrustum(...)` (0x555960, two dword arguments here)
    fn renderer_request_objects_in_frustum(a: u32, b: u32) = cdecl 0x555960 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn streaming_load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
    /// `CTimer::Update()` (0x561B10)
    fn timer_update() = cdecl 0x561B10 => unported;
    /// `CDamageManager::SetEngineStatus(uint32)` (0x6C22A0)
    fn damage_manager_set_engine_status(d: GPtr<u8>, s: u32) = thiscall 0x6C22A0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn vehicle_get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CWorld::FindPlayerTrain(int32)` (0x56E160)
    fn find_player_train(id: i32) -> GPtr<u8> = cdecl 0x56E160 => unported;
    /// `CStats::UpdateStatsWhenFlying(CVehicle*)` (0x55CC00)
    fn stats_update_when_flying(v: GPtr<u8>) = cdecl 0x55CC00 => unported;
    /// `CStats::UpdateStatsWhenOnMotorBike(CBike*)` (0x55CD60)
    fn stats_update_when_on_motor_bike(v: GPtr<u8>) = cdecl 0x55CD60 => unported;
    /// `CStats::UpdateStatsWhenDriving(CVehicle*)` (0x55CAC0)
    fn stats_update_when_driving(v: GPtr<u8>) = cdecl 0x55CAC0 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90, AX)
    fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> u32 = thiscall 0x53FC90 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0, AX)
    fn pad_get_accelerate(pad: GPtr<u8>) -> u32 = thiscall 0x5403F0 => unported;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50, AL)
    fn scripts_is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CVector, uint8, float, bool x5)` (0x44F460, hidden return pointer)
    fn paths_find_node_closest_to_coors(pf: GPtr<u8>, out: GPtr<u8>, x: u32, y: u32, z: u32, ty: u32, dist: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u8> = thiscall 0x44F460 => unported;
    /// `CCullZones::NoPolice()` (0x72DD50, AL)
    fn cull_zones_no_police() -> u8 = cdecl 0x72DD50 => unported;
    /// `CCullZones::PoliceAbandonCars()` (0x72DD60, AL)
    fn cull_zones_police_abandon_cars() -> u8 = cdecl 0x72DD60 => unported;
    /// 0x56EC80 (proposed `CPlayerCrossHair::Process(int32, CPad*)`)
    fn player_cross_hair_process(c: GPtr<u8>, idx: i32, pad: GPtr<u8>) = thiscall 0x56EC80 => unported;
    /// `CTaskSimpleSwim::ApplyRollAndPitch(CPed*)` (0x68A8E0)
    fn swim_apply_roll_and_pitch(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x68A8E0 => unported;
    /// `CTaskSimpleJetPack::ApplyRollAndPitch(CPed*)` (0x67B720)
    fn jet_pack_apply_roll_and_pitch(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x67B720 => unported;
    /// `CPedIntelligence::GetTaskInAir()` (0x601150)
    fn get_task_in_air(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601150 => unported;
    /// 0x678E80 (empty `CTaskSimpleInAir` method taking the ped, spec `CPed.md`)
    fn task_in_air_678e80(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x678E80 => unported;
    /// `CPedIK::PitchForSlope()` (0x5FE0E0)
    fn ped_ik_pitch_for_slope(ik: GPtr<u8>) = thiscall 0x5FE0E0 => unported;
    /// `CClumpModelInfo::GetFrameFromName(RpClump*, const char*)` (0x4C5400)
    fn clump_get_frame_from_name(c: GPtr<u8>, name: u32) -> GPtr<u8> = cdecl 0x4C5400 => unported;
    /// `RwMatrixRotate(RwMatrix*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7F1FD0)
    fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: u32, combine: u32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// `RwMatrixScale(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F22C0)
    fn rw_matrix_scale(m: GPtr<u8>, s: GPtr<u8>, combine: u32) -> GPtr<u8> = cdecl 0x7F22C0 => unported;
    /// `Fx_c::GetFxQuality()` (0x49EA50)
    fn fx_get_fx_quality(fx: GPtr<u8>) -> i32 = thiscall 0x49EA50 => unported;
    /// `CRealTimeShadowManager::DoShadowThisFrame(CPhysical*)` (0x706BA0)
    fn realtime_shadow_do_shadow_this_frame(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x706BA0 => unported;
    /// `CShadows::StoreShadowForPedObject(CEntity*, float x6)` (0x707B40)
    fn shadows_store_shadow_for_ped_object(p: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) = cdecl 0x707B40 => unported;
    /// `CPed::ShoulderBoneRotation(RpClump*)` (0x5DF560)
    fn ped_shoulder_bone_rotation(c: GPtr<u8>) = cdecl 0x5DF560 => unported;
    /// `CWindModifiers::FindWindModifier(CVector, float*, float*)` (0x72C950, AL)
    fn wind_find_wind_modifier(x: u32, y: u32, z: u32, a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x72C950 => unported;
    /// `CVehicle::IsOpenTopCar()` (virtual, vtable +0x9C, AL)
    fn vehicle_is_open_top_car(v: GPtr<u8>) -> u8 = thiscall vslot 0x9C => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn keygen_get_uppercase_key(s: u32) -> u32 = cdecl 0x53CF30 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, hidden return pointer)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `Fx_c::AddBlood(CVector&, CVector&, int32, float)` (0x49EB00)
    fn fx_add_blood(fx: GPtr<u8>, p: GPtr<u8>, d: GPtr<u8>, n: i32, bright: u32) = thiscall 0x49EB00 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPedModelInfo::AnimatePedColModelSkinnedWorld(RpClump*)` (0x4C7170)
    fn ped_mi_animate_ped_col_model_skinned_world(mi: GPtr<u8>, c: GPtr<u8>) = thiscall 0x4C7170 => unported;
    /// `CWorld::FindPlayerSpeed(int32)` (0x56E090)
    fn find_player_speed(id: i32) -> GPtr<u8> = cdecl 0x56E090 => unported;
    /// `CPed::IsPedHeadAbovePos(float)` (0x5F02C0, AL)
    fn ped_is_ped_head_above_pos(p: GPtr<u8>, z: u32) -> u8 = thiscall 0x5F02C0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(c: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `RwEngineInstance->dOpenDevice.fpRenderStateGet` (`*(0xC97B24) + 0x24`)
    fn rw_render_state_get(state: u32, value: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0x24 => unported;
    /// `RwEngineInstance->dOpenDevice.fpRenderStateSet` (`*(0xC97B24) + 0x20`)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn task_mgr_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPostEffects::IsVisionFXActive()` (0x7034F0, AL)
    fn post_effects_is_vision_fx_active() -> u8 = cdecl 0x7034F0 => unported;
    /// `CPostEffects::InfraredVisionStoreAndSetLightsForHeatObjects(CPed*)` (0x701320)
    fn post_effects_infrared_store_and_set_lights(p: GPtr<u8>) = cdecl 0x701320 => unported;
    /// `CPostEffects::NightVisionSetLights()` (0x7012E0)
    fn post_effects_night_vision_set_lights() = cdecl 0x7012E0 => unported;
    /// `CPostEffects::InfraredVisionRestoreLightsForHeatObjects()` (0x701410)
    fn post_effects_infrared_restore_lights() = cdecl 0x701410 => unported;
    /// `CEntity::Render()` (0x534310)
    fn entity_render(e: GPtr<u8>) = thiscall 0x534310 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    fn intel_get_task_hold(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CVisibilityPlugins::AddWeaponPedForPC(CPed*)` (0x5E46D0)
    fn visibility_add_weapon_ped_for_pc(p: GPtr<u8>) = cdecl 0x5E46D0 => unported;
    /// 0x5DF4E0 (catalogue `CPed::ResetGunFlashAlpha`)
    fn ped_reset_gun_flash_alpha(p: GPtr<u8>) = thiscall 0x5DF4E0 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `RpClumpRender(RpClump*)` (0x749B20)
    fn rp_clump_render(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B20 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    fn get_task_jet_pack(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CTaskSimpleJetPack::RenderJetPack(CPed*)` (0x67F6A0)
    fn jet_pack_render(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x67F6A0 => unported;
    /// `CMBlur::ClearDrunkBlur()` (0x71D780)
    fn mblur_clear_drunk_blur() = cdecl 0x71D780 => unported;
    /// `CMBlur::SetDrunkBlur(float)` (0x71D730)
    fn mblur_set_drunk_blur(f: f32) = cdecl 0x71D730 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier(m: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CWanted::Update()` (0x562C90)
    fn wanted_update(w: GPtr<u8>) = thiscall 0x562C90 => unported;
    /// `CPad::GetWeapon(CPed*)` (0x540180, EAX tested whole)
    fn pad_get_weapon(pad: GPtr<u8>, ped: GPtr<u8>) -> i32 = thiscall 0x540180 => unported;
    /// `CPed::GetWeaponSkill()` (0x5E6580; EAX pushed whole to GetWeaponInfo)
    fn ped_get_weapon_skill(p: GPtr<u8>) -> i32 = thiscall 0x5E6580 => unported;
    /// `CWeaponInfo::GetTargetHeadRange()` (0x743D50, ST0)
    fn weapon_info_get_target_head_range(i: GPtr<u8>) -> f64 = thiscall 0x743D50 => unported;
    /// `CPed::GetTransformedBonePosition(RwV3d&, ePedBones, bool)` (0x5E01C0)
    fn get_transformed_bone_position(p: GPtr<u8>, v: GPtr<u8>, bone: i32, update: u32) = thiscall 0x5E01C0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CWeaponEffects::MarkTarget(int32, CVector, uint8 r, g, b, a, float size, uint8)` (0x742BF0; colours read as bytes)
    fn weapon_effects_mark_target(idx: u32, x: u32, y: u32, z: u32, r: u32, g: u32, b: u32, a: u32, size: u32, flag: u32) = cdecl 0x742BF0 => unported;
    /// `CPlayerPed::HandleSprintEnergy(bool, float)` (0x60A550, AL)
    fn handle_sprint_energy(p: GPtr<u8>, b: u32, f: u32) -> u8 = thiscall 0x60A550 => unported;
    /// `CPlayerPed::ClearWeaponTarget()` (0x609C80)
    fn player_clear_weapon_target(p: GPtr<u8>) = thiscall 0x609C80 => unported;
    /// `CPad::WeaponJustDown(CPed*)` (0x540250, AL)
    fn pad_weapon_just_down(pad: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x540250 => unported;
    /// `CCamera::Using1stPersonWeaponMode()` (0x50BFF0, AL)
    fn camera_using_1st_person_weapon_mode(c: GPtr<u8>) -> u8 = thiscall 0x50BFF0 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float, float)` (0x506EA0)
    fn audio_report_frontend_event(a: GPtr<u8>, id: u32, vol: u32, spd: u32) = thiscall 0x506EA0 => unported;
    /// `CPed::IsPedShootable()` (0x5DEFD0, AL)
    fn ped_is_ped_shootable(p: GPtr<u8>) -> u8 = thiscall 0x5DEFD0 => unported;
    /// `CWorld::FindPlayerSlotWithPedPointer(void*)` (0x563FA0)
    fn find_player_slot_with_ped_pointer(p: GPtr<u8>) -> i32 = cdecl 0x563FA0 => unported;
    /// `CPlayerPed::ProcessWeaponSwitch(CPad*)` (0x60D850)
    fn player_process_weapon_switch(p: GPtr<u8>, pad: GPtr<u8>) = thiscall 0x60D850 => unported;
    /// `CPlayerPed::ProcessAnimGroups()` (0x6098F0)
    fn player_process_anim_groups(p: GPtr<u8>) = thiscall 0x6098F0 => unported;
    /// `CPed::SetLookFlag(float, bool, bool)` (0x5DEDC0)
    fn ped_set_look_flag(p: GPtr<u8>, dir: u32, a: u32, b: u32) = thiscall 0x5DEDC0 => unported;
    /// `CPed::SetLookTimer(uint32)` (0x5DF8D0)
    fn ped_set_look_timer(p: GPtr<u8>, t: u32) = thiscall 0x5DF8D0 => unported;
    /// `CPed::ClearLookFlag()` (0x5E1950)
    fn ped_clear_look_flag(p: GPtr<u8>) = thiscall 0x5E1950 => unported;
    /// `CPad::GetTarget()` (0x540670, AL)
    fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `CPlayerPed::ProcessGroupBehaviour(CPad*)` (0x60D350)
    fn player_process_group_behaviour(p: GPtr<u8>, pad: GPtr<u8>) = thiscall 0x60D350 => unported;
    /// `CCarCtrl::RegisterVehicleOfInterest(CVehicle*)` (0x423DE0)
    fn car_ctrl_register_vehicle_of_interest(v: GPtr<u8>) = cdecl 0x423DE0 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    fn entity_update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CPad::IsDPadUpPressed()` (0x53EE60, AL)
    fn pad_is_dpad_up_pressed(pad: GPtr<u8>) -> u8 = thiscall 0x53EE60 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0)
    fn group_count_members_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CPedGroup::FindDistanceToNearestMember(CPed**)` (0x5FB0A0, ST0)
    fn group_find_distance_to_nearest_member(g: GPtr<u8>, out: u32) -> f64 = thiscall 0x5FB0A0 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0)
    fn group_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90, ST0)
    fn general_get_random_number_in_range_f(a: f32, b: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_get_random_number_in_range_i(a: i32, b: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CPhysical::GetLightingTotal()` (0x544850, ST0)
    fn physical_get_lighting_total(p: GPtr<u8>) -> f64 = thiscall 0x544850 => unported;
    /// `CEntryExitManager::WeAreInInteriorTransition()` (0x43E400, AL)
    fn entry_exit_we_are_in_interior_transition() -> u8 = cdecl 0x43E400 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, unk, life)` (0x4AB290, `ret 0x1C`)
    fn fx_prt_mult_ctor(m: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, unk: u32, life: u32) = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float time, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall 0x4AA440 => unported;
    /// `SurfaceInfos_c::ProducesFootDust(uint32)` (0x55E8B0, AL)
    fn produces_foot_dust(si: GPtr<u8>, s: u32) -> u8 = thiscall 0x55E8B0 => unported;
    /// `SurfaceInfos_c::LeavesFootsteps(uint32)` (0x55E890, AL)
    fn leaves_footsteps(si: GPtr<u8>, s: u32) -> u8 = thiscall 0x55E890 => unported;
    /// `SurfaceInfos_c::IsShallowWater(uint32)` (0x55E730, AL)
    fn is_shallow_water(si: GPtr<u8>, s: u32) -> u8 = thiscall 0x55E730 => unported;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0)
    fn get_adhesion_group(si: GPtr<u8>, s: u32) -> i32 = thiscall 0x55E5C0 => unported;
    /// `CPed::GetBonePosition(RwV3d& out, ePedBones, bool)` (0x5E4280, `ret 0xC`)
    fn get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: i32, update: u32) = thiscall 0x5E4280 => unported;
    /// `CCullZones::CamNoRain()` (0x72DDB0, AL)
    fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0, AL)
    fn player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool, CVector*)` (0x6EB690, AL)
    fn get_water_level(x: u32, y: u32, z: u32, level: GPtr<u8>, touching: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CPlaceable::GetTopDirection()` (0x41CCB0, hidden return pointer)
    fn get_top_direction(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `Fx_c::TriggerFootSplash(CVector&)` (0x4A1150)
    fn fx_trigger_foot_splash(fx: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x4A1150 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(int32, float, float, CPhysical*, uint8, int32, uint32)` (0x4E2BB0)
    fn ped_audio_add_event(a: GPtr<u8>, id: u32, vol: u32, spd: u32, p: u32, b: u32, c: u32, d: u32) = thiscall 0x4E2BB0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*)` (0x4D15E0)
    fn clump_get_first_association(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D15E0 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*)` (0x4D6AB0)
    fn get_next_association(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6AB0 => unported;
    /// `CPedClothesDesc::GetIsWearingBalaclava()` (0x5A7950, AL)
    fn get_is_wearing_balaclava(d: GPtr<u8>) -> u8 = thiscall 0x5A7950 => unported;
    /// `CEventSoundQuiet::CEventSoundQuiet(CEntity*, float, uint32, const CVector&)` (0x5E05B0, `ret 0x10`)
    fn event_sound_quiet_ctor(ev: GPtr<u8>, e: GPtr<u8>, vol: u32, t: u32, pos: GPtr<u8>) = thiscall 0x5E05B0 => unported;
    /// `CEventSoundQuiet::~CEventSoundQuiet()` (0x5DEA00)
    fn event_sound_quiet_dtor(ev: GPtr<u8>) = thiscall 0x5DEA00 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPed::DoFootLanded(bool, uint8)` (0x5E5380, `ret 8`)
    fn do_foot_landed_call(p: GPtr<u8>, left: u32, wobble: u32) = thiscall 0x5E5380 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn task_mgr_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskSimpleLand::RightFootLanded()` (0x678FE0, AL)
    fn task_land_right_foot_landed(t: GPtr<u8>) -> u8 = thiscall 0x678FE0 => unported;
    /// `CTaskSimpleLand::LeftFootLanded()` (0x679010, AL)
    fn task_land_left_foot_landed(t: GPtr<u8>) -> u8 = thiscall 0x679010 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    fn get_task_use_gun(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPedIntelligence::GetTaskFighting()` (0x600F30)
    fn get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `SurfaceInfos_c::IsStairs(uint32)` (0x55E7B0, AL)
    fn is_stairs(si: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E7B0 => unported;
    /// `CAEPedAudioEntity::Service()` (0x4E2EE0)
    fn ped_audio_service(a: GPtr<u8>) = thiscall 0x4E2EE0 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn get_task_swim(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn hanim_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `FxManager_c::CreateFxSystem(char*, RwV3d*, RwMatrix*, bool)` (0x4A9BE0)
    fn fx_create_system(m: GPtr<u8>, name: u32, pos: GPtr<u8>, mat: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    fn fx_destroy_system(m: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `FxSystem_c::SetLocalParticles(uint8)` (0x4AA910)
    fn fx_set_local_particles(fx: GPtr<u8>, b: u32) = thiscall 0x4AA910 => unported;
    /// `FxSystem_c::CopyParentMatrix()` (0x4AA890)
    fn fx_copy_parent_matrix(fx: GPtr<u8>) = thiscall 0x4AA890 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `CEntity::PruneReferences()` (0x571A90)
    fn prune_references(e: GPtr<u8>) = thiscall 0x571A90 => unported;
    /// `CVisibilityPlugins::GetClumpAlpha(RpClump*)` (0x732B20)
    fn get_clump_alpha(c: GPtr<u8>) -> i32 = cdecl 0x732B20 => unported;
    /// `CVisibilityPlugins::SetClumpAlpha(RpClump*, int32)` (0x732B00)
    fn set_clump_alpha(c: GPtr<u8>, a: i32) = cdecl 0x732B00 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CCollision::ProcessColModels(const CMatrix&, CColModel&, const CMatrix&, CColModel&, CColPoint*, CColPoint*, float*, bool)` (0x4185C0)
    fn process_col_models(m1: GPtr<u8>, c1: GPtr<u8>, m2: GPtr<u8>, c2: GPtr<u8>, pts: u32, a: u32, b: u32, c: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, AL)
    fn group_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPed::ProcessBuoyancy()` (0x5E1FA0)
    fn process_buoyancy(p: GPtr<u8>) = thiscall 0x5E1FA0 => unported;
    /// `CPedIntelligence::ProcessFirst()` (0x6073A0)
    fn intel_process_first(i: GPtr<u8>) = thiscall 0x6073A0 => unported;
    /// `CPedIntelligence::Process()` (0x608260)
    fn intel_process(i: GPtr<u8>) = thiscall 0x608260 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden return pointer)
    fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, cdecl, hidden return pointer)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, hidden return pointer)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CPhysical::SkipPhysics()` (0x5433B0)
    fn physical_skip_physics(p: GPtr<u8>) = thiscall 0x5433B0 => unported;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(p: GPtr<u8>) = thiscall 0x5485E0 => unported;
    /// `CPed::RequestDelayedWeapon()` (0x5E8910)
    fn request_delayed_weapon(p: GPtr<u8>) = thiscall 0x5E8910 => unported;
    /// `CPed::PlayFootSteps()` (0x5E57F0)
    fn play_foot_steps_call(p: GPtr<u8>) = thiscall 0x5E57F0 => unported;
    /// `CPed::CalculateNewVelocity()` (0x5E4C50)
    fn calculate_new_velocity_call(p: GPtr<u8>) = thiscall 0x5E4C50 => unported;
    /// `CPed::UpdatePosition()` (0x5E1B10)
    fn update_position(p: GPtr<u8>) = thiscall 0x5E1B10 => unported;
    /// `CPed::SetMoveAnim()` (virtual, vtable +0x5C)
    fn v_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// `CEntity::FlagToDestroyWhenNextProcessed()` (virtual, vtable +0x54)
    fn v_flag_to_destroy(p: GPtr<u8>) = thiscall vslot 0x54 => unported;
    /// `CLocalisation::Blood()` (0x56D230, AL)
    fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `CShadows::AddPermanentShadow(uint8, RwTexture*, CVector*, float x4, int16 x4, float, uint32, float)` (0x706F60;
    /// every argument a dword on the stack)
    fn add_permanent_shadow(ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, r: u32, g: u32, b: u32, a: u32, dist: u32, time: u32, up: u32) = cdecl 0x706F60 => unported;
    /// `CPopulation::UpdatePedCount(CPed*, uint8)` (0x611570)
    fn update_ped_count(p: GPtr<u8>, b: u32) = cdecl 0x611570 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) = thiscall 0x5EFFE0 => unported;
    /// `CAEWeaponAudioEntity::AddAudioEvent(int32)` (0x4E69F0)
    fn weapon_audio_add_event(a: GPtr<u8>, id: u32) = thiscall 0x4E69F0 => unported;
    /// `CAEPedWeaponAudioEntity::Service()` (0x4E6AE0)
    fn weapon_audio_service(a: GPtr<u8>) = thiscall 0x4E6AE0 => unported;
    /// `CPed::ProcessControl()` (0x5E8CD0)
    fn ped_process_control_call(p: GPtr<u8>) = thiscall 0x5E8CD0 => unported;
    /// `CWeapon::Update(CPed* owner)` (0x73DB40)
    fn weapon_update(w: GPtr<u8>, owner: GPtr<u8>) = thiscall 0x73DB40 => unported;
    /// `CPed::IsPedInControl()` (0x5E3960, AL; the result is discarded by the tail call)
    fn is_ped_in_control(p: GPtr<u8>) -> u8 = thiscall 0x5E3960 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn intel_set_decision_maker_type(i: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540, AL)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CCamera::GetGameCamPosition()` (0x50AE50)
    fn camera_get_game_cam_position(c: GPtr<u8>) -> GPtr<u8> = thiscall 0x50AE50 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool x8)` (0x56BA00, AL)
    fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, b6: u32, b7: u32, b8: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CCopPed::AddCriminalToKill(CPed*)` (0x5DDEB0, `ret 4`)
    fn cop_add_criminal_to_kill_call(cop: GPtr<u8>, criminal: GPtr<u8>) -> i32 = thiscall 0x5DDEB0 => unported;
}

/// The entity position pointer `m_matrix ? &m_matrix->pos : &m_placement.pos`.
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    ent_pos(e).cast()
}

fn f32c(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The active weapon slot `&m_aWeapons[(int8)m_nActiveWeaponSlot]`.
fn active_weapon(p: GPtr<u8>) -> GPtr<u8> {
    let slot = i32::from(rd::<i8>(p, 0x718));
    p.byte_add((slot * 0x1C + 0x5A0) as isize)
}

/// One gun-flash fade step of `CPed::ProcessControl` (0x5E8FD2..0x5E9054, 0x5E9054..0x5E90AB): `alpha` (i16 at
/// `off`) shrinks by `ftol(ts * 0.02f * 1000.0f) * progress` (i16 at `off + 2`), compared unsigned against the
/// 32-bit product, 16-bit result.
fn gun_flash_fade(p: GPtr<u8>, off: isize) {
    let alpha = rd::<i16>(p, off);
    if alpha <= 0 {
        return;
    }
    let prog = rd::<i16>(p, off + 2);
    let t = ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(f32c(0x3CA3_D70A)).fmul_m(1000.0));
    let lim = i32::from(prog).wrapping_mul(t);
    if (i32::from(alpha) as u32) > (lim as u32) {
        let d = t.wrapping_mul(i32::from(prog));
        wr::<i16>(p, off, (i32::from(alpha).wrapping_sub(d)) as i16);
    } else {
        wr::<i16>(p, off, 0);
    }
}

/// `CPed::ProcessControl` (0x5E8CD0). Spec `CPed.md` 0x5E8CD0 steps 1-23, re-read against the bytes:
/// molotov flame FX for players, the early returns (replay ped, attack wave outside area 0 / above z 950), reference
/// pruning, clump alpha fade, the knocked-off-bike collision release (vehicle col models via `ms_modelInfoPtrs`), the
/// head-stuck release, the per-frame flag resets, the two gun-flash fades (unsigned compare against the 32-bit
/// product, 16-bit stores), buoyancy, player wetness, the postponed exit (no weapon-audio service on that path),
/// physics or `SkipPhysics`, the movement chain, blood drips (four `rand()` at their original return addresses),
/// ped count, drugged speech and the chainsaw audio.
#[hook(addr = 0x5E8CD0, conv = thiscall, name = "CPed::ProcessControl", expect = "83 EC 14 53 55", this = "CPed")]
pub fn process_control(this: GPtr<u8>) {
    ped_audio_service(this.byte_add(0x138));

    // 2. molotov flame of a player holding a molotov
    let ty = rd::<i32>(this, 0x598);
    if ty == 0 || ty == 1 {
        let w = active_weapon(this);
        if rd::<i32>(w, 0) == 0x12 {
            let destroy = rd::<u8>(this, 0x474) & 2 != 0
                || (rd::<u8>(this, 0x1C) as i8) >= 0
                || (rd::<u32>(this, 0x40) & 0x100 != 0 && !get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null())
                || rd::<u32>(this, 0x4F4) == 0;
            if destroy {
                let fx = rd::<GPtr<u8>>(w, 0x18);
                if !fx.is_null() {
                    fx_destroy_system(g::<u8>(G_FX_MAN), fx);
                    wr::<u32>(w, 0x18, 0);
                }
            } else if rd::<u32>(w, 0x18) == 0 {
                let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
                let idx = hanim_id_get_index(h, 0x18);
                let arr = hanim_get_matrix_array(h);
                let m = arr.byte_add((idx << 6) as isize);
                let mut zero = [0u32; 3];
                let fx = fx_create_system(g::<u8>(G_FX_MAN), MOLOTOV_FLAME, lp(&mut zero).cast(), m, 0);
                wr::<GPtr<u8>>(w, 0x18, fx);
                if !fx.is_null() {
                    fx_set_local_particles(fx, 1);
                    fx_copy_parent_matrix(rd::<GPtr<u8>>(w, 0x18));
                    fx_play(rd::<GPtr<u8>>(w, 0x18));
                }
            }
        }
    }

    // 3. early returns
    if rd::<u32>(this, 0x478) & 0x200000 != 0 {
        return;
    }
    if rd::<u32>(this, 0x474) & 0x1000 != 0 {
        if g::<i32>(CURR_AREA).read() != 0 {
            return;
        }
        let mut tmp = [0u32; 3];
        let c = find_player_coors(lp(&mut tmp).cast(), -1);
        // fcomp 950.0; test ah, 0x41; je return
        if !x(c.wrapping_index(2).read()).fcom(x(950.0)).test_any(0x41) {
            return;
        }
    }

    // 4. reference pruning
    let fc = g::<u32>(FRAME_COUNTER).read();
    if (u32::from(rd::<u8>(this, 0x20)).wrapping_add(fc)) & 0x1F == 0 {
        prune_references(this);
    }

    // 5. alpha
    let mut a = get_clump_alpha(rd::<GPtr<u8>>(this, 0x18));
    if rd::<u8>(this, 0x470) & 8 != 0 {
        a -= 8;
        if a < 0 {
            a = 0;
        }
    } else if a < 0xFF {
        a += 0x10;
        if a > 0xFF {
            a = 0xFF;
        }
    }
    set_clump_alpha(rd::<GPtr<u8>>(this, 0x18), a);

    // 6. knocked off a bike: release the ignored-collision vehicle once clear of it
    let f2 = rd::<u32>(this, 0x470);
    if (f2 as i32) < 0
        && (rd::<u8>(this, 0x46C) & 1 != 0 || f2 & 0x80000 != 0)
        && rd::<u32>(this, 0x568) == 0
        && f2 & 0x200000 == 0
        && rd::<u32>(this, 0x588) == NO_GROUND_Z
    {
        // fcomp 0.01f; test ah, 5; jp skip: go on Less only
        let m = X87::from_f64_exact(magnitude_sqr(this.byte_add(0x44)));
        if !m.fcom(x(f32c(0x3C23_D70A))).test_parity(5) {
            let veh = rd::<GPtr<u8>>(this, 0x58C);
            if !veh.is_null() {
                let mis = g::<GPtr<u8>>(MODEL_INFO_PTRS);
                let vcol = rd::<GPtr<u8>>(mis.wrapping_index(i32::from(rd::<i16>(veh, 0x22))).read(), 0x14);
                let vmat = rd::<GPtr<u8>>(veh, 0x14);
                let pcol = rd::<GPtr<u8>>(mis.wrapping_index(i32::from(rd::<i16>(this, 0x22))).read(), 0x14);
                let pmat = rd::<GPtr<u8>>(this, 0x14);
                if process_col_models(pmat, pcol, vmat, vcol, COL_POINTS, 0, 0, 0) == 0 {
                    wr::<u32>(this, 0x470, rd::<u32>(this, 0x470) & 0x7FFF_FFFF);
                    wr::<u32>(this, 0x128, 0);
                }
            } else {
                wr::<u32>(this, 0x470, f2 & 0x7FFF_FFFF);
            }
        }
    }

    // 7. head stuck in collision
    let f2 = rd::<u32>(this, 0x470);
    if f2 & 0x200000 != 0 && rd::<u32>(this, 0x478) & 0x100 != 0 {
        let z = x(pos_ptr(this).cast::<f32>().wrapping_index(2).read()).fadd_m(1.5);
        if !z.fcom(x(rd::<f32>(this, 0x588))).test_parity(5) {
            wr::<u32>(this, 0x470, f2 & 0xFFDF_FFFF);
        }
    }

    // 8. per-frame flag resets
    wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & 0xFFFE_FFFD);
    let f2_local = rd::<u32>(this, 0x470) & 0xFFF7_FFFF;
    wr::<u32>(this, 0x470, f2_local);
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FEFF);
    wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) & 0x7FFF_FFFE);
    wr::<u32>(this, 0x478, rd::<u32>(this, 0x478) & 0xFFEF_BEFF);
    wr::<u32>(this, 0x588, NO_GROUND_Z);

    // 9. gun flash fades
    gun_flash_fade(this, 0x504);
    gun_flash_fade(this, 0x508);

    // 10.
    if rd::<u8>(this, 0x46C) & 1 == 0 {
        wr::<u32>(this, 0x470, f2_local & 0xFFDF_FFFF);
    }

    // 11. script peds in the player's group do not check above their head
    if rd::<u8>(this, 0x484) == 2 && find_player_ped(-1) != this {
        let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
        let grp = rd::<i32>(pd, 0x38);
        let m = g::<u8>(GROUP_MEMBERSHIP_0).byte_add(grp.wrapping_mul(0x2D4) as isize);
        if group_is_member(m, this) != 0 {
            wr::<u32>(this, 0x478, rd::<u32>(this, 0x478) & 0xFFFF_FEFF);
        }
    }

    // 12.
    process_buoyancy(this);

    // 13. player wetness
    let pd = rd::<GPtr<u8>>(this, 0x480);
    if !pd.is_null() {
        if rd::<u32>(this, 0x40) & 0x100 != 0 && rd::<u8>(pd, 0x8E) > 0x32 {
            let w = rd::<i8>(pd, 0x8C);
            if w < 0x64 {
                wr::<i8>(pd, 0x8C, w.wrapping_add(1));
            }
        } else {
            let w = rd::<i8>(pd, 0x8C);
            if w > 0 {
                wr::<i8>(pd, 0x8C, w - 1);
            }
        }
    }

    // 14. standing on something processed later: postpone
    if rd::<u8>(this, 0x46C) & 1 != 0 && g::<u8>(FORCE_PROCESS_CONTROL).read() == 0 {
        let s = rd::<GPtr<u8>>(this, 0x568);
        if !s.is_null()
            && ((rd::<u32>(s, 0x1C) >> 5) & 1 != 0 || (rd::<u8>(s, 0x36) & 7 == 2 && rd::<u32>(s, 0x594) == 6))
        {
            wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x40);
            return;
        }
    }

    // 15.
    intel_process_first(rd::<GPtr<u8>>(this, 0x47C));

    // 16. limit the upward speed in the air
    let standing = rd::<u8>(this, 0x46C) & 1 != 0;
    if !standing {
        // fcomp qword 0.25; test ah, 0x41; jne skip
        if !x(rd::<f32>(this, 0x4C)).fcom(X87::from_f64_exact(0.25)).test_any(0x41) {
            if !rd::<GPtr<u8>>(this, 0x480).is_null() {
                wr::<u32>(this, 0x4C, 0x3E80_0000);
            } else {
                let k = sa_core::orig::x87_binary(0x822130, f64::from(f32c(0x3F73_3333)), f64::from(g::<f32>(TIME_STEP).read()));
                let mut out = [0u32; 3];
                let r = vector_mul_scalar(lp(&mut out).cast(), this.byte_add(0x44), k as f32);
                let r = r.cast::<u32>();
                wr::<u32>(this, 0x44, r.read());
                wr::<u32>(this, 0x48, r.wrapping_index(1).read());
                wr::<u32>(this, 0x4C, r.wrapping_index(2).read());
            }
        }
    }

    // 17. physics
    let zero_eq = |v: f32| x(v).fcom(X87::zero()) == Cmp::Equal;
    let ty = rd::<i32>(this, 0x598);
    let skip = ty != 0
        && ty != 1
        && standing
        && zero_eq(rd::<f32>(this, 0x44))
        && zero_eq(rd::<f32>(this, 0x48))
        && zero_eq(rd::<f32>(this, 0x4C))
        && matches!(rd::<i32>(this, 0x534), 0 | 1)
        && zero_eq(rd::<f32>(this, 0x4D8))
        && zero_eq(rd::<f32>(this, 0x4DC))
        && rd::<i32>(this, 0x530) != 0x29
        && rd::<u32>(this, 0x46C) & 0x200 == 0
        && rd::<u32>(this, 0x568) == 0;
    if skip {
        physical_skip_physics(this);
    } else {
        physical_process_control(this);
    }

    // 18. movement chain
    request_delayed_weapon(this);
    play_foot_steps_call(this);
    wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) & 0xFFEF_7FFF);
    intel_process(rd::<GPtr<u8>>(this, 0x47C));
    if rd::<i32>(this, 0x530) != 0x37 {
        calculate_new_velocity_call(this);
    }
    update_position(this);
    v_set_move_anim(this);

    // 19.
    wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) & 0xFFF0_FFFF);

    // 20. blood drips
    if (rd::<u32>(this, 0x46C) & 0x40000 != 0 || rd::<u8>(this, 0x72F) != 0)
        && localisation_blood() != 0
        && rd::<u32>(this, 0x46C) & 0x100 == 0
    {
        let n = rd::<u8>(this, 0x72F);
        if n != 0 {
            wr::<u8>(this, 0x72F, n - 1);
        }
        if g::<u8>(FRAME_COUNTER).read() & 3 == 0 {
            let cam = g::<u8>(THE_CAMERA);
            let cm = rd::<GPtr<u8>>(cam, 0x14);
            let cpos = if cm.is_null() { cam.byte_add(4) } else { cm.byte_add(0x30) };
            let mut d = [0u32; 3];
            let dv = vector_sub(lp(&mut d).cast(), pos_ptr(this), cpos);
            let m2 = X87::from_f64_exact(magnitude_sqr(dv));
            // fcomp 2500.0f; test ah, 5; jp skip: go on Less only
            if !m2.fcom(x(2500.0)).test_parity(5) {
                let r = at_site(0x5E93B1, rand) & 0x7F;
                let m = rd::<GPtr<u8>>(this, 0x14);
                let size = f32_of(xi(r).fmul_m(f32c(0x3AC4_9BA6)).fadd_m(f32c(0x3E19_999A)));
                let r = (at_site(0x5E93D4, rand) & 0x7F) - 0x40;
                let px = f32_of(xi(r).fmul_m(f32c(0x3BE5_6042)).fadd_m(rd::<f32>(m, 0x30)));
                let m = rd::<GPtr<u8>>(this, 0x14);
                let r = (at_site(0x5E93F7, rand) & 0x7F) - 0x40;
                let m3 = rd::<GPtr<u8>>(this, 0x14);
                let py = f32_of(xi(r).fmul_m(f32c(0x3BE5_6042)).fadd_m(rd::<f32>(m, 0x34)));
                let pz = f32_of(x(rd::<f32>(m3, 0x38)).fadd_m(1.0));
                let time = (at_site(0x5E942C, rand) & 0xFFF) + 0x7D0;
                let neg = x(size).fchs().to_f32();
                let mut p = [px, py, pz];
                add_permanent_shadow(
                    1,
                    g::<u32>(BLOOD_POOL_TEX).read(),
                    lp(&mut p).cast(),
                    size.to_bits(),
                    0,
                    0,
                    neg.to_bits(),
                    0xFF,
                    0xC8,
                    0,
                    0,
                    0x4080_0000,
                    time as u32,
                    0x3F80_0000,
                );
            }
        }
    }

    // 21. ped count
    let f1 = rd::<u32>(this, 0x46C);
    if f1 & 0x100 != 0 {
        if rd::<u32>(this, 0x58C) == 0 {
            wr::<u32>(this, 0x46C, f1 & 0xFFFF_FEFF);
        } else {
            update_ped_count(this, 1);
        }
    } else {
        update_ped_count(this, 0);
    }

    // 22. drugged speech
    let fc = g::<u32>(FRAME_COUNTER).read();
    if (u32::from(rd::<u16>(this, 0x20)).wrapping_add(fc)) & 0x3FFF == 0 && rd::<u32>(this, 0x478) & 0x400 != 0 {
        ped_say(this, 0x51, 0, 0x3F80_0000, 0, 0, 0);
    }

    // 23. chainsaw
    if rd::<i32>(active_weapon(this), 0) == 9
        && rd::<i32>(this, 0x530) != 0x10
        && rd::<u32>(this, 0x46C) & 0x100 == 0
        && get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null()
    {
        weapon_audio_add_event(this.byte_add(0x394), 0x99);
    }
    weapon_audio_service(this.byte_add(0x394));
}

/// `CCivilianPed::ProcessControl` (0x5DDBF0): nothing for peds created by mission type 3; else `CPed::ProcessControl`,
/// then (unless postponed or dead) `CWeapon::Update(active, this)`.
#[hook(addr = 0x5DDBF0, conv = thiscall, name = "CCivilianPed::ProcessControl", expect = "56 8B F1 80 BE 84 04 00 00 03", this = "CPed")]
pub fn civilian_process_control(this: GPtr<u8>) {
    if rd::<u8>(this, 0x484) == 3 {
        return;
    }
    ped_process_control_call(this);
    if (rd::<u32>(this, 0x1C) >> 6) & 1 != 0 || rd::<i32>(this, 0x530) == 0x37 {
        return;
    }
    weapon_update(active_weapon(this), this);
}

/// `CCopPed::ProcessControl` (0x5DE160): decision maker 7 becomes 1 while the player is wanted; `CPed::ProcessControl`;
/// unless postponed/dead: `CWeapon::Update(active, this)`; unless dying: speech 0xDC with a targeted object (+0x71C);
/// the dead `field_79D` block (on-screen line of sight from the game camera to `pos + (0, 0, 0.7)` every 32nd frame at
/// phase 17, else `FlagToDestroyWhenNextProcessed`), ported as the bytes have it.
#[hook(addr = 0x5DE160, conv = thiscall, name = "CCopPed::ProcessControl", expect = "83 EC 48 56 6A FF", this = "CPed")]
pub fn cop_process_control(this: GPtr<u8>) {
    let wanted = find_player_wanted(-1);
    if rd::<u32>(wanted, 0x2C) != 0 {
        let intel = rd::<GPtr<u8>>(this, 0x47C);
        if rd::<i32>(intel, 0xB4) == 7 {
            intel_set_decision_maker_type(intel, 1);
        }
    }
    ped_process_control_call(this);
    if (rd::<u32>(this, 0x1C) >> 6) & 1 != 0 || rd::<i32>(this, 0x530) == 0x37 {
        return;
    }
    weapon_update(active_weapon(this), this);
    if rd::<i32>(this, 0x530) == 0x36 {
        return;
    }
    if rd::<u32>(this, 0x71C) != 0 {
        ped_say(this, 0xDC, 0, 0x3F80_0000, 0, 0, 0);
    }
    if rd::<u8>(this, 0x79D) == 0 {
        return;
    }
    if entity_get_is_on_screen(this) == 0 {
        v_flag_to_destroy(this);
        return;
    }
    let fc = g::<u32>(FRAME_COUNTER).read();
    if (u32::from(rd::<u16>(this, 0x20)).wrapping_add(fc)) & 0x1F != 0x11 {
        return;
    }
    // locals: hit entity (+4), offset (0, 0, 0.7f) (+8), target (+0x14), colpoint (+0x20, 0x2C bytes)
    let mut ent = [0u32; 1];
    let mut off = [0u32, 0, 0x3F33_3333];
    let mut target = [0u32; 3];
    let mut cp = [0u32; 11];
    let t = vector_add(lp(&mut target).cast(), pos_ptr(this), lp(&mut off).cast());
    let cam = camera_get_game_cam_position(g::<u8>(THE_CAMERA));
    process_line_of_sight(cam, t, lp(&mut cp).cast(), lp(&mut ent).cast(), 1, 0, 0, 0, 0, 0, 0, 0);
    if ent[0] != 0 {
        v_flag_to_destroy(this);
    }
}

/// `CEmergencyPed::ProcessControl` (0x5DE400): `CPed::ProcessControl`; unless postponed/dead `CWeapon::Update(active,
/// nullptr)`; unless dying a second `Update(active, nullptr)` (slot re-read); type 19 (fireman) ends with a tail call of
/// `CPed::IsPedInControl` (result unused). The spec says 18 or 19; the bytes (`sub eax, 0x12; je ret; dec eax; jne ret`)
/// return for 18.
#[hook(addr = 0x5DE400, conv = thiscall, name = "CEmergencyPed::ProcessControl", expect = "56 8B F1 E8 C8 A8 00 00", this = "CPed")]
pub fn emergency_process_control(this: GPtr<u8>) {
    ped_process_control_call(this);
    if (rd::<u32>(this, 0x1C) >> 6) & 1 != 0 || rd::<i32>(this, 0x530) == 0x37 {
        return;
    }
    weapon_update(active_weapon(this), GPtr::NULL);
    if rd::<i32>(this, 0x530) == 0x36 {
        return;
    }
    weapon_update(active_weapon(this), GPtr::NULL);
    let ty = rd::<i32>(this, 0x598);
    if ty == 0x13 {
        is_ped_in_control(this);
    }
}

/// `CCopPed::AddCriminalToKill(CPed*)` (0x5DDEB0): returns the slot index (EAX) or -1. Rejects null, players, cop /
/// medic / fireman types (6, 18, 19), types >= 24 and mission peds (created-by 2). Clears dead (`health <= 0`,
/// NaN kept) criminals from the five slots (+0x7AC), rejects a criminal whose health is `<= 0`; returns the index of
/// an existing entry; else stores it in the first free slot, registers the reference, sets F4 bit 11, the kill-time
/// `now + 300000` (+0x54C), F2 bit 6, +0x790 = 0.3f, the criminal vehicle's +0x4A6 = 0xFF (word) and +0x42C |= 0x40,
/// and repeats for the partner cop (+0x7A8, result discarded).
#[hook(addr = 0x5DDEB0, conv = thiscall, name = "CCopPed::AddCriminalToKill", expect = "83 EC 08 53 55", this = "CPed")]
pub fn cop_add_criminal_to_kill(this: GPtr<u8>, criminal: GPtr<u8>) -> i32 {
    if criminal.is_null() || ped_is_player(criminal) != 0 {
        return -1;
    }
    let ty = rd::<i32>(criminal, 0x598);
    if ty == 6 || ty == 0x12 || ty == 0x13 || ty >= 0x18 || rd::<u8>(criminal, 0x484) == 2 {
        return -1;
    }
    for i in 0..5 {
        let slot = this.byte_add(0x7AC + i * 4);
        let c = rd::<GPtr<u8>>(slot, 0);
        // fcomp 0; test ah, 0x41; jp keep: keep when health > 0 or NaN
        if !c.is_null() && !x(rd::<f32>(c, 0x540)).fcom(X87::zero()).test_parity(0x41) {
            clean_up_old_reference(c, slot);
            wr::<u32>(slot, 0, 0);
        }
    }
    // fcomp 0; test ah, 0x41; jnp return -1: go on when health > 0 or NaN
    if !x(rd::<f32>(criminal, 0x540)).fcom(X87::zero()).test_parity(0x41) {
        return -1;
    }
    let mut free = -1i32;
    for i in 0..5 {
        let c = rd::<GPtr<u8>>(this, 0x7AC + i as isize * 4);
        if c.is_null() && free == -1 {
            free = i;
        } else if c == criminal {
            return i;
        }
    }
    if free == -1 {
        return -1;
    }
    let slot = this.byte_add(0x7AC + free as isize * 4);
    wr::<GPtr<u8>>(slot, 0, criminal);
    register_reference(criminal, slot);
    wr::<u32>(criminal, 0x478, rd::<u32>(criminal, 0x478) | 0x800);
    wr::<u32>(criminal, 0x54C, g::<u32>(TIME_MS).read().wrapping_add(0x493E0));
    wr::<u32>(criminal, 0x470, rd::<u32>(criminal, 0x470) | 0x40);
    wr::<u32>(criminal, 0x790, 0x3E99_999A);
    let veh = rd::<GPtr<u8>>(criminal, 0x58C);
    if !veh.is_null() {
        wr::<u16>(veh, 0x4A6, 0xFF);
        let veh = rd::<GPtr<u8>>(criminal, 0x58C);
        wr::<u8>(veh, 0x42C, rd::<u8>(veh, 0x42C) | 0x40);
    }
    let partner = rd::<GPtr<u8>>(this, 0x7A8);
    if !partner.is_null() {
        cop_add_criminal_to_kill_call(partner, criminal);
    }
    free
}

/// x87 `FABS`.
fn fabs(v: X87) -> X87 {
    X87::from_f64_exact(v.to_f64().abs())
}

/// `_CIpow` (0x822130) of `0.9f ^ ts` (`fld 0.9f; fld ts; call`), the ST0 result.
fn pow_09_ts() -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(0x822130, f64::from(f32c(0x3F66_6666)), f64::from(g::<f32>(TIME_STEP).read())))
}

/// `clamp1(v)` of 0x5E509D / 0x5E50EA: -1 when `-1 > v`, 1 when `1 < v`, else `v` (NaN kept).
fn clamp1(v: f32) -> f32 {
    if x(-1.0).fcom(x(v)) == Cmp::Greater {
        -1.0
    } else if x(1.0).fcom(x(v)) == Cmp::Less {
        1.0
    } else {
        v
    }
}

/// `_CIasin` (0x821E70) of an f32, the ST0 result.
fn asin(v: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_unary(0x821E70, f64::from(v)))
}

/// `CPed::CalculateNewVelocity` (0x5E4C50). Spec `CPed.md` 0x5E4C50, re-read against the bytes. Part 1 (heading turn,
/// skipped in the air / landing / states 54, 55, 63): `S = rate * DEG2RAD * ts` (f32), `cur = LimitRadianAngle(cur)`
/// stored, the goal kept unrounded on the x87 stack and wrapped by 2pi, `D = g - cur` (f32); the turn accumulator
/// (+0x564) rules for players / NPCs; move state 2/3 for turning NPCs without a gun or fight task; body roll 0.
/// Part 2 (slope pitch/roll from the contact normal, `asin` through `_CIasin`, the dead roll decay for NPCs that the
/// bytes zero right after). Part 3 (anim moving shift, `/ ts` or `* 0.01f` in tiny steps).
#[hook(addr = 0x5E4C50, conv = thiscall, name = "CPed::CalculateNewVelocity", expect = "83 EC 18 53 56", this = "CPed")]
pub fn calculate_new_velocity(this: GPtr<u8>) {
    let ts = g::<f32>(TIME_STEP);
    let ty = rd::<i32>(this, 0x598);
    let npc = ty != 0 && ty != 1;
    let state = rd::<i32>(this, 0x530);
    if rd::<u32>(this, 0x46C) & 0x600 == 0 && state != 0x36 && state != 0x37 && state != 0x3F {
        let s = f32_of(x(rd::<f32>(this, 0x560)).fmul_m(f32c(0x3C8E_FA35)).fmul_m(ts.read()));
        let goal = rd::<f32>(this, 0x55C);
        let cur = limit_radian_angle(rd::<f32>(this, 0x558));
        wr::<f32>(this, 0x558, X87::from_f64_exact(cur).to_f32());
        let mut gv = X87::from_f64_exact(limit_radian_angle(goal));
        let cur = rd::<f32>(this, 0x558);
        if x(cur).fadd_m(PI).fcom(gv) == Cmp::Less {
            gv = gv.fsub_m(TWO_PI);
        } else if x(cur).fsub_m(PI).fcom(gv) == Cmp::Greater {
            gv = gv.fadd_m(TWO_PI);
        }
        let d = f32_of(gv.fsub_m(rd::<f32>(this, 0x558)));
        if !npc {
            wr::<u32>(this, 0x564, 0x3F80_0000);
        } else {
            let dc = x(d).fcom(X87::zero());
            if !dc.c0() && x(rd::<f32>(this, 0x564)).fcom(X87::zero()) == Cmp::Less {
                wr::<u32>(this, 0x564, 0x3DCC_CCCD);
            } else if dc == Cmp::Less && x(rd::<f32>(this, 0x564)).fcom(X87::zero()) == Cmp::Greater {
                wr::<f32>(this, 0x564, x(f32c(0x3DCC_CCCD)).fchs().to_f32());
            }
        }
        let t = fabs(x(rd::<f32>(this, 0x564))).fmul_m(s);
        let mut turned = false;
        if x(d).fcom(t) == Cmp::Greater {
            wr::<f32>(this, 0x558, f32_of(t.fadd_m(rd::<f32>(this, 0x558))));
            let ma = x(ts.read()).fmul_m(f32c(0x3DCC_CCCD)).fadd_m(rd::<f32>(this, 0x564));
            wr::<f32>(this, 0x564, f32_of(ma));
            if ma.fcom(x(1.0)) == Cmp::Greater {
                wr::<u32>(this, 0x564, 0x3F80_0000);
            }
            turned = true;
        } else if x(-1.0).fmul(t).fcom(x(d)) == Cmp::Greater {
            wr::<f32>(this, 0x558, f32_of(x(rd::<f32>(this, 0x558)).fsub(t)));
            let ma = x(ts.read()).fmul_m(f32c(0x3DCC_CCCD)).fsubr_m(rd::<f32>(this, 0x564));
            wr::<f32>(this, 0x564, f32_of(ma));
            if ma.fcom(x(-1.0)) == Cmp::Less {
                wr::<u32>(this, 0x564, 0xBF80_0000);
            }
            turned = true;
        } else if npc && x(f32c(0x3DCC_CCCD)).fmul_m(s).fcom(fabs(x(d))) == Cmp::Less {
            wr::<f32>(this, 0x558, f32_of(x(d).fmul_m(0.5).fadd_m(rd::<f32>(this, 0x558))));
            wr::<f32>(this, 0x564, f32_of(x(rd::<f32>(this, 0x564)).fmul_m(0.5)));
        } else {
            wr::<f32>(this, 0x558, f32_of(x(d).fadd_m(rd::<f32>(this, 0x558))));
            let v = fabs(x(d)).fdiv_m(s);
            let v = if x(f32c(0x3DCC_CCCD)).fcom(v) == Cmp::Greater { x(f32c(0x3DCC_CCCD)) } else { v };
            wr::<f32>(this, 0x564, f32_of(v));
        }
        let ms = rd::<i32>(this, 0x534);
        if npc
            && (ms == 1 || ms == 0)
            && turned
            && get_task_use_gun(rd::<GPtr<u8>>(this, 0x47C)).is_null()
            && get_task_fighting(rd::<GPtr<u8>>(this, 0x47C)).is_null()
        {
            let st = if x(d).fcom(X87::zero()) == Cmp::Greater { 2 } else { 3 };
            wr::<i32>(this, 0x534, st);
        } else {
            let ms = rd::<i32>(this, 0x534);
            if ms == 2 || ms == 3 {
                wr::<i32>(this, 0x534, 1);
            }
        }
        wr::<u32>(this, 0x524, 0);
    }

    // Part 2: slope
    let m = rd::<GPtr<u8>>(this, 0x14);
    let (nx, ny, nz) = (rd::<f32>(this, 0x578), rd::<f32>(this, 0x57C), rd::<f32>(this, 0x580));
    let nf = f32_of(x(nz).fmul_m(rd::<f32>(m, 0x18)).fadd(x(ny).fmul_m(rd::<f32>(m, 0x14))).fadd(x(nx).fmul_m(rd::<f32>(m, 0x10))));
    let nr = f32_of(x(nz).fmul_m(rd::<f32>(m, 8)).fadd(x(ny).fmul_m(rd::<f32>(m, 4))).fadd(x(nx).fmul_m(rd::<f32>(m, 0))));
    let roll_pitch = |this: GPtr<u8>, roll: bool| {
        if roll {
            wr::<f32>(this, 0x520, f32_of(asin(clamp1(nr))));
        }
        let a = asin(clamp1(nf));
        let p = a.fmul(x(1.0).fsub_m(0.75)).fadd(x(0.75).fmul_m(rd::<f32>(this, 0x518)));
        wr::<f32>(this, 0x518, f32_of(p));
    };
    if (rd::<u32>(this, 0x528) >> 3) & 1 != 0 {
        if rd::<i32>(this, 0x530) == 0x36 || rd::<u32>(this, 0x474) & 0x100000 != 0 {
            roll_pitch(this, true);
        } else if rd::<i32>(this, 0x534) >= 4
            && is_stairs(g::<u8>(SURFACE_INFOS), u32::from(rd::<u8>(this, 0xBA))) == 0
            && rd::<u32>(this, 0x478) & 0x4000 == 0
        {
            wr::<u32>(this, 0x520, 0);
            roll_pitch(this, false);
        } else {
            let mut k = 0.0f32;
            let pz = x(rd::<f32>(this, 0x518)).fcom(X87::zero()) == Cmp::Equal;
            let rz = x(rd::<f32>(this, 0x520)).fcom(X87::zero()) == Cmp::Equal;
            if !pz || !rz {
                k = pow_09_ts().to_f32();
            }
            if fabs(x(rd::<f32>(this, 0x518))).fcom(x(f32c(0x3C23_D70A))) == Cmp::Greater {
                wr::<f32>(this, 0x518, f32_of(x(k).fmul_m(rd::<f32>(this, 0x518))));
            } else {
                wr::<u32>(this, 0x518, 0);
            }
            if fabs(x(rd::<f32>(this, 0x520))).fcom(x(f32c(0x3CA3_D70A))) == Cmp::Greater {
                wr::<f32>(this, 0x520, f32_of(x(k).fmul_m(rd::<f32>(this, 0x520))));
            } else {
                wr::<u32>(this, 0x520, 0);
            }
        }
    } else if rd::<i32>(this, 0x534) >= 4 && npc {
        if fabs(x(rd::<f32>(this, 0x520))).fcom(x(f32c(0x3CA3_D70A))) == Cmp::Greater {
            // dead store: the next instruction (0x5E519F) zeroes the roll
            wr::<f32>(this, 0x520, f32_of(pow_09_ts().fmul_m(rd::<f32>(this, 0x520))));
        }
        wr::<u32>(this, 0x520, 0);
    }

    // Part 3: anim moving shift
    let clamp0 = |v: X87| if X87::zero().fcom(v) == Cmp::Greater { X87::zero() } else { v };
    let sf = f32_of(clamp0(x(1.0).fsub(x(nf).fmul_m(nf))).fsqrt());
    let b = clamp0(x(1.0).fsub(x(nr).fmul_m(nr)));
    wr::<u32>(this, 0x550, 0);
    wr::<u32>(this, 0x554, 0);
    let v = x(sf).fmul_m(rd::<f32>(this, 0x4DC));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let vx = v.fmul_m(rd::<f32>(m, 0x10));
    let vy = f32_of(v.fmul_m(rd::<f32>(m, 0x14)));
    wr::<f32>(this, 0x550, f32_of(vx.fadd_m(rd::<f32>(this, 0x550))));
    wr::<f32>(this, 0x554, f32_of(x(vy).fadd_m(rd::<f32>(this, 0x554))));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let w = b.fsqrt().fmul_m(rd::<f32>(this, 0x4D8));
    let wx = w.fmul_m(rd::<f32>(m, 0));
    let wy = f32_of(w.fmul_m(rd::<f32>(m, 4)));
    wr::<f32>(this, 0x550, f32_of(wx.fadd_m(rd::<f32>(this, 0x550))));
    wr::<f32>(this, 0x554, f32_of(x(wy).fadd_m(rd::<f32>(this, 0x554))));
    if x(ts.read()).fcom(x(f32c(0x3C23_D70A))) == Cmp::Less && g::<u8>(SLOW_MOTION).read() == 0 {
        wr::<f32>(this, 0x550, f32_of(x(rd::<f32>(this, 0x550)).fmul_m(f32c(0x3C23_D70A))));
        wr::<f32>(this, 0x554, f32_of(x(rd::<f32>(this, 0x554)).fmul_m(f32c(0x3C23_D70A))));
    } else {
        let r = x(1.0).fdiv_m(ts.read());
        wr::<f32>(this, 0x550, f32_of(r.fmul_m(rd::<f32>(this, 0x550))));
        wr::<f32>(this, 0x554, f32_of(r.fmul_m(rd::<f32>(this, 0x554))));
    }
}

/// `TheCamera` position: `m_matrix ? &m_matrix->pos : &m_placement.pos` (0xB6F03C / 0xB6F02C).
fn camera_pos() -> GPtr<f32> {
    let cam = g::<u8>(THE_CAMERA);
    let m = rd::<GPtr<u8>>(cam, 0x14);
    if m.is_null() { cam.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}

orig! {
    /// 0x5E3630 (`foot_splash_particles`), register convention EBX = ped, EDI = pos, EAX = count, one stack word
    fn call_foot_splash_particles(ped: GPtr<u8>, pos: GPtr<f32>, count: i32, unused: u32) = reg("args = ebx edi eax stack") 0x5E3630 => foot_splash_particles;
    /// 0x5E37C0 (`foot_dust_particles`), register convention ESI = ped, EBX = pos, EAX = count, one stack word
    fn call_foot_dust_particles(ped: GPtr<u8>, pos: GPtr<f32>, count: i32, unused: u32) = reg("args = esi ebx eax stack") 0x5E37C0 => foot_dust_particles;
}

/// The shared head of 0x5E3630 / 0x5E37C0: on screen, and the camera's 2D squared distance to `pos` not above 100
/// (`dy*dy + dx*dx`; `test ah, 0x41; je return`: NaN goes on).
fn foot_fx_visible(ped: GPtr<u8>, pos: GPtr<f32>) -> bool {
    if entity_get_is_on_screen(ped) == 0 {
        return false;
    }
    let c = camera_pos();
    let dx = x(pos.read()).fsub_m(c.read());
    let dy = x(pos.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
    dy.fmul(dy).fadd(dx.fmul(dx)).fcom(x(100.0)).test_any(0x41)
}

/// `FxPrtMult_c(1, 1, 1, 0.1f, 0.15f, 0, 0.15f)` (0x4AB290) on a local.
fn foot_fx_mult(mult: &mut [u32; 8]) {
    fx_prt_mult_ctor(lp(mult).cast(), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3DCC_CCCD, 0x3E19_999A, 0, 0x3E19_999A);
}

/// `(float)rand() * 3.0518509e-05f * k + base` (fild, two fmul, fadd), the x87 value.
fn rand_scaled(site: u32, k: f32) -> X87 {
    let r = at_site(site, rand);
    xi(r).fmul_m(f32c(0x3800_0100)).fmul_m(k)
}

/// 0x5E3630 (unnamed; "AddFootSplashParticles", only caller DoFootLanded 0x5E562B): register arguments EBX = ped,
/// EDI = pos, EAX = count (signed, `jle` skips 0 and below), one stack word the caller pushes (0.15f) and pops that
/// the body never reads; ESI is saved, nothing is returned. When on screen and within 10 m of the camera (2D):
/// `count` times `p = pos` with x/y jittered by `rand()/32767*0.2 - 0.1` (two rand() in that order), two `m_Splash`
/// (`g_fx` +0x30, 0xA9AE30, re-read per call) particles, velocity (0,0,0) then `forward * 1.5f`.
#[hook(addr = 0x5E3630, conv = reg("args = ebx edi eax stack"), name = "FUN_005e3630", expect = "83 EC 44 56 8B CB",
       unverified = "regconv-ped-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "register convention EBX/EDI/EAX + an unread stack word; two rand() per particle pair")]
pub fn foot_splash_particles(ped: GPtr<u8>, pos: GPtr<f32>, count: i32, _unused: u32) {
    if !foot_fx_visible(ped, pos) {
        return;
    }
    let mut mult = [0u32; 8];
    foot_fx_mult(&mut mult);
    let mut n = count;
    if n <= 0 {
        return;
    }
    loop {
        let mut p = [pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read()];
        p[0] = f32_of(rand_scaled(0x5E36CD, f32c(0x3E4C_CCCD)).fadd_m(p[0]).fsub_m(f32c(0x3DCC_CCCD)));
        let ry = rand_scaled(0x5E36F4, f32c(0x3E4C_CCCD));
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let mut v0 = [0u32; 3];
        p[1] = f32_of(ry.fadd_m(p[1]).fsub_m(f32c(0x3DCC_CCCD)));
        let mut v1 = [
            f32_of(x(rd::<f32>(m, 0x10)).fmul_m(1.5)),
            f32_of(x(rd::<f32>(m, 0x14)).fmul_m(1.5)),
            f32_of(x(rd::<f32>(m, 0x18)).fmul_m(1.5)),
        ];
        fx_add_particle(g::<GPtr<u8>>(FX_SPLASH).read(), lp(&mut p).cast(), lp(&mut v0).cast(), 0, lp(&mut mult).cast(), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
        fx_add_particle(g::<GPtr<u8>>(FX_SPLASH).read(), lp(&mut p).cast(), lp(&mut v1).cast(), 0, lp(&mut mult).cast(), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
        n -= 1;
        if n == 0 {
            break;
        }
    }
}

/// 0x5E37C0 (unnamed; "AddFootDustParticles", only caller DoFootLanded 0x5E563F): register arguments ESI = ped,
/// EBX = pos, EAX = count, one unread stack word (the caller pushes 0 and pops it); EDI is saved, nothing is
/// returned. Same visibility test as 0x5E3630, then `ProducesFootDust(contact surface)`; `count` times: p jittered
/// as above, velocity x/y `rand()/32767*0.3 - 0.15`, z 0 (four rand() always), one `m_SmokeII3expand` (0xA9AE24)
/// particle unless the byte 0xB72C70 is set.
#[hook(addr = 0x5E37C0, conv = reg("args = esi ebx eax stack"), name = "FUN_005e37c0", expect = "83 EC 38 57 8B CE",
       unverified = "regconv-ped-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "register convention ESI/EBX/EAX + an unread stack word; four rand() per particle")]
pub fn foot_dust_particles(ped: GPtr<u8>, pos: GPtr<f32>, count: i32, _unused: u32) {
    if !foot_fx_visible(ped, pos) {
        return;
    }
    let mut mult = [0u32; 8];
    foot_fx_mult(&mut mult);
    if produces_foot_dust(g::<u8>(SURFACE_INFOS), u32::from(rd::<u8>(ped, 0xBA))) == 0 {
        return;
    }
    let mut n = count;
    if n <= 0 {
        return;
    }
    loop {
        let mut p = [pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read()];
        p[0] = f32_of(rand_scaled(0x5E387B, f32c(0x3E4C_CCCD)).fadd_m(p[0]).fsub_m(f32c(0x3DCC_CCCD)));
        p[1] = f32_of(rand_scaled(0x5E38A2, f32c(0x3E4C_CCCD)).fadd_m(p[1]).fsub_m(f32c(0x3DCC_CCCD)));
        let vx = f32_of(rand_scaled(0x5E38C9, f32c(0x3E99_999A)).fsub_m(f32c(0x3E19_999A)));
        let vy = f32_of(rand_scaled(0x5E38EC, f32c(0x3E99_999A)).fsub_m(f32c(0x3E19_999A)));
        let mut v = [vx, vy, 0.0];
        if g::<u8>(NO_FOOT_DUST).read() == 0 {
            fx_add_particle(g::<GPtr<u8>>(FX_SMOKE_II3_EXPAND).read(), lp(&mut p).cast(), lp(&mut v).cast(), 0, lp(&mut mult).cast(), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
        }
        n -= 1;
        if n == 0 {
            break;
        }
    }
}

/// `CPed::DoFootLanded(bool leftFoot, uint8 makeWobble)` (0x5E5380). Spec `CPed.md` 0x5E5380, re-read against the
/// bytes: foot point `P` from the foot bone (43 / 53) and `forward * 0.2f` (z: the product stored as f32 first, x/y
/// unrounded); bloody footprints (+0x750 counter, unsigned); footprints on surfaces that leave them within 10 m of the
/// camera (player 5000 ms, others 2000 ms); rain splash and foot dust (the register-argument helpers 0x5E3630 /
/// 0x5E37C0, called through `orig! reg`); wobble; water splash at `pos + 2 * ts * moveSpeed` / water level when
/// `pos.z - 0.4f > level` (submerged), or shallow-water splash `P + (0, 0, 0.3f) + top * k` (0.3f / k read from .data
/// 0x8D21EC / 0x8D21E8). Not reproduced: `CWaterLevel::GetWaterLevel` writes the level into the original's own
/// `leftFoot` argument slot (stack-only).
#[hook(addr = 0x5E5380, conv = thiscall, name = "CPed::DoFootLanded", expect = "83 EC 30 56 8B F1", this = "CPed")]
pub fn do_foot_landed(this: GPtr<u8>, left_foot: u8, make_wobble: u8) {
    if rd::<u32>(this, 0x474) & 0x400 == 0 || rd::<u32>(this, 0x1C) & 0x800000 != 0 {
        return;
    }
    let mut b = [0f32; 3];
    get_bone_position(this, lp(&mut b).cast(), if left_foot != 0 { 0x2B } else { 0x35 }, 0);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let f = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18)];
    let r = [rd::<f32>(m, 0), rd::<f32>(m, 4), rd::<f32>(m, 8)];
    let bz = x(b[2]).fsub_m(f32c(0x3DCC_CCCD));
    let fx2 = x(f[0]).fmul_m(f32c(0x3E4C_CCCD));
    let fy2 = x(f[1]).fmul_m(f32c(0x3E4C_CCCD));
    let fz2 = f32_of(x(f[2]).fmul_m(f32c(0x3E4C_CCCD)));
    let mut p = [f32_of(fx2.fadd_m(b[0])), f32_of(x(b[1]).fadd(fy2)), f32_of(x(fz2).fadd(bz))];
    let pp: GPtr<u8> = lp(&mut p).cast();

    if rd::<u32>(this, 0x46C) & 0x1000_0000 != 0 && localisation_blood() != 0 {
        add_permanent_shadow(
            1,
            g::<u32>(BLOOD_POOL_TEX).read(),
            pp,
            f32_of(x(f[0]).fmul_m(f32c(0x3E85_1EB8))).to_bits(),
            f32_of(x(f[1]).fmul_m(f32c(0x3E85_1EB8))).to_bits(),
            f32_of(x(r[0]).fmul_m(f32c(0x3E0F_5C29))).to_bits(),
            f32_of(x(r[1]).fmul_m(f32c(0x3E0F_5C29))).to_bits(),
            0xFF,
            0xC8,
            0,
            0,
            0x4080_0000,
            0xBB8,
            0x3F80_0000,
        );
        let n = rd::<u32>(this, 0x750);
        if n > 0x14 {
            wr::<u32>(this, 0x750, n - 0x14);
        } else {
            let f1 = rd::<u32>(this, 0x46C);
            wr::<u32>(this, 0x750, 0);
            wr::<u32>(this, 0x46C, f1 & 0xEFFF_FFFF);
        }
    }

    if leaves_footsteps(g::<u8>(SURFACE_INFOS), u32::from(rd::<u8>(this, 0xBA))) != 0 {
        let pos = ent_pos(this);
        let c = camera_pos();
        let dx = x(c.read()).fsub_m(pos.read());
        let dy = x(c.wrapping_index(1).read()).fsub_m(pos.wrapping_index(1).read());
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if d.fcom(x(10.0)) == Cmp::Less {
            let ty = rd::<i32>(this, 0x598);
            let time = if ty == 0 || ty == 1 { 5000 } else { 2000 };
            add_permanent_shadow(
                1,
                g::<u32>(SHADOW_PED_TEX).read(),
                pp,
                f32_of(x(f[0]).fmul_m(f32c(0xBE85_1EB8))).to_bits(),
                f32_of(x(f[1]).fmul_m(f32c(0xBE85_1EB8))).to_bits(),
                f32_of(x(r[0]).fmul_m(f32c(0xBDCC_CCCD))).to_bits(),
                f32_of(x(r[1]).fmul_m(f32c(0xBDCC_CCCD))).to_bits(),
                0x78,
                0xFA,
                0xFA,
                0x32,
                0x4080_0000,
                time,
                0x3F80_0000,
            );
        }
    }

    if x(g::<f32>(WEATHER_RAIN).read()).fcom(x(f32c(0x3DCC_CCCD))) == Cmp::Greater
        && cam_no_rain() == 0
        && player_no_rain() == 0
        && g::<i32>(CURR_AREA).read() == 0
    {
        call_foot_splash_particles(this, pp.cast(), 4, 0x3E19_999A);
    }
    call_foot_dust_particles(this, pp.cast(), 4, 0);

    if make_wobble != 0 {
        wr::<u32>(this, 0x758, 0x40C9_0FDB);
        let sx = x(rd::<f32>(this, 0x550));
        let sy = x(rd::<f32>(this, 0x554));
        let w = sx.fmul(sx).fadd(sy.fmul(sy)).fmul_m(20.0).fadd_m(f32c(0x3ECC_CCCD));
        wr::<f32>(this, 0x75C, f32_of(w));
    }

    let mut s = [0f32; 3];
    if rd::<u32>(this, 0x40) & 0x100 != 0 {
        let pos = ent_pos(this);
        let (px, py, pz) = (pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read());
        let mut w = [0f32; 1];
        get_water_level(px.to_bits(), py.to_bits(), f32_of(x(pz).fadd_m(1.5)).to_bits(), lp(&mut w).cast(), 1, 0);
        let ts = g::<f32>(TIME_STEP).read();
        let ax = f32_of(x(ts).fmul_m(rd::<f32>(this, 0x44)).fdouble());
        let ay = x(ts).fmul_m(rd::<f32>(this, 0x48)).fdouble();
        s[0] = f32_of(x(ax).fadd_m(px));
        s[1] = f32_of(ay.fadd_m(py));
        s[2] = w[0];
        // fcomp level; test ah, 0x41; jne return: splash only when pos.z - 0.4f > level
        if x(pz).fsub_m(f32c(0x3ECC_CCCD)).fcom(x(w[0])) != Cmp::Greater {
            return;
        }
    } else {
        if is_shallow_water(g::<u8>(SURFACE_INFOS), u32::from(rd::<u8>(this, 0xBA))) == 0 {
            return;
        }
        s[0] = p[0];
        s[1] = p[1];
        s[2] = f32_of(x(g::<f32>(SHALLOW_SPLASH_Z).read()).fadd_m(p[2]));
        let mut top = [0f32; 3];
        let t = get_top_direction(this, lp(&mut top).cast()).cast::<f32>();
        let k = g::<f32>(SHALLOW_SPLASH_K);
        let a = x(k.read()).fmul_m(t.read());
        let b2 = x(k.read()).fmul_m(t.wrapping_index(1).read());
        let c2 = f32_of(x(k.read()).fmul_m(t.wrapping_index(2).read()));
        s[0] = f32_of(a.fadd_m(s[0]));
        s[1] = f32_of(x(s[1]).fadd(b2));
        s[2] = f32_of(x(c2).fadd_m(s[2]));
    }
    fx_trigger_foot_splash(g::<u8>(G_FX), lp(&mut s).cast());
    ped_audio_add_event(this.byte_add(0x138), 0x36, 0, 0x3F80_0000, 0, 0x27, 0, 0);
}

/// `20 * log10(mult)` as `fldlg2; fld mult; fyl2x; fmul 20.0f`, stored as f32. `mult` is 0.5f or 1.0f here: FYL2X is
/// not affected by the precision control, so 0.5f gives `-lg2` (the 64-bit FLDLG2 constant, exact) and the fmul rounds
/// `-lg2 * 20` to 24 bits: -6.0205998f (0xC0C0A8C1, computed with exact arithmetic); 1.0f gives +0.
fn skate_volume(mult: f32) -> u32 {
    if mult.to_bits() == 0x3F00_0000 {
        0xC0C0_A8C1
    } else {
        debug_assert!(mult.to_bits() == 0x3F80_0000);
        0
    }
}

/// The step sound (volume, speed) of 0x5E5C8D / 0x5E5DB5: ducking (-18, 0.8); move state 6 (-6, 1.1), 7 (0, 1.2),
/// else (-12, 0.9); anim group 0x45 (not when ducking): -6 and -0.1.
fn step_sound(this: GPtr<u8>, ducking: bool) -> (f32, f32) {
    if ducking {
        return (f32c(0xC190_0000), f32c(0x3F4C_CCCD));
    }
    let (mut vol, mut spd) = match rd::<i32>(this, 0x534) {
        6 => (f32c(0xC0C0_0000), f32c(0x3F8C_CCCD)),
        7 => (0.0, f32c(0x3F99_999A)),
        _ => (f32c(0xC140_0000), f32c(0x3F66_6666)),
    };
    if rd::<i32>(this, 0x4D4) == 0x45 {
        vol = f32_of(x(vol).fsub_m(6.0));
        spd = f32_of(x(spd).fsub_m(f32c(0x3DCC_CCCD)));
    }
    (vol, spd)
}

/// `CPed::PlayFootSteps` (0x5E57F0). Spec `CPed.md` 0x5E57F0, re-read against the bytes (the spec's notes on the
/// gta-reversed draft apply). The `makeWobble` argument of both `DoFootLanded` calls is the dword local whose low
/// byte holds "first association is WALK/RUN/SPRINT" (its upper bytes are uninitialised stack in the original;
/// `DoFootLanded` reads the byte only). `minT` stays unrounded on the x87 stack, `maxT` is stored as f32. The skater
/// volume is `20 * log10(mult)` ([`skate_volume`]). The `CEventSoundQuiet` of the first foot is built on the Rust
/// stack through the original constructor/destructor; the SEH frame is not reproduced.
#[hook(addr = 0x5E57F0, conv = thiscall, name = "CPed::PlayFootSteps", expect = "6A FF 68 78 D9 83 00", this = "CPed")]
pub fn play_foot_steps(this: GPtr<u8>) {
    let first = clump_get_first_association(rd::<GPtr<u8>>(this, 0x18));
    let wrs = u32::from(matches!(rd::<i16>(first, 0x2C), 0..=2));
    let skater = rd::<u32>(this, 0x59C) == g::<u32>(PED_STATS_PTRS).read().wrapping_add(0x820);
    let f1 = rd::<u32>(this, 0x46C);
    if f1 & 0x1000_0000 != 0 {
        let n = rd::<u32>(this, 0x750);
        if n != 0 && n < 0x12C {
            let n = n - 1;
            wr::<u32>(this, 0x750, n);
            if n == 0 {
                wr::<u32>(this, 0x46C, f1 & 0xEFFF_FFFF);
            }
        }
    }
    if rd::<u8>(this, 0x46C) & 1 == 0 {
        return;
    }

    'landing: {
        let mut walk_total = 0f32;
        let mut idle_total = 0f32;
        let mut walk = GPtr::<u8>::NULL;
        let mut a = first;
        loop {
            let fl = rd::<u16>(a, 0x2E);
            if fl & 0x100 != 0 {
                walk_total = f32_of(x(rd::<f32>(a, 0x18)).fadd_m(walk_total));
                walk = a;
            } else if (fl >> 10) & 1 == 0
                && rd::<i16>(a, 0x2C) != 0xDF
                && ((fl as u8 >> 4) & 1 != 0 || rd::<u32>(this, 0x46C) & 0x0400_0000 == 0)
            {
                idle_total = f32_of(x(rd::<f32>(a, 0x18)).fadd_m(idle_total));
            }
            a = get_next_association(a);
            if a.is_null() {
                break;
            }
        }
        if walk.is_null()
            || x(walk_total).fcom(x(0.5)) != Cmp::Greater
            || x(idle_total).fcom(x(1.0)) != Cmp::Less
        {
            break 'landing;
        }
        let total = rd::<f32>(rd::<GPtr<u8>>(walk, 0x14), 0x10);
        let mut min_t = x(total).fmul_m(f32c(0x3D88_8889));
        let mut max_t = f32_of(x(total).fmul_m(0.5).fadd(min_t));
        let ducking = (rd::<u32>(this, 0x46C) >> 0x1A) & 1 != 0;
        if ducking {
            min_t = min_t.fadd_m(f32c(0x3E4C_CCCD));
            max_t = f32_of(x(max_t).fadd_m(f32c(0x3E4C_CCCD)));
        }
        let ct = rd::<f32>(walk, 0x20);
        let ct_dt = || x(rd::<f32>(walk, 0x28)).fsubr_m(rd::<f32>(walk, 0x20));

        if skater {
            let k = if rd::<i16>(walk, 0x2C) == 0 { f32c(0x3F08_8889) } else { f32c(0x3EAA_AAAB) };
            let mult = match get_adhesion_group(g::<u8>(SURFACE_INFOS), u32::from(rd::<u8>(this, 0xBA))) {
                3 => {
                    if at_site(0x5E5A15, rand) & 0x7F != 0 {
                        wr::<f32>(this, 0x4D8, f32_of(x(rd::<f32>(this, 0x4D8)).fmul_m(0.5)));
                        wr::<f32>(this, 0x4DC, f32_of(x(rd::<f32>(this, 0x4DC)).fmul_m(0.5)));
                    }
                    0.5f32
                }
                4 => {
                    if at_site(0x5E59DF, rand) & 0x3F != 0 {
                        wr::<f32>(this, 0x4D8, f32_of(x(rd::<f32>(this, 0x4D8)).fmul_m(f32c(0x3E4C_CCCD))));
                        wr::<f32>(this, 0x4DC, f32_of(x(rd::<f32>(this, 0x4DC)).fmul_m(f32c(0x3E4C_CCCD))));
                    }
                    break 'landing;
                }
                5 => {
                    wr::<f32>(this, 0x4D8, f32_of(x(rd::<f32>(this, 0x4D8)).fmul_m(f32c(0x3E99_999A))));
                    wr::<f32>(this, 0x4DC, f32_of(x(rd::<f32>(this, 0x4DC)).fmul_m(f32c(0x3E99_999A))));
                    break 'landing;
                }
                _ => 1.0f32,
            };
            let spd = if rd::<i16>(walk, 0x2C) != 0 { 0x3F80_0000 } else { 0x3F40_0000 };
            let ct = rd::<f32>(walk, 0x20);
            if x(ct).fcom(X87::zero()) == Cmp::Greater {
                // ct - dt <= 0 (Less / Equal) plays event 56
                if matches!(ct_dt().fcom(X87::zero()), Cmp::Less | Cmp::Equal) {
                    if rd::<u8>(this, 0x1B4) != 0 {
                        ped_audio_add_event(this.byte_add(0x138), 0x38, skate_volume(mult), spd, 0, 0, 0, 0);
                    }
                    break 'landing;
                }
            }
            if x(mult).fcom(x(f32c(0x3E4C_CCCD))) == Cmp::Greater
                && x(rd::<f32>(walk, 0x20)).fcom(x(k)) == Cmp::Greater
                && matches!(ct_dt().fcom(x(k)), Cmp::Less | Cmp::Equal)
                && rd::<u8>(this, 0x1B4) != 0
            {
                let spd = if rd::<i16>(walk, 0x2C) != 0 { 0x3F80_0000 } else { 0x3F40_0000 };
                ped_audio_add_event(this.byte_add(0x138), 0x39, skate_volume(mult), spd, 0, 0, 0, 0);
            }
            break 'landing;
        }

        // first foot: minT <= ct (fcom; Less / Equal) and ct - dt < minT
        let _ = ct;
        let first_foot = matches!(min_t.fcom(x(rd::<f32>(walk, 0x20))), Cmp::Less | Cmp::Equal) && {
            let d = x(rd::<f32>(walk, 0x20)).fsub_m(rd::<f32>(walk, 0x28));
            d.fcom(min_t) == Cmp::Less
        };
        if first_foot {
            if ped_is_player(this) != 0 && !rd::<GPtr<u8>>(this, 0x480).is_null() {
                let bal = get_is_wearing_balaclava(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x480), 4)) != 0;
                let ms = rd::<i32>(this, 0x534);
                let vol: Option<f32> = if ms == 7 {
                    Some(if bal { 65.0 } else { 55.0 })
                } else if ms == 5 || ms == 6 {
                    let pd = rd::<GPtr<u8>>(this, 0x480);
                    let r = rd::<f32>(pd, 0x14);
                    if x(r).fcom(x(2.0)) != Cmp::Less {
                        Some(if bal { 55.0 } else { 45.0 })
                    } else {
                        let v = if bal && x(r).fcom(x(f32c(0x3F8C_CCCD))) == Cmp::Greater {
                            Some(x(r).fsub_m(1.0).fmul_m(20.0).fadd_m(30.0))
                        } else if x(r).fcom(x(1.5)) == Cmp::Greater {
                            Some(x(r).fsub_m(1.0).fmul_m(15.0).fadd_m(30.0))
                        } else {
                            None
                        };
                        match v {
                            Some(v) if v.fcom(X87::zero()) == Cmp::Greater => Some(f32_of(v)),
                            _ => None,
                        }
                    }
                } else {
                    None
                };
                if let Some(vol) = vol {
                    let mut zero = [0u32; 3];
                    let mut ev = [0u32; 16];
                    let evp: GPtr<u8> = lp(&mut ev).cast();
                    event_sound_quiet_ctor(evp, this, vol.to_bits(), 0xFFFF_FFFF, lp(&mut zero).cast());
                    event_group_add(get_event_global_group(), evp, 0);
                    event_sound_quiet_dtor(evp);
                }
            }
            let (vol, spd) = step_sound(this, ducking);
            if rd::<u8>(this, 0x1B4) != 0 {
                ped_audio_add_event(this.byte_add(0x138), 0x36, vol.to_bits(), spd.to_bits(), 0, 0, 0, 0);
            }
            do_foot_landed_call(this, 1, wrs);
            break 'landing;
        }

        // second foot: ct >= maxT and ct - dt < maxT
        if !x(rd::<f32>(walk, 0x20)).fcom(x(max_t)).c0() {
            let d = x(rd::<f32>(walk, 0x28)).fsubr_m(rd::<f32>(walk, 0x20));
            if d.fcom(x(max_t)) == Cmp::Less {
                let (vol, spd) = step_sound(this, ducking);
                if rd::<u8>(this, 0x1B4) != 0 {
                    ped_audio_add_event(this.byte_add(0x138), 0x37, vol.to_bits(), spd.to_bits(), 0, 0, 0, 0);
                }
                do_foot_landed_call(this, 0, wrs);
            }
        }
    }

    // landing
    if rd::<u32>(this, 0x46C) & 0x400 != 0 {
        let t = task_mgr_get_simplest_active_task(rd::<GPtr<u8>>(this, 0x47C).byte_add(4));
        if task_get_type(t) == 0xF2 {
            if task_land_right_foot_landed(t) != 0 {
                do_foot_landed_call(this, 0, 1);
            } else if task_land_left_foot_landed(t) != 0 {
                do_foot_landed_call(this, 1, 1);
            }
        }
    }
}

/// `CPlayerPed::ProcessControl` (0x60EA90), from the bytes (gta-reversed's structure differs, see below). Player data
/// `pd` (+0x480) is re-read wherever the bytes re-read it.
///
/// Order: car-danger counter (byte, unsigned), drunk fade and `SetDrunkBlur(drunk / 255)`, breath refill and the
/// "require breath" flag (pd +0x34 bit 7), `CPed::ProcessControl`, F4 |= 0x100, **return when postponed**
/// (gta-reversed tests `bCanPointGunAtTarget` instead), pad of player 0/1, `CWanted::Update` of `pd->m_Wanted`,
/// `PruneReferences`, minigun spin (info of skill 1; the spin-down also runs when the use-gun task's anim is past
/// its loop end, not only without the task), chainsaw `GetTaskSwim` (result unused), the targeted-object marker
/// (markColor from the vehicle / object health * 0.001f too; other entity types skip the clamp with 1.0f; the colour
/// lerp `ftol((1-c)*255 + c*0)` etc. in that x87 order; `MarkTarget` gets the dword slots whose low bytes are the
/// colours), sprint energy per move state, `CWeapon::Update`, dead/dying exit through `ClearWeaponTarget`, frontend
/// fire-fail sounds, weapon switch, `ProcessAnimGroups`, the look-direction block (`atan2(-front.x, front.y)` through
/// the f64 FPATAN model), the stand-still timer from the **2D** speed, weapon-change lock, last firing time, group
/// behaviour, vehicle of interest, `UpdateRpHAnim` when invisible, the D-pad gang toggle (DPadDown inlined), the water
/// cover percentage (bounding box added to `pos.z`, not `pos.z + 1.5f` as gta-reversed), the group "keep up" chatter
/// (`FindDistanceToNearestMember` called twice, seven members) and the breathing speech in the dark.
#[hook(addr = 0x60EA90, conv = thiscall, name = "CPlayerPed::ProcessControl", expect = "83 EC 34 53 55", this = "CPed")]
pub fn player_process_control(this: GPtr<u8>) {
    let pd = || rd::<GPtr<u8>>(this, 0x480);
    let ts = || g::<f32>(TIME_STEP).read();

    let p = pd();
    let n = rd::<u8>(p, 0x21);
    if n != 0 {
        wr::<u8>(p, 0x21, n - 1);
    }
    let p = pd();
    if rd::<u8>(p, 0x21) == 0 {
        wr::<u32>(p, 0x30, 0);
    }
    let p = pd();
    if rd::<u8>(p, 0x41) != 0 {
        if i32::from(rd::<u8>(p, 0x40)) - 1 > 0 {
            wr::<u8>(p, 0x40, rd::<u8>(p, 0x40).wrapping_sub(1));
        } else {
            wr::<u8>(p, 0x40, 0);
            mblur_clear_drunk_blur();
            wr::<u8>(pd(), 0x41, 0);
        }
    }
    let d = rd::<u8>(pd(), 0x40);
    if d != 0 {
        mblur_set_drunk_blur(f32_of(xi(i32::from(d)).fmul_m(f32c(0x3B80_8081))));
    }
    let p = pd();
    if (rd::<u8>(p, 0x34) as i8) < 0 {
        let m = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(8));
        if m.fcom(x(rd::<f32>(p, 0x44))) == Cmp::Greater {
            let p = pd();
            wr::<f32>(p, 0x44, f32_of(x(ts()).fdouble().fadd_m(rd::<f32>(p, 0x44))));
        }
        let p = pd();
        wr::<u32>(p, 0x34, rd::<u32>(p, 0x34) & 0xFFFF_FF7F);
    }
    let p = pd();
    wr::<u32>(p, 0x34, rd::<u32>(p, 0x34) | 0x80);
    ped_process_control_call(this);
    wr::<u32>(this, 0x478, rd::<u32>(this, 0x478) | 0x100);
    if (rd::<u32>(this, 0x1C) >> 6) & 1 != 0 {
        return;
    }
    let pad = match rd::<i32>(this, 0x598) {
        0 => pad_get_pad(0),
        1 => pad_get_pad(1),
        _ => GPtr::NULL,
    };
    let p = pd();
    wanted_update(if p.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(p, 0) });
    prune_references(this);

    // minigun spin
    if rd::<i32>(active_weapon(this), 0) == 0x26 {
        let info = get_weapon_info(0x26, 1);
        let mut spun_up = false;
        if !get_task_use_gun(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
            let anim = rd::<GPtr<u8>>(get_task_use_gun(rd::<GPtr<u8>>(this, 0x47C)), 0x2C);
            if !anim.is_null() && x(rd::<f32>(anim, 0x28)).fsubr_m(rd::<f32>(anim, 0x20)).fcom(x(rd::<f32>(info, 0x44))) == Cmp::Less {
                spun_up = true;
                let p = pd();
                if x(rd::<f32>(p, 0x64)).fcom(x(f32c(0x3EE6_6666))) == Cmp::Less {
                    wr::<f32>(p, 0x64, f32_of(x(ts()).fmul_m(f32c(0x3CCC_CCCD)).fadd_m(rd::<f32>(p, 0x64))));
                    let p = pd();
                    if x(rd::<f32>(p, 0x64)).fcom(x(f32c(0x3EE6_6666))) == Cmp::Greater {
                        wr::<u32>(p, 0x64, 0x3EE6_6666);
                    }
                }
                let fire = pad_get_weapon(pad, this) != 0
                    && rd::<i32>(active_weapon(this), 0xC) > 0
                    && !x(rd::<f32>(anim, 0x20)).fcom(x(rd::<f32>(info, 0x40))).c0();
                weapon_audio_add_event(this.byte_add(0x394), if fire { 0x96 } else { 0x97 });
            }
        }
        if !spun_up {
            let p = pd();
            if x(rd::<f32>(p, 0x64)).fcom(X87::zero()) == Cmp::Greater {
                wr::<f32>(p, 0x64, f32_of(x(ts()).fmul_m(f32c(0x3B44_9BA6)).fsubr_m(rd::<f32>(p, 0x64))));
                let p = pd();
                if x(rd::<f32>(p, 0x64)).fcom(X87::zero()) == Cmp::Less {
                    wr::<u32>(p, 0x64, 0);
                }
            }
        }
    }
    if rd::<i32>(active_weapon(this), 0) == 9 && rd::<i32>(this, 0x530) != 0x10 && rd::<u32>(this, 0x46C) & 0x100 == 0 {
        get_task_swim(rd::<GPtr<u8>>(this, 0x47C));
    }

    // targeted object marker
    let target = rd::<GPtr<u8>>(this, 0x71C);
    if !target.is_null() {
        let slot = this.byte_add(0x79C);
        let old = rd::<GPtr<u8>>(slot, 0);
        if !old.is_null() {
            clean_up_old_reference(old, slot);
            wr::<u32>(slot, 0, 0);
        }
        let target = rd::<GPtr<u8>>(this, 0x71C);
        let mut eff: [f32; 3];
        let mut c = 1.0f32;
        let ty = rd::<u8>(target, 0x36) & 7;
        let mut clamp = true;
        if ty == 3 {
            let wt = rd::<i32>(active_weapon(this), 0);
            let skill = ped_get_weapon_skill(this);
            let info = get_weapon_info(wt, skill);
            let t = rd::<GPtr<u8>>(this, 0x71C);
            c = f32_of(x(rd::<f32>(t, 0x540)).fdiv_m(rd::<f32>(t, 0x544)));
            let range = X87::from_f64_exact(weapon_info_get_target_head_range(info)).to_f32();
            let mut head = false;
            if ped_is_alive(t) & 0xFF != 0 && rd::<i32>(get_weapon_info(rd::<i32>(active_weapon(this), 0), 1), 0) == 1 {
                let mut tmp = [0u32; 3];
                let dv = vector_sub(lp(&mut tmp).cast(), pos_ptr(t), pos_ptr(this));
                let m2 = X87::from_f64_exact(magnitude_sqr(dv));
                if x(range).fmul_m(range).fcom(m2) == Cmp::Greater {
                    head = true;
                }
            }
            let p = pd();
            if head {
                wr::<u32>(p, 0x70, 5);
                wr::<u32>(pd(), 0x74, 0x3D4C_CCCD);
            } else {
                wr::<u32>(p, 0x70, 3);
                wr::<u32>(pd(), 0x74, 0x3E4C_CCCD);
            }
            let p = pd();
            eff = [rd::<f32>(p, 0x74), rd::<f32>(p, 0x78), rd::<f32>(p, 0x7C)];
            get_transformed_bone_position(t, lp(&mut eff).cast(), rd::<i32>(p, 0x70), 0);
            let in_veh = rd::<u32>(t, 0x46C) & 0x100 != 0;
            let moving = x(c).fcom(X87::zero()) == Cmp::Greater && !in_veh && rd::<i32>(t, 0x534) != 1;
            if moving {
                let mut tmp = [0u32; 3];
                let v = vector_mul_scalar(lp(&mut tmp).cast(), t.byte_add(0x44), ts());
                vector_add_assign(lp(&mut eff).cast(), v);
            } else if in_veh {
                let veh = rd::<GPtr<u8>>(t, 0x58C);
                if !veh.is_null() {
                    let mut s1 = [0u32; 3];
                    let mut s2 = [0u32; 3];
                    let a = vector_add(lp(&mut s1).cast(), veh.byte_add(0x44), veh.byte_add(0x50));
                    let v = vector_mul_scalar(lp(&mut s2).cast(), a, ts());
                    vector_add_assign(lp(&mut eff).cast(), v);
                }
            }
        } else if ty == 2 {
            c = f32_of(x(rd::<f32>(target, 0x4C0)).fmul_m(f32c(0x3A83_126F)));
            let sx = x(rd::<f32>(target, 0x44)).fadd_m(rd::<f32>(target, 0x50));
            let sy = x(rd::<f32>(target, 0x48)).fadd_m(rd::<f32>(target, 0x54));
            let sz = f32_of(x(rd::<f32>(target, 0x4C)).fadd_m(rd::<f32>(target, 0x58)));
            let tx = f32_of(sx.fmul_m(ts()));
            let ty2 = sy.fmul_m(ts());
            let tz = x(sz).fmul_m(ts());
            let pos = pos_ptr(target).cast::<f32>();
            eff = [
                f32_of(x(tx).fadd_m(pos.read())),
                f32_of(ty2.fadd_m(pos.wrapping_index(1).read())),
                f32_of(tz.fadd_m(pos.wrapping_index(2).read())),
            ];
        } else if ty == 4 {
            c = f32_of(x(rd::<f32>(target, 0x154)).fmul_m(f32c(0x3A83_126F)));
            let pos = pos_ptr(target);
            let mut s1 = [0u32; 3];
            let mut s2 = [0u32; 3];
            let v = vector_mul_scalar(lp(&mut s1).cast(), target.byte_add(0x44), ts());
            let r = vector_add(lp(&mut s2).cast(), pos, v).cast::<f32>();
            eff = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
        } else {
            let pos = pos_ptr(target).cast::<f32>();
            eff = [pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read()];
            clamp = false;
        }
        let (r, gg, b) = if !clamp || x(c).fcom(X87::zero()) == Cmp::Greater {
            if clamp && x(c).fcom(x(1.0)) == Cmp::Greater {
                c = 1.0;
            }
            let one_c = x(1.0).fsub_m(c);
            let c0 = x(c).fmul_m(0.0);
            let r = ftol32(one_c.fmul_m(255.0).fadd(c0)) as u8;
            let one_c0 = one_c.fmul_m(0.0);
            let gg = ftol32(x(c).fmul_m(255.0).fadd(one_c0)) as u8;
            let b = ftol32(one_c0.fadd(c0)) as u8;
            (r, gg, b)
        } else {
            (0, 0, 0)
        };
        let pos = pos_ptr(this).cast::<f32>();
        let dx = x(eff[0]).fsub_m(pos.read());
        let dy = x(eff[1]).fsub_m(pos.wrapping_index(1).read());
        let dz = x(eff[2]).fsub_m(pos.wrapping_index(2).read());
        let len = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy)).fsqrt();
        let size = f32_of(len.fmul_m(f32c(0x3CA3_D70A)).fsubr_m(1.0));
        weapon_effects_mark_target(
            rd::<u32>(this, 0x598),
            eff[0].to_bits(),
            eff[1].to_bits(),
            eff[2].to_bits(),
            u32::from(r),
            u32::from(gg),
            u32::from(b),
            0xFF,
            size.to_bits(),
            0,
        );
    }

    // sprint energy
    match rd::<i32>(this, 0x534) {
        0 => {
            if rd::<u32>(this, 0x46C) & 0x100 != 0 {
                let veh = rd::<GPtr<u8>>(this, 0x58C);
                if !veh.is_null() && rd::<i32>(veh, 0x594) != 0xA {
                    handle_sprint_energy(this, 0, 0x3F80_0000);
                }
            }
        }
        6 => {
            let m = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(7));
            let p = pd();
            if m.fcom(x(rd::<f32>(p, 0x18))) == Cmp::Greater {
                let p = pd();
                wr::<f32>(p, 0x18, f32_of(x(ts()).fmul_m(f32c(0x3E19_999A)).fadd_m(rd::<f32>(p, 0x18))));
            }
        }
        7 => {}
        _ => {
            handle_sprint_energy(this, 0, 0x3F80_0000);
        }
    }

    weapon_update(active_weapon(this), this);
    let st = rd::<i32>(this, 0x530);
    if st == 0x37 || st == 0x36 {
        player_clear_weapon_target(this);
        return;
    }

    if !pad.is_null() {
        if pad_weapon_just_down(pad, this) != 0 {
            let wt = rd::<i32>(active_weapon(this), 0);
            if (camera_using_1st_person_weapon_mode(g::<u8>(THE_CAMERA)) == 0 || rd::<i32>(active_weapon(this), 4) == 3)
                && get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null()
            {
                if wt == 0x22 {
                    audio_report_frontend_event(g::<u8>(AUDIO_ENGINE), 0x29, 0, 0x3F80_0000);
                } else if wt == 0x23 || wt == 0x24 {
                    audio_report_frontend_event(g::<u8>(AUDIO_ENGINE), 0x2A, 0, 0x3F80_0000);
                }
            }
        }
        if ped_is_ped_shootable(this) != 0 && rd::<i32>(this, 0x530) != 0x24 {
            let slot = find_player_slot_with_ped_pointer(this);
            if g::<u32>(PLAYERS + 0xB0).byte_add((slot.wrapping_mul(0x190)) as isize).read() == 0 {
                player_process_weapon_switch(this, pad);
            }
        }
    }
    player_process_anim_groups(this);

    // look direction
    let cams = g::<u8>(CAMERA_CAMS);
    let cam = cams.byte_add(i32::from(g::<u8>(CAMERA_ACTIVE_CAM).read()).wrapping_mul(0x238) as isize);
    if !pad.is_null() && rd::<i16>(cam, 0xC) == 4 && rd::<u32>(cam, 0x28) == 0 {
        wr::<u32>(this, 0x748, 0);
        let cam = cams.byte_add(i32::from(g::<u8>(CAMERA_ACTIVE_CAM).read()).wrapping_mul(0x238) as isize);
        let a = x(rd::<f32>(cam, 0x190)).fchs().fpatan(x(rd::<f32>(cam, 0x194)));
        let look_x = X87::from_f64_exact(limit_radian_angle(a.to_f32()));
        let mut look = look_x.to_f32();
        let angle = fabs(look_x.fsub_m(rd::<f32>(this, 0x558))).to_f32();
        if rd::<i32>(this, 0x530) != 0x10
            && x(angle).fcom(x(f32c(0x3F06_0A92))) == Cmp::Greater
            && x(angle).fcom(x(f32c(0x40B8_4E89))) == Cmp::Less
        {
            if x(angle).fcom(x(f32c(0x4027_8D36))) == Cmp::Greater && x(angle).fcom(x(f32c(0x406A_9280))) == Cmp::Less {
                let dir1 = X87::from_f64_exact(limit_radian_angle(f32_of(x(rd::<f32>(this, 0x558)).fsub_m(f32c(0x4027_8D36))))).to_f32();
                let dir2 = X87::from_f64_exact(limit_radian_angle(f32_of(x(rd::<f32>(this, 0x558)).fadd_m(f32c(0x4027_8D36))))).to_f32();
                look = dir1;
                if rd::<u32>(this, 0x73C) != 0x4974_23F0 && rd::<u32>(this, 0x46C) & 0x0400_0000 == 0 {
                    let a1 = fabs(x(dir1).fsub_m(rd::<f32>(this, 0x73C)));
                    let a2 = fabs(x(dir2).fsub_m(rd::<f32>(this, 0x73C)));
                    if a2.fcom(a1) != Cmp::Greater {
                        look = dir2;
                    }
                }
            }
            ped_set_look_flag(this, look.to_bits(), 1, 0);
            let t = ftol32(x(ts()).fmul_m(f32c(0x3CA3_D70A)).fmul_m(1000.0));
            let mut tv = xi(t);
            if t < 0 {
                tv = tv.fadd_m(4_294_967_296.0);
            }
            ped_set_look_timer(this, ftol32(tv.fmul_m(5.0)) as u32);
        } else {
            ped_clear_look_flag(this);
        }
    }
    if rd::<i32>(this, 0x534) == 7 && rd::<u8>(this, 0x46C) & 4 != 0 {
        ped_clear_look_flag(this);
        ped_set_look_timer(this, 0xFA);
    }

    // stand-still timer (2D speed)
    let vx = x(rd::<f32>(this, 0x44));
    let vy = x(rd::<f32>(this, 0x48));
    let sp = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
    let p = pd();
    if sp.fcom(x(f32c(0x3DCC_CCCD))) == Cmp::Less {
        let t = rd::<u32>(p, 0x24);
        let now = g::<u32>(TIME_MS).read();
        if t == 0 {
            wr::<u32>(p, 0x24, now.wrapping_add(0x1F4));
        } else if now > t {
            wr::<u32>(p, 0x34, rd::<u32>(p, 0x34) | 1);
        }
    } else {
        wr::<u32>(p, 0x24, 0);
        let p = pd();
        wr::<u32>(p, 0x34, rd::<u32>(p, 0x34) & 0xFFFF_FFFE);
    }

    if rd::<u8>(pd(), 0x85) != 0 && ped_is_player(this) != 0 {
        let r = pad_get_target(pad_get_pad(0));
        if r == 0 {
            wr::<u8>(pd(), 0x85, 0);
        }
    }
    if rd::<i32>(this, 0x530) != 0xC && rd::<i32>(active_weapon(this), 4) == 1 {
        wr::<u32>(pd(), 0x6C, g::<u32>(TIME_MS).read());
    }
    player_process_group_behaviour(this, pad);
    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        car_ctrl_register_vehicle_of_interest(rd::<GPtr<u8>>(this, 0x58C));
    }
    if (rd::<u8>(this, 0x1C) as i8) >= 0 {
        entity_update_rp_hanim(this);
    }
    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        let p0 = pad_get_pad(0);
        if rd::<u16>(p0, 0x12) != 0 && rd::<u16>(p0, 0x42) == 0 {
            wr::<u8>(pd(), 0x8D, 0);
        } else if pad_is_dpad_up_pressed(pad_get_pad(0)) != 0 {
            wr::<u8>(pd(), 0x8D, 1);
        }
    }

    // water cover
    if rd::<u32>(this, 0x40) & 0x100 != 0 {
        let pos = pos_ptr(this).cast::<f32>();
        let (px, py, pz) = (pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read());
        let lvl = pd().byte_add(0x90);
        if get_water_level(px.to_bits(), py.to_bits(), f32_of(x(pz).fadd_m(1.5)).to_bits(), lvl, 1, 0) != 0 {
            let min_z = f32_of(x(pz).fadd_m(rd::<f32>(entity_get_col_model(this), 8)));
            let max_z = f32_of(x(pz).fadd_m(rd::<f32>(entity_get_col_model(this), 0x14)));
            let p = pd();
            let l = rd::<f32>(p, 0x90);
            if !x(l).fcom(x(max_z)).c0() {
                wr::<u8>(p, 0x8E, 0x64);
            } else if !x(l).fcom(x(min_z)).test_parity(0x41) {
                wr::<u8>(p, 0x8E, 0);
            } else {
                let v = x(l).fsub_m(min_z).fdiv(x(max_z).fsub_m(min_z)).fmul_m(100.0);
                wr::<u8>(p, 0x8E, ftol32(v) as u8);
            }
        } else {
            wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FEFF);
        }
    } else {
        wr::<u8>(pd(), 0x8E, 0);
    }

    // group "keep up" chatter
    if g::<u8>(FRAME_COUNTER).read() & 0x7F == 0 && find_player_vehicle(-1, 0).is_null() {
        let grp = |this: GPtr<u8>| g::<u8>(PED_GROUPS).byte_add(rd::<i32>(rd::<GPtr<u8>>(this, 0x480), 0x38).wrapping_mul(0x2D4) as isize);
        if rd::<u8>(grp(this), 4) != 0 {
            let n = group_count_members_excluding_leader(grp(this).byte_add(8));
            if n > 0
                && X87::from_f64_exact(group_find_distance_to_nearest_member(grp(this), 0)).fcom(x(20.0)) == Cmp::Greater
                && X87::from_f64_exact(group_find_distance_to_nearest_member(grp(this), 0)).fcom(x(100.0)) == Cmp::Less
                && g::<i32>(CURR_AREA).read() == 0
            {
                ped_say(this, if n == 1 { 0x9E } else { 0x9D }, 0, 0x3F80_0000, 0, 0, 0);
                for i in 0..7 {
                    let m = group_get_member(grp(this).byte_add(8), i);
                    if !m.is_null()
                        && X87::from_f64_exact(general_get_random_number_in_range_f(0.0, 1.0)).fcom(x(0.5)) == Cmp::Less
                    {
                        let off = general_get_random_number_in_range_i(0xBB8, 0x1194);
                        ped_say(m, 0x5C, off as u32, 0x3F80_0000, 0, 0, 0);
                    }
                }
            }
        }
    }

    if rd::<u32>(this, 0x46C) & 0x100 == 0
        && !X87::from_f64_exact(physical_get_lighting_total(this)).fcom(x(f32c(0x3D4C_CCCD))).test_parity(0x41)
        && entry_exit_we_are_in_interior_transition() == 0
    {
        ped_say(this, 0x152, 0, 0x3F80_0000, 0, 0, 0);
    }
}

/// `CPed::Render()` (0x5E7680), from the bytes (gta-reversed's version differs in the in-vehicle LOD test and the
/// render-state handling). Players save render state 0x1E (`RwEngineInstance` get/set) and set it to 1; the value is
/// restored only on the normal exit (the in-vehicle "not visible" exits skip the restore, as the original). Skips:
/// `bDontRender`; invisible entities unless 0xC7C728 is set and 0xC7C724 != 2; peds in a vehicle without the
/// 0x2C0 / 0x338 tasks when F1 bit 13 is clear, or (not a bike, not sub type 2, not a player) farther than
/// `(LODDistMultiplier * (type 5 ? 40 : 25))` from the camera. Then the vision-FX light wrappers around
/// `CEntity::Render`, the weapon (`AddWeaponPedForPC` + `ResetGunFlashAlpha` with a gun flash), the goggles clump
/// (bone 5 matrix copied into its frame, position `RwV3dTransformPoints((0, 0.084f, 0))`), the jetpack, F3 bit 29.
#[hook(addr = 0x5E7680, conv = thiscall, name = "CPed::Render", expect = "83 EC 14 53 55", this = "CPed")]
pub fn render(this: GPtr<u8>) {
    let ty = rd::<i32>(this, 0x598);
    let player = ty == 0 || ty == 1;
    let mut rs = [1u32];
    if player {
        rw_render_state_get(0x1E, lp(&mut rs).cast());
        rw_render_state_set(0x1E, 1);
    }
    if rd::<u8>(this, 0x474) & 2 != 0 {
        return;
    }
    if (rd::<u8>(this, 0x1C) as i8) >= 0 && (g::<u8>(0xC7C728).read() == 0 || g::<u32>(0xC7C724).read() == 2) {
        return;
    }
    if rd::<u32>(this, 0x46C) & 0x100 != 0
        && !rd::<GPtr<u8>>(this, 0x58C).is_null()
        && task_mgr_find_active_task_by_type(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), 0x2C0).is_null()
        && task_mgr_find_active_task_by_type(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), 0x338).is_null()
    {
        if rd::<u32>(this, 0x46C) & 0x2000 == 0 {
            return;
        }
        let veh = rd::<GPtr<u8>>(this, 0x58C);
        let vt = rd::<i32>(veh, 0x590);
        if vt != 9 && rd::<i32>(veh, 0x594) != 2 && ped_is_player(this) == 0 {
            let mut tmp = [0u32; 3];
            let dv = vector_sub(lp(&mut tmp).cast(), pos_ptr(this), camera_pos().cast());
            let d2 = X87::from_f64_exact(magnitude_sqr(dv));
            let k = if vt == 5 { 40.0 } else { 25.0 };
            let lk = x(g::<f32>(0xB6F118).read()).fmul_m(k);
            if d2.fcom(lk.fmul(lk)) == Cmp::Greater {
                return;
            }
        }
    }
    if post_effects_is_vision_fx_active() != 0 {
        post_effects_infrared_store_and_set_lights(this);
        post_effects_night_vision_set_lights();
        entity_render(this);
        post_effects_infrared_restore_lights();
    } else {
        entity_render(this);
    }
    let pd = rd::<GPtr<u8>>(this, 0x480);
    let draw_weapon = pd.is_null() || rd::<u32>(pd, 0x34) & 0x1000 != 0;
    if rd::<u32>(this, 0x4F4) != 0
        && draw_weapon
        && (rd::<u32>(this, 0x40) & 0x100 == 0 || get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null())
        && intel_get_task_hold(rd::<GPtr<u8>>(this, 0x47C), 0).is_null()
    {
        visibility_add_weapon_ped_for_pc(this);
        if rd::<i16>(this, 0x504) > 0 || rd::<i16>(this, 0x508) > 0 {
            ped_reset_gun_flash_alpha(this);
        }
    }
    if rd::<u32>(this, 0x4FC) != 0 {
        let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
        let idx = hanim_id_get_index(h, 5);
        let m = hanim_get_matrix_array(h).byte_add((idx << 6) as isize);
        let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x4FC), 4);
        for i in 0..16 {
            wr::<u32>(frame, 0x10 + i * 4, rd::<u32>(m, i * 4));
        }
        let mut v = [0u32, 0x3DAC_0831, 0];
        let vp: GPtr<u8> = lp(&mut v).cast();
        rw_v3d_transform_points(vp, vp, 1, m);
        wr::<u32>(frame, 0x40, v[0]);
        wr::<u32>(frame, 0x44, v[1]);
        wr::<u32>(frame, 0x48, v[2]);
        rw_frame_update_objects(frame);
        rp_clump_render(rd::<GPtr<u8>>(this, 0x4FC));
    }
    if !get_task_jet_pack(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
        jet_pack_render(get_task_jet_pack(rd::<GPtr<u8>>(this, 0x47C)), this);
    }
    wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) | 0x2000_0000);
    if player {
        rw_render_state_set(0x1E, rs[0]);
    }
}

/// The matrix of bone `id` in the hierarchy `h`: `RpHAnimHierarchyGetMatrixArray(h) + RpHAnimIDGetIndex(h, id) * 64`
/// (index first, as the bytes call them).
fn bone_matrix(h: GPtr<u8>, id: i32) -> GPtr<u8> {
    let idx = hanim_id_get_index(h, id);
    hanim_get_matrix_array(h).byte_add((idx << 6) as isize)
}

/// The inlined `CGeneral::GetRandomNumberInRange(lo, hi)` of PreRenderAfterTest: `(float)rand() * (1/32767) * (hi -
/// lo) + lo` with `hi` / `lo` the stored f32 bounds, rand attributed to `site`.
fn rand_range(site: u32, lo: f32, hi: f32) -> X87 {
    xi(at_site(site, rand)).fmul_m(f32c(0x3800_0100)).fmul(x(hi).fsub_m(lo)).fadd_m(lo)
}

/// One random scale of the breathing block: `r = k * speed` (k read from .data), bounds `1 + r` / `1 - r` stored as
/// f32 (recomputed per component, as the bytes do), three components with rand at `sites`.
fn breath_scale(k: usize, speed: f32, sites: [u32; 3]) -> [f32; 3] {
    let mut s = [0f32; 3];
    for (i, site) in sites.iter().enumerate() {
        let r = x(g::<f32>(k).read()).fmul_m(speed);
        let hi = f32_of(x(1.0).fadd(r));
        let lo = f32_of(x(1.0).fsub(r));
        s[i] = f32_of(rand_range(*site, lo, hi));
    }
    s
}

/// `CPed::PreRenderAfterTest()` (0x5E65A0), from the bytes. gta-reversed deviations kept as the bytes have them:
/// the high-quality shadow path only for FX quality 3 or (2 and ped type 0); the low-quality path stores the dummy
/// shadow without the `m_pShadowData` test; dead peds take the ducking flag as the real-time shadow decision; the
/// clothes test compares only model key 0 with "vest" / "torso"; the breathing speed of an open-top driver is the
/// vehicle speed's dot product with its forward vector; the wobble rotates bones 302/301 for **non**-players; the
/// earthquake swing is `GetRandomNumberInRange(-q, q) * 0.0025f`; the rain splash z offset is `radius * 0.75f` (no
/// random), velocity `playerSpeed * 50`; the wet drip offsets `rand*0.6-0.3`, `rand*0.6-0.3`, `rand*1.0-0.8` and alpha
/// `wetness * alpha * 0.01f`. Every `rand()` keeps its original return address.
#[hook(addr = 0x5E65A0, conv = thiscall, name = "CPed::PreRenderAfterTest", expect = "83 EC 70 53 55", this = "CPed")]
pub fn pre_render_after_test(this: GPtr<u8>) {
    let intel = || rd::<GPtr<u8>>(this, 0x47C);
    let clear_slope = || wr::<u32>(this, 0x528, rd::<u32>(this, 0x528) & 0xFFFF_FFF7);
    if !get_task_swim(intel()).is_null() {
        swim_apply_roll_and_pitch(get_task_swim(intel()), this);
        clear_slope();
    } else if !get_task_jet_pack(intel()).is_null() {
        jet_pack_apply_roll_and_pitch(get_task_jet_pack(intel()), this);
        clear_slope();
    }
    if !get_task_in_air(intel()).is_null() {
        task_in_air_678e80(get_task_in_air(intel()), this);
        clear_slope();
    } else if (rd::<u32>(this, 0x528) >> 3) & 1 != 0 {
        ped_ik_pitch_for_slope(this.byte_add(0x50C));
    } else {
        let ty = rd::<i32>(this, 0x598);
        // fcomp 0; test ah, 0x44; jnp skip: skip only when equal
        if ty != 0 && ty != 1 && x(rd::<f32>(this, 0x518)).fcom(X87::zero()) != Cmp::Equal {
            ped_ik_pitch_for_slope(this.byte_add(0x50C));
        }
    }
    wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) | 0x400);
    entity_update_rp_hanim(this);

    // minigun barrel spin
    if g::<u8>(SKIP_PROCESS_THIS_FRAME).read() == 0 {
        let obj = rd::<GPtr<u8>>(this, 0x4F4);
        if !obj.is_null() && !rd::<GPtr<u8>>(this, 0x480).is_null() && rd::<i32>(active_weapon(this), 0) == 0x26 {
            let f = clump_get_frame_from_name(obj, MINIGUN2);
            if !f.is_null() {
                let a = x(g::<f32>(TIME_STEP).read()).fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x480), 0x64)).fmul_m(f32c(0x4265_2EE0));
                rw_matrix_rotate(f.byte_add(0x10), g::<u8>(XAXIS_IK), f32_of(a).to_bits(), 1);
            }
        }
    }

    // shadows
    if (rd::<u8>(this, 0x1C) as i8) < 0 && g::<u16>(SHADOW_STRENGTH).read() != 0 {
        let mut enter_task = GPtr::<u8>::NULL;
        let mut leave = false;
        let tm = intel().byte_add(4);
        if rd::<u32>(this, 0x46C) & 0x100 != 0 {
            if !task_mgr_find_active_task_by_type(tm, 0x2C0).is_null()
                || !task_mgr_find_active_task_by_type(intel().byte_add(4), 0x2BF).is_null()
            {
                leave = true;
            }
        } else {
            enter_task = task_mgr_find_active_task_by_type(tm, 0x2BD);
        }
        let hq = fx_get_fx_quality(g::<u8>(G_FX)) == 3
            || (fx_get_fx_quality(g::<u8>(G_FX)) == 2 && rd::<i32>(this, 0x598) == 0);
        let mut dummy = !hq;
        if hq {
            let mut b = [0f32; 3];
            get_bone_position(this, lp(&mut b).cast(), 0, 0);
            let c = camera_pos();
            let dy = x(b[1]).fsub_m(c.wrapping_index(1).read());
            let dx = x(b[0]).fsub_m(c.read());
            let d = dy.fmul(dy).fadd(dx.fmul(dx));
            // test ah, 0x41; jp skip: go on when Less / Equal
            if !d.fcom(x(g::<f32>(MAX_PED_SHADOW_DIST_SQR).read())).test_parity(0x41)
                && rd::<u32>(this, 0x40) & 0x100 == 0
            {
                let shadable = |v: GPtr<u8>| matches!(rd::<i32>(v, 0x594), 0xA | 9 | 2);
                let mut ok = true;
                if rd::<u32>(this, 0x46C) & 0x100 != 0 {
                    let v = rd::<GPtr<u8>>(this, 0x58C);
                    ok = !v.is_null() && shadable(v);
                }
                if !enter_task.is_null() {
                    let v = rd::<GPtr<u8>>(enter_task, 0xC);
                    ok = !v.is_null() && shadable(v);
                }
                let mut bs = [0f32; 3];
                get_bone_position(this, lp(&mut bs).cast(), 3, 0);
                let draw = if ped_is_alive(this) & 0xFF != 0
                    && x(pos_ptr(this).cast::<f32>().wrapping_index(2).read()).fsub_m(f32c(0x3E4C_CCCD)).fcom(x(bs[2])) != Cmp::Greater
                {
                    ok
                } else {
                    rd::<u32>(this, 0x46C) & 0x0400_0000 != 0
                };
                if draw {
                    realtime_shadow_do_shadow_this_frame(g::<u8>(REALTIME_SHADOW_MAN), this);
                    dummy = rd::<u32>(this, 0x134) == 0;
                }
            }
        }
        if dummy && (rd::<u32>(this, 0x46C) & 0x100 == 0 || leave) {
            let i = g::<i32>(TIMECYC_CURRENT_STORED).read();
            let rdv = |base: usize| g::<u32>(base).wrapping_index(i).read();
            shadows_store_shadow_for_ped_object(this, rdv(0xB79E90), rdv(0xB79E50), rdv(0xB79F90), rdv(0xB79F50), rdv(0xB79F10), rdv(0xB79ED0));
        }
    }

    if rd::<u16>(this, 0x22) == 0 {
        ped_shoulder_bone_rotation(rd::<GPtr<u8>>(this, 0x18));
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x80_0000);
    }

    // breathing (rain on the player / open-top driving)
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
    let mut driving_open = false;
    let mut rain = false;
    let mut wind = [1.0f32];
    let ty = rd::<i32>(this, 0x598);
    if ty == 0 || ty == 1 {
        let p = pos_ptr(this).cast::<u32>();
        let wp: GPtr<u8> = lp(&mut wind).cast();
        if wind_find_wind_modifier(p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), wp, wp) != 0
            && player_no_rain() == 0
        {
            rain = true;
        }
    }
    if rd::<i32>(this, 0x530) == 0x32 {
        let v = rd::<GPtr<u8>>(this, 0x58C);
        if !v.is_null() {
            let vt = rd::<i32>(v, 0x590);
            if vt == 9 || (vt == 0 && vehicle_is_open_top_car(v) != 0) {
                driving_open = true;
            }
        }
    }
    let pd = rd::<GPtr<u8>>(this, 0x480);
    let clothed = !pd.is_null()
        && (rd::<u32>(rd::<GPtr<u8>>(pd, 4), 0) == keygen_get_uppercase_key(STR_VEST)
            || rd::<u32>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x480), 4), 0) == keygen_get_uppercase_key(STR_TORSO));
    if !clothed && (rain || driving_open) {
        let mut speed = 0f32;
        if driving_open {
            let v = rd::<GPtr<u8>>(this, 0x58C);
            let m = rd::<GPtr<u8>>(v, 0x14);
            speed = f32_of(
                x(rd::<f32>(v, 0x4C)).fmul_m(rd::<f32>(m, 0x18)).fadd(x(rd::<f32>(v, 0x48)).fmul_m(rd::<f32>(m, 0x14))).fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(m, 0x10))),
            );
        }
        if rain {
            let a = fabs(x(wind[0]).fsub_m(1.0));
            if a.fcom(x(speed)) == Cmp::Greater {
                speed = f32_of(a);
            }
        }
        let mut s = breath_scale(BREATH_K1, speed, [0x5E6A9B, 0x5E6AE4, 0x5E6B2D]);
        rw_matrix_scale(bone_matrix(h, 4), lp(&mut s).cast(), 1);
        let mut s = breath_scale(BREATH_K2, speed, [0x5E6B98, 0x5E6BE1, 0x5E6C2A]);
        rw_matrix_scale(bone_matrix(h, 0x1F), lp(&mut s).cast(), 1);
        rw_matrix_scale(bone_matrix(h, 0x15), lp(&mut s).cast(), 1);
        if driving_open || get_task_jet_pack(intel()).is_null() {
            rw_matrix_scale(bone_matrix(h, 3), lp(&mut s).cast(), 1);
        }
        let mut s = breath_scale(BREATH_K3, speed, [0x5E6CF2, 0x5E6D3B, 0x5E6D84]);
        rw_matrix_scale(bone_matrix(h, 0x20), lp(&mut s).cast(), 1);
        rw_matrix_scale(bone_matrix(h, 0x16), lp(&mut s).cast(), 1);
    }

    // head removed while talking
    if rd::<u32>(this, 0x46C) & 0x8000 != 0 && rd::<u8>(this, 0x754) == 2 {
        let mut z = [0u32; 3];
        for id in [5, 8, 6, 7] {
            rw_matrix_scale(bone_matrix(h, id), lp(&mut z).cast(), 1);
        }
    }

    // wobble
    let mut axis = [0u32, 0, 0x3F80_0000];
    if x(rd::<f32>(this, 0x758)).fcom(X87::zero()) == Cmp::Greater {
        let angle = f32_of(x(rd::<f32>(this, 0x758)).fsin().fmul_m(g::<f32>(WOBBLE_FACTOR).read()).fchs()).to_bits();
        let w = x(g::<f32>(TIME_STEP).read()).fmul_m(rd::<f32>(this, 0x75C)).fsubr_m(rd::<f32>(this, 0x758));
        wr::<f32>(this, 0x758, f32_of(w));
        let ty = rd::<i32>(this, 0x598);
        if ty != 0 && ty != 1 {
            rw_matrix_rotate(bone_matrix(h, 0x12E), lp(&mut axis).cast(), angle, 1);
            rw_matrix_rotate(bone_matrix(h, 0x12D), lp(&mut axis).cast(), angle, 1);
        }
        rw_matrix_rotate(bone_matrix(h, 0xC9), lp(&mut axis).cast(), angle, 1);
    }

    // earthquake
    let q = g::<f32>(WEATHER_EARTHQUAKE).read();
    if x(q).fcom(X87::zero()) == Cmp::Greater {
        let lo = x(q).fchs().to_f32();
        let angle = f32_of(rand_range(0x5E6FC7, lo, q).fmul_m(f32c(0x3B23_D70A))).to_bits();
        for id in [0x2A, 0x34, 0x21, 0x17, 0x20, 0x16, 0x2B, 0x35, 0x22, 0x18, 5] {
            rw_matrix_rotate(bone_matrix(h, id), lp(&mut axis).cast(), angle, 2);
        }
    }

    // neck blood while talking with the head removed
    if rd::<u32>(this, 0x46C) & 0x8000 != 0
        && rd::<u8>(this, 0x754) == 2
        && rd::<i32>(this, 0x530) != 0x37
        && rd::<u8>(this, 0x478) & 0x20 == 0
        && (g::<u32>(FRAME_COUNTER).read() & 7) as u8 > 3
    {
        let h2 = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
        let m = bone_matrix(h2, 5);
        let mut p = [rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), rd::<u32>(m, 0x38)];
        let mut d = [0u32; 3];
        scalar_mul_vector(lp(&mut d).cast(), f32c(0x3F19_999A), rd::<GPtr<u8>>(this, 0x14).byte_add(0x20));
        fx_add_blood(g::<u8>(G_FX), lp(&mut p).cast(), lp(&mut d).cast(), 0x10, rd::<u32>(this, 0x12C));
    }

    // rain splashes on the ped's col spheres
    if x(g::<f32>(WEATHER_RAIN).read()).fcom(x(f32c(0x3E99_999A))) == Cmp::Greater
        && x(g::<f32>(THE_CAMERA + 0x158).read()).fcom(x(15.0)) == Cmp::Greater
        && rd::<u32>(this, 0x46C) & 0x100 == 0
        && g::<i32>(CURR_AREA).read() == 0
        && x(pos_ptr(this).cast::<f32>().wrapping_index(2).read()).fcom(x(900.0)) == Cmp::Less
        && cam_no_rain() == 0
    {
        let mut t = [0u32; 3];
        let dv = vector_sub(lp(&mut t).cast(), camera_pos().cast(), pos_ptr(this));
        if X87::from_f64_exact(vector_magnitude(dv)).fcom(x(25.0)) == Cmp::Less {
            let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
            ped_mi_animate_ped_col_model_skinned_world(mi, rd::<GPtr<u8>>(this, 0x18));
            let col = rd::<GPtr<u8>>(mi, 0x34);
            let sp = find_player_speed(-1).cast::<f32>();
            let s = [sp.read(), sp.wrapping_index(1).read(), sp.wrapping_index(2).read()];
            let small = |v: f32| {
                let mut a = x(v);
                if a.fcom(X87::zero()) == Cmp::Less {
                    a = a.fchs();
                }
                a.fcom(x(f32c(0x3D4C_CCCD))) != Cmp::Greater
            };
            let st = rd::<i32>(this, 0x530);
            let ok = small(s[0])
                && small(s[1])
                && !matches!(st, 0x2A | 0x36 | 0x37 | 0x10 | 0x11)
                && ped_is_ped_head_above_pos(this, 0x3E99_999A) != 0
                && clump_get_association(rd::<GPtr<u8>>(this, 0x18), 0xA).is_null();
            let data = rd::<GPtr<u8>>(col, 0x2C);
            let n = i32::from(rd::<i16>(data, 0));
            if ok && n > 0 {
                for i in 0..n {
                    let sph = rd::<GPtr<u8>>(data, 8).byte_add((i * 0x14) as isize);
                    if !matches!(rd::<u8>(sph, 0x11), 5 | 6 | 9) {
                        continue;
                    }
                    let mut mult = [0u32; 8];
                    fx_prt_mult_ctor(lp(&mut mult).cast(), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3EB3_3333, 0x3C23_D70A, 0, 0x3CF5_C28F);
                    let mut p = [rd::<f32>(sph, 0), rd::<f32>(sph, 4), rd::<f32>(sph, 8)];
                    p[0] = f32_of(X87::from_f64_exact(general_get_random_number_in_range_f(f32c(0xBDA3_D70A), f32c(0x3DA3_D70A))).fadd_m(p[0]));
                    p[1] = f32_of(X87::from_f64_exact(general_get_random_number_in_range_f(f32c(0xBDA3_D70A), f32c(0x3DA3_D70A))).fadd_m(p[1]));
                    p[2] = f32_of(x(rd::<f32>(sph, 0xC)).fmul_m(0.75).fadd_m(p[2]));
                    let mut v = [f32_of(x(s[0]).fmul_m(50.0)), f32_of(x(s[1]).fmul_m(50.0)), f32_of(x(s[2]).fmul_m(50.0))];
                    fx_add_particle(g::<GPtr<u8>>(FX_SPLASH).read(), lp(&mut p).cast(), lp(&mut v).cast(), 0, lp(&mut mult).cast(), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
                }
            }
        }
    }

    // wet player drips
    let pd = rd::<GPtr<u8>>(this, 0x480);
    if !pd.is_null() && rd::<i8>(pd, 0x8C) > 0 && rd::<u8>(pd, 0x8E) < 0x1E {
        let mut mult = [0u32; 8];
        fx_prt_mult_ctor(lp(&mut mult).cast(), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3E4C_CCCD, 0x3E19_999A, 0, 0x3DCC_CCCD);
        let pp = pos_ptr(this).cast::<f32>();
        let mut p = [pp.read(), pp.wrapping_index(1).read(), pp.wrapping_index(2).read()];
        p[0] = f32_of(rand_scaled(0x5E7570, f32c(0x3F19_999A)).fadd_m(p[0]).fsub_m(f32c(0x3E99_999A)));
        p[1] = f32_of(rand_scaled(0x5E7597, f32c(0x3F19_999A)).fadd_m(p[1]).fsub_m(f32c(0x3E99_999A)));
        p[2] = f32_of(rand_scaled(0x5E75BE, 1.0).fadd_m(p[2]).fsub_m(f32c(0x3F4C_CCCD)));
        let mut v = [0u32; 3];
        let wet = i32::from(rd::<i8>(rd::<GPtr<u8>>(this, 0x480), 0x8C));
        let alpha = f32::from_bits(mult[3]);
        mult[3] = f32_of(xi(wet).fmul_m(alpha).fmul_m(f32c(0x3C23_D70A))).to_bits();
        fx_add_particle(g::<GPtr<u8>>(FX_WATER_SPLASH).read(), lp(&mut p).cast(), lp(&mut v).cast(), 0, lp(&mut mult).cast(), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
    }

    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(this, 0x58C);
        if !v.is_null() {
            wr::<u32>(this, 0x12C, rd::<u32>(v, 0x12C));
        }
    }
}

/// `_ftol2(ts * 0.02f * 1000.0f)`, the frame time in ms as the bytes compute it.
fn dt_ms() -> i32 {
    ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(f32c(0x3CA3_D70A)).fmul_m(1000.0))
}

/// `(float)(uint32)v` as `fild` + the 2^32 correction for negative dwords.
fn u32_to_x(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(4_294_967_296.0) } else { r }
}

/// `FPV()`: `Players[PlayerInFocus].m_pPed && ped.bInVehicle ? ped.m_pVehicle : null` (re-evaluated at each use).
fn fpv() -> GPtr<u8> {
    let pif = i32::from(g::<u8>(PLAYER_IN_FOCUS).read());
    let ped = g::<GPtr<u8>>(PLAYERS).byte_add((pif * 0x190) as isize).read();
    if !ped.is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0 { rd::<GPtr<u8>>(ped, 0x58C) } else { GPtr::NULL }
}

/// `Players[PlayerInFocus].m_pPed`.
fn pif_ped() -> GPtr<u8> {
    let pif = i32::from(g::<u8>(PLAYER_IN_FOCUS).read());
    g::<GPtr<u8>>(PLAYERS).byte_add((pif * 0x190) as isize).read()
}

/// The stunt accumulation tail: `+0x114 = _ftol2(max(0, (float)(uint32)+0x114 - (float)(uint32)dt * k))`.
fn stunt_time_buffer(this: GPtr<u8>, k: f32) {
    let a = u32_to_x(rd::<u32>(this, 0x114));
    let b = u32_to_x(dt_ms() as u32).fmul_m(k);
    let mut t = a.fsub(b);
    if X87::zero().fcom(t) == Cmp::Greater {
        t = X87::zero();
    }
    wr::<i32>(this, 0x114, ftol32(t));
}

/// `+0x114 -= _ftol2(ts * 0.02f * -1000.0f)` (the DEC exits of the stunt counters).
fn stunt_dec(this: GPtr<u8>) {
    let d = ftol32(x(g::<f32>(TIME_STEP).read()).fmul_m(f32c(0x3CA3_D70A)).fmul_m(-1000.0));
    wr::<u32>(this, 0x114, rd::<u32>(this, 0x114).wrapping_sub(d as u32));
}

/// A stunt record: `best_dist = dist; best_time = time; SetNewRecordStat(stat_t, (float)(time / 1000));
/// SetNewRecordStat(stat_d, dist)`.
fn stunt_record(this: GPtr<u8>, time_off: isize, dist_off: isize, best_time: isize, best_dist: isize, stat_t: u32, stat_d: u32) {
    let t = rd::<u32>(this, time_off);
    wr::<u32>(this, best_dist, rd::<u32>(this, dist_off));
    let secs = f32_of(xi((t / 1000) as i32));
    wr::<u32>(this, best_time, t);
    stats_set_new_record_stat(stat_t, secs.to_bits());
    stats_set_new_record_stat(stat_d, rd::<u32>(this, dist_off));
}

/// Block 1 of `CPlayerInfo::Process` (0x56F9A2-0x5700EE): two-wheel / wheelie / stoppie counters.
fn stunt_counters(this: GPtr<u8>) {
    let eq1 = |v: f32| x(v).fcom(x(1.0)) == Cmp::Equal;
    let lt1 = |v: f32| x(v).fcom(x(1.0)) == Cmp::Less;
    let ped = rd::<GPtr<u8>>(this, 0);
    let veh = if rd::<u32>(ped, 0x46C) & 0x100 != 0 { rd::<GPtr<u8>>(ped, 0x58C) } else { GPtr::NULL };
    // exits: Z0 0x5700D0, Z1 0x5700D6, Z2 0x5700E2, Z3 0x5700BC, Z4 0x5700EE
    enum Exit {
        Z0,
        Z1,
        Z2,
        Z3,
        Z4,
    }
    let z = (|| {
        if veh.is_null() {
            return Exit::Z0;
        }
        let w = |o: isize| rd::<f32>(veh, o);
        let undamaged = || x(rd::<f32>(veh, 0xD8)).fcom(X87::zero()) == Cmp::Equal;
        match rd::<i32>(veh, 0x590) {
            0 => {
                if rd::<u8>(veh, 0x960) < 3 {
                    wr::<u32>(this, 0x100, rd::<u32>(this, 0x100).wrapping_add(dt_ms() as u32));
                } else {
                    wr::<u32>(this, 0x100, 0);
                }
                let acc = if eq1(w(0x7EC)) && eq1(w(0x7F0)) {
                    Some(lt1(w(0x7E4)) && lt1(w(0x7E8)) && undamaged())
                } else if eq1(w(0x7E4)) && eq1(w(0x7E8)) {
                    Some(lt1(w(0x7EC)) && lt1(w(0x7F0)) && undamaged())
                } else {
                    None
                };
                let f8 = rd::<u32>(this, 0xF8);
                match acc {
                    Some(true) => {
                        wr::<u32>(this, 0xF8, f8.wrapping_add(dt_ms() as u32));
                        wr::<f32>(this, 0xFC, f32_of(x(rd::<f32>(veh, 0xD4)).fadd_m(rd::<f32>(this, 0xFC))));
                        stunt_time_buffer(this, 0.5);
                        Exit::Z2
                    }
                    Some(false) => {
                        if f8 > 0 && rd::<u32>(this, 0x114) < 0x1F4 {
                            stunt_dec(this);
                            return Exit::Z2;
                        }
                        if f8 >= 0x7D0 {
                            stunt_record(this, 0xF8, 0xFC, 0x118, 0x11C, 0x13, 0xC);
                        }
                        wr::<u32>(this, 0xFC, 0);
                        Exit::Z1
                    }
                    None => {
                        if f8 == 0 {
                            return Exit::Z2;
                        }
                        if f8 >= 0x7D0 {
                            stunt_record(this, 0xF8, 0xFC, 0x118, 0x11C, 0x13, 0xC);
                        }
                        wr::<u32>(this, 0xFC, 0);
                        Exit::Z1
                    }
                }
            }
            9 => {
                // jnp ACC when `w < 1`; else `w2 < 1` and damage intensity == 0
                let acc_test = |a: isize, b: isize| lt1(w(a)) || (lt1(w(b)) && undamaged());
                let acc_tail = |this: GPtr<u8>, cnt: isize, dist: isize| {
                    wr::<u32>(this, cnt, rd::<u32>(this, cnt).wrapping_add(dt_ms() as u32));
                    wr::<f32>(this, dist, f32_of(x(rd::<f32>(veh, 0xD4)).fadd_m(rd::<f32>(this, dist))));
                    stunt_time_buffer(this, f32c(0x3E4C_CCCD));
                    wr::<u32>(this, 0xF8, 0);
                    wr::<u32>(this, 0x100, 0);
                };
                let dec_tail = |this: GPtr<u8>| {
                    stunt_dec(this);
                    wr::<u32>(this, 0xF8, 0);
                    wr::<u32>(this, 0x100, 0);
                };
                let end_rear = |this: GPtr<u8>| {
                    if rd::<u32>(this, 0x104) >= 0x1388 {
                        stunt_record(this, 0x104, 0x108, 0x120, 0x124, 0x11, 0xA);
                    }
                    wr::<u32>(this, 0x104, 0);
                    wr::<u32>(this, 0x108, 0);
                };
                let r_front = |this: GPtr<u8>| {
                    if rd::<u32>(this, 0x10C) >= 0x7D0 {
                        stunt_record(this, 0x10C, 0x110, 0x128, 0x12C, 0x12, 0xB);
                    }
                    wr::<u32>(this, 0x10C, 0);
                    wr::<u32>(this, 0x110, 0);
                };
                if eq1(w(0x720)) && eq1(w(0x724)) && rd::<u32>(this, 0x10C) == 0 {
                    if acc_test(0x728, 0x72C) {
                        acc_tail(this, 0x104, 0x108);
                        return Exit::Z4;
                    }
                    if rd::<u32>(this, 0x104) > 0 && rd::<u32>(this, 0x114) < 0x1F4 {
                        dec_tail(this);
                        return Exit::Z4;
                    }
                    end_rear(this);
                    return Exit::Z3;
                }
                if rd::<u32>(this, 0x104) > 0 {
                    end_rear(this);
                    return Exit::Z3;
                }
                if eq1(w(0x728)) && eq1(w(0x72C)) {
                    if acc_test(0x720, 0x724) {
                        acc_tail(this, 0x10C, 0x110);
                        return Exit::Z4;
                    }
                    if rd::<u32>(this, 0x10C) > 0 && rd::<u32>(this, 0x114) < 0x1F4 {
                        dec_tail(this);
                        return Exit::Z4;
                    }
                }
                r_front(this);
                Exit::Z3
            }
            _ => Exit::Z0,
        }
    })();
    let (z0, z1, z2, z3) = match z {
        Exit::Z0 => (true, true, true, false),
        Exit::Z1 => (false, true, true, false),
        Exit::Z2 => (false, false, true, false),
        Exit::Z3 => (false, false, false, true),
        Exit::Z4 => (false, false, false, false),
    };
    if z3 {
        wr::<u32>(this, 0x114, 0);
        wr::<u32>(this, 0xF8, 0);
        wr::<u32>(this, 0x100, 0);
    }
    if z0 {
        wr::<u32>(this, 0x100, 0);
    }
    if z1 {
        wr::<u32>(this, 0x114, 0);
        wr::<u32>(this, 0xF8, 0);
    }
    if z2 {
        wr::<u32>(this, 0x104, 0);
        wr::<u32>(this, 0x10C, 0);
    }
}

/// `MakeAbortable(ped, 1, null)` of the ped's active task: true when there is no active task or it aborted.
fn active_task_aborted(ped: GPtr<u8>) -> bool {
    let t = task_mgr_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    t.is_null() || task_make_abortable(t, ped, 1, 0) != 0
}

/// `ped.m_pIntelligence->m_TaskMgr.SetTask(task, 3, false)`.
fn set_primary_task(ped: GPtr<u8>, task: GPtr<u8>) {
    task_mgr_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), task, 3, 0);
}

/// `new CTaskComplexLeaveCar(veh, 0, 0, true, false)` (0x34 bytes; null stays null).
fn new_leave_car(veh: GPtr<u8>) -> GPtr<u8> {
    let t = task_operator_new(0x34);
    if t.is_null() { t } else { task_leave_car_ctor(t, veh, 0, 0, 1, 0) }
}

/// `new CTaskComplexEnterCarAsDriver(veh)` (0x50 bytes).
fn new_enter_as_driver(veh: GPtr<u8>) -> GPtr<u8> {
    let t = task_operator_new(0x50);
    if t.is_null() { t } else { task_enter_car_as_driver_ctor(t, veh) }
}

/// Blocks 4-6 of `CPlayerInfo::Process` (0x5702AA-0x570A9E): leaving a vehicle, stealing an object, entering the
/// nearest car. Returns false when the exit gate fails (the caller then clears `m_bTryingToExitCar`).
fn leave_or_enter(this: GPtr<u8>, pad: GPtr<u8>, player_index: i32) -> bool {
    let ped = rd::<GPtr<u8>>(this, 0);
    if pad_get_target(pad) != 0 || rd::<u32>(ped, 0x474) & 0x0400_0000 == 0 || rd::<u8>(ped, 0x478) & 2 != 0 {
        return false;
    }
    if pad_exit_vehicle_just_down(pad) == 0
        && !(rd::<u8>(this, 0xD5) != 0 && pad_get_exit_vehicle(pad) != 0 && rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x46C) & 0x100 != 0)
    {
        return false;
    }
    let ped = rd::<GPtr<u8>>(this, 0);
    wr::<u8>(this, 0xD5, 0);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        if rd::<u32>(this, 0xB0) != 0 {
            return true;
        }
        let under = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x48C);
        if !under.is_null() && bridge_this_is_a_bridge_object_moving_up(i32::from(rd::<i16>(under, 0x22))) != 0 {
            return true;
        }
        let ped = rd::<GPtr<u8>>(this, 0);
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        let st = rd::<u8>(veh, 0x36) >> 3;
        if st == 5 || st == 6 || rd::<i32>(veh, 0x4F8) == 4 {
            return true;
        }
        let t = task_mgr_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
        if !t.is_null() && task_make_abortable(t, rd::<GPtr<u8>>(this, 0), 1, 0) == 0 {
            wr::<u8>(this, 0xD5, 1);
            return true;
        }
        let ped = rd::<GPtr<u8>>(this, 0);
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        if rd::<i32>(veh, 0x590) == 5 {
            let task = new_leave_car(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x58C));
            set_primary_task(rd::<GPtr<u8>>(this, 0), task);
            let ped = rd::<GPtr<u8>>(this, 0);
            wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x80);
            return true;
        }
        if vehicle_can_ped_step_out_car(veh, 0) != 0 || {
            let ped = rd::<GPtr<u8>>(this, 0);
            vehicle_can_ped_jump_out_car(rd::<GPtr<u8>>(ped, 0x58C), ped) != 0
        } {
            let task = new_leave_car(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x58C));
            set_primary_task(rd::<GPtr<u8>>(this, 0), task);
            wr::<u8>(this, 0xD5, 1);
            player_info_give_player_parachute(this);
            return true;
        }
        if rd::<u8>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x58C), 0x36) & 0xF8 == 0 {
            wr::<u8>(this, 0xD5, 1);
        }
        return true;
    }

    // on foot: steal an object
    if rd::<u32>(ped, 0xFC) != 0 {
        return true;
    }
    let obj = player_info_find_object_to_steal(this, ped);
    let s = task_mgr_get_simplest_active_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4));
    let hold = intel_get_task_hold(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C), 0);
    if task_get_type(s) == 0xFE
        || !get_task_fighting(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C)).is_null()
        || (!hold.is_null() && rd::<u32>(hold, 8) != 0)
    {
        return true;
    }
    if !obj.is_null() {
        let ped = rd::<GPtr<u8>>(this, 0);
        let t = task_mgr_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
        if !t.is_null() && task_make_abortable(t, rd::<GPtr<u8>>(this, 0), 1, 0) == 0 {
            return true;
        }
        let task = task_operator_new(0x34);
        let task = if task.is_null() { task } else { task_go_pick_up_entity_ctor(task, obj, 0x51) };
        let mut ev = [0u32; 16];
        let evp: GPtr<u8> = lp(&mut ev).cast();
        event_script_command_ctor(evp, 3, task, 0);
        let pif = pif_ped();
        event_group_add(rd::<GPtr<u8>>(pif, 0x47C).byte_add(0x68), evp, 0);
        event_script_command_dtor(evp);
        return true;
    }

    // nearest car (block 5)
    let mut found = [0u32; 1];
    let mut dist = [0u32; 1];
    let p = pos_ptr(rd::<GPtr<u8>>(this, 0)).cast::<f32>();
    let min_x = f32_of(x(p.read()).fsub_m(10.0));
    let p = pos_ptr(rd::<GPtr<u8>>(this, 0)).cast::<f32>();
    let max_x = f32_of(x(p.read()).fadd_m(10.0));
    let p = pos_ptr(rd::<GPtr<u8>>(this, 0)).cast::<f32>();
    let min_y = f32_of(x(p.wrapping_index(1).read()).fsub_m(10.0));
    let p = pos_ptr(rd::<GPtr<u8>>(this, 0)).cast::<f32>();
    let max_y = f32_of(x(p.wrapping_index(1).read()).fadd_m(10.0));
    let sector = |v: f32| ftol32(X87::from_f64_exact(floor_f(f32_of(x(v).fmul_m(f32c(0x3CA3_D70A)).fadd_m(60.0)))));
    let x0 = sector(min_x);
    let y0 = sector(min_y);
    let x1 = sector(max_x);
    let y1 = sector(max_y);
    world_set_next_scan_code();
    let mut yy = y0;
    while yy <= y1 {
        let mut xx = x0;
        while xx <= x1 {
            let list = world_get_repeat_sector(xx, yy);
            player_info_find_closest_car_sector_list(
                this,
                list,
                rd::<GPtr<u8>>(this, 0),
                min_x.to_bits(),
                min_y.to_bits(),
                max_x.to_bits(),
                max_y.to_bits(),
                lp(&mut dist).cast(),
                lp(&mut found).cast(),
            );
            xx += 1;
        }
        yy += 1;
    }

    // enter car (block 6)
    let mut veh = GPtr::<u8>::from_addr(found[0] as usize);
    if veh.is_null() {
        return true;
    }
    if rd::<i32>(veh, 0x594) == 0xB {
        let tr = rd::<GPtr<u8>>(veh, 0x4C4);
        if !tr.is_null() {
            veh = tr;
        }
    }
    if vehicle_can_be_driven(veh) == 0 {
        return true;
    }
    if !get_task_jet_pack(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C)).is_null() {
        let ped = rd::<GPtr<u8>>(this, 0);
        jet_pack_drop_jet_pack(get_task_jet_pack(rd::<GPtr<u8>>(ped, 0x47C)), ped);
    }
    if rd::<i32>(veh, 0x590) == 5 {
        if rd::<u32>(veh, 0x460) != 0 || !active_task_aborted(rd::<GPtr<u8>>(this, 0)) {
            return true;
        }
        set_primary_task(rd::<GPtr<u8>>(this, 0), new_enter_as_driver(veh));
        return true;
    }
    let ped = rd::<GPtr<u8>>(this, 0);
    let t = task_mgr_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if !t.is_null() {
        let mut swim_door = false;
        if task_get_type(t) == 0x10C && !get_task_swim(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C)).is_null() {
            let m = rd::<u16>(veh, 0x22);
            if matches!(m, 0x1CC | 0x21B | 0x1BF | 0x1A1) {
                let mut door = [0u32; 1];
                let mut pos = [0u32; 3];
                if car_enter_exit_get_nearest_car_door(rd::<GPtr<u8>>(this, 0), veh, lp(&mut pos).cast(), lp(&mut door).cast()) != 0 {
                    let swim = get_task_swim(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C));
                    wr::<u32>(swim, 0x14, pos[0]);
                    wr::<u32>(swim, 0x18, pos[1]);
                    wr::<u32>(swim, 0x1C, pos[2]);
                    wr::<u32>(swim, 0x58, 0x1388);
                    swim_door = true;
                }
            }
        }
        if !swim_door && task_make_abortable(t, rd::<GPtr<u8>>(this, 0), 1, 0) == 0 {
            return true;
        }
    }
    if !g::<GPtr<u8>>(PLAYERS + 0x190).read().is_null() && g::<u8>(PLAYERS_CAN_BE_IN_SEPARATE_CARS).read() == 0 {
        let other = g::<u8>(PLAYERS).byte_add(((player_index.wrapping_add(1)) % 2 * 0x190) as isize);
        let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(other, 0), 0x47C);
        let td = rd::<GPtr<u8>>(intel, 0x14);
        let tp = rd::<GPtr<u8>>(intel, 0x10);
        let type_d = if td.is_null() { 0 } else { task_get_type(td) };
        let mut drv = GPtr::<u8>::NULL;
        let type_p = if tp.is_null() {
            0
        } else {
            let ty = task_get_type(tp);
            if matches!(ty, 0x2BD | 0x2BE | 0x2BF | 0x2CB) {
                drv = rd::<GPtr<u8>>(tp, 0xC);
            }
            ty
        };
        let mut pas = GPtr::<u8>::NULL;
        if type_d == 1 || type_p == 1 || type_d == 0x2C5 || type_p == 0x2C5 {
            let op = rd::<GPtr<u8>>(other, 0);
            let v = rd::<GPtr<u8>>(op, 0x58C);
            if rd::<GPtr<u8>>(v, 0x460) == op {
                drv = v;
            } else {
                pas = v;
            }
        }
        if type_p == 0x2BC {
            pas = rd::<GPtr<u8>>(tp, 0xC);
        }
        if drv == veh {
            let t = task_operator_new(0x50);
            let t = if t.is_null() { t } else { task_enter_car_as_passenger_ctor(t, veh, 0, 0) };
            set_primary_task(rd::<GPtr<u8>>(this, 0), t);
        } else if pas == veh || (drv.is_null() && pas.is_null()) {
            set_primary_task(rd::<GPtr<u8>>(this, 0), new_enter_as_driver(veh));
        }
    } else {
        set_primary_task(rd::<GPtr<u8>>(this, 0), new_enter_as_driver(veh));
    }
    true
}

/// `CPlayerInfo::Process(uint32 playerIndex)` (0x56F8D0), from the bytes and the instruction-level spec
/// (`CPlayerInfo.md`, blocks 0-12; gta-reversed has only a plugin wrapper). Kept as the bytes have them: the stunt
/// "best" fields take the latest qualifying run; the road density remap is applied to the stored value every
/// frame; the parachute stream test skips on NaN; the steal-object event goes to PlayerInFocus's ped; the co-op
/// partner index is the C remainder `(index + 1) % 2`; FPV() is re-evaluated at every use and dereferenced without
/// a null check where the bytes do; with FPV() set the on-foot stats block is skipped (also when the train / speed
/// tests fail); the wanted pointer is dereferenced without a null check; the chase-value position sample is taken when
/// `time / 20000` changes (`mul 0xD1B71759; shr edx, 14`; the spec says 10000, found by the police-gang split run).
/// Not reproduced: the SEH frame around the
/// `new`ed tasks (states only matter for unwinding).
#[hook(addr = 0x56F8D0, conv = thiscall, name = "CPlayerInfo::Process", expect = "6A FF 64 A1 00 00 00 00", this = "CPlayerInfo")]
pub fn player_info_process(this: GPtr<u8>, player_index: i32) {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    let pad = pad_get_pad(player_index);
    let now = || g::<u32>(TIME_MS).read();

    // block 0: taxi fare
    let ped = rd::<GPtr<u8>>(this, 0);
    let taxi = rd::<u8>(this, 0xD4) != 0 && rd::<u32>(ped, 0x46C) & 0x100 != 0 && {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        let m = rd::<u16>(v, 0x22);
        (m == 0x1A4 || m == 0x1B6) && rd::<GPtr<u8>>(v, 0x460) == ped && rd::<u8>(v, 0x484) != 0
    };
    if taxi {
        let timer = rd::<u32>(this, 0xCC);
        let e = now().wrapping_sub(timer);
        if e >= 1000 {
            let s = e / 1000;
            wr::<u32>(this, 0xB8, rd::<u32>(this, 0xB8).wrapping_add(s));
            wr::<u32>(this, 0xCC, timer.wrapping_add(s * 1000));
        }
    } else {
        wr::<u32>(this, 0xCC, now());
    }

    // block 1
    stunt_counters(this);

    // block 2
    player_info_work_out_energy_from_hunger(this);
    let disp = rd::<i32>(this, 0xBC);
    let money = rd::<i32>(this, 0xB8);
    if disp != money {
        let diff = money.wrapping_sub(disp);
        let a = diff.wrapping_abs();
        let step = if a > 100_000 {
            12345
        } else if a > 10_000 {
            1234
        } else if a > 1000 {
            123
        } else if a <= 50 {
            1
        } else {
            42
        };
        wr::<i32>(this, 0xBC, if diff >= 0 { disp.wrapping_add(step) } else { disp.wrapping_sub(step) });
    }
    let mh = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(9)).to_f32();
    wr::<f32>(rd::<GPtr<u8>>(this, 0), 0x544, mh);
    let ped = rd::<GPtr<u8>>(this, 0);
    wr::<u8>(this, 0x14F, ftol32(x(rd::<f32>(ped, 0x544))) as u8);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && !X87::from_f64_exact(get_stat_value(0xDF)).fcom(x(400.0)).c0() {
        player_info_stream_parachute_weapon(this, 1);
    } else if rd::<u8>(this, 0x188) != 0 {
        game_logic_is_coop_game_going_on();
        if rd::<u8>(this, 0x188) != 0 {
            set_model_is_deletable(0x173);
            wr::<u8>(this, 0x188, 0);
            wr::<u32>(this, 0x18C, 0);
        }
    }

    // block 3: road density
    if g::<u8>(FRAME_COUNTER).read() & 0xF == 0 {
        let ped = rd::<GPtr<u8>>(this, 0);
        let e = if rd::<u32>(ped, 0x46C) & 0x100 != 0 { rd::<GPtr<u8>>(ped, 0x58C) } else { ped };
        let p = pos_ptr(e).cast::<u32>();
        let d = paths_calc_road_density(g::<u8>(THE_PATHS), p.read(), p.wrapping_index(1).read());
        wr::<f32>(this, 0x134, X87::from_f64_exact(d).to_f32());
    }
    let mut d = x(rd::<f32>(this, 0x134)).fsub_m(1.0).fmul_m(f32c(0x3F19_999A)).fadd_m(1.0);
    wr::<f32>(this, 0x134, f32_of(d));
    if x(0.5).fcom(d) == Cmp::Greater {
        d = x(0.5);
    }
    wr::<f32>(this, 0x134, f32_of(d));
    if x(f32c(0x3FB9_999A)).fcom(d) == Cmp::Less {
        d = x(f32c(0x3FB9_999A));
    }
    wr::<f32>(this, 0x134, f32_of(d));

    // blocks 4-6
    if !leave_or_enter(this, pad, player_index) {
        wr::<u8>(this, 0xD5, 0);
    }

    // block 7: remote-vehicle explosion fade
    if rd::<u8>(this, 0xDD) != 0 {
        let t0 = rd::<u32>(this, 0xE0);
        let ep = g::<u32>(TIME_PREV_MS).read().wrapping_sub(t0);
        let en = now().wrapping_sub(t0);
        let cam = g::<u8>(THE_CAMERA);
        if ep < 1000 && en >= 1000 && rd::<u8>(this, 0xDC) == 0 && rd::<u8>(this, 0xDF) != 0 {
            camera_set_fade_colour(cam, 0, 0, 0);
            camera_fade(cam, 0x3F80_0000, 0);
        }
        if en > 2000 {
            if rd::<u8>(this, 0xDC) == 0 && rd::<u8>(this, 0xDF) != 0 {
                camera_restore_with_jump_cut(cam);
                camera_set_fade_colour(cam, 0, 0, 0);
                camera_fade(cam, 0x3F80_0000, 1);
                camera_process(cam);
                timer_stop();
                renderer_request_objects_in_frustum(0, 0);
                streaming_load_all_requested_models(0);
                timer_update();
            }
            wr::<u8>(this, 0xDD, 0);
            let pif = i32::from(g::<u8>(PLAYER_IN_FOCUS).read());
            let slot = g::<u8>(PLAYERS + 0xB0).byte_add((pif * 0x190) as isize);
            let r = rd::<GPtr<u8>>(slot, 0);
            if !r.is_null() {
                wr::<u32>(r, 0x1C, rd::<u32>(r, 0x1C) | 0x800);
            }
            wr::<u32>(slot, 0, 0);
            let p = pif_ped();
            if !p.is_null() && rd::<u32>(p, 0x46C) & 0x100 != 0 && rd::<u32>(p, 0x58C) != 0 {
                let v = fpv();
                wr::<u8>(v, 0x36, rd::<u8>(v, 0x36) & 7);
            }
        }
    }

    // block 8: upside down
    if g::<u8>(FRAME_COUNTER).read() & 0x1F == 0 {
        let ok = !fpv().is_null()
            && rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x46C) & 0x100 != 0
            && x(rd::<f32>(rd::<GPtr<u8>>(fpv(), 0x14), 0x28)).fcom(X87::zero()) == Cmp::Less
            && X87::from_f64_exact(vector_magnitude(fpv().byte_add(0x44))).fcom(x(f32c(0x3D4C_CCCD))) == Cmp::Less
            && (rd::<i32>(fpv(), 0x590) == 0 || rd::<i32>(fpv(), 0x590) == 5)
            && rd::<u32>(fpv(), 0x40) & 0x100 == 0;
        if ok {
            let n = rd::<u32>(this, 0xF0);
            let inc = if x(rd::<f32>(rd::<GPtr<u8>>(fpv(), 0x14), 0x28)).fcom(x(-0.5)) == Cmp::Less { 2 } else { 1 };
            wr::<u32>(this, 0xF0, n.wrapping_add(inc));
        } else {
            wr::<u32>(this, 0xF0, 0);
        }
        if rd::<u32>(this, 0xF0) > 6 && rd::<u8>(fpv(), 0x42A) & 0x20 != 0 {
            let h = if x(rd::<f32>(fpv(), 0x4C0)).fcom(x(249.0)) == Cmp::Greater { 249.0 } else { rd::<f32>(fpv(), 0x4C0) };
            wr::<f32>(fpv(), 0x4C0, h);
            if rd::<i32>(fpv(), 0x590) == 0 {
                damage_manager_set_engine_status(fpv().byte_add(0x5A0), 0xE1);
                wr::<u32>(fpv(), 0x928, 0);
            }
        }
    }

    // blocks 9 / 10: driving and on-foot stats
    if !fpv().is_null() {
        let speed = || rd::<u32>(fpv(), 0xD4);
        if rd::<u16>(fpv(), 0x22) == 0x1C9 {
            stats_increment_stat(7, speed());
        } else if rd::<i32>(fpv(), 0x594) == 0xA {
            stats_increment_stat(0x1B, speed());
        } else {
            for (a, stat) in [(3, 8), (5, 9), (1, 4), (2, 5), (4, 6)] {
                if vehicle_get_vehicle_appearance(fpv()) == a {
                    stats_increment_stat(stat, speed());
                }
            }
            if (vehicle_get_vehicle_appearance(fpv()) == 5 || vehicle_get_vehicle_appearance(fpv()) == 3)
                && X87::from_f64_exact(vector_magnitude(fpv().byte_add(0x44))).fcom(x(f32c(0x3E4C_CCCD))) == Cmp::Greater
            {
                stats_increment_stat(0xA9, f32_of(x(g::<f32>(TIME_STEP).read()).fmul_m(16.0)).to_bits());
            }
        }
        if find_player_train(-1).is_null() {
            let v = fpv();
            let (vx, vy, vz) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)), x(rd::<f32>(v, 0x4C)));
            if vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fcom(X87::zero()) == Cmp::Greater {
                match vehicle_get_vehicle_appearance(fpv()) {
                    2 => {
                        if rd::<i32>(fpv(), 0x594) != 0xA {
                            stats_update_when_on_motor_bike(fpv());
                        }
                    }
                    3 | 5 => stats_update_when_flying(fpv()),
                    4 => {}
                    _ => stats_update_when_driving(fpv()),
                }
            }
        }
    } else {
        let p = pif_ped();
        if x(rd::<f32>(p, 0xD4)).fcom(X87::zero()) == Cmp::Greater {
            let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C);
            if !get_task_swim(intel).is_null() {
                if pad_get_ped_walk_left_right(pad_get_pad(0)) as i16 != 0 || pad_get_accelerate(pad_get_pad(0)) as i16 != 0 {
                    stats_increment_stat(0x1A, rd::<u32>(pif_ped(), 0xD4));
                }
            } else if !get_task_jet_pack(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C)).is_null() {
                stats_increment_stat(0xAD, f32_of(x(g::<f32>(TIME_STEP).read()).fmul_m(16.0)).to_bits());
            } else {
                stats_increment_stat(3, rd::<u32>(pif_ped(), 0xD4));
            }
        }
    }

    // block 11: chase value
    let pd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x480);
    let w = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
    if rd::<u32>(w, 0x2C) == 0 || scripts_is_player_on_a_mission() != 0 {
        wr::<u32>(this, 0x148, 0);
    } else {
        let f = g::<u32>(CHASE_INIT);
        if f.read() & 1 == 0 {
            f.write(f.read() | 1);
        }
        // `mul 0xD1B71759; shr edx, 14`: a 20-second bucket (the spec says 10000; the shift makes it / 20000)
        if g::<u32>(TIME_PREV_MS).read() / 20000 != now() / 20000 {
            let mut c = [0u32; 3];
            let p = find_player_coors(lp(&mut c).cast(), -1);
            let saved = g::<f32>(CHASE_SAVED);
            g::<u8>(CHASE_MOVED).write(0);
            let dx = x(saved.read()).fsub_m(p.read());
            let dy = x(saved.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read());
            let dz = x(saved.wrapping_index(2).read()).fsub_m(p.wrapping_index(2).read());
            let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
            if d.fcom(x(10.0)) != Cmp::Less {
                g::<u8>(CHASE_MOVED).write(1);
            }
            let mut c2 = [0u32; 3];
            let p = find_player_coors(lp(&mut c2).cast(), -1).cast::<u32>();
            let s = g::<u32>(CHASE_SAVED);
            s.write(p.read());
            s.wrapping_index(1).write(p.wrapping_index(1).read());
            s.wrapping_index(2).write(p.wrapping_index(2).read());
            let mut c3 = [0u32; 3];
            let p = find_player_coors(lp(&mut c3).cast(), -1).cast::<u32>();
            let mut node = [0u16; 2];
            paths_find_node_closest_to_coors(
                g::<u8>(THE_PATHS),
                lp(&mut node).cast(),
                p.read(),
                p.wrapping_index(1).read(),
                p.wrapping_index(2).read(),
                0,
                0x4270_0000,
                1,
                0,
                0,
                0,
                0,
            );
            g::<u8>(CHASE_NEAR_NODE).write(u8::from(node[0] != 0xFFFF));
        }
        let pd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x480);
        let w = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
        let target = match rd::<i32>(w, 0x2C) {
            1 => x(31.0),
            2 => x(62.0),
            3 => x(125.0),
            4 => x(250.0),
            5 => x(500.0),
            6 => x(1000.0),
            _ => X87::zero(),
        };
        let delta_x = target.fsub_m(rd::<f32>(this, 0x148)).fmul_m(g::<f32>(TIME_STEP).read()).fmul_m(f32c(0x38D1_B717));
        let delta = f32_of(delta_x);
        if delta_x.fcom(X87::zero()) == Cmp::Less
            || (g::<u8>(CHASE_MOVED).read() != 0
                && g::<u8>(CHASE_NEAR_NODE).read() != 0
                && cull_zones_no_police() == 0
                && cull_zones_police_abandon_cars() == 0
                && g::<i32>(CURR_AREA).read() == 0)
        {
            wr::<f32>(this, 0x148, f32_of(x(delta).fadd_m(rd::<f32>(this, 0x148))));
        }
    }

    // block 12
    player_cross_hair_process(this.byte_add(0x158), player_index, pad);
    for off in [0xB8, 0xBC] {
        let v = rd::<i32>(this, off);
        wr::<i32>(this, off, if v >= 999_999_999 { 999_999_999 } else { v });
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_CONTROL,
    HOOK_CIVILIAN_PROCESS_CONTROL,
    HOOK_COP_PROCESS_CONTROL,
    HOOK_EMERGENCY_PROCESS_CONTROL,
    HOOK_COP_ADD_CRIMINAL_TO_KILL,
    HOOK_CALCULATE_NEW_VELOCITY,
    HOOK_DO_FOOT_LANDED,
    HOOK_FOOT_SPLASH_PARTICLES,
    HOOK_FOOT_DUST_PARTICLES,
    HOOK_PLAY_FOOT_STEPS,
    HOOK_PLAYER_PROCESS_CONTROL,
    HOOK_RENDER,
    HOOK_PRE_RENDER_AFTER_TEST,
    HOOK_PLAYER_INFO_PROCESS,
];
