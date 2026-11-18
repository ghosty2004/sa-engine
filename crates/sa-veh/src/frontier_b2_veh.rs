//! FRONTIER-2 turn 2, lane B (batch `frontier2-b6`): vehicle functions on the phase B frontier: upgrades, the
//! boost jump. Ported from the 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use crate::common::{g, model_info, rd, wr, x};
use crate::mvpd_v6::horn_just_down;
use sa_core::field::addr32;
use crate::common::rand_at;
use sa_core::bindings::{
    entity_clean_up_old_reference, entity_register_reference, find_player_ped, general_random_int_in_range, task_new,
};
use sa_world::bindings::{add_ped_in_car, leave_car_and_wander_ctor, tm_set_task};
use crate::automobile_control::{update_tractor_link, update_trailer_link};
use sa_core::bindings::matrix_set_rotate_z_only;
use sa_world::bindings::{
    audio_add_event, automobile_place_on_road_properly, physical_add_to_moving_list, placeable_set_posn,
    remove_from_moving_list, switch_vehicle_to_real_physics, vector_sub_assign,
};
use sa_world::world::common::{entity_pos, placeable_heading};
use sa_core::bindings::{
    frame_for_all_children, frame_for_all_objects, quat_get, rp_clump_add_atomic, rp_clump_for_all_atomics,
    rp_atomic_set_frame, rp_geometry_for_all_materials, rw_frame_add_child, rw_frame_destroy, rw_matrix_update,
};
use sa_stream::modelinfo::clump::set_frame_hierarchy_id;
use sa_stream::bindings::{get_frame_from_id, model_info_add_ref, ped_is_player, set_atomic_render_callback};
use sa_stream::modelinfo::vehicle::call::set_atomic_flag;
use sa_world::bindings::{get_pad_from_player, physical_apply_move_force_5429f0, physical_apply_turn_force, vector_scale};

