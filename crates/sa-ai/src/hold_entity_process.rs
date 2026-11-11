//! `CTaskSimpleHoldEntity::ProcessPed` (batch `mvpm-ai-2`): the shared per-frame body of the hold, pick-up and put-down
//! entity tasks. Ported from the 1.0 US bytes (gta-reversed `TaskSimpleHoldEntity.cpp` as a reading aid; the bytes
//! win: only the put-down task makes the secondary hold task abortable). Unverified (docs/verify/HOWTO.md).
//!
//! Layout (from the bytes): +8 held entity, +0xC offset (CVector), +0x1C heading, +0x20 animation id (0xBF none),
//! +0x24 animation group, +0x30 animation hierarchy, +0x34 u8 entity dropped, +0x35 u8 entity requires processing,
//! +0x36 u8 no drop at animation end, +0x38 animation association; pick-up task: +0x3C move-until progress, +0x40
//! pick-up offset (CVector2D used).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_task_make_abortable(t: GPtr<u8>, p: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimpleHoldEntity::ChoosePutDownHeight(CPed*)` (0x693440, `ret 4`)
    fn hold_choose_put_down_height(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x693440 => unported;
    /// `CTaskSimpleHoldEntity::ReleaseEntity()` (0x6916E0)
    fn hold_release_entity(t: GPtr<u8>) = thiscall 0x6916E0 => unported;
    /// `CPhysical::SetIsStatic(bool)` (vtable +0x10, `ret 4`)
    fn v_set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CTaskSimpleHoldEntity::StartAnim(CPed*)` (0x692FF0, `ret 4`)
    fn hold_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x692FF0 => unported;
    /// `CTaskSimpleHoldEntity::DropEntity(CPed*, bool)` (0x6930F0, `ret 8`)
    fn hold_drop_entity(t: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x6930F0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden result pointer first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::ExitVehicleJustDown()` (0x540120)
    fn pad_exit_vehicle_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540120 => unported;
    /// `CTaskSimplePutDownEntity::CTaskSimplePutDownEntity()` (0x691990)
    fn put_down_entity_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x691990 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn event_script_command_ctor(e: GPtr<u8>, idx: i32, task: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIME_STEP: usize = 0xB7CB5C;
/// 0.1f (0x870A64): the pick-up approach scale
const K_TENTH: usize = 0x870A64;
/// 0.0f (0x858B50)
const K_ZERO: usize = 0x858B50;

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}
fn secondary(ped: GPtr<u8>) -> GPtr<u8> {
    tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 4)
}

/// `CTaskSimpleHoldEntity::ProcessPed(CPed*)` (0x693C40, thiscall, `ret 4`), AL.
///
/// When +0x35 is set it is cleared first. A put-down task (type 0x135) without entity, hierarchy and group takes over
/// the entity of a secondary (slot 4) hold task (type 0x133, entity and group set; the secondary is fetched again for
/// each test): entity + registered reference, the offset (three dwords), animation id 0x13F, the group,
/// `ChoosePutDownHeight(ped)` and `ReleaseEntity()` on the hold task; otherwise the dropped flag is set. Then without an
/// entity, or when dropped, the dropped flag is set; else the entity loses its uses-collision bit (+0x1C bit 0); an
/// object (type 4 in +0x36 & 7) that is static (+0x1C bit 2 or bit 18) gets `SetIsStatic(false)` and
/// `AddToMovingList()`, and every object gets +0x40 |= 0x2000000 and +0xB8 = 0; the heading (+0x1C) becomes
/// `atan2(-forward.x, forward.y) - ped rotation (+0x558)` (FPATAN), or `placement heading - ped rotation` without a matrix.
///
/// A dropped task returns 1. With an animation (id != 0xBF or a hierarchy): no association → `StartAnim(ped)`; an
/// association whose blend delta (+0x1C) is below 0 → `DropEntity(ped, true)` unless +0x36. Then with an entity, by task
/// type: hold (0x133) in a carry group (0x51..0x53) of a player whose pad has `ExitVehicleJustDown()` → a new
/// `CTaskSimplePutDownEntity` (task pool, NULL when that fails) in a stack `CEventScriptCommand(3, task, false)` added to
/// the ped's event group, then the event destructor; pick-up (0x134) while the association is missing or its time
/// (+0x20) is below +0x3C, with a pick-up offset (x or y non-zero): `p = entity matrix * offset - ped position`
/// (x, y stored as f32, z kept on the FPU for the first product), `shift.x (+0x4D8) += ((p.z*R.z + p.y*R.y) + p.x*R.x)
/// / timeStep * 0.1` and `shift.y (+0x4DC)` the same with the forward row (the ped matrix is read without a null
/// check), and the aiming rotation (+0x55C) = `atan2(-d.x, d.y)` of `entity position - ped position` (`VectorSub`);
/// put-down (0x135): the secondary hold task (type 0x133) gets `MakeAbortable(ped, 1, NULL)`. Returns 0.
#[hook(addr = 0x693c40, conv = thiscall, name = "CTaskSimpleHoldEntity::ProcessPed", expect = "64 A1 00 00 00 00",
       unverified = "mvpm-ai-2", route = "L2", hazards = "x87-transcendental,heap-writes,stack-residue", recs = "grove-walk",
       note = "two FPATANs (sa_math::x87 model); the CEventScriptCommand and the transformed vectors live on the Rust stack; AL result with the upper EAX bytes of the last callee")]
