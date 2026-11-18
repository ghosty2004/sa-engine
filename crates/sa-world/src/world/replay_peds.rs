//! Replay peds: creating the peds a recording needs and applying their update packets during playback.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::le_or_nan;

use super::common::{g, rd, wr, x, xi};
use super::replay_setup::{pool_slot, scan_replay_buffers_from, PED_POOL, PED_POOL_CONVERSION, VEHICLE_POOL, VEHICLE_POOL_CONVERSION};
use crate::bindings::{
    add_weapon_model, anim_blend_animation, civilian_ped_ctor, find_player_ped, matrix_copy_ctor, matrix_dtor,
    ped_remove_weapon_model, player_re_apply_move_anims, set_char_created_by,
};
use sa_anim::bindings::assoc_set_current_time;
use sa_core::bindings::{
    clothes_rebuild_player, decompress_into_full_matrix, entity_clean_up_old_reference, entity_register_reference,
    load_all_requested_models, matrix_assign, matrix_set_unity, player_ped_ctor, streaming_request_model, world_add,
    world_remove,
};
use sa_core::pools::call::ped_new_at;

orig! {
    /// `CMatrix::ScaleAll(float)` (0x45AF40)
    fn matrix_scale_all(m: GPtr<u8>, s: f32) = thiscall 0x45AF40 => sa_core::cmatrix::scale_all;
    /// `CMatrix::operator+=(const CMatrix&)` (0x59ADF0)
    fn matrix_add_assign(m: GPtr<u8>, rhs: GPtr<u8>) -> GPtr<u8> = thiscall 0x59ADF0 => sa_core::matrix_more::add_assign;
    /// `CAnimBlendAssociation::SetBlend(float amount, float delta)` (0x4CEBA0)
    fn assoc_set_blend(a: GPtr<u8>, amount: u32, delta: u32) = thiscall 0x4CEBA0 => sa_anim::assoc::set_blend;
    /// `RpAnimBlendClumpRemoveAssociations(RpClump*, uint32 mask)` (0x4D6820)
    fn clump_remove_associations(clump: GPtr<u8>, mask: u32) = cdecl 0x4D6820 => sa_anim::rp_anim_blend::rp_anim_blend_clump_remove_associations;
}

/// `CAnimManager::ms_aAnimAssocGroups` (stride 20: +0 the anim block, +8 the anim count; a block's +0x10 = loaded).
const ANIM_ASSOC_GROUPS: usize = 0xB4_EA34;
/// `CStreaming::ms_aInfoForModel[].m_nLoadState` (stride 20; 1 = loaded).
const MODEL_LOAD_STATE: usize = 0x8E_4CD0;
const ONE: u32 = 0x3F80_0000;
const HUNDRED: f32 = 100.0;
/// `ANIM_TIME_COMPRESS`, `ANIM_SPEED_COMPRESS`, `ANIM_BLEND_COMPRESS` reciprocals (1 / 63.75, 1 / 85, 1 / 127.5).
const TIME_SCALE: usize = 0x85_9C2C;
const SPEED_SCALE: usize = 0x85_9C28;
const BLEND_SCALE: usize = 0x85_9C24;

/// The playback's matrix blend: two stack `CMatrix` (the packet's decompressed one and a copy of the entity's), the
/// copy scaled by `1 - t` and the packet's by `t`, summed into the copy and assigned to the entity's matrix (+0x14).
/// `rest` runs before both are destroyed (the replay ped / vehicle updates run their whole tail in between).
pub fn blend_replay_matrix(entity: GPtr<u8>, compressed: GPtr<u8>, t: f32, rest: impl FnOnce()) {
    let mut ma = [0u32; 0x48 / 4];
    let mut mb = [0u32; 0x48 / 4];
    let a = GPtr::from_raw(ma.as_mut_ptr().cast::<u8>());
    let b = GPtr::from_raw(mb.as_mut_ptr().cast::<u8>());
    wr::<u32>(a, 0x40, 0);
    wr::<u32>(a, 0x44, 0);
    decompress_into_full_matrix(compressed, a);
    matrix_copy_ctor(b, rd::<GPtr<u8>>(entity, 0x14));
    matrix_scale_all(b, x(g::<f32>(0x85_8624).read()).fsub_m(t).to_f32());
    matrix_scale_all(a, t);
    matrix_add_assign(b, a);
    matrix_assign(rd::<GPtr<u8>>(entity, 0x14), b);
    rest();
    matrix_dtor(b);
    matrix_dtor(a);
}

