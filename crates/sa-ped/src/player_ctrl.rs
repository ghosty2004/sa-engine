//! `CPlayerPed` / `CPlayerInfo` control functions (PORT-PED-2, `docs/arch/ped-port-2.md`). Ported from the
//! 1.0 US bytes (gta-reversed `PlayerPed.cpp` / `PlayerInfo.cpp` and `docs/re/specs/PED/CPlayerPed*.md`,
//! `CPlayerInfo*.md` as reading aids).
//!
//! `CPlayerInfo` fields used here (0x190): +0 ped, +4 `CPlayerPedData` (inline; +4 = its wanted pointer,
//! +0x1C = its sprint energy), +0xB0 remote vehicle, +0xB4 special collectable, +0x144 hours without
//! eating (i16), +0x188 parachute requested, +0x18C parachute timer. `CPlayerPed`: +0x480 player data.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CCamera TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `gFireManager`
const FIRE_MANAGER: usize = 0xB71F80;
/// `TheText`
const THE_TEXT: usize = 0xC1B340;
/// `CClock::ms_nGameClockHours`
const CLOCK_HOURS: usize = 0xB70153;
/// `MODEL_GUN_PARA` (0x173)
const MI_PARACHUTE: i32 = 0x173;

orig! {
    /// `CPlayerPed::GetPadFromPlayer` (0x609560)
    pub fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => crate::player_ped::get_pad_from_player;
    /// `CPlayerPed::ClearAdrenaline` (0x60A070)
    pub fn clear_adrenaline(p: GPtr<u8>) = thiscall 0x60A070 => crate::player_ped::clear_adrenaline;
    /// `CPlayerPed::ClearWeaponTarget` (0x609C80)
    pub fn clear_weapon_target(p: GPtr<u8>) = thiscall 0x609C80 => crate::player_ped::clear_weapon_target;
    /// `CPlayerInfo::GetPos_Hook(CVector*)` (0x56DFB0)
    pub fn player_info_get_pos(pi: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> = thiscall 0x56DFB0 => crate::player_info::get_pos;
    /// `CWorld::StopAllLawEnforcersInTheirTracks` (0x566C10)
    pub fn stop_all_law_enforcers_in_their_tracks() = cdecl 0x566C10 => unported;
    /// `CPad::StopPadsShaking` (0x541D70)
    pub fn stop_pads_shaking() = cdecl 0x541D70 => unported;
    /// `CPedIntelligence::ClearTasks(bool primary, bool secondary)` (0x601420)
    pub fn intel_clear_tasks(intel: GPtr<u8>, primary: u32, secondary: u32) = thiscall 0x601420 => unported;
    /// `CFireManager::ExtinguishPoint(CVector, float)` (0x539450)
    pub fn fire_extinguish_point(fm: GPtr<u8>, x: f32, y: f32, z: f32, r: f32) = thiscall 0x539450 => unported;
    /// `CExplosion::RemoveAllExplosionsInArea(CVector, float)` (0x7369E0)
    pub fn remove_all_explosions_in_area(x: f32, y: f32, z: f32, r: f32) = cdecl 0x7369E0 => unported;
    /// `CProjectileInfo::RemoveAllProjectiles` (0x7399B0)
    pub fn remove_all_projectiles() = cdecl 0x7399B0 => unported;
    /// `CWorld::SetAllCarsCanBeDamaged(bool)` (0x5668F0)
    pub fn set_all_cars_can_be_damaged(b: u32) = cdecl 0x5668F0 => unported;
    /// `CWorld::ExtinguishAllCarFiresInArea(CVector, float)` (0x566950)
    pub fn extinguish_all_car_fires_in_area(x: f32, y: f32, z: f32, r: f32) = cdecl 0x566950 => unported;
    /// `CReplay::DisableReplays` (0x45B150)
    pub fn disable_replays() = cdecl 0x45B150 => unported;
    /// `CReplay::EnableReplays` (0x45B160)
    pub fn enable_replays() = cdecl 0x45B160 => unported;
    /// `CGameLogic::IsCoopGameGoingOn` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    pub fn set_model_is_deletable(model: i32) = cdecl 0x409C10 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    pub fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPad::StartShake(int16 time, uint8 freq, uint32 unk)` (0x53F920)
    pub fn pad_start_shake(pad: GPtr<u8>, time: u32, freq: u32, unk: u32) = thiscall 0x53F920 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    pub fn text_get(t: GPtr<u8>, key: u32) -> u32 = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(text, quick, permanent, addToBrief)` (0x588BE0)
    pub fn hud_set_help_message(text: u32, quick: u32, permanent: u32, brief: u32) = cdecl 0x588BE0 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    pub fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CStats::DecrementStat(uint16, float)` (0x559FA0)
    pub fn decrement_stat(stat: u32, v: f32) = cdecl 0x559FA0 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8 increase, uint32 stat, float)` (0x55B980)
    pub fn display_script_stat_update_message(inc: u32, stat: u32, v: f32) = cdecl 0x55B980 => unported;
    /// `CCamera::SetNewPlayerWeaponMode(eCamMode, int16, int16)` (0x50BFB0)
    pub fn camera_set_new_player_weapon_mode(cam: GPtr<u8>, mode: u32, max_zoom: u32, min_zoom: u32) = thiscall 0x50BFB0 => unported;
    /// `CCamera::ClearPlayerWeaponMode` (0x50AB10)
    pub fn camera_clear_player_weapon_mode(cam: GPtr<u8>) = thiscall 0x50AB10 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    pub fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::GetAnimAssociation(AssocGroupId, AnimationId)` (0x4D3A60)
    pub fn get_anim_association(group: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D3A60 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    pub fn add_animation(clump: GPtr<u8>, group: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CPed::operator new(size)` (0x5E4720)
    pub fn ped_operator_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CPlayerPed::CPlayerPed(int32 playerId, bool groupCreated)` (0x60D5B0, `ret 8`)
    pub fn player_ped_ctor(p: GPtr<u8>, id: i32, b: u32) -> GPtr<u8> = thiscall 0x60D5B0 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    pub fn set_orientation(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x439A80 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CWeapon::StopWeaponEffect` (0x73B360)
    pub fn weapon_stop_weapon_effect(w: GPtr<u8>) = thiscall 0x73B360 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280)
    pub fn ped_set_current_weapon(p: GPtr<u8>, ty: i32) = thiscall 0x5E6280 => unported;
    /// `CPed::GetWeaponSkill()` (0x5E6580)
    pub fn ped_get_weapon_skill_current(p: GPtr<u8>) -> u8 = thiscall 0x5E6580 => crate::ped::get_weapon_skill_current;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(ty: i32, skill: i32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPad::SprintJustDown` (0x5407F0)
    pub fn pad_sprint_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x5407F0 => unported;
    /// `CPad::GetSprint` (0x5407A0)
    pub fn pad_get_sprint(pad: GPtr<u8>) -> u8 = thiscall 0x5407A0 => unported;
    /// `CPlayerPed::HandleSprintEnergy(bool, float)` (0x60A550)
    pub fn handle_sprint_energy(p: GPtr<u8>, sprint: u32, rate: f32) -> u8 = thiscall 0x60A550 => crate::player_ped::handle_sprint_energy;
    /// `CWeapon::GenerateDamageEvent(CPed* victim, CEntity* creator, eWeaponType, int32 damage, ePedPieceTypes, int32 dir)` (0x73A530)
    pub fn generate_damage_event(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, damage: i32, piece: i32, dir: i32) = cdecl 0x73A530 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, ST0)
    pub fn get_fat_and_muscle_modifier(a: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CPlayerPed::SetRealMoveAnim` (0x60A9C0)
    pub fn set_real_move_anim(p: GPtr<u8>) = thiscall 0x60A9C0 => unported;
    /// `CCam::Using3rdPersonMouseCam` (0x50A850)
    pub fn cam_using_3rd_person_mouse_cam(cam: GPtr<u8>) -> u8 = thiscall 0x50A850 => unported;
    /// `CPed::CanStrafeOrMouseControl` (0x5DF090)
    pub fn ped_can_strafe_or_mouse_control(p: GPtr<u8>) -> u8 = thiscall 0x5DF090 => unported;
    /// `CVisibilityPlugins::GetClumpModelInfo(RpClump*)` (0x732AC0)
    pub fn get_clump_model_info(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x732AC0 => unported;
    /// `CBaseModelInfo::GetModelType` (vtable slot 0x10, AL)
    pub fn model_info_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    pub fn get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CPedClothesDesc::GetIsWearingBalaclava` (0x5A7950)
    pub fn clothes_get_is_wearing_balaclava(d: GPtr<u8>) -> u8 = thiscall 0x5A7950 => unported;
    /// `CClothes::GetDefaultPlayerMotionGroup` (0x5A81B0)
    pub fn get_default_player_motion_group() -> i32 = cdecl 0x5A81B0 => unported;
    /// `CPlayerPed::ReApplyMoveAnims` (0x609650)
    pub fn re_apply_move_anims_call(p: GPtr<u8>) = thiscall 0x609650 => re_apply_move_anims;
    /// `CAnimManager::GetAnimBlockName(AssocGroupId)` (0x4D3A30)
    pub fn get_anim_block_name(group: i32) -> GPtr<u8> = cdecl 0x4D3A30 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    pub fn get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    pub fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    pub fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CDarkel::FrenzyOnGoing` (0x43D1F0)
    pub fn frenzy_on_going() -> u8 = cdecl 0x43D1F0 => unported;
    /// `CPad::CycleWeaponRightJustDown` (0x540640)
    pub fn pad_cycle_weapon_right_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540640 => unported;
    /// `CPad::CycleWeaponLeftJustDown` (0x540610)
    pub fn pad_cycle_weapon_left_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540610 => unported;
    /// `CPad::GetWeapon(CPed*)` (0x540180; the caller tests all of EAX)
    pub fn pad_get_weapon(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x540180 => unported;
    /// `CWeapon::HasWeaponAmmoToBeUsed` (0x73B2A0)
    pub fn weapon_has_ammo_to_be_used(w: GPtr<u8>) -> u8 = thiscall 0x73B2A0 => unported;
    /// `CPedIntelligence::GetTaskUseGun` (0x600F70)
    pub fn get_task_use_gun(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPed::RemoveWeaponAnims(int32 slot, float blendDelta)` (0x5F0250)
    pub fn ped_remove_weapon_anims(p: GPtr<u8>, slot: i32, delta: f32) = thiscall 0x5F0250 => unported;
    /// `CPlayerPed::MakeChangesForNewWeapon(eWeaponType)` (0x60B460)
    pub fn make_changes_for_new_weapon_call(p: GPtr<u8>, ty: i32) = thiscall 0x60B460 => make_changes_for_new_weapon;
    /// `CMBlur::ClearDrunkBlur` (0x71D780)
    pub fn clear_drunk_blur() = cdecl 0x71D780 => unported;
    /// `CPed::ClearAimFlag` (0x5DEF20)
    pub fn ped_clear_aim_flag(p: GPtr<u8>) = thiscall 0x5DEF20 => unported;
    /// `CPed::ClearLookFlag` (0x5E1950)
    pub fn ped_clear_look_flag(p: GPtr<u8>) = thiscall 0x5E1950 => crate::ped::clear_look_flag;
    /// `CFire::Extinguish` (0x5393F0)
    pub fn fire_extinguish(f: GPtr<u8>) = thiscall 0x5393F0 => unported;
    /// `CPed::SetPedState(ePedState)` (0x5E4500)
    pub fn ped_set_ped_state(p: GPtr<u8>, s: i32) = thiscall 0x5E4500 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    pub fn ped_set_move_state(p: GPtr<u8>, s: i32) = thiscall 0x5DEC00 => unported;
    /// `CPedIntelligence::FlushIntelligence` (0x601DA0)
    pub fn intel_flush_intelligence(intel: GPtr<u8>) = thiscall 0x601DA0 => unported;
    /// `RpAnimBlendClumpRemoveAllAssociations(RpClump*)` (0x4D6C00)
    pub fn clump_remove_all_associations(clump: GPtr<u8>) = cdecl 0x4D6C00 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    pub fn task_operator_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimplePlayerOnFoot::CTaskSimplePlayerOnFoot` (0x685750)
    pub fn task_player_on_foot_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x685750 => unported;
    /// `CTaskComplexFacial::CTaskComplexFacial` (0x690D20)
    pub fn task_complex_facial_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x690D20 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0)
    pub fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32 slot)` (0x681B60)
    pub fn task_manager_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, slot: i32) = thiscall 0x681B60 => unported;
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
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0)
    pub fn event_group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    pub fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470)
    pub fn group_intel_add_event(gi: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F7470 => unported;
    /// `CEventPlayerCommandToGroup` destructor body (0x4B2470, unnamed in the catalogue)
    pub fn event_player_command_to_group_dtor(e: GPtr<u8>) = thiscall 0x4B2470 => unported;
    /// `CPlayerPedData::SetInitialState` (0x56F210)
    pub fn ped_data_set_initial_state(d: GPtr<u8>) = thiscall 0x56F210 => crate::player_info::set_initial_state;
}

/// The vector `GetPos_Hook` stores into a 12-byte stack buffer, read back through the returned pointer.
fn info_pos(this: GPtr<u8>) -> [f32; 3] {
    let mut buf = [0u32; 3];
    let r = player_info_get_pos(this, lp(&mut buf).cast());
    [f32::from_bits(r.read()), f32::from_bits(r.wrapping_index(1).read()), f32::from_bits(r.wrapping_index(2).read())]
}

/// `CPlayerInfo::MakePlayerSafe(bool, float radius)` (0x56E870, `ret 8`; the body continues at 0x401CEC
/// and back). The pad is `GetPadFromPlayer` (not null-checked). On: wanted +0x1E bit 2 set,
/// `StopAllLawEnforcersInTheirTracks`, pad +0x10E bit 5 set, `StopPadsShaking`, six ped +0x40 flags set
/// (one RMW each), player data +0x34 bit 4 cleared, `ClearAdrenaline`, a negative sprint energy (+0x1C,
/// x87 compare with 0.0) zeroed, `ClearTasks(true, false)`, then the fires at the player position
/// (`GetPos_Hook` before each call) are extinguished (radius), explosions removed (4000.0), projectiles
/// removed, cars made damage-proof, car fires extinguished (radius), replays disabled, the weapon target
/// cleared. Off: the reverse flags, data +0x34 bit 4 set, cars damageable, replays enabled.
#[hook(addr = 0x56E870, conv = thiscall, name = "CPlayerInfo::MakePlayerSafe", expect = "83 EC 0C 56 8B F1", this = "CPlayerInfo")]
pub fn make_player_safe(this: GPtr<u8>, toggle: u32, radius: f32) {
    let pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0));
    let wanted = rd::<GPtr<u8>>(this, 4);
    let wf = rd::<u8>(wanted, 0x1E);
    let ped = || rd::<GPtr<u8>>(this, 0);
    const BITS: [u32; 6] = [0x40_0000, 0x4_0000, 0x8_0000, 0x80_0000, 0x10_0000, 0x20_0000];
    if toggle as u8 != 0 {
        wr::<u8>(wanted, 0x1E, wf | 4);
        stop_all_law_enforcers_in_their_tracks();
        wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) | 0x20);
        stop_pads_shaking();
        for b in BITS {
            wr::<u32>(ped(), 0x40, rd::<u32>(ped(), 0x40) | b);
        }
        let d = rd::<GPtr<u8>>(ped(), 0x480);
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & !0x10);
        clear_adrenaline(ped());
        if !x(rd::<f32>(this, 0x1C)).fcom(x(0.0)).test_parity(5) {
            wr::<u32>(this, 0x1C, 0);
        }
        intel_clear_tasks(rd::<GPtr<u8>>(ped(), 0x47C), 1, 0);
        let p = info_pos(this);
        fire_extinguish_point(g::<u8>(FIRE_MANAGER), p[0], p[1], p[2], radius);
        let p = info_pos(this);
        remove_all_explosions_in_area(p[0], p[1], p[2], 4000.0);
        remove_all_projectiles();
        set_all_cars_can_be_damaged(0);
        let p = info_pos(this);
        extinguish_all_car_fires_in_area(p[0], p[1], p[2], radius);
        disable_replays();
        clear_weapon_target(ped());
    } else {
        wr::<u8>(wanted, 0x1E, wf & !4);
        wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) & 0xDF);
        for b in BITS {
            wr::<u32>(ped(), 0x40, rd::<u32>(ped(), 0x40) & !b);
        }
        let d = rd::<GPtr<u8>>(ped(), 0x480);
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) | 0x10);
        set_all_cars_can_be_damaged(1);
        enable_replays();
    }
}

