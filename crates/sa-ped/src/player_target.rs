//! CPlayerPed targeting, lock-on, group and move-anim functions; CPlayerInfo search helpers (PORT-PED-3,
//! helper B). Ported from the 1.0 US bytes (instruction addresses cited); `docs/re/specs/PED/CPlayerPed.md`,
//! `CPlayerInfo.md` and gta-reversed as reading aids. Original bugs are kept.
//!
//! Fields used: `CPlayerPed` +0x480 player data (`CPlayerPedData`: +0x38 group id, +0x85 m_bDontAllowWeaponChange),
//! +0x5A0 weapons (0x1C each), +0x718 active slot (int8), +0x71C targeted object, +0x79C mouse target.
//! `CPedGroups::ms_groups` 0xC09920 (0x2D4 each: +0x4 m_bMembersEnterLeadersVehicle, +0x8 membership,
//! +0x30 intelligence).
//!
//! `GetATanOfXY` / `TargetWeaponRangeMultiplier` / `FindTargetPriority` / `DistToLine` leave their result in
//! ST0; through `orig!` it arrives as an f64 (the GetATanOfXY raw-FPATAN path keeps 53 of 64 bits, the
//! pending ledger entry "GetATanOfXY raw FPATAN path").

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;
use sa_world::world::common::cam_pos;

/// `CPedGroups::ms_groups`
const GROUPS: usize = 0xC09920;
const GROUP_SIZE: usize = 0x2D4;
/// `CCamera::m_bUseMouse3rdPerson`
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
/// `PLAYER_MAX_TARGET_VIEW_ANGLE` (.data, 140.0f)
const PLAYER_MAX_TARGET_VIEW_ANGLE: usize = 0x8D243C;

/// `.rdata` constants (read as memory operands by the original)
const PI: f32 = f32::from_bits(0x4049_0FDB);
const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
const MINUS_PI: f32 = f32::from_bits(0xC049_0FDB);
/// 0x86D1D0 (50 degrees)
const NEIGHBOUR_MAX_ANGLE: f32 = f32::from_bits(0x3F5F_66F3);
/// 0x859878 (180/pi)
const RAD_TO_DEG: f32 = f32::from_bits(0x4265_2EE0);

