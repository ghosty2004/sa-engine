//! `CProjectileInfo` (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `ProjectileInfo.cpp`, `docs/re/specs/WEAPON/CProjectileInfo.md`).
//!
//! `gaProjectileInfo[32]` (0x24) at 0xC891A8: +0 weapon type, +4 creator, +8 destroy time, +0xC ...,
//! +0x10 active, +0x14 last position, +0x20 `FxSystem_c*`. `CProjectileInfo::ms_apProjectile[32]`
//! (`CProjectile*`) at 0xC89110. Projectile objects live in the object pool (0x19C each).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const INFO_SIZE: usize = 0x24;
pub const NUM_PROJECTILES: usize = 32;
const _: () = assert!(INFO_SIZE == sa_core::layouts::size::CProjectileInfo);

/// `g_fxMan`
const FX_MAN: usize = 0xA9AE80;

sa_core::globals! {
    list GLOBALS;
    /// `CProjectileInfo::ms_apProjectile[32]`
    pub static PROJECTILES @ 0xC89110: [GPtr<u8>; NUM_PROJECTILES] = [GPtr::NULL; NUM_PROJECTILES];
    /// `gaProjectileInfo[32]`
    pub static INFOS @ 0xC891A8: [u8; NUM_PROJECTILES * INFO_SIZE] = [0; NUM_PROJECTILES * INFO_SIZE];
}

orig! {
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810, thiscall)
    pub fn destroy_fx_system(man: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    pub fn clear_blip_for_entity(ty: i32, handle: i32) = cdecl 0x587C60 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CEntity` scalar deleting destructor (vtable slot 0)
    pub fn entity_vdelete(e: GPtr<u8>, flag: i32) = thiscall vslot 0x0 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CEntity* creator, eExplosionType, CVector pos,
    /// uint32 lifetime, bool makeSound, float camShake, bool invisible)` (0x736A50)
    /// `CPool<CObject>::GetRef(CObject*)` (thunk 0x465070, thiscall)
    pub fn object_pool_get_ref(pool: GPtr<u8>, obj: GPtr<u8>) -> i32 = thiscall 0x465070 => unported;
    /// `CProjectile` destructor (0x5A40E0, thiscall)
    pub fn projectile_dtor(this: GPtr<u8>) = thiscall 0x5A40E0 => unported;
    /// `CObject::operator delete(void*)` (0x5A1F20)
    pub fn object_delete(p: GPtr<u8>) = cdecl 0x5A1F20 => unported;
    pub fn add_explosion(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, x: f32, y: f32, z: f32, life: u32, sound: u8, shake: f32, invisible: u8) = cdecl 0x736A50 => unported;
}

pub fn info(i: usize) -> GPtr<u8> {
    INFOS.ptr().cast::<u8>().byte_add((i * INFO_SIZE) as isize)
}

fn projectile_slot(i: usize) -> GPtr<GPtr<u8>> {
    let i = i as i32;
    PROJECTILES.ptr().cast::<GPtr<u8>>().wrapping_index(i)
}

/// `CProjectileInfo::Initialise()` (0x737B40): `ms_apProjectile` cleared; every info: type 16,
/// creator 0, time 0, inactive, fx null.
#[hook(addr = 0x737B40, conv = cdecl, name = "CProjectileInfo::Initialise", expect = "57 B9 20 00 00 00")]
pub fn initialise() {
    PROJECTILES.set([GPtr::NULL; NUM_PROJECTILES]);
    for i in 0..NUM_PROJECTILES {
        let p = info(i);
        wu(p, 0, 0x10);
        wu(p, 4, 0);
        wu(p, 0xC, 0);
        wb(p, 0x10, 0);
        wu(p, 0x20, 0);
    }
}

/// `CProjectileInfo::GetProjectileInfo(int32 i)` (0x737BF0).
#[hook(addr = 0x737BF0, conv = cdecl, name = "CProjectileInfo::GetProjectileInfo", expect = "8B 44 24 04 8D 04 C0")]
pub fn get_projectile_info(i: i32) -> GPtr<u8> {
    INFOS.ptr().cast::<u8>().byte_add((i as isize).wrapping_mul(INFO_SIZE as isize))
}

