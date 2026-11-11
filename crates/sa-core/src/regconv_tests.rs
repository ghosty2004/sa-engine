//! Register-convention hooks end to end (HARNESS-5): one test hook per convention variant. On 32-bit
//! x86 (`cargo test -p sa-core --target i686-unknown-linux-gnu`) each hook's naked entry is called
//! through [`call_regs`] with the original's register picture, which checks the entry's argument
//! moves, result registers, kept registers, stack cleanup and the 80-bit ST0 result; on every target
//! the registrations (`HookReg`) are checked. The attributes are spelled `#[sa_macros::hook(` so that
//! `hooks::ALL`'s source count does not see these test hooks.
#![allow(unsafe_code)]
#![cfg_attr(not(target_arch = "x86"), allow(dead_code))]

use super::*;
use crate::abi::{AbiRet, Conv, HookReg};
use crate::mem::GPtr;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

static SEEN: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];

fn seen(i: usize) -> u32 {
    SEEN[i].load(Relaxed)
}

/// AL + two stack words, caller pops, void (GetCharacterPositionInCharSet's shape).
#[sa_macros::hook(addr = 0x6FD7C0, conv = reg("args = al stack stack"), name = "test::al_stack", expect = "0F B6 C0 83 C0")]
fn al_stack(c: u8, a: u32, b: GPtr<u32>) {
    SEEN[0].store(u32::from(c), Relaxed);
    SEEN[1].store(a, Relaxed);
    SEEN[2].store(b.addr() as u32, Relaxed);
}

/// EBX + ESI, an X87 result with a 64-bit significand (`_CalcDistanceSqrToEntity`'s registers, an
/// FYL2X-wide value as AudioLog10 returns).
#[sa_macros::hook(addr = 0x5DBE40, conv = reg("args = ebx esi"), name = "test::ebx_esi_x87", expect = "83 EC 10 57 8B")]
fn ebx_esi_x87(e: u32, p: GPtr<u8>) -> sa_math::x87::X87 {
    SEEN[0].store(e, Relaxed);
    SEEN[1].store(p.addr() as u32, Relaxed);
    sa_math::x87::X87::lg2().fyl2x(sa_math::x87::X87::from_f32(3.0))
}

/// A stack float, X87 result (AudioLog10's convention: cdecl + 80-bit ST0).
#[sa_macros::hook(addr = 0x4D9E50, conv = reg("args = stack"), name = "test::stack_x87", expect = "D9 44 24 04 D8")]
fn stack_x87(x: f32) -> sa_math::x87::X87 {
    sa_math::x87::X87::lg2().fyl2x(sa_math::x87::X87::from_f32(x))
}

/// AL in, AL out with the caller's upper EAX.
#[sa_macros::hook(addr = 0x401000, conv = reg("args = al; ret = al:entry"), name = "test::al_entry", expect = "90 90 90 90 90")]
fn al_entry(c: u8) -> u8 {
    c.wrapping_add(1)
}

/// EAX (this) + EDX + DI + one stack word, AX result with the caller's upper EAX, callee pops,
/// ECX kept.
#[sa_macros::hook(addr = 0x401010, conv = reg("args = eax edx bx stack; ret = ax:entry; keep = ecx; pop"),
    name = "test::mixed", expect = "90 90 90 90 90", this = "CPool<int32>")]
fn mixed(this: GPtr<u8>, d: u32, v: u16, s: i32) -> u16 {
    SEEN[0].store(this.addr() as u32, Relaxed);
    SEEN[1].store(d, Relaxed);
    SEEN[2].store(u32::from(v), Relaxed);
    SEEN[3].store(s as u32, Relaxed);
    0xBEEF
}

/// AX only (upper EAX undefined), EDX kept, a 2-word stack argument.
#[sa_macros::hook(addr = 0x401020, conv = reg("args = ecx stack stack; ret = ax; keep = edx"), name = "test::ax_only", expect = "90 90 90 90 90")]
fn ax_only(c: u32, d: f64, w: u32) -> u16 {
    SEEN[0].store(c, Relaxed);
    SEEN[1].store(d.to_bits() as u32, Relaxed);
    SEEN[2].store((d.to_bits() >> 32) as u32, Relaxed);
    SEEN[3].store(w, Relaxed);
    7
}

/// ESI in, EDX:EAX out, void-style keep of EAX not allowed (u64 result).
#[sa_macros::hook(addr = 0x401030, conv = reg("args = esi edi"), name = "test::wide", expect = "90 90 90 90 90")]
fn wide(a: u32, b: u32) -> u64 {
    (u64::from(a) << 32) | u64::from(b)
}

/// A void function whose callers keep a value in EAX and ECX (`keep = eax ecx`), CL argument.
#[sa_macros::hook(addr = 0x401040, conv = reg("args = cl; keep = eax ecx"), name = "test::keep_all", expect = "90 90 90 90 90")]
fn keep_all(c: i8) {
    SEEN[0].store(c as u32, Relaxed);
}

