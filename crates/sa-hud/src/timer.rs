//! `COnscreenTimer` / `COnscreenTimerEntry` / `COnscreenCounterEntry` (PORT-HUD): the script clock and
//! counters (`CUserDisplay::OnscnTimer` 0xBA1788). Ported from the 1.0 US bytes (incl. the `.HOODLUM`
//! bodies); `docs/re/specs/HUD/COnscreenTimer.md` and gta-reversed `OnscreenTimer*.cpp` /
//! `OnscreenCounterEntry.cpp` as reading aids.
//!
//! Layout (0x154): +0 clock (`COnscreenTimerEntry`, 0x40: +0 var id, +4 key[10], +0xE text[42], +0x38
//! enabled, +0x39 direction, +0x3C beep secs); +0x40 four counters (0x44 each: +0 var id, +4 max var id,
//! +8 key[10], +0x12 type u16, +0x14 text[42], +0x3E enabled, +0x3F flash, +0x40 colour); +0x150 display,
//! +0x151 paused. Var ids are byte offsets into `CTheScripts::ScriptSpace` (0xA49960).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::timer::TIME_STEP;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, rd, sprintf1, sprintf2, wr, x};

/// `CTheScripts::ScriptSpace` (owned by sa-script)
const SCRIPT_SPACE: u32 = 0xA49960;
/// `CHud::m_Wants_To_Draw_Hud`
const HUD_WANTS_TO_DRAW: usize = 0xBAA45D;
/// `CReplay::Mode`
const REPLAY_MODE: usize = 0xA43088;
/// `TheCamera` byte +0x35 (0xB6F065), tested before the countdown beep
const CAM_BYTE_B6F065: usize = 0xB6F065;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// "%d"
const FMT_D: usize = 0x859508;
/// "%02d:%02d"
const FMT_CLOCK: usize = 0x859A6C;
/// "%d / %d"
const FMT_D_OF_D: usize = 0x859A78;