/// `CPlayerInfo::Clear` (0x56F330): resets the per-session fields; the special collectable (+0xB4) loses
/// entity flag +0x40 bit 26 and is dropped; +0xD0 = `CTimer::m_snTimeInMilliseconds`; the word +0x144 of
/// `Players[PlayerInFocus]` (not `this`, as the original) is zeroed; then the inlined
/// `StreamParachuteWeapon(false)` (`IsCoopGameGoingOn` called and ignored, the parachute released).
#[hook(addr = 0x56F330, conv = thiscall, name = "CPlayerInfo::Clear", expect = "53 56 8B F1 8B 86 B4 00 00 00", this = "CPlayerInfo")]
pub fn clear(this: GPtr<u8>) {
    let c = rd::<GPtr<u8>>(this, 0xB4);
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 0xB0, 0);
    if !c.is_null() {
        wr::<u32>(c, 0x40, rd::<u32>(c, 0x40) & 0xFBFF_FFFF);
        wr::<u32>(this, 0xB4, 0);
    }
    wr::<u32>(this, 0xBC, 0);
    wr::<u32>(this, 0xB8, 0);
    wr::<u8>(this, 0xDC, 0);
    wr::<u16>(this, 0x130, 0);
    wr::<f32>(this, 0x134, 1.0);
    for o in [0xDD, 0xDE, 0xDF, 0xD5, 0xD4] {
        wr::<u8>(this, o, 0);
    }
    wr::<u32>(this, 0xCC, 0);
    wr::<u32>(this, 0xD0, sa_core::timer::TIME_MS.get());
    wr::<u8>(this, 0x150, 0x64);
    wr::<u8>(this, 0x14F, 0x64);
    wr::<u8>(this, 0x153, 1);
    wr::<u32>(this, 0xC0, 0);
    wr::<u32>(this, 0xC4, 3);
    for o in [
        0xE4, 0xE8, 0xEC, 0xF4, 0xF0, 0xF8, 0xFC, 0x100, 0x104, 0x108, 0x10C, 0x110, 0x114, 0x118, 0x11C, 0x120, 0x124, 0x128,
        0x12C,
    ] {
        wr::<u32>(this, o, 0);
    }
    for o in [0x14C, 0x14D, 0x14E, 0x151, 0x152] {
        wr::<u8>(this, o, 0);
    }
    for o in [0x138, 0x13C, 0x140] {
        wr::<u32>(this, o, 0);
    }
    let focus = usize::from(g::<u8>(PLAYER_IN_FOCUS).read());
    g::<u16>(PLAYERS + 0x144 + focus * PLAYER_INFO_SIZE).write(0);
    wr::<u16>(this, 0x156, 1);
    wr::<u32>(this, 0x148, 0);
    wr::<u8>(this, 0x154, 0);
    wr::<u8>(this, 0x158, 0);
    wr::<u32>(this, 0x18C, 0);
    if rd::<u8>(this, 0x188) != 0 {
        is_coop_game_going_on();
        if rd::<u8>(this, 0x188) != 0 {
            set_model_is_deletable(MI_PARACHUTE);
            wr::<u8>(this, 0x188, 0);
            wr::<u32>(this, 0x18C, 0);
        }
    }
}

