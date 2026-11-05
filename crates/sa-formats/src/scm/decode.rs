//! SCM instruction decoder.
//!
//! # Instruction format (1.0 US)
//! `CRunningScript::ProcessOneCommand` 0x469EB0 reads a `u16`: bit 15 is the NOT flag, the low 15
//! bits are the opcode, dispatched through the table at 0x8A6168 (27 ranges of 100). Each handler
//! then reads its own parameters with the reader functions below; there is no parameter table in
//! the exe. The per-opcode reads were derived from the exe by `tools/catalog/script_params.py`
//! (symbolic execution of every handler with its opcode) into [`super::opcodes`].
//!
//! Every parameter starts with a type byte. Payload by type, and which reader accepts it:
//!
//! | type | payload | C | S | V | T | G | `*` |
//! |---|---|---|---|---|---|---|---|
//! | 0x00 | end of argument list | | | | | | ends |
//! | 0x01 | `i32` | x | | | | | x |
//! | 0x02 / 0x03 | `u16` global / local variable | x | x | x | | 0x02 | x |
//! | 0x04 | `i8` | x | | | | | x |
//! | 0x05 | `i16` | x | | | | | x |
//! | 0x06 | `f32` | x | | | | | x |
//! | 0x07 / 0x08 | array: `u16` base, `u16` index variable, `u8` size, `u8` flags (bit 7: global index) | x | x | x | | 0x07 | x |
//! | 0x09 | 8-byte string | | | | x | | |
//! | 0x0A / 0x0B | `u16` global / local 8-byte string variable | | | x | x | | |
//! | 0x0C / 0x0D | 8-byte string array (6 bytes as 0x07) | | | x | x | | |
//! | 0x0E | `i8` length + bytes (a negative length reads nothing) | | | | x | | |
//! | 0x0F | 16-byte string | | | | x | | |
//! | 0x10 / 0x11 | `u16` global / local 16-byte string variable | | | x | x | | |
//! | 0x12 / 0x13 | 16-byte string array | | | x | x | | |
//!
//! Readers: C = `CollectParameters` 0x464080 (jump table 0x464224, types 1–8), S =
//! `StoreParameters` 0x464370 (0x4644DC, types 2, 3, 7, 8), V = `GetPointerToScriptVariable`
//! 0x464790 (0x4648B0/0x4648C4), T = `ReadTextLabelFromScript` 0x463D50 (0x464054, types
//! 9–0x13), G = `GetIndexOfGlobalVariable` 0x464700 (body 0x156F340, types 2 and 7), `*` =
//! `ReadParametersForNewlyStartedScript` 0x464500. A type a reader does not handle consumes only
//! the type byte (the exe then desynchronises); the decoder reproduces that and reports it.
//!
//! Jump targets (`GOTO`, `GOTO_IF_FALSE`, `GOSUB`, `START_NEW_SCRIPT`, `LAUNCH_MISSION`, `SWITCH_*`)
//! are `i32`: `CRunningScript::UpdatePC` 0x464DA0 treats `>= 0` as an offset into `ScriptSpace`
//! (main.scm) and `< 0` as `base - value` (mission / streamed script buffer).
//!
//! # Walk
//! Decoding follows control flow from each script's entry (main: the first byte after the
//! header; missions and streamed scripts: offset 0), so data placed in code is never decoded as
//! instructions. Bytes no path reaches are reported separately (zero fill, linearly decodable
//! dead code, or undecodable bytes).

use super::opcodes::OPCODES;
use super::{MainScm, StreamedEntry};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::sync::OnceLock;

/// The reader a parameter slot goes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    /// `CollectParameters`.
    Collect,
    /// `StoreParameters`.
    Store,
    /// `GetPointerToScriptVariable`.
    Var,
    /// `ReadTextLabelFromScript`.
    Text,
    /// `GetIndexOfGlobalVariable`.
    Global,
    /// `ReadParametersForNewlyStartedScript` (until a 0 type byte).
    VarArgs,
    /// Raw bytes the handler takes from `m_IP` itself (no type byte): `DECLARE_MISSION_FLAG`
    /// (3, type byte skipped unread) and `SAVE_STRING_TO_DEBUG_FILE` (128).
    Raw(u8),
}

impl Slot {
    fn accepts(self, ty: u8) -> bool {
        match self {
            Slot::Collect | Slot::VarArgs => (1..=8).contains(&ty),
            Slot::Store => matches!(ty, 2 | 3 | 7 | 8),
            Slot::Var => matches!(ty, 2 | 3 | 7 | 8 | 0x0A..=0x0D | 0x10..=0x13),
            Slot::Text => (9..=0x13).contains(&ty),
            Slot::Global => matches!(ty, 2 | 7),
            Slot::Raw(_) => true,
        }
    }
}

