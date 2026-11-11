//! `CDecision`, `CDecisionMaker`, `CDecisionMakerTypesFileLoader` (the non-file parts) and `CMentalState::Process`
//! (PORT-AI-5). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
//!
//! `CDecision` (0x3C bytes): +0 event-response task types int32[6], +0x18 probabilities uint8[6][4], +0x30
//! bool[6][2]. `CDecisionMaker` = 0x29 decisions (0x99C bytes). `CMentalState`: +0 / +1 anger (current, last),
//! +0x10 / +0x11 ped health byte (current, last), +0x12 / +0x13 vehicle health byte (current, last).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CDecision::SetDefault` (0x600530)
    fn decision_set_default(d: GPtr<u8>) = thiscall 0x600530 => unported;
    /// `CDecision::CDecision` (0x6040C0)
    fn decision_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x6040C0 => unported;
    /// `CDecisionMaker::CDecisionMaker` (0x4650A0)
    fn decision_maker_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x4650A0 => unported;
    /// `CDecisionMakerTypes::GetInstance` (0x4684F0, cdecl)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// `CDecisionMakerTypes::RemoveDecisionMaker(int32)` (0x6043A0, `ret 4`)
    fn decision_maker_types_remove(t: GPtr<u8>, i: i32) = thiscall 0x6043A0 => unported;
    /// `CDecisionMakerTypes::AddDecisionMaker(CDecisionMaker*, int32, bool)` (0x607050, `ret 0xC`)
    fn decision_maker_types_add(t: GPtr<u8>, dm: GPtr<u8>, ty: u32, b: u32) -> u32 = thiscall 0x607050 => unported;
    /// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, CDecisionMaker*)` (0x6076B0, cdecl)
    fn file_loader_load_decision_maker_ptr(name: u32, dm: GPtr<u8>) = cdecl 0x6076B0 => unported;
    /// `CEventEditableResponse::CEventEditableResponse` (0x4AC450)
    fn editable_response_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AC450 => unported;
    /// `CEventEditableResponse::~CEventEditableResponse` (0x4AC480)
    fn editable_response_dtor(e: GPtr<u8>) = thiscall 0x4AC480 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn eg_add(eg: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// `CDecision::SetDefault` (0x600530): types -1, probabilities 0, flags 0. The caller `CDecision::CDecision`
/// (0x6040C0) returns ECX after the call, so the hook keeps ECX.
#[hook(addr = 0x600530, conv = thiscall, name = "CDecision::SetDefault", expect = "33 C0 8D 51 30", preserve = "ecx")]
pub fn decision_set_default_port(this: GPtr<u8>) {
    for i in 0..6 {
        wr::<u32>(this, 4 * i, 0xFFFF_FFFF);
        wr::<u32>(this, 0x18 + 4 * i, 0);
        wr::<u16>(this, 0x30 + 2 * i, 0);
    }
}

/// `CDecision::CDecision` (0x6040C0, catalogued as `CDecision`): `SetDefault`, returns this.
#[hook(addr = 0x6040C0, conv = thiscall, name = "CDecision", expect = "E8 6B C4 FF FF")]
pub fn decision_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    decision_set_default(this);
    this
}

/// `CDecisionMaker::CDecisionMaker` (0x4650A0, `.HOODLUM` stub; catalogued as `CDecisionMaker`): constructs the 0x29
/// decisions, then sets every one to its default again. Returns this.
#[hook(addr = 0x4650A0, conv = thiscall, name = "CDecisionMaker", expect = "E9 DB F1 0F 01")]
pub fn decision_maker_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..0x29 {
        decision_ctor(this.byte_add(0x3C * i));
    }
    for i in 0..0x29 {
        decision_set_default(this.byte_add(0x3C * i));
    }
    this
}

/// `CDecision::Set(int32* types, float* probs, int32* bools, float*)` (0x600570, `ret 0x10`; the fourth argument is
/// unused): for each of the 6 responses the type, the four probabilities `_ftol`'d to bytes and the two flags (!= 0).
#[hook(addr = 0x600570, conv = thiscall, name = "CDecision::Set", expect = "51 53 55 8B 6C 24 18")]
pub fn decision_set(this: GPtr<u8>, types: GPtr<u8>, probs: GPtr<u8>, bools: GPtr<u8>, _facial: u32) {
    for i in 0..6isize {
        wr::<u32>(this, 4 * i, rd::<u32>(types, 4 * i));
        for k in 0..4isize {
            wr::<u8>(this, 0x18 + 4 * i + k, ftol32(x(rd::<f32>(probs, 0x10 * i + 4 * k))) as u8);
        }
        wr::<u8>(this, 0x30 + 2 * i, (rd::<u32>(bools, 8 * i) != 0) as u8);
        wr::<u8>(this, 0x31 + 2 * i, (rd::<u32>(bools, 8 * i + 4) != 0) as u8);
    }
}

