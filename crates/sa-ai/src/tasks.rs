//! Hand-ported `CTask*` bodies (PORT-AI-1, helper A): task constructors, `Clone`, `MakeAbortable`,
//! sub-task creation/control, `CTaskManager` / `CTaskCategories` leaves and task static helpers.
//! Ported from the 1.0 US bytes (the EXE wins over gta-reversed). Every call to another catalogued function
//! goes through `orig!` at the original address; task objects keep their original vtables and come from the
//! original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    // ---- virtual slots (CTask / CTaskComplex vtable layout) ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable +0x20)
    fn v_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;

    // ---- base constructors ----
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;

    // ---- other catalogued functions ----
    /// `CTaskComplexWander::CreateFirstSubTask(CPed*)` (0x6740E0)
    fn wander_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6740E0 => unported;
    /// `CTaskComplexPartner::CreateFirstSubTask(CPed*)` (0x681F20)
    fn partner_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x681F20 => unported;
    /// `CTaskComplexEvasiveStep::CreateSubTask(int32 taskType)` (0x6533E0)
    fn evasive_step_create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x6533E0 => unported;
    /// `CTaskComplexCarDrive::CreateSubTask(int32 taskType, CPed*)` (0x642FA0)
    fn car_drive_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x642FA0 => unported;
    /// `CTaskComplexSequence::CreateNextSubTask-Internal(CPed*, int32* progress, int32* repeat)` (0x632C70)
    fn sequence_create_next_sub_task_internal(t: GPtr<u8>, ped: GPtr<u8>, progress: GPtr<u8>, repeat: GPtr<u8>) -> GPtr<u8> = thiscall 0x632C70 => unported;
    /// `CTaskComplexCarDrive::Constructor_1` (0x63C940, `ret 0x10`)
    fn car_drive_ctor_1(t: GPtr<u8>, a0: u32, a1: u32, a2: i32, a3: u32) -> GPtr<u8> = thiscall 0x63C940 => unported;
    /// `CTaskComplexWanderStandard::CTaskComplexWanderStandard` (0x48E4F0, `ret 0xC`)
    fn wander_standard_ctor(t: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> = thiscall 0x48E4F0 => unported;
    /// `CTaskSimpleRunTimedAnim::Constructor-2` (0x61ABE0, 8 stack arguments)
    fn run_timed_anim_ctor2(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x61ABE0 => unported;
    /// `CTask` virtual destructor (vtable +0, called with the deleting flag)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTaskComplex::~CTaskComplex` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CTaskSimpleCarDrive::Constructor(veh, a1, a2)` (0x63C340, `ret 0xC`)
    fn car_drive_simple_ctor(t: GPtr<u8>, veh: u32, a1: u32, a2: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTrafficLights::LightForPeds()` (0x49D400)
    fn traffic_lights_light_for_peds() -> u8 = cdecl 0x49D400 => unported;
    /// `CPedAttractorManager::GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::BroadcastArrival-(CPed*, CPedAttractor*)` (0x5EFE20, `ret 8`)
    fn attractor_manager_broadcast_arrival(m: GPtr<u8>, ped: GPtr<u8>, attr: GPtr<u8>) -> u8 = thiscall 0x5EFE20 => unported;
    /// `CPedAttractor::GetHeadOfQueue()` (0x5EB590)
    fn attractor_get_head_of_queue(a: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EB590 => unported;
    /// 0x66F450 (the `CTaskComplexWanderCriminal` base constructor, `ret 0x10`)
    fn wander_ctor_66f450(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> GPtr<u8> = thiscall 0x66F450 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn vehicle_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// 0x669D20 (the `CTaskComplexWander` destructor reached by `CTaskComplexWanderCop`'s destructor)
    fn wander_dtor_669d20(t: GPtr<u8>) = thiscall 0x669D20 => unported;
    /// `IKChainManager_c::IsArmPointing(int32 arm, CPed*)` (0x6182B0, `ret 8`)
    fn ik_is_arm_pointing(m: GPtr<u8>, arm: i32, ped: GPtr<u8>) -> u8 = thiscall 0x6182B0 => unported;
    /// `IKChainManager_c::AbortPointArm(int32 arm, CPed*, int32 blendTime)` (0x6182F0, `ret 0xC`)
    fn ik_abort_point_arm(m: GPtr<u8>, arm: i32, ped: GPtr<u8>, blend: i32) = thiscall 0x6182F0 => unported;
    /// `CAEPedSpeechAudioEntity::ReleasePedConversation()` (0x4E52A0, static)
    fn release_ped_conversation() = cdecl 0x4E52A0 => unported;
    /// `CTaskComplexPartner::~CTaskComplexPartner` (0x683A40)
    fn partner_dtor(t: GPtr<u8>) = thiscall 0x683A40 => unported;
    /// `CTaskSimpleCarDrive::ProcessPed(CPed*)` (0x644470)
    fn car_drive_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x644470 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimpleRunAnim::Constructor2` (0x61A900, `ret 0x18`)
    fn run_anim_ctor2(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) -> GPtr<u8> = thiscall 0x61A900 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(cb, data)` (0x4CEBC0)
    fn anim_set_delete_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CTaskGoToVehicleAndLean::Constructor(a0, a1)` (0x660E60, `ret 8`)
    fn go_to_vehicle_and_lean_ctor(t: GPtr<u8>, a0: u32, a1: u32) -> GPtr<u8> = thiscall 0x660E60 => unported;
    /// `CPed::Say(uint16 id, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) = thiscall 0x5EFFE0 => unported;
    /// `CTaskSimpleEvasiveStep::StartAnim(CPed*)` (0x655EA0)
    fn evasive_step_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x655EA0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex-by-name(const char*)` (0x4D3990)
    fn anim_get_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CTaskGangHasslePed::Constructor` (0x65FED0, `ret 0x10`)
    fn gang_hassle_ped_ctor(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> GPtr<u8> = thiscall 0x65FED0 => unported;
    /// `CAnimManager::BlendAnimation-id(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CTaskSimplePlayHandSignalAnim::CTaskSimplePlayHandSignalAnim` (0x61AE50, `ret 0x10`)
    fn play_hand_signal_anim_ctor_orig(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> GPtr<u8> = thiscall 0x61AE50 => unported;
    /// `CTaskComplexGangLeader::ShouldLoadGangAnims()` (0x65E7F0, static; the callers leave `this` in ECX,
    /// passed the same way here)
    fn should_load_gang_anims(ecx: GPtr<u8>) -> u8 = thiscall 0x65E7F0 => unported;
    /// `CVehicle::ClearGettingOutFlags(uint8 doors)` (0x6D3060, `ret 4`)
    fn vehicle_clear_getting_out_flags(v: GPtr<u8>, doors: u32) = thiscall 0x6D3060 => unported;
    /// `CTaskUtilityLineUpPedWithCar::Destructor` (0x64FC00)
    fn line_up_ped_with_car_dtor(u: GPtr<u8>) = thiscall 0x64FC00 => unported;
    /// CRT `_free` (0x8214BD)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `FUN_005fe9c0(int32 pedType)` (0x5FE9C0, cdecl)
    fn ped_type_5fe9c0(ty: i32) -> u8 = cdecl 0x5FE9C0 => unported;
    /// `CPedIntelligence::AreFriends(CPed*, CPed*)` (0x601D10, static)
    fn ped_intelligence_are_friends(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x601D10 => unported;
    /// `CEvent::HasEditableResponse` (CEvent vtable +0x24)
    fn v_event_has_editable_response(e: GPtr<u8>) -> u8 = thiscall vslot 0x24 => unported;
    /// `CEvent::GetEventType` (CEvent vtable +4)
    fn v_event_get_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEventGroup::Add-(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CTaskComplexBeInCouple::AbortArmIK(CPed*)` (0x6837F0)
    fn be_in_couple_abort_arm_ik_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6837F0 => unported;
    /// `CTaskComplexPlayHandSignalAnim::GetAnimIdForPed(CPed*)` (0x61B460)
    fn play_hand_signal_get_anim_id_orig(t: GPtr<u8>, ped: GPtr<u8>) -> i32 = thiscall 0x61B460 => unported;
    /// `CTaskComplexPlayHandSignalAnim::CreateSubTask(int32)` (0x61B2F0, `ret 4`)
    fn play_hand_signal_create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x61B2F0 => unported;
    /// `CTaskComplexPartnerDeal::Constructor` (0x684190, `ret 0x1C`; the last three are a CVector by value)
    fn partner_deal_ctor(t: GPtr<u8>, a0: GPtr<u8>, a1: u32, a2: u32, a3: u32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x684190 => unported;
    /// `CTaskComplexGangFollower::Constructor` (0x65EAA0, `ret 0x1C`; a CVector by value at args 3..5)
    fn gang_follower_ctor(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, vx: u32, vy: u32, vz: u32, a6: u32) -> GPtr<u8> = thiscall 0x65EAA0 => unported;
    /// CRT `_atexit` (0x821D1E)
    fn crt_atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    fn crt_eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `FUN_005f6450`: `CPedGroupDefaultTaskAllocatorFollowAnyMeans` constructor (vtable only)
    fn alloc_ctor_5f6450(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F6450 => unported;
    /// `FUN_005f6480`: `CPedGroupDefaultTaskAllocatorFollowLimited` constructor
    fn alloc_ctor_5f6480(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F6480 => unported;
    /// `FUN_005f64b0`: `CPedGroupDefaultTaskAllocatorStandStill` constructor
    fn alloc_ctor_5f64b0(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F64B0 => unported;
    /// `FUN_005f64e0`: `CPedGroupDefaultTaskAllocatorChat` constructor
    fn alloc_ctor_5f64e0(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F64E0 => unported;
    /// `FUN_005f6510`: `CPedGroupDefaultTaskAllocatorRandom` constructor
    fn alloc_ctor_5f6510(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F6510 => unported;
    /// `FUN_005f6540`: `CPedGroupDefaultTaskAllocatorSitInLeaderCar` constructor
    fn alloc_ctor_5f6540(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F6540 => unported;
    /// `FUN_00694850`: the formation offsets constructor
    fn formation_offsets_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x694850 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTaskComplexWander::GetWanderType` (vtable +0x2C)
    fn v_get_wander_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x2C => unported;
    /// `CTask::GetSubTask` (vtable +8)
    fn v_get_sub_task(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CTaskSimpleRunNamedAnim::Constructor-Anim` (0x61A990, `ret 0x24`)
    fn run_named_anim_ctor(t: GPtr<u8>, a0: GPtr<u8>, a1: GPtr<u8>, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> GPtr<u8> = thiscall 0x61A990 => unported;
    /// `CAnimManager::BlendAnimation-hier(RpClump*, CAnimBlendHierarchy*, int32 flags, float delta)` (0x4D4410)
    fn anim_blend_animation_hier(clump: u32, hier: u32, flags: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4410 => unported;
    /// `CTaskComplexLeaveCar::CreateSubTask(int32, CPed*)` (0x641530, `ret 8`)
    fn leave_car_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x641530 => unported;
    /// `CTaskManager::FindTaskByType(int32 slot, int32 type)` (0x6817D0, `ret 8`)
    fn task_manager_find_task_by_type(tm: GPtr<u8>, slot: i32, ty: i32) -> GPtr<u8> = thiscall 0x6817D0 => unported;
    /// `RpAnimBlend::RpAnimBlendClumpGetAssociation-AnimId(RpClump*, uint32)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CTaskUtilityLineUpPedWithCar::ProcessPed(CPed*, CVehicle*, CAnimBlendAssociation*)` (0x6513A0)
    fn line_up_ped_with_car_process_ped(u: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x6513A0 => unported;
    /// `CPed::SetPedPositionInCar` (0x5DF910)
    fn ped_set_ped_position_in_car(p: GPtr<u8>) = thiscall 0x5DF910 => unported;
    /// `CTaskSimpleFacial::GetAnimId(int32 type)` (0x690CC0, static)
    fn facial_get_anim_id(ty: u32) -> u32 = cdecl 0x690CC0 => unported;
    /// `CTaskTimer::IsOutOfTime` (0x420E30)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::LookAt(name, CPed*, CEntity*, time, bone, CVector* pos, bool, float speed, blend,
    /// priority, bool)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `CGeneral::GetRandomNumberInRange_int32(min, max)` (0x407180; draws its own rand)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`)
    fn anim_set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020, `ret 4`)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, a: u32) = thiscall 0x59B020 => unported;
    /// `CPlayerPed::GetPadFromPlayer` (0x609560)
    fn player_get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetSprint` (0x5407A0)
    fn pad_get_sprint(pad: GPtr<u8>) -> u8 = thiscall 0x5407A0 => unported;
    /// `CStats::GetFatAndMuscleModifier(int32)` (0x559AF0, float in ST0)
    fn stats_get_fat_and_muscle_modifier(a: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CTaskSimpleRunNamedAnim::OffsetPedPosition(CPed*)` (0x61AB00, `ret 4`)
    fn run_named_anim_offset_ped_position(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61AB00 => unported;
    /// `RpAnimBlend::RpAnimBlendClumpGetAssociation-Hier(RpClump*, bool, CAnimBlendHierarchy*)` (0x4D68E0)
    fn clump_get_association_hier(clump: u32, b: u32, hier: u32) -> GPtr<u8> = cdecl 0x4D68E0 => unported;
    /// `CPed::GetBonePosition-Original(RwV3d* out, uint32 bone, bool)` (0x5E4280, `ret 0xC`)
    fn ped_get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: u32, b: u32) = thiscall 0x5E4280 => unported;
    /// `CEventGroup::RemoveInvalidEvents(bool)` (0x4AB760, `ret 4`)
    fn event_group_remove_invalid_events(g: GPtr<u8>, b: u32) = thiscall 0x4AB760 => unported;
    /// `CEventGroup::Reorganise()` (0x4AB700)
    fn event_group_reorganise(g: GPtr<u8>) = thiscall 0x4AB700 => unported;
    /// `CTaskSimpleRunNamedAnim::StartAnim(CPed*)` (0x61BB10)
    fn run_named_anim_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61BB10 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(CVehicle*, CPed*)` (0x64F110)
    fn car_enter_exit_compute_target_door_to_exit(v: GPtr<u8>, ped: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool)` (0x6478B0, `ret 0xC`)
    fn car_set_ped_out_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10)
    fn car_set_ped_out_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// `CTaskSimpleCarSetPedOut` destructor (0x647950)
    fn car_set_ped_out_dtor(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blend)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: i32) = thiscall 0x618280 => unported;
    /// `CUpsideDownCarCheck::IsCarUpsideDown-vehicle(const CVehicle*)` (0x463830, `this` unused)
    fn upside_down_car_check_is_car_upside_down(c: GPtr<u8>, v: GPtr<u8>) -> u8 = thiscall 0x463830 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, `ret 4`)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CTaskUtilityLineUpPedWithCar::GetLocalPositionToOpenCarDoor(CVector* out, CVehicle*, float, assoc)`
    /// (0x64FC10, `ret 0x10`)
    fn line_up_get_local_position_to_open_car_door(u: GPtr<u8>, out: GPtr<u8>, veh: GPtr<u8>, t: u32, a: u32) -> GPtr<u8> = thiscall 0x64FC10 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790, cdecl)
    fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CTaskSimplePause::Constructor(int32 time)` (0x48E750)
    fn simple_pause_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    /// CRT `wprintf`-like debug print (0x821982, `FID_conflict:_wprintf`, cdecl)
    fn crt_debug_printf(fmt: u32) -> i32 = cdecl 0x821982 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0, `ret 4`)
    fn group_membership_is_leader(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CEventLeaderExitedCarAsDriver::Constructor` (0x4B8300)
    fn event_leader_exited_car_as_driver_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8300 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0, `ret 8`)
    fn event_group_event_ctor(g: GPtr<u8>, ped: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CPedGroupIntelligence::AddEvent-(CEventGroupEvent*)` (0x5F7470, `ret 4`)
    fn group_intelligence_add_event(gi: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5F7470 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    fn event_group_event_dtor(g: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPed::SetRadioStation` (0x5DFD90)
    fn ped_set_radio_station(p: GPtr<u8>) = thiscall 0x5DFD90 => unported;
    /// `CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver` (0x4F5BA0)
    fn vehicle_audio_player_about_to_exit_as_driver(a: GPtr<u8>) = thiscall 0x4F5BA0 => unported;
    /// `CTaskComplexLeaveBoat::Constructor(CVehicle*, int32 delay)` (0x63BB60, `ret 8`)
    fn leave_boat_ctor(t: GPtr<u8>, v: u32, delay: u32) -> GPtr<u8> = thiscall 0x63BB60 => unported;
    /// `CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed(veh, time)` (0x5FF940, `ret 8`)
    fn car_drive_timed_ctor_orig(t: GPtr<u8>, v: u32, time: u32) -> GPtr<u8> = thiscall 0x5FF940 => unported;
    /// `CTaskSimpleRunAnim::Constructor(groupId, animId, float delta, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: u32, delta: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskSimpleStandStill::Constructor(time, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: i32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexSeekEntityStandard::Constructor` (0x46AC10, body in `.HOODLUM`, `ret 0x20`)
    fn seek_entity_standard_ctor(t: GPtr<u8>, ent: u32, time: u32, scan: u32, radius: u32, a4: u32, a5: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
}

/// `CTaskComplexWanderGang::ScanForStuff(CPed*)` (0x66F640): `ret 4`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn wander_gang_scan_for_stuff(_this: GPtr<u8>, _ped: GPtr<u8>) {}

/// `CTaskComplexWanderProstitute::CreateFirstSubTask(CPed*)` (0x674920): tail jump to
/// `CTaskComplexWander::CreateFirstSubTask`.
#[hook(addr = 0x674920, conv = thiscall, name = "CTaskComplexWanderProstitute::CreateFirstSubTask", expect = "E9 BB F7 FF FF 90")]
pub fn wander_prostitute_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wander_create_first_sub_task(this, ped)
}

/// `CTaskComplexPartnerDeal::CreateFirstSubTask(CPed*)` (0x6823B0): tail jump to
/// `CTaskComplexPartner::CreateFirstSubTask`.
#[hook(addr = 0x6823B0, conv = thiscall, name = "CTaskComplexPartnerDeal::CreateFirstSubTask", expect = "E9 6B FB FF FF 90")]
pub fn partner_deal_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    partner_create_first_sub_task(this, ped)
}

/// `CTaskComplexObserveTrafficLights::CreateFirstSubTask(CPed*)` (0x6318D0): tail jump to the virtual
/// `CreateNextSubTask(ped)` (vtable +0x20).
#[hook(addr = 0x6318D0, conv = thiscall, name = "CTaskComplexObserveTrafficLights::CreateFirstSubTask", expect = "8B 01 FF 60 20")]
pub fn observe_traffic_lights_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    v_create_next_sub_task(this, ped)
}

/// `CTaskComplexEvasiveStep::CreateFirstSubTask(CPed*)` (0x653540): overwrites its argument with 0x386
/// and tail-jumps to `CreateSubTask(0x386)`.
#[hook(addr = 0x653540, conv = thiscall, name = "CTaskComplexEvasiveStep::CreateFirstSubTask", expect = "C7 44 24 04 86 03 00 00")]
pub fn evasive_step_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    evasive_step_create_sub_task(this, 0x386)
}

/// `CTaskComplexBeInCouple::CreateFirstSubTask(CPed*)` (0x6837E0): tail jump to the virtual
/// `CreateNextSubTask(ped)` (vtable +0x20).
#[hook(addr = 0x6837E0, conv = thiscall, name = "CTaskComplexBeInCouple::CreateFirstSubTask", expect = "8B 01 FF 60 20")]
pub fn be_in_couple_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    v_create_next_sub_task(this, ped)
}

/// `CTaskComplex::MakeAbortable(CPed*, int32, const CEvent*)` (0x4211B0): tail jump to the sub-task's
/// virtual `MakeAbortable` (vtable +0x18) with the same arguments.
#[hook(addr = 0x4211B0, conv = thiscall, name = "CTaskComplex::MakeAbortable", expect = "8B 49 08 8B 01")]
pub fn task_complex_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, priority, event)
}

/// `CTaskManager::GetTaskSecondary(int32)` (0x681810): `m_aSecondaryTasks[i]` (+0x14 + 4i).
#[hook(addr = 0x681810, conv = thiscall, name = "CTaskManager::GetTaskSecondary", expect = "8B 44 24 04 8B 44 81 14")]
pub fn task_manager_get_task_secondary(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, (i as isize).wrapping_mul(4).wrapping_add(0x14))
}

/// `CTaskSimpleAnim::FinishRunAnimCB(CAnimBlendAssociation*, void* task)` (0x61A8A0, cdecl): sets bit 0 of
/// the task's flags byte (+0xC), then clears its anim pointer (+8).
#[hook(addr = 0x61A8A0, conv = cdecl, name = "CTaskSimpleAnim::FinishRunAnimCB", expect = "8B 44 24 08 80 48 0C 01")]
pub fn task_simple_anim_finish_run_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    wr::<u8>(task, 0xC, rd::<u8>(task, 0xC) | 1);
    wr::<u32>(task, 8, 0);
}

/// `CTaskComplexCarDriveWander::CreateSubTaskCannotGetInCar(CPed*)` (0x643240):
/// `CTaskComplexCarDrive::CreateSubTask(0x390, ped)`.
#[hook(addr = 0x643240, conv = thiscall, name = "CTaskComplexCarDriveWander::CreateSubTaskCannotGetInCar", expect = "8B 44 24 04 50")]
pub fn car_drive_wander_create_sub_task_cannot_get_in_car(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    car_drive_create_sub_task(this, 0x390, ped)
}

/// `CTaskCategories::IsFollowPedTask` (0x698610, cdecl): only clears the two output bytes (arguments 2 and
/// 3); the task argument is not read.
#[hook(addr = 0x698610, conv = cdecl, name = "CTaskCategories::IsFollowPedTask", expect = "8B 44 24 08 8B 4C 24 0C")]
pub fn task_categories_is_follow_ped_task(_task: GPtr<u8>, out_a: GPtr<u8>, out_b: GPtr<u8>) {
    wr::<u8>(out_a, 0, 0);
    wr::<u8>(out_b, 0, 0);
}

/// `CTaskComplexSequence::CreateNextSubTask-V(CPed*)` (0x638A40): the internal helper with
/// `&m_nCurrentTaskIndex` (+0xC) and `&m_nRepeatedCount`-like counter at +0x34.
#[hook(addr = 0x638A40, conv = thiscall, name = "CTaskComplexSequence::CreateNextSubTask-V", expect = "8D 41 34 50 8B 44 24 08")]
pub fn sequence_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    sequence_create_next_sub_task_internal(this, ped, this.byte_add(0xC), this.byte_add(0x34))
}

/// `CTaskComplexObserveTrafficLightsAndAchieveHeading::CTaskComplexObserveTrafficLightsAndAchieveHeading`
/// (0x631910): `CTaskComplex` base, +0xC = a0, vtable 0x86DFC4, +0x10 = a1.
#[hook(addr = 0x631910, conv = thiscall, name = "CTaskComplexObserveTrafficLightsAndAchieveHeading::CTaskComplexObserveTrafficLightsAndAchieveHeading", expect = "56 8B F1 E8 98 8A FE FF")]
pub fn observe_traffic_lights_and_achieve_heading_ctor(this: GPtr<u8>, a0: u32, a1: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, a0);
    wr::<u32>(this, 0, 0x86DFC4);
    wr::<u32>(this, 0x10, a1);
    this
}

/// `CTaskSimpleEvasiveStep::FinishAnimEvasiveStepCB(CAnimBlendAssociation*, void* task)` (0x653290, cdecl):
/// sets bit 2 of byte +0x2E of the task's anim (+0x10); when that anim's float +0x1C is not below the
/// constant at 0x858B50 (unordered counts as below), it becomes -4.0; then +0xC = 1 and +0x10 = null.
#[hook(addr = 0x653290, conv = cdecl, name = "CTaskSimpleEvasiveStep::FinishAnimEvasiveStepCB", expect = "8B 4C 24 08 8B 41 10")]
pub fn evasive_step_finish_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(task, 0x10);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
    let a = rd::<GPtr<u8>>(task, 0x10);
    if !x(rd::<f32>(a, 0x1C)).fcom(x(g::<f32>(0x858B50).read())).test_any(0x01) {
        wr::<u32>(a, 0x1C, 0xC080_0000);
    }
    wr::<u8>(task, 0xC, 1);
    wr::<u32>(task, 0x10, 0);
}

/// `CTaskComplexCarDriveWander::CTaskComplexCarDriveWander(veh, a1, a2)` (0x63CB10):
/// `CTaskComplexCarDrive::Constructor_1(veh, a2, -1, a1)`, then vtable 0x86E96C.
#[hook(addr = 0x63CB10, conv = thiscall, name = "CTaskComplexCarDriveWander::CTaskComplexCarDriveWander", expect = "8B 44 24 08 8B 54 24 04")]
pub fn car_drive_wander_ctor(this: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> {
    car_drive_ctor_1(this, a0, a2, -1, a1);
    wr::<u32>(this, 0, 0x86E96C);
    this
}

/// `CTaskComplexWanderProstitute::CTaskComplexWanderProstitute(a0, a1, a2)` (0x672690):
/// `CTaskComplexWanderStandard(a0, a1, a2)`, vtable 0x870148, +0x38 = 0.
#[hook(addr = 0x672690, conv = thiscall, name = "CTaskComplexWanderProstitute::CTaskComplexWanderProstitute", expect = "8B 44 24 0C 8B 54 24 04")]
pub fn wander_prostitute_ctor(this: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> {
    wander_standard_ctor(this, a0, a1, a2);
    wr::<u32>(this, 0, 0x870148);
    wr::<u32>(this, 0x38, 0);
    this
}

/// `CTaskCategories::IsKillPedTask(CTask*, bool*, bool*)` (0x6985E0, cdecl): clears both outputs, then sets
/// the first when `task->GetTaskType()` is in 0x3FB..=0x3FD.
#[hook(addr = 0x6985E0, conv = cdecl, name = "CTaskCategories::IsKillPedTask", expect = "8B 44 24 0C 8B 4C 24 04")]
pub fn task_categories_is_kill_ped_task(task: GPtr<u8>, out_a: GPtr<u8>, out_b: GPtr<u8>) {
    wr::<u8>(out_a, 0, 0);
    wr::<u8>(out_b, 0, 0);
    let t = v_get_task_type(task);
    if (0x3FB..=0x3FD).contains(&t) {
        wr::<u8>(out_a, 0, 1);
    }
}

/// `CTaskSimpleChat::CTaskSimpleChat(a0)` (0x5F65A0): `CTaskSimpleRunTimedAnim::Constructor-2(0, 0xC, 4.0,
/// -4.0, a0, 0x1A0, 0x86C7B4, 0)`, then vtable 0x86C78C.
#[hook(addr = 0x5F65A0, conv = thiscall, name = "CTaskSimpleChat::CTaskSimpleChat", expect = "8B 44 24 04 56")]
pub fn simple_chat_ctor(this: GPtr<u8>, a0: u32) -> GPtr<u8> {
    run_timed_anim_ctor2(this, 0, 0xC, 0x4080_0000, 0xC080_0000, a0, 0x1A0, 0x86C7B4, 0);
    wr::<u32>(this, 0, 0x86C78C);
    this
}

/// `CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading(heading, a1, a2)` (0x667E20): `CTaskSimple` base,
/// clears bit 0 of byte +0x14, +8 = heading, vtable 0x86FD74, +0xC = a1, +0x10 = a2.
#[hook(addr = 0x667E20, conv = thiscall, name = "CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading", expect = "56 8B F1 E8 68 25 FB FF")]
pub fn achieve_heading_ctor(this: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0x14, rd::<u8>(this, 0x14) & 0xFE);
    wr::<u32>(this, 8, a0);
    wr::<u32>(this, 0, 0x86FD74);
    wr::<u32>(this, 0xC, a1);
    wr::<u32>(this, 0x10, a2);
    this
}

/// `CTaskSimpleFacial` constructor (0x690C70, `(a0, a1)`): `CTaskSimple` base, vtable 0x870AB8, +8 = +0xC =
/// 0, bytes +0x10/+0x11 = 0, +0x18 = a1, +0x14 = a0.
#[hook(addr = 0x690C70, conv = thiscall, name = "CTaskSimpleFacial", expect = "56 8B F1 E8 18 97 F8 FF")]
pub fn simple_facial_ctor(this: GPtr<u8>, a0: u32, a1: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x870AB8);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u8>(this, 0x10, 0);
    wr::<u8>(this, 0x11, 0);
    wr::<u32>(this, 0x18, a1);
    wr::<u32>(this, 0x14, a0);
    this
}

/// `CTaskSimpleSay::Constructor(a0, a1)` (0x48E360, body in `.HOODLUM` at 0x15655E0): `CTaskSimple` base,
/// +8 = a0, vtable 0x85A1DC, +0xC = a1, +0x10 = +0x14 = 0, bytes +0x18/+0x19 = 0.
#[hook(addr = 0x48E360, conv = thiscall, name = "CTaskSimpleSay::Constructor", expect = "E9 7B 72 0D 01")]
pub fn simple_say_ctor(this: GPtr<u8>, a0: u32, a1: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 8, a0);
    wr::<u32>(this, 0, 0x85A1DC);
    wr::<u32>(this, 0xC, a1);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 0x18, 0);
    wr::<u8>(this, 0x19, 0);
    this
}

/// `CPedGroupDefaultTaskAllocators::Get(int32)` (0x5F7200, cdecl): the allocator object for 0..=5 through
/// the jump table at 0x5F7238 (note: 4 -> 0xC0991C, 5 -> 0xC09918), else null.
#[hook(addr = 0x5F7200, conv = cdecl, name = "CPedGroupDefaultTaskAllocators::Get", expect = "8B 4C 24 04 33 C0", preserve = "edx")]
pub fn default_task_allocators_get(i: u32) -> u32 {
    match i {
        0 => 0xC09908,
        1 => 0xC0990C,
        2 => 0xC09910,
        3 => 0xC09914,
        4 => 0xC0991C,
        5 => 0xC09918,
        _ => 0,
    }
}

/// `CTaskComplex::SetSubTask(CTask*)` (0x61A430): when the sub-task changes, deletes the old one through its
/// virtual destructor (flag 1), stores the new one at +8 and sets its parent (+4) to `this` when non-null.
#[hook(addr = 0x61A430, conv = thiscall, name = "CTaskComplex::SetSubTask", expect = "56 8B F1 8B 4E 08")]
pub fn task_complex_set_sub_task(this: GPtr<u8>, sub: GPtr<u8>) {
    let old = rd::<GPtr<u8>>(this, 8);
    if old == sub {
        return;
    }
    if !old.is_null() {
        v_delete(old, 1);
    }
    wr::<GPtr<u8>>(this, 8, sub);
    if !sub.is_null() {
        wr::<GPtr<u8>>(sub, 4, this);
    }
}

/// `CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed(veh, time)` (0x5FF940):
/// `CTaskSimpleCarDrive::Constructor(veh, 0, 0)`, vtable 0x859E50, +0x60 = time, +0x64 = +0x68 = 0,
/// bytes +0x6C/+0x6D = 0.
#[hook(addr = 0x5FF940, conv = thiscall, name = "CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed", expect = "8B 44 24 04 53")]
pub fn car_drive_timed_ctor(this: GPtr<u8>, veh: u32, time: u32) -> GPtr<u8> {
    car_drive_simple_ctor(this, veh, 0, 0);
    wr::<u32>(this, 0, 0x859E50);
    wr::<u32>(this, 0x60, time);
    wr::<u32>(this, 0x64, 0);
    wr::<u32>(this, 0x68, 0);
    wr::<u8>(this, 0x6C, 0);
    wr::<u8>(this, 0x6D, 0);
    this
}

/// `CTaskComplexObserveTrafficLights::ControlSubTask(CPed*)` (0x6318E0): while the pedestrian light is not
/// on, asks the sub-task to abort (`MakeAbortable(ped, 0, null)`) and returns null if it agrees; otherwise
/// the sub-task.
#[hook(addr = 0x6318E0, conv = thiscall, name = "CTaskComplexObserveTrafficLights::ControlSubTask", expect = "56 8B F1 E8 18 BB E6 FF")]
pub fn observe_traffic_lights_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if traffic_lights_light_for_peds() == 0 && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL) != 0 {
        return GPtr::NULL;
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskComplexGoToAttractor::CreateNextSubTask(CPed*)` (0x66B6C0): broadcasts the arrival at the attractor
/// (+0xC); when the ped's flag 0x200000 (+0x474) is set and the ped is not the head of the attractor's queue,
/// clears that flag. Returns null.
#[hook(addr = 0x66B6C0, conv = thiscall, name = "CTaskComplexGoToAttractor::CreateNextSubTask", expect = "56 8B 74 24 08")]
pub fn go_to_attractor_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let attr = rd::<GPtr<u8>>(this, 0xC);
    let m = get_ped_attractor_manager();
    attractor_manager_broadcast_arrival(m, ped, attr);
    if rd::<u32>(ped, 0x474) & 0x200000 != 0 && attractor_get_head_of_queue(rd::<GPtr<u8>>(this, 0xC)) != ped {
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFDF_FFFF);
    }
    GPtr::NULL
}

/// `CTaskComplexWanderCriminal::CTaskComplexWanderCriminal(a0, a1, a2)` (0x48E610, body in `.HOODLUM` at
/// 0x156F300): base constructor 0x66F450(a0, a1, a2, [0x86FCB0]), vtable 0x85A23C, +0x28 = +0x2C = 0,
/// bytes +0x30/+0x31 = 0, +0x34 = 0.
#[hook(addr = 0x48E610, conv = thiscall, name = "CTaskComplexWanderCriminal::CTaskComplexWanderCriminal", expect = "E9 EB 0C 0E 01")]
pub fn wander_criminal_ctor(this: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> {
    let k = g::<u32>(0x86FCB0).read();
    wander_ctor_66f450(this, a0, a1, a2, k);
    wr::<u32>(this, 0, 0x85A23C);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x2C, 0);
    wr::<u8>(this, 0x30, 0);
    wr::<u8>(this, 0x31, 0);
    wr::<u32>(this, 0x34, 0);
    this
}

/// `CTaskSimpleCarCloseDoorFromOutside::ComputeAnimID(int32* group, int32* anim)` (0x646600): door
/// (+0x14) 8/9/10/11 -> anim 0x17D/0x17F/0x17C/0x17E (jump table at 0x646664; other doors leave `*anim`
/// as it is), then `*group = ms_vehicleAnimGroups[veh->m_pHandling->animGroup].GetGroup(*anim)` for the
/// vehicle at +0x10.
#[hook(addr = 0x646600, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromOutside::ComputeAnimID", expect = "8B 41 14 8B 54 24 08")]
pub fn car_close_door_from_outside_compute_anim_id(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) {
    match rd::<i32>(this, 0x14).wrapping_sub(8) {
        0 => wr::<u32>(anim, 0, 0x17D),
        1 => wr::<u32>(anim, 0, 0x17F),
        2 => wr::<u32>(anim, 0, 0x17C),
        3 => wr::<u32>(anim, 0, 0x17E),
        _ => {}
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let a = rd::<u32>(anim, 0);
    let grp = u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE));
    let r = vehicle_anim_group_get_group(GPtr::from_addr((grp * 0x94).wrapping_add(0xC1CDC0) as usize), a);
    wr::<u32>(group, 0, r);
}

/// 0x460D80 (catalogued as `CTaskComplexWanderCop::CTaskComplexWanderCop`, actually its destructor):
/// vtable 0x859C80, deletes the task at +0x28 (virtual destructor, flag 1), then the base destructor 0x669D20.
#[hook(addr = 0x460D80, conv = thiscall, name = "CTaskComplexWanderCop::CTaskComplexWanderCop", expect = "6A FF 68 88 7B 83 00")]
pub fn wander_cop_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x859C80);
    let t = rd::<GPtr<u8>>(this, 0x28);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wander_dtor_669d20(this);
}

/// `CTaskSimplePlayHandSignalAnim::CTaskSimplePlayHandSignalAnim(a0, a1, bool a2, bool a3)` (0x61AE50):
/// `CTaskSimple` base; byte +0xC = (old & 0xF8) | ((a3 & 1) << 2); +0x10 = a0; +8 = +0x1C = +0x20 = 0;
/// vtable 0x86D5B8; +0x14 = a1; byte +0x18 = (a2 != 0).
#[hook(addr = 0x61AE50, conv = thiscall, name = "CTaskSimplePlayHandSignalAnim::CTaskSimplePlayHandSignalAnim", expect = "56 8B F1 E8 38 F5 FF FF")]
pub fn play_hand_signal_anim_ctor(this: GPtr<u8>, a0: u32, a1: u32, a2: u8, a3: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u8>(this, 0xC, ((a3 & 1) << 2) | (rd::<u8>(this, 0xC) & 0xF8));
    wr::<u32>(this, 0x10, a0);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0, 0x86D5B8);
    wr::<u32>(this, 0x14, a1);
    wr::<u8>(this, 0x18, u8::from(a2 != 0));
    this
}

/// Destructor shape `vtbl = V; if (ref) ref->CleanUpOldReference(&ref); base dtor`.
fn dtor_with_ref(this: GPtr<u8>, vtbl: u32, off: isize, simple: bool) {
    wr::<u32>(this, 0, vtbl);
    let e = rd::<GPtr<u8>>(this, off);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(off));
    }
    if simple {
        task_simple_dtor(this);
    } else {
        task_complex_dtor(this);
    }
}

/// `CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger` (0x647080): vtable 0x86EE04,
/// releases the vehicle reference at +0x10, `CTaskSimple` destructor.
#[hook(addr = 0x647080, conv = thiscall, name = "CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger", expect = "6A FF 68 C8 26 84 00")]
pub fn car_set_ped_in_as_passenger_dtor(this: GPtr<u8>) {
    dtor_with_ref(this, 0x86EE04, 0x10, true);
}

/// `CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep` (0x653350): vtable 0x86F218, releases the entity
/// reference at +0x18, `CTaskComplex` destructor.
#[hook(addr = 0x653350, conv = thiscall, name = "CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep", expect = "6A FF 68 D8 33 84 00")]
pub fn evasive_step_complex_dtor(this: GPtr<u8>) {
    dtor_with_ref(this, 0x86F218, 0x18, false);
}

/// `CTaskComplexBeInCouple::~CTaskComplexBeInCouple` (0x683780): vtable 0x870724, releases the partner
/// reference at +0x10, `CTaskComplex` destructor.
#[hook(addr = 0x683780, conv = thiscall, name = "CTaskComplexBeInCouple::~CTaskComplexBeInCouple", expect = "6A FF 68 38 68 84 00")]
pub fn be_in_couple_dtor(this: GPtr<u8>) {
    dtor_with_ref(this, 0x870724, 0x10, false);
}

/// `CTaskSimpleLand::~CTaskSimpleLand` (0x678EE0): vtable 0x8704FC; when the anim (+8) is set, resets its
/// finish callback to 0x4D1390 (`CDefaultAnimCallback::DefaultAnimCB`) with null data; `CTaskSimple`
/// destructor.
#[hook(addr = 0x678EE0, conv = thiscall, name = "CTaskSimpleLand::~CTaskSimpleLand", expect = "6A FF 68 08 60 84 00")]
pub fn simple_land_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x8704FC);
    let a = rd::<GPtr<u8>>(this, 8);
    if !a.is_null() {
        anim_set_finish_callback(a, 0x4D1390, 0);
    }
    task_simple_dtor(this);
}

/// `CTaskComplexPlayHandSignalAnim::GetAnimIdForPed(CPed*)` (0x61B460, `ret 4`, `this` unused): by the
/// ped's weapon-slot word at +0x598: 7 -> 0x140, 8 -> 0x141, 9 -> 0x142, 0xB -> 0x144, 0xE -> 0x143, else -1.
#[hook(addr = 0x61B460, conv = thiscall, name = "CTaskComplexPlayHandSignalAnim::GetAnimIdForPed", expect = "8B 44 24 04 8B 80 98 05 00 00", preserve = "edx")]
pub fn play_hand_signal_get_anim_id_for_ped(_this: GPtr<u8>, ped: GPtr<u8>) -> i32 {
    match rd::<i32>(ped, 0x598) {
        7 => 0x140,
        8 => 0x141,
        9 => 0x142,
        0xB => 0x144,
        0xE => 0x143,
        _ => -1,
    }
}

/// `CTaskComplexBeInCouple::AbortArmIK(CPed*)` (0x6837F0): for arm 0 then arm 1 of `g_ikChainMan`
/// (0xC15448), aborts the point-arm IK (blend 250) when it is pointing.
#[hook(addr = 0x6837F0, conv = thiscall, name = "CTaskComplexBeInCouple::AbortArmIK", expect = "56 8B 74 24 08")]
pub fn be_in_couple_abort_arm_ik(_this: GPtr<u8>, ped: GPtr<u8>) {
    let m = GPtr::<u8>::from_addr(0xC15448);
    for arm in 0..2 {
        if ik_is_arm_pointing(m, arm, ped) != 0 {
            ik_abort_point_arm(m, arm, ped, 0xFA);
        }
    }
}

/// `CTaskComplexPartnerChat::~CTaskComplexPartnerChat` (0x684320): vtable 0x8707C4; when bytes +0x74 and
/// +0x76 are set, `ReleasePedConversation()`; then `CTaskComplexPartner`'s destructor.
#[hook(addr = 0x684320, conv = thiscall, name = "CTaskComplexPartnerChat::~CTaskComplexPartnerChat", expect = "6A FF 68 88 69 84 00")]
pub fn partner_chat_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x8707C4);
    if rd::<u8>(this, 0x74) != 0 && rd::<u8>(this, 0x76) != 0 {
        release_ped_conversation();
    }
    partner_dtor(this);
}

/// `CTaskComplexEvasiveStep::CreateNextSubTask(CPed*)` (0x6534F0): after sub-task type 0x1F5 ->
/// `CreateSubTask(0x516)`, after 0x386 -> `CreateSubTask(0x1F5)`, else null.
#[hook(addr = 0x6534F0, conv = thiscall, name = "CTaskComplexEvasiveStep::CreateNextSubTask", expect = "56 8B F1 8B 4E 08")]
pub fn evasive_step_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x1F5 => evasive_step_create_sub_task(this, 0x516),
        0x386 => evasive_step_create_sub_task(this, 0x1F5),
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexPartnerChat::MakeAbortable(CPed*, int32, const CEvent*)` (0x682C60): forwards to the
/// sub-task; when it agrees and bytes +0x74 and +0x76 are set, releases the conversation and clears +0x76.
#[hook(addr = 0x682C60, conv = thiscall, name = "CTaskComplexPartnerChat::MakeAbortable", expect = "8B 54 24 0C 56")]
pub fn partner_chat_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, priority, event) == 0 {
        return 0;
    }
    if rd::<u8>(this, 0x74) != 0 && rd::<u8>(this, 0x76) != 0 {
        release_ped_conversation();
        wr::<u8>(this, 0x76, 0);
    }
    1
}

/// `CTaskSimpleCarDriveTimed::ProcessPed(CPed*)` (0x46F610): starts the timer on the first call (+0x64 =
/// now, +0x68 = duration +0x60, +0x6C = 1), restarts it when +0x6D is set; done (true) once
/// `now >= start + duration` (unsigned), else `CTaskSimpleCarDrive::ProcessPed`.
#[hook(addr = 0x46F610, conv = thiscall, name = "CTaskSimpleCarDriveTimed::ProcessPed", expect = "8A 41 6C 84 C0")]
pub fn car_drive_timed_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let now = || g::<u32>(0xB7CB84).read();
    if rd::<u8>(this, 0x6C) == 0 {
        let d = rd::<u32>(this, 0x60);
        wr::<u32>(this, 0x64, now());
        wr::<u32>(this, 0x68, d);
        wr::<u8>(this, 0x6C, 1);
        if rd::<u8>(this, 0x6C) == 0 {
            return car_drive_process_ped(this, ped);
        }
    }
    if rd::<u8>(this, 0x6D) != 0 {
        wr::<u32>(this, 0x64, now());
        wr::<u8>(this, 0x6D, 0);
    }
    let end = rd::<u32>(this, 0x68).wrapping_add(rd::<u32>(this, 0x64));
    if now() >= end {
        return 1;
    }
    car_drive_process_ped(this, ped)
}