/// A parameter value.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    /// Type 0: end of a variable argument list.
    End,
    /// Types 1, 4, 5 (sign-extended).
    Int(i32),
    /// Type 6.
    Float(f32),
    /// Variable reference (types 2, 3, 0x0A, 0x0B, 0x10, 0x11).
    Var(u16),
    /// Array element (types 7, 8, 0x0C, 0x0D, 0x12, 0x13).
    Array { base: u16, index: u16, size: u8, flags: u8 },
    /// Type 9.
    Str8([u8; 8]),
    /// Type 0x0F.
    Str16([u8; 16]),
    /// Type 0x0E (length byte kept in `len`).
    Pascal { len: i8, bytes: Vec<u8> },
    /// A type the slot's reader does not handle: only the type byte is consumed.
    Unhandled,
    /// Inline bytes of a [`Slot::Raw`] operand (the first byte is in `Param::ty`).
    Raw(Vec<u8>),
}

/// A decoded parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: u8,
    pub slot: Slot,
    pub value: ParamValue,
}

impl Param {
    /// The `i32` of an immediate integer parameter.
    pub fn as_int(&self) -> Option<i32> {
        match self.value {
            ParamValue::Int(v) => Some(v),
            _ => None,
        }
    }

    /// Text of a string literal parameter, up to the first NUL.
    pub fn as_str(&self) -> Option<&[u8]> {
        match &self.value {
            ParamValue::Str8(s) => Some(crate::bytes::c_str(s)),
            ParamValue::Str16(s) => Some(crate::bytes::c_str(s)),
            ParamValue::Pascal { bytes, .. } => Some(bytes),
            _ => None,
        }
    }
}

/// A decoded instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    /// Offset inside the script's buffer.
    pub offset: usize,
    /// The raw opcode word (bit 15 = NOT flag).
    pub raw: u16,
    pub params: Vec<Param>,
    /// Size in bytes.
    pub len: usize,
}

impl Instruction {
    pub fn opcode(&self) -> u16 {
        self.raw & 0x7FFF
    }
    pub fn not_flag(&self) -> bool {
        self.raw & 0x8000 != 0
    }
}

/// Parameter reads of one handler path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Fixed(Slot, u16),
    /// `C?`/`S?`: a count the exe analysis could not determine.
    Unknown(Slot),
    VarArgs,
}

/// One opcode of the exe's table.
#[derive(Debug, Clone)]
pub struct OpcodeInfo {
    pub opcode: u16,
    pub name: &'static str,
    /// Address of the switch case in `ProcessCommands*`.
    pub handler: u32,
    /// The reads spec from the exe analysis (alternatives separated by `|`).
    pub reads: &'static str,
    /// Analysis status (`ok`, `path-dependent`, `unknown-count`).
    pub status: &'static str,
    alternatives: Vec<Vec<Tok>>,
}

impl OpcodeInfo {
    /// Whether the exe analysis gives exactly one parameter list.
    pub fn is_exact(&self) -> bool {
        self.status == "ok"
            && self.alternatives.len() == 1
            && !self.alternatives[0].iter().any(|t| matches!(t, Tok::Unknown(_)))
    }

    /// Candidate slot lists (alternatives with an unknown count expanded to 0..=32).
    fn candidates(&self) -> Vec<(Vec<Slot>, bool)> {
        let mut out = Vec::new();
        for alt in &self.alternatives {
            let mut partial: Vec<(Vec<Slot>, bool)> = vec![(Vec::new(), false)];
            for t in alt {
                let mut next = Vec::new();
                for (p, va) in &partial {
                    match t {
                        Tok::Fixed(s, n) => {
                            let mut q = p.clone();
                            q.extend(std::iter::repeat_n(*s, *n as usize));
                            next.push((q, *va));
                        }
                        Tok::VarArgs => next.push((p.clone(), true)),
                        Tok::Unknown(s) => {
                            for n in 0..=32 {
                                let mut q = p.clone();
                                q.extend(std::iter::repeat_n(*s, n));
                                next.push((q, *va));
                            }
                        }
                    }
                }
                partial = next;
            }
            out.extend(partial);
        }
        out.sort();
        out.dedup();
        out
    }
}

fn parse_alt(s: &str) -> Vec<Tok> {
    let mut v = Vec::new();
    for t in s.split_whitespace() {
        let tok = match t {
            "-" => continue,
            "V" => Tok::Fixed(Slot::Var, 1),
            "T" => Tok::Fixed(Slot::Text, 1),
            "G" => Tok::Fixed(Slot::Global, 1),
            "*" => Tok::VarArgs,
            "P" => continue,
            _ if t.starts_with('R') => Tok::Fixed(Slot::Raw(t[1..].parse().unwrap_or(0)), 1),
            _ => {
                let slot = if t.starts_with('C') { Slot::Collect } else { Slot::Store };
                match t[1..].parse::<u16>() {
                    Ok(n) => Tok::Fixed(slot, n),
                    Err(_) => Tok::Unknown(slot),
                }
            }
        };
        // `VarArgs` must come last; the exe's '*' readers do (START_NEW_SCRIPT and friends).
        v.push(tok);
    }
    v
}

/// The opcode table (index = opcode, `None` = no handler).
pub fn opcode_table() -> &'static [Option<OpcodeInfo>] {
    static T: OnceLock<Vec<Option<OpcodeInfo>>> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = vec![None; 0x8000];
        for &(op, name, handler, reads, status) in OPCODES {
            t[op as usize] = Some(OpcodeInfo {
                opcode: op,
                name,
                handler,
                reads,
                status,
                alternatives: reads.split('|').map(|a| parse_alt(a.trim())).collect(),
            });
        }
        t
    })
}

