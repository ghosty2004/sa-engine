//! MVP-C AI ports, batch `mvpc-ai-3`: task constructors/destructors/clones reached while loading and on foot
//! (`CTaskComplexWalkRoundCar`, `CTaskComplexCarDrive`, `CTaskSimpleIKChain`, `CTaskSimpleRunAnim::Clone`),
//! `CTaskSimpleSay::ProcessPed`, `CTaskComplexSequence::CreateNextSubTask` and `CTaskSimpleCarDrive::StartBopping`.
//! Ported from the 1.0 US bytes (protection fragments inlined; the exe wins over gta-reversed). Every call to
//! another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: u32) -> u32 = thiscall 0x61A3B0 => crate::gen_ctors::g_ctask_complex_61a3b0;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_o(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => sa_world::world::entity::register_reference;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => sa_world::world::entity::clean_up_old_reference;
    /// `CPointRoute::operator new(uint32)` (0x41B7C0, cdecl)
    fn point_route_new(size: u32) -> GPtr<u8> = cdecl 0x41B7C0 => sa_world::world::mvpa2::point_route_new;
    /// `IKChainManager_c::RemoveIKChain(IKChain_c*)` (0x618170, `ret 4`)
    fn ik_manager_remove_ik_chain(m: GPtr<u8>, c: GPtr<u8>) = thiscall 0x618170 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => crate::tasks::task_manager_find_active_task_by_type;
    /// CRT `rand()` (0x821B1E); called through `at_site` with the original return address.
    fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180, cdecl)
    fn random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90, cdecl, ST0)
    fn random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `RtQuatRotate(RtQuat*, const RwV3d* axis, RwReal angle, RwOpCombineType)` (0x7EB7C0, cdecl)
    fn rt_quat_rotate(q: GPtr<u8>, axis: u32, angle: f32, op: u32) -> GPtr<u8> = cdecl 0x7EB7C0 => unported;
    /// `CTaskSimpleCarDrive::StartBopping(CPed*)` (0x642760, `ret 4`)
    fn start_bopping_o(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x642760 => crate::mvpc_ai3::car_drive_start_bopping;
    /// `CInterestingEvents::Add(EType, CEntity*)` (0x602590, `ret 8`)
    fn interesting_events_add(ie: GPtr<u8>, ty: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490, cdecl; AL)
    #[allow(clippy::too_many_arguments)]
    fn world_los_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, peds: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u32 = cdecl 0x56A490 => unported;
    /// `CEventSexyPed::CEventSexyPed(CPed*)` (0x4AEDF0, `ret 4`)
    fn event_sexy_ped_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AEDF0 => unported;
    /// `CEventSexyPed::~CEventSexyPed` (0x4AEEA0)
    fn event_sexy_ped_dtor(e: GPtr<u8>) = thiscall 0x4AEEA0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u8) -> GPtr<u8> = thiscall 0x4AB420 => crate::group::event_group_add;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector*)` (0x5F1FA0, cdecl)
    fn bbox_corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncached(float z, const CVector*, CVector&)` (0x5F1600, cdecl)
    fn bbox_centre_uncached(z: u32, corners: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F1600 => crate::mvpa_ai1::compute_entity_bounding_box_centre_uncached;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10,
    /// `ret 0x14`)
    fn col_sphere_set(s: GPtr<u8>, r: f32, c: GPtr<u8>, m: u32, f: u32, l: u32) = thiscall 0x40FD10 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// `CTaskComplexWalkRoundCar::CTaskComplexWalkRoundCar(int32 moveState, const CVector& target, CVehicle*, bool
/// isPedGoingForCarDoor, bool)` (0x6541B0, `ret 0x14`; the SEH handler push sits in a protection fragment):
/// `CTaskComplex`, then byte +0xC = moveState, bit 0 of +0xD = the door flag (bit 1 cleared), the low nibble of +0xE
/// = the fifth argument, vtable 0x86F308, +0x10..+0x18 = target, +0x1C = vehicle (reference registered when set),
/// +0x24/+0x28 = 0, bytes +0x2C/+0x2D = 0, +0x54 = -1, +0x20 = a new empty `CPointRoute` (count 0; null when the
/// allocation fails).
#[hook(addr = 0x6541B0, conv = thiscall, name = "CTaskComplexWalkRoundCar::Constructor", expect = "6A FF E9 F1 CE DA FF",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes,entry-protection-read", recs = "grove-walk")]
pub fn walk_round_car_ctor(this: GPtr<u8>, ms: u32, target: GPtr<u8>, veh: GPtr<u8>, door: u32, flags: u32) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    let b0d = rd::<u8>(this, 0xD);
    wr::<u8>(this, 0xC, ms as u8);
    let b0e = rd::<u8>(this, 0xE);
    wr::<u8>(this, 0xD, (b0d & 0xFC) | (door as u8 & 1));
    wr::<u8>(this, 0xE, b0e ^ ((b0e ^ flags as u8) & 0xF));
    wr::<u32>(this, 0, 0x86F308);
    wr::<u32>(this, 0x10, rd::<u32>(target, 0));
    wr::<u32>(this, 0x14, rd::<u32>(target, 4));
    wr::<u32>(this, 0x18, rd::<u32>(target, 8));
    wr::<GPtr<u8>>(this, 0x1C, veh);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x28, 0);
    wr::<u8>(this, 0x2C, 0);
    wr::<u8>(this, 0x2D, 0);
    wr::<u32>(this, 0x54, 0xFFFF_FFFF);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x1C));
    }
    let r = point_route_new(0x64);
    if !r.is_null() {
        wr::<u32>(r, 0, 0);
    }
    wr::<GPtr<u8>>(this, 0x20, r);
    this
}

