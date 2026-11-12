//! The `CPedAttractor` base class (batch `std13-ai-1`): the constructor, `RegisterPed` / `DeRegisterPed`,
//! `SetTaskForPed`, `BroadcastDeparture` and the queue geometry (`ComputeAttractPos` / `ComputeAttractHeading` and
//! their random deltas). Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the bytes are
//! followed). Unverified (docs/verify/HOWTO.md).
//!
//! Layout (0x8C bytes): +0 vtable, +4 effect, +8 entity (a registered reference), +0xC the attracted peds
//! (`vector<CPed*>`, first +0x10, last +0x14), +0x1C the arrived peds / queue (first +0x20, last +0x24), +0x2C the
//! `CPedTaskPair` vector (first +0x30, last +0x34; 0x14-byte pairs: ped, task, slot, -, used flag +0x10), +0x3C maximum
//! peds, +0x40 spacing, +0x44 achieve-queue time, +0x48 shuffle time, +0x4C arrive range, +0x50 heading range, +0x54
//! delta position, +0x58 delta heading, +0x5C position, +0x68 queue direction, +0x74 use direction, +0x80 move state,
//! +0x84 the script name. Vtable 0x86C538: 0 `GetType`, 4 deleting destructor, 8 `IsAtHeadOfQueue`, 0xC
//! `ComputeAttractPos`, 0x10 `ComputeAttractHeading`, 0x14 `BroadcastDeparture`; nine of the ten derived classes keep
//! 0x8..0x14 (`CPedTriggerScriptAttractor`, 0x86C5B4, overrides them).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(dst: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::SetScale(float)` (0x59AED0)
    fn matrix_set_scale(m: GPtr<u8>, s: u32) = thiscall 0x59AED0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` into `out` (0x59C670, thiscall on the result)
    fn vector_from_multiply(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x59C670 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// CRT `sprintf(buf, fmt)` (0x821BB5; the script name is the format)
    fn crt_sprintf(buf: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` (0x53CBE0, ST0)
    fn radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `std::vector<CPed*>::push_back` (0x5EE200, `ret 4`)
    fn peds_push_back(v: GPtr<u8>, val: GPtr<u8>) = thiscall 0x5EE200 => crate::attractor_ptr_vectors::push_back_p;
    /// `std::vector<CTask*>::push_back` (0x5EE300, `ret 4`)
    fn tasks_push_back(v: GPtr<u8>, val: GPtr<u8>) = thiscall 0x5EE300 => crate::attractor_ptr_vectors::push_back_t;
    /// `uninitialized_fill_n<CTask**>` (0x5EB310)
    fn tasks_fill_n(dest: u32, n: u32, val: GPtr<u8>) -> u32 = cdecl 0x5EB310 => crate::attractor_ptr_vectors::fill_n_t;
    /// `std::vector<CTask*>::_Insert_n` (0x5ED4B0, `ret 0xC`)
    fn tasks_insert_n(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5ED4B0 => crate::attractor_ptr_vectors::insert_n_t;
    /// `std::vector<CPedTaskPair>::push_back` (0x5EE270, `ret 4`)
    fn pairs_push_back(v: GPtr<u8>, val: GPtr<u8>) = thiscall 0x5EE270 => crate::attractor_task_pair_vector::push_back;
    /// `std::copy<CPedTaskPair*>` (0x5EAB80, result through the first argument)
    fn pairs_copy(out: GPtr<u8>, first: u32, last: u32, dest: u32) -> GPtr<u8> = cdecl 0x5EAB80 => crate::attractor_task_pair_vector::copy;
    /// `CTaskComplexGoToAttractor(CPedAttractor*, const CVector&, float heading, float time, int slot, int moveState)`
    /// (0x66B640, `ret 0x18`)
    fn go_to_attractor_ctor(t: GPtr<u8>, a: GPtr<u8>, pos: GPtr<u8>, heading: u32, time: u32, slot: i32, ms: u32) -> GPtr<u8> = thiscall 0x66B640 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskSimpleStandStill(int ms, bool looped, bool useAnimIdle, float blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CPedAttractor::ComputeAttractPos(int, CVector&)` (vtable slot 0xC, `ret 8`)
    fn compute_attract_pos_v(a: GPtr<u8>, slot: i32, out: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// `CPedAttractor::ComputeAttractHeading(int, float&)` (vtable slot 0x10, `ret 8`)
    fn compute_attract_heading_v(a: GPtr<u8>, slot: i32, out: GPtr<u8>) = thiscall vslot 0x10 => unported;
    /// `CPedAttractor::BroadcastDeparture(CPed*)` (vtable slot 0x14, `ret 4`)
    fn broadcast_departure_v(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x14 => unported;
    /// `CTask`'s deleting destructor (vtable slot 0, `ret 4`)
    fn task_delete_v(t: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    fn set_task_for_ped_o(a: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) = thiscall 0x5EECA0 => set_task_for_ped;
    fn compute_delta_pos_o(a: GPtr<u8>) -> X87 = thiscall 0x5E9600 => compute_delta_pos;
    fn compute_delta_heading_o(a: GPtr<u8>) -> X87 = thiscall 0x5E9640 => compute_delta_heading;
}

/// `CPedAttractor::ms_tasks` (`std::vector<CTask*>`: first 0xC09860, last 0xC09864, end 0xC09868)
const MS_TASKS: usize = 0xC0985C;
/// The `1 / RAND_MAX` float the random deltas scale by
const INV_RAND_MAX: usize = 0x858C7C;
/// `PEDMOVE_WALK`
const MOVE_WALK: u32 = 4;

fn pa<const N: usize>(v: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// Element count of a pointer vector whose first / last are at `+first` / `+first + 4` (0 without storage).
fn count4(a: GPtr<u8>, first: isize) -> i32 {
    let f = rd::<u32>(a, first);
    if f == 0 { 0 } else { (rd::<u32>(a, first + 4).wrapping_sub(f) as i32) >> 2 }
}

/// Erases the dword at `it` from the pointer vector whose last is at `+last` (shift down, last -= 4).
fn erase4(a: GPtr<u8>, last: isize, it: u32) {
    let end = rd::<u32>(a, last);
    let mut p = it.wrapping_add(4);
    while p != end {
        GPtr::<u32>::from_addr(p as usize - 4).write(GPtr::<u32>::from_addr(p as usize).read());
        p = p.wrapping_add(4);
    }
    wr::<u32>(a, last, rd::<u32>(a, last).wrapping_sub(4));
}

/// The address of `ped` in the pointer vector [first, last) at `+first`, if any.
fn find4(a: GPtr<u8>, first: isize, ped: GPtr<u8>) -> Option<u32> {
    let (mut p, end) = (rd::<u32>(a, first), rd::<u32>(a, first + 4));
    while p != end {
        if GPtr::<u32>::from_addr(p as usize).read() == ped.addr() as u32 {
            return Some(p);
        }
        p = p.wrapping_add(4);
    }
    None
}

/// `CPedAttractor::CPedAttractor(C2dEffectPedAttractor*, CEntity*, eMoveState, int maxPeds, float spacing, float
/// achieveQueueTime, float shuffleTime, float arriveRange, float headingRange, float deltaPos, float deltaHeading)`
/// (0x5EDFB0, thiscall, `ret 0x2C`; entry `push -1; jmp 0x404CB7`, the protection fragment pushes the SEH handler).
/// Stores the vtable 0x86C538, the effect, the entity (`RegisterReference` on +8 when non-null), clears the three
/// vectors and copies the arguments to +0x3C..+0x58 and +0x80. A local `CMatrix` (attached matrix 0, not owned) is the
/// entity's matrix (allocated and updated first when it has none) or `SetScale(1.0)` without an entity: position =
/// matrix * effect position, queue direction = `Multiply3x3(matrix, effect +0x10)`, use direction =
/// `Multiply3x3(matrix, effect +0x1C)`; `sprintf(+0x84, effect +0x38)`; `~CMatrix`. Returns `this`.
#[hook(addr = 0x5EDFB0, conv = thiscall, name = "CPedAttractor::Constructor", expect = "6A FF E9 00 6D E1 FF",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "entry `push -1; jmp` into the protection fragment 0x404CB7; its rel32 is read as a key at 0x40206A (keycells); the original keeps `this` in a local")]
#[allow(clippy::too_many_arguments)]
pub fn ped_attractor_ctor(
    this: GPtr<u8>,
    fx: GPtr<u8>,
    entity: GPtr<u8>,
    move_state: u32,
    max_peds: u32,
    spacing: u32,
    achieve_time: u32,
    shuffle_time: u32,
    arrive_range: u32,
    heading_range: u32,
    delta_pos: u32,
    delta_heading: u32,
) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C538);
    wr::<GPtr<u8>>(this, 4, fx);
    wr::<GPtr<u8>>(this, 8, entity);
    for off in [0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x30, 0x34, 0x38] {
        wr::<u32>(this, off, 0);
    }
    wr::<u32>(this, 0x3C, max_peds);
    wr::<u32>(this, 0x48, shuffle_time);
    wr::<u32>(this, 0x40, spacing);
    wr::<u32>(this, 0x44, achieve_time);
    wr::<u32>(this, 0x54, delta_pos);
    wr::<u32>(this, 0x4C, arrive_range);
    wr::<u32>(this, 0x50, heading_range);
    wr::<u32>(this, 0x58, delta_heading);
    wr::<u32>(this, 0x80, move_state);
    if !entity.is_null() {
        entity_register_reference(entity, this.byte_add(8));
    }
    let mut mat = [0u32; 0x48 / 4];
    let e = rd::<GPtr<u8>>(this, 8);
    if e.is_null() {
        matrix_set_scale(pa(&mut mat), 1.0f32.to_bits());
    } else {
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        matrix_assign(pa(&mut mat), rd::<GPtr<u8>>(e, 0x14));
    }
    vector_from_multiply(this.byte_add(0x5C), pa(&mut mat), fx);
    for (src, dst) in [(0x10, 0x68), (0x1C, 0x74)] {
        let mut v = [rd::<u32>(fx, src), rd::<u32>(fx, src + 4), rd::<u32>(fx, src + 8)];
        let mut out = [0u32; 3];
        let r = multiply3x3(pa(&mut out), pa(&mut mat), pa(&mut v));
        for i in 0..3 {
            wr::<u32>(this, dst + 4 * i, rd::<u32>(r, 4 * i));
        }
    }
    crt_sprintf(this.byte_add(0x84), fx.byte_add(0x38));
    matrix_dtor(pa(&mut mat));
    this
}

/// `CPedAttractor::SetTaskForPed(CPed*, CTask*)` (0x5EECA0, thiscall, `ret 8`). The pair of `ped` (searched over the
/// count taken at entry): when its task is unused and non-null it is erased from `ms_tasks` and deleted (deleting
/// destructor, flag 1; the delete runs whenever the task is non-null after the search); then task = `task`, used = 0
/// and `ms_tasks.push_back(&pair.task)`. Without a pair: `push_back` of {ped, task, -1, 0, used 0} and `task` pushed
/// to `ms_tasks` inline (`uninitialized_fill_n` at last, or `_Insert_n` without room).
#[hook(addr = 0x5EECA0, conv = thiscall, name = "CPedAttractor::SetTaskForPed", expect = "8B 41 30 83 EC 14 85 C0",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,uninit-locals", recs = "grove-walk",
       note = "the local pair's bytes +0x11..+0x13 are stack residue in the original (0 here)")]