/// `CTaskSimpleEvasiveStep::CTaskSimpleEvasiveStep(CEntity*)` (0x653160): `CTaskSimple` base, vtable
/// 0x86F1F4, +8 = entity, byte +0xC = 0, +0x10 = 0, registers the reference at +8 when non-null.
#[hook(addr = 0x653160, conv = thiscall, name = "CTaskSimpleEvasiveStep::CTaskSimpleEvasiveStep", expect = "6A FF 68 78 33 84 00")]
pub fn evasive_step_simple_ctor(this: GPtr<u8>, ent: GPtr<u8>) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x86F1F4);
    wr::<GPtr<u8>>(this, 8, ent);
    wr::<u8>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    if !ent.is_null() {
        entity_register_reference(ent, this.byte_add(8));
    }
    this
}

/// `CTaskSimpleScratchHead::Clone` (0x48DF60): a 0x20-byte task, `CTaskSimpleRunAnim::Constructor2(0, 0x87,
/// 4.0, 0x1A5, 0x85A128, 0)`, vtable 0x85A100; null when the pool is full.
#[hook(addr = 0x48DF60, conv = thiscall, name = "CTaskSimpleScratchHead::Clone", expect = "6A FF 68 FB 84 83 00")]
pub fn scratch_head_clone(_this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x20);
    if t.is_null() {
        return t;
    }
    run_anim_ctor2(t, 0, 0x87, 0x4080_0000, 0x1A5, 0x85A128, 0);
    wr::<u32>(t, 0, 0x85A100);
    t
}