/// `CTaskComplexCarDrive::CTaskComplexCarDrive(CVehicle*, float speed, int32 carModelIndexToCreate, int32
/// carDrivingStyle)` (0x63C940, `ret 0x10`; SEH push in a protection fragment): `CTaskComplex`, +0x10 = speed,
/// +0x14 = model, vtable 0x86E934, +0xC = vehicle, +0x18 = driving style, byte +0x1C = 1, byte +0x20 = 0; the
/// vehicle's reference is registered when set.
#[hook(addr = 0x63C940, conv = thiscall, name = "CTaskComplexCarDrive::Constructor_1", expect = "6A FF E9 BE 74 DC FF",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn car_drive_ctor(this: GPtr<u8>, veh: GPtr<u8>, speed: u32, model: u32, style: u32) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0x10, speed);
    wr::<u32>(this, 0x14, model);
    wr::<u32>(this, 0, 0x86E934);
    wr::<GPtr<u8>>(this, 0xC, veh);
    wr::<u32>(this, 0x18, style);
    wr::<u8>(this, 0x1C, 1);
    wr::<u8>(this, 0x20, 0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0xC));
    }
    this
}

/// `CTaskSimpleIKChain::~CTaskSimpleIKChain` (0x633A90; SEH push in a protection fragment): vtable 0x86DB48; an
/// IK chain (+0x10) is removed from `g_ikChainMan` (0xC15448); the entity reference +0x28 is cleaned up when set;
/// then `~CTaskSimple`.
#[hook(addr = 0x633A90, conv = thiscall, name = "CTaskSimpleIKChain::Destructor", expect = "6A FF E9 C5 E7 DC FF",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes,entry-protection-read", recs = "grove-walk")]
pub fn ik_chain_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86DB48);
    let c = rd::<GPtr<u8>>(this, 0x10);
    if !c.is_null() {
        ik_manager_remove_ik_chain(g::<u8>(0xC15448), c);
    }
    let e = rd::<GPtr<u8>>(this, 0x28);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x28));
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleRunAnim::Clone` (0x61B6D0): a 0x20-byte task (null when the pool is full): `CTaskSimple`, then bit 2
/// of byte +0xC = bit 2 of the source's (bits 0..1 cleared), +0x14 = source +0x14, +8 = 0, vtable 0x86D528, +0x10 =
/// source +0x10, +0x18 = source +0x18, u16 +0x1C = 0x190.
#[hook(addr = 0x61B6D0, conv = thiscall, name = "CTaskSimpleRunAnim::Clone", expect = "6A FF 68 1B F2 83 00",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn run_anim_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_o(0x20);
    if t.is_null() {
        return t;
    }
    let s18 = rd::<u32>(this, 0x18);
    let bit = (rd::<u8>(this, 0xC) & 4) >> 2;
    let s14 = rd::<u32>(this, 0x14);
    let s10 = rd::<u32>(this, 0x10);
    task_simple_ctor(t);
    let c = rd::<u8>(t, 0xC);
    wr::<u32>(t, 0x14, s14);
    wr::<u8>(t, 0xC, ((bit & 1) << 2) | (c & 0xF8));
    wr::<u32>(t, 8, 0);
    wr::<u32>(t, 0, 0x86D528);
    wr::<u32>(t, 0x10, s10);
    wr::<u32>(t, 0x18, s18);
    wr::<u16>(t, 0x1C, 0x190);
    t
}

/// `CTaskSimpleSay::ProcessPed(CPed*)` (0x48E440, `ret 4`, AL): starts its timer (+0x10 start = now, +0x14 interval
/// = +0xC, +0x18 started) when not started, `ped->Say(u16 +8, 0, 1.0, false, false, false)`, then honours a pending
/// reset (+0x19: start = now) and returns `now >= start + interval` (unsigned); false when the timer is not started.
#[hook(addr = 0x48E440, conv = thiscall, name = "CTaskSimpleSay::ProcessPed", expect = "56 8B F1 8A 46 18",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn say_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x18) == 0 {
        let iv = rd::<u32>(this, 0xC);
        wr::<u32>(this, 0x10, g::<u32>(TIME_MS).read());
        wr::<u32>(this, 0x14, iv);
        wr::<u8>(this, 0x18, 1);
    }
    ped_say(ped, rd::<u16>(this, 8) as u32, 0, 0x3F80_0000, 0, 0, 0);
    if rd::<u8>(this, 0x18) == 0 {
        return 0;
    }
    if rd::<u8>(this, 0x19) != 0 {
        wr::<u32>(this, 0x10, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 0x19, 0);
    }
    let end = rd::<u32>(this, 0x14).wrapping_add(rd::<u32>(this, 0x10));
    (g::<u32>(TIME_MS).read() >= end) as u8
}

/// `CTaskComplexSequence::CreateNextSubTask(CPed*, int32& taskIndex, int32& repeatCount)` (0x632C70, `ret 0xC`):
/// `++taskIndex`. Without repeat mode (+0x30 == 0): the clone of task `taskIndex` (+0x10 + 4i), or null at index 8
/// or an empty slot. In repeat mode: at index 8 or an empty slot `taskIndex` = 0 and `++repeatCount`; null when the
/// mode is not 1 (loop forever) and `repeatCount` reached it; else the clone of task `taskIndex` (no null test).
#[hook(addr = 0x632C70, conv = thiscall, name = "CTaskComplexSequence::CreateNextSubTask-Internal", expect = "8B 44 24 08 8B 10 42",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn sequence_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>, idx: GPtr<u8>, rep: GPtr<u8>) -> GPtr<u8> {
    let i = rd::<i32>(idx, 0).wrapping_add(1);
    wr::<i32>(idx, 0, i);
    if rd::<i32>(this, 0x30) == 0 {
        if i == 8 {
            return GPtr::NULL;
        }
        let t = rd::<GPtr<u8>>(this, 0x10 + 4 * i as isize);
        if t.is_null() {
            return GPtr::NULL;
        }
        return v_clone(t);
    }
    if i == 8 || rd::<GPtr<u8>>(this, 0x10 + 4 * i as isize).is_null() {
        wr::<i32>(idx, 0, 0);
        wr::<i32>(rep, 0, rd::<i32>(rep, 0).wrapping_add(1));
    }
    let mode = rd::<i32>(this, 0x30);
    if mode != 1 && rd::<i32>(rep, 0) == mode {
        return GPtr::NULL;
    }
    v_clone(rd::<GPtr<u8>>(this, 0x10 + 4 * rd::<i32>(idx, 0) as isize))
}

/// `CTaskSimpleCarDrive::StartBopping(CPed*)` (0x642760, `ret 4`): joins the bopping of another occupant of the
/// ped's vehicle (+0x58C): the driver (+0x460) first, then passengers +0x464..+0x46C (each other than the ped and
/// non-null), the first with an active task 0x2C5 whose start (+0x20) is not -1 gives start and period (+0x28):
/// +0x20 = start, +0x28 = period, +0x2C = f32((now - start) % period) / period, +0x30 = ((now - start) / period) % 2
/// (signed). Otherwise a new beat: +0x20 = now, a = ftol(rand() & 0xFFFF * [0x858B14] * [0x86EBC4]), +0x28 =
/// ftol([0x858624] / ((60 - a) * [0x859044]) * [0x858C4C]), +0x2C = 0. The original keeps its period in its pushed-ECX
/// slot (initially `this`, so a driver without the task leaves `this` as the period candidate) and its integer
/// temporaries in its own argument slot.
#[hook(addr = 0x642760, conv = thiscall, name = "CTaskSimpleCarDrive::StartBopping", expect = "51 53 8B D9 8B 4C 24 0C",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes,stack-residue", recs = "driving",
       note = "rand at 0x642818 (at_site 0x64281D); writes its own argument slot")]
pub fn car_drive_start_bopping(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut start = -1i32;
    let mut period = this.addr() as u32 as i32;
    let drv = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x460);
    if !drv.is_null() && drv != ped {
        let t = tm_find_active_task_by_type(rd::<GPtr<u8>>(drv, 0x47C).byte_add(4), 0x2C5);
        if !t.is_null() {
            start = rd::<i32>(t, 0x20);
            period = rd::<i32>(t, 0x28);
        }
    }
    if start == -1 {
        for off in [0x464isize, 0x468, 0x46C] {
            let p = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), off);
            if p.is_null() || p == ped {
                continue;
            }
            let t = tm_find_active_task_by_type(rd::<GPtr<u8>>(p, 0x47C).byte_add(4), 0x2C5);
            if !t.is_null() {
                period = rd::<i32>(t, 0x28);
                start = rd::<i32>(t, 0x20);
            }
            if start != -1 {
                break;
            }
        }
    }
    if start == -1 {
        wr::<u32>(this, 0x20, g::<u32>(TIME_MS).read());
        let r = at_site(0x64281D, rand) & 0xFFFF;
        let a = ftol32(
            X87::from_i32(r)
                .fmul_m(g::<f32>(0x858B14).read())
                .fmul_m(g::<f32>(0x86EBC4).read()),
        );
        let v = X87::from_i32(60i32.wrapping_sub(a))
            .fmul_m(g::<f32>(0x859044).read())
            .fdivr_m(g::<f32>(0x858624).read())
            .fmul_m(g::<f32>(0x858C4C).read());
        wr::<i32>(this, 0x28, ftol32(v));
        wr::<u32>(this, 0x2C, 0);
        return;
    }
    wr::<i32>(this, 0x20, start);
    wr::<i32>(this, 0x28, period);
    let d = (g::<u32>(TIME_MS).read() as i32).wrapping_sub(start);
    let q = d / period;
    let rem = d % period;
    wr::<f32>(this, 0x2C, f32_of(X87::from_i32(rem).fdiv(X87::from_i32(period))));
    wr::<i32>(this, 0x30, q % 2);
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingSphere(const CPed&, CEntity&, CColSphere&)` (0x5F3C20, cdecl): the
/// entity's box corners at the ped's z (computed twice: once from the f32-reloaded z, once from the z dword, which
/// the original also stores into its own first argument slot), the centre of the second set, then the largest
/// squared distance `(dz*dz + dy*dy) + dx*dx` (x87, compared unrounded, kept as its f32 store) of the first set's
/// corners from the centre, starting from [0x858B50]; radius = `sqrt(max) * [0x858F14]` (f32);
/// `CColSphere::Set(radius, centre, 0, 0, 0xFF)`.
#[hook(addr = 0x5F3C20, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingSphere", expect = "83 EC 6C 56 8B 74 24 74",
       unverified = "mvpc-ai-3", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
       note = "FSQRT of the max squared distance; writes the z dword and the f32 distances into its first argument slot")]
