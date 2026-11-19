//! FRONTIER-2 part 4 (batch `frontier2-9`): small `AI` entries of the `tools/verify/frontier.py` list (task
//! destructors, `MakeAbortable`s, timers and flags), ported from the 1.0 US bytes.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{dtor_669d20, task_complex_dtor};
use crate::car_slow_drag_out::urgent_only_make_abortable;
use crate::common::*;
use sa_stream::bindings::anim_remove_block_ref;
use sa_core::bindings::get_animation_block_index;
use sa_world::bindings::{enter_car_dtor, entity_clean_up_old_reference, fx_kill, ped_set_ped_state, stats_increment_stat};
use crate::bindings::stand_still_process_ped;
use crate::bindings::{in_air_ctor as simple_in_air_ctor, task_complex_ctor, task_simple_ctor};
use sa_anim::bindings::{anim_get_animation, get_animation_block};
use sa_core::bindings::task_new as task_new_ai;
use sa_core::bindings::{find_player_ped, general_get_radian_angle_between_points, general_limit_radian_angle};

orig! {
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskSimpleDie::ProcessPed(CPed*)` (0x6397E0, `ret 4`, AL)
    fn die_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6397E0 => unported;
    /// `sprintf(char*, const char*, ...)` (0x821BB5) with the format alone
    fn sprintf0(buf: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
}

/// `CTaskComplexScreamInCarThenLeave::MakeAbortable(CPed*, int32, const CEvent*)` (0x63C320): priority 2 → the
/// sub-task's `MakeAbortable(ped, 2, event)` (tail jump), else false.
#[hook(addr = 0x63C320, conv = thiscall, name = "CTaskComplexScreamInCarThenLeave::MakeAbortable", expect = "83 7C 24 08 02 75 10",
       unverified = "frontier2-9", route = "L2", hazards = "ax-al-return")]
pub fn scream_in_car_then_leave_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    urgent_only_make_abortable(this, ped, priority, event)
}

/// `CTaskComplexShuffleSeats::MakeAbortable(CPed*, int32, const CEvent*)` (0x63D220): as 0x63C320.
#[hook(addr = 0x63D220, conv = thiscall, name = "CTaskComplexShuffleSeats::MakeAbortable", expect = "83 7C 24 08 02 75 10",
       unverified = "frontier2-9", route = "L2", hazards = "ax-al-return")]
pub fn shuffle_seats_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    urgent_only_make_abortable(this, ped, priority, event)
}

/// `CTaskComplexGetOnBoatSeat::MakeAbortable(CPed*, int32, const CEvent*)` (0x6492E0): as 0x63C320.
#[hook(addr = 0x6492E0, conv = thiscall, name = "CTaskComplexGetOnBoatSeat::MakeAbortable", expect = "83 7C 24 08 02 75 10",
       unverified = "frontier2-9", route = "L2", hazards = "ax-al-return")]
pub fn get_on_boat_seat_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    urgent_only_make_abortable(this, ped, priority, event)
}

/// `CTaskComplexFleePoint::MakeAbortable(CPed*, int32, const CEvent*)` (0x65B420): priority 0 (immediate) first resets
/// the flee timer (+0x24 = -1, +0x28 = now, +0x2C = -1, +0x30 = 1); then the sub-task's `MakeAbortable(ped, priority,
/// event)`.
#[hook(addr = 0x65B420, conv = thiscall, name = "CTaskComplexFleePoint::MakeAbortable", expect = "8B 44 24 08 85 C0 56",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn flee_point_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority == 0 {
        wr::<i32>(this, 0x24, -1);
        wr::<u32>(this, 0x28, g::<u32>(0xB7_CB84).read());
        wr::<i32>(this, 0x2C, -1);
        wr::<u8>(this, 0x30, 1);
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, priority, event)
}

/// The task with vtable 0x86E1DC, `ProcessPed(CPed*)` (0x632B80, `ret 4`, AL): a 2000 ms timer (+8 start, +0xC length,
/// +0x10 set, +0x11 reset requested) started on the first call; a pending reset restarts it; true once `now >= start +
/// length` (unsigned).
#[hook(addr = 0x632B80, conv = thiscall, name = "vt_86e1dc::ProcessPed", expect = "8A 51 10 32 C0",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes,ax-al-return")]
pub fn vt_86e1dc_process_ped(this: GPtr<u8>, _ped: GPtr<u8>) -> u8 {
    let now = || g::<u32>(0xB7_CB84).read();
    if rd::<u8>(this, 0x10) == 0 {
        wr::<u32>(this, 8, now());
        wr::<u32>(this, 0xC, 2000);
        wr::<u8>(this, 0x10, 1);
    }
    if rd::<u8>(this, 0x11) != 0 {
        wr::<u32>(this, 8, now());
        wr::<u8>(this, 0x11, 0);
    }
    u8::from(now() >= rd::<u32>(this, 0xC).wrapping_add(rd::<u32>(this, 8)))
}