orig! {
    /// `CVehicle::GetTowHitchPos(CVector& out, bool always, CVehicle*)` (vtable +0xEC, AL)
    fn v_get_tow_hitch_pos(v: GPtr<u8>, out: GPtr<u8>, always: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xEC => unported;
    /// `CVehicle::GetTowBarPos(CVector& out, bool always, CVehicle*)` (vtable +0xF0, AL)
    fn v_get_tow_bar_pos(v: GPtr<u8>, out: GPtr<u8>, always: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xF0 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CBaseModelInfo::CreateInstance()` (vtable +0x2C)
    fn v_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `CVisibilityPlugins::SetAtomicId(RpAtomic*, int32)` (0x732290)
    fn set_atomic_id(a: GPtr<u8>, id: u32) = cdecl 0x732290 => unported;
    /// `FUN_004c88f0(RpAtomic*, int32)` (0x4C88F0, returns its first argument)
    fn atomic_passthrough_4c88f0(a: GPtr<u8>, b: u32) -> GPtr<u8> = cdecl 0x4C88F0 => unported;
    /// `CCustomCarPlateMgr::SetupClumpAfterVehicleUpgrade(RpClump*, RpMaterial*, uint8)` (0x6FDFE0)
    fn setup_clump_after_vehicle_upgrade(c: GPtr<u8>, plate: u32, ty: u32) -> GPtr<u8> = cdecl 0x6FDFE0 => unported;
    /// `CBaseModelInfo::AsDamageAtomicModelInfoPtr()` (vtable +0x8)
    fn v_as_damage_atomic_model_info(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CVehicle::CreateReplacementAtomic(CBaseModelInfo*, RwFrame*, int32 id, bool damaged, bool wheel)` (0x6D3700)
    fn create_replacement_atomic_call(v: GPtr<u8>, mi: GPtr<u8>, frame: GPtr<u8>, id: u32, damaged: u32, wheel: u32) -> GPtr<u8> = thiscall 0x6D3700 => vehicle_create_replacement_atomic;
}

/// The tail both upgrade atomic constructors share: the render callback by `HasAlphaMaterialCB` (0x4C77C0 over the
/// geometry +0x18, the flag kept in the low byte of the caller's model-info argument slot: 0x733F80 and flag 0x40
/// with alpha, else 0x733240), `FUN_004c88f0(atomic, 0)`.
fn upgrade_atomic_rendering(atomic: GPtr<u8>, mi: GPtr<u8>, flag_alpha: bool) {
    let mut slot = [addr32(mi) & !0xFF];
    let sp = GPtr::from_raw(slot.as_mut_ptr()).cast::<u8>();
    rp_geometry_for_all_materials(rd(atomic, 0x18), 0x4C_77C0, sp);
    if rd::<u8>(sp, 0) != 0 {
        set_atomic_render_callback(atomic, 0x73_3F80);
        if flag_alpha {
            set_atomic_flag(atomic, 0x40);
        }
    } else {
        set_atomic_render_callback(atomic, 0x73_3240);
    }
    atomic_passthrough_4c88f0(atomic, 0);
}

/// `CVehicle::CreateReplacementAtomic(CBaseModelInfo* mi, RwFrame* frame, int32 id, bool damaged, bool wheel)` (0x6D3700,
/// thiscall, `ret 0x14`): with `damaged` the flag 0xA9B0B0 is set for the instance; `CreateInstance` (vtable +0x2C),
/// the model referenced, the atomic added to the clump (+0x18). With `wheel` its own frame gets an identity modelling
/// matrix (flags |= 0x20003, the pad dwords untouched, then `RwMatrixUpdate`), hierarchy id 0 and becomes a child of
/// `frame`; otherwise the atomic moves to `frame` and its own frame is destroyed. Atomic id `id & ~3`, flag 2
/// (damaged) or 1, the render callback by alpha (no 0x40 flag), `FUN_004c88f0(atomic, 0)`; 0xA9B0B0 cleared. Returns
/// the atomic.
#[hook(addr = 0x6D3700, conv = thiscall, name = "CVehicle::CreateReplacementAtomic", expect = "8A 44 24 10 53 55",
       this = "CVehicle", unverified = "frontier2-b28", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the alpha flag lives in the low byte of the caller's model-info argument slot")]
pub fn vehicle_create_replacement_atomic(this: GPtr<u8>, mi: GPtr<u8>, frame: GPtr<u8>, id: u32, damaged: u32, wheel: u32) -> GPtr<u8> {
    let damaged = damaged as u8 != 0;
    if damaged {
        g::<u8>(0xA9_B0B0).write(1);
    }
    let atomic = v_create_instance(mi);
    model_info_add_ref(mi.cast());
    let own = rd::<GPtr<u8>>(atomic, 4);
    rp_clump_add_atomic(rd(this, 0x18), atomic);
    if wheel as u8 != 0 {
        let m = own.byte_add(0x10);
        for (o, v) in [(0x28, 0x3F80_0000u32), (0x14, 0x3F80_0000), (0, 0x3F80_0000)] {
            wr::<u32>(m, o, v);
        }
        let flags = rd::<u32>(m, 0xC) | 0x20003;
        for o in [0x10, 8, 4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
            wr::<u32>(m, o, 0);
        }
        wr::<u32>(m, 0xC, flags);
        rw_matrix_update(m);
        set_frame_hierarchy_id(own, 0);
        rw_frame_add_child(frame, own);
    } else {
        rp_atomic_set_frame(atomic, frame);
        rw_frame_destroy(own);
    }
    set_atomic_id(atomic, id & !3);
    set_atomic_flag(atomic, if damaged { 2 } else { 1 });
    upgrade_atomic_rendering(atomic, mi, false);
    g::<u8>(0xA9_B0B0).write(0);
    atomic
}

/// `CVehicle::AddReplacementUpgrade(int32 modelIndex, int32 nodeId)` (0x6D3830, thiscall, `ret 8`; the fragment
/// 0x408121 pushes `RemoveObjectsCB` 0x6D33B0 = `[0x15608C0] + [0x41FA94] + 1`).
///
/// The node's frame (`GetFrameFromId`) loses its objects and children (`RemoveObjectsCB` 0x6D33B0 / 0x6D3420, which
/// leave the removed atomic's id in the local); the model's instance (`CreateInstance`) is referenced, added to the
/// clump and moved to that frame (its own frame destroyed); atomic id `removed & ~3`, flag 1, the render callback by
/// alpha (no 0x40 flag), `FUN_004c88f0`; 0xA9B0B0 cleared. Node 19 with handling flag 0x2000 (+0x384 +0xCC) also gets
/// the mirrored wheel (`CreateReplacementAtomic(mi, frame, removed, false, true)`, its x position = the frame's * -2.0,
/// `RwMatrixUpdate`); nodes 12 / 13 `SetupClumpAfterVehicleUpgrade`. A damage atomic model (vtable +8) adds the
/// damaged version (`CreateReplacementAtomic(.., true, false)`) and, with a frame, hides it
/// (`SetVehicleAtomicVisibilityCB` 0x6D2690 / 0x6D26D0, 1).
#[hook(addr = 0x6D3830, conv = thiscall, name = "CVehicle::AddReplacementUpgrade", expect = "83 EC 0C 8B 44 24 14",
       this = "CVehicle", unverified = "frontier2-b28", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the alpha flag lives in the low byte of the caller's modelIndex argument slot")]
pub fn vehicle_add_replacement_upgrade(this: GPtr<u8>, model: i32, node: u32) {
    let frame = get_frame_from_id(rd(this, 0x18), node as i32);
    let mut removed = 0u32;
    let rp = addr32(GPtr::from_raw(&mut removed as *mut u32).cast::<u8>());
    frame_for_all_objects(frame, 0x6D_33B0, rp);
    frame_for_all_children(frame, 0x6D_3420, rp);
    let mi = model_info(model);
    let atomic = v_create_instance(mi);
    model_info_add_ref(mi.cast());
    let own = rd::<GPtr<u8>>(atomic, 4);
    rp_clump_add_atomic(rd(this, 0x18), atomic);
    rp_atomic_set_frame(atomic, frame);
    rw_frame_destroy(own);
    set_atomic_id(atomic, removed & !3);
    set_atomic_flag(atomic, 1);
    upgrade_atomic_rendering(atomic, GPtr::from_addr(model as u32 as usize), false);
    g::<u8>(0xA9_B0B0).write(0);
    if node == 0x13 {
        if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x2000 != 0 {
            let a = create_replacement_atomic_call(this, mi, frame, removed, 0, 1);
            let m = rd::<GPtr<u8>>(a, 4).byte_add(0x10);
            wr::<f32>(m, 0x30, x(rd::<f32>(frame, 0x40)).fmul_m(f32::from_bits(0xC000_0000)).to_f32());
            rw_matrix_update(m);
        }
    } else if node == 0xC || node == 0xD {
        let vmi = model_info(i32::from(rd::<i16>(this, 0x22)));
        setup_clump_after_vehicle_upgrade(rd(this, 0x18), rd(vmi, 0x24), u32::from(rd::<u8>(vmi, 0x31)));
    }
    if !v_as_damage_atomic_model_info(mi).is_null() {
        create_replacement_atomic_call(this, mi, frame, removed, 1, 0);
        if !frame.is_null() {
            frame_for_all_objects(frame, 0x6D_2690, 1);
            frame_for_all_children(frame, 0x6D_26D0, 1);
        }
    }
}

/// `CVehicle::CreateUpgradeAtomic(CBaseModelInfo*, const UpgradePosnDesc*, RwFrame* parent, bool damaged)` (0x6D3510,
/// thiscall, `ret 0x10`): with `damaged` the flag 0xA9B0B0 is set for the instance; the model's instance
/// (`CreateInstance`, vtable +0x2C); its frame (+4) gets the descriptor's rotation (`CQuaternion::Get` of +0xC into
/// the modelling matrix +0x10) and position (+0 into +0x40), `RwMatrixUpdate`; the atomic added to the vehicle's clump
/// (+0x18), its frame to `parent`; the model referenced; atomic id 2 (damaged) or 1, flags 0x800 and 0x400; when a
/// material has alpha (`HasAlphaMaterialCB` 0x4C77C0 over the geometry +0x18, the flag kept in the low byte of the
/// caller's model-info argument slot) render callback 0x733F80 and flag 0x40, else 0x733240; `FUN_004c88f0(atomic,
/// 0)`; 0xA9B0B0 cleared. Returns the atomic.
#[hook(addr = 0x6D3510, conv = thiscall, name = "CVehicle::CreateUpgradeAtomic", expect = "83 EC 08 8A 44 24 18",
       this = "CVehicle", unverified = "frontier2-b6", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the alpha flag lives in the low byte of the caller's model-info argument slot")]
pub fn vehicle_create_upgrade_atomic(this: GPtr<u8>, mi: GPtr<u8>, desc: GPtr<u8>, parent: GPtr<u8>, damaged: u32) -> GPtr<u8> {
    if damaged as u8 != 0 {
        g::<u8>(0xA9_B0B0).write(1);
    }
    let atomic = v_create_instance(mi);
    let frame = rd::<GPtr<u8>>(atomic, 4);
    let m = frame.byte_add(0x10);
    quat_get(desc.byte_add(0xC), m);
    for o in [0isize, 4, 8] {
        wr::<u32>(m, 0x30 + o, rd(desc, o));
    }
    rw_matrix_update(m);
    rp_clump_add_atomic(rd(this, 0x18), atomic);
    rw_frame_add_child(parent, frame);
    model_info_add_ref(mi.cast());
    set_atomic_id(atomic, if damaged as u8 != 0 { 2 } else { 1 });
    set_atomic_flag(atomic, 0x800);
    set_atomic_flag(atomic, 0x400);
    upgrade_atomic_rendering(atomic, mi, true);
    g::<u8>(0xA9_B0B0).write(0);
    atomic
}

/// `CVehicle::AddUpgrade(int32 modelIndex, int32 upgradeIndex)` (0x6DFA20, thiscall, `ret 8`): the upgrade's
/// position descriptor (vehicle structure +0x5C of the model info, +0xB4 + 0x20 * index) and its parent frame
/// (`GetFrameFromId(clump, desc +0x1C)`); the upgrade's atomics removed (`RpClumpForAllAtomics(RemoveUpgradeCB,
/// index)`); `CreateUpgradeAtomic(.., false)`; unless the descriptor's parent id is 1: the damaged descriptor (indices
/// 0, 1, 2, 6, 12 use the structure's +0x114 / +0x134 / +0x154 / +0x194 / +0x254 when its id (+0x1C) is set, else the
/// same) with `CreateUpgradeAtomic(.., true)`; with a parent frame its objects and children hidden
/// (`SetVehicleAtomicVisibilityCB` 0x6D2690 / 0x6D26D0, 1); `SetupClumpAfterVehicleUpgrade(clump, +0x24, +0x31)` of the
/// vehicle's model info. The parent frame is kept in the caller's `modelIndex` argument slot (not modelled).
#[hook(addr = 0x6DFA20, conv = thiscall, name = "CVehicle::AddUpgrade", expect = "51 53 8B 5C 24 10",
       this = "CVehicle", unverified = "frontier2-b6", route = "L2", hazards = "heap-writes",
       note = "the parent frame is kept in the caller's modelIndex argument slot")]
