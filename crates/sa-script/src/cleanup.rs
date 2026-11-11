//! `CMissionCleanup` list maintenance (PORT-SCRIPT-3): `AddEntityToList`, `RemoveEntityFromList`.
//! Follows the 1.0 US bytes (`.HOODLUM` bodies behind the stubs) and `docs/re/specs/SCRIPT/CMissionCleanup.md`.
//!
//! Layout (gr `MissionCleanup.h`, size 0x25C): 75 entries `{ u8 type +0, i32 handle +4 }` (stride 8), `i8
//! m_Count` at +0x258. Entry types: 1 vehicle, 2 ped, 3 object (others are not looked up here).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Number of entries.
const ENTRIES: u16 = 0x4B;
/// `m_Count` (i8).
const COUNT: isize = 0x258;

orig! {
    /// `CEntity::GetIsStatic()` (0x4633E0), AL
    fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
}

/// `CMissionCleanup::AddEntityToList(int32 handle, MissionCleanUpEntityType type)` (stub 0x4637E0 → 0x1565D40,
/// `ret 8`): the first free entry (type 0, u16 counter below 75) gets `handle` (+4), then the low byte of
/// `type` (+0); `m_Count` + 1 (byte, wraps). A full list changes nothing. Duplicates are not checked. The
/// body never writes ECX (`preserve = "ecx"`).
#[hook(addr = 0x4637E0, conv = thiscall, name = "CMissionCleanup::AddEntityToList", expect = "E9 5B 25 10 01", this = "CMissionCleanup", preserve = "ecx")]
pub fn add_entity_to_list(m: GPtr<u8>, handle: i32, ty: u32) {
    let mut i: u16 = 0;
    while i < ENTRIES {
        let e = m.byte_add(8 * i as isize);
        if e.read() == 0 {
            e.byte_add(4).cast::<i32>().write(handle);
            e.write(ty as u8);
            let c = m.byte_add(COUNT);
            c.write(c.read().wrapping_add(1));
            return;
        }
        i += 1;
    }
}

/// `CMissionCleanup::RemoveEntityFromList(int32 handle, MissionCleanUpEntityType type)` (stub 0x4654B0 →
/// 0x156E810, `ret 8`): every one of the 75 entries (no early exit) whose type byte equals the low byte of
/// `type` and whose handle equals `handle`: types 1/2/3 look the handle up in the vehicle/ped/object pool
/// (`idx = handle >> 8`, valid when `flags[idx] == (u8)handle`, `objects + idx * size`); a found entity with
/// `m_nFlags` (+0x1C) bit 0x40000 gets the bit cleared and, when `!GetIsStatic()`, `AddToMovingList()`.
/// Then the entry is cleared (handle, then type) and `m_Count` - 1 (byte). With type 0 every empty slot
/// whose handle matches is "removed" again (original behaviour).
#[hook(addr = 0x4654B0, conv = thiscall, name = "CMissionCleanup::RemoveEntityFromList", expect = "E9 5B 93 10 01", this = "CMissionCleanup")]
pub fn remove_entity_from_list(m: GPtr<u8>, handle: i32, ty: u32) {
    for i in 0..ENTRIES as isize {
        let e = m.byte_add(8 * i);
        let t = e.read();
        if t != ty as u8 || e.byte_add(4).cast::<i32>().read() != handle {
            continue;
        }
        let pool = match t {
            1 => Some((0xB74494usize, 0xA18i32)),
            2 => Some((0xB74490, 0x7C4)),
            3 => Some((0xB7449C, 0x19C)),
            _ => None,
        };
        if let Some((cell, size)) = pool {
            let p = GPtr::<GPtr<u8>>::from_addr(cell).read();
            let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
            let idx = handle >> 8;
            if flags.byte_add(idx as isize).read() == handle as u8 {
                let objects = p.cast::<GPtr<u8>>().read();
                let ent = objects.byte_add(idx.wrapping_mul(size) as isize);
                if !ent.is_null() {
                    let fl = ent.byte_add(0x1C).cast::<u32>();
                    let v = fl.read();
                    if v & 0x40000 != 0 {
                        fl.write(v & 0xFFFB_FFFF);
                        if entity_get_is_static(ent) == 0 {
                            physical_add_to_moving_list(ent);
                        }
                    }
                }
            }
        }
        e.byte_add(4).cast::<i32>().write(0);
        e.write(0);
        let c = m.byte_add(COUNT);
        c.write(c.read().wrapping_sub(1));
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_ADD_ENTITY_TO_LIST, HOOK_REMOVE_ENTITY_FROM_LIST];
