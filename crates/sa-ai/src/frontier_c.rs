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

// ---- FRONTIER-2 turn 2, lane C (batch `frontier2-c5`) ----

orig! {
    /// `CPatrolRoute::operator new` (0x41B810, cdecl: `CPatrolRoutePool::New` on 0xB744B4)
    fn patrol_route_new(size: u32) -> GPtr<u8> = cdecl 0x41B810 => unported;
    /// `CPatrolRoute::CPatrolRoute()` (0x66D440)
    fn patrol_route_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x66D440 => crate::mvpc_ai1::init_c18db8_ctor;
    /// `CPatrolRoute::operator=(const CPatrolRoute&)` (0x66D4B0)
    fn patrol_route_assign(this: GPtr<u8>, other: GPtr<u8>) -> u32 = thiscall 0x66D4B0 => patrol_route_assign_port;
}

/// `CPatrolRoute::operator=(const CPatrolRoute& other)` (0x66D4B0, thiscall, `ret 4`; `FUN_0066d4b0` in the catalogue;
/// FRONTIER-2 lane C turn 4): the count (+0) copied; then for `i` from 0 while `i < this->count` (signed, re-read): the
/// position `+0x144 + 0xC i` (three dwords) and the two C strings at `+4 + 0x28 i` and `+0x1C + 0x28 i` (byte copies up
/// to the NUL) from `other`. The SecuROM fragment 0x401764 only zeroes the loop counter, which lives in the argument's
/// stack slot. EAX: `other` when the count is not positive, else the final counter.
#[hook(addr = 0x66D4B0, conv = thiscall, name = "FUN_0066d4b0", expect = "83 EC 08 8B 44 24 0C",
       unverified = "frontier2-c10", route = "L2", hazards = "none")]
pub fn patrol_route_assign_port(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let n = rd::<i32>(other, 0);
    wr::<i32>(this, 0, n);
    if n <= 0 {
        return other.addr() as u32;
    }
    let copy_str = |off: isize| {
        let mut k = 0;
        loop {
            let c = rd::<u8>(other, off + k);
            wr::<u8>(this, off + k, c);
            k += 1;
            if c == 0 {
                break;
            }
        }
    };
    let mut i: i32 = 0;
    loop {
        let v = 0x144 + 0xC * i as isize;
        for k in 0..3 {
            wr::<u32>(this, v + 4 * k, rd::<u32>(other, v + 4 * k));
        }
        copy_str(4 + 0x28 * i as isize);
        copy_str(0x1C + 0x28 * i as isize);
        i += 1;
        if i >= rd::<i32>(this, 0) {
            return i as u32;
        }
    }
}

/// `CTaskComplexBeCop::Constructor(int32 moveState, uint8 dir, CTaskComplexGoToPointAndStandStill* task)` (stub 0x4993D0
/// → .HOODLUM 0x156E930, thiscall, `ret 0xC`): `CTaskComplexWander::Constructor(this, moveState, dir, true,
/// [0x86FCB0])`, +0x2C / +0x30 = 0, the bytes +0x34 / +0x35 = 0, +0x38..+0x48 = 0, the bytes +0x4C / +0x4D = 0, the
/// task at +0x28, vtable 0x85A428 (`GetTaskType` 0x450 TASK_COMPLEX_BE_COP). Returns `this`.
#[hook(addr = 0x4993D0, conv = thiscall, name = "CTaskComplexBeCop::Constructor", expect = "E9 5B 55 0D 01",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn be_cop_ctor(this: GPtr<u8>, move_state: u32, dir: u32, task: GPtr<u8>) -> GPtr<u8> {
    wander_ctor(this, move_state, dir, 1, g::<u32>(0x86FCB0).read());
    wr::<u32>(this, 0x2C, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u8>(this, 0x34, 0);
    wr::<u8>(this, 0x35, 0);
    for off in [0x38isize, 0x3C, 0x40, 0x44, 0x48] {
        wr::<u32>(this, off, 0);
    }
    wr::<u8>(this, 0x4C, 0);
    wr::<u8>(this, 0x4D, 0);
    wr::<GPtr<u8>>(this, 0x28, task);
    wr::<u32>(this, 0, 0x85A428);
    this
}

