//! `CTimer` (0x5617E0–0x561B07) except `CTimer::Update` (0x561B10), whose hook stays the sa-hook
//! passthrough that marks the frame boundary (determinism mode, INF-4).
//!
//! Ported from the 1.0 US bytes; where gta-reversed's `Timer.cpp` differs the original is followed
//! (listed in `docs/arch/pilot-ports.md`): `Initialise` divides only the low dword of the
//! performance frequency by 1000; `UpdateVariables` accumulates the unclipped time as
//! `ftol((float)nonClipped + delta)` (not `nonClipped += (uint32)delta`), clips the clipped step with
//! an unsigned compare against 300 after `_ftol`, and every float op is x87 under PC=24.

use crate::mem::{GBool, GPtr, TimerFn};
use crate::{globals, hook, orig};
use sa_math::x87::X87;

globals! {
    list TIMER_GLOBALS;
    /// `CTimer::ms_fnTimerFunction`: 0x5617A0 (QueryPerformanceCounter) or 0x5617C0 (RsTimer).
    pub static TIMER_FUNCTION @ 0xB7CB28: Option<TimerFn> = None;
    /// `CTimer::m_snTimerDivider` (timer ticks per millisecond)
    pub static TIMER_DIVIDER @ 0xB7CB2C: u32 = 0;
    /// `CTimer::m_snRenderPauseTime`
    pub static RENDER_PAUSE_TIME @ 0xB7CB30: u64 = 0;
    /// `CTimer::m_snRenderStartTime`
    pub static RENDER_START_TIME @ 0xB7CB38: u64 = 0;
    /// `CTimer::m_sbEnableTimeDebug`
    pub static ENABLE_TIME_DEBUG @ 0xB7CB40: GBool = GBool::FALSE;
    /// `CTimer::m_snRenderTimerPauseCount`
    pub static RENDER_TIMER_PAUSE_COUNT @ 0xB7CB44: u32 = 0;
    /// `CTimer::m_CodePause`
    pub static CODE_PAUSE @ 0xB7CB48: GBool = GBool::FALSE;
    /// `CTimer::m_UserPause`
    pub static USER_PAUSE @ 0xB7CB49: GBool = GBool::FALSE;
    /// `CTimer::m_FrameCounter`
    pub static FRAME_COUNTER @ 0xB7CB4C: u32 = 0;
    /// `CTimer::game_FPS`
    pub static GAME_FPS @ 0xB7CB50: f32 = 0.0;
    /// `CTimer::ms_fOldTimeStep`
    pub static OLD_TIME_STEP @ 0xB7CB54: f32 = 0.0;
    /// `CTimer::ms_fTimeStepNonClipped`
    pub static TIME_STEP_NON_CLIPPED @ 0xB7CB58: f32 = 0.0;
    /// `CTimer::ms_fTimeStep`
    pub static TIME_STEP @ 0xB7CB5C: f32 = 0.0;
    /// `CTimer::ms_fSlowMotionScale`
    pub static SLOW_MOTION_SCALE @ 0xB7CB60: f32 = 0.0;
    /// `CTimer::ms_fTimeScale`
    pub static TIME_SCALE @ 0xB7CB64: f32 = 0.0;
    /// `CTimer::m_snPreviousTimeInMillisecondsNonClipped`
    pub static PREV_TIME_MS_NON_CLIPPED @ 0xB7CB68: u32 = 0;
    /// `CTimer::m_snPPPPreviousTimeInMilliseconds`
    pub static PPPP_TIME_MS @ 0xB7CB6C: u32 = 0;
    /// `CTimer::m_snPPPreviousTimeInMilliseconds`
    pub static PPP_TIME_MS @ 0xB7CB70: u32 = 0;
    /// `CTimer::m_snPPreviousTimeInMilliseconds`
    pub static PP_TIME_MS @ 0xB7CB74: u32 = 0;
    /// `CTimer::m_snPreviousTimeInMilliseconds`
    pub static PREV_TIME_MS @ 0xB7CB78: u32 = 0;
    /// `CTimer::m_snTimeInMillisecondsPauseMode`
    pub static TIME_MS_PAUSE_MODE @ 0xB7CB7C: u32 = 0;
    /// `CTimer::m_snTimeInMillisecondsNonClipped`
    pub static TIME_MS_NON_CLIPPED @ 0xB7CB80: u32 = 0;
    /// `CTimer::m_snTimeInMilliseconds`
    pub static TIME_MS @ 0xB7CB84: u32 = 0;
    /// `CTimer::bSlowMotionActive`
    pub static SLOW_MOTION_ACTIVE @ 0xB7CB88: GBool = GBool::FALSE;
    /// `CTimer::bSkipProcessThisFrame`
    pub static SKIP_PROCESS_THIS_FRAME @ 0xB7CB89: GBool = GBool::FALSE;
    /// `CSpecialFX::bSnapShotActive` (read by `UpdateVariables`)
    pub static SNAPSHOT_ACTIVE @ 0xC7C714: GBool = GBool::FALSE;
}

