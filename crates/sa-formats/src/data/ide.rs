//! IDE (item definition) files, as `CFileLoader::LoadObjectTypes` (0x5B8400) reads them.
//!
//! The file is opened `"rb"` and read with `LoadLine` (0x536F80). Lines whose first byte is NUL
//! or `#` are skipped. Outside a section, a line whose first four bytes are a section name opens
//! that section (`objs tobj weap hier anim cars peds path 2dfx txdp`, compared byte by byte,
//! case-sensitive); any other line is ignored. Inside a section, a line starting with `end`
//! closes it; every other line goes to the section's loader:
//!
//! | section | loader | format(s) |
//! |---|---|---|
//! | objs | 0x5B3C60 | `%d %s %s %f %d`; if ret != 5 or draw < 4: `%d %s %s %d` (must be 4, else the line is rejected) then by type 1/2/3 `%d %s %s %d %f %d`, `%d %s %s %d %f %f %d`, `%d %s %s %d %f %f %f %d` |
//! | tobj | 0x5B3DE0 | `%d %s %s %f %d %d %d`; same fallback with `%d %s %s %d [%f..] %d %d %d` |
//! | weap | 0x5B3FB0 | `%d %s %s %s %d %f` (no return check) |
//! | hier | 0x5B4040 | `%d %s %s` (must be 3) |
//! | anim | 0x5B40C0 | `%d %s %s %s %f %d` (must be 6; anim defaults to `null`, draw to 2000) |
//! | cars | 0x5B6F30 | `%d %s %s %s %s %s %s %s %d %d %x %d %f %f %d` (no return check) |
//! | peds | 0x5B7420 | `%d %s %s %s %s %s %x %x %s %d %d %s %s %s` (no return check) |
//! | path | 0x5B41C0 + 0x5B41F0/0x5B4380 | header `%d %d %s`, then 12 node lines (VC leftover, kept raw) |
//! | 2dfx | 0x5B7670 | `%d %f %f %f %d`, then a second format chosen by the type (see [`Effect2d`]) |
//! | txdp | 0x5B75E0 | `%s %s` |

use super::{DataFile, ScanRec, Vars, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

/// `objs` entry (atomic model).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeObj {
    pub line: usize,
    pub id: Option<i32>,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
    /// The type field of the old format (number of draw distances), when the fallback ran.
    pub obj_type: Option<i32>,
    /// The (first) draw distance; the 2nd/3rd of the old format are read and discarded.
    pub draw_distance: Option<f32>,
    pub flags: Option<u32>,
}

/// `tobj` entry (timed atomic model).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeTobj {
    pub line: usize,
    pub id: Option<i32>,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
    pub obj_type: Option<i32>,
    pub draw_distance: Option<f32>,
    pub flags: Option<u32>,
    pub time_on: Option<i32>,
    pub time_off: Option<i32>,
}

/// `weap` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeWeap {
    pub line: usize,
    pub ret: i32,
    pub id: Option<i32>,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
    pub anim: Option<Vec<u8>>,
    /// Fifth field (object count in the format); read and not used by the loader.
    pub num_objs: Option<i32>,
    pub draw_distance: Option<f32>,
}

/// `hier` entry (clump model without animation).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeHier {
    pub line: usize,
    pub id: i32,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
}

/// `anim` entry (animated clump model).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeAnim {
    pub line: usize,
    pub id: i32,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
    pub anim: Vec<u8>,
    pub draw_distance: f32,
    pub flags: Option<u32>,
}

/// `cars` entry (vehicle model).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeCar {
    pub line: usize,
    pub ret: i32,
    pub id: i32,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
    /// `car`, `mtruck`, `quad`, `heli`, `plane`, `boat`, `train`, `f_heli`, `f_plane`, `bike`,
    /// `bmx`, `trailer`.
    pub vehicle_type: Option<Vec<u8>>,
    pub handling: Option<Vec<u8>>,
    pub game_name: Option<Vec<u8>>,
    pub anims: Option<Vec<u8>>,
    /// `normal`, `poorfamily`, ... `ignore`.
    pub class: Option<Vec<u8>>,
    pub frequency: Option<i32>,
    pub flags: Option<i32>,
    /// `%x` component rules.
    pub comp_rules: Option<u32>,
    /// Wheel model id, or the steering angle for bikes/BMX.
    pub misc: Option<i32>,
    /// Not initialised in 1.0 US: `None` keeps the stack garbage (the "Windows 11 24H2" bug).
    pub wheel_scale_front: Option<f32>,
    pub wheel_scale_rear: Option<f32>,
    /// Wheel upgrade class (initialised to -1).
    pub wheel_upgrade_class: i32,
}