/// `CTaskComplexFollowPatrolRoute::Constructor(int32 moveState, const CPatrolRoute& route, int16 mode, float
/// targetRadius, float slowDownDistance)` (0x674930, thiscall, `ret 0x14`): the `CTaskComplex` part, +0x10 = moveState
/// (word), bit 1 of +0x20 cleared, vtable 0x870114, +0xC = mode (word); a new `CPatrolRoute` (0x41B810, 0x1A4 bytes;
/// constructed by 0x66D440 when the pool gave one) at +0x1C, assigned from `route` (0x66D4B0, also on NULL); +0x14 /
/// +0x18 = the two floats, bit 0 of +0x20 set, the words +0x12 / +0xE = 0. Returns `this`. The SEH frame is not
/// modelled.
#[hook(addr = 0x674930, conv = thiscall, name = "CTaskComplexFollowPatrolRoute::Constructor", expect = "6A FF 68 E3 5A 84 00",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn follow_patrol_route_ctor(this: GPtr<u8>, move_state: u32, route: GPtr<u8>, mode: u32, radius: u32, slow: u32) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u16>(this, 0x10, move_state as u16);
    let flags = rd::<u32>(this, 0x20) & !2;
    wr::<u32>(this, 0, 0x870114);
    wr::<u16>(this, 0xC, mode as u16);
    wr::<u32>(this, 0x20, flags);
    let mut r = patrol_route_new(0x1A4);
    if !r.is_null() {
        r = patrol_route_ctor(r);
    }
    wr::<GPtr<u8>>(this, 0x1C, r);
    patrol_route_assign(r, route);
    wr::<u32>(this, 0x14, radius);
    wr::<u32>(this, 0x18, slow);
    wr::<u32>(this, 0x20, rd::<u32>(this, 0x20) | 1);
    wr::<u16>(this, 0x12, 0);
    wr::<u16>(this, 0xE, 0);
    this
}

/// The avoid-entity task's look-at release (0x66AD10, thiscall, `ret 4`; unnamed, called from its `ControlSubTask`):
/// with bit 0 of +0x50 set and `IKChainManager_c::IsLooking(ped)` (g_ikChainMan 0xC15448), `AbortLookAt(ped, 250)`.
#[hook(addr = 0x66AD10, conv = thiscall, name = "FUN_0066ad10", expect = "F6 41 50 01 74 25",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn avoid_entity_release_look(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x50) & 1 == 0 {
        return;
    }
    if ik_is_looking(GPtr::from_addr(IK_CHAIN_MAN), ped) != 0 {
        ik_abort_look_at(GPtr::from_addr(IK_CHAIN_MAN), ped, 250);
    }
}

/// `g_ikChainMan` (IKChainManager_c)
const IK_CHAIN_MAN: usize = 0xC15448;

/// The `CTaskSimple` part the two constructors of the looped-middle anim task (vtable 0x86D594) share, and its flag
/// byte: bit 2 of +0xC = `hold & 1`, bits 0-1 cleared with it, the high bits kept.
fn looped_anim_flags(this: GPtr<u8>, hold: u32) -> u8 {
    task_simple_ctor(this);
    ((hold as u8 & 1) << 2) | (rd::<u8>(this, 0xC) & 0xF8)
}

/// The looped-middle anim task's constructor by ids (0x61AC90, thiscall, `ret 0x1C`; vtable 0x86D594, not in
/// gta-reversed): `(group, anim, blendDelta, a3, a4, a5, hold)` → +0x10 / +0x14 / +0x48 / +0x4C / +0x50 / +0x5C, the
/// flag byte (`looped_anim_flags`), +8 = 0, the hierarchy +0x44 and the flags +0x18 = 0, +0x60 / +0x64 = 0, the bytes
/// +0x68 / +0x69 = 0, `GetTaskType` word +0x6C = 0x1A8 (TASK_SIMPLE_ANIM_LOOPED_MIDDLE). Returns `this`. Caller: its
/// `Clone` 0x61B890.
#[hook(addr = 0x61AC90, conv = thiscall, name = "FUN_0061ac90", expect = "56 8B F1 E8 F8 F6 FF FF",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn looped_anim_ctor_ids(this: GPtr<u8>, group: u32, anim: u32, blend: u32, a3: u32, a4: u32, a5: u32, hold: u32) -> GPtr<u8> {
    let f = looped_anim_flags(this, hold);
    wr::<u8>(this, 0xC, f);
    wr::<u32>(this, 0x10, group);
    wr::<u32>(this, 0x14, anim);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0x48, blend);
    wr::<u32>(this, 0, 0x86D594);
    wr::<u32>(this, 0x60, 0);
    wr::<u32>(this, 0x64, 0);
    wr::<u8>(this, 0x68, 0);
    wr::<u8>(this, 0x69, 0);
    wr::<u32>(this, 0x4C, a3);
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u16>(this, 0x6C, 0x1A8);
    wr::<u32>(this, 0x50, a4);
    wr::<u32>(this, 0x5C, a5);
    this
}

