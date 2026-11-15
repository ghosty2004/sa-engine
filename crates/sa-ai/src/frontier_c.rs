//! Task and event constructors (and one assignment) the catalogue had unnamed, reached through the original by ported
//! code (FRONTIER-2 lane C, batch `frontier2-c2`). Classes from the vtables they install (slot 4 / slot 1 type against
//! gta-reversed's `eTaskType` / `eEventType`) and their callers. Ported from the 1.0 US bytes; the SEH frames are not
//! modelled. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

use crate::bindings::task_simple_ctor;
use sa_world::bindings::{entity_register_reference, task_complex_ctor};

orig! {
    /// `CEventKnockOffBike::From(const CEventKnockOffBike&)` (0x4AFDD0, thiscall)
    fn knock_off_bike_from(this: GPtr<u8>, other: GPtr<u8>) = thiscall 0x4AFDD0 => unported;
}

/// The `CEvent` part the event constructors inline: `+4 = 0` (time active), `+8 = 0` (byte).
fn event_base(this: GPtr<u8>) {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
}

/// `CEventSexyVehicle::Constructor(CVehicle*)` (0x4AF010, thiscall, `ret 4`; vtable 0x85B0F8, `GetEventType` 21): the
/// vehicle at +0xC, its reference registered when set. Returns `this`.
#[hook(addr = 0x4AF010, conv = thiscall, name = "CEventSexyVehicle::Constructor", expect = "6A FF 68 88 9D 83 00",
       unverified = "frontier2-c2", route = "L2", hazards = "heap-writes")]
pub fn sexy_vehicle_ctor(this: GPtr<u8>, vehicle: GPtr<u8>) -> GPtr<u8> {
    event_base(this);
    wr::<u32>(this, 0, 0x85B0F8);
    wr::<GPtr<u8>>(this, 0xC, vehicle);
    if !vehicle.is_null() {
        entity_register_reference(vehicle, this.byte_add(0xC));
    }
    this
}

/// `CEventVehicleThreat::Constructor(CVehicle*)` (0x4AFB50, thiscall, `ret 4`; vtable 0x85B298, `GetEventType` 30): the
/// editable-response fields (+0xC = 1, +0xE = 200, +0x10 = -1), the vehicle at +0x14, its reference registered when
/// set. Returns `this`.
#[hook(addr = 0x4AFB50, conv = thiscall, name = "CEventVehicleThreat::Constructor", expect = "6A FF 68 08 9F 83 00",
       unverified = "frontier2-c2", route = "L2", hazards = "heap-writes")]
pub fn vehicle_threat_ctor(this: GPtr<u8>, vehicle: GPtr<u8>) -> GPtr<u8> {
    event_base(this);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 200);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<u32>(this, 0, 0x85B298);
    wr::<GPtr<u8>>(this, 0x14, vehicle);
    if !vehicle.is_null() {
        entity_register_reference(vehicle, this.byte_add(0x14));
    }
    this
}

/// `CEventKnockOffBike::operator=(const CEventKnockOffBike&)` (0x4B4AA0, thiscall, `ret 4`): `From(other)` (0x4AFDD0);
/// returns `this`.
#[hook(addr = 0x4B4AA0, conv = thiscall, name = "CEventKnockOffBike::operator=", expect = "8B 44 24 04 56 50 8B F1",
       unverified = "frontier2-c2", route = "L2", hazards = "heap-writes")]
pub fn knock_off_bike_assign(this: GPtr<u8>, other: GPtr<u8>) -> GPtr<u8> {
    knock_off_bike_from(this, other);
    this
}

/// `CTaskComplexPresentIDToCop::Constructor(CPed* cop)` (0x6591D0, thiscall, `ret 4`): the `CTaskComplex` part, vtable
/// 0x86F544, the cop at +0xC with its reference registered (no null test). Returns `this`.
#[hook(addr = 0x6591D0, conv = thiscall, name = "CTaskComplexPresentIDToCop::Constructor", expect = "6A FF 68 08 3C 84 00",
       unverified = "frontier2-c2", route = "L2", hazards = "heap-writes")]
pub fn present_id_to_cop_ctor(this: GPtr<u8>, cop: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0, 0x86F544);
    wr::<GPtr<u8>>(this, 0xC, cop);
    entity_register_reference(cop, this.byte_add(0xC));
    this
}

