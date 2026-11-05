//! Game data definition files (FMT-2): IDE, IPL (text and binary), `gta.dat`, `handling.cfg`,
//! `timecyc.dat`, the other `data/*.dat` tables and the path node files.
//!
//! Text files are split into lines with the exact reader the exe uses ([`crate::lines`]) and
//! every field is read with the same `sscanf` format and control flow as the exe's loader
//! ([`crate::scanf`]), taken from the 1.0 US code (Ghidra decompile, cross-checked with
//! gta-reversed). The parsers keep what the loader reads, not what it later derives (model
//! lookups, enum conversions), so FMT-2 part b can compare the derived tables separately.
//!
//! Fields are `Option` where the exe's `sscanf` may leave the destination unassigned; `None`
//! then means "the variable keeps its previous (often uninitialised) value". Names are kept as
//! raw bytes (`%s` copies without a length limit; the exe's 24-byte buffers would overflow on
//! longer names, which do not occur in the install).
//!
//! Every parsed file implements [`DataFile`]: per-file counts and a canonical text dump with one
//! line per record, fields in file order, floats in Rust's shortest round-trip form.

pub mod gta_dat;
pub mod ide;
pub mod ipl;
pub mod ipl_bin;
pub mod nodes;

/// Other tables (weapons, shops, streaming ini, fonts, train tracks, ...).
pub mod misc;
/// Vehicle and population tables.
pub mod vehicle;
/// World and rendering tables.
pub mod world;

/// Parses one data file into its [`DataFile`] form.
pub type Parser = fn(&[u8]) -> crate::Result<Box<dyn DataFile>>;

/// Finds the parser for a file of the install. `rel_path` is the path relative to the game
/// directory, lower case, with `/` separators (e.g. `data/handling.cfg`, `data/maps/la/lan.ide`).
/// Files inside IMG archives are looked up by `img/<entry name>` (e.g. `img/nodes12.dat`,
/// `img/lae_stream0.ipl`). Returns `None` for files no parser covers.
pub fn parser_for(rel_path: &str) -> Option<Parser> {
    if let Some(p) = core_lookup(rel_path) {
        return Some(p);
    }
    vehicle::lookup(rel_path)
        .or_else(|| world::lookup(rel_path))
        .or_else(|| misc::lookup(rel_path))
}

fn core_lookup(rel_path: &str) -> Option<Parser> {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    if rel_path.starts_with("img/") {
        if name.ends_with(".ipl") {
            return Some(|d| Ok(Box::new(ipl_bin::BinaryIpl::parse(d)?)));
        }
        if name.starts_with("nodes") && name.ends_with(".dat") {
            return Some(|d| Ok(Box::new(nodes::PathNodes::parse(d)?)));
        }
        return None;
    }
    if !rel_path.starts_with("data/") {
        return None;
    }
    if rel_path.starts_with("data/paths/nodes") && name.ends_with(".dat") {
        return Some(|d| Ok(Box::new(nodes::PathNodes::parse(d)?)));
    }
    if name.ends_with(".ide") {
        return Some(|d| Ok(Box::new(ide::Ide::parse(d))));
    }
    if name.ends_with(".ipl") || name.ends_with(".zon") {
        return Some(|d| Ok(Box::new(ipl::Ipl::parse(d))));
    }
    if rel_path == "data/gta.dat" || rel_path == "data/default.dat" || rel_path == "data/gta_quick.dat" {
        return Some(|d| Ok(Box::new(gta_dat::LevelFile::parse(d))));
    }
    None
}

use crate::scanf::{Scan, Val, sscanf};
use std::fmt::Write;

