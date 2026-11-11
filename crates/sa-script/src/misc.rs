//! Non-switch SCRIPT functions (PORT-SCRIPT-4): `CBoundingBox::SetMinMax` and the three `CTheScripts` search light
//! movers. Follow the 1.0 US bytes (`.HOODLUM` bodies behind the stubs) and `docs/re/specs/SCRIPT/CBoundingBox.md`,
//! `CTheScripts.md`.
//!
//! Script search lights: `CTheScripts::ScriptSearchLightArray` at 0xA94D68, stride 0x7C; byte +3 the movement state,
//! +0x14/+0x18 the current x/y, +0x28..+0x3C two points, +0x40 the speed, +0x48 the followed entity (a registered
//! reference).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

orig! {
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CTheScripts::ScriptSearchLightArray` 0xA94D68 (0x7C bytes per light).
const SEARCH_LIGHTS: u32 = 0xA94D68;

/// The light of a script handle (`GetActualScriptThingIndex(h, 2)`), or `None` for a negative index.
fn light(h: i32) -> Option<GPtr<u8>> {
    let i = get_actual_script_thing_index(h, 2);
    (i >= 0).then(|| GPtr::from_addr(SEARCH_LIGHTS.wrapping_add((i as u32).wrapping_mul(0x7C)) as usize))
}

#[inline]
fn wr(l: GPtr<u8>, off: isize, v: u32) {
    l.byte_add(off).cast::<u32>().write(v);
}

/// `if (l->entity) l->entity->CleanUpOldReference(&l->entity)` (+0x48).
fn clean_up_entity(l: GPtr<u8>) {
    let slot = l.byte_add(0x48);
    let e = slot.cast::<GPtr<u8>>().read();
    if !e.is_null() {
        clean_up_old_reference(e, slot);
    }
}

/// `CBoundingBox::SetMinMax(CVector min, CVector max)` (stub 0x470100 → 0x15649B0, thiscall, `ret 0x18`): the six
/// dwords copied raw to `this+0..0x14`; returns `this`. (The body leaves ECX = this + 0xC; no caller reads it.)
#[hook(addr = 0x470100, conv = thiscall, name = "CBoundingBox::SetMinMax", expect = "E9 AB 48 0F 01", this = "CBoundingBox")]
pub fn bounding_box_set_min_max(b: GPtr<u8>, x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32) -> GPtr<u8> {
    let p = b.cast::<u32>();
    for (k, v) in [x0, y0, z0, x1, y1, z1].into_iter().enumerate() {
        p.wrapping_index(k as i32).write(v);
    }
    b
}

/// `CTheScripts::MoveSearchLightBetweenTwoPoints(int32 h, CVector p1, CVector p2, float speed)` (0x493360, cdecl):
/// `d1 = dy1*dy1 + dx1*dx1`, `d2` likewise (x87 registers, `dx = light.x (+0x14) - x`, `dy = light.y (+0x18) - y`);
/// +0x28..+0x3C = p1, p2 (raw); state (+3) = `d1 <= d2` ? 0x81 : 0x82 (greater or unordered); +0x40 = speed;
/// the followed entity's reference cleaned up and cleared.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x493360, conv = cdecl, name = "CTheScripts::MoveSearchLightBetweenTwoPoints", expect = "8B 44 24 04 6A 02")]
pub fn move_search_light_between_two_points(h: i32, x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32, speed: u32) {
    let Some(l) = light(h) else { return };
    let lx = l.byte_add(0x14).cast::<f32>().read();
    let ly = l.byte_add(0x18).cast::<f32>().read();
    let sq = |x: u32, y: u32| {
        let a = X87::from_f32(lx).fsub_m(f32::from_bits(x));
        let b = X87::from_f32(ly).fsub_m(f32::from_bits(y));
        b.fmul(b).fadd(a.fmul(a))
    };
    let d1 = sq(x1, y1);
    let d2 = sq(x2, y2);
    for (k, v) in [x1, y1, z1, x2, y2, z2].into_iter().enumerate() {
        wr(l, 0x28 + 4 * k as isize, v);
    }
    let near_first = matches!(d1.fcom(d2), Cmp::Less | Cmp::Equal);
    l.byte_add(3).write(if near_first { 0x81 } else { 0x82 });
    wr(l, 0x40, speed);
    clean_up_entity(l);
    wr(l, 0x48, 0);
}

/// `CTheScripts::MoveSearchLightToEntity(int32 h, CEntity* e, float speed)` (stub 0x493420 → 0x156F000, cdecl):
/// state 0x83, the two points zeroed, +0x40 = speed; the old entity's reference cleaned up (a disguised call to
/// 0x571A00 returning to 0x156F064); `+0x48 = e; e->RegisterReference(&+0x48)` (no null check).
#[hook(addr = 0x493420, conv = cdecl, name = "CTheScripts::MoveSearchLightToEntity", expect = "E9 DB BB 0D 01")]
pub fn move_search_light_to_entity(h: i32, e: GPtr<u8>, speed: u32) {
    let Some(l) = light(h) else { return };
    l.byte_add(3).write(0x83);
    for k in 0..6 {
        wr(l, 0x28 + 4 * k, 0);
    }
    wr(l, 0x40, speed);
    clean_up_entity(l);
    let slot = l.byte_add(0x48);
    slot.cast::<GPtr<u8>>().write(e);
    register_reference(e, slot);
}

/// `CTheScripts::MoveSearchLightToPointAndStop(int32 h, CVector p, float speed)` (stub 0x493480 → 0x1562220,
/// cdecl): +0x28..+0x30 = p (raw), +0x34..+0x3C = 0, state 0x84, +0x40 = speed; the followed entity's reference
/// cleaned up (disguised call to 0x571A00) and cleared.
#[hook(addr = 0x493480, conv = cdecl, name = "CTheScripts::MoveSearchLightToPointAndStop", expect = "E9 9B ED 0C 01")]
pub fn move_search_light_to_point_and_stop(h: i32, x: u32, y: u32, z: u32, speed: u32) {
    let Some(l) = light(h) else { return };
    for (k, v) in [x, y, z, 0, 0, 0].into_iter().enumerate() {
        wr(l, 0x28 + 4 * k as isize, v);
    }
    l.byte_add(3).write(0x84);
    wr(l, 0x40, speed);
    clean_up_entity(l);
    wr(l, 0x48, 0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_BOUNDING_BOX_SET_MIN_MAX,
    HOOK_MOVE_SEARCH_LIGHT_BETWEEN_TWO_POINTS,
    HOOK_MOVE_SEARCH_LIGHT_TO_ENTITY,
    HOOK_MOVE_SEARCH_LIGHT_TO_POINT_AND_STOP,
];
