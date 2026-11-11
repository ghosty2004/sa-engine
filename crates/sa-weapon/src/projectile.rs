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
    pub fn add_explosion(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, x: f32, y: f32, z: f32, life: u32, sound: u8, shake: f32, invisible: u8) -> u8 = cdecl 0x736A50 => crate::explosion::add_explosion;
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

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, b: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CAudioEngine::ReportObjectDestruction(CEntity*)` (0x506ED0, thiscall)
    pub fn audio_report_object_destruction(ae: GPtr<u8>, e: GPtr<u8>) = thiscall 0x506ED0 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volume, float speed)` (0x506EA0, thiscall)
    pub fn audio_report_frontend_event(ae: GPtr<u8>, ev: i32, vol: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CProjectileInfo::RemoveProjectile(CProjectileInfo*, CProjectile*)` (0x7388F0)
    pub fn remove_projectile_call(info: GPtr<u8>, obj: GPtr<u8>) = cdecl 0x7388F0 => remove_projectile;
    /// `CPed::IsPointerValid()` (0x5E4220, thiscall)
    pub fn ped_is_pointer_valid(ped: GPtr<u8>) -> u8 = thiscall 0x5E4220 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, thiscall)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::GetWeaponSlot(eWeaponType)` (0x5DF200, thiscall)
    pub fn ped_get_weapon_slot(ped: GPtr<u8>, ty: i32) -> i32 = thiscall 0x5DF200 => unported;
    /// `CWorld::SetPedsChoking(float x, y, z, float radius, CEntity*)` (0x565800); raw position words
    pub fn set_peds_choking(x: u32, y: u32, z: u32, r: f32, e: GPtr<u8>) = cdecl 0x565800 => unported;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730): `a x b`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, vehicles, peds,
    /// objects, dummies, seeThrough, ignoreSomeObjects)` (0x56A490)
    #[allow(clippy::too_many_arguments)]
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bd: u8, v: u8, p: u8, o: u8, d: u8, s: u8, i: u8) -> u8 = cdecl 0x56A490 => unported;
    /// `CPhysical::AttachEntityToEntity(CEntity*, CVector* offset, RtQuat*)` (quat overload, 0x54D690, thiscall)
    pub fn attach_entity_to_entity_quat(this: GPtr<u8>, e: GPtr<u8>, off: GPtr<u8>, q: GPtr<u8>) = thiscall 0x54D690 => unported;
}

/// `CReplay::Mode`
const REPLAY_MODE: usize = 0xA43088;
/// `CWorld::pIgnoreEntity`
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `1 / 32767` (0x858C7C)
const RAND_K: u32 = 0x3800_0100;
/// 9.9f (0x872C00), the projectile speed limit
const MAX_SPEED: u32 = 0x411E_6666;

/// `CProjectileInfo::RemoveProjectile(CProjectileInfo* info, CProjectile* obj)` (0x7388F0, cdecl): the
/// explosion by weapon type (`AddExplosion(null, X, type, obj position, 0, 1, -1.0, 0)`): grenade (16) and
/// free-fall bomb (21) type 0 by the creator; molotov (18) type 1, then `ReportObjectDestruction(obj)`;
/// rocket (19) type 2 by the creator, or by its driver (+0x460, may be null) for a vehicle creator;
/// heat-seeking rocket (20) type 2 when the creator is player 0's ped, else 3; other types none. Then
/// the info is deactivated, its fx killed and cleared, the object's radar blip cleared, the object
/// removed from the world and deleted (vtable slot 0 with 1). `ms_apProjectile` is not touched.
#[hook(addr = 0x7388F0, conv = cdecl, name = "CProjectileInfo::RemoveProjectile", expect = "55 8B 6C 24 08")]
pub fn remove_projectile(info: GPtr<u8>, obj: GPtr<u8>) {
    let c = rp(info, 4);
    let pos = |o: GPtr<u8>| {
        let p = rv_bits(pos_ptr(o), 0);
        [f32::from_bits(p[0]), f32::from_bits(p[1]), f32::from_bits(p[2])]
    };
    let boom = |by: GPtr<u8>, ty: i32| {
        let p = pos(obj);
        add_explosion(GPtr::NULL, by, ty, p[0], p[1], p[2], 0, 1, -1.0, 0);
    };
    match ru(info, 0) {
        0x10 | 0x15 => boom(c, 0),
        0x12 => {
            boom(c, 1);
            audio_report_object_destruction(g(AUDIO_ENGINE), obj);
        }
        0x13 => {
            if !c.is_null() && etype(c) == 2 {
                boom(rp(c, 0x460), 2);
            } else {
                boom(c, 2);
            }
        }
        0x14 => {
            let ty = if c == find_player_ped(-1) { 2 } else { 3 };
            boom(c, ty);
        }
        _ => {}
    }
    let fx = rp(info, 0x20);
    wb(info, 0x10, 0);
    if !fx.is_null() {
        fx_kill(fx);
        wu(info, 0x20, 0);
    }
    clear_blip_for_entity(3, object_ref(obj));
    world_remove(obj);
    if !obj.is_null() {
        entity_vdelete(obj, 1);
    }
}