/// `CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep` (0x6531D0): vtable 0x86F1F4; resets the anim's (+0x10)
/// finish callback to 0x4D1390 / null; releases the entity reference at +8; `CTaskSimple` destructor.
#[hook(addr = 0x6531D0, conv = thiscall, name = "CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep", expect = "6A FF 68 98 33 84 00")]
pub fn evasive_step_simple_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F1F4);
    let a = rd::<GPtr<u8>>(this, 0x10);
    if !a.is_null() {
        anim_set_finish_callback(a, 0x4D1390, 0);
    }
    let e = rd::<GPtr<u8>>(this, 8);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(8));
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleInAir::~CTaskSimpleInAir` (0x678D50): vtable 0x8704D8; resets the anim's (+0x1C) delete
/// callback to 0x4D1390 / null; releases the entity reference at +0x38; `CTaskSimple` destructor.
#[hook(addr = 0x678D50, conv = thiscall, name = "CTaskSimpleInAir::~CTaskSimpleInAir", expect = "6A FF 68 E8 5F 84 00")]
pub fn in_air_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x8704D8);
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if !a.is_null() {
        anim_set_delete_callback(a, 0x4D1390, 0);
    }
    let e = rd::<GPtr<u8>>(this, 0x38);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x38));
    }
    task_simple_dtor(this);
}

/// `CTaskGoToVehicleAndLean::Clone` (0x6621B0): a 0x24-byte task built by `Constructor(+0xC, +0x10)`
/// (its return value is returned); null when the pool is full.
#[hook(addr = 0x6621B0, conv = thiscall, name = "CTaskGoToVehicleAndLean::Clone", expect = "6A FF 68 0B 4A 84 00")]
pub fn go_to_vehicle_and_lean_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x24);
    if t.is_null() {
        return GPtr::NULL;
    }
    go_to_vehicle_and_lean_ctor(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10))
}

/// `CTaskComplexWanderGang::CTaskComplexWanderGang(a0, uint8 a1, a2, bool a3, a4)` (0x66F5C0):
/// `CTaskComplex` base, +0xC = a0, byte +0x10 = a1, +0x14 = a4, words +0x18/+0x1C = 0xFFFF, +0x20 = 0,
/// vtable 0x8700D4, byte +0x24 = (old & 0xF0) | (a3 & 1), then the inlined wander timer (+0x28/+0x2C/+0x30/
/// +0x31 cleared, +0x34 = a2) started (+0x2C = a2, +0x28 = now, +0x30 = 1).
#[hook(addr = 0x66F5C0, conv = thiscall, name = "CTaskComplexWanderGang::CTaskComplexWanderGang", expect = "56 8B F1 E8 E8 AD FA FF")]
pub fn wander_gang_ctor(this: GPtr<u8>, a0: u32, a1: u8, a2: u32, a3: u8, a4: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, a0);
    wr::<u8>(this, 0x10, a1);
    wr::<u32>(this, 0x14, a4);
    wr::<u16>(this, 0x18, 0xFFFF);
    wr::<u16>(this, 0x1C, 0xFFFF);
    let f = rd::<u8>(this, 0x24);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0, 0x8700D4);
    wr::<u8>(this, 0x24, (f & 0xF0) | (a3 & 1));
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x2C, 0);
    wr::<u8>(this, 0x30, 0);
    wr::<u8>(this, 0x31, 0);
    wr::<u32>(this, 0x34, a2);
    let now = g::<u32>(0xB7CB84).read();
    wr::<u32>(this, 0x2C, a2);
    wr::<u32>(this, 0x28, now);
    wr::<u8>(this, 0x30, 1);
    this
}

/// `CTaskSimpleEvasiveStep::ProcessPed(CPed*)` (0x657A60): done when finished (+0xC) or without an entity
/// (+8); while no anim (+0x10) is playing: says 0x4A when the entity is a vehicle (type 2) and starts the anim.
#[hook(addr = 0x657A60, conv = thiscall, name = "CTaskSimpleEvasiveStep::ProcessPed", expect = "56 8B F1 8A 46 0C")]
pub fn evasive_step_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xC) != 0 {
        return 1;
    }
    let e = rd::<GPtr<u8>>(this, 8);
    if e.is_null() {
        return 1;
    }
    if !rd::<GPtr<u8>>(this, 0x10).is_null() {
        return 0;
    }
    if rd::<u8>(e, 0x36) & 7 == 2 {
        ped_say(ped, 0x4A, 0, 0x3F80_0000, 0, 0, 0);
    }
    evasive_step_start_anim(this, ped);
    0
}

/// `CTaskComplexGangFollower::Destructor` (0x65EBB0): vtable 0x86F938; releases the leader reference at
/// +0x10; when bit 0 of +0x3D is set, removes the reference on the anim block named at 0x86F92C and clears
/// the bit; `CTaskComplex` destructor.
#[hook(addr = 0x65EBB0, conv = thiscall, name = "CTaskComplexGangFollower::Destructor", expect = "6A FF 68 08 44 84 00")]
pub fn gang_follower_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F938);
    let e = rd::<GPtr<u8>>(this, 0x10);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x10));
    }
    if rd::<u8>(this, 0x3D) & 1 != 0 {
        let i = anim_get_block_index(GPtr::from_addr(0x86F92C));
        anim_remove_block_ref(i);
        wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) & 0xFE);
    }
    task_complex_dtor(this);
}

/// `CTaskCategories::ComputeTaskCategory(CTask*)` (0x698680, cdecl): starts at `task != null` (1/0), then
/// three `GetTaskType` calls each test 0x3FB..=0x3FD (-> 4, -> 2, -> 4 as the result).
#[hook(addr = 0x698680, conv = cdecl, name = "CTaskCategories::ComputeTaskCategory", expect = "53 56 8B 74 24 0C")]
pub fn task_categories_compute_task_category(task: GPtr<u8>) -> u8 {
    let kill = |t: i32| (0x3FB..=0x3FD).contains(&t);
    let mut bl = u8::from(!task.is_null());
    if kill(v_get_task_type(task)) {
        bl = 4;
    }
    if kill(v_get_task_type(task)) {
        bl = 2;
    }
    if kill(v_get_task_type(task)) { 4 } else { bl }
}

/// `CTaskComplexCarDriveWander::Clone` (0x63DD00): a 0x24-byte task, `CTaskComplexCarDrive::Constructor_1(
/// +0xC, (float)+0x10, -1, +0x18)`, vtable 0x86E96C; null when the pool is full.
#[hook(addr = 0x63DD00, conv = thiscall, name = "CTaskComplexCarDriveWander::Clone", expect = "6A FF 68 BB 1B 84 00")]
pub fn car_drive_wander_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x24);
    if t.is_null() {
        return t;
    }
    let a3 = rd::<u32>(this, 0x18);
    let f = f32_of(x(rd::<f32>(this, 0x10))).to_bits();
    car_drive_ctor_1(t, rd::<u32>(this, 0xC), f, -1, a3);
    wr::<u32>(t, 0, 0x86E96C);
    t
}

