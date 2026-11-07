//! Test-only (VER-2, verifier finding V-9): original callers that rely on a hooked function leaving
//! ECX or EDX unchanged.
//!
//! The C calling conventions make ECX and EDX caller-saved, so a Rust shim may return with any value
//! in them. MSVC's whole-program optimisation sometimes keeps a value in one of them across a call
//! whose callee it knows does not write it (0x5B4177: `mov edx, eax; call 0x5B3AD0; add esp, 8;
//! test dl, 0x20`). Such a hook must declare `#[hook(preserve = "edx")]`.
//!
//! For every hook, every direct call (`E8 rel32`) into its address in the game code (`.text`
//! 0x401000–0x857000 and `.HOODLUM` 0x1556000–0x1577000) is checked: from the instruction after the
//! call, all paths are followed (fall-through, direct jumps, both sides of conditional jumps, up to
//! [`MAX_STEPS`] instructions per path) until the register is fully written, or a `call`, `ret`,
//! indirect jump or invalid instruction ends the path. A read of a byte of ECX/EDX that the path has
//! not written before is a use of the callee's value. Not counted as reads (the same rules as
//! `tools/verify/regs_after.py`):
//! - `xor r, r` / `sub r, r` (a write);
//! - `lea r, [r]` (an alignment no-op);
//! - `push ecx` / `push edx` used as stack allocation: the pushed slot `[esp]` is written (dword,
//!   write-only) before anything reads it or moves ESP (e.g. `push ecx; ...; fstp dword ptr [esp]`).
//!
//! A site is only checked when the `E8` lies on an instruction boundary of a linear decode from the
//! start of its containing catalogued procedure (otherwise the bytes are part of another instruction
//! or data). Calls through pointers and virtual calls are not covered (their call sites are not
//! known statically).

use iced_x86::{
    Code, Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfoFactory, Mnemonic,
    OpAccess, OpKind, Register,
};
use std::collections::{BTreeMap, BTreeSet};

/// Instructions followed per path after the call.
pub const MAX_STEPS: usize = 64;

/// Game code ranges scanned for call sites.
pub const CODE: [(u32, u32); 2] = [(0x401000, 0x857000), (0x1556000, 0x1577000)];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reg {
    Ecx,
    Edx,
}

impl Reg {
    pub fn mask(self) -> u8 {
        match self {
            Reg::Ecx => sa_core::abi::PRESERVE_ECX,
            Reg::Edx => sa_core::abi::PRESERVE_EDX,
        }
    }
}

/// ECX/EDX family and byte mask of a register (bit 0 = low byte, bit 1 = byte 1, bits 2-3 = upper
/// half).
fn family(r: Register) -> Option<(Reg, u8)> {
    Some(match r {
        Register::ECX => (Reg::Ecx, 0xF),
        Register::CX => (Reg::Ecx, 0x3),
        Register::CL => (Reg::Ecx, 0x1),
        Register::CH => (Reg::Ecx, 0x2),
        Register::EDX => (Reg::Edx, 0xF),
        Register::DX => (Reg::Edx, 0x3),
        Register::DL => (Reg::Edx, 0x1),
        Register::DH => (Reg::Edx, 0x2),
        _ => return None,
    })
}

/// A read of the callee's ECX/EDX after a call.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Use {
    pub site: u32,
    pub reg: Reg,
    pub at: u32,
    pub text: String,
}

fn decode_at(exe: &crate::testexe::Exe, ip: u32) -> Instruction {
    let b = exe.bytes(ip, 16);
    let mut d = Decoder::with_ip(32, b, ip as u64, DecoderOptions::NONE);
    d.decode()
}

