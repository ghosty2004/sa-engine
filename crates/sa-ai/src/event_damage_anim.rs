//! FRONTIER-2 (batch `frontier2-1`): the damage event's animation choice (`CEventDamage::ComputeDamageAnim`, user-hit
//! in melee) and the fight combo tests it calls. Ported from the 1.0 US bytes; where gta-reversed differs the bytes
//! are followed (the knock-back force is normalised in 2D, z = 5).

use sa_core::abi::HookReg;
use sa_core::field::gf;
use sa_core::bindings::{
    general_random_int_in_range, get_weapon_info, intel_get_task_use_gun, ped_get_bone_position, tm_get_active_task,
    tm_get_simplest_active_task, vector2d_normalise,
};
use sa_core::crt::rand_at;
use sa_core::{hook, orig};
use sa_math::fpu::{gt, lt};
use sa_math::vector::CVector;
use sa_world::bindings::{
    clump_get_first_association_4d6a70, intel_get_task_fighting, physical_apply_move_force_5429f0,
    stats_get_fat_and_muscle_modifier,
};
use sa_world::world::common::entity_pos;

use crate::bindings::{fight_get_combo_anim_group_id, history_get_current_event};
use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7_CB84;

orig! {
    /// `CTaskSimpleFight::IsHitComboSet` (0x4ABDF0)
    fn fight_is_hit_combo_set_o(t: GPtr<u8>) -> u8 = thiscall 0x4ABDF0 => fight_is_hit_combo_set;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// virtual `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// virtual `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
}

/// The fight move table row of the task's combo set (+0x24): `max(combo - 4, 0) * 0x88`.
fn combo_row(t: GPtr<u8>) -> usize {
    let c = i32::from(rd::<i8>(t, 0x24)) - 4;
    (if c < 0 { 0 } else { c } as usize) * 0x88
}

/// `CTaskSimpleFight::IsComboSet` (0x4ABDC0): bit `4 + move (+0x25)` of the combo row's word at 0xC17154.
#[hook(addr = 0x4ABDC0, conv = thiscall, name = "CTaskSimpleFight::IsComboSet", expect = "0F BE 41 24 8A 49 25",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn fight_is_combo_set(this: GPtr<u8>) -> u8 {
    let w = u32::from(g::<u16>(0xC1_7154 + combo_row(this)).read());
    u8::from(w & 0x10u32.wrapping_shl(u32::from(rd::<u8>(this, 0x25))) != 0)
}

/// `CTaskSimpleFight::IsHitComboSet` (0x4ABDF0): moves 0..=2 only (signed), bit `12 + move` of the same word.
#[hook(addr = 0x4ABDF0, conv = thiscall, name = "CTaskSimpleFight::IsHitComboSet", expect = "8B C1 8A 48 25 80 F9 02",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn fight_is_hit_combo_set(this: GPtr<u8>) -> u8 {
    let m = rd::<u8>(this, 0x25);
    if m as i8 > 2 {
        return 0;
    }
    let w = u32::from(g::<u16>(0xC1_7154 + combo_row(this)).read());
    u8::from(0x1000u32.wrapping_shl(u32::from(m)) & w != 0)
}

/// `float(time) + c` with MSVC's unsigned-to-float fix-up (`fild`, `+ 2^32` when negative), then `_ftol`.
fn time_plus(t: u32, c: f32) -> u32 {
    let mut v = xi(t as i32);
    if (t as i32) < 0 {
        v = v.fadd_m(gf(0x85_8C54));
    }
    ftol32(v.fadd_m(c)) as u32
}