pub fn compute_entity_bounding_sphere(ped: GPtr<u8>, e: GPtr<u8>, sphere: GPtr<u8>) {
    let mut c1 = [0u32; 12];
    let mut c2 = [0u32; 12];
    let mut ctr = [0u32; 3];
    let p1 = local(&mut c1);
    let p2 = local(&mut c2);
    let pc = local(&mut ctr);
    bbox_corners_uncached(f32_of(x(rd::<f32>(pos(ped), 8))).to_bits(), e, p1);
    let z = rd::<u32>(pos(ped), 8);
    bbox_corners_uncached(z, e, p2);
    bbox_centre_uncached(z, p2, pc);
    let thr = x(g::<f32>(0x858B50).read());
    let mut best = thr;
    for k in 0..4isize {
        let c = p1.byte_add(0xC * k);
        let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(pc, 0));
        let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(pc, 4));
        let dz = x(rd::<f32>(c, 8)).fsub_m(rd::<f32>(pc, 8));
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let against = if k == 0 { thr } else { best };
        if !d2.fcom(against).test_any(0x41) {
            best = x(f32_of(d2));
        }
    }
    let r = f32_of(best.fsqrt().fmul_m(g::<f32>(0x858F14).read()));
    col_sphere_set(sphere, r, pc, 0, 0, 0xFF);
}