/// `peds` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdePed {
    pub line: usize,
    pub ret: i32,
    pub id: i32,
    pub model: Option<Vec<u8>>,
    pub txd: Option<Vec<u8>>,
    pub ped_type: Option<Vec<u8>>,
    pub stat: Option<Vec<u8>>,
    pub anim_group: Option<Vec<u8>>,
    /// `%x`: which car classes the ped may drive.
    pub cars_can_drive_mask: Option<u32>,
    /// `%x` flags.
    pub flags: Option<u32>,
    pub anim_file: Option<Vec<u8>>,
    pub radio1: Option<i32>,
    pub radio2: Option<i32>,
    pub voice_type: Option<Vec<u8>>,
    pub voice_min: Option<Vec<u8>>,
    pub voice_max: Option<Vec<u8>>,
}

/// `txdp` entry (texture dictionary parent).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeTxdParent {
    pub line: usize,
    pub txd: Option<Vec<u8>>,
    pub parent: Option<Vec<u8>>,
}

/// `2dfx` entry. Type-dependent second formats (0x5B7670):
///
/// * 0 light: `%d %f %f %f %d %d %d %d %d`, then two `"`-quoted names copied by hand (corona
///   and shadow texture), then `%f %f %f %f %d %d %d %d %d %d %d %d %d` on the text after the
///   second closing quote;
/// * 1 particle: `%d %f %f %f %d %s`;
/// * 3 ped attractor: `%d %f %f %f %d %d %f %f %f %f %f %f %f %f %f %d %d %s`;
/// * 5: 38 fields (`%d %f %f %f %d %d %f %f %f %f` + 28 `%d`, several pre-set to -1 and one to 50);
/// * 6 enex: `%d %f %f %f %d %f %f %f %f %f %f %f %d %d %s %d`;
/// * 7 road sign: `%d %f %f %f %d %f %f %f %f %f %d %s %s %s %s`;
/// * 8: `%d %f %f %f %d %d`; 9: `%d %f %f %f %d %f %f %d`;
/// * 10: `%d %f %f %f %d %f %f %f %f %f %f %f %f %f %d`;
/// * other types: only the first format.
#[derive(Debug, Clone, PartialEq)]
pub struct Ide2dfx {
    pub rec: ScanRec,
    /// For type 0: the two quoted strings (`None` if the line has fewer quotes; the exe would
    /// read past the line then).
    pub quoted: Option<(Vec<u8>, Vec<u8>)>,
}

/// Raw line of an IDE `path` section (header or node). The loaders it reaches are VC
/// leftovers whose results are discarded; no IDE in the install has a non-empty path section.
#[derive(Debug, Clone, PartialEq)]
pub struct IdePathLine {
    pub line: usize,
    pub text: Vec<u8>,
    /// `true` for a header line (`%d %d %s`), `false` for one of the 12 node lines.
    pub header: bool,
}

/// A parsed IDE file. Records keep file order within each section kind.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ide {
    pub objs: Vec<IdeObj>,
    pub tobj: Vec<IdeTobj>,
    pub weap: Vec<IdeWeap>,
    pub hier: Vec<IdeHier>,
    pub anim: Vec<IdeAnim>,
    pub cars: Vec<IdeCar>,
    pub peds: Vec<IdePed>,
    pub path: Vec<IdePathLine>,
    pub fx2d: Vec<Ide2dfx>,
    pub txdp: Vec<IdeTxdParent>,
    /// `objs`/`tobj`/`hier`/`anim` lines the loader rejects (returns -1 without adding a model).
    pub rejected: Vec<(usize, &'static str, Vec<u8>)>,
    /// Lines outside any section that are not a section header (ignored by the exe).
    pub ignored: Vec<(usize, Vec<u8>)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    None,
    Objs,
    Tobj,
    Weap,
    Hier,
    Anim,
    Cars,
    Peds,
    Path,
    Fx2d,
    Txdp,
}

