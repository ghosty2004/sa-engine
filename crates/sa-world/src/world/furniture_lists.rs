//! WRITER-WORLD-2 (batch `world6-3`): furniture manager setup, the random furniture id of a sub
//! group, and the water creature teardown. Ported from the 1.0 US bytes; gta-reversed as a reading
//! aid (`FurnitureSubGroup_c::GetRandomId`'s first branch is dead in the bytes: a zero-extended byte is
//! compared with -1).

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, xi};

use sa_core::bindings::list_add_item;
use sa_core::bindings::list_remove_item;
use sa_core::bindings::world_remove;
orig! {
    /// `FurnitureGroup_c::Init()` (0x5910A0)
    fn furniture_group_init(grp: GPtr<u8>) -> i32 = thiscall 0x5910A0 => unported;
    /// `FurnitureManager_c::LoadFurniture()` (0x5C0280)
    fn load_furniture(man: GPtr<u8>) = thiscall 0x5C0280 => unported;
    /// the deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `FurnitureManager_c::Init()` (0x5C04A0): clears the two counters 0xBAB37C / 0xBAB378, puts the
/// 512 furniture items (+0x6C, 0x10 each) on the free list (+0x206C), initialises the 9 groups (0xC
/// each) and loads the furniture definitions. Returns 1.
#[hook(addr = 0x5C04A0, conv = thiscall, name = "FurnitureManager_c::Init", this = "FurnitureManager_c",
       expect = "53 55 56 8B E9",
       unverified = "world6-3", route = "split", hazards = "heap-writes",
       note = "LoadFurniture reads furnitur.dat (file I/O): one-time initialiser")]
pub fn furniture_manager_init(this: GPtr<u8>) -> i32 {
    g::<u16>(0xBA_B37C).write(0);
    g::<u16>(0xBA_B378).write(0);
    for k in 0..0x200 {
        list_add_item(this.byte_add(0x206C), this.byte_add(0x6C + 0x10 * k));
    }
    for k in 0..9 {
        furniture_group_init(this.byte_add(0xC * k));
    }
    load_furniture(this);
    1
}

/// `FurnitureSubGroup_c::GetRandomId(uint8 rating)` (0x590FD0): among the furniture of the sub
/// group (+0xC list, next at +4) whose rating range (+0xE ..= +0xF) contains `rating`, the one at
/// `ftol(rand() * count / 32768)`; its id (+0xA, int16), or -1 when none matches.
#[hook(addr = 0x590FD0, conv = thiscall, name = "FurnitureSubGroup_c::GetRandomId", this = "FurnitureSubGroup_c",
       expect = "51 53 8A 5C 24 0C",
       unverified = "world6-3", route = "L2", hazards = "none",
       note = "rand at_site 0x591050")]
pub fn furniture_sub_group_get_random_id(this: GPtr<u8>, rating: u8) -> i32 {
    let fits = |f: GPtr<u8>| rating >= rd::<u8>(f, 0xE) && rating <= rd::<u8>(f, 0xF);
    let mut count = 0;
    let mut f = rd::<GPtr<u8>>(this, 0xC);
    while !f.is_null() {
        if fits(f) {
            count += 1;
        }
        f = rd::<GPtr<u8>>(f, 4);
    }
    let r = at_site(0x59_1050, rand) & 0xFFFF;
    let pick = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul(xi(count)));
    let mut i = 0;
    let mut f = rd::<GPtr<u8>>(this, 0xC);
    while !f.is_null() {
        if fits(f) {
            if i == pick {
                return i32::from(rd::<i16>(f, 0xA));
            }
            i += 1;
        }
        f = rd::<GPtr<u8>>(f, 4);
    }
    -1
}

/// `WaterCreature_c::Exit()` (0x6E3E60): moves the creature from the active list (0xC1F93C) to the
/// free list (0xC1F930), removes and deletes its object (+8) and decrements the object counter
/// 0xBB4A70.
#[hook(addr = 0x6E3E60, conv = thiscall, name = "WaterCreature_c::Exit", this = "WaterCreature_c",
       expect = "56 8B F1 56 B9 3C F9 C1 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn water_creature_exit(this: GPtr<u8>) {
    list_remove_item(GPtr::from_addr(0xC1_F93C), this);
    list_add_item(GPtr::from_addr(0xC1_F930), this);
    world_remove(rd::<GPtr<u8>>(this, 8));
    let o = rd::<GPtr<u8>>(this, 8);
    if !o.is_null() {
        v_delete(o, 1);
    }
    wr::<u32>(this, 8, 0);
    let c = g::<u16>(0xBB_4A70);
    c.write(c.read().wrapping_sub(1));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FURNITURE_MANAGER_INIT, HOOK_FURNITURE_SUB_GROUP_GET_RANDOM_ID, HOOK_WATER_CREATURE_EXIT];