/// `CTaskComplexGangFollower::MakeAbortable(CPed*, int32, const CEvent*)` (0x65EC30): without a sub-task, or
/// when the sub-task agrees, clears ped flags 0x400000 (+0x474) and 0x10000 (+0x478) and returns true.
#[hook(addr = 0x65EC30, conv = thiscall, name = "CTaskComplexGangFollower::MakeAbortable", expect = "8B 49 08 85 C9")]
pub fn gang_follower_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    let sub = rd::<GPtr<u8>>(this, 8);
    if !sub.is_null() {
        if v_make_abortable(sub, ped, priority, event) == 0 {
            return 0;
        }
        let b = rd::<u32>(ped, 0x478) & 0xFFFE_FFFF;
        let a = rd::<u32>(ped, 0x474) & 0xFFBF_FFFF;
        wr::<u32>(ped, 0x474, a);
        wr::<u32>(ped, 0x478, b);
    } else {
        let b = rd::<u32>(ped, 0x478) & 0xFFFE_FFFF;
        let a = rd::<u32>(ped, 0x474) & 0xFFBF_FFFF;
        wr::<u32>(ped, 0x478, b);
        wr::<u32>(ped, 0x474, a);
    }
    1
}

/// `CTaskGangHasslePed::Clone` (0x6620D0): a 0x2C-byte task built by `Constructor(+0xC, +0x10, +0x14,
/// +0x18)` (its return value is returned); null when the pool is full.
#[hook(addr = 0x6620D0, conv = thiscall, name = "CTaskGangHasslePed::Clone", expect = "6A FF 68 CB 49 84 00")]
pub fn gang_hassle_ped_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x2C);
    if t.is_null() {
        return GPtr::NULL;
    }
    gang_hassle_ped_ctor(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18))
}

/// `CTaskSimpleSitIdle::StartAnim(CPed*)` (0x631260): +0x14 = now, +0x18 = +0xC, +0x1C = 1; blends anim
/// (0x35, 0x136) when +8 is set, else (0, 0x96), delta 256.0, into +0x10; when one was created its speed
/// (+0x24) = rand() * [0x858C7C] * [0x858CC8] + [0x858C20] (rand call returns to 0x6312B6).
#[hook(addr = 0x631260, conv = thiscall, name = "CTaskSimpleSitIdle::StartAnim", expect = "56 8B F1 8B 46 0C")]
pub fn sit_idle_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let d = rd::<u32>(this, 0xC);
    wr::<u32>(this, 0x14, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x18, d);
    wr::<u8>(this, 0x1C, 1);
    let clump = rd::<u32>(ped, 0x18);
    let a = if rd::<u8>(this, 8) != 0 {
        anim_blend_animation(clump, 0x35, 0x136, 0x4380_0000)
    } else {
        anim_blend_animation(clump, 0, 0x96, 0x4380_0000)
    };
    wr::<GPtr<u8>>(this, 0x10, a);
    if a.is_null() {
        return;
    }
    let r = sa_core::orig::at_site(0x6312B6, rand);
    let v = xi(r)
        .fmul_m(g::<f32>(0x858C7C).read())
        .fmul_m(g::<f32>(0x858CC8).read())
        .fadd_m(g::<f32>(0x858C20).read());
    wr::<f32>(rd::<GPtr<u8>>(this, 0x10), 0x24, f32_of(v));
}

/// `CTaskComplexPartner::StreamRequiredAnims` (0x682310): unless already done (+0x6E): when the block name
/// at +0x5E starts with the byte at 0x858B54 (one-byte `repe cmpsb`), just marks done; otherwise, when the
/// block is loaded (`ms_aAnimBlocks[i].bLoaded == 1`) adds a reference and marks done, else requests model
/// `i + 0x63E7` with flags 8.
#[hook(addr = 0x682310, conv = thiscall, name = "CTaskComplexPartner::StreamRequiredAnims", expect = "55 8B E9 8A 45 6E")]
pub fn partner_stream_required_anims(this: GPtr<u8>) {
    if rd::<u8>(this, 0x6E) != 0 {
        return;
    }
    if rd::<u8>(this, 0x5E) != g::<u8>(0x858B54).read() {
        let i = anim_get_block_index(this.byte_add(0x5E));
        if rd::<u8>(GPtr::from_addr(0xB5D4B0), (i as isize).wrapping_mul(32)) != 1 {
            streaming_request_model(i.wrapping_add(0x63E7), 8);
            return;
        }
        anim_add_block_ref(i);
    }
    wr::<u8>(this, 0x6E, 1);
}

/// `CTaskComplexGoToPointAndStandStill::SelectMoveState(goTo, CPed*, float walkRadius, float runRadius)`
/// (0x668250, thiscall with `this` unused, `ret 0x10`): 2D squared distance from the ped (matrix position,
/// else +4) to the go-to task's target (+0xC/+0x10); `goTo->+8` = 4 when strictly below walkRadius²,
/// else 6 when strictly below runRadius², else 7 (equal/NaN take the larger state).
#[hook(addr = 0x668250, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::SelectMoveState", expect = "8B 4C 24 08 8B 41 14", preserve = "edx")]
pub fn go_to_point_and_stand_still_select_move_state(_this: GPtr<u8>, go_to: GPtr<u8>, ped: GPtr<u8>, walk: f32, run: f32) {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(go_to, 0xC));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(go_to, 0x10));
    let d2 = dx.fmul(dx).fadd(dy.fmul(dy));
    let state = if d2.fcom(x(walk).fmul_m(walk)) == sa_math::x87::Cmp::Less {
        4
    } else if d2.fcom(x(run).fmul_m(run)) == sa_math::x87::Cmp::Less {
        6
    } else {
        7
    };
    wr::<u32>(go_to, 8, state);
}

/// `CTaskSimplePlayHandSignalAnim::Clone` (0x61B980): a 0x24-byte task built by the constructor
/// `(+0x10, +0x14, byte +0x18, (byte +0xC & 4) >> 2)` (its return value is returned); null when the pool is
/// full.
#[hook(addr = 0x61B980, conv = thiscall, name = "CTaskSimplePlayHandSignalAnim::Clone", expect = "6A FF 68 9B F2 83 00")]
pub fn play_hand_signal_anim_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x24);
    if t.is_null() {
        return GPtr::NULL;
    }
    let a3 = u32::from((rd::<u8>(this, 0xC) & 4) >> 2);
    let a2 = u32::from(rd::<u8>(this, 0x18));
    play_hand_signal_anim_ctor_orig(t, rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), a2, a3)
}

