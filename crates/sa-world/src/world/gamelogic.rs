//! `CGameLogic` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed `GameLogic.cpp` and
//! `docs/re/specs/WORLD/CGameLogic.deviations.md` as reading aids).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};
use super::cworld::{PLAYERS, PLAYER_INFO_SIZE, PLAYER_IN_FOCUS};

sa_core::globals! {
    list GLOBALS;
    /// `CGameLogic::NumAfterDeathStartPoints`
    pub static NUM_AFTER_DEATH_START_POINTS @ 0x96A890: i32 = 0;
    /// `CGameLogic::SkipToBeFinishedByScript`
    pub static SKIP_TO_BE_FINISHED_BY_SCRIPT @ 0x96A894: u8 = 0;
    /// `CGameLogic::SkipVehicle`
    pub static SKIP_VEHICLE @ 0x96A898: GPtr<u8> = GPtr::NULL;
    /// `CGameLogic::SkipTimer`
    pub static SKIP_TIMER @ 0x96A89C: u32 = 0;
    /// `CGameLogic::SkipState`
    pub static SKIP_STATE @ 0x96A8A0: i32 = 0;
    /// `CGameLogic::SkipHeading`
    pub static SKIP_HEADING @ 0x96A8A4: u32 = 0;
    /// `CGameLogic::bScriptCoopGameGoingOn`
    pub static SCRIPT_COOP_GAME_GOING_ON @ 0x96A8A8: u8 = 0;
    /// `CGameLogic::TimeOfLastEvent`
    pub static TIME_OF_LAST_EVENT @ 0x96A8AC: u32 = 0;
    /// `CGameLogic::GameState`
    pub static GAME_STATE @ 0x96A8B0: u8 = 0;
    /// `CGameLogic::ActivePlayers`
    pub static ACTIVE_PLAYERS @ 0x96A8B1: u8 = 0;
    /// `CGameLogic::nPrintFocusHelpTimer`
    pub static PRINT_FOCUS_HELP_TIMER @ 0x96A8B4: u32 = 0;
    /// `CGameLogic::nPrintFocusHelpCounter`
    pub static PRINT_FOCUS_HELP_COUNTER @ 0x96A8B8: u32 = 0;
    /// `CGameLogic::SkipPosition`
    pub static SKIP_POSITION @ 0x96A8D4: [u32; 3] = [0; 3];
    /// `CGameLogic::SavedWeapons` (`CWeapon[13]`, 0x1C each)
    pub static SAVED_WEAPONS @ 0x96A9B8: [u32; 13 * 7] = [0; 13 * 7];
}

/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;