pub fn vehicle_add_upgrade(this: GPtr<u8>, model: i32, index: i32) {
    let vmi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let umi = model_info(model);
    let mut desc = rd::<GPtr<u8>>(vmi, 0x5C).byte_add(0xB4 + 0x20 * index as isize);
    let frame = get_frame_from_id(rd(this, 0x18), rd(desc, 0x1C));
    rp_clump_for_all_atomics(rd(this, 0x18), 0x6D_3300, index as u32);
    vehicle_create_upgrade_atomic(this, umi, desc, frame, 0);
    if rd::<i32>(desc, 0x1C) == 1 {
        return;
    }
    let alt = match index {
        0 => Some(0x114),
        1 => Some(0x134),
        2 => Some(0x154),
        6 => Some(0x194),
        0xC => Some(0x254),
        _ => None,
    };
    if let Some(o) = alt {
        let s = rd::<GPtr<u8>>(vmi, 0x5C);
        if rd::<i32>(s, o + 0x1C) != -1 {
            desc = s.byte_add(o);
        }
    }
    vehicle_create_upgrade_atomic(this, umi, desc, frame, 1);
    if !frame.is_null() {
        frame_for_all_objects(frame, 0x6D_2690, 1);
        frame_for_all_children(frame, 0x6D_26D0, 1);
    }
    let vmi = model_info(i32::from(rd::<i16>(this, 0x22)));
    setup_clump_after_vehicle_upgrade(rd(this, 0x18), rd(vmi, 0x24), u32::from(rd::<u8>(vmi, 0x31)));
}

