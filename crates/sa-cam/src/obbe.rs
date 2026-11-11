//! PORT-CAM-3: the five "Obbe cinema camera" selectors `CCamera::ProcessObbeCinemaCamera{Car, Train,
//! Heli, Plane, Boat}` (0x5267C0 / 0x526950 / 0x526AE0 / 0x526C80 / 0x526E20). One body with a parameter
//! row each (`docs/re/specs/CAM/CCamera.md` 0x5267C0), ported from the 1.0 US bytes (gta-reversed: `assert(0)`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ACTIVE_CAM, CAMS, CCAM_SIZE, g, rd, wr};

orig! {
    /// `CCamera::IsItTimeForNewcam(int32 seq, int32 startTimeMs)` (0x51D770, AL)
    pub fn is_it_time_for_newcam(this: GPtr<u8>, seq: i32, start: u32) -> u8 = thiscall 0x51D770 => unported;
    /// `CCamera::TryToStartNewCamMode(int32 seq)` (0x51E560, AL)
    pub fn try_to_start_new_cam_mode(this: GPtr<u8>, seq: i32) -> u8 = thiscall 0x51E560 => unported;
    /// "active camera under water" (0x50B830, cdecl, AL)
    pub fn active_cam_under_water() -> u8 = cdecl 0x50B830 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool, bool, bool)` (0x588BE0)
    pub fn hud_set_help_message(msg: GPtr<u8>, quick: u32, permanent: u32, add_to_brief: u32) = cdecl 0x588BE0 => unported;
}

const DID_CINEMA: usize = 0xB6EC2D;
const BYTE_B6EC34: usize = 0xB6EC34;
const MESSAGE_COUNT: usize = 0x8CC381;
const STEP: usize = 0x8CC471;
const TIME_MS: usize = 0xB7CB84;

/// One selector's parameters (spec table).
struct Row {
    idx: usize,
    last: usize,
    table: usize,
    n: i32,
    under_water: bool,
    /// car / train outcome (`last = now` always, failure tries `table[N]`); else heli / plane / boat
    car_like: bool,
}

const CAR: Row = Row {
    idx: 0x8CCEEC,
    last: 0xB70120,
    table: 0x8CC828,
    n: 11,
    under_water: false,
    car_like: true,
};
const TRAIN: Row = Row {
    idx: 0x8CCEF0,
    last: 0xB70124,
    table: 0x8CC858,
    n: 6,
    under_water: false,
    car_like: true,
};
const HELI: Row = Row {
    idx: 0x8CCEF4,
    last: 0xB70128,
    table: 0x8CC874,
    n: 7,
    under_water: true,
    car_like: false,
};
const PLANE: Row = Row {
    idx: 0x8CCEF8,
    last: 0xB7012C,
    table: 0x8CC894,
    n: 6,
    under_water: true,
    car_like: false,
};
const BOAT: Row = Row {
    idx: 0x8CCEFC,
    last: 0xB70130,
    table: 0x8CC8B0,
    n: 3,
    under_water: false,
    car_like: false,
};

fn seq(r: &Row, i: i32) -> i32 {
    g::<i32>(r.table + 4 * i as usize).read()
}

/// `idx = (idx + step) % N` (signed `idiv`), `< 0` -> N - 1, `> N - 1` -> 0 (dead).
fn next(r: &Row) -> i32 {
    let s = i32::from(g::<i8>(STEP).read());
    let v = s.wrapping_add(g::<i32>(r.idx).read()) % r.n;
    g::<i32>(r.idx).write(v);
    let v = if v < 0 {
        r.n - 1
    } else if v > r.n - 1 {
        0
    } else {
        return v;
    };
    g::<i32>(r.idx).write(v);
    v
}

fn finish(this: GPtr<u8>, idx: i32) {
    wr::<i32>(this, 0xC34, idx);
    g::<u8>(DID_CINEMA).write(1);
}

fn obbe(this: GPtr<u8>, r: &Row) {
    let now = || g::<u32>(TIME_MS).read();
    let mut new_cam = false;
    if g::<u8>(DID_CINEMA).read() == 0 {
        g::<i32>(r.idx).write(-1);
        if g::<i8>(MESSAGE_COUNT).read() > 0 && rd::<u8>(this, 0xCF0) == 0 {
            let c = g::<i8>(MESSAGE_COUNT).read().wrapping_sub(1);
            g::<i8>(MESSAGE_COUNT).write(c);
            let t = text_get(GPtr::from_addr(0xC1B340), GPtr::from_addr(0x863248));
            hud_set_help_message(t, 1, 0, 0);
        }
        g::<u8>(BYTE_B6EC34).write(1);
        if g::<u8>(DID_CINEMA).read() == 0 {
            new_cam = true;
        }
    }
    if !new_cam {
        new_cam = (r.under_water && active_cam_under_water() != 0)
            || is_it_time_for_newcam(this, seq(r, g::<i32>(r.idx).read()), g::<u32>(r.last).read()) != 0;
    }
    if !new_cam {
        finish(this, g::<i32>(r.idx).read());
        return;
    }
    let i = next(r);
    let mut tries = 0;
    let mut ok = try_to_start_new_cam_mode(this, seq(r, i)) != 0;
    let first = ok;
    while !ok {
        if tries > r.n {
            break;
        }
        let i = next(r);
        tries += 1;
        ok = try_to_start_new_cam_mode(this, seq(r, i)) != 0;
    }
    if r.car_like {
        g::<u32>(r.last).write(now());
        if tries >= r.n {
            g::<i32>(r.idx).write(r.n);
            try_to_start_new_cam_mode(this, seq(r, r.n));
        }
        finish(this, g::<i32>(r.idx).read());
        return;
    }
    if first || tries < r.n {
        let t = now();
        finish(this, g::<i32>(r.idx).read());
        g::<u32>(r.last).write(t);
        return;
    }
    g::<i32>(r.idx).write(r.n);
    let i = isize::from(rd::<u8>(this, ACTIVE_CAM));
    if rd::<u16>(this, CAMS + i * CCAM_SIZE + 0xC) == 0x12 {
        finish(this, r.n);
        return;
    }
    try_to_start_new_cam_mode(this, 0x1D);
    g::<u32>(r.last).write(now());
    finish(this, g::<i32>(r.idx).read());
}

/// `CCamera::ProcessObbeCinemaCameraCar()` (0x5267C0)
#[hook(addr = 0x5267C0, conv = thiscall, name = "CCamera::ProcessObbeCinemaCameraCar", expect = "A0 2D EC B6 00 53 56 57 33 FF", this = "CCamera")]
pub fn obbe_car(this: GPtr<u8>) {
    obbe(this, &CAR)
}

/// `CCamera::ProcessObbeCinemaCameraTrain()` (0x526950)
#[hook(addr = 0x526950, conv = thiscall, name = "CCamera::ProcessObbeCinemaCameraTrain", expect = "A0 2D EC B6 00 53 56 57 33 FF", this = "CCamera")]
pub fn obbe_train(this: GPtr<u8>) {
    obbe(this, &TRAIN)
}

/// `CCamera::ProcessObbeCinemaCameraHeli()` (0x526AE0)
#[hook(addr = 0x526AE0, conv = thiscall, name = "CCamera::ProcessObbeCinemaCameraHeli", expect = "A0 2D EC B6 00 53 56 57 33 FF", this = "CCamera")]
pub fn obbe_heli(this: GPtr<u8>) {
    obbe(this, &HELI)
}

/// `CCamera::ProcessObbeCinemaCameraPlane()` (0x526C80; byte 0xB6EC34 is set before the message test
/// in this copy, the same effect)
#[hook(addr = 0x526C80, conv = thiscall, name = "CCamera::ProcessObbeCinemaCameraPlane", expect = "A0 2D EC B6 00 53 56 57 33 FF", this = "CCamera")]
pub fn obbe_plane(this: GPtr<u8>) {
    obbe(this, &PLANE)
}

/// `CCamera::ProcessObbeCinemaCameraBoat()` (0x526E20)
#[hook(addr = 0x526E20, conv = thiscall, name = "CCamera::ProcessObbeCinemaCameraBoat", expect = "A0 2D EC B6 00 53 56 57 33 FF", this = "CCamera")]
pub fn obbe_boat(this: GPtr<u8>) {
    obbe(this, &BOAT)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_OBBE_CAR,
    HOOK_OBBE_TRAIN,
    HOOK_OBBE_HELI,
    HOOK_OBBE_PLANE,
    HOOK_OBBE_BOAT,
];

/// This module's globals (none).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
