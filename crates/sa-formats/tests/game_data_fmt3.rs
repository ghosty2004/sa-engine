//! FMT-3 whole-install checks against the user's game data (read-only). Ignored by default; run
//! with
//!
//! ```text
//! SA_GAME_DIR=/path/to/gta-sa cargo test -p sa-formats --release --test game_data_fmt3 -- --ignored --nocapture
//! ```
//!
//! Without `SA_GAME_DIR` the test uses `<workspace>/wine/gta-sa`. One test per format: IFP
//! (`anim/ped.ifp`, `anim/anim.img`, `anim/cuts.img`), GXT (`text/*.gxt`), SCM (`main.scm`,
//! `script.img`, plus the opcode usage table; `SA_SCRIPT_USAGE=<path>` writes
//! `catalog/script-usage.csv`), vehicle recordings (`carrec.img`) and save files (none in the
//! install; `SA_SAVE_DIR` may point at a directory with `GTASAsf*.b` files).

use sa_formats::carrec::Recording;
use sa_formats::gxt::{Gxt, key_hash};
use sa_formats::ifp::{Ifp, ifp_notes};
use sa_formats::img::ImgDirectory;
use sa_formats::save::SaveFile;
use sa_formats::scm::{self, MainScm, ScriptSet};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn game_dir() -> PathBuf {
    let dir = std::env::var_os("SA_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wine/gta-sa"));
    assert!(
        dir.join("models").is_dir(),
        "game data not found at {} (set SA_GAME_DIR)",
        dir.display()
    );
    dir
}

/// Reads every entry of an IMG archive: `(name, data)` (data padded to whole sectors).
fn img_entries(path: &Path) -> Vec<(String, Vec<u8>)> {
    let data = std::fs::read(path).unwrap();
    let dir = ImgDirectory::parse(&data).unwrap();
    dir.entries
        .iter()
        .map(|e| (e.name_lossy(), dir.entry_data(&data, e).unwrap().to_vec()))
        .collect()
}

#[derive(Default)]
struct Report {
    failures: Vec<String>,
    counts: BTreeMap<String, u64>,
    notes: Vec<String>,
}

impl Report {
    fn count(&mut self, key: impl Into<String>, n: u64) {
        *self.counts.entry(key.into()).or_default() += n;
    }

    fn fail(&mut self, name: &str, msg: impl std::fmt::Display) {
        self.failures.push(format!("{name}: {msg}"));
    }

    fn finish(self, title: &str) {
        println!("== {title} ==");
        for (k, v) in &self.counts {
            println!("{v:>10}  {k}");
        }
        if !self.notes.is_empty() {
            println!("-- notes ({}) --", self.notes.len());
            for n in self.notes.iter().take(80) {
                println!("{n}");
            }
        }
        println!("-- unexpected failures ({}) --", self.failures.len());
        for f in self.failures.iter().take(50) {
            println!("{f}");
        }
        assert!(
            self.failures.is_empty(),
            "{} unexpected failures",
            self.failures.len()
        );
    }
}