/// `CDecisionMakerTypesFileLoader::UnloadDecisionMaker(int32)` (0x607A70, cdecl): `RemoveDecisionMaker` on the
/// instance (the argument pushed for `GetInstance` is consumed by the thiscall).
#[hook(addr = 0x607A70, conv = cdecl, name = "CDecisionMakerTypesFileLoader::UnloadDecisionMaker", expect = "8B 44 24 04 50")]
pub fn file_loader_unload_decision_maker(i: i32) {
    let t = decision_maker_types_get_instance();
    decision_maker_types_remove(t, i);
}

/// `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, int32 type, bool)` (0x607D30, cdecl; entry `nop;
/// jmp 0x40168C`, the island allocates the 0x99C-byte frame and jumps back): a `CDecisionMaker` on the stack (a Rust
/// local here), loaded from the file and added to the instance; returns `AddDecisionMaker`'s EAX.
#[hook(addr = 0x607D30, conv = cdecl, name = "CDecisionMakerTypesFileLoader::LoadDecisionMaker-enum", expect = "90 E9 56 99 DF FF")]
pub fn file_loader_load_decision_maker_enum(name: u32, ty: u32, b: u32) -> u32 {
    let mut dm = [0u32; 0x99C / 4];
    let p: GPtr<u8> = GPtr::from_raw(&mut dm as *mut [u32; 0x99C / 4]).cast();
    decision_maker_ctor(p);
    file_loader_load_decision_maker_ptr(name, p);
    let t = decision_maker_types_get_instance();
    decision_maker_types_add(t, p, ty, b)
}

/// Adds an editable-response event with vtable `vtbl`, built on the stack (a Rust local here), to the ped's event
/// group, then destroys it (vtable stored again before the destructor, as the original does).
fn add_event(ped: GPtr<u8>, vtbl: u32) {
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 8]).cast();
    editable_response_ctor(e);
    wr::<u32>(e, 0, vtbl);
    eg_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    wr::<u32>(e, 0, vtbl);
    editable_response_dtor(e);
}

/// `CMentalState::Process(const CPed&)` (0x6008A0, `ret 4`): records the ped's health (`_ftol` of +0x540, low byte)
/// and, in a vehicle, the vehicle's (+0x4C0). On foot, health falling below 50 / 10 raises the events with vtables
/// 0x86CBA0 / 0x86CBE8; in a vehicle the original compares the zero-extended health bytes with 600 and 300, which no
/// byte reaches, so nothing is raised (kept). Anger (+0) rising above 3 / 6 raises 0x86CC30 / 0x86CC78. Then the
/// current values become the last ones.
#[hook(addr = 0x6008A0, conv = thiscall, name = "CMentalState::Process", expect = "6A FF 64 A1 00 00 00 00")]
pub fn mental_state_process(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u8>(this, 0x10, ftol32(x(rd::<f32>(ped, 0x540))) as u8);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() {
            wr::<u8>(this, 0x12, ftol32(x(rd::<f32>(v, 0x4C0))) as u8);
        }
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        let last = rd::<u8>(this, 0x11);
        if last >= 0x32 {
            let cur = rd::<u8>(this, 0x10);
            if cur < 0xA {
                add_event(ped, 0x86CBE8);
            } else if cur < 0x32 {
                add_event(ped, 0x86CBA0);
            }
        } else if last >= 0xA && rd::<u8>(this, 0x10) < 0xA {
            add_event(ped, 0x86CBE8);
        }
    } else {
        let last = rd::<u8>(this, 0x13) as i32;
        if last >= 0x258 {
            let cur = rd::<u8>(this, 0x12) as i32;
            if cur < 0x12C {
                add_event(ped, 0x86CBE8);
            } else if cur < 0x258 {
                add_event(ped, 0x86CBA0);
            }
        } else if last >= 0x12C && (rd::<u8>(this, 0x12) as i32) < 0x12C {
            add_event(ped, 0x86CBE8);
        }
    }
    let last = rd::<u8>(this, 1);
    if last <= 3 {
        let cur = rd::<u8>(this, 0);
        if cur > 6 {
            add_event(ped, 0x86CC78);
        } else if cur > 3 {
            add_event(ped, 0x86CC30);
        }
    } else if last <= 6 && rd::<u8>(this, 0) > 6 {
        add_event(ped, 0x86CC78);
    }
    let a = rd::<u8>(this, 0);
    let h = rd::<u8>(this, 0x10);
    let vh = rd::<u8>(this, 0x12);
    wr::<u8>(this, 1, a);
    wr::<u8>(this, 0x11, h);
    wr::<u8>(this, 0x13, vh);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DECISION_SET_DEFAULT_PORT,
    HOOK_DECISION_CTOR_PORT,
    HOOK_DECISION_MAKER_CTOR_PORT,
    HOOK_DECISION_SET,
    HOOK_FILE_LOADER_UNLOAD_DECISION_MAKER,
    HOOK_FILE_LOADER_LOAD_DECISION_MAKER_ENUM,
    HOOK_MENTAL_STATE_PROCESS,
];