/// `CPlayerInfo::WorkOutEnergyFromHunger` (0x56E610). Statics: 0xB9B8F4 bit 0 (init guard, dword RMW),
/// 0xB9B8F2 last processed hour, 0xB9B8F1 last hungry state, 0xB9B8F0 message shown. Skipped with the
/// never-hungry cheat (0x969174), disabled pad 0 controls (+0x10E word), a menu (0xBA82E3), widescreen
/// (0xB6F065), a cutscene (0xB5F851), co-op, a remote vehicle, no ped or an attached ped (+0xFC). Per new
/// hour +0x144 counts up; above 48 hours: stomach rumble (`Say(0x151, 0, 1.0, 0, 0, 0)` on the ped kept
/// in ECX), pad 0 shake (400, 110, 0); the first time the "NOTEAT" help, then fat (stat 0x15) and muscle
/// (0x17) lose 25 each, or the health 2.0.
#[hook(addr = 0x56E610, conv = thiscall, name = "CPlayerInfo::WorkOutEnergyFromHunger", expect = "A0 74 91 96 00", this = "CPlayerInfo")]
pub fn work_out_energy_from_hunger(this: GPtr<u8>) {
    if g::<u8>(0x969174).read() != 0 {
        return;
    }
    let guard = g::<u32>(0xB9B8F4);
    let last_hour = g::<u8>(0xB9B8F2);
    let hungry = g::<u8>(0xB9B8F1);
    let shown = g::<u8>(0xB9B8F0);
    let hour = g::<u8>(CLOCK_HOURS);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
        last_hour.write(hour.read());
    }
    if rd::<u16>(get_pad(0), 0x10E) != 0
        || g::<u8>(0xBA82E3).read() != 0
        || g::<u8>(0xB6F065).read() != 0
        || g::<u8>(0xB5F851).read() != 0
        || is_coop_game_going_on() != 0
        || rd::<u32>(this, 0xB0) != 0
    {
        return;
    }
    let ped = rd::<GPtr<u8>>(this, 0);
    if ped.is_null() || rd::<u32>(ped, 0xFC) != 0 {
        return;
    }
    let lh = last_hour.read();
    if hour.read() != lh {
        let n = rd::<u16>(this, 0x144);
        if n == 0 {
            hungry.write(0);
        }
        wr::<u16>(this, 0x144, n.wrapping_add(1));
    }
    if rd::<i16>(this, 0x144) <= 0x30 {
        shown.write(0);
    } else {
        if hour.read() == lh {
            return;
        }
        ped_say(ped, 0x151, 0, 1.0, 0, 0, 0);
        pad_start_shake(get_pad(0), 0x190, 0x6E, 0);
        if shown.read() == 0 {
            hud_set_help_message(text_get(g::<u8>(THE_TEXT), 0x865028), 1, 0, 1);
            shown.write(1);
        } else {
            let mut dec = false;
            if !X87::from_f64_exact(get_stat_value(0x15)).fcom(x(0.0)).test_any(0x41) {
                decrement_stat(0x15, 25.0);
                display_script_stat_update_message(0, 0x15, 25.0);
                dec = true;
                if hungry.read() == 0 {
                    hungry.write(rd::<u8>(this, 0x144).wrapping_add(0x18));
                }
            }
            let muscle = !X87::from_f64_exact(get_stat_value(0x17)).fcom(x(0.0)).test_any(0x41);
            let h = hungry.read();
            if muscle && (rd::<i16>(this, 0x144) > i16::from(h) || h == 0) {
                decrement_stat(0x17, 25.0);
                display_script_stat_update_message(0, 0x17, 25.0);
            } else if !dec {
                let p = rd::<GPtr<u8>>(this, 0);
                wr::<f32>(p, 0x540, x(rd::<f32>(p, 0x540)).fsub_m(2.0).to_f32());
            }
        }
    }
    let hv = hour.read();
    if hv != last_hour.read() {
        last_hour.write(hv);
    }
}