/// The looped-middle anim task's constructor by names (0x61AD10, thiscall, `ret 0x20`; vtable 0x86D594): `(name,
/// group name, flags, blendDelta, a4, a5, a6, hold)` → the flags +0x18, +0x48, +0x4C, +0x50, +0x5C and the fields of
/// `looped_anim_ctor_ids`; the names copied (byte loops, NUL included) to +0x1C and +0x34;
/// `CAnimManager::GetAnimation(+0x1C, GetAnimationBlock(+0x34))` at +0x44 (no NULL test of the block); +0x10 = 0,
/// +0x14 = 0xBF. Returns `this`. The SEH frame is not modelled.
#[hook(addr = 0x61AD10, conv = thiscall, name = "FUN_0061ad10", expect = "6A FF 68 48 F1 83 00",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn looped_anim_ctor_names(this: GPtr<u8>, name: GPtr<u8>, group: GPtr<u8>, flags: u32, blend: u32, a4: u32, a5: u32,
                              a6: u32, hold: u32) -> GPtr<u8> {
    let f = looped_anim_flags(this, hold);
    wr::<u32>(this, 8, 0);
    wr::<u8>(this, 0xC, f);
    wr::<u32>(this, 0x18, flags);
    wr::<u32>(this, 0x48, blend);
    wr::<u32>(this, 0, 0x86D594);
    wr::<u32>(this, 0x60, 0);
    wr::<u32>(this, 0x64, 0);
    wr::<u8>(this, 0x68, 0);
    wr::<u8>(this, 0x69, 0);
    wr::<u32>(this, 0x50, a5);
    wr::<u32>(this, 0x5C, a6);
    wr::<u16>(this, 0x6C, 0x1A8);
    wr::<u32>(this, 0x4C, a4);
    copy_cstr(this.byte_add(0x1C), name);
    copy_cstr(this.byte_add(0x34), group);
    let block = get_animation_block(this.byte_add(0x34));
    wr::<GPtr<u8>>(this, 0x44, anim_get_animation(this.byte_add(0x1C), block));
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0xBF);
    this
}

/// The original's inline `strcpy` loop: bytes up to and including the NUL.
fn copy_cstr(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = rd::<u8>(src, i);
        wr::<u8>(dst, i, c);
        if c == 0 {
            return;
        }
        i += 1;
    }
}

/// The looped-middle anim task's anim start (0x61ADF0, thiscall, `ret 4`; called from its `ProcessPed` 0x61BCB0): with a
/// hierarchy (+0x44) `CAnimManager::BlendAnimation(ped clump, hierarchy, flags +0x18, blendDelta +0x48)`, else
/// `BlendAnimation(ped clump, group +0x10, anim +0x14, blendDelta)`; the association at +8 gets the finish callback
/// 0x61A8A0 (`CTaskSimpleAnim::FinishRunAnimCB`) with `this`.
#[hook(addr = 0x61ADF0, conv = thiscall, name = "FUN_0061adf0", expect = "56 8B F1 8B 46 44",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn looped_anim_start(this: GPtr<u8>, ped: GPtr<u8>) {
    let hier = rd::<GPtr<u8>>(this, 0x44);
    let blend = rd::<f32>(this, 0x48);
    let clump = rd::<GPtr<u8>>(ped, 0x18);
    let a = if !hier.is_null() {
        anim_blend_animation_hier(clump, hier, rd::<u32>(this, 0x18), blend)
    } else {
        anim_blend_animation(clump, rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), blend)
    };
    wr::<GPtr<u8>>(this, 8, a);
    assoc_set_finish_callback(a, 0x61A8A0, this.addr() as u32);
}

/// The be-kicked-on-ground task's anim start (0x61FD20, thiscall, `ret 4`; vtable 0x86D7D8, `GetTaskType` 0x3EF
/// TASK_SIMPLE_BE_KICKED_ON_GROUND, called from its `ProcessPed` 0x61FE00). Without `CLocalisation::KickingWhenDown()`
/// the task is done (byte +8 = 1). Otherwise anim 0x27 (group 0) when the ped's clump has an association with flag
/// 0x800, else 0x24, blended at 8.0 into +0xC, its time set to 0, its flag byte +0x2E: bit 0 set, bit 3 cleared. When
/// `KickingWhenDown()` (asked again) the matrix of the ped's bone (`+0x490 → +0x14`) in the skin hierarchy transforms a
/// local zero vector (`RwV3dTransformPoints`, result unused). Then the finish callback 0x61FD10 with `this`.
#[hook(addr = 0x61FD20, conv = thiscall, name = "FUN_0061fd20", expect = "83 EC 0C 57 8B F9",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn kicked_on_ground_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    if localisation_kicking_when_down() == 0 {
        wr::<u8>(this, 8, 1);
        return;
    }
    let has = !clump_get_first_association_4d6a70(rd::<GPtr<u8>>(ped, 0x18), 0x800).is_null();
    let anim = if has { 0x27 } else { 0x24 };
    let a = anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, anim, 8.0);
    wr::<GPtr<u8>>(this, 0xC, a);
    assoc_set_current_time(a, 0.0);
    let a = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    let a = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xF7);
    if localisation_kicking_when_down() != 0 {
        let mut v = [0f32; 3];
        let vp: GPtr<f32> = GPtr::from_raw(&raw mut v).cast();
        let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
        let idx = rp_hanim_id_get_index(h, rd::<u32>(rd::<GPtr<u8>>(ped, 0x490), 0x14));
        let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add(idx.wrapping_shl(6) as isize);
        rw_v3d_transform_points(vp, vp, 1, m);
    }
    assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0xC), 0x61FD10, this.addr() as u32);
}

/// The kill-all-threats task's threat scan (0x6221D0, thiscall, `ret 4`; vtable 0x86DA1C, unnamed, called from its
/// `CreateFirstSubTask` / `CreateNextSubTask`): a local acquaintance-scanner state (result 0; timer +0 / +4 = 0, bytes
/// +8 / +9 = 0, byte +0xC = 1, bytes +0xD / +0xE = 0) runs `FUN_00607a90(ped, 4, ped intelligence + 0x130, 16, &result,
/// &ped)` (the original's own argument slot as the last out parameter); returns the result.
#[hook(addr = 0x6221D0, conv = thiscall, name = "FUN_006221d0", expect = "83 EC 14 33 C0",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes,uninit-locals,stack-residue",
       note = "the scanner state has uninitialised bytes (+0xA, +0xB, +0xF); the argument slot is an out parameter")]
