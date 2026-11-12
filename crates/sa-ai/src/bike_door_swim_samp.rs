//! `CTaskSimplePickUpBike::StartAnim` / `ProcessPed`, `CTaskComplexGotoDoorAndOpen::ControlSubTask` and the
//! `CTaskSimpleSwim` destructor, executed by the original under SA:MP 0.3.7 (`catalog/samp-priority.csv`)
//! (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CVehicleAnimGroupData::m_vehicleAnimGroups` (0x94 bytes each)
const VEH_ANIM_GROUPS: u32 = 0xC1CDC0;
/// `CTaskSimplePickUpBike::FinishAnimPickUpBikeCB`
const PICK_UP_BIKE_CB: u32 = 0x6462F0;
/// 0.0f
const K_ZERO: usize = 0x858B50;

orig! {
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn veh_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610, cdecl)
    fn anim_blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CTaskSimplePickUpBike::StartAnim(const CPed*)` (0x64AE90, `ret 4`)
    fn o_pick_up_bike_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64AE90 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(i: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0, cdecl)
    fn anim_remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskSimple::~CTaskSimple` (`FUN_0061a3a0`)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTaskSimple::CTaskSimple` (`FUN_0061a390`)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetEventPriority` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0, cdecl)
    fn clump_get_association(clump: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CPed::RestoreHeadingRate` (0x5DFD60)
    fn ped_restore_heading_rate(p: GPtr<u8>) = thiscall 0x5DFD60 => unported;
    /// `FxSystem_c::Kill` (0x4AA3F0)
    fn fx_system_kill(f: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `FxManager_c::CreateFxSystem(char* name, RwV3d* pos, RwMatrix* parent, bool)` (0x4A9BE0, `ret 0x10`)
    fn fx_manager_create_fx_system(m: GPtr<u8>, name: u32, pos: GPtr<u8>, parent: u32, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::CopyParentMatrix` (0x4AA890)
    fn fx_system_copy_parent_matrix(f: GPtr<u8>) = thiscall 0x4AA890 => unported;
    /// `FxSystem_c::Play` (0x4AA2F0)
    fn fx_system_play(f: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, i: i32) = thiscall 0x681B60 => unported;
    /// `CTaskSimpleRunAnim::CTaskSimpleRunAnim(AssocGroupId, AnimationId, float delta, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: u32, delta: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(int32 moveState, const CVector&, float radius, bool, bool)`
    /// (0x667CD0, `ret 0x14`)
    fn go_to_point_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u8>, radius: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x667CD0 => unported;
    /// `IKChain_c::UpdateEntity(CEntity*)` (0x617E00, `ret 4`)
    fn ik_chain_update_entity(c: GPtr<u8>, e: u32) = thiscall 0x617E00 => unported;
    /// `IKChain_c::UpdateOffset(int32 bone, CVector offset)` (0x617E20, `ret 0x10`; the vector by value)
    fn ik_chain_update_offset(c: GPtr<u8>, bone: u32, x: u32, y: u32, z: u32) = thiscall 0x617E20 => unported;
    /// `IKChain_c::UpdateTarget(bool)` (0x617E50, `ret 4`)
    fn ik_chain_update_target(c: GPtr<u8>, b: u32) = thiscall 0x617E50 => unported;
    /// `CPopulation::IsSunbather(int32 modelIndex)` (0x611760, cdecl; AL)
    fn population_is_sunbather(mi: i32) -> u8 = cdecl 0x611760 => unported;
    /// `CTaskSimpleSwim::CTaskSimpleSwim(const CVector* pos, CPed* ped)` (0x688930, `ret 8`)
    fn swim_ctor(t: GPtr<u8>, pos: u32, ped: u32) -> GPtr<u8> = thiscall 0x688930 => unported;
    /// `CTaskComplexDie::CTaskComplexDie(eWeaponType, AssocGroupId, AnimationId, float delta, float speed, bool,
    /// bool, eFallDir, bool)` (0x630040, `ret 0x24`)
    #[allow(clippy::too_many_arguments)]
    fn complex_die_ctor(t: GPtr<u8>, w: u32, g: u32, a: u32, d: u32, s: u32, b1: u32, b2: u32, dir: u32, b3: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
}

/// `CTaskSimplePickUpBike::StartAnim(const CPed*)` (0x64AE90, `ret 4`): by the door (+0x14) and the bike's roll
/// (its matrix right.z, +0x14 → +8): doors 0xA / 0xB pick 0x15F when right.z is above 0.0 (`test ah, 0x41; je`)
/// else 0x161; other doors 0x160 when below 0.0 (`test ah, 5; jp`) else 0x162. The anim of the bike's anim group
/// is blended at 8.0, kept (+0xC) with `FinishAnimPickUpBikeCB`.
#[hook(addr = 0x64AE90, conv = thiscall, name = "CTaskSimplePickUpBike::StartAnim", expect = "56 8B F1 8B 46 14",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn pick_up_bike_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let door = rd::<u32>(this, 0x14);
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let rz = x(rd::<f32>(rd::<GPtr<u8>>(veh, 0x14), 8)).fcom(x(g::<f32>(K_ZERO).read()));
    let anim = if door == 0xA || door == 0xB {
        if rz.test_any(0x41) { 0x161 } else { 0x15F }
    } else if rz.test_parity(0x05) {
        0x162
    } else {
        0x160
    };
    let id = u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE));
    let ag: GPtr<u8> = GPtr::from_addr(id.wrapping_mul(0x94).wrapping_add(VEH_ANIM_GROUPS) as usize);
    let group = veh_anim_group_get_group(ag, anim);
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim, 0x4100_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    assoc_set_finish_callback(a, PICK_UP_BIKE_CB, this.addr() as u32);
}