/// `CPlayerPed::ProcessPlayerWeapon(CPed*)` (0x6097F0, `ret 4`): empty.
// Not registered: the body (`ret 4`, 3 bytes) is shorter than the 5-byte entry patch (sa-hook TooShort).
pub fn process_player_weapon(_this: GPtr<u8>, _ped: GPtr<u8>) {}

/// `CPlayerPed::UpdateCameraWeaponModes(CPad*)` (0x609830, `ret 4`; the pad is not read): by the active
/// weapon, `SetNewPlayerWeaponMode(mode, 0, 0)` with M4 (0x1F) 0x22, sniper (0x22) 7, RPG (0x23) 8,
/// heat-seeker (0x24) 0x33, camera (0x2B) 0x2E; any other weapon clears the player weapon mode.
#[hook(addr = 0x609830, conv = thiscall, name = "CPlayerPed::UpdateCameraWeaponModes", expect = "0F BE 81 18 07 00 00", this = "CPlayerPed")]
pub fn update_camera_weapon_modes(this: GPtr<u8>, _pad: GPtr<u8>) {
    let w = rd::<i32>(this, 0x5A0 + isize::from(rd::<i8>(this, 0x718)) * 0x1C);
    let mode = match w {
        0x1F => 0x22,
        0x22 => 7,
        0x23 => 8,
        0x24 => 0x33,
        0x2B => 0x2E,
        _ => {
            camera_clear_player_weapon_mode(g::<u8>(THE_CAMERA));
            return;
        }
    };
    camera_set_new_player_weapon_mode(g::<u8>(THE_CAMERA), mode, 0, 0);
}

/// `CPlayerPed::SetMoveAnim` (0x609490): empty.
// Not registered: the body (`ret`, 1 byte) is shorter than the 5-byte entry patch (sa-hook TooShort).
pub fn set_move_anim(_this: GPtr<u8>) {}

/// `CPlayerPed::ReApplyMoveAnims` (0x609650): for the anims {0, 1, 2, 3, 5} playing on the clump whose
/// hierarchy (+0x14, first dword) differs from the ped's anim group's (+0x4D4) static association's
/// (+0x10), the group's anim is added, takes over blend delta (+0x1C) and amount (+0x18, x87 copy), and
/// the old one gets flag +0x2E bit 2 and blend delta -1000.
#[hook(addr = 0x609650, conv = thiscall, name = "CPlayerPed::ReApplyMoveAnims", expect = "83 EC 14 53 55", this = "CPlayerPed")]
pub fn re_apply_move_anims(this: GPtr<u8>) {
    for id in [0u32, 1, 2, 3, 5] {
        let a = clump_get_association(rd::<GPtr<u8>>(this, 0x18), id);
        if a.is_null() {
            continue;
        }
        let s = get_anim_association(rd::<u32>(this, 0x4D4), id);
        let sh = rd::<GPtr<u8>>(s, 0x10);
        if rd::<u32>(rd::<GPtr<u8>>(a, 0x14), 0) == rd::<u32>(sh, 0) {
            continue;
        }
        let n = add_animation(rd::<GPtr<u8>>(this, 0x18), rd::<u32>(this, 0x4D4), id);
        wr::<u32>(n, 0x1C, rd::<u32>(a, 0x1C));
        wr::<f32>(n, 0x18, x(rd::<f32>(a, 0x18)).to_f32());
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        wr::<f32>(a, 0x1C, -1000.0);
    }
}

/// `CPlayerPed::SetupPlayerPed(int32 playerId)` (0x60D790, cdecl): `new CPlayerPed(id, false)` from the
/// ped pool into `Players[id].ped` (null when the pool is full; then the ped-type write and the calls below
/// use null as the original), player 1 gets ped type 1, `SetOrientation(0, 0, 0)`, `CWorld::Add`, weapon
/// accuracy (+0x71A) 100, player state (+0xDC) 0. The SEH frame around the constructor is not built.
#[hook(addr = 0x60D790, conv = cdecl, name = "CPlayerPed::SetupPlayerPed", expect = "6A FF 68 7B ED 83 00")]
pub fn setup_player_ped(id: i32) {
    let m = ped_operator_new(0x7A4);
    let p = if m.is_null() { GPtr::NULL } else { player_ped_ctor(m, id, 0) };
    let slot = PLAYERS.wrapping_add((id as usize).wrapping_mul(PLAYER_INFO_SIZE));
    g::<GPtr<u8>>(slot).write(p);
    if id == 1 {
        wr::<u32>(p, 0x598, 1);
    }
    set_orientation(g::<GPtr<u8>>(slot).read(), 0.0, 0.0, 0.0);
    world_add(g::<GPtr<u8>>(slot).read());
    wr::<u8>(g::<GPtr<u8>>(slot).read(), 0x71A, 0x64);
    g::<u8>(slot + 0xDC).write(0);
}

fn pdata(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x480)
}

/// The active weapon slot's offset in the ped (`+0x5A0 + slot * 0x1C`).
fn active_weapon_off(p: GPtr<u8>) -> isize {
    0x5A0 + isize::from(rd::<i8>(p, 0x718)) * 0x1C
}

