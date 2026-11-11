//! L1 specs for PORT-AI (`sa_ai`): the entries whose hooks preserve a register (VER-2 preserve
//! specs).
//!
//! Entities are scratch windows: a ped/vehicle region holds `m_placement` (+4..+0x10) and `m_matrix`
//! (+0x14, sometimes null), a matrix region the position (+0x30..). The ports read `m_matrix` and a
//! task's entity pointer as `GPtr`s (8 bytes on the host): the Rust side runs on a host copy with
//! those fields widened ([`crate::hosted::Widened`]; the 4 bytes after each are not read).

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_ai::{car_enter, misc, tasks};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
use sa_core::mem::GPtr;

fn hash(v: &[u32]) -> u32 {
    v.iter()
        .fold(0x811C_9DC5u32, |h, &w| (h ^ w).wrapping_mul(0x0100_0193))
}

/// A small index around the function's cases, sometimes any int.
fn g_index(r: &mut Rng) -> u32 {
    match r.below(10) {
        0..=7 => (r.below(13) as i32 - 2) as u32,
        8 => r.pick(&[0x8000_0000, 0x7FFF_FFFF, 0xFFFF_FFFF, 0x1_0001]),
        _ => r.next_u32(),
    }
}

/// An entity: +4..+0x10 position, +0x14 `m_matrix` (the matrix region `mat`, or null).
fn entity(mat: usize) -> Region {
    Region {
        words: vec![W::R, W::F, W::F, W::F, W::R, W::PN(mat)],
        kind: Kind::Any,
    }
}

/// A matrix region: the position at +0x30..+0x3C.
fn matrix() -> Region {
    let mut w = vec![W::R; 15];
    for x in &mut w[12..15] {
        *x = W::F;
    }
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// `CPed::m_nActiveWeaponSlot`-style word at +0x598 (window word 0): the mapped values often.
const PED_598: u32 = 0x598;

pub fn all() -> Vec<Spec> {
    vec![
        // the callers keep EDX across the call (VER-2)
        b(
            "AiPedGroupDefaultTaskAllocatorsGet",
            "CPedGroupDefaultTaskAllocators::Get",
            0x5F7200,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_index)])
        .preserve(PRESERVE_EDX)
        .rust(|c| ExpRet::Eax(tasks::default_task_allocators_get(c.i(0)))),
        b(
            "AiPlayHandSignalGetAnimIdForPed",
            "CTaskComplexPlayHandSignalAnim::GetAnimIdForPed",
            0x61B460,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(1)])
        .args(vec![A::I(|r| r.next_u32()), A::K(SCRATCH + 0x10 - PED_598)])
        .fix(|v, l| {
            let s = l.word_slot[0][0].unwrap();
            let h = hash(v);
            if !h.is_multiple_of(4) {
                v[s] = [7, 8, 9, 0xB, 0xE, 0, 10, 0xD, 0xF][(h >> 2) as usize % 9];
            }
        })
        .preserve(PRESERVE_EDX)
        .rust(|c| {
            let ped = c
                .mem
                .as_mut_ptr()
                .wrapping_add(c.lay.off[0])
                .wrapping_sub(PED_598 as usize);
            let r = tasks::play_hand_signal_get_anim_id_for_ped(GPtr::NULL, GPtr::from_raw(ped));
            ExpRet::Eax(r as u32)
        }),
        b(
            "AiGoToPointAndStandStillSelectMoveState",
            "CTaskComplexGoToPointAndStandStill::SelectMoveState",
            0x668250,
            Thiscall,
            Void,
        )
        .regions(vec![
            Region {
                words: vec![W::R, W::R, W::R, W::F, W::F],
                kind: Kind::Any,
            },
            entity(2),
            matrix(),
        ])
        .args(vec![A::I(|r| r.next_u32()), A::P(0), A::P(1), A::F, A::F])
        .preserve(PRESERVE_EDX)
        .rust(|c| {
            let (go_to, ped) = (c.addr(0), c.addr(1));
            let mut w = crate::hosted::Widened::new(&c.mem, &[c.lay.off[1] + 0x14]);
            let (go_to, ped) = (w.at(go_to), w.at(ped));
            tasks::go_to_point_and_stand_still_select_move_state(GPtr::NULL, go_to, ped, c.f(3), c.f(4));
            w.finish(&mut c.mem);
            ExpRet::Void
        }),
        // CTaskSimpleFight::LoadMeleeData keeps ECX across the call (0x5BF1FD..0x5BF241)
        b("AiFun5BD3B0", "FUN_005bd3b0", 0x5BD3B0, Cdecl, Eax)
            .args(vec![A::I(g_index)])
            .preserve(PRESERVE_ECX)
            .rust(|c| ExpRet::Eax(misc::fun_005bd3b0(c.i(0) as i32))),
        b("AiFun5BD360", "FUN_005bd360", 0x5BD360, Cdecl, Al)
            .regions(vec![raw(1)])
            .args(vec![A::P(0)])
            .fix(|v, l| {
                let s = l.word_slot[0][0].unwrap();
                let h = hash(v);
                if !h.is_multiple_of(4) {
                    let ch = [b'H', b'L', b'G', b'B', b'h', 0, b'X'][(h >> 2) as usize % 7];
                    v[s] = (v[s] & !0xFF) | u32::from(ch);
                }
            })
            .preserve(PRESERVE_ECX)
            .rust(|c| {
                let s = GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[0]));
                ExpRet::Al(misc::fun_005bd360(s))
            }),
        b(
            "AiGoToCarDoorAndStandStillIsVehicleInRange",
            "CTaskComplexGoToCarDoorAndStandStill::IsVehicleInRange",
            0x6458A0,
            Thiscall,
            Eax,
        )
        .regions(vec![
            Region {
                words: vec![W::R, W::R, W::R, W::P(1), W::R, W::R, W::R, W::R, W::F],
                kind: Kind::Any,
            },
            entity(2),
            matrix(),
            entity(4),
            matrix(),
        ])
        .args(vec![A::P(0), A::P(3)])
        .preserve(PRESERVE_ECX)
        .rust(|c| {
            let (this, ped) = (c.addr(0), c.addr(3));
            let o = &c.lay.off;
            let mut w = crate::hosted::Widened::new(&c.mem, &[o[0] + 0xC, o[1] + 0x14, o[3] + 0x14]);
            let (this, ped) = (w.at(this), w.at(ped));
            let r = car_enter::go_to_car_door_is_vehicle_in_range(this, ped);
            w.finish(&mut c.mem);
            ExpRet::Eax(r)
        }),
    ]
}
