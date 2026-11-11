//! `CEvent*`, `CEventEditableResponse`, `CEventGroupEvent`, `CEventHandler`/`CEventScanner` pieces and the
//! inform-friends/group event queues (PORT-AI-1, helper B). Ported from the 1.0 US bytes
//! (`docs/re/specs/AI/CEvent*.md`, `_unattributed-I1.md`); the exe wins over gta-reversed.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEventPool::New` (0x4B5570, thiscall on the pool): a free event pool slot or null.
    fn event_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B5570 => unported;
    /// `CEventAcquaintancePed::AffectsPed(CPed*)` (0x4AFA30)
    fn acquaintance_ped_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4AFA30 => unported;
    /// `CPedGroupMembership::IsMember(CPed*)` (0x5F6A10)
    fn membership_is_member(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn ev_get_event_type(ev: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetEventPriority` (vtable +8)
    fn ev_get_event_priority(ev: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CEvent::GetLifeTime` (vtable +0xC)
    fn ev_get_life_time(ev: GPtr<u8>) -> i32 = thiscall vslot 0xC => unported;
}

/// The event pool (`CPools::ms_pEventPool`).
fn event_pool() -> GPtr<u8> {
    g::<GPtr<u8>>(EVENT_POOL).read()
}

// ---- leaves ------------------------------------------------------------------------------------

/// `CEvent::ReportCriminalEvent(CPed*)` (0x420EA0): `ret 4`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
#[sa_core::native(addr = 0x420EA0, conv = thiscall, name = "CEvent::ReportCriminalEvent")]
pub fn event_report_criminal_event_420ea0(_this: GPtr<u8>, _ped: GPtr<u8>) {}

/// `CEventSeenCop::AffectsPed(CPed*)` (0x4AFB40): tail jump to `CEventAcquaintancePed::AffectsPed`.
/// Not hookable: a bare `jmp rel32` entry is taken for a `.HOODLUM` stub by sa-hook (its target is not in `.HOODLUM`).
#[sa_core::native(addr = 0x4AFB40, conv = thiscall, name = "CEventSeenCop::AffectsPed")]
pub fn event_seen_cop_affects_ped_4afb40(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    acquaintance_ped_affects_ped(this, ped)
}

/// `CEvent::operator new(uint32)` (0x4B5620): `ecx = CPools::ms_pEventPool; jmp CEventPool::New`.
#[hook(addr = 0x4B5620, conv = cdecl, name = "CEvent::operator new", expect = "8B 0D AC 44 B7 00")]
pub fn event_operator_new_4b5620(_size: u32) -> GPtr<u8> {
    event_pool_new(event_pool())
}

/// `CEvent::operator delete(void*)` (0x4B5630): the inlined `CPool::Delete`: slot index
/// `(p - objects) / 0x44` (signed, truncating), `flags[i] |= 0x80`, `firstFree = min(firstFree, i)`.
#[hook(addr = 0x4B5630, conv = cdecl, name = "CEvent::operator delete", expect = "8B 4C 24 04 56")]
pub fn event_operator_delete_4b5630(p: GPtr<u8>) {
    let pool = event_pool();
    let objs = rd::<u32>(pool, 0);
    let idx = (p.addr() as u32).wrapping_sub(objs) as i32 / 0x44;
    let flags = rd::<GPtr<u8>>(pool, 4);
    let f = at::<u8>(flags, idx as isize);
    f.write(f.read() | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

/// `CEventEditableResponse::~CEventEditableResponse` (0x4AC480): `nop; jmp 0x402129` (protection-area
/// stub: `vtbl = 0x85AB38` (CEvent), back to the `ret` at 0x4AC486).
/// Not hookable: protection code reads the patched bytes (0x407EA1 reads 0x4AC482, INF-20), and a
/// caller (0x4B5671) reads ECX afterwards.
#[sa_core::native(addr = 0x4AC480, conv = thiscall, name = "CEventEditableResponse::~CEventEditableResponse")]
pub fn event_editable_response_dtor_4ac480(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AB38);
}

/// `vt_85ae88::GetSourceEntity` (0x4AE230): the ped at +0x14 → its +0x460 pointer, or null.
#[hook(addr = 0x4AE230, conv = thiscall, name = "vt_85ae88::GetSourceEntity", expect = "8B 41 14 85 C0")]
pub fn vt_85ae88_get_source_entity_4ae230(this: GPtr<u8>) -> u32 {
    let p = rd::<GPtr<u8>>(this, 0x14);
    if p.is_null() { 0 } else { rd::<u32>(p, 0x460) }
}

/// `CEventGroupEvent::AffectsPedGroup(CPedGroup*)` (0x4AE0E0): `group->membership.IsMember(this->ped)`.
#[hook(addr = 0x4AE0E0, conv = thiscall, name = "CEventGroupEvent::AffectsPedGroup", expect = "8B 41 0C 8B 4C 24 04")]
pub fn event_group_event_affects_ped_group_4ae0e0(this: GPtr<u8>, group: GPtr<u8>) -> u8 {
    let ped = rd::<GPtr<u8>>(this, 0xC);
    u8::from(membership_is_member(group.byte_add(8), ped) != 0)
}

/// `CEventInAir::Clone` (0x4B76A0): inlined `operator new` + ctor (`+4 = 0`, `+8 = 0`, `vtbl = 0x85B3C0`).
#[hook(addr = 0x4B76A0, conv = thiscall, name = "CEventInAir::Clone", expect = "8B 0D AC 44 B7 00")]
pub fn event_in_air_clone_4b76a0(_this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(e, 4, 0);
    wr::<u8>(e, 8, 0);
    wr::<u32>(e, 0, 0x85B3C0);
    e
}

/// `CEvent::IsValid(CPed*)` (0x4AC020): `m_bValid (+8) || GetLifeTime() >= m_nTime (+4)` (EAX = 0/1).
#[hook(addr = 0x4AC020, conv = thiscall, name = "CEvent::IsValid", expect = "56 8B F1 8A 46 08")]
pub fn event_is_valid_4ac020(this: GPtr<u8>, _ped: GPtr<u8>) -> u32 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    if rd::<i32>(this, 4) > ev_get_life_time(this) {
        0
    } else {
        1
    }
}

/// `CEventBuildingCollision::TakesPriorityOver(const CEvent&)` (0x4AD050): the event types differ
/// (`other.GetEventType()` first, then this one's).
#[hook(addr = 0x4AD050, conv = thiscall, name = "CEventBuildingCollision::TakesPriorityOver", expect = "56 8B F1 8B 4C 24 08")]
pub fn event_building_collision_takes_priority_over_4ad050(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    let o = ev_get_event_type(other);
    u8::from(o != ev_get_event_type(this))
}

/// `CEvent::TakesPriorityOver(const CEvent&)` (0x4ABC30): `GetEventPriority() >= other.GetEventPriority()`
/// (other's first).
#[hook(addr = 0x4ABC30, conv = thiscall, name = "CEvent::TakesPriorityOver", expect = "56 8B F1 8B 4C 24 08")]
pub fn event_takes_priority_over_4abc30(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    let o = ev_get_event_priority(other);
    u8::from(ev_get_event_priority(this) >= o)
}

/// `vt_85b070::AffectsPed(CPed*)` (0x4AEDC0): `ped->IsAlive() && this[+0x10] != null`.
#[hook(addr = 0x4AEDC0, conv = thiscall, name = "vt_85b070::AffectsPed", expect = "56 8B F1 8B 4C 24 08")]
pub fn vt_85b070_affects_ped_4aedc0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    u8::from(ped_is_alive(ped) != 0 && rd::<u32>(this, 0x10) != 0)
}

