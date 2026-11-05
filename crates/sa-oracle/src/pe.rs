//! Minimal PE32 loader for `gta_sa.exe` 1.0 US: sections at the preferred base, imports, and a flat
//! image whose IAT slots point at trap addresses (so a call through the IAT lands in a known, unexecutable
//! place and is reported with the import's name).

use std::path::{Path, PathBuf};

/// Every IAT slot `i` is patched to `TRAP_BASE + TRAP_STRIDE * i`.
pub const TRAP_BASE: u32 = 0x0002_0000;
pub const TRAP_STRIDE: u32 = 16;

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub va: u32,
    pub vsize: u32,
    pub writable: bool,
    pub exec: bool,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub iat: u32,
    pub trap: u32,
    pub name: String,
}

#[derive(Clone)]
pub struct Image {
    pub base: u32,
    pub sections: Vec<Section>,
    pub imports: Vec<Import>,
    /// SizeOfImage bytes as loaded at `base`, IAT patched.
    pub flat: Vec<u8>,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Locates the exe: `$SA_GTA_EXE`, `<workspace>/re/gta/1.0-US/gta_sa.exe`, then
/// `~/sa-engine/re/gta/1.0-US/gta_sa.exe` (worktrees have no `re/`).
pub fn find_exe() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("SA_GTA_EXE") {
        cands.push(p.into());
    }
    cands.push(crate::workspace_root().join("re/gta/1.0-US/gta_sa.exe"));
    if let Ok(h) = std::env::var("HOME") {
        cands.push(PathBuf::from(h).join("sa-engine/re/gta/1.0-US/gta_sa.exe"));
    }
    cands.into_iter().find(|p| p.is_file())
}

impl Image {
    pub fn load(path: &Path) -> Result<Image, String> {
        let d = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Image::parse(&d)
    }

    pub fn parse(d: &[u8]) -> Result<Image, String> {
        if d.len() < 0x40 || &d[..2] != b"MZ" {
            return Err("not an MZ file".into());
        }
        let pe = u32_at(d, 0x3C) as usize;
        if &d[pe..pe + 4] != b"PE\0\0" || u16_at(d, pe + 4) != 0x14C {
            return Err("not a PE32 i386 image".into());
        }
        let nsec = u16_at(d, pe + 6) as usize;
        let opt = pe + 24;
        let opt_size = u16_at(d, pe + 20) as usize;
        let base = u32_at(d, opt + 28);
        let size = u32_at(d, opt + 56);
        let mut flat = vec![0u8; size as usize];
        let hdr = (u32_at(d, opt + 60) as usize).min(d.len()).min(flat.len());
        flat[..hdr].copy_from_slice(&d[..hdr]);
        let mut sections = Vec::new();
        for s in 0..nsec {
            let h = opt + opt_size + s * 40;
            let name = String::from_utf8_lossy(&d[h..h + 8])
                .trim_end_matches('\0')
                .to_string();
            let (vsize, va, rsize, raw, ch) = (
                u32_at(d, h + 8),
                u32_at(d, h + 12),
                u32_at(d, h + 16),
                u32_at(d, h + 20),
                u32_at(d, h + 36),
            );
            let n = (rsize.min(vsize.max(rsize)) as usize).min(d.len().saturating_sub(raw as usize));
            let n = n.min(flat.len().saturating_sub(va as usize));
            flat[va as usize..va as usize + n].copy_from_slice(&d[raw as usize..raw as usize + n]);
            sections.push(Section {
                name,
                va: base + va,
                vsize: vsize.max(rsize),
                writable: ch & 0x8000_0000 != 0,
                exec: ch & 0x2000_0000 != 0,
            });
        }
        let mut img = Image {
            base,
            sections,
            imports: Vec::new(),
            flat,
        };
        let (imp_rva, _imp_size) = (u32_at(d, opt + 96 + 8), u32_at(d, opt + 96 + 12));
        if imp_rva != 0 {
            img.parse_imports(imp_rva)?;
        }
        Ok(img)
    }

    fn rd32(&self, rva: u32) -> u32 {
        u32_at(&self.flat, rva as usize)
    }

    fn cstr(&self, rva: u32) -> String {
        let s = &self.flat[rva as usize..];
        let n = s.iter().position(|&b| b == 0).unwrap_or(0);
        String::from_utf8_lossy(&s[..n]).into_owned()
    }

    fn parse_imports(&mut self, mut desc: u32) -> Result<(), String> {
        let mut idx = 0u32;
        loop {
            let (oft, name, ft) = (self.rd32(desc), self.rd32(desc + 12), self.rd32(desc + 16));
            if name == 0 && ft == 0 {
                break;
            }
            let dll = self.cstr(name);
            let mut i = 0;
            loop {
                let lookup = if oft != 0 {
                    self.rd32(oft + 4 * i)
                } else {
                    self.rd32(ft + 4 * i)
                };
                if lookup == 0 {
                    break;
                }
                let func = if lookup & 0x8000_0000 != 0 {
                    format!("#{}", lookup & 0xFFFF)
                } else {
                    self.cstr(lookup + 2)
                };
                let iat_rva = ft + 4 * i;
                let trap = TRAP_BASE + TRAP_STRIDE * idx;
                self.flat[iat_rva as usize..iat_rva as usize + 4].copy_from_slice(&trap.to_le_bytes());
                self.imports.push(Import {
                    iat: self.base + iat_rva,
                    trap,
                    name: format!("{dll}!{func}"),
                });
                idx += 1;
                i += 1;
            }
            desc += 20;
        }
        Ok(())
    }

    pub fn end(&self) -> u32 {
        self.base + self.flat.len() as u32
    }

    /// Bytes at a virtual address (empty outside the image).
    pub fn bytes(&self, addr: u32, n: usize) -> &[u8] {
        if addr < self.base || addr >= self.end() {
            return &[];
        }
        let o = (addr - self.base) as usize;
        &self.flat[o..(o + n).min(self.flat.len())]
    }

    pub fn u32(&self, addr: u32) -> Option<u32> {
        let b = self.bytes(addr, 4);
        (b.len() == 4).then(|| u32_at(b, 0))
    }

    pub fn import_at_trap(&self, addr: u32) -> Option<&Import> {
        if addr < TRAP_BASE {
            return None;
        }
        let i = ((addr - TRAP_BASE) / TRAP_STRIDE) as usize;
        self.imports.get(i)
    }

    pub fn trap_end(&self) -> u32 {
        TRAP_BASE + TRAP_STRIDE * self.imports.len() as u32
    }

    pub fn section_of(&self, addr: u32) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| addr >= s.va && addr < s.va + s.vsize)
    }
}