/// Decodes one parameter at `pos` for a slot. Returns `None` when the data ends.
fn read_param(code: &[u8], pos: usize, slot: Slot) -> Option<(Param, usize)> {
    let ty = *code.get(pos)?;
    if let Slot::Raw(n) = slot {
        let b = code.get(pos..pos + n as usize)?.to_vec();
        return Some((Param { ty, slot, value: ParamValue::Raw(b) }, pos + n as usize));
    }
    let p = pos + 1;
    let le16 = |at: usize| -> Option<u16> { Some(u16::from_le_bytes([*code.get(at)?, *code.get(at + 1)?])) };
    let le32 = |at: usize| -> Option<u32> {
        Some(u32::from_le_bytes([*code.get(at)?, *code.get(at + 1)?, *code.get(at + 2)?, *code.get(at + 3)?]))
    };
    if slot == Slot::VarArgs && ty == 0 {
        return Some((Param { ty, slot, value: ParamValue::End }, p));
    }
    if !slot.accepts(ty) {
        return Some((Param { ty, slot, value: ParamValue::Unhandled }, p));
    }
    let (value, end) = match ty {
        1 => (ParamValue::Int(le32(p)? as i32), p + 4),
        4 => (ParamValue::Int(i32::from(*code.get(p)? as i8)), p + 1),
        5 => (ParamValue::Int(i32::from(le16(p)? as i16)), p + 2),
        6 => (ParamValue::Float(f32::from_bits(le32(p)?)), p + 4),
        2 | 3 | 0x0A | 0x0B | 0x10 | 0x11 => (ParamValue::Var(le16(p)?), p + 2),
        7 | 8 | 0x0C | 0x0D | 0x12 | 0x13 => (
            ParamValue::Array {
                base: le16(p)?,
                index: le16(p + 2)?,
                size: *code.get(p + 4)?,
                flags: *code.get(p + 5)?,
            },
            p + 6,
        ),
        9 => {
            let mut s = [0u8; 8];
            s.copy_from_slice(code.get(p..p + 8)?);
            (ParamValue::Str8(s), p + 8)
        }
        0x0F => {
            let mut s = [0u8; 16];
            s.copy_from_slice(code.get(p..p + 16)?);
            (ParamValue::Str16(s), p + 16)
        }
        0x0E => {
            let len = *code.get(p)? as i8;
            let n = len.max(0) as usize;
            (
                ParamValue::Pascal {
                    len,
                    bytes: code.get(p + 1..p + 1 + n)?.to_vec(),
                },
                p + 1 + n,
            )
        }
        _ => unreachable!("accepts() covers every handled type"),
    };
    Some((Param { ty, slot, value }, end))
}

/// Decodes the parameters of one candidate slot list. `strict` rejects unhandled types.
fn read_params(code: &[u8], mut pos: usize, slots: &[Slot], varargs: bool, strict: bool) -> Option<(Vec<Param>, usize)> {
    let mut params = Vec::with_capacity(slots.len());
    for &s in slots {
        let (p, e) = read_param(code, pos, s)?;
        if strict && p.value == ParamValue::Unhandled {
            return None;
        }
        params.push(p);
        pos = e;
    }
    if varargs {
        loop {
            let (p, e) = read_param(code, pos, Slot::VarArgs)?;
            let end = p.value == ParamValue::End;
            if strict && p.value == ParamValue::Unhandled {
                return None;
            }
            params.push(p);
            pos = e;
            if end {
                break;
            }
            if params.len() > 64 {
                return None;
            }
        }
    }
    Some((params, pos))
}

/// How many instructions decode in a row (strictly: every type handled by its reader) from
/// `pos`, up to `depth`; reaching `end` exactly counts as the full depth.
fn chain(code: &[u8], pos: usize, end: usize, depth: u32) -> u32 {
    if depth == 0 || pos == end {
        return depth;
    }
    if pos + 2 > end {
        return 0;
    }
    let raw = u16::from_le_bytes([code[pos], code[pos + 1]]);
    let Some(info) = &opcode_table()[(raw & 0x7FFF) as usize] else {
        return 0;
    };
    let mut best = 0;
    for (slots, va) in info.candidates() {
        if let Some((_, e)) = read_params(&code[..end], pos + 2, &slots, va, true) {
            best = best.max(1 + chain(code, e, end, depth - 1));
            if best == depth {
                break;
            }
        }
    }
    best
}

/// Whether a plausible instruction starts at `pos` (lookahead for ambiguous handlers).
fn plausible(code: &[u8], pos: usize, end: usize, depth: u32) -> bool {
    if pos == end {
        return true;
    }
    if pos + 2 > end {
        return false;
    }
    let raw = u16::from_le_bytes([code[pos], code[pos + 1]]);
    let Some(info) = &opcode_table()[(raw & 0x7FFF) as usize] else {
        return false;
    };
    if depth == 0 {
        return true;
    }
    let cands = info.candidates();
    if cands.len() > 8 {
        return true;
    }
    cands.iter().any(|(slots, va)| {
        read_params(&code[..end], pos + 2, slots, *va, true)
            .is_some_and(|(_, e)| plausible(code, e, end, depth - 1))
    })
}

