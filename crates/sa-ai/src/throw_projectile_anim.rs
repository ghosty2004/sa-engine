//! `CTaskSimpleThrowProjectile::StartAnim` / `ProcessPed`, executed by the original under SA:MP 0.3.7
//! (`catalog/samp-priority.csv`) (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every
//! call to another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTaskSimpleThrowProjectile::FinishAnimThrowProjectileCB`
const THROW_CB: u32 = 0x61F890;
/// The near-throw distance² (0x858628)
const K_NEAR_SQ: usize = 0x858628;

use sa_world::bindings::anim_blend_animation;
use sa_core::bindings::assoc_set_finish_callback;
use sa_core::bindings::get_weapon_info as weapon_info_get;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::vector_ne;
use crate::bindings::weapon_fire;
use sa_core::bindings::get_anim_hierarchy_from_skin_clump;
use sa_core::bindings::rp_hanim_hierarchy_get_matrix_array as hanim_hierarchy_get_matrix_array;
use sa_core::bindings::rp_hanim_id_get_index as hanim_id_get_index;
use sa_core::bindings::rw_v3d_transform_points;
orig! {
    /// `CTaskSimpleThrowProjectile::StartAnim(CPed*)` (0x6259E0, `ret 4`)
    fn o_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6259E0 => unported;
}

use sa_world::world::common::entity_pos;

/// `(dz·dz + dy·dy) + dx·dx` of `a - b` on the x87 (each difference `fld; fsub` of the f32 coordinates), compared
/// with [0x858628]: true when above (`test ah, 0x41; je`).
fn far(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let d = |o: isize| x(rd::<f32>(a, o)).fsub_m(rd::<f32>(b, o));
    let (dx, dy, dz) = (d(0), d(4), d(8));
    !dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fcom(x(g::<f32>(K_NEAR_SQ).read())).test_any(0x41)
}

/// `CTaskSimpleThrowProjectile::StartAnim(CPed*)` (0x6259E0, `ret 4`): the anim group is the current weapon's
/// (`GetWeaponInfo(type, 1)` +0x1C). Before the throw (+9 clear) anim 0xE4 at 16.0; afterwards anim 0xE6 (far
/// throw) at 1000.0 when the target entity (+0x10) or a non-zero target point (+0x14) is farther than
/// `sqrt([0x858628])`, or when the throw was not held (+0xA clear) and the weapon is not 0x27; else 0xE5. The
/// association is kept (+0xC) with `FinishAnimThrowProjectileCB`. The zero vector compared with the point is a
/// stack local in the original; a Rust local here.
#[hook(addr = 0x6259E0, conv = thiscall, name = "CTaskSimpleThrowProjectile::StartAnim", expect = "83 EC 0C 55 56",
       unverified = "ai7-12", route = "L2", hazards = "heap-writes", recs = "weapon-combat")]
pub fn throw_projectile_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let cur = |p: GPtr<u8>| rd::<u32>(p, rd::<u8>(p, 0x718) as i8 as isize * 0x1C + 0x5A0);
    let info = weapon_info_get(cur(ped) as i32, 1);
    let a = if rd::<u8>(this, 9) == 0 {
        anim_blend_animation(GPtr::from_addr(rd::<u32>(ped, 0x18) as usize), rd::<u32>(info, 0x1C), 0xE4, f32::from_bits(0x4180_0000))
    } else {
        let target = rd::<GPtr<u8>>(this, 0x10);
        let mut anim = 0;
        if !target.is_null() && far(entity_pos(target), entity_pos(ped)) {
            anim = 0xE6;
        }
        if anim == 0 {
            let mut zero = [0u32; 3];
            let z: GPtr<u8> = GPtr::from_raw(zero.as_mut_ptr()).cast();
            let point = this.byte_add(0x14);
            if vector_ne(point, z) != 0 && far(point, entity_pos(ped)) {
                anim = 0xE6;
            }
        }
        if anim == 0 {
            anim = if rd::<u8>(this, 0xA) != 0 || cur(ped) == 0x27 { 0xE5 } else { 0xE6 };
        }
        anim_blend_animation(GPtr::from_addr(rd::<u32>(ped, 0x18) as usize), rd::<u32>(info, 0x1C), anim, f32::from_bits(0x447A_0000))
    };
    wr::<GPtr<u8>>(this, 0xC, a);
    assoc_set_finish_callback(a, THROW_CB, GPtr::from_addr((this.addr() as u32) as usize));
}