/// `byte * [scale]` as `fild m32; fmul m32`.
fn scaled(b: u8, scale: usize) -> sa_math::x87::X87 {
    xi(i32::from(b)).fmul_m(g::<f32>(scale).read())
}

/// The anim group's block is set and loaded.
fn group_loaded(group: u8) -> bool {
    let block = rd::<GPtr<u8>>(g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read(), isize::from(group) * 20);
    !block.is_null() && rd::<u8>(block, 0x10) != 0
}

/// `CReplay::RestoreClothesDesc(CPedClothesDesc& desc, tReplayClothesBlock& packet)` (0x45C7D0, cdecl): the ten model
/// keys and eighteen texture keys (packet +4 .. +0x74) copied to desc +0 .. +0x70, the fat and muscle stats (int16
/// +0x74 / +0x76) as floats to +0x70 / +0x74.
#[hook(addr = 0x45C7D0, conv = cdecl, name = "CReplay::RestoreClothesDesc", expect = "8B 44 24 08 8B 50 04",
       unverified = "frontier2-b31", route = "L2", hazards = "none")]
pub fn restore_clothes_desc(desc: GPtr<u8>, packet: GPtr<u8>) {
    for k in 0..28isize {
        wr::<u32>(desc, 4 * k, rd::<u32>(packet, 4 + 4 * k));
    }
    wr::<f32>(desc, 0x70, xi(i32::from(rd::<i16>(packet, 0x74))).to_f32());
    wr::<f32>(desc, 0x74, xi(i32::from(rd::<i16>(packet, 0x76))).to_f32());
}

/// `CReplay::RetrievePedAnimation(CPed* ped, CStoredAnimationState& state)` (0x45B4D0, cdecl). The state holds three
/// entries of (u16 anim, time, speed, blend / group bytes) at +0, +6 and +0xC.
///
/// The first anim is blended at 100 from the ped's anim group (+0x4D4) for ids up to 3, else from group +4 when that
/// group's block is loaded (otherwise group 0 anim 0); time, speed, `SetBlend(1, 1)` and no callback. The second
/// (+6, blend byte +0xA, both non-zero): ids up to 3 from the ped's group, else from group +0xB only when it has
/// anims; time, speed, `SetBlend(blend, 1)`, no callback. Then the partial anims (flag 0x10) are removed and the third
/// (+0xC) is blended at 1000 from group +0x11 when its blend is above 0, its id is not 3 and the group is loaded:
/// time, speed, blend amount and a zero blend delta set directly.
#[hook(addr = 0x45B4D0, conv = cdecl, name = "CReplay::RetrievePedAnimation", expect = "51 55 56 57 8B 7C 24 18",
       unverified = "frontier2-b31", route = "L2", hazards = "heap-writes")]