/// Which script a unit is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScriptKind {
    Main,
    Mission(usize),
    Streamed(usize),
}

/// One script buffer to decode.
#[derive(Debug, Clone)]
pub struct Unit {
    pub kind: ScriptKind,
    pub name: String,
    /// The bytes the script runs from (main: the whole main.scm; mission: its slice; streamed:
    /// the script.img entry cut to the table size).
    pub data: Vec<u8>,
    /// Decodable range.
    pub start: usize,
    pub end: usize,
    /// Offset of `data[0]` in main.scm (for reports; 0 for streamed scripts).
    pub file_base: usize,
}

/// The decoded scripts of one install.
pub struct ScriptSet {
    pub units: Vec<Unit>,
    notes: Vec<String>,
    /// The body of main.scm header segment id 4 (`aaa.scm` in script.img repeats it).
    stamp: Option<Vec<u8>>,
}

impl ScriptSet {
    /// Builds the units: main code, every mission, every streamed script (`streamed` = the
    /// `script.img` entries as `(name, data)`).
    pub fn new(scm: &MainScm, streamed: &[(String, Vec<u8>)]) -> ScriptSet {
        let file = scm.to_bytes();
        let mut notes = Vec::new();
        let mut units = Vec::new();
        let (main_size, offsets) = scm.missions().map_or((file.len() as u32, &[][..]), |(m, o)| (m, o));
        units.push(Unit {
            kind: ScriptKind::Main,
            name: "MAIN".into(),
            data: file.clone(),
            start: scm.code_start,
            end: main_size as usize,
            file_base: 0,
        });
        for (i, &o) in offsets.iter().enumerate() {
            let end = offsets.get(i + 1).map_or(file.len(), |&n| n as usize);
            let (o, end) = (o as usize, end.min(file.len()));
            if o > end {
                notes.push(format!("mission {i}: offset {o:#x} after its end {end:#x}"));
                continue;
            }
            units.push(Unit {
                kind: ScriptKind::Mission(i),
                name: format!("mission{i}"),
                data: file[o..end].to_vec(),
                start: 0,
                end: end - o,
                file_base: o,
            });
        }
        let table: &[StreamedEntry] = scm.streamed();
        for (i, (name, data)) in streamed.iter().enumerate() {
            let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
            let entry = table.iter().find(|e| e.name().eq_ignore_ascii_case(stem.as_bytes()));
            let size = match entry {
                Some(e) => e.size as usize,
                None => {
                    notes.push(format!("script.img {name}: no entry in the main.scm streamed table"));
                    data.len()
                }
            };
            if size > data.len() {
                notes.push(format!("script.img {name}: table size {size} > entry size {}", data.len()));
            }
            let size = size.min(data.len());
            units.push(Unit {
                kind: ScriptKind::Streamed(i),
                name: stem.to_string(),
                data: data[..size].to_vec(),
                start: 0,
                end: size,
                file_base: 0,
            });
        }
        let stamp = scm.segments.iter().find(|s| s.id == 4).map(|s| s.rest.clone());
        ScriptSet { units, notes, stamp }
    }

    /// Header-level observations.
    pub fn header_notes(&self) -> Vec<String> {
        self.notes.clone()
    }

    /// Decodes every unit by following control flow.
    pub fn decode_all(&self) -> DecodeReport {
        // Pass 1 finds which parameter list the data selects for each opcode the exe analysis
        // leaves ambiguous; pass 2 uses the majority list to break ties.
        let first = self.decode_pass(BTreeMap::new());
        let prefer = first
            .choices
            .iter()
            .filter_map(|(op, ch)| ch.iter().max_by_key(|(_, n)| **n).map(|(k, _)| (*op, k.clone())))
            .collect();
        let mut rep = self.decode_pass(prefer);
        let tb = std::mem::take(&mut rep.tie_breaks);
        rep.notes.extend(tb);
        rep
    }

    fn decode_pass(&self, prefer: BTreeMap<u16, String>) -> DecodeReport {
        let mut rep = DecodeReport {
            prefer,
            ..Default::default()
        };
        let mut extra_main_entries: BTreeSet<usize> = BTreeSet::new();
        // Units other than main first: they may add entries into main (positive jump targets).
        let mut order: Vec<usize> = (1..self.units.len()).collect();
        order.push(0);
        for ui in order {
            let u = &self.units[ui];
            if matches!(u.kind, ScriptKind::Streamed(_)) && Some(&u.data) == self.stamp.as_ref() {
                rep.notes.push(format!(
                    "script.img {}: {} bytes equal to main.scm header segment 4 (global space size, build number), not code; not decoded",
                    u.name,
                    u.data.len()
                ));
                rep.stamps += 1;
                continue;
            }
            let mut entries = vec![u.start];
            if u.kind == ScriptKind::Main {
                entries.extend(extra_main_entries.iter().copied());
            }
            let d = decode_unit(u, &entries, &mut extra_main_entries, &mut rep);
            rep.units.push(d);
        }
        rep.units.sort_by(|a, b| a.kind.cmp(&b.kind));
        rep.finish_choices();
        rep
    }
}

