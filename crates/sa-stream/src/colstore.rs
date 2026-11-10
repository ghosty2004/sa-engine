//! `CColStore` (0x410390–0x4114D0): the collision-file slots (`ColDef` pool at 0x965560, 255 slots),
//! their quad tree and the per-frame collision streaming. PORT-STREAM owner A.
//!
//! Ported from the 1.0 US bytes (capstone + Ghidra, `.HOODLUM` bodies followed); gta-reversed's
//! `Collision/ColStore.cpp` is a reading aid. Where they differ the original is kept:
//! - every slot loop is the fixed range 1..=254 (`ebp += 0x2C` while `< 0x2BD4`), the pool pointer
//!   re-read after each call (gr: `ms_pColPool->GetSize()`);
//! - `AddRef`/`RemoveRef`/`IncludeModelIndex` use `GetAt` without a null check: a free slot writes at
//!   address 0x26/0x22/0x24 (gr-deviations `ColStore.cpp:112`/`:388`, keep-original);
//! - `AddColSlot` (0x411140) does **not** copy the name into `ColDef::name` (*new*: gr `strcpy_s`); the
//!   `"stadint"` prefix length is passed through the no-CD indirection 0xCB73E0 (not decodable
//!   statically; 7 = gr's `starts_with("stadint")`, the only value consistent with the other prefix
//!   tests and the string length);
//! - `FindColSlot` (0x410390) is `return -1` (spec CColStore.md);
//! - float compares are the original's x87 compares (`test ah,5 / jnp` = "not below or unordered",
//!   `test ah,0x41 / je` = "above"), so NaN coordinates behave as in the original.

use sa_core::abi::HookReg;
use sa_core::mem::{GBool, GPtr};
use sa_core::pool::CPool;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::shared::{field, info, model_info, ColDef, ColPool, COL_POOL, RESOURCE_ID_COL};

sa_core::globals! {
    list GLOBALS;
    /// `CColStore::ms_EntityAreaCode`
    pub static ENTITY_AREA_CODE @ 0x965554: i32 = 0;
    /// `CColStore::ms_bCollisionNeeded`
    pub static COLLISION_NEEDED @ 0x965558: GBool = GBool::FALSE;
    /// `ms_pQuadTree` (`CQuadTreeNode<ColDef*>*`)
    pub static QUAD_TREE @ 0x96555C: GPtr<u8> = GPtr::NULL;
    /// `CColStore::ms_vecCollisionNeeded`
    pub static COLLISION_NEEDED_POS @ 0x965580: [f32; 3] = [0.0; 3];
    // foreign (owner: streaming) -- CStreaming::ms_disableStreaming is declared in streaming.rs
}

/// `CTheScripts::MissionCleanUp.m_Objects` (75 × {u8 type, i32 handle}) at 0xA90850 (read only).
const MISSION_CLEANUP_OBJECTS: usize = 0xA90850;
const MISSION_CLEANUP_COUNT: usize = 75;
/// float constants of the image
const F_120: u32 = 0x42F0_0000; // 0x858BB0
const F_M110: u32 = 0xC2DC_0000; // 0x858BAC
const F_110: u32 = 0x42DC_0000; // 0x858BA8
const F_M80: u32 = 0xC2A0_0000; // 0x858BA0
const F_80: u32 = 0x42A0_0000; // 0x858A14
const F_20: u32 = 0x41A0_0000; // 0x858BA4
/// "procobj", "proc_int", "proc_int2" (0x858BDC, 0x858BD0, 0x858BC4)
const PROCEDURAL: [u32; 3] = [0x858BDC, 0x858BD0, 0x858BC4];
/// (prefix, length) pairs of the interior test, in the original's order.
const INTERIOR_PREFIXES: [(u32, u32); 13] = [
    (0x8589AC, 6), // int_la
    (0x8589A4, 6), // int_sf
    (0x85899C, 7), // int_veg
    (0x858990, 8), // int_cont
    (0x858A04, 8), // gen_int1
    (0x8589F8, 8), // gen_int2 (called through a computed return, 0x15633A0)
    (0x8589EC, 8), // gen_int3
    (0x8589E0, 8), // gen_int4
    (0x8589D4, 8), // gen_int5
    (0x8589C8, 8), // gen_intb
    (0x8589BC, 8), // savehous
    (0x858984, 8), // levelmap (after the two exact compares below)
    (0x8589B4, 7), // stadint (length through 0xCB73E0, see the module docs)
];
/// "props", "props2" (exact `_stricmp`; "props2" called through a computed return)
const INTERIOR_EXACT: [u32; 2] = [0x858BBC, 0x858BB4];
/// "generic" (0x858A34), "CollisionFiles" (0x858BE4)
const STR_GENERIC: u32 = 0x858A34;
const STR_COLLISION_FILES: u32 = 0x858BE4;
/// "LOADCOL" (0x858B88)
const STR_LOADCOL: u32 = 0x858B88;