orig! {
    /// `CPedGroupMembership::CountMembersExcludingLeader` (0x5F6AA0)
    pub fn membership_count_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CPedGroupMembership::IsMember(CPed*)` (0x5F6A10)
    pub fn membership_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPedGroupMembership::RemoveAllFollowers(bool)` (0x5FB190)
    pub fn membership_remove_all_followers(m: GPtr<u8>, b: u32) = thiscall 0x5FB190 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    pub fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CEvent::Clone` (vtable slot 0x10)
    pub fn v_event_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0, `ret 8`)
    pub fn event_group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    pub fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`)
    pub fn group_intel_add_event(gi: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5F7470 => unported;
    /// `CPedGroupIntelligence::ReportAllBarScriptTasksFinished` (0x5F8780)
    pub fn group_intel_report_all_bar_script_tasks_finished(gi: GPtr<u8>) = thiscall 0x5F8780 => unported;
    /// `CPedGroup::PlayerGaveCommand_Attack(CPed* player, CPed* target)` (0x5F7CC0, `ret 8`)
    pub fn group_player_gave_command_attack(g: GPtr<u8>, player: GPtr<u8>, target: GPtr<u8>) = thiscall 0x5F7CC0 => unported;
    /// `CTagManager::IsTag(const CEntity*)` (0x49CCE0)
    pub fn tag_is_tag(e: GPtr<u8>) -> u8 = cdecl 0x49CCE0 => unported;
    /// `CTagManager::GetAlpha(CEntity*)` (0x49CF90)
    pub fn tag_get_alpha(e: GPtr<u8>) -> u8 = cdecl 0x49CF90 => unported;
    /// `CObject::CanBeTargetted` (0x59F320)
    pub fn object_can_be_targetted(o: GPtr<u8>) -> u8 = thiscall 0x59F320 => unported;
    /// `CPlayerPed::FindPedToAttack` (0x60C5F0)
    pub fn find_ped_to_attack_call(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x60C5F0 => find_ped_to_attack;
    /// `CPed::GetBonePosition(RwV3d& out, boneTag, bool updateSkin)` (0x5E4280, `ret 0xC`)
    pub fn ped_get_bone_position(ped: GPtr<u8>, out: GPtr<f32>, bone: i32, update: u32) = thiscall 0x5E4280 => unported;
    /// `CWorld::ProcessLineOfSight(origin, target, CColPoint&, CEntity*&, buildings, vehicles, peds, objects,
    /// dummies, seeThrough, cameraIgnore, shootThrough)` (0x56BA00)
    pub fn world_process_line_of_sight(a: GPtr<f32>, b: GPtr<f32>, cp: GPtr<u8>, e: GPtr<GPtr<u8>>, bd: u32, vh: u32, pd: u32, ob: u32, du: u32, st: u32, cam: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWeapon::TargetWeaponRangeMultiplier(CEntity* victim, CEntity* owner)` (0x73B380, ST0)
    pub fn target_weapon_range_multiplier(victim: GPtr<u8>, owner: GPtr<u8>) -> f64 = cdecl 0x73B380 => unported;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60, AL)
    pub fn ped_get_weapon_skill_type(p: GPtr<u8>, ty: i32) -> u8 = thiscall 0x5E3B60 => crate::ped::get_weapon_skill_type;
    /// `CPlayerPed::CanIKReachThisTarget(CVector, CWeapon*, bool)` (0x609F80, `ret 0x14`)
    pub fn can_ik_reach_this_target(p: GPtr<u8>, tx: f32, ty: f32, tz: f32, w: GPtr<u8>, b: u32) -> u8 = thiscall 0x609F80 => crate::player_ped::can_ik_reach_this_target;
    /// `CPlayerPed::DoesTargetHaveToBeBroken(CEntity*, CWeapon*)` (0x60C0C0, `ret 8`)
    pub fn does_target_have_to_be_broken_call(p: GPtr<u8>, t: GPtr<u8>, w: GPtr<u8>) -> u8 = thiscall 0x60C0C0 => does_target_have_to_be_broken;
    /// `CGeneral::GetATanOfXY(float x, float y)` (0x53CC70, ST0)
    pub fn get_atan_of_xy(xv: f32, yv: f32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CPlayerPed::FindTargetPriority(CEntity*)` (0x609DE0, `ret 4`, ST0)
    pub fn find_target_priority(p: GPtr<u8>, t: GPtr<u8>) -> f64 = thiscall 0x609DE0 => unported;
    /// `CPlayerPed::LOSBlockedBetweenPeds(CEntity*, CEntity*)` (0x60B550, cdecl)
    pub fn los_blocked_between_peds_call(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x60B550 => los_blocked_between_peds;
    /// `CWorld::FindPlayerCoors(CVector* out, int32 id)` (0x56E010)
    pub fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    pub fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CCollision::DistToLine(const CVector&, const CVector&, const CVector&)` (0x417610, ST0)
    pub fn dist_to_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 = cdecl 0x417610 => unported;
    /// `sub_609380` (unnamed: any of the recruit cheats 76/77/78 active; cdecl, AL)
    pub fn recruit_cheats_active() -> u8 = cdecl 0x609380 => unported;
    /// `CPad::GetGroupControlForward` (0x541190)
    pub fn pad_get_group_control_forward(pad: GPtr<u8>) -> u8 = thiscall 0x541190 => unported;
    /// `CPad::GetGroupControlBack` (0x5411B0)
    pub fn pad_get_group_control_back(pad: GPtr<u8>) -> u8 = thiscall 0x5411B0 => unported;
    /// `CPlayerPed::DisbandPlayerGroup` (0x60A0A0)
    pub fn disband_player_group_call(p: GPtr<u8>) = thiscall 0x60A0A0 => disband_player_group;
    /// `CPlayerPed::MakeThisPedJoinOurGroup(CPed*)` (0x60C840, `ret 4`)
    pub fn make_this_ped_join_our_group_call(p: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x60C840 => make_this_ped_join_our_group;
    /// `CPlayerPed::TellGroupToStartFollowingPlayer(bool, bool, bool)` (0x60A1D0, `ret 0xC`)
    pub fn tell_group_to_start_following_player_call(p: GPtr<u8>, a: u32, b: u32, c: u32) = thiscall 0x60A1D0 => tell_group_to_start_following_player;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    pub fn ped_type_get_ped_flag(ty: i32) -> u32 = cdecl 0x608830 => unported;
    /// `CAcquaintance::SetAsAcquaintance(int32, uint32)` (0x608DA0, `ret 8`)
    pub fn acq_set_as_acquaintance(a: GPtr<u8>, ty: i32, flags: u32) = thiscall 0x608DA0 => unported;
    /// `CAcquaintance::ClearAsAcquaintance(int32, uint32)` (0x608980, `ret 8`)
    pub fn acq_clear_as_acquaintance(a: GPtr<u8>, ty: i32, flags: u32) = thiscall 0x608980 => unported;
    /// `CPedGroupIntelligence::SetDefaultTaskAllocatorType(int32)` (0x5FBB70)
    pub fn group_intel_set_default_task_allocator_type(gi: GPtr<u8>, ty: i32) = thiscall 0x5FBB70 => unported;
    /// `CEventPlayerCommandToGroup::CEventPlayerCommandToGroup(ePlayerGroupCommand, CPed*)` (0x4B23D0, `ret 8`)
    pub fn event_player_command_to_group_ctor(e: GPtr<u8>, cmd: i32, target: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B23D0 => unported;
    /// `CEventEditableResponse::ComputeResponseTaskType(CPedGroup*)` (0x4B57A0)
    pub fn event_compute_response_task_type_group(e: GPtr<u8>, group: GPtr<u8>) = thiscall 0x4B57A0 => unported;
    /// `CEventEditableResponse::WillRespond` (0x4AC490)
    pub fn event_will_respond(e: GPtr<u8>) -> u8 = thiscall 0x4AC490 => unported;
    /// `CEvent::operator new(uint32)` (0x4B5620)
    pub fn event_operator_new(size: u32) -> GPtr<u8> = cdecl 0x4B5620 => unported;
    /// `CEventPlayerCommandToGroup` destructor body (0x4B2470, unnamed in the catalogue)
    pub fn event_player_command_to_group_dtor(e: GPtr<u8>) = thiscall 0x4B2470 => unported;
    /// `CPedGroup::FindDistanceToFurthestMember` (0x5FB010, ST0)
    pub fn group_find_distance_to_furthest_member(g: GPtr<u8>) -> f64 = thiscall 0x5FB010 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0)
    pub fn intel_find_task_by_type(intel: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CEventGroup::GetEventOfType(eEventType)` (0x4AB650)
    pub fn event_group_get_event_of_type(eg: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// `CTask::GetTaskType` (vtable slot 0x10)
    pub fn v_task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    pub fn task_operator_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexSmartFleeEntity::CTaskComplexSmartFleeEntity(CEntity*, bool, float, int32, int32, float)` (0x65C430, `ret 0x18`)
    pub fn task_smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, dist: f32, time: i32, a: u32, c: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    pub fn event_script_command_ctor(e: GPtr<u8>, index: i32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand` destructor (0x4B0A50, unnamed in the catalogue)
    pub fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    pub fn event_group_add(eg: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEntity::GetIsOnScreen` (0x534540)
    pub fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CEntity::FlagToDestroyWhenNextProcessed` (vtable slot 0x54)
    pub fn v_flag_to_destroy_when_next_processed(e: GPtr<u8>) = thiscall vslot 0x54 => unported;
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool, bool, bool, bool, bool)` (0x564A20)
    pub fn world_find_objects_in_range(p: GPtr<f32>, r: f32, b2d: u32, n: GPtr<i16>, max: u32, list: GPtr<GPtr<u8>>, bd: u32, vh: u32, pd: u32, ob: u32, du: u32) = cdecl 0x564A20 => unported;
    /// `CCarCtrl::PossiblyRemoveVehicle(CVehicle*)` (0x424F80)
    pub fn car_ctrl_possibly_remove_vehicle(v: GPtr<u8>) = cdecl 0x424F80 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    pub fn task_mgr_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CAEPedSpeechAudioEntity::SetCJMood(int16, uint32, int16, int16, int16)` (0x4E3ED0)
    pub fn speech_set_cj_mood(a: i32, b: u32, c: i32, d: i32, e: i32) = cdecl 0x4E3ED0 => unported;
    /// `CStats::FindMaxNumberOfGroupMembers` (0x559A50)
    pub fn stats_find_max_number_of_group_members() -> i32 = cdecl 0x559A50 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    pub fn ped_groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::RemoveMember(CPed*)` (0x5FB210)
    pub fn membership_remove_member(m: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FB210 => unported;
    /// `CTaskComplexBeInGroup::CTaskComplexBeInGroup(int32 groupId, bool)` (0x632E50, `ret 8`)
    pub fn task_be_in_group_ctor(t: GPtr<u8>, id: i32, b: u32) -> GPtr<u8> = thiscall 0x632E50 => unported;
    /// `CPedGroupMembership::AddFollower(CPed*)` (0x5F8020)
    pub fn membership_add_follower(m: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5F8020 => unported;
    /// `CPedGroup::Process` (0x5FC7E0)
    pub fn group_process(g: GPtr<u8>) = thiscall 0x5FC7E0 => unported;
    /// `CPed::GiveWeaponWhenJoiningGang` (0x5E8BE0)
    pub fn ped_give_weapon_when_joining_gang(p: GPtr<u8>) = thiscall 0x5E8BE0 => unported;
    /// `CEventNewGangMember::CEventNewGangMember(CPed*)` (0x608F70, `ret 4`)
    pub fn event_new_gang_member_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x608F70 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32, float)` (0x55B980)
    pub fn display_script_stat_update_message(inc: u32, stat: u32, v: f32) = cdecl 0x55B980 => unported;
    /// `CPool<CPed>::GetRef(CPed*)` thunk (0x4442D0, body in .HOODLUM; unnamed in the catalogue)
    pub fn ped_pool_get_ref(pool: GPtr<u8>, ped: GPtr<u8>) -> i32 = thiscall 0x4442D0 => unported;
    /// `CRadar::SetEntityBlip(eBlipType, int32, uint32, eBlipDisplay, const char*)` (0x5839A0)
    pub fn radar_set_entity_blip(ty: i32, h: i32, colour: u32, display: i32, name: u32) -> i32 = cdecl 0x5839A0 => unported;
    /// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0)
    pub fn radar_change_blip_scale(b: i32, s: i32) = cdecl 0x583CC0 => unported;
    /// `CRadar::ChangeBlipColour(int32, uint32)` (0x583AB0)
    pub fn radar_change_blip_colour(b: i32, c: u32) = cdecl 0x583AB0 => unported;
    /// `CRadar::SetBlipFriendly(int32, bool)` (0x583EB0)
    pub fn radar_set_blip_friendly(b: i32, f: u32) = cdecl 0x583EB0 => unported;
    /// `CEventDontJoinPlayerGroup::CEventDontJoinPlayerGroup(CPed*)` (0x6090E0, `ret 4`)
    pub fn event_dont_join_player_group_ctor(e: GPtr<u8>, player: GPtr<u8>) -> GPtr<u8> = thiscall 0x6090E0 => unported;
    /// `CEventDontJoinPlayerGroup` destructor (0x609140, unnamed in the catalogue)
    pub fn event_dont_join_player_group_dtor(e: GPtr<u8>) = thiscall 0x609140 => unported;
    /// `CPlaceable::AllocateMatrix` (0x54F560)
    pub fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, unnamed in the catalogue)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CEntity::GetBoundCentre(CVector&)` (0x534250)
    pub fn entity_get_bound_centre(e: GPtr<u8>, out: GPtr<f32>) = thiscall 0x534250 => unported;
    /// `CEntity::GetHeightAboveRoad` (vtable slot 0xD4, ST0)
    pub fn v_get_height_above_road(e: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CCarEnterExit::GetNearestCarDoor(const CPed*, const CVehicle*, CVector&, int32&)` (0x6528F0)
    pub fn get_nearest_car_door(ped: GPtr<u8>, veh: GPtr<u8>, pos: GPtr<f32>, door: GPtr<i32>) -> u8 = cdecl 0x6528F0 => unported;
    /// `CVector::Magnitude` (0x4082C0, ST0)
    pub fn vector_magnitude(v: GPtr<f32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPedGeometryAnalyser::LiesInsideBoundingBox(const CPed&, const CVector&, CEntity&)` (0x5F3880)
    pub fn lies_inside_bounding_box(ped: GPtr<u8>, pos: GPtr<f32>, e: GPtr<u8>) -> u8 = cdecl 0x5F3880 => unported;
    /// `CCranes::IsThisCarBeingCarriedByAnyCrane(CVehicle*)` (0x6F3FF0)
    pub fn cranes_is_this_car_being_carried(v: GPtr<u8>) -> u8 = cdecl 0x6F3FF0 => unported;
    /// `CPlayerInfo::EvaluateCarPosition(CEntity*, CPed*, float, float*, CVehicle**)` (0x56DAD0, `ret 0x14`)
    pub fn evaluate_car_position(pi: GPtr<u8>, e: GPtr<u8>, ped: GPtr<u8>, d: f32, out_d: GPtr<f32>, out_v: GPtr<GPtr<u8>>) = thiscall 0x56DAD0 => crate::player_info::evaluate_car_position;
    /// CRT `floor(double)` (0x8219F0, ST0)
    pub fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes` (0x563470)
    pub fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CPed::GetWeaponSkill()` (0x5E6580, AL)
    pub fn ped_get_weapon_skill_current(p: GPtr<u8>) -> u8 = thiscall 0x5E6580 => crate::ped::get_weapon_skill_current;
    /// `CCamera::Find3rdPersonCamTargetVector(float range, CVector source, CVector& cam, CVector& point)` (0x514970, `ret 0x18`)
    pub fn camera_find_3rd_person_cam_target_vector(cam: GPtr<u8>, range: f32, sx: f32, sy: f32, sz: f32, out_cam: GPtr<f32>, out_point: GPtr<f32>) = thiscall 0x514970 => unported;
    /// `CWorld::ResetLineTestOptions` (0x5631C0)
    pub fn world_reset_line_test_options() = cdecl 0x5631C0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPedIntelligence::IsInSeeingRange(const CVector&)` (0x600C60)
    pub fn intel_is_in_seeing_range(intel: GPtr<u8>, p: GPtr<f32>) -> u8 = thiscall 0x600C60 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40)
    pub fn ped_groups_are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CEventGunAimedAt::CEventGunAimedAt(CPed*)` (0x4B0700, `ret 4`, unnamed in the catalogue)
    pub fn event_gun_aimed_at_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B0700 => unported;
    /// `CEventGunAimedAt` destructor (0x4B07B0, unnamed in the catalogue)
    pub fn event_gun_aimed_at_dtor(e: GPtr<u8>) = thiscall 0x4B07B0 => unported;
    /// `CPlayerPed::EvaluateNeighbouringTarget` (0x60D1C0, `ret 0x18`)
    pub fn evaluate_neighbouring_target_call(p: GPtr<u8>, t: GPtr<u8>, out_t: GPtr<GPtr<u8>>, out_s: GPtr<f32>, max_dist: f32, heading: f32, left: u32) = thiscall 0x60D1C0 => evaluate_neighbouring_target;
    /// `CPlayerPed::EvaluateTarget` (0x60D020, `ret 0x18`)
    pub fn evaluate_target_call(p: GPtr<u8>, t: GPtr<u8>, out_t: GPtr<GPtr<u8>>, out_s: GPtr<f32>, max_dist: f32, heading: f32, b: u32) = thiscall 0x60D020 => evaluate_target;
    /// `CGameLogic::IsCoopGameGoingOn` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetPedWalkLeftRight` (0x53FC90, AX)
    pub fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CPad::GetPedWalkUpDown` (0x53FD30, AX)
    pub fn pad_get_ped_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0, ST0)
    pub fn get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    pub fn limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CVector::Normalise` (0x59C910)
    pub fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, result through the hidden pointer; unnamed in the catalogue)
    pub fn vector_mul_scalar(out: GPtr<f32>, v: GPtr<f32>, s: f32) -> GPtr<f32> = cdecl 0x40FEC0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, result through the hidden pointer; unnamed in the catalogue)
    pub fn vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `RwEngineInstance->dOpenDevice.fpRenderStateSet(state, value)` (table slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(RxObjSpace3DVertex*, uint32, RwMatrix*, uint32)` (0x7EF450)
    pub fn rw_im3d_transform(v: u32, n: u32, m: u32, flags: u32) -> u32 = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    pub fn rw_im3d_render_indexed_primitive(ty: u32, idx: u32, n: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    pub fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    pub fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CAnimBlendAssociation` deleting destructor (vtable slot 0)
    pub fn v_assoc_delete(a: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(void(*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBC0)
    pub fn assoc_set_delete_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBC0 => unported;
    /// `CPed::RestoreHeadingRate` (0x5DFD60)
    pub fn ped_restore_heading_rate(p: GPtr<u8>) = thiscall 0x5DFD60 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    pub fn blend_animation(clump: GPtr<u8>, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    pub fn add_animation(clump: GPtr<u8>, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CPlayerPed::GetPadFromPlayer` (0x609560)
    pub fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => crate::player_ped::get_pad_from_player;
    /// `CPad::GetForceCameraBehindPlayer` (0x540AE0)
    pub fn pad_get_force_camera_behind_player(pad: GPtr<u8>) -> u8 = thiscall 0x540AE0 => unported;
    /// `CPad::AimWeaponLeftRight(CPed*)` (0x541040, AX)
    pub fn pad_aim_weapon_left_right(pad: GPtr<u8>, ped: u32) -> i16 = thiscall 0x541040 => unported;
    /// `CPedIntelligence::GetTaskFighting` (0x600F30)
    pub fn intel_get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `CPedIntelligence::GetTaskUseGun` (0x600F70)
    pub fn intel_get_task_use_gun(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0)
    pub fn intel_get_task_duck(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CPedIntelligence::GetTaskThrow` (0x600FB0)
    pub fn intel_get_task_throw(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600FB0 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    pub fn intel_get_task_jet_pack(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity*, bool, bool, bool, bool, bool, bool)` (0x569E20)
    pub fn world_test_sphere_against_world(cx: f32, cy: f32, cz: f32, r: f32, ignore: u32, b: u32, v: u32, p: u32, o: u32, d: u32, cam: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CClothes::GetDefaultPlayerMotionGroup` (0x5A81B0)
    pub fn get_default_player_motion_group() -> i32 = cdecl 0x5A81B0 => unported;
    /// `CStats::UpdateStatsWhenSprinting` (0x55C660)
    pub fn stats_update_when_sprinting() = cdecl 0x55C660 => unported;
    /// `CStats::UpdateStatsWhenRunning` (0x55C6F0)
    pub fn stats_update_when_running() = cdecl 0x55C6F0 => unported;
    /// `CPlayerPed::GetButtonSprintResults(eSprintType)` (0x60A820, ST0; EDX preserved for the original caller)
    pub fn get_button_sprint_results(p: GPtr<u8>, ty: i32) -> f64 = thiscall 0x60A820 => unported;
}

/// `m_pPlayerData`
fn pdata(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x480)
}

/// `CPedGroups::ms_groups[pd->m_nPlayerGroup]` (the player data re-read, as every use in the original does)
fn player_group(p: GPtr<u8>) -> GPtr<u8> {
    g::<u8>(GROUPS + rd::<i32>(pdata(p), 0x38) as usize * GROUP_SIZE)
}

/// `&m_aWeapons[(int8)m_nActiveWeaponSlot]`
fn active_weapon(p: GPtr<u8>) -> GPtr<u8> {
    p.byte_add(0x5A0 + rd::<i8>(p, 0x718) as isize * 0x1C)
}

/// Entity type bits (`[+0x36] & 7`).
fn ent_type(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// `sqrt((dz*dz + dy*dy) + dx*dx)` of `t - s` (memory operands; the inlined CVector::Magnitude).
fn dist3(t: GPtr<f32>, s: GPtr<f32>) -> X87 {
    let dx = x(t.read()).fsub_m(s.read());
    let dy = x(t.wrapping_index(1).read()).fsub_m(s.wrapping_index(1).read());
    let dz = x(t.wrapping_index(2).read()).fsub_m(s.wrapping_index(2).read());
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// The two inlined `while (a > pi) a -= 2pi; while (a < -pi) a += 2pi;` loops (memory constants).
fn wrap_pi(mut a: X87) -> X87 {
    while !a.fcom(x(PI)).test_any(0x41) {
        a = a.fsub_m(TWO_PI);
    }
    while !a.fcom(x(MINUS_PI)).test_parity(5) {
        a = a.fadd_m(TWO_PI);
    }
    a
}

/// `CPlayerPed::DisbandPlayerGroup` (0x60A0A0): with followers, `Say(n > 1 ? 0x95 : 0x96, 0, 1.0f, 0, 0, 0)`;
/// then `RemoveAllFollowers(true)` on the group membership.
#[hook(addr = 0x60A0A0, conv = thiscall, name = "CPlayerPed::DisbandPlayerGroup", expect = "56 8B F1 8B 86 80 04 00 00", this = "CPlayerPed")]
pub fn disband_player_group(this: GPtr<u8>) {
    let n = membership_count_excluding_leader(player_group(this).byte_add(8));
    if n > 0 {
        ped_say(this, if n > 1 { 0x95 } else { 0x96 }, 0, 1.0, 0, 0, 0);
    }
    membership_remove_all_followers(player_group(this).byte_add(8), 1);
}

/// `CPlayerPed::MakeGroupRespondToPlayerTakingDamage(CEventDamage&)` (0x60A110, `ret 4`): when the event has a
/// source entity (+0x14), the group has members and m_bMembersEnterLeadersVehicle: a stack
/// `CEventGroupEvent(this, event.Clone())` is added to the group intelligence and destroyed.
#[hook(addr = 0x60A110, conv = thiscall, name = "CPlayerPed::MakeGroupRespondToPlayerTakingDamage", expect = "64 A1 00 00 00 00 6A FF", this = "CPlayerPed")]
pub fn make_group_respond_to_player_taking_damage(this: GPtr<u8>, ev: GPtr<u8>) {
    if rd::<u32>(ev, 0x14) == 0 {
        return;
    }
    if membership_count_excluding_leader(player_group(this).byte_add(8)) < 1 {
        return;
    }
    let grp = player_group(this);
    if rd::<u8>(grp, 4) == 0 {
        return;
    }
    let gi = grp.byte_add(0x30);
    let clone = v_event_clone(ev);
    let mut ge = [0u32; 5];
    let gp: GPtr<u8> = lp(&mut ge).cast();
    event_group_event_ctor(gp, this, clone);
    group_intel_add_event(gi, gp);
    event_group_event_dtor(gp);
}

/// `CPlayerPed::PlayerWantsToAttack` (0x60CC50): see `CPlayerPed.md`. Target = m_pTargetedObject, or the mouse
/// target with the 3rd-person mouse camera; a ped target is used as is, a tag or a targettable object ends the
/// function, anything else (or none) asks `FindPedToAttack`.
#[hook(addr = 0x60CC50, conv = thiscall, name = "CPlayerPed::PlayerWantsToAttack", expect = "57 8B F9 8B 87 80 04 00 00", this = "CPlayerPed")]
pub fn player_wants_to_attack(this: GPtr<u8>) {
    if membership_count_excluding_leader(player_group(this).byte_add(8)) < 1 {
        return;
    }
    let grp = player_group(this);
    if rd::<u8>(grp, 4) == 0 {
        return;
    }
    group_intel_report_all_bar_script_tasks_finished(grp.byte_add(0x30));
    let mut t = rd::<GPtr<u8>>(this, 0x71C);
    if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && t.is_null() {
        t = rd::<GPtr<u8>>(this, 0x79C);
    }
    let mut target = GPtr::NULL;
    let mut search = true;
    if !t.is_null() {
        if ent_type(t) == 3 {
            target = t;
            search = false;
        } else {
            if tag_is_tag(t) != 0 {
                return;
            }
            if ent_type(t) == 4 && object_can_be_targetted(t) != 0 {
                return;
            }
        }
    }
    if search {
        target = find_ped_to_attack_call(this);
    }
    if !target.is_null() {
        group_player_gave_command_attack(player_group(this), this, target);
    }
}

/// The point `LOSBlockedBetweenPeds` uses for an entity: a ped's bone 4 (`GetBonePosition(out, 4, false)`),
/// else the position (copied).
fn los_point(e: GPtr<u8>, out: &mut [f32; 3]) {
    if ent_type(e) == 3 {
        ped_get_bone_position(e, lp(out).cast(), 4, 0);
    } else {
        *out = vec3(ent_pos(e));
    }
}

/// `CPlayerPed::LOSBlockedBetweenPeds(CEntity* a, CEntity* b)` (0x60B550, cdecl, AL): line of sight from a's
/// point (a ducking ped's z + 0.35f, x87) to b's point against buildings and objects (shoot-through on); the
/// hit entity is written into a's argument slot (initially a), so the result is `hit && hitEntity != b`.
#[hook(addr = 0x60B550, conv = cdecl, name = "CPlayerPed::LOSBlockedBetweenPeds", expect = "83 EC 44 56 8B 74 24 4C")]
pub fn los_blocked_between_peds(a: GPtr<u8>, b: GPtr<u8>) -> u8 {
    let mut pa = [0f32; 3];
    let mut pb = [0f32; 3];
    los_point(a, &mut pa);
    if ent_type(a) == 3 && rd::<u32>(a, 0x46C) & 0x0400_0000 != 0 {
        pa[2] = x(pa[2]).fadd_m(f32::from_bits(0x3EB3_3333)).to_f32();
    }
    los_point(b, &mut pb);
    let mut cp = [0u32; 0xB];
    let mut hit = a;
    let r = world_process_line_of_sight(lp(&mut pa).cast(), lp(&mut pb).cast(), lp(&mut cp).cast(), lp(&mut hit), 1, 0, 0, 1, 0, 0, 0, 1);
    if r == 0 {
        return 0;
    }
    (hit != b) as u8
}

/// `CPlayerPed::DoesTargetHaveToBeBroken(CEntity* target, CWeapon* weapon)` (0x60C0C0, `ret 8`, AL): true when
/// the target's byte +0x1C bit 7 is clear; when `range(GetWeaponInfo(type, GetWeaponSkill(type))) *
/// TargetWeaponRangeMultiplier(target, this)` (f32 stored) is below the 3D distance (`(dz*dz + dy*dy) + dx*dx`
/// of f32-stored differences); for a spraycan (0x29) on a building tag whose alpha is 255; else
/// `!CanIKReachThisTarget(target pos, weapon, false)`.
#[hook(addr = 0x60C0C0, conv = thiscall, name = "CPlayerPed::DoesTargetHaveToBeBroken", expect = "83 EC 0C 53 55 56 57", this = "CPlayerPed")]
pub fn does_target_have_to_be_broken(this: GPtr<u8>, target: GPtr<u8>, weapon: GPtr<u8>) -> u8 {
    if rd::<i8>(target, 0x1C) >= 0 {
        return 1;
    }
    let mult = X87::from_f64_exact(target_weapon_range_multiplier(target, this)).to_f32();
    let s = ent_pos(this);
    let t = ent_pos(target);
    let dx = x(t.read()).fsub_m(s.read()).to_f32();
    let ty = rd::<i32>(weapon, 0);
    let dy = x(t.wrapping_index(1).read()).fsub_m(s.wrapping_index(1).read()).to_f32();
    let dz = x(t.wrapping_index(2).read()).fsub_m(s.wrapping_index(2).read()).to_f32();
    let skill = ped_get_weapon_skill_type(this, ty);
    let info = get_weapon_info(ty, i32::from(skill));
    let d = x(dz).fmul_m(dz).fadd(x(dy).fmul_m(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
    let r = x(rd::<f32>(info, 4)).fmul_m(mult);
    if !r.fcom(d).test_parity(5) {
        return 1;
    }
    if rd::<i32>(weapon, 0) == 0x29 && ent_type(target) == 1 && tag_is_tag(target) != 0 && tag_get_alpha(target) == 0xFF {
        return 1;
    }
    let p = vec3(ent_pos(target));
    (can_ik_reach_this_target(this, p[0], p[1], p[2], weapon, 0) == 0) as u8
}

/// `CPlayerPed::EvaluateNeighbouringTarget(target, CEntity** outTarget, float* outScore, float maxDist,
/// float baseHeading, bool leftSide)` (0x60D1C0, `ret 0x18`): see `CPlayerPed.md`.
#[hook(addr = 0x60D1C0, conv = thiscall, name = "CPlayerPed::EvaluateNeighbouringTarget", expect = "8B 41 14 85 C0", this = "CPlayerPed")]
pub fn evaluate_neighbouring_target(this: GPtr<u8>, target: GPtr<u8>, out_target: GPtr<GPtr<u8>>, out_score: GPtr<f32>, max_dist: f32, heading: f32, left: u32) {
    let d = dist3(ent_pos(target), ent_pos(this));
    if d.fcom(x(max_dist)).test_parity(0x41) {
        return;
    }
    if does_target_have_to_be_broken_call(this, target, active_weapon(this)) != 0 {
        return;
    }
    let t = ent_pos(target);
    let c = cam_pos();
    let ay = x(t.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read()).to_f32();
    let ax = x(t.read()).fsub_m(c.read()).to_f32();
    let a = wrap_pi(X87::from_f64_exact(get_atan_of_xy(ax, ay)).fsub_m(heading));
    let abs = if !a.fcom(x(0.0)).test_parity(5) { a.fchs() } else { a };
    if abs.fcom(x(NEIGHBOUR_MAX_ANGLE)).test_parity(5) {
        return;
    }
    let c0 = a.fcom(x(0.0));
    let score = if left as u8 != 0 {
        if c0.test_parity(0x41) { a.fchs() } else { x(-100000.0) }
    } else if c0.test_any(1) {
        a
    } else {
        x(-100000.0)
    };
    if score.fcom(x(out_score.read())).test_any(0x41) {
        return;
    }
    out_target.write(target);
    out_score.write(score.to_f32());
}

/// `CPlayerPed::EvaluateTarget(target, CEntity** outTarget, float* outScore, float maxDist, float baseHeading,
/// bool)` (0x60D020, `ret 0x18`; the last argument is unused). Distance `d` (f32 stored) must be `<= maxDist`
/// and the target must not have to be broken; `a = GetATanOfXY(target - this) - heading` wrapped into
/// [-pi, pi]; `score = 1 - |a * 57.29578f| / PLAYER_MAX_TARGET_VIEW_ANGLE` (f32 stored into the target argument
/// slot), divided by `sqrt(sqrt(d))` when `d > 1`, times `FindTargetPriority(target)`; taken when greater than
/// `*outScore` and the line of sight is not blocked.
#[hook(addr = 0x60D020, conv = thiscall, name = "CPlayerPed::EvaluateTarget", expect = "51 56 8B F1 8B 46 14", this = "CPlayerPed")]
pub fn evaluate_target(this: GPtr<u8>, target: GPtr<u8>, out_target: GPtr<GPtr<u8>>, out_score: GPtr<f32>, max_dist: f32, heading: f32, _unused: u32) {
    let d = dist3(ent_pos(target), ent_pos(this));
    let dl = d.to_f32();
    if d.fcom(x(max_dist)).test_parity(0x41) {
        return;
    }
    if does_target_have_to_be_broken_call(this, target, active_weapon(this)) != 0 {
        return;
    }
    let t = ent_pos(target);
    let s = ent_pos(this);
    let ay = x(t.wrapping_index(1).read()).fsub_m(s.wrapping_index(1).read()).to_f32();
    let ax = x(t.read()).fsub_m(s.read()).to_f32();
    let mut a = wrap_pi(X87::from_f64_exact(get_atan_of_xy(ax, ay)).fsub_m(heading));
    a = a.fmul_m(RAD_TO_DEG);
    if !a.fcom(x(0.0)).test_parity(5) {
        a = a.fchs();
    }
    let mut sc = a.fdiv_m(g::<f32>(PLAYER_MAX_TARGET_VIEW_ANGLE).read()).fsubr_m(1.0).to_f32();
    if !x(dl).fcom(x(1.0)).test_any(0x41) {
        sc = x(dl).fsqrt().fsqrt().fdivr_m(sc).to_f32();
    }
    let s2 = X87::from_f64_exact(find_target_priority(this, target)).fmul_m(sc);
    let s2f = s2.to_f32();
    if s2.fcom(x(out_score.read())).test_any(0x41) {
        return;
    }
    if los_blocked_between_peds_call(this, target) != 0 {
        return;
    }
    out_target.write(target);
    out_score.write(s2f);
}

/// `CPlayerPed::FindPedToAttack` (0x60C5F0, EAX): over the ped pool (top-down) the non-player, alive peds that
/// are not in the player's group and not GANG2 (8): `d = DistToLine(O, E, P)` with `O = FindPlayerCoors()` (z
/// zeroed), `E = O + camera front * 100` (z 0) and `P` the ped position (z 0); `+ (p2d - 20) * 0.2f` when the 2D
/// distance `p2d` exceeds 20; gang types 7..16: `max(d * 0.5f - 2, 0)` (NaN -> 0); the smallest `d` below
/// 99999.9f (initial best, f32) wins.
#[hook(addr = 0x60C5F0, conv = thiscall, name = "CPlayerPed::FindPedToAttack", expect = "83 EC 30 53 8B 1D 90 44 B7 00", this = "CPlayerPed")]
pub fn find_ped_to_attack(this: GPtr<u8>) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = pool_size(pool);
    let mut best = GPtr::NULL;
    let mut best_score = f32::from_bits(0x47C3_4FF3);
    let mut o = [0f32; 3];
    find_player_coors(lp(&mut o).cast(), -1);
    let fx = x(g::<f32>(0xB6F9AC).read()).fmul_m(100.0);
    let fy = x(g::<f32>(0xB6F9B0).read()).fmul_m(100.0);
    let mut e = [0f32; 3];
    o[2] = 0.0;
    e[0] = fx.fadd_m(o[0]).to_f32();
    e[1] = x(o[1]).fadd(fy).to_f32();
    if i == 0 {
        return GPtr::NULL;
    }
    while i != 0 {
        let idx = (i - 1) as u32;
        i -= 1;
        if pool_flag(pool, idx) & 0x80 != 0 {
            continue;
        }
        let ped = pool_slot(pool, idx, PED_SLOT);
        if ped.is_null() || ped_is_player(ped) != 0 || ped_is_alive(ped) as u8 == 0 {
            continue;
        }
        if membership_is_member(player_group(this).byte_add(8), ped) != 0 {
            continue;
        }
        let pt = rd::<i32>(ped, 0x598);
        if pt == 8 {
            continue;
        }
        let mut p = vec3(ent_pos(ped));
        p[2] = 0.0;
        let dl = X87::from_f64_exact(dist_to_line(lp(&mut o).cast(), lp(&mut e).cast(), lp(&mut p).cast()));
        let dx = x(p[0]).fsub_m(o[0]);
        let dy = x(p[1]).fsub_m(o[1]);
        let pd = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        let mut d = dl;
        if !pd.fcom(x(20.0)).test_any(0x41) {
            d = pd.fsub_m(20.0).fmul_m(f32::from_bits(0x3E4C_CCCD)).fadd(d);
        }
        let pt = rd::<i32>(ped, 0x598);
        if (7..=16).contains(&pt) {
            d = d.fmul_m(0.5).fsub_m(2.0);
            if d.fcom(x(0.0)).test_any(0x41) {
                d = x(0.0);
            }
        }
        if !d.fcom(x(best_score)).test_parity(5) {
            best_score = d.to_f32();
            best = ped;
        }
    }
    best
}

/// `(uint16)_ftol(CTimer::ms_fTimeStepNonClipped * 0.02f * 1000.0f)` (x87)
fn step_ms() -> u16 {
    ftol32(x(g::<f32>(0xB7CB58).read()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0)) as u16
}

/// `CPlayerPed::ProcessGroupBehaviour(CPad*)` (0x60D350, `ret 4`): see `CPlayerPed.md`. The forward hold
/// disbands only when its counter (pd +0x8A, u16) is exactly 1200, the back hold (+0x88) at >= 1200; a short
/// forward press recruits a GANG2 ped (or any ped with a recruit cheat) unless group stuff is disabled, else
/// tells the group to follow; the back press recruits only GANG2 (the disabled flag was tested before) or
/// tells the group to stop following. Every 32nd frame (counter & 0x1F == 6) every GANG2 ped's acquaintance
/// 4 with ped type 6 is set when the player's wanted level is > 0, else cleared (the wanted pointer is not
/// null-checked).
#[hook(addr = 0x60D350, conv = thiscall, name = "CPlayerPed::ProcessGroupBehaviour", expect = "A0 2E EC B6 00", this = "CPlayerPed")]
pub fn process_group_behaviour(this: GPtr<u8>, pad: GPtr<u8>) {
    let mut t = rd::<GPtr<u8>>(this, 0x71C);
    if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && t.is_null() {
        t = rd::<GPtr<u8>>(this, 0x79C);
    }
    let is_ped = |t: GPtr<u8>| !t.is_null() && ent_type(t) == 3;
    if find_player_vehicle(-1, 0).is_null() {
        let fwd = pad_get_group_control_forward(pad);
        let pd = pdata(this);
        if fwd != 0 {
            wr::<u16>(pd, 0x8A, rd::<u16>(pd, 0x8A).wrapping_add(step_ms()));
            let pd = pdata(this);
            if rd::<u16>(pd, 0x8A) == 0x4B0 && rd::<u32>(pd, 0x34) & 0x100 == 0 {
                disband_player_group_call(this);
            }
        } else {
            let v = rd::<u16>(pd, 0x8A);
            if v != 0 && v < 0x4B0 {
                if is_ped(t) && (rd::<i32>(t, 0x598) == 8 || recruit_cheats_active() != 0) {
                    if rd::<u32>(pd, 0x34) & 0x100 == 0 {
                        make_this_ped_join_our_group_call(this, t);
                    }
                } else {
                    tell_group_to_start_following_player_call(this, 1, 1, 0);
                }
            }
            wr::<u16>(pdata(this), 0x8A, 0);
        }
    }
    if rd::<u32>(pdata(this), 0x34) & 0x100 != 0 {
        return;
    }
    if find_player_vehicle(-1, 0).is_null() {
        if pad_get_group_control_back(pad) != 0 {
            let pd = pdata(this);
            wr::<u16>(pd, 0x88, rd::<u16>(pd, 0x88).wrapping_add(step_ms()));
            if rd::<u16>(pdata(this), 0x88) >= 0x4B0 {
                disband_player_group_call(this);
            }
        } else {
            let v = rd::<u16>(pdata(this), 0x88);
            if v != 0 && v < 0x4B0 {
                if is_ped(t) && rd::<i32>(t, 0x598) == 8 {
                    make_this_ped_join_our_group_call(this, t);
                } else {
                    tell_group_to_start_following_player_call(this, 0, 1, 0);
                }
            }
            wr::<u16>(pdata(this), 0x88, 0);
        }
    }
    if g::<u32>(0xB7CB4C).read() & 0x1F != 6 {
        return;
    }
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = pool_size(pool);
    while i != 0 {
        let idx = (i - 1) as u32;
        i -= 1;
        if pool_flag(pool, idx) & 0x80 != 0 {
            continue;
        }
        let ped = pool_slot(pool, idx, PED_SLOT);
        if ped.is_null() || rd::<i32>(ped, 0x598) != 8 {
            continue;
        }
        let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
        let w = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
        let f = ped_type_get_ped_flag(6);
        if rd::<i32>(w, 0x2C) > 0 {
            acq_set_as_acquaintance(ped.byte_add(0x4E0), 4, f);
        } else {
            acq_clear_as_acquaintance(ped.byte_add(0x4E0), 4, f);
        }
    }
}

/// `CPlayerPed::TellGroupToStartFollowingPlayer(bool follow, bool say, bool force)` (0x60A1D0, `ret 0xC`).
/// Returns when pd +0x34 bit 9 (always follow) is set and `!follow`, when bit 10 (never follow) is set and
/// `follow`, or (without `force`) when the group has no members. Sets m_bMembersEnterLeadersVehicle = follow
/// and the default task allocator 5. With `follow`: a stack `CEventPlayerCommandToGroup(0, null)` with vtable
/// 0x86C710 computes its response task type for the group; if it will respond, a heap one (cmd 2, vtable
/// 0x86D120) gets that type (+0xE, written through the pointer even when the allocation failed) and is added
/// to the group intelligence in a stack `CEventGroupEvent(this, e)`. With `say` and members: the furthest
/// member distance picks the line (follow: 0x9B/0x9C below 3.0, 0x99/0x9A below 10.0, else 0x97/0x98 for
/// more than one / one member; stop: 0x9F / 0xA0); `Say(id, 0, 1.0f, 0, 0, 0)`.
#[hook(addr = 0x60A1D0, conv = thiscall, name = "CPlayerPed::TellGroupToStartFollowingPlayer", expect = "64 A1 00 00 00 00 6A FF", this = "CPlayerPed")]
pub fn tell_group_to_start_following_player(this: GPtr<u8>, follow: u32, say: u32, force: u32) {
    let follow = follow as u8;
    let pd = pdata(this);
    if rd::<u32>(pd, 0x34) & 0x200 != 0 && follow == 0 {
        return;
    }
    if rd::<u32>(pd, 0x34) & 0x400 != 0 && follow != 0 {
        return;
    }
    if force as u8 == 0 && membership_count_excluding_leader(player_group(this).byte_add(8)) == 0 {
        return;
    }
    wr::<u8>(player_group(this), 4, follow);
    group_intel_set_default_task_allocator_type(player_group(this).byte_add(0x30), 5);
    if follow != 0 {
        let mut ev = [0u32; 7];
        let evp: GPtr<u8> = lp(&mut ev).cast();
        event_player_command_to_group_ctor(evp, 0, GPtr::NULL);
        wr::<u32>(evp, 0, 0x86C710);
        event_compute_response_task_type_group(evp, player_group(this));
        if event_will_respond(evp) != 0 {
            let e = event_operator_new(0x1C);
            if !e.is_null() {
                event_player_command_to_group_ctor(e, 2, GPtr::NULL);
                wr::<u32>(e, 0, 0x86D120);
            }
            wr::<i16>(e, 0xE, rd::<i16>(evp, 0xE));
            let mut ge = [0u32; 5];
            let gp: GPtr<u8> = lp(&mut ge).cast();
            event_group_event_ctor(gp, this, e);
            group_intel_add_event(player_group(this).byte_add(0x30), gp);
            event_group_event_dtor(gp);
        }
        wr::<u32>(evp, 0, 0x86C710);
        event_player_command_to_group_dtor(evp);
    }
    if say as u8 == 0 {
        return;
    }
    let n = membership_count_excluding_leader(player_group(this).byte_add(8));
    if n <= 0 {
        return;
    }
    let d = X87::from_f64_exact(group_find_distance_to_furthest_member(player_group(this)));
    let id = if follow != 0 {
        if !d.fcom(x(3.0)).test_parity(5) {
            if n > 1 { 0x9B } else { 0x9C }
        } else if !d.fcom(x(10.0)).test_parity(5) {
            if n > 1 { 0x99 } else { 0x9A }
        } else if n > 1 {
            0x97
        } else {
            0x98
        }
    } else if n > 1 {
        0x9F
    } else {
        0xA0
    };
    ped_say(this, id, 0, 1.0, 0, 0, 0);
}

/// `CPlayerPed::KeepAreaAroundPlayerClear` (0x60C1E0; entry `push -1; jmp 0x401771`, a protection stub that
/// builds the SEH frame and returns to 0x60C1E7). Peds: the 16 ped-scanner entities (intelligence +0x130)
/// created by population (+0x484 == 1), not in a vehicle, alive and not in group 0: on screen and without
/// bKeepTasksAfterCleanUp (+0x478 bit 4) they get a stack `CEventScriptCommand(3, new
/// CTaskComplexSmartFleeEntity(this, false, 1000.0f, 100000, [0x86F678], [0xC18CF0]), false)` (task +0x30 = 4
/// written even when the allocation failed) unless a flee task (911) on this player or a script-command
/// event (0x20) carrying a 911 task exists; off screen they are flagged for destruction (vtable +0x54).
/// Vehicles (FindObjectsInRange around this ped's position, 15.0f, 2D, max 6) not created by a mission
/// (+0x4A4 != 2) with status not 0/9: temp action 1 for 5000 ms when the 3D squared distance from P (the
/// vehicle's position when this ped is in one, else its own) exceeds 25, else 3 / 9 (by the sign of the
/// forward-dot, m_matrix dereferenced without a null check) for 2000 ms; then PossiblyRemoveVehicle.
#[hook(addr = 0x60C1E0, conv = thiscall, name = "CPlayerPed::KeepAreaAroundPlayerClear", expect = "6A FF E9 8A 55 DF FF", this = "CPlayerPed")]
pub fn keep_area_around_player_clear(this: GPtr<u8>) {
    let intel = rd::<GPtr<u8>>(this, 0x47C);
    for i in 0..16isize {
        let ped = rd::<GPtr<u8>>(intel, 0x130 + i * 4);
        if ped.is_null() || rd::<u8>(ped, 0x484) != 1 || rd::<u32>(ped, 0x46C) & 0x100 != 0 {
            continue;
        }
        if ped_is_alive(ped) as u8 == 0 || membership_is_member(g::<u8>(GROUPS + 8), ped) != 0 {
            continue;
        }
        if entity_get_is_on_screen(ped) != 0 && rd::<u8>(ped, 0x478) & 0x10 == 0 {
            let t = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x38F);
            if !t.is_null() && rd::<GPtr<u8>>(t, 0xC) == this {
                continue;
            }
            let e = event_group_get_event_of_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), 0x20);
            if !e.is_null() {
                let et = rd::<GPtr<u8>>(e, 0x10);
                if !et.is_null() && v_task_get_type(et) == 0x38F {
                    continue;
                }
            }
            let mut t = task_operator_new(0x40);
            if !t.is_null() {
                let a = g::<u32>(0xC18CF0).read();
                let b = g::<u32>(0x86F678).read();
                t = task_smart_flee_entity_ctor(t, this, 0, 1000.0, 100000, b, a);
            }
            let mut ev = [0u32; 6];
            let evp: GPtr<u8> = lp(&mut ev).cast();
            wr::<u32>(t, 0x30, 4);
            event_script_command_ctor(evp, 3, t, 0);
            event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
            event_script_command_dtor(evp);
        } else {
            v_flag_to_destroy_when_next_processed(ped);
        }
    }
    let veh = rd::<GPtr<u8>>(this, 0x58C);
    let pp = if rd::<u32>(this, 0x46C) & 0x100 != 0 && !veh.is_null() { ent_pos(veh) } else { ent_pos(this) };
    let p = vec3(pp);
    let mut c = vec3(ent_pos(this));
    let mut n: i16 = 0;
    let mut list = [GPtr::<u8>::NULL; 6];
    world_find_objects_in_range(lp(&mut c).cast(), 15.0, 1, lp(&mut n), 6, lp(&mut list).cast(), 0, 1, 0, 0, 0);
    let mut k = 0i32;
    while k < i32::from(n) {
        let v = list[k as usize];
        k += 1;
        if rd::<u8>(v, 0x4A4) == 2 {
            continue;
        }
        let st = rd::<u8>(v, 0x36) >> 3;
        if st == 0 || st == 9 {
            continue;
        }
        let m = rd::<GPtr<u8>>(v, 0x14);
        let vp = ent_pos(v);
        let dx = x(vp.read()).fsub_m(p[0]);
        let dy = x(vp.wrapping_index(1).read()).fsub_m(p[1]);
        let dz = x(vp.wrapping_index(2).read()).fsub_m(p[2]);
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let now = g::<u32>(0xB7CB84).read();
        if !d2.fcom(x(25.0)).test_any(0x41) {
            wr::<u8>(v, 0x3BB, 1);
            wr::<u32>(v, 0x3BC, now.wrapping_add(5000));
        } else {
            let ty = x(p[1]).fsub_m(vp.wrapping_index(1).read()).fmul_m(rd::<f32>(m, 0x14));
            let tx = x(p[0]).fsub_m(vp.read()).fmul_m(rd::<f32>(m, 0x10));
            if !ty.fadd(tx).fcom(x(0.0)).test_any(0x41) {
                wr::<u8>(v, 0x3BB, 3);
            } else {
                wr::<u8>(v, 0x3BB, 9);
            }
            wr::<u32>(v, 0x3BC, g::<u32>(0xB7CB84).read().wrapping_add(2000));
        }
        car_ctrl_possibly_remove_vehicle(v);
    }
}

/// `CPlayerPed::MakeThisPedJoinOurGroup(CPed*)` (0x60C840, `ret 4`): see `CPlayerPed.md` (the two `Count() <
/// max` tests with two calls, the stat-181 tests with two `GetStatValue` calls, the refusal event).
#[hook(addr = 0x60C840, conv = thiscall, name = "CPlayerPed::MakeThisPedJoinOurGroup", expect = "64 A1 00 00 00 00 6A FF", this = "CPlayerPed")]
pub fn make_this_ped_join_our_group(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(ped, 0x478) & 0x400 != 0 {
        ped_say(this, 0x52, 0, 1.0, 0, 0, 0);
        return;
    }
    if !task_mgr_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 1000).is_null() {
        return;
    }
    if rd::<i32>(ped, 0x598) != 8 && g::<u8>(0x96917C).read() == 0 && g::<u8>(0x96917D).read() == 0 && g::<u8>(0x96917E).read() == 0 {
        return;
    }
    if membership_is_member(player_group(this).byte_add(8), ped) != 0 {
        return;
    }
    speech_set_cj_mood(-1, 10000, 1, -1, -1);
    ped_say(this, 0x7F, 0, 1.0, 1, 0, 0);
    let mut max = stats_find_max_number_of_group_members();
    let lim = i32::from(rd::<u8>(pdata(this), 0x43));
    if max >= lim {
        max = lim;
    }
    if X87::from_f64_exact(get_stat_value(0xB5)).fcom(x(1.0)) == sa_math::x87::Cmp::Equal
        || X87::from_f64_exact(get_stat_value(0xB5)).fcom(x(2.0)) == sa_math::x87::Cmp::Equal
    {
        max = 0;
    }
    let count = || membership_count_excluding_leader(player_group(this).byte_add(8));
    let join = (recruit_cheats_active() != 0 && count() < 7) || (count() < max && count() < max);
    if !join {
        ped_say(ped, 0x7D, 2500, 1.0, 1, 0, 0);
        let mut ev = [0u32; 8];
        let evp: GPtr<u8> = lp(&mut ev).cast();
        event_dont_join_player_group_ctor(evp, this);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
        event_dont_join_player_group_dtor(evp);
        return;
    }
    let old = ped_groups_get_peds_group(ped);
    if !old.is_null() {
        membership_remove_member(old.byte_add(8), ped);
    }
    let mut task = task_operator_new(0x28);
    if !task.is_null() {
        let id = rd::<i32>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x38);
        task = task_be_in_group_ctor(task, id, 0);
    }
    let mut ev = [0u32; 6];
    let evp: GPtr<u8> = lp(&mut ev).cast();
    event_script_command_ctor(evp, 3, task, 0);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
    membership_add_follower(player_group(this).byte_add(8), ped);
    group_process(player_group(this));
    ped_give_weapon_when_joining_gang(ped);
    let mut e = event_operator_new(0x10);
    if !e.is_null() {
        e = event_new_gang_member_ctor(e, ped);
    }
    let mut ge = [0u32; 5];
    let gp: GPtr<u8> = lp(&mut ge).cast();
    event_group_event_ctor(gp, this, e);
    group_intel_add_event(player_group(this).byte_add(0x30), gp);
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) & 0xFFEF_FFFF);
    increment_stat(0xEE, 1.0);
    display_script_stat_update_message(1, 0x150, 1.0);
    let colour = (((u32::from(g::<u8>(0x8D1345).read()) << 8 | u32::from(g::<u8>(0x8D1351).read())) << 8
        | u32::from(g::<u8>(0x8D135D).read()))
        << 8)
        | 0xFF;
    let h = ped_pool_get_ref(g::<GPtr<u8>>(PED_POOL).read(), ped);
    let blip = radar_set_entity_blip(2, h, colour, 2, 0x86D1E0);
    radar_change_blip_scale(blip, 2);
    radar_change_blip_colour(blip, colour);
    radar_set_blip_friendly(blip, 1);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x2000);
    let sep = g::<u32>(0x86C6C0).read();
    wr::<u32>(player_group(this).byte_add(8), 0x24, sep);
    ped_say(ped, 0x7E, 2500, 1.0, 1, 0, 0);
    event_group_event_dtor(gp);
    event_script_command_dtor(evp);
}

/// `CPlayerInfo::FindClosestCarSectorList(CPtrListDoubleLink&, CPed*, float, float, float, float, float*
/// outDist, CVehicle** outVehicle)` (0x56F4E0, `ret 0x20`; the four bounds are unused): see `CPlayerInfo.md`.
/// The bound centre is fetched and then overwritten with the position; `zd = (dy + dx) * 0.0f + dz` (x87,
/// f32 stored).
#[hook(addr = 0x56F4E0, conv = thiscall, name = "CPlayerInfo::FindClosestCarSectorList", expect = "83 EC 38 8B 44 24 3C", this = "CPlayerInfo")]
#[allow(clippy::too_many_arguments)]
pub fn find_closest_car_sector_list(this: GPtr<u8>, list: GPtr<u8>, ped: GPtr<u8>, _min_x: f32, _min_y: f32, _max_x: f32, _max_y: f32, out_d: GPtr<f32>, out_v: GPtr<GPtr<u8>>) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let code = g::<u16>(0xB7CD78).read();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == code || rd::<u8>(e, 0x1C) & 1 == 0 || ent_type(e) != 2 {
            continue;
        }
        let st = rd::<u8>(e, 0x36) >> 3;
        wr::<u16>(e, 0x2C, code);
        if st == 5 || st == 6 {
            continue;
        }
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let m = rd::<GPtr<u8>>(e, 0x14);
        if x(rd::<f32>(m, 0x28)).fcom(x(f32::from_bits(0x3E99_999A))).test_any(0x41) && (ent_type(e) != 2 || rd::<i32>(e, 0x590) != 9) {
            continue;
        }
        let mut c = [0f32; 3];
        entity_get_bound_centre(e, lp(&mut c).cast());
        c = vec3(ent_pos(e));
        let h = X87::from_f64_exact(v_get_height_above_road(e));
        c[2] = h.fsubr_m(c[2]).fadd_m(1.0).to_f32();
        let pp = ent_pos(ped);
        let dx = x(pp.read()).fsub_m(c[0]);
        let dy = x(pp.wrapping_index(1).read()).fsub_m(c[1]);
        let dz = x(pp.wrapping_index(2).read()).fsub_m(c[2]);
        let mut zd = dy.fadd(dx).fmul_m(0.0).fadd(dz).to_f32();
        let mut accept = false;
        if rd::<i16>(e, 0x22) == 0x241 {
            let mut door = [0f32; 3];
            let mut id = 0i32;
            if get_nearest_car_door(ped, e, lp(&mut door).cast(), lp(&mut id)) != 0 {
                let v = x(ent_pos(ped).wrapping_index(2).read()).fsub_m(door[2]);
                let v = if v.to_f64().is_sign_negative() { v.fchs() } else { v };
                zd = v.to_f32();
                if !v.fcom(x(1.0)).test_parity(5) {
                    accept = true;
                }
            }
        }
        if !accept {
            let a = x(zd);
            let a = if a.to_f64().is_sign_negative() { a.fchs() } else { a };
            // accepted below |zd| < 2.0; else only boats continue
            if a.fcom(x(2.0)).test_parity(5) {
                if rd::<i32>(e, 0x590) != 5 {
                    continue;
                }
                let pz = ent_pos(ped).wrapping_index(2).read();
                let below = !x(c[2]).fcom(x(pz)).test_parity(5)
                    && !x(ent_pos(ped).wrapping_index(2).read()).fsub_m(4.0).fcom(x(c[2])).test_parity(5);
                if !below && (rd::<i32>(e, 0x590) != 5 || rd::<GPtr<u8>>(ped, 0x584) != e) {
                    continue;
                }
            }
        }
        if rd::<u8>(e, 0x42E) & 0x20 == 0 {
            continue;
        }
        let mut door = [0f32; 3];
        let mut id = 0i32;
        if get_nearest_car_door(ped, e, lp(&mut door).cast(), lp(&mut id)) == 0 {
            continue;
        }
        let ep = ent_pos(e);
        let pp = ent_pos(ped);
        let mut v = [0f32; 3];
        v[0] = x(pp.read()).fsub_m(ep.read()).to_f32();
        v[1] = x(pp.wrapping_index(1).read()).fsub_m(ep.wrapping_index(1).read()).to_f32();
        let mag = X87::from_f64_exact(vector_magnitude(lp(&mut v).cast()));
        let mut d = mag.to_f32();
        let mut take = false;
        if !mag.fcom(x(10.0)).test_any(0x41) && lies_inside_bounding_box(ped, ent_pos(ped), e) != 0 {
            d = 10.0;
            take = true;
        }
        if !take && x(d).fcom(x(10.0)).test_parity(0x41) {
            continue;
        }
        if cranes_is_this_car_being_carried(e) != 0 {
            continue;
        }
        evaluate_car_position(this, e, ped, d, out_d, out_v);
    }
}

