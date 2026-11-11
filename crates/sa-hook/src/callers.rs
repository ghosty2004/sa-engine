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
//! [`MAX_STEPS`] instructions per path) until the register is fully written, or a `ret`, indirect
//! call or jump or invalid instruction ends the path. A read of a byte of ECX/EDX that the path has
//! not written before is a use of the callee's value. A direct `call` on the path is a use too when
//! the called function reads the register it is entered with ([`callee_effect`]: a thiscall's `this`,
//! a fastcall's arguments; found by INF-20: ApplySpeed calls `ApplyMoveSpeed` then `ApplyTurnSpeed`
//! without reloading ECX, 0x547D96/0x547D9B); the path steps over a callee that keeps the register
//! and ends at one that may write it. Not counted as reads (the same rules as
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
    Code, Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfoFactory, Mnemonic, OpAccess,
    OpKind, Register,
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
        // `xchg [esp], r` with the pushed register: the register gets its value back (protection
        // chunks: `push edx; mov edx, ...; add edx, ...; xchg [esp], edx`)
        if slot == 0
            && i.mnemonic() == Mnemonic::Xchg
            && i.op0_kind() == OpKind::Memory
            && i.memory_base() == Register::ESP
            && i.memory_index() == Register::None
            && i.memory_displacement32() == 0
            && i.op1_kind() == OpKind::Register
            && i.op1_register() == pushed
        {
            return true;
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
            && info.used_registers().iter().any(|u| {
                u.register() == Register::ESP && matches!(u.access(), OpAccess::Write | OpAccess::ReadWrite)
            });
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

/// ECX/EDX accesses of one instruction, with the scan's exceptions applied: `(reg, read, written
/// bytes)`. A read is reported only for a register none of whose bytes `written` (the path's mask,
/// indexed by [`Reg`]) has set. Not counted as reads: `xor/sub/sbb r, r` and `or r, -1` (they set
/// the register), the no-ops `lea r, [r]` (alignment), `shl/shr/sar/rol/ror r, 0`, `xchg r, r` and
/// `mov r, r` (protection-chunk filler), `push r` that is a stack allocation or a save
/// ([`push_is_allocation`]) or the first instruction of a procedure, and the ECX input of `cpuid`
/// for a leaf that ignores it (`mov eax, imm` right before). Once the code writes a part of a
/// register (`setcc cl`, `mov dx, [m]`), a later full-width read is MSVC's partial-register idiom (a
/// `bool`/`int16` pushed or incremented as a dword, the stale upper bytes unused), not a use of the
/// incoming value; hence "none of whose bytes".
fn reg_events(
    exe: &crate::testexe::Exe,
    starts: &[u32],
    f: &mut InstructionInfoFactory,
    i: &Instruction,
    written: [u8; 2],
) -> Vec<(Reg, bool, u8)> {
    let ip = i.ip32();
    let ops: Vec<_> = (0..i.op_count()).map(|k| i.op_kind(k)).collect();
    let same_regs =
        ops.len() == 2 && ops.iter().all(|k| *k == OpKind::Register) && i.op0_register() == i.op1_register();
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
    if lea_nop {
        return Vec::new();
    }
    let push_alloc = matches!(i.code(), Code::Push_r32)
        && matches!(i.op0_register(), Register::ECX | Register::EDX)
        && (starts.binary_search(&ip).is_ok() || push_is_allocation(exe, i));
    let cpuid_leaf = (i.mnemonic() == Mnemonic::Cpuid).then(|| {
        let p = decode_at(exe, ip.wrapping_sub(5));
        (p.code() == Code::Mov_r32_imm32 && p.op0_register() == Register::EAX && p.next_ip32() == ip)
            .then(|| p.immediate32())
    });
    let info = f.info(i);
    let mut out = Vec::new();
    for u in info.used_registers() {
        let Some((reg, m)) = family(u.register()) else {
            continue;
        };
        let reads = matches!(
            u.access(),
            OpAccess::Read | OpAccess::CondRead | OpAccess::ReadWrite | OpAccess::ReadCondWrite
        );
        let cpuid_skip = reg == Reg::Ecx && cpuid_leaf.flatten().is_some_and(cpuid_ignores_ecx);
        let read = reads && written[reg as usize] == 0 && !sets && !push_alloc && !cpuid_skip;
        let w = if matches!(u.access(), OpAccess::Write | OpAccess::ReadWrite) {
            m
        } else {
            0
        };
        if read || w != 0 {
            out.push((reg, read, w));
        }
    }
    out
}

/// What a directly called function does with the ECX/EDX value it is entered with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Some path reads it before writing it: an argument (`this` of a thiscall, the first argument of
    /// a fastcall), or a value the callee hands on. The address of the first read found.
    Reads(u32),
    /// Every path returns without writing it.
    Keeps,
    /// Some path writes it (or the analysis cannot tell: indirect branches, too deep or too long).
    Clobbers,
}