/// `CPlayerPed::MakeChangesForNewWeapon(eWeaponType)` (0x60B460, `ret 4`): stops the current weapon's
/// effect, clears the camera weapon mode in ped state 0xC, `SetCurrentWeapon(type)`, chosen slot (data
/// +0x20) = active slot, data +0x2C = 0, ammo in clip = `min(total, info clip size)` (signed, i16 clip
/// size), weapons without "can aim" (info +0x18 bit 0) clear the weapon target, without bit 3 clear data
/// +0x34 bit 3, the clump's anim 0xE0 gets flags +0x2E |= 9, and the camera weapon mode is cleared.
#[hook(addr = 0x60B460, conv = thiscall, name = "CPlayerPed::MakeChangesForNewWeapon-", expect = "56 8B F1 0F BE 86 18 07 00 00", this = "CPlayerPed")]
pub fn make_changes_for_new_weapon(this: GPtr<u8>, ty: i32) {
    let w = this.byte_add(active_weapon_off(this));
    if !w.is_null() {
        weapon_stop_weapon_effect(w);
    }
    if rd::<i32>(this, 0x530) == 0xC {
        camera_clear_player_weapon_mode(g::<u8>(THE_CAMERA));
    }
    ped_set_current_weapon(this, ty);
    wr::<i8>(pdata(this), 0x20, rd::<i8>(this, 0x718));
    wr::<u32>(pdata(this), 0x2C, 0);
    let wt = rd::<i32>(this, active_weapon_off(this));
    let sk = ped_get_weapon_skill_current(this);
    let info = get_weapon_info(wt, i32::from(sk));
    let off = active_weapon_off(this);
    let clip = i32::from(rd::<i16>(info, 0x20));
    let total = rd::<i32>(this, off + 0xC);
    wr::<i32>(this, off + 8, if total < clip { total } else { clip });
    if rd::<u8>(info, 0x18) & 1 == 0 {
        clear_weapon_target(this);
    }
    if (rd::<u32>(info, 0x18) >> 3) & 1 == 0 {
        let d = pdata(this);
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_FFF7);
    }
    let a = clump_get_association(rd::<GPtr<u8>>(this, 0x18), 0xE0);
    if !a.is_null() {
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 9);
    }
    camera_clear_player_weapon_mode(g::<u8>(THE_CAMERA));
}

/// Sprint parameter `off` of sprint type `ty` (0x8D2460 + type * 0x20).
fn sprint_param(ty: i32, off: u32) -> f32 {
    g::<f32>(0x8D2460u32.wrapping_add((ty as u32).wrapping_shl(5)).wrapping_add(off) as usize).read()
}

/// `CPlayerPed::ControlButtonSprint(eSprintType)` (0x60A610, `ret 4`, ST0). Table 0x8D2460 + type*0x20:
/// +0 tap add, +4 hold decay, +8 idle decay, +0xC threshold, +0x10 max, +0x14 / +0x18 energy rates, +0x1C
/// result factor. Without player data 0.0. "Can sprint" = data +0x84 clear and (button time > 0 or sprint
/// energy > 0). Sprint just down: time = `min(time + add, max)` (the compare keeps the sum on the stack,
/// the max stored as f32); else held: `max(time - step*decay, 1.0)`; else a positive time decays to at
/// least 0.0. Then above the threshold `r = time / threshold` (f32) and rate +0x18 (`r > 0` required),
/// else a positive time with "can sprint" gives `r = 1.0` and rate +0x14; otherwise 0.0. With
/// `HandleSprintEnergy(true, rate)` the result is `max(r - 1, 0) * factor + 1`, else the time is zeroed
/// and 0.0 returned. The pad is `GetPad(0/1)` for ped types 0/1, else null (called through null).
#[hook(addr = 0x60A610, conv = thiscall, name = "CPlayerPed::ControlButtonSprint", expect = "51 57 8B F9 8B 87 80 04 00 00", this = "CPlayerPed")]
pub fn control_button_sprint(this: GPtr<u8>, ty: i32) -> X87 {
    if pdata(this).is_null() {
        return x(0.0);
    }
    let pad = match rd::<i32>(this, 0x598) {
        0 => get_pad(0),
        1 => get_pad(1),
        _ => GPtr::NULL,
    };
    let d = pdata(this);
    let can = rd::<u8>(d, 0x84) == 0
        && (!x(rd::<f32>(d, 0x1C)).fcom(x(0.0)).test_any(0x41) || !x(rd::<f32>(d, 0x18)).fcom(x(0.0)).test_any(0x41));
    if pad_sprint_just_down(pad) != 0 && can {
        let d = pdata(this);
        let v = x(sprint_param(ty, 0)).fadd_m(rd::<f32>(d, 0x1C));
        let m = sprint_param(ty, 0x10);
        let s = if x(m).fcom(v).test_parity(5) { v } else { x(m) };
        wr::<f32>(d, 0x1C, s.to_f32());
    } else if pad_get_sprint(pad) != 0 && can {
        let d = pdata(this);
        let v = x(sa_core::timer::TIME_STEP.get()).fmul_m(sprint_param(ty, 4)).fsubr_m(rd::<f32>(d, 0x1C));
        let s = if x(1.0).fcom(v).test_any(0x41) { v } else { x(1.0) };
        wr::<f32>(d, 0x1C, s.to_f32());
    } else {
        let d = pdata(this);
        if !x(rd::<f32>(d, 0x1C)).fcom(x(0.0)).test_any(0x41) {
            let v = x(sa_core::timer::TIME_STEP.get()).fmul_m(sprint_param(ty, 8)).fsubr_m(rd::<f32>(d, 0x1C));
            let s = if x(0.0).fcom(v).test_any(0x41) { v } else { x(0.0) };
            wr::<f32>(d, 0x1C, s.to_f32());
        }
    }
    let d = pdata(this);
    let t = rd::<f32>(d, 0x1C);
    let thr = sprint_param(ty, 0xC);
    let (r, rate) = if !x(t).fcom(x(thr)).test_any(0x41) {
        let r = x(t).fdiv_m(thr);
        let rate = sprint_param(ty, 0x18);
        if r.fcom(x(0.0)).test_any(0x41) {
            return x(0.0);
        }
        (r.to_f32(), rate)
    } else {
        if x(t).fcom(x(0.0)).test_any(0x41) || !can {
            return x(0.0);
        }
        (1.0, sprint_param(ty, 0x14))
    };
    if handle_sprint_energy(this, 1, rate) == 0 {
        wr::<u32>(pdata(this), 0x1C, 0);
        return x(0.0);
    }
    let mut v = x(r).fsub_m(1.0);
    if !x(0.0).fcom(v).test_any(0x41) {
        v = x(0.0);
    }
    v.fmul_m(sprint_param(ty, 0x1C)).fadd_m(1.0)
}

/// `CPlayerPed::HandlePlayerBreath(bool underWater, float rate)` (0x60A8D0, `ret 8`). Under water without
/// the infinite-oxygen cheat (0x96916E): with breath (data +0x44) > 0 and ped flag +0x470 bit 20 the breath
/// drops by `step * rate` (not below 0.0); otherwise `GenerateDamageEvent(this, this, 0x35,
/// _ftol(step * rate * 3.0), 3, 0)`. Above water the breath refills by `2 * step * rate` while below
/// `GetFatAndMuscleModifier(8)`. Data +0x34 bit 7 is cleared in every case.
#[hook(addr = 0x60A8D0, conv = thiscall, name = "CPlayerPed::HandlePlayerBreath", expect = "8A 44 24 04 84 C0 56", this = "CPlayerPed")]
pub fn handle_player_breath(this: GPtr<u8>, under_water: u32, rate: f32) {
    if under_water as u8 != 0 && g::<u8>(0x96916E).read() == 0 {
        let t = x(sa_core::timer::TIME_STEP.get()).fmul_m(rate);
        let d = pdata(this);
        if !x(rd::<f32>(d, 0x44)).fcom(x(0.0)).test_any(0x41) && rd::<u32>(this, 0x470) & 0x10_0000 != 0 {
            let v = t.fsubr_m(rd::<f32>(d, 0x44));
            let s = if x(0.0).fcom(v).test_any(0x41) { v } else { x(0.0) };
            wr::<f32>(d, 0x44, s.to_f32());
        } else {
            let dmg = ftol32(t.fmul_m(3.0));
            generate_damage_event(this, this, 0x35, dmg, 3, 0);
        }
    } else {
        let d = pdata(this);
        let m = X87::from_f64_exact(get_fat_and_muscle_modifier(8));
        if !m.fcom(x(rd::<f32>(d, 0x44))).test_any(0x41) {
            let t = x(sa_core::timer::TIME_STEP.get()).fmul_m(rate);
            let d = pdata(this);
            wr::<f32>(d, 0x44, t.fadd(t).fadd_m(rd::<f32>(d, 0x44)).to_f32());
        }
    }
    let d = pdata(this);
    wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_FF7F);
}

