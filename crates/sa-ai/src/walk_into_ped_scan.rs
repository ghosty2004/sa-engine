//! The potential-walk-into-ped scanner `FUN_00606580` (batch `mvpk-ai-4`): called from `CEventScanner::ScanForEvents`,
//! raises `CEventPotentialWalkIntoPed` for a ped walking a go-to route into a nearby ped. Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CPedIntelligence::GetMoveStateFromGoToTask()` (0x601D70)
    fn intel_get_move_state_from_go_to_task(i: GPtr<u8>) -> u32 = thiscall 0x601D70 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CVector::Difference(const CVector& a, const CVector& b)` (0x40FE00, `ret 8`): `this = a - b`
    fn vector_difference(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x40FE00 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, returns the hidden out pointer)
    fn vector_mul(out: GPtr<u8>, v: GPtr<u8>, f: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CColSphere::Set(float radius, const CVector& center, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10,
    /// `ret 0x14`)
    fn col_sphere_set(s: GPtr<u8>, r: u32, c: GPtr<u8>, m: u32, f: u32, l: u32) = thiscall 0x40FD10 => unported;
    /// `CColSphere::IntersectEdge(const CVector& a, const CVector& b, CVector& p1, CVector& p2)` (0x4100E0, `ret 0x10`)
    fn col_sphere_intersect_edge(s: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, p1: GPtr<u8>, p2: GPtr<u8>) -> u8 = thiscall 0x4100E0 => unported;
    /// `approxEqual(float, float, float)` (0x4EEA80)
    fn approx_equal(a: u32, b: u32, eps: u32) -> u8 = cdecl 0x4EEA80 => unported;
    /// `CEventPotentialWalkIntoPed::CEventPotentialWalkIntoPed(CPed*, const CVector& target, int32 moveState)` (0x4AE6E0,
    /// `ret 0xC`)
    fn walk_into_ped_ctor(e: GPtr<u8>, other: GPtr<u8>, target: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x4AE6E0 => unported;
    /// `CEventPotentialWalkIntoPed::~CEventPotentialWalkIntoPed()` (0x4AE7A0)
    fn walk_into_ped_dtor(e: GPtr<u8>) = thiscall 0x4AE7A0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `FUN_00606580(CPed* ped, int32 enabled)` (thiscall, `ret 8`; `this` unused). Nothing when `enabled` is 0, the ped
/// lacks flag +0x1C bit 0, its simplest active task is no go-to task, or the go-to move state is 1. `target` = that
/// task +0xC. Over the 16 nearby peds of the intelligence (+0x130): alive, with flag +0x1C bit 0, `d = other - ped`
/// position (`Difference`) with `(d.y*F.y + d.z*F.z) + d.x*F.x` above 0 (F = the ped matrix forward, read without a null
/// check), a 0.7 sphere at the other ped (material 0, flags 0, lighting 0xFF) that `IntersectEdge(ped position,
/// target)` hits, and `|d|²` below the best so far (start 2.5², f32) → the closest. With one: unless the closest also
/// runs a go-to task with `DotProduct(ped F, other F)` not below 0.923 and then (both move speeds +0x44 times 50 with z
/// zeroed: mine `a = |·|²`, its `b = |·|² + 0.25`, f32) the best above 1.0, `b` not `approxEqual(0, 0.01)` and `b`
/// above `a` → a stack `CEventPotentialWalkIntoPed(closest, target, move state)` into the ped's event group. The
/// original keeps the loop index, the best distance and `b` in its own argument slots.
#[hook(addr = 0x606580, conv = thiscall, name = "FUN_00606580", expect = "64 A1 00 00 00 00",
       route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the event and the sphere live on the Rust stack; argument slots reused as locals")]
pub fn walk_into_ped_scanner_scan(_this: GPtr<u8>, ped: GPtr<u8>, enabled: u32) {
    if enabled == 0 || rd::<u8>(ped, 0x1C) & 1 == 0 {
        return;
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let task = tm_get_simplest_active_task(intel.byte_add(4));
    if task.is_null() || task_is_go_to_task(task) == 0 || intel_get_move_state_from_go_to_task(intel) == 1 {
        return;
    }
    let target = task.byte_add(0xC);
    let r = k(0x86C8C0);
    let mut best = x(r).fmul_m(r).to_f32();
    let mut closest = GPtr::<u8>::from_addr(0);
    let mut d = [0f32; 3];
    let d_p: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    let mut sphere = [0u32; 6];
    let sphere_p: GPtr<u8> = GPtr::from_raw(sphere.as_mut_ptr()).cast();
    let mut p1 = [0f32; 3];
    let mut p2 = [0f32; 3];
    for i in 0..16 {
        let other = rd::<GPtr<u8>>(intel, 0x130 + 4 * i);
        if other.is_null() || ped_is_alive(other) == 0 || rd::<u8>(other, 0x1C) & 1 == 0 {
            continue;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        vector_difference(d_p, entity_pos(other), entity_pos(ped));
        let dot = x(rd::<f32>(d_p, 4))
            .fmul_m(rd::<f32>(m, 0x14))
            .fadd(x(rd::<f32>(d_p, 8)).fmul_m(rd::<f32>(m, 0x18)))
            .fadd(x(rd::<f32>(d_p, 0)).fmul_m(rd::<f32>(m, 0x10)));
        if dot.fcom(x(k(0x858B50))).test_any(0x41) {
            continue;
        }
        col_sphere_set(sphere_p, 0x3F33_3333, entity_pos(other), 0, 0, 0xFF);
        if col_sphere_intersect_edge(
            sphere_p,
            entity_pos(ped),
            target,
            GPtr::from_raw(p1.as_mut_ptr()).cast(),
            GPtr::from_raw(p2.as_mut_ptr()).cast(),
        ) == 0
        {
            continue;
        }
        let m2 = X87::from_f64_exact(vector_magnitude_sqr(d_p));
        if m2.fcom(x(best)).test_parity(5) {
            continue;
        }
        best = m2.to_f32();
        closest = other;
    }
    if closest.is_null() {
        return;
    }
    let s = tm_get_simplest_active_task(rd::<GPtr<u8>>(closest, 0x47C).byte_add(4));
    if !s.is_null()
        && task_is_go_to_task(s) != 0
        && !X87::from_f64_exact(vector_dot_product(rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10), rd::<GPtr<u8>>(closest, 0x14).byte_add(0x10)))
            .fcom(x(k(0x86CD98)))
            .test_any(1)
    {
        let mut va = [0f32; 3];
        let mut vb = [0f32; 3];
        let va_p: GPtr<u8> = GPtr::from_raw(va.as_mut_ptr()).cast();
        let vb_p: GPtr<u8> = GPtr::from_raw(vb.as_mut_ptr()).cast();
        vector_mul(va_p, ped.byte_add(0x44), 0x4248_0000);
        vector_mul(vb_p, closest.byte_add(0x44), 0x4248_0000);
        wr::<f32>(va_p, 8, 0.0);
        wr::<f32>(vb_p, 8, 0.0);
        let a = X87::from_f64_exact(vector_magnitude_sqr(va_p)).to_f32();
        let b = X87::from_f64_exact(vector_magnitude_sqr(vb_p)).fadd_m(k(0x86CD94)).to_f32();
        if !x(best).fcom(x(k(0x86CD90))).test_any(0x41)
            && approx_equal(b.to_bits(), 0, 0x3C23_D70A) == 0
            && !x(b).fcom(x(a)).test_any(0x41)
        {
            return;
        }
    }
    let ms = intel_get_move_state_from_go_to_task(rd::<GPtr<u8>>(ped, 0x47C));
    let mut ev = [0u32; 0x10];
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    walk_into_ped_ctor(ev_p, closest, target, ms);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0);
    walk_into_ped_dtor(ev_p);
}

pub const HOOKS: &[HookReg] = &[HOOK_WALK_INTO_PED_SCANNER_SCAN];
