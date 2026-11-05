//! Calling conventions, hook registration records and the hook shims' runtime support (ADR-001 §4).
#![allow(unsafe_code)]

/// Calling convention of an original function (MSVC 7.1, 32-bit x86).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    /// Arguments on the stack, caller cleans (`extern "C"`).
    Cdecl,
    /// Arguments on the stack, callee cleans.
    Stdcall,
    /// `this` in ECX, arguments on the stack, callee cleans.
    Thiscall,
    /// First two 32-bit integer arguments in ECX/EDX, rest on the stack, callee cleans.
    Fastcall,
}

/// One ported function, as `#[hook]` registers it. `sa-hook` turns every record of
/// [`crate::hooks::ALL`] into a hook-table entry (patched at attach, toggled Rust/original at frame
/// boundaries).
#[derive(Clone, Copy)]
pub struct HookReg {
    /// Address in gta_sa.exe 1.0 US (for a `.HOODLUM` stub, the stub address).
    pub addr: u32,
    /// Catalogue name (`catalog/functions.csv` `owner::name`), the control channel's key.
    pub name: &'static str,
    pub conv: Conv,
    /// Bytes that must be found at `addr` before patching (at least the 5 replaced by the entry jmp).
    pub expect: &'static [u8],
    /// Address of the shim with the original convention (0 on targets other than 32-bit x86).
    pub entry: fn() -> usize,
    /// Calls that came in through the shim (i.e. from original code while the hook is on Rust).
    pub calls: fn() -> u32,
    /// 32-bit stack argument words (after `this` in ECX for thiscall, ECX/EDX for fastcall).
    pub stack_words: u8,
    /// Where the result is returned.
    pub ret: AbiRet,
    /// For thiscall: the `this` type (catalog/layouts.csv name) and its size, when declared.
    pub this_type: Option<&'static str>,
    pub this_size: u32,
}

/// Where a function returns its result (MSVC 32-bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiRet {
    Void,
    /// `bool`/8-bit result in AL
    Al,
    /// 32-bit integer or pointer in EAX
    Eax,
    /// 64-bit integer in EDX:EAX
    EaxEdx,
    /// float in ST0
    St0,
}

/// 32-bit stack words an argument of type `T` occupies.
pub const fn stack_words<T>() -> u8 {
    core::mem::size_of::<T>().div_ceil(4) as u8
}

impl HookReg {
    /// A 5-byte `jmp rel32` entry: one of the 501 stubs into `.HOODLUM` (CLAUDE.md), hooked by
    /// retargeting the stub instead of relocating code.
    pub const fn is_hoodlum_stub(&self) -> bool {
        self.expect.len() == 5 && self.expect[0] == 0xE9
    }
}

/// Converts a port's return value into what the original returns in registers.
///
/// Identity for integers, pointers and `f32`; [`sa_math::x87::X87`] goes back as `f64`, which the
/// i686 C ABI returns in ST0 through `fld qword` — exact for every value the 24-bit-significand,
/// extended-exponent model holds, so callers that keep computing on ST0 see the original's value.
pub trait IntoAbi {
    type Abi;
    /// Where the original returns it.
    const RET: AbiRet;
    fn into_abi(self) -> Self::Abi;
}

macro_rules! identity_abi {
    ($($t:ty => $r:ident),*) => {$(
        impl IntoAbi for $t {
            type Abi = $t;
            const RET: AbiRet = AbiRet::$r;
            #[inline(always)]
            fn into_abi(self) -> $t {
                self
            }
        }
    )*};
}
identity_abi!(() => Void, bool => Al, u8 => Al, i8 => Al, u16 => Eax, i16 => Eax, u32 => Eax, i32 => Eax,
    u64 => EaxEdx, i64 => EaxEdx, f32 => St0, f64 => St0);

impl<T> IntoAbi for *mut T {
    type Abi = *mut T;
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn into_abi(self) -> *mut T {
        self
    }
}

impl<T> IntoAbi for *const T {
    type Abi = *const T;
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn into_abi(self) -> *const T {
        self
    }
}

impl<T> IntoAbi for crate::mem::GPtr<T> {
    type Abi = crate::mem::GPtr<T>;
    const RET: AbiRet = AbiRet::Eax;
    #[inline(always)]
    fn into_abi(self) -> Self {
        self
    }
}

impl IntoAbi for sa_math::x87::X87 {
    type Abi = f64;
    const RET: AbiRet = AbiRet::St0;
    #[inline(always)]
    fn into_abi(self) -> f64 {
        self.to_f64()
    }
}

/// MXCSR for the duration of a hook shim (ADR-001 §5).
///
/// The game runs with MXCSR 0x9FA0 (flush-to-zero on, measured by INF-3). The original's code is x87,
/// which MXCSR does not affect, but Rust compiles f32/f64 arithmetic (including the `X87` model) to
/// SSE, where FTZ would flush subnormal results. The guard clears FTZ and DAZ on entry and restores
/// the caller's exact MXCSR (sticky flags included) on exit, so a Rust port leaves MXCSR as the x87
/// original would: untouched. No-op off x86.
pub struct FpuGuard {
    #[cfg(target_arch = "x86")]
    saved: u32,
}

/// MXCSR flush-to-zero (bit 15) and denormals-are-zero (bit 6).
pub const MXCSR_FTZ_DAZ: u32 = 0x8040;

impl FpuGuard {
    #[inline(always)]
    pub fn enter() -> FpuGuard {
        #[cfg(target_arch = "x86")]
        {
            let mut saved: u32 = 0;
            // SAFETY: stores/loads MXCSR from/to locals; only FTZ/DAZ change.
            unsafe {
                core::arch::asm!("stmxcsr [{0}]", in(reg) &mut saved, options(nostack, preserves_flags));
                let clean = saved & !MXCSR_FTZ_DAZ;
                core::arch::asm!("ldmxcsr [{0}]", in(reg) &clean, options(nostack, preserves_flags));
            }
            FpuGuard { saved }
        }
        #[cfg(not(target_arch = "x86"))]
        FpuGuard {}
    }
}

impl Drop for FpuGuard {
    #[inline(always)]
    fn drop(&mut self) {
        #[cfg(target_arch = "x86")]
        // SAFETY: restores the MXCSR value saved on entry.
        unsafe {
            core::arch::asm!("ldmxcsr [{0}]", in(reg) &self.saved, options(nostack, preserves_flags));
        }
    }
}
