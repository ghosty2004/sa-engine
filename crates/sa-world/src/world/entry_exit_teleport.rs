//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `CEntryExit::FindValidTeleportPoint`. Ported
//! from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;
use sa_math::x87::X87;

use super::common::{g, rd, wr, x, xi};

use sa_core::bindings::world_get_is_line_of_sight_clear as get_is_line_of_sight_clear;
use crate::bindings::world_test_sphere_against_world as test_sphere_against_world;

/// 0.35 (0x3EB33333): the test sphere radius.
const RADIUS: u32 = 0x3EB3_3333;

/// A sphere of radius 0.35 at `p` touches buildings, vehicles, peds, objects or dummies.
fn blocked(p: [u32; 3]) -> bool {
    !test_sphere_against_world(f32::from_bits(p[0]), f32::from_bits(p[1]), f32::from_bits(p[2]), f32::from_bits(RADIUS), GPtr::NULL, 1, 1, 1, 1, 1, 0).is_null()
}

/// `CEntryExit::FindValidTeleportPoint(CVector* out)` (0x43EAF0): `out` = the exit position of the
/// current spawn point (0x96A7C0, +0x20); when a 0.35 sphere there is blocked, 8 points on a circle
/// of radius 1.25 (angles `i * 2pi / 8`, same z) and then 8 on radius 2 are tried: the first free
/// one with a clear line of sight from `out` (buildings, vehicles, objects, dummies) replaces it.
#[hook(addr = 0x43EAF0, conv = thiscall, name = "CEntryExit::FindValidTeleportPoint", this = "CEntryExit",
       expect = "83 EC 0C A1 C0 A7 96 00",
       unverified = "world3-2", route = "L2", hazards = "x87-transcendental",
       note = "fsin/fcos of the angles; the original keeps the loop index in its argument slot")]
pub fn find_valid_teleport_point(_this: GPtr<u8>, out: GPtr<u8>) {
    let sp = g::<GPtr<u8>>(0x96_A7C0).read();
    wr::<u32>(out, 0, rd::<u32>(sp, 0x20));
    wr::<f32>(out, 4, x(rd::<f32>(sp, 0x24)).to_f32());
    wr::<f32>(out, 8, x(rd::<f32>(sp, 0x28)).to_f32());
    let cur = [rd::<u32>(out, 0), rd::<u32>(out, 4), rd::<u32>(out, 8)];
    if !blocked(cur) {
        return;
    }
    for ring in 0..2 {
        for i in 0..8 {
            let o = [rd::<f32>(out, 0), rd::<f32>(out, 4)];
            let z = rd::<u32>(out, 8);
            let a = xi(i).fmul_m(f32::from_bits(0x40C9_0FDB)).fmul_m(0.125);
            let scale = |v: X87| if ring == 0 { v.fmul_m(1.25) } else { v.fadd(v) };
            let cx = scale(a.fcos()).fadd_m(o[0]).to_f32();
            let cy = scale(a.fsin()).fadd_m(o[1]).to_f32();
            let mut cand = [cx.to_bits(), cy.to_bits(), z];
            if blocked(cand) {
                continue;
            }
            let cp: GPtr<u8> = GPtr::from_raw(&mut cand as *mut [u32; 3]).cast();
            if get_is_line_of_sight_clear(out, cp, 1, 1, 0, 1, 1, 0, 0) != 0 {
                for (k, v) in cand.iter().enumerate() {
                    wr::<u32>(out, 4 * k as isize, *v);
                }
                return;
            }
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIND_VALID_TELEPORT_POINT];