/// `CTaskInteriorSitOnChair::Constructor(int32 duration, InteriorInfo_t* info, bool)` (0x675C30, thiscall, `ret 0xC`):
/// the `CTaskSimple` part, vtable 0x870314; `+8 = duration`, `+0xC = info`, `+0x10 = flag` (byte), `+0x18 = -1`, the
/// anim (+0x14), +0x1C, +0x20 and the bytes +0x24, +0x25, +0x28..+0x2A cleared. Returns `this`.
#[hook(addr = 0x675C30, conv = thiscall, name = "CTaskInteriorSitOnChair::Constructor", expect = "56 8B F1 E8 58 47 FA FF",
       unverified = "frontier2-c2", route = "L2", hazards = "heap-writes")]
pub fn interior_sit_on_chair_ctor(this: GPtr<u8>, duration: u32, info: u32, flag: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x870314);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u8>(this, 0x24, 0);
    wr::<u8>(this, 0x25, 0);
    wr::<u32>(this, 8, duration);
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 0x29, 0);
    wr::<u8>(this, 0x28, 0);
    wr::<u8>(this, 0x2A, 0);
    wr::<u32>(this, 0xC, info);
    wr::<u8>(this, 0x10, flag as u8);
    wr::<i32>(this, 0x18, -1);
    this
}

/// `CTaskSimpleDuckWhileShotsWhizzing::Constructor(uint16 duration)` (0x692680, thiscall, `ret 4`): the `CTaskSimpleDuck`
/// fields inlined (vtable 0x870BC4 first: +0x18 / +0x19 = 0, +0x1A / +0x1B = 1, +0x1C / +0x20 = 0, +0x24 = 0, +0x25 =
/// 0xFF, the anims +0x10 / +0x14 cleared, the start time +8 = `CTimer::m_snTimeInMilliseconds`, the duration word +0xC,
/// +0xE = 0xFFFF), then vtable 0x870BEC. Returns `this`.
#[hook(addr = 0x692680, conv = thiscall, name = "CTaskSimpleDuckWhileShotsWhizzing::Constructor",
       expect = "56 8B F1 E8 08 7D F8 FF", unverified = "frontier2-c2", route = "L2", hazards = "heap-writes")]
pub fn duck_while_shots_whizzing_ctor(this: GPtr<u8>, duration: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x870BC4);
    wr::<u8>(this, 0x18, 0);
    wr::<u8>(this, 0x19, 0);
    wr::<u8>(this, 0x1A, 1);
    wr::<u8>(this, 0x1B, 1);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u8>(this, 0x24, 0);
    wr::<u8>(this, 0x25, 0xFF);
    let now = g::<u32>(0xB7CB84).read();
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 8, now);
    wr::<u16>(this, 0xC, duration as u16);
    wr::<u16>(this, 0xE, 0xFFFF);
    wr::<u32>(this, 0, 0x870BEC);
    this
}

/// The script attractor record's ped-type test (0x5FE960, thiscall, `ret 4`; unnamed; the counterpart of the claim
/// 0x492CB0): without the restriction flag (+0x20) true; else true when one of the four slots (+0) holds -2 with
/// ped type (+0x10 + 4i) equal to the argument. AL.
#[hook(addr = 0x5FE960, conv = thiscall, name = "FUN_005fe960", expect = "8A 41 20 84 C0 75 05",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn script_attractor_accepts_ped_type(this: GPtr<u8>, ped_type: u32) -> u8 {
    if rd::<u8>(this, 0x20) == 0 {
        return 1;
    }
    for i in 0..4isize {
        if rd::<i32>(this, 4 * i) == -2 && rd::<u32>(this, 0x10 + 4 * i) == ped_type {
            return 1;
        }
    }
    0
}

/// `CTaskComplexGoPickUpEntity::Constructor(CEntity* entity, int32 animGroup)` (0x6919C0, thiscall, `ret 8`): the
/// `CTaskComplex` part, vtable 0x870B98, the entity (+0xC; its reference registered when set), -999.0 at +0x10 and +0x1C,
/// +0x14 / +0x18 / +0x20 / +0x24 / +0x28 = 0, the anim group +0x2C, the block-held byte +0x30 = 0. Returns `this`.
#[hook(addr = 0x6919C0, conv = thiscall, name = "CTaskComplexGoPickUpEntity::Constructor", expect = "6A FF 68 C8 72 84 00",
       unverified = "frontier2-c3", route = "L2", hazards = "heap-writes")]