/// `CPlayerPed::SetPlayerMoveBlendRatio(CVector* target)` (0x60C520, `ret 4`): with a target, data +0x14 =
/// `2 * sqrt(dy*dy + dx*dx)` (x87, from the entity position) capped at 2.0 after the f32 store; without,
/// by move state 4 / 6 / 7: 1.0 / 1.8 / 2.5, else 0. Then `SetRealMoveAnim`.
#[hook(addr = 0x60C520, conv = thiscall, name = "CPlayerPed::SetPlayerMoveBlendRatio", expect = "8B 54 24 04 85 D2", this = "CPlayerPed")]
pub fn set_player_move_blend_ratio(this: GPtr<u8>, target: GPtr<f32>) {
    if !target.is_null() {
        let p = ent_pos(this);
        let dx = x(target.read()).fsub_m(p.read());
        let dy = x(target.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read());
        let s = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        let d = pdata(this);
        wr::<f32>(d, 0x14, s.fadd(s).to_f32());
        let d = pdata(this);
        if !x(rd::<f32>(d, 0x14)).fcom(x(2.0)).test_any(0x41) {
            wr::<f32>(d, 0x14, 2.0);
        }
    } else {
        let v: u32 = match rd::<i32>(this, 0x534) {
            4 => 0x3F80_0000,
            6 => 0x3FE6_6666,
            7 => 0x4020_0000,
            _ => 0,
        };
        wr::<u32>(pdata(this), 0x14, v);
    }
    set_real_move_anim(this);
}

/// Anim block id of weapon anim group `grp` (dword at 0xC170D0 + (grp - 4) * 0x88).
fn weapon_group_block(grp: i32) -> i32 {
    g::<i32>(0xC170D0u32.wrapping_add((grp.wrapping_sub(4) as u32).wrapping_mul(0x88)) as usize).read()
}

/// The anim block of assoc group `id` (`ms_aAnimAssocGroups` at [0xB4EA34], 20 bytes each, +0).
fn group_anim_block(id: i32) -> GPtr<u8> {
    let t = g::<u32>(0xB4EA34).read();
    g::<GPtr<u8>>(t.wrapping_add((id as u32).wrapping_mul(20)) as usize).read()
}

/// `CAnimManager::ms_aAnimBlocks` index of a block (`(b - 0xB5D4A0) sar 5`).
fn block_index(b: GPtr<u8>) -> i32 {
    ((b.addr() as u32).wrapping_sub(0xB5D4A0) as i32) >> 5
}

/// One weapon anim-block slot of `ProcessAnimGroups` (player data `off` = +0x48 / +0x4C): a different
/// block marks the old one for release, then the new one is requested (not loaded) or referenced and
/// stored (loaded, slot empty).
fn stream_anim_slot(this: GPtr<u8>, off: isize, blk: i32, release: &mut bool) {
    let cur = rd::<i32>(pdata(this), off);
    if blk == cur {
        return;
    }
    if cur != 0 {
        *release = true;
    }
    let mut b = group_anim_block(blk);
    if b.is_null() {
        b = get_animation_block(get_anim_block_name(blk));
    }
    let idx = block_index(b);
    if rd::<u8>(b, 0x10) != 1 {
        request_model(idx.wrapping_add(0x63E7), 8);
    } else if rd::<i32>(pdata(this), off) == 0 {
        add_anim_block_ref(idx);
        wr::<i32>(pdata(this), off, blk);
    }
}

fn release_anim_slot(this: GPtr<u8>, off: isize) {
    let v = rd::<i32>(pdata(this), off);
    if v != 0 {
        remove_anim_block_ref(block_index(group_anim_block(v)));
        wr::<i32>(pdata(this), off, 0);
    }
}

/// `e + CClothes::GetDefaultPlayerMotionGroup() - 0x36`
fn motion_group(e: i32) -> i32 {
    e.wrapping_add(get_default_player_motion_group()).wrapping_sub(0x36)
}

/// The default player group: player 2 (ped type 1) with a model motion group (+0x24) other than 0x36
/// uses it, else `motion_group(0x36)`.
fn default_motion_group(this: GPtr<u8>) -> i32 {
    if rd::<i32>(this, 0x598) == 1 {
        let mi = g::<GPtr<u8>>(0xA9B0C8usize.wrapping_add((i32::from(rd::<i16>(this, 0x22)) as usize).wrapping_mul(4))).read();
        let mg = rd::<i32>(mi, 0x24);
        if mg != 0x36 {
            return mg;
        }
    }
    motion_group(0x36)
}

/// `CPlayerPed::ProcessAnimGroups` (0x6098F0; not reversed by gta-reversed). Move group (+0x4D4): with the
/// move direction (data +0x50) outside (-0.8727, 0.8727) (x87 compares) and the active camera a 3rd-person
/// mouse cam with `CanStrafeOrMouseControl`, the old group re-based (`old + default - 0x36`; 0x36 itself
/// takes the default path). Otherwise by the weapon model held (+0x4F4 clump, model type 4, weapon
/// +0x24): rockets 0x39; a jetpack task 0x46; melee 5/6/7 0x3F; 9/0x25/0x26 0x42; guns 0x1F/0x1E/0x1B/
/// 0x19/0x22/0x21 0x3C (all re-based); a balaclava 0x45; else the default. A changed group is stored and
/// `ReApplyMoveAnims` runs. Weapon anim blocks (data +0x48 from weapon info +0x6E, +0x4C from ped +0x72D,
/// group 4 = none) are streamed and referenced; a weapon with a fire type or a ped in a vehicle releases
/// both.
#[hook(addr = 0x6098F0, conv = thiscall, name = "CPlayerPed::ProcessAnimGroups", expect = "51 55 56 8B F1", this = "CPlayerPed")]
pub fn process_anim_groups(this: GPtr<u8>) {
    let d = pdata(this);
    let old = rd::<i32>(this, 0x4D4);
    let cam_check = x(rd::<f32>(d, 0x50)).fcom(x(f32::from_bits(0xBF5F_66F3))).test_any(0x41)
        || x(rd::<f32>(d, 0x50)).fcom(x(f32::from_bits(0x3F5F_66F3))).test_parity(5);
    let new = 'g: {
        if cam_check {
            let cam = g::<u8>(0xB6F19C + usize::from(g::<u8>(0xB6F081).read()) * 0x238);
            if cam_using_3rd_person_mouse_cam(cam) != 0 && ped_can_strafe_or_mouse_control(this) != 0 {
                if old != 0x36 {
                    break 'g motion_group(old);
                }
                break 'g default_motion_group(this);
            }
        }
        let mut w = 0;
        let clump = rd::<GPtr<u8>>(this, 0x4F4);
        if !clump.is_null() {
            let mi = get_clump_model_info(clump);
            if !mi.is_null() && model_info_get_model_type(mi) == 4 {
                w = rd::<i32>(mi, 0x24);
                if w == 0x23 || w == 0x24 {
                    break 'g motion_group(0x39);
                }
            }
        }
        if !get_task_jet_pack(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
            break 'g 0x46;
        }
        match w {
            5..=7 => motion_group(0x3F),
            9 | 0x25 | 0x26 => motion_group(0x42),
            0x1F | 0x1E | 0x1B | 0x19 | 0x22 | 0x21 => motion_group(0x3C),
            _ => {
                if clothes_get_is_wearing_balaclava(rd::<GPtr<u8>>(pdata(this), 4)) != 0 {
                    0x45
                } else {
                    default_motion_group(this)
                }
            }
        }
    };
    if rd::<i32>(this, 0x4D4) != new {
        wr::<i32>(this, 0x4D4, new);
        re_apply_move_anims_call(this);
    }
    let wt = rd::<i32>(this, active_weapon_off(this));
    let sk = ped_get_weapon_skill_current(this);
    let info = get_weapon_info(wt, i32::from(sk));
    let mut r1 = false;
    let mut r2 = false;
    if rd::<u32>(info, 0) != 0 || rd::<u32>(this, 0x46C) & 0x100 != 0 {
        r1 = true;
        r2 = true;
    } else {
        let a1 = i32::from(rd::<i8>(info, 0x6E));
        if a1 == 4 {
            r1 = rd::<i32>(pdata(this), 0x48) != 0;
        } else {
            stream_anim_slot(this, 0x48, weapon_group_block(a1), &mut r1);
        }
        let a2 = i32::from(rd::<u8>(this, 0x72D));
        if a2 == 4 {
            r2 = rd::<i32>(pdata(this), 0x4C) != 0;
        } else {
            stream_anim_slot(this, 0x4C, weapon_group_block(a2), &mut r2);
        }
    }
    if r1 {
        release_anim_slot(this, 0x48);
    }
    if r2 {
        release_anim_slot(this, 0x4C);
    }
}

