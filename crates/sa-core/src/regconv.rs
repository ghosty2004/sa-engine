//! Register calling conventions (HARNESS-5, ADR-001 §4b): functions the compiler gave a non-standard
//! convention (arguments in EAX/EBX/ESI/EDI, a byte in AL, results in AL/AX with the caller's upper
//! EAX, an 80-bit ST0 result, callers relying on registers the callee leaves alone).
//!
//! One spec string describes such a function, used by both `#[hook(conv = reg("..."))]` and
//! `orig! { fn f(..) = reg("...") 0xADDR => path; }`:
//!
//! ```text
//! args = ebx esi               one location per Rust argument, in order:
//!                              eax ecx edx ebx esi edi ebp   (32-bit register)
//!                              ax cx dx bx si di             (16-bit register; the type is 2 bytes)
//!                              al cl dl bl                   (8-bit register; the type is 1 byte)
//!                              stack                         (the next stack slot(s), in order)
//!                              omitted: every argument on the stack
//! ret = al:entry               the result: auto (default: from the Rust type, see below),
//!                              al / ax (only the low byte/word is defined),
//!                              al:entry / ax:entry (upper EAX bits = the caller's EAX at entry)
//! keep = ecx edx               caller-saved registers returned unchanged (eax only for void results)
//! pop                          the callee pops its stack arguments (`ret 4n`); default: caller pops
//! ```
//!
//! Clauses are separated by `;`, register lists by spaces or commas. Examples:
//! `reg("args = ebx esi")` (CPlantMgr::_CalcDistanceSqrToEntity, X87 result in ST0),
//! `reg("args = al stack stack")` (GetCharacterPositionInCharSet), `reg("args = stack")` with an `X87`
//! result (AudioLog10: the full 80-bit FYL2X value in ST0), `reg("args = ecx edx di stack; pop")`.
//!
//! Results ("auto"): integers and pointers in EAX (u64/i64 in EDX:EAX), `bool`/`u8`/`i8` in AL,
//! `f32`/`f64` in ST0 (loaded exactly), [`sa_math::x87::X87`] in ST0 with all 80 bits (`fld tbyte`, so
//! a 64-bit FYL2X/FPTAN result survives; the standard conventions return it as an f64). EBX, ESI, EDI
//! and EBP are always returned unchanged; EAX/EDX not carrying the result are returned as the caller
//! passed them (so `keep = eax` costs nothing for a void function).
//!
//! Phase A (`#[hook]`, i686): a naked entry moves the registers into an `extern "C"` shim (register
//! arguments as words, stack arguments copied with their types), calls it with a 16-byte aligned stack,
//! loads EAX/EDX/ST0 from the shim's [`RegOut`] and returns as the original. `orig!`: [`call_regs`]
//! loads the registers and stack words, calls, and captures EAX..EDI and (for a float result) ST0.
//! Phases B/C and host tests: the registers are just the Rust arguments.
#![allow(unsafe_code)]

use crate::abi::AbiRet;
use sa_math::x87::X87;

/// Most arguments of a register-convention function.
pub const MAX_ARGS: usize = 12;
/// Most stack argument words [`call_regs`] passes.
pub const MAX_STACK: usize = 16;

/// Where one argument lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Loc {
    Stack = 0,
    Eax,
    Ecx,
    Edx,
    Ebx,
    Esi,
    Edi,
    Ebp,
    Ax,
    Cx,
    Dx,
    Bx,
    Al,
    Cl,
    Dl,
    Bl,
    Si,
    Di,
}

/// Register index order of [`Loc::reg`], [`RegConv::keep`], [`RegFrame::regs`].
pub const REG_NAMES: [&str; 7] = ["eax", "ecx", "edx", "ebx", "esi", "edi", "ebp"];