/// Common interface of every parsed data file.
pub trait DataFile {
    /// Record counts for reports (`(label, count)`).
    fn counts(&self) -> Vec<(String, u64)>;
    /// Canonical text dump, one record per line.
    fn dump(&self) -> String;
    /// Lines the exe reads differently from what the file's author evidently intended
    /// (short `sscanf` results, ignored lines, ...). Informational, not errors.
    fn notes(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A value that can be written into a canonical dump.
pub trait DumpVal {
    fn dump_to(&self, out: &mut String);
}

impl DumpVal for i32 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for u32 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for i16 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for u16 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for i8 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for u8 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for usize {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }
}
impl DumpVal for bool {
    fn dump_to(&self, out: &mut String) {
        out.push_str(if *self { "1" } else { "0" });
    }
}
impl DumpVal for f32 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self:?}");
    }
}
impl DumpVal for f64 {
    fn dump_to(&self, out: &mut String) {
        let _ = write!(out, "{self:?}");
    }
}
impl DumpVal for [u8] {
    fn dump_to(&self, out: &mut String) {
        dump_bytes(self, out);
    }
}
impl DumpVal for Vec<u8> {
    fn dump_to(&self, out: &mut String) {
        dump_bytes(self, out);
    }
}
impl<T: DumpVal> DumpVal for Option<T> {
    fn dump_to(&self, out: &mut String) {
        match self {
            Some(v) => v.dump_to(out),
            None => out.push('~'),
        }
    }
}
impl<T: DumpVal, const N: usize> DumpVal for [T; N] {
    fn dump_to(&self, out: &mut String) {
        out.push('[');
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            v.dump_to(out);
        }
        out.push(']');
    }
}
impl<T: DumpVal + ?Sized> DumpVal for &T {
    fn dump_to(&self, out: &mut String) {
        (**self).dump_to(out);
    }
}

/// Writes a byte string quoted, escaping `"`, `\`, and bytes outside printable ASCII as `\xNN`.
pub fn dump_bytes(b: &[u8], out: &mut String) {
    out.push('"');
    for &c in b {
        match c {
            b'"' | b'\\' => {
                out.push('\\');
                out.push(c as char);
            }
            0x20..=0x7E => out.push(c as char),
            _ => {
                let _ = write!(out, "\\x{c:02x}");
            }
        }
    }
    out.push('"');
}

/// Appends one dump record: `kind name=value name=value ...\n`.
#[macro_export]
#[doc(hidden)]
macro_rules! dump_rec {
    ($out:expr, $kind:expr $(, $name:ident = $val:expr)* $(,)?) => {{
        let out: &mut String = $out;
        out.push_str($kind);
        $(
            out.push(' ');
            out.push_str(stringify!($name));
            out.push('=');
            $crate::data::DumpVal::dump_to(&$val, out);
        )*
        out.push('\n');
    }};
}
pub(crate) use dump_rec;