/// `.rdata` constants (f32 bits).
pub mod k {
    /// 0x858C54: 2^32, added after `fild` of an unsigned value with bit 31 set
    pub const TWO_POW_32: u32 = 0x4F80_0000;
    /// 0x858BA4: 20.0
    pub const TWENTY: u32 = 0x41A0_0000;
    /// 0x858C28: 0.05 (1 / 20 ms per step)
    pub const STEP_PER_MS: u32 = 0x3D4C_CCCD;
    /// 0x858C58: 0.01 (minimum unclipped step while not paused)
    pub const MIN_STEP_NON_CLIPPED: u32 = 0x3C23_D70A;
    /// 0x858B3C: 3.0 (maximum step)
    pub const MAX_STEP: u32 = 0x4040_0000;
    /// 0x858C14: 1e-5 (minimum step)
    pub const MIN_STEP: u32 = 0x3727_C5AC;
    /// 0x858624: 1.0
    pub const ONE: u32 = 0x3F80_0000;
}

/// Timer function addresses `Initialise` stores.
pub const QPC_TIMER_FUNCTION: u32 = 0x5617A0;
pub const MS_TIMER_FUNCTION: u32 = 0x5617C0;

orig! {
    /// `QueryPerformanceFrequency` as `CTimer::Initialise` calls it (`call [0x858114]` at 0x56186C).
    pub fn query_performance_frequency(out: GPtr<u64>) -> i32 = stdcall site 0x56186C => unported;
    /// `QueryPerformanceCounter` as `CTimer::GetCurrentTimeInCycles` calls it (0x561A88; a virtual-clock
    /// site in determinism mode).
    pub fn query_performance_counter_cycles(out: GPtr<u64>) -> i32 = stdcall site 0x561A88 => unported;
}

/// `fild dword [u32]` followed by `fadd [2^32]` when the sign bit is set: the original's
/// unsigned-to-float conversion.
pub fn fild_u32(v: u32) -> X87 {
    let x = X87::from_i32(v as i32);
    if (v as i32) < 0 {
        x.fadd_m(f32::from_bits(k::TWO_POW_32))
    } else {
        x
    }
}

/// Calls the timer function stored in `ms_fnTimerFunction` (an original function in phase A).
fn timer_function() -> u64 {
    let f = TIMER_FUNCTION
        .get()
        .expect("CTimer: ms_fnTimerFunction is null (Initialise not run)");
    call_timer_fn(f)
}

#[allow(unsafe_code)]
fn call_timer_fn(f: TimerFn) -> u64 {
    // SAFETY: the pointer was stored by `CTimer::Initialise` (one of the two timer functions).
    unsafe { f() }
}

/// `CTimer::Initialise` (0x5617E0; entry `xor eax,eax; jmp 0x401220`, whose chunk stores
/// `m_UserPause` and jumps back to 0x5617E7).
#[hook(addr = 0x5617E0, conv = cdecl, name = "CTimer::Initialise", expect = "33 C0 E9 39 FA E9 FF")]
pub fn initialise() {
    USER_PAUSE.set(GBool::FALSE);
    CODE_PAUSE.set(GBool::FALSE);
    SLOW_MOTION_ACTIVE.set(GBool::FALSE);
    SKIP_PROCESS_THIS_FRAME.set(GBool::FALSE);
    PPPP_TIME_MS.set(0);
    PPP_TIME_MS.set(0);
    PP_TIME_MS.set(0);
    PREV_TIME_MS.set(0);
    PREV_TIME_MS_NON_CLIPPED.set(0);
    TIME_MS.set(0);
    FRAME_COUNTER.set(0);
    RENDER_TIMER_PAUSE_COUNT.set(0);
    ENABLE_TIME_DEBUG.set(GBool::FALSE);
    GAME_FPS.set(0.0);
    TIME_MS_NON_CLIPPED.set(1);
    TIME_MS_PAUSE_MODE.set(1);
    TIME_SCALE.set(1.0);
    TIME_STEP.set(1.0);
    OLD_TIME_STEP.set(1.0);
    SLOW_MOTION_SCALE.set(-1.0);
    // The LARGE_INTEGER is uninitialised stack in the original; it is only read when QPF succeeds.
    let mut freq: u64 = 0;
    let ok = query_performance_frequency(GPtr::from_raw(&raw mut freq));
    let f: TimerFn = if ok != 0 {
        // mul dword [esp] by 0x10624DD3, shr edx, 6: the low dword divided by 1000 (unsigned)
        TIMER_DIVIDER.set((freq as u32) / 1000);
        timer_fn_at(QPC_TIMER_FUNCTION)
    } else {
        TIMER_DIVIDER.set(1);
        timer_fn_at(MS_TIMER_FUNCTION)
    };
    TIMER_FUNCTION.set(Some(f));
    RENDER_START_TIME.set(call_timer_fn(f));
}