/// Decoding result of one unit.
#[derive(Debug, Clone)]
pub struct UnitDecode {
    pub kind: ScriptKind,
    pub name: String,
    /// Argument of the first `SCRIPT_NAME` (0x03A4) by offset, if any.
    pub script_name: Option<String>,
    pub instructions: Vec<Instruction>,
    pub size: usize,
    pub covered: usize,
    /// Unreached byte ranges `(start, end, class)`.
    pub gaps: Vec<(usize, usize, GapClass)>,
}

/// What an unreached byte range holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GapClass {
    /// Only zero bytes.
    Zero,
    /// Decodes linearly as instructions up to its end (dead code).
    DeadCode,
    /// Not decodable as instructions (data).
    Data,
}

/// Results over all units.
#[derive(Default)]
pub struct DecodeReport {
    pub units: Vec<UnitDecode>,
    failures: Vec<String>,
    notes: Vec<String>,
    /// opcode -> chosen candidate slot count -> occurrences (ambiguous handlers).
    choices: BTreeMap<u16, BTreeMap<String, u64>>,
    /// Pass 2: the list each ambiguous opcode's occurrences select most often in pass 1.
    prefer: BTreeMap<u16, String>,
    tie_breaks: Vec<String>,
    stamps: u64,
}

impl DecodeReport {
    fn finish_choices(&mut self) {
        for (op, ch) in &self.choices {
            let info = opcode_table()[*op as usize].as_ref();
            let name = info.map_or("", |i| i.name);
            let reads = info.map_or("", |i| i.reads);
            let list: Vec<String> = ch.iter().map(|(k, v)| format!("{k} x{v}")).collect();
            self.notes.push(format!(
                "{op:04X} {name}: exe paths [{reads}], data selects {}",
                list.join(", ")
            ));
            if ch.len() > 1 {
                self.failures.push(format!(
                    "{op:04X} {name}: occurrences need different parameter lists ({})",
                    list.join(", ")
                ));
            }
        }
    }

    /// Decode failures (instructions on a reachable path that cannot be decoded).
    pub fn failures(&self) -> Vec<String> {
        self.failures.clone()
    }

    /// Informational notes (ambiguous handlers resolved by the data, cross-unit jumps, ...).
    pub fn notes(&self) -> Vec<String> {
        self.notes.clone()
    }

    /// Summary counts.
    pub fn counts(&self) -> Vec<(String, u64)> {
        let mut m: BTreeMap<String, u64> = BTreeMap::new();
        for u in &self.units {
            let k = match u.kind {
                ScriptKind::Main => "main",
                ScriptKind::Mission(_) => "missions",
                ScriptKind::Streamed(_) => "streamed",
            };
            *m.entry(format!("{k}: units")).or_default() += 1;
            *m.entry(format!("{k}: instructions")).or_default() += u.instructions.len() as u64;
            *m.entry(format!("{k}: bytes")).or_default() += u.size as u64;
            *m.entry(format!("{k}: bytes covered by reachable instructions")).or_default() += u.covered as u64;
            for (s, e, c) in &u.gaps {
                *m.entry(format!("{k}: unreached bytes, {c:?}")).or_default() += (e - s) as u64;
                *m.entry(format!("{k}: unreached ranges, {c:?}")).or_default() += 1;
            }
        }
        let ops: BTreeSet<u16> = self.units.iter().flat_map(|u| u.instructions.iter().map(|i| i.opcode())).collect();
        m.insert("distinct opcodes used".into(), ops.len() as u64);
        m.insert("streamed: entries holding the header version words (not decoded)".into(), self.stamps);
        let exact = ops
            .iter()
            .filter(|o| opcode_table()[**o as usize].as_ref().is_some_and(|i| i.is_exact()))
            .count();
        m.insert("distinct opcodes used, parameter list exact from the exe".into(), exact as u64);
        m.insert("distinct opcodes used, parameter list selected by the data".into(), self.choices.len() as u64);
        m.into_iter().collect()
    }
}

