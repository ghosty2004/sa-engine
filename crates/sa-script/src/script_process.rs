//! `CTheScripts::Process` (batch `mvpi-script-1`): the per-frame script update. Ported from the 1.0
//! US bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// `CReplay::Mode` (u8; 1 = playback)
const REPLAY_MODE: usize = 0xA43088;
/// `CTimer::ms_fTimeStepNonClipped`
const TIME_STEP_NON_CLIPPED: usize = 0xB7CB58;
/// `CTheScripts::CommandsExecuted` / `ScriptsUpdated` (u16)
const COMMANDS_EXECUTED: usize = 0xA447F4;
const SCRIPTS_UPDATED: usize = 0xA447F8;
/// `CTheScripts::UpsideDownCars`, `StuckCars`, `MissionCleanUp`
const UPSIDE_DOWN_CARS: usize = 0xA4892C;
const STUCK_CARS: usize = 0xA90AB0;
const MISSION_CLEANUP: usize = 0xA90850;
/// The byte counted down once per frame while above 0 (0xA444B0)
const COUNTDOWN_A444B0: usize = 0xA444B0;
/// `CTheScripts::UseTextCommands` (u8)
const USE_TEXT_COMMANDS: usize = 0xA44B67;
/// `CTheScripts::IntroTextLines` (0x60 of 0x44) and `NumberOfIntroTextLinesThisFrame` (u16)
const INTRO_TEXT_LINES: usize = 0xA913E8;
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// `CTheScripts::IntroRectangles` (0x80 of 0x3C) and `NumberOfIntroRectanglesThisFrame` (u16)
const INTRO_RECTANGLES: usize = 0xA92D68;
const NUM_INTRO_RECTANGLES: usize = 0xA44B5C;
/// The two mission timers (`LocalVariablesForCurrentMission` slots 32 / 33)
const MISSION_TIMER_A: usize = 0xA489E0;
const MISSION_TIMER_B: usize = 0xA489E4;
/// `CTheScripts::pActiveScripts`
const ACTIVE_SCRIPTS: usize = 0xA8B42C;
/// `CPools::ms_pPedPool`
const PED_POOL: usize = 0xB74490;

orig! {
    /// `CUpsideDownCarCheck::UpdateTimers()` (0x4655E0)
    fn upside_down_update_timers(c: GPtr<u8>) = thiscall 0x4655E0 => unported;
    /// `CStuckCarCheck::Process()` (0x465680)
    fn stuck_car_check_process(c: GPtr<u8>) = thiscall 0x465680 => unported;
    /// `CMissionCleanup::CheckIfCollisionHasLoadedForMissionObjects()` (0x4652D0)
    fn mission_cleanup_check_collision(c: GPtr<u8>) = thiscall 0x4652D0 => unported;
    /// `CTheScripts::DrawScriptSpheres()` (0x4810E0)
    fn draw_script_spheres() = cdecl 0x4810E0 => crate::script_tables::draw_script_spheres;
    /// `CTheScripts::ProcessAllSearchLights()` (0x4939F0)
    fn process_all_search_lights() = cdecl 0x4939F0 => unported;
    /// `CTheScripts::ProcessWaitingForScriptBrainArray()` (0x46CF00)
    fn process_waiting_for_script_brain_array() = cdecl 0x46CF00 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0, catalogue `FUN_007170c0`): returns `this`
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => rgba_ctor_host;
    /// `CLoadingScreen::NewChunkLoaded()` (0x590D00)
    fn loading_screen_new_chunk_loaded() = cdecl 0x590D00 => unported;
    /// `CRunningScript::Process()` (0x469F00)
    fn running_script_process(s: GPtr<u8>) -> i8 = thiscall 0x469F00 => crate::vm::process;
    /// `CPedIntelligence::RecordEventForScript(int32 event, int32 priority)` (0x602050)
    fn intel_record_event_for_script(i: GPtr<u8>, ev: i32, prio: i32) = thiscall 0x602050 => unported;
}

/// Phase B/C path of [`rgba_ctor`].
pub fn rgba_ctor_host(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> {
    for (i, v) in [r, g, b, a].into_iter().enumerate() {
        c.byte_add(i as isize).write(v as u8);
    }
    c
}

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `CRGBA(r, g, b, a)` into a stack temporary, its four bytes copied to `dst`.
fn put_rgba(dst: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) {
    let mut tmp = [0u8; 4];
    let c = rgba_ctor(GPtr::from_raw(tmp.as_mut_ptr()), r, g, b, a);
    for i in 0..4 {
        dst.byte_add(i).write(c.byte_add(i).read());
    }
}

/// `CTheScripts::Process()` (0x46A000, cdecl). Nothing during replay playback. The frame's
/// milliseconds `_ftol(timeStepNonClipped * 0.02 * 1000)` (x87); `CommandsExecuted` and
/// `ScriptsUpdated` = 0; the upside-down and stuck car checks, the mission cleanup's collision check,
/// script spheres, search lights, the waiting-for-brain array; the byte 0xA444B0 counts down. With
/// `UseTextCommands`: the 0x60 intro text lines reset (scale 0.48 / 1.12, colour (225, 225, 225, 255),
/// bytes +0xC..+0xF and +0x24 = 0, wrap 182, centre size 640, background (128, 128, 128, 128), +0x1C = 1,
/// drop colour (0, 0, 0, 255), +0x21 = 2, +0x22 / +0x23 = 0, +0x28 = 1, +0x2C..+0x38 = 0, +0x3C / +0x40
/// = -1, in the exe's store order, each colour through `CRGBA`), their count 0, the 0x80 intro
/// rectangles' +0 dword and +4 byte cleared, the rectangle count 0 (always), and `UseTextCommands` 1
/// becomes 0. Both mission timers grow by the milliseconds; `CLoadingScreen::NewChunkLoaded()`; every
/// active script while the list goes on and the next one is active: `ScriptsUpdated`++, its two timers
/// (+0xBC, +0xC0) grow, `CRunningScript::Process` (the next pointer read before the call);
/// `NewChunkLoaded()` again; then, from the last ped pool slot down, every used slot's ped created by a
/// mission (+0x484 = 2) gets `RecordEventForScript(0, 0)` on its intelligence (+0x47C).
#[hook(addr = 0x46a000, conv = cdecl, name = "CTheScripts::Process", expect = "A0 88 30 A4 00",
       unverified = "mvpi-script-1", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "the whole per-frame script update; every case nests the script VM")]