/// `vt_85b0f8::AffectsPed(CPed*)` (0x4AF100): `ped->IsAlive() && this[+0xC] != null`.
#[hook(addr = 0x4AF100, conv = thiscall, name = "vt_85b0f8::AffectsPed", expect = "56 8B F1 8B 4C 24 08")]
pub fn vt_85b0f8_affects_ped_4af100(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    u8::from(ped_is_alive(ped) != 0 && rd::<u32>(this, 0xC) != 0)
}

/// `vt_85ae88::AffectsPed(CPed*)` (0x4AE2D0): alive, no `+0xFC` (attached-to), `+0x46C` bit 8 clear,
/// source ped (+0x14) set and its `+0x590` (vehicle type) != 5.
#[hook(addr = 0x4AE2D0, conv = thiscall, name = "vt_85ae88::AffectsPed", expect = "56 8B 74 24 08")]
pub fn vt_85ae88_affects_ped_4ae2d0(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 || rd::<u32>(ped, 0xFC) != 0 || rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return 0;
    }
    let src = rd::<GPtr<u8>>(this, 0x14);
    if src.is_null() {
        return 0;
    }
    u8::from(rd::<u32>(src, 0x590) != 5)
}

// ---- scalar deleting destructors: `call dtor; if (flags & 1) CEvent::operator delete; return this` ---

orig! {
    fn dtor_420f30(this: GPtr<u8>) = thiscall 0x420F30 => unported;
    fn dtor_5dea00(this: GPtr<u8>) = thiscall 0x5DEA00 => unported;
    fn dtor_5f6200(this: GPtr<u8>) = thiscall 0x5F6200 => unported;
    fn dtor_5fed80(this: GPtr<u8>) = thiscall 0x5FED80 => unported;
    fn dtor_5ff030(this: GPtr<u8>) = thiscall 0x5FF030 => unported;
    fn dtor_5ff0f0(this: GPtr<u8>) = thiscall 0x5FF0F0 => unported;
    fn dtor_5ff1b0(this: GPtr<u8>) = thiscall 0x5FF1B0 => unported;
    fn dtor_5ff3a0(this: GPtr<u8>) = thiscall 0x5FF3A0 => unported;
    fn dtor_65dc10(this: GPtr<u8>) = thiscall 0x65DC10 => unported;
    fn dtor_6a0650(this: GPtr<u8>) = thiscall 0x6A0650 => unported;
}

fn sdd(this: GPtr<u8>, flags: u32, dtor: fn(GPtr<u8>)) -> GPtr<u8> {
    dtor(this);
    if flags & 1 != 0 {
        event_delete(this);
    }
    this
}

/// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x420FB0): scalar deleting, dtor `FUN_00420f30`.
#[hook(addr = 0x420FB0, conv = thiscall, name = "CEventAcquaintancePed::~CEventAcquaintancePed", expect = "56 8B F1 E8 78 FF FF FF")]
pub fn event_acquaintance_ped_sdd_420fb0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_420f30)
}

/// `CEventSoundQuiet::~CEventSoundQuiet` (0x5DEAB0): scalar deleting, dtor `FUN_005dea00`.
#[hook(addr = 0x5DEAB0, conv = thiscall, name = "CEventSoundQuiet::~CEventSoundQuiet", expect = "56 8B F1 E8 48 FF FF FF")]
pub fn event_sound_quiet_sdd_5deab0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5dea00)
}

/// `vt_86c6d0::Destructor` (0x5F6300): scalar deleting, dtor `FUN_005f6200`.
#[hook(addr = 0x5F6300, conv = thiscall, name = "vt_86c6d0::Destructor", expect = "56 8B F1 E8 F8 FE FF FF")]
pub fn vt_86c6d0_sdd_5f6300(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5f6200)
}

/// `CEventPedCollisionWithPlayer::~CEventPedCollisionWithPlayer` (0x5FEE20): scalar deleting, dtor `FUN_005fed80`.
#[hook(addr = 0x5FEE20, conv = thiscall, name = "CEventPedCollisionWithPlayer::~CEventPedCollisionWithPlayer", expect = "56 8B F1 E8 58 FF FF FF")]
pub fn event_ped_collision_with_player_sdd_5fee20(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5fed80)
}

/// `vt_86ca38::Destructor` (0x5FF0B0): scalar deleting, dtor `FUN_005ff030`.
#[hook(addr = 0x5FF0B0, conv = thiscall, name = "vt_86ca38::Destructor", expect = "56 8B F1 E8 78 FF FF FF")]
pub fn vt_86ca38_sdd_5ff0b0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5ff030)
}

/// `vt_86ca80::Destructor` (0x5FF170): scalar deleting, dtor `FUN_005ff0f0`.
#[hook(addr = 0x5FF170, conv = thiscall, name = "vt_86ca80::Destructor", expect = "56 8B F1 E8 78 FF FF FF")]
pub fn vt_86ca80_sdd_5ff170(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5ff0f0)
}

