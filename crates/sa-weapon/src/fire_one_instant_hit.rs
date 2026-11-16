//! FRONTIER-2 turn 8, lane B (batch `frontier2-b19`): `FireOneInstantHitRound` (0x73AF00), the heli / plane
//! machine-gun round. Ported from the 1.0 US bytes with gta-reversed (Weapon.cpp `FireOneInstantHitRound`) as a
//! reading aid; unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::{rd3f as v3, v3_ptr as vp};
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{f32_of, ge_or_nan, x, xi};

use crate::bindings::{audio_report_bullet_hit, generate_damage_event};
use crate::weapon_impact::ci_asin;
use sa_core::bindings::vector_normalise;
use sa_world::bindings::{
    add_animation, bullet_traces_add_trace, add_light, get_local_direction, get_water_level, process_line_of_sight, vehicle_inflict_damage,
};
use sa_world::world::common::{entity_pos, rd, wr};

/// `AudioEngine` (CAudioEngine, 0xB6BC90).
const AUDIO_ENGINE: usize = 0xB6_BC90;
/// 0.22 (the light's green, pushed by the protection fragment 0x4015A0 as `~[0x1560654] ^ [0x499692]`, evaluated
/// with tools/re/vm_eval.py: 0x3E6147AE), 0.25, 3.0, 0.02, 10.0 (0x85862C), 57.29578 (0x859878).
const K_GREEN: u32 = 0x3E61_47AE;
const K_RED: u32 = 0x3E80_0000;
const K_RADIUS: u32 = 0x4040_0000;
const K_TRACE_RADIUS: u32 = 0x3CA3_D70A;
const K_RAD2DEG: u32 = 0x4265_2EE0;
/// `WEAPON_UZI_DRIVEBY` (0x34) for peds, `WEAPON_MICRO_UZI` (0x1C) for vehicles, `SURFACE_WATER_SHALLOW` (0x27).
const WEAPON_PED: i32 = 0x34;
const WEAPON_VEHICLE: i32 = 0x1C;
const SURFACE_WATER_SHALLOW: u32 = 0x27;

/// `FireOneInstantHitRound(const CVector& start, const CVector& end, int32 intensity)` (0x73AF00, cdecl). A point
/// light (3.0, 0.25 / 0.22 / 0) at `start`; `ProcessLineOfSight(start, end)` (everything but see-through and camera
/// ignore; the hit entity starts null); a bullet trace (0.02, 750 ms, alpha 150) to the hit point or `end`. A ped
/// that is not dying / dead (state 0x36 / 0x37) plays the shot animation `0x1C + GetLocalDirection(start - pos)`
/// (the direction vector's z slot holds pos.x: only x / y are read) with blend amount 0 and delta 8.0 (not in
/// gta-reversed) and gets `GenerateDamageEvent(ped, null, 0x34, intensity, cp.pieceTypeB, dir)`; a vehicle gets
/// `InflictDamage(null, 0x1C, (float)intensity, 0)`. Any hit then reports the bullet hit when `dot = normalise(end
/// - start) . cp.normal` (`(nz*dz + ny*dy) + nx*dx`, compared in the register) is below 0, at `asin(-f32(dot)) *
/// 57.29578`; no hit: a water level under `end.z + 10` reports a shallow-water hit at `(end.x, end.y, level)`.
#[hook(addr = 0x73AF00, conv = cdecl, name = "CWeapon::FireOneInstantHitRound", expect = "83 EC 44 53 55 56 8B 74 24 54",
       unverified = "frontier2-b19", route = "L2", hazards = "x87-transcendental",
       note = "_CIasin through sa_core::orig::x87_unary; the light's green 0.22 comes from the protection fragment 0x4015A0 (vm_eval.py)")]
pub fn fire_one_instant_hit_round(start: GPtr<u8>, end: GPtr<u8>, intensity: i32) {
    let s = v3(start);
    add_light(
        0,
        s[0],
        s[1],
        s[2],
        0.0,
        0.0,
        0.0,
        f32::from_bits(K_RADIUS),
        f32::from_bits(K_RED),
        f32::from_bits(K_GREEN),
        0.0,
        0,
        0,
        GPtr::NULL,
    );
    let mut hit: GPtr<u8> = GPtr::NULL;
    let mut cp = [0u32; 11];
    let cpp = GPtr::<u32>::from_raw(cp.as_mut_ptr()).cast::<u8>();
    process_line_of_sight(start, end, cpp, GPtr::from_raw(&mut hit).cast(), 1, 1, 1, 1, 1, 1, 0, 0);
    bullet_traces_add_trace(start.cast(), if hit.is_null() { end } else { cpp }.cast(), K_TRACE_RADIUS, 750, 150);
    if hit.is_null() {
        let e = v3(end);
        let mut level = 0f32;
        let z = f32_of(x(e[2]).fadd_m(10.0));
        if get_water_level(e[0], e[1], z, GPtr::from_raw(&mut level).cast(), 1, GPtr::NULL) != 0 {
            let mut at = [e[0], e[1], level];
            audio_report_bullet_hit(GPtr::from_addr(AUDIO_ENGINE), GPtr::NULL, SURFACE_WATER_SHALLOW, vp(&mut at), 0.0);
        }
        return;
    }
    match rd::<u8>(hit, 0x36) & 7 {
        3 => {
            let state = rd::<i32>(hit, 0x530);
            if state != 0x36 && state != 0x37 {
                let p = entity_pos(hit);
                let px = rd::<f32>(p, 0);
                let mut d = [f32_of(x(s[0]).fsub_m(px)), f32_of(x(s[1]).fsub_m(rd::<f32>(p, 4))), px];
                let dir = get_local_direction(hit, GPtr::from_raw(d.as_mut_ptr()));
                let assoc = add_animation(rd::<GPtr<u8>>(hit, 0x18), 0, (dir + 0x1C) as u32);
                wr::<u32>(assoc, 0x18, 0);
                wr::<u32>(assoc, 0x1C, 0x4100_0000);
                generate_damage_event(hit, GPtr::NULL, WEAPON_PED, intensity, i32::from(rd::<u8>(cpp, 0x24)), dir as u32);
            }
        }
        2 => {
            let dmg = f32_of(xi(intensity));
            vehicle_inflict_damage(hit, GPtr::NULL, WEAPON_VEHICLE, dmg, 0.0, 0.0, 0.0);
        }
        _ => {}
    }
    let e = v3(end);
    let mut d = [f32_of(x(e[0]).fsub_m(s[0])), f32_of(x(e[1]).fsub_m(s[1])), f32_of(x(e[2]).fsub_m(s[2]))];
    vector_normalise(GPtr::from_raw(&mut d));
    let n = [rd::<f32>(cpp, 0x10), rd::<f32>(cpp, 0x14), rd::<f32>(cpp, 0x18)];
    let dot = x(n[2]).fmul_m(d[2]).fadd(x(n[1]).fmul_m(d[1])).fadd(x(n[0]).fmul_m(d[0]));
    if ge_or_nan(dot, x(0.0)) {
        return;
    }
    let angle = f32_of(ci_asin(x(f32_of(dot)).fchs()).fmul_m(f32::from_bits(K_RAD2DEG)));
    audio_report_bullet_hit(GPtr::from_addr(AUDIO_ENGINE), hit, u32::from(rd::<u8>(cpp, 0x23)), cpp, angle);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FIRE_ONE_INSTANT_HIT_ROUND];