/// The sector index `_ftol(floor((double)(v * 0.02f + 60.0f)))` of a register value, and the f32-stored copy
/// the second conversion uses.
fn sector_of(v: X87) -> (i32, f32) {
    let s = v.fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(60.0);
    let l = s.to_f32();
    (ftol32(X87::from_f64_exact(crt_floor(s.to_f64()))), l)
}

/// `_ftol(floor((double)l))` of an f32 local.
fn floor_ftol(l: f32) -> i32 {
    ftol32(X87::from_f64_exact(crt_floor(f64::from(l))))
}

/// `CPlayerInfo::FindObjectToSteal(CPed*)` (0x56DBD0, `ret 4`; ECX unused): see `CPlayerInfo.md`. The ped's
/// m_matrix is dereferenced before its null test; the scan code is bumped but never written into the objects;
/// the last qualifying liftable object in scan order wins.
#[hook(addr = 0x56DBD0, conv = thiscall, name = "CPlayerInfo::FindObjectToSteal", expect = "83 EC 30 8B 4C 24 34", this = "CPlayerInfo")]
pub fn find_object_to_steal(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let a = x(rd::<f32>(m, 0x10)).fmul_m(0.5);
    let b = x(rd::<f32>(m, 0x14)).fmul_m(0.5);
    let c = x(rd::<f32>(m, 0x18)).fmul_m(0.5).to_f32();
    let a3 = a.fmul_m(3.0).to_f32();
    let b3 = b.fmul_m(3.0);
    let c3 = x(c).fmul_m(3.0);
    let p = if m.is_null() { ped.byte_add(4).cast::<f32>() } else { m.byte_add(0x30).cast::<f32>() };
    let q = [
        x(a3).fadd_m(p.read()).to_f32(),
        b3.fadd_m(p.wrapping_index(1).read()).to_f32(),
        c3.fadd_m(p.wrapping_index(2).read()).to_f32(),
    ];
    let min_x = x(q[0]).fsub_m(3.0);
    let max_x = x(q[0]).fadd_m(3.0).to_f32();
    let min_y = x(q[1]).fsub_m(3.0).to_f32();
    let max_y = x(q[1]).fadd_m(3.0).to_f32();
    let (r, l) = sector_of(min_x);
    let x0 = if r > 0 { floor_ftol(l) } else { 0 };
    let (r, l) = sector_of(x(min_y));
    let y0 = if r > 0 { floor_ftol(l) } else { 0 };
    let (r, l) = sector_of(x(max_x));
    let x1 = if r < 0x77 { floor_ftol(l) } else { 0x77 };
    let (r, l) = sector_of(x(max_y));
    let y1 = if r < 0x77 { floor_ftol(l) } else { 0x77 };
    let mut code = g::<u16>(0xB7CD78).read();
    if code == 0xFFFF {
        world_clear_scan_codes();
        code = 1;
    } else {
        code += 1;
    }
    g::<u16>(0xB7CD78).write(code);
    let mut result = GPtr::NULL;
    let mut y = y0;
    while y <= y1 {
        let mut xs = x0;
        while xs <= x1 {
            let idx = ((y & 15) << 4) as usize + (xs & 15) as usize;
            let mut node = g::<GPtr<u8>>(0xB992C0 + idx * 0xC).read();
            while !node.is_null() {
                let e = rd::<GPtr<u8>>(node, 0);
                node = rd::<GPtr<u8>>(node, 4);
                let ep = ent_pos(e);
                let dx = x(ep.read()).fsub_m(q[0]).to_f32();
                let dy = x(ep.wrapping_index(1).read()).fsub_m(q[1]).to_f32();
                let dz = x(ep.wrapping_index(2).read()).fsub_m(q[2]).to_f32();
                if rd::<u32>(e, 0x140) & 0x2000 == 0 || rd::<u16>(e, 0x2C) == code {
                    continue;
                }
                let d2 = x(dz).fmul_m(dz).fadd(x(dy).fmul_m(dy)).fadd(x(dx).fmul_m(dx));
                if d2.fcom(x(4.5)).test_parity(5) {
                    continue;
                }
                let pm = rd::<GPtr<u8>>(ped, 0x14);
                let f = |o: isize| x(dz).fmul_m(rd::<f32>(pm, o + 8)).fadd(x(dy).fmul_m(rd::<f32>(pm, o + 4))).fadd(x(dx).fmul_m(rd::<f32>(pm, o)));
                let side = f(0);
                let side = if side.to_f64().is_sign_negative() { side.fchs() } else { side }.fmul_m(3.0);
                let fw0 = f(0x10);
                let mut fw = f(0x10);
                if !fw0.fcom(x(0.0)).test_parity(5) {
                    fw = fw.fsubr_m(10.0);
                }
                if !side.fadd(fw).fcom(x(1000.0)).test_parity(5) {
                    result = e;
                }
            }
            xs += 1;
        }
        y += 1;
    }
    result
}

