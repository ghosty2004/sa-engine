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

pub const HOOKS: &[HookReg] = &[
    HOOK_HIT_PED_WITH_CAR_CREATE_SUB_TASK,
    HOOK_BE_HIT_START_ANIM,
    HOOK_HIT_PED_WITH_CAR_HIT_HURTS_PED,
    HOOK_HIT_PED_WITH_CAR_COMPUTE_EVASIVE_STEP_MOVE_DIR,
];
