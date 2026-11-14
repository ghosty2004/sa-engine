//! WRITER-NATIVE (batch `native-24`): 0x61C740, `CTaskSimpleFight`'s strike damage (unnamed), called when a punch
//! lands; FRONTIER-1 (`frontier-1-2`): the hit itself (0x61CBA0) and the victim's block test (0x61C650). Ported
//! from the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use sa_core::orig::at_site;
use sa_world::world::common::entity_pos;

use crate::common::*;

use sa_stream::bindings::ped_is_player;
use sa_world::bindings::stats_get_fat_and_muscle_modifier;

/// `float CTaskSimpleFight::<strike damage>(CPed* ped)` (0x61C740, `ret 4`, ST0).
///
/// `d = (float)(uint8)[0xC17125 + (+0x24 - 4) * 0x88 + +0x25]` (the move table's damage byte). The player: 50 with
/// player-data (+0x480) flag +0x34 bit 1, else `GetFatAndMuscleModifier(4) * d`. Other peds by the current weapon
/// (+0x5A0 + slot +0x718 * 0x1C): brass knuckles (1) `d * 1.5`, fists (0) `d * pedStats(+0x59C) +0x28`, else `d`.
#[hook(addr = 0x61C740, conv = thiscall, name = "FUN_0061c740",
       expect = "51 0F BE 41 24 0F BE 49 25",
       unverified = "native-24", route = "L2", hazards = "none", recs = "weapon-combat",
       note = "ST0 result")]
pub fn fight_strike_damage(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let i = (i32::from(rd::<i8>(this, 0x24)) - 4).wrapping_mul(0x88) + i32::from(rd::<i8>(this, 0x25));
    let d = xi(i32::from(g::<u8>((0xC1_7125u32).wrapping_add(i as u32) as usize).read())).to_f32();
    if ped_is_player(ped) != 0 {
        if rd::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x34) & 2 != 0 {
            return x(50.0);
        }
        return X87::from_f64_exact(stats_get_fat_and_muscle_modifier(4)).fmul_m(d);
    }
    let slot = i32::from(rd::<i8>(ped, 0x718));
    let w = rd::<u32>(ped, (0x5A0 + slot * 0x1C) as isize);
    match w {
        1 => x(d).fmul_m(1.5),
        0 => x(d).fmul_m(rd::<f32>(rd::<GPtr<u8>>(ped, 0x59C), 0x28)),
        _ => x(d),
    }
}

// ---- FRONTIER-1 (batch `frontier-1-2`): a punch that reaches a ped ---------------------------------------------

use sa_core::bindings::get_weapon_info as weapon_info_get;
use sa_core::bindings::ped_is_alive;
use sa_core::bindings::tm_get_active_task;
use sa_core::bindings::vector_normalise;
use crate::bindings::event_sound_quiet_dtor;
use sa_world::bindings::fx_add_blood;
use sa_world::bindings::get_event_global_group;
use sa_world::bindings::intel_get_task_fighting;
use sa_world::bindings::ped_weapon_audio_add_audio_event;
use sa_world::bindings::event_sound_quiet_ctor;
use sa_world::bindings::generate_damage_event as weapon_generate_damage_event;
use crate::bindings::fight_strike_damage_c as fight_strike_damage_o;
orig! {
    /// `FUN_0061c650` (0x61C650): the victim's fight task blocks the hit
    fn fight_block_hit_o(t: GPtr<u8>, victim: GPtr<u8>, attacker: GPtr<u8>, combo: u32, level: u32) -> u8 = thiscall 0x61C650 => fight_block_hit;
    /// `CPedGeometryAnalyser::ComputePedShotSide(CPed&, const CVector&)` (0x5F13F0)
    fn compute_ped_shot_side(ped: GPtr<u8>, pos: GPtr<u8>) -> i32 = cdecl 0x5F13F0 => crate::ped_geometry::compute_ped_shot_side;
}