/// `CTaskSimplePickUpBike::ProcessPed(CPed*)` (0x64D910, `ret 4`): done (AL = 1) when finished (+8) or without a
/// bike (+0x10). Starts the anim when none plays; then, unless the bike's flag byte +0x614 already has bit 3, sets
/// it once a pick-up anim (0x15F..0x162, assoc +0x2C) is past [0x86F0DC] in its current time (+0x20). AL = 0.
#[hook(addr = 0x64D910, conv = thiscall, name = "CTaskSimplePickUpBike::ProcessPed", expect = "56 8B F1 8A 46 08",
       unverified = "ai7-12", route = "L2", hazards = "ax-al-return,heap-writes", recs = "car-ai")]
pub fn pick_up_bike_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 || rd::<u32>(this, 0x10) == 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        o_pick_up_bike_start_anim(this, ped);
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let fl = rd::<u8>(veh, 0x614);
    if fl & 8 != 0 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() {
        return 0;
    }
    let id = rd::<u16>(a, 0x2C);
    if (0x15F..=0x162).contains(&id) && !x(rd::<f32>(a, 0x20)).fcom(x(g::<f32>(0x86F0DC).read())).test_any(0x41) {
        wr::<u8>(veh, 0x614, fl | 8);
    }
    0
}

/// `CTaskComplexGotoDoorAndOpen::ControlSubTask(CPed*)` (0x66C1F0, `ret 4`): with a timer (+0x30; +0x31 restarts it
/// at now into +0x28) that has run out (`now >= start + +0x2C`, unsigned), or without a door entity (+0xC) and with
/// flag bit 0 of +0x34, the sub-task is asked to abort (`MakeAbortable(ped, 1, null)`); when it agrees, a player's
/// pad 0 gets bit 3 of +0x10E and the result is null. Otherwise the sub-task (+8).
#[hook(addr = 0x66C1F0, conv = thiscall, name = "CTaskComplexGotoDoorAndOpen::ControlSubTask", expect = "56 8B F1 8A 46 30",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes")]
pub fn goto_door_and_open_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let mut stop = false;
    if rd::<u8>(this, 0x30) != 0 {
        if rd::<u8>(this, 0x31) != 0 {
            wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x31, 0);
        }
        let end = rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28));
        stop = g::<u32>(TIME_MS).read() >= end;
    }
    if !stop {
        stop = rd::<u32>(this, 0xC) == 0 && rd::<u8>(this, 0x34) & 1 != 0;
    }
    if stop && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        if ped_is_player(ped) != 0 {
            let pad = pad_get_pad(0);
            wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) | 8);
        }
        return 0;
    }
    rd::<u32>(this, 8)
}

