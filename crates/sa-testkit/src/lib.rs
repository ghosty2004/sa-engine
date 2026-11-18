//! Test support shared by the port crates' `hooks.rs` registration tests (ADR-003 §3.2: one definition instead
//! of a copy in every crate). A dev-dependency only; host tests (Rust-owned memory).

use std::path::Path;

use sa_core::mem::GlobalDesc;

/// Initial bytes of a virtual address range in the exe image (`.bss` beyond the raw data = 0).
pub fn exe_image_bytes(exe: &[u8], va: u32, n: usize) -> Option<Vec<u8>> {
    let u16_at = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]) as usize;
    let u32_at = |o: usize| u32::from_le_bytes(exe[o..o + 4].try_into().unwrap());
    let pe = u32_at(0x3C) as usize;
    let (nsec, opt) = (u16_at(pe + 6), u16_at(pe + 20));
    let base = u32_at(pe + 24 + 28);
    for s in 0..nsec {
        let h = pe + 24 + opt + s * 40;
        let (vsize, va0, rsize, raw) = (u32_at(h + 8), base + u32_at(h + 12), u32_at(h + 16), u32_at(h + 20));
        if va >= va0 && va + n as u32 <= va0 + vsize.max(rsize) {
            return Some(
                (0..n as u32)
                    .map(|k| {
                        let o = va + k - va0;
                        if o < rsize { exe[(raw + o) as usize] } else { 0 }
                    })
                    .collect(),
            );
        }
    }
    None
}

/// The exe's size of a global: pointer-sized elements are 4 bytes there, possibly 8 on a 64-bit host. Globals
/// made of pointers are checked as all-zero (their exe image must be zero too).
pub fn exe_size(g: &GlobalDesc) -> usize {
    let p = core::mem::size_of::<usize>();
    if p == 8 && g.size.is_multiple_of(8) && g.init.iter().all(|b| *b == 0) { g.size / 2 } else { g.size }
}

/// The original image (`$SA_GTA_EXE` or `~/sa-engine/re/gta/1.0-US/gta_sa.exe`), if present.
pub fn exe() -> Option<Vec<u8>> {
    let path = std::env::var("SA_GTA_EXE").map(std::path::PathBuf::from).unwrap_or_else(|_| {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("sa-engine/re/gta/1.0-US/gta_sa.exe")
    });
    std::fs::read(path).ok()
}

/// A constant a port carries in place of bytes of the original image equals them (checked when the exe is
/// available).
pub fn check_image_bytes(va: u32, bytes: &[u8]) {
    let Some(exe) = exe() else {
        eprintln!("note: gta_sa.exe not found, {va:#x} not checked");
        return;
    };
    let img = exe_image_bytes(&exe, va, bytes.len()).unwrap_or_else(|| panic!("{va:#x} not in the image"));
    assert_eq!(img, bytes, "{va:#x}");
}

/// The dword of the original image at `va`, if the exe is available.
pub fn image_u32(va: u32) -> Option<u32> {
    let b = exe_image_bytes(&exe()?, va, 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Phase B/C storage starts with the original image's initial values (checked when the exe is available).
pub fn check_global_initial_values(globals: &[&[GlobalDesc]]) {
    let Some(exe) = exe() else {
        eprintln!("note: gta_sa.exe not found, initial values not checked");
        return;
    };
    for g in globals.iter().flat_map(|l| l.iter()) {
        let n = exe_size(g);
        let img = exe_image_bytes(&exe, g.addr, n).unwrap_or_else(|| panic!("{} not in the image", g.name));
        if n == g.size {
            assert_eq!(&img[..], &g.init[..n], "{} @ {:#x}", g.name, g.addr);
        } else {
            assert!(img.iter().all(|b| *b == 0), "{} @ {:#x}", g.name, g.addr);
        }
    }
}

/// The `#[hook(` registrations in the `.rs` files under `dir` (its `hooks.rs` excluded).
pub fn count_hooks(dir: &Path) -> usize {
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            n += count_hooks(&p);
        } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().unwrap() != "hooks.rs" {
            let s = std::fs::read_to_string(&p).unwrap();
            n += s.lines().filter(|l| l.trim_start().starts_with("#[hook(")).count();
        }
    }
    n
}
