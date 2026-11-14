//! WRITER-MVP-B (helperB) AI ports: `CCover` building cover points, task/event constructors and
//! destructors, `CPedGroupIntelligence::AddEvent`, `CEntityScanner::Clear`, `CConversationNode::ClearRecursively`
//! and small helpers. Ported from the 1.0 US bytes (the exe wins over gta-reversed); every call to another
//! catalogued function goes through `orig!` at the original address. SEH frame set-up/tear-down of the
//! originals (fs:0 links) has no observable effect and is not modelled.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CCover::m_aPoints` (100 x 0x1C; +0 used flag, +0x10 entity)
const COVER_POINTS: usize = 0xC197C8;
/// `CCover::m_NumPoints`
const COVER_NUM_POINTS: usize = 0xC197A4;
/// `CConversations::m_aNodes` (0x18 each)
const CONVERSATION_NODES: usize = 0x969570;

orig! {
    /// `CWorld::FindPlayerCoors(CVector* out, int32 id)` (0x56E010, cdecl)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CEntity::GetBoundCentre(CVector* out)` (0x534250, `ret 4`)
    fn entity_get_bound_centre(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x534250 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, `ret 4`)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `CPlaceable::AllocateMatrix` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790, cdecl)
    fn multiply3x3(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C790 => unported;
    /// `operator*(CVector* out, const CMatrix*, const CVector*)` (0x59C890, cdecl)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `CCover::AddCoverPoint(int32, CEntity*, CVector*, char, uint8)` (0x698F30, cdecl)
    fn add_cover_point(ty: u32, e: GPtr<u8>, pos: GPtr<f32>, a: u32, b: u32) = cdecl 0x698F30 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `FUN_0061a3a0` (0x61A3A0): `CTaskSimple` base destructor
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => crate::mvpb_ai::task_simple_dtor_61a3a0;
    /// `FUN_0061a3d0` (0x61A3D0): `CTaskComplex` base destructor
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => crate::mvpb_ai::task_complex_dtor_61a3d0;
    /// `thunk_FUN_0040387c` (0x669D20): base destructor `FUN_0048e600` tail-jumps to
    fn dtor_669d20(t: GPtr<u8>) = thiscall 0x669D20 => unported;
    /// `CEventEditableResponse::~CEventEditableResponse` (0x4AC480)
    fn editable_response_dtor(e: GPtr<u8>) = thiscall 0x4AC480 => unported;
    /// `CEventGroup::Flush(bool)` (0x4AB370, `ret 4`)
    fn event_group_flush(g: GPtr<u8>, avoid: u32) -> u32 = thiscall 0x4AB370 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, `ret 4`)
    fn group_membership_is_member(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CEventEditableResponse::ComputeResponseTaskType(CPedGroup*)` (0x4B57A0, `ret 4`)
    fn compute_response_task_type_group(e: GPtr<u8>, g: GPtr<u8>) = thiscall 0x4B57A0 => unported;
    /// `CEventEditableResponse::WillRespond` (0x4AC490)
    fn will_respond(e: GPtr<u8>) -> u8 = thiscall 0x4AC490 => unported;
    /// `CEventGroupEvent::BaseEventTakesPriorityOverBaseEvent(const CEventGroupEvent&)` (0x4AE100, `ret 4`)
    fn base_event_takes_priority(e: GPtr<u8>, o: GPtr<u8>) -> u8 = thiscall 0x4AE100 => unported;
    /// `CEvent::GetEventType` (vtable +0x4)
    fn v_get_event_type(e: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn v_event_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEvent::AffectsPedGroup(CPedGroup*)` (vtable +0x18)
    fn v_affects_ped_group(e: GPtr<u8>, g: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEvent::HasEditableResponse` (vtable +0x24)
    fn v_has_editable_response(e: GPtr<u8>) -> u8 = thiscall vslot 0x24 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_get_source_entity(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

/// The column model of an entity's model (`ms_modelInfoPtrs[m_nModelIndex]`).
fn model_info(e: GPtr<u8>) -> GPtr<u8> {
    let idx = rd::<i16>(e, 0x22) as i32;
    rd::<GPtr<u8>>(GPtr::from_addr(MODEL_INFO_PTRS), 4 * idx as isize)
}

/// `CCover::ShouldThisBuildingHaveItsCoverPointsCreated(CBuilding*)` (0x699230, cdecl): true (AL = 1) when the
/// distance from player 0's position (`FindPlayerCoors(-1)`) to the building's bound centre is less than the
/// model's bounding-sphere radius + 30 (x87, PC24: `sqrt((dz*dz + dy*dy) + dx*dx)`); unordered gives false.
#[hook(addr = 0x699230, conv = cdecl, name = "CCover::ShouldThisBuildingHaveItsCoverPointsCreated",
       expect = "83 EC 18 56 57 8D 44 24 08", route = "L2",
       hazards = "x87-transcendental", note = "fsqrt result compared, not stored")]
pub fn cover_should_building_have_cover_points(b: GPtr<u8>) -> u8 {
    let mut ploc = [0.0f32; 3];
    let mut cloc = [0.0f32; 3];
    let pp = find_player_coors(GPtr::from_raw(ploc.as_mut_ptr()), -1);
    let c = entity_get_bound_centre(b, GPtr::from_raw(cloc.as_mut_ptr()));
    let dx = x(c.read()).fsub_m(pp.read());
    let dy = x(c.wrapping_index(1).read()).fsub_m(pp.wrapping_index(1).read());
    let dz = x(c.wrapping_index(2).read()).fsub_m(pp.wrapping_index(2).read());
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let col = rd::<GPtr<u8>>(model_info(b), 0x14);
    let r = x(rd::<f32>(col, 0x24)).fadd_m(30.0);
    u8::from(r.fcom(d) == sa_math::x87::Cmp::Greater)
}

/// `CCover::RemoveCoverPointsForThisEntity(CEntity*)` (0x698740, cdecl): every used cover point (100) whose entity
/// equals the argument releases its reference (when non-null), is marked unused, its entity cleared and the count
/// decremented.
#[hook(addr = 0x698740, conv = cdecl, name = "CCover::RemoveCoverPointsForThisEntity",
       expect = "56 57 8B 7C 24 0C", route = "L2", hazards = "none")]
pub fn cover_remove_points_for_entity(e: GPtr<u8>) {
    for i in 0..100usize {
        let pt: GPtr<u8> = GPtr::from_addr(COVER_POINTS + 0x1C * i);
        if rd::<u8>(pt, 0) == 0 {
            continue;
        }
        let ent = rd::<GPtr<u8>>(pt, 0x10);
        if e.addr() != ent.addr() {
            continue;
        }
        if !ent.is_null() {
            clean_up_old_reference(ent, pt.byte_add(0x10));
        }
        let n = rd::<i32>(GPtr::from_addr(COVER_NUM_POINTS), 0);
        wr::<u8>(pt, 0, 0);
        wr::<u32>(pt, 0x10, 0);
        wr::<i32>(GPtr::from_addr(COVER_NUM_POINTS), 0, n.wrapping_sub(1));
    }
}

/// `CCover::FindCoverPointsForThisBuilding(CBuilding*)` (0x699120, cdecl): for every 2d effect of type 9 of the
/// building's model, rotates the effect's direction (x, y, 0) by the building matrix (allocating/updating the matrix
/// first when missing), turns `atan2(-x, y) * 40.743664` into a byte (`_ftol`), transforms the effect position and
/// calls `AddCoverPoint(3, building, &pos, effect+0x18, angle)`. Kept original detail: the angle byte is stored into
/// the low byte of the caller's argument slot (the building pointer), and that whole dword is what is pushed as the
/// last argument.
#[hook(addr = 0x699120, conv = cdecl, name = "CCover::FindCoverPointsForThisBuilding",
       expect = "83 EC 34 53 55 56", route = "L2",
       hazards = "x87-transcendental", note = "FPATAN result scaled and truncated by _ftol; overwrites the low byte of its own argument slot")]
pub fn cover_find_points_for_building(b: GPtr<u8>) {
    let mi = model_info(b);
    let n = rd::<u8>(mi, 0xD) as i32;
    let mut slot = b.addr() as u32;
    for i in 0..n {
        let eff = get_2d_effect(mi, i);
        if rd::<u8>(eff, 0xC) != 9 {
            continue;
        }
        let vy = f32_of(x(rd::<f32>(eff, 0x14)));
        let mut v = [rd::<f32>(eff, 0x10), vy, 0.0f32];
        if rd::<u32>(b, 0x14) == 0 {
            allocate_matrix(b);
            simple_transform_update_matrix(b.byte_add(4), rd::<GPtr<u8>>(b, 0x14));
        }
        let mut out = [0.0f32; 3];
        let r = multiply3x3(GPtr::from_raw(out.as_mut_ptr()), rd::<GPtr<u8>>(b, 0x14), GPtr::from_raw(v.as_mut_ptr()));
        v[0] = r.read();
        v[1] = r.wrapping_index(1).read();
        v[2] = r.wrapping_index(2).read();
        let ang = x(v[0]).fchs().fpatan(x(v[1])).fmul_m(40.743_664);
        slot = (slot & 0xFFFF_FF00) | (ftol32(ang) as u32 & 0xFF);
        if rd::<u32>(b, 0x14) == 0 {
            allocate_matrix(b);
            simple_transform_update_matrix(b.byte_add(4), rd::<GPtr<u8>>(b, 0x14));
        }
        let mut pos = [0.0f32; 3];
        matrix_mul_vec(GPtr::from_raw(pos.as_mut_ptr()), rd::<GPtr<u8>>(b, 0x14), eff);
        add_cover_point(3, b, GPtr::from_raw(pos.as_mut_ptr()), rd::<u8>(eff, 0x18) as u32, slot);
    }
}

/// `CTaskComplexFacial::Constructor` (0x690D20): `CTaskComplex` base, vtable 0x870ADC, +0x10 = +0x18 = -1, +0xC =
/// `(int)((rand() & 0xFFFF) * 3.0517578e-5 * 100) > 70`, +0xD = +0xE = 0. Returns this.
#[hook(addr = 0x690D20, conv = thiscall, name = "CTaskComplexFacial::Constructor", this = "CTaskComplexFacial",
       expect = "51 56 8B F1 E8 87 96 F8 FF", route = "L2", hazards = "heap-writes")]
pub fn task_complex_facial_ctor(this: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x870ADC);
    wr::<u32>(this, 0x10, 0xFFFF_FFFF);
    wr::<u32>(this, 0x18, 0xFFFF_FFFF);
    let r = sa_core::orig::at_site(0x690D3D, rand) & 0xFFFF;
    let v = ftol32(xi(r).fmul_m(3.051_757_8e-5).fmul_m(100.0));
    wr::<u8>(this, 0xC, u8::from(v > 0x46));
    wr::<u8>(this, 0xD, 0);
    wr::<u8>(this, 0xE, 0);
    this
}

/// Shared shape of the reference-holding event/task destructors: vtable `vt`, release the entity reference at
/// `off`, then `last` as the base vtable (0 = none).
fn ref_dtor(this: GPtr<u8>, vt: u32, off: isize, last: u32) {
    wr::<u32>(this, 0, vt);
    let e = rd::<GPtr<u8>>(this, off);
    if !e.is_null() {
        clean_up_old_reference(e, this.byte_add(off));
    }
    if last != 0 {
        wr::<u32>(this, 0, last);
    }
}

/// `FUN_004af890` (0x4AF890): event destructor: vtable 0x85B250, releases the entity at +0x14, base vtable 0x85AB38.
#[hook(addr = 0x4AF890, conv = thiscall, name = "FUN_004af890", expect = "6A FF 68 E8 9E 83 00",
       route = "L2", hazards = "heap-writes")]
pub fn event_dtor_4af890(this: GPtr<u8>) {
    ref_dtor(this, 0x85B250, 0x14, 0x85AB38);
}

/// `CEventVehicleToSteal::~CEventVehicleToSteal` (0x4AF700, the non-deleting body): vtable 0x85B210, releases the vehicle at +0xC, base
/// vtable 0x85AB38 (reached when a ped's car-steal event is deleted, e.g. near parked cars at Grove Street).
#[hook(addr = 0x4AF700, conv = thiscall, name = "FUN_004af700", expect = "6A FF 68 A8 9E 83 00",
       unverified = "framelimit-ai", route = "L2", hazards = "heap-writes")]
pub fn event_vehicle_to_steal_dtor(this: GPtr<u8>) {
    ref_dtor(this, 0x85B210, 0xC, 0x85AB38);
}

/// `CEventVehicleCollision::~CEventVehicleCollision` (0x4AC920, the non-deleting body, SEH frame): vtable 0x85AC28,
/// releases the vehicle at +0x14, base vtable 0x85AB38. The CARJACK branch ports the same body (jack_events.rs); keep
/// one registration when integrating.
#[hook(addr = 0x4AC920, conv = thiscall, name = "FUN_004ac920", expect = "6A FF 68 A8 99 83 00",
       unverified = "framelimit-ai", route = "L2", hazards = "heap-writes")]
pub fn event_vehicle_collision_dtor(this: GPtr<u8>) {
    ref_dtor(this, 0x85AC28, 0x14, 0x85AB38);
}

/// `FUN_005dea00` (0x5DEA00): `CEventSoundQuiet` destructor: vtable 0x86C2B0, releases the entity at +0x14, then
/// `CEventEditableResponse::~CEventEditableResponse`.
#[hook(addr = 0x5DEA00, conv = thiscall, name = "FUN_005dea00", expect = "6A FF 68 08 D8 83 00",
       route = "L2", hazards = "heap-writes")]
pub fn event_dtor_5dea00(this: GPtr<u8>) {
    ref_dtor(this, 0x86C2B0, 0x14, 0);
    editable_response_dtor(this);
}

/// `FUN_004aedf0(CEntity*)` (0x4AEDF0, `ret 4`): event constructor: +4 = 0, +8 = 0 (byte), +0xC = 1 (byte),
/// +0xE = 200 (word), +0x10 = -1 (word), vtable 0x85B0B0, +0x14 = entity (registered when non-null). Returns this.
#[hook(addr = 0x4AEDF0, conv = thiscall, name = "FUN_004aedf0", expect = "6A FF 68 48 9D 83 00",
       route = "L2", hazards = "heap-writes")]
pub fn event_ctor_4aedf0(this: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<u32>(this, 0, 0x85B0B0);
    wr(this, 0x14, e);
    if !e.is_null() {
        register_reference(e, this.byte_add(0x14));
    }
    this
}

/// `FUN_004aeea0` (0x4AEEA0): destructor of the 0x4AEDF0 event: vtable 0x85B0B0, releases +0x14, base vtable
/// 0x85AB38.
#[hook(addr = 0x4AEEA0, conv = thiscall, name = "FUN_004aeea0", expect = "6A FF 68 68 9D 83 00",
       route = "L2", hazards = "heap-writes")]
pub fn event_dtor_4aeea0(this: GPtr<u8>) {
    ref_dtor(this, 0x85B0B0, 0x14, 0x85AB38);
}

/// `FUN_004aecd0(uint8, CEntity*)` (0x4AECD0, `ret 8`): event constructor: +4 = 0, +8 = 0 (byte), +0xC = the
/// byte argument, vtable 0x85B070, +0x10 = entity (registered when non-null). Returns this.
#[hook(addr = 0x4AECD0, conv = thiscall, name = "FUN_004aecd0", expect = "6A FF 68 08 9D 83 00",
       unverified = "mvpb-ai-1", route = "L2", hazards = "heap-writes")]
pub fn event_ctor_4aecd0(this: GPtr<u8>, a: u8, e: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, a);
    wr::<u32>(this, 0, 0x85B070);
    wr(this, 0x10, e);
    if !e.is_null() {
        register_reference(e, this.byte_add(0x10));
    }
    this
}

/// `FUN_004aed60` (0x4AED60): destructor of the 0x4AECD0 event: vtable 0x85B070, releases +0x10, base vtable
/// 0x85AB38.
#[hook(addr = 0x4AED60, conv = thiscall, name = "FUN_004aed60", expect = "6A FF 68 28 9D 83 00",
       unverified = "mvpb-ai-1", route = "L2", hazards = "heap-writes")]
pub fn event_dtor_4aed60(this: GPtr<u8>) {
    ref_dtor(this, 0x85B070, 0x10, 0x85AB38);
}

/// `CTaskSimpleCarSetPedInAsDriver(CVehicle*, CTaskUtilityLineUpPedWithCar*)` (0x6470E0, `ret 8`): `CTaskSimple`
/// base, vtable 0x86EE28, +8 = 0 (byte), +0xC = 0, +0x10 = vehicle, +0x14 = second argument, +0x18..+0x1A = 0; the
/// vehicle reference is registered when non-null. Returns this.
#[hook(addr = 0x6470E0, conv = thiscall, name = "CTaskSimpleCarSetPedInAsDriver", expect = "6A FF 68 E8 26 84 00",
       route = "L2", hazards = "heap-writes")]
pub fn car_set_ped_in_as_driver_ctor(this: GPtr<u8>, veh: GPtr<u8>, a: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x86EE28);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr(this, 0x10, veh);
    wr::<u32>(this, 0x14, a);
    wr::<u8>(this, 0x18, 0);
    wr::<u8>(this, 0x19, 0);
    wr::<u8>(this, 0x1A, 0);
    if !veh.is_null() {
        register_reference(veh, this.byte_add(0x10));
    }
    this
}

/// `FUN_00647170` (0x647170): `CTaskSimpleCarSetPedInAsDriver` destructor: vtable 0x86EE28, releases the vehicle
/// at +0x10, then the `CTaskSimple` base destructor.
#[hook(addr = 0x647170, conv = thiscall, name = "FUN_00647170", expect = "6A FF 68 08 27 84 00",
       route = "L2", hazards = "heap-writes")]
pub fn car_set_ped_in_as_driver_dtor(this: GPtr<u8>) {
    ref_dtor(this, 0x86EE28, 0x10, 0);
    task_simple_dtor(this);
}

/// `CTaskSimpleCarSetPedInAsPassenger(CVehicle*, int32 door, CTaskUtilityLineUpPedWithCar*)` (0x646FE0, `ret 0xC`):
/// `CTaskSimple` base, +0x14 = door, vtable 0x86EE04, +8 = 0 (byte), +0xC = 0, +0x10 = vehicle, +0x18 = third
/// argument, +0x1C..+0x1E = 0; the vehicle reference is registered when non-null. Returns this.
#[hook(addr = 0x646FE0, conv = thiscall, name = "CTaskSimpleCarSetPedInAsPassenger", expect = "6A FF E9 13 AC DB FF",
       route = "L2", hazards = "heap-writes,entry-protection-read")]
pub fn car_set_ped_in_as_passenger_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, a: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0x14, door);
    wr::<u32>(this, 0, 0x86EE04);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr(this, 0x10, veh);
    wr::<u32>(this, 0x18, a);
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x1D, 0);
    wr::<u8>(this, 0x1E, 0);
    if !veh.is_null() {
        register_reference(veh, this.byte_add(0x10));
    }
    this
}

/// `CTaskComplexWander::Constructor(int32 moveState, uint8 dir, bool, float)` (0x66F450, `ret 0x10`):
/// `CTaskComplex` base, +0xC = move state, +0x10 = direction byte, vtable 0x86FE84, +0x14 = fourth argument,
/// +0x18 = +0x1C = 0xFFFF (words), +0x24 = (+0x24 & 0xF0) | (flag & 1), +0x20 = 0. Returns this.
#[hook(addr = 0x66F450, conv = thiscall, name = "CTaskComplexWander::Constructor", this = "CTaskComplexWander",
       expect = "56 8B F1 E8 58 AF FA FF", route = "L2", hazards = "heap-writes")]
pub fn task_complex_wander_ctor(this: GPtr<u8>, ms: u32, dir: u8, flag: u8, f: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, ms);
    wr::<u8>(this, 0x10, dir);
    wr::<u32>(this, 0, 0x86FE84);
    wr::<u32>(this, 0x14, f);
    wr::<u16>(this, 0x18, 0xFFFF);
    wr::<u16>(this, 0x1C, 0xFFFF);
    let b = rd::<u8>(this, 0x24);
    wr::<u8>(this, 0x24, (b & 0xF0) | (flag & 1));
    wr::<u32>(this, 0x20, 0);
    this
}

/// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0, `ret 4`): ped non-null and equal to the leader (+0x20).
#[hook(addr = 0x5F69C0, conv = thiscall, name = "CPedGroupMembership::IsLeader", this = "CPedGroupMembership",
       expect = "8B 44 24 04 85 C0", route = "L2", hazards = "none")]
pub fn group_membership_is_leader(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    u32::from(!ped.is_null() && rd::<u32>(this, 0x20) == ped.addr() as u32)
}

/// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`): only `CEventGroupEvent` (type 0x2F) that affects
/// the group (outer and inner event, `AffectsPedGroup`); rejected when the inner event's source is a ped (type 3)
/// that is a member of the group; editable inner events compute their group response and must `WillRespond`. With no
/// current event (+8) the clone is stored; otherwise the new one must `BaseEventTakesPriorityOverBaseEvent` the
/// current one, which is deleted and replaced by the clone. Returns AL = stored.
#[hook(addr = 0x5F7470, conv = thiscall, name = "CPedGroupIntelligence::AddEvent-", this = "CPedGroupIntelligence",
       expect = "56 8B 74 24 08 8B 06", route = "L2", hazards = "heap-writes")]
