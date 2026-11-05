//! Function extents and names from `catalog/procedures.csv` (in-repo, addresses and names only).
//!
//! A function's extent is [start, next catalogued start). The catalogue merges Ghidra's function list
//! with gta-reversed hook addresses, so the next start is the closest known function entry; padding
//! (`nop`/`int3`) between functions falls inside the extent, which is harmless for the allow-list.

use std::collections::BTreeMap;

pub struct Catalog {
    /// start -> name (gta-reversed name, else Ghidra name)
    starts: BTreeMap<u32, String>,
    /// start -> body ranges from `catalog/extents.csv` (CAT-7: chunks outside [start, next start), such
    /// as `.HOODLUM` bodies behind a stub and tails the compiler moved elsewhere)
    chunks: BTreeMap<u32, Vec<(u32, u32)>>,
}

impl Catalog {
    pub fn load() -> Result<Catalog, String> {
        let path = crate::workspace_root().join("catalog/procedures.csv");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut c = Catalog::parse(&text)?;
        if let Ok(ext) = std::fs::read_to_string(crate::workspace_root().join("catalog/extents.csv")) {
            c.chunks = parse_extents(&ext)?;
        }
        Ok(c)
    }

    pub fn parse(text: &str) -> Result<Catalog, String> {
        let mut starts = BTreeMap::new();
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().ok_or("empty catalogue")?.split(',').collect();
        let col = |n: &str| {
            header
                .iter()
                .position(|h| *h == n)
                .ok_or(format!("no column {n}"))
        };
        let (ca, cg, ch) = (col("address")?, col("gr_name")?, col("ghidra_name")?);
        for l in lines {
            let f: Vec<&str> = l.split(',').collect();
            let Some(a) = f
                .get(ca)
                .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            else {
                continue;
            };
            let gr = f.get(cg).copied().unwrap_or("");
            let name = if gr.is_empty() {
                f.get(ch).copied().unwrap_or("")
            } else {
                gr
            };
            starts.insert(a, name.to_string());
        }
        Ok(Catalog {
            starts,
            chunks: BTreeMap::new(),
        })
    }

    /// [start, end) of the function starting at `addr`.
    pub fn extent(&self, addr: u32) -> Option<(u32, u32)> {
        self.starts.get(&addr)?;
        let end = self.starts.range(addr + 1..).next().map(|(a, _)| *a)?;
        Some((addr, end))
    }

    /// Every code range of the function at `addr`: [start, next start) plus the body ranges of
    /// `catalog/extents.csv`.
    pub fn ranges(&self, addr: u32) -> Option<Vec<(u32, u32)>> {
        let mut r = vec![self.extent(addr)?];
        for &x in self.chunks.get(&addr).into_iter().flatten() {
            if !r.contains(&x) {
                r.push(x);
            }
        }
        Some(r)
    }

    /// Name of the function containing `addr` (with offset when not at its start).
    pub fn describe(&self, addr: u32) -> String {
        match self.starts.range(..=addr).next_back() {
            Some((s, n)) if *s == addr => format!("{n} ({addr:#x})"),
            Some((s, n)) => format!("{n}+{:#x} ({addr:#x})", addr - s),
            None => format!("{addr:#x}"),
        }
    }
}

fn hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

/// `address` -> `ranges` ("lo-hi;lo-hi") of `catalog/extents.csv`.
fn parse_extents(text: &str) -> Result<BTreeMap<u32, Vec<(u32, u32)>>, String> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or("empty extents.csv")?.split(',').collect();
    let col = |n: &str| {
        header
            .iter()
            .position(|h| *h == n)
            .ok_or(format!("extents.csv: no column {n}"))
    };
    let (ca, cr) = (col("address")?, col("ranges")?);
    let mut out = BTreeMap::new();
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        let (Some(a), Some(rs)) = (f.get(ca).and_then(|s| hex(s)), f.get(cr)) else {
            continue;
        };
        let v: Vec<(u32, u32)> = rs
            .split(';')
            .filter_map(|r| {
                let (lo, hi) = r.split_once('-')?;
                Some((hex(lo)?, hex(hi)?))
            })
            .collect();
        if !v.is_empty() {
            out.insert(a, v);
        }
    }
    Ok(out)
}