/// `CAutomobile::BoostJumpControl()` (0x6A3A60, thiscall): with a player driver (+0x460) whose pad
/// (`GetPadFromPlayer`) has the horn just down and the wheel ratio +0x7D4 below [0x870FD0] (compared four times, as
/// the bytes do): `ApplyMoveForce(mass (+0x8C) * [0x858FCC] * (0, 0, 1))`, then `ApplyTurnForce(up * (turn mass (+0x90)
/// * [0x858C58]), forward * 1.0)` (the vector products through `operator*(float, const CVector&)` 0x40FE90).
#[hook(addr = 0x6A3A60, conv = thiscall, name = "CAutomobile::BoostJumpControl", expect = "83 EC 24 53 56 8B F1",
       this = "CAutomobile", unverified = "frontier2-b6", route = "L2", hazards = "none")]
pub fn automobile_boost_jump_control(this: GPtr<u8>) {
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if driver.is_null() || ped_is_player(driver) == 0 {
        return;
    }
    if get_pad_from_player(rd(this, 0x460)).is_null() {
        return;
    }
    if horn_just_down(get_pad_from_player(rd(this, 0x460))) == 0 {
        return;
    }
    let below = || !x(rd::<f32>(this, 0x7D4)).fcom(x(g::<f32>(0x87_0FD0).read())).test_parity(5);
    for _ in 0..4 {
        if !below() {
            return;
        }
    }
    let mut up = [0u32, 0, 0x3F80_0000];
    let mut out = [0u32; 3];
    let k = x(rd::<f32>(this, 0x8C)).fmul_m(g::<f32>(0x85_8FCC).read()).to_f32();
    let r = vector_scale(GPtr::from_raw(out.as_mut_ptr()).cast(), k, GPtr::from_raw(up.as_mut_ptr()).cast());
    physical_apply_move_force_5429f0(this, CVector::new(rd(r, 0), rd(r, 4), rd(r, 8)));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let mut o1 = [0u32; 3];
    let p = vector_scale(GPtr::from_raw(o1.as_mut_ptr()).cast(), 1.0, m.byte_add(0x10));
    let k2 = x(rd::<f32>(this, 0x90)).fmul_m(g::<f32>(0x85_8C58).read()).to_f32();
    let mut o2 = [0u32; 3];
    let f = vector_scale(GPtr::from_raw(o2.as_mut_ptr()).cast(), k2, m.byte_add(0x20));
    physical_apply_turn_force(this, CVector::new(rd(f, 0), rd(f, 4), rd(f, 8)), CVector::new(rd(p, 0), rd(p, 4), rd(p, 8)));
}

