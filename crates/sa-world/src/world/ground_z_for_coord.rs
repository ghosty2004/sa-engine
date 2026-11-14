//! WRITER-WORLD-2 (batches `world6-1`, `world6-3`): the vertical-line height queries.
//! `CWorld::FindGroundZForCoord`'s body continues in the no-CD protection chunk 0x401BC5. Decoded
//! by hand from the exe image: the chunk pushes `ror([0x15606C0], [0x407AD0] & 31)` = `ror(0xF4000188, 9)` = 0xC47A0000 (-1000.0, the line length) and jumps back
//! to 0x569694. 0x407AD0 (the entry of `CStreaming::AreAnimsUsedByRequestedModels`) already has a
//! keycells reader for this very instruction (0x401BCE), so the decoded value stays the same when
//! that entry is hooked.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::x;

use crate::bindings::world_process_vertical_line as process_vertical_line;
orig! {
    /// The CRT `printf`-style debug print (0x821982), the doubles as dword pairs
    #[allow(clippy::too_many_arguments)]
    fn debug_print(fmt: u32, x0: u32, x1: u32, y0: u32, y1: u32, z0: u32, z1: u32) = cdecl 0x821982 => unported;
}

/// A vertical line from `pos` of length `len` against buildings and dummies: the hit's z.
fn vertical_hit(pos: [f32; 3], len: f32) -> Option<f32> {
    let mut pos = pos;
    let mut cp = [0f32; 11];
    let mut ent: u32 = 0;
    let hit = process_vertical_line(
        GPtr::from_raw(&mut pos as *mut [f32; 3]).cast(),
        len,
        GPtr::from_raw(&mut cp as *mut [f32; 11]).cast(),
        GPtr::from_raw(&mut ent as *mut u32).cast(),
        1, 0, 0, 0, 1, 0, GPtr::NULL,
    );
    if hit == 0 { None } else { Some(cp[2]) }
}

/// `CWorld::FindLowestZForCoord(float x, float y)` (0x5697F0): a vertical line from `(x, y, -1000)`
/// of length 1000 against buildings and dummies; the hit's z, else 20.0.
#[hook(addr = 0x5697F0, conv = cdecl, name = "CWorld::FindLowestZForCoord",
       expect = "83 EC 38 8B 44 24 3C",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn find_lowest_z_for_coord(px: f32, py: f32) -> X87 {
    x(vertical_hit([px, py, -1000.0], 1000.0).unwrap_or(20.0))
}

/// `CWorld::FindRoofZFor3DCoord(float x, float y, float z, bool* found)` (0x569750): a vertical line
/// from `(x, y, z)` of length 1000 (upwards) against buildings and dummies: the hit's z with
/// `*found = true`; else 20.0 with `*found = false`, or, without `found`, after printing "THERE IS NO
/// MAP BELOW THE FOLLOWING COORS:%f %f %f" (0x864FA0) with the three coordinates.
#[hook(addr = 0x569750, conv = cdecl, name = "CWorld::FindRoofZFor3DCoord",
       expect = "83 EC 38 8B 44 24 3C",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn find_roof_z_for_3d_coord(px: f32, py: f32, pz: f32, found: GPtr<u8>) -> X87 {
    match vertical_hit([px, py, pz], 1000.0) {
        Some(z) => {
            if !found.is_null() {
                found.write(1);
            }
            x(z)
        }
        None => {
            if found.is_null() {
                let d = |v: f32| {
                    let b = f64::from(v).to_bits();
                    (b as u32, (b >> 32) as u32)
                };
                let ((x0, x1), (y0, y1), (z0, z1)) = (d(px), d(py), d(pz));
                debug_print(0x86_4FA0, x0, x1, y0, y1, z0, z1);
            } else {
                found.write(0);
            }
            x(20.0)
        }
    }
}

/// `CWorld::FindGroundZForCoord(float x, float y)` (0x569660): a vertical line from `(x, y, 1000)`
/// of length -1000 against buildings and dummies; the hit's z, else 20.0 (0x858BA4).
#[hook(addr = 0x569660, conv = cdecl, name = "CWorld::FindGroundZForCoord",
       expect = "83 EC 38 8B 44 24 3C",
       unverified = "world6-1", route = "L2", hazards = "none",
       note = "the line length -1000.0 is pushed by protection chunk 0x401BC5 (decoded from the exe image)")]
pub fn find_ground_z_for_coord(px: f32, py: f32) -> X87 {
    let mut pos = [px, py, 1000.0];
    let mut cp = [0f32; 11];
    let mut ent: u32 = 0;
    let hit = process_vertical_line(
        GPtr::from_raw(&mut pos as *mut [f32; 3]).cast(),
        f32::from_bits(0xC47A_0000),
        GPtr::from_raw(&mut cp as *mut [f32; 11]).cast(),
        GPtr::from_raw(&mut ent as *mut u32).cast(),
        1, 0, 0, 0, 1, 0, GPtr::NULL,
    );
    if hit == 0 {
        x(20.0)
    } else {
        x(cp[2])
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIND_GROUND_Z_FOR_COORD, HOOK_FIND_LOWEST_Z_FOR_COORD, HOOK_FIND_ROOF_Z_FOR_3D_COORD];