orig! {
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// CRT `_strnicmp` (0x835AC7)
    pub fn strnicmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x835AC7 => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `free` (0x8214BD)
    pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CPool<ColDef>::New` (0x411030)
    pub fn col_pool_new(pool: GPtr<ColPool>) -> GPtr<ColDef> = thiscall 0x411030 => col_pool_new_rust;
    /// `CPool<ColDef>::CPool(size, name)` (stub 0x410F80)
    pub fn col_pool_ctor(pool: GPtr<ColPool>, size: i32, name: GPtr<u8>) -> GPtr<ColPool> = thiscall 0x410F80 => unported;
    /// `CQuadTreeNode::operator new` (0x552C80)
    pub fn quad_tree_operator_new(size: u32) -> GPtr<u8> = cdecl 0x552C80 => unported;
    /// `CQuadTreeNode::CQuadTreeNode(const CRect&, int32 level)` (0x552830)
    pub fn quad_tree_ctor(node: GPtr<u8>, rect: GPtr<f32>, level: i32) -> GPtr<u8> = thiscall 0x552830 => unported;
    /// `CQuadTreeNode::~CQuadTreeNode` (0x552520)
    pub fn quad_tree_dtor(node: GPtr<u8>) = thiscall 0x552520 => unported;
    /// `CQuadTreeNode::operator delete` (0x552C90)
    pub fn quad_tree_operator_delete(p: GPtr<u8>) = cdecl 0x552C90 => unported;
    /// `CQuadTreeNode::ForAllMatching(const CVector2D&, cb)` (0x5529F0)
    pub fn quad_tree_for_all_matching(node: GPtr<u8>, v: GPtr<f32>, cb: u32) = thiscall 0x5529F0 => for_all_matching_vec;
    /// `CQuadTreeNode::AddItem(item, const CRect&)` (0x552CD0)
    pub fn quad_tree_add_item(node: GPtr<u8>, item: GPtr<ColDef>, rect: GPtr<ColDef>) = thiscall 0x552CD0 => add_item;
    /// `CQuadTreeNode::DeleteItem(item)` (0x552A40)
    pub fn quad_tree_delete_item(node: GPtr<u8>, item: GPtr<ColDef>) = thiscall 0x552A40 => delete_item;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, include_remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CStreaming::RequestModel(id, flags)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => crate::streaming::request_model;
    /// `CStreaming::RemoveModel(id)` (0x4089A0)
    pub fn remove_model(id: i32) = cdecl 0x4089A0 => crate::streaming::remove_model;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(only_priority: u8) = cdecl 0x40EA10 => crate::streaming::load_all_requested_models;
    /// `CFileLoader::LoadCollisionFile(const char*, uint8)` (0x5B4E60)
    pub fn load_collision_file_name(name: GPtr<u8>, slot: i32) = cdecl 0x5B4E60 => unported;
    /// `CFileLoader::LoadCollisionFile(uint8*, uint32, uint8)` (0x538440)
    pub fn load_collision_file_buf(data: GPtr<u8>, size: i32, slot: i32) -> u8 = cdecl 0x538440 => unported;
    /// `CFileLoader::LoadCollisionFileFirstTime(uint8*, uint32, uint8)` (0x5B5000)
    pub fn load_collision_file_first_time(data: GPtr<u8>, size: i32, slot: i32) -> u8 = cdecl 0x5B5000 => unported;
    /// `CColModel::RemoveCollisionVolumes` (0x40F9E0)
    pub fn col_model_remove_collision_volumes(cm: GPtr<u8>) = thiscall 0x40F9E0 => unported;
    /// `CColAccel::isCacheLoading` (0x5B2AC0)
    pub fn col_accel_is_cache_loading() -> u8 = cdecl 0x5B2AC0 => unported;
    /// `CColAccel::addColDef(ColDef)` (0x5B2DD0, the 0x2C-byte ColDef by value)
    pub fn col_accel_add_col_def(def: ColDef) = cdecl 0x5B2DD0 => unported;
    /// `CColAccel::getColDef(ColDef&)` (0x5B2E60)
    pub fn col_accel_get_col_def(def: GPtr<ColDef>) = cdecl 0x5B2E60 => unported;
    /// `CColAccel::cacheLoadCol` (0x5B2CC0)
    pub fn col_accel_cache_load_col() = cdecl 0x5B2CC0 => unported;
    /// `CColStore::LoadAllCollision` (0x410E60): LoadAllBoundingBoxes' `jmp`, through the hooked entry
    /// as in the original (a split-run L2 recording then sees the same nested call)
    pub fn load_all_collision_call() = cdecl 0x410E60 => load_all_collision;
    /// `CCamera::GetScreenFadeStatus` (0x50AE20, `TheCamera`)
    pub fn camera_get_screen_fade_status(cam: GPtr<u8>) -> i32 = thiscall 0x50AE20 => unported;
    /// `CMenuManager::MessageScreen(const char*, bool, bool)` (0x579330, `FrontEndMenuManager`)
    pub fn menu_message_screen(menu: GPtr<u8>, text: GPtr<u8>, black: u8, camera_update: u8) = thiscall 0x579330 => unported;
    /// `CTimer::Suspend` (0x5619D0)
    pub fn timer_suspend() = cdecl 0x5619D0 => sa_core::timer::suspend;
    /// `CTimer::Resume` (0x561A00)
    pub fn timer_resume() = cdecl 0x561A00 => sa_core::timer::resume;
    /// `CRect::IsPointInside(const CVector2D&, float tolerance)` (0x4042D0)
    pub fn rect_is_point_inside(rect: GPtr<ColDef>, pos: GPtr<f32>, tolerance: f32) -> u8 = thiscall 0x4042D0 => unported;
    /// CRT `operator delete[]` (0x8213AE)
    pub fn operator_delete_array(p: GPtr<u8>) = cdecl 0x8213AE => unported;
}

/// `TheCamera` (0xB6F028), `FrontEndMenuManager` (0xBA6748)
const THE_CAMERA: usize = 0xB6F028;
const FRONT_END_MENU_MANAGER: usize = 0xBA6748;

pub fn for_all_matching_vec(node: GPtr<u8>, v: GPtr<f32>, cb: u32) {
    sa_core::quadtree::for_all_matching_vec(node, v.cast(), cb)
}
pub fn add_item(node: GPtr<u8>, item: GPtr<ColDef>, rect: GPtr<ColDef>) {
    sa_core::quadtree::add_item(node, item.cast(), rect.cast())
}
pub fn delete_item(node: GPtr<u8>, item: GPtr<ColDef>) {
    sa_core::quadtree::delete_item(node, item.cast())
}
pub fn col_pool_new_rust(pool: GPtr<ColPool>) -> GPtr<ColDef> {
    sa_core::pool::pool_new(pool)
}

/// `CPool<ColDef>::GetAt(i)`: the slot, or null when the map byte has bit 7 set (`cmp byte, 0; jl`).
pub fn get_at(slot: i32) -> GPtr<ColDef> {
    let pool: CPool<ColDef> = COL_POOL.get().read();
    if (pool.byte_map.wrapping_index(slot).read() as i8) < 0 {
        GPtr::NULL
    } else {
        pool.objects.wrapping_index(slot)
    }
}

/// The slot `i` of the pool with the original's iteration test (used, non-null).
fn used_slot(i: i32) -> GPtr<ColDef> {
    get_at(i)
}

const AREA: isize = 0x00;
const MODEL_START: isize = 0x22;
const MODEL_END: isize = 0x24;
const REF_COUNT: isize = 0x26;
const ACTIVE: isize = 0x28;
const REQUIRED: isize = 0x29;
const PROCEDURAL_OFF: isize = 0x2A;
const INTERIOR: isize = 0x2B;

fn byte_at(p: GPtr<ColDef>, off: isize) -> GPtr<u8> {
    field(p, off)
}
fn f_at(p: GPtr<ColDef>, off: isize) -> f32 {
    field::<f32, ColDef>(p, off).read()
}

/// `fld a; fcomp b; fnstsw; test ah, 5; jnp skip`: proceed when `a >= b` or unordered.
fn ge_or_nan(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}
/// `fld a; fcomp b; fnstsw; test ah, 0x41; je skip`: proceed when `a <= b` or unordered.
fn le_or_nan(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}
fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `CColStore::FindColSlot(const char*)` (0x410390): `or eax, -1; ret` (the name is never read).
/// Not hookable: `or eax,-1; ret` is 4 bytes.
pub fn find_col_slot(_name: GPtr<u8>) -> i32 {
    -1
}

/// `CColStore::AddCollisionNeededAtPosn(const CVector&)` (0x4103A0 → 0x15672B0): stores x, y, the
/// flag, then z (the original's order).
#[hook(addr = 0x4103A0, conv = cdecl, name = "CColStore::AddCollisionNeededAtPosn", expect = "E9 0B 6F 15 01")]
pub fn add_collision_needed_at_posn(pos: GPtr<f32>) {
    let p = COLLISION_NEEDED_POS.ptr().cast::<f32>();
    p.write(pos.read());
    p.wrapping_index(1).write(pos.wrapping_index(1).read());
    COLLISION_NEEDED.set(GBool::TRUE);
    p.wrapping_index(2).write(pos.wrapping_index(2).read());
}

/// `SetIfCollisionIsRequired(const CVector2D&, ColDef*)` (0x4103D0, quad tree callback): with
/// `ms_EntityAreaCode == 0` an interior def is skipped; then `left <= x <= right`,
/// `[+0xC] <= y <= [+4]` (x87 compares, NaN passes) → `m_bCollisionIsRequired = 1`.
#[hook(addr = 0x4103D0, conv = cdecl, name = "CColStore::SetIfCollisionIsRequired", expect = "A1 54 55 96 00")]
pub fn set_if_collision_is_required(pos: GPtr<f32>, def: GPtr<ColDef>) {
    set_if_collision_is_required_with(ENTITY_AREA_CODE.get(), pos, def)
}

/// [`set_if_collision_is_required`] with `ms_EntityAreaCode` passed in (pure: L1).
pub fn set_if_collision_is_required_with(area: i32, pos: GPtr<f32>, def: GPtr<ColDef>) {
    if area == 0 && byte_at(def, INTERIOR).read() != 0 {
        return;
    }
    let (px, py) = (pos.read(), pos.wrapping_index(1).read());
    if ge_or_nan(x(px), x(f_at(def, 0x0)))
        && le_or_nan(x(px), x(f_at(def, 0x8)))
        && ge_or_nan(x(py), x(f_at(def, 0xC)))
        && le_or_nan(x(py), x(f_at(def, 0x4)))
    {
        byte_at(def, REQUIRED).write(1);
    }
}

/// `SetIfCollisionIsRequiredReducedBB(const CVector2D&, ColDef*)` (0x410470): the rect tests first
/// (`left - (-80) <= x`, `x <= right - 80`, `[+0xC] - (-80) <= y`, `y <= [+4] - 80`, x87), then
/// `(ms_EntityAreaCode != 0) == (interior != 0)` → required.
#[hook(addr = 0x410470, conv = cdecl, name = "CColStore::SetIfCollisionIsRequiredReducedBB", expect = "8B 4C 24 08 D9 01")]
pub fn set_if_collision_is_required_reduced_bb(pos: GPtr<f32>, def: GPtr<ColDef>) {
    set_if_collision_is_required_reduced_bb_with(ENTITY_AREA_CODE.get(), pos, def)
}

/// [`set_if_collision_is_required_reduced_bb`] with `ms_EntityAreaCode` passed in (pure: L1).
pub fn set_if_collision_is_required_reduced_bb_with(area: i32, pos: GPtr<f32>, def: GPtr<ColDef>) {
    let (px, py) = (pos.read(), pos.wrapping_index(1).read());
    // fld [def]; fsub m; fcomp [pos]; test ah,0x41; je skip  → proceed when (def - m) <= pos or NaN
    if !le_or_nan(x(f_at(def, 0x0)).fsub_m(k(F_M80)), x(px)) {
        return;
    }
    // fld [def+8]; fsub 80; fcomp [pos]; test ah,5; jnp skip → proceed when (def+8 - 80) >= pos or NaN
    if !ge_or_nan(x(f_at(def, 0x8)).fsub_m(k(F_80)), x(px)) {
        return;
    }
    if !le_or_nan(x(f_at(def, 0xC)).fsub_m(k(F_M80)), x(py)) {
        return;
    }
    if !ge_or_nan(x(f_at(def, 0x4)).fsub_m(k(F_80)), x(py)) {
        return;
    }
    let interior = byte_at(def, INTERIOR).read() != 0;
    if (area != 0) == interior {
        byte_at(def, REQUIRED).write(1);
    }
}

/// `inc/dec word [GetAt(slot) + 0x26]` without a null check (a free slot writes at address 0x26).
fn add_to_ref(slot: i32, d: i16) {
    let p: GPtr<i16> = field(get_at(slot), REF_COUNT);
    p.write(p.read().wrapping_add(d));
}

/// `CColStore::AddRef(int32)` (0x4107A0 → 0x1569960).
#[hook(addr = 0x4107A0, conv = cdecl, name = "CColStore::AddRef", expect = "E9 BB 91 15 01")]
pub fn add_ref(slot: i32) {
    add_to_ref(slot, 1)
}

/// `CColStore::RemoveRef(int32)` (0x4107D0 → 0x156DBE0).
#[hook(addr = 0x4107D0, conv = cdecl, name = "CColStore::RemoveRef", expect = "E9 0B D4 15 01")]
pub fn remove_ref(slot: i32) {
    add_to_ref(slot, -1)
}

/// `CColStore::GetBoundingBox(int32)` (0x410800 → 0x1565240): `&GetAt(slot)->m_Area` (0 when free).
#[hook(addr = 0x410800, conv = cdecl, name = "CColStore::GetBoundingBox", expect = "E9 3B 4A 15 01")]
pub fn get_bounding_box(slot: i32) -> GPtr<ColDef> {
    get_at(slot)
}

/// `CColStore::IncludeModelIndex(int32 slot, int32 model)` (0x410820 → 0x156A260): signed 16-bit
/// compares against the sign-extended start/end, word stores of the truncated model id.
#[hook(addr = 0x410820, conv = cdecl, name = "CColStore::IncludeModelIndex", expect = "E9 3B 9A 15 01")]
pub fn include_model_index(slot: i32, model: i32) {
    let def = get_at(slot);
    let start: GPtr<i16> = field(def, MODEL_START);
    if model < start.read() as i32 {
        start.write(model as i16);
    }
    let end: GPtr<i16> = field(def, MODEL_END);
    if (end.read() as i32) < model {
        end.write(model as i16);
    }
}

/// `CColStore::AddColSlot(const char*)` (0x411140 → 0x1563290): a new pool entry with the flags
/// cleared, area words (+0 = +0xC = 1e6, +8 = +4 = -1e6, stored in that order), model range
/// (0x7FFF, 0x8000),
/// ref count 0, then the procedural and interior name tests. The name is not stored (see the
/// module docs). Returns the slot index (`(def - objects) / 0x2C`, MSVC magic division).
#[hook(addr = 0x411140, conv = cdecl, name = "CColStore::AddColSlot", expect = "E9 4B 21 15 01")]
pub fn add_col_slot(name: GPtr<u8>) -> i32 {
    let def = col_pool_new(COL_POOL.get());
    byte_at(def, ACTIVE).write(0);
    byte_at(def, REQUIRED).write(0);
    byte_at(def, PROCEDURAL_OFF).write(0);
    byte_at(def, INTERIOR).write(0);
    field::<u32, ColDef>(def, 0x0).write(0x4974_2400);
    field::<u32, ColDef>(def, 0xC).write(0x4974_2400);
    field::<u32, ColDef>(def, 0x8).write(0xC974_2400);
    field::<u32, ColDef>(def, 0x4).write(0xC974_2400);
    field::<u16, ColDef>(def, MODEL_START).write(0x7FFF);
    field::<u16, ColDef>(def, MODEL_END).write(0x8000);
    field::<u16, ColDef>(def, REF_COUNT).write(0);
    let s = |a: u32| GPtr::<u8>::from_addr(a as usize);
    if PROCEDURAL.iter().any(|&p| stricmp(name, s(p)) == 0) {
        byte_at(def, PROCEDURAL_OFF).write(1);
    }
    let mut interior = false;
    for (i, &(p, n)) in INTERIOR_PREFIXES.iter().enumerate() {
        if i == 11 {
            // "props" / "props2" (exact) come before "levelmap"
            if INTERIOR_EXACT.iter().any(|&e| stricmp(name, s(e)) == 0) {
                interior = true;
                break;
            }
        }
        if strnicmp(name, s(p), n) == 0 {
            interior = true;
            break;
        }
    }
    if interior {
        byte_at(def, INTERIOR).write(1);
    }
    let objects = COL_POOL.get().read().objects;
    def.index_from(objects)
}

/// `CColStore::SetCollisionRequired(const CVector&, int32 area)` (0x4104E0 → 0x156CF50): area -1 =
/// the player's area code (`FindPlayerPed(-1)`) or `CGame::currArea` (read through a protection
/// cell that decodes to 0xB72914); stored to `ms_EntityAreaCode`; then the quad tree is walked at
/// `(x, y)` with `SetIfCollisionIsRequired` (area == currArea) or the reduced-BB variant.
#[hook(addr = 0x4104E0, conv = cdecl, name = "CColStore::SetCollisionRequired", expect = "E9 6B CA 15 01")]
pub fn set_collision_required(pos: GPtr<f32>, area: i32) {
    let mut area = area;
    if area == -1 {
        let ped = find_player_ped(-1);
        area = if ped.is_null() {
            sa_core::game::CURR_AREA.get()
        } else {
            field::<u8, u8>(ped, 0x2F).read() as i32
        };
    }
    for_all_with_area(pos.read(), pos.wrapping_index(1).read(), area);
}

/// The common tail: `ms_EntityAreaCode = area`, copy (x, y) to a local, `ForAllMatching`.
fn for_all_with_area(px: f32, py: f32, area: i32) {
    let same = area == sa_core::game::CURR_AREA.get();
    ENTITY_AREA_CODE.set(area);
    let mut v = [px, py];
    let cb = if same { 0x4103D0 } else { 0x410470 };
    quad_tree_for_all_matching(QUAD_TREE.get(), GPtr::from_raw(v.as_mut_ptr()), cb);
}

/// `IsPointInside(pos, -110)` as inlined in RequestCollision/HasCollisionLoaded (x87).
fn inside_110(def: GPtr<ColDef>, px: f32, py: f32) -> bool {
    le_or_nan(x(f_at(def, 0x0)).fsub_m(k(F_M110)), x(px))
        && ge_or_nan(x(f_at(def, 0x8)).fsub_m(k(F_110)), x(px))
        && le_or_nan(x(f_at(def, 0xC)).fsub_m(k(F_M110)), x(py))
        && ge_or_nan(x(f_at(def, 0x4)).fsub_m(k(F_110)), x(py))
}

/// `CColStore::RequestCollision(const CVector&, int32 area)` (0x410C00).
#[hook(addr = 0x410C00, conv = cdecl, name = "CColStore::RequestCollision", expect = "8B 44 24 08 83 EC 08")]
pub fn request_collision(pos: GPtr<f32>, area: i32) {
    set_collision_required(pos, area);
    for i in 1..255 {
        let def = used_slot(i);
        if def.is_null() || byte_at(def, REQUIRED).read() == 0 {
            continue;
        }
        if inside_110(def, pos.read(), pos.wrapping_index(1).read()) {
            request_model(i + RESOURCE_ID_COL, 0x18);
        }
        byte_at(def, REQUIRED).write(0);
    }
}

/// `CColStore::HasCollisionLoaded(const CVector&, int32 area)` (0x410CE0, AL): the first required
/// def (flag cleared) containing the point (-110) that is not active clears the flags of every used
/// slot from it on and returns false; otherwise true.
#[hook(addr = 0x410CE0, conv = cdecl, name = "CColStore::HasCollisionLoaded", expect = "8B 44 24 08 83 EC 08")]
pub fn has_collision_loaded(pos: GPtr<f32>, area: i32) -> bool {
    set_collision_required(pos, area);
    for i in 1..255 {
        let def = used_slot(i);
        if def.is_null() || byte_at(def, REQUIRED).read() == 0 {
            continue;
        }
        byte_at(def, REQUIRED).write(0);
        if inside_110(def, pos.read(), pos.wrapping_index(1).read()) && byte_at(def, ACTIVE).read() == 0 {
            for j in i..255 {
                let d = used_slot(j);
                if !d.is_null() {
                    byte_at(d, REQUIRED).write(0);
                }
            }
            return false;
        }
    }
    true
}

/// `CColStore::LoadCol(int32 slot, const char* file)` (0x410690): `GetAt` first, then
/// `CFileLoader::LoadCollisionFile(file, slot)`, then `m_bActive = 1` (no null check).
#[hook(addr = 0x410690, conv = cdecl, name = "CColStore::LoadCol-", expect = "8B 0D 60 55 96 00")]
pub fn load_col_file(slot: i32, file: GPtr<u8>) {
    let def = get_at(slot);
    load_collision_file_name(file, slot);
    byte_at(def, ACTIVE).write(1);
}

/// `CColStore::LoadCol(int32 slot, uint8* data, int32 size)` (0x4106D0 → 0x15613A0, AL): first-time
/// load when `start > end` (signed words), else the normal load; success sets `m_bActive`.
#[hook(addr = 0x4106D0, conv = cdecl, name = "CColStore::LoadCol-2", expect = "E9 CB 0C 15 01")]
pub fn load_col_buffer(slot: i32, data: GPtr<u8>, size: i32) -> bool {
    let def = get_at(slot);
    let start = field::<i16, ColDef>(def, MODEL_START).read();
    let end = field::<i16, ColDef>(def, MODEL_END).read();
    let ok = if end < start {
        load_collision_file_first_time(data, size, slot)
    } else {
        load_collision_file_buf(data, size, slot)
    };
    if ok != 0 {
        byte_at(def, ACTIVE).write(1);
        true
    } else {
        false
    }
}

/// `CColStore::RemoveCol(int32)` (0x410730 → 0x1564E90): `m_bActive = 0`, then for every model id in
/// `[start, end]` (end re-read each iteration) whose model info has a col model, is a LOD (flags
/// byte +0x12 bit 7) and whose col model's slot byte (+0x28) equals the slot (compared as `uint8` with
/// the full 32-bit slot): `RemoveCollisionVolumes`.
#[hook(addr = 0x410730, conv = cdecl, name = "CColStore::RemoveCol", expect = "E9 5B 47 15 01")]
pub fn remove_col(slot: i32) {
    let def = get_at(slot);
    let mut i = field::<i16, ColDef>(def, MODEL_START).read() as i32;
    byte_at(def, ACTIVE).write(0);
    while i <= field::<i16, ColDef>(def, MODEL_END).read() as i32 {
        let mi = model_info(i);
        if !mi.is_null() {
            let cm = field::<GPtr<u8>, _>(mi, 0x14).read();
            if !cm.is_null()
                && (field::<u8, _>(mi, 0x12).read() as i8) < 0
                && field::<u8, u8>(cm, 0x28).read() as u32 == slot as u32
            {
                col_model_remove_collision_volumes(cm);
            }
        }
        i += 1;
    }
}

/// `CColStore::RemoveColSlot(int32)` (0x411330 → 0x1567210): `RemoveCol` when active, quad tree
/// `DeleteItem(GetAt(slot))`, then the inlined `CPool::Delete(GetAt(slot))`: map byte `|= 0x80`,
/// `first_free = min(first_free, idx)` (signed).
#[hook(addr = 0x411330, conv = cdecl, name = "CColStore::RemoveColSlot", expect = "E9 DB 5E 15 01")]
pub fn remove_col_slot(slot: i32) {
    if byte_at(get_at(slot), ACTIVE).read() != 0 {
        remove_col(slot);
    }
    quad_tree_delete_item(QUAD_TREE.get(), get_at(slot));
    let pool_ptr = COL_POOL.get();
    let def = get_at(slot);
    let pool = pool_ptr.read();
    let idx = def.index_from(pool.objects);
    let m = pool.byte_map.wrapping_index(idx);
    m.write(m.read() | 0x80);
    let ff: GPtr<i32> = field(pool_ptr, 0x0C);
    if idx < ff.read() {
        ff.write(idx);
    }
}

/// `CColStore::Initialise()` (0x4113F0): `ms_bCollisionNeeded = 0`; pool `new CPool<ColDef>(255,
/// "CollisionFiles")` when none; `AddColSlot("generic")`; `ms_pQuadTree = new CQuadTreeNode(rect, 3)`
/// with the stack rect {-3000, 3000, 3000, -3000} (left, bottom, right, top as stored).
#[hook(addr = 0x4113F0, conv = cdecl, name = "CColStore::Initialise", expect = "6A FF 68 A6 71 83 00")]
pub fn initialise() {
    let mut rect: [u32; 4] = [0xC53B_8000, 0x453B_8000, 0x453B_8000, 0xC53B_8000];
    COLLISION_NEEDED.set(GBool::FALSE);
    if COL_POOL.get().is_null() {
        let p = operator_new(0x14);
        let pool = if p.is_null() {
            GPtr::NULL
        } else {
            col_pool_ctor(p.cast(), 0xFF, GPtr::from_addr(STR_COLLISION_FILES as usize))
        };
        COL_POOL.set(pool);
    }
    add_col_slot(GPtr::from_addr(STR_GENERIC as usize));
    let n = quad_tree_operator_new(0x28);
    let tree = if n.is_null() {
        GPtr::NULL
    } else {
        quad_tree_ctor(n, GPtr::from_raw(rect.as_mut_ptr()).cast(), 3)
    };
    QUAD_TREE.set(tree);
}

/// `CColStore::Shutdown()` (0x4114D0): `RemoveColSlot(i)` for used slots 0..=254 (ascending), the
/// inlined pool destructor (`delete[]` objects and map when `size > 0` and `+0x10` is set, then zeroes
/// the four fields; `free` of the pool), `ms_pColPool = 0`; quad tree destructor + delete, `ms_pQuadTree = 0`.
#[hook(addr = 0x4114D0, conv = cdecl, name = "CColStore::Shutdown", expect = "51 A1 60 55 96 00")]
pub fn shutdown() {
    for i in 0..255 {
        if !get_at(i).is_null() {
            remove_col_slot(i);
        }
    }
    let pool = COL_POOL.get();
    if !pool.is_null() {
        let pv = pool.read();
        if pv.size > 0 {
            if pv.owns_arrays.get() {
                operator_delete_array(pv.objects.cast());
                operator_delete_array(field::<GPtr<u8>, _>(pool, 0x4).read());
            }
            field::<u32, _>(pool, 0x0).write(0);
            field::<u32, _>(pool, 0x4).write(0);
            field::<u32, _>(pool, 0x8).write(0);
            field::<u32, _>(pool, 0xC).write(0);
        }
        free(pool.cast());
    }
    let tree = QUAD_TREE.get();
    COL_POOL.set(GPtr::NULL);
    if !tree.is_null() {
        quad_tree_dtor(tree);
        quad_tree_operator_delete(tree);
    }
    QUAD_TREE.set(GPtr::NULL);
}

/// `CColStore::LoadCollision(CVector pos, bool ignorePlayerVeh)` (0x410860; entry `jmp 0x401ECF` reads
/// `CStreaming::ms_disableStreaming` and jumps back to 0x410865). See gta-reversed for the shape; the
/// original: player vehicle speed added as `pos += speed * 20` (x87, x and y only); mission cleanup
/// entities (0xA90850, 75 × 8 bytes) of type 1 (vehicle: not wrecked) / 2 (ped: state not 0x36/0x37)
/// whose physical flags (+0x40) have neither bit 13 nor 14 set mark their area with the reduced-BB
/// callback; then per used slot 1..=254 request (0x18) or remove; `ms_bCollisionNeeded = 0` unless
/// `ignorePlayerVeh`.
#[hook(addr = 0x410860, conv = cdecl, name = "CColStore::LoadCollision", expect = "E9 6A 16 FF FF 83")]
pub fn load_collision(px: f32, py: f32, pz: f32, ignore_player_veh: u8) {
    let _ = pz;
    if crate::streaming::DISABLE_STREAMING.get().get() {
        return;
    }
    let (mut px, mut py) = (px, py);
    if !find_player_ped(-1).is_null() && ignore_player_veh == 0 {
        let veh = find_player_vehicle(-1, 0);
        if !veh.is_null() {
            let sp = veh.byte_add(0x44).cast::<f32>();
            px = x(sp.read()).fmul_m(k(F_20)).fadd_m(px).to_f32();
            py = x(sp.wrapping_index(1).read()).fmul_m(k(F_20)).fadd_m(py).to_f32();
        }
    }
    let ped = find_player_ped(-1);
    let area = if ped.is_null() {
        sa_core::game::CURR_AREA.get()
    } else {
        field::<u8, u8>(ped, 0x2F).read() as i32
    };
    for_all_with_area(px, py, area);
    if COLLISION_NEEDED.get().get() {
        set_collision_required(COLLISION_NEEDED_POS.ptr().cast(), sa_core::game::CURR_AREA.get());
    }
    for n in 0..MISSION_CLEANUP_COUNT {
        let o = GPtr::<u8>::from_addr(MISSION_CLEANUP_OBJECTS + 8 * n);
        let kind = o.read();
        let handle = field::<i32, u8>(o, 4).read();
        let e = match kind {
            1 => {
                let pool = sa_core::pool::VEHICLE_POOL.get().read();
                let idx = handle >> 8;
                if pool.byte_map.wrapping_index(idx).read() != handle as u8 {
                    continue;
                }
                let v = pool.objects.cast::<u8>().byte_add((idx.wrapping_mul(0xA18)) as isize);
                if v.is_null() || field::<u8, u8>(v, 0x36).read() & 0xF8 == 0x28 {
                    continue;
                }
                v
            }
            2 => {
                let pool = sa_core::pool::PED_POOL.get().read();
                let idx = handle >> 8;
                if pool.byte_map.wrapping_index(idx).read() != handle as u8 {
                    continue;
                }
                let p = pool.objects.cast::<u8>().byte_add((idx.wrapping_mul(0x7C4)) as isize);
                if p.is_null() {
                    continue;
                }
                let st = field::<i32, u8>(p, 0x530).read();
                if st == 0x36 || st == 0x37 {
                    continue;
                }
                p
            }
            _ => continue,
        };
        if field::<u32, u8>(e, 0x40).read() & 0x6000 != 0 {
            continue;
        }
        ENTITY_AREA_CODE.set(field::<u8, u8>(e, 0x2F).read() as i32);
        let m = field::<GPtr<u8>, u8>(e, 0x14).read();
        let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
        let mut v = [field::<f32, u8>(pos, 0).read(), field::<f32, u8>(pos, 4).read()];
        quad_tree_for_all_matching(QUAD_TREE.get(), GPtr::from_raw(v.as_mut_ptr()), 0x410470);
    }
    for i in 1..255 {
        let def = used_slot(i);
        if def.is_null() {
            continue;
        }
        if byte_at(def, REQUIRED).read() != 0
            || byte_at(def, PROCEDURAL_OFF).read() != 0
            || field::<i16, ColDef>(def, REF_COUNT).read() > 0
        {
            if byte_at(def, ACTIVE).read() == 0 {
                request_model(i + RESOURCE_ID_COL, 0x18);
            }
            byte_at(def, REQUIRED).write(0);
        } else if byte_at(def, ACTIVE).read() != 0 {
            remove_model(i + RESOURCE_ID_COL);
        }
    }
    if ignore_player_veh == 0 {
        COLLISION_NEEDED.set(GBool::FALSE);
    }
}

/// `CColStore::RemoveAllCollision()` (0x410E00): `RemoveModel` for used slots whose streaming flags
/// have neither GAME_REQUIRED nor MISSION_REQUIRED (`& 6`).
#[hook(addr = 0x410E00, conv = cdecl, name = "CColStore::RemoveAllCollision", expect = "51 A1 60 55 96 00")]
pub fn remove_all_collision() {
    for i in 1..255 {
        if !used_slot(i).is_null() && info(i + RESOURCE_ID_COL).read().flags & 6 == 0 {
            remove_model(i + RESOURCE_ID_COL);
        }
    }
}

/// `CColStore::LoadAllCollision()` (0x410E60): request (flags 0), load all, remove, per used slot.
#[hook(addr = 0x410E60, conv = cdecl, name = "CColStore::LoadAllCollision", expect = "51 A1 60 55 96 00")]
pub fn load_all_collision() {
    for i in 1..255 {
        if !used_slot(i).is_null() {
            request_model(i + RESOURCE_ID_COL, 0);
            load_all_requested_models(0);
            remove_model(i + RESOURCE_ID_COL);
        }
    }
}

/// `CColStore::LoadAllBoundingBoxes()` (0x4113D0): `CColAccel::cacheLoadCol()` when the cache is
/// loading, else `LoadAllCollision()`.
#[hook(addr = 0x4113D0, conv = cdecl, name = "CColStore::LoadAllBoundingBoxes", expect = "E8 EB 16 1A 00")]
pub fn load_all_bounding_boxes() {
    if col_accel_is_cache_loading() != 0 {
        col_accel_cache_load_col();
    } else {
        load_all_collision_call();
    }
}

/// `CColStore::BoundingBoxesPostProcess()` (0x410EC0): per used slot 1..=254: from the cache
/// (`getColDef`), or grow the area by 120 (left -, right +, [+0xC] -, [+4] +; x87 then `fstp`) and
/// `addColDef(*def)` (by value); then `ms_pQuadTree->AddItem(def, &def->m_Area)`.
#[hook(addr = 0x410EC0, conv = cdecl, name = "CColStore::BoundingBoxesPostProcess", expect = "51 A1 60 55 96 00")]
pub fn bounding_boxes_post_process() {
    for i in 1..255 {
        let def = used_slot(i);
        if def.is_null() {
            continue;
        }
        if col_accel_is_cache_loading() != 0 {
            col_accel_get_col_def(def);
        } else {
            let f = |off: isize| field::<f32, ColDef>(def, off);
            f(0x0).write(x(f(0x0).read()).fsub_m(k(F_120)).to_f32());
            f(0x8).write(x(f(0x8).read()).fadd_m(k(F_120)).to_f32());
            f(0xC).write(x(f(0xC).read()).fsub_m(k(F_120)).to_f32());
            f(0x4).write(x(f(0x4).read()).fadd_m(k(F_120)).to_f32());
            col_accel_add_col_def(def.read());
        }
        quad_tree_add_item(QUAD_TREE.get(), def, def.byte_add(AREA));
    }
}

/// `CColStore::EnsureCollisionIsInMemory(const CVector&)` (0x410AD0): only when streaming is enabled
/// and the player's area (or currArea) equals currArea; per required used slot: when
/// `CRect::IsPointInside(pos, -110)` (called, 0x4042D0) and the COL model is not loaded: request
/// (0x18), "LOADCOL" message screen when the fade status is 0, `CTimer::Suspend`,
/// `LoadAllRequestedModels(true)`, `CTimer::Resume`; the required flag is cleared.
#[hook(addr = 0x410AD0, conv = cdecl, name = "CColStore::EnsureCollisionIsInMemory", expect = "A0 B0 54 96 00")]
pub fn ensure_collision_is_in_memory(pos: GPtr<f32>) {
    if crate::streaming::DISABLE_STREAMING.get().get() {
        return;
    }
    let mut area = sa_core::game::CURR_AREA.get();
    if !find_player_ped(-1).is_null() {
        area = field::<u8, u8>(find_player_ped(-1), 0x2F).read() as i32;
    }
    if area != sa_core::game::CURR_AREA.get() {
        return;
    }
    set_collision_required(pos, -1);
    for i in 1..255 {
        let def = used_slot(i);
        if def.is_null() || byte_at(def, REQUIRED).read() == 0 {
            continue;
        }
        let mut v = [pos.read(), pos.wrapping_index(1).read()];
        if rect_is_point_inside(def, GPtr::from_raw(v.as_mut_ptr()), k(F_M110)) != 0
            && info(i + RESOURCE_ID_COL).read().load_state != 1
        {
            request_model(i + RESOURCE_ID_COL, 0x18);
            if camera_get_screen_fade_status(GPtr::from_addr(THE_CAMERA)) == 0 {
                menu_message_screen(GPtr::from_addr(FRONT_END_MENU_MANAGER), GPtr::from_addr(STR_LOADCOL as usize), 0, 0);
            }
            timer_suspend();
            load_all_requested_models(1);
            timer_resume();
        }
        byte_at(def, REQUIRED).write(0);
    }
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_COLLISION_NEEDED_AT_POSN,
    HOOK_SET_IF_COLLISION_IS_REQUIRED,
    HOOK_SET_IF_COLLISION_IS_REQUIRED_REDUCED_BB,
    HOOK_ADD_REF,
    HOOK_REMOVE_REF,
    HOOK_GET_BOUNDING_BOX,
    HOOK_INCLUDE_MODEL_INDEX,
    HOOK_ADD_COL_SLOT,
    HOOK_SET_COLLISION_REQUIRED,
    HOOK_REQUEST_COLLISION,
    HOOK_HAS_COLLISION_LOADED,
    HOOK_LOAD_COL_FILE,
    HOOK_LOAD_COL_BUFFER,
    HOOK_REMOVE_COL,
    HOOK_REMOVE_COL_SLOT,
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_LOAD_COLLISION,
    HOOK_REMOVE_ALL_COLLISION,
    HOOK_LOAD_ALL_COLLISION,
    HOOK_LOAD_ALL_BOUNDING_BOXES,
    HOOK_BOUNDING_BOXES_POST_PROCESS,
    HOOK_ENSURE_COLLISION_IS_IN_MEMORY,
];
