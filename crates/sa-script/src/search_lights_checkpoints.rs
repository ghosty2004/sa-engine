//! The script search lights and checkpoints of `CTheScripts` that SA:MP executes (batch `samp-script-1`):
//! `HighlightImportantArea`, `AddScriptSearchLight`, `AttachSearchlightToSearchlightObject`, `AddScriptCheckpoint`,
//! `RemoveScriptCheckpoint`, `RenderAllSearchLights`. Ported from the 1.0 US bytes; three bodies live in `.HOODLUM`
//! behind 5-byte stubs, and two of them reach `CEntity` reference helpers and `CCheckpoints::DeleteCP` through
//! no-CD disguised calls (`call $+5; push r; r = key op key; xchg [esp], r; add [esp+4], N; ret`), decoded from the
//! exe image. Original bugs are kept (the free-slot searches can step one record past their arrays).
//!
//! `CTheScripts::ScriptSearchLightArray[8]` (0x7C each) at 0xA94D68: +0 used, +1/+2 flags, +3 = 0x80, +4 id
//! counter, +8 position, +0x14 target, +0x20 target radius, +0x24 base radius, +0x28..+0x40 movement state
//! (`script_search_lights`), +0x44 attached entity, +0x48 followed entity, +0x4C/+0x50/+0x54 the three objects of
//! `AttachSearchlightToSearchlightObject` in its argument order, +0x58/+0x64/+0x70 the three cone vectors
//! `CHeli::SearchLightCone` updates. `CTheScripts::ScriptCheckpointArray[20]` (8 each) at
//! 0xA44070: +0 used, +2 id counter, +4 `CCheckpoint*`. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CTheScripts::GetNewUniqueScriptThingIndex(int32 index, uint8 type)` (0x483720, cdecl)
    fn get_new_unique_script_thing_index(index: i32, ty: u32) -> i32 = cdecl 0x483720 => crate::script_things::get_new_unique_script_thing_index;
    /// `CTheScripts::GetActualScriptThingIndex(int32 handle, uint8 type)` (0x4839A0, cdecl)
    fn get_actual_script_thing_index(handle: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CCheckpoints::DeleteCP(uint32 id, uint16 type)` (0x722FC0, cdecl)
    fn checkpoints_delete_cp(id: u32, ty: u32) = cdecl 0x722FC0 => unported;
    /// `CCheckpoints::PlaceMarker(uint32 id, uint16 type, CVector& pos, CVector& dir, float size, uint8 r, g, b, a,
    /// uint16 pulsePeriod, float pulseFraction, int16 rotateRate)` (0x722C40, cdecl): the colour bytes and the
    /// 16-bit arguments are pushed as whole dwords
    fn checkpoints_place_marker(id: i32, ty: u32, pos: GPtr<u8>, dir: GPtr<u8>, size: u32, r: u32, g: u32, b: u32,
        a: u32, period: u32, fraction: u32, rotate: u32) -> GPtr<u8> = cdecl 0x722C40 => unported;
    /// `CWorld::FindGroundZForCoord(float x, float y)` (0x569660, cdecl, ST0; exact as f64)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CShadows::RenderIndicatorShadow(uint32 id, uint8 type, RwTexture*, CVector* pos, float frontX, float frontY,
    /// float sideX, float sideY, int16 intensity)` (0x70CCB0, cdecl)
    fn shadows_render_indicator_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32,
        intensity: u32) = cdecl 0x70CCB0 => unported;
    /// `CHeli::SearchLightCone(int32 id, CVector start, CVector end, float targetRadius, float power, uint8
    /// clipIfColliding, uint8 drawShadow, CVector& coneSpot0, CVector& coneSpot1, CVector& coneSpot2, bool,
    /// float baseRadius, float, float, float)` (0x6C58E0, cdecl; the vectors by value as three dwords each)
    fn heli_search_light_cone(id: u32, sx: u32, sy: u32, sz: u32, ex: u32, ey: u32, ez: u32, target_radius: u32,
        power: u32, clip: u32, shadow: u32, c0: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, flag: u32, base_radius: u32,
        a: u32, b: u32, c: u32) = cdecl 0x6C58E0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790, cdecl): returns `out`
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
}

