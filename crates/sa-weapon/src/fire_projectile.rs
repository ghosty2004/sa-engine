//! `CWeapon::FireProjectile` (0x741360, batch `samp2-weapon-1`): rockets, heat-seekers and thrown weapons.
//! Ported from the 1.0 US bytes (gta-reversed's version copies the origin, keeps a heat-seeker target that
//! has not been locked for 1.5 s, and checks `projType == ROCKET && target && targetPos` for the aimed
//! rocket; the bytes differ on all three). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;
use crate::weapon::ped_is_player;
use crate::weapon_fire::{
    allocate_matrix, event_group_add, event_gun_shot_ctor, event_gun_shot_dtor, get_event_global_group,
    interesting_events_add, normalise, report_crime, simple_transform_update_matrix,
};

use sa_world::bindings::placeable_get_matrix;
use sa_world::bindings::weapon_effects_is_locked_on;
orig! {
    /// `CProjectileInfo::AddProjectile(CEntity* creator, eWeaponType, CVector origin, float force, CVector*
    /// dir, CEntity* target)` (0x737C80)
    #[allow(clippy::too_many_arguments)]
    fn add_projectile(creator: GPtr<u8>, ty: u32, x: f32, y: f32, z: f32, force: f32, dir: GPtr<u8>, target: GPtr<u8>) -> u8 = cdecl 0x737C80 => crate::projectile_add::add_projectile;
    /// `CProjectileInfo::RemoveNotAdd(CEntity* creator, eWeaponType, CVector pos)` (0x737C00)
    fn remove_not_add(creator: GPtr<u8>, ty: u32, x: f32, y: f32, z: f32) = cdecl 0x737C00 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    #[allow(clippy::too_many_arguments)]
    fn world_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bd: u32, vh: u32, pd: u32, ob: u32, du: u32, st: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity* ignore, bool x6)` (0x569E20)
    #[allow(clippy::too_many_arguments)]
    fn world_test_sphere(x: f32, y: f32, z: f32, r: f32, ignore: GPtr<u8>, bd: u32, vh: u32, pd: u32, ob: u32, du: u32, cam: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CWeapon::PickTargetForHeatSeekingMissile(CVector origin, CVector dir, float, CEntity* ignore, bool,
    /// CEntity* last)` (0x73F910)
    #[allow(clippy::too_many_arguments)]
    fn pick_target_for_heat_seeking_missile(px: f32, py: f32, pz: f32, dx: f32, dy: f32, dz: f32, k: f32, ignore: GPtr<u8>, b: u32, last: GPtr<u8>) -> GPtr<u8> = cdecl 0x73F910 => unported;
}

/// `CWorld::pIgnoreEntity` (0xB7CD68).
const IGNORE_ENTITY: usize = 0xB7_CD68;

fn ensure_matrix(e: GPtr<u8>) {
    if rp(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rp(e, 0x14));
    }
}

/// `m ? m + 0x30 : e + 4`.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rp(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rf(p, 0), rf(p, 4), rf(p, 8)]
}

fn fp(v: &mut [f32; 3]) -> GPtr<u8> {
    lp(v).cast()
}

