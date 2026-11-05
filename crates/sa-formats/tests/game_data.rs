//! Whole-install checks against the user's game data (read-only). Ignored by default; run with
//!
//! ```text
//! SA_GAME_DIR=/path/to/gta-sa cargo test -p sa-formats --release --test game_data -- --ignored --nocapture
//! ```
//!
//! Without `SA_GAME_DIR` the test uses `<workspace>/wine/gta-sa`.
//!
//! Every IMG archive in the install has its directory round-tripped. Every `.dff`, `.txd` and
//! `.col` (loose, and inside the IMG archives of `models/`) is parsed and serialised back; the
//! result must equal the input byte for byte. DFF clumps and TXD dictionaries also go through
//! the typed decoders and back. Files that are expected to fail are listed in
//! [`EXPECTED_FAILURES`] with the reason; any other failure fails the test.

use sa_formats::col::{self, ColBody, ColFile, ColVersion};
use sa_formats::img::{ImgDirectory, SECTOR_SIZE};
use sa_formats::rw::dff::Clump;
use sa_formats::rw::txd::TexDictionary;
use sa_formats::rw::{ChunkBody, RwStream, chunk::is_container, id};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Files that do not parse strictly, with the explanation (checked to fail exactly as stated).
const EXPECTED_FAILURES: &[(&str, &str, &str)] = &[(
    "models/coll/peds.col",
    // substring of the expected error
    "COLL body: truncated at offset 0x194: needed 3724 bytes",
    "peds.col is not loaded as collision by SA (default.dat/gta.dat load only WEAPONS.COL). The exe \
     opens it only in ValidateVersion (0x5BA060), which reads a version text from bytes 100, 200, \
     ... (each byte - 23). Those stamp bytes overwrite model data: byte 400 (0x190) is model 0 \
     'cop''s box count low byte, 0x85 = 'n' + 23, so the count reads 133 with no box data. \
     Independently, model 7 'male01' (0x1550) holds one byte less than its declared size, so a \
     game-style walk loses sync at 0x1834 (next FourCC reads 'OLL\\xd0')",
)];

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

fn ext_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

#[derive(Default)]
struct Report {
    failures: Vec<String>,
    expected_failures: Vec<String>,
    counts: BTreeMap<String, u64>,
    chunk_types: BTreeMap<u32, u64>,
}

impl Report {
    fn count(&mut self, key: impl Into<String>, n: u64) {
        *self.counts.entry(key.into()).or_default() += n;
    }

    fn fail(&mut self, name: &str, msg: impl std::fmt::Display) {
        self.failures.push(format!("{name}: {msg}"));
    }

    fn check_rw(&mut self, name: &str, data: &[u8], kind: &str) {
        let st = match RwStream::parse(data) {
            Ok(s) => s,
            Err(e) => return self.fail(name, e),
        };
        if st.to_bytes() != data {
            return self.fail(name, "RW stream round trip differs");
        }
        self.count(format!("{kind}: files parsed + byte-exact"), 1);
        if !st.trailing.is_empty() {
            if st.trailing.iter().all(|&b| b == 0) {
                self.count(format!("{kind}: trailing zero padding (files)"), 1);
            } else {
                self.count(format!("{kind}: trailing NON-ZERO bytes (files)"), 1);
                self.fail(name, format!("{} trailing non-zero bytes", st.trailing.len()));
            }
        }
        self.count(format!("{kind}: top-level chunks"), st.chunks.len() as u64);
        for c in &st.chunks {
            c.visit(&mut |c, _| {
                *self.chunk_types.entry(c.ty).or_default() += 1;
                if is_container(c.ty) && matches!(c.body, ChunkBody::Data(_)) {
                    self.counts
                        .entry("RW: container chunks kept raw (did not tile)".into())
                        .or_default();
                    *self
                        .counts
                        .get_mut("RW: container chunks kept raw (did not tile)")
                        .unwrap() += 1;
                }
            });
            let original = c.to_bytes();
            match c.ty {
                id::CLUMP => match Clump::from_chunk(c).and_then(|t| t.to_chunk()) {
                    Ok(t) if t.to_bytes() == original => {
                        self.count("typed: clumps decoded + re-encoded byte-exact", 1)
                    }
                    Ok(_) => self.fail(name, "typed clump round trip differs"),
                    Err(e) => self.fail(name, e),
                },
                id::TEXDICTIONARY => match TexDictionary::from_chunk(c).and_then(|t| {
                    let n = t.textures.len() as u64;
                    let compressed = t
                        .textures
                        .iter()
                        .filter(|x| x.compression() != sa_formats::rw::txd::Compression::None)
                        .count() as u64;
                    t.to_chunk().map(|c| (c, n, compressed))
                }) {
                    Ok((t, n, compressed)) if t.to_bytes() == original => {
                        self.count("typed: tex dictionaries decoded + re-encoded byte-exact", 1);
                        self.count("typed: native textures", n);
                        self.count("typed: native textures DXT-compressed", compressed);
                    }
                    Ok(_) => self.fail(name, "typed TXD round trip differs"),
                    Err(e) => self.fail(name, e),
                },
                other => self.count(
                    format!(
                        "{kind}: other top-level chunk {other:#x} ({}) kept raw",
                        id::name(other)
                    ),
                    1,
                ),
            }
        }
    }