/// Instructions followed per path inside a callee by [`callee_effect`].
pub const CALLEE_STEPS: usize = 512;
/// Nesting of calls followed by [`callee_effect`].
pub const CALLEE_DEPTH: u32 = 3;

/// An instruction of an MSVC prologue: `push r32`/`push imm`, `mov ebp, esp`, `sub esp, imm`, the
/// SEH frame setup (`mov eax, fs:[0]`, `mov fs:[0], esp`) and direct `jmp`s (protection splices).
fn is_prologue(i: &Instruction) -> bool {
    let fs0 = |k: u32| {
        i.op_kind(k) == OpKind::Memory && i.segment_prefix() == Register::FS && i.memory_displacement32() == 0
    };
    match i.code() {
        Code::Push_r32 | Code::Pushd_imm8 | Code::Pushd_imm32 => true,
        Code::Mov_r32_rm32 | Code::Mov_rm32_r32 => {
            (i.op0_register() == Register::EBP && i.op1_register() == Register::ESP)
                || (i.op0_register() == Register::EAX && fs0(1))
                || (fs0(0) && i.op1_register() == Register::ESP)
        }
        Code::Mov_EAX_moffs32 => i.segment_prefix() == Register::FS && i.memory_displacement32() == 0,
        Code::Sub_rm32_imm8 | Code::Sub_rm32_imm32 => i.op0_register() == Register::ESP,
        _ => i.flow_control() == FlowControl::UnconditionalBranch && i.op0_kind() == OpKind::NearBranch32,
    }
}

/// The instruction at `ip`, after at most 3 one-byte `push ecx`/`push edx`, follows a frame setup:
/// `mov ebp, esp` (`8B EC`) or the SEH registration `mov fs:[0], esp` (`64 89 25 00 00 00 00`). Covers
/// prologues reached by a jump (a function entered by a `jcc` from another, or through a protection
/// splice), where the path's prologue flag is lost.
fn after_frame_setup(exe: &crate::testexe::Exe, ip: u32) -> bool {
    let mut at = ip;
    for _ in 0..=3 {
        if exe.bytes(at.wrapping_sub(2), 2) == [0x8B, 0xEC]
            || exe.bytes(at.wrapping_sub(7), 7) == [0x64, 0x89, 0x25, 0, 0, 0, 0]
        {
            return true;
        }
        if !matches!(exe.bytes(at.wrapping_sub(1), 1), [0x51 | 0x52]) {
            return false;
        }
        at -= 1;
    }
    false
}

/// [`Effect`] of the function at `entry` on `reg`, following its direct calls up to [`CALLEE_DEPTH`].
/// Memoised in `memo` (a recursive call in progress counts as [`Effect::Clobbers`]).
pub fn callee_effect(
    exe: &crate::testexe::Exe,
    starts: &[u32],
    entry: u32,
    reg: Reg,
    depth: u32,
    memo: &mut BTreeMap<(u32, Reg), Effect>,
) -> Effect {
    if let Some(&e) = memo.get(&(entry, reg)) {
        return e;
    }
    if depth == 0 || exe.bytes(entry, 1).is_empty() {
        return Effect::Clobbers;
    }
    memo.insert((entry, reg), Effect::Clobbers);
    let mut f = InstructionInfoFactory::new();
    let mut keeps = true;
    // (ip, steps, still in the prologue)
    let mut work = vec![(entry, 0usize, true)];
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    let result = 'walk: {
        while let Some((ip, steps, prologue)) = work.pop() {
            if !seen.insert(ip) {
                continue;
            }
            total += 1;
            if steps >= CALLEE_STEPS
                || total > 8 * CALLEE_STEPS
                || !CODE.iter().any(|&(lo, hi)| (lo..hi).contains(&ip))
            {
                // too long, or outside game code (the no-CD `.text` is never executed: a path that
                // gets there went through protection code the walk cannot follow)
                keeps = false;
                continue;
            }
            let i = decode_at(exe, ip);
            if i.is_invalid() {
                keeps = false;
                continue;
            }
            // `push ecx`/`push edx` in a prologue reserves a local (MSVC: `push ebp; mov ebp, esp;
            // push ecx`, or after the SEH frame setup)
            let prologue = prologue && is_prologue(&i);
            let events = if matches!(i.code(), Code::Push_r32) && (prologue || after_frame_setup(exe, ip)) {
                Vec::new()
            } else {
                reg_events(exe, starts, &mut f, &i, [0, 0])
            };
            let mut written = false;
            for (r, read, w) in events {
                if r == reg && read {
                    break 'walk Effect::Reads(ip);
                }
                written |= r == reg && w != 0;
            }
            if written {
                // this path no longer carries the incoming value
                keeps = false;
                continue;
            }
            let next = i.next_ip32();
            match i.flow_control() {
                FlowControl::Next => work.push((next, steps + 1, prologue)),
                FlowControl::UnconditionalBranch if i.op0_kind() == OpKind::NearBranch32 => {
                    work.push((i.near_branch32(), steps + 1, prologue))
                }
                FlowControl::ConditionalBranch => {
                    work.push((next, steps + 1, prologue));
                    if i.op0_kind() == OpKind::NearBranch32 {
                        work.push((i.near_branch32(), steps + 1, prologue));
                    }
                }
                FlowControl::Call if i.op0_kind() == OpKind::NearBranch32 => {
                    match callee_effect(exe, starts, i.near_branch32(), reg, depth - 1, memo) {
                        Effect::Reads(at) => break 'walk Effect::Reads(at),
                        Effect::Keeps => work.push((next, steps + 1, prologue)),
                        Effect::Clobbers => keeps = false,
                    }
                }
                FlowControl::Return => {}
                // indirect calls and jumps, interrupts
                _ => keeps = false,
            }
        }
        if keeps { Effect::Keeps } else { Effect::Clobbers }
    };
    memo.insert((entry, reg), result);
    result
}