pub fn kill_all_threats_scan(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut l = [0u32; 5];
    let lp: GPtr<u8> = GPtr::from_raw(&raw mut l).cast();
    wr::<u8>(lp, 0x10, 1);
    let mut arg = ped;
    let ap: GPtr<u8> = GPtr::from_raw(&raw mut arg).cast();
    let list = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x130);
    crate::mvpc_ai1::acq_scanner_scan_entities(lp.byte_add(4), ped, 4, list, 0x10, lp, ap);
    rd::<GPtr<u8>>(lp, 0)
}

/// A `CTaskComplexSeekEntity` constructor (0x66CA40, thiscall, `ret 0x20`; vtable 0x87004C, `GetTaskType` 0x38B
/// TASK_COMPLEX_SEEK_ENTITY, the calculator vtable 0x86FD18 at +0x40; unnamed instantiation, callers 0x671330 and
/// 0x6749F0): the `CTaskComplex` part, the entity +0xC, +0x10 / +0x14 / +0x18 / +0x1C / +0x24 from the arguments, +0x20,
/// +0x28, +0x2C, +0x34, +0x38, +0x44..+0x4C = 0, the bytes +0x30 / +0x31 / +0x3C / +0x3D = 0; flag byte +0x54: bit 0 =
/// `a6 & 1`, bit 1 = `a7 & 1`, bits 2-3 cleared; +0x50 = 6; the entity's reference registered when set. Returns `this`.
/// The SEH frame is not modelled.
#[hook(addr = 0x66CA40, conv = thiscall, name = "FUN_0066ca40", expect = "6A FF 68 F3 52 84 00",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn seek_entity_ctor(this: GPtr<u8>, entity: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32)
                        -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<GPtr<u8>>(this, 0xC, entity);
    wr::<u32>(this, 0x10, a1);
    wr::<u32>(this, 0x14, a2);
    wr::<u32>(this, 0x18, a3);
    wr::<u32>(this, 0, 0x87004C);
    wr::<u32>(this, 0x1C, a4);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0x24, a5);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x2C, 0);
    wr::<u8>(this, 0x30, 0);
    wr::<u8>(this, 0x31, 0);
    wr::<u32>(this, 0x34, 0);
    wr::<u32>(this, 0x38, 0);
    wr::<u8>(this, 0x3C, 0);
    wr::<u8>(this, 0x3D, 0);
    wr::<u32>(this, 0x40, 0x86FD18);
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x48, 0);
    wr::<u32>(this, 0x4C, 0);
    let flags = ((a7 as u8 & 1) << 1) | (rd::<u8>(this, 0x54) & 0xF0) | (a6 as u8 & 1);
    wr::<u8>(this, 0x54, flags);
    wr::<u32>(this, 0x50, 6);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0xC));
    }
    this
}

/// The group default task allocators' destructors (thiscall; the targets of the
/// `__dtor_CPedGroupDefaultTaskAllocators__ms_*Allocator` stubs, catalogued by the CAT-12 tail-target split): each
/// stores the base `CPedGroupDefaultTaskAllocator` vtable 0x86C754 (batch `frontier2-c6`).
fn allocator_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C754);
}

/// `CPedGroupDefaultTaskAllocatorFollowAnyMeans::~CPedGroupDefaultTaskAllocatorFollowAnyMeans()` (0x5F6460).
#[hook(addr = 0x5F6460, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorFollowAnyMeans::~CPedGroupDefaultTaskAllocatorFollowAnyMeans",
       expect = "C7 01 54 C7 86 00", unverified = "frontier2-c6", route = "L2", hazards = "none")]
pub fn allocator_follow_any_means_dtor(this: GPtr<u8>) {
    allocator_dtor(this);
}

/// `CPedGroupDefaultTaskAllocatorFollowLimited::~CPedGroupDefaultTaskAllocatorFollowLimited()` (0x5F6490).
#[hook(addr = 0x5F6490, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorFollowLimited::~CPedGroupDefaultTaskAllocatorFollowLimited",
       expect = "C7 01 54 C7 86 00", unverified = "frontier2-c6", route = "L2", hazards = "none")]
pub fn allocator_follow_limited_dtor(this: GPtr<u8>) {
    allocator_dtor(this);
}

/// `CPedGroupDefaultTaskAllocatorStandStill::~CPedGroupDefaultTaskAllocatorStandStill()` (0x5F64C0).
#[hook(addr = 0x5F64C0, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorStandStill::~CPedGroupDefaultTaskAllocatorStandStill",
       expect = "C7 01 54 C7 86 00", unverified = "frontier2-c6", route = "L2", hazards = "none")]
pub fn allocator_stand_still_dtor(this: GPtr<u8>) {
    allocator_dtor(this);
}

