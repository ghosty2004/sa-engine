//! WRITER-WORLD-2 (batch `world6-1`): `CWorld::FindGroundZForCoord`, whose body continues in the no-CD
//! protection chunk 0x401BC5. Decoded by hand from the exe image: the chunk pushes `ror([0x15606C0],
//! [0x407AD0] & 31)` = `ror(0xF4000188, 9)` = 0xC47A0000 (-1000.0, the line length) and jumps back
//! to 0x569694. 0x407AD0 (the entry of `CStreaming::AreAnimsUsedByRequestedModels`) already has a
//! keycells reader for this very instruction (0x401BCE), so the decoded value stays the same when
//! that entry is hooked.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::x;

orig! {
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings, bool
    /// vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(pos: GPtr<u8>, len: f32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
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
        1, 0, 0, 0, 1, 0, 0,
    );
    if hit == 0 {
        x(20.0)
    } else {
        x(cp[2])
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIND_GROUND_Z_FOR_COORD];
