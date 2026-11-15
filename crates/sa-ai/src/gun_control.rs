//! FRONTIER-2 (batch `frontier2-1`): `CTaskSimpleGunControl::ProcessPed` (0x625270), the AI ped's gun loop (left open by
//! FRONTIER-1). Ported from the 1.0 US bytes; gta-reversed differs, the bytes are followed: a burst recomputes its
//! length with a second `RandomInt(0, 2)` when the first result is not below 1, the task finishes only for the
//! listed use-gun states (not always), and the leisure test reads the original's byte flag.

use sa_core::abi::HookReg;
use sa_core::bindings::{
    clump_get_association, general_random_int_in_range, get_weapon_info, intel_get_task_use_gun, task_new,
    tm_get_task_secondary, tm_set_task_secondary,
};
use sa_core::crt::rand_at;
use sa_core::field::gf;
use sa_core::{hook, orig};
use sa_math::fpu::{ge_or_nan, le_or_nan};
use sa_world::bindings::{anim_blend_animation, intel_get_task_duck, ped_get_weapon_skill, ped_set_move_state};
use sa_world::world::common::entity_pos;

use crate::bindings::{
    duck_force_stop_move, use_gun_control_gun, use_gun_control_gun_move, use_gun_ctor, use_gun_require_pistol_whip,
};
use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`, `CTimer::ms_fTimeStep`.
const TIME_MS: usize = 0xB7_CB84;
const TIME_STEP: usize = 0xB7_CB5C;

orig! {
    /// `CTaskSimpleUseGun::Reset(CPed*, CEntity*, CVector, int8 command, int16 burst)` (0x624DC0)
    fn use_gun_reset(t: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, x: u32, y: u32, z: u32, cmd: u32, burst: u32) -> u8
        = thiscall 0x624DC0 => unported;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// virtual `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
}

/// The original's `ForceStopDuckingMove`: a ducking ped (+0x46C bit 26) with a duck task stops its move.
fn force_stop_ducking_move(ped: GPtr<u8>) {
    if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 && !intel_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1).is_null() {
        duck_force_stop_move(intel_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1));
    }
}

