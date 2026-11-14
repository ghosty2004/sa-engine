//! `CWeapon::FireInstantHit` (0x73FB10, batch `samp2-weapon-1`): every hitscan shot of peds and vehicles
//! (SA:MP's on-foot and driver shooting). Unreversed in gta-reversed (`plugin::CallMethodAndReturn`), ported
//! from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! Frame (S = entry ESP, body ESP B = S - 0x1B4): the start S (B+0x2C), the end E (B+0x10), the hit entity
//! (B+0x28), the spread (B+0x1C), the muzzle copy M (B+0x78), the shot direction (B+0x6C), the col point
//! (B+0x90), the camera-path origin (B+0x38) and the two events (B+0x174, B+0x140) are Rust locals. The SEH
//! frame is not reproduced.
//!
//! **Original bug kept, made deterministic**: on the NPC path that skips the line of sight (a ped firer with
//! a target but neither `+0x528 & 1` nor `forceAim`) the shot direction is never written, and the gunshell
//! position uses it (`AddGunshell`). The original reads whatever an earlier call left in its frame;
//! sa-hook's determinism mode zeroes the frame at the entry (`detwin.rs`, `fire_instant_hit_scrub`), and
//! this port starts the direction (and the col point) at zero, which is what the scrubbed original reads.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;
use crate::projectile::cross_product;
use crate::weapon::{
    birds_handle_gun_shot, intel_get_task_use_gun, ped_is_player, shadows_gun_shot_sets_oil_on_fire,
    world_process_line_of_sight,
};
use crate::weapon_fire::{
    allocate_matrix, check_for_shooting_vehicle_occupant_call, do_bullet_impact, do_drive_by_auto_aiming,
    entity_get_col_model, event_group_add, event_gun_shot_ctor, event_gun_shot_dtor, event_whizzed_by_ctor,
    event_whizzed_by_dtor, find_player_ped, get_anim_hierarchy_from_skin_clump, get_event_global_group,
    interesting_events_add, matrix_dtor, normalise, ped_get_weapon_skill, point_lights_add_light,
    rp_hanim_hierarchy_get_matrix_array, rp_hanim_id_get_index, rw_v3d_transform_points,
    simple_transform_update_matrix, target_weapon_range_multiplier_call, vector_magnitude,
    world_reset_line_test_options,
};

use crate::bindings::audio_report_bullet_hit;
use sa_core::bindings::ped_get_bone_position;
use sa_world::bindings::ped_get_transformed_bone_position;
use crate::bindings::add_gunshell as add_gunshell_call;
use sa_world::bindings::cam_get_two_player_aim_vector;
use sa_world::bindings::fx_trigger_gunshot;
use sa_world::bindings::camera_find_3rd_person_cam_target_vector;
use sa_world::bindings::process_col_models as collision_process_col_models;
use sa_stream::bindings::animate_ped_col_model_skinned as ped_mi_animate_col_model_skinned;
orig! {
    /// `CWeapon::DoDoomAiming(CEntity* owner, CVector* start, CVector* end)` (0x73CDC0)
    fn do_doom_aiming(owner: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) = cdecl 0x73CDC0 => unported;
    /// `FUN_006E61B0(CVector start, CVector end, CVector* out)` (`CWaterLevel::TestLineAgainstWater`), AL
    #[allow(clippy::too_many_arguments)]
    fn test_line_against_water(sx: f32, sy: f32, sz: f32, ex: f32, ey: f32, ez: f32, out: GPtr<u8>) -> u8 = cdecl 0x6E61B0 => unported;
    /// `Fx_c::TriggerBulletSplash(CVector& pos)` (0x4A10E0, thiscall)
    fn fx_trigger_bullet_splash(fx: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x4A10E0 => unported;
    /// `CWeapon::SetUpPelletCol(int32 n, CEntity* owner, CEntity* victim, CVector& point, CColPoint&, CMatrix&
    /// out)` (0x73C710, thiscall `ret 0x18`)
    #[allow(clippy::too_many_arguments)]
    fn set_up_pellet_col(this: GPtr<u8>, n: i32, owner: GPtr<u8>, victim: GPtr<u8>, point: GPtr<u8>, cp: GPtr<u8>, mat: GPtr<u8>) -> u8 = thiscall 0x73C710 => unported;
}

/// `CWorld::pIgnoreEntity` (0xB7CD68) and the line-test options `bIncludeDeadPeds` (0xB7CD6F),
/// `bIncludeBikers` (0xB7CD70), `bIncludeCarTyres` (0xB7CD71).
const IGNORE_ENTITY: usize = 0xB7_CD68;
const INCLUDE_DEAD_PEDS: usize = 0xB7_CD6F;
const INCLUDE_BIKERS: usize = 0xB7_CD70;
const INCLUDE_CAR_TYRES: usize = 0xB7_CD71;
/// The shotgun pellet spread (`.bss` float written here, 0xB7CD64).
const PELLET_SPREAD: usize = 0xB7_CD64;
/// `g_fx` (0xA9AE00), `AudioEngine` (0xB6BC90), `g_InterestingEvents` (0xC0B058), `TheCamera` (0xB6F028).
const G_FX: usize = 0xA9_AE00;
const AUDIO_ENGINE: usize = 0xB6_BC90;
const INTERESTING_EVENTS: usize = 0xC0_B058;
const THE_CAMERA: usize = 0xB6_F028;
/// `TheCamera.Cams[0].Front` (0xB6F32C) / `.Up` (0xB6F350) / `.Mode` (0xB6F1A8), `Cams[0]` (0xB6F19C).
const CAM0_FRONT: usize = 0xB6_F32C;
const CAM0_UP: usize = 0xB6_F350;
const CAM0_MODE: usize = 0xB6_F1A8;
const CAM0: usize = 0xB6_F19C;
/// `CWeapon` statics in `.data`: the spread scale 0.75 (0x8D6110), the range for full spread 5.0 (0x8D6114),
/// the sway rate (0x8D6118), the pellet spread numerator 0.05 (0x8D611C), the pellet counts 15 / 8 (0x8D6120,
/// 0x8D6124 for the SPAS).
const SPREAD_SCALE: usize = 0x8D_6110;
const SPREAD_RANGE: usize = 0x8D_6114;
const SWAY_RATE: usize = 0x8D_6118;
const PELLET_NUM: usize = 0x8D_611C;
const PELLETS: usize = 0x8D_6120;
const PELLETS_SPAS: usize = 0x8D_6124;
/// `CColPoint` arrays the pellet test writes (0xB9ACD0, 0x2C each), the pellet col model (0xC8A7DC), the
/// gunflash-light toggle byte (0xC8A80C), `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8).
const PELLET_POINTS: usize = 0xB9_ACD0;
const PELLET_COL: usize = 0xC8_A7DC;
const FLASH_TOGGLE: usize = 0xC8_A80C;
const MODEL_INFOS: usize = 0xA9_B0C8;

/// `CColPoint` (0x2C bytes), `CEventGunShot` / `CEventGunShotWhizzedBy` (0x34), `CMatrix` (0x48).
const COL_POINT_WORDS: usize = 0x2C / 4;
const EVENT_WORDS: usize = 0x34 / 4;
const MATRIX_WORDS: usize = 0x48 / 4;

fn ensure_matrix(e: GPtr<u8>) {
    if rp(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rp(e, 0x14));
    }
}