/// `CTrain::RemoveRandomPassenger()` (0x6F6850, thiscall; the bytes add a passenger, SEH frame not reproduced):
/// nothing while 0xA43088 == 1, when the player (`FindPlayerPed(-1)`) sits in this train (+0x58C), when the player's
/// primary task (intelligence +0x47C, +0x10) is of type 0x2BD or 0x2BC (`GetTaskType`, vtable +0x10, asked twice) for
/// this train (+0xC), when the player is in the train (+0x46C bit 0x100 and +0x58C) and is (direction bit 6 of +0x5B8
/// set) or is not (clear) its driver (+0x460), or when the passenger count (+0x5CB low nibble) is 0. A referenced
/// passenger (+0x5CC) in a vehicle (bit 0x100) also stops it, else the reference is dropped. Then a ped of type
/// `GetRandomNumberInRange(0, 12)` (12 becomes 11; 25 when `rand() & 0xFFFF < 100`, site 0x6F6969) is added in the
/// car (`AddPedInCar(this, !direction, type, 0, 0, 0)`); it gets +0x478 |= 0x40000, the count nibble decremented, the
/// reference (`RegisterReference`) and a `CTaskComplexLeaveCarAndWander(this, 0, 0, 1)` as its primary task (slot 3).
#[hook(addr = 0x6F6850, conv = thiscall, name = "CTrain::RemoveRandomPassenger", expect = "6A FF 68 1B 8B 84 00",
       this = "CTrain", unverified = "frontier2-b6", route = "L2", hazards = "heap-writes", note = "rand site 0x6F6969")]