pub fn retrieve_ped_animation(ped: GPtr<u8>, state: GPtr<u8>) {
    let clump = || rd::<GPtr<u8>>(ped, 0x18);
    let ped_group = || rd::<u32>(ped, 0x4D4);
    let anim = rd::<u16>(state, 0);
    let first = if anim <= 3 {
        anim_blend_animation(clump(), ped_group(), u32::from(anim), HUNDRED)
    } else if group_loaded(rd::<u8>(state, 4)) {
        anim_blend_animation(clump(), u32::from(rd::<u8>(state, 4)), u32::from(anim), HUNDRED)
    } else {
        anim_blend_animation(clump(), 0, 0, HUNDRED)
    };
    assoc_set_current_time(first, scaled(rd::<u8>(state, 2), TIME_SCALE).to_f32());
    wr::<f32>(first, 0x24, scaled(rd::<u8>(state, 3), SPEED_SCALE).to_f32());
    assoc_set_blend(first, ONE, ONE);
    wr::<u32>(first, 0x30, 0);

    let blend = rd::<u8>(state, 0xA);
    let anim = rd::<u16>(state, 6);
    if blend != 0 && anim != 0 {
        let time = scaled(rd::<u8>(state, 8), TIME_SCALE).to_f32();
        let speed = scaled(rd::<u8>(state, 9), SPEED_SCALE).to_f32();
        let amount = scaled(blend, BLEND_SCALE).to_f32();
        let group = rd::<u8>(state, 0xB);
        let second = if anim <= 3 {
            anim_blend_animation(clump(), ped_group(), u32::from(anim), HUNDRED)
        } else if rd::<i32>(g::<GPtr<u8>>(ANIM_ASSOC_GROUPS).read(), isize::from(group) * 20 + 8) > 0 {
            anim_blend_animation(clump(), u32::from(group), u32::from(anim), HUNDRED)
        } else {
            GPtr::NULL
        };
        if !second.is_null() {
            assoc_set_current_time(second, time);
            wr::<f32>(second, 0x24, speed);
            assoc_set_blend(second, amount.to_bits(), ONE);
            wr::<u32>(second, 0x30, 0);
        }
    }

    clump_remove_associations(clump(), 0x10);
    let anim = rd::<u16>(state, 0xC);
    if anim == 0 {
        return;
    }
    let time = scaled(rd::<u8>(state, 0xE), TIME_SCALE).to_f32();
    let speed = scaled(rd::<u8>(state, 0xF), SPEED_SCALE).to_f32();
    let amount = scaled(rd::<u8>(state, 0x10), BLEND_SCALE);
    let group = rd::<u8>(state, 0x11);
    if le_or_nan(amount, x(0.0)) || anim == 3 || !group_loaded(group) {
        return;
    }
    let third = anim_blend_animation(clump(), u32::from(group), u32::from(anim), 1000.0);
    assoc_set_current_time(third, time);
    wr::<f32>(third, 0x24, speed);
    wr::<f32>(third, 0x18, amount.to_f32());
    wr::<u32>(third, 0x1C, 0);
}

/// `CReplay::ProcessPedUpdate(CPed* ped, float interpolation, CAddressInReplayBuffer& buffer)` (0x45CA70, cdecl; the
/// entry is `nop; jmp 0x403CF3`, a fragment that only loads `fs:[0]` for the SEH frame). Applies the ped update packet
/// at `buffer` (offset +0, base +4) to `ped` when there is one, then advances the offset by the packet's 0x34 bytes.
///
/// Heading byte +2 times 0.0245 to both the current and aiming rotation (+0x558 / +0x55C); the matrix blended with the
/// packet's (+0x18) by [`blend_replay_matrix`]; vehicle byte +3 non-zero: the vehicle reference (+0x58C) moves to
/// conversion entry `byte - 1` and, when that slot holds a vehicle, the in-vehicle flag (+0x46C bit 8) is set; zero:
/// the flag is cleared and the reference dropped. A new anim group (int16 +0x2E) is stored and, for the player,
/// `ReApplyMoveAnims`. Flags byte +0x31 bits 0..2 go to the entity flags (+0x1C) bits 7, 30 and 31, health byte +0x30
/// times 0.01 to +0x12C, the anims from +4, the weapon model replaced by int16 +0x2C (requested when not loaded),
/// the area from `CGame::currArea`, and the ped removed from and added to the world.
#[hook(addr = 0x45CA70, conv = cdecl, name = "CReplay::ProcessPedUpdate", expect = "90 E9 7D 72 FA FF",
       unverified = "frontier2-b31", route = "L2", hazards = "heap-writes")]
pub fn process_ped_update(ped: GPtr<u8>, interpolation: f32, buffer: GPtr<u8>) {
    let packet = rd::<GPtr<u8>>(buffer, 4).byte_add(rd::<u32>(buffer, 0) as isize);
    if !ped.is_null() {
        let heading = xi(i32::from(rd::<i8>(packet, 2))).fmul_m(g::<f32>(0x85_9C44).read()).to_f32();
        wr::<f32>(ped, 0x558, heading);
        wr::<f32>(ped, 0x55C, heading);
        blend_replay_matrix(ped, packet.byte_add(0x18), interpolation, || apply_ped_update(ped, packet));
    }
    wr::<u32>(buffer, 0, rd::<u32>(buffer, 0).wrapping_add(0x34));
}

