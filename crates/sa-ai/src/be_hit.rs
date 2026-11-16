//! FRONTIER-2 (batch `frontier2-1`): `CTaskSimpleBeHit::StartAnim` (0x620910), reached from the melee scenario once a
//! punch lands. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_core::bindings::task_new;
use sa_core::bindings::clump_get_association;
use sa_anim::bindings::{assoc_set_current_time, assoc_set_finish_callback};
use sa_world::bindings::{anim_blend_animation, intel_get_task_fighting};

use crate::common::*;
use crate::bindings::{compute_entity_hit_side_ped, evasive_step_ctor, fall_and_get_up_ctor, leave_group_ctor};
use sa_math::fpu::{gt, lt};
use crate::bindings::compute_ped_hit_side;
use sa_core::bindings::vector_normalise;
use sa_world::bindings::{
    damage_calc_compute_damage_response, damage_calc_ctor, damage_calc_dtor, event_damage_affects_ped, event_damage_ctor,
    event_damage_dtor, event_group_add, physical_apply_move_force_5429f0,
};
use sa_world::world::common::entity_pos;
use sa_core::bindings::general_random_int_in_range;
use sa_core::timer::{fild_u32, FRAME_COUNTER, TIME_MS};
use sa_world::bindings::{clear_aim_flag, ped_clear_look_flag};

/// `CTaskSimpleBeHit::StartAnim(CPed* ped)` (0x620910, `ret 4`).
///
/// With no anim chosen yet (+0x10 == 0xBF): a fight-combo hit from the attacker (+8, direction +0x20 == 0, its fight
/// task's combo set (+0x24, signed) >= 4 and move (+0x25, signed) <= 2) takes move + 0xDB in the combo's group;
/// otherwise hit side +0x18 == 3 → 0x20, direction 1 / 2 / 3 → 0x21 / 0x22 / 0x23, else 0x20 when +0x1C <= 5 (signed)
/// and 0x25 above. Not started (+0xD clear): `BlendAnimation(clump, group, anim, 8.0)`, time 0, started; else the
/// clump's running association of the anim (none: +0xC set, nothing else). The association (+0x24) gets
/// `FinishAnimBeHitCB` (0x620900) with the task.
#[hook(addr = 0x620910, conv = thiscall, name = "CTaskSimpleBeHit::StartAnim", expect = "56 8B F1 81 7E 10 BF 00 00 00",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn be_hit_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(this, 0x10) == 0xBF {
        let mut combo = false;
        let attacker = rd::<GPtr<u8>>(this, 8);
        if rd::<u32>(this, 0x20) == 0 && !attacker.is_null() {
            let f = intel_get_task_fighting(rd::<GPtr<u8>>(attacker, 0x47C));
            if !f.is_null() {
                let c = rd::<i8>(f, 0x24);
                let m = rd::<i8>(f, 0x25);
                if c >= 4 && m <= 2 {
                    wr::<i32>(this, 0x10, i32::from(m) + 0xDB);
                    let row = (i32::from(c) - 4).wrapping_mul(0x88);
                    wr::<u32>(this, 0x14, g::<u32>((0xC1_70D0 + row) as u32 as usize).read());
                    combo = true;
                }
            }
        }
        if !combo {
            let a = if rd::<u32>(this, 0x18) == 3 {
                0x20
            } else {
                match rd::<u32>(this, 0x20) {
                    1 => 0x21,
                    2 => 0x22,
                    3 => 0x23,
                    _ => {
                        if rd::<i32>(this, 0x1C) <= 5 {
                            0x20
                        } else {
                            0x25
                        }
                    }
                }
            };
            wr::<u32>(this, 0x10, a);
        }
    }
    if rd::<u8>(this, 0xD) == 0 {
        let a = anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), rd::<u32>(this, 0x14), rd::<u32>(this, 0x10), 8.0);
        wr(this, 0x24, a);
        assoc_set_current_time(a, 0.0);
        wr::<u8>(this, 0xD, 1);
    } else {
        let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), rd::<u32>(this, 0x10));
        wr(this, 0x24, a);
        if a.is_null() {
            wr::<u8>(this, 0xC, 1);
            return;
        }
    }
    assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x24), 0x62_0900, this.addr() as u32);
}

