//! PORT-PED-2 (`docs/arch/ped-port-2.md`): ped render helpers and weapon handling on the ped
//! (`CPed::PreRender`, `SetupLighting`, `AddWeaponModel`, `RemoveWeaponModel`, `GiveWeapon`,
//! `GiveDelayedWeapon`, `RequestDelayedWeapon`, `RemoveWeaponAnims`). RenderWare and FX calls stay calls
//! into the original (`orig!`). Ported from the 1.0 US bytes (gta-reversed `Ped.cpp` as a reading aid).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `g_fxMan` (`FxManager_c`)
const FX_MAN: usize = 0xA9AE80;
/// `CStreaming::ms_aInfoForModel` load state bytes (20 bytes per model, state at +0x10 of 0x8E4CC0)
const MODEL_LOAD_STATE: usize = 0x8E4CD0;
/// "gunflash" frame name (`.rdata`)
const GUNFLASH_NAME: u32 = 0x85A728;
/// the jetpack thrust FX name passed to `CreateFxSystem` (`.rdata`)
const PARACHUTE_FX_NAME: u32 = 0x85AA44;

orig! {
    /// `CPed::PreRenderAfterTest()` (0x5E65A0)
    fn pre_render_after_test(p: GPtr<u8>) = thiscall 0x5E65A0 => unported;
    /// `ActivateDirectional()` (0x735C80)
    fn activate_directional() = cdecl 0x735C80 => unported;
    /// `CRenderer::SetupLightingForEntity(CPhysical*)` (0x553E40)
    fn setup_lighting_for_entity(e: GPtr<u8>) -> u32 = cdecl 0x553E40 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    fn fx_destroy_system(m: GPtr<u8>, s: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, CVector*, RwMatrix*, bool)` (0x4A9BE0)
    fn fx_create_system(m: GPtr<u8>, name: u32, pos: GPtr<f32>, mat: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::SetLocalParticles(bool)` (0x4AA910)
    fn fx_set_local_particles(s: GPtr<u8>, b: u32) = thiscall 0x4AA910 => unported;
    /// `FxSystem_c::CopyParentMatrix()` (0x4AA890)
    fn fx_copy_parent_matrix(s: GPtr<u8>) = thiscall 0x4AA890 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(s: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `CVisibilityPlugins::GetAtomicModelInfo`-style lookup of a clump's model info (0x732AC0)
    fn get_clump_model_info(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x732AC0 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    fn model_info_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0)
    fn model_info_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    fn rp_skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RpClumpDestroy(RpClump*)` (0x74A310)
    fn rp_clump_destroy(c: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    /// model info vtable slot 0x2C (`CreateInstance`)
    fn model_info_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `CClumpModelInfo::GetFrameFromName(RpClump*, const char*)` (0x4C5400)
    fn get_frame_from_name(c: GPtr<u8>, name: u32) -> GPtr<u8> = cdecl 0x4C5400 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CPed::RemoveWeaponModel(int32)` (0x5E3990)
    fn remove_weapon_model_call(p: GPtr<u8>, model: i32) = thiscall 0x5E3990 => remove_weapon_model;
    /// `CPed::AddWeaponModel(int32)` (0x5E5ED0)
    fn add_weapon_model_call(p: GPtr<u8>, model: i32) = thiscall 0x5E5ED0 => add_weapon_model;
    /// `CPed::RemoveGogglesModel()` (0x5DF170)
    fn remove_goggles_model(p: GPtr<u8>) = thiscall 0x5DF170 => unported;
    /// `CWeapon::Reload(CPed*)` (0x73AEB0)
    fn weapon_reload(w: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x73AEB0 => unported;
    /// `CWeapon::Shutdown()` (0x73A380)
    fn weapon_shutdown(w: GPtr<u8>) = thiscall 0x73A380 => unported;
    /// `CWeapon::Initialise(eWeaponType, int32, CPed*)` (0x73B4A0)
    fn weapon_initialise(w: GPtr<u8>, ty: u32, ammo: u32, ped: GPtr<u8>) = thiscall 0x73B4A0 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32, bool)` (0x5E6080)
    fn give_weapon_call(p: GPtr<u8>, ty: u32, ammo: u32, b: u32) -> u32 = thiscall 0x5E6080 => give_weapon;
    /// `CPed::RequestDelayedWeapon()` (0x5E8910)
    fn request_delayed_weapon_call(p: GPtr<u8>) = thiscall 0x5E8910 => request_delayed_weapon;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    fn intel_get_task_hold(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CPed::DropEntityThatThisPedIsHolding(bool)` (0x5E0360)
    fn drop_entity_that_this_ped_is_holding(p: GPtr<u8>, b: u32) = thiscall 0x5E0360 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    fn clump_get_association(c: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn blend_animation(c: GPtr<u8>, g: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
}

fn weapon_info(ty: u32) -> GPtr<u8> {
    get_weapon_info(ty as i32, 1)
}

/// `CPed::PreRender()` (0x5E8A20): `PreRenderAfterTest` unless the ped state is 0x32 (tail jump).
#[hook(addr = 0x5E8A20, conv = thiscall, name = "CPed::PreRender", expect = "83 B9 30 05 00 00 32", this = "CPed")]
pub fn pre_render(this: GPtr<u8>) {
    if rd::<i32>(this, 0x530) != 0x32 {
        pre_render_after_test(this);
    }
}

/// `CPed::SetupLighting()` (0x553F00): `ActivateDirectional()`, then `SetupLightingForEntity(this)`'s EAX.
#[hook(addr = 0x553F00, conv = thiscall, name = "CPed::SetupLighting", expect = "56 8B F1 E8 78 1D 1E 00", this = "CPed")]
pub fn setup_lighting(this: GPtr<u8>) -> u32 {
    activate_directional();
    setup_lighting_for_entity(this)
}

/// `CPed::RemoveWeaponModel(int32 modelIndex)` (0x5E3990, `ret 4`). Player types first destroy the
/// active weapon slot's FX system (+0x18 of the slot). With a weapon clump (+0x4F4) whose model info is
/// `modelIndex`'s (or any, for -1): its model info's ref released, the skinned clump's atomics reset
/// (`RpClumpForAllAtomics(.., 0x734B90, 0)`, only when the first atomic's geometry has a skin), the clump
/// destroyed, +0x4F4/+0x4F8 cleared. Always: gun-flash words +0x504/+0x508 = 0, +0x740 = -1.
#[hook(addr = 0x5E3990, conv = thiscall, name = "CPed::RemoveWeaponModel", expect = "53 56 8B F1 8B 86 98 05 00 00", this = "CPed")]
pub fn remove_weapon_model(this: GPtr<u8>, model: i32) {
    let p = this;
    let ty = rd::<u32>(p, 0x598);
    if ty == 0 || ty == 1 {
        let w = p.byte_add(0x5A0 + isize::from(rd::<i8>(p, 0x718)) * 0x1C);
        let fx = rd::<GPtr<u8>>(w, 0x18);
        if !fx.is_null() {
            fx_destroy_system(g::<u8>(FX_MAN), fx);
            wr::<u32>(w, 0x18, 0);
        }
    }
    let clump = rd::<GPtr<u8>>(p, 0x4F4);
    if !clump.is_null() {
        let matches = model == -1 || {
            let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((model as usize).wrapping_mul(4))).read();
            get_clump_model_info(clump) == mi
        };
        if matches {
            model_info_remove_ref(get_clump_model_info(rd::<GPtr<u8>>(p, 0x4F4)));
            let atomic = get_first_atomic(rd::<GPtr<u8>>(p, 0x4F4));
            if !atomic.is_null() && !rp_skin_geometry_get_skin(rd::<GPtr<u8>>(atomic, 0x18)).is_null() {
                rp_clump_for_all_atomics(rd::<GPtr<u8>>(p, 0x4F4), 0x734B90, 0);
            }
            rp_clump_destroy(rd::<GPtr<u8>>(p, 0x4F4));
            wr::<u32>(p, 0x4F4, 0);
            wr::<u32>(p, 0x4F8, 0);
        }
    }
    wr::<u16>(p, 0x504, 0);
    wr::<u16>(p, 0x508, 0);
    wr::<i32>(p, 0x740, -1);
}

/// `CPed::AddWeaponModel(int32 modelIndex)` (0x5E5ED0, `ret 4`). Nothing for -1 or when the active slot's
/// byte +0x15 is set. Otherwise an existing weapon clump is removed (`RemoveWeaponModel(-1)`), the model
/// info's `CreateInstance` becomes +0x4F4 with its "gunflash" frame in +0x4F8 (0 without an instance),
/// the model info gets a ref, +0x740 = model; for player types holding weapon 0x12 with model 0x158 and
/// no FX system yet, an FX system is created on bone 0x18's matrix (position (0,0,0) in a stack local),
/// made local, given the parent matrix and played.
#[hook(addr = 0x5E5ED0, conv = thiscall, name = "CPed::AddWeaponModel", expect = "83 EC 0C 55 8B 6C 24 14", this = "CPed")]
pub fn add_weapon_model(this: GPtr<u8>, model: i32) {
    let p = this;
    if model == -1 {
        return;
    }
    let slot = isize::from(rd::<i8>(p, 0x718)) * 0x1C;
    if rd::<u8>(p, 0x5B5 + slot) != 0 {
        return;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((model as usize).wrapping_mul(4))).read();
    if !rd::<GPtr<u8>>(p, 0x4F4).is_null() {
        remove_weapon_model_call(p, -1);
    }
    let inst = model_info_create_instance(mi);
    wr::<GPtr<u8>>(p, 0x4F4, inst);
    if inst.is_null() {
        wr::<u32>(p, 0x4F8, 0);
    } else {
        wr::<GPtr<u8>>(p, 0x4F8, get_frame_from_name(inst, GUNFLASH_NAME));
    }
    model_info_add_ref(mi);
    wr::<i32>(p, 0x740, model);
    let ty = rd::<u32>(p, 0x598);
    if ty != 0 && ty != 1 {
        return;
    }
    let w = p.byte_add(0x5A0 + isize::from(rd::<i8>(p, 0x718)) * 0x1C);
    if rd::<u32>(w, 0) != 0x12 || model != 0x158 || rd::<u32>(w, 0x18) != 0 {
        return;
    }
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(p, 0x18));
    let idx = rp_hanim_id_get_index(h, 0x18);
    let mats = rp_hanim_hierarchy_get_matrix_array(h);
    let mut pos = [0f32; 3];
    let fx = fx_create_system(
        g::<u8>(FX_MAN),
        PARACHUTE_FX_NAME,
        lp(&mut pos).cast(),
        mats.byte_add((idx.wrapping_shl(6)) as isize),
        0,
    );
    wr::<GPtr<u8>>(w, 0x18, fx);
    if !fx.is_null() {
        fx_set_local_particles(fx, 1);
        fx_copy_parent_matrix(rd::<GPtr<u8>>(w, 0x18));
        fx_play(rd::<GPtr<u8>>(w, 0x18));
    }
}

/// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080, `ret 0xC`; EAX = the weapon's slot).
/// Same type in the slot: slot 10 returns at once; otherwise total ammo += ammo capped at 99999 (signed
/// compare), `Reload`, and a weapon in state 3 (out of ammo) with ammo > 0 goes to state 0. Another type:
/// a previous weapon (type != 0) adds its total ammo for slots 3/4/5, its model is removed, the goggles
/// model too when the new weapon shares the night-vision (0x2D) slot, `Shutdown`; then `Initialise(type,
/// ammo, this)` and the model added when the slot is the active one and flag 0x100 of +0x46C is clear.
/// Finally state 0 unless state 3.
#[hook(addr = 0x5E6080, conv = thiscall, name = "CPed::GiveWeapon-", expect = "51 53 8B 5C 24 0C", this = "CPed")]
pub fn give_weapon(this: GPtr<u8>, ty: u32, ammo: u32, _like_unused: u32) -> u32 {
    let p = this;
    let mut slot = rd::<u32>(weapon_info(ty), 0x14);
    let w = p.byte_add(0x5A0 + (slot as isize).wrapping_mul(0x1C));
    let old = rd::<u32>(w, 0);
    if old == ty {
        if slot == 0xA {
            return slot;
        }
        let total = rd::<u32>(w, 0xC).wrapping_add(ammo);
        wr::<u32>(w, 0xC, total);
        wr::<u32>(w, 0xC, if (total as i32) < 99999 { total } else { 99999 });
        weapon_reload(w, p);
        if rd::<u32>(w, 4) != 3 {
            wr::<u32>(w, 4, 0);
            return slot;
        }
        if rd::<i32>(w, 0xC) > 0 {
            wr::<u32>(w, 4, 0);
        }
    } else {
        let mut ammo = ammo;
        if old != 0 {
            if matches!(slot, 3..=5) {
                ammo = ammo.wrapping_add(rd::<u32>(w, 0xC));
            }
            remove_weapon_model_call(p, rd::<i32>(weapon_info(old), 0xC));
            slot = rd::<u32>(weapon_info(ty), 0x14);
            if slot == rd::<u32>(weapon_info(0x2D), 0x14) {
                remove_goggles_model(p);
            }
            weapon_shutdown(w);
        }
        weapon_initialise(w, ty, ammo, p);
        let active = i32::from(rd::<i8>(p, 0x718));
        if slot as i32 == active && rd::<u32>(p, 0x46C) & 0x100 == 0 {
            let t = rd::<u32>(p, 0x5A0 + active as isize * 0x1C);
            add_weapon_model_call(p, rd::<i32>(weapon_info(t), 0xC));
        }
    }
    if rd::<u32>(w, 4) != 3 {
        wr::<u32>(w, 4, 0);
    }
    slot
}

/// `CPed::RequestDelayedWeapon()` (0x5E8910): with a delayed weapon (+0x710 != 0x37) both of its models
/// (weapon info +0xC/+0x10, -1 = none) are requested (flags 8); once both are loaded (load state 1) the
/// weapon is given with the delayed ammo (+0x714) and +0x710 reset to 0x37.
#[hook(addr = 0x5E8910, conv = thiscall, name = "CPed::RequestDelayedWeapon", expect = "57 8B F9 8B 87 10 07 00 00", this = "CPed")]
pub fn request_delayed_weapon(this: GPtr<u8>) {
    let p = this;
    if rd::<u32>(p, 0x710) == 0x37 {
        return;
    }
    let m1 = rd::<i32>(weapon_info(rd::<u32>(p, 0x710)), 0xC);
    let m2 = rd::<i32>(weapon_info(rd::<u32>(p, 0x710)), 0x10);
    if m1 != -1 {
        request_model(m1, 8);
    }
    if m2 != -1 {
        request_model(m2, 8);
    }
    let loaded = |m: i32| g::<u8>(MODEL_LOAD_STATE.wrapping_add((m as usize).wrapping_mul(20))).read() == 1;
    if m1 != -1 && !loaded(m1) {
        return;
    }
    if m2 != -1 && !loaded(m2) {
        return;
    }
    give_weapon_call(p, rd::<u32>(p, 0x710), rd::<u32>(p, 0x714), 1);
    wr::<u32>(p, 0x710, 0x37);
}

/// `CPed::GiveDelayedWeapon(eWeaponType, uint32 ammo)` (0x5E89B0, `ret 8`): non-player peds holding an
/// entity of type 6 (`GetTaskHold(false)` with an entity and byte +0x18 == 6) drop it; then, with no
/// delayed weapon pending, the weapon is stored (+0x714 ammo, +0x710 type) and requested.
#[hook(addr = 0x5E89B0, conv = thiscall, name = "CPed::GiveDelayedWeapon", expect = "56 8B F1 8B 86 98 05 00 00", this = "CPed")]
pub fn give_delayed_weapon(this: GPtr<u8>, ty: u32, ammo: u32) {
    let p = this;
    let pt = rd::<u32>(p, 0x598);
    if pt != 0 && pt != 1 {
        let t = intel_get_task_hold(rd::<GPtr<u8>>(p, 0x47C), 0);
        if !t.is_null() && !rd::<GPtr<u8>>(t, 8).is_null() && rd::<u8>(t, 0x18) == 6 {
            drop_entity_that_this_ped_is_holding(p, 1);
        }
    }
    if rd::<u32>(p, 0x710) == 0x37 {
        wr::<u32>(p, 0x714, ammo);
        wr::<u32>(p, 0x710, ty);
        request_delayed_weapon_call(p);
    }
}

/// `CPed::RemoveWeaponAnims(int32, float blendDelta)` (0x5F0250, `ret 8`; the first argument is not read).
/// 0x22 times (the original repeats the same id): the clump's association 0xE0 gets flag 4 of byte +0x2E
/// and, when bit 4 of that byte is set, `blendDelta` (+0x1C); otherwise a "found one" flag is set. With the
/// flag, `BlendAnimation(clump, animGroup +0x4D4, 3, -blendDelta)` (`fld`/`fchs`/`fstp`: an SNaN delta
/// comes back quiet).
#[hook(addr = 0x5F0250, conv = thiscall, name = "CPed::RemoveWeaponAnims", expect = "53 55 56 8B F1 32 DB", this = "CPed")]
pub fn remove_weapon_anims(this: GPtr<u8>, _unused: i32, delta: u32) {
    let p = this;
    let mut blend = false;
    for _ in 0..0x22 {
        let a = clump_get_association(rd::<GPtr<u8>>(p, 0x18), 0xE0);
        if a.is_null() {
            continue;
        }
        let f = rd::<u8>(a, 0x2E) | 4;
        wr::<u8>(a, 0x2E, f);
        if (f >> 4) & 1 != 0 {
            wr::<u32>(a, 0x1C, delta);
        } else {
            blend = true;
        }
    }
    if blend {
        let q = if delta & 0x7F80_0000 == 0x7F80_0000 && delta & 0x7F_FFFF != 0 { delta | 0x40_0000 } else { delta };
        blend_animation(rd::<GPtr<u8>>(p, 0x18), rd::<u32>(p, 0x4D4), 3, f32::from_bits(q ^ 0x8000_0000));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PRE_RENDER,
    HOOK_SETUP_LIGHTING,
    HOOK_REMOVE_WEAPON_MODEL,
    HOOK_ADD_WEAPON_MODEL,
    HOOK_GIVE_WEAPON,
    HOOK_REQUEST_DELAYED_WEAPON,
    HOOK_GIVE_DELAYED_WEAPON,
    HOOK_REMOVE_WEAPON_ANIMS,
];