/// Reads of ECX/EDX after the call at `site` that see the callee's value, on any path from
/// `site + 5` (fall-through, direct jumps, both sides of conditional jumps, up to [`MAX_STEPS`]
/// instructions per path), with the exceptions of [`reg_events`]. A path carries a register until it
/// writes it. Calls on the path (INF-20 extension; before, a path ended at the next `call`):
/// - a direct call whose callee reads the register it is entered with ([`Effect::Reads`]: a
///   thiscall's `this` in ECX, a fastcall's arguments in ECX/EDX, at any nesting up to
///   [`CALLEE_DEPTH`]) is a use, reported at the `call` (e.g. 0x547D9B `call ApplyTurnSpeed` right
///   after `call ApplyMoveSpeed`, with `this` still in ECX);
/// - a direct call whose callee keeps the register ([`Effect::Keeps`]) is stepped over, the path
///   still carrying it; any other direct call ends the register's path;
///
/// `ret`, indirect calls and jumps and invalid instructions end the path. Indirect calls are not
/// followed: their convention is unknown (imports and RenderWare's function tables are stdcall/cdecl;
/// treating them as ECX reads gave 60+ false reports), and a virtual call on the object still in ECX
/// first reads ECX to load the vtable, which is reported by itself.
pub fn uses_after(exe: &crate::testexe::Exe, starts: &[u32], site: u32) -> Vec<Use> {
    let mut memo = BTreeMap::new();
    uses_after_memo(exe, starts, site, &mut memo)
}

