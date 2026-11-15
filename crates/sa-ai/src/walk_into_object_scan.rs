//! The potential-walk-into-object scanner `FUN_00606890` (batch `mvpl-ai-1`): called from `CEventScanner::ScanForEvents`,
//! raises `CEventPotentialWalkIntoObject` for a ped on a go-to route that is about to walk into the closest object of a
//! stack `CObjectScanner`. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

use crate::bindings::entity_scanner_clear;
use crate::bindings::geometry_line_of_sight_clear as los_clear_ped;
use crate::bindings::task_is_go_to_task;
use sa_core::bindings::tm_get_simplest_active_task;
use sa_core::bindings::vector_difference;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::entity_get_bound_centre;
use crate::bindings::event_group_add;
use sa_world::bindings::vector_magnitude_sqr;
use sa_world::bindings::vector_sub;
orig! {
    /// `CObjectScanner::CObjectScanner()` (0x606010)
    fn object_scanner_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x606010 => unported;
    /// `CObjectScanner::ScanForObjectsInRange(CPed&)` (0x5FFF30, `ret 4`)
    fn object_scanner_scan_in_range(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FFF30 => unported;
    /// `CPedGeometryAnalyser::GetIsLineOfSightClear(const CVector&, const CVector&, CEntity&)` (0x5F2F00)
    fn los_clear_v3d(a: GPtr<u8>, b: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x5F2F00 => unported;
    /// `CEventPotentialWalkIntoObject::CEventPotentialWalkIntoObject(CObject*, int32 moveState)` (0x4AE5D0, `ret 8`)
    fn walk_into_object_ctor(e: GPtr<u8>, obj: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x4AE5D0 => unported;
    /// `CEventPotentialWalkIntoObject::~CEventPotentialWalkIntoObject()` (`FUN_004ae670`)
    fn walk_into_object_dtor(e: GPtr<u8>) = thiscall 0x4AE670 => unported;
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_TIME_MS: usize = 0xB7CB84;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `CEntityScanner` vtable (0x86CD40): the `CObjectScanner` destructor's base part
const ENTITY_SCANNER_VTABLE: u32 = 0x86CD40;
/// The scan range (0x86C8FC; squared on the x87)
const K_RANGE: usize = 0x86C8FC;
/// The minimum line-of-sight distance to the object (0x86C900)
const K_MIN_DIST: usize = 0x86C900;
/// 1.0 (0x858624): the ped's height band half-width
const K_ONE: usize = 0x858624;

use sa_core::field::gf as k;
use sa_world::world::common::entity_pos;

/// `FUN_00606890(CPed* ped)` (thiscall, `ret 4`; `this` = a task timer: +0 start, +4 period, +8 set, +9 reset): unset →
/// start = now, set, period 500 (the set byte is re-read: 0 → nothing); reset → start = now, cleared; not `now >= start +
/// period` (unsigned) → nothing; else (now, 500), set. `ms` = the simplest active task's +8 when it is a go-to task, else
/// 1. A player, `ms == 1`, or no go-to simplest task (fetched again; `target` = its +0xC) → nothing. Otherwise a stack
/// `CObjectScanner` scans the ped's range (`ScanForObjectsInRange`); its closest object (+0x4C) must exist, have flags
/// +0x140 bits 0 and 10 clear and +0x1C bit 0 (uses collision) set, with `|ped - object|²` (`Difference`,
/// `MagnitudeSqr`) below `range * range` (x87, `jne` on `test ah, 0x41`). Height band: with the bound centre's z `bz`
/// (`GetBoundCentre`, z read through the returned pointer) and the model's col sphere radius `r` (model info +0x14 →
/// +0x24): `ped z - 1` not above `r + bz`, and `f32(ped z + 1)` not below `f32(bz - r)` (unordered passes both). Then
/// `GetIsLineOfSightClear(ped, target, object, dist)` (dist zeroed first) false, `dist` above the minimum, and the line
/// from `ped - (0, 0, 0.75)` (`VectorSub`) to `target` blocked by the object (`GetIsLineOfSightClear(v3d)` false) → a
/// stack `CEventPotentialWalkIntoObject(object, ms)` into the ped's event group (+0x47C → +0x68), then its destructor.
/// The scanner's destructor (base vtable 0x86CD40, `CEntityScanner::Clear`) runs on every path after the scan.
#[hook(addr = 0x606890, conv = thiscall, name = "FUN_00606890", expect = "64 A1 00 00 00 00",
       route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the object scanner, the event and the vectors live on the Rust stack (the scanner registers entity references to its slots until Clear)")]
pub fn walk_into_object_scanner_scan(this: GPtr<u8>, ped: GPtr<u8>) {
    let now = || g::<u32>(TIMER_TIME_MS).read();
    if rd::<u8>(this, 8) == 0 {
        wr::<u32>(this, 0, now());
        wr::<u8>(this, 8, 1);
        let set = rd::<u8>(this, 8);
        wr::<u32>(this, 4, 500);
        if set == 0 {
            return;
        }
    }
    if rd::<u8>(this, 9) != 0 {
        wr::<u32>(this, 0, now());
        wr::<u8>(this, 9, 0);
    }
    let t = now();
    if t < rd::<u32>(this, 4).wrapping_add(rd::<u32>(this, 0)) {
        return;
    }
    wr::<u32>(this, 4, 500);
    wr::<u32>(this, 0, t);
    wr::<u8>(this, 8, 1);
    let task = tm_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    let ms = if !task.is_null() && task_is_go_to_task(task) != 0 { rd::<u32>(task, 8) } else { 1 };
    if ped_is_player(ped) != 0 || ms == 1 {
        return;
    }
    let task = tm_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if task.is_null() || task_is_go_to_task(task) == 0 {
        return;
    }
    let mut scanner = [0u32; 0x14];
    let sc: GPtr<u8> = GPtr::from_raw(scanner.as_mut_ptr()).cast();
    object_scanner_ctor(sc);
    object_scanner_scan_in_range(sc, ped);
    let obj = rd::<GPtr<u8>>(sc, 0x4C);
    if !obj.is_null() {
        test_object(ped, task, obj, ms);
    }
    wr::<u32>(sc, 0, ENTITY_SCANNER_VTABLE);
    entity_scanner_clear(sc);
}

/// The object tests and the event (0x6069AC..0x606B5B) of [`walk_into_object_scanner_scan`].
fn test_object(ped: GPtr<u8>, task: GPtr<u8>, obj: GPtr<u8>, ms: u32) {
    let flags = rd::<u32>(obj, 0x140);
    if flags & 0x400 != 0 || flags & 1 != 0 || rd::<u8>(obj, 0x1C) & 1 == 0 {
        return;
    }
    let mut v = [0f32; 3];
    let v_p: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    vector_difference(v_p, entity_pos(ped), entity_pos(obj));
    let m2 = X87::from_f64_exact(vector_magnitude_sqr(v_p.cast::<u8>()).to_f64());
    let range = k(K_RANGE);
    if x(range).fmul_m(range).fcom(m2).test_any(0x41) {
        return;
    }
    let target = task.byte_add(0xC);
    let bc = entity_get_bound_centre(obj, v_p);
    let mut slot = rd::<f32>(bc, 8);
    let low = x(rd::<f32>(entity_pos(ped), 8)).fsub_m(k(K_ONE));
    let high = x(rd::<f32>(entity_pos(ped), 8)).fadd_m(k(K_ONE)).to_f32();
    let idx = i32::from(rd::<i16>(obj, 0x22));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((idx as u32).wrapping_mul(4)) as usize).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let r = rd::<f32>(col, 0x24);
    let bottom = x(slot).fsub(x(r)).to_f32();
    let top = x(r).fadd_m(slot);
    if !low.fcom(top).test_any(0x41) {
        return;
    }
    if !x(high).fcom(x(bottom)).test_parity(5) {
        return;
    }
    slot = 0.0;
    let slot_p: GPtr<u8> = GPtr::from_raw(&mut slot as *mut f32).cast();
    if los_clear_ped(ped, target, obj, slot_p) != 0 {
        return;
    }
    if x(slot).fcom(x(k(K_MIN_DIST))).test_any(0x41) {
        return;
    }
    let mut down = [0f32, 0.0, 0.75];
    let mut start = [0f32; 3];
    let s = vector_sub(
        GPtr::from_raw(start.as_mut_ptr()).cast(),
        entity_pos(ped),
        GPtr::from_raw(down.as_mut_ptr()).cast(),
    );
    if los_clear_v3d(s, target, obj) != 0 {
        return;
    }
    let mut ev = [0u32; 7];
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    walk_into_object_ctor(ev_p, obj, ms);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0);
    walk_into_object_dtor(ev_p);
}

pub const HOOKS: &[HookReg] = &[HOOK_WALK_INTO_OBJECT_SCANNER_SCAN];