/// The finish callback `CTaskSimpleBeHitWhileMoving`'s anim gets (0x61FF30: clears +0x14, sets +8). The original pushes
/// it through the SecuROM fragment 0x401122: `ror([0x15605F8], [0x698EFA])` on the exe image = 0x61FF30.
const BE_HIT_WHILE_MOVING_CB: u32 = 0x61_FF30;

/// The anim of 0x61FF40 is `anim` in group 0 (+0x14): the clump's running association of it unless there is none or
/// its blend delta (+0x1C) is below 0 (ordered), in which case `BlendAnimation(clump, 0, anim, 8.0)`; then time 0 and
/// the flags +0x2E |= 1, |= 8 (two read-modify-writes).
fn be_hit_while_moving_restart(this: GPtr<u8>, ped: GPtr<u8>, anim: u32) {
    let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), anim);
    wr(this, 0x14, a);
    if a.is_null() || lt(x(rd::<f32>(a, 0x1C)), x(0.0)) {
        wr(this, 0x14, anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, anim, 8.0));
    }
    assoc_set_current_time(rd::<GPtr<u8>>(this, 0x14), 0.0);
    let a = rd::<GPtr<u8>>(this, 0x14);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
    let a = rd::<GPtr<u8>>(this, 0x14);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 8);
}

/// `(ped +0x20 (u16) + CTimer::m_FrameCounter) % 1000`, unsigned.
fn be_hit_while_moving_frame_jitter(ped: GPtr<u8>) -> u32 {
    u32::from(rd::<u16>(ped, 0x20)).wrapping_add(FRAME_COUNTER.get()) % 1000
}

/// `CTaskSimpleBeHitWhileMoving::StartAnim(CPed* ped)` (0x61FF40, `ret 4`; unnamed in the catalogue, task type 1009,
/// vtable 0x86D7FC, called by its ProcessPed 0x620290 while +0x14 is null). Fields: +0xC hit type, +0x10 direction,
/// +0x14 the association.
///
/// Hit type 4 or 2 with (non-player and `rand() & 1`) or `rand() & 7 == 0`: by direction 1 / 2 / 3 → 0x19, 0x22 or
/// 0x1A (`rand() & 1`), 0x1B; otherwise 0x14 for type 2, else 0x25 / 0x20 / 0x13 (`rand() & 1` twice). 0x22 / 0x25 /
/// 0x20 restart that anim (see `be_hit_while_moving_restart`). The others, when the ped's +0xFC is clear:
/// `ClearLookFlag`, `ClearAimFlag`, the clump's running association of the anim gets time 0, flag +0x2E |= 1, blend
/// +0x18 = 0, delta +0x1C = 8.0 (none: +0x14 = `BlendAnimation(clump, 0, anim, 8.0)`; the 0xBF branch for the
/// player's 0x6E / 0x6F is unreachable but kept); then the +0x744 timer: the player's from that association (0x6E /
/// 0x6F: `+ 100 - its +0x20 * 1000`, else `+ 500`), a non-player's from the frame jitter `+ 500`, and without the
/// association `jitter + now + 1000`. Any other case: a non-player with `rand() & 1` clear only draws
/// `GetRandomNumberInRange(1000, 3000)` (result unused); otherwise direction 1 / 2 / 3 / else → anim 0x1D / 0x1E /
/// 0x1F / 0x1C restarted. Every path ends with `SetFinishCallback(+0x14, 0x61FF30, this)`, also those that leave +0x14
/// null (as the original, which then writes through null + 0x30). Returns EAX = the callback (SetFinishCallback's
/// first argument).
#[hook(addr = 0x61FF40, conv = thiscall, name = "FUN_0061ff40", expect = "53 55 8B E9 8B 45 0C",
       unverified = "frontier2-c13", route = "L2", hazards = "heap-writes")]
