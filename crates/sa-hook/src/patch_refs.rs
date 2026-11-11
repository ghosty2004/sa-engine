//! Test only (HARNESS-2; the checks of `tools/verify/hookscan.py` that add coverage to sa-hook's install
//! test): references into the bytes a hook patches that neither [`crate::data_reads`] nor the
//! installer's own branch check see.
//!
//! What the other checks cover: `data_reads.rs` reads `catalog/code-data-reads.csv` (memory operands
//! and immediates of code instructions that decode in a protection chunk or on a linear sweep from a
//! function start, into the first 24 bytes of a catalogued function); `x86::inbound_branch` in
//! `every_hook_installs_on_the_exe` scans the 0x400 bytes after an inline hook's entry for a branch
//! into its relocated bytes; `callers.rs` covers hookscan's second check (ECX/EDX used after a call)
//! with path following. Added here, over the whole image:
//!
//! 1. [`branches_into`]: every direct branch (`call`/`jmp`/`jcc`/`loop`/`jecxz`, rel8 and rel32) in the
//!    code sections whose target lies strictly inside a patched range (after the entry, before the end
//!    of the patch), confirmed on a linear decode from the containing catalogued procedure. Such a
//!    branch would land in the middle of the hook's `jmp` (or of a retargeted stub's rel32). Covers
//!    branches from other functions and from far inside large functions (beyond 0x400 bytes).
//! 2. [`values_into`]: every dword, at any alignment, of the executable sections whose value lies
//!    strictly inside a patched range. It fails the test when it is the 32-bit immediate or
//!    displacement of an instruction on a linear decode of a catalogued code range (`push`/`mov`
//!    immediates, `lea`/`mov`/`adc` displacements, memory operands of any instruction, decoded in
//!    a protection chunk or not); other places are listed for review (hookscan's "undecoded"). The
//!    entry address itself is a function pointer and is not counted. Absolute reads that
//!    [`crate::keycells::READERS`] redirects are not counted either.
//!
//! "Strictly inside": an inline hook patches `[entry, entry + relocated length)`; a `.HOODLUM` stub
//! keeps its `E9` and patches the rel32, so `(entry, entry + 5)`.

use iced_x86::{ConstantOffsets, Decoder, DecoderOptions, Instruction};

use crate::keycells::READERS;
use crate::table::HOOKS;

/// One patched range: (entry, end of the patched bytes, hook name, first changed byte: the entry for
/// an inline hook, entry + 1 for a stub, whose `E9` stays), sorted by entry.
pub type Patch = (u32, u32, &'static str, u32);

pub fn patches(exe: &crate::testexe::Exe) -> Vec<Patch> {
    let mut v: Vec<Patch> = HOOKS
        .iter()
        .map(|h| {
            let (lo, hi) = crate::data_reads::changed(exe, h.addr, h.kind);
            (h.addr, hi, h.name, lo)
        })
        .collect();
    v.sort();
    v
}

/// The patch `v` lies strictly inside (after its entry, before its end).
pub fn inside(p: &[Patch], v: u32) -> Option<&Patch> {
    let k = p.partition_point(|x| x.0 < v);
    let x = p.get(k.checked_sub(1)?)?;
    (x.0 < v && v < x.1).then_some(x)
}

/// The code ranges of every catalogued function (`catalog/extents.csv`, column `ranges`: the body
/// Ghidra found, `.HOODLUM` chunks included), sorted. Bytes outside them are not code: the no-CD build
/// left junk after most `.HOODLUM` stubs (0x4045E0 `jmp 0x156CFF0` is followed by `jmp 0x404609; int3`,
/// 0x441130's stub by bytes that decode as `adc`/`xchg`/`jmp`), and a decode there finds "branches"
/// that nothing executes.
pub fn code_ranges() -> Vec<(u32, u32)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ext = std::fs::read_to_string(root.join("catalog/extents.csv")).unwrap();
    let hex = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
    let mut v = Vec::new();
    for l in ext.lines().skip(1) {
        for r in l
            .split(',')
            .nth(10)
            .unwrap_or("")
            .split(';')
            .filter(|r| r.contains('-'))
        {
            let (lo, hi) = r.split_once('-').unwrap();
            v.push((hex(lo), hex(hi)));
        }
    }
    v.sort();
    v.dedup();
    v
}

/// The instruction that holds byte `at` on a linear decode from the start of a catalogued code
/// range containing `at` ([`code_ranges`]), or None (no range holds `at`, or `at` is in the middle of
/// no valid decoded instruction).
pub fn insn_covering(
    exe: &crate::testexe::Exe,
    ranges: &[(u32, u32)],
    at: u32,
) -> Option<(Instruction, ConstantOffsets)> {
    let k = ranges.partition_point(|r| r.0 <= at);
    // the nearest range starting at or before `at` that contains it (ranges rarely overlap)
    let &(start, _) = ranges[..k].iter().rev().take(8).find(|r| at < r.1)?;
    let code = exe.bytes(start, (at - start) as usize + 16);
    let mut d = Decoder::with_ip(32, code, start as u64, DecoderOptions::NONE);
    let mut i = Instruction::default();
    while d.can_decode() {
        d.decode_out(&mut i);
        let (s, e) = (i.ip32(), i.ip32() + i.len() as u32);
        if s <= at && at < e {
            return (!i.is_invalid()).then(|| (i, d.get_constant_offsets(&i)));
        }
        if s > at {
            return None;
        }
    }
    None
}