/// `CPedGroupDefaultTaskAllocatorChat::~CPedGroupDefaultTaskAllocatorChat()` (0x5F64F0).
#[hook(addr = 0x5F64F0, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorChat::~CPedGroupDefaultTaskAllocatorChat",
       expect = "C7 01 54 C7 86 00", unverified = "frontier2-c6", route = "L2", hazards = "none")]
pub fn allocator_chat_dtor(this: GPtr<u8>) {
    allocator_dtor(this);
}

/// `CPedGroupDefaultTaskAllocatorRandom::~CPedGroupDefaultTaskAllocatorRandom()` (0x5F6520).
#[hook(addr = 0x5F6520, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorRandom::~CPedGroupDefaultTaskAllocatorRandom",
       expect = "C7 01 54 C7 86 00", unverified = "frontier2-c6", route = "L2", hazards = "none")]
pub fn allocator_random_dtor(this: GPtr<u8>) {
    allocator_dtor(this);
}

/// `CPedGroupDefaultTaskAllocatorSitInLeaderCar::~CPedGroupDefaultTaskAllocatorSitInLeaderCar()` (0x5F6550).
#[hook(addr = 0x5F6550, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorSitInLeaderCar::~CPedGroupDefaultTaskAllocatorSitInLeaderCar",
       expect = "C7 01 54 C7 86 00", unverified = "frontier2-c6", route = "L2", hazards = "none")]
pub fn allocator_sit_in_leader_car_dtor(this: GPtr<u8>) {
    allocator_dtor(this);
}

/// `CTaskComplexReactToAttack::CreateSubTask(int32 taskType)` (0x620C30, thiscall, `ret 4`; vtable 0x86D868's
/// `CreateFirstSubTask` / `CreateNextSubTask` call it; the entry is `nop; jmp` into the protection fragment 0x4020FF,
/// which only loads the SEH chain). Type 1008 (TASK_SIMPLE_BE_HIT): a new 0x28-byte task, `CTaskSimpleBeHit(attacker,
/// +0x20, +0x1C, ftol(+0x18))` with the attacker (+0x14) only when it is a ped (type bits of +0x36 == 3), else NULL;
/// 200: a new `CTaskSimpleNone` (8 bytes: `CTaskSimple` part, vtable 0x859CFC); 208 (TASK_COMPLEX_FALL_AND_GET_UP): a
/// new 0x18-byte `CTaskComplexFallAndGetUp(anim, group 0, 1000)` with anim 0x1A when +0x1C == 2, else 0x14. A failed
/// allocation and every other type give NULL. The SEH frame is not modelled. Not hookable: the protection reads the
/// entry bytes as data (0x40135D `add esi, [0x620C31]`), so a phase B native.
#[sa_core::native(addr = 0x620C30, conv = thiscall, name = "CTaskComplexReactToAttack::CreateSubTask")]
pub fn react_to_attack_create_sub_task(this: GPtr<u8>, task_type: i32) -> GPtr<u8> {
    match task_type {
        1008 => {
            let attacker = rd::<GPtr<u8>>(this, 0x14);
            let src = if !attacker.is_null() && rd::<u8>(attacker, 0x36) & 7 == 3 { attacker.addr() as u32 } else { 0 };
            let t = task_new(0x28);
            if t.is_null() {
                return GPtr::NULL;
            }
            let time = sa_math::fpu::ftol32(X87::from_f32(rd::<f32>(this, 0x18)));
            crate::handler::be_hit_ctor(t, src, rd::<u32>(this, 0x20), rd::<u32>(this, 0x1C), time)
        }
        200 => {
            let t = task_new(8);
            if t.is_null() {
                return GPtr::NULL;
            }
            task_simple_ctor(t);
            wr::<u32>(t, 0, 0x859CFC);
            t
        }
        208 => {
            let anim = if rd::<u32>(this, 0x1C) == 2 { 0x1A } else { 0x14 };
            let t = task_new(0x18);
            if t.is_null() {
                return GPtr::NULL;
            }
            fall_and_get_up_ctor(t, anim, 0, 1000)
        }
        _ => GPtr::NULL,
    }
}

/// The nearest ped a scripted 2D effect can take (0x603570, cdecl `(C2dEffect* effect)`; unnamed, called from
/// 0x6060A0). Over the ped pool (0xB74490) from the last slot down (the pool pointer read once, its object array
/// every step): a used slot whose active task (`CTaskManager::GetActiveTask` of intelligence +4) is not type 233; its
/// squared distance to the effect position (x87 `dz² + dy² + dx²`, stored f32) below the best so far (start FLT_MAX;
/// `test ah, 5` / `jp`); the effect's script record (`CScripted2dEffects::GetIndex`, 36 bytes from 0xC3A200) without a
/// ped restriction (byte +0x20 == 0), or listing the ped's model (`int16` +0x22) among its four ids, or a -2 entry whose
/// ped type (+0x10 + 4k) equals the ped's (+0x598); then a local `CMatrix` (owner fields cleared, `SetScale(1.0)`)
/// and `CPedAttractorManager::IsApproachable(effect, matrix, 0, ped)` decides: approachable → new best. Returns the best
/// ped or NULL. The SEH frame is not modelled.
#[hook(addr = 0x603570, conv = cdecl, name = "FUN_00603570", expect = "6A FF 68 78 E8 83 00",
       unverified = "frontier2-c6", route = "L2", hazards = "uninit-locals",
       note = "the local CMatrix is SetScale'd; its flags word is not written by the original")]