/// The part of [`process_ped_update`] between the matrix blend and the stack matrices' destruction.
fn apply_ped_update(ped: GPtr<u8>, packet: GPtr<u8>) {
    let slot = ped.byte_add(0x58C);
    let vehicle = rd::<u8>(packet, 3);
    if vehicle != 0 {
        let old = rd::<GPtr<u8>>(slot, 0);
        if !old.is_null() {
            entity_clean_up_old_reference(old, slot);
        }
        let index = g::<i32>(VEHICLE_POOL_CONVERSION + 4 * (usize::from(vehicle) - 1)).read();
        let veh = pool_slot(VEHICLE_POOL, 0xA18, index);
        wr::<GPtr<u8>>(slot, 0, veh);
        if !veh.is_null() {
            entity_register_reference(veh, slot);
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x100);
        }
    } else {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !0x100);
        let old = rd::<GPtr<u8>>(slot, 0);
        if !old.is_null() {
            entity_clean_up_old_reference(old, slot);
        }
        wr::<u32>(slot, 0, 0);
    }
    let group = i32::from(rd::<i16>(packet, 0x2E));
    if group != rd::<i32>(ped, 0x4D4) {
        wr::<i32>(ped, 0x4D4, group);
        if ped == find_player_ped(-1) {
            player_re_apply_move_anims(ped);
        }
    }
    let bits = u32::from(rd::<u8>(packet, 0x31));
    let mut f = rd::<u32>(ped, 0x1C);
    f ^= ((bits << 7) ^ f) & 0x80;
    wr::<u32>(ped, 0x1C, f);
    f ^= ((bits << 29) ^ f) & 0x4000_0000;
    wr::<u32>(ped, 0x1C, f);
    wr::<u32>(ped, 0x1C, (((bits << 29) ^ f) & 0x7FFF_FFFF) ^ (bits << 29));
    wr::<f32>(ped, 0x12C, xi(i32::from(rd::<u8>(packet, 0x30))).fmul_m(g::<f32>(0x85_8C58).read()).to_f32());
    retrieve_ped_animation(ped, packet.byte_add(4));
    ped_remove_weapon_model(ped, -1);
    let weapon = rd::<i16>(packet, 0x2C);
    if weapon != -1 {
        let model = i32::from(weapon);
        if model_loaded(model) {
            add_weapon_model(ped, model);
        } else {
            streaming_request_model(model, 0);
        }
    }
    wr::<u8>(ped, 0x2F, g::<u8>(0xB7_2914).read());
    world_remove(ped);
    world_add(ped);
}

fn model_loaded(model: i32) -> bool {
    g::<u8>((MODEL_LOAD_STATE as isize + model as isize * 20) as usize).read() == 1
}

/// The ped pool slot a recording's ped byte maps to (`m_PedPoolConversion`).
fn ped_conversion(pool_byte: u8) -> i32 {
    g::<i32>(PED_POOL_CONVERSION + 4 * usize::from(pool_byte)).read()
}

/// `CReplay::DealWithNewPedPacket(tReplayPedHeaderBlock& packet, bool loadModel, tReplayClothesBlock* clothes)` (stub
/// 0x45CEA0 → 0x156ED70, cdecl). Null when the packet's pool slot (conversion of byte +1) already holds a ped. For the
/// player (ped type byte +4 == 0) with `loadModel` the model (int16 +2) is requested (flags 0: the protection decodes
/// the word from 0x15605B4, zero in the image) and loaded. A model not loaded is requested and null returned. The
/// player is `new (slot << 8) CPlayerPed(0, true)` with its clothes restored from `clothes` and rebuilt; another type
/// only without `loadModel`, as `new (slot << 8) CCivilianPed(type, model)` (the protection's VM gives its size
/// 0x79C and the computed call `CPed::operator new`). The ped is marked as playback (status 1, `bUsedForReplay`), its
/// matrix set to unity, created by 3 (mission) and removed from and added to the world.
#[hook(addr = 0x45CEA0, conv = cdecl, name = "CReplay::DealWithNewPedPacket", expect = "E9 CB 1E 11 01",
       unverified = "frontier2-b31", route = "L2", hazards = "heap-writes")]
