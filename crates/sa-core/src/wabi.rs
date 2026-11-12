//! Phases B/C on wasm32: the one signature every address-registry entry has.
//!
//! On 32-bit x86 (phase B native) a registry entry has the original calling convention, and
//! `orig!`'s dispatch calls it with the declaration's signature: a declaration and the port may
//! disagree on a type (`u8` against `u32`, `f32` against `X87`, a pointer against an integer) and the
//! x86 stack and registers still carry the value as the original does. wasm32 checks every indirect
//! call against the callee's exact signature (a mismatch traps), so there every entry is
//! type-erased: `extern "C" fn(callee, args, ret)`.
//!
//! * `args` holds the arguments as the original's stack words: each argument's bytes, rounded up
//!   to a multiple of 4 (an `f64` takes two words, a struct passed by value its size), `this` /
//!   fastcall register arguments first, in declaration order. A variadic CRT entry (`sprintf`)
//!   reads its words exactly as the x86 entry reads the caller's stack.
//! * `ret` is EAX:EDX (and memory for a returned struct) plus ST0: a callee returning `f32` / `f64`
//!   / `X87` writes ST0, anything else its bytes; the caller reads ST0 when it declares a float
//!   type, else the bytes, as the original's caller reads ST0 or EAX:EDX.
//!
//! `callee` is the original address the caller dispatched to (stubs use it to find their value).

#![allow(unsafe_code)]

use core::mem::{size_of, ManuallyDrop};

/// Words of the argument buffer (512 bytes; the largest declaration passes far fewer).
pub const ARG_WORDS: usize = 128;
/// Bytes of a returned value (EAX:EDX or a returned struct).
pub const RET_BYTES: usize = 256;

/// The type-erased entry signature.
pub type Erased = extern "C" fn(callee: u32, args: *const u32, ret: *mut Ret);

/// A call's return slots.
#[repr(C, align(8))]
pub struct Ret {
    /// ST0 as the f64 the callee returned (f32 widened exactly).
    pub st0: f64,
    /// EAX:EDX, or a returned struct's bytes.
    pub bytes: [u8; RET_BYTES],
}

impl Ret {
    pub const fn new() -> Ret {
        Ret {
            st0: 0.0,
            bytes: [0; RET_BYTES],
        }
    }
}

impl Default for Ret {
    fn default() -> Self {
        Ret::new()
    }
}

/// The caller's argument words.
#[repr(C, align(8))]
pub struct Args {
    words: [u32; ARG_WORDS],
    len: usize,
}

impl Default for Args {
    fn default() -> Self {
        Args::new()
    }
}

impl Args {
    pub const fn new() -> Args {
        Args {
            words: [0; ARG_WORDS],
            len: 0,
        }
    }

    /// Appends `v`'s bytes, rounded up to whole words.
    pub fn push<T>(&mut self, v: T) {
        let n = size_of::<T>();
        let w = n.div_ceil(4);
        assert!(self.len + w <= ARG_WORDS, "wabi: too many argument words");
        let v = ManuallyDrop::new(v);
        // SAFETY: `n` bytes of `v` into the zeroed word buffer, which has room (checked).
        unsafe {
            core::ptr::copy_nonoverlapping(
                (&raw const v).cast::<u8>(),
                self.words.as_mut_ptr().add(self.len).cast::<u8>(),
                n,
            );
        }
        self.len += w;
    }

    pub fn as_ptr(&self) -> *const u32 {
        self.words.as_ptr()
    }
}

/// The callee's view of the argument words.
pub struct Reader {
    p: *const u32,
    off: usize,
}

impl Reader {
    /// # Safety
    /// `p` points at an [`Args`] buffer of [`ARG_WORDS`] words.
    pub unsafe fn new(p: *const u32) -> Reader {
        Reader { p, off: 0 }
    }

