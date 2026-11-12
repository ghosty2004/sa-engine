//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `CWorld::RemoveReferencesToDeletedObject` and
//! `WaterCreatureManager_c::TryToExitGroup`. Ported from the 1.0 US bytes; gta-reversed as a reading
//! aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x};

orig! {
    /// `CPhysical::RemoveRefsToEntity(CEntity*)` (0x544280)
    fn physical_remove_refs_to_entity(p: GPtr<u8>, e: GPtr<u8>) = thiscall 0x544280 => unported;
    /// virtual `CVehicle::RemoveRefsToVehicle(CEntity*)` (vtable +0xA0)
    fn v_remove_refs_to_vehicle(v: GPtr<u8>, e: GPtr<u8>) = thiscall vslot 0xA0 => unported;
}

/// Every object of the pool at `cell` (slot size `size`), from the last slot down, except `skip`.
fn for_each_rev(cell: usize, size: isize, skip: GPtr<u8>, mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(cell).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let o = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * size);
        if o.is_null() || o == skip {
            continue;
        }
        f(o);
    }
}

/// `CWorld::RemoveReferencesToDeletedObject(CEntity*)` (0x565510): every other ped, vehicle and
/// object drops its references to `e` (`CPhysical::RemoveRefsToEntity`); peds also clear their
/// +0x568 pointer to it, vehicles also run their virtual `RemoveRefsToVehicle`.
#[hook(addr = 0x565510, conv = cdecl, name = "CWorld::RemoveReferencesToDeletedObject",
       expect = "53 8B 1D 90 44 B7 00",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "walks the three pools; the reference removals edit heap lists")]
pub fn remove_references_to_deleted_object(e: GPtr<u8>) {
    for_each_rev(0xB7_4490, 0x7C4, e, |p| {
        physical_remove_refs_to_entity(p, e);
        if rd::<GPtr<u8>>(p, 0x568) == e {
            wr::<u32>(p, 0x568, 0);
        }
    });
    for_each_rev(0xB7_4494, 0xA18, e, |v| {
        physical_remove_refs_to_entity(v, e);
        v_remove_refs_to_vehicle(v, e);
    });
    for_each_rev(0xB7_449C, 0x19C, e, |o| {
        physical_remove_refs_to_entity(o, e);
    });
}

/// `WaterCreatureManager_c::TryToExitGroup(WaterCreature_c*)` (0x6E45B0): the group of the creature
/// (its leader +0x1C, or itself) is collected from the manager's list (+0x1A0C, next +4); when every
/// member's entity (+8) is more than 60 units from the camera (squared distance above 3600), each
/// member's exit flag (+0xD) is set.
#[hook(addr = 0x6E45B0, conv = thiscall, name = "WaterCreatureManager_c::TryToExitGroup",
       expect = "8B 54 24 04 8B 42 1C",
       unverified = "world3-2", route = "L2", hazards = "none",
       note = "the original collects into a 32-entry stack array without a bound; the port stops at 32")]
pub fn water_creature_try_to_exit_group(this: GPtr<u8>, c: GPtr<u8>) {
    let l = rd::<GPtr<u8>>(c, 0x1C);
    let leader = if l.is_null() { c } else { l };
    let mut group = [GPtr::<u8>::from_addr(0); 32];
    let mut n = 0usize;
    let mut node = rd::<GPtr<u8>>(this, 0x1A0C);
    while !node.is_null() {
        let next = rd::<GPtr<u8>>(node, 4);
        if (node == leader || rd::<GPtr<u8>>(node, 0x1C) == leader) && n < group.len() {
            group[n] = node;
            n += 1;
        }
        node = next;
    }
    for &m in &group[..n] {
        let cm = g::<GPtr<u8>>(0xB6_F03C).read();
        let cp = if cm.is_null() { GPtr::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
        let ent = rd::<GPtr<u8>>(m, 8);
        let em = rd::<GPtr<u8>>(ent, 0x14);
        let p = if em.is_null() { ent.byte_add(4) } else { em.byte_add(0x30) };
        let a = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(cp, 0));
        let b = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(cp, 4));
        let z = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(cp, 8));
        let d2 = a.fmul(a).fadd(z.fmul(z)).fadd(b.fmul(b));
        // `test ah, 0x41; jnp`: exactly one of C0/C3 (below or equal, ordered) -> stay
        let st = d2.fcom(x(3600.0));
        if !st.test_parity(0x41) {
            return;
        }
    }
    for &m in &group[..n] {
        wr::<u8>(m, 0xD, 1);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_REMOVE_REFERENCES_TO_DELETED_OBJECT, HOOK_WATER_CREATURE_TRY_TO_EXIT_GROUP];
