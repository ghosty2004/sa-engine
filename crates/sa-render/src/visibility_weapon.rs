//! `CVisibilityPlugins::RenderWeaponCB` (0x733670, batch `std10-render-2`) and
//! `CVisibilityPlugins::RenderWeaponPedsForPC` (0x732F30, batch `std12-render-1`): ported from the 1.0 US bytes
//! (gta-reversed's versions as reading aids). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

orig! {
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `AtomicDefaultRenderCallBack(atomic)` (0x7491C0)
    fn atomic_default_render(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7491C0 => unported;
    /// `RwRenderStateSet` (`RwEngineInstance->dOpenDevice.fpRenderStateSet`, +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// virtual `CPed::SetupLighting()` (vtable +0x4C)
    fn ped_setup_lighting(p: GPtr<u8>) -> u8 = thiscall vslot 0x4C => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwMatrixTranslate(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F2450)
    fn rw_matrix_translate(m: GPtr<u8>, t: GPtr<u8>, op: u32) -> GPtr<u8> = cdecl 0x7F2450 => unported;
    /// `RwMatrixRotate(RwMatrix*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7F1FD0), the angle as bits
    fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: u32, op: u32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// `CPed::SetGunFlashAlpha(bool)` (0x5DF400)
    fn ped_set_gun_flash_alpha(p: GPtr<u8>, right: u32) = thiscall 0x5DF400 => unported;
    /// `CPed::ResetGunFlashAlpha()` (0x5DF4E0)
    fn ped_reset_gun_flash_alpha(p: GPtr<u8>) = thiscall 0x5DF4E0 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `RpClumpRender(RpClump*)` (0x749B20)
    fn rp_clump_render(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B20 => unported;
    /// `CPed::GetWeaponSkill()` (0x5E6580)
    fn ped_get_weapon_skill(p: GPtr<u8>) -> u32 = thiscall 0x5E6580 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
}

/// `CVisibilityPlugins::ms_weaponPedsForPC`: the used list's head link and the tail link's `prev` (links: data +0,
/// prev +4).
const WEAPON_PEDS_HEAD: usize = 0xC88224;
const WEAPON_PEDS_TAIL_PREV: usize = 0xC88234;

/// The ped's active weapon type (`m_aWeapons[m_nActiveWeaponSlot].m_nType`: slot +0x718 as i8, 0x1C-byte weapons
/// at +0x5A0).
fn active_weapon_type(ped: GPtr<u8>) -> u32 {
    rd::<u32>(ped, 0x5A0 + 0x1C * isize::from(rd::<i8>(ped, 0x718)))
}

/// 64-byte `rep movs` of an `RwMatrix`.
fn copy_matrix(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..16isize {
        wr::<u32>(dst, 4 * i, rd::<u32>(src, 4 * i));
    }
}

/// `CVisibilityPlugins::RenderWeaponPedsForPC()` (0x732F30, cdecl). Render states: z-test, z-write and fog on,
/// source blend 5 (src alpha), destination blend 6 (inverse src alpha), alpha reference 20. Then the weapon peds
/// list from the tail's `prev` back to the head: for a ped with a weapon clump (+0x4F4): `SetupLighting`; the
/// hand bone (24, or 3 for the parachute, weapon type 0x2E) matrix of the skin hierarchy (for bone 0 the clump
/// frame's modelling matrix: unreached) copied into the weapon frame's modelling matrix; for the parachute
/// `RwMatrixTranslate(0x8D60B0)` and `RwMatrixRotate(0x8D2338, 90)` (pre-concat); `SetGunFlashAlpha(false)`,
/// `RwFrameUpdateObjects`, `RpClumpRender`; with the weapon info's twin-pistol flag (+0x18 bit 11) the same with
/// the left hand (bone 0x22), `RwMatrixRotate(0x8D60A4, 180)`, `RwMatrixTranslate(0x8D6098)` and
/// `SetGunFlashAlpha(true)`; then `ResetGunFlashAlpha`.
#[hook(addr = 0x732F30, conv = cdecl, name = "CVisibilityPlugins::RenderWeaponPedsForPC", expect = "83 EC 0C A1 24 7B C9 00",
       unverified = "std12-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn render_weapon_peds_for_pc() {
    let mut link = g::<GPtr<u8>>(WEAPON_PEDS_TAIL_PREV).read();
    for (state, value) in [(6, 1), (8, 1), (0xE, 1), (0xA, 5), (0xB, 6), (0x1E, 0x14)] {
        rw_render_state_set(state, value);
    }
    while link.addr() != WEAPON_PEDS_HEAD {
        let ped = rd::<GPtr<u8>>(link, 0);
        if !ped.is_null() && !rd::<GPtr<u8>>(ped, 0x4F4).is_null() {
            ped_setup_lighting(ped);
            let bone = if active_weapon_type(ped) == 0x2E { 3 } else { 0x18 };
            let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
            let idx = rp_hanim_id_get_index(hier, bone);
            let mut src = rp_hanim_hierarchy_get_matrix_array(hier).byte_add((idx << 6) as isize);
            if bone == 0 {
                let c = rd::<GPtr<u8>>(ped, 0x18);
                src = if c.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(c, 4).byte_add(0x10) };
            }
            let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x4F4), 4);
            let m = frame.byte_add(0x10);
            copy_matrix(m, src);
            if active_weapon_type(ped) == 0x2E {
                rw_matrix_translate(m, GPtr::from_addr(0x8D60B0), 1);
                rw_matrix_rotate(m, GPtr::from_addr(0x8D2338), 0x42B4_0000, 1);
            }
            ped_set_gun_flash_alpha(ped, 0);
            rw_frame_update_objects(frame);
            rp_clump_render(rd::<GPtr<u8>>(ped, 0x4F4));
            let ty = active_weapon_type(ped);
            let info = get_weapon_info(ty, ped_get_weapon_skill(ped));
            if (rd::<u32>(info, 0x18) >> 11) & 1 != 0 {
                let idx = rp_hanim_id_get_index(hier, 0x22);
                let src = rp_hanim_hierarchy_get_matrix_array(hier).byte_add((idx << 6) as isize);
                copy_matrix(m, src);
                rw_matrix_rotate(m, GPtr::from_addr(0x8D60A4), 0x4334_0000, 1);
                rw_matrix_translate(m, GPtr::from_addr(0x8D6098), 1);
                ped_set_gun_flash_alpha(ped, 1);
                rw_frame_update_objects(frame);
                rp_clump_render(rd::<GPtr<u8>>(ped, 0x4F4));
            }
            ped_reset_gun_flash_alpha(ped);
        }
        link = rd::<GPtr<u8>>(link, 4);
    }
}