pub fn be_hit_while_moving_start_anim(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let ty = rd::<u32>(this, 0xC);
    let dir = rd::<u32>(this, 0x10);
    let main = (ty == 4 || ty == 2) && ((ped_is_player(ped) == 0 && rand() & 1 != 0) || rand() & 7 == 0);
    if main {
        let anim = match dir {
            1 => Ok(0x19),
            2 => if rand() & 1 != 0 { Err(0x22) } else { Ok(0x1A) },
            3 => Ok(0x1B),
            _ if rd::<u32>(this, 0xC) == 2 => Ok(0x14),
            _ => if rand() & 1 != 0 { Err(0x25) } else if rand() & 1 != 0 { Err(0x20) } else { Ok(0x13) },
        };
        match anim {
            Err(a) => be_hit_while_moving_restart(this, ped, a),
            Ok(anim) if rd::<u32>(ped, 0xFC) == 0 => {
                ped_clear_look_flag(ped);
                clear_aim_flag(ped);
                let clump = rd::<GPtr<u8>>(ped, 0x18);
                let mut a = GPtr::<u8>::NULL;
                if anim == 0xBF {
                    if ped_is_player(ped) != 0 {
                        a = clump_get_association(clump, 0x6E);
                        if a.is_null() {
                            a = clump_get_association(clump, 0x6F);
                        }
                    }
                } else {
                    a = clump_get_association(clump, anim);
                    if a.is_null() {
                        wr(this, 0x14, anim_blend_animation(clump, 0, anim, 8.0));
                    } else {
                        assoc_set_current_time(a, 0.0);
                        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 1);
                        wr::<u32>(a, 0x18, 0);
                        wr::<u32>(a, 0x1C, 0x4100_0000);
                    }
                }
                if a.is_null() {
                    let v = be_hit_while_moving_frame_jitter(ped).wrapping_add(TIME_MS.get()).wrapping_add(1000);
                    wr::<u32>(ped, 0x744, v);
                } else {
                    // the anim's total time (+0x14 hierarchy +0x10) in ms
                    let t = x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)).fmul_m(1000.0);
                    let v = if ped_is_player(ped) == 0 {
                        xi(be_hit_while_moving_frame_jitter(ped) as i32).fadd(t).fadd(fild_u32(TIME_MS.get())).fadd_m(500.0)
                    } else if matches!(rd::<u16>(a, 0x2C), 0x6E | 0x6F) {
                        t.fadd(fild_u32(TIME_MS.get())).fsub(x(rd::<f32>(a, 0x20)).fmul_m(1000.0)).fadd_m(100.0)
                    } else {
                        t.fadd(fild_u32(TIME_MS.get())).fadd_m(500.0)
                    };
                    wr::<i32>(ped, 0x744, ftol32(v));
                }
            }
            Ok(_) => {}
        }
    } else if ped_is_player(ped) == 0 && rand() & 1 == 0 {
        general_random_int_in_range(1000, 3000);
    } else {
        let anim = match dir {
            1 => 0x1D,
            2 => 0x1E,
            3 => 0x1F,
            _ => 0x1C,
        };
        be_hit_while_moving_restart(this, ped, anim);
    }
    assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), BE_HIT_WHILE_MOVING_CB, this.addr() as u32);
    BE_HIT_WHILE_MOVING_CB
}

/// `CTaskComplexHitPedWithCar::HitHurtsPed(CPed* ped)` (0x653AE0, `ret 4`): true when the impact speed (+0x10) is above
/// `[0x86F124]` (player) / `[0x86F11C]` (others), or when the ped's +0xE8 is below `[0x858C9C]` and the speed above
/// `[0x858B3C]`.
#[hook(addr = 0x653AE0, conv = thiscall, name = "CTaskComplexHitPedWithCar::HitHurtsPed", expect = "56 57 8B 7C 24 0C 8B F1",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn hit_ped_with_car_hit_hurts_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let k = if ped_is_player(ped) != 0 { g::<f32>(0x86_F124).read() } else { g::<f32>(0x86_F11C).read() };
    let speed = rd::<f32>(this, 0x10);
    if lt(x(k), x(speed)) {
        return 1;
    }
    if lt(x(rd::<f32>(ped, 0xE8)), x(g::<f32>(0x85_8C9C).read())) && gt(x(speed), x(g::<f32>(0x85_8B3C).read())) {
        return 1;
    }
    0
}

/// `CTaskComplexHitPedWithCar::ComputeEvasiveStepMoveDir(CVector* out, const CPed& ped, const CVehicle& veh)`
/// (0x653B40, cdecl, returns `out`): by the vehicle side the ped is on (`ComputeEntityHitSide`), the vehicle's forward
/// (side 0) or right (3) axis, or the right (1) / forward (2) axis times `[0x858C1C]`. Another side copies the
/// uninitialised local (zero here).
#[hook(addr = 0x653B40, conv = cdecl, name = "CTaskComplexHitPedWithCar::ComputeEvasiveStepMoveDir", expect = "8B 44 24 08 83 EC 0C 56",
       unverified = "frontier2-1", route = "L2", hazards = "uninit-locals")]