/// Lossy text of a byte string (for messages).
pub(crate) fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The local variables of a loader, as successive `sscanf` calls leave them.
///
/// Loaders often call `sscanf` several times on the same line with the same destination
/// variables; a later call only overwrites the variables it reaches. `Vars` mirrors that: each
/// scan assigns its reached slots to the named variables.
#[derive(Debug, Clone, Default)]
pub(crate) struct Vars(Vec<(&'static str, Val)>);

impl Vars {
    /// Sets a variable's initial value (the loader's initialiser).
    pub(crate) fn init(mut self, name: &'static str, v: Val) -> Self {
        self.set(name, v);
        self
    }

    pub(crate) fn set(&mut self, name: &'static str, v: Val) {
        match self.0.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 = v,
            None => self.0.push((name, v)),
        }
    }

    /// Runs `sscanf(line, fmt)` and assigns the reached slots to `names` (in format order;
    /// `""` = a variable the parser does not keep). Returns the `sscanf` return value.
    pub(crate) fn scan(&mut self, line: &[u8], fmt: &str, names: &[&'static str]) -> i32 {
        let s = sscanf(line, fmt);
        debug_assert_eq!(s.vals.len(), names.len(), "slot names for {fmt}");
        for (v, n) in s.vals.into_iter().zip(names) {
            if let (Some(v), false) = (v, n.is_empty()) {
                self.set(n, v);
            }
        }
        s.ret
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Val> {
        self.0.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }
    pub(crate) fn i(&self, name: &str) -> Option<i32> {
        self.get(name).and_then(Val::as_i32)
    }
    pub(crate) fn u(&self, name: &str) -> Option<u32> {
        self.get(name).and_then(Val::as_u32)
    }
    pub(crate) fn f(&self, name: &str) -> Option<f32> {
        self.get(name).and_then(Val::as_f32)
    }
    pub(crate) fn s(&self, name: &str) -> Option<Vec<u8>> {
        self.get(name).and_then(Val::as_bytes).map(<[u8]>::to_vec)
    }
}

/// A generic record: the format(s) a loader applied to one line and what they returned. Used
/// for records whose fields depend on a type field (2DFX, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct ScanRec {
    /// 1-based line number.
    pub line: usize,
    /// `(format, sscanf result)` in call order.
    pub scans: Vec<(&'static str, Scan)>,
}

impl ScanRec {
    pub(crate) fn run(line: usize, text: &[u8], fmts: &[&'static str]) -> Self {
        ScanRec {
            line,
            scans: fmts.iter().map(|f| (*f, sscanf(text, f))).collect(),
        }
    }

    pub(crate) fn push(&mut self, text: &[u8], fmt: &'static str) -> &Scan {
        self.scans.push((fmt, sscanf(text, fmt)));
        &self.scans.last().unwrap().1
    }

    pub(crate) fn dump_to(&self, out: &mut String) {
        for (i, (fmt, s)) in self.scans.iter().enumerate() {
            let _ = write!(out, " scan{i}={fmt:?} ret={} vals=[", s.ret);
            dump_vals(&s.vals, out);
            out.push(']');
        }
    }
}

/// Writes scan values separated by spaces (`~` for unassigned).
pub(crate) fn dump_vals(vals: &[Option<Val>], out: &mut String) {
    for (j, v) in vals.iter().enumerate() {
        if j > 0 {
            out.push(' ');
        }
        match v {
            None => out.push('~'),
            Some(Val::Int(x)) => x.dump_to(out),
            Some(Val::Float(x)) => x.dump_to(out),
            Some(Val::Double(x)) => x.dump_to(out),
            Some(Val::Str(x)) => x.dump_to(out),
        }
    }
}

/// The values of named loader variables after one line was processed. `None` = never assigned
/// so far (the exe's variable holds stack garbage or its initialiser was not modelled).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fields(pub Vec<(&'static str, Option<Val>)>);

impl Fields {
    pub(crate) fn snapshot(v: &Vars, names: &[&'static str]) -> Fields {
        Fields(
            names
                .iter()
                .filter(|n| !n.is_empty())
                .map(|n| (*n, v.get(n).cloned()))
                .collect(),
        )
    }

    pub fn get(&self, name: &str) -> Option<&Val> {
        self.0
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| v.as_ref())
    }
    pub fn i32(&self, name: &str) -> Option<i32> {
        self.get(name).and_then(Val::as_i32)
    }
    pub fn u32(&self, name: &str) -> Option<u32> {
        self.get(name).and_then(Val::as_u32)
    }
    pub fn f32(&self, name: &str) -> Option<f32> {
        self.get(name).and_then(Val::as_f32)
    }
    pub fn bytes(&self, name: &str) -> Option<&[u8]> {
        self.get(name).and_then(Val::as_bytes)
    }

    /// Appends ` name=value` pairs (`~` for unassigned).
    pub(crate) fn dump_to(&self, out: &mut String) {
        for (n, v) in &self.0 {
            out.push(' ');
            out.push_str(n);
            out.push('=');
            match v {
                None => out.push('~'),
                Some(Val::Int(x)) => x.dump_to(out),
                Some(Val::Float(x)) => x.dump_to(out),
                Some(Val::Double(x)) => x.dump_to(out),
                Some(Val::Str(x)) => x.dump_to(out),
            }
        }
    }
}

/// One `sscanf`-read line: the return value and the loader variables after the call.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rec {
    /// 1-based physical line number.
    pub line: usize,
    /// `sscanf` return value.
    pub ret: i32,
    pub fields: Fields,
}

impl Rec {
    pub(crate) fn dump_to(&self, kind: &str, out: &mut String) {
        use std::fmt::Write;
        let _ = write!(out, "{kind} line={} ret={}", self.line, self.ret);
        self.fields.dump_to(out);
        out.push('\n');
    }
}
