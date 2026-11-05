//! FMT-2 whole-install check of the data definition files (read-only). Ignored by default:
//!
//! ```text
//! SA_GAME_DIR=/path/to/gta-sa cargo test -p sa-formats --release --test game_data_defs -- --ignored --nocapture
//! ```
//!
//! * Every file under `data/` plus `stream.ini` is looked up with `sa_formats::data::parser_for`
//!   and parsed. Files without a parser must be listed in [`NOT_PARSED`] with the reason (the
//!   inventory: not loaded by the exe, or not a data definition file handled by FMT-2).
//! * Every `.ipl` (binary) and `nodes*.dat` entry inside the IMG archives is parsed and
//!   written back; the bytes must be identical. The loose `data/Paths/NODES*.DAT` copies too.
//! * Every level file `IDE`/`IPL` path must exist in the install (case-insensitive).
//! * With `SA_DUMP_DIR` set, the canonical dump of every parsed file is written there (one
//!   `.txt` per file), for FMT-2 part b.

use sa_formats::data::nodes::PathNodes;
use sa_formats::data::{DataFile, gta_dat, ipl_bin::BinaryIpl, parser_for};
use sa_formats::img::ImgDirectory;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Files under `data/` (lower-case relative paths, prefix match for directories) that no FMT-2
/// parser covers, with the reason. Each entry must match at least one file.
const NOT_PARSED: &[(&str, &str)] = &[
    ("data/script/", "SCM / script.img: FMT-3"),
    (
        "data/paths/carrec.img",
        "IMG archive of .rrr car recordings: FMT-3 (directory checked by FMT-1)",
    ),
    (
        "data/icons/",
        "Windows icons for the installer/shortcuts, not read by gta_sa.exe",
    ),
    (
        "data/decision/",
        "decision makers outside allowed/: not loaded (the exe only references data\\decision\\ for \
         PedEvent.txt and data\\decision\\allowed\\); older layout, the loader's 80-field format reads 21",
    ),
    (
        "data/audioevents.txt",
        "not loaded: no file-name string in gta_sa.exe (audio tool list)",
    ),
    (
        "data/paths/spath0.dat",
        "not loaded: no file-name string in gta_sa.exe",
    ),
    (
        "data/paths/train.dat",
        "not loaded: no file-name string in gta_sa.exe (tracks*.dat are used)",
    ),
    (
        "data/paths/train2.dat",
        "not loaded: no file-name string in gta_sa.exe (tracks*.dat are used)",
    ),
    (
        "data/animviewer.dat",
        "not loaded: no file-name string in gta_sa.exe (anim viewer tool)",
    ),
    (
        "data/main.sc",
        "not loaded: no file-name string in gta_sa.exe (script source stub)",
    ),
    (
        "data/numplate.dat",
        "not loaded: no file-name string in gta_sa.exe",
    ),
    (
        "data/polydensity.dat",
        "not loaded: no file-name string in gta_sa.exe (binary tool data)",
    ),
];

