//! WRITER-NATIVE (batch `native-19`): the `CEventPassObject` destructor body (0x65DCF0) and
//! `CTaskComplexPassObject::CreateFirstSubTask` (0x6613A0), reached when ambient peds hand objects over on the
//! native walk. Ported from the 1.0 US bytes; x87 arithmetic follows the original's instruction order.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

use crate::bindings::event_base_dtor as event_dtor;
use crate::bindings::ik_abort_point_arm;
use crate::bindings::ik_is_arm_pointing;
use sa_world::bindings::entity_clean_up_old_reference;
use sa_core::bindings::entity_register_reference;
use crate::bindings::tm_set_task_secondary;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::hold_entity_drop_entity;
use sa_world::bindings::ped_get_held_entity as ped_get_entity_held;
use crate::bindings::tm_find_active_task_by_type;
use sa_world::bindings::turn_to_face_entity_ctor;
use crate::bindings::task_new as task_new_c;
use sa_world::bindings::hold_entity_ctor;
orig! {
    /// `CTaskComplexPassObject::AbortIK(CPed*)` (0x661290)
    fn pass_object_abort_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x661290 => unported;
}

/// `CEventPassObject::~CEventPassObject()` body (0x65DCF0, SEH frame, no `delete`): vtable 0x86F8B8,
/// `CleanUpOldReference(&+0xC)` when the giver is set, then the `CEvent` destructor body.
#[hook(addr = 0x65DCF0, conv = thiscall, name = "FUN_0065dcf0",
       expect = "6A FF 68 18 43 84 00",
       unverified = "native-19", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_pass_object_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_F8B8);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0xC));
    }
    event_dtor(this);
}

use sa_world::world::common::entity_pos as pos;