/// `vt_86cac8::Destructor` (0x5FF230): scalar deleting, dtor `FUN_005ff1b0`.
#[hook(addr = 0x5FF230, conv = thiscall, name = "vt_86cac8::Destructor", expect = "56 8B F1 E8 78 FF FF FF")]
pub fn vt_86cac8_sdd_5ff230(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5ff1b0)
}

/// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x5FF420): scalar deleting, dtor `FUN_005ff3a0`.
#[hook(addr = 0x5FF420, conv = thiscall, name = "CEventAcquaintancePed::~CEventAcquaintancePed", expect = "56 8B F1 E8 78 FF FF FF")]
pub fn event_acquaintance_ped_sdd_5ff420(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_5ff3a0)
}

/// `CEventLeanOnVehicle::~CEventLeanOnVehicle` (0x65DBF0): scalar deleting, dtor `FUN_0065dc10`.
#[hook(addr = 0x65DBF0, conv = thiscall, name = "CEventLeanOnVehicle::~CEventLeanOnVehicle", expect = "56 8B F1 E8 18 00 00 00")]
pub fn event_lean_on_vehicle_sdd_65dbf0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_65dc10)
}

/// `CEventVehicleDamageCollision::~CEventVehicleDamageCollision` (0x6A06D0): scalar deleting, dtor `FUN_006a0650`.
#[hook(addr = 0x6A06D0, conv = thiscall, name = "CEventVehicleDamageCollision::~CEventVehicleDamageCollision", expect = "56 8B F1 E8 78 FF FF FF")]
pub fn event_vehicle_damage_collision_sdd_6a06d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd(this, flags, dtor_6a0650)
}

// ---- scalar deleting destructors with the pool delete inlined -----------------------------------

/// The inlined `CPool::Delete` of the scalar deleting destructors: slot `(p - objects) / size`
/// (signed, truncating), `flags[i] |= 0x80`, `firstFree = min(firstFree, i)`.
pub(crate) fn pool_delete_inline(pool_ptr: usize, p: GPtr<u8>, size: i32) {
    let pool = g::<GPtr<u8>>(pool_ptr).read();
    let idx = (p.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / size;
    let f = at::<u8>(rd::<GPtr<u8>>(pool, 4), idx as isize);
    f.write(f.read() | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

fn sdd_inline(this: GPtr<u8>, flags: u32, dtor: fn(GPtr<u8>)) -> GPtr<u8> {
    dtor(this);
    if flags & 1 != 0 {
        pool_delete_inline(EVENT_POOL, this, 0x44);
    }
    this
}

orig! {
    fn dtor_4aca70(this: GPtr<u8>) = thiscall 0x4ACA70 => unported;
    fn dtor_4acdd0(this: GPtr<u8>) = thiscall 0x4ACDD0 => unported;
    fn dtor_4acff0(this: GPtr<u8>) = thiscall 0x4ACFF0 => unported;
    fn dtor_4ae070(this: GPtr<u8>) = thiscall 0x4AE070 => unported;
    fn dtor_4ae3c0(this: GPtr<u8>) = thiscall 0x4AE3C0 => unported;
    fn dtor_4ae7a0(this: GPtr<u8>) = thiscall 0x4AE7A0 => unported;
    fn dtor_4aed60(this: GPtr<u8>) = thiscall 0x4AED60 => unported;
    fn dtor_4aeea0(this: GPtr<u8>) = thiscall 0x4AEEA0 => unported;
    fn dtor_4af0a0(this: GPtr<u8>) = thiscall 0x4AF0A0 => unported;
    fn dtor_4af400(this: GPtr<u8>) = thiscall 0x4AF400 => unported;
    fn dtor_4b0a50(this: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    fn dtor_4b0d00(this: GPtr<u8>) = thiscall 0x4B0D00 => unported;
}

/// `CEventPedCollisionWithPed::~CEventPedCollisionWithPed` (0x4B5980): `FUN_004aca70`, inlined pool delete.
#[hook(addr = 0x4B5980, conv = thiscall, name = "CEventPedCollisionWithPed::~CEventPedCollisionWithPed", expect = "56 57 8B F9 E8 E7 70 FF FF")]
pub fn event_ped_collision_with_ped_sdd_4b5980(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4aca70)
}

/// `CEventObjectCollision::~CEventObjectCollision` (0x4B59D0): `FUN_004acdd0`, inlined pool delete.
#[hook(addr = 0x4B59D0, conv = thiscall, name = "CEventObjectCollision::~CEventObjectCollision", expect = "56 57 8B F9 E8 F7 73 FF FF")]
pub fn event_object_collision_sdd_4b59d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4acdd0)
}

/// `CEventBuildingCollision::~CEventBuildingCollision` (0x4B5A20): `FUN_004acff0`, inlined pool delete.
#[hook(addr = 0x4B5A20, conv = thiscall, name = "CEventBuildingCollision::~CEventBuildingCollision", expect = "56 57 8B F9 E8 C7 75 FF FF")]
pub fn event_building_collision_sdd_4b5a20(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4acff0)
}

/// `CEventGroupEvent::~CEventGroupEvent` (0x4B5E50): the complete dtor 0x4AE070, inlined pool delete.
#[hook(addr = 0x4B5E50, conv = thiscall, name = "CEventGroupEvent::~CEventGroupEvent", expect = "56 57 8B F9 E8 17 82 FF FF")]
pub fn event_group_event_sdd_4b5e50(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4ae070)
}

/// `CEventPotentialWalkIntoVehicle::~CEventPotentialWalkIntoVehicle` (0x4B5EF0): `FUN_004ae3c0`, inlined pool delete.
#[hook(addr = 0x4B5EF0, conv = thiscall, name = "CEventPotentialWalkIntoVehicle::~CEventPotentialWalkIntoVehicle", expect = "56 57 8B F9 E8 C7 84 FF FF")]
pub fn event_potential_walk_into_vehicle_sdd_4b5ef0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4ae3c0)
}

/// `CEventPotentialWalkIntoPed::~CEventPotentialWalkIntoPed` (0x4B5F90): `FUN_004ae7a0`, inlined pool delete.
#[hook(addr = 0x4B5F90, conv = thiscall, name = "CEventPotentialWalkIntoPed::~CEventPotentialWalkIntoPed", expect = "56 57 8B F9 E8 07 88 FF FF")]
pub fn event_potential_walk_into_ped_sdd_4b5f90(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4ae7a0)
}

