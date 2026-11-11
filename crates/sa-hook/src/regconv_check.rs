//! Test-only (HARNESS-5): every `#[hook(conv = reg("..."))]` registration against the exe.
//!
//! A register-convention spec is a claim about the original's code, so it is checked there:
//! 1. **inputs**: every register the spec names is read (in the bytes the location covers) on some
//!    path from the entry before the path writes it;
//! 2. **no missing input**: no other general register (EAX..EBP) is read before written on any path
//!    from the entry, except as a save (`push r`) — a read the spec does not list would be an argument
//!    the Rust port never receives;
//! 3. **stack cleanup**: every reachable `ret` pops `4 * stack words` with `pop`, nothing without;
//! 4. **result bits callers read**: after every direct call into the hook, a caller that reads EAX
//!    bytes the hook does not define (all of EAX for a void result, EAX[31:8] for an AL result,
//!    EAX[31:16] for AX) must be covered by the spec (`keep = eax` for void, `ret = al:entry` /
//!    `ax:entry`); ECX/EDX reads are the `callers` test's (`keep = ecx/edx` feed `preserve`).
//!
//! Paths follow fall-through, direct jumps, both sides of conditional jumps and the entries of a
//! switch table `jmp [r*4 + table]` (up to [`STEPS`] instructions); a `call` (direct or indirect) is
//! stepped over as an ABI call (EAX/ECX/EDX written, the others kept); `ret` and other indirect
//! branches end a path. `mov r32, r32` of one register (an alignment no-op) is neither a read nor a
//! write. With `ret = al:entry` / `ax:entry` the EAX bytes the result keeps from the entry are not
//! inputs (e.g. `_rpMeshGetNextSerialNumber`'s `lea esi, [eax + 1]` after `mov ax, ...`). Byte granularity as `callers`: AL, AH and
//! the upper half are tracked separately.

use crate::callers::{CODE, call_sites, on_boundary, procedure_starts};
use crate::testexe::Exe;
use iced_x86::{Code, Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfoFactory, OpAccess, OpKind, Register};
use std::collections::{BTreeMap, BTreeSet};

/// Instructions followed per path.
const STEPS: usize = 400;

fn decode(exe: &Exe, ip: u32) -> Instruction {
    let b = exe.bytes(ip, 16);
    Decoder::with_ip(32, b, ip as u64, DecoderOptions::NONE).decode()
}

/// (register index EAX 0 .. EBP 6, byte mask: 1 low byte, 2 byte 1, 0xC upper half)
fn family(r: Register) -> Option<(usize, u8)> {
    use Register::*;
    Some(match r {
        EAX => (0, 0xF),
        AX => (0, 3),
        AL => (0, 1),
        AH => (0, 2),
        ECX => (1, 0xF),
        CX => (1, 3),
        CL => (1, 1),
        CH => (1, 2),
        EDX => (2, 0xF),
        DX => (2, 3),
        DL => (2, 1),
        DH => (2, 2),
        EBX => (3, 0xF),
        BX => (3, 3),
        BL => (3, 1),
        BH => (3, 2),
        ESI => (4, 0xF),
        SI => (4, 3),
        EDI => (5, 0xF),
        DI => (5, 3),
        EBP => (6, 0xF),
        BP => (6, 3),
        _ => return Option::None,
    })
}