/// f32 result in ST0 from an EDI argument.
#[sa_macros::hook(addr = 0x401050, conv = reg("args = edi"), name = "test::f32_st0", expect = "90 90 90 90 90")]
fn f32_st0(x: f32) -> f32 {
    x * 2.0
}

/// A partial port with a register convention (ADR-002 fallback through `call_regs` in phase A).
#[sa_macros::hook(addr = 0x401060, conv = reg("args = eax esi"), name = "test::fallback", expect = "90 90 90 90 90", fallback)]
fn partial(a: u32, b: GPtr<u8>) -> u32 {
    if a == 0 { partial_original(a, b) } else { a + 1 }
}

#[test]
fn fallback_with_a_register_convention() {
    assert_eq!(partial(1, GPtr::NULL), 2);
    assert!(HOOK_PARTIAL.set_original.is_some());
    // phases B/C: no original body
    assert!(std::panic::catch_unwind(|| partial(0, GPtr::NULL)).is_err());
}

#[test]
fn registrations() {
    let r: &HookReg = &HOOK_AL_STACK;
    assert_eq!((r.conv, r.stack_words, r.ret, r.preserve, r.arg8), (Conv::Cdecl, 2, AbiRet::Void, 0, 0));
    assert_eq!(r.regs.unwrap().args[0], Loc::Al);
    let r = &HOOK_EBX_ESI_X87;
    assert_eq!((r.conv, r.stack_words, r.ret), (Conv::Cdecl, 0, AbiRet::St0));
    assert_eq!(r.regs.unwrap().input_regs(), 0b11000);
    let r = &HOOK_STACK_X87;
    assert_eq!((r.stack_words, r.ret), (1, AbiRet::St0));
    assert_eq!(HOOK_AL_ENTRY.ret, AbiRet::Eax);
    let r = &HOOK_MIXED;
    assert_eq!((r.conv, r.stack_words, r.ret, r.preserve), (Conv::Stdcall, 1, AbiRet::Eax, crate::abi::PRESERVE_ECX));
    assert_eq!((r.this_type, r.regs.unwrap().this_reg()), (Some("CPool<int32>"), Some(0)));
    let r = &HOOK_AX_ONLY;
    assert_eq!((r.stack_words, r.ret, r.preserve), (3, AbiRet::Ax, crate::abi::PRESERVE_EDX));
    assert_eq!(HOOK_WIDE.ret, AbiRet::EaxEdx);
    assert_eq!((HOOK_KEEP_ALL.ret, HOOK_KEEP_ALL.preserve), (AbiRet::Void, crate::abi::PRESERVE_ECX));
    assert_eq!(HOOK_F32_ST0.ret, AbiRet::St0);
    // plain Rust off the hook
    assert_eq!(al_entry(4), 5);
    assert_eq!(wide(1, 2), 0x1_0000_0002);
}

#[cfg(target_arch = "x86")]
mod x86 {
    use super::*;
    use std::sync::Mutex;

    /// The entries share `SEEN`: one at a time.
    static LOCK: Mutex<()> = Mutex::new(());

    fn frame(r: &HookReg, regs: [u32; 6], stack: &[u32], st0: bool) -> RegFrame {
        let mut f = RegFrame::new((r.entry)() as u32);
        f.regs[..6].copy_from_slice(&regs);
        f.stack[..stack.len()].copy_from_slice(stack);
        f.nstack = stack.len() as u32;
        f.want_st0 = st0 as u8;
        f
    }

    fn call(f: &mut RegFrame) {
        // SAFETY: the entries follow their declared conventions.
        unsafe { call_regs(f) }
    }

    const R: [u32; 6] = [0xA0A0_A041, 0xC0C0_C0C0, 0xD0D0_D0D0, 0xB0B0_B0B0, 0x5151_5151, 0xD1D1_D1D1];

    fn callee_saved_kept(f: &RegFrame) {
        assert_eq!(&f.out[3..6], &R[3..6], "EBX/ESI/EDI returned unchanged");
    }

    #[test]
    fn al_and_stack_words() {
        let _g = LOCK.lock().unwrap();
        let before = (HOOK_AL_STACK.calls)();
        let mut f = frame(&HOOK_AL_STACK, R, &[0x1234_5678, 0x0040_1000], false);
        call(&mut f);
        assert_eq!((seen(0), seen(1), seen(2)), (0x41, 0x1234_5678, 0x0040_1000));
        assert_eq!(f.out[0], R[0], "void: EAX as the caller passed it");
        assert_eq!(f.esp_delta, -8, "caller pops");
        callee_saved_kept(&f);
        assert_eq!((HOOK_AL_STACK.calls)(), before + 1);
    }

