//! 32-bit x86 instruction-length decoder and prologue relocator for inline hooks.
//!
//! The decoder covers the whole legacy one-byte and two-byte (`0F`) opcode maps of 32-bit protected
//! mode, including x87, MMX/SSE, 3DNow! (used by RenderWare), the `0F 38`/`0F 3A` three-byte maps and 16-bit addressing (`67`).
//! VEX/EVEX/XOP encodings do not occur in gta_sa.exe (2005, MSVC 7.1); `C4`/`C5`/`62`/`8F` are decoded
//! as their legacy forms (LES/LDS/BOUND/POP). The tests compare it with iced-x86 over every catalogue
//! function entry of the real exe.
//!
//! [`relocate`] copies whole instructions (≥ `min` bytes) from a function entry to a trampoline and
//! re-encodes every relative branch: `jcc rel8` → `jcc rel32`, `jmp rel8` → `jmp rel32`,
//! `call/jmp/jcc rel32` re-targeted, `loop/jecxz rel8` expanded. Branches that land inside the stolen
//! range are mapped to the copy.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Ran past the end of the supplied bytes.
    Truncated,
    /// Undefined or unsupported opcode (byte, offset).
    Invalid(u8, usize),
    /// More than 15 bytes.
    TooLong,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "instruction runs past the buffer"),
            DecodeError::Invalid(b, o) => write!(f, "invalid/unsupported opcode {b:#04x} at +{o}"),
            DecodeError::TooLong => write!(f, "instruction longer than 15 bytes"),
        }
    }
}

/// A relative branch: where its displacement sits and how wide it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// `70..7F rel8` (condition code 0..15).
    JccShort(u8),
    /// `0F 80..8F rel32`.
    JccNear(u8),
    /// `EB rel8`.
    JmpShort,
    /// `E9 rel32`.
    JmpNear,
    /// `E8 rel32`.
    Call,
    /// `E0..E3 rel8` (loopne, loope, loop, jecxz).
    Loop(u8),
}