impl Ide {
    pub fn parse(data: &[u8]) -> Ide {
        let mut ide = Ide::default();
        let mut section = Section::None;
        let mut path_index: i32 = -1;
        for l in load_lines(data, Mode::Binary) {
            let t = &l.text[..];
            let c = l.first();
            if c == 0 || c == b'#' {
                continue;
            }
            if section == Section::None {
                let h = |s: &[u8; 4]| t.len() >= 4 && &t[..4] == s;
                section = if h(b"objs") {
                    Section::Objs
                } else if h(b"tobj") {
                    Section::Tobj
                } else if h(b"weap") {
                    Section::Weap
                } else if h(b"hier") {
                    Section::Hier
                } else if h(b"anim") {
                    Section::Anim
                } else if h(b"cars") {
                    Section::Cars
                } else if h(b"peds") {
                    Section::Peds
                } else if h(b"path") {
                    Section::Path
                } else if h(b"2dfx") {
                    Section::Fx2d
                } else if h(b"txdp") {
                    Section::Txdp
                } else {
                    ide.ignored.push((l.number, t.to_vec()));
                    Section::None
                };
                continue;
            }
            if t.starts_with(b"end") {
                section = Section::None;
                continue;
            }
            let n = l.number;
            match section {
                Section::None => unreachable!(),
                Section::Objs => match parse_objs(n, t) {
                    Some(o) => ide.objs.push(o),
                    None => ide.rejected.push((n, "objs", t.to_vec())),
                },
                Section::Tobj => match parse_tobj(n, t) {
                    Some(o) => ide.tobj.push(o),
                    None => ide.rejected.push((n, "tobj", t.to_vec())),
                },
                Section::Weap => ide.weap.push(parse_weap(n, t)),
                Section::Hier => {
                    let mut v = Vars::default().init("id", Val::Int(-1));
                    if v.scan(t, "%d %s %s", &["id", "model", "txd"]) != 3 {
                        ide.rejected.push((n, "hier", t.to_vec()));
                    } else {
                        ide.hier.push(IdeHier {
                            line: n,
                            id: v.i("id").unwrap(),
                            model: v.s("model"),
                            txd: v.s("txd"),
                        });
                    }
                }
                Section::Anim => {
                    let mut v = Vars::default()
                        .init("id", Val::Int(-1))
                        .init("anim", Val::Str(b"null".to_vec()))
                        .init("draw", Val::Float(2000.0));
                    let names = ["id", "model", "txd", "anim", "draw", "flags"];
                    if v.scan(t, "%d %s %s %s %f %d", &names) != 6 {
                        ide.rejected.push((n, "anim", t.to_vec()));
                    } else {
                        ide.anim.push(IdeAnim {
                            line: n,
                            id: v.i("id").unwrap(),
                            model: v.s("model"),
                            txd: v.s("txd"),
                            anim: v.s("anim").unwrap(),
                            draw_distance: v.f("draw").unwrap(),
                            flags: v.u("flags"),
                        });
                    }
                }
                Section::Cars => ide.cars.push(parse_cars(n, t)),
                Section::Peds => ide.peds.push(parse_peds(n, t)),
                Section::Path => {
                    ide.path.push(IdePathLine {
                        line: n,
                        text: t.to_vec(),
                        header: path_index == -1,
                    });
                    if path_index == -1 {
                        path_index = 0;
                    } else {
                        path_index += 1;
                        if path_index == 12 {
                            path_index = -1;
                        }
                    }
                }
                Section::Fx2d => ide.fx2d.push(parse_2dfx(n, t)),
                Section::Txdp => {
                    let mut v = Vars::default();
                    v.scan(t, "%s %s", &["txd", "parent"]);
                    ide.txdp.push(IdeTxdParent {
                        line: n,
                        txd: v.s("txd"),
                        parent: v.s("parent"),
                    });
                }
            }
        }
        ide
    }
}

fn parse_objs(line: usize, t: &[u8]) -> Option<IdeObj> {
    let mut v = Vars::default();
    let r = v.scan(t, "%d %s %s %f %d", &["id", "model", "txd", "draw", "flags"]);
    if r != 5 || v.f("draw").is_some_and(|d| d < 4.0) {
        if v.scan(t, "%d %s %s %d", &["id", "model", "txd", "type"]) != 4 {
            return None;
        }
        match v.i("type") {
            Some(1) => {
                v.scan(
                    t,
                    "%d %s %s %d %f %d",
                    &["id", "model", "txd", "type", "draw", "flags"],
                );
            }
            Some(2) => {
                v.scan(
                    t,
                    "%d %s %s %d %f %f %d",
                    &["id", "model", "txd", "type", "draw", "", "flags"],
                );
            }
            Some(3) => {
                v.scan(
                    t,
                    "%d %s %s %d %f %f %f %d",
                    &["id", "model", "txd", "type", "draw", "", "", "flags"],
                );
            }
            _ => {}
        }
    }
    Some(IdeObj {
        line,
        id: v.i("id"),
        model: v.s("model"),
        txd: v.s("txd"),
        obj_type: v.i("type"),
        draw_distance: v.f("draw"),
        flags: v.u("flags"),
    })
}