/// The timer function at an original address (phase A) or its phase B/C replacement.
fn timer_fn_at(addr: u32) -> TimerFn {
    #[cfg(sa_phase_a)]
    {
        #[allow(unsafe_code)]
        // SAFETY: 0x5617A0 / 0x5617C0 are `uint64 (*)()` cdecl functions of the exe.
        unsafe {
            core::mem::transmute::<usize, TimerFn>(addr as usize)
        }
    }
    #[cfg(not(sa_phase_a))]
    {
        match addr {
            QPC_TIMER_FUNCTION => qpc_timer_function,
            _ => ms_timer_function,
        }
    }
}

/// Phases B/C: 0x5617A0 (`QueryPerformanceCounter` → the 64-bit count). The counter comes from
/// the `query_performance_counter_cycles` declaration, which `sa-game` binds to sa-platform's
/// Win32 time; an unbound declaration aborts here (a panic cannot unwind out of `extern "C"`).
#[cfg(not(sa_phase_a))]
#[allow(unsafe_code)]
unsafe extern "C" fn qpc_timer_function() -> u64 {
    let mut c: u64 = 0;
    query_performance_counter_cycles(GPtr::from_raw(&raw mut c));
    c
}

/// Phases B/C: 0x5617C0 (`RsTimer()` zero-extended: milliseconds).
#[cfg(not(sa_phase_a))]
#[allow(unsafe_code)]
unsafe extern "C" fn ms_timer_function() -> u64 {
    crate::platform::rs_timer() as u64
}

/// `CTimer::Shutdown` (0x5618C0).
#[hook(addr = 0x5618C0, conv = cdecl, name = "CTimer::Shutdown", expect = "C6 05 40 CB B7 00 00")]
pub fn shutdown() {
    ENABLE_TIME_DEBUG.set(GBool::FALSE);
}

/// The values `CTimer::UpdateVariables` writes, as a pure function of what it reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepIn {
    pub time_elapsed: f32,
    pub divider: u32,
    pub non_clipped_ms: u32,
    pub time_ms: u32,
    pub time_step: f32,
    pub user_pause: bool,
    pub code_pause: bool,
    pub snapshot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepOut {
    pub non_clipped_ms: u32,
    pub time_ms: u32,
    pub time_step_non_clipped: f32,
    pub old_time_step: f32,
    pub time_step: f32,
}

/// Pure part of `CTimer::UpdateVariables` (0x5618D0), instruction by instruction.
pub fn step(i: StepIn) -> StepOut {
    let delta = fild_u32(i.divider).fdivr_m(i.time_elapsed); // fdivr [esp+8]
    let nc = fild_u32(i.non_clipped_ms).fadd(delta); // fadd st, st(1)
    let non_clipped_ms = nc.ftol() as u32;
    let mut ts_nc = X87::constant(k::STEP_PER_MS).fmul(delta).to_f32(); // fstp [0xB7CB58]
    let mut clipped = delta.ftol() as u32;
    if clipped > 300 {
        clipped = 300;
    }
    let time_ms = i.time_ms.wrapping_add(clipped);
    // fcomp [0.01]; test ah,5; jp: taken for >=/unordered
    let below = !X87::from_f32(ts_nc)
        .fcom(X87::constant(k::MIN_STEP_NON_CLIPPED))
        .test_parity(5);
    if below && !i.user_pause && !i.code_pause && !i.snapshot {
        ts_nc = f32::from_bits(k::MIN_STEP_NON_CLIPPED);
    }
    let old_time_step = i.time_step;
    let under_max = !X87::from_f32(ts_nc)
        .fcom(X87::constant(k::MAX_STEP))
        .test_parity(5);
    let v = if under_max {
        ts_nc
    } else {
        f32::from_bits(k::MAX_STEP)
    };
    // fcomp [1e-5]; test ah,0x41; jp: taken for > / unordered; falls through for < and ==
    let at_most_min = !X87::from_f32(v)
        .fcom(X87::constant(k::MIN_STEP))
        .test_parity(0x41);
    let time_step = if at_most_min {
        f32::from_bits(k::MIN_STEP)
    } else {
        v
    };
    StepOut {
        non_clipped_ms,
        time_ms,
        time_step_non_clipped: ts_nc,
        old_time_step,
        time_step,
    }
}

