//! `CIniFile::LoadIniFile` (0x56D070): the ped/car density multipliers from `gta3.ini`.
//!
//! Ported from the 1.0 US bytes. Each multiplier is read with `sscanf(gString, "%f", &m)` and clamped:
//! `if (0.5 > m) m = 0.5; else if (3.0 < m) m = 3.0;` (x87 compares against the f32 in memory), so a
//! NaN is kept (gta-reversed's `if (m >= 0.5) … else m = 0.5` turns NaN into 0.5). Afterwards, with or
//! without the file, `CPopulation::MaxNumberOfPedsInUse = 25` and `CCarCtrl::MaxNumberOfCarsInUse = 12`.

use crate::mem::GPtr;
use crate::{globals, hook, orig};
use sa_math::x87::{Cmp, X87};

globals! {
    list GLOBALS;
    /// `CIniFile::PedNumberMultiplier`
    pub static PED_NUMBER_MULTIPLIER @ 0x8CDF14: f32 = 1.0;
    /// `CIniFile::CarNumberMultiplier`
    pub static CAR_NUMBER_MULTIPLIER @ 0x8CDF18: f32 = 1.0;
    /// `CPopulation::MaxNumberOfPedsInUse`
    pub static MAX_NUMBER_OF_PEDS_IN_USE @ 0x8D2538: u32 = 25;
    /// `CCarCtrl::MaxNumberOfCarsInUse`
    pub static MAX_NUMBER_OF_CARS_IN_USE @ 0x8A5B24: u32 = 12;
}

/// `gString` (0xB71670, 352 bytes)
pub const G_STRING: u32 = 0xB71670;
/// `""` (0x858B54)
pub const EMPTY: u32 = 0x858B54;
/// `"r"` (0x85A53C)
pub const MODE_R: u32 = 0x85A53C;
/// `"gta3.ini"` (0x864FFC)
pub const GTA3_INI: u32 = 0x864FFC;
/// `"%f"` (0x862EF0)
pub const FMT_F: u32 = 0x862EF0;

orig! {
    /// `CFileMgr::SetDir` (0x5387D0)
    pub fn set_dir(path: GPtr<u8>) -> i32 = cdecl 0x5387D0 => crate::filemgr::set_dir;
    /// `CFileMgr::OpenFile` (0x538900)
    pub fn open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => crate::filemgr::open_file;
    /// `CFileMgr::ReadLine` (0x5389B0)
    pub fn read_line(f: GPtr<u8>, buf: GPtr<u8>, n: i32) -> bool = cdecl 0x5389B0 => crate::filemgr::read_line;
    /// `CFileMgr::CloseFile` (0x5389D0)
    pub fn close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => crate::filemgr::close_file;
    /// CRT `sscanf` (0x8220AD), the one-output form used here
    pub fn sscanf(s: GPtr<u8>, fmt: GPtr<u8>, out: GPtr<f32>) -> i32 = cdecl 0x8220AD => unported;
}

/// The clamp applied after each `sscanf`.
pub fn clamp_multiplier(m: f32) -> f32 {
    let mx = X87::from_f32(m);
    // fld 0.5; fcomp m; test ah, 41h; jne -> second test
    if X87::constant(0x3F00_0000).fcom(mx) == Cmp::Greater {
        return f32::from_bits(0x3F00_0000);
    }
    // fld 3.0; fcomp m; test ah, 5; jp -> keep
    if !X87::constant(0x4040_0000).fcom(mx).test_parity(5) {
        return f32::from_bits(0x4040_0000);
    }
    m
}

fn read_multiplier(f: GPtr<u8>, g: &crate::mem::Global<f32>) {
    let buf = GPtr::<u8>::from_addr(G_STRING as usize);
    read_line(f, buf, 200);
    sscanf(buf, GPtr::from_addr(FMT_F as usize), g.ptr());
    let v = g.get();
    let c = clamp_multiplier(v);
    if c.to_bits() != v.to_bits() {
        g.set(c);
    }
}

/// `CIniFile::LoadIniFile()` (0x56D070, cdecl).
#[hook(addr = 0x56D070, conv = cdecl, name = "CIniFile::LoadIniFile", expect = "56 68 54 8B 85 00")]
pub fn load_ini_file() {
    set_dir(GPtr::from_addr(EMPTY as usize));
    let f = open_file(GPtr::from_addr(GTA3_INI as usize), GPtr::from_addr(MODE_R as usize));
    if !f.is_null() {
        read_multiplier(f, &PED_NUMBER_MULTIPLIER);
        read_multiplier(f, &CAR_NUMBER_MULTIPLIER);
        close_file(f);
    }
    MAX_NUMBER_OF_PEDS_IN_USE.set(25);
    MAX_NUMBER_OF_CARS_IN_USE.set(12);
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[HOOK_LOAD_INI_FILE];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn clamp() {
        assert_eq!(clamp_multiplier(0.1), 0.5);
        assert_eq!(clamp_multiplier(0.5), 0.5);
        assert_eq!(clamp_multiplier(2.0), 2.0);
        assert_eq!(clamp_multiplier(3.0), 3.0);
        assert_eq!(clamp_multiplier(7.0), 3.0);
        assert!(clamp_multiplier(f32::NAN).is_nan());
        assert_eq!(clamp_multiplier(f32::NEG_INFINITY), 0.5);
    }
}