/// `CTaskComplexOpenPassengerDoor::~CTaskComplexOpenPassengerDoor` (0x640420): vtable 0x86EB3C, then a tail jump into
/// `CTaskComplexEnterCar`'s destructor (0x63DFA0).
#[hook(addr = 0x640420, conv = thiscall, name = "CTaskComplexOpenPassengerDoor::~CTaskComplexOpenPassengerDoor", expect = "C7 01 3C EB 86 00 E9",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn open_passenger_door_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_EB3C);
    enter_car_dtor(this);
}

/// `CTaskInteriorBeInShop::Destructor` (0x675460): vtable 0x870244, then a tail jump into `CTaskComplexWander`'s
/// destructor (0x669D20).
#[hook(addr = 0x675460, conv = thiscall, name = "CTaskInteriorBeInShop::Destructor", expect = "C7 01 44 02 87 00 E9",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn interior_be_in_shop_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x87_0244);
    dtor_669d20(this);
}

/// `CTaskComplexGangJoinRespond::Destructor` (0x661720): vtable 0x86FB5C; with the anim block referenced (+0xD) its
/// reference is released (`RemoveAnimBlockRef(GetAnimationBlockIndex(0x86F92C))`) and the flag cleared; then
/// `CTaskComplex`'s destructor (0x61A3D0).
#[hook(addr = 0x661720, conv = thiscall, name = "CTaskComplexGangJoinRespond::Destructor", expect = "6A FF 68 98 48 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn gang_join_respond_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_FB5C);
    if rd::<u8>(this, 0xD) != 0 {
        anim_remove_block_ref(get_animation_block_index(GPtr::from_addr(0x86_F92C)) as u32);
        wr::<u8>(this, 0xD, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskSimpleSwim::DestroyFxSystem()` (0x68AA50): kills the swim particle system (+0x5C) when set and clears it.
#[hook(addr = 0x68AA50, conv = thiscall, name = "CTaskSimpleSwim::DestroyFxSystem", expect = "56 8B F1 8B 4E 5C",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn swim_destroy_fx_system(this: GPtr<u8>) {
    let fx = rd::<GPtr<u8>>(this, 0x5C);
    if !fx.is_null() {
        fx_kill(fx);
        wr::<u32>(this, 0x5C, 0);
    }
}

/// `CTaskComplexWalkRoundCar::GoingForDoor()` (0x654720, AL): flag +0xD bit 0, a vehicle (+0x1C) and its +0x429 bit 2.
#[hook(addr = 0x654720, conv = thiscall, name = "CTaskComplexWalkRoundCar::GoingForDoor", expect = "8A 51 0D B0 01",
       unverified = "frontier2-9", route = "L2", hazards = "ax-al-return")]
pub fn walk_round_car_going_for_door(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xD) & 1 == 0 {
        return 0;
    }
    let v = rd::<GPtr<u8>>(this, 0x1C);
    u8::from(!v.is_null() && rd::<u8>(v, 0x429) & 4 != 0)
}

/// `CTaskSimpleDrown::ProcessPed(CPed*)` (0x639900, `ret 4`, AL): on the first call (+0x28 set) the ped's state
/// becomes 0x36 (dead), +0x46C bit 9 is cleared and the player's stat 0xAA (drowned) is incremented by 1.0; then
/// `CTaskSimpleDie::ProcessPed`.
#[hook(addr = 0x639900, conv = thiscall, name = "CTaskSimpleDrown::ProcessPed", expect = "56 8B F1 8A 46 28",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn drown_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x28) != 0 {
        ped_set_ped_state(ped, 0x36);
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF);
        if find_player_ped(-1) == ped {
            stats_increment_stat(0xAA, 1.0);
        }
        wr::<u8>(this, 0x28, 0);
    }
    die_process_ped(this, ped)
}

/// The stand-still task with vtable 0x85B8DC, `ProcessPed(CPed*)` (0x62F520, `ret 4`, AL): `CTaskSimpleStandStill::
/// ProcessPed` (result unused); while the entity at ped +0x584 has model [0x8CD674] or [0x8CD678] (word ids) the ped's
/// aimed heading +0x55C = `LimitRadianAngle(f32 GetRadianAngleBetweenPoints(e.move.x, e.move.y, 0, 0))` and false;
/// otherwise true.
#[hook(addr = 0x62F520, conv = thiscall, name = "vt_85b8dc::ProcessPed", expect = "56 8B 74 24 08 56",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn vt_85b8dc_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    stand_still_process_ped(this, ped);
    let e = rd::<GPtr<u8>>(ped, 0x584);
    if e.is_null() {
        return 1;
    }
    let m = i32::from(rd::<i16>(e, 0x22));
    if m != i32::from(g::<u16>(0x8C_D674).read()) && m != i32::from(g::<u16>(0x8C_D678).read()) {
        return 1;
    }
    let a = sa_math::x87::X87::from_f64_exact(general_get_radian_angle_between_points(rd::<f32>(e, 0x44), rd::<f32>(e, 0x48), 0.0, 0.0).to_f64()).to_f32();
    let h = sa_math::x87::X87::from_f64_exact(general_limit_radian_angle(a)).to_f32();
    wr::<f32>(ped, 0x55C, h);
    0
}

/// `CTaskComplexSunbathe::Destructor` (0x632050): vtable 0x86E0AC; the towel object (+0x34): `CleanUpOldReference` and
/// its +0x13C = 3; the two referenced anim blocks (+0xE / +0x2C, +0xF / +0x30) released and their flags cleared; then
/// `CTaskComplex`'s destructor (0x61A3D0).
#[hook(addr = 0x632050, conv = thiscall, name = "CTaskComplexSunbathe::Destructor", expect = "6A FF 68 18 06 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn sunbathe_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_E0AC);
    let slot = this.byte_add(0x34);
    let towel = rd::<GPtr<u8>>(slot, 0);
    if !towel.is_null() {
        entity_clean_up_old_reference(towel, slot);
        wr::<u8>(rd::<GPtr<u8>>(slot, 0), 0x13C, 3);
    }
    for (flag, block) in [(0xE, 0x2C), (0xF, 0x30)] {
        if rd::<u8>(this, flag) != 0 {
            anim_remove_block_ref(rd::<u32>(this, block));
            wr::<u8>(this, flag, 0);
        }
    }
    task_complex_dtor(this);
}