/// `CTaskSimpleCarDrive::ProcessHeadBopping(CPed*, bool, float dist)` (0x6428C0, `ret 0xC`). `bop` = the flag
/// argument. Not bopping (bit 0 of +0x5C clear): a start needs the blend +0x44 == 0 and a 1-in-~200 draw
/// (`GetRandomNumberInRange(0, 1000) > 995`), else `bop` = false; on a start: `StartBopping` when +0x20 is -1 (then
/// +0x34 = +0x20) else +0x34 = now; +0x38 = random 1..3 (which axes), +0x40 = random 3.0..8.0 (amplitude), +0x44 =
/// +0x3C = 0. Bopping: s = f32 `fsin(+0x2C * [0x858CB8])`; after 5000 ms (unsigned, from +0x34) a draw > 995 stops
/// it. Bit 0 of +0x5C = `bop`; the blend +0x44 moves by [0x858C28] towards 1 (clamped above) or 0 (clamped below)
/// and +0x3C += (s - +0x3C) * blend (bopping) or * (1 - blend). With `dist` < [0x859A44] and +0x3C > 0, angle =
/// f32(+0x40 * +0x3C) (kept in the original's own `bop` argument slot) rotates the head quaternion (ped +0x490, +0x10)
/// about 0x86EBD4 (when +0x38 >= 1; negated unless +0x30 == 0) and about 0x86EBC8 (negated unless +0x38 == 2), both
/// `rwCOMBINEPRECONCAT`, and ped flag 0x4000 at +0x470 is set.
#[hook(addr = 0x6428C0, conv = thiscall, name = "CTaskSimpleCarDrive::ProcessHeadBopping", expect = "51 53 8A 5C 24 10",
       unverified = "mvpc-ai-3", route = "L2", hazards = "x87-transcendental,stack-residue,heap-writes", recs = "driving",
       note = "FSIN of the beat phase; writes the angle into its own second argument slot")]