    #[test]
    fn ebx_esi_and_wide_st0() {
        let _g = LOCK.lock().unwrap();
        let mut f = frame(&HOOK_EBX_ESI_X87, R, &[], true);
        call(&mut f);
        assert_eq!((seen(0), seen(1)), (R[3], R[4]));
        let want = ebx_esi_x87(0, GPtr::NULL);
        assert!(want.is_wide(), "the test value needs all 64 bits");
        assert_eq!(f.st0, want.to_ext(), "80-bit ST0");
        assert_eq!(f.esp_delta, 0);
        callee_saved_kept(&f);
    }

    #[test]
    fn stack_float_to_st0() {
        let _g = LOCK.lock().unwrap();
        let mut f = frame(&HOOK_STACK_X87, R, &[7.5f32.to_bits()], true);
        call(&mut f);
        assert_eq!(f.st0, stack_x87(7.5).to_ext());
        assert_eq!(f.esp_delta, -4);
        let mut f = frame(&HOOK_F32_ST0, R, &[], true);
        f.regs[5] = 1.25f32.to_bits();
        call(&mut f);
        assert_eq!(f.st0, sa_math::x87::X87::from_f32(2.5).to_ext());
    }

    #[test]
    fn al_and_ax_with_the_entry_eax() {
        let _g = LOCK.lock().unwrap();
        let mut f = frame(&HOOK_AL_ENTRY, R, &[], false);
        call(&mut f);
        assert_eq!(f.out[0], 0xA0A0_A042);
        let mut f = frame(&HOOK_MIXED, R, &[0xFFFF_FFFE], false);
        f.regs[0] = 0x1234_0000;
        call(&mut f);
        assert_eq!((seen(0), seen(1), seen(2), seen(3)), (0x1234_0000, R[2], 0xB0B0, 0xFFFF_FFFE));
        assert_eq!(f.out[0], 0x1234_BEEF, "AX with the caller's upper EAX");
        assert_eq!(f.out[1], R[1], "keep = ecx");
        assert_eq!(f.esp_delta, 0, "pop: the callee pops its stack word");
        callee_saved_kept(&f);
    }

    #[test]
    fn ax_only_keeps_edx_and_copies_a_double() {
        let _g = LOCK.lock().unwrap();
        let d = 1.5f64.to_bits();
        let mut f = frame(&HOOK_AX_ONLY, R, &[d as u32, (d >> 32) as u32, 99], false);
        call(&mut f);
        assert_eq!((seen(0), seen(1), seen(2), seen(3)), (R[1], d as u32, (d >> 32) as u32, 99));
        assert_eq!(f.out[0] & 0xFFFF, 7);
        assert_eq!(f.out[2], R[2], "keep = edx");
        assert_eq!(f.esp_delta, -12);
    }

    #[test]
    fn edx_eax_and_kept_eax_ecx() {
        let _g = LOCK.lock().unwrap();
        let mut f = frame(&HOOK_WIDE, R, &[], false);
        call(&mut f);
        assert_eq!((f.out[0], f.out[2]), (R[5], R[4]));
        let mut f = frame(&HOOK_KEEP_ALL, R, &[], false);
        f.regs[1] = 0x1234_56F0;
        call(&mut f);
        assert_eq!(seen(0), 0xFFFF_FFF0, "CL sign-extended as i8");
        assert_eq!((f.out[0], f.out[1]), (R[0], 0x1234_56F0));
    }

    // A hand-written register-convention callee for `call_regs` itself:
    // EAX = EAX + ECX + EDX + EBX + ESI + EDI + [esp+4] + [esp+8]; ST0 = 1.0; ECX = 0; `ret 8`.
    core::arch::global_asm!(
        ".globl sa_core_test_regsum",
        "sa_core_test_regsum:",
        "add eax, ecx",
        "add eax, edx",
        "add eax, ebx",
        "add eax, esi",
        "add eax, edi",
        "add eax, dword ptr [esp + 4]",
        "add eax, dword ptr [esp + 8]",
        "xor ecx, ecx",
        "fld1",
        "ret 8",
    );

    unsafe extern "C" {
        fn sa_core_test_regsum();
    }

    #[test]
    fn call_regs_loads_and_captures() {
        let mut f = RegFrame::new(sa_core_test_regsum as *const () as usize as u32);
        f.regs[..6].copy_from_slice(&[1, 2, 4, 8, 16, 32]);
        f.stack[..2].copy_from_slice(&[64, 128]);
        f.nstack = 2;
        f.want_st0 = 1;
        call(&mut f);
        assert_eq!(f.out, [255, 0, 4, 8, 16, 32]);
        assert_eq!(f.st0, sa_math::x87::X87::one().to_ext());
        assert_eq!(f.esp_delta, 0);
    }
}
