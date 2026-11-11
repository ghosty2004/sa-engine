//! L1 specs for PORT-ANIM-INPUT `sa_input::pad` (`CPad` and its state classes; see
//! `docs/arch/anim-input-port.md`). Region 0 is a `CPad` (0x134 bytes = 77 words) whose int16 fields
//! are shaped by [`shape_pad`] so that the edge tests ("pressed" = new != 0 && old == 0), the
//! `DisablePlayerControls` / `Mode` branches, the `DisableControls` bytes and the drunk-buffer index
//! see every path.
//!
//! `#[hook(entry_eax)]` ports read the caller's EAX: the L1 harness enters with EAX = the function
//! address, so their Rust side runs after `set_entry_eax(addr)` (under [`crate::host_globals`]).
//! Globals the original reads (`byte_8CD782`, `CCamera::m_bUseMouse3rdPerson`, `bInvertLook4Pad`,
//! `byte_B73403`, `CTimer::m_snTimeInMilliseconds`) are moved into scratch by operand patches; the
//! Rust side gets the same bytes through sa-input's / sa-core's host globals.

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, Inputs, RetKind, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_input::pad;

/// Words of a `CPad`.
pub const PAD_WORDS: usize = 0x134 / 4;
/// Scratch offset of region 1 when region 0 is a `CPad` (0x10 + round16(0x134 + 0x10)).
const AFTER_PAD: u32 = 0x160;

pub fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x1A_9A_D0;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// Sets the int16 at byte offset `off` of region `r` (both halves of a raw word are inputs).
pub fn set_i16(v: &mut Inputs, l: &Layout, r: usize, off: usize, x: i16) {
    let s = l.word_slot[r][off / 4].expect("raw word");
    let sh = 8 * (off % 4);
    v[s] = (v[s] & !(0xFFFF << sh)) | ((x as u16 as u32) << sh);
}

/// Sets the dword at byte offset `off` (4-aligned) of region `r`.
pub fn set_u32(v: &mut Inputs, l: &Layout, r: usize, off: usize, x: u32) {
    let s = l.word_slot[r][off / 4].expect("raw word");
    v[s] = x;
}

/// Sets the byte at byte offset `off` of region `r`.
pub fn set_u8(v: &mut Inputs, l: &Layout, r: usize, off: usize, x: u8) {
    let s = l.word_slot[r][off / 4].expect("raw word");
    let sh = 8 * (off % 4);
    v[s] = (v[s] & !(0xFF << sh)) | (u32::from(x) << sh);
}

/// A stick/button value: 0 often, 0xFF / ±128 sometimes, else random.
fn g_axis(r: &mut Rng) -> i16 {
    match r.below(8) {
        0..=2 => 0,
        3 => 255,
        4 => 128,
        5 => -128,
        6 => (r.below(256) as i32 - 128) as i16,
        _ => r.next_u32() as i16,
    }
}

/// Shapes the `CPad` in region `r`: the two controller states (0x00..0x60) get axis-like values,
/// `DisablePlayerControls` (+0x10E) is 0 in 70 % of the cases, `Mode` (+0x10A) is 0..3 in 85 %,
/// `DrunkDrivingBufferUsed` (+0x74) 0..9, the `DisableControls` bytes (+0x119..+0x11F) 0 in 70 %.
pub fn shape_pad(v: &mut Inputs, l: &Layout, r: usize) {
    let mut g = Rng::new(seed(v));
    // the drunk index always stays inside the region (an index outside it reads unmapped memory)
    let drunk = if g.chance(90) {
        g.below(10)
    } else {
        (g.below(148) as i32 - 48) as u32
    };
    set_u32(v, l, r, 0x74, drunk);
    if g.chance(10) {
        return;
    }
    for off in (0..0x60).step_by(2) {
        let x = g_axis(&mut g);
        set_i16(v, l, r, off, x);
    }
    let dis = if g.chance(70) { 0 } else { g.next_u32() as i16 | 1 };
    set_i16(v, l, r, 0x10E, dis);
    let mode = if g.chance(85) {
        g.below(4) as i16
    } else {
        g.next_u32() as i16
    };
    set_i16(v, l, r, 0x10A, mode);
    for off in 0x119..=0x11F {
        let b = if g.chance(70) { 0 } else { g.next_u32() as u8 | 1 };
        set_u8(v, l, r, off, b);
    }
}