/// The aimed-at reaction shared by `Compute3rdPersonMouseTarget` and `FindNextWeaponLockOnTarget`: in a group
/// (and not in this ped's group) a heap `CEventGunAimedAt(this)` in a stack `CEventGroupEvent(tgt, e)` added to
/// the group intelligence; without a group a stack `CEventGunAimedAt(this)` added to the target's event group.
fn gun_aimed_at(this: GPtr<u8>, tgt: GPtr<u8>) {
    let grp = ped_groups_get_peds_group(tgt);
    if !grp.is_null() {
        if ped_groups_are_in_same_group(tgt, this) != 0 {
            return;
        }
        let mut e = event_operator_new(0x18);
        if !e.is_null() {
            e = event_gun_aimed_at_ctor(e, this);
        }
        let mut ge = [0u32; 5];
        let gp: GPtr<u8> = lp(&mut ge).cast();
        event_group_event_ctor(gp, tgt, e);
        group_intel_add_event(grp.byte_add(0x30), gp);
        event_group_event_dtor(gp);
    } else {
        let mut ev = [0u32; 6];
        let evp: GPtr<u8> = lp(&mut ev).cast();
        event_gun_aimed_at_ctor(evp, this);
        event_group_add(rd::<GPtr<u8>>(tgt, 0x47C).byte_add(0x68), evp, 0);
        event_gun_aimed_at_dtor(evp);
    }
}

