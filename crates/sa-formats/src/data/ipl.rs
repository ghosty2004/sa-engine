//! Text IPL (item placement) files and `.zon` files, as `CFileLoader::LoadScene` (0x5B8700)
//! reads them.
//!
//! The file is opened `"rb"` and read with `LoadLine` (0x536F80). Lines starting with NUL or `#`
//! are skipped. Outside a section, a line whose first four bytes are `inst mult zone cull path
//! occl grge enex pick cars jump tcyc auzo` opens that section; other lines are ignored. Inside a
//! section, a line starting with `end` closes it; other lines go to the section's loader:
//!
//! | section | loader | format | accepted when |
//! |---|---|---|---|
//! | inst | 0x538690 | `%d %s %d %f %f %f %f %f %f %f %d` | always (no return check) |
//! | zone | 0x5B4AB0 | `%s %d %f %f %f %f %f %f %d %s` | ret == 10 |
//! | cull | 0x5B4B40 | `%f %f %f %f %f %f %f %f %f %d %f %f %f %f` (ret == 14, mirror zone), else `%f %f %f %f %f %f %f %f %f %d %d` (no check) | |
//! | occl | 0x5B4C80 | `%f %f %f %f %f %f %f %f %f %d ` (rot y/x and flags pre-set to 0) | always |
//! | grge | 0x5B4530 | `%f %f %f %f %f %f %f %f %d %d %s` | ret == 11 |
//! | enex | 0x5B8030 | `%f %f %f %f %f %f %f %f %f %f %f %d %d %s %d %d %d %d` (peds 2, time on 0, off 24 pre-set) | always |
//! | pick | 0x5B47B0 | `%d %f %f %f` | ret == 4 (and a known pickup weapon id) |
//! | cars | 0x5B4740 | `%f %f %f %f %d %d %d %d %d %d %d %d` | ret == 12 |
//! | jump | 0x5B45D0 | 15 `%f` + `%d` | ret == 16 |
//! | tcyc | 0x5B81D0 | `%f %f %f %f %f %f %d %d %f %f %f %f` (last three pre-set 100, 1, 1; when ret < 12 the last takes the 11th's value) | always |
//! | auzo | 0x5B4D70 | `%s %d %d %f %f %f %f %f %f` (box, ret == 9) else `%s %d %d %f %f %f %f` (sphere) | |
//! | mult | — | no loader: lines are ignored | |
//! | path | — | **the first non-`end` line of a `path` section ends the whole file** (the loop breaks) | |
//!
//! The `occl` loader also receives the file name as passed by `LoadLevel` and sets the
//! "interior" flag when the three bytes at `len-7 .. len-5` are `int` (see [`occl_interior`]).

use super::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

/// `inst` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Inst {
    pub line: usize,
    pub ret: i32,
    pub id: Option<i32>,
    pub model: Option<Vec<u8>>,
    /// Interior/area id (with flags in the upper bits).
    pub interior: Option<i32>,
    pub pos: [Option<f32>; 3],
    pub rot: [Option<f32>; 4],
    pub lod: Option<i32>,
}

/// `zone` entry (map.zon/info.zon).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Zone {
    pub line: usize,
    pub ret: i32,
    pub name: Option<Vec<u8>>,
    pub zone_type: Option<i32>,
    pub min: [Option<f32>; 3],
    pub max: [Option<f32>; 3],
    pub level: Option<i32>,
    pub text_label: Option<Vec<u8>>,
}

/// `cull` entry. `ret14` is the first `sscanf` result; when it is 14 the mirror form is used
/// (the 14 values in `f`/`flags`/`mirror`), else the second form (`flags` + `wanted_drop`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cull {
    pub line: usize,
    pub ret14: i32,
    pub ret11: Option<i32>,
    /// The nine leading floats: centre xyz, then six more (the two edge vectors and z range).
    pub f: [Option<f32>; 9],
    pub flags: Option<i32>,
    /// Mirror plane (4 floats) for the 14-field form.
    pub mirror: [Option<f32>; 4],
    /// Last field of the 11-field form (pre-set to 0).
    pub wanted_drop: i32,
}

/// `occl` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Occl {
    pub line: usize,
    pub ret: i32,
    /// centre x, centre y, bottom z, width, length, height, rot z.
    pub f: [Option<f32>; 7],
    pub rot_y: f32,
    pub rot_x: f32,
    pub flags: i32,
}