    fn check_col(&mut self, name: &str, rel: &str, data: &[u8]) {
        let file = match ColFile::parse(data) {
            Ok(f) => f,
            Err(e) => {
                let expected = EXPECTED_FAILURES.iter().find(|(p, _, _)| *p == rel);
                let Some((_, _, why)) = expected.filter(|(_, n, _)| e.to_string().contains(n)) else {
                    return self.fail(name, e);
                };
                let (models, trailing) = col::split_models(data).map_err(|e| e.to_string()).unwrap();
                let mut msg = format!(
                    "{name}: {e}\n    game-style split: {} models, then {} trailing bytes starting {:02x?}\n    why: {why}",
                    models.len(),
                    trailing.len(),
                    &trailing[..trailing.len().min(8)]
                );
                if let Some(text) = col::peds_col_version_text(data) {
                    msg.push_str(&format!(
                        "\n    ValidateVersion text: {:?}",
                        String::from_utf8_lossy(&text)
                    ));
                }
                self.expected_failures.push(msg);
                return;
            }
        };
        match file.to_bytes() {
            Ok(b) if b == data => {}
            Ok(_) => return self.fail(name, "COL round trip differs"),
            Err(e) => return self.fail(name, e),
        }
        self.count("col: files parsed + byte-exact", 1);
        if !file.trailing.is_empty() {
            if file.trailing.iter().all(|&b| b == 0) {
                self.count("col: trailing zero padding (files)", 1);
            } else {
                self.fail(name, format!("{} trailing non-zero bytes", file.trailing.len()));
            }
        }
        for m in &file.models {
            self.count(format!("col: models {:?}", m.version), 1);
            if let ColBody::V2(b) = &m.body {
                if !b.gaps.is_empty() {
                    self.count("col: COL2+ models with uncovered (gap) bytes", 1);
                    self.count(
                        "col: COL2+ gap bytes total",
                        b.gaps.iter().map(|g| g.bytes.len() as u64).sum(),
                    );
                    if b.gaps.iter().any(|g| g.bytes.iter().any(|&x| x != 0)) {
                        self.count("col: COL2+ models with NON-ZERO gap bytes", 1);
                    }
                }
                if !b.vertices_tail.is_empty() {
                    self.count(
                        format!("col: vertex region tail {} bytes (models)", b.vertices_tail.len()),
                        1,
                    );
                }
                if !b.shadow_vertices_tail.is_empty() {
                    self.count(
                        format!(
                            "col: shadow vertex region tail {} bytes (models)",
                            b.shadow_vertices_tail.len()
                        ),
                        1,
                    );
                }
                let max_ref = b
                    .faces
                    .iter()
                    .flat_map(|f| [f.a, f.b, f.c])
                    .max()
                    .map_or(0, |m| m as usize + 1);
                if b.vertices.len() > max_ref {
                    self.count("col: models with vertices beyond the highest face index", 1);
                }
                if b.vertices.len() < max_ref {
                    self.fail(
                        name,
                        format!("model {:?}: face index beyond vertex region", m.name()),
                    );
                }
                if b.face_groups.is_some() {
                    self.count("col: models with face groups", 1);
                }
                if !b.shadow_faces.is_empty() {
                    self.count("col: models with shadow mesh", 1);
                }
                if m.version == ColVersion::V4 {
                    self.count("col: COL4 models", 1);
                }
            }
        }
    }
}

fn read_range(f: &mut File, start: u64, len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(start)).unwrap();
    f.read_exact(&mut buf).unwrap();
    buf
}