/// `vt_85b070::Destructor` (0x4B60D0): `FUN_004aed60`, inlined pool delete.
#[hook(addr = 0x4B60D0, conv = thiscall, name = "vt_85b070::Destructor", expect = "56 57 8B F9 E8 87 8C FF FF")]
pub fn vt_85b070_sdd_4b60d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4aed60)
}

/// `vt_85b0b0::Destructor` (0x4B6120): `FUN_004aeea0`, inlined pool delete.
#[hook(addr = 0x4B6120, conv = thiscall, name = "vt_85b0b0::Destructor", expect = "56 57 8B F9 E8 77 8D FF FF")]
pub fn vt_85b0b0_sdd_4b6120(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4aeea0)
}

/// `vt_85b0f8::Destructor` (0x4B6170): `FUN_004af0a0`, inlined pool delete.
#[hook(addr = 0x4B6170, conv = thiscall, name = "vt_85b0f8::Destructor", expect = "56 57 8B F9 E8 27 8F FF FF")]
pub fn vt_85b0f8_sdd_4b6170(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4af0a0)
}

/// `CEventAttractor::~CEventAttractor` (0x4B6260): `FUN_004af400`, inlined pool delete.
#[hook(addr = 0x4B6260, conv = thiscall, name = "CEventAttractor::~CEventAttractor", expect = "56 57 8B F9 E8 97 91 FF FF")]
pub fn event_attractor_sdd_4b6260(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4af400)
}

/// `CEventScriptCommand::~CEventScriptCommand` (0x4B6440): `FUN_004b0a50`, inlined pool delete.
#[hook(addr = 0x4B6440, conv = thiscall, name = "CEventScriptCommand::~CEventScriptCommand", expect = "56 57 8B F9 E8 07 A6 FF FF")]
pub fn event_script_command_sdd_4b6440(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4b0a50)
}

/// `CEventInAir::~CEventInAir` (0x4B64D0): `FUN_004b0d00` (`vtbl = 0x85AB38`), inlined pool delete.
/// The original keeps using ECX after the call (the callee leaves it = `this`); the port uses `this`.
#[hook(addr = 0x4B64D0, conv = thiscall, name = "CEventInAir::~CEventInAir", expect = "56 E8 2A A8 FF FF")]
pub fn event_in_air_sdd_4b64d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, dtor_4b0d00)
}

// ---- Clone / CloneEditable ----------------------------------------------------------------------

