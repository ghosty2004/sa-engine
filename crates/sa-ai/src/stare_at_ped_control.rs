//! WRITER-NATIVE (batch `native-22`): `CTaskComplexStareAtPed`'s destructor (0x660310) and `ControlSubTask`
//! (0x6605C0), reached in fist fights (gang members staring at the player) on the native build. Ported from the
//! 1.0 US bytes; x87 arithmetic follows the original's instruction order.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use crate::bindings::do_gang_abuse_speech_orig as gang_do_abuse_speech;
use sa_anim::bindings::ik_abort_look_at;
use crate::bindings::should_load_gang_anims as gang_should_load_gang_anims;
use crate::bindings::task_complex_dtor;
use sa_world::bindings::entity_clean_up_old_reference;
use sa_core::bindings::general_random_int_in_range;
use sa_stream::bindings::streaming_request_model;
use crate::bindings::tm_get_task_secondary;
use crate::bindings::tm_set_task_secondary;
use sa_anim::bindings::anim_add_block_ref;
use sa_stream::bindings::anim_remove_block_ref;
use sa_core::bindings::get_animation_block_index as anim_get_block_index;
use sa_world::bindings::ik_is_looking;
use sa_world::bindings::membership_get_leader;
use sa_world::bindings::vector_magnitude_sqr;
use sa_world::bindings::vector_sub;
use crate::bindings::ik_get_look_at_entity;
use crate::bindings::task_new as task_new_c;
use sa_world::bindings::run_anim_ctor;
use sa_world::bindings::ik_look_at;

/// `"gangs"` (0x86F92C)
const GANGS: usize = 0x86_F92C;
/// `g_ikChainMan` (0xC15448)
const IK_MAN: usize = 0xC1_5448;

/// `CTaskComplexStareAtPed::~CTaskComplexStareAtPed()` (0x660310, SEH frame, no `delete`): vtable 0x86FA2C,
/// `CleanUpOldReference(&+0x10)` with a target, the "gangs" anim block reference released when held (+0x25),
/// then the `CTaskComplex` destructor body.
#[hook(addr = 0x660310, conv = thiscall, name = "CTaskComplexStareAtPed::Destructor",
       expect = "6A FF 68 E8 45 84 00",
       unverified = "native-22", route = "L2", hazards = "heap-writes", recs = "weapon-combat,grove-walk")]