fn parse_tobj(line: usize, t: &[u8]) -> Option<IdeTobj> {
    let mut v = Vars::default();
    let r = v.scan(
        t,
        "%d %s %s %f %d %d %d",
        &["id", "model", "txd", "draw", "flags", "on", "off"],
    );
    if r != 7 || v.f("draw").is_some_and(|d| d < 4.0) {
        if v.scan(t, "%d %s %s %d", &["id", "model", "txd", "type"]) != 4 {
            return None;
        }
        match v.i("type") {
            Some(1) => {
                v.scan(
                    t,
                    "%d %s %s %d %f %d %d %d",
                    &["id", "model", "txd", "type", "draw", "flags", "on", "off"],
                );
            }
            Some(2) => {
                v.scan(
                    t,
                    "%d %s %s %d %f %f %d %d %d",
                    &["id", "model", "txd", "type", "draw", "", "flags", "on", "off"],
                );
            }
            Some(3) => {
                v.scan(
                    t,
                    "%d %s %s %d %f %f %f %d %d %d",
                    &["id", "model", "txd", "type", "draw", "", "", "flags", "on", "off"],
                );
            }
            _ => {}
        }
    }
    Some(IdeTobj {
        line,
        id: v.i("id"),
        model: v.s("model"),
        txd: v.s("txd"),
        obj_type: v.i("type"),
        draw_distance: v.f("draw"),
        flags: v.u("flags"),
        time_on: v.i("on"),
        time_off: v.i("off"),
    })
}

fn parse_weap(line: usize, t: &[u8]) -> IdeWeap {
    let mut v = Vars::default();
    let ret = v.scan(
        t,
        "%d %s %s %s %d %f",
        &["id", "model", "txd", "anim", "num", "draw"],
    );
    IdeWeap {
        line,
        ret,
        id: v.i("id"),
        model: v.s("model"),
        txd: v.s("txd"),
        anim: v.s("anim"),
        num_objs: v.i("num"),
        draw_distance: v.f("draw"),
    }
}

fn parse_cars(line: usize, t: &[u8]) -> IdeCar {
    let mut v = Vars::default()
        .init("id", Val::Int(-1))
        .init("upgrade", Val::Int(-1));
    let ret = v.scan(
        t,
        "%d %s %s %s %s %s %s %s %d %d %x %d %f %f %d",
        &[
            "id", "model", "txd", "type", "handling", "gxt", "anims", "class", "freq", "flags", "comp",
            "misc", "wf", "wr", "upgrade",
        ],
    );
    IdeCar {
        line,
        ret,
        id: v.i("id").unwrap(),
        model: v.s("model"),
        txd: v.s("txd"),
        vehicle_type: v.s("type"),
        handling: v.s("handling"),
        game_name: v.s("gxt"),
        anims: v.s("anims"),
        class: v.s("class"),
        frequency: v.i("freq"),
        flags: v.i("flags"),
        comp_rules: v.u("comp"),
        misc: v.i("misc"),
        wheel_scale_front: v.f("wf"),
        wheel_scale_rear: v.f("wr"),
        wheel_upgrade_class: v.i("upgrade").unwrap(),
    }
}

fn parse_peds(line: usize, t: &[u8]) -> IdePed {
    let mut v = Vars::default().init("id", Val::Int(-1));
    let ret = v.scan(
        t,
        "%d %s %s %s %s %s %x %x %s %d %d %s %s %s",
        &[
            "id", "model", "txd", "type", "stat", "animgrp", "mask", "flags", "animfile", "r1", "r2",
            "voice", "vmin", "vmax",
        ],
    );
    IdePed {
        line,
        ret,
        id: v.i("id").unwrap(),
        model: v.s("model"),
        txd: v.s("txd"),
        ped_type: v.s("type"),
        stat: v.s("stat"),
        anim_group: v.s("animgrp"),
        cars_can_drive_mask: v.u("mask"),
        flags: v.u("flags"),
        anim_file: v.s("animfile"),
        radio1: v.i("r1"),
        radio2: v.i("r2"),
        voice_type: v.s("voice"),
        voice_min: v.s("vmin"),
        voice_max: v.s("vmax"),
    }
}