pub fn set_task_for_ped(this: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) {
    let first = rd::<u32>(this, 0x30);
    let n = if first == 0 { 0 } else { rd::<u32>(this, 0x34).wrapping_sub(first) as i32 / 0x14 };
    for i in 0..n {
        let p = GPtr::<u8>::from_addr(first as usize + 0x14 * i as usize);
        if rd::<GPtr<u8>>(p, 0) != ped {
            continue;
        }
        let old = rd::<u32>(p, 4);
        if rd::<u8>(p, 0x10) == 0 && old != 0 {
            let tasks = GPtr::<u8>::from_addr(MS_TASKS);
            if let Some(it) = find4(tasks, 4, GPtr::from_addr(old as usize)) {
                erase4(tasks, 8, it);
            }
            let t = rd::<GPtr<u8>>(p, 4);
            if !t.is_null() {
                task_delete_v(t, 1);
            }
        }
        wr::<GPtr<u8>>(p, 4, task);
        wr::<u8>(p, 0x10, 0);
        tasks_push_back(GPtr::from_addr(MS_TASKS), p.byte_add(4));
        return;
    }
    let mut pair = [ped.addr() as u32, task.addr() as u32, 0xFFFF_FFFF, 0, 0];
    pairs_push_back(this.byte_add(0x2C), pa(&mut pair));
    let tasks = GPtr::<u8>::from_addr(MS_TASKS);
    let (tf, tl) = (rd::<u32>(tasks, 4), rd::<u32>(tasks, 8));
    let size = (tl.wrapping_sub(tf) as i32 >> 2) as u32;
    let cap = (rd::<u32>(tasks, 0xC).wrapping_sub(tf) as i32 >> 2) as u32;
    let tp = pa(&mut pair).byte_add(4);
    if tf != 0 && size < cap {
        tasks_fill_n(tl, 1, tp);
        wr::<u32>(tasks, 8, tl.wrapping_add(4));
    } else {
        tasks_insert_n(tasks, tl, 1, tp);
    }
}