pub fn go_pick_up_entity_ctor(this: GPtr<u8>, entity: GPtr<u8>, anim_group: u32) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<GPtr<u8>>(this, 0xC, entity);
    wr::<u32>(this, 0, 0x870B98);
    wr::<u32>(this, 0x10, 0xC479_C000);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u32>(this, 0x1C, 0xC479_C000);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x2C, anim_group);
    if !entity.is_null() {
        entity_register_reference(entity, this.byte_add(0xC));
    }
    wr::<u32>(this, 0x28, 0);
    wr::<u8>(this, 0x30, 0);
    this
}

/// `CTaskComplexReactToAttack::Constructor(a, CPed* attacker, b, c, d)` (0x620B00, thiscall, `ret 0x14`): the
/// `CTaskComplex` part, vtable 0x86D868, `+0x10 = a`, `+0x18 = b`, `+0x1C = c`, `+0x20 = d`, the bytes +0xC / +0xD = 0,
/// the attacker at +0x14 with its reference registered when set. Returns `this`.
#[hook(addr = 0x620B00, conv = thiscall, name = "CTaskComplexReactToAttack::Constructor", expect = "6A FF 68 B8 F5 83 00",
       unverified = "frontier2-c3", route = "L2", hazards = "heap-writes")]
pub fn react_to_attack_ctor(this: GPtr<u8>, a: u32, attacker: GPtr<u8>, b: u32, c: u32, d: u32) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0x18, b);
    wr::<u32>(this, 0x10, a);
    wr::<u32>(this, 0x1C, c);
    wr::<u32>(this, 0, 0x86D868);
    wr::<u8>(this, 0xC, 0);
    wr::<u8>(this, 0xD, 0);
    wr::<GPtr<u8>>(this, 0x14, attacker);
    wr::<u32>(this, 0x20, d);
    if !attacker.is_null() {
        entity_register_reference(attacker, this.byte_add(0x14));
    }
    this
}

/// `CPointRoute::Reverse()` (0x667720, thiscall; unnamed): the `+0` count of 12-byte points (+4) reversed in place
/// (pairs swapped from both ends while the front index is below the back one).
#[hook(addr = 0x667720, conv = thiscall, name = "CPointRoute::Reverse", expect = "8B 01 83 EC 0C 56 33 F6",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn point_route_reverse(this: GPtr<u8>) {
    let mut j = rd::<i32>(this, 0).wrapping_sub(1);
    let mut i = 0i32;
    if j <= 0 {
        return;
    }
    loop {
        let a = this.byte_add(4 + 12 * i as isize);
        let b = this.byte_add(4 + 12 * j as isize);
        let t: [u32; 3] = core::array::from_fn(|k| rd::<u32>(a, 4 * k as isize));
        for k in 0..3isize {
            wr::<u32>(a, 4 * k, rd::<u32>(b, 4 * k));
        }
        for (k, v) in t.iter().enumerate() {
            wr::<u32>(b, 4 * k as isize, *v);
        }
        i += 1;
        j -= 1;
        if i >= j {
            return;
        }
    }
}

/// `CTaskComplexAvoidEntity::Constructor(int32 moveState, CEntity* entity, const CVector& pos)` (0x66AA20, thiscall,
/// `ret 0xC`): the `CTaskComplex` part, vtable 0x86FF00, `+0x10 = moveState`, the entity at +0xC (registered when set),
/// the position copied to +0x20 and +0x2C, +0x44 / +0x48 = 0, the bytes +0x4C / +0x4D = 0 and bit 0 of +0x50 cleared.
/// Returns `this`.
#[hook(addr = 0x66AA20, conv = thiscall, name = "FUN_0066aa20", expect = "6A FF 68 78 50 84 00",
       unverified = "frontier2-c4", route = "L2", hazards = "heap-writes")]