fn check_ifp(rep: &mut Report, label: &str, data: &[u8], padded: bool) {
    let ifp = match Ifp::parse(data) {
        Ok(i) => i,
        Err(e) => return rep.fail(label, e),
    };
    if ifp.to_bytes() != data {
        return rep.fail(label, "round trip differs");
    }
    let kind = match &ifp {
        Ifp::Anp(p) if p.anp3 => "ANP3",
        Ifp::Anp(_) => "ANP2",
        Ifp::Anpk(_) => "ANPK",
    };
    rep.count(format!("ifp {kind}: files parsed + byte-exact"), 1);
    rep.count(format!("ifp {kind}: animations"), ifp.num_animations() as u64);
    rep.count(format!("ifp {kind}: sequences"), ifp.num_sequences() as u64);
    rep.count(format!("ifp {kind}: key frames"), ifp.num_key_frames() as u64);
    let trailing = match &ifp {
        Ifp::Anp(p) => &p.trailing,
        Ifp::Anpk(p) => &p.trailing,
    };
    if !trailing.is_empty() {
        if !padded {
            rep.fail(
                label,
                format!("{} trailing bytes in a loose file", trailing.len()),
            );
        } else if trailing.iter().any(|&b| b != 0) {
            rep.fail(label, format!("{} trailing bytes, not all zero", trailing.len()));
        } else {
            rep.count(
                format!("ifp {kind}: files with zero sector padding after the data"),
                1,
            );
        }
    }
    for (k, n) in ifp_notes(&ifp) {
        rep.count(format!("ifp {kind}: {k}"), n);
    }
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn ifp_whole_install() {
    let root = game_dir();
    let mut rep = Report::default();
    let ped = std::fs::read(root.join("anim/ped.ifp")).unwrap();
    check_ifp(&mut rep, "anim/ped.ifp", &ped, false);
    for img in ["anim/anim.img", "anim/cuts.img"] {
        for (name, data) in img_entries(&root.join(img)) {
            if name.to_ascii_lowercase().ends_with(".ifp") {
                check_ifp(&mut rep, &format!("{img}:{name}"), &data, true);
            }
        }
    }
    rep.finish("FMT-3 IFP");
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn gxt_whole_install() {
    let root = game_dir();
    let mut rep = Report::default();
    let mut names: Vec<_> = std::fs::read_dir(root.join("text"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gxt")))
        .collect();
    names.sort();
    for path in names {
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        let data = std::fs::read(&path).unwrap();
        let gxt = match Gxt::parse(&data) {
            Ok(g) => g,
            Err(e) => {
                rep.fail(&label, e);
                continue;
            }
        };
        if gxt.to_bytes() != data {
            rep.fail(&label, "round trip differs");
            continue;
        }
        rep.count("gxt: files parsed + byte-exact", 1);
        rep.count(format!("gxt: files with {}-bit characters", gxt.char_bits), 1);
        rep.count("gxt: tables (TABL entries)", gxt.tabl.len() as u64);
        let mut keys = 0;
        for t in &gxt.tables {
            keys += t.keys.len() as u64;
            match t.check() {
                Ok(n) => rep.count("gxt: strings decoded", n as u64),
                Err(e) => rep.fail(&format!("{label}:{}", t.name_lossy()), e),
            }
        }
        rep.count("gxt: keys", keys);
        for n in gxt.notes() {
            if n.contains(" zero gap bytes before table") {
                rep.count("gxt: zero gaps (1-3 bytes) before mission tables", 1);
            } else {
                rep.notes.push(format!("{label}: {n}"));
            }
        }
        for t in &gxt.tables[1..] {
            if t.start % 4 == 0 {
                rep.count("gxt: mission tables starting at a multiple of 4", 1);
            } else {
                rep.fail(
                    &label,
                    format!("table {} at unaligned {:#x}", t.name_lossy(), t.start),
                );
            }
        }
        // Keys of known strings must hash to entries of the main table.
        let main = &gxt.tables[0];
        for k in ["CDERROR", "FEM_MM", "FESZ_CA", "IE23"] {
            match main.get(k) {
                Some(_) => rep.count("gxt: known keys found via key_hash", 1),
                None => rep.fail(
                    &label,
                    format!("key {k} ({:#010x}) not in MAIN", key_hash(k.as_bytes())),
                ),
            }
        }
    }
    rep.finish("FMT-3 GXT");
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn scm_whole_install() {
    let root = game_dir();
    let mut rep = Report::default();
    let main = std::fs::read(root.join("data/script/main.scm")).unwrap();
    let scm = match MainScm::parse(&main) {
        Ok(s) => s,
        Err(e) => {
            rep.fail("main.scm", e);
            return rep.finish("FMT-3 SCM");
        }
    };
    if scm.to_bytes() != main {
        rep.fail("main.scm", "round trip differs");
    } else {
        rep.count("scm: main.scm header segments + code byte-exact", 1);
    }
    for (k, v) in scm.counts() {
        rep.count(format!("scm: {k}"), v);
    }
    let streamed = img_entries(&root.join("data/script/script.img"));
    rep.count("scm: script.img entries", streamed.len() as u64);
    let set = ScriptSet::new(&scm, &streamed);
    for n in set.header_notes() {
        rep.notes.push(n);
    }
    let decoded = set.decode_all();
    for (k, v) in decoded.counts() {
        rep.count(format!("scm decode: {k}"), v);
    }
    for f in decoded.failures() {
        rep.fail("scm decode", f);
    }
    for n in decoded.notes() {
        if n.contains("jumps into main.scm") {
            rep.count(
                "scm decode: mission/streamed instructions jumping into main.scm code",
                1,
            );
        } else if n.contains("tie broken by the majority list") {
            rep.count(
                "scm decode: ambiguous occurrences resolved by the majority list",
                1,
            );
        } else {
            rep.notes.push(n);
        }
    }
    if let Some(out) = std::env::var_os("SA_SCRIPT_USAGE") {
        std::fs::write(&out, scm::usage_csv(&decoded)).unwrap();
        println!("wrote {}", PathBuf::from(out).display());
    }
    rep.finish("FMT-3 SCM");
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn carrec_whole_install() {
    let root = game_dir();
    let mut rep = Report::default();
    for (name, data) in img_entries(&root.join("data/Paths/carrec.img")) {
        let label = format!("carrec.img:{name}");
        let rec = match Recording::parse(&data) {
            Ok(r) => r,
            Err(e) => {
                rep.fail(&label, e);
                continue;
            }
        };
        if rec.to_bytes() != data {
            rep.fail(&label, "round trip differs");
            continue;
        }
        rep.count("carrec: entries parsed + byte-exact", 1);
        rep.count("carrec: frames", rec.frames.len() as u64);
        for (k, v) in rec.notes() {
            rep.count(format!("carrec: {k}"), v);
        }
    }
    rep.finish("FMT-3 carrec");
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn saves_whole_install() {
    let mut rep = Report::default();
    let dir = std::env::var_os("SA_SAVE_DIR").map(PathBuf::from);
    let mut files = Vec::new();
    if let Some(d) = &dir {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            let n = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
            if n.starts_with("gtasasf") && n.ends_with(".b") {
                files.push(p);
            }
        }
    }
    files.sort();
    for p in &files {
        let label = p.display().to_string();
        let data = std::fs::read(p).unwrap();
        match SaveFile::parse(&data) {
            Ok(s) => {
                if s.to_bytes() != data {
                    rep.fail(&label, "round trip differs");
                    continue;
                }
                rep.count("save: files parsed + byte-exact", 1);
                rep.count("save: blocks", s.blocks.len() as u64);
                if s.checksum_ok() {
                    rep.count("save: checksum matches", 1);
                } else {
                    rep.fail(&label, "checksum mismatch");
                }
            }
            Err(e) => rep.fail(&label, e),
        }
    }
    rep.count("save: files found (SA_SAVE_DIR)", files.len() as u64);
    rep.finish("FMT-3 saves");
}