/// Replaces the reference at `this + off` (`CleanUpOldReference` on a non-null old entity, store,
/// `RegisterReference` on the new one).
fn set_reference(this: GPtr<u8>, off: isize, e: GPtr<u8>) {
    let slot = this.byte_add(off);
    let old = rd::<GPtr<u8>>(this, off);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(this, off, e);
    entity_register_reference(e, slot);
}

/// `CPlayerPed::Compute3rdPersonMouseTarget(bool useCamTargetVector)` (0x60B650, `ret 4`): see
/// `CPlayerPed.md`. The line of sight (peds only, pIgnoreEntity = this, bIncludeBikers) writes its hit into
/// the weapon-range local; the timeout clears +0x79C without CleanUpOldReference.
#[hook(addr = 0x60B650, conv = thiscall, name = "CPlayerPed::Compute3rdPersonMouseTarget", expect = "64 A1 00 00 00 00 6A FF", this = "CPlayerPed")]
pub fn compute_3rd_person_mouse_target(this: GPtr<u8>, use_cam: u32) {
    let mut tgt = GPtr::<u8>::NULL;
    if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 {
        let ty = rd::<i32>(active_weapon(this), 0);
        let skill = ped_get_weapon_skill_current(this);
        let range = rd::<f32>(get_weapon_info(ty, i32::from(skill)), 4);
        let pp = ent_pos(this);
        let p = vec3(pp);
        let mut s = [0f32; 3];
        let mut t = [0f32; 3];
        if use_cam as u8 != 0 {
            let q = vec3(pp);
            camera_find_3rd_person_cam_target_vector(g::<u8>(0xB6F028), range, q[0], q[1], q[2], lp(&mut s).cast(), lp(&mut t).cast());
        } else {
            let cam = usize::from(g::<u8>(0xB6F081).read()) * 0x238;
            s = vec3(g::<f32>(0xB6F338 + cam));
            let f = vec3(g::<f32>(0xB6F32C + cam));
            let dz = x(s[2]).fsub_m(p[2]);
            let dy = x(s[1]).fsub_m(p[1]);
            let dx = x(s[0]).fsub_m(p[0]);
            let d = x(f[1]).fmul(dy).fadd(x(f[2]).fmul(dz)).fadd(dx.fmul_m(f[0]));
            if !d.fcom(x(0.0)).test_parity(5) {
                let zd = x(f[2]).fmul(d);
                let yd = x(f[1]).fmul(d);
                let xd = d.fmul_m(f[0]);
                s[0] = x(s[0]).fsub(xd).to_f32();
                s[1] = x(s[1]).fsub(yd).to_f32();
                s[2] = x(s[2]).fsub(zd).to_f32();
            }
            let xr = x(f[0]).fmul_m(range);
            let yr = x(f[1]).fmul_m(range);
            let zr = x(f[2]).fmul_m(range).to_f32();
            t[0] = x(s[0]).fadd(xr).to_f32();
            t[1] = yr.fadd_m(s[1]).to_f32();
            t[2] = x(zr).fadd_m(s[2]).to_f32();
        }
        let mut hit = GPtr::<u8>::NULL;
        let mut cp = [0u32; 0xB];
        g::<GPtr<u8>>(0xB7CD68).write(this);
        g::<u8>(0xB7CD6F).write(1);
        let r = world_process_line_of_sight(lp(&mut s).cast(), lp(&mut t).cast(), lp(&mut cp).cast(), lp(&mut hit), 0, 0, 1, 0, 0, 0, 0, 0);
        if r != 0 && hit != this && ped_is_alive(hit) as u8 != 0 {
            tgt = hit;
        }
        world_reset_line_test_options();
    }
    if tgt.is_null() {
        if rd::<u32>(this, 0x79C) != 0 && rd::<u32>(this, 0x7A0) < g::<u32>(0xB7CB84).read() {
            wr::<u32>(this, 0x79C, 0);
        }
        return;
    }
    if tgt != rd::<GPtr<u8>>(this, 0x79C) {
        set_reference(this, 0x79C, tgt);
    }
    wr::<u32>(this, 0x7A0, g::<u32>(0xB7CB84).read().wrapping_add(1000));
    let ty = rd::<i32>(active_weapon(this), 0);
    if rd::<u32>(get_weapon_info(ty, 1), 0) == 0 {
        return;
    }
    let ty = rd::<i32>(active_weapon(this), 0);
    let skill = ped_get_weapon_skill_current(this);
    if rd::<u8>(get_weapon_info(ty, i32::from(skill)), 0x18) & 1 == 0 {
        return;
    }
    let intel = rd::<GPtr<u8>>(tgt, 0x47C);
    if intel_is_in_seeing_range(intel, ent_pos(this)) == 0 {
        return;
    }
    let intel = rd::<GPtr<u8>>(tgt, 0x47C);
    let mut task = rd::<GPtr<u8>>(intel, 4);
    if task.is_null() {
        task = rd::<GPtr<u8>>(intel, 8);
    }
    if task.is_null() {
        task = rd::<GPtr<u8>>(intel, 0xC);
    }
    if !task.is_null() && v_task_get_type(task) == 0x259 {
        return;
    }
    if rd::<i32>(active_weapon(this), 0) != 0x17 {
        ped_say(this, 0xB0, 0, 1.0, 0, 0, 0);
    }
    gun_aimed_at(this, tgt);
}

