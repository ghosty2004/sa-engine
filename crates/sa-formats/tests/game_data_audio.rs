//! Whole-install check of the audio containers (read-only). Ignored by default; run with
//!
//! ```text
//! SA_GAME_DIR=/path/to/gta-sa cargo test -p sa-formats --release --test game_data_audio -- --ignored --nocapture
//! ```
//!
//! Without `SA_GAME_DIR` the test uses `<workspace>/wine/gta-sa`.
//!
//! * `audio/CONFIG/*.dat`: parsed and serialised back byte-exactly.
//! * SFX paks: every bank header named by `BankLkup.dat` is parsed and serialised back; sound
//!   offsets are checked against the bank size; the banks must tile each pak file.
//! * Stream packs: every pack named by a `TrakLkup.dat` entry is decrypted, split into tracks
//!   (header + Ogg pages, CRC-checked) and rebuilt; the rebuilt encrypted file must equal the
//!   original byte for byte.

use sa_formats::audio::config::{
    BankLookup, BankSlots, EventVolumes, LookupEntry, PakFiles, StreamPacks, TrackLookup,
};
use sa_formats::audio::sfx::{BANK_HEADER_SIZE, BankHeader, BankLocation, PakLayout};
use sa_formats::audio::stream::{SizeKind, StreamPack, xor_in_place};
use sa_formats::data::DataFile;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

fn game_dir() -> PathBuf {
    let dir = std::env::var_os("SA_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wine/gta-sa"));
    assert!(
        dir.join("audio").is_dir(),
        "game data not found at {} (set SA_GAME_DIR)",
        dir.display()
    );
    dir
}

/// Finds `name` in `dir` case-insensitively (the install mixes `CONFIG`, `sfx`, `streams`).
fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.map(|e| e.unwrap().path()).find(|p| {
        p.file_name()
            .is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(name))
    })
}

#[derive(Default)]
struct Report {
    failures: Vec<String>,
    counts: BTreeMap<String, u64>,
}

impl Report {
    fn count(&mut self, key: impl Into<String>, n: u64) {
        *self.counts.entry(key.into()).or_default() += n;
    }
    fn fail(&mut self, what: &str, msg: impl std::fmt::Display) {
        self.failures.push(format!("{what}: {msg}"));
    }
    fn data_file(&mut self, name: &str, f: &dyn DataFile, round_trip_ok: bool) {
        if !round_trip_ok {
            self.fail(name, "round trip differs");
        } else {
            self.count(format!("{name}: byte-exact round trip"), 1);
        }
        for (k, v) in f.counts() {
            self.count(k, v);
        }
    }
}