/// `CTimer::UpdateVariables(float timeElapsed)` (0x5618D0, cdecl).
#[hook(addr = 0x5618D0, conv = cdecl, name = "CTimer::UpdateVariables", expect = "51 A1 2C CB B7 00")]
pub fn update_variables(time_elapsed: f32) {
    let o = step(StepIn {
        time_elapsed,
        divider: TIMER_DIVIDER.get(),
        non_clipped_ms: TIME_MS_NON_CLIPPED.get(),
        time_ms: TIME_MS.get(),
        time_step: TIME_STEP.get(),
        user_pause: USER_PAUSE.get().get(),
        code_pause: CODE_PAUSE.get().get(),
        snapshot: SNAPSHOT_ACTIVE.get().get(),
    });
    TIME_MS_NON_CLIPPED.set(o.non_clipped_ms);
    TIME_STEP_NON_CLIPPED.set(o.time_step_non_clipped);
    TIME_MS.set(o.time_ms);
    OLD_TIME_STEP.set(o.old_time_step);
    TIME_STEP.set(o.time_step);
}

/// `CTimer::Update` (0x561B10, cdecl). Phase B entry only: in phase A sa-hook keeps its own hook here
/// (the frame boundary of determinism mode), so this port is `#[native]`. Nothing without a timer
/// function. `game_FPS = 1000 / (nonClipped - prevNonClipped)` (`fild` of the unsigned difference,
/// `fdivr`), `m_sbEnableTimeDebug = 1`, the previous-time chain shifted (PPPP ← PPP ← PP ← P ← now) and
/// `prevNonClipped = nonClipped`; then the timer function's 64-bit count minus the render start time
/// as an unsigned 64-bit value rounded once to float (`fild` of the low 63 bits plus 2^63 when the top
/// bit is set, `faddp` at PC=24), multiplied by `ms_fTimeScale` unless a pause is active; the render
/// start time becomes the count; `m_snTimeInMillisecondsPauseMode = ftol(pauseMode + elapsed /
/// divider)`; `UpdateVariables(paused ? 0 : elapsed)`; `m_FrameCounter += 1`.
#[crate::native(addr = 0x561B10, conv = cdecl, name = "CTimer::Update")]
pub fn update() {
    if TIMER_FUNCTION.get().is_none() {
        return;
    }
    let now_nc = TIME_MS_NON_CLIPPED.get();
    let d = now_nc.wrapping_sub(PREV_TIME_MS_NON_CLIPPED.get());
    let x = fild_u32(d);
    ENABLE_TIME_DEBUG.set(GBool::TRUE);
    let fps = x.fdivr_m(1000.0);
    PPPP_TIME_MS.set(PPP_TIME_MS.get());
    PPP_TIME_MS.set(PP_TIME_MS.get());
    PP_TIME_MS.set(PREV_TIME_MS.get());
    PREV_TIME_MS.set(TIME_MS.get());
    PREV_TIME_MS_NON_CLIPPED.set(now_nc);
    GAME_FPS.set(fps.to_f32());
    let now = timer_function();
    let elapsed_u = now.wrapping_sub(RENDER_START_TIME.get());
    // fild qword (low 63 bits) + 2^63 when the top bit is set: one rounding to 24 bits (PC=24)
    let mut elapsed = elapsed_u as f32;
    let paused = USER_PAUSE.get().get() || CODE_PAUSE.get().get();
    if !paused {
        elapsed = X87::from_f32(TIME_SCALE.get()).fmul_m(elapsed).to_f32();
    }
    let div = fild_u32(TIMER_DIVIDER.get());
    RENDER_START_TIME.set(now);
    let step = div.fdivr_m(elapsed);
    let pm = fild_u32(TIME_MS_PAUSE_MODE.get()).fadd(step);
    TIME_MS_PAUSE_MODE.set(pm.ftol() as u32);
    update_variables(if paused { 0.0 } else { elapsed });
    FRAME_COUNTER.set(FRAME_COUNTER.get().wrapping_add(1));
}