/// `rand() * (1/32767)` as the x87 value.
fn rand_unit(site: u32) -> X87 {
    xi(sa_core::orig::at_site(site, rand)).fmul_m(cf(RAND_K))
}

/// `sqrt((x*x + y*y) + z*z)` of the three f32 at `v` as the bytes compute it (`fld z, y, x; ...`).
fn magnitude_xyz(v: GPtr<u8>) -> X87 {
    let (vx, vy, vz) = (x(rf(v, 0)), x(rf(v, 4)), x(rf(v, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt()
}

/// `if (|mv| > 9.9f) mv *= 9.9f / |mv|` (strict, x87; each component stored as f32).
fn clamp_speed(mv: GPtr<u8>) {
    let m = magnitude_xyz(mv);
    let mut f = Fpu::new();
    f.push(m);
    f.fcom_m(cf(MAX_SPEED));
    if f.jne(0x41) {
        return;
    }
    let k = m.fdivr_m(cf(MAX_SPEED));
    for off in [0, 4, 8] {
        wf(mv, off, k.fmul_m(rf(mv, off)).to_f32());
    }
}

/// `CWorld::GetIsLineOfSightClear(from, to, 1, 1, 1, 1, 0, 0, 0)`.
fn los_clear(from: GPtr<u8>, to: GPtr<u8>) -> bool {
    get_is_line_of_sight_clear(from, to, 1, 1, 1, 1, 0, 0, 0) != 0
}

/// The smoke trail of a rocket (types 19, 20; 0x738C73..0x738F1C).
fn rocket_trail(o: GPtr<u8>) {
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = lp(&mut mult).cast();
    crate::weapon::fx_prt_mult(mp, cf(0x3E99_999A), cf(0x3E99_999A), cf(0x3E99_999A), cf(0x3E99_999A), 0.5, 1.0, cf(0x3DA3_D70A));
    let ts = time_step();
    let s = [x(ts).fmul_m(rf(o, 0x44)).to_f32(), x(ts).fmul_m(rf(o, 0x48)).to_f32(), x(ts).fmul_m(rf(o, 0x4C)).to_f32()];
    let len = x(s[0]).fmul_m(s[0]).fadd(x(s[2]).fmul_m(s[2])).fadd(x(s[1]).fmul_m(s[1])).fsqrt();
    let n = ftol32(len).max(1);
    let nf = xi(n).to_f32();
    for j in 0..n {
        let v = rand_unit(0x738D25).fmul_m(0.25).fadd_m(0.25).to_f32();
        wf(mp, 8, v);
        wf(mp, 4, v);
        wf(mp, 0, v);
        wf(mp, 0x18, rand_unit(0x738D59).fmul_m(cf(0x3D23_D70A)).fadd_m(cf(0x3DA3_D70A)).to_f32());
        let f = xi(j).fdiv_m(nf).fsubr_m(1.0);
        let e0 = x(s[0]).fmul(f).to_f32();
        let e4 = x(s[1]).fmul(f).to_f32();
        let ez = x(s[2]).fmul(f);
        let pp = pos_ptr(o);
        let pz = ez.fsubr_m(rf(pp, 8)).to_f32();
        let py = x(rf(pp, 4)).fsub_m(e4).to_f32();
        let px = x(rf(pp, 0)).fsub_m(e0).to_f32();
        let mut p = [px, py, pz];
        let unit2 = |site: u32| {
            let r = rand_unit(site);
            r.fadd(r).fsub_m(1.0).to_f32()
        };
        let mut v = [0f32; 3];
        v[0] = unit2(0x738DE5);
        v[1] = unit2(0x738E07);
        v[2] = unit2(0x738E29);
        normalise(lp(&mut v).cast());
        let mut w = rv_bits(o, 0x44);
        normalise(lp(&mut w).cast());
        let mut c = [0u32; 3];
        let cp = cross_product(lp(&mut c).cast(), lp(&mut w).cast(), lp(&mut v).cast());
        let cv = rv(cp, 0);
        let mut vel = [x(cv[0]).fmul_m(1.5).to_f32(), x(cv[1]).fmul_m(1.5).to_f32(), x(cv[2]).fmul_m(1.5).to_f32()];
        crate::weapon::fx_add_particle(g::<GPtr<u8>>(0xA9AE20).read(), lp(&mut p).cast(), lp(&mut vel).cast(), 0.0, mp, -1.0, cf(0x3F99_999A), cf(0x3F19_999A), 0);
    }
}

/// The heat-seeking steering of type 20 (0x7391B6..0x7396B8); `info->m_pVictim` is non-null.
fn heat_seek(info: GPtr<u8>, o: GPtr<u8>) {
    let creator_is_player = |c: GPtr<u8>| c == find_player_ped(-1) || c == find_player_vehicle(-1, 0);
    if rp(info, 8) == find_player_vehicle(-1, 0) {
        audio_report_frontend_event(g(AUDIO_ENGINE), 0x65, 0.0, 1.0);
    }
    let m = rp(o, 0x14);
    let mut fw = rv_bits(m, 0x10);
    let fwp: GPtr<u8> = lp(&mut fw).cast();
    let p = rv(pos_ptr(o), 0);
    let fwf = rv(fwp, 0);
    let mut q = [x(fwf[0]).fadd_m(p[0]).to_f32(), x(p[1]).fadd_m(fwf[1]).to_f32(), x(p[2]).fadd_m(fwf[2]).to_f32()];
    let qp: GPtr<u8> = lp(&mut q).cast();
    let r0 = st0(crate::weapon::evaluate_target_call(rp(info, 8), qp, fwp, cf(0x3F99_999A), 1, GPtr::NULL)).to_f32();
    let mut best = 0f32;
    let mut best_e = GPtr::<u8>::NULL;
    for k in 0..NUM_PROJECTILES {
        let pi = crate::projectile::info(k);
        let e = projectile_slot(k).read();
        if ru(pi, 0) != 0x3A || rb(pi, 0x10) == 0 {
            continue;
        }
        let r = st0(crate::weapon::evaluate_target_call(e, qp, fwp, cf(0x3F99_999A), 1, GPtr::NULL));
        let mut f = Fpu::new();
        f.push(r);
        f.fcom_m(best);
        if f.ah() & 1 == 0 {
            best = r.to_f32();
            best_e = e;
        }
    }
    let mut t = best_e;
    let keep = !t.is_null() && {
        let mut f = Fpu::new();
        f.fld(best);
        f.fcomp_m(r0);
        f.ah() & 0x41 == 0
    };
    if !keep {
        t = rp(info, 8);
    }
    let plane = etype(t) == 2 && creator_is_player(rp(info, 4)) && ru(t, 0x594) == 4;
    let mv = o.byte_add(0x44);
    let pp = pos_ptr(o);
    let az = x(rf(mv, 8)).fmul_m(100.0).to_f32();
    let mut a = [
        x(rf(mv, 0)).fmul_m(100.0).fadd_m(rf(pp, 0)).to_f32(),
        x(rf(mv, 4)).fmul_m(100.0).fadd_m(rf(pp, 4)).to_f32(),
        x(az).fadd_m(rf(pp, 8)).to_f32(),
    ];
    if plane {
        a = rv(pp, 0);
    }
    let tp = pos_ptr(t);
    let op = pos_ptr(o);
    let dx = x(rf(op, 0)).fsub_m(rf(tp, 0));
    let dy = x(rf(op, 4)).fsub_m(rf(tp, 4));
    let dz = x(rf(op, 8)).fsub_m(rf(tp, 8));
    let dist = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy)).fsqrt();
    let d = dist.to_f32();
    let lim = |c: f32, cmp: X87| {
        let mut f = Fpu::new();
        f.push(cmp);
        f.fcomp_m(c);
        if f.jp(5) { x(c) } else { x(d) }
    };
    let mut k = lim(50.0, dist);
    if plane {
        k = lim(1.5, x(d));
    }
    let lx = x(k.fmul_m(rf(t, 0x44)).to_f32()).fadd_m(rf(tp, 0));
    let ly = x(k.fmul_m(rf(t, 0x48)).to_f32()).fadd_m(rf(tp, 4));
    let lz = k.fmul_m(rf(t, 0x4C)).fadd_m(rf(tp, 8)).to_f32();
    let mut cur = rv_bits(mv, 0);
    let mut dir = [lx.fsub_m(a[0]).to_f32(), ly.fsub_m(a[1]).to_f32(), x(lz).fsub_m(a[2]).to_f32()];
    let curp: GPtr<u8> = lp(&mut cur).cast();
    normalise(curp);
    let c = rv(curp, 0);
    let dot = x(dir[2]).fmul_m(c[2]).fadd(x(dir[1]).fmul_m(c[1])).fadd(x(dir[0]).fmul_m(c[0]));
    let mut f = Fpu::new();
    f.push(dot);
    f.fcom_m(0.0);
    if !f.jp(5) {
        let cx = x(c[0]).fmul(dot);
        let cy = x(c[1]).fmul(dot);
        let cz = x(c[2]).fmul(dot).to_f32();
        dir[0] = x(dir[0]).fsub(cx).to_f32();
        dir[1] = x(dir[1]).fsub(cy).to_f32();
        dir[2] = x(dir[2]).fsub_m(cz).to_f32();
    }
    normalise(lp(&mut dir).cast());
    let mut drag = 1.0f32;
    let mut rate = if creator_is_player(rp(info, 4)) { x(cf(0x3C3F_B15A)) } else { x(cf(0x3C13_74BC)) };
    let mut f = Fpu::new();
    f.push(magnitude_xyz(t.byte_add(0x44)));
    f.fcomp_m(cf(0x3F4C_CCCD));
    if !f.jne(0x41) {
        rate = rate.fmul_m(cf(0x3F99_999A));
    }
    if plane {
        let r = g::<u32>(0x8D6108).read();
        drag = sa_world::physical::shared::ci_pow(x(g::<f32>(0x8D6104).read()), x(time_step())).to_f32();
        rate = x(f32::from_bits(r));
    }
    for off in [0, 4, 8] {
        wf(mv, off, x(drag).fmul_m(rf(mv, off)).to_f32());
    }
    let rts = rate.fmul_m(time_step());
    let mx = x(dir[0]).fmul(rts);
    let my = x(dir[1]).fmul(rts).to_f32();
    let mz = x(dir[2]).fmul(rts).to_f32();
    wf(mv, 0, mx.fadd_m(rf(mv, 0)).to_f32());
    wf(mv, 4, x(my).fadd_m(rf(mv, 4)).to_f32());
    wf(mv, 8, x(mz).fadd_m(rf(mv, 8)).to_f32());
    clamp_speed(mv);
    wv_bits(rp(o, 0x14), 0x10, cur);
}

/// The rocket hit test (0x7396BB, then HIT at 0x739090): a collided object (+0x40 bit 0x200) hits at
/// once; else the line from the last position to the current one (creator ignored, the object's
/// collision flag cleared meanwhile, +0x128 = creator afterwards); a hit removes the projectile unless
/// its last collision (+0xB9 count, +0xBC entity) was with the creator or model 0x159.
fn rocket_hit_test(info: GPtr<u8>, o: GPtr<u8>) {
    if ru(o, 0x40) >> 9 & 1 == 0 {
        g::<GPtr<u8>>(IGNORE_ENTITY).write(rp(info, 4));
        wu(o, 0x1C, ru(o, 0x1C) & !1);
        let clear = los_clear(info.byte_add(0x14), pos_ptr(o));
        g::<u32>(IGNORE_ENTITY).write(0);
        wu(o, 0x1C, ru(o, 0x1C) | 1);
        wp(o, 0x128, rp(info, 4));
        if clear {
            return;
        }
    }
    projectile_hit(info, o);
}

/// HIT (0x739090).
fn projectile_hit(info: GPtr<u8>, o: GPtr<u8>) {
    if rb(o, 0xB9) != 0 {
        let e = rp(o, 0xBC);
        if !e.is_null() && (e == rp(info, 4) || rh(e, 0x22) == 0x159) {
            return;
        }
    }
    remove_projectile_call(info, o);
}

/// `CProjectileInfo::Update()` (0x738B20, cdecl): nothing in replay mode 1; for every active info
/// (`o = ms_apProjectile[i]` read first):
/// 1. submerged (`o->+0x40 & 0x100`): fx killed and cleared;
/// 2. a ped creator failing `IsPointerValid` is cleared (plain store);
/// 3. types 39/16/17: elasticity (+0x9C) > 0.1 and every `|mv| < 0.05` -> 0.03; type 17 after
///    `destroy - 17500` with `GetRandomNumberInRange(0, 100) < 10`: `SetPedsChoking(pos, 6.0, creator)`;
/// 4. types 19/20: the smoke trail (`n = max(_ftol(|ts*mv|), 1)` particles, five `rand()` each);
/// 5. expired (`now > destroy != 0`): satchel (39) of a player ped creator: destroy time 0 unless the
///    creator still holds detonator (40) ammo (creator dereferenced without a null check); other types:
///    `RemoveProjectile`;
/// 6. else by type: 19 gravity along the forward axis (`ts * 0.008`), speed clamp 9.9, hit test;
///    20 steering towards the victim or the best active flare (`EvaluateTargetForHeatSeekingMissile`),
///    then the hit test (no victim: hit test only); 0x3A flares stop at the last position when the line
///    is blocked; 18/21: removal on collision (+0x40 bit 0x200) or a blocked line, unless still within
///    `sqrt(2)` of the creator; 39: attached to the entity it touched (+0xDC) when +0xD8 > 0 and not yet
///    attached;
/// 7. the last position (+0x14) = the object's position, read also after `RemoveProjectile` deleted it
///    (original: freed pool memory, kept).
///
/// Kept original (deviations from `CProjectileInfo.md` found in the bytes): the heat-seeker's turn-rate
/// boost tests the TARGET's speed (`|T->m_vecMoveSpeed| > 0.8`, EBX = target at 0x73958B), not the
/// missile's. x87: the rocket's gravity x term and the trail's z term stay unrounded as in the bytes.
#[hook(addr = 0x738B20, conv = cdecl, name = "CProjectileInfo::Update", expect = "A0 88 30 A4 00")]
pub fn update() {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    for i in 0..NUM_PROJECTILES {
        let o = projectile_slot(i).read();
        let p = info(i);
        if rb(p, 0x10) == 0 {
            continue;
        }
        if ru(o, 0x40) & 0x100 != 0 {
            let fx = rp(p, 0x20);
            if !fx.is_null() {
                fx_kill(fx);
                wu(p, 0x20, 0);
            }
        }
        let c = rp(p, 4);
        if !c.is_null() && etype(c) == 3 && ped_is_pointer_valid(c) == 0 {
            wu(p, 4, 0);
        }
        let ty = ru(p, 0);
        if matches!(ty, 0x27 | 0x10 | 0x11) {
            let mut f = Fpu::new();
            f.fld(rf(o, 0x9C));
            f.fcomp_m(cf(0x3DCC_CCCD));
            if !f.jne(0x41) {
                let small = |off: isize| {
                    let mut f = Fpu::new();
                    f.fld(rf(o, off));
                    f.fabs();
                    f.fcomp_m(cf(0x3D4C_CCCD));
                    !f.jp(5)
                };
                if small(0x44) && small(0x48) && small(0x4C) {
                    wu(o, 0x9C, 0x3CF5_C28F);
                }
            }
            if ru(p, 0) == 0x11
                && time_ms() > ru(p, 0xC).wrapping_sub(0x445C)
                && sa_core::orig::at_site(0x738C1E, || random_int(0, 100)) < 10
            {
                let pp = pos_ptr(o);
                set_peds_choking(ru(pp, 0), ru(pp, 4), ru(pp, 8), 6.0, rp(p, 4));
            }
        }
        if matches!(ru(p, 0), 0x13 | 0x14) {
            rocket_trail(o);
        }
        let destroy = ru(p, 0xC);
        if time_ms() > destroy && destroy != 0 {
            if ru(p, 0) == 0x27 {
                let c = rp(p, 4);
                if etype(c) == 3 && ped_is_player(c) != 0 {
                    let c = rp(p, 4);
                    let s = ped_get_weapon_slot(c, 0x28) as isize;
                    let keep = ru(c, 0x5A0 + 0x1C * s) == 0x28 && {
                        let s = ped_get_weapon_slot(c, 0x28) as isize;
                        ru(c, 0x5AC + 0x1C * s) != 0
                    };
                    if !keep {
                        wu(p, 0xC, 0);
                    }
                }
            } else {
                remove_projectile_call(p, o);
            }
        } else {
            match ru(p, 0) {
                0x13 => {
                    let gk = x(time_step()).fmul_m(cf(0x3C03_126F));
                    let fwd = rp(o, 0x14).byte_add(0x10);
                    let gx = gk.fmul_m(rf(fwd, 0));
                    let gy = gk.fmul_m(rf(fwd, 4)).to_f32();
                    let gz = gk.fmul_m(rf(fwd, 8)).to_f32();
                    wf(o, 0x44, gx.fadd_m(rf(o, 0x44)).to_f32());
                    wf(o, 0x48, x(gy).fadd_m(rf(o, 0x48)).to_f32());
                    wf(o, 0x4C, x(gz).fadd_m(rf(o, 0x4C)).to_f32());
                    clamp_speed(o.byte_add(0x44));
                    rocket_hit_test(p, o);
                }
                0x14 => {
                    if !rp(p, 8).is_null() {
                        heat_seek(p, o);
                    }
                    rocket_hit_test(p, o);
                }
                0x3A => {
                    wu(o, 0x1C, ru(o, 0x1C) & !1);
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(rp(p, 4));
                    let clear = los_clear(p.byte_add(0x14), pos_ptr(o));
                    wu(o, 0x1C, ru(o, 0x1C) | 1);
                    g::<u32>(IGNORE_ENTITY).write(0);
                    if !clear {
                        wv_bits(o, 0x44, [0, 0, 0]);
                        // fld z, y, x; fstp x, y, z (SNaN quieted like the x87)
                        let last = [x(rf(p, 0x14)).to_f32(), x(rf(p, 0x18)).to_f32(), x(rf(p, 0x1C)).to_f32()];
                        if rp(o, 0x14).is_null() {
                            wv(o, 4, last);
                        } else {
                            wf(rp(o, 0x14), 0x30, last[0]);
                            wf(rp(o, 0x14), 0x34, last[1]);
                            wf(rp(o, 0x14), 0x38, last[2]);
                        }
                    }
                }
                0x12 | 0x15 => {
                    let mut here = rv_bits(pos_ptr(o), 0);
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(rp(p, 4));
                    wu(o, 0x1C, ru(o, 0x1C) & !1);
                    let c = rp(p, 4);
                    let near = !c.is_null() && {
                        let cp = pos_ptr(c);
                        let dx = x(rf(p, 0x14)).fsub_m(rf(cp, 0));
                        let dy = x(rf(p, 0x18)).fsub_m(rf(cp, 4));
                        let dz = x(rf(p, 0x1C)).fsub_m(rf(cp, 8));
                        let mut f = Fpu::new();
                        f.push(dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy)));
                        f.fcomp_m(2.0);
                        !f.jp(5)
                    };
                    if !near && (ru(o, 0x40) & 0x200 != 0 || !los_clear(p.byte_add(0x14), lp(&mut here).cast())) {
                        remove_projectile_call(p, o);
                    }
                    g::<u32>(IGNORE_ENTITY).write(0);
                    wu(o, 0x1C, ru(o, 0x1C) | 1);
                }
                0x27 => {
                    let mut f = Fpu::new();
                    f.fld(rf(o, 0xD8));
                    f.fcomp_m(0.0);
                    let e = rp(o, 0xDC);
                    if !f.jne(0x41) && !e.is_null() && rp(o, 0xFC).is_null() {
                        attach_entity_to_entity_quat(o, e, GPtr::NULL, GPtr::NULL);
                        wu(o, 0x1C, ru(o, 0x1C) & !1);
                    }
                }
                _ => {}
            }
        }
        wv_bits(p, 0x14, rv_bits(pos_ptr(o), 0));
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_REMOVE_PROJECTILE,
    HOOK_UPDATE,

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