impl Loc {
    /// Index of the 32-bit register (0 = EAX, 1 = ECX, 2 = EDX, 3 = EBX, 4 = ESI, 5 = EDI, 6 = EBP).
    pub const fn reg(self) -> Option<usize> {
        Some(match self {
            Loc::Stack => return None,
            Loc::Eax | Loc::Ax | Loc::Al => 0,
            Loc::Ecx | Loc::Cx | Loc::Cl => 1,
            Loc::Edx | Loc::Dx | Loc::Dl => 2,
            Loc::Ebx | Loc::Bx | Loc::Bl => 3,
            Loc::Esi | Loc::Si => 4,
            Loc::Edi | Loc::Di => 5,
            Loc::Ebp => 6,
        })
    }
    /// Bytes of the register the function reads (0 for the stack).
    pub const fn width(self) -> usize {
        match self {
            Loc::Stack => 0,
            Loc::Al | Loc::Cl | Loc::Dl | Loc::Bl => 1,
            Loc::Ax | Loc::Cx | Loc::Dx | Loc::Bx | Loc::Si | Loc::Di => 2,
            _ => 4,
        }
    }
    const fn parse(w: &[u8]) -> Option<Loc> {
        Some(match w {
            b"stack" => Loc::Stack,
            b"eax" => Loc::Eax,
            b"ecx" => Loc::Ecx,
            b"edx" => Loc::Edx,
            b"ebx" => Loc::Ebx,
            b"esi" => Loc::Esi,
            b"edi" => Loc::Edi,
            b"ebp" => Loc::Ebp,
            b"ax" => Loc::Ax,
            b"cx" => Loc::Cx,
            b"dx" => Loc::Dx,
            b"bx" => Loc::Bx,
            b"al" => Loc::Al,
            b"cl" => Loc::Cl,
            b"dl" => Loc::Dl,
            b"bl" => Loc::Bl,
            b"si" => Loc::Si,
            b"di" => Loc::Di,
            _ => return None,
        })
    }
}

/// The `ret = ...` clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RetMode {
    /// from the Rust type (see the module docs)
    Auto = 0,
    /// AL only (a 1-byte type)
    Al,
    /// AX only (a 2-byte type): [`AbiRet::Ax`]
    Ax,
    /// AL, EAX[31:8] = the caller's EAX at entry (compared as a whole EAX)
    AlEntry,
    /// AX, EAX[31:16] = the caller's EAX at entry (compared as a whole EAX)
    AxEntry,
}

/// A parsed register-convention spec (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegConv {
    pub args: [Loc; MAX_ARGS],
    pub nargs: u8,
    pub ret: RetMode,
    /// caller-saved registers returned unchanged, bit per [`REG_NAMES`] index (EAX, ECX, EDX)
    pub keep: u8,
    /// the callee pops its stack arguments
    pub pop: bool,
}

const fn is_sep(c: u8) -> bool {
    c == b' ' || c == b',' || c == b'\t' || c == b'\n'
}

const fn eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn trim(s: &[u8]) -> &[u8] {
    let (mut a, mut b) = (0, s.len());
    while a < b && is_sep(s[a]) {
        a += 1;
    }
    while b > a && is_sep(s[b - 1]) {
        b -= 1;
    }
    s.split_at(b).0.split_at(a).1
}