/// `CWeapon::FireProjectile(CEntity* firer, CVector* origin, CEntity* targetEnt, CVector* targetPos, float
/// force)` (0x741360, thiscall `ret 0x14`; entry `push -1; jmp` into the protection chunk 0x403C19, which
/// pushes the SEH handler).
///
/// **The original writes through `origin`** (the caller's vector moves).
/// * Rocket launchers (35, 36): the projectile type is 19 (+1 for the heat-seeker), the line start L =
///   `*origin` before it moves. A player ped fires only in active camera modes 0x22, 7, 8, 0x33, 0x2A, 0x27,
///   0x28, 0x34 (else false) and `origin += camera front`; others `origin += forward`. A player: for the
///   heat-seeker locked on (`IsLockedOn(0)`) with a press time (`pd+0x94`), `PickTargetForHeatSeekingMissile(
///   pos, forward, 1.2, firer, false, pd+0x98)`; only a target equal to `pd+0x98` held longer than 1500 ms is
///   kept (crosshair 0xC8A84C..: red, rotation 1, deactivate 0), anything else fires a plain rocket (19) at
///   nothing. A non-player ped with a target entity / position: false unless the 8 m line from L towards it
///   (`normalise(T - L) * 8`, buildings only, ignoring the firer) is clear. The end of the main line is the
///   moved origin.
/// * Others: `t = (origin - pos) . forward`; below 0.3 the origin moves forward by `0.3 - t`; the end E =
///   origin (+ forward * 0.6 when above the firer); L = `origin - forward * ((origin - pos) . forward)`.
///
/// `GetIsLineOfSightClear(L, E, buildings, vehicles, objects)` ignoring the firer: clear: an aimed rocket
/// (type 19 with a target entity or position) `AddProjectile(firer, 19, origin, force, normalise(T - L),
/// target)`, else `AddProjectile(firer, type, origin, force, null, target)`. Blocked: grenades / satchels
/// (16, 39) of a ped test a 0.3 sphere (peds only) at `pos - forward - (0, 0, 0.4)`; free: `AddProjectile(
/// firer, type, that point, 0.0, null, target)`, else `RemoveNotAdd(firer, type, origin)`; other types
/// `RemoveNotAdd`, other firers nothing. Then the ignore entity is cleared, crime 17 (ped) / 1 (vehicle with
/// a driver, by the driver), `g_InterestingEvents.Add(22, ped / driver)`, a `CEventGunShot(firer, origin, T
/// or targetPos or origin, silenced for 23 / 17)` to the global group; true. The original also stores
/// temporaries over its `firer` / `origin` argument slots (stack only).
#[hook(addr = 0x741360, conv = thiscall, name = "CWeapon::FireProjectile", this = "CWeapon",
       expect = "6A FF E9 B2 28 CC FF",
       unverified = "samp2-weapon-1", route = "L2", hazards = "heap-writes,stack-residue",
       recs = "weapon-combat",
       note = "entry jmps to the protection chunk 0x403C19 (SEH handler push); writes *origin")]