/// `CTaskSimpleSwim::~CTaskSimpleSwim` (0x688A00): `vtbl = 0x870920`; with +9 set releases the anim block of the
/// player's swim block (`(ped [0xB4EA34] +0x58C - 0xB5D4A0) >> 5`, arithmetic); drops the references at +0x38 and
/// +0x20; the `CTaskSimple` destructor.
#[hook(addr = 0x688A00, conv = thiscall, name = "CTaskSimpleSwim::~CTaskSimpleSwim", expect = "6A FF 68 28 6C 84 00",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes")]
pub fn swim_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870920);
    if rd::<u8>(this, 9) != 0 {
        let p = g::<GPtr<u8>>(0xB4EA34).read();
        let idx = (rd::<u32>(p, 0x58C).wrapping_sub(0xB5D4A0) as i32) >> 5;
        anim_remove_anim_block_ref(idx);
    }
    for off in [0x38, 0x20] {
        let slot = this.byte_add(off);
        let e = rd::<GPtr<u8>>(slot, 0);
        if !e.is_null() {
            entity_clean_up_old_reference(e, slot);
        }
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleSwim::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x68B100, `ret 0xC`): an urgent abort
/// (priority 2) blends the ped's anim 3 of its anim group (+0x4D4) at 1000.0, sets +0x534 / +0x538 = 1, blends out
/// the swim anim (+0xC, unless 0xBF) at -1000.0 and restores the heading rate; urgent aborts and events of priority
/// >= 0x47 or of type 9 with +0x40 and +0xC set kill the fx system (+0x5C) and return AL = 1. Otherwise AL = 0.
#[hook(addr = 0x68B100, conv = thiscall, name = "CTaskSimpleSwim::MakeAbortable", expect = "83 7C 24 08 02",
       unverified = "ai7-12", route = "L2", hazards = "ax-al-return,heap-writes", recs = "air-water")]
pub fn swim_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority == 2 {
        anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 0x447A_0000);
        wr::<u32>(ped, 0x534, 1);
        wr::<u32>(ped, 0x538, 1);
        let anim = rd::<u32>(this, 0xC);
        if anim != 0xBF {
            let a = clump_get_association(rd::<u32>(ped, 0x18), anim);
            if !a.is_null() {
                wr::<f32>(a, 0x1C, -1000.0);
            }
        }
        ped_restore_heading_rate(ped);
    } else {
        if event.is_null() {
            return 0;
        }
        if v_event_priority(event) < 0x47
            && !(v_event_type(event) == 9 && rd::<u8>(event, 0x40) != 0 && rd::<u8>(event, 0xC) != 0)
        {
            return 0;
        }
    }
    let fx = rd::<GPtr<u8>>(this, 0x5C);
    if !fx.is_null() {
        fx_system_kill(fx);
        wr::<u32>(this, 0x5C, 0);
    }
    1
}

/// `CTaskSimpleSwim::CreateFxSystem(CPed*, RwMatrix*)` (0x68A9F0, `ret 8`; the ped is unused): `g_fxMan`
/// (0xA9AE80) creates the system named at 0x870954 at a zero offset (a stack vector; Rust local) on the matrix; a
/// created system (+0x5C) copies its parent matrix and plays.
#[hook(addr = 0x68A9F0, conv = thiscall, name = "CTaskSimpleSwim::CreateFxSystem", expect = "83 EC 0C 8B 44 24 14",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes", recs = "air-water")]
pub fn swim_create_fx_system(this: GPtr<u8>, _ped: GPtr<u8>, matrix: u32) {
    let mut v = [0u32; 3];
    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    let fx = fx_manager_create_fx_system(GPtr::from_addr(0xA9AE80), 0x870954, vp, matrix, 0);
    wr::<GPtr<u8>>(this, 0x5C, fx);
    if !fx.is_null() {
        fx_system_copy_parent_matrix(fx);
        fx_system_play(rd::<GPtr<u8>>(this, 0x5C));
    }
}