fn fptr(v: &mut [f32; 3]) -> GPtr<u8> {
    lp(v).cast()
}

use sa_world::world::misc2::below as lt;
/// The three-`rand()` scatter the original uses on two paths (calls at the given return addresses):
/// `E.x += (r3/32767*0.4 - 0.2) * sp`, `E.y += (r2/32767*0.4 - 0.2) * sp` (both products kept at 80 bits),
/// `E.z = (f32)((r1/32767*0.2 - 0.1) * sp) + E.z`.
fn scatter(e: &mut [f32; 3], sp: f32, sites: [u32; 3]) {
    let unit = cf(0x3800_0100);
    let r1 = sa_core::orig::at_site(sites[0], rand);
    let f1 = xi(r1).fmul_m(unit).to_f32();
    let r2 = sa_core::orig::at_site(sites[1], rand);
    let f2 = xi(r2).fmul_m(unit).to_f32();
    let r3 = sa_core::orig::at_site(sites[2], rand);
    let a3 = xi(r3).fmul_m(unit).fmul_m(cf(0x3ECC_CCCD)).fsub_m(cf(0x3E4C_CCCD));
    let a2 = x(f2).fmul_m(cf(0x3ECC_CCCD)).fsub_m(cf(0x3E4C_CCCD));
    let dz = x(f1).fmul_m(cf(0x3E4C_CCCD)).fsub_m(cf(0x3DCC_CCCD)).fmul_m(sp).to_f32();
    let a2s = a2.fmul_m(sp);
    e[0] = a3.fmul_m(sp).fadd_m(e[0]).to_f32();
    e[1] = x(e[1]).fadd(a2s).to_f32();
    e[2] = x(dz).fadd_m(e[2]).to_f32();
}

/// The aim sway of the player paths: `t = (double)(u32)CTimer::m_snTimeInMilliseconds * rate` (x87:
/// `fild` + 2^32 when negative), `s = (f32)sin(t)`, `c = (f32)cos(t)`; E += vs * sp * s + vc * sp * c with
/// the original's rounding (x / y of each product stored f32, z kept; `E.z = vc.z*sp*c + (vs.z*sp*s + E.z)`).
fn sway(e: &mut [f32; 3], vs: [f32; 3], vc: [f32; 3], sp: f32) {
    let ms = sa_core::timer::TIME_MS.get() as i32;
    let mut t = xi(ms);
    if ms < 0 {
        t = t.fadd_m(cf(0x4F80_0000));
    }
    let t = t.fmul_m(g::<f32>(SWAY_RATE).read());
    let zs = x(vs[2]).fmul_m(sp);
    let ys = x(vs[1]).fmul_m(sp).to_f32();
    let xs = x(vs[0]).fmul_m(sp).to_f32();
    let s = t.fsin().to_f32();
    let zss = zs.fmul_m(s);
    e[0] = x(xs).fmul_m(s).fadd_m(e[0]).to_f32();
    e[1] = x(e[1]).fadd(x(ys).fmul_m(s)).to_f32();
    let zz = zss.fadd_m(e[2]);
    let zc = x(vc[2]).fmul_m(sp);
    let yc = x(vc[1]).fmul_m(sp).to_f32();
    let xc = x(vc[0]).fmul_m(sp).to_f32();
    let c = t.fcos().to_f32();
    let zcc = zc.fmul_m(c);
    e[0] = x(xc).fmul_m(c).fadd_m(e[0]).to_f32();
    e[1] = x(e[1]).fadd(x(yc).fmul_m(c)).to_f32();
    e[2] = zcc.fadd(zz).to_f32();
}