#[test]
#[ignore = "needs the GTA SA install (SA_GAME_DIR); run explicitly"]
fn whole_install_round_trips() {
    let root = game_dir();
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut rep = Report::default();

    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let ext = ext_of(&rel);
        match ext.as_str() {
            "img" => {
                let mut f = File::open(path).unwrap();
                let file_len = f.metadata().unwrap().len();
                let prefix = read_range(&mut f, 0, 8);
                let hlen = match ImgDirectory::header_len_from_prefix(&prefix) {
                    Ok(n) => n,
                    Err(e) => {
                        rep.fail(&rel, e);
                        continue;
                    }
                };
                let head = read_range(&mut f, 0, hlen);
                let dir = match ImgDirectory::parse(&head) {
                    Ok(d) => d,
                    Err(e) => {
                        rep.fail(&rel, e);
                        continue;
                    }
                };
                if dir.to_bytes() != head {
                    rep.fail(&rel, "IMG directory round trip differs");
                    continue;
                }
                rep.count("img: archives, directory parsed + byte-exact", 1);
                rep.count("img: directory entries", dir.entries.len() as u64);
                // The bytes between the directory and the first entry must be zero padding.
                let first = dir.entries.iter().map(|e| e.byte_offset()).min().unwrap_or(hlen);
                if first < hlen {
                    rep.fail(&rel, "entry data overlaps the directory");
                } else if read_range(&mut f, hlen as u64, first - hlen)
                    .iter()
                    .any(|&b| b != 0)
                {
                    rep.fail(&rel, "non-zero bytes between directory and first entry");
                }
                let mut spans: Vec<_> = dir
                    .entries
                    .iter()
                    .map(|e| (e.byte_offset(), e.byte_len()))
                    .collect();
                spans.sort_unstable();
                if spans.windows(2).any(|w| w[0].0 + w[0].1 > w[1].0) {
                    rep.fail(&rel, "overlapping entries");
                }
                for e in &dir.entries {
                    if e.size_in_archive != 0 {
                        rep.count("img: entries with size_in_archive != 0", 1);
                    }
                    if e.name_raw[e.name().len()..].iter().any(|&b| b != 0) {
                        rep.count("img: entries with non-zero bytes after the name NUL", 1);
                    }
                    if (e.byte_offset() + e.byte_len()) as u64 > file_len {
                        rep.fail(&rel, format!("entry {} past end of archive", e.name_lossy()));
                        continue;
                    }
                    let ename = e.name_lossy();
                    let eext = ext_of(&ename);
                    if !rel.starts_with("models/") || !matches!(eext.as_str(), "dff" | "txd" | "col") {
                        rep.count(format!("img: entries not decoded here (.{eext}, FMT-2/3)"), 1);
                        continue;
                    }
                    let data = read_range(&mut f, e.byte_offset() as u64, e.byte_len());
                    let label = format!("{rel}:{ename}");
                    if e.byte_len() == 0 {
                        rep.fail(&label, "empty entry");
                        continue;
                    }
                    rep.count(format!("img entries .{eext}"), 1);
                    if eext == "col" {
                        rep.check_col(&label, &label, &data);
                    } else {
                        rep.check_rw(&label, &data, &eext);
                    }
                }
                debug_assert_eq!(SECTOR_SIZE, 2048);
            }
            "dff" | "txd" => {
                rep.count(format!("loose .{ext}"), 1);
                let data = std::fs::read(path).unwrap();
                rep.check_rw(&rel, &data, &ext);
            }
            "col" => {
                rep.count("loose .col", 1);
                let data = std::fs::read(path).unwrap();
                rep.check_col(&rel, &rel, &data);
            }
            _ => {}
        }
    }

    println!("== sa-formats whole-install report ({}) ==", root.display());
    for (k, v) in &rep.counts {
        println!("{v:>10}  {k}");
    }
    println!("-- RW chunk types (all depths) --");
    for (t, n) in &rep.chunk_types {
        println!("{n:>10}  {t:#010x} {}", id::name(*t));
    }
    println!("-- expected failures ({}) --", rep.expected_failures.len());
    for f in &rep.expected_failures {
        println!("{f}");
    }
    println!("-- unexpected failures ({}) --", rep.failures.len());
    for f in rep.failures.iter().take(50) {
        println!("{f}");
    }
    assert!(
        rep.failures.is_empty(),
        "{} unexpected failures",
        rep.failures.len()
    );
    let hit = EXPECTED_FAILURES
        .iter()
        .filter(|(p, _, _)| rep.expected_failures.iter().any(|f| f.starts_with(p)))
        .count();
    assert_eq!(hit, EXPECTED_FAILURES.len(), "an expected failure did not occur");
}