pub fn car_drive_process_head_bopping(this: GPtr<u8>, ped: GPtr<u8>, bop: u32, dist: f32) {
    let mut b = bop as u8;
    let mut s = 0f32;
    if b != 0 {
        if rd::<u8>(this, 0x5C) & 1 == 0 {
            let c = x(rd::<f32>(this, 0x44)).fcom(x(g::<f32>(0x858B50).read()));
            if c.test_parity(0x44) || random_int_in_range(0, 1000) <= 0x3E3 {
                b = 0;
            } else {
                if rd::<i32>(this, 0x20) == -1 {
                    start_bopping_o(this, ped);
                    wr::<u32>(this, 0x34, rd::<u32>(this, 0x20));
                } else {
                    wr::<u32>(this, 0x34, g::<u32>(TIME_MS).read());
                }
                let axes = random_int_in_range(1, 3);
                wr::<i32>(this, 0x38, axes);
                let amp = random_float_in_range(0x4040_0000, 0x4100_0000);
                wr::<f32>(this, 0x40, f32_of(X87::from_f64_exact(amp)));
                wr::<u32>(this, 0x44, 0);
                wr::<u32>(this, 0x3C, 0);
            }
        } else {
            s = f32_of(x(rd::<f32>(this, 0x2C)).fmul_m(g::<f32>(0x858CB8).read()).fsin());
            let el = g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(this, 0x34));
            if el > 5000 && random_int_in_range(0, 1000) > 0x3E3 {
                b = 0;
            }
        }
    }
    let f = rd::<u8>(this, 0x5C);
    wr::<u8>(this, 0x5C, (f & !1) | (b & 1));
    let one = g::<f32>(0x858624).read();
    let zero = g::<f32>(0x858B50).read();
    let step = g::<f32>(0x858C28).read();
    let v = if b & 1 != 0 {
        if !x(rd::<f32>(this, 0x44)).fcom(x(one)).test_parity(5) {
            wr::<f32>(this, 0x44, f32_of(x(rd::<f32>(this, 0x44)).fadd_m(step)));
        }
        if !x(rd::<f32>(this, 0x44)).fcom(x(one)).test_any(0x41) {
            wr::<u32>(this, 0x44, 0x3F80_0000);
        }
        x(s).fsub_m(rd::<f32>(this, 0x3C)).fmul_m(rd::<f32>(this, 0x44))
    } else {
        if !x(rd::<f32>(this, 0x44)).fcom(x(zero)).test_any(0x41) {
            wr::<f32>(this, 0x44, f32_of(x(rd::<f32>(this, 0x44)).fsub_m(step)));
        }
        if !x(rd::<f32>(this, 0x44)).fcom(x(zero)).test_parity(5) {
            wr::<u32>(this, 0x44, 0);
        }
        x(s).fsub_m(rd::<f32>(this, 0x3C)).fmul(x(one).fsub_m(rd::<f32>(this, 0x44)))
    };
    wr::<f32>(this, 0x3C, f32_of(v.fadd_m(rd::<f32>(this, 0x3C))));
    if x(dist).fcom(x(g::<f32>(0x859A44).read())).test_parity(5) {
        return;
    }
    if x(rd::<f32>(this, 0x3C)).fcom(x(zero)).test_any(0x41) {
        return;
    }
    let a = f32_of(x(rd::<f32>(this, 0x40)).fmul_m(rd::<f32>(this, 0x3C)));
    let q = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x490), 0x10);
    if rd::<i32>(this, 0x38) >= 1 {
        let ang = if rd::<u32>(this, 0x30) == 0 { a } else { f32_of(x(a).fchs()) };
        rt_quat_rotate(q, 0x86EBD4, ang, 1);
    }
    let ang = if rd::<i32>(this, 0x38) == 2 { a } else { f32_of(x(a).fchs()) };
    rt_quat_rotate(q, 0x86EBC8, ang, 1);
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x4000);
}