pub fn avoid_entity_ctor(this: GPtr<u8>, move_state: u32, entity: GPtr<u8>, pos: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0x10, move_state);
    wr::<GPtr<u8>>(this, 0xC, entity);
    wr::<u32>(this, 0, 0x86FF00);
    for base in [0x20isize, 0x2C] {
        for k in 0..3isize {
            wr::<u32>(this, base + 4 * k, rd::<u32>(pos, 4 * k));
        }
    }
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x48, 0);
    wr::<u8>(this, 0x4C, 0);
    wr::<u8>(this, 0x4D, 0);
    wr::<u8>(this, 0x50, rd::<u8>(this, 0x50) & 0xFE);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexFallAndStayDown::CreateSubTask(int32 taskType)` (0x678BD0, thiscall, `ret 4`): type 0xCF → a new 0x20-byte
/// task (`CTask::operator new`; NULL stays NULL): the `CTaskSimple` part, vtable 0x870430, the fall parameters +0xC /
/// +0x10 taken from this task's +0xC / +0x10 (read before the constructor), +0x18 / +0x1C = 99999999, +8 = 0, +0x14 = 0.
/// Any other type → NULL.
#[hook(addr = 0x678BD0, conv = thiscall, name = "FUN_00678bd0", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-c4", route = "L2", hazards = "heap-writes")]
pub fn fall_and_stay_down_create_sub_task(this: GPtr<u8>, task_type: i32) -> GPtr<u8> {
    if task_type != 0xCF {
        return GPtr::NULL;
    }
    let t = task_new(0x20);
    if t.is_null() {
        return GPtr::NULL;
    }
    let (b, a) = (rd::<u32>(this, 0x10), rd::<u32>(this, 0xC));
    task_simple_ctor(t);
    wr::<u32>(t, 0xC, a);
    wr::<u32>(t, 0x18, 0x05F5_E0FF);
    wr::<u32>(t, 0x1C, 0x05F5_E0FF);
    wr::<u32>(t, 0x10, b);
    wr::<u32>(t, 0, 0x870430);
    wr::<u8>(t, 8, 0);
    wr::<u32>(t, 0x14, 0);
    t
}

/// The extinguish-fire task's fire search (0x659990, thiscall, `ret 4`; unnamed): `CFireManager::FindNearestFire(&pos,
/// false, true)` (gFireManager 0xB71F80) from the ped's position; a fire whose squared distance (x87: `dz² + dx²` then
/// `+ dy²`) is below 100 is returned, else NULL (NaN → NULL).
#[hook(addr = 0x659990, conv = thiscall, name = "FUN_00659990", expect = "56 8B 74 24 08 8B 46 14",
       unverified = "frontier2-c4", route = "L2", hazards = "none")]
pub fn extinguish_fire_find_fire(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let fire = find_nearest_fire(GPtr::from_addr(0xB71F80), sa_world::world::common::entity_pos(ped), 0, 1);
    if fire.is_null() {
        return GPtr::NULL;
    }
    let p = sa_world::world::common::entity_pos(ped);
    let d = |k: isize| X87::from_f32(rd::<f32>(p, 4 * k)).fsub_m(rd::<f32>(fire, 4 + 4 * k));
    let (dx, dy, dz) = (d(0), d(1), d(2));
    let sum = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if sum.fcom(X87::from_f32(100.0)).test_parity(5) {
        return GPtr::NULL;
    }
    fire
}

use sa_math::x87::X87;
use crate::bindings::task_new;
use sa_world::bindings::fire_find_nearest_fire as find_nearest_fire;

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_AVOID_ENTITY_CTOR,
    HOOK_FALL_AND_STAY_DOWN_CREATE_SUB_TASK,
    HOOK_EXTINGUISH_FIRE_FIND_FIRE,
    HOOK_GO_PICK_UP_ENTITY_CTOR,
    HOOK_REACT_TO_ATTACK_CTOR,
    HOOK_POINT_ROUTE_REVERSE,
    HOOK_SCRIPT_ATTRACTOR_ACCEPTS_PED_TYPE,
    HOOK_SEXY_VEHICLE_CTOR,
    HOOK_VEHICLE_THREAT_CTOR,
    HOOK_KNOCK_OFF_BIKE_ASSIGN,
    HOOK_PRESENT_ID_TO_COP_CTOR,
    HOOK_INTERIOR_SIT_ON_CHAIR_CTOR,
    HOOK_DUCK_WHILE_SHOTS_WHIZZING_CTOR,
];