/// `playerData->m_fAttackButtonCounter += (float)info->m_nAnimLoop... ` : `pd+0x2C += (i16)info+0x22 * 0.04f`.
fn add_recoil(pd: GPtr<u8>, info: GPtr<u8>) {
    let v = xi(i32::from(rh(info, 0x22) as i16)).fmul_m(cf(0x3D23_D70A)).fadd_m(rf(pd, 0x2C)).to_f32();
    wf(pd, 0x2C, v);
}

/// `min(SPREAD_RANGE / range * mul, 1.0)` as the original computes it (the quotient recomputed when kept).
fn range_factor(info: GPtr<u8>, mul: Option<f32>) -> X87 {
    let q = |m: Option<f32>| {
        let v = x(rf(info, 8)).fdivr_m(g::<f32>(SPREAD_RANGE).read());
        match m {
            Some(k) => v.fmul_m(k),
            None => v,
        }
    };
    let v = q(mul);
    if !x(1.0).fcom(v).test_parity(5) { x(1.0) } else { q(mul) }
}

/// `E = base + v * range` as the vehicle paths build it (`v.x*r` kept, `v.y*r` and `r*v.z` stored f32,
/// then `base.x + vx*r`, `E.y + base.y`, `base.z + E.z`).
fn extend(e: &mut [f32; 3], base: [f32; 3], r: f32) {
    let xr = x(e[0]).fmul_m(r);
    e[1] = x(e[1]).fmul_m(r).to_f32();
    e[2] = x(r).fmul_m(e[2]).to_f32();
    e[0] = x(base[0]).fadd(xr).to_f32();
    e[1] = x(e[1]).fadd_m(base[1]).to_f32();
    e[2] = x(base[2]).fadd_m(e[2]).to_f32();
}

/// The line of sight every path ends with: `HandleGunShot(from, E)`, `GunShotSetsOilOnFire(from, E)`,
/// `ProcessLineOfSight(from, E, cp, hit, 1, 1, 1, 1, 1, 0, 0, 1)`.
fn shoot_line(from: GPtr<u8>, e: GPtr<u8>, cp: GPtr<u8>, hit: GPtr<u8>) {
    birds_handle_gun_shot(from, e);
    shadows_gun_shot_sets_oil_on_fire(from, e);
    world_process_line_of_sight(from, e, cp, hit, 1, 1, 1, 1, 1, 0, 0, 1);
}