/// `CTaskComplexSmartFleeEntity::Constructor(entity, bool a1, a2, a3, a4, a5)` (0x65C430): `CTaskComplex`
/// base, +0xC = entity, +0x1C = a3, +0x20 = a2, byte +0x24 = a1, +0x28 = a4, +0x2C = a5, vtable 0x86F784,
/// +0x30 = 7, +0x34 = +0x38 = 0, bytes +0x3C/+0x3D = 0; registers the entity reference when non-null.
#[hook(addr = 0x65C430, conv = thiscall, name = "CTaskComplexSmartFleeEntity::Constructor", expect = "6A FF 68 28 40 84 00")]
pub fn smart_flee_entity_ctor(this: GPtr<u8>, a0: u32, a1: u8, a2: u32, a3: u32, a4: u32, a5: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, a0);
    wr::<u32>(this, 0x1C, a3);
    wr::<u32>(this, 0x20, a2);
    wr::<u8>(this, 0x24, a1);
    wr::<u32>(this, 0x28, a4);
    wr::<u32>(this, 0x2C, a5);
    wr::<u32>(this, 0, 0x86F784);
    wr::<u32>(this, 0x30, 7);
    wr::<u32>(this, 0x34, 0);
    wr::<u32>(this, 0x38, 0);
    wr::<u8>(this, 0x3C, 0);
    wr::<u8>(this, 0x3D, 0);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexPartnerDeal::StreamRequiredAnims` (0x6823C0): not yet streamed (+0x6E == 0): when gang anims
/// should load, adds a reference to the block named at +0x5E if loaded (and marks +0x6E) or requests it
/// (model `i + 0x63E7`, flags 8). Streamed: when they should no longer load, removes the reference and
/// clears +0x6E.
#[hook(addr = 0x6823C0, conv = thiscall, name = "CTaskComplexPartnerDeal::StreamRequiredAnims", expect = "56 8B F1 8A 46 6E")]
pub fn partner_deal_stream_required_anims(this: GPtr<u8>) {
    if rd::<u8>(this, 0x6E) == 0 {
        if should_load_gang_anims(this) == 0 {
            return;
        }
        let i = anim_get_block_index(this.byte_add(0x5E));
        if rd::<u8>(GPtr::from_addr(0xB5D4B0), (i as isize).wrapping_mul(32)) != 1 {
            streaming_request_model(i.wrapping_add(0x63E7), 8);
            return;
        }
        anim_add_block_ref(i);
        wr::<u8>(this, 0x6E, 1);
    } else if should_load_gang_anims(this) == 0 {
        let i = anim_get_block_index(this.byte_add(0x5E));
        anim_remove_block_ref(i);
        wr::<u8>(this, 0x6E, 0);
    }
}

/// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x61A6F0): vtable 0x86D504; with an anim (+8): finish callback reset
/// to 0x4D1390 / null, flag word +0x2E |= 4; unless bit 2 of +0xC is set, when blendAmount (+0x18) > the
/// constant at 0x858B50 and blendDelta (+0x1C) is not below it, blendDelta = -4.0; then +8 = null;
/// `CTaskSimple` destructor.
#[hook(addr = 0x61A6F0, conv = thiscall, name = "CTaskSimpleAnim::~CTaskSimpleAnim", expect = "6A FF 68 08 F1 83 00")]
pub fn simple_anim_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D504);
    let a = rd::<GPtr<u8>>(this, 8);
    if !a.is_null() {
        anim_set_finish_callback(a, 0x4D1390, 0);
        let a = rd::<GPtr<u8>>(this, 8);
        wr::<u16>(a, 0x2E, rd::<u16>(a, 0x2E) | 4);
        if rd::<u8>(this, 0xC) & 4 == 0 {
            let a = rd::<GPtr<u8>>(this, 8);
            let k = x(g::<f32>(0x858B50).read());
            if !x(rd::<f32>(a, 0x18)).fcom(k).test_any(0x41) && !x(rd::<f32>(a, 0x1C)).fcom(k).test_any(0x01) {
                wr::<u32>(a, 0x1C, 0xC080_0000);
            }
        }
        wr::<u32>(this, 8, 0);
    }
    task_simple_dtor(this);
}

/// `CTaskComplexLeaveCar::~CTaskComplexLeaveCar` (0x63B970): vtable 0x86E828; with a vehicle (+0xC): clears
/// its getting-out flags for door bits +0x20, subtracts byte +0x21 from the vehicle's byte +0x485 and releases
/// the reference; destroys and frees the line-up utility (+0x1C, CRT heap); `CTaskComplex` destructor.
#[hook(addr = 0x63B970, conv = thiscall, name = "CTaskComplexLeaveCar::~CTaskComplexLeaveCar", expect = "6A FF 68 38 16 84 00")]
pub fn leave_car_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E828);
    let v = rd::<GPtr<u8>>(this, 0xC);
    if !v.is_null() {
        vehicle_clear_getting_out_flags(v, u32::from(rd::<u8>(this, 0x20)));
        let v = rd::<GPtr<u8>>(this, 0xC);
        wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_sub(rd::<u8>(this, 0x21)));
        entity_clean_up_old_reference(rd::<GPtr<u8>>(this, 0xC), this.byte_add(0xC));
    }
    let u = rd::<GPtr<u8>>(this, 0x1C);
    if !u.is_null() {
        line_up_ped_with_car_dtor(u);
        crt_free(u);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexPartner::~CTaskComplexPartner` (0x683A40): vtable 0x870664; releases the partner reference
/// at +0x38; when the anims were streamed (+0x6E), removes the block reference unless the block name at +0x5E
/// starts with the byte at 0x858B54, and clears +0x6E; `CTaskComplex` destructor.
#[hook(addr = 0x683A40, conv = thiscall, name = "CTaskComplexPartner::~CTaskComplexPartner", expect = "6A FF 68 B8 68 84 00")]
pub fn partner_dtor_port(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870664);
    let e = rd::<GPtr<u8>>(this, 0x38);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x38));
    }
    if rd::<u8>(this, 0x6E) != 0 {
        if rd::<u8>(this, 0x5E) != g::<u8>(0x858B54).read() {
            let i = anim_get_block_index(this.byte_add(0x5E));
            anim_remove_block_ref(i);
        }
        wr::<u8>(this, 0x6E, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexWanderStandard::WillChat(CPed* a, CPed* b)` (0x66AED0): false when either ped type (+0x598)
/// is 0x14 or 6, either ped is a player, or `FUN_005fe9c0` holds for either type; else `AreFriends(a, b)`.
#[hook(addr = 0x66AED0, conv = thiscall, name = "CTaskComplexWanderStandard::WillChat", expect = "56 57 8B 7C 24 0C")]
pub fn wander_standard_will_chat(_this: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u8 {
    let ta = rd::<i32>(a, 0x598);
    if ta == 0x14 {
        return 0;
    }
    let tb = rd::<i32>(b, 0x598);
    if tb == 0x14 || ta == 6 || tb == 6 {
        return 0;
    }
    if ped_is_player(a) != 0 || ped_is_player(b) != 0 {
        return 0;
    }
    if ped_type_5fe9c0(rd::<i32>(a, 0x598)) != 0 || ped_type_5fe9c0(rd::<i32>(b, 0x598)) != 0 {
        return 0;
    }
    u8::from(ped_intelligence_are_friends(a, b) != 0)
}

/// `CTaskComplexBeInCouple::MakeAbortable(CPed*, int32, const CEvent*)` (0x6847C0): does not ask the
/// sub-task. With a partner (+0x10) and an event whose `HasEditableResponse` holds: with the event's +0xC byte
/// cleared meanwhile, events of type 9, 0x1F or 0xF (one virtual call per test) are added to the partner's
/// event group (intelligence +0x68, `Add(event, false)`); the byte is restored. Then `AbortArmIK(ped)`; true.
#[hook(addr = 0x6847C0, conv = thiscall, name = "CTaskComplexBeInCouple::MakeAbortable", expect = "57 8B F9 8B 47 10")]
pub fn be_in_couple_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, _priority: i32, event: GPtr<u8>) -> u8 {
    if !rd::<GPtr<u8>>(this, 0x10).is_null() && !event.is_null() && v_event_has_editable_response(event) != 0 {
        let bl = rd::<u8>(event, 0xC);
        wr::<u8>(event, 0xC, 0);
        if v_event_get_type(event) == 9 || v_event_get_type(event) == 0x1F || v_event_get_type(event) == 0xF {
            let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x47C);
            event_group_add(intel.byte_add(0x68), event, 0);
        }
        wr::<u8>(event, 0xC, bl);
    }
    be_in_couple_abort_arm_ik_orig(this, ped);
    1
}

/// `CTaskComplexPlayHandSignalAnim::CreateFirstSubTask(CPed*)` (0x61B4F0): null while arm 1 is pointing;
/// resolves the anim id (+0xC) through `GetAnimIdForPed` when -1 (null if still -1); bit 3 of +0x14 = ped
/// model (+0x22, signed word) is 0x67, 0x69 or 0x6B; then `CreateSubTask(0xCB)`.
#[hook(addr = 0x61B4F0, conv = thiscall, name = "CTaskComplexPlayHandSignalAnim::CreateFirstSubTask", expect = "56 57 8B 7C 24 0C")]
pub fn play_hand_signal_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if ik_is_arm_pointing(GPtr::from_addr(0xC15448), 1, ped) != 0 {
        return GPtr::NULL;
    }
    if rd::<i32>(this, 0xC) == -1 {
        let a = play_hand_signal_get_anim_id_orig(this, ped);
        wr::<i32>(this, 0xC, a);
        if a == -1 {
            return GPtr::NULL;
        }
    }
    let m = i32::from(rd::<i16>(ped, 0x22));
    if m == 0x67 || m == 0x69 || m == 0x6B {
        wr::<u8>(this, 0x14, rd::<u8>(this, 0x14) | 8);
    } else {
        wr::<u8>(this, 0x14, rd::<u8>(this, 0x14) & 0xF7);
    }
    play_hand_signal_create_sub_task(this, 0xCB)
}

/// `CTaskComplexPartnerDeal::Clone` (0x684D10): a 0x74-byte task built by `Constructor(this + 0x14, +0x38,
/// byte +0x58, +0x3C, CVector +0x40)` (its return value is returned); null when the pool is full.
#[hook(addr = 0x684D10, conv = thiscall, name = "CTaskComplexPartnerDeal::Clone", expect = "6A FF 68 6B 6A 84 00")]
pub fn partner_deal_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x74);
    if t.is_null() {
        return GPtr::NULL;
    }
    let (vx, vy, vz) = (rd::<u32>(this, 0x40), rd::<u32>(this, 0x44), rd::<u32>(this, 0x48));
    let a3 = rd::<u32>(this, 0x3C);
    let a2 = u32::from(rd::<u8>(this, 0x58));
    let a1 = rd::<u32>(this, 0x38);
    partner_deal_ctor(t, this.byte_add(0x14), a1, a2, a3, vx, vy, vz)
}

/// `CTaskComplexGangFollower::Clone` (0x65ECB0): a 0x4C-byte task built by `Constructor(+0xC, +0x10,
/// byte +0x3C, CVector +0x20, +0x38)`; then copies bit 2 of byte +0x3D into the result (also when the pool was
/// full and the result is null: the original writes through null there).
#[hook(addr = 0x65ECB0, conv = thiscall, name = "CTaskComplexGangFollower::Clone", expect = "6A FF 68 2B 44 84 00")]
pub fn gang_follower_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x4C);
    let r = if t.is_null() {
        GPtr::NULL
    } else {
        let a6 = rd::<u32>(this, 0x38);
        let (vx, vy, vz) = (rd::<u32>(this, 0x20), rd::<u32>(this, 0x24), rd::<u32>(this, 0x28));
        let a2 = u32::from(rd::<u8>(this, 0x3C));
        gang_follower_ctor(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), a2, vx, vy, vz, a6)
    };
    let cl = rd::<u8>(r, 0x3D);
    let dl = (rd::<u8>(this, 0x3D) ^ cl) & 4;
    wr::<u8>(r, 0x3D, cl ^ dl);
    r
}

/// `CTaskComplexWanderStandard::Clone` (0x48E530): a 0x38-byte task, `CTaskComplexWander::Constructor(+0xC,
/// byte +0x10, byte +0x24 & 1, [0x86FCB0])`, vtable 0x85A200, the wander timer fields (+0x28..+0x34) cleared;
/// null when the pool is full.
#[hook(addr = 0x48E530, conv = thiscall, name = "CTaskComplexWanderStandard::Clone", expect = "6A FF 68 DB 85 83 00")]
pub fn wander_standard_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x38);
    if t.is_null() {
        return GPtr::NULL;
    }
    let k = g::<u32>(0x86FCB0).read();
    let a2 = u32::from(rd::<u8>(this, 0x24)) & 0xFFFF_FF01;
    let a1 = u32::from(rd::<u8>(this, 0x10));
    wander_ctor_66f450(t, rd::<u32>(this, 0xC), a1, a2, k);
    wr::<u32>(t, 0, 0x85A200);
    wr::<u32>(t, 0x28, 0);
    wr::<u32>(t, 0x2C, 0);
    wr::<u8>(t, 0x30, 0);
    wr::<u8>(t, 0x31, 0);
    wr::<u32>(t, 0x34, 0);
    t
}

/// `__init_CTaskSimpleGoTo__ms_fLookAtThresholdDotProduct` (0x852840): `fcos` of the double at 0x870190,
/// stored as f32 at 0xC18D48 (static initialiser).
#[hook(addr = 0x852840, conv = cdecl, name = "__init_CTaskSimpleGoTo__ms_fLookAtThresholdDotProduct", expect = "DD 05 90 01 87 00")]
pub fn init_simple_go_to_look_at_threshold() {
    let v = X87::from_f64_exact(g::<f64>(0x870190).read()).fcos();
    g::<f32>(0xC18D48).write(f32_of(v));
}

/// `__init_CPedGroupDefaultTaskAllocators__ms_FollowAnyMeansAllocator` (0x851270): ctor 0x5F6450 on
/// 0xC09908, register 0x856800; returns `_atexit`'s result.
#[hook(addr = 0x851270, conv = cdecl, name = "__init_CPedGroupDefaultTaskAllocators__ms_FollowAnyMeansAllocator", expect = "B9 08 99 C0 00")]
pub fn init_allocator_follow_any_means() -> i32 {
    alloc_ctor_5f6450(GPtr::from_addr(0xC09908));
    crt_atexit(0x856800)
}

/// `__init_CPedGroupDefaultTaskAllocators__ms_FollowLimitedAllocator` (0x851290): ctor 0x5F6480 on
/// 0xC0990C, register 0x856810.
#[hook(addr = 0x851290, conv = cdecl, name = "__init_CPedGroupDefaultTaskAllocators__ms_FollowLimitedAllocator", expect = "B9 0C 99 C0 00")]
pub fn init_allocator_follow_limited() -> i32 {
    alloc_ctor_5f6480(GPtr::from_addr(0xC0990C));
    crt_atexit(0x856810)
}

/// `__init_CPedGroupDefaultTaskAllocators__ms_StandStillAllocator` (0x8512B0): ctor 0x5F64B0 on 0xC09910,
/// register 0x856820.
#[hook(addr = 0x8512B0, conv = cdecl, name = "__init_CPedGroupDefaultTaskAllocators__ms_StandStillAllocator", expect = "B9 10 99 C0 00")]
pub fn init_allocator_stand_still() -> i32 {
    alloc_ctor_5f64b0(GPtr::from_addr(0xC09910));
    crt_atexit(0x856820)
}

/// `__init_CPedGroupDefaultTaskAllocators__ms_ChatAllocator` (0x8512D0): ctor 0x5F64E0 on 0xC09914,
/// register 0x856830.
#[hook(addr = 0x8512D0, conv = cdecl, name = "__init_CPedGroupDefaultTaskAllocators__ms_ChatAllocator", expect = "B9 14 99 C0 00")]
pub fn init_allocator_chat() -> i32 {
    alloc_ctor_5f64e0(GPtr::from_addr(0xC09914));
    crt_atexit(0x856830)
}

/// `__init_CPedGroupDefaultTaskAllocators__ms_RandomAllocator` (0x8512F0): ctor 0x5F6510 on 0xC09918,
/// register 0x856840.
#[hook(addr = 0x8512F0, conv = cdecl, name = "__init_CPedGroupDefaultTaskAllocators__ms_RandomAllocator", expect = "B9 18 99 C0 00")]
pub fn init_allocator_random() -> i32 {
    alloc_ctor_5f6510(GPtr::from_addr(0xC09918));
    crt_atexit(0x856840)
}

/// `__init_CPedGroupDefaultTaskAllocators__ms_SitInLeaderCarAllocator` (0x851310): ctor 0x5F6540 on
/// 0xC0991C, register 0x856850.
#[hook(addr = 0x851310, conv = cdecl, name = "__init_CPedGroupDefaultTaskAllocators__ms_SitInLeaderCarAllocator", expect = "B9 1C 99 C0 00")]
pub fn init_allocator_sit_in_leader_car() -> i32 {
    alloc_ctor_5f6540(GPtr::from_addr(0xC0991C));
    crt_atexit(0x856850)
}

/// `__init_CTaskComplexFollowLeaderInFormation__ms_offsets` (0x852F10): ctor 0x694850 on 0xC196E8,
/// register 0x8569B0.
#[hook(addr = 0x852F10, conv = cdecl, name = "__init_CTaskComplexFollowLeaderInFormation__ms_offsets", expect = "B9 E8 96 C1 00")]
pub fn init_follow_leader_in_formation_offsets() -> i32 {
    formation_offsets_ctor(GPtr::from_addr(0xC196E8));
    crt_atexit(0x8569B0)
}

