//! L1 specs for PORT-ANIM-INPUT `sa_input::ccm` (`CControllerConfigManager`; see
//! `docs/arch/anim-input-port.md`).
//!
//! The manager is 0x12E4 bytes, more than the 0x500-byte scratch image, so `this` is passed as a
//! constant `SCRATCH + 0x10 - BASE`: region 0 (the first region always starts at scratch + 0x10) then
//! lies at `this + BASE` and covers the fields the function touches (a range of `m_Actions` or the
//! stick flags); the action arguments are kept inside that range. The Rust side gets the same
//! `this` (region 0's host address minus `BASE`).

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, Inputs, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_input::ccm;

/// `m_Actions[0]`.
const ACTIONS: u32 = 0xB70;
/// Actions 0..31 (0x400 bytes).
const LOW_WORDS: usize = 0x400 / 4;

fn tp(c: &mut Ctx, r: usize) -> GPtr<u8> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]))
}

/// The host `this` for region 0 at `this + base`.
fn this_at(c: &mut Ctx, base: u32) -> GPtr<u8> {
    tp(c, 0).byte_add(-(base as isize))
}

/// The scratch `this` for region 0 at `this + base`.
const fn k_this(base: u32) -> A {
    A::K(SCRATCH + 0x10 - base)
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0xCC_A1_7E;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// A key value: `rsNULL`, 0, small key codes, sometimes random.
fn g_key(r: &mut Rng) -> u32 {
    match r.below(8) {
        0 | 1 => 0x420,
        2 => 0,
        3 => r.below(8),
        4..=6 => r.below(0x430),
        _ => r.next_u32(),
    }
}

fn g_type(r: &mut Rng) -> u32 {
    if r.chance(90) { r.below(4) } else { r.below(7) }
}

/// An action in 0..`n`.
fn g_action_lo(r: &mut Rng) -> u32 {
    r.below(30)
}

/// Shapes a `m_Actions` region (initiator, order word pairs): initiators are often the key argument
/// `key_arg` (so bindings match), `rsNULL` or 0; orders are mostly 0..5.
fn shape_keys(v: &mut Inputs, l: &Layout, r: usize, key_arg: Option<usize>) {
    let mut g = Rng::new(seed(v));
    let k = key_arg.map(|a| v[l.arg_slot[a].unwrap()]);
    let n = l.word_slot[r].len();
    for w in 0..n {
        let s = l.word_slot[r][w].unwrap();
        v[s] = if w % 2 == 0 {
            match (g.below(10), k) {
                (0..=3, Some(k)) => k,
                (0..=1, None) | (4 | 5, _) => 0x420,
                (2 | 3, None) | (6, _) => 0,
                (7 | 8, _) => g.below(0x430),
                _ => g.next_u32(),
            }
        } else {
            match g.below(10) {
                0..=7 => g.below(6),
                8 => u32::MAX,
                _ => g.next_u32(),
            }
        };
    }
}

/// Shapes a `CControllerState` region (12 words of two int16): 0, 0xFF, ±128 or random.
fn shape_state(v: &mut Inputs, l: &Layout, r: usize) {
    let mut g = Rng::new(seed(v) ^ 0x5157);
    for s in l.word_slot[r].iter().flatten() {
        let mut half = || -> u32 {
            (match g.below(7) {
                0..=2 => 0i16,
                3 => 0xFF,
                4 => 128,
                5 => -128,
                _ => g.next_u32() as i16,
            }) as u16 as u32
        };
        let (a, b) = (half(), half());
        v[*s] = a | (b << 16);
    }
}

fn state_region() -> Region {
    raw(12)
}

pub fn all() -> Vec<Spec> {
    vec![
        // ---- joystick buttons (this at region 0, 0x154 bytes)
        b(
            "CcmGetJoyButtonJustUp",
            "CControllerConfigManager::GetJoyButtonJustUp",
            0x52D1C0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(0x154 / 4)])
        .args(vec![A::P(0)])
        .rust(|c| ExpRet::Eax(ccm::get_joy_button_just_up(tp(c, 0)))),
        b(
            "CcmGetJoyButtonJustDown",
            "CControllerConfigManager::GetJoyButtonJustDown",
            0x52D1E0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(0x154 / 4)])
        .args(vec![A::P(0)])
        .rust(|c| ExpRet::Eax(ccm::get_joy_button_just_down(tp(c, 0)))),
        // ---- stick flags (region 0 = this + 0x12D0)
        b(
            "CcmClearSimButtonPressCheckers",
            "CControllerConfigManager::ClearSimButtonPressCheckers",
            0x52DD90,
            Thiscall,
            Void,
        )
        .regions(vec![raw(4)])
        .args(vec![k_this(0x12D0)])
        .rust(|c| {
            ccm::clear_sim_button_press_checkers(this_at(c, 0x12D0));
            ExpRet::Void
        }),
        // ---- argument-only
        b(
            "CcmGetIsKeyBlank",
            "CControllerConfigManager::GetIsKeyBlank",
            0x52F2A0,
            Thiscall,
            Al,
        )
        .args(vec![A::K(SCRATCH), A::I(g_key), A::I(g_type)])
        .rust(|c| ExpRet::Al(ccm::get_is_key_blank(GPtr::NULL, c.i(1), c.i(2)))),
        b(
            "CcmGetActionType",
            "CControllerConfigManager::GetActionType",
            0x52F2F0,
            Thiscall,
            Eax,
        )
        .args(vec![
            A::K(SCRATCH),
            A::I(|r| if r.chance(90) { r.below(0x40) } else { r.next_u32() }),
        ])
        .rust(|c| ExpRet::Eax(ccm::get_action_type(GPtr::NULL, c.i(1) as i32))),
        b(
            "CcmGetIsActionAButtonCombo",
            "CControllerConfigManager::GetIsActionAButtonCombo",
            0x52F550,
            Thiscall,
            Al,
        )
        .args(vec![A::K(SCRATCH), A::I(|r| r.next_u32())])
        .rust(|c| ExpRet::Al(ccm::get_is_action_a_button_combo(GPtr::NULL, c.i(1) as i32))),
        // ---- m_Actions 0..31 (region 0 = this + 0xB70)
        b(
            "CcmGetNumOfSettingsForAction",
            "CControllerConfigManager::GetNumOfSettingsForAction",
            0x52F4A0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![k_this(ACTIONS), A::I(g_action_lo)])
        .fix(|v, l| shape_keys(v, l, 0, None))
        .rust(|c| {
            ExpRet::Eax(ccm::get_num_of_settings_for_action(
                this_at(c, ACTIONS),
                c.i(1) as i32,
            ))
        }),
        b(
            "CcmGetControllerKeyAssociatedWithAction",
            "CControllerConfigManager::GetControllerKeyAssociatedWithAction",
            0x52F4F0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![k_this(ACTIONS), A::I(g_action_lo), A::I(g_type)])
        .rust(|c| {
            let (a, t) = (c.i(1) as i32, c.i(2) as i32);
            ExpRet::Eax(ccm::get_controller_key_associated_with_action(
                this_at(c, ACTIONS),
                a,
                t,
            ))
        }),
        b(
            "CcmGetMouseButtonAssociatedWithAction",
            "CControllerConfigManager::GetMouseButtonAssociatedWithAction",
            0x52F580,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![k_this(ACTIONS), A::I(g_action_lo)])
        .rust(|c| {
            ExpRet::Eax(ccm::get_mouse_button_associated_with_action(
                this_at(c, ACTIONS),
                c.i(1) as i32,
            ))
        }),
        b(
            "CcmSetMouseButtonAssociatedWithAction",
            "CControllerConfigManager::SetMouseButtonAssociatedWithAction",
            0x52F590,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![k_this(ACTIONS), A::I(g_action_lo), A::I(|r| r.below(9))])
        .fix(|v, l| shape_keys(v, l, 0, None))
        .rust(|c| {
            let (a, m) = (c.i(1) as i32, c.i(2));
            ccm::set_mouse_button_associated_with_action(this_at(c, ACTIONS), a, m);
            ExpRet::Void
        }),
        b(
            "CcmResetSettingOrder",
            "CControllerConfigManager::ResetSettingOrder",
            0x52F5F0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![k_this(ACTIONS), A::I(g_action_lo)])
        .fix(|v, l| shape_keys(v, l, 0, None))
        .rust(|c| {
            ccm::reset_setting_order(this_at(c, ACTIONS), c.i(1) as i32);
            ExpRet::Void
        }),
        b(
            "CcmClearSettingsAssociatedWithAction",
            "CControllerConfigManager::ClearSettingsAssociatedWithAction",
            0x52FD70,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![k_this(ACTIONS), A::I(g_action_lo), A::I(g_type)])
        .callees(&[0x52F5F0])
        .fix(|v, l| shape_keys(v, l, 0, None))
        .rust(|c| {
            let (a, t) = (c.i(1) as i32, c.i(2) as i32);
            ccm::clear_settings_associated_with_action(this_at(c, ACTIONS), a, t);
            ExpRet::Void
        }),
        b(
            "CcmSetControllerKeyAssociatedWithAction",
            "CControllerConfigManager::SetControllerKeyAssociatedWithAction",
            0x530490,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![
            k_this(ACTIONS),
            A::I(g_action_lo),
            A::I(g_key),
            A::I(g_type),
        ])
        .callees(&[0x52F5F0])
        .fix(|v, l| shape_keys(v, l, 0, None))
        .rust(|c| {
            let (a, k, t) = (c.i(1) as i32, c.i(2), c.i(3) as i32);
            ccm::set_controller_key_associated_with_action(this_at(c, ACTIONS), a, k, t);
            ExpRet::Void
        }),
        b(
            "CcmDeleteMatching1rstPersonControls",
            "CControllerConfigManager::DeleteMatching1rstPersonControls",
            0x531C20,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![
            k_this(ACTIONS),
            A::I(g_action_lo),
            A::I(g_key),
            A::I(g_type),
        ])
        .callees(&[0x52FD70, 0x52F5F0])
        .fix(|v, l| shape_keys(v, l, 0, Some(2)))
        .rust(|c| {
            let (a, k, t) = (c.i(1) as i32, c.i(2), c.i(3) as i32);
            ccm::delete_matching_1rst_person_controls(this_at(c, ACTIONS), a, k, t);
            ExpRet::Void
        }),
        b(
            "CcmDeleteMatchingVehicle3rdPersonControls",
            "CControllerConfigManager::DeleteMatchingVehicle_3rdPersonControls",
            0x531BC0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS)])
        .args(vec![
            k_this(ACTIONS),
            A::I(g_action_lo),
            A::I(g_key),
            A::I(g_type),
        ])
        .callees(&[0x52FD70, 0x52F5F0])
        .fix(|v, l| shape_keys(v, l, 0, Some(2)))
        .rust(|c| {
            let (a, k, t) = (c.i(1) as i32, c.i(2), c.i(3) as i32);
            ccm::delete_matching_vehicle_3rd_person_controls(this_at(c, ACTIONS), a, k, t);
            ExpRet::Void
        }),
        // ---- bindings -> CControllerState (region 0 = actions, region 1 = the state)
        b(
            "CcmButtonDownFirstPersonOnly",
            "CControllerConfigManager::AffectControllerStateOn_ButtonDown_FirstPersonOnly",
            0x52F9E0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS), state_region()])
        .args(vec![k_this(ACTIONS), A::I(g_key), A::I(g_type), A::P(1)])
        .fix(|v, l| {
            shape_keys(v, l, 0, Some(1));
            shape_state(v, l, 1);
        })
        .rust(|c| {
            let (k, t, st) = (c.i(1), c.i(2) as i32, tp(c, 1));
            ccm::affect_button_down_first_person_only(this_at(c, ACTIONS), k, t, st);
            ExpRet::Void
        }),
        b(
            "CcmButtonDownVehicleAndThirdPersonOnly",
            "CControllerConfigManager::AffectControllerStateOn_ButtonDown_VehicleAndThirdPersonOnly",
            0x52FD20,
            Thiscall,
            Void,
        )
        .regions(vec![raw(LOW_WORDS), state_region()])
        .args(vec![k_this(ACTIONS), A::I(g_key), A::I(g_type), A::P(1)])
        .fix(|v, l| {
            shape_keys(v, l, 0, Some(1));
            shape_state(v, l, 1);
        })
        .rust(|c| {
            let (k, t, st) = (c.i(1), c.i(2) as i32, tp(c, 1));
            ccm::affect_button_down_vehicle_and_third_person_only(this_at(c, ACTIONS), k, t, st);
            ExpRet::Void
        }),
        // actions 32..55 (region 0 = this + 0xF70)
        b(
            "CcmButtonUpAllPlayerStates",
            "CControllerConfigManager::AffectControllerStateOn_ButtonUp_All_Player_States",
            0x52FD40,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x300 / 4), state_region()])
        .args(vec![k_this(0xF70), A::I(g_key), A::I(g_type), A::P(1)])
        .fix(|v, l| {
            shape_keys(v, l, 0, Some(1));
            shape_state(v, l, 1);
        })
        .rust(|c| {
            let (k, t, st) = (c.i(1), c.i(2) as i32, tp(c, 1));
            ccm::affect_button_up_all_player_states(this_at(c, 0xF70), k, t, st);
            ExpRet::Void
        }),
    ]
}