/// Weapon slot `s` of a ped (`+0x5A0 + s * 0x1C`).
fn weapon_slot(p: GPtr<u8>, s: i8) -> GPtr<u8> {
    p.byte_add(0x5A0 + isize::from(s) * 0x1C)
}

/// The cycle loops' "usable" test of the chosen slot: a weapon with ammo, outside co-op or allowed for
/// two players.
fn chosen_slot_usable(this: GPtr<u8>) -> bool {
    let s = rd::<i8>(pdata(this), 0x20);
    if rd::<i32>(weapon_slot(this, s), 0) == 0 {
        return false;
    }
    if weapon_has_ammo_to_be_used(weapon_slot(this, rd::<i8>(pdata(this), 0x20))) == 0 {
        return false;
    }
    if is_coop_game_going_on() == 0 {
        return true;
    }
    crate::player_ped::weapon_can_be_used_for_2_player(weapon_slot(this, rd::<i8>(pdata(this), 0x20))) != 0
}

/// `CPlayerPed::ProcessWeaponSwitch(CPad*)` (0x60D850, `ret 4`; not reversed by gta-reversed). Nothing
/// during a frenzy, attached (+0xFC) or with a jetpack task. Without a weapon target (+0x71C), data +0x34
/// bits 3/11 and data +0x85, cycling right/left (outside the camera weapon modes listed in the bytes,
/// word 0xB6F858) moves the chosen slot (data +0x20, i8) to the next usable slot (right: past 12 wraps to
/// 0; left: below 0 wraps to 12, slot 0 stops). Then for a non-melee active weapon (fire type != 0) with
/// no ammo left (unless firing the minigun, or in camera modes 0x22/7/8/0x33): the detonator with a
/// satchel in slot 8 picks slot 8, else the slot below; empty slots are skipped downwards (slot 5 stops
/// when it holds weapon 5), below 0 gives 0. A chosen slot that differs from the active one (unless the
/// use-gun task is in state 2/3, or 4 with +0x2C set) removes the weapon anims (-1000) and, for a slot
/// other than -1, `MakeChangesForNewWeapon(type of the slot)`.
#[hook(addr = 0x60D850, conv = thiscall, name = "CPlayerPed::ProcessWeaponSwitch", expect = "56 57 8B F1 E8 97 F9 E2 FF", this = "CPlayerPed")]
pub fn process_weapon_switch(this: GPtr<u8>, pad: GPtr<u8>) {
    let chosen = || at::<i8>(pdata(this), 0x20);
    let active = || rd::<i8>(this, 0x718);
    let mode = || g::<u16>(0xB6F858).read();
    'end: {
        if frenzy_on_going() != 0 || rd::<u32>(this, 0xFC) != 0 || !get_task_jet_pack(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
            break 'end;
        }
        'ammo: {
            let d = pdata(this);
            let f = rd::<u32>(d, 0x34);
            if rd::<u32>(this, 0x71C) != 0 || f & 8 != 0 || rd::<u8>(d, 0x85) != 0 || f & 0x800 != 0 {
                break 'ammo;
            }
            if pad_cycle_weapon_right_just_down(pad) != 0 {
                if matches!(mode(), 0x22 | 0x2A | 7 | 0x27 | 8 | 0x28 | 0x33 | 0x34 | 0x2E) {
                    break 'ammo;
                }
                chosen().write(active().wrapping_add(1));
                while chosen().read() < 0xD {
                    if chosen_slot_usable(this) {
                        break 'ammo;
                    }
                    chosen().write(chosen().read().wrapping_add(1));
                }
                chosen().write(0);
            } else if pad_cycle_weapon_left_just_down(pad) != 0 {
                if matches!(mode(), 0x22 | 7 | 8 | 0x33 | 0x2E) {
                    break 'ammo;
                }
                chosen().write(active().wrapping_sub(1));
                loop {
                    if chosen().read() < 0 {
                        chosen().write(0xC);
                    }
                    if chosen().read() == 0 || chosen_slot_usable(this) {
                        break 'ammo;
                    }
                    chosen().write(chosen().read().wrapping_sub(1));
                }
            }
        }
        let info = get_weapon_info(rd::<i32>(weapon_slot(this, active()), 0), 1);
        if rd::<u32>(info, 0) == 0 {
            break 'end;
        }
        if pad_get_weapon(pad, this) != 0 && rd::<i32>(weapon_slot(this, active()), 0) == 0x26 {
            break 'end;
        }
        let cl = active();
        if rd::<i32>(weapon_slot(this, cl), 0xC) > 0 {
            break 'end;
        }
        if matches!(mode(), 0x22 | 7 | 8 | 0x33) {
            return;
        }
        if rd::<i32>(weapon_slot(this, cl), 0) == 0x28 && rd::<i32>(this, 0x680) == 0x27 {
            chosen().write(8);
        } else {
            chosen().write(cl.wrapping_sub(1));
        }
        let mut s = chosen().read();
        if s >= 0 {
            loop {
                if s == 5 && rd::<i32>(this, 0x62C) == 5 {
                    break 'end;
                }
                if rd::<i32>(weapon_slot(this, s), 0xC) > 0 && !matches!(s, 0x10..=0x12) {
                    break 'end;
                }
                let n = chosen().read().wrapping_sub(1);
                chosen().write(n);
                s = chosen().read();
                if n < 0 {
                    break;
                }
            }
        }
        chosen().write(0);
    }
    if rd::<i8>(pdata(this), 0x20) == active() {
        return;
    }
    let intel = || rd::<GPtr<u8>>(this, 0x47C);
    if !get_task_use_gun(intel()).is_null() {
        let st = rd::<u8>(get_task_use_gun(intel()), 0x10);
        if st == 2 || st == 3 {
            return;
        }
        let t = get_task_use_gun(intel());
        if rd::<u8>(t, 0x10) == 4 && rd::<u32>(t, 0x2C) != 0 {
            return;
        }
    }
    ped_remove_weapon_anims(this, i32::from(active()), -1000.0);
    let s = rd::<i8>(pdata(this), 0x20);
    if s != -1 {
        make_changes_for_new_weapon_call(this, rd::<i32>(weapon_slot(this, s), 0));
    }
}