pub fn pgi_add_event(this: GPtr<u8>, ev: GPtr<u8>) -> u8 {
    if v_get_event_type(ev) != 0x2F {
        return 0;
    }
    if v_affects_ped_group(ev, rd::<GPtr<u8>>(this, 0)) == 0 {
        return 0;
    }
    let inner = rd::<GPtr<u8>>(ev, 0x10);
    if v_affects_ped_group(inner, rd::<GPtr<u8>>(this, 0)) == 0 {
        return 0;
    }
    let src = v_get_source_entity(rd::<GPtr<u8>>(ev, 0x10));
    if !src.is_null()
        && rd::<u8>(src, 0x36) & 7 == 3
        && group_membership_is_member(rd::<GPtr<u8>>(this, 0).byte_add(8), src) != 0
    {
        return 0;
    }
    let inner = rd::<GPtr<u8>>(ev, 0x10);
    if v_has_editable_response(inner) != 0 {
        compute_response_task_type_group(inner, rd::<GPtr<u8>>(this, 0));
        if will_respond(inner) == 0 {
            return 0;
        }
    }
    let cur = rd::<GPtr<u8>>(this, 8);
    if cur.is_null() {
        let c = v_event_clone(ev);
        wr(this, 8, c);
        return 1;
    }
    if base_event_takes_priority(ev, cur) == 0 {
        return 0;
    }
    let cur = rd::<GPtr<u8>>(this, 8);
    if !cur.is_null() {
        v_delete(cur, 1);
    }
    let c = v_event_clone(ev);
    wr(this, 8, c);
    1
}