orig! {
    /// CRT `strncpy(dst, src, n)` (0x821F40)
    pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volumeChange, float speed)` (0x506EA0)
    pub fn report_frontend_audio_event(engine: GPtr<u8>, event: i32, volume: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `COnscreenTimerEntry::Process()` (0x44CB10)
    pub fn timer_entry_process_call(this: GPtr<u8>) = thiscall 0x44CB10 => timer_entry_process;
}

/// The script variable at byte offset `id` of `ScriptSpace` (32-bit wrapping address, as `[id + 0xA49960]`).
fn script_var(id: u32) -> GPtr<i32> {
    g::<i32>(SCRIPT_SPACE.wrapping_add(id) as usize)
}

fn counter(this: GPtr<u8>, i: isize) -> GPtr<u8> {
    this.byte_add(0x40 + i * 0x44)
}

/// `COnscreenCounterEntry::ProcessForDisplayCounter(eOnscreenCounter type)` (0x44CA90): the value clamped
/// at 0; type 0/1 prints "%d", type 2 "%d / %d" with the variable at `m_nMaxVarValue`, others nothing.
#[hook(addr = 0x44CA90, conv = thiscall, name = "COnscreenCounterEntry::ProcessForDisplayCounter", expect = "8B 01 8B 80 60 99 A4 00")]
pub fn process_for_display_counter(this: GPtr<u8>, ty: u32) {
    let v = script_var(rd::<u32>(this, 0)).read();
    let t1 = if v < 0 { 0 } else { v };
    let ty = ty as u16;
    let text = this.byte_add(0x14);
    match ty {
        0 | 1 => {
            sprintf1(text, g(FMT_D), t1);
        }
        2 => {
            let max = script_var(rd::<u32>(this, 4)).read();
            sprintf2(text, g(FMT_D_OF_D), t1, max);
        }
        _ => {}
    }
}

/// `COnscreenCounterEntry::SetColourID(uint8)` (0x44CB00).
#[hook(addr = 0x44CB00, conv = thiscall, name = "COnscreenCounterEntry::SetColourID", expect = "8A 44 24 04 88 41 40")]
pub fn set_colour_id(this: GPtr<u8>, colour: u32) {
    wr::<u8>(this, 0x40, colour as u8);
}

/// `COnscreenTimerEntry::Process()` (0x44CB10): advances the clock variable by
/// `_ftol(TimeStep * 0.02 * 1000)` ms; counting down below 0 clears the clock, otherwise a beep
/// (front-end event 0x21) while the old value's whole seconds are below the beep threshold.
#[hook(addr = 0x44CB10, conv = thiscall, name = "COnscreenTimerEntry::Process", expect = "56 8B F1 57 8B 3E")]
pub fn timer_entry_process(this: GPtr<u8>) {
    let id = rd::<u32>(this, 0);
    if id == 0 {
        return;
    }
    let step = ftol32(x(TIME_STEP.get()).fmul_m(0.02).fmul_m(1000.0));
    let var = script_var(id);
    let old = var.read();
    let secs = old / 1000;
    if rd::<u8>(this, 0x39) == 0 {
        var.write(old.wrapping_add(step));
        return;
    }
    var.write(old.wrapping_sub(step));
    let cur = script_var(rd::<u32>(this, 0));
    if cur.read() < 0 {
        cur.write(0);
        wr::<u32>(this, 0, 0);
        wr::<u8>(this, 4, 0);
        wr::<u8>(this, 0x38, 0);
        return;
    }
    if (secs as u32) < rd::<u32>(this, 0x3C) && g::<u8>(CAM_BYTE_B6F065).read() == 0 {
        report_frontend_audio_event(g(AUDIO_ENGINE), 0x21, 0.0, 1.0);
    }
}

/// `COnscreenTimer::Init()` (stub 0x44CBC0): counters (var, max, key, type, enabled cleared; flash = 1;
/// text and colour untouched), then the clock (var, key cleared, disabled, direction 1, beep 12 s).
#[hook(addr = 0x44CBC0, conv = thiscall, name = "COnscreenTimer::Init", expect = "E9 0B 36 12 01")]
pub fn timer_init(this: GPtr<u8>) {
    wr::<u8>(this, 0x151, 0);
    for i in 0..4 {
        let c = counter(this, i);
        wr::<u32>(c, 0, 0);
        wr::<u32>(c, 4, 0);
        wr::<u32>(c, 8, 0);
        wr::<u32>(c, 0xC, 0);
        wr::<u16>(c, 0x10, 0);
        wr::<u16>(c, 0x12, 0);
        wr::<u8>(c, 0x3E, 0);
        wr::<u8>(c, 0x3F, 1);
    }
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u16>(this, 0xC, 0);
    wr::<u8>(this, 0x38, 0);
    wr::<u8>(this, 0x39, 1);
    wr::<u32>(this, 0x3C, 12);
}

/// `COnscreenTimer::ProcessForDisplay()` (stub 0x44CC20): formats the clock ("%02d:%02d" of
/// `(t/1000/60) % 100`, `t/1000 % 60`) and the counters, setting the enabled / display flags.
#[hook(addr = 0x44CC20, conv = thiscall, name = "COnscreenTimer::ProcessForDisplay", expect = "E9 AB D3 11 01")]
pub fn process_for_display(this: GPtr<u8>) {
    if g::<u8>(HUD_WANTS_TO_DRAW).read() == 0 {
        return;
    }
    wr::<u8>(this, 0x150, 0);
    wr::<u8>(this, 0x38, 0);
    let id = rd::<u32>(this, 0);
    if id != 0 {
        let q = script_var(id).read() / 1000;
        sprintf2(this.byte_add(0xE), g(FMT_CLOCK), (q / 60) % 100, q % 60);
        wr::<u8>(this, 0x38, 1);
        wr::<u8>(this, 0x150, 1);
    }
    for i in 0..4 {
        let c = counter(this, i);
        wr::<u8>(c, 0x3E, 0);
        let id = rd::<u32>(c, 0);
        if id == 0 {
            continue;
        }
        let v = script_var(id).read();
        let t1 = if v < 0 { 0 } else { v };
        let text = c.byte_add(0x14);
        match rd::<u16>(c, 0x12) {
            0 | 1 => {
                sprintf1(text, g(FMT_D), t1);
            }
            2 => {
                let max = script_var(rd::<u32>(c, 4)).read();
                sprintf2(text, g(FMT_D_OF_D), t1, max);
            }
            _ => {}
        }
        wr::<u8>(c, 0x3E, 1);
        wr::<u8>(this, 0x150, 1);
    }
}

/// `COnscreenTimer::Process()` (0x44CD30): the clock only, not during replay playback or when paused.
#[hook(addr = 0x44CD30, conv = thiscall, name = "COnscreenTimer::Process", expect = "80 3D 88 30 A4 00 01")]
pub fn timer_process(this: GPtr<u8>) {
    if g::<u8>(REPLAY_MODE).read() == 1 || rd::<u8>(this, 0x151) != 0 {
        return;
    }
    timer_entry_process_call(this);
}

/// `COnscreenTimer::AddClock(uint32 varId, const char* key, eTimerDirection)` (stub 0x44CD50): only when
/// the clock is free; key copied with `strncpy(.., 10)` or cleared when null.
#[hook(addr = 0x44CD50, conv = thiscall, name = "COnscreenTimer::AddClock", expect = "E9 8B 16 12 01")]
pub fn add_clock(this: GPtr<u8>, var_id: u32, key: GPtr<u8>, direction: u32) {
    if rd::<u32>(this, 0) != 0 {
        return;
    }
    wr::<u32>(this, 0, var_id);
    wr::<u8>(this, 0x39, direction as u8);
    if key.is_null() {
        wr::<u8>(this, 4, 0);
    } else {
        strncpy(this.byte_add(4), key, 10);
    }
}

/// `COnscreenTimer::AddCounter(uint32 varId, eOnscreenCounter type, const char* key, uint16 counterIndex)`
/// (stub 0x44CDA0): only when that counter is free; max var 0, flash 1, colour 3. The `strncpy` length
/// (10) is computed by the protection from static data (`[0x1560488] + [0x53C25D] + 1` = 0x13B18B +
/// 0xFFEC4E7E + 1).
#[hook(addr = 0x44CDA0, conv = thiscall, name = "COnscreenTimer::AddCounter", expect = "E9 AB 4E 11 01")]
pub fn add_counter(this: GPtr<u8>, var_id: u32, ty: u32, key: GPtr<u8>, index: u32) {
    let c = this.byte_add(0x40 + (index as u16) as isize * 0x44);
    if rd::<u32>(c, 0) != 0 {
        return;
    }
    wr::<u32>(c, 0, var_id);
    if key.is_null() {
        wr::<u8>(c, 8, 0);
    } else {
        strncpy(c.byte_add(8), key, 10);
    }
    wr::<u32>(c, 4, 0);
    wr::<u16>(c, 0x12, ty as u16);
    wr::<u8>(c, 0x3F, 1);
    wr::<u8>(c, 0x40, 3);
}

/// `COnscreenTimer::ClearClock(uint32 varId)` (stub 0x44CE60).
#[hook(addr = 0x44CE60, conv = thiscall, name = "COnscreenTimer::ClearClock", expect = "E9 CB 88 11 01")]
pub fn clear_clock(this: GPtr<u8>, var_id: u32) {
    if rd::<u32>(this, 0) != var_id {
        return;
    }
    wr::<u32>(this, 0, 0);
    wr::<u8>(this, 4, 0);
    wr::<u8>(this, 0x38, 0);
    wr::<u8>(this, 0x39, 1);
}

/// `COnscreenTimer::ClearCounter(uint32 varId)` (stub 0x44CE80): every counter on that variable.
#[hook(addr = 0x44CE80, conv = thiscall, name = "COnscreenTimer::ClearCounter", expect = "E9 0B DF 11 01")]
pub fn clear_counter(this: GPtr<u8>, var_id: u32) {
    for i in 0..4 {
        let c = counter(this, i);
        if rd::<u32>(c, 0) == var_id {
            wr::<u32>(c, 0, 0);
            wr::<u32>(c, 4, 0);
            wr::<u8>(c, 8, 0);
            wr::<u16>(c, 0x12, 0);
            wr::<u8>(c, 0x3E, 0);
        }
    }
}

/// `COnscreenTimer::SetCounterFlashWhenFirstDisplayed(uint32 varId, bool)` (stub 0x44CEB0).
#[hook(addr = 0x44CEB0, conv = thiscall, name = "COnscreenTimer::SetCounterFlashWhenFirstDisplayed", expect = "E9 6B 1D 12 01")]
pub fn set_counter_flash_when_first_displayed(this: GPtr<u8>, var_id: u32, flash: u32) {
    for i in 0..4 {
        let c = counter(this, i);
        if rd::<u32>(c, 0) == var_id {
            wr::<u8>(c, 0x3F, flash as u8);
        }
    }
}

/// `COnscreenTimer::SetClockBeepCountdownSecs(uint32 varId, uint32 secs)` (stub 0x44CEE0). The original
/// walks four 0x40-byte entries from `this` (the clock, then counter-misaligned offsets 0x40/0x80/0xC0)
/// and writes +0x3C of each whose first dword matches.
#[hook(addr = 0x44CEE0, conv = thiscall, name = "COnscreenTimer::SetClockBeepCountdownSecs", expect = "E9 4B 52 11 01")]
pub fn set_clock_beep_countdown_secs(this: GPtr<u8>, var_id: u32, secs: u32) {
    for i in 0..4 {
        let e = this.byte_add(i * 0x40);
        if rd::<u32>(e, 0) == var_id {
            wr::<u32>(e, 0x3C, secs);
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_FOR_DISPLAY_COUNTER,
    HOOK_SET_COLOUR_ID,
    HOOK_TIMER_ENTRY_PROCESS,
    HOOK_TIMER_INIT,
    HOOK_PROCESS_FOR_DISPLAY,
    HOOK_TIMER_PROCESS,
    HOOK_ADD_CLOCK,
    HOOK_ADD_COUNTER,
    HOOK_CLEAR_CLOCK,
    HOOK_CLEAR_COUNTER,
    HOOK_SET_COUNTER_FLASH_WHEN_FIRST_DISPLAYED,
    HOOK_SET_CLOCK_BEEP_COUNTDOWN_SECS,
];
/// This module's globals.
pub const GLOBALS: &[GlobalDesc] = &[];