fn game_dir() -> PathBuf {
    let dir = std::env::var_os("SA_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wine/gta-sa"));
    assert!(
        dir.join("data").is_dir(),
        "game data not found at {} (set SA_GAME_DIR)",
        dir.display()
    );
    dir
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[derive(Default)]
struct Report {
    counts: BTreeMap<String, u64>,
    failures: Vec<String>,
    notes: BTreeMap<String, Vec<String>>,
    files_by_kind: BTreeMap<String, u64>,
    not_parsed: Vec<(String, &'static str)>,
    dump_dir: Option<PathBuf>,
}

impl Report {
    fn add(&mut self, label: &str, f: &dyn DataFile) {
        for (k, v) in f.counts() {
            *self.counts.entry(k).or_default() += v;
        }
        let n = f.notes();
        if !n.is_empty() {
            self.notes.insert(label.to_string(), n);
        }
        if let Some(d) = &self.dump_dir {
            let name = label.replace(['/', ':'], "_");
            std::fs::write(d.join(format!("{name}.txt")), f.dump()).unwrap();
        }
    }
}

fn kind_of(rel: &str) -> String {
    let name = rel.rsplit('/').next().unwrap();
    match name.rsplit_once('.') {
        Some((_, e)) => format!(".{e}"),
        None => name.to_string(),
    }
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn data_files() {
    let root = game_dir();
    let mut rep = Report {
        dump_dir: std::env::var_os("SA_DUMP_DIR").map(PathBuf::from),
        ..Default::default()
    };
    if let Some(d) = &rep.dump_dir {
        std::fs::create_dir_all(d).unwrap();
    }
    let mut files = Vec::new();
    walk(&root.join("data"), &mut files);
    files.push(root.join("stream.ini"));
    let mut all_rel: Vec<String> = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        all_rel.push(rel.clone());
        let Some(parser) = parser_for(&rel) else {
            match NOT_PARSED.iter().find(|(p, _)| rel.starts_with(p)) {
                Some((_, why)) => rep.not_parsed.push((rel, why)),
                None => rep
                    .failures
                    .push(format!("{rel}: no parser and not listed in NOT_PARSED")),
            }
            continue;
        };
        let data = std::fs::read(path).unwrap();
        *rep.files_by_kind.entry(kind_of(&rel)).or_default() += 1;
        match parser(&data) {
            Ok(f) => rep.add(&rel, f.as_ref()),
            Err(e) => {
                rep.failures.push(format!("{rel}: {e}"));
                continue;
            }
        }
        // Binary formats must round-trip.
        if rel == "data/paths/roadblox.dat" {
            let r = sa_formats::data::misc::roadblox::RoadBlox::parse(&data).unwrap();
            if r.to_bytes() != data {
                rep.failures.push(format!("{rel}: roadblox round trip differs"));
            } else {
                *rep.counts
                    .entry("roadblox.dat: byte-exact round trip".into())
                    .or_default() += 1;
            }
        }
        if rel.starts_with("data/paths/nodes") {
            let n = PathNodes::parse(&data).unwrap();
            if n.to_bytes() != data {
                rep.failures.push(format!("{rel}: nodes round trip differs"));
            } else {
                *rep.counts
                    .entry("nodes: loose files byte-exact".into())
                    .or_default() += 1;
            }
        }
    }
    for (p, _) in NOT_PARSED {
        if !all_rel.iter().any(|r| r.starts_with(p)) {
            rep.failures.push(format!("NOT_PARSED entry {p} matches no file"));
        }
    }

    // Level files: every IDE/IPL they name must exist.
    for lvl in ["data/default.dat", "data/gta.dat"] {
        let f = gta_dat::LevelFile::parse(&std::fs::read(root.join(lvl)).unwrap());
        for d in [gta_dat::Directive::Ide, gta_dat::Directive::Ipl] {
            for p in f.paths(d) {
                let want = String::from_utf8_lossy(p).replace('\\', "/").to_lowercase();
                if !all_rel.contains(&want) {
                    rep.failures.push(format!("{lvl}: {} {want} not found", d.name()));
                } else {
                    *rep.counts
                        .entry(format!("level: {} paths found", d.name()))
                        .or_default() += 1;
                }
            }
        }
    }

    // IMG entries.
    let mut imgs = Vec::new();
    walk(&root, &mut imgs);
    imgs.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("img")));
    for img in &imgs {
        let irel = img.strip_prefix(&root).unwrap().to_string_lossy().to_lowercase();
        let mut f = File::open(img).unwrap();
        let mut prefix = [0u8; 8];
        f.read_exact(&mut prefix).unwrap();
        let hlen = ImgDirectory::header_len_from_prefix(&prefix).unwrap();
        let mut head = vec![0u8; hlen];
        f.seek(SeekFrom::Start(0)).unwrap();
        f.read_exact(&mut head).unwrap();
        let dir = ImgDirectory::parse(&head).unwrap();
        for e in &dir.entries {
            let name = e.name_lossy().to_lowercase();
            let key = format!("img/{name}");
            let Some(parser) = parser_for(&key) else { continue };
            let mut data = vec![0u8; e.byte_len()];
            f.seek(SeekFrom::Start(e.byte_offset() as u64)).unwrap();
            f.read_exact(&mut data).unwrap();
            let label = format!("{irel}:{name}");
            *rep.files_by_kind
                .entry(format!("img {}", kind_of(&name)))
                .or_default() += 1;
            match parser(&data) {
                Ok(p) => rep.add(&label, p.as_ref()),
                Err(err) => {
                    rep.failures.push(format!("{label}: {err}"));
                    continue;
                }
            }
            let back = if name.ends_with(".ipl") {
                BinaryIpl::parse(&data).unwrap().to_bytes()
            } else {
                PathNodes::parse(&data).unwrap().to_bytes()
            };
            if back != data {
                rep.failures.push(format!("{label}: round trip differs"));
            } else {
                *rep.counts
                    .entry(format!("img {}: byte-exact round trips", kind_of(&name)))
                    .or_default() += 1;
            }
        }
    }

    println!("== FMT-2 data files ({}) ==", root.display());
    println!("-- files parsed, by kind --");
    for (k, v) in &rep.files_by_kind {
        println!("{v:>10}  {k}");
    }
    println!("-- record counts --");
    for (k, v) in &rep.counts {
        println!("{v:>10}  {k}");
    }
    println!("-- files without a parser ({}) --", rep.not_parsed.len());
    for (r, why) in &rep.not_parsed {
        println!("  {r}: {why}");
    }
    let total_notes: usize = rep.notes.values().map(Vec::len).sum();
    println!(
        "-- loader notes: {total_notes} in {} files (how the exe reads unusual lines) --",
        rep.notes.len()
    );
    for (f, n) in &rep.notes {
        println!("  {f}: {} notes", n.len());
        for x in n.iter().take(6) {
            println!("      {x}");
        }
    }
    println!("-- failures ({}) --", rep.failures.len());
    for f in &rep.failures {
        println!("  {f}");
    }
    assert!(rep.failures.is_empty(), "{} failures", rep.failures.len());
}
