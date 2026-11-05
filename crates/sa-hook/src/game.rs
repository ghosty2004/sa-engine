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

    /// Clamps to [-25, 25], then wraps into [-PI, PI). The port lives in `sa-math` (x87-faithful,
    /// L1-verified); the hook returns its exact ST0 value.
    pub use sa_math::general::limit_radian_angle;
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

/// Runs a Rust float computation under the default MXCSR (0x1F80) and restores the game's MXCSR
/// (0x9FA0, FTZ on) afterwards, sticky flags included. The `sa_math::x87` model is f64 arithmetic that
/// was verified under the default MXCSR; the original x87 code does not depend on MXCSR.
#[inline(always)]
pub(crate) fn ieee<T>(f: impl FnOnce() -> T) -> T {
    #[cfg(all(windows, target_arch = "x86"))]
    {
        let mut old = 0u32;
        let ieee = 0x1F80u32;
        // SAFETY: saves MXCSR into a local and loads another local.
        unsafe {
            std::arch::asm!("stmxcsr [{0}]", "ldmxcsr [{1}]", in(reg) &mut old, in(reg) &ieee,
                options(nostack));
        }
        let r = f();
        // SAFETY: as above.
        unsafe { std::arch::asm!("ldmxcsr [{0}]", in(reg) &old, options(nostack)) };
        r
    }
    #[cfg(not(all(windows, target_arch = "x86")))]
    f()
}

/// ST0 return value; with the `SA_L2_MUTANT` self-test active for `id` (and `when`), the f32 result's
/// last bit is flipped (a deliberately wrong implementation the L2 oracle must catch).
fn st0(id: Id, v: sa_math::x87::X87, when: bool) -> f64 {
    if when && crate::l2::mutant(id) {
        return f32::from_bits(v.to_f32().to_bits() ^ 1) as f64;
    }
    v.to_f64()
}

/// `CGeneral::LimitRadianAngle` (cdecl `float(float)`). Returns the exact ST0 value: an f64 return is
/// loaded with `fld qword` (exact), so ST0 holds the x87 model's value bit for bit.
pub extern "C" fn limit_radian_angle_hook(angle: f32) -> f64 {
    table::count(Id::LimitRadianAngle);
    let v = ieee(|| sa_math::general::limit_radian_angle_x87(angle));
    st0(Id::LimitRadianAngle, v, angle.abs() > 1.0)
}

/// `CGeneral::LimitAngle` (cdecl `float(float)`). Where the original never returns (±inf,
/// |a| >= 2^33, ledger "original UB"), this spins too.
pub extern "C" fn limit_angle_hook(angle: f32) -> f64 {
    table::count(Id::LimitAngle);
    match ieee(|| sa_math::general::limit_angle_checked(angle)) {
        Some(v) => st0(Id::LimitAngle, v, angle.abs() > 1.0),
        None => loop {
            std::hint::spin_loop();
        },
    }
}

/// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (cdecl, ST0).
pub extern "C" fn get_radian_angle_between_points_hook(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 {
    table::count(Id::GetRadianAngleBetweenPoints);
    let v = ieee(|| sa_math::general::get_radian_angle_between_points_x87(x1, y1, x2, y2));
    st0(Id::GetRadianAngleBetweenPoints, v, true)
}

/// `CGeneral::GetATanOfXY(float x, float y)` (cdecl, ST0). On the raw-FPATAN path ST0 has 53 bits
/// here, 64 in the original (ledger: pending).
pub extern "C" fn get_atan_of_xy_hook(x: f32, y: f32) -> f64 {
    table::count(Id::GetATanOfXY);
    let v = ieee(|| sa_math::general::get_atan_of_xy_x87(x, y));
    st0(Id::GetATanOfXY, v, true)
}

/// `CGeneral::SolveQuadratic(float a, float b, float c, float* x1, float* x2)` (cdecl, bool in AL).
/// The original stores `*x2` before `*x1`.
///
/// # Safety
/// `x1` and `x2` must be valid for a 4-byte write (the original signature).
pub unsafe extern "C" fn solve_quadratic_hook(a: f32, b: f32, c: f32, x1: *mut f32, x2: *mut f32) -> bool {
    table::count(Id::SolveQuadratic);
    match ieee(|| sa_math::general::solve_quadratic(a, b, c)) {
        Some((r1, r2)) => {
            let (r1, r2) = if crate::l2::mutant(Id::SolveQuadratic) {
                (r2, r1)
            } else {
                (r1, r2)
            };
            // SAFETY: the caller passes two valid float pointers (original signature).
            unsafe {
                std::ptr::write_unaligned(x2, r2);
                std::ptr::write_unaligned(x1, r1);
            }
            true
        }
        None => false,
    }
}

/// `CVector::Normalise()` (thiscall, `this` in ECX). Only on x86 (the thiscall ABI).
///
/// # Safety
/// `this` must point to a CVector (3 floats).
#[cfg(target_arch = "x86")]
pub unsafe extern "thiscall" fn vector_normalise_hook(this: *mut f32) {
    table::count(Id::VectorNormalise);
    // SAFETY: `this` is a CVector (3 floats) per the original signature.
    unsafe {
        let v = [
            std::ptr::read_unaligned(this),
            std::ptr::read_unaligned(this.add(1)),
            std::ptr::read_unaligned(this.add(2)),
        ];
        let r = ieee(|| sa_math::vector::normalise(v));
        // The original stores x, y, z in this order; for a zero length it stores 1.0 into x only,
        // which `normalise` expresses by returning y and z unchanged.
        std::ptr::write_unaligned(this, r[0]);
        std::ptr::write_unaligned(this.add(1), r[1]);
        if !crate::l2::mutant(Id::VectorNormalise) {
            std::ptr::write_unaligned(this.add(2), r[2]);
        }
    }
}

/// Entry address of [`vector_normalise_hook`] (0 on hosts without the thiscall ABI).
pub fn vector_normalise_entry() -> usize {
    #[cfg(target_arch = "x86")]
    {
        vector_normalise_hook as unsafe extern "thiscall" fn(*mut f32) as usize
    }
    #[cfg(not(target_arch = "x86"))]
    0
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
        game_logic::is_coop_game_going_on(p1, p2) ^ crate::l2::mutant(Id::IsCoopGameGoingOn) as u32
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