/// `MNEMONIC at ADDR [bytes]` of a decoded instruction.
fn show(exe: &crate::testexe::Exe, i: &Instruction) -> String {
    let b: String = exe
        .bytes(i.ip32(), i.len())
        .iter()
        .map(|x| format!("{x:02X}"))
        .collect();
    format!("{:?} at {:#x} [{b}]", i.mnemonic(), i.ip32())
}

/// Relative branch at `o` of `code` (at `va`): (instruction length, target), by opcode only.
fn rel_branch(code: &[u8], o: usize, va: u32) -> Option<(u32, u32)> {
    let b = |k: usize| code.get(o + k).copied();
    let rel32 =
        |k: usize| -> Option<i32> { Some(i32::from_le_bytes([b(k)?, b(k + 1)?, b(k + 2)?, b(k + 3)?])) };
    let at = va + o as u32;
    let (len, rel) = match b(0)? {
        0xE8 | 0xE9 => (5, rel32(1)?),
        0xEB | 0x70..=0x7F | 0xE0..=0xE3 => (2, b(1)? as i8 as i32),
        0x0F if matches!(b(1)?, 0x80..=0x8F) => (6, rel32(2)?),
        _ => return None,
    };
    Some((len, at.wrapping_add(len).wrapping_add(rel as u32)))
}

/// Check 1. Returns (confirmed branches, candidates not on an instruction boundary of a linear decode
/// from a catalogued procedure: bytes inside other instructions or data, listed for review only).
pub fn branches_into(
    exe: &crate::testexe::Exe,
    p: &[Patch],
    ranges: &[(u32, u32)],
) -> (Vec<String>, Vec<String>) {
    let (mut hits, mut other) = (Vec::new(), Vec::new());
    for (va, code) in exe.code_sections() {
        for o in 0..code.len() {
            let Some((_, t)) = rel_branch(code, o, va) else {
                continue;
            };
            let Some(&(entry, end, name, _)) = inside(p, t) else {
                continue;
            };
            let at = va + o as u32;
            let line = format!(
                "{at:#x} -> {t:#x} = {name} ({entry:#x}) + {} (patched up to +{})",
                t - entry,
                end - entry
            );
            match insn_covering(exe, ranges, at).map(|x| x.0) {
                Some(i) if i.ip32() == at && i.near_branch_target() as u32 == t => {
                    hits.push(format!("{line}: {}", show(exe, &i)))
                }
                _ => other.push(line),
            }
        }
    }
    (hits, other)
}

/// The patch whose changed bytes `[lo, end)` a dword read at `v` (bytes `v..v + 4`) overlaps.
pub fn read_overlaps(p: &[Patch], v: u32) -> Option<&Patch> {
    let k = p.partition_point(|x| x.0 <= v.saturating_add(3));
    p[..k]
        .iter()
        .rev()
        .take(4)
        .find(|x| x.3 < v.saturating_add(4) && v < x.1)
}