/// `grge` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Grge {
    pub line: usize,
    pub ret: i32,
    pub f: [Option<f32>; 8],
    pub flags: Option<i32>,
    pub garage_type: Option<i32>,
    pub name: Option<Vec<u8>>,
}

/// `enex` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Enex {
    pub line: usize,
    pub ret: i32,
    /// enter xyz, enter angle, range x/y, unused, exit xyz, exit angle.
    pub f: [Option<f32>; 11],
    pub area: Option<i32>,
    pub flags: Option<i32>,
    /// The `%s` token as read (with its quotes).
    pub name_token: Option<Vec<u8>>,
    pub sky_colour: Option<i32>,
    pub num_peds: i32,
    pub time_on: i32,
    pub time_off: i32,
}

impl Enex {
    /// The name the loader passes on: `strrchr(token, '"')` is overwritten with NUL and the
    /// name starts at the token's second byte. So `"MOTEL1"` gives `MOTEL1`; a token whose
    /// only quote is its first byte (a quoted name with a space, cut by `%s`) gives the rest of
    /// the token; `None` (a null pointer) when the token has no `"`.
    pub fn name(&self) -> Option<Vec<u8>> {
        let t = self.name_token.as_ref()?;
        let q = t.iter().rposition(|&c| c == b'"')?;
        let rest = if q == 0 { &t[1..] } else { &t[1..q] };
        Some(rest.to_vec())
    }
}

/// `pick` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pick {
    pub line: usize,
    pub ret: i32,
    pub weapon: Option<i32>,
    pub pos: [Option<f32>; 3],
}

/// `cars` entry (car generator).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CarGen {
    pub line: usize,
    pub ret: i32,
    pub pos: [Option<f32>; 3],
    pub angle: Option<f32>,
    /// model, colour 1, colour 2, flags, alarm, door lock, min delay, max delay.
    pub i: [Option<i32>; 8],
}

/// `jump` entry (stunt jump).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Jump {
    pub line: usize,
    pub ret: i32,
    pub f: [Option<f32>; 15],
    pub reward: Option<i32>,
}

/// `tcyc` entry (time cycle modifier box).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tcyc {
    pub line: usize,
    pub ret: i32,
    /// min xyz, max xyz.
    pub bounds: [Option<f32>; 6],
    pub farclip: Option<i32>,
    pub extra_colour: Option<i32>,
    pub extra_colour_intensity: Option<f32>,
    /// Pre-set 100.
    pub falloff: f32,
    /// Pre-set 1.
    pub lod_dist_mult: f32,
    /// As read (pre-set 1); the loader uses `lod_dist_mult` instead when `ret < 12`.
    pub last: f32,
}

/// `auzo` entry (audio zone). `box_ret` is the 9-field result; when it is not 9 the sphere
/// form ran and `sphere_ret`/`radius` are set.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Auzo {
    pub line: usize,
    pub box_ret: i32,
    pub sphere_ret: Option<i32>,
    pub name: Option<Vec<u8>>,
    pub id: Option<i32>,
    pub enabled: Option<i32>,
    pub f: [Option<f32>; 6],
    pub radius: Option<f32>,
}

/// A parsed text IPL / ZON file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ipl {
    pub inst: Vec<Inst>,
    pub zone: Vec<Zone>,
    pub cull: Vec<Cull>,
    pub occl: Vec<Occl>,
    pub grge: Vec<Grge>,
    pub enex: Vec<Enex>,
    pub pick: Vec<Pick>,
    pub cars: Vec<CarGen>,
    pub jump: Vec<Jump>,
    pub tcyc: Vec<Tcyc>,
    pub auzo: Vec<Auzo>,
    /// Lines in `mult` sections (no loader).
    pub mult_lines: usize,
    /// Line at which a `path` section ended the read, and the lines not read after it.
    pub stopped_at_path: Option<(usize, usize)>,
    /// Lines outside sections that are not headers (ignored).
    pub ignored: Vec<(usize, Vec<u8>)>,
}

/// The `occl` interior test of 0x5B4C80 on the file name as `LoadLevel` passes it (the rest of
/// the `IPL` line, which in `gta.dat` ends with the space that replaced `\n`).
pub fn occl_interior(path: &[u8]) -> bool {
    let n = path.len();
    n >= 7 && &path[n - 7..n - 4] == b"int"
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    None,
    Path,
    Inst,
    Mult,
    Zone,
    Cull,
    Occl,
    Grge,
    Enex,
    Pick,
    Cars,
    Jump,
    Tcyc,
    Auzo,
}