/// `CPedAttractor::RegisterPed(CPed*)` (0x5EEE30, thiscall, `ret 4`, SEH frame). A ped already in the attracted list
/// is erased from it and AL = 0. AL = 0 when attracted + arrived >= the maximum (+0x3C, signed). Otherwise the ped is
/// pushed to the attracted list, slot = the arrived count, `ComputeAttractPos(slot)` / `ComputeAttractHeading(slot)`
/// through the vtable, a `CTaskComplexGoToAttractor(this, pos, heading, +0x44, slot, +0x80)` (0x2C bytes from the task
/// pool; null when full) is handed to `SetTaskForPed`, AL = 1.
#[hook(addr = 0x5EEE30, conv = thiscall, name = "CPedAttractor::RegisterPed", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,uninit-locals,ax-al-return", recs = "grove-walk",
       note = "pos / heading stay stack residue in the original when the virtual calls write nothing (no effect); 0 here")]
pub fn register_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if let Some(it) = find4(this, 0x10, ped) {
        erase4(this, 0x14, it);
        return 0;
    }
    if count4(this, 0x20) + count4(this, 0x10) >= rd::<i32>(this, 0x3C) {
        return 0;
    }
    let mut p = [ped.addr() as u32];
    peds_push_back(this.byte_add(0xC), pa(&mut p));
    let slot = count4(this, 0x20);
    let mut pos = [0u32; 3];
    compute_attract_pos_v(this, slot, pa(&mut pos));
    let mut heading = [0u32; 1];
    compute_attract_heading_v(this, slot, pa(&mut heading));
    let time = rd::<u32>(this, 0x44);
    let t = task_new(0x2C);
    let task = if t.is_null() {
        GPtr::NULL
    } else {
        go_to_attractor_ctor(t, this, pa(&mut pos), heading[0], time, slot, rd::<u32>(this, 0x80))
    };
    set_task_for_ped_o(this, ped, task);
    1
}