/// Copies the text between the next two `"` (as the light case of 0x5B7670 does) and returns it
/// with the rest after the closing quote.
fn quoted(t: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let a = t.iter().position(|&c| c == b'"')?;
    let rest = &t[a + 1..];
    let b = rest.iter().position(|&c| c == b'"')?;
    Some((rest[..b].to_vec(), &rest[b + 1..]))
}

fn parse_2dfx(line: usize, t: &[u8]) -> Ide2dfx {
    let mut rec = ScanRec::run(line, t, &["%d %f %f %f %d"]);
    let ty = rec.scans[0].1.i32(4);
    let mut q = None;
    match ty {
        Some(0) => {
            rec.push(t, "%d %f %f %f %d %d %d %d %d");
            if let Some((a, rest)) = quoted(t)
                && let Some((b, rest2)) = quoted(rest)
            {
                rec.push(rest2, "%f %f %f %f %d %d %d %d %d %d %d %d %d");
                q = Some((a, b));
            }
        }
        Some(1) => {
            rec.push(t, "%d %f %f %f %d %s");
        }
        Some(3) => {
            rec.push(t, "%d %f %f %f %d %d %f %f %f %f %f %f %f %f %f %d %d %s");
        }
        Some(5) => {
            rec.push(
                t,
                "%d %f %f %f %d %d %f %f %f %f %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d  %d %d %d %d  %d %d %d %d  %d %d %d %d",
            );
        }
        Some(6) => {
            rec.push(t, "%d %f %f %f %d %f %f %f %f %f %f %f %d %d %s %d");
        }
        Some(7) => {
            rec.push(t, "%d %f %f %f %d %f %f %f %f %f %d %s %s %s %s");
        }
        Some(8) => {
            rec.push(t, "%d %f %f %f %d %d");
        }
        Some(9) => {
            rec.push(t, "%d %f %f %f %d %f %f %d");
        }
        Some(10) => {
            rec.push(t, "%d %f %f %f %d %f %f %f %f %f %f %f %f %f %d");
        }
        _ => {}
    }
    Ide2dfx { rec, quoted: q }
}