/// `push ecx`/`push edx` that does not hand the value on: the slot is overwritten (a whole-dword,
/// write-only access) before anything reads it (MSVC reserves 4 bytes of locals or of an outgoing
/// float argument this way), or it is popped back into the same register (a save/restore pair, as
/// in the protection chunks). The slot is tracked on the straight-line code after the push
/// (following direct `jmp`s) through pushes, pops, `add/sub esp, imm` and `mov r, esp` aliases. A
/// conditional branch, a call, another ESP write, a read of the slot or popping it elsewhere means
/// "not shown".
fn push_is_allocation(exe: &crate::testexe::Exe, push: &Instruction) -> bool {
    let pushed = push.op0_register();
    let mut f = InstructionInfoFactory::new();
    let mut ip = push.next_ip32();
    // offset of the slot from the current ESP; registers holding a copy of ESP (with the slot's
    // offset from them)
    let mut slot: i64 = 0;
    let mut alias: Vec<(Register, i64)> = Vec::new();
    for _ in 0..48 {
        let i = decode_at(exe, ip);
        if i.is_invalid() {
            return false;
        }
        match i.flow_control() {
            FlowControl::Next => {}
            FlowControl::UnconditionalBranch if i.op0_kind() == OpKind::NearBranch32 => {
                ip = i.near_branch32();
                continue;
            }
            _ => return false,
        }
        if slot == 0 && i.code() == Code::Pop_r32 {
            return i.op0_register() == pushed;
        }
        let info = f.info(&i);
        for m in info.used_memory() {
            let base_slot = if m.base() == Register::ESP {
                slot
            } else if let Some(&(_, s)) = alias.iter().find(|a| a.0 == m.base()) {
                s
            } else {
                continue;
            };
            if m.index() != Register::None {
                return false;
            }
            let lo = m.displacement() as u32 as i32 as i64;
            let hi = lo + m.memory_size().size() as i64;
            if hi <= base_slot || lo >= base_slot + 4 {
                continue;
            }
            return m.access() == OpAccess::Write && lo <= base_slot && hi >= base_slot + 4;
        }
        if i.mnemonic() == Mnemonic::Mov
            && i.op0_kind() == OpKind::Register
            && i.op1_kind() == OpKind::Register
            && i.op1_register() == Register::ESP
        {
            alias.push((i.op0_register(), slot));
            ip = i.next_ip32();
            continue;
        }
        for u in info.used_registers() {
            if matches!(u.access(), OpAccess::Write | OpAccess::ReadWrite) {
                alias.retain(|a| a.0 != u.register());
            }
        }
        let inc = i.stack_pointer_increment() as i64;
        let esp_imm = matches!(i.mnemonic(), Mnemonic::Add | Mnemonic::Sub)
            && i.op0_kind() == OpKind::Register
            && i.op0_register() == Register::ESP
            && matches!(i.op1_kind(), OpKind::Immediate8to32 | OpKind::Immediate32);
        let other_esp_write = !esp_imm
            && inc == 0
            && info
                .used_registers()
                .iter()
                .any(|u| u.register() == Register::ESP && matches!(u.access(), OpAccess::Write | OpAccess::ReadWrite));
        if other_esp_write {
            return false;
        }
        let delta = if esp_imm {
            let v = i.immediate(1) as u32 as i32 as i64;
            if i.mnemonic() == Mnemonic::Add { v } else { -v }
        } else {
            inc
        };
        slot -= delta;
        if slot < 0 {
            // released by `add esp` unread (a pop of it was handled above)
            return esp_imm;
        }
        ip = i.next_ip32();
    }
    false
}

/// CPUID leaves whose result does not depend on ECX.
fn cpuid_ignores_ecx(leaf: u32) -> bool {
    matches!(leaf, 0..=3 | 5 | 6 | 0x8000_0000..=0x8000_0008)
}