fn decode_unit(u: &Unit, entries: &[usize], main_entries: &mut BTreeSet<usize>, rep: &mut DecodeReport) -> UnitDecode {
    let code = &u.data[..u.end];
    let mut mark = vec![0u8; u.end]; // 0 free, 1 instruction start, 2 inside
    let mut instrs: BTreeMap<usize, Instruction> = BTreeMap::new();
    let mut work: Vec<(usize, Option<usize>)> = entries.iter().map(|&e| (e, None)).collect();
    let label = |pos: usize| format!("{} @{:#x} (file {:#x})", u.name, pos, u.file_base + pos);
    let mut from_of: BTreeMap<usize, usize> = BTreeMap::new();
    while let Some((pos, from)) = work.pop() {
        if let Some(f) = from {
            from_of.entry(pos).or_insert(f);
        }
        if pos >= u.end || pos < u.start {
            rep.failures.push(format!("{}: jump target outside the script", label(pos)));
            continue;
        }
        match mark[pos] {
            1 => continue,
            2 => {
                rep.failures.push(format!("{}: jump target inside an instruction", label(pos)));
                continue;
            }
            _ => {}
        }
        let nfail = rep.failures.len();
        let ins = decode_at(code, pos, u, rep);
        if rep.failures.len() > nfail
            && let Some(f) = from
        {
            let last = rep.failures.last_mut().expect("just pushed");
            let _ = write!(last, " (reached from {f:#x})");
        }
        let Some(ins) = ins else {
            continue;
        };
        if mark[pos..pos + ins.len].iter().any(|&m| m != 0) {
            rep.failures.push(format!("{}: instruction overlaps another", label(pos)));
            continue;
        }
        mark[pos] = 1;
        for m in &mut mark[pos + 1..pos + ins.len] {
            *m = 2;
        }
        let next = pos + ins.len;
        let op = ins.opcode();
        let mut targets: Vec<i32> = Vec::new();
        let mut falls = true;
        match op {
            0x0002 => {
                targets.extend(ins.params.first().and_then(Param::as_int));
                falls = false;
            }
            0x004C | 0x004D | 0x0050 | 0x004F | 0x00D7 => targets.extend(ins.params.first().and_then(Param::as_int)),
            0x004E | 0x0051 => falls = false,
            0x0871 => {
                // var, case count, has default, default label, 7 x (value, label). Unused pairs
                // hold value -1 and a valid label (the compiler repeats the end label), so every
                // label is followed.
                targets.extend(ins.params.get(3).and_then(Param::as_int));
                for k in 0..7 {
                    targets.extend(ins.params.get(5 + 2 * k).and_then(Param::as_int));
                }
            }
            0x0872 => {
                // 9 x (value, label), same padding.
                for k in 0..9 {
                    targets.extend(ins.params.get(2 * k + 1).and_then(Param::as_int));
                }
            }
            _ => {}
        }
        for t in targets {
            if t < 0 {
                work.push(((-(t as i64)) as usize, Some(pos)));
            } else if u.kind == ScriptKind::Main {
                work.push((t as usize, Some(pos)));
            } else {
                rep.notes.push(format!("{}: jumps into main.scm at {t:#x}", label(pos)));
                main_entries.insert(t as usize);
            }
        }
        if falls {
            if next >= u.end {
                rep.notes.push(format!("{}: last instruction falls through the end", label(pos)));
            } else {
                work.push((next, Some(pos)));
            }
        }
        instrs.insert(pos, ins);
    }
    // Unreached ranges.
    let mut gaps = Vec::new();
    let mut i = u.start;
    while i < u.end {
        if mark[i] != 0 {
            i += 1;
            continue;
        }
        let s = i;
        while i < u.end && mark[i] == 0 {
            i += 1;
        }
        let bytes = &code[s..i];
        let class = if bytes.iter().all(|&b| b == 0) {
            GapClass::Zero
        } else if linear_decodes(code, s, i) {
            GapClass::DeadCode
        } else {
            GapClass::Data
        };
        gaps.push((s, i, class));
    }
    let covered = mark.iter().filter(|&&m| m != 0).count();
    let instructions: Vec<Instruction> = instrs.into_values().collect();
    // The first SCRIPT_NAME in offset order (most scripts start with it).
    let script_name = instructions
        .iter()
        .find(|i| i.opcode() == 0x03A4)
        .and_then(|i| i.params.first())
        .and_then(Param::as_str)
        .map(|s| String::from_utf8_lossy(s).into_owned());
    UnitDecode {
        kind: u.kind.clone(),
        name: u.name.clone(),
        script_name,
        instructions,
        size: u.end - u.start,
        covered,
        gaps,
    }
}

fn linear_decodes(code: &[u8], mut pos: usize, end: usize) -> bool {
    while pos < end {
        if pos + 2 > end {
            return false;
        }
        let raw = u16::from_le_bytes([code[pos], code[pos + 1]]);
        let Some(info) = &opcode_table()[(raw & 0x7FFF) as usize] else {
            return false;
        };
        let mut ok = None;
        for (slots, va) in info.candidates() {
            if let Some((_, e)) = read_params(&code[..end], pos + 2, &slots, va, true)
                && plausible(code, e, end, 1)
            {
                ok = Some(e);
                break;
            }
        }
        match ok {
            Some(e) => pos = e,
            None => return false,
        }
    }
    true
}