/// `FUN_0061a3a0` (0x61A3A0): `CTaskSimple` base destructor: vtable 0x86D48C.
#[hook(addr = 0x61A3A0, conv = thiscall, name = "FUN_0061a3a0", expect = "C7 01 8C D4 86 00 C3",
       route = "L2", hazards = "heap-writes")]
pub fn task_simple_dtor_61a3a0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D48C);
}

/// `FUN_0061a3d0` (0x61A3D0): `CTaskComplex` base destructor: vtable 0x86D4CC, deletes the sub-task (+8) when set
/// (scalar deleting destructor, flag 1), +8 = 0, vtable 0x86D48C.
#[hook(addr = 0x61A3D0, conv = thiscall, name = "FUN_0061a3d0", expect = "6A FF 68 E8 F0 83 00",
       route = "L2", hazards = "heap-writes")]
pub fn task_complex_dtor_61a3d0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D4CC);
    let s = rd::<GPtr<u8>>(this, 8);
    if !s.is_null() {
        v_delete(s, 1);
    }
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0, 0x86D48C);
}

/// `FUN_0048e600` (0x48E600): complete destructor: vtable 0x85A200, then tail call of 0x669D20.
#[hook(addr = 0x48E600, conv = thiscall, name = "FUN_0048e600", expect = "C7 01 00 A2 85 00",
       route = "L2", hazards = "heap-writes")]
