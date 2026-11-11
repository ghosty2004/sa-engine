//! WRITER-MVP-B (helperB) AI ports, part 4: `CPedGroupIntelligence::ProcessIgnorePlayerGroup`. Ported from the
//! 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230, cdecl)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CEvent::GetEventType` (vtable +0x4)
    fn v_get_event_type(e: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_get_source_entity(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

fn delete_slot(this: GPtr<u8>, off: isize) {
    let t = rd::<GPtr<u8>>(this, off);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, off, 0);
}

/// `CPedGroupIntelligence::ProcessIgnorePlayerGroup` (0x5F87A0): only while the player's wanted flags (+0x1E bit 4)
/// say "ignored by the player's group": unless the current event (+4) wraps an event of type 0x4D or 0x36, when the
/// group's leader is the player, or the event's source is a ped whose group is led (+0x28) by the player, the event
/// is deleted (+4 = 0), the secondary event (+0x294) deleted and cleared, and the 8 main (+0x10) and 8 secondary
/// (+0xB0) task slots (0x14 stride) deleted and cleared.
#[hook(addr = 0x5F87A0, conv = thiscall, name = "CPedGroupIntelligence::ProcessIgnorePlayerGroup", this = "CPedGroupIntelligence",
       expect = "56 6A FF 8B F1", route = "L2", hazards = "heap-writes")]
pub fn pgi_process_ignore_player_group(this: GPtr<u8>) {
    if rd::<u8>(find_player_wanted(-1), 0x1E) & 4 == 0 {
        return;
    }
    let membership = rd::<GPtr<u8>>(this, 0).byte_add(8);
    for ty in [0x4D, 0x36] {
        let ev = rd::<GPtr<u8>>(this, 4);
        if !ev.is_null() && v_get_event_type(rd::<GPtr<u8>>(ev, 0x10)) == ty {
            return;
        }
    }
    let leader = rd::<u32>(membership, 0x20);
    if find_player_ped(-1).addr() as u32 != leader {
        let ev = rd::<GPtr<u8>>(this, 4);
        if ev.is_null() {
            return;
        }
        let src = v_get_source_entity(rd::<GPtr<u8>>(ev, 0x10));
        if src.is_null() || rd::<u8>(src, 0x36) & 7 != 3 {
            return;
        }
        let g = get_peds_group(src);
        if g.is_null() {
            return;
        }
        let l = rd::<u32>(g, 0x28);
        if l != find_player_ped(-1).addr() as u32 {
            return;
        }
    }
    let ev = rd::<GPtr<u8>>(this, 4);
    if ev.is_null() {
        return;
    }
    v_delete(ev, 1);
    let e2 = rd::<GPtr<u8>>(this, 0x294);
    wr::<u32>(this, 4, 0);
    if !e2.is_null() {
        v_delete(e2, 1);
    }
    wr::<u32>(this, 0x294, 0);
    for i in 0..8isize {
        delete_slot(this, 0x10 + 0x14 * i);
    }
    for i in 0..8isize {
        delete_slot(this, 0xB0 + 0x14 * i);
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PGI_PROCESS_IGNORE_PLAYER_GROUP];