pub fn train_remove_random_passenger(this: GPtr<u8>) {
    if g::<u8>(0xA4_3088).read() == 1 {
        return;
    }
    let p = find_player_ped(-1);
    if rd::<GPtr<u8>>(p, 0x58C) == this {
        return;
    }
    let task = rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0x47C), 0x10);
    if !task.is_null() && (v_get_task_type(task) == 0x2BD || v_get_task_type(task) == 0x2BC) && rd::<GPtr<u8>>(task, 0xC) == this {
        return;
    }
    let dir = (u32::from(rd::<u8>(this, 0x5B8)) >> 6) & 1;
    let p = find_player_ped(-1);
    if rd::<u32>(p, 0x46C) & 0x100 != 0 && rd::<GPtr<u8>>(p, 0x58C) == this {
        let drives = rd::<GPtr<u8>>(this, 0x460) == p;
        if (dir == 1 && drives) || (dir == 0 && !drives) {
            return;
        }
    }
    if rd::<u8>(this, 0x5CB) & 0xF == 0 {
        return;
    }
    let slot = this.byte_add(0x5CC);
    let old = rd::<GPtr<u8>>(slot, 0);
    if !old.is_null() {
        if rd::<u32>(old, 0x46C) & 0x100 != 0 {
            return;
        }
        entity_clean_up_old_reference(old, slot);
        wr::<u32>(slot, 0, 0);
    }
    let mut ty = general_random_int_in_range(0, 0xC);
    if ty == 0xC {
        ty = 0xB;
    }
    if rand_at(0x6F_6969) & 0xFFFF < 0x64 {
        ty = 0x19;
    }
    let ped = add_ped_in_car(this, u32::from(dir == 0), ty, 0, 0, 0);
    if ped.is_null() {
        return;
    }
    wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x4_0000);
    let c = rd::<u8>(this, 0x5CB);
    wr::<u8>(this, 0x5CB, ((c.wrapping_sub(1) ^ c) & 0xF) ^ c);
    wr::<GPtr<u8>>(slot, 0, ped);
    entity_register_reference(ped, slot);
    let t = task_new(0x1C);
    let t = if t.is_null() { t } else { leave_car_and_wander_ctor(t, this, 0, 0, 1) };
    tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
}