/// `CDecisionMakerTypesFileLoader::GetGrpDMName(int32 index, char* out)` (0x600880, cdecl): `sprintf(out,
/// ms_GrpDMNames[index])` (the name table 0x8D2378 used as the format, no arguments); returns `sprintf`'s EAX.
#[hook(addr = 0x600880, conv = cdecl, name = "CDecisionMakerTypesFileLoader::GetGrpDMName", expect = "8B 44 24 04 8B 0C 85 78 23 8D 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn get_grp_dm_name(index: i32, out: GPtr<u8>) -> i32 {
    let name = g::<GPtr<u8>>(0x8D_2378usize.wrapping_add((index as isize as usize).wrapping_mul(4))).read();
    sprintf0(out, name)
}


/// `CPointRoute` remove-point (0x5F13A0, thiscall, `ret 4`; name from its caller in
/// `CTaskComplexFollowPedFootsteps::ControlSubTask`): points `index + 1 ..` move down one slot (the count +0 re-read
/// each step), then the count is decremented (also for an index at or past the last point).
#[hook(addr = 0x5F13A0, conv = thiscall, name = "FUN_005f13a0", expect = "8B 01 8B 54 24 04 48",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn point_route_remove_point(this: GPtr<u8>, index: i32) {
    let mut i = index;
    while i < rd::<i32>(this, 0).wrapping_sub(1) {
        let dst = this.byte_add(4 + 12 * i as isize);
        sa_core::field::copy3(dst, dst.byte_add(12));
        i += 1;
    }
    wr::<i32>(this, 0, rd::<i32>(this, 0).wrapping_sub(1));
}