/// The fight move table entry of the task's combo (+0x24): `0xC170D0 + (combo - 4) * 0x88`.
fn move_entry(this: GPtr<u8>) -> GPtr<u8> {
    g::<u8>((0xC1_70D0i32 + (i32::from(rd::<i8>(this, 0x24)) - 4) * 0x88) as u32 as usize)
}

/// `CTaskSimpleFight::BeHitWhileBlocking` (0x61C650, `bool (CPed* victim, CPed* attacker, int8 combo, int8 hitLevel)`, `ret 0x10`, the
/// victim's fight task): blocks the attacker's hit. Only in block state (+0x27 == 2) with a block anim (+0x1C) not
/// yet flagged (+0x2E bit 0) whose hierarchy length (+0x14 +0x10) is beyond its time (+0x20); `facing` = the
/// victim's forward (matrix +0x10) dotted with `attacker - victim` (`(dz f.z + dy f.y) + dx f.x`) is not below
/// `[0x858C24]`. Kicks (combo 8, 10, 11, 12) are blocked only by the block combos 8..12 but 9; combo 9 by 8..12;
/// combo 7 at hit level 1 never. Blocked: the anim is flagged, true.
#[hook(addr = 0x61C650, conv = thiscall, name = "CTaskSimpleFight::BeHitWhileBlocking", expect = "80 79 27 02 53 55",
       unverified = "frontier-1-2", route = "L2", hazards = "heap-writes")]
pub fn fight_block_hit(this: GPtr<u8>, victim: GPtr<u8>, attacker: GPtr<u8>, combo: u32, level: u32) -> u8 {
    if rd::<u8>(this, 0x27) != 2 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if a.is_null() || rd::<u8>(a, 0x2E) & 1 != 0 {
        return 0;
    }
    if x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)).fcom(x(rd::<f32>(a, 0x20))).test_any(0x41) {
        return 0;
    }
    let m = rd::<GPtr<u8>>(victim, 0x14);
    let vp = entity_pos(victim);
    let ap = entity_pos(attacker);
    let dx = x(rd::<f32>(ap, 0)).fsub_m(rd::<f32>(vp, 0));
    let dy = x(rd::<f32>(ap, 4)).fsub_m(rd::<f32>(vp, 4));
    let dz = x(rd::<f32>(ap, 8)).fsub_m(rd::<f32>(vp, 8));
    let dot = dz
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(dy.fmul_m(rd::<f32>(m, 0x14)))
        .fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
    let behind = !dot.fcom(x(g::<f32>(0x85_8C24).read())).test_parity(0x05);
    let c = combo as u8;
    let own = rd::<i8>(this, 0x24);
    if matches!(c, 8 | 10 | 11 | 12) && (!(8..=12).contains(&own) || own == 9) {
        return 0;
    }
    if c == 9 {
        if !(8..=12).contains(&own) {
            return 0;
        }
    } else if c == 7 && level as u8 == 1 {
        return 0;
    }
    if behind {
        return 0;
    }
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    1
}