/// The common head of the two `SetTowLink`s: status trailer (10, `& 7 | 0x50`), the mutual references (`this` +0x4C4
/// = tractor, tractor +0x4C8 = `this`, each `RegisterReference`d) and both vehicles moved to the end of the moving
/// list (removed, then added, `this` first).
fn link_trailer(this: GPtr<u8>, tractor: GPtr<u8>) {
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x50);
    wr::<GPtr<u8>>(this, 0x4C4, tractor);
    entity_register_reference(tractor, this.byte_add(0x4C4));
    wr::<GPtr<u8>>(tractor, 0x4C8, this);
    entity_register_reference(this, tractor.byte_add(0x4C8));
    remove_from_moving_list(this);
    remove_from_moving_list(rd(this, 0x4C4));
    physical_add_to_moving_list(this);
    physical_add_to_moving_list(rd(this, 0x4C4));
}

/// The heading of `tractor` (FPATAN of its forward, or its placement heading) given to `this` (`SetRotateZOnly`, or
/// the placement heading +0x10). The original keeps it in the caller's `placeOnRoad` argument slot (not modelled).
fn copy_heading(this: GPtr<u8>, tractor: GPtr<u8>) {
    let h = placeable_heading(tractor).to_f32();
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        wr::<f32>(this, 0x10, h);
    } else {
        matrix_set_rotate_z_only(m, h);
    }
}

/// `CAutomobile::SetTowLink(CVehicle* tractor, bool placeOnRoad)` (0x6B4410, virtual, thiscall, `ret 8`, AL): false
/// without a tractor, with one already (+0x4C4), or for a status other than 3, 10, 4 or 2 (2 is switched to real
/// physics first). Linked ([`link_trailer`]); without `placeOnRoad` `UpdateTrailerLink(true, false)`. Otherwise the
/// tow truck (525) / tractor (531) gets its arm (+0x86C) = 0x8D3140, the heading is copied ([`copy_heading`]), the
/// hitch (`GetTowHitchPos`, vtable +0xEC, of `this`) and the tow bar (`GetTowBarPos`, vtable +0xF0, of the tractor) are
/// asked (false when either fails); the position becomes `bar - (hitch - pos)` (`operator-=` 0x406D70) and the car is
/// placed on the road. True.
#[hook(addr = 0x6B4410, conv = thiscall, name = "CAutomobile::SetTowLink", expect = "83 EC 24 53 8B 5C 24 2C",
       this = "CAutomobile", unverified = "frontier2-b6", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "FPATAN heading kept in the caller's placeOnRoad argument slot")]
pub fn automobile_set_tow_link(this: GPtr<u8>, tractor: GPtr<u8>, place: u32) -> u8 {
    if tractor.is_null() || !rd::<GPtr<u8>>(this, 0x4C4).is_null() {
        return 0;
    }
    match rd::<u8>(this, 0x36) >> 3 {
        3 | 0xA | 4 => {}
        2 => switch_vehicle_to_real_physics(this),
        _ => return 0,
    }
    link_trailer(this, tractor);
    if place as u8 == 0 {
        update_trailer_link(this, 1, 0);
        return 1;
    }
    let mut bar = [0u32; 3];
    let mut hitch = [0u32; 3];
    let bp = GPtr::from_raw(bar.as_mut_ptr()).cast::<u8>();
    let hp = GPtr::from_raw(hitch.as_mut_ptr()).cast::<u8>();
    let model = rd::<i16>(tractor, 0x22);
    if model == 0x20D || model == 0x213 {
        wr::<u16>(rd(this, 0x4C4), 0x86C, g::<u16>(0x8D_3140).read());
    }
    copy_heading(this, tractor);
    if v_get_tow_hitch_pos(this, hp, 1, this) == 0 || v_get_tow_bar_pos(tractor, bp, 1, this) == 0 {
        return 0;
    }
    vector_sub_assign(hp, entity_pos(this));
    let mut np = [
        x(rd(bp, 0)).fsub_m(rd(hp, 0)).to_f32(),
        x(rd(bp, 4)).fsub_m(rd(hp, 4)).to_f32(),
        x(rd(bp, 8)).fsub_m(rd(hp, 8)).to_f32(),
    ];
    placeable_set_posn(this, GPtr::from_raw(np.as_mut_ptr()));
    automobile_place_on_road_properly(this);
    1
}