pub fn dtor_48e600(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85A200);
    dtor_669d20(this);
}

/// `FUN_0066f670` (0x66F670): `CTaskComplexWander` destructor: vtable 0x86FE84, then tail call of the
/// `CTaskComplex` base destructor (0x61A3D0).
#[hook(addr = 0x66F670, conv = thiscall, name = "FUN_0066f670", expect = "C7 01 84 FE 86 00",
       route = "L2", hazards = "heap-writes")]
pub fn dtor_66f670(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FE84);
    task_complex_dtor(this);
}

/// `FUN_005fe9c0(int32 pedType)` (0x5FE9C0, cdecl): 1 for the gang ped types 7..16 (signed compares), else 0 (the
/// `return 1` path goes through the chunk at 0x401DFA).
#[hook(addr = 0x5FE9C0, conv = cdecl, name = "FUN_005fe9c0", expect = "8B 44 24 04 83 F8 07",
       route = "L2", hazards = "none")]
pub fn is_gang_ped_type(ty: i32) -> u32 {
    u32::from((7..=16).contains(&ty))
}

/// `CConversationNode::ClearRecursively` (0x43A710): recurses into both child nodes (+8 and +0xA, signed node
/// indices into `CConversations::m_aNodes`, skipped when negative), then clears the node: +0 = 0 (byte),
/// +0xA = +8 = -1 (words), +0x10 = +0x14 = +0xC = 0.
/// Not hooked (WRITER-MVP-B): callers 0x43A786 and 0x156ADE2 read EDX after the call (the original
/// never touches it), so the hook needs `preserve = "edx"`, which in turn needs an L1 preserve spec.
pub fn conversation_node_clear_recursively(this: GPtr<u8>) {
    let a = rd::<i16>(this, 8);
    if a >= 0 {
        conversation_node_clear_recursively_call(GPtr::from_addr(CONVERSATION_NODES + 0x18 * a as usize));
    }
    let b = rd::<i16>(this, 0xA);
    if b >= 0 {
        conversation_node_clear_recursively_call(GPtr::from_addr(CONVERSATION_NODES + 0x18 * b as usize));
    }
    wr::<u8>(this, 0, 0);
    wr::<u16>(this, 0xA, 0xFFFF);
    wr::<u16>(this, 8, 0xFFFF);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0xC, 0);
}