/// `CProjectileInfo::RemoveFXSystem(bool destroy)` (0x737B80, thiscall `ret 4`): destroy (or kill)
/// the fx and clear the field.
#[hook(addr = 0x737B80, conv = thiscall, name = "CProjectileInfo::RemoveFXSystem", expect = "56 8B F1 8B 4E 20", this = "CProjectileInfo")]
pub fn remove_fx_system(this: GPtr<u8>, destroy: u8) {
    let fx = rp(this, 0x20);
    if fx.is_null() {
        return;
    }
    if destroy != 0 {
        destroy_fx_system(g(FX_MAN), fx);
    } else {
        fx_kill(fx);
    }
    wu(this, 0x20, 0);
}

/// `CProjectileInfo::Shutdown()` (0x737BC0): destroys every fx.
#[hook(addr = 0x737BC0, conv = cdecl, name = "CProjectileInfo::Shutdown", expect = "56 BE C8 91 C8 00")]
pub fn shutdown() {
    for i in 0..NUM_PROJECTILES {
        let p = info(i);
        let fx = rp(p, 0x20);
        if !fx.is_null() {
            destroy_fx_system(g(FX_MAN), fx);
            wu(p, 0x20, 0);
        }
    }
}

/// The object pool handle of `e` (`CPools::GetObjectRef` inlined: `(index << 8) + flag byte`; index
/// by the magic division of the byte offset by 0x19C).
pub fn object_ref(e: GPtr<u8>) -> i32 {
    let pool = g::<GPtr<u8>>(0xB7449C).read();
    let d = (e.addr() as i32).wrapping_sub(rp(pool, 0).addr() as i32);
    let q = ((i64::from(d) * 0x13E2_2CBD) >> 32) as i32 >> 5;
    let idx = q + ((q as u32) >> 31) as i32;
    let flag = i32::from(rp(pool, 4).byte_add(idx as isize).read());
    flag + (idx << 8)
}

/// `CProjectileInfo::RemoveAllProjectiles()` (0x7399B0; split chunk 0x40391D): every active info is
/// deactivated, its fx destroyed, the projectile's blip cleared, removed from the world and deleted.
#[hook(addr = 0x7399B0, conv = cdecl, name = "CProjectileInfo::RemoveAllProjectiles", expect = "53 55 56 57 E9 64 9F CC FF")]
pub fn remove_all_projectiles() {
    for i in 0..NUM_PROJECTILES {
        let p = info(i);
        let e = projectile_slot(i).read();
        if rb(p, 0x10) == 0 {
            continue;
        }
        let fx = rp(p, 0x20);
        wb(p, 0x10, 0);
        if !fx.is_null() {
            destroy_fx_system(g(FX_MAN), fx);
            wu(p, 0x20, 0);
        }
        clear_blip_for_entity(3, object_ref(e));
        world_remove(e);
        if !e.is_null() {
            entity_vdelete(e, 1);
        }
    }
}

/// `CProjectileInfo::RemoveNotAdd(CEntity* creator, eWeaponType, CVector pos)` (0x737C00): an
/// explosion instead of a projectile: grenade (16) / teargas... type 39 → explosion 0, molotov (18)
/// → 1, rockets (19, 20) → 2 (lifetime 0, sound, shake -1, visible); other types nothing.
#[hook(addr = 0x737C00, conv = cdecl, name = "CProjectileInfo::RemoveNotAdd", expect = "8B 44 24 08 83 F8 10")]
pub fn remove_not_add(creator: GPtr<u8>, ty: i32, px: f32, py: f32, pz: f32) {
    let ex = match ty {
        0x10 | 0x27 => 0,
        0x12 => 1,
        0x13 | 0x14 => 2,
        _ => return,
    };
    add_explosion(GPtr::NULL, creator, ex, px, py, pz, 0, 1, -1.0, 0);
}

/// `CProjectileInfo::RemoveIfThisIsAProjectile(CObject*)` (0x739A40): the active info whose
/// projectile is `obj` is deactivated (fx killed), the object's blip cleared, removed, deleted, and the
/// slot cleared; true if found.
#[hook(addr = 0x739A40, conv = cdecl, name = "CProjectileInfo::RemoveIfThisIsAProjectile", expect = "8B 4C 24 04 56")]
pub fn remove_if_this_is_a_projectile(obj: GPtr<u8>) -> u8 {
    let Some(i) = (0..NUM_PROJECTILES).find(|&i| projectile_slot(i).read() == obj && rb(info(i), 0x10) != 0) else {
        return 0;
    };
    let p = info(i);
    wb(p, 0x10, 0);
    let fx = rp(p, 0x20);
    if !fx.is_null() {
        fx_kill(fx);
        wu(p, 0x20, 0);
    }
    clear_blip_for_entity(3, object_ref(projectile_slot(i).read()));
    world_remove(projectile_slot(i).read());
    let e = projectile_slot(i).read();
    if !e.is_null() {
        entity_vdelete(e, 1);
    }
    projectile_slot(i).write(GPtr::NULL);
    1
}

