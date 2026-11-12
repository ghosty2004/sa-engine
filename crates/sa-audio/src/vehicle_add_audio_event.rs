//! `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420, batch `writero-audio-1`): the
//! vehicle audio entity's one-shot event dispatcher. Ported from the 1.0 US bytes (the SEH handler push
//! sits in the SecuROM fragment 0x40727C, the `mov ecx, AESoundManager` before the shared
//! `RequestNewSound` in 0x40A125); gta-reversed is a reading aid only (its volumes, flags and the
//! wave-hit volume differ from the exe). Unverified (docs/verify/HOWTO.md).
//!
//! The entry bytes (`push -1; jmp rel32`) are read as data by the no-CD protection (`adc ecx,
//! [0x4F6423]` at 0x401931): sa-hook keycells reader, hazard `entry-protection-read`.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{GPtr, g, rd, wr, x};

/// `AEAudioHardware`, `AESoundManager`
const HW: usize = 0xB5F8B8;
const SM: usize = 0xB62CB0;
/// The per-event volume table pointer (`[0xBD00F8]`, one int8 per event).
const EVENT_VOLUMES: usize = 0xBD00F8;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// 1.0f as bits
const ONE: u32 = 0x3F80_0000;
/// 2.0f / 1.5f as bits
const TWO: u32 = 0x4000_0000;
const ONE_HALF: u32 = 0x3FC0_0000;
/// 0.06f / 0.05f as bits (`CAESound` frequency variance)
const VAR_006: u32 = 0x3D75_C290;
const VAR_005: u32 = 0x3D4C_CCCD;

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bankId, int16 slot)` (0x4D88C0)
    fn is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntity(int16 event, CAEAudioEntity*)` (0x4EF570);
    /// callers test AX only
    fn are_sounds_playing(sm: GPtr<u8>, event: u32, entity: GPtr<u8>) -> u32 = thiscall 0x4EF570 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange(int, int)` (0x4D9C10)
    fn ae_random_int(min: i32, max: i32) -> i32 = cdecl 0x4D9C10 => crate::utility::get_random_number_in_range_int;
    /// `CAESound::Initialise(...)` (0x4EFE50, 14 argument dwords, `ret 0x38`)
    fn sound_initialise(sound: GPtr<u8>, slot: u32, id: u32, entity: u32, px: u32, py: u32, pz: u32, volume: u32, max_distance: u32, speed: u32, time_scale: u32, ignored_cycles: u32, flags: u32, speed_variance: u32, play_pos: u32) = thiscall 0x4EFE50 => crate::sound::initialise;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
    /// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820)
    fn sound_register_with_physical_entity(sound: GPtr<u8>, entity: GPtr<u8>) = thiscall 0x4EF820 => crate::sound::register_with_physical_entity;
    /// `CAESound::~CAESound()` (0x4EF660): drops the physical-entity reference (+8) when set
    fn sound_dtor(sound: GPtr<u8>) = thiscall 0x4EF660 => unported;
}

/// A `.rdata` / `.data` float.
fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `(float)(int8)[[0xBD00F8] + event]` (`movsx; fild; fstp dword`), exact in f32.
fn event_volume(event: u32) -> f32 {
    let table = g::<GPtr<u8>>(EVENT_VOLUMES).read();
    f32::from(rd::<i8>(table, event as isize))
}

/// The vehicle's (`this + 4`) position: its matrix's +0x30 when it has a matrix (+0x14), else the
/// placement (+4), as three dwords.
fn veh_pos(this: GPtr<u8>) -> [u32; 3] {
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    let m = rd::<u32>(veh, 0x14);
    let p: GPtr<u8> = if m != 0 { g(m as usize + 0x30) } else { veh.byte_add(4) };
    [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
}

/// `Initialise(slot, sfx, this, the vehicle's position, volume, 1.0, 1.0, 1.0, 0, 0, 0, 0)` on `snd`.
fn init(snd: GPtr<u8>, this: GPtr<u8>, slot: u32, sfx: u32, volume: f32) {
    let p = veh_pos(this);
    sound_initialise(snd, slot, sfx, this.addr() as u32, p[0], p[1], p[2], volume.to_bits(), ONE, ONE, ONE, 0, 0, 0, 0);
}

/// The ped-event volume: bikes (audio type +0x80 == 1) subtract `[0x858B3C]`, BMX (2) `[0x858B44]`
/// (`fld; fsub dword; fstp dword`).
fn ped_event_volume(this: GPtr<u8>, event: u32) -> f32 {
    let v = event_volume(event);
    match rd::<i8>(this, 0x80) {
        1 => x(v).fsub_m(gf(0x858B3C)).to_f32(),
        2 => x(v).fsub_m(gf(0x858B44)).to_f32(),
        _ => v,
    }
}

/// The shared tail of the ped-collision sounds: +0x1C speed, +0x20 = 0.06, +0x18 = 1.5, flags word +0x56,
/// play-time word +0x5C, event +0xC, requested.
fn ped_sound_tail(snd: GPtr<u8>, speed: u32, flags: u16, play_time: u16, event: u32) {
    wr::<u32>(snd, 0x1C, speed);
    wr::<u32>(snd, 0x20, VAR_006);
    wr::<u32>(snd, 0x18, ONE_HALF);
    wr::<u16>(snd, 0x56, flags);
    wr::<u16>(snd, 0x5C, play_time);
    wr::<u32>(snd, 0xC, event);
    sm_request_new_sound(g(SM), snd);
}

/// The suspension sounds (events 0x6B..0x6F): volume `f32(event volume + [extra])`; nothing while a
/// sound of this event plays for this entity; else slot 0x13 `sfx`, +0x20 = 0.05, +0x18 = 2.0 and (for
/// `start_pct`) flags word +0x56 = 0x20 with play time +0x5C = 0, event +0xC, requested.
fn suspension(snd: GPtr<u8>, this: GPtr<u8>, event: u32, extra: usize, sfx: u32, start_pct: bool) {
    let vol = x(event_volume(event)).fadd_m(gf(extra)).to_f32();
    if are_sounds_playing(g(SM), event, this) as u16 != 0 {
        return;
    }
    init(snd, this, 0x13, sfx, vol);
    if start_pct {
        wr::<u16>(snd, 0x56, 0x20);
        wr::<u16>(snd, 0x5C, 0);
    }
    wr::<u32>(snd, 0x20, VAR_005);
    wr::<u32>(snd, 0x18, TWO);
    wr::<u32>(snd, 0xC, event);
    sm_request_new_sound(g(SM), snd);
}

/// The sounds registered with the vehicle (nitro, trailer, harvester): +0x1C `speed` (`None` = left at
/// `Initialise`'s 1.0), +0x20 `var`, +0x18 = 2.0, flags word +0x56 = 0x80, registered with the vehicle
/// (`this + 4`), event +0xC, requested.
fn registered(snd: GPtr<u8>, this: GPtr<u8>, speed: Option<u32>, var: u32, event: u32) {
    if let Some(s) = speed {
        wr::<u32>(snd, 0x1C, s);
    }
    wr::<u32>(snd, 0x20, var);
    wr::<u32>(snd, 0x18, TWO);
    wr::<u16>(snd, 0x56, 0x80);
    sound_register_with_physical_entity(snd, g(rd::<u32>(this, 4) as usize));
    wr::<u32>(snd, 0xC, event);
    sm_request_new_sound(g(SM), snd);
}

/// A stack `CAESound` (0x74 bytes at entry ESP - 0x80) whose words start as the original's stack
/// residue (read through the hook's `peek` frame; a direct Rust call gets zeros). Only +8 (the physical
/// entity) is cleared before use.
fn stack_sound() -> [u32; 0x74 / 4] {
    let mut buf = [0u32; 0x74 / 4];
    for (i, w) in buf.iter_mut().enumerate() {
        *w = sa_core::abi::peek_stack(0x4F6420, -0x80 + 4 * i as i32).unwrap_or(0);
    }
    buf
}

/// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents event, float p1)` (0x4F6420, thiscall, `ret 8`).
///
/// A stack `CAESound` (only +8 cleared; `~CAESound` on every path). Nothing unless banks 0x8A (slot
/// 0x13), 0x27 (slot 2) and 0x1B (slot 3) are loaded and `event - 0x4E` ≤ 0x2C (unsigned, whole dword).
/// Every sound below is `Initialise(slot, sfx, this, the vehicle's position, volume, 1.0 x3, 0 x4)` then
/// the listed fields, then `RequestNewSound`; "vol(e)" is `(float)(int8)[[0xBD00F8] + e]`. Through the
/// exe's byte jump table 0x4F751C / 0x4F74C4:
/// - 0x4E..0x53 (door / bonnet / boot open): slot 0x13, vol(0x4E), sfx by door type (int8 +0x98) 0 → 0x27,
///   1 → 0x29, 2 → 0x28, 3 → 0x2B, 4 → 0x2A, else 0x28; +0x20 = 0.02.
/// - 0x54..0x59 (close): vol(0x54), 0 → 0x20, 1 → 0x22, 2 → 0x21, 3 → 0x24, 4 → 0x23, else 0x21; +0x20 = 0.02.
/// - 0x5A tyre burst: slot 3 sfx 0; 0x5B windscreen: slot 2 sfx 0x44; 0x5C light smash: slot 2 sfx 0x35
///   (their own vol).
/// - 0x63 boat wave hit: nothing before `+0xCC + [0x8CBD7C]` ms (unsigned) or when `p1 < [0x8CBD74]`;
///   `t = p1 / [0x8CBD78]` (stored as f32; nothing when the full-width quotient is below 1.0 and t < 0.0);
///   t ≥ 1.0 or NaN → 1.0, else nothing when t ≤ `[0x858C14]`; nothing when `CWeather::UnderWaterness`
///   ≥ 0.5 or unordered; volume `f32(AudioLog10(t) * 20 + [0xB6B9C8] + vol(0x63))` (x87, full-width
///   AudioLog10), slot 2 sfx 0x42; then +0xCC = the current time.
/// - 0x64 train clack: nothing when the dummy slot (word +0xE0) is -1 or bank (word +0xDC) is not loaded
///   in it (the slot argument dword keeps the upper half of the volume-table pointer, as EAX); else sfx
///   `GetRandomNumberInRange(3, 5)` in that slot, +0x1C = 1.0, +0x20 = 0.1, +0x18 = 2.0.
/// - 0x66 nitro: bank 0x8F in slot 5 and no 0x66 sound of this entity: slot 5 sfx 0x53, +0x1C = 0.89,
///   +0x20 = 0.03, registered (see [`registered`]).
/// - 0x67 engine start: slot 0x13 sfx 0x25, +0x20 = 0.05, +0x18 = 2.0, event 0x67.
/// - 0x6B..0x6F suspension: see [`suspension`] (0x6B sfx 0x26 + `[0x8CC1B8]`, 0x6C 0x1F + `[0x8CC1BC]`,
///   0x6D 0x2C + `[0xB6BAC4]` start-percentage, 0x6E 0x2C + `[0xB6BAC0]` start-percentage, 0x6F 0x26 +
///   `[0x8CC1B4]`).
/// - 0x71 / 0x72 trailer attach / detach: no 0x71 and no 0x72 sound of this entity and bank 0x8F in slot 5:
///   slot 5 sfx 0x46, +0x1C = 0.56 / 0.5, +0x20 = 0.05, registered.
/// - 0x73 harvester: no 0x73 sound and bank 0x45 in slot 0x28: slot 0x28 sfx 4, +0x20 = 0.05, registered.
/// - 0x77 ped crunch, 0x78 drive over, 0x79 knock down, 0x7A bounce (volume see [`ped_event_volume`];
///   bank 0x27 in slot 2): 0x77 sfx 0x1D, speed 0.8 / play time 40 while a 0x77 sound plays, else 0.94 / 0;
///   0x78 sfx `GetRandomNumberInRange(0x2F, 0x31)`, speed 1.0, event **0x79** (original bug kept);
///   0x79 / 0x7A: sfx 0x21 / 0x1F at `volume - [0x858CA0]`, then `GetRandomNumberInRange(0x14, 0x1C)` at
///   the volume with play time 55 / 25. All with flags 0x20 (see [`ped_sound_tail`]).
///
/// The words of the stack sound `Initialise` does not write keep the original's stack residue
/// (`peek`); `RequestNewSound` copies them.
#[hook(addr = 0x4F6420, conv = thiscall, name = "CAEVehicleAudioEntity::AddAudioEvent-0", expect = "6A FF E9 55 0E F1 FF",
       peek = 0x80, unverified = "writero-audio-1", route = "L2", hazards = "uninit-locals,entry-protection-read",
       recs = "driving")]