pub fn deal_with_new_ped_packet(packet: GPtr<u8>, load_model: u8, clothes: GPtr<u8>) -> GPtr<u8> {
    let index = ped_conversion(rd::<u8>(packet, 1));
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    if rd::<i8>(rd::<GPtr<u8>>(pool, 4), index as isize) >= 0
        && rd::<u32>(pool, 0).wrapping_add((index as u32).wrapping_mul(0x7C4)) != 0
    {
        return GPtr::NULL;
    }
    let ty = rd::<u8>(packet, 4);
    let model = i32::from(rd::<i16>(packet, 2));
    if ty == 0 && load_model != 0 {
        streaming_request_model(model, 0);
        load_all_requested_models(0);
    }
    if !model_loaded(model) {
        streaming_request_model(model, 0);
        return GPtr::NULL;
    }
    let ped = if ty == 0 {
        let mem = ped_new_at(0x7A4, ped_conversion(rd::<u8>(packet, 1)) << 8);
        let ped = if mem.is_null() { GPtr::NULL } else { player_ped_ctor(mem, 0, 1) };
        restore_clothes_desc(rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x480), 4), clothes);
        clothes_rebuild_player(ped, 1);
        ped
    } else {
        if load_model != 0 {
            return GPtr::NULL;
        }
        let mem = ped_new_at(0x79C, ped_conversion(rd::<u8>(packet, 1)) << 8);
        if mem.is_null() { GPtr::NULL } else { civilian_ped_ctor(mem, u32::from(ty), model as u32) }
    };
    wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x20_0000);
    wr::<u8>(ped, 0x36, (rd::<u8>(ped, 0x36) & 7) | 8);
    matrix_set_unity(rd::<GPtr<u8>>(ped, 0x14));
    set_char_created_by(ped, 3);
    world_remove(ped);
    world_add(ped);
    ped
}

/// `CReplay::CreatePlayerPed()` (0x45D540, cdecl): the buffers walked from the playback slot (0x97FB6C) on; the first
/// ped header packet (type 3) goes through [`deal_with_new_ped_packet`] (loading its model, the clothes packet right
/// after its 8 bytes); once that gave a ped, the first ped update (type 4) whose conversion slot holds that ped is
/// applied with [`process_ped_update`] (interpolation 1) and the ped returned. Null when no such update follows,
/// even after a ped was created.
#[hook(addr = 0x45D540, conv = cdecl, name = "CReplay::CreatePlayerPed", expect = "83 EC 10 8B 15 90 44 B7 00",
       unverified = "frontier2-b31", route = "L2", hazards = "heap-writes")]
pub fn create_player_ped() -> GPtr<u8> {
    let mut ped = GPtr::NULL;
    let mut done = false;
    scan_replay_buffers_from(u32::from(g::<u8>(0x97_FB6C).read()), |buf, off, ty| {
        let packet = buf.byte_add(off as isize);
        if ty == 3 && ped.is_null() {
            ped = deal_with_new_ped_packet(packet, 1, packet.byte_add(8));
        } else if ty == 4
            && !ped.is_null()
            && pool_slot(PED_POOL, 0x7C4, ped_conversion(rd::<u8>(packet, 1))) == ped
        {
            let mut address = [off, buf.addr() as u32, 0];
            process_ped_update(ped, 1.0, GPtr::from_raw(address.as_mut_ptr().cast::<u8>()));
            done = true;
        }
        done
    });
    if done { ped } else { GPtr::NULL }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_RESTORE_CLOTHES_DESC,
    HOOK_RETRIEVE_PED_ANIMATION,
    HOOK_PROCESS_PED_UPDATE,
    HOOK_DEAL_WITH_NEW_PED_PACKET,
    HOOK_CREATE_PLAYER_PED,
];