/// `__init_CPedScriptedTaskRecord__ms_scriptedTasks` (0x851680): 0x80 records of 0x14 bytes at 0xC0B1E8
/// (ctor 0x608330, dtor 0x6082E0), register 0x8568B0.
#[hook(addr = 0x851680, conv = cdecl, name = "__init_CPedScriptedTaskRecord__ms_scriptedTasks", expect = "68 E0 82 60 00")]
pub fn init_ped_scripted_task_record() -> i32 {
    crt_eh_vector_ctor(GPtr::from_addr(0xC0B1E8), 0x14, 0x80, 0x608330, 0x6082E0);
    crt_atexit(0x8568B0)
}

/// `__init_CTaskSequences__ms_taskSequence` (0x8520E0): 0x40 sequences of 0x40 bytes at 0xC178F0
/// (ctor 0x632BD0, dtor 0x6389F0), register 0x856920.
#[hook(addr = 0x8520E0, conv = cdecl, name = "__init_CTaskSequences__ms_taskSequence", expect = "68 F0 89 63 00")]
pub fn init_task_sequences() -> i32 {
    crt_eh_vector_ctor(GPtr::from_addr(0xC178F0), 0x40, 0x40, 0x632BD0, 0x6389F0);
    crt_atexit(0x856920)
}

/// `__init_CScriptedBrainTaskStore__ms_entries` (0x852110): 0x30 entries of 0x10 bytes at 0xC188F0
/// (ctor 0x62EC40, dtor 0x62EC50), register 0x856900.
#[hook(addr = 0x852110, conv = cdecl, name = "__init_CScriptedBrainTaskStore__ms_entries", expect = "68 50 EC 62 00")]
pub fn init_scripted_brain_task_store() -> i32 {
    crt_eh_vector_ctor(GPtr::from_addr(0xC188F0), 0x10, 0x30, 0x62EC40, 0x62EC50);
    crt_atexit(0x856900)
}

/// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl; tail at 0x61A371 reached through the protection island
/// 0x4011AF, plain code): true when `GetTaskType()` is 0x384, else when a second `GetTaskType()` is 0x39E.
#[hook(addr = 0x61A360, conv = cdecl, name = "CTask::IsGoToTask", expect = "56 8B 74 24 08")]
pub fn task_is_go_to_task(task: GPtr<u8>) -> u8 {
    if v_get_task_type(task) == 0x384 {
        return 1;
    }
    u8::from(v_get_task_type(task) == 0x39E)
}

/// `CTaskComplexEvasiveStep::CTaskComplexEvasiveStep(CEntity*, const CVector*)` (0x6532D0; the island at
/// 0x4074EF only pushes the SEH handler, the body continues at 0x6532D7): `CTaskComplex` base, vtable
/// 0x86F218, +0xC..+0x14 = the vector, +0x18 = entity (reference registered when non-null).
#[hook(addr = 0x6532D0, conv = thiscall, name = "CTaskComplexEvasiveStep::CTaskComplexEvasiveStep", expect = "6A FF E9 18 42 DB FF")]
pub fn evasive_step_complex_ctor(this: GPtr<u8>, ent: GPtr<u8>, pos: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x86F218);
    wr::<u32>(this, 0xC, rd::<u32>(pos, 0));
    wr::<u32>(this, 0x10, rd::<u32>(pos, 4));
    wr::<u32>(this, 0x14, rd::<u32>(pos, 8));
    wr::<GPtr<u8>>(this, 0x18, ent);
    if !ent.is_null() {
        entity_register_reference(ent, this.byte_add(0x18));
    }
    this
}

/// `CTaskComplexBeInCouple::CTaskComplexBeInCouple(CPed* partner, bool a1, bool a2, bool a3, a4)` (0x6836F0;
/// island 0x408055 pushes the SEH handler, body at 0x6836F7): `CTaskComplex` base, bytes +0x14/+0x15 = a1/a2,
/// vtable 0x870724, +0x10 = partner, byte +0x16 = a3, +0x18 = a4, byte +0x1C = 0; registers the partner
/// reference when non-null.
#[hook(addr = 0x6836F0, conv = thiscall, name = "CTaskComplexBeInCouple::CTaskComplexBeInCouple", expect = "6A FF E9 5E 49 D8 FF")]
pub fn be_in_couple_ctor(this: GPtr<u8>, partner: GPtr<u8>, a1: u8, a2: u8, a3: u8, a4: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u8>(this, 0x14, a1);
    wr::<u8>(this, 0x15, a2);
    wr::<u32>(this, 0, 0x870724);
    wr::<GPtr<u8>>(this, 0x10, partner);
    wr::<u8>(this, 0x16, a3);
    wr::<u32>(this, 0x18, a4);
    wr::<u8>(this, 0x1C, 0);
    if !partner.is_null() {
        entity_register_reference(partner, this.byte_add(0x10));
    }
    this
}

/// `CTaskComplexWanderStandard::SetNextMinScanTime(CPed*)` (0x66AF60; island 0x401390, body at 0x66AF6B):
/// when the ped's active task has this task's type and wander type, sets that active task's +0x34 to
/// now + 100000 (the active task is fetched again for the write).
#[hook(addr = 0x66AF60, conv = thiscall, name = "CTaskComplexWanderStandard::SetNextMinScanTime", expect = "53 8B 5C 24 08")]
pub fn wander_standard_set_next_min_scan_time(this: GPtr<u8>, ped: GPtr<u8>) {
    let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    let t = task_manager_get_active_task(tm);
    if t.is_null() {
        return;
    }
    let a = v_get_task_type(t);
    if a != v_get_task_type(this) {
        return;
    }
    let w = v_get_wander_type(t);
    if w != v_get_wander_type(this) {
        return;
    }
    let t = task_manager_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    wr::<u32>(t, 0x34, g::<u32>(0xB7CB84).read().wrapping_add(0x186A0));
}

/// `CTaskManager::FindActiveTaskByType(int32 type)` (0x681740): walks the chain (`GetSubTask`) of the first
/// non-null primary task only (slots 0..5), then, if nothing matched, the chains of the six secondary tasks
/// (+0x14..); returns the first task whose `GetTaskType()` equals `type`, else null. After a match the
/// original still calls `GetSubTask` once on the matched task before leaving the chain (kept).
#[hook(addr = 0x681740, conv = thiscall, name = "CTaskManager::FindActiveTaskByType", expect = "53 56 57 33 DB")]
pub fn task_manager_find_active_task_by_type(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let mut found = GPtr::<u8>::NULL;
    let walk = |mut t: GPtr<u8>, found: &mut GPtr<u8>| {
        while !t.is_null() && found.is_null() {
            if v_get_task_type(t) == ty {
                *found = t;
            }
            t = v_get_sub_task(t);
        }
    };
    if let Some(i) = (0..5).find(|&i| !rd::<GPtr<u8>>(this, i * 4).is_null()) {
        walk(rd::<GPtr<u8>>(this, i * 4), &mut found);
        if !found.is_null() {
            return found;
        }
    }
    for j in 0..6 {
        walk(rd::<GPtr<u8>>(this, 0x14 + j * 4), &mut found);
    }
    found
}

/// `CTaskSimpleStandStill::Clone` (0x635CF0): a 0x20-byte task with the inlined constructor: `CTaskSimple`
/// base, +8 = duration, vtable 0x86DD2C, timer (+0xC, +0x10, bytes +0x14/+0x15) cleared, bytes +0x18/+0x19
/// and +0x1C copied; null when the pool is full.
#[hook(addr = 0x635CF0, conv = thiscall, name = "CTaskSimpleStandStill::Clone", expect = "6A FF 68 DB 0A 84 00")]
pub fn stand_still_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x20);
    if t.is_null() {
        return GPtr::NULL;
    }
    let b19 = rd::<u8>(this, 0x19);
    let d1c = rd::<u32>(this, 0x1C);
    let b18 = rd::<u8>(this, 0x18);
    let d8 = rd::<u32>(this, 8);
    task_simple_ctor(t);
    wr::<u32>(t, 8, d8);
    wr::<u32>(t, 0, 0x86DD2C);
    wr::<u32>(t, 0xC, 0);
    wr::<u32>(t, 0x10, 0);
    wr::<u8>(t, 0x14, 0);
    wr::<u8>(t, 0x15, 0);
    wr::<u8>(t, 0x18, b18);
    wr::<u8>(t, 0x19, b19);
    wr::<u32>(t, 0x1C, d1c);
    t
}

/// `CTaskSimpleRunNamedAnim::Clone` (0x61B770): a 0x64-byte task built by `Constructor-Anim(this + 0x10,
/// this + 0x28, +0x5C, +0x38, +0x40, flag bits 1, 4, 5, 2 of byte +0xC)` (its return value is returned); null
/// when the pool is full. The bit-1 argument's upper 24 bits are stale EDX in the original (callee reads a
/// bool byte); passed zero-extended here.
#[hook(addr = 0x61B770, conv = thiscall, name = "CTaskSimpleRunNamedAnim::Clone", expect = "6A FF 68 3B F2 83 00")]
pub fn run_named_anim_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x64);
    if t.is_null() {
        return GPtr::NULL;
    }
    let f = rd::<u8>(this, 0xC);
    let bit = |m: u8, s: u8| u32::from((f & m) >> s);
    run_named_anim_ctor(
        t,
        this.byte_add(0x10),
        this.byte_add(0x28),
        rd::<u32>(this, 0x5C),
        rd::<u32>(this, 0x38),
        rd::<u32>(this, 0x40),
        bit(2, 1),
        bit(0x10, 4),
        bit(0x20, 5),
        bit(4, 2),
    )
}

/// `CTaskSimpleRunNamedAnim::StartAnim(CPed*)` (0x61BB10): starts the timer when the duration (+0x40) is
/// >= 0; sets anim flag 4 in +0x5C unless flag 8 is set; blends the hierarchy (+0x3C) with flags +0x5C and
/// delta +0x38 into +8; finish callback `FinishRunAnimMovePedCB` when bit 5 of +0xC is set, else
/// `FinishRunAnimCB` as finish callback (flag 8 or bit 4 of +0xC) or delete callback; +0x60 = anim id
/// (+0x2C word).
#[hook(addr = 0x61BB10, conv = thiscall, name = "CTaskSimpleRunNamedAnim::StartAnim", expect = "56 8B F1 8B 46 40")]
pub fn run_named_anim_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let d = rd::<i32>(this, 0x40);
    if d >= 0 {
        wr::<u32>(this, 0x44, g::<u32>(0xB7CB84).read());
        wr::<i32>(this, 0x48, d);
        wr::<u8>(this, 0x4C, 1);
    }
    let fl = rd::<u32>(this, 0x5C);
    if fl & 8 == 0 {
        wr::<u32>(this, 0x5C, fl | 4);
    }
    let a = anim_blend_animation_hier(rd::<u32>(ped, 0x18), rd::<u32>(this, 0x3C), rd::<u32>(this, 0x5C), rd::<u32>(this, 0x38));
    let b = rd::<u8>(this, 0xC);
    wr::<GPtr<u8>>(this, 8, a);
    if b & 0x20 != 0 {
        anim_set_finish_callback(a, 0x61AAA0, this.addr() as u32);
    } else if rd::<u8>(this, 0x5C) & 8 != 0 || b & 0x10 != 0 {
        anim_set_finish_callback(a, 0x61A8A0, this.addr() as u32);
    } else {
        anim_set_delete_callback(a, 0x61A8A0, this.addr() as u32);
    }
    wr::<u16>(this, 0x60, rd::<u16>(rd::<GPtr<u8>>(this, 8), 0x2C));
}

/// `CTaskComplexLeaveCar::ControlSubTask(CPed*)` (0x6421B0): null without a vehicle (+0xC). When the ped is
/// not in a vehicle (bit 8 of +0x46C) and the sub-task type is none of 0x330, 0x32E, 0xCE, 0xD4 (one virtual
/// call per test): `CreateSubTask(0x330)`. Otherwise, when +0x18 is clear and the sub-task is 0x329, asks it
/// to abort (`MakeAbortable(ped, 0, null)`); returns the sub-task.
#[hook(addr = 0x6421B0, conv = thiscall, name = "CTaskComplexLeaveCar::ControlSubTask", expect = "56 8B F1 8B 46 0C")]
pub fn leave_car_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        return GPtr::NULL;
    }
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(ped, 0x46C) & 0x100 == 0
        && v_get_task_type(sub()) != 0x330
        && v_get_task_type(sub()) != 0x32E
        && v_get_task_type(sub()) != 0xCE
        && v_get_task_type(sub()) != 0xD4
    {
        return leave_car_create_sub_task(this, 0x330, ped);
    }
    if rd::<u8>(this, 0x18) == 0 && v_get_task_type(sub()) == 0x329 {
        v_make_abortable(sub(), ped, 0, GPtr::NULL);
    }
    sub()
}

/// `CTaskSimpleCarDrive::SetPedPosition(CPed*)` (0x63C770): when flag 8 of +0x5C is set, takes over the
/// line-up utility (+0x10) of another active car-drive task (`FindTaskByType(4, 0x2C5)`) and clears the flag.
/// With a utility: finds the first playing association among the 12 anim ids at 0x8D2E9C, runs the
/// utility's `ProcessPed(ped, vehicle, assoc)`, destroys and frees it (CRT heap), +0x10 = null. Without:
/// `SetPedPositionInCar` when the ped is in this task's vehicle. Always true.
#[hook(addr = 0x63C770, conv = thiscall, name = "CTaskSimpleCarDrive::SetPedPosition", expect = "53 8B 5C 24 08")]
pub fn car_drive_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x5C) & 8 != 0 {
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        let t = task_manager_find_task_by_type(tm, 4, 0x2C5);
        if !t.is_null() && t != this {
            let u = rd::<u32>(t, 0x10);
            if u != 0 {
                wr::<u32>(this, 0x10, u);
                wr::<u32>(t, 0x10, 0);
            }
        }
        wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) & 0xF7);
    }
    if !rd::<GPtr<u8>>(this, 0x10).is_null() {
        let mut a = GPtr::<u8>::NULL;
        for i in 0..12usize {
            a = clump_get_association(rd::<u32>(ped, 0x18), g::<u32>(0x8D2E9C + i * 4).read());
            if !a.is_null() {
                break;
            }
        }
        let veh = rd::<GPtr<u8>>(this, 8);
        line_up_ped_with_car_process_ped(rd::<GPtr<u8>>(this, 0x10), ped, veh, a);
        let u = rd::<GPtr<u8>>(this, 0x10);
        if !u.is_null() {
            line_up_ped_with_car_dtor(u);
            crt_free(u);
        }
        wr::<u32>(this, 0x10, 0);
        return 1;
    }
    let v = rd::<GPtr<u8>>(this, 8);
    if !v.is_null() && rd::<GPtr<u8>>(ped, 0x58C) == v && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        ped_set_ped_position_in_car(ped);
    }
    1
}