pub fn scripted_effect_nearest_ped(effect: GPtr<u8>) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let mut best = f32::from_bits(0x7F7F_FFFF);
    let mut best_ped = GPtr::NULL;
    let mut i = rd::<i32>(pool, 8);
    if i == 0 {
        return GPtr::NULL;
    }
    let mut off = i.wrapping_mul(0x7C4);
    loop {
        i -= 1;
        off = off.wrapping_sub(0x7C4);
        let used = rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 == 0;
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(off as isize);
        if used && !ped.is_null() {
            let t = tm_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
            if t.is_null() || task_get_type(t) != 0xE9 {
                let p = sa_world::world::common::entity_pos(ped);
                let d: [X87; 3] = core::array::from_fn(|k| X87::from_f32(rd::<f32>(effect, 4 * k as isize)).fsub_m(rd::<f32>(p, 4 * k as isize)));
                let dist = d[2].fmul(d[2]).fadd(d[1].fmul(d[1])).fadd(d[0].fmul(d[0])).to_f32();
                if !X87::from_f32(dist).fcom(X87::from_f32(best)).test_parity(5) && effect_accepts_ped(effect, ped) {
                    let mut m = [0u8; 0x48];
                    let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr());
                    matrix_set_scale(mp, 1.0);
                    if crate::attractor_range_scan::attractor_manager_is_approachable(effect, mp, 0, ped) != 0 {
                        best = dist;
                        best_ped = ped;
                    }
                    sa_world::bindings::matrix_dtor(mp);
                }
            }
        }
        if i == 0 {
            return best_ped;
        }
    }
}

/// The scripted 2D effect's ped restriction (0x603570's inline test): no restriction, the ped's model among the
/// record's four ids, or a -2 id whose ped type matches.
fn effect_accepts_ped(effect: GPtr<u8>, ped: GPtr<u8>) -> bool {
    let idx = scripted_2d_effects_get_index(effect);
    let rec = GPtr::<u8>::from_addr(0xC3A200).byte_add(idx.wrapping_mul(36) as isize);
    if rd::<u8>(rec, 0x20) == 0 {
        return true;
    }
    let model = i32::from(rd::<i16>(ped, 0x22));
    if (0..4).any(|k| rd::<i32>(rec, 4 * k) == model) {
        return true;
    }
    let ty = rd::<u32>(ped, 0x598);
    (0..4).any(|k| rd::<i32>(rec, 4 * k) == -2 && rd::<u32>(rec, 0x10 + 4 * k) == ty)
}

const XLEN_MSG: &str = "the original throws std::length_error(\"vector<T> too long\") (__CxxThrowException), which ends the game";

/// `std::vector<CPed*>::_Xlen()` (0x5ECAB0): throws `length_error("vector<T> too long")`. Phase B cannot raise a C++
/// exception; the native stops the game the way the throw does.
#[sa_core::native(addr = 0x5ECAB0, conv = thiscall, name = "std::vector<CPed*>::_Xlen")]
pub fn vector_ped_xlen(_this: GPtr<u8>) {
    panic!("std::vector<CPed*>::_Xlen: {XLEN_MSG}")
}

/// `std::vector<CPedTaskPair>::_Xlen()` (0x5ECB30): as `vector_ped_xlen`.
#[sa_core::native(addr = 0x5ECB30, conv = thiscall, name = "std::vector<CPedTaskPair>::_Xlen")]
pub fn vector_task_pair_xlen(_this: GPtr<u8>) {
    panic!("std::vector<CPedTaskPair>::_Xlen: {XLEN_MSG}")
}

/// `std::vector<CTask*>::_Xlen()` (0x5ECBB0): as `vector_ped_xlen`.
#[sa_core::native(addr = 0x5ECBB0, conv = thiscall, name = "std::vector<CTask*>::_Xlen")]
pub fn vector_task_xlen(_this: GPtr<u8>) {
    panic!("std::vector<CTask*>::_Xlen: {XLEN_MSG}")
}

/// `std::vector<CPedAttractor*>::_Xlen()` (0x5ECCB0): as `vector_ped_xlen`.
#[sa_core::native(addr = 0x5ECCB0, conv = thiscall, name = "std::vector<CPedAttractor*>::_Xlen")]
pub fn vector_attractor_xlen(_this: GPtr<u8>) {
    panic!("std::vector<CPedAttractor*>::_Xlen: {XLEN_MSG}")
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    NATIVE_VECTOR_PED_XLEN,
    NATIVE_VECTOR_TASK_PAIR_XLEN,
    NATIVE_VECTOR_TASK_XLEN,
    NATIVE_VECTOR_ATTRACTOR_XLEN,
    NATIVE_REACT_TO_ATTACK_CREATE_SUB_TASK,
];