type PadFn = fn(GPtr<u8>) -> ExpRet;

/// A `this`-only `CPad` query (region 0 = the pad, shaped).
fn q(
    key: &'static str,
    name: &'static str,
    addr: u32,
    ret: RetKind,
    callees: &'static [u32],
    f: PadFn,
) -> Spec {
    b(key, name, addr, Thiscall, ret)
        .regions(vec![raw(PAD_WORDS)])
        .args(vec![A::P(0)])
        .callees(callees)
        .fix(|v, l| shape_pad(v, l, 0))
        .rust(move |c| f(tp(c, 0)))
}

/// A `#[hook(entry_eax)]` `this`-only query: the harness enters with EAX = `addr`.
fn qe(
    key: &'static str,
    name: &'static str,
    addr: u32,
    callees: &'static [u32],
    f: fn(GPtr<u8>) -> u32,
) -> Spec {
    b(key, name, addr, Thiscall, Eax)
        .regions(vec![raw(PAD_WORDS)])
        .args(vec![A::P(0)])
        .callees(callees)
        .fix(|v, l| shape_pad(v, l, 0))
        .rust(move |c| {
            let _g = crate::host_globals();
            sa_core::abi::set_entry_eax(addr);
            ExpRet::Eax(f(tp(c, 0)))
        })
}

/// `(CPed*)` queries reading the four flag bytes (region 1: `byte_8CD782`, `m_bUseMouse3rdPerson`,
/// `bInvertLook4Pad`, `byte_B73403`, patched at `sites`: (instruction operand address, flag index))
/// and `ped->+0xFC` (region 2, 0x100 bytes; the null-ped path is left to L2).
fn qped(
    key: &'static str,
    name: &'static str,
    addr: u32,
    sites: &[(u32, u32)],
    f: fn(GPtr<u8>, GPtr<u8>) -> u32,
) -> Spec {
    let patches = sites
        .iter()
        .map(|&(at, k)| (at, (SCRATCH + AFTER_PAD + k).to_le_bytes().to_vec()))
        .collect();
    b(key, name, addr, Thiscall, Eax)
        .regions(vec![raw(PAD_WORDS), raw(1), raw(0x40)])
        .args(vec![A::P(0), A::P(2)])
        .patch(patches)
        .callees(&[0x821B40])
        .fix(|v, l| {
            shape_pad(v, l, 0);
            let mut g = Rng::new(seed(v) ^ 0x77);
            for k in 0..4 {
                let b = if g.chance(50) { 0 } else { g.next_u32() as u8 | 1 };
                set_u8(v, l, 1, k, b);
            }
            let att = if g.chance(50) { 0 } else { g.next_u32() | 1 };
            set_u32(v, l, 2, 0xFC, att);
        })
        .rust(move |c| {
            let _g = crate::host_globals();
            sa_core::abi::set_entry_eax(addr);
            pad::BYTE_8CD782.set(c.byte(1, 0));
            pad::USE_MOUSE_3RD_PERSON.set(c.byte(1, 1));
            pad::INVERT_LOOK_4_PAD.set(c.byte(1, 2));
            pad::BYTE_B73403.set(c.byte(1, 3));
            // the ped pointer is compared by value only for the upper EAX half: pass the guest address
            let ped = tp::<u8>(c, 2);
            let r = f(tp(c, 0), ped);
            let host_hi = (ped.addr() as u32) & 0xFFFF_0000;
            let guest_hi = c.addr(2) & 0xFFFF_0000;
            ExpRet::Eax(if r & 0xFFFF_0000 == host_hi && host_hi != guest_hi {
                guest_hi | (r & 0xFFFF)
            } else {
                r
            })
        })
}

fn al(v: u8) -> ExpRet {
    ExpRet::Al(v)
}

/// Mouse controller state region (0x14 bytes): button bytes zero half of the time, floats.
fn mouse_region() -> Region {
    Region {
        words: vec![W::R, W::R, W::F, W::F, W::F],
        kind: Kind::Any,
    }
}

