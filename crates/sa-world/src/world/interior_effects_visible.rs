//! WRITER-WORLD-2 (batch `world6-2`, SA:MP-executed): `InteriorManager_c`'s search for the interior
//! 2D effects around the camera. Ported from the 1.0 US bytes; gta-reversed has neither reversed.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{cam_pos, g, rd, wr, x};

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890): the result through the hidden pointer
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)` (0x564A20)
    fn find_objects_in_range(pos: GPtr<u8>, radius: f32, b2d: u32, count: GPtr<u8>, max: u32, out: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564A20 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `InteriorManager_c::IsInteriorEffectVisible(C2dEffect*, CEntity*)` (0x598690)
    fn is_interior_effect_visible_call(this: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>) -> u32 = thiscall 0x598690 => is_interior_effect_visible;
    /// `InteriorManager_c::AddSameGroupEffectInfos(intVisibleEffect_t*, int32)` (0x598430)
    fn add_same_group_effect_infos(this: GPtr<u8>, info: GPtr<u8>, max: i32) = thiscall 0x598430 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8), `CGame::currArea` (0xB72914).
const MODEL_INFOS: usize = 0xA9_B0C8;
const CURR_AREA: usize = 0xB7_2914;

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

fn model_info(e: GPtr<u8>) -> GPtr<u8> {
    g::<GPtr<u8>>((MODEL_INFOS as isize + isize::from(rd::<i16>(e, 0x22)) * 4) as usize).read()
}

/// `min <= v <= max` on all three axes in the bytes' order (`test ah, 1` against the minimum, `test
/// ah, 0x41; jp` against the maximum: NaN fails).
fn inside(v: [f32; 3], mn: &[f32; 3], mx: &[f32; 3]) -> bool {
    (0..3).all(|k| !x(v[k]).fcom(x(mn[k])).test_any(0x01) && !x(v[k]).fcom(x(mx[k])).test_parity(0x41))
}

fn read3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// The position of a ped: its matrix position, else its placement.
fn ped_pos(ped: GPtr<u8>) -> [f32; 3] {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    read3(if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) })
}

/// `InteriorManager_c::IsInteriorEffectVisible(C2dEffect* effect, CEntity* entity)` (0x598690,
/// `effect` unused): the world box of the entity's model bounding box (its eight corners through
/// the entity matrix, allocated from the placement when missing; start 1e6 / -1e6) contains the
/// camera position, player 0's position or player 1's (when present). The result is `bl | dword
/// [esp+0x12] | dword [esp+0x13]`: AL is the "any" bit, the upper bytes carry the two flags and the
/// low bytes of the box's min x (reproduced).
#[hook(addr = 0x598690, conv = thiscall, name = "InteriorManager_c::IsInteriorEffectVisible", this = "InteriorManager_c",
       expect = "81 EC 88 00 00 00",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       note = "may allocate the entity's matrix; EAX = bl | dword[esp+0x12] | dword[esp+0x13] (flags and min.x bytes)")]
pub fn is_interior_effect_visible(_this: GPtr<u8>, _effect: GPtr<u8>, e: GPtr<u8>) -> u32 {
    let col = rd::<GPtr<u8>>(model_info(e), 0x14);
    let mn0 = read3(col);
    let mx0 = read3(col.byte_add(0xC));
    let (a, b) = (mx0, mn0);
    let mut corners = [
        [a[0], a[1], a[2]],
        [a[0], b[1], a[2]],
        [b[0], b[1], a[2]],
        [b[0], a[1], a[2]],
        [a[0], a[1], b[2]],
        [a[0], b[1], b[2]],
        [b[0], b[1], b[2]],
        [b[0], a[1], b[2]],
    ];
    let mut mn = [f32::from_bits(0x4974_23F0); 3];
    let mut mx = [f32::from_bits(0xC974_23F0); 3];
    for c in corners.iter_mut() {
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            allocate_matrix(e);
            update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let mut out = [0f32; 3];
        let r = matrix_mul_vec(p(&mut out), rd::<GPtr<u8>>(e, 0x14), p(c));
        *c = read3(r);
        for k in 0..3 {
            if !x(c[k]).fcom(x(mn[k])).test_parity(0x05) {
                mn[k] = c[k];
            }
        }
        for k in 0..3 {
            if !x(c[k]).fcom(x(mx[k])).test_any(0x41) {
                mx[k] = c[k];
            }
        }
    }
    let b13 = u32::from(inside(read3(cam_pos().cast()), &mn, &mx));
    let b12 = u32::from(inside(ped_pos(find_player_ped(0)), &mn, &mx));
    let mut bl = 0u32;
    if !find_player_ped(1).is_null() && inside(ped_pos(find_player_ped(1)), &mn, &mx) {
        bl = 1;
    }
    let m = mn[0].to_bits();
    let (b14, b15, b16) = (m & 0xFF, (m >> 8) & 0xFF, (m >> 16) & 0xFF);
    let d12 = b12 | (b13 << 8) | (b14 << 16) | (b15 << 24);
    let d13 = b13 | (b14 << 8) | (b15 << 16) | (b16 << 24);
    bl | d12 | d13
}