/// `CTaskSimpleGunControl::ProcessPed(CPed* ped)` (0x625270, `ret 4`).
///
/// Fields: +8 finished, +0xC target, +0x10 target position, +0x28 leisure time, +0x2C burst (int16), +0x2E firing
/// command (int8), +0x30 attack interval multiplier, +0x34 next attack time, +0x38 first time, +0x39 LOS blocked,
/// +0x3A aim immediately. See the module docs for where the bytes differ from gta-reversed's reading.
#[hook(addr = 0x625270, conv = thiscall, name = "CTaskSimpleGunControl::ProcessPed", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "frontier2-1", route = "L2", hazards = "x87-transcendental,heap-writes,stack-residue",
       note = "rand at_site 0x62573E; the leisure flag and two temporaries live in the argument slot; the SEH frame is not reproduced")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn gun_control_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    let tm = || intel().byte_add(4);
    if rd::<u8>(this, 8) != 0 {
        let ug = intel_get_task_use_gun(intel());
        if !ug.is_null() {
            let c = rd::<u8>(ug, 0x38);
            wr::<u8>(ug, 0x38, if c < 4 { c } else { 4 });
            wr::<u8>(ug, 0xB, 0);
        }
        force_stop_ducking_move(ped);
        return 1;
    }
    ped_set_move_state(ped, 1);
    let mut cmd: u32 = 0;
    let mut dont_check = false;
    let target_pos = || (rd::<u32>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18));
    if rd::<i8>(this, 0x2E) < 6 {
        if tm_get_task_secondary(tm(), 0).is_null() {
            let t = task_new(0x3C);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let (px, py, pz) = target_pos();
                use_gun_ctor(t, rd::<GPtr<u8>>(this, 0xC), px, py, pz, 1, u32::from(rd::<u16>(this, 0x2C)),
                             u32::from(rd::<u8>(this, 0x3A)))
            };
            tm_set_task_secondary(tm(), t, 0);
            wr::<u32>(this, 0x34, 0);
            wr::<u8>(this, 0x38, 0);
            wr::<u8>(this, 0x3A, 0);
        } else if v_task_type(tm_get_task_secondary(tm(), 0)) != 0x3F9 {
            v_make_abortable(tm_get_task_secondary(tm(), 0), ped, 1, GPtr::NULL);
            return 0;
        } else if rd::<u8>(this, 0x38) != 0 {
            wr::<u32>(this, 0x34, 0);
            wr::<u8>(this, 0x38, 0);
            let sec = tm_get_task_secondary(tm(), 0);
            let (px, py, pz) = target_pos();
            use_gun_reset(sec, ped, rd::<GPtr<u8>>(this, 0xC), px, py, pz, 1, u32::from(rd::<u16>(this, 0x2C)));
        }
    }
    let ug = intel_get_task_use_gun(intel());
    let ty = rd::<i32>(ped, 0x598);
    let gunstand = if (7..=15).contains(&ty) {
        true
    } else if ty == 6 && rd::<u32>(ped, 0x534) == 1 && rd::<u32>(ped, 0x538) == 1 && !ug.is_null() {
        let wi = rd::<GPtr<u8>>(ug, 0x30);
        !wi.is_null() && (rd::<u32>(wi, 0x18) >> 1) & 1 != 0
    } else {
        false
    };
    if gunstand {
        let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0xBE);
        let blend = a.is_null()
            || (!ge_or_nan(x(rd::<f32>(a, 0x18)), x(gf(0x85_8624)))
                && !x(rd::<f32>(a, 0x1C)).fcom(x(gf(0x85_8B50))).test_parity(0x41));
        if blend {
            anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, 0xBE, 4.0);
        }
    }
    let mut winfo = GPtr::<u8>::NULL;
    if !ug.is_null() {
        wr::<u8>(this, 0x39, u8::from(rd::<u8>(ug, 0xC) != 0));
        winfo = rd::<GPtr<u8>>(ug, 0x30);
    }
    if winfo.is_null() {
        let slot = isize::from(rd::<i8>(ped, 0x718));
        let wt = rd::<i32>(ped, 0x5A0 + slot * 0x1C);
        winfo = get_weapon_info(wt, ped_get_weapon_skill(ped));
    }
    let slot = isize::from(rd::<i8>(ped, 0x718));
    if rd::<u32>(ped, 0x5A4 + slot * 0x1C) == 2 && (rd::<u32>(winfo, 0x18) >> 12) & 1 != 0 {
        cmd = 4;
    } else {
        let fc = rd::<i8>(this, 0x2E);
        match fc {
            0 => cmd = 1,
            1 | 2 => {
                if rd::<u32>(this, 0x34) == 0 {
                    cmd = 3;
                } else {
                    cmd = 1;
                    if fc == 1 {
                        dont_check = true;
                    }
                }
            }
            3 => {
                let next = rd::<u32>(this, 0x34);
                if g::<u32>(TIME_MS).read() < next {
                    if next == u32::MAX && !ug.is_null() && rd::<u8>(ug, 0x10) != 3 {
                        wr::<u32>(this, 0x34, 0);
                    }
                    cmd = 1;
                } else {
                    cmd = 3;
                    wr::<u32>(this, 0x34, u32::MAX);
                    let clip = rd::<i16>(winfo, 0x20);
                    wr(this, 0x2C, clip);
                    if clip > 1 && rd::<u8>(ped, 0x719) < 100 {
                        let v = i32::from(rd::<u8>(ped, 0x719)) * 4 / 30;
                        let m = if i32::from(clip) < v { i32::from(clip) } else { v };
                        wr(this, 0x2C, m as i16);
                        let r = general_random_int_in_range(0, 2);
                        let mut c = i32::from(rd::<i16>(this, 0x2C)) - r;
                        if c < 1 {
                            c = 1;
                        } else {
                            let r2 = general_random_int_in_range(0, 2);
                            c = i32::from(rd::<i16>(this, 0x2C)) - r2;
                        }
                        wr(this, 0x2C, c as i16);
                    }
                    if !ug.is_null() {
                        wr(ug, 0x34, rd::<u16>(this, 0x2C));
                    }
                }
            }
            4 => {
                if !ug.is_null() && rd::<u8>(ug, 0x10) == 4 {
                    cmd = 6;
                    wr::<i8>(this, 0x2E, 6);
                } else {
                    cmd = 3;
                }
            }
            5 => cmd = 3,
            6 => {
                cmd = 6;
                dont_check = true;
            }
            _ => {}
        }
    }
    let leisure = rd::<i32>(this, 0x28);
    if leisure > -1 {
        let t = ftol32(x(gf(TIME_STEP)).fmul_m(gf(0x85_8B38)).fmul_m(gf(0x85_8C4C)));
        let c = leisure.wrapping_sub(t);
        wr(this, 0x28, if c < 0 { 0 } else { c });
    }
    if dont_check || rd::<i32>(this, 0x28) == 0 {
        let finish = if ug.is_null() || (rd::<i8>(this, 0x2E) == 6 && rd::<u8>(ug, 8) != 0) {
            true
        } else if rd::<u8>(ug, 0xB) != 0 && !matches!(rd::<u8>(ug, 0x10), 2 | 3) {
            wr::<u8>(ug, 0x38, 2);
            wr::<u8>(ug, 0xB, 0);
            true
        } else {
            false
        };
        if finish {
            wr::<u8>(this, 8, 1);
        }
        if rd::<u8>(this, 8) != 0 {
            force_stop_ducking_move(ped);
            return 1;
        }
        if rd::<i32>(this, 0x28) == 0 && !ug.is_null() {
            wr::<i8>(this, 0x2E, 6);
            use_gun_control_gun(ug, ped, rd::<GPtr<u8>>(this, 0xC), 6);
        }
    }
    if rd::<u32>(this, 0x34) == 0 {
        let p = g::<u32>(0x8D_2E78).read();
        wr(this, 0x34, p);
        if rd::<u32>(winfo, 0) == 2 {
            wr(this, 0x34, p.wrapping_mul(2));
        }
        let r = rand_at(0x62_573E);
        let n = rd::<u32>(this, 0x34);
        let a = xi(r).fmul_m(gf(0x85_8C7C)).fmul_m(gf(0x85_8B8C)).fadd_m(gf(0x85_8F34));
        let mut b = xi(n as i32);
        if (n as i32) < 0 {
            b = b.fadd_m(gf(0x85_8C54));
        }
        let c = xi(i32::from(rd::<u8>(ped, 0x719))).fmul_m(rd::<f32>(this, 0x30)).fmul_m(gf(0x85_8CEC));
        let t = ftol32(a.fmul(b).fdiv(c));
        wr(this, 0x34, (t as u32).wrapping_add(g::<u32>(TIME_MS).read()));
    }
    let mut blocked = false;
    if cmd as u16 == 3 && use_gun_require_pistol_whip(ped, rd::<GPtr<u8>>(this, 0xC)) != 0 {
        cmd = 5;
    } else {
        let ducking = (rd::<u32>(ped, 0x46C) >> 0x1A) & 1 != 0;
        let arm = if ducking { rd::<u32>(ped, 0x474) & 0x4_0000 != 0 } else { rd::<u8>(ped, 0x476) & 1 != 0 };
        if arm {
            cmd = 1;
            blocked = true;
        }
    }
    if !ug.is_null() {
        use_gun_control_gun(ug, ped, rd::<GPtr<u8>>(this, 0xC), cmd);
    }
    let target = rd::<GPtr<u8>>(this, 0xC);
    let zero = x(gf(0x85_8B50));
    let d: [f32; 3] = if target.is_null() {
        if !x(rd::<f32>(this, 0x10)).fcom(zero).test_parity(0x44)
            && !x(rd::<f32>(this, 0x14)).fcom(zero).test_parity(0x44)
        {
            return 0;
        }
        let pp = entity_pos(ped);
        [
            x(rd::<f32>(this, 0x10)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
            x(rd::<f32>(this, 0x14)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
            x(rd::<f32>(this, 0x18)).fsub_m(rd::<f32>(pp, 8)).to_f32(),
        ]
    } else {
        let pp = entity_pos(ped);
        let tp = entity_pos(target);
        [
            x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
            x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
            x(rd::<f32>(tp, 8)).fsub_m(rd::<f32>(pp, 8)).to_f32(),
        ]
    };
    wr(ped, 0x55C, x(d[0]).fchs().fpatan(x(d[1])).to_f32());
    let sq = x(d[2]).fmul_m(d[2]).fadd(x(d[0]).fmul_m(d[0])).fadd(x(d[1]).fmul_m(d[1])).to_f32();
    if !ug.is_null() {
        let near = !ge_or_nan(x(sq), x(gf(0x85_8B44)));
        let fire_still = rd::<u8>(ug, 0x10) == 3 && (rd::<u32>(winfo, 0x18) >> 5) & 1 == 0;
        let moves = (near && !fire_still) || blocked;
        if moves && rd::<u32>(ped, 0x46C) & 0x20_0000 == 0 {
            let mut v = [0f32, 1.0];
            use_gun_control_gun_move(ug, GPtr::from_raw(v.as_mut_ptr()).cast());
            return 0;
        }
    }
    if !rd::<GPtr<u8>>(this, 0xC).is_null() && rd::<u32>(ped, 0x478) & 0x200 == 0 {
        let r = x(rd::<f32>(winfo, 8));
        if !le_or_nan(x(sq), r.fmul(r)) {
            wr::<i32>(this, 0x28, 0);
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_GUN_CONTROL_PROCESS_PED];
