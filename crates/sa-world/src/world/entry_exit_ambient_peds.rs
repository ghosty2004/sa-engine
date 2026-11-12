//! WRITER-WORLD-2 (batch `world6-3`): `CEntryExit::GenerateAmbientPeds`. Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use super::common::{g, rd, x};

orig! {
    /// `CPopulation::CreateWaitingCoppers(CVector, float)` (0x6133F0)
    fn create_waiting_coppers(pos: CVector, angle: f32) = cdecl 0x6133F0 => unported;
    /// `CPopulation::PopulateInterior(int32, CVector)` (0x616470)
    fn populate_interior(area: i32, pos: CVector) = cdecl 0x616470 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
}

/// `CEntryExit::GenerateAmbientPeds(const CVector& pos)` (0x43E8B0): clears the police-station
/// flag (0xC0FCB6). Outside (area 0): the interior area variable 0x8D253C = 0 and waiting coppers
/// at `pos` facing the active entry's heading (+0x2C of [0x96A7C0], degrees to radians). Inside: the
/// flag is set when this entry is named POLICE1..4 (case-insensitive), 0x8D253C = the area of the
/// linked exit (+0x38) or of this entry (+0x36), and the interior is populated at `pos`.
#[hook(addr = 0x43E8B0, conv = thiscall, name = "CEntryExit::GenerateAmbientPeds", this = "CEntryExit",
       expect = "A1 14 29 B7 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn generate_ambient_peds(this: GPtr<u8>, pos: GPtr<u8>) {
    let curr_area = g::<i32>(0xB7_2914).read();
    g::<u8>(0xC0_FCB6).write(0);
    let p = CVector::new(rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
    if curr_area == 0 {
        g::<i32>(0x8D_253C).write(0);
        let active = g::<GPtr<u8>>(0x96_A7C0).read();
        let angle = x(rd::<f32>(active, 0x2C)).fmul_m(f32::from_bits(0x3C8E_FA35)).to_f32();
        create_waiting_coppers(p, angle);
        return;
    }
    if [0x85_95E4usize, 0x85_95DC, 0x85_95D4, 0x85_95CC]
        .iter()
        .any(|&s| crt_stricmp(this, GPtr::from_addr(s)) == 0)
    {
        g::<u8>(0xC0_FCB6).write(1);
    }
    let link = rd::<GPtr<u8>>(this, 0x38);
    let area = i32::from(rd::<i8>(if link.is_null() { this } else { link }, 0x36));
    g::<i32>(0x8D_253C).write(area);
    populate_interior(area, p);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GENERATE_AMBIENT_PEDS];