pub fn hit_ped_with_car_compute_evasive_step_move_dir(out: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> {
    let mut v = [0u32; 3];
    let side = compute_entity_hit_side_ped(ped, veh) as u32;
    let axis = |off: isize| {
        let m = rd::<GPtr<u8>>(veh, 0x14).byte_add(off);
        [rd::<u32>(m, 0), rd::<u32>(m, 4), rd::<u32>(m, 8)]
    };
    let scaled = |a: [u32; 3]| {
        let k = g::<f32>(0x85_8C1C).read();
        a.map(|c| x(f32::from_bits(c)).fmul_m(k).to_f32().to_bits())
    };
    match side {
        0 => v = axis(0x10),
        1 => v = scaled(axis(0)),
        2 => v = scaled(axis(0x10)),
        3 => v = axis(0),
        _ => {}
    }
    wr(out, 0, v[0]);
    wr(out, 4, v[1]);
    wr(out, 8, v[2]);
    out
}

orig! {
    /// `CTaskSimpleHitFromBehind::Constructor()` (0x652EE0)
    fn hit_from_behind_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x652EE0 => unported;
    /// `CTaskSimpleKillPedWithCar::Constructor(CVehicle*, float)` (0x653C10)
    fn kill_ped_with_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, speed: u32) -> GPtr<u8> = thiscall 0x653C10 => unported;
    /// `CTaskSimpleHurtPedWithCar::Constructor(CVehicle*, float)` (0x653EA0)
    fn hurt_ped_with_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, speed: u32) -> GPtr<u8> = thiscall 0x653EA0 => unported;
}

/// `CTaskComplexHitPedWithCar::CreateSubTask(int32 type)` (0x6560E0, `ret 4`; the entry `nop; jmp` runs its first
/// instruction in the no-CD chunk 0x40216C): 0x1F6 `CTaskComplexEvasiveStep(vehicle +0xC, &dir +0x1C)`, 0xC8
/// `CTaskSimpleLeaveGroup`, 0xD0 `CTaskComplexFallAndGetUp(anim by hit side +0x14: 0x1A / 0x19 / 0x18 / 0x1B, else the
/// type itself (the argument slot), group 0, time +0x18)`, 0x19E `CTaskSimpleHitFromBehind`, 0x1FA
/// `CTaskSimpleKillPedWithCar(+0xC, +0x10)`, 0x1FB `CTaskSimpleHurtPedWithCar(+0xC, +0x10)`; anything else (and a
/// failed allocation) null.
#[hook(addr = 0x6560E0, conv = thiscall, name = "CTaskComplexHitPedWithCar::CreateSubTask", expect = "90 E9 86 C0 DA FF",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes", note = "the SEH frame is not reproduced")]
pub fn hit_ped_with_car_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let make = |size: u32, f: &dyn Fn(GPtr<u8>) -> GPtr<u8>| {
        let t = task_new(size);
        if t.is_null() { GPtr::NULL } else { f(t) }
    };
    match ty {
        0x1F6 => make(0x1C, &|t| evasive_step_ctor(t, rd::<GPtr<u8>>(this, 0xC), this.byte_add(0x1C))),
        0xC8 => make(8, &|t| leave_group_ctor(t)),
        0xD0 => {
            let anim = match rd::<u32>(this, 0x14) {
                0 => 0x1A,
                1 => 0x19,
                2 => 0x18,
                3 => 0x1B,
                _ => ty as u32,
            };
            make(0x18, &|t| fall_and_get_up_ctor(t, anim, 0, rd::<u32>(this, 0x18)))
        }
        0x19E => make(0x20, &|t| hit_from_behind_ctor(t)),
        0x1FA => make(0x10, &|t| kill_ped_with_car_ctor(t, rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x10))),
        0x1FB => make(0x14, &|t| hurt_ped_with_car_ctor(t, rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x10))),
        _ => GPtr::NULL,
    }
}