const SEARCH_LIGHTS: usize = 0xA9_4D68;
const SEARCH_LIGHT_SIZE: usize = 0x7C;
const NUM_SEARCH_LIGHTS: usize = 0xA9_0830;
const CHECKPOINTS: usize = 0xA4_4070;
const NUM_CHECKPOINTS: usize = 0xA4_4068;
/// `.rdata`: 0.5 (0x858B8C), -100.0 (0x859014), 2.0 (0x858CA0)
const K_HALF: usize = 0x85_8B8C;
const K_MINUS_100: usize = 0x85_9014;
const K_TWO: usize = 0x85_8CA0;
const ONE: u32 = 0x3F80_0000;

fn gp<T: sa_core::mem::GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

fn light(i: usize) -> GPtr<u8> {
    gp(SEARCH_LIGHTS + i * SEARCH_LIGHT_SIZE)
}

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: usize) -> T {
    p.byte_add(off as isize).cast::<T>().read()
}

fn wr<T: sa_core::mem::GameData>(p: GPtr<u8>, off: usize, v: T) {
    p.byte_add(off as isize).cast::<T>().write(v)
}

fn kf(a: usize) -> f32 {
    gp::<f32>(a).read()
}

/// The reference slot `p + off`: `CleanUpOldReference` on its old entity (when set), then the slot := `e`, then
/// `RegisterReference` on `e` (when set). `reg = false`: only the clean-up and a null store.
fn set_ref(p: GPtr<u8>, off: usize, e: GPtr<u8>, reg: bool) {
    let slot = p.byte_add(off as isize);
    let old = rd::<GPtr<u8>>(p, off);
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    wr(p, off, e);
    if reg && !e.is_null() {
        register_reference(e, slot);
    }
}

/// `CTheScripts::HighlightImportantArea(int32 id, float x1, float y1, float x2, float y2, float z)` (0x485E00,
/// cdecl): the box's minimum and maximum per axis (`x1 > x2` ordered picks `x2` as minimum, else `x1`), the centre
/// `((max + min) * 0.5f)` per axis (f32); `z <= -100.0f` (ordered: `test ah, 0x41; jp`) takes
/// `FindGroundZForCoord(cx, cy) + 2.0f`, else `z`; then `CShadows::RenderIndicatorShadow(id, 2, nullptr, &centre,
/// maxX - cx, 0, 0, cy - maxY, 0)`.
#[hook(addr = 0x485E00, conv = cdecl, name = "CTheScripts::HighlightImportantArea-", expect = "D9 44 24 08 83 EC 0C",
       unverified = "samp-script-1", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn highlight_important_area(id: u32, x1: f32, y1: f32, x2: f32, y2: f32, z: f32) {
    let xf = X87::from_f32;
    let (min_x, max_x) = if !xf(x1).fcom(xf(x2)).test_any(0x41) { (x2, x1) } else { (x1, x2) };
    let (min_y, max_y) = if !xf(y1).fcom(xf(y2)).test_any(0x41) { (y2, y1) } else { (y1, y2) };
    let cx = xf(max_x).fadd_m(min_x).fmul_m(kf(K_HALF)).to_f32();
    let cy = xf(max_y).fadd_m(min_y).fmul_m(kf(K_HALF)).to_f32();
    let cz = if xf(z).fcom(xf(kf(K_MINUS_100))).test_parity(0x41) {
        z
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(cx.to_bits(), cy.to_bits())).fadd_m(kf(K_TWO)).to_f32()
    };
    let mut c = [cx.to_bits(), cy.to_bits(), cz.to_bits()];
    let side_y = xf(cy).fsub_m(max_y).to_f32();
    let front_x = xf(max_x).fsub_m(cx).to_f32();
    shadows_render_indicator_shadow(id, 2, 0, GPtr::from_raw(c.as_mut_ptr()).cast(), front_x.to_bits(), 0, 0,
        side_y.to_bits(), 0);
}