/// `FUN_00603bf0` (0x603BF0, `ret 0xC`): the sexy-ped scanner (`CEventScanner` +0xBC) over `count` nearby peds.
/// Its timer (interval 500 ms; started on first use, pending reset honoured) gates the scan and restarts. Peds
/// with flag 0x100 at +0x46C are skipped unless they are mission peds (+0x484 == 2). Candidates: female civilians
/// (type 5) whose stats sexiness (+0x59C, i8 +0x27) exceeds the ped's, without flag 0x100; d = candidate position
/// - ped position (f32 x/y, z kept unrounded for its square), d2 = (dz*dz + dy*dy) + dx*dx (stored as f32 into the
/// original's own first argument slot). Unrounded d2 within [0x859AA4]: a player ped records interesting event 9; the nearest
/// so far (f32 d2 against the best, start 1e10) in front of the ped (d . forward > 0) with a clear line of sight (buildings and objects)
/// wins. A winner raises `CEventSexyPed` into the ped's event group and the timer restarts with 3000 ms.
#[hook(addr = 0x603BF0, conv = thiscall, name = "FUN_00603bf0", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "mvpc-ai-3", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "stores the f32 squared distance into its own first argument slot")]
pub fn sexy_ped_scanner_scan(this: GPtr<u8>, ped: GPtr<u8>, ents: GPtr<u8>, n: i32) {
    if rd::<u8>(this, 8) == 0 {
        wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 8, 1);
        wr::<u32>(this, 4, 500);
    }
    if rd::<u8>(this, 9) != 0 {
        wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 9, 0);
    }
    let end = rd::<u32>(this, 4).wrapping_add(rd::<u32>(this, 0));
    let now = g::<u32>(TIME_MS).read();
    if now < end {
        return;
    }
    wr::<u32>(this, 0, now);
    wr::<u32>(this, 4, 500);
    wr::<u8>(this, 8, 1);
    if rd::<u8>(ped, 0x484) != 2 && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return;
    }
    let mut best_d = f32::from_bits(0x5015_02F9);
    let mut best = GPtr::<u8>::NULL;
    let mut pp = [0u32; 3];
    let ppp = local(&mut pp);
    let p = pos(ped);
    wr::<u32>(ppp, 0, rd::<u32>(p, 0));
    wr::<u32>(ppp, 4, rd::<u32>(p, 4));
    wr::<u32>(ppp, 8, rd::<u32>(p, 8));
    let mut i = 0i32;
    while i < n {
        let o = rd::<GPtr<u8>>(ents, 4 * i as isize);
        i += 1;
        if o.is_null() || rd::<i32>(o, 0x598) != 5 {
            continue;
        }
        if rd::<i8>(rd::<GPtr<u8>>(o, 0x59C), 0x27) <= rd::<i8>(rd::<GPtr<u8>>(ped, 0x59C), 0x27) {
            continue;
        }
        if rd::<u32>(o, 0x46C) & 0x100 != 0 {
            continue;
        }
        let mut op = [0u32; 3];
        let opp = local(&mut op);
        let q = pos(o);
        wr::<u32>(opp, 0, rd::<u32>(q, 0));
        wr::<u32>(opp, 4, rd::<u32>(q, 4));
        wr::<u32>(opp, 8, rd::<u32>(q, 8));
        let dx = f32_of(x(rd::<f32>(opp, 0)).fsub_m(rd::<f32>(ppp, 0)));
        let dy = f32_of(x(rd::<f32>(opp, 4)).fsub_m(rd::<f32>(ppp, 4)));
        let dz_e = x(rd::<f32>(opp, 8)).fsub_m(rd::<f32>(ppp, 8));
        let dz = f32_of(dz_e);
        let d2e = dz_e.fmul_m(dz).fadd(x(dy).fmul_m(dy)).fadd(x(dx).fmul_m(dx));
        let d2 = f32_of(d2e);
        if d2e.fcom(x(g::<f32>(0x859AA4).read())).test_parity(5) {
            continue;
        }
        if !rd::<GPtr<u8>>(ped, 0x480).is_null() {
            interesting_events_add(g::<u8>(0xC0B058), 9, o);
        }
        if x(d2).fcom(x(best_d)).test_parity(5) {
            continue;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let dot = x(dz)
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x(dy).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(dx).fmul_m(rd::<f32>(m, 0x10)));
        if dot.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
            continue;
        }
        if world_los_clear(ppp, opp, 1, 0, 0, 1, 0, 0, 0) as u8 != 0 {
            best = o;
            best_d = d2;
        }
    }
    if best.is_null() {
        return;
    }
    let mut ev = [0u32; 8];
    let e = local(&mut ev);
    event_sexy_ped_ctor(e, best);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 4, 3000);
    wr::<u8>(this, 8, 1);
    event_sexy_ped_dtor(e);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_WALK_ROUND_CAR_CTOR,
    HOOK_CAR_DRIVE_CTOR,
    HOOK_IK_CHAIN_DTOR,
    HOOK_RUN_ANIM_CLONE,
    HOOK_SAY_PROCESS_PED,
    HOOK_SEQUENCE_CREATE_NEXT_SUB_TASK,
    HOOK_CAR_DRIVE_START_BOPPING,
    HOOK_COMPUTE_ENTITY_BOUNDING_SPHERE,
    HOOK_CAR_DRIVE_PROCESS_HEAD_BOPPING,
    HOOK_SEXY_PED_SCANNER_SCAN,
];