fn decode_at(code: &[u8], pos: usize, u: &Unit, rep: &mut DecodeReport) -> Option<Instruction> {
    let label = || format!("{} @{:#x} (file {:#x})", u.name, pos, u.file_base + pos);
    if pos + 2 > code.len() {
        rep.failures.push(format!("{}: truncated opcode", label()));
        return None;
    }
    let raw = u16::from_le_bytes([code[pos], code[pos + 1]]);
    let op = raw & 0x7FFF;
    let Some(info) = &opcode_table()[op as usize] else {
        rep.failures.push(format!("{}: opcode {op:04X} has no handler in the exe", label()));
        return None;
    };
    let cands = info.candidates();
    let (slots, va, params, end) = if info.is_exact() {
        let (slots, va) = cands[0].clone();
        match read_params(code, pos + 2, &slots, va, false) {
            Some((p, e)) => (slots, va, p, e),
            None => {
                rep.failures.push(format!("{}: {op:04X} {} parameters run past the end", label(), info.name));
                return None;
            }
        }
    } else {
        // Score each candidate by how many following instructions decode strictly; keep the
        // candidates with the best score.
        const DEPTH: u32 = 8;
        let mut scored = Vec::new();
        for (slots, va) in &cands {
            if let Some((p, e)) = read_params(code, pos + 2, slots, *va, true) {
                scored.push((chain(code, e, code.len(), DEPTH), (slots.clone(), *va, p, e)));
            }
        }
        let best = scored.iter().map(|s| s.0).max().unwrap_or(0);
        let mut ok: Vec<_> = scored.into_iter().filter(|s| s.0 == best && best > 0).map(|s| s.1).collect();
        let key_of = |c: &(Vec<Slot>, bool, Vec<Param>, usize)| format!("{}{}", c.0.len(), if c.1 { "+var" } else { "" });
        if ok.len() > 1
            && let Some(pref) = rep.prefer.get(&op)
            && let Some(k) = ok.iter().position(|c| &key_of(c) == pref)
        {
            // Several lists fit this occurrence equally well: take the list the other
            // occurrences of the opcode select (pass 1 majority).
            rep.tie_breaks.push(format!("{}: {op:04X} {}: tie broken by the majority list {pref}", label(), info.name));
            let c = ok.swap_remove(k);
            ok = vec![c];
        }
        if ok.len() != 1 {
            rep.failures.push(format!(
                "{}: {op:04X} {}: {} of {} candidate parameter lists fit the data",
                label(),
                info.name,
                ok.len(),
                cands.len()
            ));
            if ok.is_empty() {
                return None;
            }
        }
        let c = ok.swap_remove(0);
        let key = key_of(&c);
        *rep.choices.entry(op).or_default().entry(key).or_default() += 1;
        c
    };
    let _ = (slots, va);
    if let Some(p) = params.iter().find(|p| p.value == ParamValue::Unhandled) {
        rep.failures.push(format!(
            "{}: {op:04X} {}: type {:#04x} in a {:?} slot (the exe consumes only the type byte)",
            label(),
            info.name,
            p.ty,
            p.slot
        ));
    }
    Some(Instruction {
        offset: pos,
        raw,
        params,
        len: end - pos,
    })
}