/// `CTaskSimpleThrowProjectile::ProcessPed(CPed*)` (0x62AF50, `ret 4`): done (AL = 1) when finished (+8) or when the
/// current weapon's info (`GetWeaponInfo(type, 1)`) lacks flag 0x100 (+0x18). Starts the anim when none plays.
/// While the throw anim 0xE5 / 0xE6 crosses its release time this frame (info +0x48, or +0x54 for 0xE6: below the
/// current time +0x20 and not below `current - step` (+0x28)) with assoc flag bit 0 (+0x2E): a player's hold time
/// (+0x20, turned into a duration first if +0xA is clear) is capped at 0x215 and stored scaled by [0x858C28] into
/// the player data (+0x480 → +0x2C); the target entity's position (+0x10) is copied to +0x14; the weapon's fire
/// offset (info +0x24) is transformed by the hand bone (the ped's +0x4A0 → +0x14 bone id) and the weapon fires at
/// the target point (+0x14) when it is not the zero vector, else with no target. AL = 0. The two vectors are
/// stack locals in the original; Rust locals here.
#[hook(addr = 0x62AF50, conv = thiscall, name = "CTaskSimpleThrowProjectile::ProcessPed", expect = "83 EC 18 53 56",
       unverified = "ai7-12", route = "L2", hazards = "ax-al-return,heap-writes", recs = "weapon-combat")]
pub fn throw_projectile_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let slot = rd::<u8>(ped, 0x718) as i8 as isize * 0x1C + 0x5A0;
    let info = weapon_info_get(rd::<u32>(ped, slot) as i32, 1);
    if rd::<u8>(this, 8) != 0 || (rd::<u32>(info, 0x18) >> 8) & 1 == 0 {
        return 1;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() {
        o_start_anim(this, ped);
        return 0;
    }
    let id = rd::<u16>(a, 0x2C);
    if id != 0xE5 && id != 0xE6 {
        return 0;
    }
    let t = x(rd::<f32>(info, if id == 0xE6 { 0x54 } else { 0x48 }));
    let cur = rd::<f32>(a, 0x20);
    if t.fcom(x(cur)).test_parity(0x05) {
        return 0;
    }
    if x(cur).fsub_m(rd::<f32>(a, 0x28)).fcom(t).test_parity(0x41) {
        return 0;
    }
    if rd::<u8>(a, 0x2E) & 1 == 0 {
        return 0;
    }
    if ped_is_player(ped) != 0 {
        if rd::<u8>(this, 0xA) == 0 {
            wr::<u32>(this, 0x20, g::<u32>(0xB7CB84).read().wrapping_sub(rd::<u32>(this, 0x20)));
        }
        if rd::<i32>(this, 0x20) > 0x215 {
            wr::<i32>(this, 0x20, 0x215);
        }
        let v = xi(rd::<i32>(this, 0x20)).fmul_m(g::<f32>(0x858C28).read()).to_f32();
        wr::<f32>(rd::<GPtr<u8>>(ped, 0x480), 0x2C, v);
    }
    let target = rd::<GPtr<u8>>(this, 0x10);
    if !target.is_null() {
        let p = entity_pos(target);
        for k in 0..3 {
            wr::<u32>(this, 0x14 + k * 4, rd::<u32>(p, k * 4));
        }
    }
    let mut vec = [rd::<u32>(info, 0x24), rd::<u32>(info, 0x28), rd::<u32>(info, 0x2C)];
    let vp: GPtr<u8> = GPtr::from_raw(vec.as_mut_ptr()).cast();
    let hier = GPtr::<u8>::addr(get_anim_hierarchy_from_skin_clump(GPtr::from_addr(rd::<u32>(ped, 0x18) as usize))) as u32;
    let idx = hanim_id_get_index(GPtr::from_addr(hier as usize), rd::<u32>(rd::<GPtr<u8>>(ped, 0x4A0), 0x14)) as u32;
    let mats = GPtr::<u8>::addr(hanim_hierarchy_get_matrix_array(GPtr::from_addr(hier as usize))) as u32;
    rw_v3d_transform_points(vp.cast::<f32>(), vp.cast::<f32>(), 1, GPtr::from_addr(mats.wrapping_add(idx << 6) as usize));
    let mut zero = [0u32; 3];
    let z: GPtr<u8> = GPtr::from_raw(zero.as_mut_ptr()).cast();
    let point = this.byte_add(0x14);
    let has_point = vector_ne(point, z) != 0;
    let weapon = ped.byte_add(rd::<u8>(ped, 0x718) as i8 as isize * 0x1C + 0x5A0);
    let tpos = if has_point { point.addr() as u32 } else { 0 };
    weapon_fire(weapon, ped, vp, vp, GPtr::NULL, GPtr::from_addr(tpos as usize), GPtr::NULL);
    0
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_THROW_PROJECTILE_START_ANIM, HOOK_THROW_PROJECTILE_PROCESS_PED];