/// `CTheScripts::AddScriptSearchLight(CVector pos, CEntity* entity, CVector target, float targetRadius, float
/// baseRadius)` (0x493000 → `.HOODLUM` 0x15692C0, cdecl, 9 dwords): the slot is 0 when record 0 is free, else
/// the first free record from 1 on (up to 8: one past the array when all are used); +1 = 0, +3 = 0x80, +0 = +2 =
/// 1; position +8, target +0x14, radii +0x20/+0x24; +0x28..+0x40 = 0; the references: +0x48 cleared, +0x44 :=
/// `entity` (registered), +0x4C, +0x50, +0x54 cleared; `++NumberOfScriptSearchLights` (0xA90830 word); returns
/// `GetNewUniqueScriptThingIndex(slot, 2)`.
#[hook(addr = 0x493000, conv = cdecl, name = "CTheScripts::AddScriptSearchLight", expect = "E9 BB 62 0D 01",
       unverified = "samp-script-1", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "body in .HOODLUM at 0x15692C0")]
pub fn add_script_search_light(px: u32, py: u32, pz: u32, entity: GPtr<u8>, tx: u32, ty: u32, tz: u32, target_radius: u32,
    base_radius: u32) -> i32 {
    let mut slot = 0usize;
    if gp::<u8>(SEARCH_LIGHTS).read() != 0 {
        let mut off = 0usize;
        while off < 0x3E0 {
            let used = gp::<u8>(SEARCH_LIGHTS + SEARCH_LIGHT_SIZE + off).read();
            off += SEARCH_LIGHT_SIZE;
            slot += 1;
            if used == 0 {
                break;
            }
        }
    }
    let l = light(slot);
    wr::<u8>(l, 1, 0);
    wr::<u8>(l, 3, 0x80);
    wr::<u8>(l, 0, 1);
    wr::<u8>(l, 2, 1);
    for (o, v) in [(8, px), (0xC, py), (0x10, pz), (0x14, tx), (0x18, ty), (0x1C, tz), (0x24, base_radius), (0x20, target_radius)] {
        wr::<u32>(l, o, v);
    }
    for o in (0x28..=0x40).step_by(4) {
        wr::<u32>(l, o, 0);
    }
    set_ref(l, 0x48, GPtr::NULL, false);
    set_ref(l, 0x44, entity, true);
    set_ref(l, 0x4C, GPtr::NULL, false);
    set_ref(l, 0x50, GPtr::NULL, false);
    let bulb = rd::<GPtr<u8>>(l, 0x54);
    if !bulb.is_null() {
        clean_up_old_reference(bulb, l.byte_add(0x54));
    }
    let n = gp::<u16>(NUM_SEARCH_LIGHTS);
    n.write(n.read().wrapping_add(1));
    wr::<u32>(l, 0x54, 0);
    get_new_unique_script_thing_index(slot as i32, 2)
}

/// `CTheScripts::AttachSearchlightToSearchlightObject(int32 handle, CObject* tower, CObject* housing, CObject* bulb,
/// CVector offset)` (0x4934F0 → `.HOODLUM` 0x1565770, cdecl): `GetActualScriptThingIndex(handle, 2)`; negative →
/// nothing. Otherwise the record's +8 = offset; +0x44 cleared (its `CleanUpOldReference` reached through a
/// disguised call, 0x15657B2 → 0x571A00); +0x4C := tower, +0x50 := housing, +0x54 := bulb, each registered (the
/// last `RegisterReference` through a disguised call, 0x1565835 → 0x571B70).
#[hook(addr = 0x4934F0, conv = cdecl, name = "CTheScripts::AttachSearchlightToSearchlightObject", expect = "E9 7B 22 0D 01",
       unverified = "samp-script-1", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "body in .HOODLUM at 0x1565770")]
