//! Backend-independent description of one call into original code and of what it produced.
//!
//! Both backends (Unicorn, [`crate::emu`]; real CPU under Wine, [`crate::native`]) take a [`Call`] and
//! return an [`Outcome`] decoded from the same 128-byte context block (`native/thunk.S` documents the
//! layout), captured by the same instruction sequence (`native/exit_stub.S`).

/// Fixed addresses shared by both backends.
pub const SCRATCH: u32 = 0x1000_0000;
/// Bytes of scratch memory per call (initialised from [`Call::scratch`], read back into the outcome and
/// compared whole). 0x500 holds the largest MATH-2 layout (a CMatrixLinkList with eight links).
pub const SCRATCH_LEN: usize = 0x500;
/// Scratch memory mapped at [`SCRATCH`] (both backends). The bytes from [`SCRATCH_LEN`] up to
/// [`SCRATCH_MAX`] are not reset between calls and not compared.
pub const SCRATCH_MAX: usize = 0x8000;
/// A fake thread environment block (TIB) for FS-relative accesses (SEH frames): FS base under Unicorn.
/// Its first dword (the SEH chain head) is reset to [`TIB_SEH_END`] before every call; a function that
/// links an SEH frame must unlink it again (checked by the runner through `Outcome::fs0`).
pub const TIB: u32 = 0x1000_8000;
pub const TIB_SEH_END: u32 = 0xFFFF_FFFF;
/// Dword pattern the stack below the arguments is filled with before every call (both backends), so
/// reads of uninitialised stack slots are deterministic.
pub const STACK_FILL: u32 = 0x5A5A_5A5A;
/// Bytes of stack filled with [`STACK_FILL`] below the arguments.
pub const STACK_FILL_LEN: u32 = 0x800;
/// Code blob for x87 fidelity probes.
pub const PROBE: u32 = 0x1010_0000;
pub const PROBE_LEN: usize = 0x1_0000;
/// Context block bytes (128 until HARNESS-5 added the entry registers at 128..152).
pub const CTX_LEN: usize = 160;

/// The game's runtime FPU state, measured in-game by INF-3: x87 CW 0x007F (24-bit precision, round to
/// nearest, all exceptions masked) and MXCSR 0x9FA0 (FTZ on, all exceptions masked, PE flag set).
pub const GAME_CW: u16 = 0x007F;
pub const GAME_MXCSR: u32 = 0x9FA0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    /// args on the stack, caller cleans
    Cdecl,
    /// args on the stack, callee cleans
    Stdcall,
    /// `this` in ECX, args on the stack, callee cleans
    Thiscall,
    /// first two 32-bit integer/pointer args in ECX, EDX, rest on the stack, callee cleans
    Fastcall,
}