fn opt<const N: usize, T: Clone>(f: impl Fn(usize) -> Option<T>) -> [Option<T>; N] {
    std::array::from_fn(f)
}

impl Ipl {
    pub fn parse(data: &[u8]) -> Ipl {
        let mut ipl = Ipl::default();
        let mut section = Section::None;
        let lines = load_lines(data, Mode::Binary);
        let total = lines.len();
        for (idx, l) in lines.iter().enumerate() {
            let t = &l.text[..];
            let c = l.first();
            if c == 0 || c == b'#' {
                continue;
            }
            if section == Section::None {
                let h = |s: &[u8; 4]| t.len() >= 4 && &t[..4] == s;
                let s = [
                    (b"inst", Section::Inst),
                    (b"mult", Section::Mult),
                    (b"zone", Section::Zone),
                    (b"cull", Section::Cull),
                    (b"path", Section::Path),
                    (b"occl", Section::Occl),
                    (b"grge", Section::Grge),
                    (b"enex", Section::Enex),
                    (b"pick", Section::Pick),
                    (b"cars", Section::Cars),
                    (b"jump", Section::Jump),
                    (b"tcyc", Section::Tcyc),
                    (b"auzo", Section::Auzo),
                ]
                .into_iter()
                .find(|(n, _)| h(n));
                match s {
                    Some((_, s)) => section = s,
                    None => ipl.ignored.push((l.number, t.to_vec())),
                }
                continue;
            }
            if t.starts_with(b"end") {
                section = Section::None;
                continue;
            }
            let n = l.number;
            match section {
                Section::None => unreachable!(),
                Section::Path => {
                    ipl.stopped_at_path = Some((n, total - idx - 1));
                    break;
                }
                Section::Mult => ipl.mult_lines += 1,
                Section::Inst => ipl.inst.push(parse_inst(n, t)),
                Section::Zone => {
                    let mut v = Vars::default();
                    let ret = v.scan(
                        t,
                        "%s %d %f %f %f %f %f %f %d %s",
                        &[
                            "name", "type", "x1", "y1", "z1", "x2", "y2", "z2", "level", "text",
                        ],
                    );
                    ipl.zone.push(Zone {
                        line: n,
                        ret,
                        name: v.s("name"),
                        zone_type: v.i("type"),
                        min: [v.f("x1"), v.f("y1"), v.f("z1")],
                        max: [v.f("x2"), v.f("y2"), v.f("z2")],
                        level: v.i("level"),
                        text_label: v.s("text"),
                    });
                }
                Section::Cull => {
                    const F: [&str; 9] = ["f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8"];
                    let mut v = Vars::default().init("wd", Val::Int(0));
                    let mut names: Vec<&'static str> = F.to_vec();
                    names.extend(["flags", "m0", "m1", "m2", "m3"]);
                    let ret14 = v.scan(t, "%f %f %f %f %f %f %f %f %f %d %f %f %f %f", &names);
                    let mut ret11 = None;
                    if ret14 != 14 {
                        let mut names: Vec<&'static str> = F.to_vec();
                        names.extend(["flags", "wd"]);
                        ret11 = Some(v.scan(t, "%f %f %f %f %f %f %f %f %f %d %d", &names));
                    }
                    ipl.cull.push(Cull {
                        line: n,
                        ret14,
                        ret11,
                        f: opt(|i| v.f(F[i])),
                        flags: v.i("flags"),
                        mirror: [v.f("m0"), v.f("m1"), v.f("m2"), v.f("m3")],
                        wanted_drop: v.i("wd").unwrap(),
                    });
                }
                Section::Occl => {
                    const F: [&str; 7] = ["f0", "f1", "f2", "f3", "f4", "f5", "f6"];
                    let mut v = Vars::default()
                        .init("ry", Val::Float(0.0))
                        .init("rx", Val::Float(0.0))
                        .init("flags", Val::Int(0));
                    let mut names: Vec<&'static str> = F.to_vec();
                    names.extend(["ry", "rx", "flags"]);
                    let ret = v.scan(t, "%f %f %f %f %f %f %f %f %f %d ", &names);
                    ipl.occl.push(Occl {
                        line: n,
                        ret,
                        f: opt(|i| v.f(F[i])),
                        rot_y: v.f("ry").unwrap(),
                        rot_x: v.f("rx").unwrap(),
                        flags: v.i("flags").unwrap(),
                    });
                }
                Section::Grge => {
                    const F: [&str; 8] = ["f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7"];
                    let mut v = Vars::default();
                    let mut names: Vec<&'static str> = F.to_vec();
                    names.extend(["flags", "type", "name"]);
                    let ret = v.scan(t, "%f %f %f %f %f %f %f %f %d %d %s", &names);
                    ipl.grge.push(Grge {
                        line: n,
                        ret,
                        f: opt(|i| v.f(F[i])),
                        flags: v.i("flags"),
                        garage_type: v.i("type"),
                        name: v.s("name"),
                    });
                }
                Section::Enex => {
                    const F: [&str; 11] = ["f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10"];
                    let mut v = Vars::default()
                        .init("peds", Val::Int(2))
                        .init("on", Val::Int(0))
                        .init("off", Val::Int(24));
                    let mut names: Vec<&'static str> = F.to_vec();
                    names.extend(["area", "flags", "name", "sky", "peds", "on", "off"]);
                    let ret = v.scan(t, "%f %f %f %f %f %f %f %f %f %f %f %d %d %s %d %d %d %d", &names);
                    ipl.enex.push(Enex {
                        line: n,
                        ret,
                        f: opt(|i| v.f(F[i])),
                        area: v.i("area"),
                        flags: v.i("flags"),
                        name_token: v.s("name"),
                        sky_colour: v.i("sky"),
                        num_peds: v.i("peds").unwrap(),
                        time_on: v.i("on").unwrap(),
                        time_off: v.i("off").unwrap(),
                    });
                }
                Section::Pick => {
                    let mut v = Vars::default();
                    let ret = v.scan(t, "%d %f %f %f", &["w", "x", "y", "z"]);
                    ipl.pick.push(Pick {
                        line: n,
                        ret,
                        weapon: v.i("w"),
                        pos: [v.f("x"), v.f("y"), v.f("z")],
                    });
                }
                Section::Cars => {
                    const I: [&str; 8] = ["i0", "i1", "i2", "i3", "i4", "i5", "i6", "i7"];
                    let mut v = Vars::default();
                    let mut names: Vec<&'static str> = vec!["x", "y", "z", "a"];
                    names.extend(I);
                    let ret = v.scan(t, "%f %f %f %f %d %d %d %d %d %d %d %d", &names);
                    ipl.cars.push(CarGen {
                        line: n,
                        ret,
                        pos: [v.f("x"), v.f("y"), v.f("z")],
                        angle: v.f("a"),
                        i: opt(|k| v.i(I[k])),
                    });
                }
                Section::Jump => {
                    const F: [&str; 15] = [
                        "f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
                        "f13", "f14",
                    ];
                    let mut v = Vars::default();
                    let mut names: Vec<&'static str> = F.to_vec();
                    names.push("reward");
                    let ret = v.scan(t, "%f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %d", &names);
                    ipl.jump.push(Jump {
                        line: n,
                        ret,
                        f: opt(|i| v.f(F[i])),
                        reward: v.i("reward"),
                    });
                }
                Section::Tcyc => {
                    let mut v = Vars::default()
                        .init("falloff", Val::Float(100.0))
                        .init("lod", Val::Float(1.0))
                        .init("last", Val::Float(1.0));
                    let ret = v.scan(
                        t,
                        "%f %f %f %f %f %f %d %d %f %f %f %f",
                        &[
                            "x1", "y1", "z1", "x2", "y2", "z2", "far", "extra", "pct", "falloff", "lod",
                            "last",
                        ],
                    );
                    ipl.tcyc.push(Tcyc {
                        line: n,
                        ret,
                        bounds: [v.f("x1"), v.f("y1"), v.f("z1"), v.f("x2"), v.f("y2"), v.f("z2")],
                        farclip: v.i("far"),
                        extra_colour: v.i("extra"),
                        extra_colour_intensity: v.f("pct"),
                        falloff: v.f("falloff").unwrap(),
                        lod_dist_mult: v.f("lod").unwrap(),
                        last: v.f("last").unwrap(),
                    });
                }
                Section::Auzo => {
                    let mut v = Vars::default();
                    let box_ret = v.scan(
                        t,
                        "%s %d %d %f %f %f %f %f %f",
                        &["name", "id", "on", "f0", "f1", "f2", "f3", "f4", "f5"],
                    );
                    let mut sphere_ret = None;
                    if box_ret != 9 {
                        sphere_ret = Some(v.scan(
                            t,
                            "%s %d %d %f %f %f %f",
                            &["name", "id", "on", "f0", "f1", "f2", "r"],
                        ));
                    }
                    ipl.auzo.push(Auzo {
                        line: n,
                        box_ret,
                        sphere_ret,
                        name: v.s("name"),
                        id: v.i("id"),
                        enabled: v.i("on"),
                        f: [v.f("f0"), v.f("f1"), v.f("f2"), v.f("f3"), v.f("f4"), v.f("f5")],
                        radius: v.f("r"),
                    });
                }
            }
        }
        ipl
    }
}

