//! The plane route planner's terrain probes (FRONTIER-2 lane C turn 3, batch `frontier2-c7`): unnamed in the
//! catalogue and in gta-reversed, called only by `CCarCtrl::FlyAIPlaneInCertainDirection`'s heading search (carctrl6).
//! Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::rd;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::bindings::plane_probe_line;
use sa_core::bindings::vector_normalise;
use sa_world::bindings::process_line_of_sight;
use sa_world::world::common::entity_pos;

/// The plane's line probe (0x422E10, cdecl, AL; unnamed): from `pos + moveSpeed * 50` (x and y from the f32-rounded
/// products, z from the x87 one) along the heading's unit vector (`cos`, `sin`, 0, `CVector::Normalise`d) tilted by
/// `pitch` (`dir * cos(pitch) + (0, 0, sin(pitch))`, the x part through `0 * sin(pitch)` as the original adds it) times
/// 200; `CWorld::ProcessLineOfSight` against buildings only (shoot-through on). A hit stores the colpoint's z into
/// `*out` (fld / fstp) and returns 1; else 0.
#[hook(addr = 0x422E10, conv = cdecl, name = "FUN_00422e10", expect = "83 EC 78 8B 44 24 7C",
       unverified = "frontier2-c7", route = "L2", hazards = "x87-transcendental")]
pub fn plane_probe_line_port(v: GPtr<u8>, pitch: f32, heading: f32, out: GPtr<f32>) -> u8 {
    let ms = |k: isize| X87::from_f32(rd::<f32>(v, 0x44 + 4 * k)).fmul_m(50.0);
    let (sx, sy, sz) = (ms(0).to_f32(), ms(1).to_f32(), ms(2));
    let pos = entity_pos(v);
    let z = sz.fadd_m(rd::<f32>(pos, 8));
    let y = X87::from_f32(sy).fadd_m(rd::<f32>(pos, 4));
    let x = X87::from_f32(sx).fadd_m(rd::<f32>(pos, 0));
    let mut start = [x.to_f32(), y.to_f32(), z.to_f32()];
    let mut dir = [X87::from_f32(heading).fcos().to_f32(), X87::from_f32(heading).fsin().to_f32(), 0.0f32];
    vector_normalise(GPtr::from_raw(&raw mut dir));
    let s = X87::from_f32(pitch).fsin();
    let zs = X87::zero().fmul(s);
    let (zs32, s32) = (zs.to_f32(), s.to_f32());
    let c = X87::from_f32(pitch).fcos();
    let xc = X87::from_f32(dir[0]).fmul(c);
    let yc = X87::from_f32(dir[1]).fmul(c).to_f32();
    let zc = c.fmul_m(dir[2]).to_f32();
    let sum_x = xc.fadd_m(zs32).to_f32();
    let sum_y = X87::from_f32(yc).fadd(zs);
    let sum_z = X87::from_f32(zc).fadd_m(s32);
    let ex = X87::from_f32(sum_x).fmul_m(200.0).to_f32();
    let ey = sum_y.fmul_m(200.0).to_f32();
    let ez = sum_z.fmul_m(200.0);
    let mut end = [
        X87::from_f32(ex).fadd_m(start[0]).to_f32(),
        X87::from_f32(ey).fadd_m(start[1]).to_f32(),
        ez.fadd_m(start[2]).to_f32(),
    ];
    let mut cp = [0u32; 11];
    let mut ent = 0u32;
    let hit = process_line_of_sight(
        GPtr::from_raw(&raw mut start).cast(),
        GPtr::from_raw(&raw mut end).cast(),
        GPtr::from_raw(&raw mut cp).cast(),
        GPtr::from_raw(&raw mut ent).cast(),
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
    );
    if hit == 0 {
        return 0;
    }
    out.write(sa_math::fpu::fld_fst(f32::from_bits(cp[2])));
    1
}

/// The plane route planner's terrain probe (0x422F80, cdecl, ST0; unnamed): [`plane_probe_line_port`] along
/// `heading` at each of the six pitches of the table 0x8A5B2C (6, 3, 0, -20, -40, -60 degrees, read as dwords); a hit
/// of the first returns 100000.0 at once; otherwise the highest hit z (`fcomp`: replaced when not below the best, NaN
/// included), 0.0 when nothing hit.
#[hook(addr = 0x422F80, conv = cdecl, name = "FUN_00422f80", expect = "83 EC 08 53 8B 5C 24 10",
       unverified = "frontier2-c7", route = "L2", hazards = "x87-transcendental")]
pub fn plane_probe_heading(v: GPtr<u8>, heading: f32) -> X87 {
    let mut best = 0.0f32;
    for i in 0..6usize {
        let mut tmp = 0.0f32;
        let pitch = GPtr::<f32>::from_addr(0x8A5B2C + 4 * i).read();
        if plane_probe_line(v, pitch, heading, GPtr::from_raw(&raw mut tmp)) == 0 {
            continue;
        }
        if i == 0 {
            return X87::from_f32(100000.0);
        }
        if X87::from_f32(best).fcom(X87::from_f32(tmp)).test_any(0x41) {
            best = tmp;
        }
    }
    X87::from_f32(best)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PLANE_PROBE_LINE_PORT, HOOK_PLANE_PROBE_HEADING];