/// `CTaskSimpleFacial::ProcessPed(CPed*)` (0x692E80). With anim 0xA0 (talking) playing on the clump: when
/// this task's anim is 0xA0, `(rand() & 0xFFFF) * [0x858B14] * [0x858628]` < 40 (ftol) re-randomises that
/// anim's speed (+0x24) = `rand() * [0x858C7C] * [0x858FA0] + [0x858B8C]` (rand returns to 0x692EC3 and
/// 0x692EEB), false; otherwise the own anim (if any) is blended out (flag 4 at +0x2E, delta -4.0), true.
/// Without 0xA0: first call blends the anim in (delta 4.0) and starts the timer (+8 = now, +0xC = +0x18,
/// +0x10 = 1); later, done when the anim is gone, or blends it out once the timer ran out.
#[hook(addr = 0x692E80, conv = thiscall, name = "CTaskSimpleFacial::ProcessPed", expect = "53 55 56 8B F1")]
pub fn facial_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let id = facial_get_anim_id(rd::<u32>(this, 0x14));
    let a = clump_get_association(rd::<u32>(ped, 0x18), id);
    let talk = clump_get_association(rd::<u32>(ped, 0x18), 0xA0);
    if !talk.is_null() {
        if id != 0xA0 {
            if !a.is_null() {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
                wr::<u32>(a, 0x1C, 0xC080_0000);
            }
            return 1;
        }
        let r = sa_core::orig::at_site(0x692EC3, rand) & 0xFFFF;
        let v = xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x858628).read());
        if ftol32(v) < 0x28 {
            let r = sa_core::orig::at_site(0x692EEB, rand);
            let s = xi(r)
                .fmul_m(g::<f32>(0x858C7C).read())
                .fmul_m(g::<f32>(0x858FA0).read())
                .fadd_m(g::<f32>(0x858B8C).read());
            wr::<f32>(a, 0x24, f32_of(s));
        }
        return 0;
    }
    if rd::<u8>(this, 0x10) == 0 {
        if !a.is_null() {
            return 1;
        }
        anim_blend_animation(rd::<u32>(ped, 0x18), 0, id, 0x4080_0000);
        wr::<u32>(this, 0xC, rd::<u32>(this, 0x18));
        wr::<u32>(this, 8, g::<u32>(0xB7CB84).read());
        wr::<u8>(this, 0x10, 1);
        return 0;
    }
    if a.is_null() {
        return 1;
    }
    if task_timer_is_out_of_time(this.byte_add(8)) == 0 {
        return 0;
    }
    wr::<u32>(a, 0x1C, 0xC080_0000);
    1
}

/// `CTaskSimpleCarDrive::TriggerIK(CPed*)` (0x63C500): with a vehicle (+8), by its mission byte +0x3BA:
/// missions 2..=6: unless already looking, `(rand() & 0xFFFF) * [0x858B14] * [0x858628]` < 5 (ftol; rand
/// returns to 0x63C5DC) makes the ped look at the player (bone 5); missions 0xF/0x10: unless already
/// looking, `GetRandomNumberInRange(0, 100)` < 5 and a vehicle target (+0x41C, entity type 2) make it look at
/// that vehicle's driver (bone 5) or, without a driver, at the vehicle (bone -1). Other missions draw nothing.
/// LookAt: name 0x86E928, 3000 ms, no offset, speed 0.25, blend 500, priority 3.
#[hook(addr = 0x63C500, conv = thiscall, name = "CTaskSimpleCarDrive::TriggerIK", expect = "57 8B F9 8B 47 08")]
pub fn car_drive_trigger_ik(this: GPtr<u8>, ped: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 8);
    if veh.is_null() {
        return;
    }
    let ik = GPtr::<u8>::from_addr(0xC15448);
    let look = |target: GPtr<u8>, bone: i32| ik_look_at(ik, 0x86E928, ped, target, 0xBB8, bone, 0, 0, 0x3E80_0000, 0x1F4, 3, 0);
    match rd::<u8>(veh, 0x3BA) {
        2..=6 => {
            if ik_is_looking(ik, ped) != 0 {
                return;
            }
            let r = sa_core::orig::at_site(0x63C5DC, rand) & 0xFFFF;
            let v = xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x858628).read());
            if ftol32(v) >= 5 {
                return;
            }
            let p = find_player_ped(0);
            look(p, 5);
        }
        0xF | 0x10 => {
            if ik_is_looking(ik, ped) != 0 {
                return;
            }
            if general_random_int_in_range(0, 100) >= 5 {
                return;
            }
            let t = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x41C);
            if t.is_null() || rd::<u8>(t, 0x36) & 7 != 2 {
                return;
            }
            let d = rd::<GPtr<u8>>(t, 0x460);
            if d.is_null() {
                look(t, -1);
            } else {
                look(d, 5);
            }
        }
        _ => {}
    }
}

/// `CTaskSimpleLand::ProcessPed(CPed*)` (0x67D380). Finished (bit 0 of +0x10): clears ped flag 0x400
/// (+0x46C); unless bit 1, rewinds anims 0, 1, 2 of the clump (`SetCurrentTime(0)`, each stored in +8 on
/// the way) and clears +8; true. Otherwise: blends in anim +0xC (group 0, delta 100.0, finish callback
/// `FinishAnimCB` 0x678FA0) when there is none and bit 1 is clear; while bit 2 is set: unless the matrix
/// up.z is exactly 1.0 (raw dword compare) and the move speed (+0x50..+0x58) equals [0x858B50], zeroes the
/// move speed and re-aligns the heading (+0x558) through `SetRotateZOnly` (or +0x10 without a matrix); sets
/// ped flag 0x400; for players: sprint (ped state +0x534 == 7 and `GetSprint`) sets the anim speed (+0x24)
/// to 2.0, and unless the anim id is 10 the speed is scaled by `GetFatAndMuscleModifier(2)`; then bit 2 is
/// cleared and bit 0 set when bit 1 is. False.
#[hook(addr = 0x67D380, conv = thiscall, name = "CTaskSimpleLand::ProcessPed", expect = "83 EC 0C 56 8B F1")]
pub fn simple_land_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let f = rd::<u8>(this, 0x10);
    if f & 1 != 0 {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FBFF);
        if rd::<u8>(this, 0x10) & 2 == 0 {
            for id in 0..3u32 {
                let a = clump_get_association(rd::<u32>(ped, 0x18), id);
                wr::<GPtr<u8>>(this, 8, a);
                if !a.is_null() {
                    anim_set_current_time(a, 0);
                }
            }
            wr::<u32>(this, 8, 0);
        }
        return 1;
    }
    if rd::<GPtr<u8>>(this, 8).is_null() && f & 2 == 0 {
        let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, rd::<u32>(this, 0xC), 0x42C8_0000);
        wr::<GPtr<u8>>(this, 8, a);
        anim_set_finish_callback(a, 0x678FA0, this.addr() as u32);
    }
    if rd::<u8>(this, 0x10) & 4 == 0 {
        return 0;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let k = x(g::<f32>(0x858B50).read());
    let eq = |off: isize| x(rd::<f32>(ped, off)).fcom(k) == sa_math::x87::Cmp::Equal;
    let still = rd::<u32>(m, 0x28) == 0x3F80_0000 && eq(0x50) && eq(0x54) && eq(0x58);
    if !still {
        wr::<u32>(ped, 0x50, 0);
        wr::<u32>(ped, 0x54, 0);
        wr::<u32>(ped, 0x58, 0);
        let h = rd::<u32>(ped, 0x558);
        if m.is_null() {
            wr::<u32>(ped, 0x10, h);
        } else {
            matrix_set_rotate_z_only(m, h);
        }
    }
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x400);
    if ped_is_player(ped) != 0 {
        let pad = player_get_pad_from_player(ped);
        if rd::<i32>(ped, 0x534) == 7 && pad_get_sprint(pad) != 0 {
            wr::<u32>(rd::<GPtr<u8>>(this, 8), 0x24, 0x4000_0000);
        }
        if rd::<i32>(this, 0xC) != 0xA {
            let s = rd::<f32>(rd::<GPtr<u8>>(this, 8), 0x24);
            let v = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(2)).fmul_m(s);
            wr::<f32>(rd::<GPtr<u8>>(this, 8), 0x24, f32_of(v));
        }
    }
    let b = rd::<u8>(this, 0x10) & 0xFB;
    wr::<u8>(this, 0x10, b);
    if b & 2 != 0 {
        wr::<u8>(this, 0x10, b | 1);
    }
    0
}

/// `CTaskSimpleRunNamedAnim::ProcessPed(CPed*)` (0x61BF20). Offsets the ped first when bit 6 of +0xC is
/// set. Finished (bit 0): unless bit 5, when blendDelta (+0x38) > [0x858628], the hierarchy's association
/// exists and the ped (matrix position, else +4) is farther than [0x858624] from its bone 3 position
/// (`sqrt((dz² + dy²) + dx²)`), blends that association out (flag 4 at +0x2E, delta = +0x38 * [0x858C1C]);
/// true. Running: true without a hierarchy (+0x3C) or its +4; flag bit 1 of +0xC with anim flags (+0x5C)
/// lacking 8 or having 2 purges the ped's event group; when the timer (+0x44, started +0x4C) ran out: with
/// anim flag 8 asks itself to abort (`MakeAbortable(ped, 0, null)`), else resets the anim's finish
/// callback, finishes (bit 0, +8 = null) and returns true. Without an anim (+8): true while the hierarchy's
/// association exists with +0x38 set, else `StartAnim`. Returns bit 0 of +0xC.
#[hook(addr = 0x61BF20, conv = thiscall, name = "CTaskSimpleRunNamedAnim::ProcessPed", expect = "83 EC 0C 56 8B F1")]
pub fn run_named_anim_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xC) & 0x40 != 0 {
        run_named_anim_offset_ped_position(this, ped);
    }
    let cl = rd::<u8>(this, 0xC);
    if cl & 1 != 0 {
        if cl & 0x20 != 0 {
            return 1;
        }
        let a = clump_get_association_hier(rd::<u32>(ped, 0x18), 1, rd::<u32>(this, 0x3C));
        let mut v = [0f32; 3];
        let vp: GPtr<u8> = GPtr::from_raw(&mut v as *mut [f32; 3]).cast();
        ped_get_bone_position(ped, vp, 3, 0);
        if x(rd::<f32>(this, 0x38)).fcom(x(g::<f32>(0x858628).read())).test_any(0x41) || a.is_null() {
            return 1;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(vp, 0));
        let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(vp, 4));
        let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(vp, 8));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        if d.fcom(x(g::<f32>(0x858624).read())).test_any(0x41) {
            return 1;
        }
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        let s = x(rd::<f32>(this, 0x38)).fmul_m(g::<f32>(0x858C1C).read());
        wr::<f32>(a, 0x1C, f32_of(s));
        return 1;
    }
    let h = rd::<GPtr<u8>>(this, 0x3C);
    if h.is_null() || rd::<u32>(h, 4) == 0 {
        return 1;
    }
    if cl & 2 != 0 {
        let fl = rd::<u32>(this, 0x5C);
        if fl & 8 == 0 || fl & 2 != 0 {
            event_group_remove_invalid_events(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), 1);
            event_group_reorganise(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68));
        }
    }
    if rd::<u8>(this, 0x4C) != 0 && task_timer_is_out_of_time(this.byte_add(0x44)) != 0 {
        if rd::<u8>(this, 0x5C) & 8 == 0 {
            let a = rd::<GPtr<u8>>(this, 8);
            if !a.is_null() {
                anim_set_finish_callback(a, 0x4D1390, 0);
            }
            wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) | 1);
            wr::<u32>(this, 8, 0);
            return 1;
        }
        v_make_abortable(this, ped, 0, GPtr::NULL);
    }
    if rd::<u32>(this, 8) == 0 {
        let a = clump_get_association_hier(rd::<u32>(ped, 0x18), 1, rd::<u32>(this, 0x3C));
        if !a.is_null() && rd::<u32>(a, 0x38) != 0 {
            return 1;
        }
        run_named_anim_start_anim_orig(this, ped);
    }
    rd::<u8>(this, 0xC) & 1
}

/// `CTaskSimpleCarDrive::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x63C670). Priority 2
/// (immediate): when the ped is in its vehicle (bit 8 of +0x46C, +0x58C), a stack `CTaskSimpleCarSetPedOut(
/// this->vehicle, ComputeTargetDoorToExit(veh, ped), false)` (0x18 bytes) is run once (`+0x12 = 1` for bikes,
/// type +0x590 == 9, with an event) and destroyed; aborts the look-at IK (blend 250) if looking; true.
/// Other priorities: anim flag 8 in +0x5C, returns bit 5 of +0x5C clear.
#[hook(addr = 0x63C670, conv = thiscall, name = "CTaskSimpleCarDrive::MakeAbortable", expect = "64 A1 00 00 00 00")]
pub fn car_drive_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority != 2 {
        wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) | 8);
        return u8::from(rd::<u8>(this, 0x5C) & 0x20 == 0);
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() {
            let door = car_enter_exit_compute_target_door_to_exit(v, ped);
            let mut so = [0u32; 6];
            let sp: GPtr<u8> = GPtr::from_raw(&mut so as *mut [u32; 6]).cast();
            car_set_ped_out_ctor(sp, rd::<GPtr<u8>>(this, 8), door, 0);
            if rd::<i32>(rd::<GPtr<u8>>(ped, 0x58C), 0x590) == 9 && !event.is_null() {
                wr::<u8>(sp, 0x12, 1);
            }
            car_set_ped_out_process_ped(sp, ped);
            car_set_ped_out_dtor(sp);
        }
    }
    let ik = GPtr::<u8>::from_addr(0xC15448);
    if ik_is_looking(ik, ped) != 0 {
        ik_abort_look_at(ik, ped, 0xFA);
    }
    1
}

/// `CTaskSimpleCarGetOut::ComputeAnimID_ToHook(int32* group, int32* anim)` (0x647330). For automobiles
/// (+0x590 == 0) +0x10 = `IsCarUpsideDown(veh)` (on a stack `CUpsideDownCarCheck`). An upside-down exit
/// is cancelled when the vehicle matrix right.z (+8) is below [0x858F40] for doors 10/11, or above
/// [0x858B8C] for other doors. Upside down: doors 8/9 -> anim 0x6D, 10/11 -> 0x6C (other doors leave `*anim`),
/// `*group = 0`. Otherwise doors 8/9/10/11 -> 0x176/0x178/0x175/0x177 (others leave `*anim`) and
/// `*group = vehicle anim group.GetGroup(*anim)`.
#[hook(addr = 0x647330, conv = thiscall, name = "CTaskSimpleCarGetOut::ComputeAnimID_ToHook", expect = "83 EC 30 56 8B F1")]
pub fn car_get_out_compute_anim_id(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0x14);
    if rd::<u32>(veh, 0x590) == 0 {
        let mut chk = [0u32; 0xC];
        let cp: GPtr<u8> = GPtr::from_raw(&mut chk as *mut [u32; 0xC]).cast();
        let r = upside_down_car_check_is_car_upside_down(cp, veh);
        wr::<u8>(this, 0x10, u8::from(r != 0));
    }
    if rd::<u8>(this, 0x10) != 0 {
        let door = rd::<i32>(this, 0x18);
        let rz = x(rd::<f32>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x14), 0x14), 8));
        let cancel = if door == 0xA || door == 0xB {
            rz.fcom(x(g::<f32>(0x858F40).read())) == sa_math::x87::Cmp::Less
        } else {
            !rz.fcom(x(g::<f32>(0x858B8C).read())).test_any(0x41)
        };
        if cancel {
            wr::<u8>(this, 0x10, 0);
        }
    }
    if rd::<u8>(this, 0x10) != 0 {
        match rd::<i32>(this, 0x18).wrapping_sub(8) {
            0 | 1 => wr::<u32>(anim, 0, 0x6D),
            2 | 3 => wr::<u32>(anim, 0, 0x6C),
            _ => {}
        }
        wr::<u32>(group, 0, 0);
        return;
    }
    match rd::<i32>(this, 0x18).wrapping_sub(8) {
        0 => wr::<u32>(anim, 0, 0x176),
        1 => wr::<u32>(anim, 0, 0x178),
        2 => wr::<u32>(anim, 0, 0x175),
        3 => wr::<u32>(anim, 0, 0x177),
        _ => {}
    }
    let a = rd::<u32>(anim, 0);
    let v = rd::<GPtr<u8>>(this, 0x14);
    let grp = u32::from(rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0xDE));
    let r = vehicle_anim_group_get_group(GPtr::from_addr((grp * 0x94).wrapping_add(0xC1CDC0) as usize), a);
    wr::<u32>(group, 0, r);
}