orig! {
    /// `CConversationNode::ClearRecursively` (0x43A710), the self-call (through the original address, so the
    /// hook toggle decides)
    fn conversation_node_clear_recursively_call(n: GPtr<u8>) = thiscall 0x43A710 => crate::mvpb_ai::conversation_node_clear_recursively;
}

/// `CTaskComplexFacial::StopAll` (0x691250): sets the stop flag (+0xE) when the sub-task (+8) is a
/// `CTaskSimpleFacial` (type 0x12F).
#[hook(addr = 0x691250, conv = thiscall, name = "CTaskComplexFacial::StopAll", this = "CTaskComplexFacial",
       expect = "56 8B F1 8B 4E 08", route = "L2", hazards = "heap-writes")]
pub fn task_complex_facial_stop_all(this: GPtr<u8>) {
    if v_get_task_type(rd::<GPtr<u8>>(this, 8)) == 0x12F {
        wr::<u8>(this, 0xE, 1);
    }
}

/// `CEntityScanner::Clear` (0x5FF9D0): releases and clears the 16 entity slots (+0xC) and the closest entity
/// (+0x4C).
#[hook(addr = 0x5FF9D0, conv = thiscall, name = "CEntityScanner::Clear", this = "CEntityScanner",
       expect = "53 56 8B D9 57", route = "L2", hazards = "heap-writes")]