pub fn attach_searchlight_to_searchlight_object(handle: i32, tower: GPtr<u8>, housing: GPtr<u8>, bulb: GPtr<u8>,
    ox: u32, oy: u32, oz: u32) {
    let i = get_actual_script_thing_index(handle, 2);
    if i < 0 {
        return;
    }
    let l = gp::<u8>((i as u32).wrapping_mul(0x7C).wrapping_add(SEARCH_LIGHTS as u32) as usize);
    wr::<u32>(l, 8, ox);
    wr::<u32>(l, 0xC, oy);
    wr::<u32>(l, 0x10, oz);
    set_ref(l, 0x44, GPtr::NULL, false);
    set_ref(l, 0x4C, tower, true);
    set_ref(l, 0x50, housing, true);
    set_ref(l, 0x54, bulb, true);
}

/// `CTheScripts::AddScriptCheckpoint(CVector pos, CVector pointTo, float radius, int32 type)` (0x4935A0, cdecl): the
/// slot is 0 when record 0 is free, else the first free record from 1 on (up to 20: one past the array when all
/// are used); marked used; `id = GetNewUniqueScriptThingIndex(slot, 3)`. The colour: types 0..2 alpha 0x20, 3..8
/// alpha 0x60, both red 0xFF, green 0, blue 0, stored as bytes into the low byte of the argument slots of x
/// (alpha), y (blue), z (green), pointTo.x (red) and pushed as those whole dwords; for other types the four argument
/// dwords are pushed unchanged. `CCheckpoints::PlaceMarker(id, type, &pos, &pointTo, radius, r, g, b, a, 0x400,
/// 0.075f, 0)`; `++NumberOfScriptCheckpoints` (0xA44068 word); the record's +4 = the marker. Returns `id`.
#[hook(addr = 0x4935A0, conv = cdecl, name = "CTheScripts::AddScriptCheckpoint", expect = "A0 70 40 A4 00",
       unverified = "samp-script-1", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "the original writes the colour bytes into its own argument slots (x, y, z, pointTo.x); no caller reads them")]
pub fn add_script_checkpoint(x: u32, y: u32, z: u32, dx: u32, dy: u32, dz: u32, radius: u32, ty: u32) -> i32 {
    let mut slot = 0usize;
    if gp::<u8>(CHECKPOINTS).read() != 0 {
        while slot < 0x14 {
            let used = gp::<u8>(CHECKPOINTS + 8 + slot * 8).read();
            slot += 1;
            if used == 0 {
                break;
            }
        }
    }
    gp::<u8>(CHECKPOINTS + slot * 8).write(1);
    let id = get_new_unique_script_thing_index(slot as i32, 3);
    let mut pos = [x, y, z];
    let mut dir = [dx, dy, dz];
    let (mut a, mut b, mut g, mut r) = (x, y, z, dx);
    if ty <= 8 {
        let alpha = if ty <= 2 { 0x20 } else { 0x60 };
        a = (a & !0xFF) | alpha;
        r = (r & !0xFF) | 0xFF;
        g &= !0xFF;
        b &= !0xFF;
    }
    let cp = checkpoints_place_marker(id, ty, GPtr::from_raw(pos.as_mut_ptr()).cast(), GPtr::from_raw(dir.as_mut_ptr()).cast(),
        radius, r, g, b, a, 0x400, 0x3D99_999A, 0);
    let n = gp::<u16>(NUM_CHECKPOINTS);
    n.write(n.read().wrapping_add(1));
    gp::<GPtr<u8>>(CHECKPOINTS + 4 + slot * 8).write(cp);
    id
}

/// `CTheScripts::RemoveScriptCheckpoint(int32 handle)` (0x4936C0 → `.HOODLUM` 0x156B9F0, cdecl):
/// `GetActualScriptThingIndex(handle, 3)`; -1 → nothing. Otherwise the record's used byte := 0; when it holds a
/// checkpoint, `CCheckpoints::DeleteCP(cp->id (+4), cp->type (+0 word))` (a disguised call, 0x156BA24 → 0x722FC0);
/// `--NumberOfScriptCheckpoints`; +4 := null.
#[hook(addr = 0x4936C0, conv = cdecl, name = "CTheScripts::RemoveScriptCheckpoint", expect = "E9 2B 83 0D 01",
       unverified = "samp-script-1", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "body in .HOODLUM at 0x156B9F0")]
