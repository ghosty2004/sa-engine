//! The registry probes of the per-frame measurement ([`crate::perf::PROBES`]): with the measurement on, the address
//! registry's entry of each probed port is replaced by a wrapper that opens the probe's phase and calls the port's
//! entry with the same arguments (the wrapper has the entry's ABI). The ports themselves are not touched; with the
//! measurement off nothing is replaced.
//!
//! i686: one typed wrapper per probe and signature ([`crate::perf::Sig`]: the original convention and argument
//! words; the result is passed through as EAX:EDX). wasm32: registry entries are type-erased (`sa_core::wabi`), so one
//! wrapper serves every probe and finds the port's entry by the callee address.

#![allow(unsafe_code)]

use std::sync::OnceLock;

/// `(address, port entry, probe index)` of every installed probe, by address.
static INSTALLED: OnceLock<Vec<(u32, usize, usize)>> = OnceLock::new();

/// Replaces the registry entries (`(address, entry, name)`, before `registry::install`) of the probed ports by their
/// wrappers. Returns how many were installed.
pub fn wrap(v: &mut [(u32, usize, &'static str)]) -> usize {
    let mut installed = Vec::new();
    for (i, pr) in crate::perf::PROBES.iter().enumerate() {
        let Some(w) = wrapper(i) else { continue };
        match v.iter_mut().find(|e| e.0 == pr.addr) {
            Some(e) => {
                installed.push((pr.addr, e.1, i));
                e.1 = w;
            }
            None => println!(
                "sa-game: perf: no registry entry for {} ({:#x}): not timed",
                pr.name, pr.addr
            ),
        }
    }
    installed.sort_unstable();
    let n = installed.len();
    #[cfg(target_arch = "x86")]
    for &(_, f, i) in &installed {
        x86::REAL[i].store(f, std::sync::atomic::Ordering::Relaxed);
    }
    let _ = INSTALLED.set(installed);
    n
}

#[cfg(target_arch = "x86")]
mod x86 {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::perf::{PROBES, Sig, probe_enter};

    /// The most probes the wrapper table holds.
    pub const MAX: usize = 64;
    const _: () = assert!(PROBES.len() <= MAX);

    /// The port entry behind probe `I`.
    pub static REAL: [AtomicUsize; MAX] = [const { AtomicUsize::new(0) }; MAX];

    fn real<F>(i: usize) -> F {
        let f = REAL[i].load(Ordering::Relaxed);
        // SAFETY: `F` is the function-pointer type of the port's registry entry (the probe's signature).
        unsafe { std::mem::transmute_copy::<usize, F>(&f) }
    }

    extern "C-unwind" fn c0<const I: usize>() -> u64 {
        let _p = probe_enter(I);
        real::<extern "C-unwind" fn() -> u64>(I)()
    }
    extern "C-unwind" fn c1<const I: usize>(a: u32) -> u64 {
        let _p = probe_enter(I);
        real::<extern "C-unwind" fn(u32) -> u64>(I)(a)
    }
    extern "thiscall-unwind" fn t1<const I: usize>(this: u32) -> u64 {
        let _p = probe_enter(I);
        real::<extern "thiscall-unwind" fn(u32) -> u64>(I)(this)
    }
    extern "thiscall-unwind" fn t3<const I: usize>(this: u32, a: u32, b: u32) -> u64 {
        let _p = probe_enter(I);
        real::<extern "thiscall-unwind" fn(u32, u32, u32) -> u64>(I)(this, a, b)
    }

    fn pick<const I: usize>() -> Option<usize> {
        let pr = PROBES.get(I)?;
        Some(match pr.sig {
            Sig::Cdecl(0) => c0::<I> as *const () as usize,
            Sig::Cdecl(1) => c1::<I> as *const () as usize,
            Sig::Thiscall(1) => t1::<I> as *const () as usize,
            Sig::Thiscall(3) => t3::<I> as *const () as usize,
            _ => return None,
        })
    }

    macro_rules! table {
        ($($i:literal)*) => { [$(pick::<$i> as fn() -> Option<usize>),*] };
    }

    /// The wrapper of every probe index (a signature without a wrapper here: none).
    pub static PICK: [fn() -> Option<usize>; MAX] = table!(
        0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
        32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63
    );
}

#[cfg(target_arch = "x86")]
fn wrapper(i: usize) -> Option<usize> {
    x86::PICK.get(i).and_then(|f| f())
}

#[cfg(target_arch = "wasm32")]
fn wrapper(_i: usize) -> Option<usize> {
    extern "C" fn w(callee: u32, args: *const u32, ret: *mut sa_core::wabi::Ret) {
        let Some(&(_, f, i)) = INSTALLED
            .get()
            .and_then(|t| t.binary_search_by_key(&callee, |e| e.0).ok().map(|k| &t[k]))
        else {
            return;
        };
        let _p = crate::perf::probe_enter(i);
        // SAFETY: registry entries on wasm32 are `Erased` function pointers; the arguments and the result slot are
        // the caller's.
        let f = unsafe { std::mem::transmute::<usize, sa_core::wabi::Erased>(f) };
        f(callee, args, ret);
    }
    Some(w as sa_core::wabi::Erased as usize)
}

/// Other targets have no original-ABI dispatch: nothing to wrap.
#[cfg(not(any(target_arch = "x86", target_arch = "wasm32")))]
fn wrapper(_i: usize) -> Option<usize> {
    None
}