/// `CTaskComplexPassObject::CreateFirstSubTask(CPed* ped)` (0x6613A0, `ret 4`, SEH frame).
///
/// Fails without a partner (+0xC), or when the ped or the partner is the player. The object held by the ped
/// (+0x10 set) or the partner goes to +0x2C (registered reference). `d = partner - ped`; when `dx^2 + dy^2 + dz^2`
/// (dx as x87 times its f32) is at most 2.25 (`[0x86E104]`, ordered): +0x20 = now, +0x24 = 1000, +0x28 = 1, +0x14 =
/// partner + d / 2 and a `CTaskComplexTurnToFaceEntityOrCoord(partner, [0x86FC7C], [0x86FC80])` (null when the
/// pool is full). Otherwise null, aborting the ped's arm pointing (arm 0, 500 ms) first.
#[hook(addr = 0x6613A0, conv = thiscall, name = "CTaskComplexPassObject::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "native-19", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "keeps dy / 2 in the ped argument slot")]
pub fn pass_object_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let fail = |ped: GPtr<u8>| {
        let ik = GPtr::<u8>::from_addr(0xC1_5448);
        if ik_is_arm_pointing(ik, 0, ped) != 0 {
            ik_abort_point_arm(ik, 0, ped, 0x1F4);
        }
        GPtr::NULL
    };
    if rd::<u32>(this, 0xC) == 0 || ped_is_player(ped) != 0 || ped_is_player(rd::<GPtr<u8>>(this, 0xC)) != 0 {
        return fail(ped);
    }
    let holder = if rd::<u8>(this, 0x10) != 0 { ped } else { rd::<GPtr<u8>>(this, 0xC) };
    let held = ped_get_entity_held(holder);
    wr::<GPtr<u8>>(this, 0x2C, held);
    if !held.is_null() {
        entity_register_reference(held, this.byte_add(0x2C));
    }
    let pp = pos(ped);
    let qp = pos(rd::<GPtr<u8>>(this, 0xC));
    let dz = x(rd::<f32>(qp, 8)).fsub_m(rd::<f32>(pp, 8));
    let dy = x(rd::<f32>(qp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
    let dx80 = x(rd::<f32>(qp, 0)).fsub_m(rd::<f32>(pp, 0));
    let dx = dx80.to_f32();
    let d2 = dx80.fmul_m(dx).fadd(x(dy).fmul_m(dy)).fadd(dz.fmul(dz));
    if d2.fcom(x(f32::from_bits(0x4010_0000))).test_parity(0x41) {
        return fail(ped);
    }
    let hz = dz.fmul_m(0.5);
    wr::<u32>(this, 0x20, g::<u32>(0xB7_CB84).read());
    let hy = x(dy).fmul_m(0.5).to_f32();
    wr::<u32>(this, 0x24, 0x3E8);
    wr::<u8>(this, 0x28, 1);
    let hx = x(dx).fmul_m(0.5).to_f32();
    let qp = pos(rd::<GPtr<u8>>(this, 0xC));
    let mz = hz.fadd_m(rd::<f32>(qp, 8));
    let my = x(hy).fadd_m(rd::<f32>(qp, 4));
    let mx = x(hx).fadd_m(rd::<f32>(qp, 0));
    wr::<f32>(this, 0x14, mx.to_f32());
    wr::<f32>(this, 0x18, my.to_f32());
    wr::<f32>(this, 0x1C, mz.to_f32());
    let t = task_new_c(0x28);
    if t.is_null() {
        return t;
    }
    turn_to_face_entity_ctor(t, rd::<GPtr<u8>>(this, 0xC), g::<u32>(0x86_FC7C).read(), g::<u32>(0x86_FC80).read())
}

/// `CTaskComplexPassObject::ControlSubTask(CPed* ped)` (0x661570, `ret 4`, SEH frame).
///
/// Without a partner (+0xC): abort the ped's arm pointing (arm 0, 500 ms), null. Before the timer (+0x28 set; +0x29
/// restarts it at now; `now >= start +0x20 + duration +0x24` unsigned) has run out: the sub-task. Then, for the
/// receiving side (+0x10 clear) with an object (+0x2C): the partner's hold-entity task (0x133) drops its entity
/// and the ped gets `CTaskSimpleHoldEntity(entity, (0, 0, 0), 6, 0x10, 0xBF, 0, true)` as secondary task 4 (null
/// when the pool is full). `AbortIK(ped)`; null.
#[hook(addr = 0x661570, conv = thiscall, name = "CTaskComplexPassObject::ControlSubTask",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "native-19", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn pass_object_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        let ik = GPtr::<u8>::from_addr(0xC1_5448);
        if ik_is_arm_pointing(ik, 0, ped) != 0 {
            ik_abort_point_arm(ik, 0, ped, 0x1F4);
        }
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x28) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x29) != 0 {
        wr::<u32>(this, 0x20, g::<u32>(0xB7_CB84).read());
        wr::<u8>(this, 0x29, 0);
    }
    let now = g::<u32>(0xB7_CB84).read();
    if now < rd::<u32>(this, 0x24).wrapping_add(rd::<u32>(this, 0x20)) {
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x10) == 0 && rd::<u32>(this, 0x2C) != 0 {
        let partner = rd::<GPtr<u8>>(this, 0xC);
        let hold = tm_find_active_task_by_type(rd::<GPtr<u8>>(partner, 0x47C).byte_add(4), 0x133);
        if !hold.is_null() {
            let entity = rd::<GPtr<u8>>(hold, 8);
            hold_entity_drop_entity(hold, rd::<GPtr<u8>>(this, 0xC), 0);
            let mut zero = [0u32; 3];
            let t = task_new_c(0x3C);
            let t = if t.is_null() {
                t
            } else {
                hold_entity_ctor(t, entity, GPtr::from_raw(zero.as_mut_ptr()).cast(), 6, 0x10, 0xBF, 0, 1)
            };
            tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
        }
    }
    pass_object_abort_ik(this, ped);
    GPtr::NULL
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_PASS_OBJECT_DTOR, HOOK_PASS_OBJECT_CREATE_FIRST_SUB_TASK, HOOK_PASS_OBJECT_CONTROL_SUB_TASK];