/// Reads and writes of instruction `i`: per register (mask read, mask written).
fn events(f: &mut InstructionInfoFactory, i: &Instruction) -> [(u8, u8); 7] {
    let mut out = [(0u8, 0u8); 7];
    // `xor r, r` / `sub r, r`: a write
    let same = i.op_count() == 2
        && i.op0_kind() == OpKind::Register
        && i.op1_kind() == OpKind::Register
        && i.op0_register() == i.op1_register()
        && matches!(i.mnemonic(), iced_x86::Mnemonic::Xor | iced_x86::Mnemonic::Sub);
    // `mov edi, edi` (MSVC's 2-byte alignment no-op, e.g. 0x4CB56E): neither a read nor a write
    if matches!(i.code(), Code::Mov_r32_rm32 | Code::Mov_rm32_r32)
        && i.op0_kind() == OpKind::Register
        && i.op1_kind() == OpKind::Register
        && i.op0_register() == i.op1_register()
    {
        return out;
    }
    let info = f.info(i);
    for u in info.used_registers() {
        let Some((r, m)) = family(u.register()) else { continue };
        let (rd, wr) = match u.access() {
            OpAccess::Read | OpAccess::CondRead => (true, false),
            OpAccess::Write | OpAccess::CondWrite => (false, true),
            OpAccess::ReadWrite | OpAccess::ReadCondWrite => (true, true),
            _ => (false, false),
        };
        if rd && !same {
            out[r].0 |= m;
        }
        // a conditional write may not happen: the path still carries the entry value
        if wr && !matches!(u.access(), OpAccess::CondWrite | OpAccess::ReadCondWrite) {
            // a 16-/8-bit write leaves the other bytes; a 32-bit write covers all
            out[r].1 |= m;
        }
    }
    out
}

/// The result of walking every path from `start` with the register bytes in `carried` (per
/// register, the bytes still holding the entry value): the first read of carried bytes per register
/// (address), the `ret` immediates seen.
struct Walk {
    reads: [Option<u32>; 7],
    /// reads that are not `push r` saves
    uses: [Option<u32>; 7],
    rets: BTreeSet<u16>,
}

fn walk(exe: &Exe, start: u32, carried: [u8; 7]) -> Walk {
    let mut f = InstructionInfoFactory::new();
    let mut w = Walk { reads: [None; 7], uses: [None; 7], rets: BTreeSet::new() };
    let mut work = vec![(start, carried, 0usize)];
    let mut seen = BTreeSet::new();
    while let Some((ip, mut c, steps)) = work.pop() {
        if steps >= STEPS || !seen.insert((ip, c)) {
            continue;
        }
        if !CODE.iter().any(|&(lo, hi)| (lo..hi).contains(&ip)) {
            continue;
        }
        let i = decode(exe, ip);
        if i.is_invalid() {
            continue;
        }
        let push = matches!(i.code(), Code::Push_r32 | Code::Push_r16);
        for (r, (rd, wr)) in events(&mut f, &i).into_iter().enumerate() {
            if rd & c[r] != 0 {
                w.reads[r].get_or_insert(ip);
                if !push {
                    w.uses[r].get_or_insert(ip);
                }
            }
            c[r] &= !wr;
        }
        let next = i.next_ip32();
        match i.flow_control() {
            FlowControl::Next => work.push((next, c, steps + 1)),
            FlowControl::UnconditionalBranch if i.op0_kind() == OpKind::NearBranch32 => {
                work.push((i.near_branch32(), c, steps + 1))
            }
            FlowControl::ConditionalBranch => {
                work.push((next, c, steps + 1));
                if i.op0_kind() == OpKind::NearBranch32 {
                    work.push((i.near_branch32(), c, steps + 1));
                }
            }
            FlowControl::Call | FlowControl::IndirectCall => {
                // an ABI call (direct, or through an IAT cell / a vtable): EAX/ECX/EDX written by the
                // callee, the rest kept
                c[0] = 0;
                c[1] = 0;
                c[2] = 0;
                work.push((next, c, steps + 1));
            }
            FlowControl::Return => {
                w.rets.insert(if i.op_count() == 1 { i.immediate16() } else { 0 });
            }
            FlowControl::IndirectBranch => {
                for t in jump_table(exe, &i) {
                    work.push((t, c, steps + 1));
                }
            }
            _ => {}
        }
    }
    w
}

