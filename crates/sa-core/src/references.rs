//! CORE ports: `CReferences` (see `docs/arch/core-port.md`).
//!
//! `CReference` = { next (+0), `CEntity**` (+4) }, 3000 of them in `aRefs` (0xB9B9A8), free list head
//! `pEmptyList` (0xBA1768).

use crate::mem::GPtr;
use crate::pools;
use crate::{globals, hook, orig};

/// `MAX_NUM_REFERENCES`
pub const MAX_REFS: u32 = 3000;

globals! {
    list GLOBALS;
    /// `CReferences::aRefs` (3000 × 8 bytes)
    pub static REFS @ 0xB9B9A8: [u32; 6000] = [0; 6000];
    /// `CReferences::pEmptyList`
    pub static EMPTY_LIST @ 0xBA1768: u32 = 0;
}

pub mod call {
    use super::*;
    orig! {
        /// `CWorld::FindPlayerVehicle(int32 playerId, bool includeRemote)` (cdecl)
        pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
        /// `CWorld::FindPlayerPed(int32 playerId)` (cdecl)
        pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
        /// `CEntity::ResolveReferences()` (thiscall)
        pub fn resolve_references(e: GPtr<u8>) = thiscall 0x571A40 => unported;
        /// `CEntity::PruneReferences()` (thiscall)
        pub fn prune_references(e: GPtr<u8>) = thiscall 0x571A90 => unported;
        /// `CPedGroupMembership::SetLeader(CPed*)` (thiscall)
        pub fn set_leader(m: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FB9C0 => unported;
        /// `CPedGroup::Process()` (thiscall)
        pub fn ped_group_process(g: GPtr<u8>) = thiscall 0x5FC7E0 => unported;
    }
}

fn at<U>(p: GPtr<u8>, off: isize) -> GPtr<U> {
    p.byte_add(off).cast()
}

/// `CReferences::Init` (0x5719B0): `pEmptyList = &aRefs[0]`; every entry (all 3000, including the
/// last, which first points one past the array, i.e. at `pEmptyList`) gets `next = &entry[i + 1]`,
/// `pEntity = 0`; then `aRefs[2999].next = 0` (0xBA1760).
#[hook(addr = 0x5719B0, conv = cdecl, name = "CReferences::Init", expect = "B8 A8 B9 B9 00")]
pub fn init() {
    let base = REFS.ptr().cast::<u8>();
    EMPTY_LIST.set(REFS.ptr().addr() as u32);
    for i in 0..MAX_REFS as isize {
        let e = base.byte_add(8 * i);
        at::<u32>(e, 0).write((REFS.ptr().addr() as u32).wrapping_add(8 * (i as u32 + 1)));
        at::<u32>(e, 4).write(0);
    }
    at::<u32>(base, 8 * (MAX_REFS as isize - 1)).write(0);
}

/// `CReferences::ListSize(CReference*)` (0x5719E0, cdecl): the number of `next` links followed until
/// null.
#[hook(addr = 0x5719E0, conv = cdecl, name = "CReferences::ListSize", expect = "8B 4C 24 04 33 C0")]
pub fn list_size(list: GPtr<u8>) -> u32 {
    count_links(list, |p| p.is_null(), |p| at::<GPtr<u8>>(p, 0).read())
}

/// `ListSize`'s loop over any pointer representation (the L1 spec drives it with scratch memory).
pub fn count_links<P: Copy>(list: P, is_null: impl Fn(P) -> bool, next: impl Fn(P) -> P) -> u32 {
    let mut n = 0u32;
    let mut p = list;
    while !is_null(p) {
        p = next(p);
        n = n.wrapping_add(1);
    }
    n
}

/// `CReferences::RemoveReferencesToPlayer` (0x571AD0; the tail lives in a protection chunk at
/// 0x403768). Player vehicle (`FindPlayerVehicle(-1, false)`, called again for the `this`) and player
/// ped get `ResolveReferences`; then the player's group (`ped->m_pPlayerData (+0x480)
/// ->m_nPlayerGroup (+0x38)`, 0x2D4-byte `CPedGroup`s at 0xC09920): `membership (+8).SetLeader(
/// FindPlayerPed(-1))` and `CPedGroup::Process` (tail jump), each re-reading the player ped.
#[hook(addr = 0x571AD0, conv = cdecl, name = "CReferences::RemoveReferencesToPlayer", expect = "6A 00 6A FF E8 F7 C5 FF FF")]
pub fn remove_references_to_player() {
    if !call::find_player_vehicle(-1, 0).is_null() {
        call::resolve_references(call::find_player_vehicle(-1, 0));
    }
    if call::find_player_ped(-1).is_null() {
        return;
    }
    call::resolve_references(call::find_player_ped(-1));
    let group = |ped: GPtr<u8>| {
        let data = at::<GPtr<u8>>(ped, 0x480).read();
        at::<u32>(data, 0x38).read().wrapping_mul(0x2D4)
    };
    let g = group(call::find_player_ped(-1));
    let membership = GPtr::<u8>::from_addr(g.wrapping_add(0xC09928) as usize);
    call::set_leader(membership, call::find_player_ped(-1));
    let g = group(call::find_player_ped(-1));
    call::ped_group_process(GPtr::from_addr(g.wrapping_add(0xC09920) as usize));
}

/// `CReferences::PruneAllReferencesInWorld` (0x571CC0): `CEntity::PruneReferences` on every used,
/// non-null slot of the ped, vehicle and object pools, each walked from the last slot down
/// (gta-reversed walks upwards).
#[hook(addr = 0x571CC0, conv = cdecl, name = "CReferences::PruneAllReferencesInWorld", expect = "53 56 57 8B 3D 90 44 B7 00")]
pub fn prune_all_references_in_world() {
    pools::for_used_slots_backwards(crate::pool::PED_POOL.get(), call::prune_references);
    pools::for_used_slots_backwards(crate::pool::VEHICLE_POOL.get(), call::prune_references);
    pools::for_used_slots_backwards(crate::pool::OBJECT_POOL.get(), call::prune_references);
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_INIT,
    HOOK_LIST_SIZE,
    HOOK_REMOVE_REFERENCES_TO_PLAYER,
    HOOK_PRUNE_ALL_REFERENCES_IN_WORLD,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn init_links_the_free_list() {
        init();
        let r = REFS.get();
        assert_eq!(EMPTY_LIST.get(), (REFS.ptr().addr() as u32));
        assert_eq!(r[0], (REFS.ptr().addr() as u32) + 8);
        assert_eq!(r[1], 0);
        assert_eq!(r[2 * 2998], (REFS.ptr().addr() as u32) + 8 * 2999);
        assert_eq!(r[2 * 2999], 0);
    }

    #[test]
    fn list_size_counts_links() {
        assert_eq!(list_size(GPtr::NULL), 0);
        let mut c: [usize; 1] = [0];
        let mut b: [usize; 1] = [c.as_mut_ptr() as usize];
        let mut a: [usize; 1] = [b.as_mut_ptr() as usize];
        assert_eq!(list_size(GPtr::from_raw(a.as_mut_ptr()).cast()), 3);
        let _ = (&mut b, &mut c);
    }
}