orig! {
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00)
    pub fn camera_set_fade_colour(cam: GPtr<u8>, r: u32, gg: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16)` (0x50AC20)
    pub fn camera_fade(cam: GPtr<u8>, t: u32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571B70 => unported;
    /// `CCollision::Test2DLineAgainst2DLine` (0x4138D0)
    pub fn test_2d_line_against_2d_line(x1: u32, y1: u32, dx1: u32, dy1: u32, x2: u32, y2: u32, dx2: f32, dy2: f32) -> u8
        = cdecl 0x4138D0 => test2d_port;
}

#[allow(clippy::too_many_arguments)]
pub fn test2d_port(x1: u32, y1: u32, dx1: u32, dy1: u32, x2: u32, y2: u32, dx2: f32, dy2: f32) -> u8 {
    let f = f32::from_bits;
    crate::collision::prim::test_2d_line_against_2d_line(f(x1), f(y1), f(dx1), f(dy1), f(x2), f(y2), dx2, dy2)
}

/// `CGameLogic::InitAtStartOfGame` (stub 0x441210 → 0x1563910).
#[hook(addr = 0x441210, conv = cdecl, name = "CGameLogic::InitAtStartOfGame", expect = "E9 FB 26 12 01")]
pub fn init_at_start_of_game() {
    ACTIVE_PLAYERS.set(1);
    SKIP_STATE.set(0);
    NUM_AFTER_DEATH_START_POINTS.set(0);
    GAME_STATE.set(0);
    TIME_OF_LAST_EVENT.set(0);
    PRINT_FOCUS_HELP_TIMER.set(0);
    PRINT_FOCUS_HELP_COUNTER.set(0);
    SCRIPT_COOP_GAME_GOING_ON.set(0);
}

/// `CGameLogic::ForceDeathRestart` (stub 0x441240 → 0x15672E0): the focus player's state byte (+0xDC) = 1,
/// `GameState` = 1, `TimeOfLastEvent` = now - 3001, fade colour black, `Fade(4.0, 0)`.
#[hook(addr = 0x441240, conv = cdecl, name = "CGameLogic::ForceDeathRestart", expect = "E9 9B 60 12 01")]
pub fn force_death_restart() {
    let p = g::<u8>(PLAYERS + usize::from(PLAYER_IN_FOCUS.get()) * PLAYER_INFO_SIZE);
    let t = sa_core::timer::TIME_MS.get();
    wr::<u8>(p, 0xDC, 1);
    GAME_STATE.set(1);
    TIME_OF_LAST_EVENT.set(t.wrapping_add(0xFFFF_F447));
    camera_set_fade_colour(g(THE_CAMERA), 0, 0, 0);
    camera_fade(g(THE_CAMERA), 0x4080_0000, 0);
}

/// `CGameLogic::IsSkipWaitingForScriptToFadeIn` (stub 0x4416C0 → 0x1569C30): state 5 → 0 and true.
#[hook(addr = 0x4416C0, conv = cdecl, name = "CGameLogic::IsSkipWaitingForScriptToFadeIn", expect = "E9 6B 85 12 01")]
pub fn is_skip_waiting_for_script_to_fade_in() -> u8 {
    if SKIP_STATE.get() == 5 {
        SKIP_STATE.set(0);
        return 1;
    }
    0
}

/// `CGameLogic::IsPointWithinLineArea(const CVector* pts, int32 n, float x, float y)` (0x4416E0): counts
/// the edges `k → (k+1) % n` crossed by the segment from (x, y) along (1e6, 0)
/// (`Test2DLineAgainst2DLine`); the parity is the result (EAX).
#[hook(addr = 0x4416E0, conv = cdecl, name = "CGameLogic::IsPointWithinLineArea", expect = "51 53 8B 5C 24 10")]
pub fn is_point_within_line_area(pts: GPtr<u8>, n: i32, px: u32, py: u32) -> u32 {
    let mut count: u32 = 0;
    if n <= 0 {
        return 0;
    }
    for k in 0..n {
        let j = (k + 1) % n;
        let pk = pts.byte_add(k as isize * 12);
        let pj = pts.byte_add(j as isize * 12);
        let dy = x(rd::<f32>(pj, 4)).fsub_m(rd::<f32>(pk, 4)).to_f32();
        let dx = x(rd::<f32>(pj, 0)).fsub_m(rd::<f32>(pk, 0)).to_f32();
        if test_2d_line_against_2d_line(px, py, 0x4974_2400, 0, rd::<u32>(pk, 0), rd::<u32>(pk, 4), dx, dy) != 0 {
            count = count.wrapping_add(1);
        }
    }
    count & 1
}

/// 2D distance `sqrt(dy*dy + dx*dx)` of `(px - cx, py - cy)` (x87).
fn dist(px: f32, py: f32, cx: u32, cy: u32) -> X87 {
    let a = x(px).fsub_m(f32::from_bits(cx));
    let b = x(py).fsub_m(f32::from_bits(cy));
    b.fmul(b).fadd(a.fmul(a)).fsqrt()
}

/// `CGameLogic::FindCityClosestToPoint(float x, float y)` (0x441B70): 1 (LS, 1670/-1137... centres at
/// 0x85984C/0x859848), 3 (0x859844/0x859840) when strictly closer (its distance kept as f32), 2
/// (0x85983C/0x859838) when strictly closer than the best.
#[hook(addr = 0x441B70, conv = cdecl, name = "CGameLogic::FindCityClosestToPoint", expect = "51 D9 44 24 08")]
pub fn find_city_closest_to_point(px: f32, py: f32) -> i32 {
    let d1 = dist(px, py, 0x44D0_C000, 0xC48E_2000);
    let d3 = dist(px, py, 0x4507_1000, 0x4505_C000);
    let (mut best, mut city) = (d1, 1);
    // fcomp st(1); test ah,5; jp: keep unless d3 < d1
    if !d3.fcom(d1).test_parity(0x05) {
        best = x(d3.to_f32());
        city = 3;
    }
    let d2 = dist(px, py, 0xC4E2_4000, 0x445D_0000);
    if !d2.fcom(best).test_parity(0x05) { 2 } else { city }
}

/// `CGameLogic::StorePedsWeapons(CPed*)` (0x441D00): copies the 13 weapons (+0x5A0, 0x1C each).
#[hook(addr = 0x441D00, conv = cdecl, name = "CGameLogic::StorePedsWeapons", expect = "8B 54 24 04 56")]
pub fn store_peds_weapons(ped: GPtr<u8>) {
    let dst = SAVED_WEAPONS.ptr().cast::<u32>();
    let src = ped.byte_add(0x5A0).cast::<u32>();
    for k in 0..13 * 7 {
        dst.wrapping_index(k).write(src.wrapping_index(k).read());
    }
}

/// `CGameLogic::SetUpSkip(CVector pos, float heading, bool afterMission, CEntity* vehicle, bool
/// finishedByScript)` (stub 0x4423C0 → 0x1563E90): ends a pending fade-out skip (state 2: black fade in
/// 0.5 s), clears pad 0's +0x10F bit 0, then stores the skip (state 4 after a mission, else 1), the
/// time, the vehicle (registering the reference) and the script flag.
#[hook(addr = 0x4423C0, conv = cdecl, name = "CGameLogic::SetUpSkip", expect = "E9 CB 1A 12 01")]
pub fn set_up_skip(px: u32, py: u32, pz: u32, heading: u32, after_mission: u32, veh: GPtr<u8>, by_script: u32) {
    if SKIP_STATE.get() == 2 {
        camera_set_fade_colour(g(THE_CAMERA), 0, 0, 0);
        camera_fade(g(THE_CAMERA), 0x3F00_0000, 1);
    }
    SKIP_STATE.set(0);
    let pad = get_pad(0);
    wr::<u8>(pad, 0x10F, rd::<u8>(pad, 0x10F) & 0xFE);
    SKIP_POSITION.set([px, py, pz]);
    SKIP_HEADING.set(heading);
    SKIP_STATE.set(if after_mission as u8 != 0 { 4 } else { 1 });
    SKIP_TIMER.set(sa_core::timer::TIME_MS.get());
    SKIP_VEHICLE.set(veh);
    if !veh.is_null() {
        register_reference(veh, SKIP_VEHICLE.ptr());
    }
    SKIP_TO_BE_FINISHED_BY_SCRIPT.set(by_script as u8);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_AT_START_OF_GAME,
    HOOK_FORCE_DEATH_RESTART,
    HOOK_IS_SKIP_WAITING_FOR_SCRIPT_TO_FADE_IN,
    HOOK_IS_POINT_WITHIN_LINE_AREA,
    HOOK_FIND_CITY_CLOSEST_TO_POINT,
    HOOK_STORE_PEDS_WEAPONS,
    HOOK_SET_UP_SKIP,
];