fn parse_inst(line: usize, t: &[u8]) -> Inst {
    let mut v = Vars::default();
    let ret = v.scan(
        t,
        "%d %s %d %f %f %f %f %f %f %f %d",
        &["id", "model", "int", "x", "y", "z", "rx", "ry", "rz", "rw", "lod"],
    );
    Inst {
        line,
        ret,
        id: v.i("id"),
        model: v.s("model"),
        interior: v.i("int"),
        pos: [v.f("x"), v.f("y"), v.f("z")],
        rot: [v.f("rx"), v.f("ry"), v.f("rz"), v.f("rw")],
        lod: v.i("lod"),
    }
}

impl DataFile for Ipl {
    fn counts(&self) -> Vec<(String, u64)> {
        let mut v = vec![
            ("ipl: inst".to_string(), self.inst.len() as u64),
            ("ipl: zone".into(), self.zone.len() as u64),
            ("ipl: cull".into(), self.cull.len() as u64),
            ("ipl: occl".into(), self.occl.len() as u64),
            ("ipl: grge".into(), self.grge.len() as u64),
            ("ipl: enex".into(), self.enex.len() as u64),
            ("ipl: pick".into(), self.pick.len() as u64),
            ("ipl: cars".into(), self.cars.len() as u64),
            ("ipl: jump".into(), self.jump.len() as u64),
            ("ipl: tcyc".into(), self.tcyc.len() as u64),
            ("ipl: auzo".into(), self.auzo.len() as u64),
            ("ipl: mult lines (ignored)".into(), self.mult_lines as u64),
            (
                "ipl: files cut short by a path section".into(),
                self.stopped_at_path.is_some() as u64,
            ),
            (
                "ipl: lines not read after a path section".into(),
                self.stopped_at_path.map_or(0, |(_, n)| n as u64),
            ),
        ];
        v.retain(|(_, n)| *n > 0);
        v
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.inst {
            dump_rec!(
                &mut o,
                "inst",
                line = r.line,
                ret = r.ret,
                id = r.id,
                model = r.model,
                interior = r.interior,
                pos = r.pos,
                rot = r.rot,
                lod = r.lod
            );
        }
        for r in &self.zone {
            dump_rec!(
                &mut o,
                "zone",
                line = r.line,
                ret = r.ret,
                name = r.name,
                ztype = r.zone_type,
                min = r.min,
                max = r.max,
                level = r.level,
                text = r.text_label
            );
        }
        for r in &self.cull {
            dump_rec!(
                &mut o,
                "cull",
                line = r.line,
                ret14 = r.ret14,
                ret11 = r.ret11,
                f = r.f,
                flags = r.flags,
                mirror = r.mirror,
                wanted_drop = r.wanted_drop
            );
        }
        for r in &self.occl {
            dump_rec!(
                &mut o,
                "occl",
                line = r.line,
                ret = r.ret,
                f = r.f,
                rot_y = r.rot_y,
                rot_x = r.rot_x,
                flags = r.flags
            );
        }
        for r in &self.grge {
            dump_rec!(
                &mut o,
                "grge",
                line = r.line,
                ret = r.ret,
                f = r.f,
                flags = r.flags,
                gtype = r.garage_type,
                name = r.name
            );
        }
        for r in &self.enex {
            dump_rec!(
                &mut o,
                "enex",
                line = r.line,
                ret = r.ret,
                f = r.f,
                area = r.area,
                flags = r.flags,
                token = r.name_token,
                name = r.name(),
                sky = r.sky_colour,
                peds = r.num_peds,
                on = r.time_on,
                off = r.time_off
            );
        }
        for r in &self.pick {
            dump_rec!(
                &mut o,
                "pick",
                line = r.line,
                ret = r.ret,
                weapon = r.weapon,
                pos = r.pos
            );
        }
        for r in &self.cars {
            dump_rec!(
                &mut o,
                "cars",
                line = r.line,
                ret = r.ret,
                pos = r.pos,
                angle = r.angle,
                i = r.i
            );
        }
        for r in &self.jump {
            dump_rec!(
                &mut o,
                "jump",
                line = r.line,
                ret = r.ret,
                f = r.f,
                reward = r.reward
            );
        }
        for r in &self.tcyc {
            dump_rec!(
                &mut o,
                "tcyc",
                line = r.line,
                ret = r.ret,
                bounds = r.bounds,
                farclip = r.farclip,
                extra = r.extra_colour,
                intensity = r.extra_colour_intensity,
                falloff = r.falloff,
                lod = r.lod_dist_mult,
                last = r.last
            );
        }
        for r in &self.auzo {
            dump_rec!(
                &mut o,
                "auzo",
                line = r.line,
                box_ret = r.box_ret,
                sphere_ret = r.sphere_ret,
                name = r.name,
                id = r.id,
                enabled = r.enabled,
                f = r.f,
                radius = r.radius
            );
        }
        if let Some((n, rest)) = self.stopped_at_path {
            dump_rec!(&mut o, "path-stop", line = n, unread = rest);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        let mut bad = |kind: &str, line: usize, ret: i32, want: i32| {
            if ret != want {
                v.push(format!("line {line}: {kind} sscanf returned {ret} of {want}"));
            }
        };
        for r in &self.inst {
            bad("inst", r.line, r.ret, 11);
        }
        for r in &self.zone {
            bad("zone (not added)", r.line, r.ret, 10);
        }
        for r in &self.grge {
            bad("grge (not added)", r.line, r.ret, 11);
        }
        for r in &self.enex {
            bad("enex", r.line, r.ret, 18);
        }
        for r in &self.pick {
            bad("pick (not added)", r.line, r.ret, 4);
        }
        for r in &self.cars {
            bad("cars (not added)", r.line, r.ret, 12);
        }
        for r in &self.jump {
            bad("jump (not added)", r.line, r.ret, 16);
        }
        for r in &self.occl {
            bad("occl", r.line, r.ret, 10);
        }
        for r in &self.cull {
            if r.ret14 != 14 {
                bad("cull (11-field form)", r.line, r.ret11.unwrap_or(-2), 11);
            }
        }
        for r in &self.auzo {
            if let Some(s) = r.sphere_ret {
                bad("auzo (sphere form)", r.line, s, 7);
            }
        }
        if let Some((n, rest)) = self.stopped_at_path {
            v.push(format!(
                "line {n}: path section ends the read; {rest} following lines are not read"
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections() {
        let src = b"inst\r\n1, a, 0, 1, 2, 3, 0, 0, 0, 1, -1\r\nend\r\nenex\r\n\
1,2,3,4,5,6,7,8,9,10,11, 0, 4, \"MOTEL1\", 0\r\nend\r\n\
grge\r\n1,2,3,4,5,6,7,8, 1, 2, \r\nend\r\npath\r\n1, -1\r\ninst\r\nend\r\n";
        let ipl = Ipl::parse(src);
        assert_eq!(ipl.inst.len(), 1);
        assert_eq!(ipl.inst[0].ret, 11);
        assert_eq!(ipl.inst[0].lod, Some(-1));
        assert_eq!(ipl.enex[0].name().as_deref(), Some(&b"MOTEL1"[..]));
        assert_eq!(ipl.enex[0].num_peds, 2);
        assert_eq!(ipl.enex[0].time_off, 24);
        assert_eq!(ipl.grge[0].ret, 10);
        assert_eq!(ipl.stopped_at_path, Some((11, 2)));
    }

    #[test]
    fn occl_interior_test() {
        assert!(occl_interior(b"DATA\\MAPS\\occluint.ipl"));
        // gta.dat lines end with the space that replaced '\n': the test misses.
        assert!(!occl_interior(b"DATA\\MAPS\\occluint.ipl "));
    }

    #[test]
    fn enex_name_without_closing_quote() {
        let e = Enex {
            name_token: Some(b"\"Pizza".to_vec()),
            ..Default::default()
        };
        assert_eq!(e.name().as_deref(), Some(&b"Pizza"[..]));
        let e = Enex {
            name_token: Some(b"abc".to_vec()),
            ..Default::default()
        };
        assert_eq!(e.name(), None);
    }
}