/// `ms_pCameraPosn`, `TheCamera.m_fLODDistMultiplier`, the frame plugin offset.
const CAMERA_POSN: usize = 0xC88050;
const LOD_DIST_MULT: usize = 0xB6F118;
const FRAME_PLUGIN_OFFSET: usize = 0x8D6090;

/// `CVisibilityPlugins::RenderWeaponCB(RpAtomic*)` (0x733670, cdecl): the clump frame's plugin data (the model info,
/// its draw distance at +0x18) times the camera's LOD multiplier is the range; rendered (`AtomicDefaultRenderCallBack`)
/// when `r * r > (dz*dz + dx*dx) + dy*dy` for the frame's LTM position relative to the camera (all on the x87
/// stack). Returns the atomic.
#[hook(addr = 0x733670, conv = cdecl, name = "CVisibilityPlugins::RenderWeaponCB", expect = "8B 0D 90 60 8D 00 56 57 8B 7C",
       unverified = "std10-render-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_weapon_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4);
    let mi = rd::<GPtr<u8>>(frame, g::<i32>(FRAME_PLUGIN_OFFSET).read() as isize);
    let pos = frame_get_ltm(frame).byte_add(0x30);
    let cam = g::<GPtr<u8>>(CAMERA_POSN).read();
    let dx = X87::from_f32(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0));
    let dy = X87::from_f32(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4));
    let dz = X87::from_f32(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(cam, 8));
    let r = X87::from_f32(g::<f32>(LOD_DIST_MULT).read()).fmul_m(rd::<f32>(mi, 0x18));
    let sum = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if !r.fmul(r).fcom(sum).test_any(0x41) {
        atomic_default_render(atomic);
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_WEAPON_CB, HOOK_RENDER_WEAPON_PEDS_FOR_PC];
