//! Test helper: maps gta_sa.exe 1.0 US sections by virtual address. The exe is never in the repo; it is
//! looked up in `$SA_GTA_EXE`, `<repo>/re/gta/1.0-US/gta_sa.exe`, then `~/sa-engine/re/...` (worktrees).
//! Tests that need it are skipped (with a note) when it is missing.

use std::path::PathBuf;

pub struct Exe {
    sections: Vec<(u32, Vec<u8>)>,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

pub fn load() -> Option<Exe> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("SA_GTA_EXE") {
        cands.push(p.into());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    cands.push(root.join("re/gta/1.0-US/gta_sa.exe"));
    if let Ok(h) = std::env::var("HOME") {
        cands.push(PathBuf::from(h).join("sa-engine/re/gta/1.0-US/gta_sa.exe"));
    }
    let Some(d) = cands.iter().find_map(|p| std::fs::read(p).ok()) else {
        eprintln!("note: gta_sa.exe 1.0 US not found, exe-based checks skipped");
        return None;
    };
    let pe = u32_at(&d, 0x3C) as usize;
    let nsec = u16_at(&d, pe + 6) as usize;
    let opt = u16_at(&d, pe + 20) as usize;
    let base = u32_at(&d, pe + 24 + 28);
    let mut sections = Vec::new();
    for s in 0..nsec {
        let h = pe + 24 + opt + s * 40;
        let (vsize, va, rsize, raw) = (
            u32_at(&d, h + 8),
            u32_at(&d, h + 12),
            u32_at(&d, h + 16),
            u32_at(&d, h + 20),
        );
        let mut bytes = vec![0u8; vsize.max(rsize) as usize];
        let n = (rsize as usize).min(d.len().saturating_sub(raw as usize));
        bytes[..n].copy_from_slice(&d[raw as usize..raw as usize + n]);
        sections.push((base + va, bytes));
    }
    Some(Exe { sections })
}

impl Exe {
    /// Up to `n` bytes at `addr` (fewer at the end of a section, empty outside the image).
    pub fn bytes(&self, addr: u32, n: usize) -> &[u8] {
        for (va, b) in &self.sections {
            if addr >= *va && ((addr - va) as usize) < b.len() {
                let o = (addr - va) as usize;
                return &b[o..(o + n).min(b.len())];
            }
        }
        &[]
    }

    /// The sections that hold code: game `.text` (0x401000), the no-CD `.text` and `.HOODLUM` (by
    /// address, as listed in CLAUDE.md).
    pub fn code_sections(&self) -> impl Iterator<Item = (u32, &[u8])> {
        self.sections
            .iter()
            .filter(|(va, _)| [0x401000, 0xCB1000, 0x1556000].contains(va))
            .map(|(va, b)| (*va, b.as_slice()))
    }
}