pub fn hold_entity_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x35) != 0 {
        wr::<u8>(this, 0x35, 0);
        if v_task_type(this) == 0x135
            && rd::<u32>(this, 8) == 0
            && rd::<u32>(this, 0x30) == 0
            && rd::<u32>(this, 0x24) == 0
        {
            let mut taken = false;
            if !secondary(ped).is_null() && v_task_type(secondary(ped)) == 0x133 {
                let h = secondary(ped);
                if !h.is_null() {
                    let e = rd::<GPtr<u8>>(h, 8);
                    if !e.is_null() && rd::<u32>(h, 0x24) != 0 {
                        wr::<GPtr<u8>>(this, 8, e);
                        entity_register_reference(e, this.byte_add(8));
                        for k in 0..3 {
                            wr::<u32>(this, 0xC + 4 * k, rd::<u32>(h, 0xC + 4 * k));
                        }
                        wr::<u32>(this, 0x20, 0x13F);
                        wr::<u32>(this, 0x24, rd::<u32>(h, 0x24));
                        hold_choose_put_down_height(this, ped);
                        hold_release_entity(h);
                        taken = true;
                    }
                }
            }
            if !taken {
                wr::<u8>(this, 0x34, 1);
            }
        }
        let e = rd::<GPtr<u8>>(this, 8);
        if e.is_null() || rd::<u8>(this, 0x34) != 0 {
            wr::<u8>(this, 0x34, 1);
        } else {
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_FFFE);
            let e = rd::<GPtr<u8>>(this, 8);
            if rd::<u8>(e, 0x36) & 7 == 4 {
                let f = rd::<u32>(e, 0x1C);
                if f & 4 != 0 || f & 0x40000 != 0 {
                    v_set_is_static(e, 0);
                    physical_add_to_moving_list(rd::<GPtr<u8>>(this, 8));
                }
                let e2 = rd::<GPtr<u8>>(this, 8);
                wr::<u32>(e2, 0x40, rd::<u32>(e2, 0x40) | 0x200_0000);
                wr::<u8>(rd::<GPtr<u8>>(this, 8), 0xB8, 0);
            }
            let e = rd::<GPtr<u8>>(this, 8);
            let m = rd::<GPtr<u8>>(e, 0x14);
            let a = if m.is_null() {
                x(rd::<f32>(e, 0x10))
            } else {
                let fx = x(rd::<f32>(m, 0x10)).fchs();
                fx.fpatan(x(rd::<f32>(m, 0x14)))
            };
            wr::<f32>(this, 0x1C, a.fsub(x(rd::<f32>(ped, 0x558))).to_f32());
        }
    }
    if rd::<u8>(this, 0x34) != 0 {
        return 1;
    }
    if rd::<u32>(this, 0x20) != 0xBF || rd::<u32>(this, 0x30) != 0 {
        let assoc = rd::<GPtr<u8>>(this, 0x38);
        if assoc.is_null() {
            hold_start_anim(this, ped);
        } else if !x(rd::<f32>(assoc, 0x1C)).fcom(x(g::<f32>(K_ZERO).read())).test_parity(5)
            && rd::<u8>(this, 0x36) == 0
        {
            hold_drop_entity(this, ped, 1);
        }
    }
    let entity = rd::<GPtr<u8>>(this, 8);
    if entity.is_null() {
        return 0;
    }
    match v_task_type(this) {
        0x133 => {
            let grp = rd::<i32>(this, 0x24);
            if (grp == 0x51 || grp == 0x52 || grp == 0x53)
                && ped_is_player(ped) != 0
                && pad_exit_vehicle_just_down(player_get_pad(ped)) != 0
            {
                let mem = task_new(0x40);
                let task = if mem.is_null() { GPtr::NULL } else { put_down_entity_ctor(mem) };
                let mut ev = [0u32; 8];
                let e: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
                event_script_command_ctor(e, 3, task, 0);
                event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
                event_script_command_dtor(e);
            }
        }
        0x134 => {
            let assoc = rd::<GPtr<u8>>(this, 0x38);
            if !assoc.is_null() && x(rd::<f32>(this, 0x3C)).fcom(x(rd::<f32>(assoc, 0x20))).test_any(0x41) {
                return 0;
            }
            let zero = x(g::<f32>(K_ZERO).read());
            let ox_ne = x(rd::<f32>(this, 0x40)).fcom(zero).test_parity(0x44);
            if !ox_ne && !x(rd::<f32>(this, 0x44)).fcom(zero).test_parity(0x44) {
                return 0;
            }
            let mut p = [0f32; 3];
            let pp: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
            matrix_mul_vec(pp, placeable_get_matrix(entity), this.byte_add(0x40));
            let pm = rd::<GPtr<u8>>(ped, 0x14);
            let pos = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
            p[0] = x(p[0]).fsub_m(rd::<f32>(pos, 0)).to_f32();
            p[1] = x(p[1]).fsub_m(rd::<f32>(pos, 4)).to_f32();
            let pz = x(p[2]).fsub_m(rd::<f32>(pos, 8));
            p[2] = pz.to_f32();
            let ts = g::<f32>(TIME_STEP).read();
            let tenth = g::<f32>(K_TENTH).read();
            let sx = pz
                .fmul_m(rd::<f32>(pm, 8))
                .fadd(x(p[1]).fmul_m(rd::<f32>(pm, 4)))
                .fadd(x(p[0]).fmul_m(rd::<f32>(pm, 0)))
                .fdiv_m(ts)
                .fmul_m(tenth)
                .fadd_m(rd::<f32>(ped, 0x4D8));
            wr::<f32>(ped, 0x4D8, sx.to_f32());
            let sy = x(p[2])
                .fmul_m(rd::<f32>(pm, 0x18))
                .fadd(x(p[1]).fmul_m(rd::<f32>(pm, 0x14)))
                .fadd(x(p[0]).fmul_m(rd::<f32>(pm, 0x10)))
                .fdiv_m(ts)
                .fmul_m(tenth)
                .fadd_m(rd::<f32>(ped, 0x4DC));
            wr::<f32>(ped, 0x4DC, sy.to_f32());
            let mut d = [0f32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            let r = vector_sub(dp, entity_pos(rd::<GPtr<u8>>(this, 8)), pos);
            let nx = x(rd::<f32>(r, 0)).fchs();
            wr::<f32>(ped, 0x55C, nx.fpatan(x(rd::<f32>(r, 4))).to_f32());
        }
        0x135 => {
            if secondary(ped).is_null() || v_task_type(secondary(ped)) != 0x133 {
                return 0;
            }
            v_task_make_abortable(secondary(ped), ped, 1, 0);
        }
        _ => {}
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_HOLD_ENTITY_PROCESS_PED];