/// `CProjectileInfo::RemoveDetonatorProjectiles()` (0x738860): every active satchel charge (type 39)
/// explodes (explosion type 0 at the projectile, creator = the info's), is deactivated (fx killed) and
/// its object gets entity flag 0x800 (+0x1C).
#[hook(addr = 0x738860, conv = cdecl, name = "CProjectileInfo::RemoveDetonatorProjectiles", expect = "55 56 57 BD 10 91 C8 00")]
pub fn remove_detonator_projectiles() {
    for i in 0..NUM_PROJECTILES {
        let p = info(i);
        if rb(p, 0x10) == 0 {
            continue;
        }
        let e = projectile_slot(i).read();
        if ru(p, 0) != 0x27 {
            continue;
        }
        let pos = rv(pos_ptr(e), 0);
        add_explosion(GPtr::NULL, rp(p, 4), 0, pos[0], pos[1], pos[2], 0, 1, -1.0, 0);
        let fx = rp(p, 0x20);
        wb(p, 0x10, 0);
        if !fx.is_null() {
            fx_kill(fx);
            wu(p, 0x20, 0);
        }
        wu(e, 0x1C, ru(e, 0x1C) | 0x800);
    }
}

/// `CProjectileInfo::IsProjectileInRange(minX, maxX, minY, maxY, minZ, maxZ, bool destroy)`
/// (0x739860): true if any active projectile of type 19/20/18/17/16 lies in the box; with `destroy`
/// each such projectile is deactivated (fx killed), its blip cleared, removed and deleted (the slot is
/// not cleared).
#[hook(addr = 0x739860, conv = cdecl, name = "CProjectileInfo::IsProjectileInRange", expect = "51 53 55 56 33 DB")]
#[allow(clippy::too_many_arguments)]
pub fn is_projectile_in_range(x1: f32, x2: f32, y1: f32, y2: f32, z1: f32, z2: f32, destroy: u8) -> u8 {
    let mut found = 0;
    for i in 0..NUM_PROJECTILES {
        let p = info(i);
        let e = projectile_slot(i).read();
        if rb(p, 0x10) == 0 || !matches!(ru(p, 0), 0x10..=0x14) {
            continue;
        }
        let inside = crate::fire::in_range(rf(pos_ptr(e), 0), x1, x2)
            && crate::fire::in_range(rf(pos_ptr(e), 4), y1, y2)
            && crate::fire::in_range(rf(pos_ptr(e), 8), z1, z2);
        if !inside {
            continue;
        }
        found = 1;
        if destroy == 0 {
            continue;
        }
        let fx = rp(p, 0x20);
        wb(p, 0x10, 0);
        if !fx.is_null() {
            fx_kill(fx);
            wu(p, 0x20, 0);
        }
        let r = object_pool_get_ref(g::<GPtr<u8>>(0xB7449C).read(), e);
        clear_blip_for_entity(3, r);
        world_remove(e);
        entity_vdelete(e, 1);
    }
    found
}

/// `CProjectile` scalar deleting destructor `(uint8 flags)` (0x5A40F0, thiscall `ret 4`): the
/// destructor, then `operator delete` with bit 0; returns `this`.
#[hook(addr = 0x5A40F0, conv = thiscall, name = "CProjectile::`scalar deleting destructor'", expect = "56 8B F1 E8 E8 FF FF FF", this = "CProjectile")]
pub fn scalar_deleting_destructor(this: GPtr<u8>, flags: u8) -> GPtr<u8> {
    projectile_dtor(this);
    if flags & 1 != 0 {
        object_delete(this);
    }
    this
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
    HOOK_GET_PROJECTILE_INFO,
    HOOK_REMOVE_FX_SYSTEM,
    HOOK_SHUTDOWN,
    HOOK_REMOVE_ALL_PROJECTILES,
    HOOK_REMOVE_NOT_ADD,
    HOOK_REMOVE_IF_THIS_IS_A_PROJECTILE,
    HOOK_REMOVE_DETONATOR_PROJECTILES,
    HOOK_IS_PROJECTILE_IN_RANGE,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
];