pub fn fire_projectile(this: GPtr<u8>, firer: GPtr<u8>, origin: GPtr<u8>, target_ent: GPtr<u8>, target_pos: GPtr<u8>,
    force: f32) -> u8 {
    let wt = ru(this, 0);
    let mut ty = wt;
    let mut target = target_ent;
    let mut lo;
    let mut lt;
    if wt != 0x23 && wt != 0x24 {
        ensure_matrix(firer);
        let m = rp(firer, 0x14);
        let p = pos_of(firer);
        let dz = x(rf(origin, 8)).fsub_m(rf(p, 8));
        let dy = x(rf(origin, 4)).fsub_m(rf(p, 4));
        let dx = x(rf(origin, 0)).fsub_m(rf(p, 0));
        let t = dy.fmul_m(rf(m, 0x14)).fadd(dz.fmul_m(rf(m, 0x18))).fadd(dx.fmul_m(rf(m, 0x10))).to_f32();
        if !x(t).fcom(x(cf(0x3E99_999A))).test_parity(5) {
            ensure_matrix(firer);
            let m = rp(firer, 0x14);
            let k = x(cf(0x3E99_999A)).fsub_m(t);
            let kz = k.fmul_m(rf(m, 0x18));
            let ky = k.fmul_m(rf(m, 0x14));
            wf(origin, 0, k.fmul_m(rf(m, 0x10)).fadd_m(rf(origin, 0)).to_f32());
            wf(origin, 4, ky.fadd_m(rf(origin, 4)).to_f32());
            wf(origin, 8, kz.fadd_m(rf(origin, 8)).to_f32());
        }
        lt = v3(origin);
        let m = rp(firer, 0x14);
        let p = pos_of(firer);
        if !x(rf(origin, 8)).fsub_m(rf(p, 8)).fcom(x(0.0)).test_any(0x41) {
            ensure_matrix(firer);
            let m = rp(firer, 0x14);
            let fz = x(rf(m, 0x18)).fmul_m(cf(0x3F19_999A));
            let fy = x(rf(m, 0x14)).fmul_m(cf(0x3F19_999A));
            let fx = x(rf(m, 0x10)).fmul_m(cf(0x3F19_999A));
            lt[0] = fx.fadd_m(lt[0]).to_f32();
            lt[1] = x(lt[1]).fadd(fy).to_f32();
            lt[2] = x(lt[2]).fadd(fz).to_f32();
        }
        let p = if m.is_null() { firer.byte_add(4) } else { m.byte_add(0x30) };
        let d = [
            x(rf(origin, 0)).fsub_m(rf(p, 0)).to_f32(),
            x(rf(origin, 4)).fsub_m(rf(p, 4)).to_f32(),
            x(rf(origin, 8)).fsub_m(rf(p, 8)).to_f32(),
        ];
        ensure_matrix(firer);
        let m = rp(firer, 0x14);
        let proj = x(d[2]).fmul_m(rf(m, 0x18)).fadd(x(d[1]).fmul_m(rf(m, 0x14))).fadd(x(d[0]).fmul_m(rf(m, 0x10))).to_f32();
        ensure_matrix(firer);
        let m = rp(firer, 0x14);
        let pz = x(proj).fmul_m(rf(m, 0x18));
        let py = x(proj).fmul_m(rf(m, 0x14)).to_f32();
        let px = x(proj).fmul_m(rf(m, 0x10)).to_f32();
        let lz = pz.fsubr_m(rf(origin, 8));
        let ly = x(rf(origin, 4)).fsub_m(py);
        lo = [x(rf(origin, 0)).fsub_m(px).to_f32(), ly.to_f32(), lz.to_f32()];
    } else {
        lo = v3(origin);
        ty = 0x13 + u32::from(wt == 0x24);
        let is_ped = etype(firer) == 3;
        let player = is_ped && ped_is_player(firer) != 0;
        if player {
            let off = usize::from(g::<u8>(0xB6_F081).read()) * 0x238;
            let mode = g::<u16>(0xB6_F1A8 + off).read();
            if !matches!(mode, 0x22 | 7 | 8 | 0x33 | 0x2A | 0x27 | 0x28 | 0x34) {
                return 0;
            }
            let f = 0xB6_F32C + off;
            wf(origin, 0, x(rf(origin, 0)).fadd_m(g::<f32>(f).read()).to_f32());
            wf(origin, 4, x(g::<f32>(f + 4).read()).fadd_m(rf(origin, 4)).to_f32());
            wf(origin, 8, x(g::<f32>(f + 8).read()).fadd_m(rf(origin, 8)).to_f32());
        } else {
            ensure_matrix(firer);
            let m = rp(firer, 0x14);
            wf(origin, 0, x(rf(origin, 0)).fadd_m(rf(m, 0x10)).to_f32());
            wf(origin, 4, x(rf(m, 0x14)).fadd_m(rf(origin, 4)).to_f32());
            wf(origin, 8, x(rf(m, 0x18)).fadd_m(rf(origin, 8)).to_f32());
        }
        if is_ped && ped_is_player(firer) != 0 {
            let pd = rp(firer, 0x480);
            let mut picked = GPtr::<u8>::NULL;
            let mut keep = false;
            if wt == 0x24 && weapon_effects_is_locked_on(0) != 0 && ru(pd, 0x94) != 0 {
                let m = placeable_get_matrix(firer);
                let p = pos_of(firer);
                let fwd = v3(m.byte_add(0x10));
                let pp = v3(p);
                picked = pick_target_for_heat_seeking_missile(pp[0], pp[1], pp[2], fwd[0], fwd[1], fwd[2],
                    cf(0x3F99_999A), firer, 0, rp(pd, 0x98));
                let held = sa_core::timer::TIME_MS.get().wrapping_sub(ru(pd, 0x94));
                if !picked.is_null() && picked == rp(pd, 0x98) && held > 0x5DC {
                    g::<u8>(0xC8_A84C).write(0xFF);
                    g::<u8>(0xC8_A84D).write(0);
                    g::<u8>(0xC8_A84E).write(0);
                    g::<f32>(0xC8_A85C).write(1.0);
                    g::<u32>(0xC8_A83C).write(0);
                    keep = true;
                }
            }
            if keep {
                target = picked;
            } else {
                ty = 0x13;
                target = GPtr::NULL;
            }
        } else if is_ped {
            target = target_ent;
            let t = if !target.is_null() { pos_of(target) } else { target_pos };
            if !t.is_null() {
                let mut d = [
                    x(rf(t, 0)).fsub_m(lo[0]).to_f32(),
                    x(rf(t, 4)).fsub_m(lo[1]).to_f32(),
                    x(rf(t, 8)).fsub_m(lo[2]).to_f32(),
                ];
                normalise(fp(&mut d));
                let dx8 = x(d[0]).fmul_m(8.0);
                let dy8 = x(d[1]).fmul_m(8.0);
                let dz8 = x(d[2]).fmul_m(8.0).to_f32();
                let mut end = [x(lo[0]).fadd(dx8).to_f32(), dy8.fadd_m(lo[1]).to_f32(), x(dz8).fadd_m(lo[2]).to_f32()];
                g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
                let clear = world_line_of_sight_clear(fp(&mut lo), fp(&mut end), 1, 0, 0, 0, 0, 0, 0);
                g::<GPtr<u8>>(IGNORE_ENTITY).write(GPtr::NULL);
                if clear == 0 {
                    return 0;
                }
            }
        } else {
            target = target_ent;
        }
        lt = v3(origin);
    }

    g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
    if world_line_of_sight_clear(fp(&mut lo), fp(&mut lt), 1, 1, 0, 1, 0, 0, 0) != 0 {
        let o = v3(origin);
        let aimed = if ty == 0x13 {
            if !target.is_null() { Some(pos_of(target)) } else if !target_pos.is_null() { Some(target_pos) } else { None }
        } else {
            None
        };
        if let Some(t) = aimed {
            let mut d = [
                x(rf(t, 0)).fsub_m(lo[0]).to_f32(),
                x(rf(t, 4)).fsub_m(lo[1]).to_f32(),
                x(rf(t, 8)).fsub_m(lo[2]).to_f32(),
            ];
            normalise(fp(&mut d));
            add_projectile(firer, 0x13, o[0], o[1], o[2], force, fp(&mut d), target);
        } else {
            add_projectile(firer, ty, o[0], o[1], o[2], force, GPtr::NULL, target);
        }
    } else if ty == 0x10 || ty == 0x27 {
        if etype(firer) == 3 {
            ensure_matrix(firer);
            let m = rp(firer, 0x14);
            let p = pos_of(firer);
            let z = x(rf(p, 8)).fsub_m(rf(m, 0x18));
            let py = x(rf(p, 4)).fsub_m(rf(m, 0x14)).to_f32();
            let px = x(rf(p, 0)).fsub_m(rf(m, 0x10)).to_f32();
            let pz = z.fsub_m(cf(0x3ECC_CCCD)).to_f32();
            if world_test_sphere(px, py, pz, cf(0x3E99_999A), GPtr::NULL, 0, 0, 1, 0, 0, 0).is_null() {
                add_projectile(firer, ty, px, py, pz, 0.0, GPtr::NULL, target);
            } else {
                let o = v3(origin);
                remove_not_add(firer, ty, o[0], o[1], o[2]);
            }
        }
    } else {
        let o = v3(origin);
        remove_not_add(firer, ty, o[0], o[1], o[2]);
    }
    g::<GPtr<u8>>(IGNORE_ENTITY).write(GPtr::NULL);
    match etype(firer) {
        3 => {
            report_crime(0x11, firer, firer);
            interesting_events_add(g(0xC0_B058), 0x16, firer);
        }
        2 => {
            let drv = rp(firer, 0x460);
            if !drv.is_null() {
                report_crime(1, firer, drv);
                interesting_events_add(g(0xC0_B058), 0x16, rp(firer, 0x460));
            }
        }
        _ => {}
    }
    let end = if !target.is_null() {
        v3(pos_of(target))
    } else if !target_pos.is_null() {
        v3(target_pos)
    } else {
        v3(origin)
    };
    let silenced = u32::from(matches!(ru(this, 0), 0x17 | 0x11));
    let o = v3(origin);
    let mut ev = [0u32; 0x34 / 4];
    let evp: GPtr<u8> = lp(&mut ev).cast();
    event_gun_shot_ctor(evp, firer, o[0], o[1], o[2], end[0], end[1], end[2], silenced);
    event_group_add(get_event_global_group(), evp, 0);
    event_gun_shot_dtor(evp);
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FIRE_PROJECTILE];