use sa_anim::bindings::{anim_get_animation, assoc_set_current_time, assoc_set_finish_callback, get_animation_block, ik_abort_look_at};
use sa_core::bindings::{get_anim_hierarchy_from_skin_clump, rp_hanim_hierarchy_get_matrix_array, rp_hanim_id_get_index,
                        rw_v3d_transform_points};
use crate::bindings::{fall_and_get_up_ctor, scripted_2d_effects_get_index};
use sa_core::bindings::{matrix_set_scale, tm_get_active_task};
use sa_stream::bindings::anim_blend_animation_hier;
use sa_world::bindings::{anim_blend_animation, clump_get_first_association_4d6a70, ik_is_looking, localisation_kicking_when_down,
                         wander_ctor};

use sa_math::x87::X87;
use crate::bindings::task_new;
use sa_world::bindings::fire_find_nearest_fire as find_nearest_fire;

/// The route round a ped group (0x66FD30, thiscall, `ret 4`; unnamed, called by a generated `CreateSubTask`; FRONTIER-2
/// lane C turn 3): a 16-slot ped list on the stack holds the task's ped (+0xC) and 15 nulls; `FUN_0066f6c0(this,
/// &sphere, &list)` fills the `CColSphere` around them; its centre z becomes the ped's own position z; then
/// `CPedGeometryAnalyser::ComputeRouteRoundSphere(ped, &sphere, this+0x14, this+0x20, this+0x38, this+0x2c)`, whose
/// EAX is returned.
#[hook(addr = 0x66FD30, conv = thiscall, name = "FUN_0066fd30", expect = "83 EC 54 56 8B F1",
       unverified = "frontier2-c7", route = "L2", hazards = "uninit-locals,ax-al-return",
       note = "the sphere's last dword (+0x10 of CColSphere) is whatever FUN_0066f6c0 leaves; EAX = ComputeRouteRoundSphere's")]
pub fn route_round_group(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let mut list = [0u32; 16];
    list[0] = rd::<u32>(this, 0xC);
    let mut sphere = [0u32; 5];
    let sp: GPtr<u8> = GPtr::from_raw(&raw mut sphere).cast();
    group_member_bounding_sphere(this, sp, GPtr::from_raw(&raw mut list).cast());
    wr::<u32>(sp, 8, rd::<u32>(sa_world::world::common::entity_pos(ped), 8));
    crate::wander::geom_compute_route_round_sphere(
        ped,
        sp,
        this.byte_add(0x14),
        this.byte_add(0x20),
        this.byte_add(0x38),
        this.byte_add(0x2C),
    )
}

use crate::bindings::group_member_bounding_sphere;

/// The bounding sphere of a ped list (0x66F6C0, thiscall, `ret 8`; unnamed, called by the group route-round 0x66FD30;
/// FRONTIER-2 lane C turn 4; `this` is not read): over the 16 slots of `peds`, the non-null peds' positions summed
/// per axis (each sum stored, f32) and counted; the centre is `(sumX * (1 / n), sumY * (1 / n), 0)` with the x87
/// reciprocal of the count (`fild`; n = 0 gives NaN); the squared radius is the largest `dy*dy + dx*dx + r*r` (r the
/// ped's model's collision bounding radius, colModel +0x24; the distances from the 80-bit differences, a new maximum
/// kept as its f32 store); `CColSphere::Set(sphere, sqrt(max) + 0.7f, &centre, 0, 0, 0xFF)`.
#[hook(addr = 0x66F6C0, conv = thiscall, name = "FUN_0066f6c0", expect = "83 EC 1C 53 56 57",
       unverified = "frontier2-c9", route = "L2", hazards = "none")]
pub fn bounding_sphere_of_peds(_this: GPtr<u8>, sphere: GPtr<u8>, peds: GPtr<u8>) {
    let ped = |i: isize| rd::<GPtr<u8>>(peds, 4 * i);
    let mut c = [0f32; 3];
    let mut n: i32 = 0;
    for i in 0..16 {
        let p = ped(i);
        if !p.is_null() {
            n += 1;
            let q = sa_world::world::common::entity_pos(p);
            for (k, ck) in c.iter_mut().enumerate() {
                *ck = X87::from_f32(*ck).fadd_m(rd::<f32>(q, 4 * k as isize)).to_f32();
            }
        }
    }
    let inv = X87::from_i32(n).fdivr_m(1.0);
    c[0] = X87::from_f32(c[0]).fmul(inv).to_f32();
    c[1] = X87::from_f32(c[1]).fmul(inv).to_f32();
    c[2] = 0.0;
    let mut max = X87::zero();
    for i in 0..16 {
        let p = ped(i);
        if !p.is_null() {
            let q = sa_world::world::common::entity_pos(p);
            let dx = X87::from_f32(rd::<f32>(q, 0)).fsub_m(c[0]);
            let dy = X87::from_f32(rd::<f32>(q, 4)).fsub_m(c[1]);
            let col = rd::<GPtr<u8>>(sa_stream::shared::model_info(i32::from(rd::<i16>(p, 0x22))).cast::<u8>(), 0x14);
            let r = X87::from_f32(rd::<f32>(col, 0x24));
            let d = dy.fmul(dy).fadd(dx.fmul(dx)).fadd(r.fmul(r));
            if !d.fcom(max).test_any(0x41) {
                max = X87::from_f32(d.to_f32());
            }
        }
    }
    let radius = max.fsqrt().fadd_m(GPtr::<f32>::from_addr(0x86FCC8).read()).to_f32();
    col_sphere_set(sphere, radius.to_bits(), GPtr::from_raw(&raw mut c).cast(), 0, 0, 0xFF);
}