/// `CTaskSimpleHurtPedWithCar::ProcessPed(CPed*)` (0x653F90, `ret 4`, AL = 1; FRONTIER-2 part 4, the crossing run).
/// The car (+8) hits the ped: a stack `CPedDamageResponseCalculator(car, 30.0 ([0x86F134]), 0x31 rammed by car, 3
/// torso, false)` and `CEventDamage(car, now, 0x31, 3, ComputePedHitSide(ped, car), false, (ped +0x46C >> 8) &
/// 0xFFFFFF01)` (the in-vehicle bit with the flag word's upper bits, as pushed); when the event affects the ped its
/// damage response is computed (`ComputeDamageResponse(ped, &event.response (+0x34), true)`); +0x10 = the response's
/// health-zero byte (event +0x40) and then the event joins the ped's group. The ped's +0x46C bit 0 and +0x470 bit 4 are
/// cleared, its move speed = (car.x, car.y) * 0.75 with z 0 (z * 0.75 stored first); the car gets
/// `ApplyMoveForce(normalise(ped - car) * (min(mass / 1400, 1) * -60))` (`1.0 < k` takes 1). The event and the
/// calculator are destroyed.
#[hook(addr = 0x653F90, conv = thiscall, name = "CTaskSimpleHurtPedWithCar::ProcessPed", expect = "6A FF 68 A0 35 84 00",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
pub fn hurt_ped_with_car_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let car = rd::<GPtr<u8>>(this, 8);
    let side = compute_ped_hit_side(ped, car);
    let mut calc = [0u32; 8];
    let cp: GPtr<u8> = GPtr::from_raw(calc.as_mut_ptr()).cast();
    damage_calc_ctor(cp, rd::<u32>(this, 8), g::<u32>(0x86_F134).read(), 0x31, 3, 0);
    let mut ev = [0u32; 0x14];
    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(ep, rd::<GPtr<u8>>(this, 8), g::<u32>(0xB7_CB84).read(), 0x31, 3, side, 0, in_veh);
    if event_damage_affects_ped(ep, ped) != 0 {
        damage_calc_compute_damage_response(cp, ped, ep.byte_add(0x34), 1);
    }
    let kill = rd::<u8>(ep, 0x40);
    wr::<u8>(this, 0x10, kill);
    if kill != 0 {
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    }
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) & 0xFFFF_FFEF);
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FFFE);
    let car = rd::<GPtr<u8>>(this, 8);
    let k75 = sa_core::field::gf(0x85_8F34);
    let mz = x(rd::<f32>(car, 0x4C)).fmul_m(k75);
    let my = x(rd::<f32>(car, 0x48)).fmul_m(k75);
    let mx = x(rd::<f32>(car, 0x44)).fmul_m(k75);
    wr::<f32>(ped, 0x44, mx.to_f32());
    wr::<f32>(ped, 0x48, my.to_f32());
    wr::<f32>(ped, 0x4C, mz.to_f32());
    wr::<u32>(ped, 0x4C, 0);
    let car = rd::<GPtr<u8>>(this, 8);
    let (pp, cpos) = (entity_pos(ped), entity_pos(car));
    let mut d = [0f32; 3];
    for (i, v) in d.iter_mut().enumerate() {
        *v = x(rd::<f32>(pp, 4 * i as isize)).fsub_m(rd::<f32>(cpos, 4 * i as isize)).to_f32();
    }
    vector_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
    let car = rd::<GPtr<u8>>(this, 8);
    let mut k = x(rd::<f32>(car, 0x8C)).fmul_m(sa_core::field::gf(0x86_4F98));
    if lt(x(1.0), k) {
        k = x(1.0);
    }
    let k = k.fmul_m(sa_core::field::gf(0x86_EBC4));
    let f = sa_math::vector::CVector::new(x(d[0]).fmul(k).to_f32(), x(d[1]).fmul(k).to_f32(), x(d[2]).fmul(k).to_f32());
    physical_apply_move_force_5429f0(car, f);
    event_damage_dtor(ep);
    damage_calc_dtor(cp);
    1
}


pub const HOOKS: &[HookReg] = &[
    HOOK_HURT_PED_WITH_CAR_PROCESS_PED,
    HOOK_HIT_PED_WITH_CAR_CREATE_SUB_TASK,
    HOOK_BE_HIT_START_ANIM,
    HOOK_BE_HIT_WHILE_MOVING_START_ANIM,
    HOOK_HIT_PED_WITH_CAR_HIT_HURTS_PED,
    HOOK_HIT_PED_WITH_CAR_COMPUTE_EVASIVE_STEP_MOVE_DIR,
];