/// `TargetWeaponRangeMultiplier(victim, this) * range` (x87, f32 stored as the maxDist argument)
fn range_for(this: GPtr<u8>, victim: GPtr<u8>, range: f32) -> f32 {
    X87::from_f64_exact(target_weapon_range_multiplier(victim, this)).fmul_m(range).to_f32()
}

/// `CanIKReachThisTarget(e's position by value, &W[act], true)`
fn ik_reach(this: GPtr<u8>, e: GPtr<u8>) -> bool {
    let p = vec3(ent_pos(e));
    can_ik_reach_this_target(this, p[0], p[1], p[2], active_weapon(this), 1) != 0
}

/// The ped exclusions shared by the lock-on searches (state 54/55, in a vehicle other than a bike or one
/// with +0x42D bit 2, bNeverEverTargetThisPed, same group, players when `bPlayersCannotTargetEachOther`).
fn ped_lock_on_excluded(this: GPtr<u8>, p: GPtr<u8>) -> bool {
    let st = rd::<i32>(p, 0x530);
    if st == 0x36 || st == 0x37 {
        return true;
    }
    if rd::<u32>(p, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(p, 0x58C);
        if v.is_null() || (rd::<i32>(v, 0x590) != 9 && rd::<u8>(v, 0x42D) & 4 == 0) {
            return true;
        }
    }
    if rd::<u32>(p, 0x470) & 0x1000_0000 != 0 || ped_groups_are_in_same_group(p, this) != 0 {
        return true;
    }
    let pt = rd::<i32>(p, 0x598);
    (pt == 0 || pt == 1) && g::<u8>(0x96A8B2).read() != 0
}

/// `CPlayerPed::FindNextWeaponLockOnTarget(CEntity* curTarget, bool leftSide)` (0x60E530, `ret 8`, AL; entry
/// `push -1; jmp 0x4038AB`, a protection stub that pushes the SEH handler and returns to 0x60E537): see
/// `CPlayerPed.md`. The forward heading is computed and discarded; objects and vehicles use the stale EBP of
/// the ped loop as TargetWeaponRangeMultiplier's victim: the ped at slot 0 (0 when that slot is free, the
/// pool base otherwise, whatever the exclusions), or curTarget when the ped pool has size 0.
// Not registered (INF-20): the no-CD protection reads the patched entry bytes as data (`mov edi, [0x60E533]`
// at 0x401E55, `sbb ebx, [0x60E533]` at 0x406405, `adc ecx, [0x60E533]` at 0x1566A2D; sa-hook's install,
// keycells and patch_refs tests reject the hook).
pub fn find_next_weapon_lock_on_target(this: GPtr<u8>, cur: GPtr<u8>, left: u32) -> u8 {
    let ty = rd::<i32>(active_weapon(this), 0);
    let skill = ped_get_weapon_skill_current(this);
    let range = rd::<f32>(get_weapon_info(ty, i32::from(skill)), 4);
    let mut best = GPtr::<u8>::NULL;
    let mut best_score: f32 = -10000.0;
    let m = rd::<GPtr<u8>>(this, 0x14);
    get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14));
    let heading = if !cur.is_null() {
        let t = ent_pos(cur);
        let c = cam_pos();
        let ay = x(t.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read()).to_f32();
        let ax = x(t.read()).fsub_m(c.read()).to_f32();
        get_atan_of_xy(ax, ay)
    } else {
        get_atan_of_xy(g::<f32>(0xB6F9AC).read(), g::<f32>(0xB6F9B0).read())
    };
    let heading = X87::from_f64_exact(heading).to_f32();
    let pb = lp(&mut best);
    let ps = lp(&mut best_score);
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = pool_size(pool);
    let mut stale = cur;
    while i != 0 {
        let idx = (i - 1) as u32;
        i -= 1;
        if pool_flag(pool, idx) & 0x80 != 0 {
            stale = GPtr::NULL;
            continue;
        }
        let p = pool_slot(pool, idx, PED_SLOT);
        stale = p;
        if p.is_null() || p == this || p == cur || ped_lock_on_excluded(this, p) {
            continue;
        }
        if los_blocked_between_peds_call(this, p) != 0 || !ik_reach(this, p) {
            continue;
        }
        let md = range_for(this, p, range);
        evaluate_neighbouring_target_call(this, p, pb, ps, md, heading, left);
    }
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let mut i = pool_size(pool);
    while i != 0 {
        let idx = (i - 1) as u32;
        i -= 1;
        if pool_flag(pool, idx) & 0x80 != 0 {
            continue;
        }
        let o = pool_slot(pool, idx, 0x19C);
        if o.is_null() || object_can_be_targetted(o) == 0 || rd::<u8>(o, 0x140) & 0x40 != 0 || rd::<u32>(o, 0x18) == 0 || !ik_reach(this, o) {
            continue;
        }
        let md = range_for(this, stale, range);
        evaluate_neighbouring_target_call(this, o, pb, ps, md, heading, left);
    }
    if is_coop_game_going_on() != 0 {
        let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
        let mut i = pool_size(pool);
        while i != 0 {
            let idx = (i - 1) as u32;
            i -= 1;
            if pool_flag(pool, idx) & 0x80 != 0 {
                continue;
            }
            let v = pool_slot(pool, idx, VEHICLE_SLOT);
            if v.is_null() || rd::<u32>(v, 0x40) & 0x2000_0000 != 0 || rd::<i32>(v, 0x594) == 10 || !ik_reach(this, v) {
                continue;
            }
            let md = range_for(this, stale, range);
            evaluate_neighbouring_target_call(this, v, pb, ps, md, heading, left);
        }
    }
    let best = pb.read();
    if best.is_null() {
        return 0;
    }
    if ent_type(best) == 3 && rd::<u32>(get_weapon_info(rd::<i32>(active_weapon(this), 0), 1), 0) != 0 {
        gun_aimed_at(this, best);
    }
    set_reference(this, 0x71C, best);
    wr::<u8>(pdata(this), 0x85, 1);
    1
}

/// `|a|` as `fcom 0; test ah,5; jp; fchs` computes it (only a Less result is negated; NaN kept).
fn abs_lt(a: X87) -> X87 {
    if !a.fcom(x(0.0)).test_parity(5) { a.fchs() } else { a }
}