pub fn stare_at_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_FA2C);
    let t = rd::<GPtr<u8>>(this, 0x10);
    if !t.is_null() {
        entity_clean_up_old_reference(t, this.byte_add(0x10));
    }
    if rd::<u8>(this, 0x25) != 0 {
        anim_remove_block_ref(anim_get_block_index(GPtr::from_addr(GANGS)) as u32);
        wr::<u8>(this, 0x25, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexStareAtPed::ControlSubTask(CPed* ped)` (0x6605C0, `ret 4`, SEH frame).
///
/// The timer (+0x18 / +0x1C, flag +0x20, restart +0x21) runs out unsigned `now >= start + duration`. The "gangs"
/// anim block is requested / referenced (+0x25) while `ShouldLoadGangAnims()`, released otherwise. Without a
/// target (+0x10) or with the timer out: abort the ped's IK look-at (500 ms) when looking, null. Otherwise gang
/// abuse speech and an IK look-at at the target (first time, or when looking at something else: "TaskStareAtPed",
/// 9999999 ms, bone 5, speed 0.15, blend 500, priority 3). Then the same abort-and-null unless the anims are loaded,
/// there is a group (+0xC) leader and it is within 8 m of the target (`|target - leader|^2 <= 64`, ordered); with no
/// secondary task 4, on `rand(0, 100) == 50` a `CTaskSimpleRunAnim(0x34, 0x117 + ftol(rand16 / 32768 * 8), 4.0)`
/// becomes secondary task 4. Returns the sub-task.
#[hook(addr = 0x6605C0, conv = thiscall, name = "CTaskComplexStareAtPed::ControlSubTask",
       expect = "6A FF 64 A1 00 00 00 00",
       unverified = "native-22", route = "L2", hazards = "heap-writes", recs = "weapon-combat,grove-walk",
       note = "rand() at site 0x6607F4")]
pub fn stare_at_ped_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut done = false;
    if rd::<u8>(this, 0x20) != 0 {
        if rd::<u8>(this, 0x21) != 0 {
            wr::<u32>(this, 0x18, g::<u32>(0xB7_CB84).read());
            wr::<u8>(this, 0x21, 0);
        }
        let now = g::<u32>(0xB7_CB84).read();
        done = now >= rd::<u32>(this, 0x1C).wrapping_add(rd::<u32>(this, 0x18));
    }
    if rd::<u8>(this, 0x25) == 0 {
        if gang_should_load_gang_anims() != 0 {
            let idx = anim_get_block_index(GPtr::from_addr(GANGS));
            let loaded = g::<u8>((0xB5_D4A0u32).wrapping_add((idx as u32).wrapping_shl(5)) as usize + 0x10).read();
            if loaded != 1 {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
            } else {
                anim_add_block_ref(idx);
                wr::<u8>(this, 0x25, 1);
            }
        }
    } else if gang_should_load_gang_anims() == 0 {
        anim_remove_block_ref(anim_get_block_index(GPtr::from_addr(GANGS)) as u32);
        wr::<u8>(this, 0x25, 0);
    }
    let ik: GPtr<u8> = GPtr::from_addr(IK_MAN);
    let fail = || {
        if ik_is_looking(ik, ped) != 0 {
            ik_abort_look_at(ik, ped, 0x1F4);
        }
        GPtr::NULL
    };
    let target = rd::<GPtr<u8>>(this, 0x10);
    if target.is_null() || done {
        return fail();
    }
    gang_do_abuse_speech(ped, target);
    let look = |t: GPtr<u8>| ik_look_at(ik, GPtr::from_addr(0x86_FA58), ped, t, 0x98_967F, 5, GPtr::NULL, 1, f32::from_bits(0x3E19_999A), 0x1F4, 3, 0);
    if rd::<u8>(this, 0x24) == 0 {
        look(rd::<GPtr<u8>>(this, 0x10));
        wr::<u8>(this, 0x24, 1);
    } else {
        let t = rd::<GPtr<u8>>(this, 0x10);
        if ik_get_look_at_entity(ik, ped) != t {
            look(t);
        }
    }
    let group = rd::<GPtr<u8>>(this, 0xC);
    if rd::<u8>(this, 0x25) == 0 || rd::<u32>(this, 0x10) == 0 || group.is_null() {
        return fail();
    }
    if membership_get_leader(group.byte_add(8)).is_null() {
        return fail();
    }
    let leader = membership_get_leader(rd::<GPtr<u8>>(this, 0xC).byte_add(8));
    let lm = rd::<GPtr<u8>>(leader, 0x14);
    let lp = if lm.is_null() { leader.byte_add(4) } else { lm.byte_add(0x30) };
    let t = rd::<GPtr<u8>>(this, 0x10);
    let tm = rd::<GPtr<u8>>(t, 0x14);
    let tp = if tm.is_null() { t.byte_add(4) } else { tm.byte_add(0x30) };
    let mut d = [0f32; 3];
    let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    vector_sub(dp, tp, lp);
    if X87::from_f64_exact(vector_magnitude_sqr(dp.cast::<u8>()).to_f64()).fcom(x(64.0)).test_parity(0x41) {
        return fail();
    }
    let tasks = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    if tm_get_task_secondary(tasks, 4).is_null() {
        let r = general_random_int_in_range(0, 0x64);
        if !xi(r).fcom(x(50.0)).test_parity(0x44) {
            let t = task_new_c(0x20);
            let t = if t.is_null() {
                t
            } else {
                let r2 = sa_core::orig::at_site(0x66_07F4, sa_core::crt::rand) & 0xFFFF;
                let a = ftol32(xi(r2).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(8.0));
                run_anim_ctor(t, 0x34, (a + 0x117) as u32, 4.0, 0)
            };
            tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_STARE_AT_PED_DTOR, HOOK_STARE_AT_PED_CONTROL_SUB_TASK];