/// `CPedAttractor::DeRegisterPed(CPed*)` (0x5EC5B0, thiscall, `ret 4`). The ped's task pair is erased (`std::copy`
/// of the rest down, last -= 0x14); a ped in the attracted list is erased from it (AL = 1); otherwise AL =
/// `BroadcastDeparture(ped)` through the vtable.
#[hook(addr = 0x5EC5B0, conv = thiscall, name = "CPedAttractor::DeRegisterPed", expect = "53 8B 5C 24 08 56 8B F1",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn deregister_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    erase_pair(this, ped);
    if let Some(it) = find4(this, 0x10, ped) {
        erase4(this, 0x14, it);
        return 1;
    }
    broadcast_departure_v(this, ped)
}

/// Erases `ped`'s pair from the pair vector (+0x30 / +0x34) by `std::copy` (0x5EAB80) when there is one.
fn erase_pair(this: GPtr<u8>, ped: GPtr<u8>) {
    let (mut p, end) = (rd::<u32>(this, 0x30), rd::<u32>(this, 0x34));
    while p != end {
        if GPtr::<u32>::from_addr(p as usize).read() == ped.addr() as u32 {
            let mut out = [0u32];
            pairs_copy(pa(&mut out), p.wrapping_add(0x14), rd::<u32>(this, 0x34), p);
            wr::<u32>(this, 0x34, rd::<u32>(this, 0x34).wrapping_sub(0x14));
            return;
        }
        p = p.wrapping_add(0x14);
    }
}

/// `-d + rand() * (1 / RAND_MAX) * (d - -d)` in x87 precision, d = the float at `off` (`CGeneral::
/// GetRandomNumberInRange(-d, d)` inlined); ST0.
fn random_delta(this: GPtr<u8>, off: isize, rand_ret: u32) -> X87 {
    let d = rd::<f32>(this, off);
    let lo = x(d).fchs().to_f32();
    let r = at_site(rand_ret, rand);
    let a = xi(r).fmul_m(g::<f32>(INV_RAND_MAX).read());
    let b = x(d).fsub_m(lo);
    a.fmul(b).fadd_m(lo)
}