fn shape_mouse(v: &mut Inputs, l: &Layout) {
    let mut g = Rng::new(seed(v));
    for off in 0..8 {
        let b = if g.chance(85) { 0 } else { g.next_u32() as u8 };
        set_u8(v, l, 0, off, b);
    }
    for w in 2..5 {
        if g.chance(50) {
            let s = l.word_slot[0][w].unwrap();
            v[s] = if g.chance(50) { 0 } else { 0x8000_0000 };
        }
    }
}

/// Every spec of this file.
pub fn all() -> Vec<Spec> {
    use pad as p;
    const LS1: &[u32] = &[0x53EDC0];
    vec![
        b("PadGetPad", "CPad::GetPad", 0x53FB70, Cdecl, Eax)
            .args(vec![A::I(
                |r| if r.chance(80) { r.below(4) } else { r.next_u32() },
            )])
            .rust(|c| ExpRet::Eax(pad::get_pad(c.i(0) as i32).addr() as u32)),
        q(
            "PadIsCrossPressed",
            "CPad::IsCrossPressed",
            0x4D59E0,
            Eax,
            &[],
            |t| ExpRet::Eax(p::is_cross_pressed(t)),
        ),
        q(
            "PadIsLeftShoulder1Pressed",
            "CPad::IsLeftShoulder1Pressed",
            0x53EDC0,
            Eax,
            &[],
            |t| ExpRet::Eax(p::is_left_shoulder1_pressed(t)),
        ),
        q(
            "PadIsRightShoulder1Pressed",
            "CPad::IsRightShoulder1Pressed",
            0x53EE20,
            Eax,
            &[],
            |t| ExpRet::Eax(p::is_right_shoulder1_pressed(t)),
        ),
        q(
            "PadIsTrianglePressed",
            "CPad::IsTrianglePressed",
            0x53EF40,
            Eax,
            &[],
            |t| ExpRet::Eax(p::is_triangle_pressed(t)),
        ),
        q(
            "PadIsCirclePressed",
            "CPad::IsCirclePressed",
            0x53EF60,
            Eax,
            &[],
            |t| ExpRet::Eax(p::is_circle_pressed(t)),
        ),
        qe(
            "PadGetSteeringLeftRight",
            "CPad::GetSteeringLeftRight",
            0x53FB80,
            &[],
            p::get_steering_left_right,
        ),
        qe(
            "PadGetSteeringUpDown",
            "CPad::GetSteeringUpDown",
            0x53FBD0,
            &[],
            p::get_steering_up_down,
        ),
        qe(
            "PadGetPedWalkLeftRight",
            "CPad::GetPedWalkLeftRight-",
            0x53FC90,
            &[],
            p::get_ped_walk_left_right,
        ),
        qe(
            "PadGetPedWalkUpDown",
            "CPad::GetPedWalkUpDown-",
            0x53FD30,
            &[],
            p::get_ped_walk_up_down,
        ),
        qe(
            "PadGetCarGunUpDown",
            "CPad::GetCarGunUpDown",
            0x53FC10,
            &[],
            p::get_car_gun_up_down,
        ),
        qe(
            "PadGetCarGunLeftRight",
            "CPad::GetCarGunLeftRight",
            0x53FC50,
            &[],
            p::get_car_gun_left_right,
        ),
        qe(
            "PadGetCarGunFired",
            "CPad::GetCarGunFired",
            0x53FF90,
            &[],
            p::get_car_gun_fired,
        ),
        qe(
            "PadCarGunJustDown",
            "CPad::CarGunJustDown",
            0x53FFE0,
            LS1,
            p::car_gun_just_down,
        ),
        qe(
            "PadGetHandBrake",
            "CPad::GetHandBrake",
            0x540040,
            &[],
            p::get_hand_brake,
        ),
        qe("PadGetBrake", "CPad::GetBrake", 0x540080, &[], p::get_brake),
        qe(
            "PadGetAccelerate",
            "CPad::GetAccelerate",
            0x5403F0,
            &[],
            p::get_accelerate,
        ),
        q("PadGetLookLeft", "CPad::GetLookLeft", 0x53FDD0, Al, &[], |t| {
            al(p::get_look_left(t))
        }),
        q("PadGetLookRight", "CPad::GetLookRight", 0x53FE10, Al, &[], |t| {
            al(p::get_look_right(t))
        }),
        q(
            "PadGetLookBehindForCar",
            "CPad::GetLookBehindForCar",
            0x53FE70,
            Al,
            &[],
            |t| al(p::get_look_behind_for_car(t)),
        ),
        q(
            "PadGetLookBehindForPed",
            "CPad::GetLookBehindForPed",
            0x53FEC0,
            Al,
            &[],
            |t| al(p::get_look_behind_for_ped(t)),
        ),
        q(
            "PadGetHydraulicJump",
            "CPad::GetHydraulicJump",
            0x53FF70,
            Al,
            &[],
            |t| al(p::get_hydraulic_jump(t)),
        ),
        q("PadGetHorn", "CPad::GetHorn", 0x53FEE0, Al, &[], |t| {
            al(p::get_horn(t))
        }),
        q(
            "PadGetExitVehicle",
            "CPad::GetExitVehicle",
            0x5400D0,
            Al,
            &[],
            |t| al(p::get_exit_vehicle(t)),
        ),
        q(
            "PadExitVehicleJustDown",
            "CPad::ExitVehicleJustDown",
            0x540120,
            Al,
            LS1,
            |t| al(p::exit_vehicle_just_down(t)),
        ),
        q("PadSub5404F0", "CPad::sub_5404F0", 0x5404F0, Al, &[], |t| {
            al(p::sub_5404f0(t))
        }),
        q("PadSub540530", "CPad::sub_540530", 0x540530, Eax, &[], |t| {
            ExpRet::Eax(p::sub_540530(t))
        }),
        q(
            "PadNextStationJustUp",
            "CPad::NextStationJustUp",
            0x5405B0,
            Al,
            &[],
            |t| al(p::next_station_just_up(t)),
        ),
        q(
            "PadLastStationJustUp",
            "CPad::LastStationJustUp",
            0x5405E0,
            Al,
            &[],
            |t| al(p::last_station_just_up(t)),
        ),
        q(
            "PadCycleWeaponLeftJustDown",
            "CPad::CycleWeaponLeftJustDown",
            0x540610,
            Al,
            &[],
            |t| al(p::cycle_weapon_left_just_down(t)),
        ),
        q(
            "PadCycleWeaponRightJustDown",
            "CPad::CycleWeaponRightJustDown",
            0x540640,
            Al,
            &[],
            |t| al(p::cycle_weapon_right_just_down(t)),
        ),
        q("PadGetTarget", "CPad::GetTarget", 0x540670, Al, &[], |t| {
            al(p::get_target(t))
        }),
        q(
            "PadGetEnterTargeting",
            "CPad::GetEnterTargeting",
            0x5406B0,
            Al,
            &[],
            |t| al(p::get_enter_targeting(t)),
        ),
        q("PadGetDuck", "CPad::GetDuck", 0x540700, Al, &[], |t| {
            al(p::get_duck(t))
        }),
        q("PadDuckJustDown", "CPad::DuckJustDown", 0x540720, Al, &[], |t| {
            al(p::duck_just_down(t))
        }),
        q("PadGetJump", "CPad::GetJump", 0x540750, Al, &[], |t| {
            al(p::get_jump(t))
        }),
        q("PadJumpJustDown", "CPad::JumpJustDown", 0x540770, Al, &[], |t| {
            al(p::jump_just_down(t))
        }),
        q("PadGetSprint", "CPad::GetSprint", 0x5407A0, Al, &[], |t| {
            al(p::get_sprint(t))
        }),
        q(
            "PadSprintJustDown",
            "CPad::SprintJustDown",
            0x5407F0,
            Al,
            &[],
            |t| al(p::sprint_just_down(t)),
        ),
        q(
            "PadShiftTargetLeftJustDown",
            "CPad::ShiftTargetLeftJustDown",
            0x540850,
            Al,
            &[],
            |t| al(p::shift_target_left_just_down(t)),
        ),
        q(
            "PadShiftTargetRightJustDown",
            "CPad::ShiftTargetRightJustDown",
            0x540880,
            Al,
            &[],
            |t| al(p::shift_target_right_just_down(t)),
        ),
        q(
            "PadCollectPickupJustDown",
            "CPad::CollectPickupJustDown",
            0x540A70,
            Al,
            &[],
            |t| al(p::collect_pickup_just_down(t)),
        ),
        q(
            "PadGetForceCameraBehindPlayer",
            "CPad::GetForceCameraBehindPlayer",
            0x540AE0,
            Al,
            &[],
            |t| al(p::get_force_camera_behind_player(t)),
        ),
        q("PadSniperZoomIn", "CPad::SniperZoomIn", 0x540B30, Al, &[], |t| {
            al(p::sniper_zoom_in(t))
        }),
        q(
            "PadSniperZoomOut",
            "CPad::SniperZoomOut",
            0x540B80,
            Al,
            &[],
            |t| al(p::sniper_zoom_out(t)),
        ),
        q(
            "PadGetGroupControlForward",
            "CPad::GetGroupControlForward",
            0x541190,
            Al,
            &[],
            |t| al(p::get_group_control_forward(t)),
        ),
        q(
            "PadGetGroupControlBack",
            "CPad::GetGroupControlBack",
            0x5411B0,
            Al,
            &[],
            |t| al(p::get_group_control_back(t)),
        ),
        q(
            "PadConversationYesJustDown",
            "CPad::ConversationYesJustDown",
            0x5411D0,
            Al,
            &[],
            |t| al(p::conversation_yes_just_down(t)),
        ),
        q(
            "PadConversationNoJustDown",
            "CPad::ConversationNoJustDown",
            0x541200,
            Al,
            &[],
            |t| al(p::conversation_no_just_down(t)),
        ),
        q(
            "PadGroupControlForwardJustDown",
            "CPad::GroupControlForwardJustDown",
            0x541230,
            Al,
            &[],
            |t| al(p::group_control_forward_just_down(t)),
        ),
        q(
            "PadGroupControlBackJustDown",
            "CPad::GroupControlBackJustDown",
            0x541260,
            Al,
            &[],
            |t| al(p::group_control_back_just_down(t)),
        ),
        b(
            "PadGetMeleeAttack",
            "CPad::GetMeleeAttack",
            0x540340,
            Thiscall,
            Al,
        )
        .regions(vec![raw(PAD_WORDS)])
        .args(vec![A::P(0), A::I(g_bool)])
        .fix(|v, l| shape_pad(v, l, 0))
        .rust(|c| {
            let f = c.i(1);
            al(p::get_melee_attack(tp(c, 0), f))
        }),
        b(
            "PadMeleeAttackJustDown",
            "CPad::MeleeAttackJustDown",
            0x540390,
            Thiscall,
            Al,
        )
        .regions(vec![raw(PAD_WORDS)])
        .args(vec![A::P(0), A::I(g_bool)])
        .callees(&[0x53EF40])
        .fix(|v, l| shape_pad(v, l, 0))
        .rust(|c| {
            let f = c.i(1);
            al(p::melee_attack_just_down(tp(c, 0), f))
        }),
        b(
            "PadSetDrunkInputDelay",
            "CPad::SetDrunkInputDelay",
            0x53F910,
            Thiscall,
            Void,
        )
        .regions(vec![raw(PAD_WORDS)])
        .args(vec![A::P(0), A::I(|r| r.next_u32())])
        .rust(|c| {
            let d = c.i(1) as i32;
            p::set_drunk_input_delay(tp(c, 0), d);
            ExpRet::Void
        }),
        // `CTimer::m_snTimeInMilliseconds` (0xB7CB84) read from region 1
        b("PadSetTouched", "CPad::SetTouched", 0x53F200, Thiscall, Void)
            .regions(vec![raw(PAD_WORDS), raw(1)])
            .args(vec![A::P(0)])
            .patch(vec![(0x53F201, (SCRATCH + AFTER_PAD).to_le_bytes().to_vec())])
            .rust(|c| {
                let _g = crate::host_globals();
                sa_core::timer::TIME_MS.set(c.u(1, 0));
                p::set_touched(tp(c, 0));
                ExpRet::Void
            }),
        b(
            "PadGetTouchedTimeDelta",
            "CPad::GetTouchedTimeDelta",
            0x53F210,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(PAD_WORDS), raw(1)])
        .args(vec![A::P(0)])
        .patch(vec![(0x53F211, (SCRATCH + AFTER_PAD).to_le_bytes().to_vec())])
        .rust(|c| {
            let _g = crate::host_globals();
            sa_core::timer::TIME_MS.set(c.u(1, 0));
            ExpRet::Eax(p::get_touched_time_delta(tp(c, 0)))
        }),
        // flag bytes: 0 = byte_8CD782, 1 = m_bUseMouse3rdPerson, 2 = bInvertLook4Pad, 3 = byte_B73403
        qped(
            "PadLookAroundLeftRight",
            "CPad::LookAroundLeftRight",
            0x540BD0,
            &[(0x540BE1, 0), (0x540BFA, 1), (0x540C03, 3)],
            p::look_around_left_right,
        ),
        qped(
            "PadLookAroundUpDown",
            "CPad::LookAroundUpDown",
            0x540CC0,
            &[(0x540CD1, 0), (0x540CEC, 1), (0x540CF5, 3), (0x540D55, 2)],
            p::look_around_up_down,
        ),
        qped(
            "PadGetPedWalkLeftRightPed",
            "CPad::GetPedWalkLeftRight-ped",
            0x540DC0,
            &[(0x540DD2, 3), (0x540DDE, 1), (0x540DFE, 0)],
            p::get_ped_walk_left_right_ped,
        ),
        qped(
            "PadGetPedWalkUpDownPed",
            "CPad::GetPedWalkUpDown-ped",
            0x540E20,
            &[(0x540E32, 3), (0x540E3E, 1), (0x540E5E, 0)],
            p::get_ped_walk_up_down_ped,
        ),
        b(
            "ControllerStateClear",
            "CControllerState::Clear",
            0x53EF80,
            Thiscall,
            Void,
        )
        .regions(vec![raw(12)])
        .args(vec![A::P(0)])
        .rust(|c| {
            p::controller_state_clear(tp(c, 0));
            ExpRet::Void
        }),
        b(
            "ControllerStateCheckForInput",
            "CControllerState::CheckForInput",
            0x53EFF0,
            Thiscall,
            Al,
        )
        .regions(vec![raw(12)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            // all-zero (but the PC flags) often, one random field set otherwise
            let mut g = Rng::new(seed(v));
            if g.chance(60) {
                for off in (0..0x28).step_by(2) {
                    set_i16(v, l, 0, off, 0);
                }
                if g.chance(60) {
                    let off = 2 * g.below(20) as usize;
                    set_i16(v, l, 0, off, g_axis(&mut g) | 1);
                }
            }
        })
        .rust(|c| al(p::controller_state_check_for_input(tp(c, 0)))),
        b(
            "KeyboardStateClear",
            "CKeyboardState::Clear",
            0x53F090,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x270 / 4)])
        .args(vec![A::P(0)])
        .rust(|c| {
            p::keyboard_state_clear(tp(c, 0));
            ExpRet::Void
        }),
        b(
            "MouseControllerStateConstructor",
            "CMouseControllerState::Constructor",
            0x53F220,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(5)])
        .args(vec![A::P(0)])
        .rust(|c| {
            p::mouse_state_constructor(tp(c, 0));
            ExpRet::Eax(c.addr(0))
        }),
        b(
            "MouseControllerStateClear",
            "CMouseControllerState::Clear",
            0x53F250,
            Thiscall,
            Void,
        )
        .regions(vec![raw(5)])
        .args(vec![A::P(0)])
        .rust(|c| {
            p::mouse_state_clear(tp(c, 0));
            ExpRet::Void
        }),
        b(
            "MouseControllerStateCheckForInput",
            "CMouseControllerState::CheckForInput",
            0x53F270,
            Thiscall,
            Al,
        )
        .regions(vec![mouse_region()])
        .args(vec![A::P(0)])
        .fix(shape_mouse)
        .rust(|c| al(p::mouse_check_for_input(tp(c, 0)))),
    ]
}