/// `CTaskSimpleCarJumpOut::CTaskSimpleCarJumpOut(CVehicle*, int32 door, CTaskUtilityLineUpPedWithCar*)` (0x64BE40,
/// `ret 0xC`): `CTaskSimple` base, door +0x18, `vtbl = 0x86EE70`, byte +8 = 0, +0xC = 0, vehicle +0x10, +0x14 = 0,
/// bytes +0x1C / +0x1D = 0, line-up +0x20; with a vehicle the reference is registered and +0x14 = its speed
/// `fsqrt((x·x + y·y) + z·z)` of the move speed (+0x44) as f32.
#[hook(addr = 0x64BE40, conv = thiscall, name = "CTaskSimpleCarJumpOut::Constructor", expect = "6A FF 68 78 30 84 00",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes,x87-transcendental", recs = "car-ai")]
pub fn car_jump_out_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, lineup: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0x18, door);
    wr::<u32>(this, 0, 0x86EE70);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<GPtr<u8>>(this, 0x10, veh);
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x1D, 0);
    wr::<u32>(this, 0x20, lineup);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x10));
        let v = rd::<GPtr<u8>>(this, 0x10);
        let (vx, vy, vz) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)), x(rd::<f32>(v, 0x4C)));
        wr::<f32>(this, 0x14, vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt().to_f32());
    }
    this
}

/// `CTaskComplexGotoDoorAndOpen::CreateNextSubTask(CPed*)` (0x66C0D0, `ret 4`): only after the 0x384 sub-task. The
/// second time (+0x34 bit 1) a player's pad 0 gets bit 3 of +0x10E and the result is null; the first time a ped
/// without secondary task 4 gets `CTaskSimpleRunAnim(0, 0x99, 8.0, false)` there (null when the pool is full), then
/// `CTaskSimpleGoToPoint(4, &+0x1C, 0.2, false, false)` and +0x34 bit 1 is set.
#[hook(addr = 0x66C0D0, conv = thiscall, name = "CTaskComplexGotoDoorAndOpen::CreateNextSubTask", expect = "64 A1 00 00 00 00",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes")]
pub fn goto_door_and_open_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if v_task_type(rd::<GPtr<u8>>(this, 8)) != 0x384 {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x34) & 2 != 0 {
        if ped_is_player(ped) != 0 {
            let pad = pad_get_pad(0);
            wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) | 8);
        }
        return GPtr::NULL;
    }
    if tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 4).is_null() {
        let t = task_new(0x20);
        let r = if t.is_null() { t } else { run_anim_ctor(t, 0, 0x99, 0x4100_0000, 0) };
        tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), r, 4);
    }
    let t = task_new(0x30);
    let r = if t.is_null() { t } else { go_to_point_ctor(t, 4, this.byte_add(0x1C), 0x3E4C_CCCD, 0, 0) };
    wr::<u8>(this, 0x34, rd::<u8>(this, 0x34) | 2);
    r
}

/// `CTaskSimpleIKLookAt::UpdateLookAtInfo(const char*, CPed*, CEntity* target, int32 time, int32 bone, CVector
/// offset, bool, float speed, int32 blendTime, int32)` (0x634050, `ret 0x30`; the name, ped and last argument are
/// unused): replaces the target reference (+0x28; +0x40 = target set), stores bone (+0x2C), offset (+0x30), flag
/// byte (+0x58), time (+8), speed (+0x3C), blend time (+0xC), the end times +0x48 = now + time and +0x50 = now +
/// blend time, +0x4C = 1.0, +0x54 = 0; an active IK chain (+0x10) is updated (entity, offset, target).
#[hook(addr = 0x634050, conv = thiscall, name = "CTaskSimpleIKLookAt::UpdateLookAtInfo", expect = "55 56 8B F1 8B 4E 28",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn ik_look_at_update_look_at_info(this: GPtr<u8>, _name: u32, _ped: u32, target: GPtr<u8>, time: u32, bone: u32,
                                      ox: u32, oy: u32, oz: u32, b: u32, speed: u32, blend: u32, _a12: u32) {
    let slot = this.byte_add(0x28);
    let old = rd::<GPtr<u8>>(slot, 0);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(slot, 0, target);
    if !target.is_null() {
        entity_register_reference(target, slot);
        wr::<u8>(this, 0x40, 1);
    } else {
        wr::<u8>(this, 0x40, 0);
    }
    wr::<u32>(this, 0x2C, bone);
    wr::<u32>(this, 0x30, ox);
    wr::<u32>(this, 0x34, oy);
    wr::<u32>(this, 0x38, oz);
    wr::<u8>(this, 0x58, b as u8);
    wr::<u32>(this, 8, time);
    wr::<u32>(this, 0x3C, speed);
    wr::<u32>(this, 0xC, blend);
    let now = g::<u32>(TIME_MS).read();
    wr::<u32>(this, 0x48, time.wrapping_add(now));
    wr::<u32>(this, 0x4C, 0x3F80_0000);
    wr::<u32>(this, 0x50, now.wrapping_add(blend));
    wr::<u32>(this, 0x54, 0);
    let chain = rd::<GPtr<u8>>(this, 0x10);
    if !chain.is_null() {
        ik_chain_update_entity(chain, rd::<u32>(slot, 0));
        let (x0, y0, z0) = (rd::<u32>(this, 0x30), rd::<u32>(this, 0x34), rd::<u32>(this, 0x38));
        ik_chain_update_offset(rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x2C), x0, y0, z0);
        ik_chain_update_target(rd::<GPtr<u8>>(this, 0x10), 1);
    }
}