pub fn remove_script_checkpoint(handle: i32) {
    let i = get_actual_script_thing_index(handle, 3);
    if i == -1 {
        return;
    }
    let rec = (i as u32).wrapping_mul(8).wrapping_add(CHECKPOINTS as u32) as usize;
    gp::<u8>(rec).write(0);
    let cp = gp::<GPtr<u8>>(rec + 4).read();
    if !cp.is_null() {
        checkpoints_delete_cp(rd::<u32>(cp, 4), u32::from(rd::<u16>(cp, 0)));
    }
    let n = gp::<u16>(NUM_CHECKPOINTS);
    n.write(n.read().wrapping_sub(1));
    gp::<u32>(rec + 4).write(0);
}

/// `CTheScripts::RenderAllSearchLights()` (0x493E30, cdecl): for each of the 8 records in use (id `0x216 + i`):
/// the cone starts at the record's position (+8) when neither the attached entity (+0x44) nor the bulb (+0x54) is
/// set; otherwise at that entity (+0x44 first) transformed: its matrix allocated and updated from the placement
/// when missing, `Multiply3x3(matrix, offset)` plus the matrix position (or the placement position without a
/// matrix), f32 per axis. `CHeli::SearchLightCone(id, start, target (+0x14), +0x20, 1.0, byte +1, byte +2, &+0x58,
/// &+0x64, &+0x70, 1, +0x24, 0, 0, 1.0)`.
#[hook(addr = 0x493E30, conv = cdecl, name = "CTheScripts::RenderAllSearchLights", expect = "83 EC 18 53 55 56 57",
       unverified = "samp-script-1", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "CHeli::SearchLightCone writes the cone vectors of each record")]
pub fn render_all_search_lights() {
    for i in 0..8usize {
        let l = light(i);
        if rd::<u8>(l, 0) == 0 {
            continue;
        }
        let mut e = rd::<GPtr<u8>>(l, 0x44);
        if e.is_null() {
            e = rd::<GPtr<u8>>(l, 0x54);
        }
        let start = if e.is_null() {
            [rd::<u32>(l, 8), rd::<u32>(l, 0xC), rd::<u32>(l, 0x10)]
        } else {
            let mut v = [rd::<u32>(l, 8), rd::<u32>(l, 0xC), rd::<u32>(l, 0x10)];
            if rd::<u32>(e, 0x14) == 0 {
                placeable_allocate_matrix(e);
                simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
            }
            let mut out = [0u32; 3];
            let r = multiply3x3(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(e, 0x14),
                GPtr::from_raw(v.as_mut_ptr()).cast());
            v = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
            let m = rd::<GPtr<u8>>(e, 0x14);
            let p = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
            let add = |a: u32, o: usize| X87::from_f32(f32::from_bits(a)).fadd_m(rd::<f32>(p, o)).to_f32().to_bits();
            [add(v[0], 0), add(v[1], 4), add(v[2], 8)]
        };
        heli_search_light_cone(
            0x216 + i as u32,
            start[0], start[1], start[2],
            rd::<u32>(l, 0x14), rd::<u32>(l, 0x18), rd::<u32>(l, 0x1C),
            rd::<u32>(l, 0x20),
            ONE,
            u32::from(rd::<u8>(l, 1)),
            u32::from(rd::<u8>(l, 2)),
            l.byte_add(0x58), l.byte_add(0x64), l.byte_add(0x70),
            1,
            rd::<u32>(l, 0x24),
            0, 0, ONE,
        );
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_HIGHLIGHT_IMPORTANT_AREA,
    HOOK_ADD_SCRIPT_SEARCH_LIGHT,
    HOOK_ATTACH_SEARCHLIGHT_TO_SEARCHLIGHT_OBJECT,
    HOOK_ADD_SCRIPT_CHECKPOINT,
    HOOK_REMOVE_SCRIPT_CHECKPOINT,
    HOOK_RENDER_ALL_SEARCH_LIGHTS,
];