impl Branch {
    pub fn disp_size(self) -> usize {
        match self {
            Branch::JccShort(_) | Branch::JmpShort | Branch::Loop(_) => 1,
            _ => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Insn {
    pub len: usize,
    /// Number of prefix bytes before the opcode.
    pub prefixes: usize,
    pub branch: Option<Branch>,
    /// Control never falls through to the next instruction (ret, jmp, iret, ud2, int3).
    pub terminal: bool,
}

impl Insn {
    /// Signed displacement of a relative branch (last bytes of the instruction).
    pub fn rel(&self, code: &[u8]) -> Option<i32> {
        let b = self.branch?;
        let end = self.len;
        Some(match b.disp_size() {
            1 => code[end - 1] as i8 as i32,
            _ => i32::from_le_bytes([code[end - 4], code[end - 3], code[end - 2], code[end - 1]]),
        })
    }

    /// Absolute target of a relative branch when the instruction lives at `addr`.
    pub fn target(&self, code: &[u8], addr: u32) -> Option<u32> {
        self.rel(code)
            .map(|r| addr.wrapping_add(self.len as u32).wrapping_add(r as u32))
    }
}

fn byte(code: &[u8], i: usize) -> Result<u8, DecodeError> {
    code.get(i).copied().ok_or(DecodeError::Truncated)
}

/// Length of a ModRM operand (ModRM + SIB + displacement) at `code[i]`; also returns the reg field.
fn modrm(code: &[u8], i: usize, addr16: bool) -> Result<(usize, u8, u8), DecodeError> {
    let m = byte(code, i)?;
    let (md, reg, rm) = (m >> 6, (m >> 3) & 7, m & 7);
    if md == 3 {
        return Ok((1, reg, md));
    }
    let mut len = 1;
    if addr16 {
        len += match (md, rm) {
            (0, 6) => 2,
            (1, _) => 1,
            (2, _) => 2,
            _ => 0,
        };
        return Ok((len, reg, md));
    }
    if rm == 4 {
        let sib = byte(code, i + 1)?;
        len += 1;
        if md == 0 && sib & 7 == 5 {
            len += 4;
        }
    } else if md == 0 && rm == 5 {
        len += 4;
    }
    len += match md {
        1 => 1,
        2 => 4,
        _ => 0,
    };
    Ok((len, reg, md))
}

/// Operand kinds after the opcode.
#[derive(Clone, Copy)]
enum Ops {
    None,
    Ib,
    Iw,
    /// imm16/32 by operand size.
    Iz,
    M,
    MIb,
    MIz,
    /// moffs: 4 bytes, 2 with `67`.
    Moffs,
    /// ptr16:32 (6 bytes, 4 with `66`).
    Far,
    /// `C8`: iw + ib.
    Enter,
    Rel(Branch),
    Invalid,
}

fn one_byte(op: u8) -> Ops {
    use Ops::*;
    match op {
        0x00..=0x3F => match op & 7 {
            0..=3 => M,
            4 => Ib,
            5 => Iz,
            // 06/07/0E/16/17/1E/1F push/pop seg, 27/2F/37/3F daa/das/aaa/aas. 0F and prefixes never get here.
            _ => None,
        },
        0x40..=0x61 => None,
        0x62 | 0x63 => M,
        0x68 => Iz,
        0x69 => MIz,
        0x6A => Ib,
        0x6B => MIb,
        0x6C..=0x6F => None,
        0x70..=0x7F => Rel(Branch::JccShort(op & 0xF)),
        0x80 | 0x82 | 0x83 => MIb,
        0x81 => MIz,
        0x84..=0x8F => M,
        0x90..=0x99 => None,
        0x9A => Far,
        0x9B..=0x9F => None,
        0xA0..=0xA3 => Moffs,
        0xA4..=0xA7 => None,
        0xA8 => Ib,
        0xA9 => Iz,
        0xAA..=0xAF => None,
        0xB0..=0xB7 => Ib,
        0xB8..=0xBF => Iz,
        0xC0 | 0xC1 => MIb,
        0xC2 => Iw,
        0xC3 => None,
        0xC4 | 0xC5 => M,
        0xC6 => MIb,
        0xC7 => MIz,
        0xC8 => Enter,
        0xC9 => None,
        0xCA => Iw,
        0xCB | 0xCC => None,
        0xCD => Ib,
        0xCE | 0xCF => None,
        0xD0..=0xD3 => M,
        0xD4 | 0xD5 => Ib,
        0xD6 | 0xD7 => None,
        0xD8..=0xDF => M,
        0xE0..=0xE3 => Rel(Branch::Loop(op & 3)),
        0xE4..=0xE7 => Ib,
        0xE8 => Rel(Branch::Call),
        0xE9 => Rel(Branch::JmpNear),
        0xEA => Far,
        0xEB => Rel(Branch::JmpShort),
        0xEC..=0xEF => None,
        0xF1 | 0xF4 | 0xF5 => None,
        0xF6 | 0xF7 => M, // immediate depends on reg, handled by the caller
        0xF8..=0xFD => None,
        0xFE | 0xFF => M,
        _ => Invalid,
    }
}

fn two_byte(op: u8) -> Ops {
    use Ops::*;
    match op {
        0x00..=0x03 => M,
        0x05..=0x09 | 0x0B | 0x0E => None,
        0x0D => M,
        0x0F => MIb, // 3DNow!: ModRM + opcode suffix byte
        0x10..=0x23 => M,
        0x28..=0x2F => M,
        0x30..=0x35 | 0x37 => None,
        0x40..=0x6F => M,
        0x70..=0x73 => MIb,
        0x74..=0x76 => M,
        0x77 => None,
        0x78 | 0x79 | 0x7C..=0x7F => M,
        0x80..=0x8F => Rel(Branch::JccNear(op & 0xF)),
        0x90..=0x9F => M,
        0xA0..=0xA2 => None,
        0xA3 | 0xA5 | 0xAB | 0xAD..=0xAF => M,
        0xA4 | 0xAC => MIb,
        0xA8..=0xAA => None,
        0xB0..=0xB9 | 0xBB..=0xBF => M,
        0xBA => MIb,
        0xC0 | 0xC1 | 0xC3 | 0xC7 => M,
        0xC2 | 0xC4..=0xC6 => MIb,
        0xC8..=0xCF => None,
        0xD0..=0xFF => M,
        _ => Invalid,
    }
}

/// ModRM forms that are undefined (register operand where memory is required, unused group slots).
/// VEX/EVEX/XOP (`C4`/`C5`/`62`/`8F` with mod=3 or reg≠0) and TSX (`C6 F8`, `C7 F8`) are rejected.
fn modrm_invalid(op: u8, op2: Option<u8>, reg: u8, md: u8, rm: u8) -> bool {
    let mem_only = md == 3;
    let reg_form = md == 3;
    match (op, op2) {
        (0x8C, None) => reg > 5,
        (0x8E, None) => reg > 5 || reg == 1,
        // x87 encodings Intel leaves undefined.
        (0xD9, None) => {
            if reg_form {
                (reg == 2 && rm != 0) || (reg == 4 && matches!(rm, 2 | 3 | 6 | 7)) || (reg == 5 && rm == 7)
            } else {
                reg == 1
            }
        }
        (0xDA, None) => reg_form && (reg == 4 || reg == 6 || reg == 7 || (reg == 5 && rm != 1)),
        (0xDB, None) => {
            if reg_form {
                reg == 7 || (reg == 4 && rm > 4)
            } else {
                reg == 4 || reg == 6
            }
        }
        (0xDD, None) => (!reg_form && reg == 5) || (reg_form && reg >= 6),
        (0xDE, None) => reg_form && reg == 3 && rm != 1,
        (0xDF, None) => reg_form && (reg == 7 || (reg == 4 && rm != 0)),
        (0xFF, None) => reg == 7 || (mem_only && (reg == 3 || reg == 5)),
        (0xFE, None) => reg > 1,
        (0x8D | 0xC4 | 0xC5 | 0x62, None) => mem_only,
        (0x8F | 0xC6 | 0xC7, None) => reg != 0,
        (0x0F, Some(0xB2 | 0xB4 | 0xB5)) => mem_only,
        _ => false,
    }
}

/// Defined 3DNow! / 3DNow!+ opcode suffixes (`0F 0F modrm suffix`).
const D3NOW_SUFFIXES: [u8; 24] = [
    0x0C, 0x0D, 0x1C, 0x1D, 0x8A, 0x8E, 0x90, 0x94, 0x96, 0x97, 0x9A, 0x9E, 0xA0, 0xA4, 0xA6, 0xA7, 0xAA,
    0xAE, 0xB0, 0xB4, 0xB6, 0xB7, 0xBB, 0xBF,
];

/// Instructions that accept a `F0` (lock) prefix: read-modify-write with a memory destination.
fn lockable(op: u8, op2: Option<u8>, reg: u8, md: u8) -> bool {
    if md == 3 {
        return false;
    }
    match (op, op2) {
        (0x00..=0x31, None) => op & 7 <= 1 && op & 0x38 != 0x38,
        (0x80..=0x83, None) => reg != 7,
        (0x86 | 0x87, None) => true,
        (0xF6 | 0xF7, None) => reg == 2 || reg == 3,
        (0xFE | 0xFF, None) => reg <= 1,
        (0x0F, Some(0xAB | 0xB3 | 0xBB | 0xB0 | 0xB1 | 0xC0 | 0xC1)) => true,
        (0x0F, Some(0xBA)) => reg >= 5,
        (0x0F, Some(0xC7)) => reg == 1,
        _ => false,
    }
}

/// Decodes one instruction at the start of `code`.
pub fn decode(code: &[u8]) -> Result<Insn, DecodeError> {
    let mut i = 0;
    let (mut op16, mut addr16, mut lock, mut rep) = (false, false, false, false);
    loop {
        match byte(code, i)? {
            0x66 => op16 = true,
            0x67 => addr16 = true,
            0xF0 => lock = true,
            0xF2 | 0xF3 => rep = true,
            0x26 | 0x2E | 0x36 | 0x3E | 0x64 | 0x65 => {}
            _ => break,
        }
        i += 1;
        if i >= 15 {
            return Err(DecodeError::TooLong);
        }
    }
    let prefixes = i;
    let op = byte(code, i)?;
    i += 1;
    let iz = if op16 { 2 } else { 4 };
    let mut terminal = matches!(op, 0xC2 | 0xC3 | 0xCA | 0xCB | 0xCC | 0xCF | 0xE9 | 0xEA | 0xEB);
    let mut op2 = None;
    let ops = if op == 0x0F {
        let o2 = byte(code, i)?;
        op2 = Some(o2);
        let op2 = o2;
        i += 1;
        match op2 {
            0x38 => {
                byte(code, i)?;
                i += 1;
                Ops::M
            }
            0x3A => {
                byte(code, i)?;
                i += 1;
                Ops::MIb
            }
            0x0B | 0xB9 | 0xFF => {
                terminal = true; // ud2 / ud1 / ud0
                two_byte(op2)
            }
            _ => two_byte(op2),
        }
    } else {
        one_byte(op)
    };
    if lock && !matches!(ops, Ops::M | Ops::MIb | Ops::MIz) {
        return Err(DecodeError::Invalid(op, prefixes));
    }
    // punpck*qdq / movq need a mandatory prefix.
    if op2.is_some_and(|o| o == 0x6C || o == 0x6D) && !op16 || op2 == Some(0xD6) && !op16 && !rep {
        return Err(DecodeError::Invalid(op2.unwrap_or(op), prefixes));
    }
    let mut branch = None;
    let len = match ops {
        Ops::None => i,
        Ops::Ib => i + 1,
        Ops::Iw => i + 2,
        Ops::Iz => i + iz,
        Ops::Moffs => i + if addr16 { 2 } else { 4 },
        Ops::Far => i + if op16 { 4 } else { 6 },
        Ops::Enter => i + 3,
        Ops::M | Ops::MIb | Ops::MIz => {
            let (l, reg, md) = modrm(code, i, addr16)?;
            let rm = code[i] & 7;
            if modrm_invalid(op, op2, reg, md, rm) || (lock && !lockable(op, op2, reg, md)) {
                return Err(DecodeError::Invalid(op2.unwrap_or(op), prefixes));
            }
            if op2 == Some(0x0F) && !D3NOW_SUFFIXES.contains(code.get(i + l).ok_or(DecodeError::Truncated)?) {
                return Err(DecodeError::Invalid(0x0F, prefixes));
            }
            let imm = match ops {
                Ops::MIb => 1,
                Ops::MIz => iz,
                _ => match (op, reg) {
                    (0xF6, 0 | 1) => 1,
                    (0xF7, 0 | 1) => iz,
                    _ => 0,
                },
            };
            if op == 0xFF && (reg == 4 || reg == 5) {
                terminal = true; // indirect jmp / jmp far
            }
            i + l + imm
        }
        Ops::Rel(b) => {
            if op16 && matches!(b, Branch::Call | Branch::JmpNear | Branch::JccNear(_)) {
                // rel16 forms truncate EIP to 16 bits: never valid in this exe.
                return Err(DecodeError::Invalid(op, prefixes));
            }
            branch = Some(b);
            i + b.disp_size()
        }
        Ops::Invalid => return Err(DecodeError::Invalid(code[i - 1], i - 1)),
    };
    if len > 15 {
        return Err(DecodeError::TooLong);
    }
    if len > code.len() {
        return Err(DecodeError::Truncated);
    }
    Ok(Insn {
        len,
        prefixes,
        branch,
        terminal,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocError {
    Decode(DecodeError, usize),
    /// Control leaves the function (ret/jmp) before `min` bytes: too short for a 5-byte patch.
    TooShort(usize),
    /// A branch inside the stolen bytes targets the middle of a stolen instruction.
    MidInstruction(usize),
}

impl std::fmt::Display for RelocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelocError::Decode(e, o) => write!(f, "decode at +{o}: {e}"),
            RelocError::TooShort(o) => write!(f, "function ends at +{o}, before the 5-byte patch"),
            RelocError::MidInstruction(o) => write!(f, "branch at +{o} targets the middle of a stolen insn"),
        }
    }
}

pub fn rel32_to(site: u32, len: u32, target: u32) -> i32 {
    target.wrapping_sub(site.wrapping_add(len)) as i32
}

/// Encodes `jmp rel32` at `site` to `target`.
pub fn jmp_rel32(site: u32, target: u32) -> [u8; 5] {
    let r = rel32_to(site, 5, target).to_le_bytes();
    [0xE9, r[0], r[1], r[2], r[3]]
}

/// Whole instructions covering at least `min` bytes; errors if control flow ends first.
pub fn steal(code: &[u8], min: usize) -> Result<Vec<Insn>, RelocError> {
    let mut out = Vec::new();
    let mut n = 0;
    while n < min {
        let insn = decode(&code[n..]).map_err(|e| RelocError::Decode(e, n))?;
        n += insn.len;
        if insn.terminal && n < min {
            return Err(RelocError::TooShort(n));
        }
        out.push(insn);
    }
    Ok(out)
}

/// Bytes an instruction occupies once relocated.
fn reloc_size(insn: &Insn) -> usize {
    match insn.branch {
        None => insn.len,
        Some(Branch::JccShort(_)) => 6,
        Some(Branch::JmpShort | Branch::JmpNear | Branch::Call) => 5,
        Some(Branch::JccNear(_)) => 6,
        // loop L1; jmp short L2; L1: jmp rel32 target; L2:
        Some(Branch::Loop(_)) => 2 + 2 + 5,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocated {
    /// Code for the trampoline, including the final `jmp src+consumed`.
    pub code: Vec<u8>,
    /// Source bytes consumed (≥ `min`).
    pub consumed: usize,
}

/// Copies the first ≥ `min` bytes of `code` (located at `src`) so they run at `dst`, followed by a
/// `jmp` back to `src + consumed`.
pub fn relocate(code: &[u8], src: u32, dst: u32, min: usize) -> Result<Relocated, RelocError> {
    let insns = steal(code, min)?;
    let mut src_off = Vec::with_capacity(insns.len());
    let mut dst_off = Vec::with_capacity(insns.len());
    let (mut s, mut d) = (0usize, 0usize);
    for insn in &insns {
        src_off.push(s);
        dst_off.push(d);
        s += insn.len;
        d += reloc_size(insn);
    }
    let consumed = s;
    let map = |target: u32, at: usize| -> Result<u32, RelocError> {
        let off = target.wrapping_sub(src) as usize;
        if target >= src && off < consumed {
            match src_off.iter().position(|&o| o == off) {
                Some(k) => Ok(dst + dst_off[k] as u32),
                None => Err(RelocError::MidInstruction(at)),
            }
        } else {
            Ok(target)
        }
    };
    let mut out = Vec::with_capacity(d + 5);
    for (k, insn) in insns.iter().enumerate() {
        let so = src_off[k];
        let bytes = &code[so..so + insn.len];
        let Some(b) = insn.branch else {
            out.extend_from_slice(bytes);
            continue;
        };
        let target = map(insn.target(bytes, src + so as u32).expect("branch"), so)?;
        let here = dst + out.len() as u32;
        match b {
            Branch::JccShort(cc) | Branch::JccNear(cc) => {
                out.extend_from_slice(&[0x0F, 0x80 | cc]);
                out.extend_from_slice(&rel32_to(here, 6, target).to_le_bytes());
            }
            Branch::JmpShort | Branch::JmpNear => out.extend_from_slice(&jmp_rel32(here, target)),
            Branch::Call => {
                out.push(0xE8);
                out.extend_from_slice(&rel32_to(here, 5, target).to_le_bytes());
            }
            Branch::Loop(n) => {
                // Keep a 0x67 prefix (cx-based loop) if present; others are meaningless here.
                let addr16 = bytes[..insn.prefixes].contains(&0x67);
                if addr16 {
                    return Err(RelocError::Decode(DecodeError::Invalid(0x67, so), so));
                }
                out.extend_from_slice(&[0xE0 | n, 0x02, 0xEB, 0x05]);
                let here = dst + out.len() as u32;
                out.extend_from_slice(&jmp_rel32(here, target));
            }
        }
    }
    let here = dst + out.len() as u32;
    out.extend_from_slice(&jmp_rel32(here, src + consumed as u32));
    Ok(Relocated { code: out, consumed })
}

/// Like [`relocate`], but when the last stolen instruction is a `call rel32` it becomes
/// `push src+consumed; jmp target` and the trailing jmp is dropped: the callee then returns straight
/// to the original code after the stolen range, with the *original* return address on the stack.
/// Observers that key on the caller (determinism mode's rand log hashes `rand()`'s return address)
/// see the same value whether the hook is installed or not. The result is equivalent: the callee
/// returns to `src+consumed`, where the plain trampoline's trailing jmp would have gone.
pub fn relocate_keep_return(code: &[u8], src: u32, dst: u32, min: usize) -> Result<Relocated, RelocError> {
    let insns = steal(code, min)?;
    let consumed: usize = insns.iter().map(|i| i.len).sum();
    let Some(last) = insns.last() else {
        return relocate(code, src, dst, min);
    };
    if last.branch != Some(Branch::Call) || insns.len() == 1 {
        return relocate(code, src, dst, min);
    }
    let call_at = consumed - last.len;
    let target = last
        .target(&code[call_at..consumed], src + call_at as u32)
        .expect("branch");
    if target >= src && target < src + consumed as u32 {
        // a call into the stolen range itself: keep the general form
        return relocate(code, src, dst, min);
    }
    // Relocate everything before the call (which ends in a jmp back to the call site) and replace
    // that trailing jmp with push + jmp.
    let mut r = relocate(code, src, dst, call_at)?;
    debug_assert_eq!(r.consumed, call_at);
    r.code.truncate(r.code.len() - 5);
    r.code.push(0x68); // push imm32
    r.code.extend_from_slice(&(src + consumed as u32).to_le_bytes());
    let here = dst + r.code.len() as u32;
    r.code.extend_from_slice(&jmp_rel32(here, target));
    r.consumed = consumed;
    Ok(r)
}

/// Linear sweep over `code` (at `base`): the first relative branch whose target lies strictly inside
/// `(lo, hi)`, i.e. one that would land in the NOP padding of a patched entry. Stops at the first
/// undecodable byte. A heuristic safety check, not a proof.
pub fn inbound_branch(code: &[u8], base: u32, lo: u32, hi: u32) -> Option<(u32, u32)> {
    let mut n = 0;
    while n < code.len() {
        let Ok(insn) = decode(&code[n..]) else { break };
        let at = base + n as u32;
        if let Some(t) = insn.target(&code[n..n + insn.len], at)
            && t > lo
            && t < hi
        {
            return Some((at, t));
        }
        n += insn.len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn len(b: &[u8]) -> usize {
        decode(b).unwrap().len
    }

    #[test]
    fn lengths() {
        assert_eq!(len(&[0x53]), 1); // push ebx
        assert_eq!(len(&[0x83, 0xEC, 0x0C]), 3); // sub esp,0xc
        assert_eq!(len(&[0x8B, 0x5C, 0x24, 0x0C]), 4); // mov ebx,[esp+0xc]
        assert_eq!(len(&[0x6A, 0xFF]), 2);
        assert_eq!(len(&[0x68, 0x58, 0xED, 0x83, 0x00]), 5);
        assert_eq!(len(&[0x64, 0xA1, 0, 0, 0, 0]), 6); // mov eax,fs:[0]
        assert_eq!(len(&[0x66, 0xFF, 0x05, 0xF4, 0x47, 0xA4, 0x00]), 7); // inc word [abs]
        assert_eq!(len(&[0x66, 0xC7, 0x00, 1, 2]), 5); // mov word [eax],imm16
        assert_eq!(len(&[0x81, 0x4E, 0x1C, 0x80, 0, 0, 0]), 7);
        assert_eq!(len(&[0x8B, 0x0D, 0x28, 0xCB, 0xB7, 0x00]), 6); // CTimer::Update entry
        assert_eq!(len(&[0xD9, 0x44, 0x24, 0x04]), 4); // fld [esp+4]
        assert_eq!(len(&[0xD9, 0x05, 0xC0, 0x3A, 0x86, 0x00]), 6); // fld [abs]
        assert_eq!(len(&[0xDF, 0xE0]), 2); // fnstsw ax
        assert_eq!(len(&[0xF6, 0xC4, 0x41]), 3); // test ah,0x41
        assert_eq!(len(&[0xF7, 0xD8]), 2); // neg eax
        assert_eq!(len(&[0xF7, 0xC1, 1, 0, 0, 0]), 6); // test ecx,1
        assert_eq!(len(&[0x8B, 0x04, 0x25, 1, 2, 3, 4]), 7); // SIB, disp32, no base
        assert_eq!(len(&[0x8D, 0xA4, 0x24, 0, 0, 0, 0]), 7); // lea esp,[esp+0]
        assert_eq!(len(&[0x0F, 0xB6, 0x44, 0x24, 0x04]), 5); // movzx eax,byte [esp+4]
        assert_eq!(len(&[0x0F, 0x28, 0xC1]), 3); // movaps
        assert_eq!(len(&[0xF3, 0x0F, 0x10, 0x05, 1, 2, 3, 4]), 8); // movss xmm0,[abs]
        assert_eq!(len(&[0x0F, 0x3A, 0x0F, 0xC1, 0x08]), 5); // palignr
        assert_eq!(len(&[0x0F, 0x38, 0x00, 0xC1]), 4); // pshufb
        assert_eq!(len(&[0xC8, 0x10, 0x00, 0x01]), 4); // enter
        assert_eq!(len(&[0xC2, 0x08, 0x00]), 3); // ret 8
        assert_eq!(len(&[0x67, 0x8B, 0x46, 0x10]), 4); // 16-bit addressing, disp8
        assert_eq!(len(&[0x67, 0x8B, 0x06, 0x34, 0x12]), 5); // [disp16]
        assert_eq!(len(&[0xA1, 1, 2, 3, 4]), 5);
        assert_eq!(len(&[0x67, 0xA1, 1, 2]), 4);
        assert_eq!(len(&[0x9A, 1, 2, 3, 4, 5, 6]), 7);
        assert_eq!(len(&[0x0F, 0x0F, 0xC1, 0x9E]), 4); // 3DNow! pfadd
        assert!(decode(&[0x0F, 0x0F, 0xC1, 0x00]).is_err()); // undefined 3DNow! suffix
        assert_eq!(len(&[0xF3, 0xA5]), 2); // rep movsd
    }

    #[test]
    fn flags() {
        assert!(decode(&[0xC3]).unwrap().terminal);
        assert!(decode(&[0xFF, 0x25, 1, 2, 3, 4]).unwrap().terminal); // jmp [abs]
        assert!(!decode(&[0xFF, 0x15, 1, 2, 3, 4]).unwrap().terminal); // call [abs]
        assert!(decode(&[0x0F, 0x0B]).unwrap().terminal);
        assert_eq!(decode(&[0x74, 0x05]).unwrap().branch, Some(Branch::JccShort(4)));
        assert_eq!(
            decode(&[0x0F, 0x84, 0, 0, 0, 0]).unwrap().branch,
            Some(Branch::JccNear(4))
        );
        assert_eq!(decode(&[0x8B]), Err(DecodeError::Truncated));
        assert!(matches!(
            decode(&[0x0F, 0x04]),
            Err(DecodeError::Invalid(0x04, 1))
        ));
        assert!(matches!(
            decode(&[0x66, 0xE8, 0, 0]),
            Err(DecodeError::Invalid(0xE8, 1))
        ));
        assert_eq!(
            decode(&[0x3E, 0x74, 0x05]).unwrap(),
            Insn {
                len: 3,
                prefixes: 1,
                branch: Some(Branch::JccShort(4)),
                terminal: false
            }
        );
    }

    #[test]
    fn steal_whole_instructions() {
        // CTimer::Update: mov ecx,[0xb7cb28] (6)
        let p = [0x8B, 0x0D, 0x28, 0xCB, 0xB7, 0x00, 0x83, 0xEC, 0x0C];
        assert_eq!(relocate(&p, 0x561B10, 0x1000, 5).unwrap().consumed, 6);
        // CGeneral::LimitRadianAngle: fld [esp+4] (4) + fld [0x863ac0] (6)
        let p = [
            0xD9, 0x44, 0x24, 0x04, 0xD9, 0x05, 0xC0, 0x3A, 0x86, 0x00, 0xD8, 0xD9,
        ];
        assert_eq!(relocate(&p, 0x53CB50, 0x1000, 5).unwrap().consumed, 10);
        // A function that returns before 5 bytes cannot be patched.
        assert_eq!(
            steal(&[0x33, 0xC0, 0xC3, 0x90, 0x90], 5),
            Err(RelocError::TooShort(3))
        );
    }

    fn target_of(code: &[u8], at: u32) -> u32 {
        decode(code).unwrap().target(code, at).unwrap()
    }

    #[test]
    fn relocates_rel32_call_and_jmp() {
        // push -1; jmp 0x40136c   (at 0x5E8620)
        let code = [0x6A, 0xFF, 0xE9, 0x45, 0x8D, 0xE1, 0xFF, 0x64];
        let r = relocate(&code, 0x5E8620, 0x1000_0000, 5).unwrap();
        assert_eq!(r.consumed, 7);
        assert_eq!(&r.code[..2], &[0x6A, 0xFF]);
        assert_eq!(target_of(&r.code[2..7], 0x1000_0002), 0x40136C);
        // call rel32 then a byte
        let code = [0xE8, 0x10, 0x00, 0x00, 0x00, 0x90];
        let r = relocate(&code, 0x400000, 0x2000_0000, 5).unwrap();
        assert_eq!(target_of(&r.code[..5], 0x2000_0000), 0x400015);
        // trailing jmp back
        assert_eq!(target_of(&r.code[5..10], 0x2000_0005), 0x400005);
    }

    #[test]
    fn trailing_call_keeps_the_original_return_address() {
        // 0x407180: push ecx; push esi; call 0x821B1E; mov esi,[esp+0Ch]
        let code = [0x51, 0x56, 0xE8, 0x97, 0xA9, 0x41, 0x00, 0x8B, 0x74, 0x24, 0x0C];
        let r = relocate_keep_return(&code, 0x407180, 0x2000_0000, 5).unwrap();
        assert_eq!(r.consumed, 7);
        assert_eq!(&r.code[..3], &[0x51, 0x56, 0x68]);
        assert_eq!(u32::from_le_bytes(r.code[3..7].try_into().unwrap()), 0x407187);
        assert_eq!(target_of(&r.code[7..12], 0x2000_0007), 0x821B1E);
        assert_eq!(r.code.len(), 12);
        // a call that is not the last stolen instruction: the plain form
        let code = [0xE8, 0x1A, 0x60, 0x00, 0x00, 0x8B, 0x48, 0x14];
        assert_eq!(
            relocate_keep_return(&code, 0x821B1E, 0x2000_0000, 5),
            relocate(&code, 0x821B1E, 0x2000_0000, 5)
        );
        // no call at all: the plain form
        let code = [0x8B, 0x51, 0x08, 0x56, 0x32, 0xC0, 0x57];
        assert_eq!(
            relocate_keep_return(&code, 0x5E45E0, 0x2000_0000, 5),
            relocate(&code, 0x5E45E0, 0x2000_0000, 5)
        );
    }

    #[test]
    fn widens_short_branches() {
        // test eax,eax; je +0x10; push esi   at 0x500000
        let code = [0x85, 0xC0, 0x74, 0x10, 0x56, 0x57];
        let r = relocate(&code, 0x500000, 0x3000_0000, 5).unwrap();
        assert_eq!(r.consumed, 5);
        assert_eq!(&r.code[..4], &[0x85, 0xC0, 0x0F, 0x84]);
        assert_eq!(target_of(&r.code[2..8], 0x3000_0002), 0x500014);
        assert_eq!(r.code[8], 0x56);
        assert_eq!(target_of(&r.code[9..14], 0x3000_0009), 0x500005);
        // jmp short with a prefix-free rel8 backwards
        let code = [0xEB, 0xFE, 0x90, 0x90, 0x90];
        let r = relocate(&code, 0x500000, 0x3000_0000, 2).unwrap();
        // jumps to itself: inside the stolen range → mapped to the copy
        assert_eq!(target_of(&r.code[..5], 0x3000_0000), 0x3000_0000);
        // ... but a short jmp is terminal, so it cannot start a 5-byte steal
        assert_eq!(steal(&code, 5), Err(RelocError::TooShort(2)));
    }

    #[test]
    fn branch_into_stolen_range_maps_to_copy() {
        // L0: dec ecx; jne L0 (rel8 -3); nop; nop     at 0x600000
        let code = [0x49, 0x75, 0xFD, 0x90, 0x90, 0x90];
        let r = relocate(&code, 0x600000, 0x4000_0000, 5).unwrap();
        assert_eq!(r.consumed, 5);
        assert_eq!(&r.code[..3], &[0x49, 0x0F, 0x85]);
        assert_eq!(target_of(&r.code[1..7], 0x4000_0001), 0x4000_0000);
        // Target in the middle of a stolen instruction is refused.
        let code = [0xB8, 1, 2, 3, 4, 0x75, 0xFA, 0x90];
        assert_eq!(
            relocate(&code, 0x600000, 0x4000_0000, 6),
            Err(RelocError::MidInstruction(5))
        );
    }

    #[test]
    fn expands_loop() {
        // loop +0x20; nop x4   at 0x700000 → target 0x700022
        let code = [0xE2, 0x20, 0x90, 0x90, 0x90, 0x90];
        let r = relocate(&code, 0x700000, 0x5000_0000, 5).unwrap();
        assert_eq!(&r.code[..4], &[0xE2, 0x02, 0xEB, 0x05]);
        assert_eq!(target_of(&r.code[4..9], 0x5000_0004), 0x700022);
        assert_eq!(&r.code[9..12], &[0x90, 0x90, 0x90]);
    }

    /// Every function entry in the catalogue (functions.csv + procedures.csv): decode up to 32
    /// instructions with our decoder and with iced-x86; lengths and branch targets must agree. Then
    /// relocate the entry (≥ 5 bytes) and check with iced that the copy is equivalent.
    #[test]
    fn differential_vs_iced_on_the_exe() {
        use iced_x86::{Decoder, DecoderOptions, FlowControl};
        let Some(exe) = crate::testexe::load() else { return };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut addrs = std::collections::BTreeSet::new();
        for f in ["catalog/functions.csv", "catalog/procedures.csv"] {
            let csv = std::fs::read_to_string(root.join(f)).unwrap();
            for l in csv.lines().skip(1) {
                if let Some(h) = l.split(',').next().and_then(|a| a.strip_prefix("0x")) {
                    addrs.insert(u32::from_str_radix(h, 16).unwrap());
                }
            }
        }
        let iced_one = |code: &[u8], ip: u32| {
            let mut d = Decoder::with_ip(32, code, ip as u64, DecoderOptions::NONE);
            d.decode()
        };
        let (mut insns, mut funcs, mut relocated, mut too_short, mut both_invalid) = (0, 0, 0, 0, 0);
        for &a in &addrs {
            let code = exe.bytes(a, 32 * 15);
            if code.is_empty() {
                continue;
            }
            funcs += 1;
            let mut n = 0;
            for _ in 0..32 {
                if n >= code.len() {
                    break;
                }
                let at = a + n as u32;
                let ice = iced_one(&code[n..], at);
                let ours = decode(&code[n..]);
                match ours {
                    Err(DecodeError::Truncated) => break,
                    Err(e) => {
                        assert!(
                            ice.is_invalid(),
                            "{at:#x}: ours {e}, iced {:?} ({:02x?})",
                            ice.code(),
                            &code[n..(n + 8).min(code.len())]
                        );
                        both_invalid += 1;
                        break;
                    }
                    Ok(insn) => {
                        assert!(!ice.is_invalid(), "{at:#x}: iced invalid, ours len {}", insn.len);
                        assert_eq!(
                            insn.len,
                            ice.len(),
                            "{at:#x}: {:?} {:02x?}",
                            ice.code(),
                            &code[n..n + ice.len()]
                        );
                        let rel = matches!(
                            ice.flow_control(),
                            FlowControl::UnconditionalBranch
                                | FlowControl::ConditionalBranch
                                | FlowControl::Call
                        ) && ice.near_branch_target() != 0
                            && !ice.is_jmp_far()
                            && !ice.is_call_far();
                        assert_eq!(insn.branch.is_some(), rel, "{at:#x}: {:?}", ice.code());
                        if rel {
                            assert_eq!(insn.target(&code[n..], at), Some(ice.near_branch_target() as u32));
                        }
                        insns += 1;
                        n += insn.len;
                        if insn.terminal {
                            assert!(
                                matches!(
                                    ice.flow_control(),
                                    FlowControl::Return
                                        | FlowControl::UnconditionalBranch
                                        | FlowControl::IndirectBranch
                                        | FlowControl::Interrupt
                                        | FlowControl::Exception
                                ),
                                "{at:#x}: {:?}",
                                ice.code()
                            );
                            break;
                        }
                    }
                }
            }
            // Relocation equivalence.
            let dst = 0x2000_0000;
            let r = match relocate(code, a, dst, 5) {
                Ok(r) => r,
                Err(RelocError::TooShort(_)) => {
                    too_short += 1;
                    continue;
                }
                Err(_) => continue,
            };
            relocated += 1;
            let (mut s, mut d) = (0usize, 0usize);
            while s < r.consumed {
                let o = iced_one(&code[s..], a + s as u32);
                let c = iced_one(&r.code[d..], dst + d as u32);
                let insn = decode(&code[s..]).unwrap();
                if let Some(Branch::Loop(_)) = insn.branch {
                    d += 9;
                } else if insn.branch.is_some() {
                    assert_eq!(o.flow_control(), c.flow_control(), "{a:#x}+{s}");
                    assert_eq!(o.condition_code(), c.condition_code(), "{a:#x}+{s}");
                    let t = o.near_branch_target() as u32;
                    let want = if t >= a && t < a + r.consumed as u32 {
                        None
                    } else {
                        Some(t)
                    };
                    if let Some(w) = want {
                        assert_eq!(c.near_branch_target() as u32, w, "{a:#x}+{s}");
                    }
                    d += c.len();
                } else {
                    assert_eq!(&r.code[d..d + insn.len], &code[s..s + insn.len]);
                    d += insn.len;
                }
                s += insn.len;
            }
            let back = iced_one(&r.code[d..], dst + d as u32);
            assert_eq!(back.near_branch_target() as u32, a + r.consumed as u32, "{a:#x}");
            assert_eq!(d + 5, r.code.len());
        }
        eprintln!(
            "differential: {funcs} entries, {insns} insns agree, {both_invalid} both-invalid stops, \
             {relocated} relocated + verified, {too_short} too short for 5 bytes"
        );
        assert!(funcs > 18_000 && insns > 250_000);
    }

    /// Linear sweep over the whole game `.text` (0x401000–0x857000): every position where both decoders
    /// accept an instruction must give the same length; where one rejects, so must the other.
    #[test]
    fn linear_sweep_vs_iced() {
        use iced_x86::{Decoder, DecoderOptions};
        let Some(exe) = crate::testexe::load() else { return };
        let (lo, hi) = (0x401000u32, 0x857000u32);
        let code = exe.bytes(lo, (hi - lo) as usize);
        let mut dec = Decoder::with_ip(32, code, lo as u64, DecoderOptions::NONE);
        let (mut n, mut ok, mut invalid) = (0usize, 0usize, 0usize);
        let mut bad: Vec<String> = Vec::new();
        while n < code.len() {
            dec.set_position(n).unwrap();
            dec.set_ip((lo as usize + n) as u64);
            let ice = dec.decode();
            let at = lo as usize + n;
            let raw = &code[n..(n + 8).min(code.len())];
            match decode(&code[n..]) {
                Ok(i) => {
                    if ice.is_invalid() || i.len != ice.len() {
                        bad.push(format!(
                            "{at:#x}: ours len {}, iced {:?} len {} {raw:02x?}",
                            i.len,
                            ice.code(),
                            ice.len()
                        ));
                    }
                    ok += 1;
                    n += i.len;
                }
                Err(DecodeError::Truncated) => break,
                Err(e) => {
                    // rel16 branches (66-prefixed) are rejected on purpose: they truncate EIP.
                    // VEX (C4/C5 with mod=3) and 3DNow! are not supported on purpose: absent from the exe.
                    let name = format!("{:?}", ice.code());
                    let rel16 = name.ends_with("rel16") || name.starts_with("VEX_");
                    if !ice.is_invalid() && !rel16 {
                        bad.push(format!(
                            "{at:#x}: ours {e}, iced {:?} len {} {raw:02x?}",
                            ice.code(),
                            ice.len()
                        ));
                    }
                    invalid += 1;
                    n += 1;
                }
            }
        }
        eprintln!(
            "linear sweep: {ok} insns decoded, {invalid} positions rejected, {} disagreements",
            bad.len()
        );
        assert!(
            bad.is_empty(),
            "{}",
            bad.iter().take(60).cloned().collect::<Vec<_>>().join("\n")
        );
        assert!(ok > 1_000_000);
    }

    #[test]
    fn inbound_branch_detection() {
        // entry: 6-byte mov, then jmp back to entry+2 (into the padding)
        let code = [0x8B, 0x0D, 1, 2, 3, 4, 0xEB, 0xFA];
        assert_eq!(
            inbound_branch(&code, 0x1000, 0x1000, 0x1006),
            Some((0x1006, 0x1002))
        );
        // a jump to the entry itself is fine (the patch starts there)
        let code = [0x8B, 0x0D, 1, 2, 3, 4, 0xEB, 0xF8];
        assert_eq!(inbound_branch(&code, 0x1000, 0x1000, 0x1006), None);
    }
}