/// `CTaskComplexInWater::CreateFirstSubTask(CPed*)` (0x635200, `ret 4`): players and sunbather models swim
/// (`CTaskSimpleSwim(null, null)`); a ped with +0x470 bit 0x100000 drowns (`CTaskComplexDie(0x35, 0, 0x8C, 4.0,
/// 0, ...)`); a member of a group led by a player (leader +0x480 set) swims after the leader; a mission ped
/// (+0x484 == 2) with a 0x384 task swims to that task's target (+0xC, copied to a stack vector first; Rust local);
/// everyone else runs anim 0xE (`CTaskSimpleRunAnim(0, 0xE, 8.0, false)`). Null when the pool is full.
#[hook(addr = 0x635200, conv = thiscall, name = "CTaskComplexInWater::CreateFirstSubTask", expect = "64 A1 00 00 00 00",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes", recs = "air-water")]
pub fn in_water_create_first_sub_task(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if ped_is_player(ped) != 0 || population_is_sunbather(i32::from(rd::<u16>(ped, 0x22) as i16)) != 0 {
        let t = task_new(0x64);
        return if t.is_null() { t } else { swim_ctor(t, 0, 0) };
    }
    if rd::<u32>(ped, 0x470) & 0x10_0000 != 0 {
        let t = task_new(0x28);
        return if t.is_null() { t } else { complex_die_ctor(t, 0x35, 0, 0x8C, 0x4080_0000, 0, 0, 0, 0, 0) };
    }
    let grp = groups_get_peds_group(ped);
    if !grp.is_null() {
        let ms = grp.byte_add(8);
        if !membership_get_leader(ms).is_null() && rd::<u32>(membership_get_leader(ms), 0x480) != 0 {
            let t = task_new(0x64);
            if t.is_null() {
                return t;
            }
            let leader = membership_get_leader(ms);
            return swim_ctor(t, 0, leader.addr() as u32);
        }
    }
    if rd::<u8>(ped, 0x484) == 2 && !intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x384).is_null() {
        let src = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x384).byte_add(0xC);
        let mut v = [rd::<u32>(src, 0), rd::<u32>(src, 4), rd::<u32>(src, 8)];
        let t = task_new(0x64);
        if t.is_null() {
            return t;
        }
        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
        return swim_ctor(t, vp.addr() as u32, 0);
    }
    let t = task_new(0x20);
    if t.is_null() { t } else { run_anim_ctor(t, 0, 0xE, 0x4100_0000, 0) }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PICK_UP_BIKE_START_ANIM,
    HOOK_PICK_UP_BIKE_PROCESS_PED,
    HOOK_GOTO_DOOR_AND_OPEN_CONTROL_SUB_TASK,
    HOOK_SWIM_DTOR,
    HOOK_SWIM_MAKE_ABORTABLE,
    HOOK_SWIM_CREATE_FX_SYSTEM,
    HOOK_CAR_JUMP_OUT_CTOR,
    HOOK_GOTO_DOOR_AND_OPEN_CREATE_NEXT_SUB_TASK,
    HOOK_IK_LOOK_AT_UPDATE_LOOK_AT_INFO,
    HOOK_IN_WATER_CREATE_FIRST_SUB_TASK,
];
