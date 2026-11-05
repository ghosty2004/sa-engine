//! Rust ports of original functions. Pure parts are plain Rust (host-tested); the `*_hook` entry points
//! have the original calling convention and are what the hook table installs.
//!
//! Sources: gta-reversed (`re/gta-reversed` @ 20d8d2a) and the 1.0 US disassembly; constants were read
//! from the exe (addresses noted next to them).

use crate::table::{self, Id};

/// `CGeneral::LimitRadianAngle` (0x53CB50), gta-reversed General.cpp.
pub mod general {
    /// 0x858FE8 / 0x863AC0
    pub const CLAMP: f32 = 25.0;
    /// 0x858CB8 (bits 0x40490FDB)
    pub const PI: f32 = std::f32::consts::PI;
    /// 0x858CBC (bits 0x40C90FDB)
    pub const TWO_PI: f32 = 2.0 * std::f32::consts::PI;

    /// Clamps to [-25, 25], then wraps into [-PI, PI). NaN passes through (the original's x87 compares
    /// are unordered for NaN and fall through both loops, as do these).
    pub fn limit_radian_angle(angle: f32) -> f32 {
        // Equivalent to the original's `< -25` / `>= 25` tests (equal at ±25, NaN passes through).
        let mut a = angle.clamp(-CLAMP, CLAMP);
        while a >= PI {
            a -= TWO_PI;
        }
        while a < -PI {
            a += TWO_PI;
        }
        a
    }
}

/// `CGameLogic::IsCoopGameGoingOn` (0x441390, a `.HOODLUM` stub → 0x156CF30):
/// `FindPlayerPed(PLAYER1) && FindPlayerPed(PLAYER2)`, i.e. both `CWorld::Players[i].m_pPed` set.
pub mod game_logic {
    /// `CWorld::Players` (0xB7CD98), `sizeof(CPlayerInfo)` = 0x190, `m_pPed` at +0.
    pub const PLAYERS: u32 = 0xB7CD98;
    pub const PLAYER_INFO_SIZE: u32 = 0x190;

    /// Pure part: the original returns 1 or 0 in the whole of EAX.
    pub fn is_coop_game_going_on(ped1: u32, ped2: u32) -> u32 {
        (ped1 != 0 && ped2 != 0) as u32
    }
}

/// `CTimer` globals (gta-reversed Timer.h).
pub mod timer {
    pub const FRAME_COUNTER: u32 = 0xB7CB4C;
    pub const TIME_IN_MS: u32 = 0xB7CB84;
}

/// `gGameState` (0xC8D4C0): 7 = main menu reached, 9 = playing (gta-reversed `eGameState`).
pub const GAME_STATE: u32 = 0xC8D4C0;

// ------------------------------------------------------------------ hook entry points -------------

/// `CTimer::Update` (cdecl, no args): Rust passthrough to the original.
pub extern "C" fn timer_update_hook() {
    table::count(Id::TimerUpdate);
    // SAFETY: `original` is the trampoline built for this exact function signature.
    unsafe {
        let f: extern "C" fn() = std::mem::transmute(table::original(Id::TimerUpdate) as usize);
        f()
    }
}

/// `CGeneral::LimitRadianAngle` (cdecl `float(float)`, result in ST0).
pub extern "C" fn limit_radian_angle_hook(angle: f32) -> f32 {
    table::count(Id::LimitRadianAngle);
    general::limit_radian_angle(angle)
}

/// `CGameLogic::IsCoopGameGoingOn` (cdecl `bool()`, the original sets all of EAX).
pub extern "C" fn is_coop_game_going_on_hook() -> u32 {
    table::count(Id::IsCoopGameGoingOn);
    #[cfg(windows)]
    {
        use game_logic::*;
        // SAFETY: fixed .data addresses of the 1.0 US exe (checked at attach).
        let (p1, p2) = unsafe {
            (
                std::ptr::read_volatile(PLAYERS as usize as *const u32),
                std::ptr::read_volatile((PLAYERS + PLAYER_INFO_SIZE) as usize as *const u32),
            )
        };
        game_logic::is_coop_game_going_on(p1, p2)
    }
    #[cfg(not(windows))]
    0
}

#[cfg(test)]
mod tests {
    use super::general::*;

    #[test]
    fn constants_match_the_exe() {
        // Bit patterns read from gta_sa.exe 1.0 US .rdata.
        assert_eq!(PI.to_bits(), 0x40490FDB);
        assert_eq!(TWO_PI.to_bits(), 0x40C90FDB);
        assert_eq!((-PI).to_bits(), 0xC0490FDB);
        assert_eq!(CLAMP.to_bits(), 0x41C80000);
    }

    #[test]
    fn limit_radian_angle_cases() {
        assert_eq!(limit_radian_angle(0.0), 0.0);
        assert_eq!(limit_radian_angle(1.0), 1.0);
        assert_eq!(limit_radian_angle(PI), PI - TWO_PI);
        assert_eq!(limit_radian_angle(-PI), -PI);
        assert_eq!(limit_radian_angle(100.0), limit_radian_angle(25.0));
        assert_eq!(limit_radian_angle(-100.0), limit_radian_angle(-25.0));
        assert_eq!(limit_radian_angle(f32::INFINITY), limit_radian_angle(25.0));
        assert_eq!(limit_radian_angle(f32::NEG_INFINITY), limit_radian_angle(-25.0));
        assert!(limit_radian_angle(f32::NAN).is_nan());
        for i in -2500..2500 {
            let r = limit_radian_angle(i as f32 / 100.0);
            assert!((-PI..PI).contains(&r), "{i} -> {r}");
        }
    }

    #[test]
    fn coop() {
        use super::game_logic::is_coop_game_going_on as c;
        assert_eq!((c(0, 0), c(1, 0), c(0, 1), c(0x1234, 0x5678)), (0, 0, 0, 1));
    }
}
