//! WRITER-NATIVE (batch `native-22`): `CTaskComplexStareAtPed`'s destructor (0x660310) and `ControlSubTask`
//! (0x6605C0), reached in fist fights (gang members staring at the player) on the native build. Ported from the
//! 1.0 US bytes; x87 arithmetic follows the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CTaskComplex::~CTaskComplex()` body (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTaskComplexGangLeader::ShouldLoadGangAnims()` (0x65E7F0)
    fn gang_should_load_gang_anims() -> u8 = cdecl 0x65E7F0 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CTaskComplexGangLeader::DoGangAbuseSpeech(CPed*, CPed*)` (0x65E860)
    fn gang_do_abuse_speech(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x65E860 => unported;
    /// `IKChainManager_c::LookAt(const char*, CPed*, CEntity*, int32 time, int32 bone, RwV3d*, bool, float speed,
    /// int32 blend, int32 priority, bool)` (0x618970)
    #[allow(clippy::too_many_arguments)]
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, posn: u32, b: u32, speed: f32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::GetLookAtEntity(CPed*)` (0x6181D0)
    fn ik_get_look_at_entity(m: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6181D0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blend)` (0x618280)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: i32) = thiscall 0x618280 => unported;
    /// `CPedGroupMembership::GetLeader()` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, i: i32) = thiscall 0x681B60 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_c(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskSimpleRunAnim(AssocGroupId, AnimationId, float blend, bool)` (0x61A8B0)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: u32, blend: f32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
}

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
        anim_remove_block_ref(anim_get_block_index(GPtr::from_addr(GANGS)));
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
        anim_remove_block_ref(anim_get_block_index(GPtr::from_addr(GANGS)));
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
    let look = |t: GPtr<u8>| ik_look_at(ik, 0x86_FA58, ped, t, 0x98_967F, 5, 0, 1, f32::from_bits(0x3E19_999A), 0x1F4, 3, 0);
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
    if X87::from_f64_exact(vector_magnitude_sqr(dp)).fcom(x(64.0)).test_parity(0x41) {
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