pub fn entity_scanner_clear(this: GPtr<u8>) {
    for i in 0..16isize {
        let off = 0xC + 4 * i;
        let e = rd::<GPtr<u8>>(this, off);
        if !e.is_null() {
            clean_up_old_reference(e, this.byte_add(off));
            wr::<u32>(this, off, 0);
        }
    }
    let e = rd::<GPtr<u8>>(this, 0x4C);
    if !e.is_null() {
        clean_up_old_reference(e, this.byte_add(0x4C));
        wr::<u32>(this, 0x4C, 0);
    }
}

/// `FUN_004ab980` (0x4AB980): `CEventGroup` destructor body: vtable 0x85AAB0, then `CEventGroup::Flush(false)`;
/// EAX is Flush's.
#[hook(addr = 0x4AB980, conv = thiscall, name = "FUN_004ab980", expect = "6A 00 C7 01 B0 AA 85 00",
       route = "L2", hazards = "heap-writes")]
pub fn event_group_dtor_4ab980(this: GPtr<u8>) -> u32 {
    wr::<u32>(this, 0, 0x85AAB0);
    event_group_flush(this, 0)
}

/// `delete` (0x6074E0, cdecl): frees a `CPedIntelligence` slot (0x294 bytes) of the pool at 0xB744C0: index =
/// (p - objects) / 0x294 (signed), flag byte |= 0x80, first-free (+0xC) lowered to it.
#[hook(addr = 0x6074E0, conv = cdecl, name = "delete", expect = "8B 4C 24 04 56",
       route = "L2", hazards = "none")]
