//! Function extents and names from `catalog/procedures.csv` (in-repo, addresses and names only).
//!
//! A function's extent is [start, next catalogued start). The catalogue merges Ghidra's function list
//! with gta-reversed hook addresses, so the next start is the closest known function entry; padding
//! (`nop`/`int3`) between functions falls inside the extent, which is harmless for the allow-list.

use std::collections::BTreeMap;

pub struct Catalog {
    /// start -> name (gta-reversed name, else Ghidra name)
    starts: BTreeMap<u32, String>,
}

impl Catalog {
    pub fn load() -> Result<Catalog, String> {
        let path = crate::workspace_root().join("catalog/procedures.csv");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Catalog::parse(&text)
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
        Ok(Catalog { starts })
    }

    /// [start, end) of the function starting at `addr`.
    pub fn extent(&self, addr: u32) -> Option<(u32, u32)> {
        self.starts.get(&addr)?;
        let end = self.starts.range(addr + 1..).next().map(|(a, _)| *a)?;
        Some((addr, end))
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
