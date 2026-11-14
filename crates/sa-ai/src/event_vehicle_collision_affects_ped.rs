//! `CEventVehicleCollision::AffectsPed` (0x4B2EE0, batch `carjack-1`): whether a vehicle's collision concerns a ped
//! (a jacked driver standing by the road, traffic around the jack). Ported from the 1.0 US bytes (gta-reversed as a
//! reading aid). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal, clippy::needless_range_loop)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0, cdecl, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CPed&, CEntity&)` (0x5F3BC0, cdecl)
    fn compute_entity_hit_side_ped(ped: GPtr<u8>, e: GPtr<u8>) -> i32 = cdecl 0x5F3BC0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, CEntity&)` (0x5F3AC0, cdecl)
    fn compute_entity_hit_side_pos(pos: GPtr<u8>, e: GPtr<u8>) -> i32 = cdecl 0x5F3AC0 => unported;
    /// `` `eh vector constructor iterator' `` (0x403D60, stdcall)
    fn vector_ctor_iterator(p: GPtr<u8>, size: u32, count: i32, ctor: u32) = stdcall 0x403D60 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes(float z, CEntity&, CVector (*)[4], float*)` (0x5F3660, cdecl)
    fn compute_entity_bbox_planes(z: u32, e: GPtr<u8>, planes: GPtr<u8>, dots: GPtr<u8>) = cdecl 0x5F3660 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexWalkRoundCar::SetNewVehicle(CVehicle*, int8)` (0x654290, `ret 8`)
    fn walk_round_car_set_new_vehicle(t: GPtr<u8>, v: GPtr<u8>, f: i32) = thiscall 0x654290 => unported;
}

/// The facing threshold (0.35) and 0.0.
const FACING_MIN: usize = 0x858F9C;
const ZERO: usize = 0x858B50;

fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `DotProduct(a, b) + d < 0.0` (x87: the ST0 dot plus the float, `fcomp` C0 alone).
fn behind(a: GPtr<u8>, b: GPtr<u8>, d: u32) -> bool {
    X87::from_f64_exact(vector_dot_product(a, b)).fadd_m(f32::from_bits(d)).fcom(x(g::<f32>(ZERO).read())) == Cmp::Less
}

/// `CEventVehicleCollision::AffectsPed(CPed*)` (0x4B2EE0, `ret 4`). A dead ped: false. Word +0xE set: true. No
/// vehicle (+0x14), a ped in a vehicle (+0x46C bit 8), a boat (+0x590 == 5) or `-DotProduct(normal +0x18, ped
/// forward) < 0.35` (`fcomp`, NaN passes; the ped's matrix read unchecked): false. A go-to simplest task (target +0xC)
/// whose target lies on the same hit side of the vehicle as the ped: false unless the vehicle tows or is towed
/// (+0x4C4 / +0x4C8); then the vehicle's four bounding-box planes at the ped's height (`eh vector constructor
/// iterator` over the CVector locals first) must hold the target or the ped inside all four (`dot + d < 0`), else
/// false. Then the active task: a walk-round-car (0x1FC) around this vehicle → false; around the vehicle it tows
/// (+0x4C8) or that tows it (+0x4C4) → `SetNewVehicle(vehicle, signed bits 2..5 of +0xD)`; a hit-ped-with-car
/// (0x1F9) with this vehicle (+0xC) → false. Otherwise true.
#[hook(addr = 0x4B2EE0, conv = thiscall, name = "CEventVehicleCollision::AffectsPed", expect = "83 EC 48 53 8B 5C 24 50",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return,uninit-locals", recs = "carjack")]
pub fn event_vehicle_collision_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 {
        return 0;
    }
    if rd::<u16>(this, 0xE) != 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(this, 0x14);
    if veh.is_null() || rd::<u32>(ped, 0x46C) & 0x100 != 0 || rd::<u32>(veh, 0x590) == 5 {
        return 0;
    }
    let fwd = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
    let d = X87::from_f64_exact(vector_dot_product(this.byte_add(0x18), fwd)).fchs();
    if d.fcom(x(g::<f32>(FACING_MIN).read())) == Cmp::Less {
        return 0;
    }
    let simplest = get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if !simplest.is_null() && task_is_go_to_task(simplest) != 0 {
        let target = simplest.byte_add(0xC);
        let side = compute_entity_hit_side_ped(ped, rd::<GPtr<u8>>(this, 0x14));
        if side == compute_entity_hit_side_pos(target, rd::<GPtr<u8>>(this, 0x14)) {
            let v = rd::<GPtr<u8>>(this, 0x14);
            if rd::<u32>(v, 0x4C4) == 0 && rd::<u32>(v, 0x4C8) == 0 {
                return 0;
            }
            let mut planes = [0u32; 12];
            let mut dots = [0u32; 4];
            vector_ctor_iterator(buf(&mut planes), 0xC, 4, 0x404350);
            let z = rd::<u32>(pos_ptr(ped), 8);
            compute_entity_bbox_planes(z, rd::<GPtr<u8>>(this, 0x14), buf(&mut planes), buf(&mut dots));
            let (mut n_target, mut n_ped) = (0, 0);
            for i in 0..4usize {
                let plane = buf(&mut planes).byte_add(i as isize * 0xC);
                if behind(target, plane, dots[i]) {
                    n_target += 1;
                }
                if behind(pos_ptr(ped), plane, dots[i]) {
                    n_ped += 1;
                }
            }
            if n_target != 4 && n_ped != 4 {
                return 0;
            }
        }
    }
    let active = get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if active.is_null() {
        return 1;
    }
    if v_get_task_type(active) == 0x1FC {
        let round = rd::<GPtr<u8>>(active, 0x1C);
        let v = rd::<GPtr<u8>>(this, 0x14);
        if round == v {
            return 0;
        }
        let towed = rd::<GPtr<u8>>(v, 0x4C8);
        let tower = rd::<GPtr<u8>>(v, 0x4C4);
        if !towed.is_null() && towed == round || !tower.is_null() && tower == round {
            let f = i32::from(((rd::<u8>(active, 0xD) << 2) as i8) >> 4);
            walk_round_car_set_new_vehicle(active, v, f);
        }
        return 1;
    }
    if v_get_task_type(active) == 0x1F9 && rd::<GPtr<u8>>(active, 0xC) == rd::<GPtr<u8>>(this, 0x14) {
        return 0;
    }
    1
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_VEHICLE_COLLISION_AFFECTS_PED];