pub fn vehicle_add_audio_event(this: GPtr<u8>, event: u32, p1: u32) {
    let mut buf = stack_sound();
    let snd = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(snd, 8, 0);
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) != 0
        && is_sound_bank_loaded(g(HW), 0x27, 2) != 0
        && is_sound_bank_loaded(g(HW), 0x1B, 3) != 0
        && event.wrapping_sub(0x4E) <= 0x2C
    {
        dispatch(snd, this, event, p1);
    }
    sound_dtor(snd);
}

fn dispatch(snd: GPtr<u8>, this: GPtr<u8>, event: u32, p1: u32) {
    match event {
        0x4E..=0x53 => {
            let sfx = match rd::<i8>(this, 0x98) {
                0 => 0x27,
                1 => 0x29,
                3 => 0x2B,
                4 => 0x2A,
                _ => 0x28,
            };
            init(snd, this, 0x13, sfx, event_volume(0x4E));
            wr::<u32>(snd, 0x20, 0x3CA3_D70A);
            sm_request_new_sound(g(SM), snd);
        }
        0x54..=0x59 => {
            let sfx = match rd::<i8>(this, 0x98) {
                0 => 0x20,
                1 => 0x22,
                3 => 0x24,
                4 => 0x23,
                _ => 0x21,
            };
            init(snd, this, 0x13, sfx, event_volume(0x54));
            wr::<u32>(snd, 0x20, 0x3CA3_D70A);
            sm_request_new_sound(g(SM), snd);
        }
        0x5A..=0x5C => {
            let (slot, sfx) = match event {
                0x5A => (3, 0),
                0x5B => (2, 0x44),
                _ => (2, 0x35),
            };
            init(snd, this, slot, sfx, event_volume(event));
            sm_request_new_sound(g(SM), snd);
        }
        0x63 => boat_hit_wave(snd, this, f32::from_bits(p1)),
        0x64 => {
            let vol = event_volume(0x64);
            let slot = rd::<u16>(this, 0xE0);
            if slot == 0xFFFF {
                return;
            }
            let table = g::<u32>(EVENT_VOLUMES).read();
            let bank = rd::<u16>(this, 0xDC) as u32;
            if is_sound_bank_loaded(g(HW), bank, (table & 0xFFFF_0000) | slot as u32) == 0 {
                return;
            }
            let sfx = ae_random_int(3, 5);
            init(snd, this, slot as u32, sfx as u32, vol);
            wr::<u32>(snd, 0x1C, ONE);
            wr::<u32>(snd, 0x20, 0x3DCC_CCCD);
            wr::<u32>(snd, 0x18, TWO);
            sm_request_new_sound(g(SM), snd);
        }
        0x66 => {
            if is_sound_bank_loaded(g(HW), 0x8F, 5) == 0 {
                return;
            }
            let vol = event_volume(0x66);
            if are_sounds_playing(g(SM), 0x66, this) as u16 != 0 {
                return;
            }
            init(snd, this, 5, 0x53, vol);
            registered(snd, this, Some(0x3F63_D70A), 0x3CF5_C28F, 0x66);
        }
        0x67 => {
            init(snd, this, 0x13, 0x25, event_volume(0x67));
            wr::<u32>(snd, 0x20, VAR_005);
            wr::<u32>(snd, 0x18, TWO);
            wr::<u32>(snd, 0xC, 0x67);
            sm_request_new_sound(g(SM), snd);
        }
        0x6B => suspension(snd, this, 0x6B, 0x8CC1B8, 0x26, false),
        0x6C => suspension(snd, this, 0x6C, 0x8CC1BC, 0x1F, false),
        0x6D => suspension(snd, this, 0x6D, 0xB6BAC4, 0x2C, true),
        0x6E => suspension(snd, this, 0x6E, 0xB6BAC0, 0x2C, true),
        0x6F => suspension(snd, this, 0x6F, 0x8CC1B4, 0x26, false),
        0x71 | 0x72 => {
            let vol = event_volume(event);
            if are_sounds_playing(g(SM), 0x71, this) as u16 != 0
                || are_sounds_playing(g(SM), 0x72, this) as u16 != 0
                || is_sound_bank_loaded(g(HW), 0x8F, 5) == 0
            {
                return;
            }
            init(snd, this, 5, 0x46, vol);
            let speed = if event == 0x71 { 0x3F0F_5C29 } else { 0x3F00_0000 };
            registered(snd, this, Some(speed), VAR_005, event);
        }
        0x73 => {
            let vol = event_volume(0x73);
            if are_sounds_playing(g(SM), 0x73, this) as u16 != 0 || is_sound_bank_loaded(g(HW), 0x45, 0x28) == 0 {
                return;
            }
            init(snd, this, 0x28, 4, vol);
            registered(snd, this, None, VAR_005, 0x73);
        }
        0x77 => {
            let vol = ped_event_volume(this, 0x77);
            if is_sound_bank_loaded(g(HW), 0x27, 2) == 0 {
                return;
            }
            let (play_time, speed) = if are_sounds_playing(g(SM), 0x77, this) as u16 != 0 {
                (0x28, 0x3F4C_CCCD)
            } else {
                (0, 0x3F70_A3D7)
            };
            init(snd, this, 2, 0x1D, vol);
            ped_sound_tail(snd, speed, 0x20, play_time, 0x77);
        }
        0x78 => {
            let vol = ped_event_volume(this, 0x78);
            if is_sound_bank_loaded(g(HW), 0x27, 2) == 0 {
                return;
            }
            let sfx = ae_random_int(0x2F, 0x31);
            init(snd, this, 2, sfx as u32, vol);
            ped_sound_tail(snd, ONE, 0x20, 0, 0x79);
        }
        0x79 | 0x7A => {
            let vol = ped_event_volume(this, event);
            if is_sound_bank_loaded(g(HW), 0x27, 2) == 0 {
                return;
            }
            let (first, play_time) = if event == 0x79 { (0x21, 0x37) } else { (0x1F, 0x19) };
            let quiet = x(vol).fsub_m(gf(0x858CA0)).to_f32();
            init(snd, this, 2, first, quiet);
            ped_sound_tail(snd, ONE, 0x20, 0, event);
            let sfx = ae_random_int(0x14, 0x1C);
            init(snd, this, 2, sfx as u32, vol);
            ped_sound_tail(snd, ONE, 0x20, play_time, event);
        }
        _ => {}
    }
}