/// `CTaskUtilityLineUpPedWithCar::GetPositionToOpenCarDoor(CVector* out, CVehicle*, float, assoc)`
/// (0x650A80): a local copy of the vehicle matrix (no null check), the local door position, rotated by
/// `Multiply3x3`, plus the vehicle position (matrix +0x30, else +4), each component rounded to f32; the local
/// matrix is destroyed; returns `out`. Locals live in a Rust buffer.
#[hook(addr = 0x650A80, conv = thiscall, name = "CTaskUtilityLineUpPedWithCar::GetPositionToOpenCarDoor", expect = "6A FF 68 68 32 84 00")]
pub fn line_up_get_position_to_open_car_door(this: GPtr<u8>, out: GPtr<u8>, veh: GPtr<u8>, t: u32, a: u32) -> GPtr<u8> {
    let mut mbuf = [0u32; 0x12];
    let mp: GPtr<u8> = GPtr::from_raw(&mut mbuf as *mut [u32; 0x12]).cast();
    let mut tmp = [0u32; 3];
    let tp: GPtr<u8> = GPtr::from_raw(&mut tmp as *mut [u32; 3]).cast();
    let mut l = [0u32; 3];
    let lp: GPtr<u8> = GPtr::from_raw(&mut l as *mut [u32; 3]).cast();
    let mut w = [0u32; 3];
    let wp: GPtr<u8> = GPtr::from_raw(&mut w as *mut [u32; 3]).cast();
    matrix_copy_ctor(mp, rd::<GPtr<u8>>(veh, 0x14));
    let r = line_up_get_local_position_to_open_car_door(this, tp, veh, t, a);
    for i in 0..3 {
        wr::<u32>(lp, i * 4, rd::<u32>(r, i * 4));
    }
    let r = multiply_3x3(wp, mp, lp);
    for i in 0..3 {
        wr::<u32>(lp, i * 4, rd::<u32>(r, i * 4));
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
    let p = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    for (i, pv) in (0isize..).zip(p) {
        let s = x(pv).fadd_m(rd::<f32>(lp, i * 4));
        wr::<f32>(out, i * 4, f32_of(s));
    }
    matrix_dtor(mp);
    out
}

/// `CTaskComplexLeaveCar::CreateFirstSubTask(CPed*)` (0x641FC0). A player being arrested (+0x530 == 0x3F) or
/// with ped flag 0x40 (+0x474), or a ped not in a vehicle (bit 8 of +0x46C; the original prints the debug
/// string 0x86EB9C first), gets `CTaskSimplePause(-1)` (0x18 bytes). Otherwise, when the ped drives the
/// vehicle (+0xC, driver +0x460): a group leader tells its group (`CEventLeaderExitedCarAsDriver`, 0x14-byte
/// event, wrapped in a stack `CEventGroupEvent` passed to `AddEvent`); non-players `SetRadioStation`, players
/// notify the vehicle audio (+0x138 of +0x58C). Then boats (+0x590 == 5) get `CTaskComplexLeaveBoat(veh, 0)`
/// (0x14 bytes), others `CTaskSimpleCarDriveTimed(veh, +0x14)` (0x70 bytes). Null when the pool is full.
#[hook(addr = 0x641FC0, conv = thiscall, name = "CTaskComplexLeaveCar::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn leave_car_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let pause = |t: GPtr<u8>| if t.is_null() { GPtr::NULL } else { simple_pause_ctor(t, -1) };
    if (rd::<i32>(ped, 0x530) == 0x3F || rd::<u8>(ped, 0x474) & 0x40 != 0) && ped_is_player(ped) != 0 {
        return pause(task_new(0x18));
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        crt_debug_printf(0x86EB9C);
        return pause(task_new(0x18));
    }
    if rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x460) == ped {
        let grp = ped_groups_get_peds_group(ped);
        if !grp.is_null() && group_membership_is_leader(grp.byte_add(8), ped) != 0 {
            let e = event_new(0x14);
            let ev = if e.is_null() { GPtr::NULL } else { event_leader_exited_car_as_driver_ctor(e) };
            let mut ge = [0u32; 5];
            let gp: GPtr<u8> = GPtr::from_raw(&mut ge as *mut [u32; 5]).cast();
            event_group_event_ctor(gp, ped, ev);
            group_intelligence_add_event(grp.byte_add(0x30), gp);
            event_group_event_dtor(gp);
        }
        if ped_is_player(ped) == 0 {
            ped_set_radio_station(ped);
        } else {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            if !v.is_null() {
                vehicle_audio_player_about_to_exit_as_driver(v.byte_add(0x138));
            }
        }
    }
    if rd::<i32>(rd::<GPtr<u8>>(this, 0xC), 0x590) == 5 {
        let t = task_new(0x14);
        if t.is_null() {
            return GPtr::NULL;
        }
        return leave_boat_ctor(t, rd::<u32>(this, 0xC), 0);
    }
    let t = task_new(0x70);
    if t.is_null() {
        return GPtr::NULL;
    }
    car_drive_timed_ctor_orig(t, rd::<u32>(this, 0xC), rd::<u32>(this, 0x14))
}

/// `CTaskComplexObserveTrafficLights::CreateNextSubTask(CPed*)` (0x6317E0): without a sub-task or after
/// 0xCB: `CTaskSimpleRunAnim(ped anim group +0x4D4, 4, 4.0, false)` (0x20 bytes); after 0x190: a stand-still
/// for `GetRandomNumberInRange(3000, 5000)` ms (drawn before the allocation; `(time, false, false, 8.0)`);
/// otherwise null. Null when the pool is full.
#[hook(addr = 0x6317E0, conv = thiscall, name = "CTaskComplexObserveTrafficLights::CreateNextSubTask", expect = "6A FF 68 46 05 84 00")]
pub fn observe_traffic_lights_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if sub.is_null() || v_get_task_type(sub) == 0xCB {
        let t = task_new(0x20);
        if t.is_null() {
            return GPtr::NULL;
        }
        return run_anim_ctor(t, rd::<u32>(ped, 0x4D4), 4, 0x4080_0000, 0);
    }
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) != 0x190 {
        return GPtr::NULL;
    }
    let n = general_random_int_in_range(0xBB8, 0x1388);
    let t = task_new(0x20);
    if t.is_null() {
        return GPtr::NULL;
    }
    stand_still_ctor(t, n, 0, 0, 0x4100_0000)
}

/// `CTaskGangHasslePed::CreateNextSubTask(CPed*)` (0x6642C0): always draws `rand()` (returns to 0x6642E0)
/// for `f = (r * [0x858C7C]) * 2 + [0x858B3C]` (doubled with `fadd st0, st0`, stored as f32); null after sub-task
/// 0x3E8; else a 0x4C-byte `CTaskComplexSeekEntityStandard(+0xC, 999999, 1000, [0x86FC2C] * f, [0x86FC28],
/// [0x86FC30], false, true)`; null when the pool is full.
#[hook(addr = 0x6642C0, conv = thiscall, name = "CTaskGangHasslePed::CreateNextSubTask", expect = "6A FF 68 AB 4B 84 00")]
pub fn gang_hassle_ped_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let r = sa_core::orig::at_site(0x6642E0, rand);
    let a = xi(r).fmul_m(g::<f32>(0x858C7C).read());
    let f = f32_of(a.fadd(a).fadd_m(g::<f32>(0x858B3C).read()));
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x3E8 {
        return GPtr::NULL;
    }
    let t = task_new(0x4C);
    if t.is_null() {
        return GPtr::NULL;
    }
    let c30 = g::<u32>(0x86FC30).read();
    let radius = f32_of(x(g::<f32>(0x86FC2C).read()).fmul_m(f)).to_bits();
    let c28 = g::<u32>(0x86FC28).read();
    seek_entity_standard_ctor(t, rd::<u32>(this, 0xC), 0xF423F, 0x3E8, radius, c28, c30, 0, 1)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_WANDER_PROSTITUTE_CREATE_FIRST_SUB_TASK,
    HOOK_PARTNER_DEAL_CREATE_FIRST_SUB_TASK,
    HOOK_OBSERVE_TRAFFIC_LIGHTS_CREATE_FIRST_SUB_TASK,
    HOOK_EVASIVE_STEP_CREATE_FIRST_SUB_TASK,
    HOOK_BE_IN_COUPLE_CREATE_FIRST_SUB_TASK,
    HOOK_TASK_COMPLEX_MAKE_ABORTABLE,
    HOOK_TASK_MANAGER_GET_TASK_SECONDARY,
    HOOK_TASK_SIMPLE_ANIM_FINISH_RUN_ANIM_CB,
    HOOK_CAR_DRIVE_WANDER_CREATE_SUB_TASK_CANNOT_GET_IN_CAR,
    HOOK_TASK_CATEGORIES_IS_FOLLOW_PED_TASK,
    HOOK_SEQUENCE_CREATE_NEXT_SUB_TASK,
    HOOK_OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING_CTOR,
    HOOK_EVASIVE_STEP_FINISH_ANIM_CB,
    HOOK_CAR_DRIVE_WANDER_CTOR,
    HOOK_WANDER_PROSTITUTE_CTOR,
    HOOK_TASK_CATEGORIES_IS_KILL_PED_TASK,
    HOOK_SIMPLE_CHAT_CTOR,
    HOOK_ACHIEVE_HEADING_CTOR,
    HOOK_SIMPLE_FACIAL_CTOR,
    HOOK_SIMPLE_SAY_CTOR,
    HOOK_DEFAULT_TASK_ALLOCATORS_GET,
    HOOK_TASK_COMPLEX_SET_SUB_TASK,
    HOOK_CAR_DRIVE_TIMED_CTOR,
    HOOK_OBSERVE_TRAFFIC_LIGHTS_CONTROL_SUB_TASK,
    HOOK_GO_TO_ATTRACTOR_CREATE_NEXT_SUB_TASK,
    HOOK_WANDER_CRIMINAL_CTOR,
    HOOK_CAR_CLOSE_DOOR_FROM_OUTSIDE_COMPUTE_ANIM_ID,
    HOOK_WANDER_COP_DTOR,
    HOOK_PLAY_HAND_SIGNAL_ANIM_CTOR,
    HOOK_CAR_SET_PED_IN_AS_PASSENGER_DTOR,
    HOOK_EVASIVE_STEP_COMPLEX_DTOR,
    HOOK_BE_IN_COUPLE_DTOR,
    HOOK_SIMPLE_LAND_DTOR,
    HOOK_PLAY_HAND_SIGNAL_GET_ANIM_ID_FOR_PED,
    HOOK_BE_IN_COUPLE_ABORT_ARM_IK,
    HOOK_PARTNER_CHAT_DTOR,
    HOOK_EVASIVE_STEP_CREATE_NEXT_SUB_TASK,
    HOOK_PARTNER_CHAT_MAKE_ABORTABLE,
    HOOK_CAR_DRIVE_TIMED_PROCESS_PED,
    HOOK_EVASIVE_STEP_SIMPLE_CTOR,
    HOOK_SCRATCH_HEAD_CLONE,
    HOOK_EVASIVE_STEP_SIMPLE_DTOR,
    HOOK_IN_AIR_DTOR,
    HOOK_GO_TO_VEHICLE_AND_LEAN_CLONE,
    HOOK_WANDER_GANG_CTOR,
    HOOK_EVASIVE_STEP_PROCESS_PED,
    HOOK_GANG_FOLLOWER_DTOR,
    HOOK_TASK_CATEGORIES_COMPUTE_TASK_CATEGORY,
    HOOK_CAR_DRIVE_WANDER_CLONE,
    HOOK_GANG_FOLLOWER_MAKE_ABORTABLE,
    HOOK_GANG_HASSLE_PED_CLONE,
    HOOK_SIT_IDLE_START_ANIM,
    HOOK_PARTNER_STREAM_REQUIRED_ANIMS,
    HOOK_GO_TO_POINT_AND_STAND_STILL_SELECT_MOVE_STATE,
    HOOK_PLAY_HAND_SIGNAL_ANIM_CLONE,
    HOOK_SMART_FLEE_ENTITY_CTOR,
    HOOK_PARTNER_DEAL_STREAM_REQUIRED_ANIMS,
    HOOK_SIMPLE_ANIM_DTOR,
    HOOK_LEAVE_CAR_DTOR,
    HOOK_PARTNER_DTOR_PORT,
    HOOK_WANDER_STANDARD_WILL_CHAT,
    HOOK_BE_IN_COUPLE_MAKE_ABORTABLE,
    HOOK_PLAY_HAND_SIGNAL_CREATE_FIRST_SUB_TASK,
    HOOK_PARTNER_DEAL_CLONE,
    HOOK_GANG_FOLLOWER_CLONE,
    HOOK_WANDER_STANDARD_CLONE,
    HOOK_INIT_SIMPLE_GO_TO_LOOK_AT_THRESHOLD,
    HOOK_INIT_ALLOCATOR_FOLLOW_ANY_MEANS,
    HOOK_INIT_ALLOCATOR_FOLLOW_LIMITED,
    HOOK_INIT_ALLOCATOR_STAND_STILL,
    HOOK_INIT_ALLOCATOR_CHAT,
    HOOK_INIT_ALLOCATOR_RANDOM,
    HOOK_INIT_ALLOCATOR_SIT_IN_LEADER_CAR,
    HOOK_INIT_FOLLOW_LEADER_IN_FORMATION_OFFSETS,
    HOOK_INIT_PED_SCRIPTED_TASK_RECORD,
    HOOK_INIT_TASK_SEQUENCES,
    HOOK_INIT_SCRIPTED_BRAIN_TASK_STORE,
    HOOK_TASK_IS_GO_TO_TASK,
    HOOK_EVASIVE_STEP_COMPLEX_CTOR,
    HOOK_BE_IN_COUPLE_CTOR,
    HOOK_WANDER_STANDARD_SET_NEXT_MIN_SCAN_TIME,
    HOOK_TASK_MANAGER_FIND_ACTIVE_TASK_BY_TYPE,
    HOOK_STAND_STILL_CLONE,
    HOOK_RUN_NAMED_ANIM_CLONE,
    HOOK_RUN_NAMED_ANIM_START_ANIM,
    HOOK_LEAVE_CAR_CONTROL_SUB_TASK,
    HOOK_CAR_DRIVE_SET_PED_POSITION,
    HOOK_FACIAL_PROCESS_PED,
    HOOK_CAR_DRIVE_TRIGGER_IK,
    HOOK_SIMPLE_LAND_PROCESS_PED,
    HOOK_RUN_NAMED_ANIM_PROCESS_PED,
    HOOK_CAR_DRIVE_MAKE_ABORTABLE,
    HOOK_CAR_GET_OUT_COMPUTE_ANIM_ID,
    HOOK_LINE_UP_GET_POSITION_TO_OPEN_CAR_DOOR,
    HOOK_LEAVE_CAR_CREATE_FIRST_SUB_TASK,
    HOOK_OBSERVE_TRAFFIC_LIGHTS_CREATE_NEXT_SUB_TASK,
    HOOK_GANG_HASSLE_PED_CREATE_NEXT_SUB_TASK,
];