/// `CWeapon::FireInstantHit(CEntity* firer, CVector* origin, CVector* muzzle, CEntity* targetEnt, CVector*
/// target, CVector* originForDriveBy, bool forceAim, bool doMuzzleFx)` (0x73FB10, thiscall `ret 0x20`); true.
///
/// Weapon info: `GetWeaponInfo(type, ped ? ped->GetWeaponSkill(type) : 1)`. S = `*originForDriveBy` if given,
/// else `*origin`; M = `*muzzle`; E = 0, hit = null, spread = 0. A ped firer: `spread = (100 - accuracy byte
/// +0x71A) / info->m_fAccuracy` (halved for a player with `+0x46C & 0x4000000`) and its `TaskUseGun`. The
/// shotguns 25/26/27: spread 0 and the pellet spread `0.05f / accuracy` (0xB7CD64).
///
/// Paths, in the original's order:
/// 1. **Task aims with a bone** (`TaskUseGun` with byte +0xE): `E = (range, 0, 0)` transformed by the bone
///    matrix of `[firer+0x4A0]+0x14` of the skin hierarchy, `dir = normalise(E - S)`; ignore = the firer's
///    vehicle (`+0x46C & 0x100`, `+0x58C`) or its attached vehicle (`+0xFC`, type 2), dead peds included;
///    the line from S.
/// 2. **Ped with a target** (`target` or `targetEnt`): only with `+0x528 & 1` or `forceAim`, else no line
///    at all (the direction stays unwritten, see the module doc). E = `*target`, else for a ped target the
///    firer's player-data bone offset transformed by `GetTransformedBonePosition` (no player data:
///    `GetBonePosition(3)`), else the target's position. `dir = (E - S) / max(|E - S|, 0.01)` (E holds it
///    too), `E = S + dir * TargetWeaponRangeMultiplier(targetEnt, firer) * range`. A player with spread:
///    `spread' = min(5 / range, 1) * spread * pd+0x2C * 0.75`, `spread = rand in [min(spread'/2, 0.2),
///    spread']`, the sway (vectors by camera 0 mode 0x35/0x37: `normalise(Front x Up)` / Up, else
///    `normalise(E x Z)` / `normalise(that x E)`), recoil `pd+0x2C += anim * 0.04`. Without player data and
///    spread > 0: a player target scales the spread by `min(|speed|, 0.33) * 0.90909 + 0.8`, then the
///    three-`rand()` scatter. Ignore = the vehicle / attached vehicle unless `+0x42D & 4`; a player firer
///    includes car tyres; dead peds included; the line from S.
/// 3. **Player without a target** in camera mode 0x35 / 0x37 / 0x41 / 0x31 (active camera):
///    `Find3rdPersonCamTargetVector(range * 3, S, C, E)`, `dir = normalise(E - S)`; with spread:
///    `spread = min(5 / range * 3, 1) * spread * pd+0x2C * 0.75`, the sway (camera 0 mode 0x35/0x37/0x31:
///    `normalise(Front x Up)` / Up, 0x31 calling `Get_TwoPlayer_AimVector` first; else `normalise(E x Z)` /
///    `normalise(that x E)`), recoil. Ignore = vehicle / attached vehicle unless `+0x42D & 4`, else the
///    firer; tyres, bikers, dead peds; the line **from C**. A hit farther (2D) than
///    `TargetWeaponRangeMultiplier(hit, firer) * range` is dropped, else `CheckForShootingVehicleOccupant`.
/// 4. **Vehicle firer**: spread 0.6, `dir = E = forward`; for status 0 / 8: `E = S + E * range`, aim flag
///    `subclass 4 or 3`, `DoDriveByAutoAiming(status 8 ? FindPlayerPed() : driver, veh, S, E, flag)`,
///    `E = normalise(E - S)`, spread 0.1 for models 447 / 469 / 564 else 0.3; the scatter, `normalise(E)`,
///    ignore = the vehicle, `E = S + E * range`; the line from S (dead peds not set here).
/// 5. **Other** (non-ped non-vehicle firers, and peds 3 skips): `E = M + forward * range`, `dir = forward`;
///    a ped with `+0x470 & 0x2000000` doom-aims unless it is a player with `info+0x18 & 1`; ignore = vehicle
///    / attached entity (any type, kept only for type 2) / the firer; dead peds; the line from S.
///
/// Then: a `CEventGunShot(firer, S, E, type 23 or 17)` and a `CEventGunShotWhizzedBy(firer, S, E, type 23
/// only)` to the global event group, `g_InterestingEvents.Add(22, firer)`, `IsPlayer(ped)` (result unused),
/// S re-read from `*origin`. Muzzle effects (`doMuzzleFx`) by type: a point light at M (radius 3, 0.25,
/// 0.22, 0) with the gunshell offset / size (22-24, 34: 0.2 / 0.25; 25-27: 0.3 / 0.45; 28, 29, 32: 0.2 / 0.3;
/// 30, 31, 38: 0.65 / 0.25 when `ftol((info+0x44 - info+0x40) * 900) >= 50` or when the toggle byte 0xC8A80C
/// becomes even); `TriggerGunshot(firer, M, dir, !(ped with +0x4F8))`; `AddGunshell(firer, M - dir * offset,
/// firer->matrix.right.xy, size)`. Water: for a vehicle/ped/object target with `+0x40 & 0x100`, or no target
/// with `E.z < S.z` for a player ped or a firer of status 0 / 8: the segment S..hit point (S..E without a hit)
/// against water, a splash and `ReportBulletHit(null, 39, point, 0)`. Impact: with a pellet spread > 0, a hit
/// and not a vehicle tyre (piece 13..16), the pellet loop (`SetUpPelletCol`, `ProcessColModels` against the
/// pellet col model, `DoBulletImpact` per pellet; peds and vehicles with fewer than half the pellets on the
/// first pass re-test from the hit point once with the hit ignored); otherwise `DoBulletImpact(firer, hit,
/// M, E, cp, 0)`. `ResetLineTestOptions`.
#[hook(addr = 0x73FB10, conv = thiscall, name = "CWeapon::FireInstantHit", this = "CWeapon",
       expect = "6A FF 68 BB 8E 84 00",
       unverified = "samp2-weapon-1", route = "L2", hazards = "x87-transcendental,uninit-locals,heap-writes",
       recs = "weapon-combat",
       note = "rand at 0x740254/0x74026B/0x740282 and 0x74090E/0x740925/0x74093C; fsin/fcos of the sway; sa-hook's determinism scrub (detwin.rs) redirects this entry before ports install; the installer then takes the scrub stub as the original (win.rs install_one)")]