/// `CTimer::Suspend` (0x5619D0).
#[hook(addr = 0x5619D0, conv = cdecl, name = "CTimer::Suspend", expect = "A0 40 CB B7 00")]
pub fn suspend() {
    if !ENABLE_TIME_DEBUG.get().get() {
        return;
    }
    let n = RENDER_TIMER_PAUSE_COUNT.get().wrapping_add(1);
    RENDER_TIMER_PAUSE_COUNT.set(n);
    if n <= 1 {
        RENDER_PAUSE_TIME.set(timer_function());
    }
}

/// `CTimer::Resume` (0x561A00).
#[hook(addr = 0x561A00, conv = cdecl, name = "CTimer::Resume", expect = "A0 40 CB B7 00")]
pub fn resume() {
    if !ENABLE_TIME_DEBUG.get().get() {
        return;
    }
    let n = RENDER_TIMER_PAUSE_COUNT.get().wrapping_sub(1);
    RENDER_TIMER_PAUSE_COUNT.set(n);
    if n == 0 {
        let now = timer_function();
        let start = now
            .wrapping_sub(RENDER_PAUSE_TIME.get())
            .wrapping_add(RENDER_START_TIME.get());
        RENDER_START_TIME.set(start);
    }
}

/// `CTimer::GetCyclesPerMillisecond` (0x561A40).
#[hook(addr = 0x561A40, conv = cdecl, name = "CTimer::GetCyclesPerMillisecond", expect = "A1 2C CB B7 00")]
pub fn get_cycles_per_millisecond() -> u32 {
    TIMER_DIVIDER.get()
}

/// Pure part of `CTimer::GetCyclesPerFrame`: `_ftol((float)divider * 20.0f)`, low dword.
pub fn cycles_per_frame(divider: u32) -> u32 {
    fild_u32(divider).fmul_m(f32::from_bits(k::TWENTY)).ftol() as u32
}

/// `CTimer::GetCyclesPerFrame` (0x561A50; tail-jumps into `_ftol`, which leaves EDX = high dword).
#[hook(addr = 0x561A50, conv = cdecl, name = "CTimer::GetCyclesPerFrame", expect = "51 A1 2C CB B7 00")]
pub fn get_cycles_per_frame() -> u32 {
    cycles_per_frame(TIMER_DIVIDER.get())
}

/// `CTimer::GetCurrentTimeInCycles` (0x561A80): low dword of the counter minus the low dword of
/// `m_snRenderStartTime`.
#[hook(addr = 0x561A80, conv = cdecl, name = "CTimer::GetCurrentTimeInCycles", expect = "83 EC 08 8D 44 24 00")]
pub fn get_current_time_in_cycles() -> u32 {
    let mut now: u64 = 0;
    query_performance_counter_cycles(GPtr::from_raw(&raw mut now));
    (now as u32).wrapping_sub(RENDER_START_TIME.get() as u32)
}

/// `CTimer::Stop` (0x561AA0).
#[hook(addr = 0x561AA0, conv = cdecl, name = "CTimer::Stop", expect = "A1 84 CB B7 00")]
pub fn stop() {
    let t = TIME_MS.get();
    PPPP_TIME_MS.set(t);
    PPP_TIME_MS.set(t);
    PP_TIME_MS.set(t);
    PREV_TIME_MS.set(t);
    ENABLE_TIME_DEBUG.set(GBool::FALSE);
    PREV_TIME_MS_NON_CLIPPED.set(TIME_MS_NON_CLIPPED.get());
}

/// Pure part of `CTimer::GetIsSlowMotionActive`: `fld scale; fcomp [1.0]; test ah,5; jp false`.
pub fn is_slow_motion(scale: f32) -> bool {
    !X87::from_f32(scale).fcom(X87::constant(k::ONE)).test_parity(5)
}

/// `CTimer::GetIsSlowMotionActive` (0x561AD0; result in AL).
#[hook(addr = 0x561AD0, conv = cdecl, name = "CTimer::GetIsSlowMotionActive", expect = "D9 05 64 CB B7 00")]
pub fn get_is_slow_motion_active() -> bool {
    is_slow_motion(TIME_SCALE.get())
}