pub fn ped_intelligence_delete(p: GPtr<u8>) {
    let pool = rd::<GPtr<u8>>(GPtr::from_addr(0xB744C0), 0);
    let d = (p.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32;
    let idx = d / 0x294;
    let flags = rd::<GPtr<u8>>(pool, 4);
    let f = flags.byte_add(idx as isize);
    f.write(f.read() | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

/// `FUN_00683840(CEntity*)` (0x683840, `ret 4`, a `CTaskComplexBeInCouple` helper): with R = the right vector of
/// the ped's (+0x10) matrix (read without a null check), returns 2 when `R.(other pos) - R.(ped pos) < 0` (x87: each
/// dot is `(Ry*Py + Rz*Pz) + Rx*Px`, unordered also gives 2), else 1. Positions come from the matrix (+0x30) when
/// one exists, else from the placement (+4).
#[hook(addr = 0x683840, conv = thiscall, name = "FUN_00683840", expect = "83 EC 0C 8B 49 10",
       unverified = "mvpb-ai-1", route = "L2", hazards = "none")]
pub fn be_in_couple_side(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let ped = rd::<GPtr<u8>>(this, 0x10);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let r = [rd::<f32>(m, 0), rd::<f32>(m, 4), rd::<f32>(m, 8)];
    let pos = |e: GPtr<u8>| {
        let mm = rd::<GPtr<u8>>(e, 0x14);
        if mm.is_null() { e.byte_add(4) } else { mm.byte_add(0x30) }
    };
    let dot = |p: GPtr<u8>| {
        x(r[1]).fmul_m(rd::<f32>(p, 4)).fadd(x(r[2]).fmul_m(rd::<f32>(p, 8))).fadd(x(r[0]).fmul_m(rd::<f32>(p, 0)))
    };
    let a = dot(pos(ped)).fchs();
    let b = dot(pos(other));
    let s = b.fadd(a);
    if s.fcom(X87::from_f32(0.0)).c0() { 2 } else { 1 }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_COVER_SHOULD_BUILDING_HAVE_COVER_POINTS,
    HOOK_COVER_REMOVE_POINTS_FOR_ENTITY,
    HOOK_COVER_FIND_POINTS_FOR_BUILDING,
    HOOK_TASK_COMPLEX_FACIAL_CTOR,
    HOOK_EVENT_DTOR_4AF890,
    HOOK_EVENT_VEHICLE_TO_STEAL_DTOR,
    HOOK_EVENT_VEHICLE_COLLISION_DTOR,
    HOOK_EVENT_DTOR_5DEA00,
    HOOK_EVENT_CTOR_4AEDF0,
    HOOK_EVENT_DTOR_4AEEA0,
    HOOK_EVENT_CTOR_4AECD0,
    HOOK_EVENT_DTOR_4AED60,
    HOOK_CAR_SET_PED_IN_AS_DRIVER_CTOR,
    HOOK_CAR_SET_PED_IN_AS_DRIVER_DTOR,
    HOOK_CAR_SET_PED_IN_AS_PASSENGER_CTOR,
    HOOK_TASK_COMPLEX_WANDER_CTOR,
    HOOK_GROUP_MEMBERSHIP_IS_LEADER,
    HOOK_PGI_ADD_EVENT,
    HOOK_TASK_SIMPLE_DTOR_61A3A0,
    HOOK_TASK_COMPLEX_DTOR_61A3D0,
    HOOK_DTOR_48E600,
    HOOK_DTOR_66F670,
    HOOK_IS_GANG_PED_TYPE,
    HOOK_TASK_COMPLEX_FACIAL_STOP_ALL,
    HOOK_ENTITY_SCANNER_CLEAR,
    HOOK_EVENT_GROUP_DTOR_4AB980,
    HOOK_PED_INTELLIGENCE_DELETE,
    HOOK_BE_IN_COUPLE_SIDE,
];