use sa_core::bindings::col_sphere_set;

/// The heading from a task's entity to a ped (0x6592A0, thiscall, `ret 4`, ST0; unnamed, called by 0x65A110; FRONTIER-2
/// lane C turn 4): `LimitRadianAngle(f32(GetRadianAngleBetweenPoints(e.x, e.y, ped.x, ped.y)))` with `e` the entity at
/// `this + 0xC` (positions from the matrix when there is one, else the placement); the f32 is stored into the ped
/// argument's slot, which no caller reads.
#[hook(addr = 0x6592A0, conv = thiscall, name = "FUN_006592a0", expect = "8B 54 24 04 8B 42 14",
       unverified = "frontier2-c10", route = "L2", hazards = "none")]
pub fn heading_from_task_entity_to_ped(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let p = sa_world::world::common::entity_pos(ped);
    let e = sa_world::world::common::entity_pos(rd::<GPtr<u8>>(this, 0xC));
    let a = sa_core::bindings::general_get_radian_angle_between_points(rd::<f32>(e, 0), rd::<f32>(e, 4), rd::<f32>(p, 0),
        rd::<f32>(p, 4));
    X87::from_f64_exact(sa_core::bindings::general_limit_radian_angle(a.to_f32()))
}

/// A sub-task by type (0x65A110, thiscall, `ret 8`; unnamed, called by generated `CreateSubTask`s; FRONTIER-2 lane C
/// turn 4): 0x19D: a `CTaskSimpleRunTimedAnim` (0x34 bytes; `Constructor-2(0, 0x8E, 4.0, -4.0, 3000, 0x19D, "HandsUp",
/// 0)`, then the vtable 0x85A29C); 0x386: a `CTaskSimpleAchieveHeading` (0x18 bytes) to the heading of 0x6592A0
/// with rate 0.5 and tolerance 0.2 (0x86FC7C / 0x86FC80; the original pushes them before the heading call, whose `ret
/// 4` leaves them as the constructor's arguments), its constructor's EAX returned; a failed allocation and every
/// other type (0x516 included) return null.
#[hook(addr = 0x65A110, conv = thiscall, name = "FUN_0065a110", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-c10", route = "L2", hazards = "none")]
pub fn create_sub_task_65a110(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x19D => {
            let t = sa_core::bindings::task_new(0x34);
            if t.is_null() {
                return GPtr::NULL;
            }
            crate::bindings::run_timed_anim_ctor2(t, 0, 0x8E, 0x4080_0000, 0xC080_0000, 3000, 0x19D, 0x85A2C4, 0);
            t.cast::<u32>().write(0x85A29C);
            t
        }
        0x386 => {
            let t = sa_core::bindings::task_new(0x18);
            if t.is_null() {
                return GPtr::NULL;
            }
            let tol = GPtr::<u32>::from_addr(0x86FC80).read();
            let rate = GPtr::<u32>::from_addr(0x86FC7C).read();
            let h = crate::bindings::task_heading_to_ped(this, ped).to_f32();
            sa_world::bindings::achieve_heading_ctor(t, h, rate, tol)
        }
        _ => GPtr::NULL,
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PATROL_ROUTE_ASSIGN_PORT,
    HOOK_HEADING_FROM_TASK_ENTITY_TO_PED,
    HOOK_CREATE_SUB_TASK_65A110,
    HOOK_BOUNDING_SPHERE_OF_PEDS,
    HOOK_ROUTE_ROUND_GROUP,
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
    HOOK_BE_COP_CTOR,
    HOOK_FOLLOW_PATROL_ROUTE_CTOR,
    HOOK_AVOID_ENTITY_RELEASE_LOOK,
    HOOK_LOOPED_ANIM_CTOR_IDS,
    HOOK_LOOPED_ANIM_CTOR_NAMES,
    HOOK_LOOPED_ANIM_START,
    HOOK_KICKED_ON_GROUND_START_ANIM,
    HOOK_KILL_ALL_THREATS_SCAN,
    HOOK_SEEK_ENTITY_CTOR,
    HOOK_ALLOCATOR_FOLLOW_ANY_MEANS_DTOR,
    HOOK_ALLOCATOR_FOLLOW_LIMITED_DTOR,
    HOOK_ALLOCATOR_STAND_STILL_DTOR,
    HOOK_ALLOCATOR_CHAT_DTOR,
    HOOK_ALLOCATOR_RANDOM_DTOR,
    HOOK_ALLOCATOR_SIT_IN_LEADER_CAR_DTOR,
    HOOK_SCRIPTED_EFFECT_NEAREST_PED,
];
