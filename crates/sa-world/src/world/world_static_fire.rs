//! WRITER-WORLD-2 (batch `world6-4`): `CWorld::RemoveStaticObjects` and `CWorld::SetCarsOnFire`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{rd, x};

orig! {
    /// `CEntity::Remove()` (vtable +0xC)
    fn v_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// the deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// the single-link pointer list flush (0x552400)
    fn flush_single(l: GPtr<u8>) = thiscall 0x552400 => unported;
    /// the double-link pointer list flush (0x552470)
    fn flush_double(l: GPtr<u8>) = thiscall 0x552470 => unported;
    /// `CFireManager::StartFire(CEntity* target, CEntity* creator, float size, uint8, uint32 time, int8 gens)` (0x53A050) on `gFireManager` (0xB71F80)
    fn start_fire_on(fm: GPtr<u8>, target: GPtr<u8>, creator: GPtr<u8>, size: f32, a: u32, time: u32, gens: u32) -> GPtr<u8> = thiscall 0x53A050 => unported;
}

/// `|v - c| < limit` the way the bytes test it: the difference negated when below 0, compared
/// unrounded (NaN fails).
fn within(v: f32, c: f32, limit: f32) -> bool {
    let d = x(v).fsub_m(c);
    let a = if !d.fcom(x(0.0)).test_parity(0x05) { d.fchs() } else { d };
    !a.fcom(x(limit)).test_parity(0x05)
}

/// `CWorld::SetCarsOnFire(float x, float y, float z, float radius, CEntity* creator)` (0x5659F0):
/// every vehicle of the pool (highest slot first) that is not wrecked, not burning (+0x490) and not
/// fire proof (+0x40 bit 19), within 5 vertically and `radius` in x and y of the point, is set on
/// fire (size 0.8, 7000 ms, 100 generations) by `creator`.
#[hook(addr = 0x5659F0, conv = cdecl, name = "CWorld::SetCarsOnFire",
       expect = "53 8B 1D 94 44 B7 00",
       unverified = "world6-4", route = "L2", hazards = "heap-writes")]
pub fn set_cars_on_fire(px: f32, py: f32, pz: f32, radius: f32, creator: GPtr<u8>) {
    let pool = GPtr::<GPtr<u8>>::from_addr(0xB7_4494).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let veh = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if veh.is_null()
            || rd::<u8>(veh, 0x36) & 0xF8 == 0x28
            || !rd::<GPtr<u8>>(veh, 0x490).is_null()
            || rd::<u32>(veh, 0x40) & 0x8_0000 != 0
        {
            continue;
        }
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
        if within(rd::<f32>(pos, 8), pz, 5.0) && within(rd::<f32>(pos, 0), px, radius) && within(rd::<f32>(pos, 4), py, radius) {
            start_fire_on(GPtr::from_addr(0xB7_1F80), veh, creator, f32::from_bits(0x3F4C_CCCD), 1, 0x1B58, 0x64);
        }
    }
}

/// Removes and deletes every entity of a pointer list (the next link read before the entity's
/// `Remove`, the node's item re-read for the delete, as the bytes do).
fn remove_all(list: GPtr<u8>) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let next = rd::<GPtr<u8>>(node, 4);
        v_remove(e);
        let t = rd::<u8>(e, 0x36) & 7;
        if t > 1 && t < 5 {
            remove_from_moving_list(e);
        }
        let again = rd::<GPtr<u8>>(node, 0);
        if !again.is_null() {
            v_delete(again, 1);
        }
        node = next;
    }
}

/// `CWorld::RemoveStaticObjects()` (0x563840): every building and dummy of the 120 x 120 sectors
/// (0xB7D0B8, 8 bytes each; both lists then flushed) and every object of the 16 x 16 repeat
/// sectors (third list, 0xB992C0 + 12k) is removed from the world and deleted.
#[hook(addr = 0x563840, conv = cdecl, name = "CWorld::RemoveStaticObjects",
       expect = "51 53 55 56 BE B8 D0 B7 00",
       unverified = "world6-4", route = "split", hazards = "heap-writes",
       note = "deletes every static entity (level teardown)")]
pub fn remove_static_objects() {
    let mut s = 0xB7_D0B8usize;
    while s < 0xB9_92B8 {
        remove_all(GPtr::from_addr(s));
        remove_all(GPtr::from_addr(s + 4));
        flush_single(GPtr::from_addr(s));
        flush_double(GPtr::from_addr(s + 4));
        s += 8;
    }
    let mut r = 0xB9_92C0usize;
    while r < 0xB9_9EC0 {
        remove_all(GPtr::from_addr(r));
        r += 0xC;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_REMOVE_STATIC_OBJECTS, HOOK_SET_CARS_ON_FIRE];