/// `InteriorManager_c::GetVisibleEffects(intVisibleEffect_t* out, int32 max)` (0x598D80): every
/// building with an RW object within 100 of the camera (2D off, at most 2048) in the current area
/// (or area 13) contributes its interior effects (2D effect type 5) that `IsInteriorEffectVisible`
/// accepts and that are not yet listed for the same entity with the same group byte (+0x11): entry
/// (0x50 bytes) +0 entity, +4 = 1, +8 effect, +0x28 effect index, +0x4C = 0, while fewer than `max`.
/// Then `AddSameGroupEffectInfos(entry, max)` for each entry. Returns the entry count.
#[hook(addr = 0x598D80, conv = thiscall, name = "InteriorManager_c::GetVisibleEffects", this = "InteriorManager_c",
       expect = "B8 1C 20 00 00",
       unverified = "world6-2", route = "L2", hazards = "none",
       note = "0x2000-byte stack array (_chkstk 0x822070); the found count starts at -1")]
pub fn get_visible_effects(this: GPtr<u8>, out: GPtr<u8>, max: i32) -> i32 {
    let mut list = [GPtr::<u8>::NULL; 0x800];
    let mut count: u32 = 0xFFFF_FFFF;
    let mut cam = read3(cam_pos().cast());
    find_objects_in_range(p(&mut cam), 100.0, 0, p(&mut count), 0x800, p(&mut list), 1, 0, 0, 0, 0);
    let mut n: i32 = 0;
    let found = i32::from(count as u16 as i16);
    let mut i = 0;
    while i < found {
        let e = list[i as usize];
        let mi = model_info(e);
        let area = rd::<u8>(e, 0x2F);
        if !rd::<GPtr<u8>>(e, 0x18).is_null()
            && (u32::from(area) == g::<u32>(CURR_AREA).read() || area == 0xD)
            && rd::<u8>(mi, 0xD) != 0
        {
            let mut j: i32 = 0;
            loop {
                let fx = get_2d_effect(mi, j);
                if rd::<u8>(fx, 0xC) == 5 && is_interior_effect_visible_call(this, fx, e) as u8 != 0 {
                    let mut dup = false;
                    for k in 0..n {
                        let ent = out.byte_add(k as isize * 0x50);
                        if rd::<GPtr<u8>>(ent, 0) == e && rd::<u8>(rd::<GPtr<u8>>(ent, 8), 0x11) == rd::<u8>(fx, 0x11) {
                            dup = true;
                        }
                    }
                    if !dup && n < max {
                        let ent = out.byte_add(n as isize * 0x50);
                        wr::<GPtr<u8>>(ent, 0, e);
                        wr::<GPtr<u8>>(ent, 8, fx);
                        wr::<i32>(ent, 0x28, j);
                        wr::<u32>(ent, 4, 1);
                        wr::<u8>(ent, 0x4C, 0);
                        n += 1;
                    }
                }
                j += 1;
                if j >= i32::from(rd::<u8>(mi, 0xD)) {
                    break;
                }
            }
        }
        i += 1;
    }
    for k in 0..n {
        add_same_group_effect_infos(this, out.byte_add(k as isize * 0x50), max);
    }
    n
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IS_INTERIOR_EFFECT_VISIBLE, HOOK_GET_VISIBLE_EFFECTS];