fn read(p: &Path) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
#[ignore = "needs the game install (SA_GAME_DIR)"]
fn whole_install_audio() {
    let root = game_dir();
    let audio = find_ci(&root, "audio").unwrap();
    let cfg = find_ci(&audio, "CONFIG").expect("audio/CONFIG");
    let mut rep = Report::default();
    let cfg_file = |n: &str| read(&find_ci(&cfg, n).unwrap_or_else(|| panic!("audio/CONFIG/{n} missing")));

    let d = cfg_file("BankLkup.dat");
    let banks = BankLookup::parse(&d).unwrap();
    rep.data_file("BankLkup.dat", &banks, banks.to_bytes() == d);
    let d = cfg_file("PakFiles.dat");
    let paks = PakFiles::parse(&d).unwrap();
    rep.data_file("PakFiles.dat", &paks, paks.to_bytes() == d);
    let d = cfg_file("BankSlot.dat");
    let slots = BankSlots::parse(&d).unwrap();
    rep.data_file("BankSlot.dat", &slots, slots.to_bytes() == d);
    if !slots.trailing.is_empty() {
        rep.fail(
            "BankSlot.dat",
            format!("{} bytes after the slots", slots.trailing.len()),
        );
    }
    let (computed, _) = slots.computed_offsets();
    let stale = slots
        .slots
        .iter()
        .zip(&computed)
        .filter(|(s, c)| s.offset != **c)
        .count();
    rep.count(
        "BankSlot.dat: slots whose stored offset differs from the computed one",
        stale as u64,
    );
    let d = cfg_file("StrmPaks.dat");
    let packs = StreamPacks::parse(&d).unwrap();
    rep.data_file("StrmPaks.dat", &packs, packs.to_bytes() == d);
    let d = cfg_file("TrakLkup.dat");
    let tracks = TrackLookup::parse(&d).unwrap();
    rep.data_file("TrakLkup.dat", &tracks, tracks.to_bytes() == d);
    let d = cfg_file("EventVol.dat");
    match EventVolumes::parse(&d) {
        Ok(ev) => rep.data_file("EventVol.dat", &ev, ev.to_bytes() == d),
        Err(e) => rep.fail("EventVol.dat", e),
    }
    for (name, trailing) in [
        ("BankLkup.dat", &banks.trailing),
        ("PakFiles.dat", &paks.trailing),
        ("StrmPaks.dat", &packs.trailing),
        ("TrakLkup.dat", &tracks.trailing),
    ] {
        if !trailing.is_empty() {
            rep.fail(name, format!("{} bytes after the last record", trailing.len()));
        }
    }

    // SFX paks.
    let sfx_dir = find_ci(&audio, "SFX").expect("audio/SFX");
    let mut by_pak: BTreeMap<u8, Vec<BankLocation>> = BTreeMap::new();
    for (i, e) in banks.banks.iter().enumerate() {
        by_pak
            .entry(e.file)
            .or_default()
            .push(BankLocation { bank: i, lookup: *e });
    }
    let mut rates: BTreeMap<u16, u64> = BTreeMap::new();
    for (&pak, locs) in &by_pak {
        let Some(entry) = paks.paks.get(pak as usize) else {
            rep.fail(
                "BankLkup.dat",
                format!("{} banks name pak {pak}, beyond PakFiles.dat", locs.len()),
            );
            continue;
        };
        let name = String::from_utf8_lossy(entry.name()).into_owned();
        let Some(path) = find_ci(&sfx_dir, &name) else {
            rep.fail(&format!("SFX/{name}"), "missing");
            continue;
        };
        let mut f = File::open(&path).unwrap();
        let len = f.metadata().unwrap().len();
        rep.count("SFX: paks", 1);
        rep.count("SFX: pak bytes", len);
        let layout = match PakLayout::check(len, locs.clone()) {
            Ok(l) => l,
            Err(e) => {
                rep.fail(&format!("SFX/{name}"), e);
                continue;
            }
        };
        if !layout.overlaps.is_empty() {
            rep.fail(
                &format!("SFX/{name}"),
                format!("overlapping banks {:?}", layout.overlaps),
            );
        }
        if !layout.gaps.is_empty() {
            rep.fail(
                &format!("SFX/{name}"),
                format!("bytes not covered by any bank {:?}", layout.gaps),
            );
        }
        for loc in &layout.banks {
            let what = format!("SFX/{name} bank {}", loc.bank);
            let mut h = vec![0u8; BANK_HEADER_SIZE];
            f.seek(SeekFrom::Start(loc.lookup.offset as u64)).unwrap();
            f.read_exact(&mut h).unwrap();
            let hdr = match BankHeader::parse(&h) {
                Ok(x) => x,
                Err(e) => {
                    rep.fail(&what, e);
                    continue;
                }
            };
            if hdr.to_bytes() != h {
                rep.fail(&what, "header round trip differs");
                continue;
            }
            rep.count("SFX: bank headers byte-exact", 1);
            rep.count("SFX: sample data bytes", loc.lookup.size as u64);
            if hdr.pad != [0, 0] {
                rep.count("SFX: banks with non-zero header pad", 1);
            }
            let sizes = match hdr.sound_sizes(loc.lookup.size) {
                Ok(s) => s,
                Err(e) => {
                    rep.fail(&what, e);
                    continue;
                }
            };
            if hdr.used().first().is_some_and(|s| s.offset != 0) {
                rep.count("SFX: banks whose first sound is not at offset 0", 1);
            }
            let unused_nonzero = hdr.sounds[hdr.used().len()..]
                .iter()
                .filter(|s| s.offset != 0 || s.loop_start != 0 || s.sample_rate != 0 || s.headroom != 0)
                .count();
            rep.count(
                "SFX: unused header entries that are not all-zero",
                unused_nonzero as u64,
            );
            rep.count("SFX: sounds", sizes.len() as u64);
            for (s, size) in hdr.used().iter().zip(&sizes) {
                *rates.entry(s.sample_rate).or_default() += 1;
                if size % 2 != 0 {
                    rep.count("SFX: sounds with an odd byte size", 1);
                }
                if *size == 0 {
                    rep.count("SFX: sounds with size 0", 1);
                }
                if s.loop_start != u32::MAX {
                    rep.count("SFX: sounds with a loop start", 1);
                    if s.loop_start > 0x7FFF {
                        // CAEStaticChannel::SetAudioBuffer (0x4F0C40) takes the loop start
                        // as an int16 (0xFFFF = no loop).
                        rep.count("SFX: loop start > 0x7FFF (truncated to int16 by the exe)", 1);
                    }
                    if s.loop_start as u64 * 2 > *size as u64 {
                        rep.count("SFX: loop start (as 16-bit sample index) beyond the sound", 1);
                    }
                }
            }
        }
    }
    for (pi, p) in paks.paks.iter().enumerate() {
        if !by_pak.contains_key(&(pi as u8)) {
            rep.fail(
                "PakFiles.dat",
                format!("pak {pi} {:?} has no bank", String::from_utf8_lossy(p.name())),
            );
        }
    }

    // Stream packs.
    let streams_dir = find_ci(&audio, "streams").expect("audio/streams");
    let mut by_pack: BTreeMap<u8, Vec<(usize, LookupEntry)>> = BTreeMap::new();
    for (i, e) in tracks.tracks.iter().enumerate() {
        by_pack.entry(e.file).or_default().push((i, *e));
    }
    let mut vorbis: BTreeMap<(u8, u32), u64> = BTreeMap::new();
    let mut extra1: BTreeMap<u32, u64> = BTreeMap::new();
    for (pi, _) in packs.names_raw.iter().enumerate() {
        if !by_pack.contains_key(&(pi as u8)) {
            rep.count(
                format!(
                    "StrmPaks.dat: entries with no track (pack {pi} {:?})",
                    String::from_utf8_lossy(packs.name(pi).unwrap())
                ),
                1,
            );
        }
    }
    for (&pack, list) in &by_pack {
        let Some(name) = packs
            .name(pack as usize)
            .map(|n| String::from_utf8_lossy(n).into_owned())
        else {
            rep.fail(
                "TrakLkup.dat",
                format!("{} tracks name pack {pack}, beyond StrmPaks.dat", list.len()),
            );
            continue;
        };
        let what = format!("streams/{name}");
        let Some(path) = find_ci(&streams_dir, &name).filter(|_| !name.is_empty()) else {
            rep.fail(&what, format!("missing ({} tracks)", list.len()));
            continue;
        };
        let raw = read(&path);
        let mut dec = raw.clone();
        xor_in_place(&mut dec, 0);
        let pack_p = match StreamPack::parse(&dec, list) {
            Ok(p) => p,
            Err(e) => {
                rep.fail(&what, e);
                continue;
            }
        };
        drop(dec);
        let mut dec = raw.clone();
        xor_in_place(&mut dec, 0);
        if pack_p.rebuild(&dec) != raw {
            rep.fail(&what, "rebuilt pack differs");
        } else {
            rep.count("streams: packs byte-exact (headers + Ogg pages + XOR)", 1);
        }
        rep.count("streams: pack bytes", raw.len() as u64);
        if !pack_p.gaps.is_empty() {
            rep.fail(&what, format!("bytes before/between tracks {:?}", pack_p.gaps));
        }
        for t in &pack_p.tracks {
            rep.count("streams: tracks", 1);
            rep.count("streams: Ogg pages", t.pages.len() as u64);
            let bad_crc = t.pages.iter().filter(|p| !p.crc_ok).count() as u64;
            if bad_crc > 0 {
                rep.fail(
                    &format!("{what} track {}", t.track),
                    format!("{bad_crc} pages with a bad CRC"),
                );
            }
            if t.is_single_clean_stream() {
                rep.count("streams: tracks that are one clean Ogg logical stream", 1);
            } else {
                rep.fail(
                    &format!("{what} track {}", t.track),
                    "not one clean Ogg logical stream",
                );
            }
            match t.size_kind {
                SizeKind::OggOnly => rep.count("streams: TrakLkup size == Ogg length", 1),
                SizeKind::IncludesHeader => {
                    rep.count("streams: TrakLkup size == Ogg length + 0x1F84", 1);
                    rep.count(format!("streams: size includes header, in pack {name}"), 1);
                }
            }
            match t.info.beat_count() {
                Some(0) => rep.count("streams: tracks with no beats (key[0] == 0)", 1),
                Some(n) => {
                    rep.count("streams: tracks with beats", 1);
                    rep.count(format!("streams: beats in pack {name}"), n as u64);
                }
                None => rep.count("streams: tracks whose 1000 beat keys are all non-zero", 1),
            }
            let ogg_len = t.ogg.end - t.ogg.start;
            if t.info.extra[0] as u64 == ogg_len {
                rep.count("streams: info word 0x1F40 == Ogg length", 1);
            } else {
                rep.count(
                    format!("streams: info word 0x1F40 != Ogg length (pack {name})"),
                    1,
                );
            }
            *extra1.entry(t.info.extra[1]).or_default() += 1;
            if t.info.field_1f80 != 1 {
                rep.count("streams: info u16 at 0x1F80 != 1", 1);
            }
            *vorbis.entry((t.vorbis_channels, t.vorbis_rate)).or_default() += 1;
        }
    }

    println!("== sa-formats audio report ({}) ==", root.display());
    for (k, v) in &rep.counts {
        println!("{v:>12}  {k}");
    }
    let mut top: Vec<_> = rates.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "-- SFX sample rates: {} distinct, {}..={} Hz; most common: {:?}",
        rates.len(),
        rates.keys().next().copied().unwrap_or(0),
        rates.keys().last().copied().unwrap_or(0),
        &top[..top.len().min(6)]
    );
    println!("-- Vorbis (channels, rate) (tracks) --");
    for ((c, r), v) in &vorbis {
        println!("{v:>12}  {c} ch {r} Hz");
    }
    println!("-- info word 0x1F44 values (tracks) --");
    for (k, v) in &extra1 {
        println!("{v:>12}  {k:#010x} ({k})");
    }
    println!("-- unexpected failures ({}) --", rep.failures.len());
    for f in &rep.failures {
        println!("{f}");
    }
    assert!(
        rep.failures.is_empty(),
        "{} unexpected failures",
        rep.failures.len()
    );
}