#[allow(clippy::too_many_arguments)]
pub fn fire_instant_hit(
    this: GPtr<u8>,
    firer: GPtr<u8>,
    origin: GPtr<u8>,
    muzzle: GPtr<u8>,
    target_ent: GPtr<u8>,
    target: GPtr<u8>,
    alt: GPtr<u8>,
    force_aim: u32,
    muzzle_fx: u32,
) -> u8 {
    let ty = ru(this, 0);
    let skill = if !firer.is_null() && etype(firer) == 3 { (ped_get_weapon_skill(firer, ty as i32) as i32) as u8 } else { 1 };
    let info = get_weapon_info(ty as i32, skill);
    let mut m: [f32; 3] = muzzle.cast::<[f32; 3]>().read();
    let mut s: [f32; 3] = origin.cast::<[f32; 3]>().read();
    let mut e = [0f32; 3];
    let mut hit = GPtr::<u8>::NULL;
    if !alt.is_null() {
        s = alt.cast::<[f32; 3]>().read();
    }
    let mut cp = [0u32; COL_POINT_WORDS];
    let mut dir = [0f32; 3];
    let mut ped = GPtr::<u8>::NULL;
    let mut spread = 0f32;
    let mut task = GPtr::<u8>::NULL;
    if etype(firer) == 3 {
        let acc = i32::from(rb(firer, 0x71A));
        let pd = rp(firer, 0x480);
        ped = firer;
        spread = xi(acc).fsubr_m(cf(0x42C8_0000)).fdiv(x(rf(info, 0x38))).to_f32();
        if !pd.is_null() && ru(firer, 0x46C) & 0x0400_0000 != 0 {
            spread = x(spread).fmul_m(0.5).to_f32();
        }
        task = intel_get_task_use_gun(rp(firer, 0x47C));
    }
    if matches!(ru(this, 0), 0x19..=0x1B) {
        spread = 0.0;
        let v = x(g::<f32>(PELLET_NUM).read()).fdiv(x(rf(info, 0x38))).to_f32();
        g::<f32>(PELLET_SPREAD).write(v);
    }
    let mut cam_target = [0f32; 3];
    let cpp: GPtr<u8> = lp(&mut cp).cast();
    let hitp: GPtr<u8> = lp(&mut hit).cast();
    let mut vehicle_line = false;
    'paths: {
        if !task.is_null() && rb(task, 0xE) != 0 {
            // 1. bone aim
            e = [rf(info, 8), 0.0, 0.0];
            let hier = get_anim_hierarchy_from_skin_clump(rp(firer, 0x18));
            let idx = rp_hanim_id_get_index(hier, (ru(rp(firer, 0x4A0), 0x14) as i32) as u32);
            let mat = rp_hanim_hierarchy_get_matrix_array(hier).byte_add((idx << 6) as isize);
            rw_v3d_transform_points(GPtr::<u8>::cast::<f32>(fptr(&mut e)), GPtr::<u8>::cast::<f32>(fptr(&mut e)), 1, mat);
            let dz = x(e[2]).fsub_m(s[2]).to_f32();
            let dy = x(e[1]).fsub_m(s[1]).to_f32();
            let dx = x(e[0]).fsub_m(s[0]).to_f32();
            dir = [dx, dy, dz];
            normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut dir)));
            let veh = rp(ped, 0x58C);
            if ru(ped, 0x46C) & 0x100 != 0 && !veh.is_null() {
                g::<GPtr<u8>>(IGNORE_ENTITY).write(veh);
            } else {
                let att = rp(ped, 0xFC);
                if !att.is_null() && etype(att) == 2 {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(att);
                }
            }
            g::<u8>(INCLUDE_DEAD_PEDS).write(1);
            shoot_line(fptr(&mut s), fptr(&mut e), cpp, hitp);
            break 'paths;
        }
        if !ped.is_null() && (!target_ent.is_null() || !target.is_null()) {
            // 2. ped with a target
            if rb(ped, 0x528) & 1 == 0 && force_aim as u8 == 0 {
                break 'paths;
            }
            if !target.is_null() {
                e = target.cast::<[f32; 3]>().read();
            } else if etype(target_ent) == 3 {
                let pd = rp(ped, 0x480);
                if !pd.is_null() {
                    e = [rf(pd, 0x74), rf(pd, 0x78), rf(pd, 0x7C)];
                    ped_get_transformed_bone_position(target_ent, fptr(&mut e), ru(pd, 0x70), 0);
                } else {
                    ped_get_bone_position(target_ent, fptr(&mut e), 3, 0);
                }
            } else {
                let mp = rp(target_ent, 0x14);
                let p = if mp.is_null() { target_ent.byte_add(4) } else { mp.byte_add(0x30) };
                e = p.cast::<[f32; 3]>().read();
            }
            let dx = x(e[0]).fsub_m(s[0]);
            e[1] = x(e[1]).fsub_m(s[1]).to_f32();
            let dz = x(e[2]).fsub_m(s[2]);
            let l2 = dx.fmul(dx).fadd(x(e[1]).fmul_m(e[1])).fadd(dz.fmul(dz));
            let len = if l2.fsqrt().fcom(x(cf(0x3C23_D70A))).test_parity(5) { l2.fsqrt() } else { x(cf(0x3C23_D70A)) };
            let inv = len.fdivr_m(1.0);
            e[0] = inv.fmul(dx).to_f32();
            dir[0] = e[0];
            e[1] = x(e[1]).fmul(inv).to_f32();
            dir[1] = e[1];
            e[2] = inv.fmul(dz).to_f32();
            dir[2] = e[2];
            let range = rf(info, 8);
            let mr = st0(target_weapon_range_multiplier_call(target_ent, ped).to_f64()).fmul_m(range);
            let exm = x(e[0]).fmul(mr);
            e[1] = x(e[1]).fmul(mr).to_f32();
            e[2] = x(e[2]).fmul(mr).to_f32();
            e[0] = x(s[0]).fadd(exm).to_f32();
            e[1] = x(e[1]).fadd_m(s[1]).to_f32();
            e[2] = x(s[2]).fadd_m(e[2]).to_f32();
            let pd = rp(ped, 0x480);
            if !pd.is_null() && !is_zero(spread) {
                let sp = range_factor(info, None)
                    .fmul_m(spread)
                    .fmul_m(rf(pd, 0x2C))
                    .fmul_m(g::<f32>(SPREAD_SCALE).read());
                spread = sp.to_f32();
                let half = sp.fmul_m(0.5);
                let lo = if !x(cf(0x3E4C_CCCD)).fcom(half).test_parity(5) { cf(0x3E4C_CCCD) } else { half.to_f32() };
                spread = st0(random_float(lo, spread).to_f64()).to_f32();
                let mode = g::<u16>(CAM0_MODE).read();
                let (vs, vc) = if mode == 0x35 || mode == 0x37 {
                    let mut up: [f32; 3] = g::<[f32; 3]>(CAM0_UP).read();
                    let mut out = [0f32; 3];
                    let r = cross_product(fptr(&mut out), g(CAM0_FRONT), fptr(&mut up));
                    let mut a: [f32; 3] = r.cast::<[f32; 3]>().read();
                    normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut a)));
                    (a, up)
                } else {
                    let mut zup = [0.0, 0.0, 1.0];
                    let mut out = [0f32; 3];
                    let mut a: [f32; 3] = cross_product(fptr(&mut out), fptr(&mut e), fptr(&mut zup)).cast::<[f32; 3]>().read();
                    normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut a)));
                    let mut u: [f32; 3] = cross_product(fptr(&mut out), fptr(&mut a), fptr(&mut e)).cast::<[f32; 3]>().read();
                    normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut u)));
                    (a, u)
                };
                sway(&mut e, vs, vc, spread);
                add_recoil(pd, info);
            } else if !x(spread).fcom(x(0.0)).test_any(0x41) {
                if !target_ent.is_null() && etype(target_ent) == 3 && ped_is_player(target_ent) != 0 {
                    let mv = target_ent.byte_add(0x44);
                    let c = x(cf(0x3EA8_F5C3));
                    let m1 = st0(vector_magnitude(mv.cast::<u32>()));
                    let v = if m1.fcom(c).test_any(0x41) { st0(vector_magnitude(mv.cast::<u32>())) } else { c };
                    spread = v.fmul_m(cf(0x3F68_BA2F)).fadd_m(cf(0x3F4C_CCCD)).fmul_m(spread).to_f32();
                }
                scatter(&mut e, spread, [0x74_0254, 0x74_026B, 0x74_0282]);
            }
            let veh = rp(ped, 0x58C);
            let att = rp(ped, 0xFC);
            if ru(ped, 0x46C) & 0x100 != 0 && !veh.is_null() && rb(veh, 0x42D) & 4 == 0 {
                g::<GPtr<u8>>(IGNORE_ENTITY).write(veh);
            } else if !att.is_null() && etype(att) == 2 && rb(att, 0x42D) & 4 == 0 {
                g::<GPtr<u8>>(IGNORE_ENTITY).write(att);
            }
            if ped_is_player(ped) != 0 {
                g::<u8>(INCLUDE_CAR_TYRES).write(1);
            }
            g::<u8>(INCLUDE_DEAD_PEDS).write(1);
            shoot_line(fptr(&mut s), fptr(&mut e), cpp, hitp);
            break 'paths;
        }
        if !ped.is_null() && ped_is_player(ped) != 0 {
            let off = usize::from(g::<u8>(0xB6_F081).read()) * 0x238;
            let amode = g::<u16>(CAM0_MODE + off).read();
            if matches!(amode, 0x35 | 0x37 | 0x41 | 0x31) {
                // 3. player camera aim
                let dist = x(rf(info, 8)).fmul_m(3.0).to_f32();
                camera_find_3rd_person_cam_target_vector(g(THE_CAMERA), dist, s[0], s[1], s[2],
                    fptr(&mut cam_target), fptr(&mut e));
                let dz = x(e[2]).fsub_m(s[2]).to_f32();
                let dy = x(e[1]).fsub_m(s[1]).to_f32();
                let dx = x(e[0]).fsub_m(s[0]).to_f32();
                dir = [dx, dy, dz];
                normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut dir)));
                let pd = rp(ped, 0x480);
                if !is_zero(spread) {
                    spread = range_factor(info, Some(3.0))
                        .fmul_m(spread)
                        .fmul_m(rf(pd, 0x2C))
                        .fmul_m(g::<f32>(SPREAD_SCALE).read())
                        .to_f32();
                    let mode = g::<u16>(CAM0_MODE).read();
                    let mut out = [0f32; 3];
                    let (vs, vc) = if matches!(mode, 0x35 | 0x37 | 0x31) {
                        if mode == 0x31 {
                            let mut aim = [0f32; 3];
                            cam_get_two_player_aim_vector(g(CAM0), fptr(&mut aim));
                        }
                        let mut up: [f32; 3] = g::<[f32; 3]>(CAM0_UP).read();
                        let mut a: [f32; 3] =
                            cross_product(fptr(&mut out), g(CAM0_FRONT), fptr(&mut up)).cast::<[f32; 3]>().read();
                        normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut a)));
                        (a, up)
                    } else {
                        let mut zup = [0.0, 0.0, 1.0];
                        let mut a: [f32; 3] =
                            cross_product(fptr(&mut out), fptr(&mut e), fptr(&mut zup)).cast::<[f32; 3]>().read();
                        normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut a)));
                        let mut u: [f32; 3] =
                            cross_product(fptr(&mut out), fptr(&mut a), fptr(&mut e)).cast::<[f32; 3]>().read();
                        normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut u)));
                        (a, u)
                    };
                    sway(&mut e, vs, vc, spread);
                    add_recoil(pd, info);
                }
                let veh = rp(ped, 0x58C);
                let att = rp(ped, 0xFC);
                if ru(ped, 0x46C) & 0x100 != 0 && !veh.is_null() && rb(veh, 0x42D) & 4 == 0 {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(veh);
                } else if !att.is_null() && etype(att) == 2 {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(att);
                    if rb(att, 0x42D) & 4 != 0 {
                        g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
                    }
                } else {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
                }
                g::<u8>(INCLUDE_CAR_TYRES).write(1);
                g::<u8>(INCLUDE_BIKERS).write(1);
                g::<u8>(INCLUDE_DEAD_PEDS).write(1);
                shoot_line(fptr(&mut cam_target), fptr(&mut e), cpp, hitp);
                if hit.is_null() {
                    break 'paths;
                }
                let dy = x(rd_f(&cp, 1)).fsub_m(cam_target[1]);
                let dx = x(rd_f(&cp, 0)).fsub_m(cam_target[0]);
                let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
                let range = rf(info, 8);
                if lt_x(st0(target_weapon_range_multiplier_call(hit, ped).to_f64()).fmul_m(range), d) {
                    hit = GPtr::NULL;
                } else if !hit.is_null() {
                    check_for_shooting_vehicle_occupant_call(hitp, cpp, ty as i32, fptr(&mut cam_target), fptr(&mut e));
                }
                break 'paths;
            }
        }
        if etype(firer) == 2 {
            // 4. vehicle
            spread = cf(0x3F19_999A);
            ensure_matrix(firer);
            let fwd: [f32; 3] = rp(firer, 0x14).byte_add(0x10).cast::<[f32; 3]>().read();
            e = fwd;
            dir = fwd;
            let status = rb(firer, 0x36) >> 3;
            if status == 0 || status == 8 {
                extend(&mut e, s, rf(info, 8));
                let sub = ru(firer, 0x594);
                let flag = u32::from(sub == 4 || sub == 3);
                let owner = if status == 8 { find_player_ped(-1) } else { rp(firer, 0x460) };
                do_drive_by_auto_aiming(owner, firer, fptr(&mut s), fptr(&mut e), flag);
                e[0] = x(e[0]).fsub_m(s[0]).to_f32();
                e[1] = x(e[1]).fsub_m(s[1]).to_f32();
                e[2] = x(e[2]).fsub_m(s[2]).to_f32();
                normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut e)));
                let model = rh(firer, 0x22);
                spread = if matches!(model, 0x1BF | 0x1D5 | 0x234) { cf(0x3DCC_CCCD) } else { cf(0x3E99_999A) };
            }
            scatter(&mut e, spread, [0x74_090E, 0x74_0925, 0x74_093C]);
            normalise(GPtr::<u8>::cast::<[f32;3]>(fptr(&mut e)));
            g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
            extend(&mut e, s, rf(info, 8));
            vehicle_line = true;
        } else {
            // 5. other
            let range = rf(info, 8);
            ensure_matrix(firer);
            let mat = rp(firer, 0x14);
            let z = x(range).fmul_m(rf(mat, 0x18));
            let yv = x(range).fmul_m(rf(mat, 0x14)).to_f32();
            let xv = x(range).fmul_m(rf(mat, 0x10)).to_f32();
            let ez = z.fadd_m(m[2]);
            let ey = x(m[1]).fadd_m(yv);
            e = [x(xv).fadd_m(m[0]).to_f32(), ey.to_f32(), ez.to_f32()];
            ensure_matrix(firer);
            dir = rp(firer, 0x14).byte_add(0x10).cast::<[f32; 3]>().read();
            if ped.is_null() {
                g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
            } else {
                if ru(ped, 0x470) & 0x0200_0000 != 0 && (ped_is_player(ped) == 0 || rb(info, 0x18) & 1 == 0) {
                    do_doom_aiming(firer, fptr(&mut s), fptr(&mut e));
                }
                let veh = rp(ped, 0x58C);
                let att = rp(ped, 0xFC);
                if ru(ped, 0x46C) & 0x100 != 0 && !veh.is_null() {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(veh);
                } else if !att.is_null() {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(att);
                    if etype(att) != 2 {
                        g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
                    }
                } else {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(firer);
                }
            }
        }
        if !vehicle_line {
            g::<u8>(INCLUDE_DEAD_PEDS).write(1);
        }
        shoot_line(fptr(&mut s), fptr(&mut e), cpp, hitp);
    }

    // events
    let silenced = u32::from(matches!(ru(this, 0), 0x17 | 0x11));
    let mut shot = [0u32; EVENT_WORDS];
    let shotp: GPtr<u8> = lp(&mut shot).cast();
    event_gun_shot_ctor(shotp, firer, s[0], s[1], s[2], e[0], e[1], e[2], silenced);
    event_group_add(get_event_global_group(), shotp, 0);
    let silenced2 = u32::from(ru(this, 0) == 0x17);
    let mut whizz = [0u32; EVENT_WORDS];
    let whizzp: GPtr<u8> = lp(&mut whizz).cast();
    event_whizzed_by_ctor(whizzp, firer, fptr(&mut s), fptr(&mut e), silenced2);
    event_group_add(get_event_global_group(), whizzp, 0);
    interesting_events_add(g(INTERESTING_EVENTS), 0x16, firer);
    if !ped.is_null() {
        ped_is_player(ped);
    }
    s = origin.cast::<[f32; 3]>().read();

    // muzzle effects
    if muzzle_fx as u8 != 0 {
        let shell: Option<(u32, u32)> = match ru(this, 0) {
            0x16..=0x18 | 0x22 => Some((0x3E4C_CCCD, 0x3E80_0000)),
            0x19..=0x1B => Some((0x3E99_999A, 0x3EE6_6666)),
            0x1C | 0x1D | 0x20 => Some((0x3E4C_CCCD, 0x3E99_999A)),
            0x1E | 0x1F | 0x26 => {
                let n = ftol32(x(rf(info, 0x44)).fsub_m(rf(info, 0x40)).fmul_m(cf(0x4461_0000)));
                if n >= 0x32 {
                    Some((0x3F26_6666, 0x3E80_0000))
                } else {
                    let t = g::<u8>(FLASH_TOGGLE);
                    let v = t.read().wrapping_add(1);
                    t.write(v);
                    if v & 1 != 0 { None } else { Some((0x3F26_6666, 0x3E80_0000)) }
                }
            }
            _ => None,
        };
        if let Some((off, size)) = shell {
            let off = cf(off);
            point_lights_add_light(0, m[0], m[1], m[2], 0.0, 0.0, 0.0, 3.0, 0.25, cf(0x3E61_47AE), 0.0, 0, 0,
                GPtr::NULL);
            let flash = if etype(firer) == 3 && !rp(firer, 0x4F8).is_null() { 0 } else { 1 };
            fx_trigger_gunshot(g(G_FX), firer, fptr(&mut m), fptr(&mut dir), flash);
            let mut pos = [
                x(dir[0]).fmul_m(off).fsubr_m(m[0]).to_f32(),
                x(dir[1]).fmul_m(off).fsubr_m(m[1]).to_f32(),
                x(dir[2]).fmul_m(off).fsubr_m(m[2]).to_f32(),
            ];
            ensure_matrix(firer);
            let mat = rp(firer, 0x14);
            let mut right = [rf(mat, 0), rf(mat, 4)];
            add_gunshell_call(this, firer, fptr(&mut pos), lp(&mut right).cast(), cf(size));
        }
    }

    // water
    let water = if !target_ent.is_null() {
        let t = etype(target_ent);
        (2..=4).contains(&t) && ru(target_ent, 0x40) & 0x100 != 0
    } else {
        (!ped.is_null() && ped_is_player(ped) != 0 && lt(e[2], s[2])) || {
            let st = rb(firer, 0x36) >> 3;
            (st == 0 || st == 8) && lt(e[2], s[2])
        }
    };
    if water {
        let mut w = [0f32; 3];
        let splash = if !hit.is_null() {
            test_line_against_water(s[0], s[1], s[2], rd_f(&cp, 0), rd_f(&cp, 1), rd_f(&cp, 2), fptr(&mut w)) != 0
        } else {
            test_line_against_water(s[0], s[1], s[2], e[0], e[1], e[2], fptr(&mut w)) != 0
        };
        if splash {
            fx_trigger_bullet_splash(g(G_FX), fptr(&mut w));
            audio_report_bullet_hit(g(AUDIO_ENGINE), GPtr::NULL, 0x27, fptr(&mut w), 0.0);
        }
    }

    // impact
    let piece_b = (cp[9] & 0xFF) as u8;
    let pellets = !x(g::<f32>(PELLET_SPREAD).read()).fcom(x(0.0)).test_any(0x41)
        && !hit.is_null()
        && !(etype(hit) == 2 && (0xD..=0x10).contains(&piece_b));
    if pellets {
        let mut pass = 0i32;
        loop {
            pass += 1;
            let n = if ru(this, 0) == 0x1B { g::<i32>(PELLETS_SPAS).read() } else { g::<i32>(PELLETS).read() };
            let mut mat = [0u32; MATRIX_WORDS];
            let matp: GPtr<u8> = lp(&mut mat).cast();
            set_up_pellet_col(this, n, firer, hit, fptr(&mut s), cpp, matp);
            let mut dists = [1f32; 15];
            let col = if etype(hit) == 3 {
                let slot = (MODEL_INFOS as u32).wrapping_add((i32::from(rh(hit, 0x22) as i16) * 4) as u32);
                ped_mi_animate_col_model_skinned(GPtr::<u8>::cast::<sa_stream::shared::CBaseModelInfo>(g::<GPtr<u8>>(slot as usize).read()), rp(hit, 0x18))
            } else {
                entity_get_col_model(hit)
            };
            ensure_matrix(hit);
            collision_process_col_models(matp, g(PELLET_COL), rp(hit, 0x14), col, g(PELLET_POINTS), g(PELLET_POINTS),
                lp(&mut dists).cast(), 0);
            // the original reads past its 15 floats for larger counts (none in the data: 15 and 8)
            let dist = |i: i32| dists.get(i as usize).copied().unwrap_or(1.0);
            let mut hits = 0i32;
            let mut last = 0i32;
            for i in 0..n {
                if lt(dist(i), 1.0) {
                    hits += 1;
                    last = i;
                }
            }
            for i in 0..n {
                if lt(dist(i), 1.0) {
                    let pt: GPtr<u8> = g(PELLET_POINTS + i as usize * 0x2C);
                    let inc = if i == last { hits.wrapping_neg() } else { 1 };
                    do_bullet_impact(this, firer, hit, fptr(&mut m), pt, pt, inc);
                }
            }
            let t = etype(hit);
            if t != 3 && t != 2 {
                do_bullet_impact(this, firer, hit, fptr(&mut m), fptr(&mut e), cpp, 0);
                hit = GPtr::NULL;
            } else {
                let full = !x(dist(0)).fcom(x(1.0)).test_parity(0x44);
                if (full || lt_x(xi(hits).fdiv(xi(n)), 0.5)) && pass < 2 {
                    g::<GPtr<u8>>(IGNORE_ENTITY).write(hit);
                    s = [rd_f(&cp, 0), rd_f(&cp, 1), rd_f(&cp, 2)];
                    hit = GPtr::NULL;
                    shoot_line(fptr(&mut s), fptr(&mut e), cpp, hitp);
                } else {
                    hit = GPtr::NULL;
                }
            }
            matrix_dtor(matp);
            if hit.is_null() {
                break;
            }
        }
    } else {
        do_bullet_impact(this, firer, hit, fptr(&mut m), fptr(&mut e), cpp, 0);
    }
    world_reset_line_test_options();
    event_whizzed_by_dtor(whizzp);
    event_gun_shot_dtor(shotp);
    1
}

/// Word `i` of the col point as a float (0..2: the point).
fn rd_f(cp: &[u32; COL_POINT_WORDS], i: usize) -> f32 {
    f32::from_bits(cp[i])
}

use sa_pop::carctrl4::lt_k as lt_x;
use crate::projectile_add::force_is_zero as is_zero;
/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FIRE_INSTANT_HIT];