/// [`uses_after`] sharing the callee analysis across call sites.
pub fn uses_after_memo(
    exe: &crate::testexe::Exe,
    starts: &[u32],
    site: u32,
    memo: &mut BTreeMap<(u32, Reg), Effect>,
) -> Vec<Use> {
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
        for (reg, read, w) in reg_events(exe, starts, &mut f, &i, [wc, wd]) {
            if read {
                out.insert(Use {
                    site,
                    reg,
                    at: ip,
                    text: format!("{:?}", i.code()),
                });
            }
            if reg == Reg::Ecx { wc |= w } else { wd |= w }
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
            FlowControl::Call if i.op0_kind() == OpKind::NearBranch32 => {
                let t = i.near_branch32();
                for (reg, w) in [(Reg::Ecx, &mut wc), (Reg::Edx, &mut wd)] {
                    if *w != 0 {
                        continue;
                    }
                    match callee_effect(exe, starts, t, reg, CALLEE_DEPTH, memo) {
                        Effect::Reads(at) => {
                            out.insert(Use {
                                site,
                                reg,
                                at: ip,
                                text: format!("call {t:#x} (reads {reg:?} at {at:#x})"),
                            });
                            *w = 0xF;
                        }
                        Effect::Keeps => {}
                        Effect::Clobbers => *w = 0xF,
                    }
                }
                work.push((next, wc, wd, steps + 1));
            }
            // ret, indirect calls and jumps, interrupts: end of the path
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
            .chain(sa_world::hooks::ALL.iter())
            .chain(sa_pop::hooks::ALL.iter())
            .chain(sa_script::hooks::ALL.iter())
            .chain(sa_weapon::hooks::ALL.iter())
            .chain(sa_ped::hooks::ALL.iter())
            .chain(sa_input::hooks::ALL.iter())
            .chain(sa_anim::hooks::ALL.iter())
            .chain(sa_veh::hooks::ALL.iter())
            .chain(sa_hud::hooks::ALL.iter())
            .chain(sa_cam::hooks::ALL.iter())
            .chain(sa_ai::hooks::ALL.iter())
            .chain(sa_audio::hooks::ALL.iter())
            .find(|r| r.addr == addr)
            .map_or(0, |r| r.preserve)
    }

    /// Every use of ECX/EDX after a direct call into a hooked function, split by whether `preserved`
    /// declares it: (declared, undeclared). Prints the counts.
    #[allow(clippy::type_complexity)]
    fn scan(exe: &crate::testexe::Exe, preserved: &dyn Fn(u32) -> u8) -> (Vec<(u32, Use)>, Vec<(u32, Use)>) {
        let starts = procedure_starts();
        let targets: BTreeSet<u32> = HOOKS.iter().map(|h| h.addr).collect();
        let sites = call_sites(exe, &targets);
        let (mut real, mut skipped, mut with_callers) = (0, 0, 0);
        let mut undeclared = Vec::new();
        let mut declared_uses = Vec::new();
        let mut memo = BTreeMap::new();
        for (&t, ss) in &sites {
            let mut any = false;
            for &s in ss {
                if !on_boundary(exe, &starts, s) {
                    skipped += 1;
                    continue;
                }
                real += 1;
                any = true;
                for u in uses_after_memo(exe, &starts, s, &mut memo) {
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
        (declared_uses, undeclared)
    }

    /// Every original caller that reads ECX/EDX after a direct call into a hooked function, directly
    /// or by calling a function that takes it as an argument, is matched by a `preserve` declaration on
    /// that hook (finding V-9; the known cases are 0x5B4177 → 0x5B3AD0 and ApplySpeed's
    /// ApplyMoveSpeed → ApplyTurnSpeed).
    #[test]
    fn callers_reading_ecx_edx_after_a_hook_are_declared() {
        let Some(exe) = crate::testexe::load() else { return };
        let (declared_uses, undeclared) = scan(&exe, &preserved);
        for (t, u) in &declared_uses {
            println!(
                "  declared: {t:#x} <- {:#x}: {:?} read at {:#x} `{}`",
                u.site, u.reg, u.at, u.text
            );
        }
        for (t, u) in &undeclared {
            println!(
                "  UNDECLARED: {t:#x} <- {:#x}: {:?} read at {:#x} `{}`",
                u.site, u.reg, u.at, u.text
            );
        }
        assert!(
            undeclared.is_empty(),
            "hooks whose callers read ECX/EDX: add #[hook(preserve = ...)]"
        );
        // the known cases must be found (guards the scan itself)
        assert!(
            declared_uses
                .iter()
                .any(|(t, u)| *t == 0x5B3AD0 && u.site == 0x5B4179 && u.reg == Reg::Edx)
        );
        assert!(
            declared_uses
                .iter()
                .any(|(t, u)| *t == 0x542DD0 && u.site == 0x547D96 && u.at == 0x547D9B)
        );
    }

    /// Negative test: without its `preserve = "ecx"`, CPhysical::ApplyMoveSpeed (0x542DD0) is reported
    /// at every ApplySpeed site that calls ApplyTurnSpeed with `this` still in ECX. The read is inside
    /// the next callee, so the scan before INF-20 (which ended a path at a `call`) missed it.
    #[test]
    fn apply_move_speed_without_preserve_is_flagged() {
        let Some(exe) = crate::testexe::load() else { return };
        let (_, undeclared) = scan(&exe, &|a| if a == 0x542DD0 { 0 } else { preserved(a) });
        let mut sites: Vec<(u32, u32)> = undeclared
            .iter()
            .filter(|(t, u)| *t == 0x542DD0 && u.reg == Reg::Ecx)
            .map(|(_, u)| (u.site, u.at))
            .collect();
        sites.sort();
        assert_eq!(
            sites,
            [
                (0x547D96, 0x547D9B),
                (0x547EDD, 0x547EE2),
                (0x5480B0, 0x5480B5),
                (0x548248, 0x54824D)
            ],
            "{undeclared:?}"
        );
        let starts = procedure_starts();
        let mut memo = BTreeMap::new();
        assert_eq!(
            callee_effect(&exe, &starts, 0x542E20, Reg::Ecx, CALLEE_DEPTH, &mut memo),
            Effect::Reads(0x542E24)
        );
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
        assert_eq!(
            u.iter().map(|x| (x.reg, x.at)).collect::<Vec<_>>(),
            [(Reg::Ecx, 0x71DC7C), (Reg::Ecx, 0x71DC7E), (Reg::Edx, 0x71DC7D)]
        );
        let push = decode_at(&exe, 0x58B3CA);
        assert_eq!(push.code(), Code::Push_r32);
        assert!(push_is_allocation(&exe, &push));
    }
}