    /// The next argument, of type `T`.
    #[allow(clippy::should_implement_trait)]
    pub fn take<T>(&mut self) -> T {
        let w = size_of::<T>().div_ceil(4);
        assert!(self.off + w <= ARG_WORDS, "wabi: argument words overrun");
        // SAFETY: inside the buffer (checked); the caller wrote this argument's bytes there (a
        // declaration with fewer bytes leaves zeros, as a narrower x86 push leaves the slot's rest).
        let v = unsafe { self.p.add(self.off).cast::<T>().read_unaligned() };
        self.off += w;
        v
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Float {
    No,
    F32,
    F64,
    X87,
}

fn float_kind<T>() -> Float {
    let n = core::any::type_name::<T>();
    match n {
        "f32" => Float::F32,
        "f64" => Float::F64,
        _ if n.ends_with("x87::X87") => Float::X87,
        _ => Float::No,
    }
}

/// Writes a callee's return value into `out`.
pub fn put_ret<R>(r: R, out: &mut Ret) {
    let r = ManuallyDrop::new(r);
    let p = (&raw const r).cast::<R>();
    // SAFETY: each arm reads `R` as the type its name says it is.
    unsafe {
        match float_kind::<R>() {
            Float::F32 => out.st0 = f64::from(p.cast::<f32>().read()),
            Float::F64 => out.st0 = p.cast::<f64>().read(),
            Float::X87 => out.st0 = p.cast::<sa_math::x87::X87>().read().to_f64(),
            Float::No => {
                let n = size_of::<R>();
                assert!(n <= RET_BYTES, "wabi: return value too large");
                core::ptr::copy_nonoverlapping(p.cast::<u8>(), out.bytes.as_mut_ptr(), n);
            }
        }
    }
}

/// Reads the caller's declared return type from `ret`.
pub fn get_ret<R>(ret: &Ret) -> R {
    let mut v = core::mem::MaybeUninit::<R>::zeroed();
    let p = v.as_mut_ptr();
    // SAFETY: each arm writes `R` as the type its name says it is; otherwise the bytes the callee
    // wrote (zeros beyond them), as the original's caller reads EAX:EDX.
    unsafe {
        match float_kind::<R>() {
            Float::F32 => p.cast::<f32>().write(ret.st0 as f32),
            Float::F64 => p.cast::<f64>().write(ret.st0),
            Float::X87 => p.cast::<sa_math::x87::X87>().write(sa_math::x87::X87::from_ext(ext_from_f64(ret.st0))),
            Float::No => {
                let n = size_of::<R>();
                assert!(n <= RET_BYTES, "wabi: return value too large");
                core::ptr::copy_nonoverlapping(ret.bytes.as_ptr(), p.cast::<u8>(), n);
            }
        }
        v.assume_init()
    }
}

/// Dispatches so far, and an optional sampler `fn(callee, count)` called every 2^20 dispatches (the web build logs
/// where a long main-loop iteration spends its time: it cannot be interrupted from outside).
pub static CALLS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static SAMPLER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Calls the erased entry `entry` registered for `callee`.
pub fn call<R>(entry: usize, callee: usize, args: &Args) -> R {
    use core::sync::atomic::Ordering;
    let n = CALLS.fetch_add(1, Ordering::Relaxed);
    if n & 0xF_FFFF == 0 {
        let s = SAMPLER.load(Ordering::Relaxed);
        if s != 0 {
            // SAFETY: stored from a `fn(usize, u32)` by the embedder.
            let f = unsafe { core::mem::transmute::<usize, fn(usize, u32)>(s) };
            f(callee, n);
        }
    }
    // SAFETY: registry entries on wasm32 are `Erased` function pointers (table indices).
    let f = unsafe { core::mem::transmute::<usize, Erased>(entry) };
    let mut ret = Ret::new();
    f(callee as u32, args.as_ptr(), &raw mut ret);
    get_ret(&ret)
}

/// The 80-bit extended encoding of `v` (exact: every f64 is representable).
pub fn ext_from_f64(v: f64) -> [u8; 10] {
    let b = v.to_bits();
    let sign = ((b >> 63) as u16) << 15;
    let exp = ((b >> 52) & 0x7FF) as i32;
    let frac = b & ((1u64 << 52) - 1);
    let (e, m): (u16, u64) = if exp == 0 {
        if frac == 0 {
            (0, 0)
        } else {
            // subnormal f64 (frac * 2^-1074): normalise the top bit to bit 63
            let s = frac.leading_zeros() as i32;
            ((16383 + 63 - 1074 - s) as u16, frac << s)
        }
    } else if exp == 0x7FF {
        (0x7FFF, (1u64 << 63) | (frac << 11))
    } else {
        ((exp - 1023 + 16383) as u16, (1u64 << 63) | (frac << 11))
    };
    let mut out = [0u8; 10];
    out[..8].copy_from_slice(&m.to_le_bytes());
    out[8..].copy_from_slice(&(sign | e).to_le_bytes());
    out
}

/// `wabi_wrap!(path; a b c)`: the erased entry (as `usize`) of the function `path` taking three
/// arguments (their types are inferred from `path`'s signature).
#[macro_export]
macro_rules! wabi_wrap {
    ($f:path; $($a:ident)*) => {{
        #[allow(unused_variables, unused_mut, non_snake_case, unsafe_code, clippy::unused_unit)]
        extern "C" fn __sa_wabi(_callee: u32, __a: *const u32, __ret: *mut $crate::wabi::Ret) {
            // SAFETY: the caller passes an `Args` buffer and a `Ret` slot.
            let mut __rd = unsafe { $crate::wabi::Reader::new(__a) };
            $(let $a = __rd.take();)*
            #[allow(unused_unsafe)]
            let __r = unsafe { $f($($a),*) };
            $crate::wabi::put_ret(__r, unsafe { &mut *__ret });
        }
        __sa_wabi as $crate::wabi::Erased as usize
    }};
}

/// A function item (zero-sized) that [`erase`] turns into an erased entry; `M` is its argument tuple.
pub trait Erase<M> {
    fn erase(self) -> usize;
}

macro_rules! erase_arity {
    ($($a:ident)*) => {
        impl<F, R, $($a),*> Erase<($($a,)*)> for F
        where
            F: Fn($($a),*) -> R + Copy + 'static,
        {
            #[allow(non_snake_case, unused_mut, unused_variables)]
            fn erase(self) -> usize {
                assert!(size_of::<F>() == 0, "wabi::erase needs a function item, not a pointer or closure with state");
                extern "C" fn tramp<F, R, $($a),*>(_callee: u32, args: *const u32, ret: *mut Ret)
                where
                    F: Fn($($a),*) -> R,
                {
                    // SAFETY: F is a zero-sized function item type (checked in `erase`): any value is it.
                    let f: F = unsafe { core::mem::MaybeUninit::<F>::uninit().assume_init() };
                    // SAFETY: the dispatcher passes an `Args` buffer and a `Ret` slot.
                    let mut rd = unsafe { Reader::new(args) };
                    $(let $a: $a = rd.take();)*
                    put_ret(f($($a),*), unsafe { &mut *ret });
                }
                tramp::<F, R, $($a),*> as Erased as usize
            }
        }
    };
}

erase_arity!();
erase_arity!(A1);
erase_arity!(A1 A2);
erase_arity!(A1 A2 A3);
erase_arity!(A1 A2 A3 A4);
erase_arity!(A1 A2 A3 A4 A5);
erase_arity!(A1 A2 A3 A4 A5 A6);
erase_arity!(A1 A2 A3 A4 A5 A6 A7);
erase_arity!(A1 A2 A3 A4 A5 A6 A7 A8);

/// The erased entry of the function item `f` (any arity up to 8; argument types from its signature).
pub fn erase<M, F: Erase<M>>(f: F) -> usize {
    f.erase()
}