impl RegConv {
    /// Parses a spec at compile time (an invalid spec is a const-evaluation panic naming the problem).
    /// The register lists of one clause are separated by spaces or commas.
    pub const fn parse(spec: &str) -> RegConv {
        let s = spec.as_bytes();
        let mut c = RegConv {
            args: [Loc::Stack; MAX_ARGS],
            nargs: 0,
            ret: RetMode::Auto,
            keep: 0,
            pop: false,
        };
        let mut seen_args = false;
        let mut start = 0;
        while start <= s.len() {
            let mut end = start;
            while end < s.len() && s[end] != b';' {
                end += 1;
            }
            let clause = trim(s.split_at(end).0.split_at(start).1);
            start = end + 1;
            if clause.is_empty() {
                continue;
            }
            // `name = value` or a bare flag
            let mut eqp = 0;
            while eqp < clause.len() && clause[eqp] != b'=' {
                eqp += 1;
            }
            let name = trim(clause.split_at(eqp).0);
            let value = if eqp < clause.len() { trim(clause.split_at(eqp + 1).1) } else { &[] as &[u8] };
            if eqp == clause.len() {
                if eq(name, b"pop") {
                    c.pop = true;
                    continue;
                }
                panic!("reg conv: unknown clause (expected args = ..., ret = ..., keep = ..., pop)");
            }
            if eq(name, b"args") {
                if seen_args {
                    panic!("reg conv: `args` given twice");
                }
                seen_args = true;
                let mut i = 0;
                while i < value.len() {
                    while i < value.len() && is_sep(value[i]) {
                        i += 1;
                    }
                    let w0 = i;
                    while i < value.len() && !is_sep(value[i]) {
                        i += 1;
                    }
                    if i == w0 {
                        break;
                    }
                    let w = value.split_at(i).0.split_at(w0).1;
                    let Some(l) = Loc::parse(w) else {
                        panic!("reg conv: an `args` location is not stack or a register name");
                    };
                    if c.nargs as usize == MAX_ARGS {
                        panic!("reg conv: too many arguments");
                    }
                    if let Some(r) = l.reg() {
                        let mut k = 0;
                        while k < c.nargs as usize {
                            if matches!(c.args[k].reg(), Some(x) if x == r) {
                                panic!("reg conv: two arguments in one register");
                            }
                            k += 1;
                        }
                    }
                    c.args[c.nargs as usize] = l;
                    c.nargs += 1;
                }
            } else if eq(name, b"ret") {
                c.ret = if eq(value, b"auto") {
                    RetMode::Auto
                } else if eq(value, b"al") {
                    RetMode::Al
                } else if eq(value, b"ax") {
                    RetMode::Ax
                } else if eq(value, b"al:entry") {
                    RetMode::AlEntry
                } else if eq(value, b"ax:entry") {
                    RetMode::AxEntry
                } else {
                    panic!("reg conv: ret must be auto, al, ax, al:entry or ax:entry")
                };
            } else if eq(name, b"keep") {
                let mut i = 0;
                while i < value.len() {
                    while i < value.len() && is_sep(value[i]) {
                        i += 1;
                    }
                    let w0 = i;
                    while i < value.len() && !is_sep(value[i]) {
                        i += 1;
                    }
                    if i == w0 {
                        break;
                    }
                    let w = value.split_at(i).0.split_at(w0).1;
                    let bit = if eq(w, b"eax") {
                        1
                    } else if eq(w, b"ecx") {
                        2
                    } else if eq(w, b"edx") {
                        4
                    } else {
                        panic!("reg conv: keep lists eax, ecx or edx (EBX/ESI/EDI/EBP are always kept)")
                    };
                    if c.keep & bit != 0 {
                        panic!("reg conv: a keep register twice");
                    }
                    c.keep |= bit;
                }
                if c.keep == 0 {
                    panic!("reg conv: keep without a register");
                }
            } else {
                panic!("reg conv: unknown clause (expected args = ..., ret = ..., keep = ..., pop)");
            }
        }
        if c.keep & 1 != 0 && !matches!(c.ret, RetMode::Auto) {
            panic!("reg conv: keep = eax conflicts with a register result");
        }
        c
    }

    /// The location of argument `i` (`Stack` past the declared list: an omitted `args` clause means
    /// every argument is on the stack).
    pub const fn loc(&self, i: usize) -> Loc {
        if i < self.nargs as usize { self.args[i] } else { Loc::Stack }
    }

    /// `keep` as `abi::PRESERVE_ECX | PRESERVE_EDX` (the `HookReg::preserve` mask the callers scan
    /// and the L1 runner read).
    pub const fn preserve_mask(&self) -> u8 {
        (if self.keep & 2 != 0 { crate::abi::PRESERVE_ECX } else { 0 })
            | (if self.keep & 4 != 0 { crate::abi::PRESERVE_EDX } else { 0 })
    }