/// `CEventDamage::ComputeDamageAnim(CPed* ped, bool makeAbortable)` (0x4B3FC0, `ret 8`).
///
/// Fields: +0x14 source, +0x1C weapon, +0x20 ped piece, +0x24 direction, +0x25 bit 1 fall down, +0x28 anim group,
/// +0x2C anim id, +0x30 blend, +0x38 / +0x3C damage (health, armour). In a vehicle: fall down for a melee weapon
/// (fire type 0, not 0 / 1 / 0x2E) or damage above 50, no anim. Otherwise as gta-reversed's reading except: the
/// knock-back (fall down without a combo and a positive force) normalises the source-to-ped vector in 2D only
/// (`CVector2D::Normalise`), clears the standing bits and applies `(-x f, -y f, 5)`; the torso tests run in the
/// original's order (weapon < 9 with health below the threshold, the fall from an object, the fight-idle move).
#[hook(addr = 0x4B3FC0, conv = thiscall, name = "CEventDamage::ComputeDamageAnim", expect = "83 EC 34 53 8B 5C 24 3C",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x4B4528; the entry 0x4B4704 jumps through the protection thunk 0x405B96 (EAX = ped +0x47C) and back")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn compute_damage_anim(this: GPtr<u8>, ped: GPtr<u8>, abortable: u32) {
    let w = || rd::<i32>(this, 0x1C);
    let dir = || rd::<u8>(this, 0x24);
    let set_fall = || wr::<u8>(this, 0x25, rd::<u8>(this, 0x25) | 2);
    let set_anim = |a: u32| wr::<u32>(this, 0x2C, a);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u32>(ped, 0x58C) != 0 {
        let wi = get_weapon_info(w(), 1);
        let melee = rd::<u32>(wi, 0) == 0 && !matches!(w(), 0 | 1 | 0x2E);
        if melee || rd::<i16>(wi, 0x22) > 50 {
            set_fall();
        }
        set_anim(0xBF);
        return;
    }
    let move_state = rd::<i32>(ped, 0x534);
    let mut heavy = matches!(w(), 0x1B | 0x19 | 0x1A | 0x18 | 0x21);
    if rd::<u8>(ped, 0x478) & 4 != 0 {
        heavy = false;
    }
    let mut strength = 1.0f32;
    let mut threshold = gf(0x85_AAF4);
    let mut force = 0.0f32;
    let mut use_gun = GPtr::<u8>::NULL;
    let mut fight = GPtr::<u8>::NULL;
    let src = rd::<GPtr<u8>>(this, 0x14);
    if !src.is_null() && rd::<u8>(src, 0x36) & 7 == 3 {
        fight = intel_get_task_fighting(rd::<GPtr<u8>>(src, 0x47C));
        if fight.is_null() {
            use_gun = intel_get_task_use_gun(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x14), 0x47C));
        }
        let pd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x14), 0x480);
        if !pd.is_null() {
            if rd::<u8>(pd, 0x34) & 2 != 0 {
                strength = 2.0;
            } else {
                strength = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(4))
                    .fadd_m(gf(0x85_8624))
                    .fmul_m(gf(0x85_8B8C))
                    .to_f32();
                threshold = x(gf(0x85_AAFC)).fmul_m(gf(0x85_8B8C)).fadd_m(gf(0x85_AAF8)).to_f32();
            }
        }
    }
    let mut head = [0f32; 3];
    ped_get_bone_position(ped, GPtr::from_raw(head.as_mut_ptr()).cast(), 5, 0);
    let simplest = tm_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    let pz = rd::<f32>(entity_pos(ped), 8);
    if lt(x(head[2]), x(pz))
        && ped_is_player(ped) == 0
        && !simplest.is_null()
        && (v_task_type(simplest) == 0xCF || v_task_type(simplest) == 0xCD)
    {
        set_fall();
        let a = clump_get_first_association_4d6a70(rd::<GPtr<u8>>(ped, 0x18), 0x800);
        set_anim(if a.is_null() { 0x24 } else { 0x27 });
    } else if rd::<u32>(this, 0x20) == 3 {
        let mut mult = false;
        if !use_gun.is_null() && rd::<u8>(use_gun, 0x10) == 5 {
            if dir() != 0 {
                mult = true;
            }
            if ped_is_player(ped) != 0 {
                mult = false;
            }
        } else if heavy && ped_is_player(ped) == 0 {
            mult = true;
        }
        if rd::<u8>(ped, 0x478) & 4 == 0
            && rd::<u32>(ped, 0x46C) & 0x400_0000 == 0
            && rd::<u32>(ped, 0xFC) == 0
            && mult
            && !rd::<GPtr<u8>>(this, 0x14).is_null()
        {
            set_fall();
            force = 5.0;
        } else if w() < 9 && lt(x(rd::<f32>(ped, 0x540)), x(threshold)) {
            set_fall();
            force = x(strength).fmul_m(gf(0x85_8C80)).to_f32();
        } else if (w() == 0x36 && {
            let s = rd::<GPtr<u8>>(this, 0x14);
            !s.is_null() && rd::<u8>(s, 0x36) & 7 == 4
        }) || (!fight.is_null() && rd::<u8>(fight, 0x25) == 4 && ped_is_player(ped) == 0 && rd::<i32>(ped, 0x534) > 4)
        {
            set_fall();
        }
    }
    let flags = rd::<u8>(this, 0x25);
    let mut no_combo = false;
    'anim: {
        if flags & 2 != 0
            && (dir() != 0
                || fight.is_null()
                || (fight_is_combo_set(fight) == 0 && fight_is_hit_combo_set_o(fight) == 0))
        {
            no_combo = true;
            break 'anim;
        }
        wr::<u8>(this, 0x25, flags & 0xFD);
        if dir() == 0 {
            if !fight.is_null() && rd::<i8>(fight, 0x24) >= 4 && rd::<i8>(fight, 0x25) <= 2 {
                wr::<u32>(this, 0x28, fight_get_combo_anim_group_id(fight));
                set_anim((i32::from(rd::<i8>(fight, 0x25)) + 0xDB) as u32);
                wr::<u32>(this, 0x30, 0x4180_0000);
                if fight_is_combo_set(fight) != 0 {
                    wr::<u8>(this, 0x25, (flags & 0xFD) | 2);
                }
                break 'anim;
            }
            if !use_gun.is_null() && rd::<u8>(use_gun, 0x10) == 5 {
                wr::<u32>(this, 0x28, g::<u32>(0xC1_70D0).read());
                set_anim(0xDC);
                wr::<u32>(this, 0x30, 0x4180_0000);
                break 'anim;
            }
        }
        let wt = w();
        let partial = (0x16..=0x21).contains(&wt) && {
            if heavy && ped_is_player(ped) == 0 {
                false
            } else if ped_is_player(ped) != 0 && !heavy {
                true
            } else {
                move_state > 1 || rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 || rd::<u8>(ped, 0x478) & 4 != 0
            }
        };
        if partial {
            set_anim(u32::from(dir()) + 0x1C);
            wr::<u32>(this, 0x28, 0);
            wr::<u32>(this, 0x30, 0x4100_0000);
            if ped_is_player(ped) != 0 {
                let pd = rd::<GPtr<u8>>(ped, 0x480);
                let t = g::<u32>(TIME_MS).read();
                if t <= rd::<u32>(pd, 0x28) || rd::<u32>(ped, 0x530) == 0x32 {
                    set_anim(0xBF);
                } else if rd::<i32>(this, 0x1C) == 0x1F {
                    wr(pd, 0x28, time_plus(t, gf(0x85_AB04)));
                } else {
                    wr(pd, 0x28, time_plus(t, gf(0x85_AB00)));
                }
            }
            break 'anim;
        }
        if ped_is_player(ped) != 0 && w() >= 0x16 {
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            let t = g::<u32>(TIME_MS).read();
            if t <= rd::<u32>(pd, 0x28) || rd::<u32>(ped, 0x530) == 0x32 {
                set_anim(0xBF);
                break 'anim;
            }
            wr(pd, 0x28, time_plus(t, gf(0x85_AB04)));
        }
        let wt = w();
        if (0x16..=0x2F).contains(&wt) {
            let mut cur: u32 = u32::MAX;
            let ev = history_get_current_event(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x38));
            if !ev.is_null() && v_event_type(ev) == 9 {
                cur = rd::<u32>(ev, 0x2C);
            }
            let anim = || rd::<u32>(this, 0x2C);
            let reroll = |base: u32, n: i32| {
                while anim() == cur {
                    set_anim((general_random_int_in_range(0, n) as u32).wrapping_add(base));
                }
            };
            match rd::<u32>(this, 0x20) {
                5 => {
                    set_anim(match dir() {
                        2 => 0xAB,
                        1 => 0xAD,
                        _ => 0xAC,
                    });
                    while anim() == cur {
                        let r = rand_at(0x4B_4528) & 0xFFFF;
                        let k = ftol32(xi(r).fmul_m(gf(0x85_8B14)).fmul_m(gf(0x85_9024)));
                        set_anim(0xABu32.wrapping_sub(k as u32));
                    }
                }
                6 => {
                    set_anim(match dir() {
                        2 => 0xAE,
                        3 => 0xB0,
                        _ => 0xAF,
                    });
                    reroll(0xAE, 3);
                }
                7 => {
                    set_anim(match dir() {
                        2 => 0xB1,
                        1 => 0xB3,
                        _ => 0xB2,
                    });
                    reroll(0xB1, 3);
                }
                8 => {
                    set_anim(match dir() {
                        2 => 0xB4,
                        3 => 0xB6,
                        _ => 0xB5,
                    });
                    reroll(0xB4, 3);
                }
                3 | 4 => {
                    match dir() {
                        0 => set_anim(0xB8),
                        1 => set_anim(0xB9),
                        2 => set_anim(0xB7),
                        3 => set_anim(0xBA),
                        _ => {}
                    }
                    reroll(0xB7, 4);
                }
                _ => set_anim(u32::from(dir()) + 0x20),
            }
        } else {
            let d = dir();
            set_anim(u32::from(d) + 0x20);
            if d == 0 {
                if gt(x(rd::<f32>(this, 0x3C)).fadd_m(rd::<f32>(this, 0x38)), x(gf(0x85_8BA4))) {
                    set_anim(0x25);
                }
            } else if d == 2 && wt <= 0xF {
                set_anim(0x28);
            }
        }
    }
    if abortable as u8 != 0 {
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        if !tm_get_active_task(tm).is_null() {
            let t = tm_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
            if v_make_abortable(t, ped, 1, this) == 0 {
                wr::<u8>(this, 0x25, rd::<u8>(this, 0x25) & 0xFD);
                wr::<u32>(this, 0x28, 0);
                set_anim(0xC0);
            }
        }
    }
    if rd::<u8>(this, 0x25) & 2 != 0 && no_combo {
        set_anim(u32::from(dir()) + 0x18);
        if gt(x(force), x(gf(0x85_8B50))) {
            let src = rd::<GPtr<u8>>(this, 0x14);
            let sp = entity_pos(src);
            let pp = entity_pos(ped);
            let mut d = [
                x(rd::<f32>(sp, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
                x(rd::<f32>(sp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
            ];
            vector2d_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
            wr(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FFFC);
            let fx = x(d[0]).fmul_m(force).fchs().to_f32();
            let fy = x(d[1]).fmul_m(force).fchs().to_f32();
            physical_apply_move_force_5429f0(ped, CVector { x: fx, y: fy, z: 5.0 });
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_IS_COMBO_SET, HOOK_FIGHT_IS_HIT_COMBO_SET, HOOK_COMPUTE_DAMAGE_ANIM];