orig! {
    fn ctor_4aedf0(this: GPtr<u8>, a: u32) -> GPtr<u8> = thiscall 0x4AEDF0 => unported;
    fn ctor_4af010(this: GPtr<u8>, a: u32) -> GPtr<u8> = thiscall 0x4AF010 => unported;
    /// `CEventAcquaintancePed::Constructor(CPed*)` (0x4AF820)
    fn acquaintance_ped_ctor(this: GPtr<u8>, ped: u32) -> GPtr<u8> = thiscall 0x4AF820 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0)
    fn group_event_ctor(this: GPtr<u8>, ped: u32, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn ev_clone(ev: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEventPotentialWalkIntoVehicle::Constructor` (0x4AE320)
    fn walk_into_vehicle_ctor(this: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AE320 => unported;
    fn ctor_4aecd0(this: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AECD0 => unported;
    /// `CEventLeanOnVehicle::Constructor` (0x65DAF0)
    fn lean_on_vehicle_ctor(this: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x65DAF0 => unported;
    /// `CEventPotentialWalkIntoPed::Constructor` (0x4AE6E0)
    fn walk_into_ped_ctor(this: GPtr<u8>, a: u32, b: GPtr<u8>, c: u32) -> GPtr<u8> = thiscall 0x4AE6E0 => unported;
    fn ctor_5f6190(this: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x5F6190 => unported;
    /// `CEventObjectCollision::Constructor` (0x4ACCF0)
    fn object_collision_ctor(this: GPtr<u8>, a: u32, b: u32, c: u32, d: GPtr<u8>, e: GPtr<u8>, f: i32) -> GPtr<u8> = thiscall 0x4ACCF0 => unported;
    /// `CEventBuildingCollision::Constructor` (0x4ACF00)
    fn building_collision_ctor(this: GPtr<u8>, a: u32, b: u32, c: u32, d: GPtr<u8>, e: GPtr<u8>, f: i32) -> GPtr<u8> = thiscall 0x4ACF00 => unported;
    /// `CEventPedCollisionWithPed::Constructor` (0x4AC990)
    fn ped_collision_ctor(this: GPtr<u8>, a: u32, b: u32, c: u32, d: GPtr<u8>, e: GPtr<u8>, f: i32, h: i32) -> GPtr<u8> = thiscall 0x4AC990 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `FUN_004ac010` (0x4AC010): `CEvent` base destructor
    fn event_base_dtor(this: GPtr<u8>) = thiscall 0x4AC010 => unported;
}

/// `vt_85b0b0::CloneEditable` (0x4B7280): inlined `operator new`, `FUN_004aedf0(e, this[+0x14])`.
#[hook(addr = 0x4B7280, conv = thiscall, name = "vt_85b0b0::CloneEditable", expect = "6A FF 68 7B A5 83 00")]
pub fn vt_85b0b0_clone_editable_4b7280(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        GPtr::NULL
    } else {
        ctor_4aedf0(e, rd::<u32>(this, 0x14))
    }
}

/// `vt_85b0f8::Clone` (0x4B72F0): inlined `operator new`, `FUN_004af010(e, this[+0xC])`.
#[hook(addr = 0x4B72F0, conv = thiscall, name = "vt_85b0f8::Clone", expect = "6A FF 68 9B A5 83 00")]
pub fn vt_85b0f8_clone_4b72f0(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        GPtr::NULL
    } else {
        ctor_4af010(e, rd::<u32>(this, 0xC))
    }
}

/// The `CEventAcquaintancePed`-derived `CloneEditable`s: `operator new(0x18)`, base ctor with the
/// ped at +0x14, then the derived vtable; returns the event (or null).
fn acquaintance_clone(this: GPtr<u8>, vtbl: u32) -> GPtr<u8> {
    let e = event_new(0x18);
    if e.is_null() {
        return GPtr::NULL;
    }
    acquaintance_ped_ctor(e, rd::<u32>(this, 0x14));
    wr::<u32>(e, 0, vtbl);
    e
}

/// `CEventAcquaintancePedHate::CloneEditable` (0x420F50): vtable 0x858E68.
/// Not hookable: protection code reads the patched bytes (`push 0x420F54` at 0x14674BE, INF-20).
#[sa_core::native(addr = 0x420F50, conv = thiscall, name = "CEventAcquaintancePedHate::CloneEditable")]
pub fn event_acquaintance_ped_hate_clone_editable_420f50(this: GPtr<u8>) -> GPtr<u8> {
    acquaintance_clone(this, 0x858E68)
}

/// `vt_86ca38::CloneEditable` (0x5FF050): vtable 0x86CA38.
#[hook(addr = 0x5FF050, conv = thiscall, name = "vt_86ca38::CloneEditable", expect = "6A FF 68 CB E5 83 00")]
pub fn vt_86ca38_clone_editable_5ff050(this: GPtr<u8>) -> GPtr<u8> {
    acquaintance_clone(this, 0x86CA38)
}

/// `vt_86ca80::CloneEditable` (0x5FF110): vtable 0x86CA80.
#[hook(addr = 0x5FF110, conv = thiscall, name = "vt_86ca80::CloneEditable", expect = "6A FF 68 EB E5 83 00")]
pub fn vt_86ca80_clone_editable_5ff110(this: GPtr<u8>) -> GPtr<u8> {
    acquaintance_clone(this, 0x86CA80)
}

/// `vt_86cac8::CloneEditable` (0x5FF1D0): vtable 0x86CAC8.
#[hook(addr = 0x5FF1D0, conv = thiscall, name = "vt_86cac8::CloneEditable", expect = "6A FF 68 0B E6 83 00")]
pub fn vt_86cac8_clone_editable_5ff1d0(this: GPtr<u8>) -> GPtr<u8> {
    acquaintance_clone(this, 0x86CAC8)
}

/// `CEventSeenCop::CloneEditable` (0x5FF3C0): vtable 0x86CB58.
#[hook(addr = 0x5FF3C0, conv = thiscall, name = "CEventSeenCop::CloneEditable", expect = "6A FF 68 4B E6 83 00")]
pub fn event_seen_cop_clone_editable_5ff3c0(this: GPtr<u8>) -> GPtr<u8> {
    acquaintance_clone(this, 0x86CB58)
}

/// `CEventGroupEvent::Clone` (0x4B6EE0): inlined `operator new`; `ctor(e, ped +0xC, event(+0x10)->Clone())`.
#[hook(addr = 0x4B6EE0, conv = thiscall, name = "CEventGroupEvent::Clone", expect = "6A FF 68 7B A4 83 00")]
pub fn event_group_event_clone_4b6ee0(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    let c = ev_clone(rd::<GPtr<u8>>(this, 0x10));
    group_event_ctor(e, rd::<u32>(this, 0xC), c)
}

/// `CEventPotentialWalkIntoVehicle::CloneEditable` (0x4B6FB0): inlined `operator new`; ctor(+0x14, +0x18).
#[hook(addr = 0x4B6FB0, conv = thiscall, name = "CEventPotentialWalkIntoVehicle::CloneEditable", expect = "6A FF 68 BB A4 83 00")]
pub fn event_potential_walk_into_vehicle_clone_editable_4b6fb0(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        GPtr::NULL
    } else {
        walk_into_vehicle_ctor(e, rd::<u32>(this, 0x14), rd::<u32>(this, 0x18))
    }
}

/// `vt_85b070::Clone` (0x4B7210): inlined `operator new`; `FUN_004aecd0(e, (u8)+0xC, +0x10)`.
#[hook(addr = 0x4B7210, conv = thiscall, name = "vt_85b070::Clone", expect = "6A FF 68 5B A5 83 00")]
pub fn vt_85b070_clone_4b7210(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    ctor_4aecd0(e, u32::from(rd::<u8>(this, 0xC)), rd::<u32>(this, 0x10))
}

/// `CEventLeanOnVehicle::Clone` (0x65DB70): `operator new(0x14)`; ctor(+0xC, +0x10).
#[hook(addr = 0x65DB70, conv = thiscall, name = "CEventLeanOnVehicle::Clone", expect = "6A FF 68 BB 42 84 00")]
pub fn event_lean_on_vehicle_clone_65db70(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_new(0x14);
    if e.is_null() {
        GPtr::NULL
    } else {
        lean_on_vehicle_ctor(e, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10))
    }
}

/// `CEventPotentialWalkIntoPed::CloneEditable` (0x4B7090): inlined `operator new`; ctor(+0x20, &+0x14, +0x24).
#[hook(addr = 0x4B7090, conv = thiscall, name = "CEventPotentialWalkIntoPed::CloneEditable", expect = "6A FF 68 FB A4 83 00")]
pub fn event_potential_walk_into_ped_clone_editable_4b7090(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    walk_into_ped_ctor(
        e,
        rd::<u32>(this, 0x20),
        this.byte_add(0x14),
        rd::<u32>(this, 0x24),
    )
}

/// `vt_86c6d0::Clone` (0x5F6280): `operator new(0x1C)`; `FUN_005f6190(e, +0xC, +0x10, (u8)+0x14, +0x18)`.
#[hook(addr = 0x5F6280, conv = thiscall, name = "vt_86c6d0::Clone", expect = "6A FF 68 5B DE 83 00")]
pub fn vt_86c6d0_clone_5f6280(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_new(0x1C);
    if e.is_null() {
        return GPtr::NULL;
    }
    ctor_5f6190(
        e,
        rd::<u32>(this, 0xC),
        rd::<u32>(this, 0x10),
        u32::from(rd::<u8>(this, 0x14)),
        rd::<u32>(this, 0x18),
    )
}

/// `CEventObjectCollision::Clone` (0x4B6CC0): inlined `operator new`;
/// ctor((u16)+0xC, +0x10, +0x14, &+0x18, &+0x24, (i16)+0xE).
#[hook(addr = 0x4B6CC0, conv = thiscall, name = "CEventObjectCollision::Clone", expect = "6A FF 68 FB A3 83 00")]
pub fn event_object_collision_clone_4b6cc0(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    object_collision_ctor(
        e,
        u32::from(rd::<u16>(this, 0xC)),
        rd::<u32>(this, 0x10),
        rd::<u32>(this, 0x14),
        this.byte_add(0x18),
        this.byte_add(0x24),
        i32::from(rd::<i16>(this, 0xE)),
    )
}

/// `CEventBuildingCollision::Clone` (0x4B6D40): as [`event_object_collision_clone_4b6cc0`] with the
/// building ctor (0x4ACF00).
#[hook(addr = 0x4B6D40, conv = thiscall, name = "CEventBuildingCollision::Clone", expect = "6A FF 68 1B A4 83 00")]
pub fn event_building_collision_clone_4b6d40(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    building_collision_ctor(
        e,
        u32::from(rd::<u16>(this, 0xC)),
        rd::<u32>(this, 0x10),
        rd::<u32>(this, 0x14),
        this.byte_add(0x18),
        this.byte_add(0x24),
        i32::from(rd::<i16>(this, 0xE)),
    )
}

/// `CEventPedCollisionWithPed::Clone` (0x4B6C40): inlined `operator new`;
/// ctor((u16)+0xC, +0x10, +0x14, &+0x18, &+0x24, (i16)+0x30, (i16)+0x32).
#[hook(addr = 0x4B6C40, conv = thiscall, name = "CEventPedCollisionWithPed::Clone", expect = "6A FF 68 DB A3 83 00")]
pub fn event_ped_collision_with_ped_clone_4b6c40(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(event_pool());
    if e.is_null() {
        return GPtr::NULL;
    }
    ped_collision_ctor(
        e,
        u32::from(rd::<u16>(this, 0xC)),
        rd::<u32>(this, 0x10),
        rd::<u32>(this, 0x14),
        this.byte_add(0x18),
        this.byte_add(0x24),
        i32::from(rd::<i16>(this, 0x30)),
        i32::from(rd::<i16>(this, 0x32)),
    )
}

/// `CEventPedCollisionWithPlayer::Clone` (0x5FEDA0): `operator new(0x34)`, the
/// `CEventPedCollisionWithPed` ctor (the float at +0x10 goes through `fld`/`fstp`), then
/// `vtbl = 0x86C970`; returns the event (or null).
#[hook(addr = 0x5FEDA0, conv = thiscall, name = "CEventPedCollisionWithPlayer::Clone", expect = "6A FF 68 6B E5 83 00")]
pub fn event_ped_collision_with_player_clone_5feda0(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_new(0x34);
    if e.is_null() {
        return GPtr::NULL;
    }
    let f = x(rd::<f32>(this, 0x10)).to_f32().to_bits();
    ped_collision_ctor(
        e,
        u32::from(rd::<u16>(this, 0xC)),
        f,
        rd::<u32>(this, 0x14),
        this.byte_add(0x18),
        this.byte_add(0x24),
        i32::from(rd::<i16>(this, 0x30)),
        i32::from(rd::<i16>(this, 0x32)),
    );
    wr::<u32>(e, 0, 0x86C970);
    e
}

/// `FUN_0065dc10` (0x65DC10): `CEventLeanOnVehicle` complete destructor: `vtbl = 0x86F878`, clean up
/// the vehicle reference at +0xC, then the `CEvent` base dtor `FUN_004ac010`.
#[hook(addr = 0x65DC10, conv = thiscall, name = "FUN_0065dc10", expect = "6A FF 68 D8 42 84 00")]
pub fn event_lean_on_vehicle_dtor_65dc10(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F878);
    let v = rd::<GPtr<u8>>(this, 0xC);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0xC));
    }
    event_base_dtor(this);
}

orig! {
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// scalar deleting destructor (vtable +0)
    fn ev_delete(ev: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0): the inlined `CEvent` ctor
/// (`+4 = 0`, `+8 = 0`), `vtbl = 0x85AE48`, `+0xC = ped`, `+0x10 = event`, then
/// `ped->RegisterReference(&+0xC)` when `ped` is set; returns `this`.
#[hook(addr = 0x4ADFD0, conv = thiscall, name = "CEventGroupEvent::CEventGroupEvent", expect = "6A FF 68 48 9B 83 00")]
pub fn event_group_event_ctor_4adfd0(this: GPtr<u8>, ped: GPtr<u8>, ev: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0, 0x85AE48);
    wr::<u32>(this, 0xC, ped.addr() as u32);
    wr::<u32>(this, 0x10, ev);
    if !ped.is_null() {
        entity_register_reference(ped, this.byte_add(0xC));
    }
    this
}

/// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070, entry through a protection stub that pushes the
/// SEH handler): `vtbl = 0x85AE48`, clean up the ped reference (+0xC), delete the event at +0x10
/// (vtable +0, flag 1) when set, then `vtbl = 0x85AB38` (the inlined `CEvent` dtor).
#[hook(addr = 0x4AE070, conv = thiscall, name = "CEventGroupEvent::~CEventGroupEvent", expect = "6A FF E9 2D BB F5 FF")]
pub fn event_group_event_dtor_4ae070(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AE48);
    let p = rd::<GPtr<u8>>(this, 0xC);
    if !p.is_null() {
        entity_clean_up_old_reference(p, this.byte_add(0xC));
    }
    let e = rd::<GPtr<u8>>(this, 0x10);
    if !e.is_null() {
        ev_delete(e, 1);
    }
    wr::<u32>(this, 0, 0x85AB38);
}

// ---- batch 3 -------------------------------------------------------------------------------------

orig! {
    /// `CPedIntelligence::IsThreatenedBy(const CPed&)` (0x601C30)
    fn intel_is_threatened_by(intel: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x601C30 => unported;
    /// `CPedIntelligence::IsFriendlyWith(const CPed&)` (0x601BC0)
    fn intel_is_friendly_with(intel: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x601BC0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn ev_get_source_entity(ev: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CEvent::TakesPriorityOver(const CEvent&)` (vtable +0x2C)
    fn ev_takes_priority_over(ev: GPtr<u8>, other: GPtr<u8>) -> u8 = thiscall vslot 0x2C => unported;
}

/// `CEventSource::ComputeEventSourceType(const CEvent&, const CPed&)` (0x4ABAC0, cdecl; the body
/// continues through the straight protection stub 0x403C75 `edi = ped->m_pIntelligence`): no source
/// or not a ped → 0; threatened by it → 3; friendly → 2; a player → 1; else 0.
#[hook(addr = 0x4ABAC0, conv = cdecl, name = "CEventSource::ComputeEventSourceType", expect = "8B 4C 24 04 8B 01")]
pub fn event_source_compute_event_source_type_4abac0(ev: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let src = ev_get_source_entity(ev);
    if src.is_null() || rd::<u8>(src, 0x36) & 7 != 3 {
        return 0;
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    if intel_is_threatened_by(intel, src) != 0 {
        return 3;
    }
    if intel_is_friendly_with(intel, src) != 0 {
        return 2;
    }
    if ped_is_player(src) != 0 {
        return 1;
    }
    0
}

/// The event types `BaseEventTakesPriorityOverBaseEvent` ranks first (0x36, 0x37, 0x39, 0x47, 0x4C, 7, 0x4D).
fn is_top_group_event(t: i32) -> bool {
    matches!(t, 0x36 | 0x37 | 0x39 | 0x47 | 0x4C | 7 | 0x4D)
}

/// `CEventGroupEvent::BaseEventTakesPriorityOverBaseEvent(const CEventGroupEvent&)` (0x4AE100): both
/// base events' types (this one's first); this one in the top set → true; the other in it → false;
/// else (tail jump) `this->event->TakesPriorityOver(*other.event)`.
#[hook(addr = 0x4AE100, conv = thiscall, name = "CEventGroupEvent::BaseEventTakesPriorityOverBaseEvent", expect = "53 56 8B F1 8B 4E 10")]
pub fn event_group_event_base_takes_priority_4ae100(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    let mine = is_top_group_event(ev_get_event_type(rd::<GPtr<u8>>(this, 0x10)));
    let theirs = is_top_group_event(ev_get_event_type(rd::<GPtr<u8>>(other, 0x10)));
    if mine {
        return 1;
    }
    if theirs {
        return 0;
    }
    ev_takes_priority_over(rd::<GPtr<u8>>(this, 0x10), rd::<GPtr<u8>>(other, 0x10))
}

// ---- batch 4 -------------------------------------------------------------------------------------

orig! {
    /// `CTaskGangHasslePed::Constructor(CPed*, int, int, int)` (0x65FED0)
    fn gang_hassle_ped_ctor(this: GPtr<u8>, ped: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x65FED0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0)
    fn ik_is_looking(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::GetLookAtEntity(CPed*)` (0x6181D0)
    fn ik_get_look_at_entity(this: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x6181D0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int)` (0x681740)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn task_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// `CEventHandler::ComputeSexyPedResponse(CEvent*, CTask*, CTask*)` (0x4B99F0): with a source ped
/// (+0x14; none keeps `m_EventResponseTask`): response 200 → `+0x24 = null`; 0x4BC →
/// `+0x24 = new CTaskGangHasslePed(ped, 0, 10000, 30000)` (task pool; null when that fails).
#[hook(addr = 0x4B99F0, conv = thiscall, name = "CEventHandler::ComputeSexyPedResponse", expect = "64 A1 00 00 00 00")]
pub fn event_handler_compute_sexy_ped_response_4b99f0(this: GPtr<u8>, ev: GPtr<u8>, _a1: u32, _a2: u32) {
    let ped = rd::<GPtr<u8>>(ev, 0x14);
    if ped.is_null() {
        return;
    }
    match i32::from(rd::<i16>(ev, 0xE)) {
        0xC8 => wr::<u32>(this, 0x24, 0),
        0x4BC => {
            let t = task_new(0x2C);
            let r = if t.is_null() {
                GPtr::NULL
            } else {
                gang_hassle_ped_ctor(t, ped, 0, 10000, 30000)
            };
            wr::<u32>(this, 0x24, r.addr() as u32);
        }
        _ => {}
    }
}

/// `CInformFriendsEventQueue::Add(CPed*, CEvent*)` (0x4AC1E0): 8 × {ped, event, time} at 0xA9AFB8.
/// An entry for the same ped with an event of the same type (the entry's type is read first) →
/// false. Otherwise the first entry without an event (noted while scanning) takes `ped` (reference
/// registered), `event` and `time = now - ftol(f32(rand & 0xFFFF) * 2^-15 * -500) + 300` (rand site
/// 0x4AC23A); none free → false.
#[hook(addr = 0x4AC1E0, conv = cdecl, name = "CInformFriendsEventQueue::Add", expect = "51 53 55 8B 6C 24 10")]
pub fn inform_friends_event_queue_add_4ac1e0(ped: GPtr<u8>, ev: GPtr<u8>) -> u8 {
    let mut free = -1i32;
    for i in 0..8 {
        let e = g::<u8>(0xA9AFB8 + 0xC * i as usize);
        if rd::<u32>(e, 0) == ped.addr() as u32 {
            let mine = ev_get_event_type(rd::<GPtr<u8>>(e, 4));
            if ev_get_event_type(ev) == mine {
                return 0;
            }
        } else if free == -1 && rd::<u32>(e, 4) == 0 {
            free = i;
        }
    }
    if free == -1 {
        return 0;
    }
    let r = at_site(0x4AC23A, rand) & 0xFFFF;
    let e = g::<u8>(0xA9AFB8 + 0xC * free as usize);
    wr::<u32>(e, 0, ped.addr() as u32);
    let f = xi(r).fmul_m(f32::from_bits(0x3800_0000)).to_f32();
    if !ped.is_null() {
        entity_register_reference_ev(ped, e);
    }
    wr::<u32>(e, 4, ev.addr() as u32);
    let d = ftol32(x(f).fmul_m(-500.0));
    wr::<u32>(
        e,
        8,
        g::<u32>(0xB7CB84)
            .read()
            .wrapping_sub(d as u32)
            .wrapping_add(0x12C),
    );
    1
}

orig! {
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference_ev(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// The first of the ped's active tasks of types 0x4B2, 0x4BF, 0x4B3.
fn find_chat_task(ped: GPtr<u8>) -> GPtr<u8> {
    for t in [0x4B2, 0x4BF, 0x4B3] {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        let r = tm_find_active_task_by_type(intel.byte_add(4), t);
        if !r.is_null() {
            return r;
        }
    }
    GPtr::NULL
}

/// `vt_85b0b0::AffectsPed(CPed*)` (0x4AEF00): both the ped and the source (+0x14) alive, the ped not
/// already looking at the source (`g_ikChainMan`), and unless one of them has no 0x4B2/0x4BF/0x4B3
/// task, their task types differ (source's type read first).
#[hook(addr = 0x4AEF00, conv = thiscall, name = "vt_85b0b0::AffectsPed", expect = "53 56 57 8B 7C 24 10")]
pub fn vt_85b0b0_affects_ped_4aef00(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 {
        return 0;
    }
    let src = rd::<GPtr<u8>>(this, 0x14);
    if src.is_null() || ped_is_alive(src) == 0 {
        return 0;
    }
    let ik = g::<u8>(0xC15448);
    if ik_is_looking(ik, ped) != 0 && ik_get_look_at_entity(ik, ped) == src.addr() as u32 {
        return 0;
    }
    let mine = find_chat_task(ped);
    if mine.is_null() {
        return 1;
    }
    let theirs = find_chat_task(rd::<GPtr<u8>>(this, 0x14));
    if theirs.is_null() {
        return 1;
    }
    let tt = task_get_task_type(theirs);
    u8::from(task_get_task_type(mine) != tt)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_OPERATOR_NEW_4B5620,
    HOOK_EVENT_OPERATOR_DELETE_4B5630,
    HOOK_VT_85AE88_GET_SOURCE_ENTITY_4AE230,
    HOOK_EVENT_GROUP_EVENT_AFFECTS_PED_GROUP_4AE0E0,
    HOOK_EVENT_IN_AIR_CLONE_4B76A0,
    HOOK_EVENT_IS_VALID_4AC020,
    HOOK_EVENT_BUILDING_COLLISION_TAKES_PRIORITY_OVER_4AD050,
    HOOK_EVENT_TAKES_PRIORITY_OVER_4ABC30,
    HOOK_VT_85B070_AFFECTS_PED_4AEDC0,
    HOOK_VT_85B0F8_AFFECTS_PED_4AF100,
    HOOK_VT_85AE88_AFFECTS_PED_4AE2D0,
    HOOK_EVENT_ACQUAINTANCE_PED_SDD_420FB0,
    HOOK_EVENT_SOUND_QUIET_SDD_5DEAB0,
    HOOK_VT_86C6D0_SDD_5F6300,
    HOOK_EVENT_PED_COLLISION_WITH_PLAYER_SDD_5FEE20,
    HOOK_VT_86CA38_SDD_5FF0B0,
    HOOK_VT_86CA80_SDD_5FF170,
    HOOK_VT_86CAC8_SDD_5FF230,
    HOOK_EVENT_ACQUAINTANCE_PED_SDD_5FF420,
    HOOK_EVENT_LEAN_ON_VEHICLE_SDD_65DBF0,
    HOOK_EVENT_VEHICLE_DAMAGE_COLLISION_SDD_6A06D0,
    HOOK_EVENT_PED_COLLISION_WITH_PED_SDD_4B5980,
    HOOK_EVENT_OBJECT_COLLISION_SDD_4B59D0,
    HOOK_EVENT_BUILDING_COLLISION_SDD_4B5A20,
    HOOK_EVENT_GROUP_EVENT_SDD_4B5E50,
    HOOK_EVENT_POTENTIAL_WALK_INTO_VEHICLE_SDD_4B5EF0,
    HOOK_EVENT_POTENTIAL_WALK_INTO_PED_SDD_4B5F90,
    HOOK_VT_85B070_SDD_4B60D0,
    HOOK_VT_85B0B0_SDD_4B6120,
    HOOK_VT_85B0F8_SDD_4B6170,
    HOOK_EVENT_ATTRACTOR_SDD_4B6260,
    HOOK_EVENT_SCRIPT_COMMAND_SDD_4B6440,
    HOOK_EVENT_IN_AIR_SDD_4B64D0,
    HOOK_VT_85B0B0_CLONE_EDITABLE_4B7280,
    HOOK_VT_85B0F8_CLONE_4B72F0,
    HOOK_VT_86CA38_CLONE_EDITABLE_5FF050,
    HOOK_VT_86CA80_CLONE_EDITABLE_5FF110,
    HOOK_VT_86CAC8_CLONE_EDITABLE_5FF1D0,
    HOOK_EVENT_SEEN_COP_CLONE_EDITABLE_5FF3C0,
    HOOK_EVENT_GROUP_EVENT_CLONE_4B6EE0,
    HOOK_EVENT_POTENTIAL_WALK_INTO_VEHICLE_CLONE_EDITABLE_4B6FB0,
    HOOK_VT_85B070_CLONE_4B7210,
    HOOK_EVENT_LEAN_ON_VEHICLE_CLONE_65DB70,
    HOOK_EVENT_POTENTIAL_WALK_INTO_PED_CLONE_EDITABLE_4B7090,
    HOOK_VT_86C6D0_CLONE_5F6280,
    HOOK_EVENT_OBJECT_COLLISION_CLONE_4B6CC0,
    HOOK_EVENT_BUILDING_COLLISION_CLONE_4B6D40,
    HOOK_EVENT_PED_COLLISION_WITH_PED_CLONE_4B6C40,
    HOOK_EVENT_PED_COLLISION_WITH_PLAYER_CLONE_5FEDA0,
    HOOK_EVENT_LEAN_ON_VEHICLE_DTOR_65DC10,
    HOOK_EVENT_GROUP_EVENT_CTOR_4ADFD0,
    HOOK_EVENT_GROUP_EVENT_DTOR_4AE070,
    HOOK_EVENT_SOURCE_COMPUTE_EVENT_SOURCE_TYPE_4ABAC0,
    HOOK_EVENT_GROUP_EVENT_BASE_TAKES_PRIORITY_4AE100,
    HOOK_EVENT_HANDLER_COMPUTE_SEXY_PED_RESPONSE_4B99F0,
    HOOK_INFORM_FRIENDS_EVENT_QUEUE_ADD_4AC1E0,
    HOOK_VT_85B0B0_AFFECTS_PED_4AEF00,
];