/// `CPedAttractor::ComputeDeltaPos()` (0x5E9600, thiscall): a random value in [-delta, delta] of +0x54, ST0.
#[hook(addr = 0x5E9600, conv = thiscall, name = "CPedAttractor::ComputeDeltaPos", expect = "83 EC 0C D9 41 54 8B 41",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk", note = "rand() at 0x5E9613")]
pub fn compute_delta_pos(this: GPtr<u8>) -> X87 {
    random_delta(this, 0x54, 0x5E9618)
}

/// `CPedAttractor::ComputeDeltaHeading()` (0x5E9640, thiscall): a random value in [-delta, delta] of +0x58, ST0.
#[hook(addr = 0x5E9640, conv = thiscall, name = "CPedAttractor::ComputeDeltaHeading", expect = "83 EC 0C D9 41 58 8B 41",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk", note = "rand() at 0x5E9653")]
pub fn compute_delta_heading(this: GPtr<u8>) -> X87 {
    random_delta(this, 0x58, 0x5E9658)
}

/// `CPedAttractor::ComputeAttractPos(int slot, CVector& out)` (0x5EA110, thiscall, `ret 8`; vtable slot 0xC). Nothing
/// without an effect. s = (float)slot: out = position - spacing * s * queue direction, with the x87 rounding of the
/// original (x: `s*qx*spacing` unrounded; y, z: `s*q` rounded to float first); slot != 0 adds `ComputeDeltaPos()` to
/// x and then to y.
#[hook(addr = 0x5EA110, conv = thiscall, name = "CPedAttractor::ComputeAttractPos-v", expect = "83 EC 1C 56 8B F1 8B 46",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_attract_pos(this: GPtr<u8>, slot: i32, out: GPtr<u8>) {
    if rd::<u32>(this, 4) == 0 {
        return;
    }
    let s = xi(slot);
    let spacing = rd::<f32>(this, 0x40);
    let a = s.fmul_m(rd::<f32>(this, 0x68));
    let b = s.fmul_m(rd::<f32>(this, 0x6C)).to_f32();
    let c = s.fmul_m(rd::<f32>(this, 0x70)).to_f32();
    let a = a.fmul_m(spacing);
    let sb = x(spacing).fmul_m(b);
    let z = x(c).fmul_m(spacing).to_f32();
    let ox = x(rd::<f32>(this, 0x5C)).fsub(a).to_f32();
    let oy = x(rd::<f32>(this, 0x60)).fsub(sb).to_f32();
    wr::<f32>(out, 0, ox);
    wr::<f32>(out, 4, oy);
    let oz = x(rd::<f32>(this, 0x64)).fsub_m(z).to_f32();
    wr::<f32>(out, 8, oz);
    if slot != 0 {
        let v = compute_delta_pos_o(this).fadd_m(rd::<f32>(out, 0)).to_f32();
        wr::<f32>(out, 0, v);
        let v = compute_delta_pos_o(this).fadd_m(rd::<f32>(out, 4)).to_f32();
        wr::<f32>(out, 4, v);
    }
}

/// `CPedAttractor::ComputeAttractHeading(int queue, float& out)` (0x5EA1C0, thiscall, `ret 8`; vtable slot 0x10).
/// Nothing without an effect. out = `GetRadianAngleBetweenPoints(dir.x, dir.y, 0, 0)` of the queue direction (+0x68)
/// when `queue` is non-zero, else of the use direction (+0x74); with `queue`, `ComputeDeltaHeading()` is added.
#[hook(addr = 0x5EA1C0, conv = thiscall, name = "CPedAttractor::ComputeAttractHeading-v", expect = "83 EC 0C 57 8B F9 8B 47",
       unverified = "std13-ai-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the original copies dir.z to a local it never reads")]
pub fn compute_attract_heading(this: GPtr<u8>, queue: i32, out: GPtr<u8>) {
    if rd::<u32>(this, 4) == 0 {
        return;
    }
    let dir = if queue != 0 { 0x68 } else { 0x74 };
    let h = radian_angle_between_points(rd::<u32>(this, dir), rd::<u32>(this, dir + 4), 0, 0);
    wr::<f32>(out, 0, X87::from_f64_exact(h).to_f32());
    if queue != 0 {
        let v = compute_delta_heading_o(this).fadd_m(rd::<f32>(out, 0)).to_f32();
        wr::<f32>(out, 0, v);
    }
}