/// `CTaskSimpleFight::FightHitPed` (0x61CBA0, `CPed* (CPed* attacker, CPed* victim, CVector* pos, CVector* dir, int16 piece)`,
/// `ret 0x14`): a punch reaches `victim`. A player victim whose active task is 0xCD is not hit (null). When the
/// victim's fight task blocks it (0x61C650): the move table's block sound (+0x70, -9.0) and for combo 5 / level 0..2
/// a second one with `[0x8D2E3C + level * 4]`; null. Otherwise: the strike damage (0x61C740, `ftol`), +0x14 = 0xFF,
/// `GenerateDamageEvent(victim, attacker, weapon, damage, piece, ComputePedShotSide(victim, attacker))`; a chainsaw
/// (weapon 9) plays weapon sound 0x9B; the hit sounds by side (front: +0x5C, with the combo 5 / combo 7 level 1
/// extras; else +0x70); a player attacker says 0x59; a `CEventSoundQuiet(attacker, 55, -1, 0)` to the global
/// group; blood (`Fx_c::AddBlood` at `pos`, along `attacker - victim` normalised, scaled by `[0x858CE8]` for kicks
/// 8..12 on a live victim, straight up 2.0 on a dead one; 8 particles, 16 for kicks, light +0x12C) unless
/// `ftol((rand() & 0xFFFF) * [0x858B14] * [0x858628])` reaches the limit (100 for kicks, none for combo 4 level 4,
/// else `ftol([0x858628] - health)`). Returns the victim when the damage event was generated, else null.
///
/// Stack: the original reuses its own argument slots for the damage, the side, the "generated" flag and a sound
/// temporary; not reproduced.
#[hook(addr = 0x61CBA0, conv = thiscall, name = "CTaskSimpleFight::FightHitPed", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier-1-2", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand at_site 0x61CF57; the argument slots hold temporaries in the original; the SEH frame is not reproduced")]
#[allow(clippy::too_many_lines)]
pub fn fight_hit_ped(this: GPtr<u8>, attacker: GPtr<u8>, victim: GPtr<u8>, pos: GPtr<u8>, _dir: GPtr<u8>, piece: i32) -> GPtr<u8> {
    let audio = attacker.byte_add(0x138);
    let sound = |off: isize, idx: i32| rd::<u32>(move_entry(this), off + 4 * idx as isize);
    let extra = |idx: i32| g::<u32>((0x8D_2E3C + 4 * idx) as u32 as usize).read();
    if ped_is_player(victim) != 0
        && task_get_type(tm_get_active_task(rd::<GPtr<u8>>(victim, 0x47C).byte_add(4))) == 0xCD
    {
        return GPtr::NULL;
    }
    if !intel_get_task_fighting(rd::<GPtr<u8>>(victim, 0x47C)).is_null() {
        let combo = u32::from(rd::<u8>(this, 0x24));
        let level = u32::from(rd::<u8>(this, 0x25));
        let f = intel_get_task_fighting(rd::<GPtr<u8>>(victim, 0x47C));
        if fight_block_hit_o(f, victim, attacker, combo, level) != 0 {
            let idx = i32::from(rd::<i8>(this, 0x25));
            ped_audio_add_audio_event(audio, sound(0x70, idx), -9.0, 1.0, victim, 0, 0, 0);
            let l = rd::<i8>(this, 0x25);
            if rd::<u8>(this, 0x24) == 5 && (0..=2).contains(&l) {
                let i = i32::from(l);
                ped_audio_add_audio_event(audio, sound(0x70, i), -9.0, 1.0, victim, 0, 0, extra(i));
            }
            return GPtr::NULL;
        }
    }
    let wslot = |p: GPtr<u8>| rd::<u32>(p, (0x5A0 + i32::from(rd::<i8>(p, 0x718)) * 0x1C) as isize);
    weapon_info_get(wslot(attacker) as i32, 1);
    let damage = ftol32(fight_strike_damage_o(this, attacker));
    wr::<u8>(this, 0x14, 0xFF);
    let side = compute_ped_shot_side(victim, entity_pos(attacker));
    let piece16 = i32::from(piece as i16);
    let generated = weapon_generate_damage_event(victim, attacker, wslot(attacker) as i32, damage, piece16, side as u32) != 0;
    if wslot(attacker) == 9 {
        ped_weapon_audio_add_audio_event(attacker.byte_add(0x394), 0x9B);
    }
    let idx = || i32::from(rd::<i8>(this, 0x25));
    if side == 0 {
        let combo = rd::<u8>(this, 0x24);
        let l = rd::<i8>(this, 0x25);
        if combo == 5 && (0..=2).contains(&l) {
            let i = i32::from(l);
            ped_audio_add_audio_event(audio, sound(0x5C, i), 0.0, 1.0, victim, 0, 0, 0);
            ped_audio_add_audio_event(audio, sound(0x5C, idx()), 0.0, 1.0, victim, 0, 0, extra(idx()));
        } else if combo == 7 && l == 1 {
            ped_audio_add_audio_event(audio, sound(0x60, 0), 0.0, 1.0, victim, 0, 0, g::<u32>(0x8D_2E40).read());
            let e = extra(idx());
            let mut v = sa_math::x87::X87::from_i32(e as i32);
            if (e as i32) < 0 {
                v = v.fadd_m(g::<f32>(0x85_8C54).read());
            }
            let t = ftol32(v.fmul_m(g::<f32>(0x86_D6A8).read())) as u32;
            ped_audio_add_audio_event(audio, sound(0x5C, idx()), 0.0, 1.0, victim, 0, 0, t);
        } else {
            ped_audio_add_audio_event(audio, sound(0x5C, idx()), 0.0, 1.0, victim, 0, 0, 0);
        }
    } else {
        ped_audio_add_audio_event(audio, sound(0x70, idx()), 0.0, 1.0, victim, 0, 0, 0);
        let l = rd::<i8>(this, 0x25);
        if rd::<u8>(this, 0x24) == 5 && (0..=2).contains(&l) {
            let i = i32::from(l);
            ped_audio_add_audio_event(audio, sound(0x70, i), 0.0, 1.0, victim, 0, 0, extra(i));
        }
    }
    if ped_is_player(attacker) != 0 {
        ped_say(attacker, 0x59, 0, 1.0, 0, 0, 0);
    }
    let mut zero = [0u32; 3];
    let mut ev = [0u32; 12];
    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_sound_quiet_ctor(ep, attacker, 55.0, -1i32 as u32, GPtr::from_raw(zero.as_mut_ptr()).cast());
    event_group_add(get_event_global_group(), ep, 0);
    let combo = rd::<u8>(this, 0x24);
    let kick = (8..=12).contains(&combo);
    let limit = if kick {
        100
    } else if combo == 4 && rd::<u8>(this, 0x25) == 4 {
        -1
    } else {
        ftol32(x(g::<f32>(0x85_8628).read()).fsub_m(rd::<f32>(victim, 0x540)))
    };
    let r = at_site(0x61_CF57, rand) & 0xFFFF;
    let roll = ftol32(xi(r).fmul_m(g::<f32>(0x85_8B14).read()).fmul_m(g::<f32>(0x85_8628).read()));
    if roll < limit {
        let mut p = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
        let vp = entity_pos(victim);
        let ap = entity_pos(attacker);
        let mut d = [
            x(rd::<f32>(ap, 0)).fsub_m(rd::<f32>(vp, 0)).to_f32(),
            x(rd::<f32>(ap, 4)).fsub_m(rd::<f32>(vp, 4)).to_f32(),
            x(rd::<f32>(ap, 8)).fsub_m(rd::<f32>(vp, 8)).to_f32(),
        ];
        let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
        vector_normalise(dp.cast::<[f32;3]>());
        if ped_is_alive(victim) as u8 == 0 {
            d = [0.0, 0.0, 2.0];
        }
        let mut amount = 8;
        if kick {
            amount = 0x10;
            if ped_is_alive(victim) as u8 != 0 {
                let k = g::<f32>(0x85_8CE8).read();
                for v in d.iter_mut() {
                    *v = x(*v).fmul_m(k).to_f32();
                }
            }
        }
        fx_add_blood(
            g::<u8>(0xA9_AE00),
            GPtr::from_raw(p.as_mut_ptr()).cast(),
            GPtr::from_raw(d.as_mut_ptr()).cast(),
            amount,
            rd::<f32>(victim, 0x12C),
        );
    }
    event_sound_quiet_dtor(ep);
    if generated { victim } else { GPtr::NULL }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_STRIKE_DAMAGE, HOOK_FIGHT_BLOCK_HIT, HOOK_FIGHT_HIT_PED];