impl DataFile for Ide {
    fn counts(&self) -> Vec<(String, u64)> {
        let mut v = vec![
            ("ide: objs".to_string(), self.objs.len() as u64),
            ("ide: tobj".into(), self.tobj.len() as u64),
            ("ide: weap".into(), self.weap.len() as u64),
            ("ide: hier".into(), self.hier.len() as u64),
            ("ide: anim".into(), self.anim.len() as u64),
            ("ide: cars".into(), self.cars.len() as u64),
            ("ide: peds".into(), self.peds.len() as u64),
            ("ide: path lines".into(), self.path.len() as u64),
            ("ide: 2dfx".into(), self.fx2d.len() as u64),
            ("ide: txdp".into(), self.txdp.len() as u64),
            ("ide: rejected lines".into(), self.rejected.len() as u64),
        ];
        v.retain(|(_, n)| *n > 0);
        v
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.objs {
            dump_rec!(
                &mut o,
                "objs",
                line = r.line,
                id = r.id,
                model = r.model,
                txd = r.txd,
                obj_type = r.obj_type,
                draw = r.draw_distance,
                flags = r.flags
            );
        }
        for r in &self.tobj {
            dump_rec!(
                &mut o,
                "tobj",
                line = r.line,
                id = r.id,
                model = r.model,
                txd = r.txd,
                obj_type = r.obj_type,
                draw = r.draw_distance,
                flags = r.flags,
                on = r.time_on,
                off = r.time_off
            );
        }
        for r in &self.weap {
            dump_rec!(
                &mut o,
                "weap",
                line = r.line,
                ret = r.ret,
                id = r.id,
                model = r.model,
                txd = r.txd,
                anim = r.anim,
                num = r.num_objs,
                draw = r.draw_distance
            );
        }
        for r in &self.hier {
            dump_rec!(
                &mut o,
                "hier",
                line = r.line,
                id = r.id,
                model = r.model,
                txd = r.txd
            );
        }
        for r in &self.anim {
            dump_rec!(
                &mut o,
                "anim",
                line = r.line,
                id = r.id,
                model = r.model,
                txd = r.txd,
                anim = r.anim,
                draw = r.draw_distance,
                flags = r.flags
            );
        }
        for r in &self.cars {
            dump_rec!(
                &mut o,
                "cars",
                line = r.line,
                ret = r.ret,
                id = r.id,
                model = r.model,
                txd = r.txd,
                vtype = r.vehicle_type,
                handling = r.handling,
                gxt = r.game_name,
                anims = r.anims,
                class = r.class,
                freq = r.frequency,
                flags = r.flags,
                comp = r.comp_rules,
                misc = r.misc,
                wheel_front = r.wheel_scale_front,
                wheel_rear = r.wheel_scale_rear,
                upgrade = r.wheel_upgrade_class
            );
        }
        for r in &self.peds {
            dump_rec!(
                &mut o,
                "peds",
                line = r.line,
                ret = r.ret,
                id = r.id,
                model = r.model,
                txd = r.txd,
                ptype = r.ped_type,
                stat = r.stat,
                animgrp = r.anim_group,
                mask = r.cars_can_drive_mask,
                flags = r.flags,
                animfile = r.anim_file,
                radio1 = r.radio1,
                radio2 = r.radio2,
                voice = r.voice_type,
                vmin = r.voice_min,
                vmax = r.voice_max
            );
        }
        for r in &self.path {
            dump_rec!(&mut o, "path", line = r.line, header = r.header, text = r.text);
        }
        for r in &self.fx2d {
            o.push_str("2dfx");
            dump_rec!(&mut o, "", line = r.rec.line);
            o.pop();
            if let Some((a, b)) = &r.quoted {
                dump_rec!(&mut o, "", q1 = a, q2 = b);
                o.pop();
            }
            r.rec.dump_to(&mut o);
            o.push('\n');
        }
        for r in &self.txdp {
            dump_rec!(&mut o, "txdp", line = r.line, txd = r.txd, parent = r.parent);
        }
        for (n, k, t) in &self.rejected {
            dump_rec!(&mut o, "rejected", line = *n, section = k.as_bytes(), text = t);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .rejected
            .iter()
            .map(|(n, k, t)| format!("line {n}: {k} line rejected by the loader: {:?}", super::lossy(t)))
            .collect();
        for c in &self.cars {
            if c.ret != 15 {
                v.push(format!("line {}: cars sscanf returned {} of 15", c.line, c.ret));
            }
        }
        for p in &self.peds {
            if p.ret != 14 {
                v.push(format!("line {}: peds sscanf returned {} of 14", p.line, p.ret));
            }
        }
        for w in &self.weap {
            if w.ret != 6 {
                v.push(format!("line {}: weap sscanf returned {} of 6", w.line, w.ret));
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_and_fallbacks() {
        let src =
            b"# c\r\nobjs\r\n100, a, b, 1, 150, 4\r\n101, c, d, 2.5, 0\r\n102 e f 300 4\r\nbad\r\nend\r\n\
anim\r\n200, x, y, z\r\n201, x, y, z, 50, 1\r\nend\r\ntxdp\r\nkid, mum\r\nend\r\njunk\r\n";
        let ide = Ide::parse(src);
        assert_eq!(ide.objs.len(), 3);
        // "100 a b 1 150 4": the new format reads draw 1 (< 4), so the old one runs: type 1.
        assert_eq!(ide.objs[0].obj_type, Some(1));
        assert_eq!(ide.objs[0].draw_distance, Some(150.0));
        assert_eq!(ide.objs[0].flags, Some(4));
        // "101 c d 2.5 0": draw 2.5 < 4; %d reads type 2 from "2.5", then the type-2 format reads
        // ".5" and "0" as the two distances and finds no flags: flags keep 0 from the first call.
        assert_eq!(ide.objs[1].obj_type, Some(2));
        assert_eq!(ide.objs[1].draw_distance, Some(0.5));
        assert_eq!(ide.objs[1].flags, Some(0));
        assert_eq!(ide.objs[2].draw_distance, Some(300.0));
        // "bad" (objs) and "200 x y z" (anim, 4 of 6 fields).
        assert_eq!(ide.rejected.len(), 2);
        assert_eq!(ide.anim.len(), 1);
        assert_eq!(ide.anim[0].draw_distance, 50.0);
        assert_eq!(ide.txdp[0].parent.as_deref(), Some(&b"mum"[..]));
        assert_eq!(ide.ignored.len(), 1);
    }
}