/// Reads of ECX/EDX after the call at `site` that see the callee's value, on any path from
/// `site + 5`. A read counts only while the path has written no byte of that register: once the
/// code writes a part (`setcc cl`, `mov dx, [m]`), a later full-width read is MSVC's
/// partial-register idiom (a `bool`/`int16` pushed or incremented as a dword, the stale upper bytes
/// unused), not a use of the callee's value. Not counted either: `xor/sub/sbb r, r` and
/// `or r, -1` (they set the register), the no-ops `lea r, [r]` (alignment), `shl/shr/sar/rol/ror r, 0`,
/// `xchg r, r` and `mov r, r` (protection-chunk filler), `push r` that is a stack
/// allocation or a save ([`push_is_allocation`]) or the first instruction of a procedure, and the
/// ECX input of `cpuid` for a leaf that ignores it (`mov eax, imm` right before).
pub fn uses_after(exe: &crate::testexe::Exe, starts: &[u32], site: u32) -> Vec<Use> {
    let mut f = InstructionInfoFactory::new();
    let mut out: BTreeSet<Use> = BTreeSet::new();
    // (ip, written ECX bytes, written EDX bytes, steps)
    let mut work = vec![(site + 5, 0u8, 0u8, 0usize)];
    let mut seen = BTreeSet::new();
    while let Some((ip, mut wc, mut wd, steps)) = work.pop() {
        if steps >= MAX_STEPS || (wc != 0 && wd != 0) || !seen.insert((ip, wc, wd)) {
            continue;
        }
        let i = decode_at(exe, ip);
        if i.is_invalid() {
            continue;
        }
        let ops: Vec<_> = (0..i.op_count()).map(|k| i.op_kind(k)).collect();
        let same_regs = ops.len() == 2
            && ops.iter().all(|k| *k == OpKind::Register)
            && i.op0_register() == i.op1_register();
        let sets = (same_regs && matches!(i.mnemonic(), Mnemonic::Xor | Mnemonic::Sub | Mnemonic::Sbb))
            || (i.mnemonic() == Mnemonic::Or
                && ops.len() == 2
                && ops[0] == OpKind::Register
                && matches!(ops[1], OpKind::Immediate8to32 | OpKind::Immediate32)
                && i.immediate(1) as u32 == u32::MAX);
        let lea_nop = (i.mnemonic() == Mnemonic::Lea
            && i.memory_base() == i.op0_register()
            && i.memory_index() == Register::None
            && i.memory_displacement32() == 0)
            || (matches!(
                i.mnemonic(),
                Mnemonic::Shl | Mnemonic::Shr | Mnemonic::Sar | Mnemonic::Rol | Mnemonic::Ror
            ) && ops.len() == 2
                && ops[0] == OpKind::Register
                && ops[1] == OpKind::Immediate8
                && i.immediate8() & 0x1F == 0)
            || (same_regs && matches!(i.mnemonic(), Mnemonic::Xchg | Mnemonic::Mov));
        let push_alloc = matches!(i.code(), Code::Push_r32)
            && matches!(i.op0_register(), Register::ECX | Register::EDX)
            && (starts.binary_search(&ip).is_ok() || push_is_allocation(exe, &i));
        let cpuid_leaf = (i.mnemonic() == Mnemonic::Cpuid).then(|| {
            let p = decode_at(exe, ip.wrapping_sub(5));
            (p.code() == Code::Mov_r32_imm32 && p.op0_register() == Register::EAX && p.next_ip32() == ip)
                .then(|| p.immediate32())
        });
        if !lea_nop {
            let info = f.info(&i);
            let mut writes: Vec<(Reg, u8)> = Vec::new();
            for u in info.used_registers() {
                let Some((reg, m)) = family(u.register()) else { continue };
                let written = if reg == Reg::Ecx { wc } else { wd };
                let reads = matches!(
                    u.access(),
                    OpAccess::Read | OpAccess::CondRead | OpAccess::ReadWrite | OpAccess::ReadCondWrite
                );
                let cpuid_skip = reg == Reg::Ecx && cpuid_leaf.flatten().is_some_and(cpuid_ignores_ecx);
                if reads && written == 0 && !sets && !push_alloc && !cpuid_skip {
                    out.insert(Use {
                        site,
                        reg,
                        at: ip,
                        text: format!("{:?}", i.code()),
                    });
                }
                if matches!(u.access(), OpAccess::Write | OpAccess::ReadWrite) {
                    writes.push((reg, m));
                }
            }
            for (reg, m) in writes {
                if reg == Reg::Ecx {
                    wc |= m
                } else {
                    wd |= m
                }
            }
        }
        let next = i.next_ip32();
        match i.flow_control() {
            FlowControl::Next => work.push((next, wc, wd, steps + 1)),
            FlowControl::UnconditionalBranch if i.op0_kind() == OpKind::NearBranch32 => {
                work.push((i.near_branch32(), wc, wd, steps + 1))
            }
            FlowControl::ConditionalBranch => {
                work.push((next, wc, wd, steps + 1));
                if matches!(i.op0_kind(), OpKind::NearBranch32) {
                    work.push((i.near_branch32(), wc, wd, steps + 1));
                }
            }
            // call, ret, indirect jumps, interrupts: end of the path
            _ => {}
        }
    }
    out.into_iter().collect()
}

/// Catalogued procedure starts (functions.csv + procedures.csv) and the start of every code range
/// of `catalog/extents.csv` (the `.HOODLUM` bodies and other chunks), sorted.
pub fn procedure_starts() -> Vec<u32> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut s = BTreeSet::new();
    let ext = std::fs::read_to_string(root.join("catalog/extents.csv")).unwrap();
    for l in ext.lines().skip(1) {
        // column 11: `lo-hi;lo-hi;...`
        if let Some(ranges) = l.split(',').nth(10) {
            for r in ranges.split(';') {
                if let Some(h) = r.split('-').next().and_then(|a| a.strip_prefix("0x")) {
                    s.insert(u32::from_str_radix(h, 16).unwrap());
                }
            }
        }
    }
    for f in ["catalog/functions.csv", "catalog/procedures.csv"] {
        let csv = std::fs::read_to_string(root.join(f)).unwrap();
        for l in csv.lines().skip(1) {
            if let Some(h) = l.split(',').next().and_then(|a| a.strip_prefix("0x")) {
                s.insert(u32::from_str_radix(h, 16).unwrap());
            }
        }
    }
    s.into_iter().collect()
}

/// The `E8` at `site` starts an instruction of a linear decode from its containing procedure.
pub fn on_boundary(exe: &crate::testexe::Exe, starts: &[u32], site: u32) -> bool {
    let k = starts.partition_point(|&a| a <= site);
    if k == 0 {
        return false;
    }
    let start = starts[k - 1];
    let code = exe.bytes(start, (site - start) as usize + 5);
    let mut d = Decoder::with_ip(32, code, start as u64, DecoderOptions::NONE);
    let mut i = Instruction::default();
    while d.can_decode() {
        d.decode_out(&mut i);
        if i.ip32() == site {
            return i.mnemonic() == Mnemonic::Call;
        }
        if i.ip32() > site {
            return false;
        }
    }
    false
}