/// Event 0x63 (`AE_BOAT_HIT_WAVE`, 0x4F66B1); see [`vehicle_add_audio_event`].
fn boat_hit_wave(snd: GPtr<u8>, this: GPtr<u8>, p1: f32) {
    let vol = event_volume(0x63);
    let due = rd::<u32>(this, 0xCC).wrapping_add(g::<u32>(0x8CBD7C).read());
    if g::<u32>(TIME_MS).read() < due {
        return;
    }
    // fld p1; fcomp [0x8CBD74]; test ah, 5; jnp → out (C0 alone: below)
    if !x(p1).fcom(x(gf(0x8CBD74))).test_parity(5) {
        return;
    }
    let q = x(p1).fdiv_m(gf(0x8CBD78));
    let mut t = q.to_f32();
    // fcomp 1.0 on the full-width quotient; jp skips the 0.0 test when not below
    if !q.fcom(x(gf(0x858624))).test_parity(5) && !x(gf(0x858B50)).fcom(x(t)).test_any(0x41) {
        return;
    }
    if x(t).fcom(x(gf(0x858624))).test_parity(5) {
        t = f32::from_bits(ONE);
    } else if x(t).fcom(x(gf(0x858C14))).test_any(0x41) {
        return;
    }
    if x(gf(0xC8132C)).fcom(x(gf(0x858B8C))).test_parity(5) {
        return;
    }
    let v = crate::utility::call_audio_log10(t)
        .fmul_m(gf(0x858BA4))
        .fadd_m(gf(0xB6B9C8))
        .fadd_m(vol)
        .to_f32();
    init(snd, this, 2, 0x42, v);
    sm_request_new_sound(g(SM), snd);
    wr::<u32>(this, 0xCC, g::<u32>(TIME_MS).read());
}

pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_ADD_AUDIO_EVENT];