/// `CPedGroups::ms_groups` base and stride; `+4` = the group's "player group" flag, `+0x30` its
/// intelligence.
const PED_GROUPS: u32 = 0xC09920;
const PED_GROUP_SIZE: u32 = 0x2D4;

fn player_group(this: GPtr<u8>, off: u32) -> GPtr<u8> {
    let grp = rd::<u32>(pdata(this), 0x38);
    g::<u8>(PED_GROUPS.wrapping_add(grp.wrapping_mul(PED_GROUP_SIZE)).wrapping_add(off) as usize)
}

/// `CPlayerPed::SetInitialState(bool groupCreated)` (0x60CD20, `ret 4`): drunk blur cleared, time scale
/// 1.0, entity flags (+0x1C bit 0, +0x40 bit 1), aim/look flags cleared, +0x46C bit 13, the ped's fire
/// extinguished, ped state 1, move state 1, ped flags, intelligence flushed, all anims removed, a new
/// `CTaskSimplePlayerOnFoot` as primary task 4, +0x46C bit 20 cleared, move group 0x36, data +0x34 bit 1
/// cleared (when there is player data), `SetRealMoveAnim`, stat temper (+0x25) 50, an attached ped
/// (+0xFC) is made visible and detached, +0x784 = 0, a new `CTaskComplexFacial` as secondary task 3.
/// Unless `groupCreated` or data +0x34 bit 10: the player's group (`ms_groups[data +0x38]`) is marked
/// (+4 = 1), its allocator type set to 5, a stack `CEventPlayerCommandToGroup(0, null)` (vtable
/// 0x86C710) computes its response; if it responds a heap copy (command 2, vtable 0x86D120) with the
/// response task id (+0xE) goes to the group intelligence as a stack `CEventGroupEvent`. Finally
/// `CPlayerPedData::SetInitialState`. The SEH frame is not built; the stack events live in Rust buffers
/// of the original sizes (0x1C in a 0x24 slot, 0x14).
#[hook(addr = 0x60CD20, conv = thiscall, name = "CPlayerPed::SetInitialState", expect = "64 A1 00 00 00 00", this = "CPlayerPed")]
pub fn set_initial_state(this: GPtr<u8>, group_created: u32) {
    clear_drunk_blur();
    g::<f32>(0xB7CB64).write(1.0);
    let f1c = rd::<u32>(this, 0x1C) | 1;
    let f40 = rd::<u32>(this, 0x40) | 2;
    wr::<u32>(this, 0x1C, f1c);
    wr::<u32>(this, 0x40, f40);
    ped_clear_aim_flag(this);
    ped_clear_look_flag(this);
    wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) | 0x2000);
    let fire = rd::<GPtr<u8>>(this, 0x730);
    if !fire.is_null() {
        fire_extinguish(fire);
    }
    ped_set_ped_state(this, 1);
    ped_set_move_state(this, 1);
    let f474 = (rd::<u32>(this, 0x474) & 0xFFFF_FFBD) | 0x400_0000;
    let f46c = rd::<u32>(this, 0x46C) & 0xFBFF_FFFF;
    wr::<u32>(this, 0x46C, f46c);
    wr::<u32>(this, 0x474, f474);
    intel_flush_intelligence(rd::<GPtr<u8>>(this, 0x47C));
    clump_remove_all_associations(rd::<GPtr<u8>>(this, 0x18));
    let t = task_operator_new(0x1C);
    let t = if t.is_null() { t } else { task_player_on_foot_ctor(t) };
    task_manager_set_task(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), t, 4, 0);
    let f46c = rd::<u32>(this, 0x46C) & 0xFFEF_FFFF;
    let d = pdata(this);
    wr::<u32>(this, 0x4D4, 0x36);
    wr::<u32>(this, 0x46C, f46c);
    if !d.is_null() {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_FFFD);
    }
    set_real_move_anim(this);
    wr::<u8>(rd::<GPtr<u8>>(this, 0x59C), 0x25, 0x32);
    if rd::<u32>(this, 0xFC) != 0 {
        let f = rd::<u32>(this, 0x1C);
        if f & 1 == 0 {
            wr::<u32>(this, 0x1C, f | 1);
        }
    }
    wr::<u32>(this, 0xFC, 0);
    wr::<u32>(this, 0x784, 0);
    let t = task_operator_new(0x20);
    let t = if t.is_null() { t } else { task_complex_facial_ctor(t) };
    task_manager_set_task_secondary(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), t, 3);
    if group_created as u8 == 0 && rd::<u32>(pdata(this), 0x34) & 0x400 == 0 {
        player_group(this, 4).write(1);
        group_intel_set_default_task_allocator_type(player_group(this, 0x30), 5);
        let mut ev = [0u32; 9];
        let evp: GPtr<u8> = lp(&mut ev).cast();
        event_player_command_to_group_ctor(evp, 0, GPtr::NULL);
        wr::<u32>(evp, 0, 0x86C710);
        event_compute_response_task_type_group(evp, player_group(this, 0));
        if event_will_respond(evp) != 0 {
            let e = event_operator_new(0x1C);
            let e = if e.is_null() {
                e
            } else {
                event_player_command_to_group_ctor(e, 2, GPtr::NULL);
                wr::<u32>(e, 0, 0x86D120);
                e
            };
            wr::<i16>(e, 0xE, rd::<i16>(evp, 0xE));
            let mut ge = [0u32; 5];
            let gep: GPtr<u8> = lp(&mut ge).cast();
            event_group_event_ctor(gep, this, e);
            group_intel_add_event(player_group(this, 0x30), gep);
            event_group_event_dtor(gep);
        }
        wr::<u32>(evp, 0, 0x86C710);
        event_player_command_to_group_dtor(evp);
    }
    let d = pdata(this);
    if !d.is_null() {
        ped_data_set_initial_state(d);
    }
}

// --- hook list ---

pub const HOOKS: &[HookReg] = &[
    HOOK_MAKE_PLAYER_SAFE,
    HOOK_CLEAR,
    HOOK_WORK_OUT_ENERGY_FROM_HUNGER,
    HOOK_UPDATE_CAMERA_WEAPON_MODES,
    HOOK_RE_APPLY_MOVE_ANIMS,
    HOOK_SETUP_PLAYER_PED,
    HOOK_MAKE_CHANGES_FOR_NEW_WEAPON,
    HOOK_CONTROL_BUTTON_SPRINT,
    HOOK_HANDLE_PLAYER_BREATH,
    HOOK_SET_PLAYER_MOVE_BLEND_RATIO,
    HOOK_PROCESS_ANIM_GROUPS,
    HOOK_PROCESS_WEAPON_SWITCH,
    HOOK_SET_INITIAL_STATE,
];
