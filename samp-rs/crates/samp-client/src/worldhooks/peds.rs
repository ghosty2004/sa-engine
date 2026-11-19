//! Ped hooks: pinned actors, ped fog and objects attached to ped bones.

use super::entity::{set_position, update_rw};
use super::fog::{rw_fog_enabled, set_fog};
use super::*;

// ---- CCivilianPed::ProcessControl (patch table row 20) ----------------------------------------------------------

/// Invulnerable actors (bullet + explosion + melee proof) cannot be pushed. Mirrors samp.dll 0x100A2CA0.
pub(super) extern "thiscall" fn civilian_process_control(ped: u32) {
    let flags: u32 = rd(ped + g::PHYSICAL_FLAGS);
    let pinned = world::invulnerable_flags(flags);
    if pinned {
        wr(ped + g::MOVE_SPEED, 0.0f32);
        wr(ped + g::MOVE_SPEED + 4, 0.0f32);
    }
    let m: u32 = rd(ped + g::ENTITY_MATRIX);
    let saved = (m != 0).then(|| v3(m + 0x30));
    thiscall0(g::CIVILIAN_PROCESS_CONTROL, ped);
    if world::invulnerable_flags(rd(ped + g::PHYSICAL_FLAGS)) {
        let m: u32 = rd(ped + g::ENTITY_MATRIX);
        if let (Some(p), true) = (saved, m != 0) {
            wr(m + 0x30, p[0]);
            wr(m + 0x34, p[1]);
            wr(m + 0x38, p[2]);
        }
        wr(ped + g::MOVE_SPEED, 0.0f32);
        wr(ped + g::MOVE_SPEED + 4, 0.0f32);
    }
}

// ---- ped Render fog (patch table rows 11–15) -----------------------------------------------------------------------

/// Fog off for the ped draw, then back to RenderWare's fog state. Mirrors samp.dll 0x100A3E70.
pub(super) extern "thiscall" fn ped_render(ped: u32) {
    set_fog(false);
    thiscall0(g::PED_RENDER, ped);
    set_fog(rw_fog_enabled());
}

// ---- attached objects on peds (patch table row 25) ---------------------------------------------------------------

pub(super) extern "C" fn process_peds_after_pre_render() {
    // SAFETY: CWorld::ProcessPedsAfterPreRender (cdecl, no arguments).
    unsafe {
        let f: extern "C" fn() = std::mem::transmute(g::PROCESS_PEDS_AFTER_PRE_RENDER as usize);
        f();
    }
    let list = try_shared(|s| if s.active { s.attached.clone() } else { Vec::new() }).unwrap_or_default();
    let mut animated: Vec<u32> = Vec::new();
    for a in list {
        place_attached(a, &mut animated);
    }
}

fn place_attached(a: Attached, animated: &mut Vec<u32>) {
    let obj = a.object;
    if obj == 0 || rd::<u32>(obj) != g::VTBL_OBJECT {
        return;
    }
    let ped = a.ped;
    if ped == 0 || rd::<u32>(ped) == g::VTBL_PLACEABLE || rd::<u32>(ped + g::ENTITY_RW_OBJECT) == 0 {
        // samp moves the object to (0,0,0) when the entity is invalid.
        set_position(obj, [0.0; 3]);
        return;
    }
    if !animated.contains(&ped) {
        thiscall0(g::UPDATE_RP_HANIM, ped);
        animated.push(ped);
    }
    let Some(bone) = bone_matrix(ped, a.bone) else { return };
    let at = |i: usize| [bone[i], bone[i + 1], bone[i + 2]];
    let bone = (crate::footsync::Rotation { right: at(0), forward: at(4), up: at(8) }, at(12));
    let (r, pos) = world::attached_transform(bone, a.offset, a.rotation, a.scale);
    let rows = [r.right, r.forward, r.up];
    vcall0(obj, 0x0C); // Remove()
    let m: u32 = rd(obj + g::ENTITY_MATRIX);
    if m != 0 {
        for (i, row) in rows.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                wr(m + 0x10 * i as u32 + 4 * j as u32, *v);
            }
        }
        wr(m + 0x30, pos[0]);
        wr(m + 0x34, pos[1]);
        wr(m + 0x38, pos[2]);
        update_rw(obj);
    }
    vcall0(obj, 0x08); // Add()
}

/// LTM of the ped bone `bone` (SA:MP bone id 1…18 = `ePedNode`): the frame data in `CPed::m_apBones` (+0x488)
/// gives the node id; its matrix comes from the skin clump's animation hierarchy.
pub fn bone_matrix(ped: u32, bone: i32) -> Option<[f32; 16]> {
    if !(1..=18).contains(&bone) {
        return None;
    }
    let frame_data: u32 = rd(ped + g::PED_BONES + 4 * bone as u32);
    if frame_data == 0 {
        return None;
    }
    let node_id: i32 = rd(frame_data + 0x14);
    let clump: u32 = rd(ped + g::ENTITY_RW_OBJECT);
    // SAFETY: GetAnimHierarchyFromSkinClump / RpHAnimIDGetIndex (cdecl) on the ped's live clump.
    let hier = unsafe {
        let f: extern "C" fn(u32) -> u32 = std::mem::transmute(g::GET_ANIM_HIERARCHY_FROM_SKIN_CLUMP as usize);
        f(clump)
    };
    if hier == 0 {
        return None;
    }
    // SAFETY: RpHAnimIDGetIndex (cdecl) on the hierarchy just returned for the live clump.
    let idx = unsafe {
        let f: extern "C" fn(u32, i32) -> i32 = std::mem::transmute(g::RP_HANIM_ID_GET_INDEX as usize);
        f(hier, node_id)
    };
    if idx < 0 {
        return None;
    }
    let mats: u32 = rd(hier + 8);
    if mats == 0 {
        return None;
    }
    let base = mats + 0x40 * idx as u32;
    Some(std::array::from_fn(|i| rd(base + 4 * i as u32)))
}