/// `CTrailer::SetTowLink(CVehicle* tractor, bool placeOnRoad)` (0x6CFDF0, virtual, thiscall, `ret 8`, AL): false
/// without a tractor or for a status other than 3, 10, 4. Linked ([`link_trailer`]); without `placeOnRoad` the
/// tractor's `UpdateTractorLink(true, false)`. Otherwise +0x9EC = 0 when above [0x859948], the heading copied, the
/// hitch / tow bar asked as `CAutomobile::SetTowLink` does (false when either fails), the position `bar - (hitch -
/// pos)` (inline x87), the move speed (+0x44) copied from the tractor, placed on the road. Then the tractor's audio
/// (+0x138) gets event 0x71; true.
#[hook(addr = 0x6CFDF0, conv = thiscall, name = "CTrailer::SetTowLink", expect = "83 EC 24 56 57 8B 7C 24 30",
       this = "CTrailer", unverified = "frontier2-b6", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "FPATAN heading kept in the caller's placeOnRoad argument slot")]
pub fn trailer_set_tow_link(this: GPtr<u8>, tractor: GPtr<u8>, place: u32) -> u8 {
    if tractor.is_null() || !matches!(rd::<u8>(this, 0x36) >> 3, 3 | 0xA | 4) {
        return 0;
    }
    link_trailer(this, tractor);
    if place as u8 == 0 {
        update_tractor_link(tractor, 1, 0);
    } else {
        if !x(rd::<f32>(this, 0x9EC)).fcom(x(g::<f32>(0x85_9948).read())).test_any(0x41) {
            wr::<u32>(this, 0x9EC, 0);
        }
        copy_heading(this, tractor);
        let mut hitch = [0u32; 3];
        let mut bar = [0u32; 3];
        let hp = GPtr::from_raw(hitch.as_mut_ptr()).cast::<u8>();
        let bp = GPtr::from_raw(bar.as_mut_ptr()).cast::<u8>();
        if v_get_tow_hitch_pos(this, hp, 1, this) == 0 || v_get_tow_bar_pos(tractor, bp, 1, this) == 0 {
            return 0;
        }
        let p = entity_pos(this);
        let d = [
            x(rd(hp, 0)).fsub_m(rd(p, 0)).to_f32(),
            x(rd(hp, 4)).fsub_m(rd(p, 4)).to_f32(),
            x(rd(hp, 8)).fsub_m(rd(p, 8)).to_f32(),
        ];
        let mut np = [
            x(rd(bp, 0)).fsub_m(d[0]).to_f32(),
            x(rd(bp, 4)).fsub_m(d[1]).to_f32(),
            x(rd(bp, 8)).fsub_m(d[2]).to_f32(),
        ];
        placeable_set_posn(this, GPtr::from_raw(np.as_mut_ptr()));
        for o in [0x44isize, 0x48, 0x4C] {
            wr::<u32>(this, o, rd(tractor, o));
        }
        automobile_place_on_road_properly(this);
    }
    audio_add_event(tractor.byte_add(0x138), 0x71, 0.0);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_VEHICLE_CREATE_REPLACEMENT_ATOMIC,
    HOOK_VEHICLE_ADD_REPLACEMENT_UPGRADE,
    HOOK_AUTOMOBILE_SET_TOW_LINK,
    HOOK_TRAILER_SET_TOW_LINK,
    HOOK_TRAIN_REMOVE_RANDOM_PASSENGER,
    HOOK_VEHICLE_CREATE_UPGRADE_ATOMIC,
    HOOK_VEHICLE_ADD_UPGRADE,
    HOOK_AUTOMOBILE_BOOST_JUMP_CONTROL,
];