/// The targets of a switch `jmp dword ptr [r*4 + table]`: the table's consecutive dwords while
/// each is a code address within 0x1000 bytes of the jump (MSVC places the tables right after the
/// function, so this may run into a following table of the same function: more paths, never fewer).
fn jump_table(exe: &Exe, i: &Instruction) -> Vec<u32> {
    let mut out = Vec::new();
    if i.op0_kind() != OpKind::Memory
        || i.memory_base() != Register::None
        || i.memory_index() == Register::None
        || i.memory_index_scale() != 4
    {
        return out;
    }
    let table = i.memory_displacement32();
    if !CODE.iter().any(|&(lo, hi)| (lo..hi).contains(&table)) {
        return out;
    }
    let near = |t: u32| t.abs_diff(i.ip32()) < 0x1000 && CODE.iter().any(|&(lo, hi)| (lo..hi).contains(&t));
    for k in 0..256u32 {
        let b = exe.bytes(table + 4 * k, 4);
        let t = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        if !near(t) {
            break;
        }
        out.push(t);
    }
    out
}

fn reg_hooks() -> Vec<&'static sa_core::abi::HookReg> {
    sa_ports::ALL.iter().filter(|r| r.regs.is_some()).collect()
}

/// The entry of a hook's original code (a `.HOODLUM` stub's target).
fn body(exe: &Exe, r: &sa_core::abi::HookReg) -> u32 {
    if r.is_hoodlum_stub() {
        let b = exe.bytes(r.addr + 1, 4);
        r.addr.wrapping_add(5).wrapping_add(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    } else {
        r.addr
    }
}

/// Checks 1-3 of the module docs for one spec at `entry`.
fn check_spec(exe: &Exe, name: &str, entry: u32, c: &sa_core::regconv::RegConv, stack_words: u8) -> Vec<String> {
    let mut bad = Vec::new();
    let mut carried = [0xF; 7];
    carried[0] = match c.ret {
        sa_core::regconv::RetMode::AlEntry => 0x1,
        sa_core::regconv::RetMode::AxEntry => 0x3,
        _ => 0xF,
    };
    let w = walk(exe, entry, carried);
    let names = sa_core::regconv::REG_NAMES;
    for k in 0..c.nargs as usize {
        let Some(reg) = c.args[k].reg() else { continue };
        if w.reads[reg].is_none() {
            bad.push(format!("{name} {entry:#x}: argument {k} in {} is never read", names[reg]));
        }
    }
    let inputs = c.input_regs();
    for (reg, at) in w.uses.iter().enumerate() {
        if let Some(at) = at
            && inputs >> reg & 1 == 0
        {
            bad.push(format!(
                "{name} {entry:#x}: reads {} at {at:#x} before writing it, but the spec has no argument there",
                names[reg]
            ));
        }
    }
    let want = if c.pop { 4 * u16::from(stack_words) } else { 0 };
    if w.rets.iter().any(|&p| p != want) || w.rets.is_empty() {
        bad.push(format!("{name} {entry:#x}: returns pop {:?}, the spec says {want}", w.rets));
    }
    bad
}

#[test]
fn register_conventions_match_the_exe() {
    let Some(exe) = crate::testexe::load() else { return };
    let hooks = reg_hooks();
    assert!(!hooks.is_empty());
    let bad: Vec<String> = hooks
        .iter()
        .flat_map(|r| check_spec(&exe, r.name, body(&exe, r), &r.regs.unwrap(), r.stack_words))
        .collect();
    assert!(bad.is_empty(), "register-convention specs disagree with the exe:\n{}", bad.join("\n"));
}

/// Negative tests: wrong specs for known functions are reported.
#[test]
fn wrong_specs_are_reported() {
    use sa_core::regconv::RegConv;
    let Some(exe) = crate::testexe::load() else { return };
    let n = |a: u32, s: &str, words: u8| check_spec(&exe, "t", a, &RegConv::parse(s), words).len();
    assert_eq!(n(0x5DBE40, "args = ebx esi", 0), 0);
    assert_eq!(n(0x5DBE40, "args = ebx", 0), 1, "ESI read without an argument");
    assert_eq!(n(0x5DBE40, "args = ebx esi edx", 0), 1, "EDX never read");
    assert_eq!(n(0x70FA70, "args = ecx edx di stack", 1), 0);
    assert_eq!(n(0x70FA70, "args = ecx edx stack", 1), 1, "DI read without an argument");
    assert_eq!(n(0x70FA70, "args = ecx edx di stack; pop", 1), 1, "plain ret, not ret 4");
    assert_eq!(n(0x6FD7C0, "args = cl stack stack", 2), 2, "AL read, CL never read");
    assert_eq!(n(0x709CF0, "args = eax ebx edi", 0), 0);
}

/// EAX bytes a caller may read after the call: those the hook does not define.
fn undefined_eax(r: &sa_core::abi::HookReg) -> u8 {
    use sa_core::abi::AbiRet;
    let c = r.regs.unwrap();
    match (c.ret, r.ret) {
        // `:entry` modes and `keep = eax`: the entry returns the caller's bytes
        (sa_core::regconv::RetMode::AlEntry | sa_core::regconv::RetMode::AxEntry, _) => 0,
        (_, AbiRet::Void) if c.keep & 1 != 0 => 0,
        (_, AbiRet::Void | AbiRet::St0) => 0xF,
        (_, AbiRet::Al) => 0xE,
        (_, AbiRet::Ax) => 0xC,
        _ => 0,
    }
}

#[test]
fn callers_read_only_defined_result_bits() {
    let Some(exe) = crate::testexe::load() else { return };
    let starts = procedure_starts();
    let hooks: BTreeMap<u32, &sa_core::abi::HookReg> = reg_hooks().into_iter().map(|r| (r.addr, r)).collect();
    let targets: BTreeSet<u32> = hooks.keys().copied().collect();
    let mut bad = Vec::new();
    let mut checked = 0;
    for (t, sites) in call_sites(&exe, &targets) {
        let r = hooks[&t];
        let m = undefined_eax(r);
        for s in sites {
            if !on_boundary(&exe, &starts, s) {
                continue;
            }
            checked += 1;
            let mut c = [0u8; 7];
            c[0] = m;
            let w = walk(&exe, s + 5, c);
            if let Some(at) = w.uses[0] {
                bad.push(format!(
                    "{} {t:#x}: the caller at {s:#x} reads EAX bytes the hook leaves undefined at {at:#x} (keep = eax / ret = al:entry / ax:entry?)",
                    r.name
                ));
            }
        }
    }
    assert!(checked > 0);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The walker on known code: 0x6FD7C0 reads AL (`movzx eax, al`) and its stack words, pops none;
/// a spec that put the character in ECX would be caught (ECX never read; AL read without an argument).
#[test]
fn walker_on_known_code() {
    let Some(exe) = crate::testexe::load() else { return };
    let w = walk(&exe, 0x6FD7C0, [0xF; 7]);
    assert_eq!(w.uses[0], Some(0x6FD7C0));
    assert!(w.uses[1].is_none());
    assert_eq!(w.rets, BTreeSet::from([0]));
    // ToggleEdge 0x70FA70: ECX, EDX and DI read; EBX/EBP/ESI only pushed (saves)
    let w = walk(&exe, 0x70FA70, [0xF; 7]);
    assert!(w.uses[1].is_some() && w.uses[2].is_some() && w.uses[5].is_some());
    assert!(w.reads[3].is_some() && w.uses[3].is_none() && w.uses[6].is_none() && w.uses[4].is_none());
    // the caller of AudioLog10 at 0x4D8D4C does not read EAX afterwards
    let w = walk(&exe, 0x4D8D51, [0xF, 0, 0, 0, 0, 0, 0]);
    assert!(w.uses[0].is_none());
    // ... while the caller of GetColModel in _CalcDistanceSqrToEntity uses its EAX (`mov edi, eax`),
    // which a void hook there would be reported for
    let w = walk(&exe, 0x5DBE4B, [0xF, 0, 0, 0, 0, 0, 0]);
    assert_eq!(w.uses[0], Some(0x5DBE4B));
    let w = walk(&exe, 0x5DBE4B, [0xC, 0, 0, 0, 0, 0, 0]);
    assert_eq!(w.uses[0], Some(0x5DBE4B), "upper EAX bytes of an AX result");
}