    /// Register inputs as a bit per [`REG_NAMES`] index.
    pub const fn input_regs(&self) -> u8 {
        let mut m = 0;
        let mut i = 0;
        while i < self.nargs as usize {
            if let Some(r) = self.args[i].reg() {
                m |= 1 << r;
            }
            i += 1;
        }
        m
    }

    /// The register `this` is passed in when it is argument 0 (`None` for the stack).
    pub const fn this_reg(&self) -> Option<usize> {
        self.loc(0).reg()
    }

    /// Where the result goes, for the hook registration (`HookReg::ret`): `auto` = the type's.
    pub const fn abi_ret(&self, auto: AbiRet) -> AbiRet {
        match self.ret {
            RetMode::Auto => auto,
            RetMode::Al => AbiRet::Al,
            RetMode::Ax => AbiRet::Ax,
            // the upper bits are defined: compared as a whole EAX
            RetMode::AlEntry | RetMode::AxEntry => AbiRet::Eax,
        }
    }

    /// Bytes of the low part a `ret` mode writes (0 = the whole result).
    pub const fn ret_low_bytes(&self) -> usize {
        match self.ret {
            RetMode::AlEntry => 1,
            RetMode::AxEntry => 2,
            _ => 0,
        }
    }

    /// Structural equality (const), used by `#[hook]` to check its own parse against this one.
    pub const fn same(&self, args: &[u8], ret: u8, keep: u8, pop: bool) -> bool {
        if self.nargs as usize != args.len() || self.ret as u8 != ret || self.keep != keep || self.pop != pop {
            return false;
        }
        let mut i = 0;
        while i < args.len() {
            if self.args[i] as u8 != args[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Checks a hook's or call's argument sizes against its spec (const): 1-/2-byte types for
/// AL/AX-style locations, at most 4 bytes in a register.
pub const fn check_arg(conv: &RegConv, i: usize, size: usize) {
    let l = conv.loc(i);
    let w = l.width();
    if w == 1 && size != 1 {
        panic!("reg conv: an 8-bit register argument needs a 1-byte type");
    }
    if w == 2 && size != 2 {
        panic!("reg conv: a 16-bit register argument needs a 2-byte type");
    }
    // (a pointer is 8 bytes on a 64-bit host; the phase A entries exist on 32-bit x86 only)
    if w == 4 && size > if core::mem::size_of::<usize>() > 4 { 8 } else { 4 } {
        panic!("reg conv: a register argument is at most 4 bytes");
    }
}

/// Checks the result type against the `ret` mode (const).
pub const fn check_ret(conv: &RegConv, size: usize) {
    match conv.ret {
        RetMode::Al | RetMode::AlEntry if size != 1 => panic!("reg conv: ret = al needs a 1-byte result type"),
        RetMode::Ax | RetMode::AxEntry if size != 2 => panic!("reg conv: ret = ax needs a 2-byte result type"),
        _ => {}
    }
}

// ------------------------------------------------------------------ hook side -----------------------

/// What a register-convention hook shim hands back to its naked entry (layout used by the entry's
/// code: EAX 0, EDX 4, ST0 8 (80 bits), ST0 present 18). The entry initialises EAX/EDX with the
/// caller's values, so a result that does not cover them leaves them as they came in.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegOut {
    pub eax: u32,
    pub edx: u32,
    pub st0: [u8; 10],
    pub has_st0: u8,
    pub _pad: u8,
}

const _: () = assert!(core::mem::size_of::<RegOut>() == 20);

/// A hook result written into [`RegOut`]. `low` = 1/2: only AL/AX are written (the `:entry` modes).
pub trait RegReturn {
    /// Where the original returns it (`ret = auto`).
    const RET: AbiRet;
    fn put(self, out: &mut RegOut, low: usize);
}

fn put_eax(out: &mut RegOut, v: u32, low: usize) {
    out.eax = match low {
        1 => (out.eax & !0xFF) | (v & 0xFF),
        2 => (out.eax & !0xFFFF) | (v & 0xFFFF),
        _ => v,
    };
}

macro_rules! reg_return_int {
    ($($t:ty => $r:ident),*) => {$(
        impl RegReturn for $t {
            const RET: AbiRet = AbiRet::$r;
            #[inline(always)]
            fn put(self, out: &mut RegOut, low: usize) {
                put_eax(out, self as u32, low)
            }
        }
    )*};
}
reg_return_int!(u8 => Al, i8 => Al, u16 => Eax, i16 => Eax, u32 => Eax, i32 => Eax);

impl RegReturn for () {
    const RET: AbiRet = AbiRet::Void;
    #[inline(always)]
    fn put(self, _: &mut RegOut, _: usize) {}
}

impl RegReturn for bool {
    const RET: AbiRet = AbiRet::Al;
    #[inline(always)]
    fn put(self, out: &mut RegOut, low: usize) {
        put_eax(out, self as u32, low)
    }
}

impl RegReturn for crate::abi::Ax {
    const RET: AbiRet = AbiRet::Ax;
    #[inline(always)]
    fn put(self, out: &mut RegOut, low: usize) {
        put_eax(out, u32::from(self.0), low)
    }
}

impl RegReturn for u64 {
    const RET: AbiRet = AbiRet::EaxEdx;
    #[inline(always)]
    fn put(self, out: &mut RegOut, _: usize) {
        out.eax = self as u32;
        out.edx = (self >> 32) as u32;
    }
}

impl RegReturn for i64 {
    const RET: AbiRet = AbiRet::EaxEdx;
    #[inline(always)]
    fn put(self, out: &mut RegOut, low: usize) {
        (self as u64).put(out, low)
    }
}

impl<T> RegReturn for crate::mem::GPtr<T> {
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn put(self, out: &mut RegOut, _: usize) {
        out.eax = self.addr() as u32;
    }
}

impl<T> RegReturn for *mut T {
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn put(self, out: &mut RegOut, _: usize) {
        out.eax = self as usize as u32;
    }
}

impl<T> RegReturn for *const T {
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn put(self, out: &mut RegOut, _: usize) {
        out.eax = self as usize as u32;
    }
}

impl RegReturn for X87 {
    const RET: AbiRet = AbiRet::St0;
    #[inline(always)]
    fn put(self, out: &mut RegOut, _: usize) {
        out.st0 = self.to_ext();
        out.has_st0 = 1;
    }
}

impl RegReturn for f32 {
    const RET: AbiRet = AbiRet::St0;
    #[inline(always)]
    fn put(self, out: &mut RegOut, low: usize) {
        // `fld dword` of the value (a signalling NaN is quieted, as the original's load does)
        X87::from_f32(self).put(out, low)
    }
}

impl RegReturn for f64 {
    const RET: AbiRet = AbiRet::St0;
    #[inline(always)]
    fn put(self, out: &mut RegOut, low: usize) {
        X87::from_f64_exact(self).put(out, low)
    }
}

/// A register argument of a hook: the shim receives the whole register as a word and narrows it to
/// the argument's type (an 8-bit location's upper bits are the caller's garbage).
pub trait FromRegWord {
    fn from_reg_word(w: u32) -> Self;
}

macro_rules! from_reg_int {
    ($($t:ty),*) => {$(
        impl FromRegWord for $t {
            #[inline(always)]
            fn from_reg_word(w: u32) -> $t {
                w as $t
            }
        }
    )*};
}
from_reg_int!(u8, i8, u16, i16, u32, i32);

impl FromRegWord for f32 {
    #[inline(always)]
    fn from_reg_word(w: u32) -> f32 {
        f32::from_bits(w)
    }
}

impl FromRegWord for crate::mem::GBool {
    #[inline(always)]
    fn from_reg_word(w: u32) -> Self {
        crate::mem::GBool(w as u8)
    }
}

impl<T> FromRegWord for crate::mem::GPtr<T> {
    #[inline(always)]
    fn from_reg_word(w: u32) -> Self {
        crate::mem::GPtr::from_addr(w as usize)
    }
}

impl<T> FromRegWord for *mut T {
    #[inline(always)]
    fn from_reg_word(w: u32) -> Self {
        w as usize as *mut T
    }
}

impl<T> FromRegWord for *const T {
    #[inline(always)]
    fn from_reg_word(w: u32) -> Self {
        w as usize as *const T
    }
}

// ------------------------------------------------------------------ call side (orig!) ---------------

/// One call through [`call_regs`] (layout used by its code: target 0, EAX..EBP 4..32, stack word count
/// 32, stack words 36 (first = `[esp+4]` at entry), outputs EAX..EDI 100..124, ST0 124 (80 bits), "pop
/// ST0" flag 134, ESP delta 136).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegFrame {
    pub target: u32,
    /// EAX, ECX, EDX, EBX, ESI, EDI at the call (EBP is the frame pointer of [`call_regs`]; an `orig!`
    /// spec cannot pass an argument in it)
    pub regs: [u32; 7],
    pub nstack: u32,
    pub stack: [u32; MAX_STACK],
    /// EAX, ECX, EDX, EBX, ESI, EDI after the call
    pub out: [u32; 6],
    pub st0: [u8; 10],
    /// 1: the callee returns a value in ST0, stored (80 bits) and popped
    pub want_st0: u8,
    pub _pad: u8,
    /// ESP after the return minus ESP before the stack words were pushed (-4n: the caller pops;
    /// 0: the callee popped them)
    pub esp_delta: i32,
}

const _: () = assert!(core::mem::offset_of!(RegFrame, nstack) == 32);
const _: () = assert!(core::mem::offset_of!(RegFrame, esp_delta) == 136);
const _: () = assert!(core::mem::offset_of!(RegFrame, out) == 100);
const _: () = assert!(core::mem::offset_of!(RegFrame, st0) == 124);
const _: () = assert!(core::mem::offset_of!(RegFrame, want_st0) == 134);

impl RegFrame {
    pub const fn new(target: u32) -> RegFrame {
        RegFrame {
            target,
            regs: [0; 7],
            nstack: 0,
            stack: [0; MAX_STACK],
            out: [0; 6],
            st0: [0; 10],
            want_st0: 0,
            _pad: 0,
            esp_delta: 0,
        }
    }

    /// Places argument `i` (of type `T`) per the spec: its bytes in the register (zero-extended), or
    /// its words appended to the stack words.
    pub fn arg<T: Copy>(&mut self, conv: &RegConv, i: usize, v: T) {
        let n = core::mem::size_of::<T>();
        let mut b = [0u8; 4 * MAX_STACK];
        assert!(n <= b.len(), "reg conv: argument too large");
        // SAFETY: copies the n bytes of a Copy value into a local buffer of at least n bytes.
        unsafe { core::ptr::copy_nonoverlapping((&raw const v).cast::<u8>(), b.as_mut_ptr(), n) };
        let words = n.div_ceil(4).max(1);
        match conv.loc(i).reg() {
            Some(r) => {
                assert!(r != 6, "reg conv: orig! cannot pass an argument in EBP");
                self.regs[r] = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            }
            None => {
                for k in 0..words {
                    let s = self.nstack as usize;
                    assert!(s < MAX_STACK, "reg conv: more than {MAX_STACK} stack words");
                    self.stack[s] = u32::from_le_bytes([b[4 * k], b[4 * k + 1], b[4 * k + 2], b[4 * k + 3]]);
                    self.nstack += 1;
                }
            }
        }
    }
}

/// An `orig!` result read from a [`RegFrame`] after the call.
pub trait FromRegFrame {
    /// The callee leaves the result in ST0 (stored and popped by [`call_regs`]).
    const ST0: bool = false;
    fn from_frame(f: &RegFrame) -> Self;
}

impl FromRegFrame for () {
    fn from_frame(_: &RegFrame) {}
}

macro_rules! from_frame_int {
    ($($t:ty),*) => {$(
        impl FromRegFrame for $t {
            #[inline(always)]
            fn from_frame(f: &RegFrame) -> $t {
                f.out[0] as $t
            }
        }
    )*};
}
from_frame_int!(u8, i8, u16, i16, u32, i32);

impl FromRegFrame for bool {
    fn from_frame(f: &RegFrame) -> bool {
        f.out[0] as u8 != 0
    }
}

impl FromRegFrame for u64 {
    fn from_frame(f: &RegFrame) -> u64 {
        u64::from(f.out[0]) | (u64::from(f.out[2]) << 32)
    }
}

impl<T> FromRegFrame for crate::mem::GPtr<T> {
    fn from_frame(f: &RegFrame) -> Self {
        crate::mem::GPtr::from_addr(f.out[0] as usize)
    }
}

impl FromRegFrame for X87 {
    const ST0: bool = true;
    fn from_frame(f: &RegFrame) -> X87 {
        X87::from_ext(f.st0)
    }
}

impl FromRegFrame for f32 {
    const ST0: bool = true;
    /// What a caller's `fstp dword` stores.
    fn from_frame(f: &RegFrame) -> f32 {
        X87::from_ext(f.st0).to_f32()
    }
}

/// Calls `f.target` with the frame's registers and stack words, and stores EAX..EDI (and, with
/// `want_st0`, ST0 as 80 bits, popped) back into the frame. EBX/ESI/EDI/EBP of the Rust caller are
/// saved around the call; the callee's stack cleanup (if any) does not matter (ESP is restored from
/// EBP, which every MSVC function preserves).
///
/// # Safety
/// `f.target` must be a function that reads at most the given registers and `nstack` stack words,
/// preserves EBP, and leaves the x87 stack empty (or holding exactly its ST0 result when `want_st0`).
#[cfg(target_arch = "x86")]
pub unsafe fn call_regs(f: &mut RegFrame) {
    // SAFETY: per the contract; every caller-saved register is declared clobbered, the callee-saved
    // ones are pushed and popped by the sequence itself, ESP is restored from EBP.
    unsafe {
        core::arch::asm!(
            "push ebp",
            "push ebx",
            "push esi",
            "push edi",
            "mov ebp, esp",
            "push eax",
            "push dword ptr [eax]",
            "mov ecx, dword ptr [eax + 32]",
            "2:",
            "test ecx, ecx",
            "jz 3f",
            "push dword ptr [eax + 32 + ecx*4]",
            "dec ecx",
            "jmp 2b",
            "3:",
            "mov ebx, dword ptr [eax + 16]",
            "mov esi, dword ptr [eax + 20]",
            "mov edi, dword ptr [eax + 24]",
            "mov ecx, dword ptr [eax + 8]",
            "mov edx, dword ptr [eax + 12]",
            "mov eax, dword ptr [eax + 4]",
            "call dword ptr [ebp - 8]",
            "xchg eax, dword ptr [ebp - 4]",
            "mov dword ptr [eax + 104], ecx",
            "mov dword ptr [eax + 136], esp",
            "lea ecx, [ebp - 8]",
            "sub dword ptr [eax + 136], ecx",
            "mov dword ptr [eax + 108], edx",
            "mov dword ptr [eax + 112], ebx",
            "mov dword ptr [eax + 116], esi",
            "mov dword ptr [eax + 120], edi",
            "mov ecx, dword ptr [ebp - 4]",
            "mov dword ptr [eax + 100], ecx",
            "cmp byte ptr [eax + 134], 0",
            "je 4f",
            "fstp tbyte ptr [eax + 124]",
            "4:",
            "mov esp, ebp",
            "pop edi",
            "pop esi",
            "pop ebx",
            "pop ebp",
            inout("eax") f as *mut RegFrame => _,
            clobber_abi("C"),
        );
    }
}

#[cfg(test)]
#[path = "regconv_tests.rs"]
mod hook_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_forms() {
        const A: RegConv = RegConv::parse("args = ebx esi");
        assert_eq!((A.nargs, A.args[0], A.args[1], A.ret, A.keep, A.pop), (2, Loc::Ebx, Loc::Esi, RetMode::Auto, 0, false));
        assert_eq!(A.input_regs(), 0b11000);
        assert_eq!(A.this_reg(), Some(3));
        const B: RegConv = RegConv::parse(" args = al, stack ,stack ; ret = al:entry; keep = ecx edx ; pop ");
        assert_eq!((B.nargs, B.args[0], B.args[2], B.ret, B.keep, B.pop), (3, Loc::Al, Loc::Stack, RetMode::AlEntry, 6, true));
        assert_eq!(B.preserve_mask(), crate::abi::PRESERVE_ECX | crate::abi::PRESERVE_EDX);
        assert_eq!(B.abi_ret(AbiRet::Al), AbiRet::Eax);
        assert_eq!(B.ret_low_bytes(), 1);
        const C: RegConv = RegConv::parse("");
        assert_eq!((C.nargs, C.loc(5)), (0, Loc::Stack));
        assert_eq!(C.this_reg(), None);
        const D: RegConv = RegConv::parse("args = ecx edx di stack; ret = ax; keep = edx");
        assert_eq!((D.args[2], D.args[2].width(), D.args[2].reg()), (Loc::Di, 2, Some(5)));
        assert!(!D.same(&[Loc::Ecx as u8, Loc::Edx as u8, Loc::Edi as u8, 0], RetMode::Ax as u8, 4, false));
        assert!(D.same(&[Loc::Ecx as u8, Loc::Edx as u8, Loc::Di as u8, 0], RetMode::Ax as u8, 4, false));
    }

    #[test]
    fn bad_specs_panic() {
        for s in [
            "args = eax ax",
            "args = xmm0",
            "ret = st1",
            "keep = ebx",
            "keep = eax; ret = al",
            "args = eax; args = ecx",
            "pops",
        ] {
            assert!(std::panic::catch_unwind(|| RegConv::parse(s)).is_err(), "{s}");
        }
    }

    #[test]
    fn out_and_frame_conversions() {
        let mut o = RegOut { eax: 0x1234_5678, edx: 9, ..Default::default() };
        0xABu8.put(&mut o, 1);
        assert_eq!(o.eax, 0x1234_56AB);
        0xBEEFu16.put(&mut o, 2);
        assert_eq!(o.eax, 0x1234_BEEF);
        7u16.put(&mut o, 0);
        assert_eq!(o.eax, 7);
        0x1_0000_0002u64.put(&mut o, 0);
        assert_eq!((o.eax, o.edx), (2, 1));
        let x = X87::from_f32(1.5);
        x.put(&mut o, 0);
        assert_eq!((o.st0, o.has_st0), (x.to_ext(), 1));
        let mut f = RegFrame::new(0x401000);
        let c = RegConv::parse("args = al stack esi stack");
        f.arg(&c, 0, 0x41u8);
        f.arg(&c, 1, 0x1122_3344u32);
        f.arg(&c, 2, 0x5566u16);
        f.arg(&c, 3, 2.0f64);
        assert_eq!(f.regs[0], 0x41);
        assert_eq!(f.regs[4], 0x5566);
        assert_eq!(f.nstack, 3);
        assert_eq!(&f.stack[..3], &[0x1122_3344, 0, 0x4000_0000]);
        f.out[0] = 0xFFFF_FF80;
        assert_eq!(<u8 as FromRegFrame>::from_frame(&f), 0x80);
        assert_eq!(<i16 as FromRegFrame>::from_frame(&f), -128);
    }
}