/// `CTaskComplexInAirAndLand::Clone()` (0x67C2E0): a new 0x10-byte `CTaskComplex` with +0xC = (+0xC != 0), +0xD = (+0xD
/// != 0), vtable 0x8704AC and +0xE = 0; null when the pool is full.
#[hook(addr = 0x67C2E0, conv = thiscall, name = "CTaskComplexInAirAndLand::Clone", expect = "6A FF 68 0B 62 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn in_air_and_land_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_ai(0x10);
    if t.is_null() {
        return GPtr::NULL;
    }
    let use_tumble = u8::from(rd::<u8>(this, 0xD) != 0);
    let probe = u8::from(rd::<u8>(this, 0xC) != 0);
    task_complex_ctor(t.addr() as u32);
    wr::<u8>(t, 0xC, probe);
    wr::<u8>(t, 0xD, use_tumble);
    wr::<u32>(t, 0, 0x87_04AC);
    wr::<u8>(t, 0xE, 0);
    t
}

/// `CTaskSimpleInAir::Clone()` (0x67C360): a new 0x3C-byte `CTaskSimpleInAir(bit 0, bit 1, bit 2 of the flags byte
/// +0x24)`; null when the pool is full. (The first flag goes out in a register whose upper bytes the original does not
/// clear; the constructor reads the byte.)
#[hook(addr = 0x67C360, conv = thiscall, name = "CTaskSimpleInAir::Clone", expect = "6A FF 68 2B 62 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn simple_in_air_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_ai(0x3C);
    if t.is_null() {
        return GPtr::NULL;
    }
    let f = u32::from(rd::<u8>(this, 0x24));
    simple_in_air_ctor(t, f & 1, (f >> 1) & 1, (f >> 2) & 1)
}

/// `CTaskSimpleDie::CTaskSimpleDie(const char* anim, const char* block, int32 flags, float blendDelta, float speed)`
/// (0x62FA60, `ret 0x14`): the `CTaskSimple` base (0x61A390), +0x14 / +0x18 / +0x1C = flags, blend delta, speed,
/// vtable 0x86DDBC, +8 = 0, +0xC = 15 (no anim group), +0x10 = 0, +0x20 bits 0-1 cleared, +0x24 = 0, then +0x10 =
/// `GetAnimation(anim, GetAnimationBlock(block))`.
#[hook(addr = 0x62FA60, conv = thiscall, name = "CTaskSimpleDie::Constructor-2", expect = "6A FF 68 58 03 84 00",
       unverified = "frontier2-9", route = "L2", hazards = "heap-writes")]
pub fn simple_die_ctor_named(this: GPtr<u8>, anim: GPtr<u8>, block: GPtr<u8>, flags: u32, blend: u32, speed: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0x14, flags);
    wr::<u32>(this, 0x1C, speed);
    wr::<u32>(this, 0x18, blend);
    wr::<u32>(this, 0, 0x86_DDBC);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0xF);
    wr::<u32>(this, 0x10, 0);
    wr::<u8>(this, 0x20, rd::<u8>(this, 0x20) & 0xFC);
    wr::<u32>(this, 0x24, 0);
    let b = get_animation_block(block);
    let a = anim_get_animation(anim, b);
    wr::<GPtr<u8>>(this, 0x10, a);
    this
}


pub const HOOKS: &[HookReg] = &[
    HOOK_SCREAM_IN_CAR_THEN_LEAVE_MAKE_ABORTABLE,
    HOOK_SHUFFLE_SEATS_MAKE_ABORTABLE,
    HOOK_GET_ON_BOAT_SEAT_MAKE_ABORTABLE,
    HOOK_FLEE_POINT_MAKE_ABORTABLE,
    HOOK_VT_86E1DC_PROCESS_PED,
    HOOK_OPEN_PASSENGER_DOOR_DTOR,
    HOOK_INTERIOR_BE_IN_SHOP_DTOR,
    HOOK_GANG_JOIN_RESPOND_DTOR,
    HOOK_SWIM_DESTROY_FX_SYSTEM,
    HOOK_WALK_ROUND_CAR_GOING_FOR_DOOR,
    HOOK_DROWN_PROCESS_PED,
    HOOK_VT_85B8DC_PROCESS_PED,
    HOOK_SUNBATHE_DTOR,
    HOOK_GET_GRP_DM_NAME,
    HOOK_POINT_ROUTE_REMOVE_POINT,
    HOOK_IN_AIR_AND_LAND_CLONE,
    HOOK_SIMPLE_IN_AIR_CLONE,
    HOOK_SIMPLE_DIE_CTOR_NAMED,
];