/// `catalog/script-usage.csv`: one row per opcode used anywhere.
pub fn usage_csv(rep: &DecodeReport) -> String {
    #[derive(Default)]
    struct Row {
        main: u64,
        missions: u64,
        streamed: u64,
        mission_names: BTreeSet<String>,
        streamed_names: BTreeSet<String>,
    }
    let mut rows: BTreeMap<u16, Row> = BTreeMap::new();
    for u in &rep.units {
        let uname = u.script_name.clone().unwrap_or_else(|| u.name.clone());
        for i in &u.instructions {
            let r = rows.entry(i.opcode()).or_default();
            match u.kind {
                ScriptKind::Main => r.main += 1,
                ScriptKind::Mission(n) => {
                    r.missions += 1;
                    r.mission_names.insert(format!("{n:03}:{uname}"));
                }
                ScriptKind::Streamed(_) => {
                    r.streamed += 1;
                    r.streamed_names.insert(u.name.clone());
                }
            }
        }
    }
    let mut out = String::from(
        "opcode,name,handler,params_source,count_main,count_missions,count_streamed,missions_using,streamed_using\n",
    );
    for (op, r) in rows {
        let info = opcode_table()[op as usize].as_ref();
        let src = match info {
            Some(i) if i.is_exact() => "exe",
            Some(_) => "exe+data",
            None => "none",
        };
        let names: Vec<_> = r.mission_names.into_iter().collect();
        let snames: Vec<_> = r.streamed_names.into_iter().collect();
        let _ = writeln!(
            out,
            "{op:04X},{},{},{src},{},{},{},{},{}",
            info.map_or("", |i| i.name),
            info.map_or(String::new(), |i| format!("{:#x}", i.handler)),
            r.main,
            r.missions,
            r.streamed,
            names.join(" "),
            snames.join(" ")
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::Put;

    fn ins(op: u16, params: &[u8]) -> Vec<u8> {
        let mut v = op.to_le_bytes().to_vec();
        v.extend_from_slice(params);
        v
    }

    #[test]
    fn table_has_known_handlers() {
        let t = opcode_table();
        let wait = t[0x0001].as_ref().unwrap();
        assert_eq!(wait.name, "COMMAND_WAIT");
        assert!(wait.is_exact());
        assert_eq!(wait.candidates(), vec![(vec![Slot::Collect], false)]);
        let sns = t[0x004F].as_ref().unwrap();
        assert_eq!(sns.candidates(), vec![(vec![Slot::Collect], true)]);
        let set_var = t[0x0004].as_ref().unwrap();
        assert_eq!(set_var.candidates()[0].0, vec![Slot::Var, Slot::Collect]);
    }

    #[test]
    fn decodes_param_types() {
        let mut c = Vec::new();
        c.push(1);
        c.put_i32(-5);
        c.extend_from_slice(&[4, 0xFF, 5, 0x34, 0x12, 6]);
        c.put_f32(1.5);
        c.extend_from_slice(&[7, 8, 0, 2, 0, 10, 0x80]);
        c.extend_from_slice(&[0x0E, 3, b'a', b'b', b'c']);
        let slots = [Slot::Collect, Slot::Collect, Slot::Collect, Slot::Collect, Slot::Collect, Slot::Text];
        let (p, e) = read_params(&c, 0, &slots, false, true).unwrap();
        assert_eq!(e, c.len());
        assert_eq!(p[0].value, ParamValue::Int(-5));
        assert_eq!(p[1].value, ParamValue::Int(-1));
        assert_eq!(p[2].value, ParamValue::Int(0x1234));
        assert_eq!(p[3].value, ParamValue::Float(1.5));
        assert_eq!(p[4].value, ParamValue::Array { base: 8, index: 2, size: 10, flags: 0x80 });
        assert_eq!(p[5].as_str(), Some(&b"abc"[..]));
        // A string type in a numeric slot: only the type byte is consumed.
        let (p, e) = read_params(&[9, 1, 2], 0, &[Slot::Collect], false, false).unwrap();
        assert_eq!((p[0].value.clone(), e), (ParamValue::Unhandled, 1));
        // Negative pascal length reads nothing.
        let (p, e) = read_param(&[0x0E, 0xF0, 1], 0, Slot::Text).unwrap();
        assert_eq!((p.value, e), (ParamValue::Pascal { len: -16, bytes: vec![] }, 2));
    }

    #[test]
    fn follows_control_flow_and_skips_data() {
        // 0: WAIT 0; GOTO_IF_FALSE -> 30 (relative); GOTO -> 22; <data>; 22: TERMINATE
        let mut c = Vec::new();
        c.extend(ins(0x0001, &[4, 0])); // 0..4
        let mut g = vec![1];
        g.put_i32(-24);
        c.extend(ins(0x004D, &g)); // 4..11
        let mut g2 = vec![1];
        g2.put_i32(-22);
        c.extend(ins(0x0002, &g2)); // 11..18
        c.extend_from_slice(&[0xFF, 0xFF, 0xEE, 0xEE]); // 18..22 data
        c.extend(ins(0x004E, &[])); // 22..24
        c.extend(ins(0x8001, &[4, 1])); // 24..28 (NOT flag set, still decodes)
        c.extend(ins(0x0051, &[])); // 28..30
        let u = Unit {
            kind: ScriptKind::Mission(0),
            name: "m".into(),
            end: c.len(),
            data: c,
            start: 0,
            file_base: 0,
        };
        let mut rep = DecodeReport::default();
        let mut extra = BTreeSet::new();
        let d = decode_unit(&u, &[0], &mut extra, &mut rep);
        assert!(rep.failures.is_empty(), "{:?}", rep.failures);
        let offs: Vec<usize> = d.instructions.iter().map(|i| i.offset).collect();
        assert_eq!(offs, vec![0, 4, 11, 22, 24, 28]);
        assert_eq!(d.gaps, vec![(18, 22, GapClass::Data)]);
        assert!(d.instructions[4].not_flag());
    }

    #[test]
    fn raw_inline_operands() {
        let t = opcode_table();
        // DECLARE_MISSION_FLAG: 3 inline bytes, whatever the first one is.
        let decl = t[0x0180].as_ref().unwrap();
        assert_eq!(decl.candidates(), vec![(vec![Slot::Raw(3)], false)]);
        let (p, e) = read_params(&[0x09, 0x34, 0x12, 0xFF], 0, &[Slot::Raw(3)], false, true).unwrap();
        assert_eq!((p[0].value.clone(), e), (ParamValue::Raw(vec![0x09, 0x34, 0x12]), 3));
        // SAVE_STRING_TO_DEBUG_FILE: 128 bytes from m_IP.
        let dbg = t[0x05B6].as_ref().unwrap();
        assert_eq!(dbg.candidates(), vec![(vec![Slot::Raw(128)], false)]);
        // SWITCH_START: 18 collected parameters (override from the disassembly).
        assert_eq!(t[0x0871].as_ref().unwrap().candidates()[0].0.len(), 18);
    }

    #[test]
    fn reports_target_inside_instruction() {
        let mut c = Vec::new();
        let mut g = vec![1];
        g.put_i32(-3);
        c.extend(ins(0x0002, &g));
        let u = Unit {
            kind: ScriptKind::Streamed(0),
            name: "s".into(),
            end: c.len(),
            data: c,
            start: 0,
            file_base: 0,
        };
        let mut rep = DecodeReport::default();
        decode_unit(&u, &[0], &mut BTreeSet::new(), &mut rep);
        assert_eq!(rep.failures.len(), 1, "{:?}", rep.failures);
    }
}