/// `CPedAttractor::BroadcastDeparture(CPed*)` (0x5EF160, thiscall, `ret 4`, SEH frame; vtable slot 0x14). AL = 0 when
/// the ped is not in the queue (+0x20). Each later queue ped i gets a sequence {`CTaskSimpleStandStill(2000 * i, 0,
/// 0, 8.0)`, `CTaskComplexGoToAttractor(this, pos(i - 1), heading(i - 1), +0x48, i - 1, walk)`} (positions and
/// headings through the vtable; `AddTask` is called on a null sequence too when the pool is full, as the original
/// does). The ped's pair and queue entry are erased; then every attracted ped (count taken once) gets a
/// `CTaskComplexGoToAttractor(this, pos(n), heading(n), +0x44, n, walk)`, n = the queue length. AL = 1.
#[hook(addr = 0x5EF160, conv = thiscall, name = "CPedAttractor::BroadcastDeparture", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,uninit-locals,stack-residue,ax-al-return",
       recs = "grove-walk", note = "the original keeps counts in its argument slot")]
pub fn broadcast_departure(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let count = count4(this, 0x20);
    let Some(idx) = (0..count).find(|&i| GPtr::<GPtr<u8>>::from_addr(rd::<u32>(this, 0x20) as usize + 4 * i as usize).read() == ped)
    else {
        return 0;
    };
    for i in idx + 1..count {
        let p = GPtr::<GPtr<u8>>::from_addr(rd::<u32>(this, 0x20) as usize + 4 * i as usize).read();
        let mut pos = [0u32; 3];
        compute_attract_pos_v(this, i - 1, pa(&mut pos));
        let mut heading = [0u32; 1];
        compute_attract_heading_v(this, i - 1, pa(&mut heading));
        let time = rd::<u32>(this, 0x48);
        let t = task_new(0x40);
        let seq = if t.is_null() { GPtr::NULL } else { sequence_ctor(t) };
        let t = task_new(0x20);
        let stand = if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 2000 * i, 0, 0, 0x4100_0000) };
        sequence_add_task(seq, stand);
        let t = task_new(0x2C);
        let go = if t.is_null() {
            GPtr::NULL
        } else {
            go_to_attractor_ctor(t, this, pa(&mut pos), heading[0], time, i - 1, MOVE_WALK)
        };
        sequence_add_task(seq, go);
        set_task_for_ped_o(this, p, seq);
    }
    erase_pair_inline(this, ped);
    erase4(this, 0x24, rd::<u32>(this, 0x20).wrapping_add(4 * idx as u32));
    let attracted = count4(this, 0x10);
    for j in 0..attracted {
        let p = GPtr::<GPtr<u8>>::from_addr(rd::<u32>(this, 0x10) as usize + 4 * j as usize).read();
        let n = count4(this, 0x20);
        let mut pos = [0u32; 3];
        compute_attract_pos_v(this, n, pa(&mut pos));
        let mut heading = [0u32; 1];
        compute_attract_heading_v(this, n, pa(&mut heading));
        let time = rd::<u32>(this, 0x44);
        let t = task_new(0x2C);
        let go = if t.is_null() {
            GPtr::NULL
        } else {
            go_to_attractor_ctor(t, this, pa(&mut pos), heading[0], time, n, MOVE_WALK)
        };
        set_task_for_ped_o(this, p, go);
    }
    1
}

/// The pair erase `BroadcastDeparture` inlines (5-dword moves, last -= 0x14).
fn erase_pair_inline(this: GPtr<u8>, ped: GPtr<u8>) {
    let (mut p, end) = (rd::<u32>(this, 0x30), rd::<u32>(this, 0x34));
    while p != end {
        if GPtr::<u32>::from_addr(p as usize).read() == ped.addr() as u32 {
            let mut s = p.wrapping_add(0x14);
            while s != end {
                for k in 0..5 {
                    let v = GPtr::<u32>::from_addr(s as usize + 4 * k).read();
                    GPtr::<u32>::from_addr(s as usize - 0x14 + 4 * k).write(v);
                }
                s = s.wrapping_add(0x14);
            }
            wr::<u32>(this, 0x34, rd::<u32>(this, 0x34).wrapping_sub(0x14));
            return;
        }
        p = p.wrapping_add(0x14);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PED_ATTRACTOR_CTOR,
    HOOK_SET_TASK_FOR_PED,
    HOOK_REGISTER_PED,
    HOOK_DEREGISTER_PED,
    HOOK_COMPUTE_DELTA_POS,
    HOOK_COMPUTE_DELTA_HEADING,
    HOOK_COMPUTE_ATTRACT_POS,
    HOOK_COMPUTE_ATTRACT_HEADING,
    HOOK_BROADCAST_DEPARTURE,
];