/// `CPlayerPed::FindWeaponLockOnTarget` (0x60DC50, AL): see `CPlayerPed.md`. Existing target: kept while
/// `TargetWeaponRangeMultiplier(t, this) * range >= |t - this|` (2D, f32 stored), else released (return
/// false; pd +0x85 untouched on both paths). Otherwise the heading H (forward, or the walk stick against the
/// camera angle 0xB6F178 when either axis exceeds 60), the spraycan tag search (raw FPATAN, wrapped once,
/// signed `ang < best` or the first candidate), then peds (view angle 140 * 0.008726646 from the ped, 90 *
/// 0.008726646 from 3 units behind it, Magnitude below range * multiplier) with EvaluateTarget, objects and
/// (co-op) vehicles; the best target becomes m_pTargetedObject with pd +0x85 = 1.
/// Precision note: the spraycan path's FPATAN results are modelled in f64 (`X87::fpatan`).
#[hook(addr = 0x60DC50, conv = thiscall, name = "CPlayerPed::FindWeaponLockOnTarget", expect = "81 EC 94 00 00 00", this = "CPlayerPed")]
pub fn find_weapon_lock_on_target(this: GPtr<u8>) -> u8 {
    let ty = rd::<i32>(active_weapon(this), 0);
    let skill = ped_get_weapon_skill_current(this);
    let info = get_weapon_info(ty, i32::from(skill));
    let range = rd::<f32>(info, 4);
    let t = rd::<GPtr<u8>>(this, 0x71C);
    if !t.is_null() {
        let tp = ent_pos(t);
        let sp = ent_pos(this);
        let dx = x(tp.read()).fsub_m(sp.read());
        let dy = x(tp.wrapping_index(1).read()).fsub_m(sp.wrapping_index(1).read());
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let m = X87::from_f64_exact(target_weapon_range_multiplier(t, this)).fmul_m(range);
        if !m.fcom(x(d)).test_any(1) {
            return 1;
        }
        let old = rd::<GPtr<u8>>(this, 0x71C);
        if !old.is_null() {
            entity_clean_up_old_reference(old, this.byte_add(0x71C));
        }
        wr::<u32>(this, 0x71C, 0);
        return 0;
    }
    let mut best = GPtr::<u8>::NULL;
    let mut best_score: f32 = -10000.0;
    let pb = lp(&mut best);
    let ps = lp(&mut best_score);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let mut h = X87::from_f64_exact(get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14))).to_f32();
    let pt = rd::<i32>(this, 0x598);
    let pad = if pt == 0 { get_pad(0) } else if pt == 1 { get_pad(1) } else { GPtr::NULL };
    let big = |v: i16| !abs_lt(xi(i32::from(v))).fcom(x(60.0)).test_any(0x41);
    if big(pad_get_ped_walk_left_right(pad)) || big(pad_get_ped_walk_up_down(pad)) {
        let ud = xi(i32::from(pad_get_ped_walk_up_down(pad))).to_f32();
        let lr = xi(-i32::from(pad_get_ped_walk_left_right(pad))).to_f32();
        let a = X87::from_f64_exact(get_radian_angle_between_points(0.0, 0.0, lr, ud));
        let a = a.fsub_m(g::<f32>(0xB6F178).read()).fadd_m(f32::from_bits(0x3FC9_0FDB)).to_f32();
        h = X87::from_f64_exact(limit_radian_angle(a)).to_f32();
    }
    if rd::<i32>(active_weapon(this), 0) == 0x29 {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let a = if !m.is_null() {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))).to_f32()
        } else {
            rd::<f32>(this, 0x10)
        };
        let mut best_ang = PI;
        let fx8 = x(rd::<f32>(m, 0x10)).fmul_m(8.0).to_f32();
        let fy8 = x(rd::<f32>(m, 0x14)).fmul_m(8.0).to_f32();
        let fz8 = x(rd::<f32>(m, 0x18)).fmul_m(8.0);
        let pp = ent_pos(this);
        let pz = fz8.fadd_m(pp.wrapping_index(2).read());
        let py = x(fy8).fadd_m(pp.wrapping_index(1).read());
        let px = x(fx8).fadd_m(pp.read());
        let mut p = [px.to_f32(), py.to_f32(), pz.to_f32()];
        let mut n: i16 = 0;
        let mut list = [GPtr::<u8>::NULL; 15];
        world_find_objects_in_range(lp(&mut p).cast(), 8.0, 0, lp(&mut n), 15, lp(&mut list).cast(), 1, 0, 0, 0, 0);
        let mut tag = GPtr::<u8>::NULL;
        let mut k = 0i32;
        while k < i32::from(n) {
            let e = list[k as usize];
            k += 1;
            if tag_is_tag(e) == 0 || tag_get_alpha(e) == 0xFF {
                continue;
            }
            let sp = ent_pos(this);
            let ep = ent_pos(e);
            let dx = x(ep.read()).fsub_m(sp.read());
            let dy = x(ep.wrapping_index(1).read()).fsub_m(sp.wrapping_index(1).read());
            let dz = x(ep.wrapping_index(2).read()).fsub_m(sp.wrapping_index(2).read());
            let s = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
            if x(range).fmul_m(range).fcom(s).test_any(0x41) {
                continue;
            }
            let mut ang = dx.fchs().fpatan(dy).fsub_m(a);
            if !ang.fcom(x(MINUS_PI)).test_parity(5) {
                ang = ang.fadd_m(TWO_PI);
            } else if !ang.fcom(x(PI)).test_any(0x41) {
                ang = ang.fsub_m(TWO_PI);
            }
            if !ang.fcom(x(best_ang)).test_parity(5) || tag.is_null() {
                best_ang = ang.to_f32();
                tag = e;
            }
        }
        if !tag.is_null() && los_blocked_between_peds_call(this, tag) == 0 {
            set_reference(this, 0x71C, tag);
            wr::<u8>(pdata(this), 0x85, 1);
            return 1;
        }
    }
    let view = |lim: usize, a: X87| -> bool {
        let a = abs_lt(wrap_pi(a));
        !a.fcom(x(g::<f32>(lim).read()).fmul_m(f32::from_bits(0x3C0E_FA35))).test_parity(5)
    };
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = pool_size(pool);
    while i != 0 {
        let idx = (i - 1) as u32;
        i -= 1;
        if pool_flag(pool, idx) & 0x80 != 0 {
            continue;
        }
        let p = pool_slot(pool, idx, PED_SLOT);
        if p.is_null() || p == this || ped_lock_on_excluded(this, p) {
            continue;
        }
        let pp = ent_pos(p);
        let sp = ent_pos(this);
        let ay = x(pp.wrapping_index(1).read()).fsub_m(sp.wrapping_index(1).read()).to_f32();
        let ax = x(pp.read()).fsub_m(sp.read()).to_f32();
        if !view(PLAYER_MAX_TARGET_VIEW_ANGLE, X87::from_f64_exact(get_atan_of_xy(ax, ay)).fsub_m(h)) {
            continue;
        }
        let mut nf = vec3(rd::<GPtr<u8>>(this, 0x14).byte_add(0x10).cast());
        vector_normalise(lp(&mut nf).cast());
        let mut tmp = [0f32; 3];
        let mut q = [0f32; 3];
        let r = vector_mul_scalar(lp(&mut tmp).cast(), lp(&mut nf).cast(), g::<f32>(0x8D2440).read());
        vector_sub(lp(&mut q).cast(), ent_pos(this), r);
        let pp = ent_pos(p);
        let ay = x(pp.wrapping_index(1).read()).fsub_m(q[1]).to_f32();
        let ax = x(pp.read()).fsub_m(q[0]).to_f32();
        if !view(0x8D2438, X87::from_f64_exact(get_atan_of_xy(ax, ay)).fsub_m(h)) {
            continue;
        }
        let mut dv = [0f32; 3];
        let dvp = vector_sub(lp(&mut dv).cast(), ent_pos(p), ent_pos(this));
        let d = X87::from_f64_exact(vector_magnitude(dvp)).to_f32();
        let r = rd::<f32>(info, 4);
        let md = X87::from_f64_exact(target_weapon_range_multiplier(p, this)).fmul_m(r).to_f32();
        if !x(d).fcom(x(md)).test_parity(5) {
            evaluate_target_call(this, p, pb, ps, md, h, 0);
        }
    }
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let mut i = pool_size(pool);
    while i != 0 {
        let idx = (i - 1) as u32;
        i -= 1;
        if pool_flag(pool, idx) & 0x80 != 0 {
            continue;
        }
        let o = pool_slot(pool, idx, 0x19C);
        if o.is_null() || object_can_be_targetted(o) == 0 || rd::<u8>(o, 0x140) & 0x40 != 0 || rd::<u32>(o, 0x18) == 0 || !ik_reach(this, o) {
            continue;
        }
        evaluate_target_call(this, o, pb, ps, rd::<f32>(info, 4), h, 1);
    }
    if is_coop_game_going_on() != 0 {
        let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
        let mut i = pool_size(pool);
        while i != 0 {
            let idx = (i - 1) as u32;
            i -= 1;
            if pool_flag(pool, idx) & 0x80 != 0 {
                continue;
            }
            let v = pool_slot(pool, idx, VEHICLE_SLOT);
            if v.is_null() || rd::<u32>(v, 0x40) & 0x2000_0000 != 0 || rd::<i32>(v, 0x594) == 10 || !ik_reach(this, v) {
                continue;
            }
            evaluate_target_call(this, v, pb, ps, rd::<f32>(info, 4), h, 1);
        }
    }
    let best = pb.read();
    if best.is_null() {
        return 0;
    }
    set_reference(this, 0x71C, best);
    wr::<u8>(pdata(this), 0x85, 1);
    1
}

/// `TempVertexBuffer` (RxObjSpace3DVertex, 0x24 each: +0 position, +0x18 colour) and `aTempBufferIndices`
const TEMP_VERTEX_BUFFER: usize = 0xC4D958;
const TEMP_BUFFER_INDICES: usize = 0xC4B958;

/// `V += normalise(camPos - V)` as DrawTriangleForMouseRecruitPed does it per vertex (D stored f32, CVector::
/// Normalise, `D.k + V.k`); the camera position is re-read each time.
fn pull_toward_camera(v: &mut [f32; 3]) {
    let c = cam_pos();
    let mut d = [
        x(c.read()).fsub_m(v[0]).to_f32(),
        x(c.wrapping_index(1).read()).fsub_m(v[1]).to_f32(),
        x(c.wrapping_index(2).read()).fsub_m(v[2]).to_f32(),
    ];
    vector_normalise(lp(&mut d).cast());
    for k in 0..3 {
        v[k] = x(d[k]).fadd_m(v[k]).to_f32();
    }
}

/// `CPlayerPed::DrawTriangleForMouseRecruitPed` (0x60BA80): see `CPlayerPed.md`. Colour bytes are the low bytes
/// of `_ftol2` of `inv*255 + r*0`, `r*255 + inv*0`, `inv*0 + r*0` (r = health / max health, clamped to 1;
/// all 0 unless r > 0).
#[hook(addr = 0x60BA80, conv = thiscall, name = "CPlayerPed::DrawTriangleForMouseRecruitPed", expect = "83 EC 58 57 8B F9", this = "CPlayerPed")]
pub fn draw_triangle_for_mouse_recruit_ped(this: GPtr<u8>) {
    if rd::<u32>(this, 0x79C) == 0 {
        return;
    }
    for (s, v) in [(8, 0), (6, 1), (10, 5), (11, 6), (12, 1), (1, 0), (29, 8)] {
        rw_render_state_set(s, v);
    }
    let t = rd::<GPtr<u8>>(this, 0x79C);
    let mut r = x(rd::<f32>(t, 0x540)).fdiv_m(rd::<f32>(t, 0x544));
    let (cr, cg, cb);
    let positive = if !r.fcom(x(1.0)).test_any(0x41) {
        r = x(1.0);
        true
    } else {
        !r.fcom(x(0.0)).test_any(0x41)
    };
    if positive {
        let inv = x(1.0).fsub(r);
        let r0 = r.fmul_m(0.0);
        cr = ftol32(inv.fmul_m(255.0).fadd(r0)) as u8;
        let inv0 = inv.fmul_m(0.0);
        cg = ftol32(r.fmul_m(255.0).fadd(inv0)) as u8;
        cb = ftol32(inv0.fadd(r0)) as u8;
    } else {
        (cr, cg, cb) = (0, 0, 0);
    }
    let mut d = dist3(ent_pos(t), ent_pos(this)).fsub_m(10.0);
    if d.fcom(x(0.0)).test_any(0x41) {
        d = x(0.0);
    }
    d = d.fmul_m(f32::from_bits(0x3CA3_D70A));
    if !x(1.0).fcom(d).test_parity(5) {
        d = x(1.0);
    }
    let s = d.fmul_m(f32::from_bits(0x3F53_3333)).fadd_m(f32::from_bits(0x3E33_3333));
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let (rx, ry, rz) = if !cm.is_null() {
        (x(rd::<f32>(cm, 0)), x(rd::<f32>(cm, 4)), rd::<f32>(cm, 8))
    } else {
        let h = g::<f32>(0xB6F038).read();
        (x(h).fcos(), x(h).fsin(), 0.0)
    };
    let rv = [rx.fmul(s).to_f32(), ry.fmul(s).to_f32(), x(rz).fmul(s).to_f32()];
    let u = x(0.0).fmul(s).to_f32();
    let p = vec3(ent_pos(t));
    let pz1 = x(p[2]).fadd_m(1.0).to_f32();
    let (mut v0, mut v1, mut v2);
    if rd::<i32>(t, 0x598) == 7 {
        v0 = [p[0], p[1], pz1];
        let a = [x(p[0]).fsub_m(rv[0]).to_f32(), x(p[1]).fsub_m(rv[1]).to_f32(), x(pz1).fsub_m(rv[2]).to_f32()];
        v1 = [x(a[0]).fadd_m(u).to_f32(), x(u).fadd_m(a[1]).to_f32(), x(a[2]).fadd(s).to_f32()];
        let b = [x(p[0]).fadd_m(rv[0]).to_f32(), x(p[1]).fadd_m(rv[1]).to_f32(), x(pz1).fadd_m(rv[2]).to_f32()];
        v2 = [x(b[0]).fadd_m(u).to_f32(), x(u).fadd_m(b[1]).to_f32(), s.fadd_m(b[2]).to_f32()];
    } else {
        let (px, py) = (x(p[0]), x(p[1]));
        v0 = [px.fadd_m(u).to_f32(), x(u).fadd(py).to_f32(), x(pz1).fadd(s).to_f32()];
        v1 = [px.fadd_m(rv[0]).to_f32(), x(rv[1]).fadd(py).to_f32(), x(pz1).fadd_m(rv[2]).to_f32()];
        v2 = [px.fsub_m(rv[0]).to_f32(), py.fsub_m(rv[1]).to_f32(), x(pz1).fsub_m(rv[2]).to_f32()];
    }
    pull_toward_camera(&mut v0);
    pull_toward_camera(&mut v1);
    pull_toward_camera(&mut v2);
    let vb = g::<u8>(TEMP_VERTEX_BUFFER);
    let rgb = (u32::from(cr) << 16) | (u32::from(cg) << 8) | u32::from(cb);
    for (k, v) in [v0, v1, v2].iter().enumerate() {
        for (j, c) in v.iter().enumerate() {
            wr::<f32>(vb, (k * 0x24 + j * 4) as isize, *c);
        }
    }
    wr::<u32>(vb, 0x18, 0xFF00_0000 | rgb);
    wr::<u32>(vb, 0x24 + 0x18, rgb);
    wr::<u32>(vb, 0x48 + 0x18, rgb);
    let ib = g::<u16>(TEMP_BUFFER_INDICES);
    for k in 0..3 {
        ib.wrapping_index(k).write(k as u16);
    }
    if rw_im3d_transform(TEMP_VERTEX_BUFFER as u32, 3, 0, 0x18) != 0 {
        rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES as u32, 3);
        rw_im3d_end();
    }
    for (s, v) in [(1, 0), (8, 1), (6, 1), (10, 5), (11, 6), (12, 0), (29, 5)] {
        rw_render_state_set(s, v);
    }
}

/// Assoc fields used by SetRealMoveAnim: +0x14 hierarchy (+0x10 total time), +0x18 blend amount, +0x1C blend
/// delta, +0x20 current time, +0x24 speed, +0x28 time step, +0x2E flags.
fn amt(a: GPtr<u8>) -> X87 {
    x(rd::<f32>(a, 0x18))
}
fn dlt(a: GPtr<u8>) -> X87 {
    x(rd::<f32>(a, 0x1C))
}
fn or_flags(a: GPtr<u8>, b: u8) {
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | b);
}
fn clear_playing(a: GPtr<u8>) {
    wr::<u16>(a, 0x2E, rd::<u16>(a, 0x2E) & 0xFFFE);
}
fn total_time(a: GPtr<u8>) -> f32 {
    rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)
}