/// `CTimer::StartUserPause` (0x561AF0).
#[hook(addr = 0x561AF0, conv = cdecl, name = "CTimer::StartUserPause", expect = "C6 05 49 CB B7 00 01")]
pub fn start_user_pause() {
    USER_PAUSE.set(GBool::TRUE);
}

/// `CTimer::EndUserPause` (0x561B00).
#[hook(addr = 0x561B00, conv = cdecl, name = "CTimer::EndUserPause", expect = "C6 05 49 CB B7 00 00")]
pub fn end_user_pause() {
    USER_PAUSE.set(GBool::FALSE);
}

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    fn base() -> StepIn {
        StepIn {
            time_elapsed: 400_000.0, // 40 ms at 10 MHz
            divider: 10_000,
            non_clipped_ms: 1000,
            time_ms: 1000,
            time_step: 1.0,
            user_pause: false,
            code_pause: false,
            snapshot: false,
        }
    }

    #[test]
    fn determinism_mode_step() {
        // INF-4's virtual clock: 40 ms per frame, ms_fTimeStep = 2.0
        let o = step(base());
        assert_eq!(o.non_clipped_ms, 1040);
        assert_eq!(o.time_ms, 1040);
        assert_eq!(o.time_step_non_clipped, 2.0);
        assert_eq!(o.time_step, 2.0);
        assert_eq!(o.old_time_step, 1.0);
    }

    #[test]
    fn clips() {
        let o = step(StepIn {
            time_elapsed: 5_000_000.0,
            ..base()
        });
        assert_eq!(o.time_ms, 1300);
        assert_eq!(o.non_clipped_ms, 1500);
        assert_eq!(o.time_step, 3.0);
        let o = step(StepIn {
            time_elapsed: 0.0,
            ..base()
        });
        assert_eq!(o.time_step_non_clipped, f32::from_bits(k::MIN_STEP_NON_CLIPPED));
        assert_eq!(o.time_step, f32::from_bits(k::MIN_STEP_NON_CLIPPED));
        let o = step(StepIn {
            time_elapsed: 0.0,
            user_pause: true,
            ..base()
        });
        assert_eq!(o.time_step_non_clipped, 0.0);
        assert_eq!(o.time_step, f32::from_bits(k::MIN_STEP));
        // NaN elapsed: the unclipped step is NaN, the clamp picks 3.0
        let o = step(StepIn {
            time_elapsed: f32::NAN,
            ..base()
        });
        assert!(o.time_step_non_clipped.is_nan());
        assert_eq!(o.time_step, 3.0);
    }

    #[test]
    fn unclipped_ms_accumulates_through_float() {
        // (float)nonClipped is exact here, the sum is rounded to 24 bits: 2^24 + 0.9 -> 2^24 + 0
        let o = step(StepIn {
            non_clipped_ms: 1 << 24,
            time_elapsed: 9_000.0,
            ..base()
        });
        assert_eq!(o.non_clipped_ms, 1 << 24);
        // gta-reversed's `nonClipped += (uint32)delta` gives 2^24 + 1 here; the original rounds
        // 16777217.5 to 24 bits: 16777218.
        let o = step(StepIn {
            non_clipped_ms: (1 << 24) + 1,
            time_elapsed: 5_000.0,
            ..base()
        });
        assert_eq!(o.non_clipped_ms, (1 << 24) + 2);
    }

    #[test]
    fn small_helpers() {
        assert_eq!(cycles_per_frame(10_000), 200_000);
        assert_eq!(cycles_per_frame(0x8000_0000), 0); // 2^31 * 20 = 2^35.3: low dword of 42949672960
        assert!(is_slow_motion(0.5));
        assert!(!is_slow_motion(1.0));
        assert!(!is_slow_motion(f32::NAN));
        assert_eq!(fild_u32(0xFFFF_FFFF).to_f64(), 4_294_967_296.0);
    }
}

/// This module's registrations (collected by [`crate::hooks`]); not `Update` (sa-hook's frame
/// boundary passthrough).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_UPDATE_VARIABLES,
    HOOK_SUSPEND,
    HOOK_RESUME,
    HOOK_GET_CYCLES_PER_MILLISECOND,
    HOOK_GET_CYCLES_PER_FRAME,
    HOOK_GET_CURRENT_TIME_IN_CYCLES,
    HOOK_STOP,
    HOOK_GET_IS_SLOW_MOTION_ACTIVE,
    HOOK_START_USER_PAUSE,
    HOOK_END_USER_PAUSE,
];