/// Every direct call site into each address of `targets`: address → call sites.
pub fn call_sites(exe: &crate::testexe::Exe, targets: &BTreeSet<u32>) -> BTreeMap<u32, Vec<u32>> {
    let mut out: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (lo, hi) in CODE {
        let code = exe.bytes(lo, (hi - lo) as usize);
        for o in 0..code.len().saturating_sub(4) {
            if code[o] != 0xE8 {
                continue;
            }
            let rel = i32::from_le_bytes([code[o + 1], code[o + 2], code[o + 3], code[o + 4]]);
            let site = lo + o as u32;
            let t = site.wrapping_add(5).wrapping_add(rel as u32);
            if targets.contains(&t) {
                out.entry(t).or_default().push(site);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::HOOKS;

    /// Registers each hook preserves (`#[hook(preserve = ...)]`; sa-hook's own hooks preserve none).
    fn preserved(addr: u32) -> u8 {
        sa_core::hooks::ALL
            .iter()
            .chain(sa_stream::hooks::ALL.iter())
            .find(|r| r.addr == addr)
            .map_or(0, |r| r.preserve)
    }

    /// Every original caller that reads ECX/EDX after a direct call into a hooked function is matched
    /// by a `preserve` declaration on that hook (finding V-9; the known case is 0x5B4177 → 0x5B3AD0).
    #[test]
    fn callers_reading_ecx_edx_after_a_hook_are_declared() {
        let Some(exe) = crate::testexe::load() else { return };
        let starts = procedure_starts();
        let targets: BTreeSet<u32> = HOOKS.iter().map(|h| h.addr).collect();
        let sites = call_sites(&exe, &targets);
        let (mut real, mut skipped, mut with_callers) = (0, 0, 0);
        let mut undeclared = Vec::new();
        let mut declared_uses = Vec::new();
        for (&t, ss) in &sites {
            let mut any = false;
            for &s in ss {
                if !on_boundary(&exe, &starts, s) {
                    skipped += 1;
                    continue;
                }
                real += 1;
                any = true;
                for u in uses_after(&exe, &starts, s) {
                    if preserved(t) & u.reg.mask() == 0 {
                        undeclared.push((t, u));
                    } else {
                        declared_uses.push((t, u));
                    }
                }
            }
            with_callers += any as usize;
        }
        println!(
            "{} hooks, {with_callers} with direct callers, {real} call sites checked, {skipped} E8 matches not on an instruction boundary",
            HOOKS.len()
        );
        for (t, u) in &declared_uses {
            println!("  declared: {t:#x} <- {:#x}: {:?} read at {:#x} `{}`", u.site, u.reg, u.at, u.text);
        }
        for (t, u) in &undeclared {
            println!("  UNDECLARED: {t:#x} <- {:#x}: {:?} read at {:#x} `{}`", u.site, u.reg, u.at, u.text);
        }
        assert!(undeclared.is_empty(), "hooks whose callers read ECX/EDX: add #[hook(preserve = ...)]");
        // the known case must be found (guards the scan itself)
        assert!(declared_uses.iter().any(|(t, u)| *t == 0x5B3AD0 && u.site == 0x5B4179 && u.reg == Reg::Edx));
    }

    /// The scan's exceptions on fixed code (the push-as-allocation sites the verifier listed).
    #[test]
    fn scan_rules_on_known_sites() {
        let Some(exe) = crate::testexe::load() else { return };
        let u = uses_after(&exe, &procedure_starts(), 0x5B4179);
        assert_eq!(u.len(), 1, "{u:?}");
        assert_eq!((u[0].reg, u[0].at), (Reg::Edx, 0x5B4181));
        let starts = procedure_starts();
        assert!(on_boundary(&exe, &starts, 0x5B4179));
        assert!(!on_boundary(&exe, &starts, 0x5B417A));
        // DoesInfiniteLineTouchScreen pushes ECX/EDX again after its first call (0x71DC7C..7E)
        let u = uses_after(&exe, &starts, 0x71DC62);
        assert_eq!(u.iter().map(|x| (x.reg, x.at)).collect::<Vec<_>>(), [(Reg::Ecx, 0x71DC7C), (Reg::Ecx, 0x71DC7E), (Reg::Edx, 0x71DC7D)]);
        let push = decode_at(&exe, 0x58B3CA);
        assert_eq!(push.code(), Code::Push_r32);
        assert!(push_is_allocation(&exe, &push));
    }
}