/// `CPlayerPed::SetRealMoveAnim` (0x60A9C0): see `CPlayerPed.md` (steps 1-6). The original's exit chunk
/// (0x60B399 -> 0x409B28 -> 0x60B39F) reads the SPRINT association from EDX, which GetButtonSprintResults
/// preserves; this port keeps the SPRINT local itself (the value EDX holds on every path to the tail).
#[hook(addr = 0x60A9C0, conv = thiscall, name = "CPlayerPed::SetRealMoveAnim", expect = "83 EC 20 53 55 56 57", this = "CPlayerPed")]
pub fn set_real_move_anim(this: GPtr<u8>) {
    let clump = || rd::<GPtr<u8>>(this, 0x18);
    let mut walk = clump_get_association(clump(), 0);
    let mut run = clump_get_association(clump(), 1);
    let mut sprint = clump_get_association(clump(), 2);
    let mut walk_start = clump_get_association(clump(), 5);
    let idle = clump_get_association(clump(), 3);
    let run_stop = clump_get_association(clump(), 6);
    let run_stopr = clump_get_association(clump(), 7);
    let turn_l = clump_get_association(clump(), 0x89);
    let turn_r = clump_get_association(clump(), 0x8A);
    let idle_tired = clump_get_association(clump(), 10);
    let group = || rd::<u32>(this, 0x4D4);
    let set_ms = |v: i32| wr::<i32>(this, 0x534, v);
    let ratio = || x(rd::<f32>(pdata(this), 0x14));
    let playing = |a: GPtr<u8>| !a.is_null() && rd::<u8>(a, 0x2E) & 1 != 0;
    if rd::<u8>(this, 0x470) & 1 != 0 {
        for a in [walk, run, sprint] {
            if !a.is_null() {
                assoc_set_current_time(a, 0.0);
            }
        }
        wr::<u32>(this, 0x470, rd::<u32>(this, 0x470) & 0xFFFF_FFFE);
    }
    'body: {
        // 2. stopping
        if playing(run_stop) || playing(run_stopr) {
            set_ms(6);
            if !run_stop.is_null() && rd::<u8>(run_stop, 0x2E) & 1 == 0 && !x(total_time(run_stop)).fcom(x(rd::<f32>(run_stop, 0x20))).test_any(0x41) {
                or_flags(run_stop, 1);
            }
            break 'body;
        }
        // 3. stop anim fading in
        let nonneg = |a: GPtr<u8>| !a.is_null() && !dlt(a).fcom(x(0.0)).test_any(1);
        if nonneg(run_stop) || nonneg(run_stopr) {
            let a = if !run_stop.is_null() { run_stop } else { run_stopr };
            if !a.is_null() {
                or_flags(a, 4);
                wr::<f32>(a, 0x18, 1.0);
                wr::<f32>(a, 0x1C, -8.0);
            }
            ped_restore_heading_rate(this);
            let i = if idle.is_null() { blend_animation(clump(), group(), 3, 8.0) } else { idle };
            wr::<f32>(i, 0x18, 0.0);
            wr::<f32>(i, 0x1C, 8.0);
            break 'body;
        }
        // 4. standing
        if ratio().fcom(x(0.0)) == sa_math::x87::Cmp::Equal && sprint.is_null() {
            let cam_mode = || rd::<u16>(g::<u8>(0xB6F1A8 + usize::from(g::<u8>(0xB6F081).read()) * 0x238), 0);
            if pad_get_force_camera_behind_player(get_pad_from_player(this)) != 0
                && pad_aim_weapon_left_right(get_pad_from_player(this), 0) != 0
                && cam_mode() == 4
            {
                let v = pad_aim_weapon_left_right(get_pad_from_player(this), 0);
                let (cur, id) = if !xi(i32::from(v)).fcom(X87::from_f64_exact(0.0)).test_parity(5) { (turn_l, 0x89) } else { (turn_r, 0x8A) };
                let mut t = cur;
                let reblend = t.is_null()
                    || !dlt(t).fcom(x(0.0)).test_parity(5)
                    || (!amt(t).fcom(x(1.0)).test_parity(5) && !dlt(t).fcom(x(0.0)).test_parity(0x41));
                if reblend {
                    t = blend_animation(clump(), 0, id, 16.0);
                }
                let v = pad_aim_weapon_left_right(get_pad_from_player(this), 0);
                let s = abs_lt(xi(i32::from(v))).fmul_m(g::<f32>(0x8D24EC).read()).fmul_m(0.0078125);
                wr::<f32>(t, 0x24, s.to_f32());
                if !idle.is_null() && !amt(idle).fcom(x(f32::from_bits(0x3C23_D70A))).test_parity(0x41) {
                    v_assoc_delete(idle, 1);
                }
            } else if idle.is_null() {
                blend_animation(clump(), group(), 3, 4.0);
            }
            let intel = || rd::<GPtr<u8>>(this, 0x47C);
            let tired = !x(rd::<f32>(pdata(this), 0x18)).fcom(x(0.0)).test_parity(5)
                && intel_get_task_fighting(intel()).is_null()
                && intel_get_task_use_gun(intel()).is_null()
                && intel_get_task_duck(intel(), 1).is_null()
                && intel_get_task_throw(intel()).is_null()
                && intel_get_task_jet_pack(intel()).is_null()
                && {
                    let p = vec3(ent_pos(this));
                    world_test_sphere_against_world(p[0], p[1], p[2], 0.5, 0, 1, 0, 0, 0, 0, 0).is_null()
                };
            if tired {
                if idle_tired.is_null() {
                    let g = if get_default_player_motion_group() == 0x37 { 0x4C } else { 0 };
                    let a = blend_animation(clump(), g, 10, 4.0);
                    or_flags(a, 1);
                }
            } else if !idle_tired.is_null() && !amt(idle_tired).fcom(x(0.0)).test_any(0x41) && !dlt(idle_tired).fcom(x(0.0)).test_any(1) {
                wr::<u8>(idle_tired, 0x2E, rd::<u8>(idle_tired, 0x2E) & 0xFE);
                wr::<f32>(idle_tired, 0x1C, -2.0);
            }
            set_ms(1);
            break 'body;
        }
        // 5. moving
        if !idle.is_null() {
            if walk_start.is_null() {
                walk_start = add_animation(clump(), group(), 5);
            } else {
                wr::<f32>(walk_start, 0x18, 1.0);
                wr::<u32>(walk_start, 0x1C, 0);
            }
            if !walk.is_null() {
                assoc_set_current_time(walk, 0.0);
            }
            if !run.is_null() {
                assoc_set_current_time(run, 0.0);
            }
            v_assoc_delete(idle, 1);
            if !idle_tired.is_null() {
                wr::<f32>(idle_tired, 0x1C, -4.0);
            }
            if !sprint.is_null() {
                v_assoc_delete(sprint, 1);
            }
            sprint = GPtr::NULL;
            set_ms(4);
        }
        if !run_stop.is_null() {
            v_assoc_delete(run_stop, 1);
            ped_restore_heading_rate(this);
        }
        if !run_stopr.is_null() {
            v_assoc_delete(run_stopr, 1);
            ped_restore_heading_rate(this);
        }
        if !turn_l.is_null() {
            v_assoc_delete(turn_l, 1);
        }
        if !turn_r.is_null() {
            v_assoc_delete(turn_r, 1);
        }
        if walk.is_null() {
            walk = add_animation(clump(), group(), 0);
            wr::<u32>(walk, 0x18, 0);
        }
        if run.is_null() {
            run = add_animation(clump(), group(), 1);
            wr::<u32>(run, 0x18, 0);
        }
        if !walk_start.is_null()
            && (rd::<u8>(walk_start, 0x2E) & 1 == 0
                || !x(total_time(walk_start)).fcom(x(rd::<f32>(walk_start, 0x28)).fadd_m(rd::<f32>(walk_start, 0x20))).test_parity(0x41))
        {
            v_assoc_delete(walk_start, 1);
            wr::<u16>(walk, 0x2E, rd::<u16>(walk, 0x2E) | 1);
            wr::<u16>(run, 0x2E, rd::<u16>(run, 0x2E) | 1);
            walk_start = GPtr::NULL;
        }
        if rd::<i32>(this, 0x534) == 7 && !walk_start.is_null() {
            set_ms(1);
        }
        let ms7 = || rd::<i32>(this, 0x534) == 7;
        if !sprint.is_null() && (!ms7() || !ratio().fcom(x(0.4)).test_parity(5)) {
            if amt(sprint).fcom(x(0.0)) == sa_math::x87::Cmp::Equal {
                or_flags(sprint, 4);
                wr::<f32>(sprint, 0x1C, -1000.0);
            } else if !dlt(sprint).fcom(x(0.0)).test_parity(5) && !amt(sprint).fcom(x(0.8)).test_parity(5) {
                if !ratio().fcom(x(1.0)).test_parity(5) {
                    wr::<f32>(sprint, 0x1C, -8.0);
                    wr::<f32>(run, 0x1C, 8.0);
                }
            } else if !ratio().fcom(x(0.4)).test_parity(5) {
                let half = !x(rd::<f32>(sprint, 0x20)).fdiv_m(total_time(sprint)).fcom(X87::from_f64_exact(0.5)).test_parity(5);
                let a = add_animation(clump(), 0, if half { 6 } else { 7 });
                wr::<f32>(a, 0x18, 1.0);
                assoc_set_delete_callback(a, 0x5DFD70, this);
                wr::<u32>(this, 0x560, 0);
                or_flags(sprint, 4);
                wr::<f32>(sprint, 0x1C, -1000.0);
                clear_playing(walk);
                clear_playing(run);
                wr::<u32>(walk, 0x18, 0);
                wr::<u32>(run, 0x18, 0);
                wr::<u32>(walk, 0x1C, 0);
                wr::<u32>(run, 0x1C, 0);
            } else if !dlt(sprint).fcom(x(0.0)).test_any(1) {
                or_flags(sprint, 4);
                wr::<f32>(sprint, 0x1C, -1.0);
                wr::<f32>(run, 0x1C, 1.0);
            }
            set_ms(if !ratio().fcom(x(1.0)).test_any(0x41) { 6 } else { 4 });
            break 'body;
        }
        if !walk_start.is_null() {
            clear_playing(walk);
            clear_playing(run);
            wr::<u32>(walk, 0x18, 0);
            wr::<u32>(run, 0x18, 0);
            break 'body;
        }
        if ms7() {
            if sprint.is_null() {
                if !amt(run).fcom(x(1.0)).test_parity(5) {
                    if amt(walk).fcom(x(0.0)) == sa_math::x87::Cmp::Equal && amt(run).fcom(x(0.0)) == sa_math::x87::Cmp::Equal {
                        wr::<f32>(walk, 0x18, 1.0);
                    }
                    if !dlt(run).fcom(x(0.0)).test_parity(0x41) {
                        run = blend_animation(clump(), group(), 1, 4.0);
                    }
                    wr::<f32>(pdata(this), 0x14, dlt(run).fadd_m(1.0).to_f32());
                    break 'body;
                }
                sprint = blend_animation(clump(), group(), 2, 2.0);
            } else if !dlt(sprint).fcom(x(0.0)).test_parity(5) {
                wr::<f32>(sprint, 0x1C, 2.0);
                wr::<f32>(run, 0x1C, -2.0);
            }
            if !sprint.is_null() {
                stats_update_when_sprinting();
            }
            break 'body;
        }
        if !ratio().fcom(x(1.0)).test_parity(5) {
            wr::<f32>(walk, 0x18, 1.0);
            wr::<u32>(run, 0x18, 0);
            wr::<u32>(walk, 0x1C, 0);
            wr::<u32>(run, 0x1C, 0);
            set_ms(4);
        } else if !ratio().fcom(x(2.0)).test_parity(5) {
            wr::<f32>(walk, 0x18, x(2.0).fsub_m(rd::<f32>(pdata(this), 0x14)).to_f32());
            wr::<f32>(run, 0x18, ratio().fsub_m(1.0).to_f32());
            wr::<u32>(walk, 0x1C, 0);
            wr::<u32>(run, 0x1C, 0);
            set_ms(6);
        } else {
            wr::<u32>(walk, 0x18, 0);
            wr::<f32>(run, 0x18, 1.0);
            wr::<u32>(walk, 0x1C, 0);
            wr::<u32>(run, 0x1C, 0);
            set_ms(6);
            stats_update_when_running();
        }
    }
    // 6. exit tail (0x409B28 / 0x60B39F)
    let pd = pdata(this);
    if rd::<u32>(pd, 0x34) & 2 != 0 {
        let s: f32 = if g::<u32>(0xB7CB84).read() > rd::<u32>(pd, 0x3C) && g::<u8>(0x969177).read() == 0 {
            wr::<u32>(pd, 0x34, rd::<u32>(pd, 0x34) & 0xFFFF_FFFD);
            g::<f32>(0xB7CB64).write(1.0);
            1.0
        } else {
            g::<u32>(0xB7CB64).write(0x3EAA_AAAB);
            2.0
        };
        for a in [walk_start, walk, run] {
            if !a.is_null() {
                wr::<f32>(a, 0x24, s);
            }
        }
        if sprint.is_null() {
            return;
        }
        wr::<f32>(sprint, 0x24, s);
    }
    if sprint.is_null() {
        return;
    }
    let cam = usize::from(g::<u8>(0xB6F081).read()) * 0x238;
    if g::<u16>(0xB6F1A8 + cam).read() == 0xF {
        wr::<u32>(sprint, 0x24, 0x3F33_3333);
        return;
    }
    let r = X87::from_f64_exact(get_button_sprint_results(this, 0));
    let r = if !x(1.0).fcom(r).test_any(0x41) { x(1.0) } else { r };
    wr::<f32>(sprint, 0x24, r.to_f32());
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DISBAND_PLAYER_GROUP,
    HOOK_MAKE_GROUP_RESPOND_TO_PLAYER_TAKING_DAMAGE,
    HOOK_PLAYER_WANTS_TO_ATTACK,
    HOOK_LOS_BLOCKED_BETWEEN_PEDS,
    HOOK_DOES_TARGET_HAVE_TO_BE_BROKEN,
    HOOK_EVALUATE_NEIGHBOURING_TARGET,
    HOOK_EVALUATE_TARGET,
    HOOK_FIND_PED_TO_ATTACK,
    HOOK_PROCESS_GROUP_BEHAVIOUR,
    HOOK_TELL_GROUP_TO_START_FOLLOWING_PLAYER,
    HOOK_KEEP_AREA_AROUND_PLAYER_CLEAR,
    HOOK_MAKE_THIS_PED_JOIN_OUR_GROUP,
    HOOK_FIND_CLOSEST_CAR_SECTOR_LIST,
    HOOK_FIND_OBJECT_TO_STEAL,
    HOOK_COMPUTE_3RD_PERSON_MOUSE_TARGET,
    HOOK_FIND_WEAPON_LOCK_ON_TARGET,
    HOOK_DRAW_TRIANGLE_FOR_MOUSE_RECRUIT_PED,
    HOOK_SET_REAL_MOVE_ANIM,
];