pub fn the_scripts_process() {
    if ga::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    let step = X87::from_f32(ga::<f32>(TIME_STEP_NON_CLIPPED).read())
        .fmul_m(f32::from_bits(0x3CA3_D70A))
        .fmul_m(1000.0);
    ga::<u16>(COMMANDS_EXECUTED).write(0);
    ga::<u16>(SCRIPTS_UPDATED).write(0);
    let ms = step.ftol() as u32;
    upside_down_update_timers(GPtr::from_addr(UPSIDE_DOWN_CARS));
    stuck_car_check_process(GPtr::from_addr(STUCK_CARS));
    mission_cleanup_check_collision(GPtr::from_addr(MISSION_CLEANUP));
    draw_script_spheres();
    process_all_search_lights();
    process_waiting_for_script_brain_array();
    let c = ga::<u8>(COUNTDOWN_A444B0);
    let v = c.read();
    if v > 0 {
        c.write(v - 1);
    }
    if ga::<u8>(USE_TEXT_COMMANDS).read() != 0 {
        for i in 0..0x60usize {
            let e = GPtr::<u8>::from_addr(INTRO_TEXT_LINES + i * 0x44);
            let w32 = |o: isize, v: u32| e.byte_add(o).cast::<u32>().write(v);
            let w8 = |o: isize, v: u8| e.byte_add(o).write(v);
            w32(0, 0x3EF5_C28F);
            w32(4, 0x3F8F_5C29);
            put_rgba(e.byte_add(8), 0xE1, 0xE1, 0xE1, 0xFF);
            w8(0xC, 0);
            w8(0x24, 0);
            w8(0xD, 0);
            w8(0xE, 0);
            w8(0xF, 0);
            w32(0x10, 0x4336_0000);
            w32(0x14, 0x4420_0000);
            put_rgba(e.byte_add(0x18), 0x80, 0x80, 0x80, 0x80);
            w8(0x1C, 1);
            put_rgba(e.byte_add(0x1D), 0, 0, 0, 0xFF);
            w8(0x21, 2);
            w8(0x22, 0);
            w8(0x23, 0);
            w32(0x28, 1);
            w32(0x2C, 0);
            w32(0x30, 0);
            w32(0x34, 0);
            w32(0x38, 0);
            w32(0x3C, 0xFFFF_FFFF);
            w32(0x40, 0xFFFF_FFFF);
        }
        ga::<u16>(NUM_INTRO_TEXT_LINES).write(0);
        for i in 0..0x80usize {
            let r = GPtr::<u8>::from_addr(INTRO_RECTANGLES + i * 0x3C);
            r.cast::<u32>().write(0);
            r.byte_add(4).write(0);
        }
        let one = ga::<u8>(USE_TEXT_COMMANDS).read() == 1;
        ga::<u16>(NUM_INTRO_RECTANGLES).write(0);
        if one {
            ga::<u8>(USE_TEXT_COMMANDS).write(0);
        }
    }
    let a = ga::<u32>(MISSION_TIMER_A).read().wrapping_add(ms);
    let b = ga::<u32>(MISSION_TIMER_B).read().wrapping_add(ms);
    let mut s = ga::<GPtr<u8>>(ACTIVE_SCRIPTS).read();
    ga::<u32>(MISSION_TIMER_A).write(a);
    ga::<u32>(MISSION_TIMER_B).write(b);
    loading_screen_new_chunk_loaded();
    while !s.is_null() {
        let n = ga::<u16>(SCRIPTS_UPDATED);
        n.write(n.read().wrapping_add(1));
        let t1 = s.byte_add(0xC0).cast::<u32>();
        let t0 = s.byte_add(0xBC).cast::<u32>();
        let (v1, v0) = (t1.read().wrapping_add(ms), t0.read().wrapping_add(ms));
        let next = s.cast::<GPtr<u8>>().read();
        t1.write(v1);
        t0.write(v0);
        running_script_process(s);
        s = next;
        if s.is_null() || s.byte_add(0xC4).read() == 0 {
            break;
        }
    }
    loading_screen_new_chunk_loaded();
    let pool = ga::<GPtr<u8>>(PED_POOL).read();
    let mut i = pool.byte_add(8).cast::<i32>().read();
    while i != 0 {
        let flags = pool.byte_add(4).cast::<u32>().read();
        let f = GPtr::<u8>::from_addr(flags.wrapping_add(i as u32).wrapping_sub(1) as usize).read();
        i -= 1;
        if f & 0x80 != 0 {
            continue;
        }
        let objs = pool.cast::<u32>().read();
        let ped = objs.wrapping_add((i as u32).wrapping_mul(0x7C4));
        if ped == 0 {
            continue;
        }
        let ped = GPtr::<u8>::from_addr(ped as usize);
        if ped.byte_add(0x484).read() == 2 {
            intel_record_event_for_script(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 0, 0);
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_THE_SCRIPTS_PROCESS];