#[derive(Clone, Copy, Debug)]
pub enum Arg {
    I32(u32),
    F32(f32),
    F64(f64),
    /// a pointer, normally into [`SCRATCH`]
    Ptr(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fpu {
    pub cw: u16,
    pub mxcsr: u32,
}

impl Default for Fpu {
    fn default() -> Self {
        Fpu {
            cw: GAME_CW,
            mxcsr: GAME_MXCSR,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Call {
    pub addr: u32,
    pub conv: Conv,
    pub ecx: u32,
    pub edx: u32,
    /// Entry registers of register-convention functions (HARNESS-5, `sa_core::regconv`): EAX
    /// (`None`: the call target, as before), EBX, ESI, EDI, EBP (0 by default).
    pub eax: Option<u32>,
    pub ebx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    /// dwords pushed, first element at the lowest address ([esp+4] at entry)
    pub stack: Vec<u32>,
    pub scratch: [u8; SCRATCH_LEN],
    pub fpu: Fpu,
}

impl Call {
    /// Lays out `args` for `conv`. For `Thiscall` the first arg is `this`.
    pub fn new(addr: u32, conv: Conv, args: &[Arg]) -> Call {
        let mut c = Call {
            addr,
            conv,
            ecx: 0,
            edx: 0,
            eax: None,
            ebx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
            stack: Vec::new(),
            scratch: [0; SCRATCH_LEN],
            fpu: Fpu::default(),
        };
        let mut regs: &[_] = match conv {
            Conv::Thiscall => &[0usize][..],
            Conv::Fastcall => &[0, 1][..],
            _ => &[][..],
        };
        for a in args {
            let int = match a {
                Arg::I32(v) | Arg::Ptr(v) => Some(*v),
                _ => None,
            };
            if let (Some(v), Some((&r, rest))) = (int, regs.split_first()) {
                if r == 0 {
                    c.ecx = v
                } else {
                    c.edx = v
                }
                regs = rest;
                continue;
            }
            match a {
                Arg::I32(v) | Arg::Ptr(v) => c.stack.push(*v),
                Arg::F32(f) => c.stack.push(f.to_bits()),
                Arg::F64(f) => {
                    let b = f.to_bits();
                    c.stack.push(b as u32);
                    c.stack.push((b >> 32) as u32);
                }
            }
        }
        c
    }

    /// ESP after return minus ESP before the arguments were pushed, for a correct callee.
    pub fn expected_esp_delta(&self) -> i32 {
        match self.conv {
            Conv::Cdecl => -4 * self.stack.len() as i32,
            _ => 0,
        }
    }

    /// The 128-byte context block (inputs part) used by both backends.
    pub fn ctx(&self) -> [u8; CTX_LEN] {
        assert!(self.stack.len() <= 16, "at most 16 stack dwords");
        let mut b = [0u8; CTX_LEN];
        let mut put = |o: usize, v: u32| b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(0, self.addr);
        put(4, self.ecx);
        put(8, self.edx);
        put(12, self.stack.len() as u32);
        for (i, v) in self.stack.iter().enumerate() {
            put(16 + 4 * i, *v);
        }
        put(80, self.fpu.cw as u32);
        put(84, self.fpu.mxcsr);
        put(128, self.eax.unwrap_or(0));
        put(132, self.eax.is_some() as u32);
        put(136, self.ebx);
        put(140, self.esi);
        put(144, self.edi);
        put(148, self.ebp);
        b
    }

    /// The register picture of a `sa_core::regconv` spec: argument `k` goes to its register (the
    /// low byte/word of an AL/AX-style location, over `fill` in the other bytes, which the original
    /// must ignore) or onto the stack. `args` are the argument words in order (one word per stack
    /// argument word).
    pub fn regconv(addr: u32, conv: &sa_core::regconv::RegConv, args: &[u32], fill: u32) -> Call {
        let pop = conv.pop;
        let mut c = Call::new(addr, if pop { Conv::Stdcall } else { Conv::Cdecl }, &[]);
        for (k, &v) in args.iter().enumerate() {
            let l = conv.loc(k);
            let Some(r) = l.reg() else {
                c.stack.push(v);
                continue;
            };
            let m: u32 = match l.width() {
                1 => 0xFF,
                2 => 0xFFFF,
                _ => u32::MAX,
            };
            let v = (fill & !m) | (v & m);
            match r {
                0 => c.eax = Some(v),
                1 => c.ecx = v,
                2 => c.edx = v,
                3 => c.ebx = v,
                4 => c.esi = v,
                5 => c.edi = v,
                _ => c.ebp = v,
            }
        }
        c
    }
}

/// What one call produced (identical fields for both backends).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub eax: u32,
    /// ECX after return (context offset 12, the input `nstack` field, overwritten by both backends)
    pub ecx: u32,
    pub edx: u32,
    pub esp_delta: i32,
    /// x87 status word right after return (TOP, condition codes, exception flags)
    pub fpsw: u16,
    /// full x87 tag word right after return (2 bits per physical register, 3 = empty)
    pub fptag: u16,
    /// ST0 as stored by `fstp tbyte`, when the x87 stack was not empty
    pub st0: Option<[u8; 10]>,
    /// ST0 as stored by `fst dword`
    pub st0_f32: Option<u32>,
    pub mxcsr: u32,
    pub scratch: [u8; SCRATCH_LEN],
}

fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

impl Outcome {
    /// Decodes the outputs of a context block plus the scratch bytes after the call.
    pub fn from_ctx(b: &[u8], scratch: &[u8]) -> Outcome {
        let has = u32_at(b, 124) != 0;
        let mut st0 = [0u8; 10];
        st0.copy_from_slice(&b[104..114]);
        let mut s = [0u8; SCRATCH_LEN];
        s.copy_from_slice(&scratch[..SCRATCH_LEN]);
        Outcome {
            eax: u32_at(b, 88),
            ecx: u32_at(b, 12),
            edx: u32_at(b, 92),
            esp_delta: u32_at(b, 96) as i32,
            fpsw: u16::from_le_bytes([b[100], b[101]]),
            fptag: u16::from_le_bytes([b[102], b[103]]),
            st0: has.then_some(st0),
            st0_f32: has.then(|| u32_at(b, 116)),
            mxcsr: u32_at(b, 120),
            scratch: s,
        }
    }

    /// Fields that differ between two outcomes (Unicorn vs CPU). `eax_mask` selects the EAX bits that
    /// carry a result. x87 status word: C0/C2/C3 = 0x4500, C1 = 0x0200, TOP = 0x3800, exception and
    /// summary flags = 0x80FF.
    pub fn diff(&self, b: &Outcome, eax_mask: u32) -> Vec<&'static str> {
        let a = self;
        let mut d = Vec::new();
        let mut f = |c: bool, n| {
            if c {
                d.push(n)
            }
        };
        f(a.st0_f32 != b.st0_f32, "st0-f32");
        f(a.st0 != b.st0, "st0-ext");
        f(a.fpsw & 0x4500 != b.fpsw & 0x4500, "C0/C2/C3");
        f(a.fpsw & 0x0200 != b.fpsw & 0x0200, "C1");
        f(a.fpsw & 0x3800 != b.fpsw & 0x3800, "TOP");
        f(a.fpsw & 0x80FF != b.fpsw & 0x80FF, "exc-flags");
        f(a.fptag != b.fptag, "tag");
        f(a.mxcsr != b.mxcsr, "mxcsr");
        f(a.scratch != b.scratch, "memory");
        f(a.esp_delta != b.esp_delta, "esp");
        f(a.eax & eax_mask != b.eax & eax_mask, "eax");
        d
    }

    /// Number of non-empty x87 registers.
    pub fn depth(&self) -> u32 {
        (0..8).filter(|i| (self.fptag >> (2 * i)) & 3 != 3).count() as u32
    }
}

/// Why a call did not produce an [`Outcome`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallError {
    /// Control reached code outside the function and its allow-list (a call, jump or bad return).
    Disallowed { target: u32, from: u32, what: String },
    /// The instruction budget ran out (a hang, or a budget that is too small).
    Budget { budget: u64 },
    /// Unmapped/protected memory access.
    Fault { kind: String, addr: u32, pc: u32 },
    /// A software interrupt or exception (int3, int n, divide error, ...).
    Interrupt { number: u32, pc: u32 },
    /// The function wrote outside the stack and scratch memory (the image is restored afterwards).
    GlobalWrite { addr: u32, size: u32, pc: u32 },
    /// The function read a writable image section (mutable global state: not an L1 function).
    GlobalRead { addr: u32, size: u32, pc: u32 },
    /// Backend failure (Unicorn error, Wine failure).
    Backend(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::Disallowed { target, from, what } => {
                write!(
                    f,
                    "control left the allow-list: block at {target:#x} ({what}), reached from block {from:#x}"
                )
            }
            CallError::Budget { budget } => write!(f, "instruction budget of {budget} exhausted"),
            CallError::Fault { kind, addr, pc } => write!(f, "memory fault {kind} at {addr:#x} (pc {pc:#x})"),
            CallError::Interrupt { number, pc } => write!(f, "interrupt {number} at pc {pc:#x}"),
            CallError::GlobalWrite { addr, size, pc } => {
                write!(f, "write of {size} bytes to image memory {addr:#x} (pc {pc:#x})")
            }
            CallError::GlobalRead { addr, size, pc } => {
                write!(
                    f,
                    "read of {size} bytes from writable image memory {addr:#x} (pc {pc:#x})"
                )
            }
            CallError::Backend(s) => write!(f, "backend error: {s}"),
        }
    }
}

pub type CallResult = Result<Outcome, CallError>;

/// Decodes an 80-bit extended value to f64 (for display only; NaN payloads and low bits are lost).
pub fn ext_to_f64(b: &[u8; 10]) -> f64 {
    let m = u64::from_le_bytes(b[..8].try_into().unwrap());
    let se = u16::from_le_bytes([b[8], b[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = (se & 0x7FFF) as i32;
    if e == 0x7FFF {
        return if m << 1 == 0 {
            sign * f64::INFINITY
        } else {
            f64::NAN
        };
    }
    sign * (m as f64) * 2f64.powi(e - 16383 - 63)
}

pub fn ext_hex(b: &[u8; 10]) -> String {
    let m = u64::from_le_bytes(b[..8].try_into().unwrap());
    let se = u16::from_le_bytes([b[8], b[9]]);
    format!("{se:04x}:{m:016x}")
}