/// Check 2. Returns (dwords that are the 32-bit immediate or displacement of an instruction on a linear
/// decode of a catalogued code range: these fail the test; dwords at any other place of an
/// executable section: listed for review). An immediate counts when it points strictly inside a
/// patch (a pointer to the entry is a function pointer); a displacement when a dword read there
/// overlaps the changed bytes (a read of the entry dword itself included: hookscan's
/// `[addr - 3, addr + 5)`). Non-executable sections are not scanned: the exe has no relocation table
/// to tell a stored code pointer from data, and the 29 dwords found there on 2026-11-11 were all
/// ASCII / UTF-16 text (e.g. 0x00550051 "Q\0U\0").
pub fn values_into(
    exe: &crate::testexe::Exe,
    p: &[Patch],
    ranges: &[(u32, u32)],
) -> (Vec<String>, Vec<String>) {
    let (mut hits, mut raw) = (Vec::new(), Vec::new());
    for (va, data) in exe.exec_sections() {
        for o in 0..data.len().saturating_sub(3) {
            let v = u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
            let (ins, ovl) = (inside(p, v), read_overlaps(p, v));
            let Some(&(entry, end, name, _)) = ins.or(ovl) else {
                continue;
            };
            let at = va + o as u32;
            if READERS
                .iter()
                .any(|r| r.at + r.disp_off as u32 == at && r.key == v)
            {
                continue;
            }
            let line = format!(
                "{at:#x} holds {v:#x} = {name} ({entry:#x}) {:+} (patched up to +{})",
                v.wrapping_sub(entry) as i32,
                end - entry
            );
            let field = insn_covering(exe, ranges, at).and_then(|(i, co)| {
                let off = (at - i.ip32()) as usize;
                let disp = co.displacement_size() == 4 && co.displacement_offset() == off && ovl.is_some();
                let imm = co.immediate_size() == 4 && co.immediate_offset() == off && ins.is_some();
                (disp || imm).then_some(i)
            });
            match field {
                Some(i) => hits.push(format!("{line}: {}", show(exe, &i))),
                None if ins.is_some() => raw.push(line),
                None => {}
            }
        }
    }
    (hits, raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered hook: no branch lands inside its patched bytes and no instruction or data
    /// dword points inside them (hookscan.py's first check, extended to every section and to
    /// branch targets). A failure names the offender; the fix is a review of that hook (a keycells
    /// redirect, a different patch, or not registering it), never a silent exception here.
    #[test]
    fn nothing_references_the_inside_of_a_patched_entry() {
        let Some(exe) = crate::testexe::load() else { return };
        let p = patches(&exe);
        let ranges = code_ranges();
        let (br, br_other) = branches_into(&exe, &p, &ranges);
        let (vals, raw) = values_into(&exe, &p, &ranges);
        for l in br_other.iter().chain(&raw) {
            eprintln!("note (not on a decoded instruction, review only): {l}");
        }
        assert!(br.is_empty(), "branches into patched bytes:\n{}", br.join("\n"));
        assert!(
            vals.is_empty(),
            "references into patched bytes:\n{}",
            vals.join("\n")
        );
    }

    /// Negative tests on hand-picked entries that are not hooked (or would be bad hooks).
    #[test]
    fn known_references_are_found() {
        let Some(exe) = crate::testexe::load() else { return };
        let ranges = code_ranges();
        // 0x645600 CCarEnterExit::GetAnimDoorOffset (unhooked by the main session for this reason):
        // the protection uses 0x645602 as an immediate.
        let p: Vec<Patch> = vec![(0x645600, 0x645600 + 5, "GetAnimDoorOffset", 0x645600)];
        let (v, raw) = values_into(&exe, &p, &ranges);
        assert!(
            v.iter().chain(&raw).any(|l| l.contains("holds 0x645602")),
            "{v:?} {raw:?}"
        );
        // a synthetic patch over a branch target that exists: the first `jmp rel32` of the game code
        // that is on a decoded boundary lands inside (target - 1, target + 4)
        let (va, code) = exe.code_sections().next().unwrap();
        let (site, t) = (0..code.len())
            .filter(|&o| code[o] == 0xE9)
            .filter_map(|o| rel_branch(code, o, va).map(|(_, t)| (va + o as u32, t)))
            .find(|&(s, t)| {
                (0x401000..0x857000).contains(&t)
                    && insn_covering(&exe, &ranges, s).is_some_and(|x| x.0.ip32() == s)
            })
            .unwrap();
        let p: Vec<Patch> = vec![(t - 1, t + 4, "synthetic", t - 1)];
        let (b, _) = branches_into(&exe, &p, &ranges);
        assert!(b.iter().any(|l| l.starts_with(&format!("{site:#x} "))), "{b:?}");
        // the entry itself is never a hit
        let p: Vec<Patch> = vec![(t, t + 5, "synthetic", t)];
        let (b, _) = branches_into(&exe, &p, &ranges);
        assert!(!b.iter().any(|l| l.starts_with(&format!("{site:#x} "))), "{b:?}");
    }

    #[test]
    fn inside_is_strict() {
        let p: Vec<Patch> = vec![(0x1000, 0x1005, "a", 0x1000), (0x2000, 0x2006, "b", 0x2001)];
        assert!(inside(&p, 0x1000).is_none());
        assert_eq!(inside(&p, 0x1001).map(|x| x.2), Some("a"));
        assert_eq!(inside(&p, 0x1004).map(|x| x.2), Some("a"));
        assert!(inside(&p, 0x1005).is_none());
        assert_eq!(inside(&p, 0x2005).map(|x| x.2), Some("b"));
        assert!(inside(&p, 0xFFF).is_none());
        // dword reads: inline `a` changes 0x1000.., the stub `b` keeps its E9 at 0x2000
        assert!(read_overlaps(&p, 0xFFC).is_none());
        assert_eq!(read_overlaps(&p, 0xFFD).map(|x| x.2), Some("a"));
        assert_eq!(read_overlaps(&p, 0x1004).map(|x| x.2), Some("a"));
        assert!(read_overlaps(&p, 0x1005).is_none());
        assert!(read_overlaps(&p, 0x1FFD).is_none());
        assert_eq!(read_overlaps(&p, 0x1FFE).map(|x| x.2), Some("b"));
        assert_eq!(read_overlaps(&p, 0x2000).map(|x| x.2), Some("b"));
        assert!(read_overlaps(&p, 0x2006).is_none());
    }
}
