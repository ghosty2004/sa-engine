//! `CIplStore` (0x4045B0–0x406355, 0x5D5420, 0x5D54A0): the IPL definitions (`CPool<IplDef>`, 256
//! slots), their quad tree, and the streamed IPL loaders.
//!
//! Ported from the 1.0 US bytes (gta-reversed `IplStore.cpp` as reading aid,
//! `docs/re/specs/STREAM/CIplStore.deviations.md`). Kept original behaviour, beyond the spec's notes:
//! - pool lookups are not null-checked where the original writes or reads through the slot
//!   (SetIsInterior, EnableDynamicStreaming, Request/RemoveIplAndIgnore, RemoveIplWhenFarAway,
//!   RemoveIpl, RemoveIplSlot, LoadIpl, LoadIplBoundingBox: [`null_deref`]); `GetIplName` of a free
//!   slot returns the address 0x10 (`&((IplDef*)0)->name`), `GetBoundingBox` returns 0;
//! - *new*: `RequestIpls` and `HaveIplsLoaded` call `SetIplsRequired(posn, arg2)` first (gr does
//!   not); `EnsureIplsAreInMemory` clears `loadRequested` of every requested slot, also when the
//!   point is outside or the IPL is loaded (gr only when it requested it); `LoadIpls` walks the slots
//!   upwards 1..255 (gr downwards from size-1) and mission entities through the inlined
//!   `GetAtRef`s; `Save` writes the constant 256 as the count and `Load` ignores the stored count
//!   (both fixed 1..255; gr uses the pool size / the stored count);
//! - `IncludeEntity` compares the full int32 pool index with the sign-extended i16 bounds and
//!   ignores other entity types (gr-deviations `IplStore.cpp:281`, keep-original);
//! - `RemoveIpl`'s loops are inclusive of `last` (spec) and tail-jump into
//!   `CTheCarGenerators::RemoveCarGenerators` with the int32 argument;
//! - `SetupRelatedIpls` copies the file name into a 32-byte stack buffer followed by the 0x34-byte
//!   `getIplDef` temporary; a longer name runs into the temporary (reproduced: one contiguous buffer)
//!   and beyond it the original's stack is smashed (original UB, panic here);
//! - the binary IPL loaders relocate the instance (and car generator) offsets in the streaming
//!   buffer in place.

use sa_core::mem::GPtr;
use sa_core::pool::{self, CPool, PED_POOL, VEHICLE_POOL, OBJECT_POOL};
use sa_core::pools::{BUILDING_POOL, DUMMY_POOL};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::shared::{IPL_POOL, IplDef, RESOURCE_ID_IPL, field, image, info, model_info, nonnull, null_deref};

sa_core::globals! {
    list GLOBALS;
    /// `CIplStore::ms_currentIPLAreaCode`
    pub static CURRENT_IPL_AREA_CODE @ 0x8E3EF8: i32 = 0;
    /// `ppCurrIplInstance`
    pub static CURR_IPL_INSTANCE @ 0x8E3EFC: GPtr<GPtr<u8>> = GPtr::NULL;
    /// `NumIplEntityIndexArrays`
    pub static NUM_IPL_ENTITY_INDEX_ARRAYS @ 0x8E3F00: i32 = 0;
    /// `IplEntityIndexArrays[40]`
    pub static IPL_ENTITY_INDEX_ARRAYS @ 0x8E3F08: [GPtr<GPtr<u8>>; 40] = [GPtr::NULL; 40];
    /// `gbIplsNeededAtPosn`
    pub static IPLS_NEEDED_AT_POSN @ 0x8E3FA8: u8 = 0;
    /// `CIplStore::ms_pQuadTree`
    pub static QUAD_TREE @ 0x8E3FAC: GPtr<u8> = GPtr::NULL;
    /// `gvecIplsNeededAtPosn`
    pub static VEC_IPLS_NEEDED_AT_POSN @ 0x8E3FD0: [f32; 3] = [0.0; 3];
}

/// `CStreaming::ms_disableStreaming` (owner: streaming)
fn disable_streaming() -> bool {
    crate::streaming::DISABLE_STREAMING.get().get()
}

/// Callbacks passed by address to `CQuadTreeNode::ForAllMatching`.
pub const CB_SET_IF_INTERIOR_IPL_IS_REQUIRED: u32 = 0x4045F0;
pub const CB_SET_IF_IPL_IS_REQUIRED: u32 = 0x404660;
pub const CB_SET_IF_IPL_IS_REQUIRED_REDUCED_BB: u32 = 0x404690;
/// `CTheScripts::MissionCleanUp` entries: { u8 type; int32 handle } x 75
pub const MISSION_CLEANUP: u32 = 0xA90850;
/// "bnry" (0x858A2C), "generic" (0x858A34), "IPL Files" (0x858A3C), "LOADCOL" (0x858A18)
pub const STR_BNRY: u32 = 0x858A2C;
pub const STR_GENERIC: u32 = 0x858A34;
pub const STR_IPL_FILES: u32 = 0x858A3C;
pub const STR_LOADCOL: u32 = 0x858A18;
/// Interior IPL names compared by SetupRelatedIpls (`_stricmp`, in the original's order).
pub const INTERIOR_NAMES: [u32; 13] = [
    0x858A04, 0x8589F8, 0x8589EC, 0x8589E0, 0x8589D4, 0x8589C8, 0x8589BC, 0x8589B4, 0x8589AC,
    0x8589A4, 0x85899C, 0x858990, 0x858984,
];
/// "_stream" as two dwords (0x85897C, 0x858980)
pub const STR_STREAM_SUFFIX: u32 = 0x85897C;
const THE_CAMERA: u32 = 0xB6F028;
const FRONT_END_MENU_MANAGER: u32 = 0xBA6748;

orig! {
    /// `CIplPool::New()` (0x4059B0, UNK-game)
    pub fn ipl_pool_new(p: GPtr<CPool<IplDef>>) -> GPtr<IplDef> = thiscall 0x4059B0 => pool::pool_new;
    /// `CIplDefPool::CPool(size, name)` (0x405900, sa-core)
    pub fn ipl_pool_ctor(p: GPtr<u8>, size: i32, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x405900 => unported;
    /// `CRect::IsPointInside(const CVector2D&, float tolerance)` (0x4042D0, AL)
    pub fn rect_is_point_inside_tol(r: GPtr<u8>, p: GPtr<u8>, tol: f32) -> u8 = thiscall 0x4042D0 => unported;
    /// `CRect::Restrict(const CRect&)` (0x404200)
    pub fn rect_restrict(r: GPtr<u8>, o: GPtr<u8>) = thiscall 0x404200 => unported;
    /// `CQuadTreeNode::ForAllMatching(const CVector2D&, cb)` (0x5529F0)
    pub fn quad_for_all_matching(node: GPtr<u8>, v: GPtr<u8>, cb: u32) = thiscall 0x5529F0 => sa_core::quadtree::for_all_matching_vec;
    /// `CQuadTreeNode::AddItem(item, rect)` (0x552CD0)
    pub fn quad_add_item(node: GPtr<u8>, item: GPtr<u8>, r: GPtr<u8>) = thiscall 0x552CD0 => sa_core::quadtree::add_item;
    /// `CQuadTreeNode::DeleteItem(item)` (0x552A40)
    pub fn quad_delete_item(node: GPtr<u8>, item: GPtr<u8>) = thiscall 0x552A40 => sa_core::quadtree::delete_item;
    /// `CQuadTreeNode::operator new(size)` (0x552C80)
    pub fn quad_operator_new(size: u32) -> GPtr<u8> = cdecl 0x552C80 => unported;
    /// `CQuadTreeNode::operator delete(p)` (0x552C90)
    pub fn quad_operator_delete(p: GPtr<u8>) = cdecl 0x552C90 => unported;
    /// `CQuadTreeNode::CQuadTreeNode(const CRect&, int levels)` (0x552830)
    pub fn quad_ctor(node: GPtr<u8>, r: GPtr<u8>, levels: i32) -> GPtr<u8> = thiscall 0x552830 => unported;
    /// `CQuadTreeNode::~CQuadTreeNode()` (0x552520)
    pub fn quad_dtor(node: GPtr<u8>) = thiscall 0x552520 => unported;
    /// `FindPlayerPed(int)` (0x56E210)
    pub fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int, bool)` (0x56E0D0)
    pub fn find_player_vehicle(n: i32, include_remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CStreaming::RequestModel(id, flags)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => crate::streaming::request_model;
    /// `CStreaming::RemoveModel(id)` (0x4089A0)
    pub fn remove_model(id: i32) = cdecl 0x4089A0 => crate::streaming::remove_model;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(only_priority: u8) = cdecl 0x40EA10 => crate::streaming::load_all_requested_models;
    /// `CCamera::GetScreenFadeStatus()` (0x50AE20)
    pub fn get_screen_fade_status(cam: GPtr<u8>) -> i32 = thiscall 0x50AE20 => unported;
    /// `CMenuManager::MessageScreen(key, blackBackground, cameraUpdateStarted)` (0x579330)
    pub fn message_screen(menu: GPtr<u8>, key: GPtr<u8>, black: u8, cam: u8) = thiscall 0x579330 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    pub fn timer_suspend() = cdecl 0x5619D0 => sa_core::timer::suspend;
    /// `CTimer::Resume()` (0x561A00)
    pub fn timer_resume() = cdecl 0x561A00 => sa_core::timer::resume;
    /// `CWorld::Add(entity)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CWorld::Remove(entity)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// entity `vtbl[0]`: the virtual deleting destructor (`delete` flag 1)
    pub fn entity_delete(e: GPtr<u8>, flags: i32) -> GPtr<u8> = thiscall vslot 0x00 => unported;
    /// entity `vtbl[2]`: `CEntity::Add()`
    pub fn entity_add(e: GPtr<u8>) = thiscall vslot 0x08 => unported;
    /// entity `vtbl[9]`: `GetBoundRect(CRect* out)` (returns `out`)
    pub fn entity_get_bound_rect(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTheCarGenerators::RemoveCarGenerators(uint8 ipl)` (0x6F3240; called with the int32 argument)
    pub fn remove_car_generators(ipl: i32) = cdecl 0x6F3240 => unported;
    /// `CColAccel::isCacheLoading()` (0x5B2AC0, AL)
    pub fn col_accel_is_cache_loading() -> u8 = cdecl 0x5B2AC0 => unported;
    /// `CColAccel::getIplDef(IplDef* ret, int slot)` (0x5B2EF0, struct return through `ret`)
    pub fn col_accel_get_ipl_def(ret: GPtr<IplDef>, slot: i32) -> GPtr<IplDef> = cdecl 0x5B2EF0 => unported;
    /// `CColAccel::setIplDef(int slot, IplDef def)` (0x5B2ED0, the def by value = 13 dwords)
    pub fn col_accel_set_ipl_def(slot: i32, def: IplDef) = cdecl 0x5B2ED0 => unported;
    /// `CFileLoader::LoadLine(char*& data, int32& size)` (0x536FE0)
    pub fn load_line(data: GPtr<GPtr<u8>>, size: GPtr<i32>) -> GPtr<u8> = cdecl 0x536FE0 => crate::fileloader::load_line_buffer;
    /// `CFileLoader::LoadObjectInstance(const char* line)` (0x538690)
    pub fn load_object_instance_text(line: GPtr<u8>) -> GPtr<u8> = cdecl 0x538690 => crate::fileloader::load_object_instance_line;
    /// `CFileLoader::LoadObjectInstance(CFileObjectInstance*, const char* name)` (0x538090)
    pub fn load_object_instance_bin(inst: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x538090 => unported;
    /// `CFileLoader::LoadCarGenerator(CFileCarGenerator*, int ipl)` (0x537990)
    pub fn load_car_generator(cg: GPtr<u8>, ipl: i32) = cdecl 0x537990 => crate::fileloader::load_car_generator_struct;
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int)` (0x5D1270)
    pub fn save_data_to_work_buffer(p: GPtr<u8>, n: i32) = cdecl 0x5D1270 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int)` (0x5D1300)
    pub fn load_data_from_work_buffer(p: GPtr<u8>, n: i32) = cdecl 0x5D1300 => unported;
    /// CRT `operator new` (0x82119A), `operator new[]` (0x821195), `operator delete[]` (0x8213AE), `free` (0x8214BD)
    pub fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    pub fn operator_new_array(n: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    pub fn operator_delete_array(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `_stricmp` (0x8229B6), `_strnicmp` (0x835AC7), `strncmp` (0x8214D0), `strrchr` (0x821490), `strchr` (0x8213D0)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    pub fn strnicmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x835AC7 => unported;
    pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    pub fn strrchr(s: GPtr<u8>, c: i32) -> GPtr<u8> = cdecl 0x821490 => unported;
    pub fn strchr(s: GPtr<u8>, c: i32) -> GPtr<u8> = cdecl 0x8213D0 => unported;
    /// Rust-internal calls through the hook table
    pub fn set_ipls_required_call(posn: GPtr<f32>, area: i32) = cdecl 0x404700 => set_ipls_required;
    pub fn include_entity_call(slot: i32, e: GPtr<u8>) = cdecl 0x404C90 => include_entity;
    pub fn remove_ipl_call(slot: i32) = cdecl 0x404B20 => remove_ipl;
    pub fn add_ipl_slot_call(name: GPtr<u8>) -> i32 = cdecl 0x405AC0 => add_ipl_slot;
    pub fn remove_ipl_slot_call(slot: i32) = cdecl 0x405B60 => remove_ipl_slot;
    pub fn remove_all_ipls_call() = cdecl 0x405720 => remove_all_ipls;
    pub fn load_ipl_bounding_box_call(slot: i32, data: GPtr<u8>, size: i32) -> u8 = cdecl 0x405C00 => load_ipl_bounding_box;
    pub fn request_ipl_and_ignore_call(slot: i32) = cdecl 0x405850 => request_ipl_and_ignore;
    pub fn remove_ipl_and_ignore_call(slot: i32) = cdecl 0x405890 => remove_ipl_and_ignore;
}

const NAME: isize = 0x10;
const FIRST_BUILDING: isize = 0x22;
const LAST_BUILDING: isize = 0x24;
const FIRST_DUMMY: isize = 0x26;
const LAST_DUMMY: isize = 0x28;
const STATIC_IDX: isize = 0x2A;
const IS_INTERIOR: isize = 0x2C;
const LOADED: isize = 0x2D;
const LOAD_REQUESTED: isize = 0x2E;
const DISABLE_DYNAMIC: isize = 0x2F;
const IGNORE_WHEN_DELETED: isize = 0x30;
const IS_LARGE: isize = 0x31;

/// Entity fields used here.
const E_IPL_INDEX: isize = 0x2E;
const E_LOD: isize = 0x30;
const E_MODEL: isize = 0x22;
const E_TYPE_STATUS: isize = 0x36;
const LOD_NUM_CHILDREN: isize = 0x34;

fn pool() -> GPtr<CPool<IplDef>> {
    IPL_POOL.get()
}

/// `ms_pPool->GetAt(i)`: null for a free slot (no bounds check).
fn at(i: i32) -> GPtr<IplDef> {
    pool::pool_at(pool(), i)
}

fn b(d: GPtr<IplDef>, off: isize) -> GPtr<u8> {
    field(d, off)
}
fn w(d: GPtr<IplDef>, off: isize) -> GPtr<i16> {
    field(d, off)
}
fn fl(d: GPtr<IplDef>, k: isize) -> f32 {
    field::<f32, _>(d, 4 * k).read()
}

/// The slot walk the loops share: used map byte and non-null `objects + 0x34 i` (pool re-read).
fn used(i: i32) -> Option<GPtr<IplDef>> {
    let p = pool().read();
    if (p.byte_map.wrapping_index(i).read() as i8) < 0 {
        return None;
    }
    let d = p.objects.wrapping_index(i);
    if d.is_null() { None } else { Some(d) }
}

/// `CIplStore::AddIplsNeededAtPosn(const CVector&)` (stub 0x4045B0 → .HOODLUM 0x1567820, cdecl).
#[hook(addr = 0x4045B0, conv = cdecl, name = "CIplStore::AddIplsNeededAtPosn", expect = "E9 6B 32 16 01")]
pub fn add_ipls_needed_at_posn(posn: GPtr<[f32; 3]>) {
    let v = posn.read();
    let g = VEC_IPLS_NEEDED_AT_POSN.ptr().cast::<f32>();
    g.write(v[0]);
    g.wrapping_index(1).write(v[1]);
    IPLS_NEEDED_AT_POSN.set(1);
    g.wrapping_index(2).write(v[2]);
}

/// `CIplStore::ClearIplsNeededAtPosn()` (stub 0x4045E0 → .HOODLUM 0x156CFF0, cdecl).
#[hook(addr = 0x4045E0, conv = cdecl, name = "CIplStore::ClearIplsNeededAtPosn", expect = "E9 0B 8A 16 01")]
pub fn clear_ipls_needed_at_posn() {
    IPLS_NEEDED_AT_POSN.set(0);
}

/// The inline `CRect::IsPointInside(p)` without tolerance: `!(x < l) && !(x > r) && !(y < b) &&
/// !(y > t)` with b = +0xC, t = +4 (`test ah,5; jnp` / `test ah,0x41; je`: NaN passes).
fn inside_exact(d: GPtr<IplDef>, x: f32, y: f32) -> bool {
    let (fx, fy) = (X87::from_f32(x), X87::from_f32(y));
    let f = |k: isize| X87::from_f32(fl(d, k));
    fx.fcom(f(0)).test_parity(5) && fx.fcom(f(2)).test_any(0x41) && fy.fcom(f(3)).test_parity(5) && fy.fcom(f(1)).test_any(0x41)
}

/// `SetIfInteriorIplIsRequired(const CVector2D&, IplDef*)` (0x4045F0, cdecl; quad tree callback).
#[hook(addr = 0x4045F0, conv = cdecl, name = "CIplStore::SetIfInteriorIplIsRequired", expect = "56 8B 74 24 0C")]
pub fn set_if_interior_ipl_is_required(posn: GPtr<u8>, d: GPtr<IplDef>) {
    let set = if b(d, IS_INTERIOR).read() != 0 {
        if CURRENT_IPL_AREA_CODE.get() == 0 {
            return;
        }
        rect_is_point_inside_tol(d.cast(), posn, -140.0) != 0
    } else {
        let p: GPtr<f32> = posn.cast();
        inside_exact(d, p.read(), p.wrapping_index(1).read())
    };
    if set {
        b(d, LOAD_REQUESTED).write(1);
    }
}

/// `SetIfIplIsRequired(const CVector2D&, IplDef*)` (0x404660, cdecl).
#[hook(addr = 0x404660, conv = cdecl, name = "CIplStore::SetIfIplIsRequired", expect = "56 8B 74 24 0C")]
pub fn set_if_ipl_is_required(posn: GPtr<u8>, d: GPtr<IplDef>) {
    if b(d, IS_INTERIOR).read() != 0 && rect_is_point_inside_tol(d.cast(), posn, -140.0) != 0 {
        b(d, LOAD_REQUESTED).write(1);
    }
}

/// `SetIfIplIsRequiredReducedBB(const CVector2D&, IplDef*)` (0x404690, cdecl): inside the box
/// shrunk by 160 (`l - (-160) <= x <= r - 160`, `b + 160 <= y <= t - 160`, x87 PC=24
/// subtractions), then by area code: normal world wants non-interiors, others interiors.
#[hook(addr = 0x404690, conv = cdecl, name = "CIplStore::SetIfIplIsRequiredReducedBB", expect = "8B 4C 24 08 D9 01")]
pub fn set_if_ipl_is_required_reduced_bb(posn: GPtr<u8>, d: GPtr<IplDef>) {
    let p: GPtr<f32> = posn.cast();
    let (x, y) = (p.read(), p.wrapping_index(1).read());
    let sub = |v: f32, k: f32| X87::from_f32(v).fsub_m(k);
    let fx = X87::from_f32(x);
    let fy = X87::from_f32(y);
    // fcomp: ST0 (bound - k) against the point
    if !sub(fl(d, 0), -160.0).fcom(fx).test_any(0x41) {
        return;
    }
    if !sub(fl(d, 2), 160.0).fcom(fx).test_parity(5) {
        return;
    }
    if !sub(fl(d, 3), -160.0).fcom(fy).test_any(0x41) {
        return;
    }
    if !sub(fl(d, 1), 160.0).fcom(fy).test_parity(5) {
        return;
    }
    let interior = b(d, IS_INTERIOR).read() != 0;
    if CURRENT_IPL_AREA_CODE.get() != 0 {
        if !interior {
            return;
        }
    } else if interior {
        return;
    }
    b(d, LOAD_REQUESTED).write(1);
}

/// `CIplStore::SetIplsRequired(const CVector&, int gameArea)` (0x404700, cdecl).
#[hook(addr = 0x404700, conv = cdecl, name = "CIplStore::SetIplsRequired", expect = "8B 4C 24 08 83 EC 08")]
pub fn set_ipls_required(posn: GPtr<f32>, area: i32) {
    let mut area = area;
    if area == -1 {
        let ped = find_player_ped(-1);
        area = if ped.is_null() {
            sa_core::game::CURR_AREA.get()
        } else {
            field::<u8, _>(ped, 0x2F).read() as i32
        };
    }
    CURRENT_IPL_AREA_CODE.set(area);
    let z = posn.wrapping_index(2).read();
    let cb = if area != 0 && !X87::from_f32(z).fcom(X87::from_f32(900.0)).c0() {
        CB_SET_IF_IPL_IS_REQUIRED
    } else if area == sa_core::game::CURR_AREA.get() {
        CB_SET_IF_INTERIOR_IPL_IS_REQUIRED
    } else {
        CB_SET_IF_IPL_IS_REQUIRED_REDUCED_BB
    };
    let mut v = [posn.read(), posn.wrapping_index(1).read()];
    quad_for_all_matching(QUAD_TREE.get(), GPtr::from_raw(v.as_mut_ptr()).cast(), cb);
}

/// `CIplStore::GetNewIplEntityIndexArray(int count)` (stub 0x404780 → .HOODLUM 0x15649E0, cdecl).
#[hook(addr = 0x404780, conv = cdecl, name = "CIplStore::GetNewIplEntityIndexArray", expect = "E9 5B 02 16 01")]
pub fn get_new_ipl_entity_index_array(count: i32) -> i32 {
    let p = operator_new_array((count as u32).wrapping_mul(4));
    let n = NUM_IPL_ENTITY_INDEX_ARRAYS.get();
    IPL_ENTITY_INDEX_ARRAYS.ptr().cast::<GPtr<u8>>().wrapping_index(n).write(p);
    NUM_IPL_ENTITY_INDEX_ARRAYS.set(n.wrapping_add(1));
    n
}

fn entity_array(i: i32) -> GPtr<GPtr<u8>> {
    IPL_ENTITY_INDEX_ARRAYS.ptr().cast::<GPtr<GPtr<u8>>>().wrapping_index(i).read()
}

/// `CIplStore::GetIplEntityIndexArray(int)` (stub 0x4047B0 → .HOODLUM 0x1569770, cdecl).
#[hook(addr = 0x4047B0, conv = cdecl, name = "CIplStore::GetIplEntityIndexArray", expect = "E9 BB 4F 16 01")]
pub fn get_ipl_entity_index_array(i: i32) -> GPtr<GPtr<u8>> {
    entity_array(i)
}

/// `CIplStore::GetIplName(int)` (0x404A60, cdecl): `&def->name`, 0x10 for a free slot.
#[hook(addr = 0x404A60, conv = cdecl, name = "CIplStore::GetIplName", expect = "8B 0D B0 3F 8E 00")]
pub fn get_ipl_name(i: i32) -> GPtr<u8> {
    let d = at(i);
    if d.is_null() { GPtr::from_addr(NAME as usize) } else { b(d, NAME) }
}

/// `CIplStore::SetIsInterior(int, bool)` (0x404A90, cdecl).
#[hook(addr = 0x404A90, conv = cdecl, name = "CIplStore::SetIsInterior", expect = "8B 0D B0 3F 8E 00")]
pub fn set_is_interior(i: i32, interior: u8) {
    b(nonnull(at(i), "CIplStore::SetIsInterior"), IS_INTERIOR).write(interior);
}

/// `CIplStore::FindIplSlot(const char*)` (0x404AC0, cdecl): ascending `_stricmp`.
#[hook(addr = 0x404AC0, conv = cdecl, name = "CIplStore::FindIplSlot", expect = "8B 0D B0 3F 8E 00")]
pub fn find_ipl_slot(name: GPtr<u8>) -> i32 {
    let mut i = 0;
    while i < pool().read().size {
        if let Some(d) = used(i)
            && stricmp(name, b(d, NAME)) == 0
        {
            return i;
        }
        i += 1;
    }
    -1
}

/// The per-entity step of RemoveIpl's three loops.
fn remove_entity_if(e: GPtr<u8>, slot: i32, object: bool) {
    if field::<u8, _>(e, E_IPL_INDEX).read() as i32 != slot {
        return;
    }
    if object {
        let dummy = field::<GPtr<u8>, _>(e, 0x170).read();
        if !dummy.is_null() {
            world_add(dummy);
        }
    }
    world_remove(e);
    entity_delete(e, 1);
}

/// `CIplStore::RemoveIpl(int)` (0x404B20, cdecl).
#[hook(addr = 0x404B20, conv = cdecl, name = "CIplStore::RemoveIpl", expect = "A1 B0 3F 8E 00")]
pub fn remove_ipl(slot: i32) {
    let d = at(slot);
    let dummies = DUMMY_POOL.get();
    let buildings = BUILDING_POOL.get();
    let objects = OBJECT_POOL.get();
    let d = nonnull(d, "CIplStore::RemoveIpl");
    let mut i = w(d, FIRST_BUILDING).read() as i32;
    let last = w(d, LAST_BUILDING).read() as i32;
    b(d, LOADED).write(0);
    if i <= last {
        loop {
            let h = buildings.read();
            if h.byte_map.wrapping_index(i).read() & 0x80 == 0 {
                let e = h.objects.wrapping_index(i);
                if !e.is_null() {
                    remove_entity_if(e.cast(), slot, false);
                }
            }
            i += 1;
            if i > w(d, LAST_BUILDING).read() as i32 {
                break;
            }
        }
    }
    let mut i = 0;
    while i < objects.read().size {
        let h = objects.read();
        if (h.byte_map.wrapping_index(i).read() as i8) >= 0 {
            let e = h.objects.wrapping_index(i);
            if !e.is_null() {
                remove_entity_if(e.cast(), slot, true);
            }
        }
        i += 1;
    }
    let mut i = w(d, FIRST_DUMMY).read() as i32;
    if i <= w(d, LAST_DUMMY).read() as i32 {
        loop {
            let h = dummies.read();
            if (h.byte_map.wrapping_index(i).read() as i8) >= 0 {
                let e = h.objects.wrapping_index(i);
                if !e.is_null() {
                    remove_entity_if(e.cast(), slot, false);
                }
            }
            i += 1;
            if i > w(d, LAST_DUMMY).read() as i32 {
                break;
            }
        }
    }
    remove_car_generators(slot);
}

/// `CIplStore::GetBoundingBox(int)` (0x404C70, cdecl): the def (= its rect), 0 for a free slot.
#[hook(addr = 0x404C70, conv = cdecl, name = "CIplStore::GetBoundingBox", expect = "8B 0D B0 3F 8E 00")]
pub fn get_bounding_box(i: i32) -> GPtr<u8> {
    at(i).cast()
}

/// `(e - objects) / slot` as the magic-multiply division computes it (signed, truncating).
fn pool_index(e: GPtr<u8>, objects: GPtr<u8>, slot: i32) -> i32 {
    (e.addr() as u32).wrapping_sub(objects.addr() as u32) as i32 / slot
}

/// `CIplStore::IncludeEntity(int, CEntity*)` (stub 0x404C90 → .HOODLUM 0x1563730, cdecl).
#[hook(addr = 0x404C90, conv = cdecl, name = "CIplStore::IncludeEntity", expect = "E9 9B EA 15 01")]
pub fn include_entity(slot: i32, e: GPtr<u8>) {
    let d = at(slot);
    let (pool_objects, lo, hi) = match field::<u8, _>(e, E_TYPE_STATUS).read() & 7 {
        1 => (BUILDING_POOL.get().read().objects.cast::<u8>(), FIRST_BUILDING, LAST_BUILDING),
        // the dummy pool pointer is read through a protection chunk (resolves to 0xB744A0)
        5 => (DUMMY_POOL.get().read().objects.cast::<u8>(), FIRST_DUMMY, LAST_DUMMY),
        _ => return,
    };
    let idx = pool_index(e, pool_objects, 0x38);
    let d = nonnull(d, "CIplStore::IncludeEntity");
    if idx < w(d, lo).read() as i32 {
        w(d, lo).write(idx as i16);
    }
    if idx > w(d, hi).read() as i32 {
        w(d, hi).write(idx as i16);
    }
}

/// `CIplStore::EnableDynamicStreaming(int, bool)` (0x404D30, cdecl): `disable = !enable`.
#[hook(addr = 0x404D30, conv = cdecl, name = "CIplStore::EnableDynamicStreaming", expect = "8B 0D B0 3F 8E 00")]
pub fn enable_dynamic_streaming(i: i32, enable: u8) {
    b(nonnull(at(i), "CIplStore::EnableDynamicStreaming"), DISABLE_DYNAMIC).write((enable == 0) as u8);
}

/// `CIplStore::SetupRelatedIpls(const char* path, int index, CEntity** buildings)` (0x404DE0,
/// cdecl): returns the number of LOD-parented instances the related IPLs stored.
#[hook(addr = 0x404DE0, conv = cdecl, name = "CIplStore::SetupRelatedIpls", expect = "83 EC 5C 8B 44 24 60")]
pub fn setup_related_ipls(path: GPtr<u8>, index: i32, buildings: GPtr<GPtr<u8>>) -> i32 {
    // 0x20-byte name buffer immediately followed by the 0x34-byte getIplDef temporary
    let mut frame = [0u8; 0x20 + 0x34];
    let mut interior = 0u8;
    let slash = strrchr(path, b'\\' as i32);
    if slash.is_null() {
        return 0;
    }
    let dot = strchr(slash, b'.' as i32);
    if dot.is_null() {
        return 0;
    }
    let mut n = 0usize;
    let mut s = slash.byte_add(1);
    while s.addr() != dot.addr() {
        if n >= frame.len() {
            panic!("CIplStore::SetupRelatedIpls: IPL name overflows the original's stack frame (original UB)");
        }
        frame[n] = s.read();
        n += 1;
        s = s.byte_add(1);
    }
    if n + 8 > frame.len() {
        panic!("CIplStore::SetupRelatedIpls: IPL name overflows the original's stack frame (original UB)");
    }
    frame[n] = 0;
    // from here on the frame is only accessed through `name` (the original's stack memory, also
    // written by getIplDef through `tmp`)
    let name = GPtr::<u8>::from_raw(frame.as_mut_ptr());
    for s in INTERIOR_NAMES {
        if stricmp(name, image(s)) == 0 {
            interior = 1;
            break;
        }
    }
    // "_stream" stored as two dwords (8 bytes incl. NUL)
    let suffix: GPtr<[u8; 8]> = image(STR_STREAM_SUFFIX);
    name.byte_add(n as isize).cast::<[u8; 8]>().write(suffix.read());
    CURR_IPL_INSTANCE.set(buildings);
    let tmp: GPtr<IplDef> = name.byte_add(0x20).cast();
    // strlen of the (possibly overwritten) name, bounded by the frame
    let len = || (0..0x54).position(|k| name.byte_add(k).read() == 0).unwrap_or(0x54) as u32;
    if col_accel_is_cache_loading() != 0 {
        let mut i = 0;
        while i < pool().read().size {
            if let Some(d) = used(i)
                && strnicmp(name, b(d, NAME), len()) == 0
            {
                let src = col_accel_get_ipl_def(tmp, i);
                d.write(src.read());
                w(d, STATIC_IDX).write(index as i16);
                b(d, IS_INTERIOR).write(interior);
                b(d, LOADED).write(0);
                quad_add_item(QUAD_TREE.get(), d.cast(), d.cast());
            }
            i += 1;
        }
    } else {
        let mut i = 0;
        while i < pool().read().size {
            if let Some(d) = used(i)
                && strnicmp(name, b(d, NAME), len()) == 0
            {
                w(d, STATIC_IDX).write(index as i16);
                b(d, IS_INTERIOR).write(interior);
                // inlined GetAt(i) again; a free slot writes through null
                b(nonnull(at(i), "CIplStore::SetupRelatedIpls"), DISABLE_DYNAMIC).write(0);
                request_model(i + RESOURCE_ID_IPL, 8);
            }
            i += 1;
        }
        load_all_requested_models(0);
    }
    let r = ((CURR_IPL_INSTANCE.get().addr() as u32).wrapping_sub(buildings.addr() as u32) as i32) >> 2;
    CURR_IPL_INSTANCE.set(GPtr::NULL);
    r
}

/// `CIplStore::RemoveRelatedIpls(int)` (0x405110, cdecl).
#[hook(addr = 0x405110, conv = cdecl, name = "CIplStore::RemoveRelatedIpls", expect = "8B 0D B0 3F 8E 00")]
pub fn remove_related_ipls(index: i32) {
    let mut i = 0;
    while i < pool().read().size {
        if let Some(d) = used(i)
            && w(d, STATIC_IDX).read() as i32 == index
        {
            remove_model(i + RESOURCE_ID_IPL);
        }
        i += 1;
    }
}

/// `min(40, v * 80)` as `fld 40; fcomp st(1); test ah,5; jp` keeps it (x87, NaN keeps `v * 80`).
fn clamp_velocity(v: f32) -> X87 {
    let m = X87::from_f32(v).fmul_m(80.0);
    if X87::from_f32(40.0).fcom(m).test_parity(5) { m } else { X87::from_f32(40.0) }
}

/// `CIplStore::LoadIpls(CVector posn, bool bAvoidLoadInPlayerVehicleMovingDirection)` (0x405170,
/// cdecl; entry through the chunk 0x403803).
#[hook(addr = 0x405170, conv = cdecl, name = "CIplStore::LoadIpls", expect = "E9 8E E6 FF FF 83")]
pub fn load_ipls(x: f32, y: f32, z: f32, avoid: u8) {
    if disable_streaming() {
        return;
    }
    let mut posn = [x, y, z];
    if !find_player_ped(-1).is_null() && avoid == 0 {
        let veh = find_player_vehicle(-1, 0);
        if !veh.is_null() {
            let v: [f32; 3] = field::<[f32; 3], _>(veh, 0x44).read();
            posn[0] = clamp_velocity(v[0]).fadd_m(posn[0]).to_f32();
            posn[1] = clamp_velocity(v[1]).fadd_m(posn[1]).to_f32();
        }
    }
    set_ipls_required_call(GPtr::from_raw(posn.as_mut_ptr()), -1);
    if IPLS_NEEDED_AT_POSN.get() != 0 {
        set_ipls_required_call(VEC_IPLS_NEEDED_AT_POSN.ptr().cast(), sa_core::game::CURR_AREA.get());
    }
    for k in 0..75u32 {
        let entry = image::<u8>(MISSION_CLEANUP + 8 * k);
        let ty = entry.read();
        let handle = field::<i32, _>(entry, 4).read();
        let e: GPtr<u8> = match ty {
            1 => {
                let v = VEHICLE_POOL.get().read();
                let idx = handle >> 8;
                if v.byte_map.wrapping_index(idx).read() != handle as u8 {
                    continue;
                }
                let e = v.objects.wrapping_index(idx).cast::<u8>();
                if e.is_null() || !field::<GPtr<u8>, _>(e, 0xFC).read().is_null() {
                    continue;
                }
                let status = field::<u8, _>(e, E_TYPE_STATUS).read() >> 3;
                if status != 3 && status != 2 && status != 4 {
                    continue;
                }
                e
            }
            2 => {
                let p = PED_POOL.get().read();
                let idx = handle >> 8;
                if p.byte_map.wrapping_index(idx).read() != handle as u8 {
                    continue;
                }
                let e = p.objects.wrapping_index(idx).cast::<u8>();
                if e.is_null() || !field::<GPtr<u8>, _>(e, 0xFC).read().is_null() {
                    continue;
                }
                let state = field::<i32, _>(e, 0x530).read();
                if state == 0x36 || state == 0x37 {
                    continue;
                }
                e
            }
            _ => continue,
        };
        if field::<u32, _>(e, 0x40).read() & 0x6000 != 0 {
            continue;
        }
        CURRENT_IPL_AREA_CODE.set(field::<u8, _>(e, 0x2F).read() as i32);
        let m = field::<GPtr<u8>, _>(e, 0x14).read();
        let pos: GPtr<f32> = if m.is_null() { field(e, 4) } else { field(m, 0x30) };
        let mut v = [pos.read(), pos.wrapping_index(1).read()];
        quad_for_all_matching(QUAD_TREE.get(), GPtr::from_raw(v.as_mut_ptr()).cast(), CB_SET_IF_IPL_IS_REQUIRED_REDUCED_BB);
    }
    for i in 1..256 {
        let Some(d) = used(i) else { continue };
        if b(d, DISABLE_DYNAMIC).read() != 0 {
            continue;
        }
        if b(d, LOAD_REQUESTED).read() != 0 {
            if b(d, LOADED).read() == 0 {
                request_model(i + RESOURCE_ID_IPL, 0x18);
            }
            b(d, LOAD_REQUESTED).write(0);
        } else if b(d, LOADED).read() != 0 {
            remove_model(i + RESOURCE_ID_IPL);
            if b(d, IGNORE_WHEN_DELETED).read() != 0 {
                b(d, DISABLE_DYNAMIC).write(1);
            }
        }
    }
    if avoid == 0 {
        IPLS_NEEDED_AT_POSN.set(0);
    }
}

/// `CIplStore::EnsureIplsAreInMemory(const CVector&)` (0x4053F0, cdecl).
#[hook(addr = 0x4053F0, conv = cdecl, name = "CIplStore::EnsureIplsAreInMemory", expect = "A0 B0 54 96 00")]
pub fn ensure_ipls_are_in_memory(posn: GPtr<f32>) {
    if disable_streaming() {
        return;
    }
    let area = if find_player_ped(-1).is_null() {
        sa_core::game::CURR_AREA.get()
    } else {
        field::<u8, _>(find_player_ped(-1), 0x2F).read() as i32
    };
    if area != sa_core::game::CURR_AREA.get() {
        return;
    }
    set_ipls_required_call(posn, -1);
    for i in 1..256 {
        let Some(d) = used(i) else { continue };
        if b(d, DISABLE_DYNAMIC).read() != 0 || b(d, LOAD_REQUESTED).read() == 0 {
            continue;
        }
        let mut v = [posn.read(), posn.wrapping_index(1).read()];
        if rect_is_point_inside_tol(d.cast(), GPtr::from_raw(v.as_mut_ptr()).cast(), -190.0) != 0
            && info(i + RESOURCE_ID_IPL).read().load_state != 1
        {
            request_model(i + RESOURCE_ID_IPL, 0x18);
            if get_screen_fade_status(image(THE_CAMERA)) == 0 {
                message_screen(image(FRONT_END_MENU_MANAGER), image(STR_LOADCOL), 0, 0);
            }
            timer_suspend();
            load_all_requested_models(1);
            timer_resume();
        }
        b(d, LOAD_REQUESTED).write(0);
    }
}

/// The inline `IsPointInside(p, -190)` of RequestIpls / HaveIplsLoaded: `x >= l - (-190)`,
/// `x <= r - 190`, `y >= b - (-190)`, `y <= t - 190` (x87 PC=24 subtractions; NaN passes).
fn inside_190(d: GPtr<IplDef>, x: f32, y: f32) -> bool {
    let sub = |v: f32, k: f32| X87::from_f32(v).fsub_m(k);
    let (fx, fy) = (X87::from_f32(x), X87::from_f32(y));
    // fcompp of x against (l + 190): skip when x < bound
    if !fx.fcom(sub(fl(d, 0), -190.0)).test_parity(5) {
        return false;
    }
    if !fx.fcom(sub(fl(d, 2), 190.0)).test_any(0x41) {
        return false;
    }
    if !sub(fl(d, 3), -190.0).fcom(fy).test_any(0x41) {
        return false;
    }
    sub(fl(d, 1), 190.0).fcom(fy).test_parity(5)
}

/// `CIplStore::RequestIpls(const CVector&, int)` (0x405520, cdecl).
#[hook(addr = 0x405520, conv = cdecl, name = "CIplStore::RequestIpls", expect = "8B 44 24 08 83 EC 08")]
pub fn request_ipls(posn: GPtr<f32>, area: i32) {
    set_ipls_required_call(posn, area);
    for i in 1..256 {
        let Some(d) = used(i) else { continue };
        if b(d, LOAD_REQUESTED).read() == 0 {
            continue;
        }
        if inside_190(d, posn.read(), posn.wrapping_index(1).read()) {
            request_model(i + RESOURCE_ID_IPL, 0x18);
        }
        b(d, LOAD_REQUESTED).write(0);
    }
}

/// `CIplStore::HaveIplsLoaded(const CVector&, int)` (0x405600, cdecl, AL): at the first requested
/// IPL around the point that is neither loaded nor disabled, clears `loadRequested` of it and every
/// later used slot and returns false.
#[hook(addr = 0x405600, conv = cdecl, name = "CIplStore::HaveIplsLoaded", expect = "8B 44 24 08 83 EC 08")]
pub fn have_ipls_loaded(posn: GPtr<f32>, area: i32) -> bool {
    set_ipls_required_call(posn, area);
    for i in 1..256 {
        let Some(d) = used(i) else { continue };
        if b(d, LOAD_REQUESTED).read() == 0 {
            continue;
        }
        if inside_190(d, posn.read(), posn.wrapping_index(1).read())
            && b(d, LOADED).read() == 0
            && b(d, DISABLE_DYNAMIC).read() == 0
        {
            for j in i..256 {
                let p = pool().read();
                let e = if p.byte_map.wrapping_index(j).read() & 0x80 != 0 {
                    GPtr::NULL
                } else {
                    p.objects.wrapping_index(j)
                };
                if !e.is_null() {
                    b(e, LOAD_REQUESTED).write(0);
                }
            }
            return false;
        }
        b(d, LOAD_REQUESTED).write(0);
    }
    true
}

/// `CIplStore::RemoveAllIpls()` (0x405720, cdecl): slots 1..255 not game/mission required.
#[hook(addr = 0x405720, conv = cdecl, name = "CIplStore::RemoveAllIpls", expect = "51 A1 B0 3F 8E 00")]
pub fn remove_all_ipls() {
    for i in 1..256 {
        if used(i).is_none() {
            continue;
        }
        if info(i + RESOURCE_ID_IPL).read().flags & 6 != 0 {
            continue;
        }
        remove_model(i + RESOURCE_ID_IPL);
    }
}

/// `CIplStore::LoadAllRemainingIpls()` (0x405780, cdecl): every slot whose box is still flipped.
#[hook(addr = 0x405780, conv = cdecl, name = "CIplStore::LoadAllRemainingIpls", expect = "83 EC 38 53 55")]
pub fn load_all_remaining_ipls() {
    let mut tmp = [0u8; 0x34];
    for i in 1..256 {
        let Some(d) = used(i) else { continue };
        if !(fl(d, 0) > fl(d, 2) || fl(d, 3) > fl(d, 1)) {
            continue;
        }
        if col_accel_is_cache_loading() != 0 {
            let src = col_accel_get_ipl_def(GPtr::from_raw(tmp.as_mut_ptr()).cast(), i);
            d.write(src.read());
            b(d, LOADED).write(0);
            quad_add_item(QUAD_TREE.get(), d.cast(), d.cast());
        } else {
            request_model(i + RESOURCE_ID_IPL, 0x18);
            load_all_requested_models(1);
            remove_model(i + RESOURCE_ID_IPL);
        }
    }
}

/// `CIplStore::RequestIplAndIgnore(int)` (stub 0x405850 → .HOODLUM 0x1565190, cdecl).
#[hook(addr = 0x405850, conv = cdecl, name = "CIplStore::RequestIplAndIgnore", expect = "E9 3B F9 15 01")]
pub fn request_ipl_and_ignore(i: i32) {
    let d = at(i);
    request_model(i + RESOURCE_ID_IPL, 8);
    let d = nonnull(d, "CIplStore::RequestIplAndIgnore");
    b(d, DISABLE_DYNAMIC).write(1);
    b(d, IGNORE_WHEN_DELETED).write(0);
}

/// `CIplStore::RemoveIplAndIgnore(int)` (stub 0x405890 → .HOODLUM 0x156A140, cdecl).
#[hook(addr = 0x405890, conv = cdecl, name = "CIplStore::RemoveIplAndIgnore", expect = "E9 AB 48 16 01")]
pub fn remove_ipl_and_ignore(i: i32) {
    let d = at(i);
    remove_model(i + RESOURCE_ID_IPL);
    let d = nonnull(d, "CIplStore::RemoveIplAndIgnore");
    b(d, DISABLE_DYNAMIC).write(1);
    b(d, IGNORE_WHEN_DELETED).write(0);
}

/// `CIplStore::RemoveIplWhenFarAway(int)` (stub 0x4058D0 → .HOODLUM 0x156DE30, cdecl).
#[hook(addr = 0x4058D0, conv = cdecl, name = "CIplStore::RemoveIplWhenFarAway", expect = "E9 5B 85 16 01")]
pub fn remove_ipl_when_far_away(i: i32) {
    let d = nonnull(at(i), "CIplStore::RemoveIplWhenFarAway");
    b(d, DISABLE_DYNAMIC).write(0);
    b(d, IGNORE_WHEN_DELETED).write(1);
}

/// `CIplStore::AddIplSlot(const char*)` (stub 0x405AC0 → .HOODLUM 0x156C470, cdecl): a new def
/// with the inverted ±1e6 box, `strcpy` of the name (no limit), dynamic streaming disabled.
#[hook(addr = 0x405AC0, conv = cdecl, name = "CIplStore::AddIplSlot", expect = "E9 AB 69 16 01")]
pub fn add_ipl_slot(name: GPtr<u8>) -> i32 {
    let d = nonnull(ipl_pool_new(pool()), "CIplStore::AddIplSlot");
    field::<f32, _>(d, 8).write(-1.0e6);
    field::<f32, _>(d, 4).write(-1.0e6);
    w(d, LAST_BUILDING).write(i16::MIN);
    w(d, LAST_DUMMY).write(i16::MIN);
    b(d, LOADED).write(0);
    b(d, LOAD_REQUESTED).write(0);
    field::<f32, _>(d, 0).write(1.0e6);
    field::<f32, _>(d, 0xC).write(1.0e6);
    w(d, FIRST_BUILDING).write(i16::MAX);
    w(d, FIRST_DUMMY).write(i16::MAX);
    w(d, STATIC_IDX).write(-1);
    b(d, IS_INTERIOR).write(0);
    b(d, IS_LARGE).write(0);
    let mut k = 0;
    loop {
        let c = name.byte_add(k).read();
        b(d, NAME + k).write(c);
        k += 1;
        if c == 0 {
            break;
        }
    }
    b(d, IGNORE_WHEN_DELETED).write(0);
    b(d, DISABLE_DYNAMIC).write(1);
    pool_index(d.cast(), pool().read().objects.cast(), 0x34)
}

/// The inlined `CPool::Delete(def)` (map |= 0x80, first_free = idx when lower; idx from the slot
/// pointer, which is null for a free slot).
fn pool_delete_slot(d: GPtr<IplDef>) {
    let p = pool();
    let h = p.read();
    let idx = pool_index(d.cast(), h.objects.cast(), 0x34);
    let m = h.byte_map.wrapping_index(idx);
    m.write(m.read() | 0x80);
    let ff = field::<i32, _>(p, 0x0C);
    if idx < ff.read() {
        ff.write(idx);
    }
}

/// `CIplStore::RemoveIplSlot(int)` (stub 0x405B60 → .HOODLUM 0x156C5F0, cdecl).
#[hook(addr = 0x405B60, conv = cdecl, name = "CIplStore::RemoveIplSlot", expect = "E9 8B 6A 16 01")]
pub fn remove_ipl_slot(i: i32) {
    let d = nonnull(at(i), "CIplStore::RemoveIplSlot");
    if b(d, LOADED).read() != 0 {
        remove_ipl_call(i);
    }
    quad_delete_item(QUAD_TREE.get(), at(i).cast());
    pool_delete_slot(at(i));
}

/// What both IPL loaders do with one loaded instance: IPL index, LOD link (`++lod->numLodChildren`).
/// Returns the entity. `track` = LoadIplBoundingBox's extras (ppCurrIplInstance).
fn link_lod(e: GPtr<u8>, slot: i32, lods: GPtr<GPtr<u8>>, track: bool) {
    let lod_index = field::<i32, _>(e, E_LOD).read();
    field::<u8, _>(e, E_IPL_INDEX).write(slot as u8);
    if lod_index != -1 {
        let lod = lods.wrapping_index(lod_index).read();
        field::<GPtr<u8>, _>(e, E_LOD).write(lod);
        let n = field::<u8, _>(nonnull(lod, "CIplStore: LOD entity"), LOD_NUM_CHILDREN);
        n.write(n.read().wrapping_add(1));
        if track {
            let pp = CURR_IPL_INSTANCE.get();
            if !pp.is_null() {
                pp.write(e);
                CURR_IPL_INSTANCE.set(pp.wrapping_index(1));
            }
        }
    } else {
        field::<GPtr<u8>, _>(e, E_LOD).write(GPtr::NULL);
    }
}

/// `isLarge` when the model's draw distance is > 150 (`fcomp; test ah,0x41; jne`: NaN not).
fn mark_large(d: GPtr<IplDef>, e: GPtr<u8>) {
    let mi = model_info(field::<i16, _>(e, E_MODEL).read() as i32);
    let dd = field::<f32, _>(mi, 0x18).read();
    if !X87::from_f32(dd).fcom(X87::from_f32(150.0)).test_any(0x41) {
        b(d, IS_LARGE).write(1);
    }
}

/// The text IPL walk of both loaders: `inst` .. `end` sections (`LoadLine` on the argument slots).
fn for_each_text_instance(mut data: GPtr<u8>, mut size: i32, mut f: impl FnMut(GPtr<u8>)) {
    let mut in_section = false;
    loop {
        let line = load_line(GPtr::from_raw(&raw mut data), GPtr::from_raw(&raw mut size));
        if line.is_null() {
            return;
        }
        let c = |k: isize| line.byte_add(k).read();
        if c(0) == 0 || c(0) == b'#' {
            continue;
        }
        if !in_section {
            if c(0) == b'i' && c(1) == b'n' && c(2) == b's' && c(3) == b't' {
                in_section = true;
            }
            continue;
        }
        if c(0) == b'e' && c(1) == b'n' && c(2) == b'd' {
            in_section = false;
            continue;
        }
        f(line);
    }
}

fn lods_of(d: GPtr<IplDef>) -> GPtr<GPtr<u8>> {
    let s = w(d, STATIC_IDX).read();
    if s == -1 { GPtr::NULL } else { entity_array(s as i32) }
}

/// `CIplStore::LoadIplBoundingBox(int, uint8* data, int size)` (0x405C00, cdecl, AL = 1).
#[hook(addr = 0x405C00, conv = cdecl, name = "CIplStore::LoadIplBoundingBox", expect = "A1 B0 3F 8E 00")]
pub fn load_ipl_bounding_box(slot: i32, data: GPtr<u8>, size: i32) -> u8 {
    let d = nonnull(at(slot), "CIplStore::LoadIplBoundingBox");
    let lods = lods_of(d);
    if strncmp(data, image(STR_BNRY), 4) == 0 {
        let count = field::<i16, _>(data, 4).read();
        if count <= 0 {
            return 1;
        }
        let rel = field::<u32, _>(data, 0x1C);
        rel.write(rel.read().wrapping_add(data.addr() as u32));
        let mut i: u32 = 0;
        while i < field::<i16, _>(data, 4).read() as i32 as u32 {
            let inst = GPtr::<u8>::from_addr(rel.read().wrapping_add(i * 0x28) as usize);
            let e = load_object_instance_bin(inst, GPtr::NULL);
            link_lod(e, slot, lods, true);
            mark_large(d, e);
            entity_add(e);
            include_entity_call(slot, e);
            let mut r = [0f32; 4];
            let r = entity_get_bound_rect(e, GPtr::from_raw(r.as_mut_ptr()).cast()).cast::<f32>();
            // inline Restrict: each bound assigned only on a strict, ordered compare
            let bb: GPtr<f32> = d.cast();
            if r.read() < bb.read() {
                bb.write(r.read());
            }
            if r.wrapping_index(2).read() > bb.wrapping_index(2).read() {
                bb.wrapping_index(2).write(r.wrapping_index(2).read());
            }
            if r.wrapping_index(3).read() < bb.wrapping_index(3).read() {
                bb.wrapping_index(3).write(r.wrapping_index(3).read());
            }
            if r.wrapping_index(1).read() > bb.wrapping_index(1).read() {
                bb.wrapping_index(1).write(r.wrapping_index(1).read());
            }
            i += 1;
        }
        return 1;
    }
    for_each_text_instance(data, size, |line| {
        let e = load_object_instance_text(line);
        link_lod(e, slot, lods, true);
        mark_large(d, e);
        entity_add(e);
        include_entity_call(slot, e);
        let mut r = [0f32; 4];
        let r = entity_get_bound_rect(e, GPtr::from_raw(r.as_mut_ptr()).cast());
        rect_restrict(d.cast(), r);
    });
    1
}

/// `CIplStore::Initialise()` (0x405EC0, cdecl).
#[hook(addr = 0x405EC0, conv = cdecl, name = "CIplStore::Initialise", expect = "6A FF 68 86 71 83 00")]
pub fn initialise() {
    // CRect(-3000, 3000, 3000, -3000) in memory order left, +4, right, +0xC
    let mut world = [-3000.0f32, 3000.0, 3000.0, -3000.0];
    IPLS_NEEDED_AT_POSN.set(0);
    if pool().is_null() {
        let m = operator_new(0x14);
        let p = if m.is_null() { GPtr::NULL } else { ipl_pool_ctor(m, 0x100, image(STR_IPL_FILES)) };
        IPL_POOL.set(p.cast());
    }
    add_ipl_slot_call(image(STR_GENERIC));
    let node = quad_operator_new(0x28);
    let node = if node.is_null() {
        GPtr::NULL
    } else {
        quad_ctor(node, GPtr::from_raw(world.as_mut_ptr()).cast(), 3)
    };
    NUM_IPL_ENTITY_INDEX_ARRAYS.set(0);
    CURR_IPL_INSTANCE.set(GPtr::NULL);
    QUAD_TREE.set(node);
}

/// `CIplStore::Shutdown()` (stub 0x405FA0 → .HOODLUM 0x15610B0, cdecl): the pool's arrays go to
/// `operator delete[]` (the first through an obfuscated call that resolves to 0x8213AE), the pool
/// to `free`; `ms_pPool`, the entity index arrays and the quad tree are released and cleared.
#[hook(addr = 0x405FA0, conv = cdecl, name = "CIplStore::Shutdown", expect = "E9 0B B1 15 01")]
pub fn shutdown() {
    remove_all_ipls_call();
    for i in 0..256 {
        if used(i).is_some() {
            remove_ipl_slot_call(i);
        }
    }
    let p = pool();
    if !p.is_null() {
        let h = p.read();
        if h.size > 0 {
            if h.owns_arrays.get() {
                operator_delete_array(h.objects.cast());
                operator_delete_array(p.read().byte_map);
            }
            field::<u32, _>(p, 0).write(0);
            field::<u32, _>(p, 4).write(0);
            field::<u32, _>(p, 8).write(0);
            field::<u32, _>(p, 0xC).write(0);
        }
        free(p.cast());
    }
    IPL_POOL.set(GPtr::NULL);
    let mut i = 0;
    while i < NUM_IPL_ENTITY_INDEX_ARRAYS.get() {
        operator_delete_array(entity_array(i).cast());
        i += 1;
    }
    NUM_IPL_ENTITY_INDEX_ARRAYS.set(0);
    let q = QUAD_TREE.get();
    if !q.is_null() {
        quad_dtor(q);
        quad_operator_delete(q);
    }
    QUAD_TREE.set(GPtr::NULL);
}

/// `CIplStore::LoadIpl(int, uint8* data, int size)` (0x406080, cdecl, AL).
#[hook(addr = 0x406080, conv = cdecl, name = "CIplStore::LoadIpl", expect = "8B 0D B0 3F 8E 00")]
pub fn load_ipl(slot: i32, data: GPtr<u8>, size: i32) -> u8 {
    let d = at(slot);
    if d.is_null() {
        null_deref("CIplStore::LoadIpl");
    }
    if fl(d, 0) > fl(d, 2) || fl(d, 3) > fl(d, 1) {
        let ok = load_ipl_bounding_box_call(slot, data, size);
        if ok != 0 {
            let k = if b(d, IS_LARGE).read() != 0 { 350.0 } else { 200.0 };
            let bb: GPtr<f32> = d.cast();
            bb.write(X87::from_f32(bb.read()).fsub_m(k).to_f32());
            bb.wrapping_index(2).write(X87::from_f32(bb.wrapping_index(2).read()).fadd_m(k).to_f32());
            bb.wrapping_index(3).write(X87::from_f32(bb.wrapping_index(3).read()).fsub_m(k).to_f32());
            bb.wrapping_index(1).write(X87::from_f32(bb.wrapping_index(1).read()).fadd_m(k).to_f32());
            col_accel_set_ipl_def(slot, d.read());
            quad_add_item(QUAD_TREE.get(), d.cast(), d.cast());
        }
        b(d, LOADED).write(1);
        return ok;
    }
    let lods = lods_of(d);
    if strncmp(data, image(STR_BNRY), 4) == 0 {
        let count = field::<i16, _>(data, 4).read();
        if count > 0 {
            let rel = field::<u32, _>(data, 0x1C);
            rel.write(rel.read().wrapping_add(data.addr() as u32));
            let mut i: u32 = 0;
            while i < field::<i16, _>(data, 4).read() as i32 as u32 {
                let inst = GPtr::<u8>::from_addr(rel.read().wrapping_add(i * 0x28) as usize);
                let e = load_object_instance_bin(inst, GPtr::NULL);
                link_lod(e, slot, lods, false);
                entity_add(e);
                include_entity_call(slot, e);
                i += 1;
            }
        }
        let cg_count = field::<i16, _>(data, 0x14).read();
        if cg_count > 0 {
            let rel = field::<u32, _>(data, 0x3C);
            rel.write(rel.read().wrapping_add(data.addr() as u32));
            let mut i: u32 = 0;
            while i < field::<i16, _>(data, 0x14).read() as i32 as u32 {
                let cg = GPtr::<u8>::from_addr(rel.read().wrapping_add(i * 0x30) as usize);
                load_car_generator(cg, slot);
                i += 1;
            }
        }
        b(d, LOADED).write(1);
        return 1;
    }
    for_each_text_instance(data, size, |line| {
        let e = load_object_instance_text(line);
        link_lod(e, slot, lods, false);
        entity_add(e);
        include_entity_call(slot, e);
    });
    b(d, LOADED).write(1);
    1
}

/// `CIplStore::Save()` (0x5D5420, cdecl, AL = 1): the count 256, then one byte per slot 1..255.
#[hook(addr = 0x5D5420, conv = cdecl, name = "CIplStore::Save", expect = "83 EC 08 56 57")]
pub fn save() -> u8 {
    let mut n: i32 = 0x100;
    save_data_to_work_buffer(GPtr::from_raw((&raw mut n).cast()), 4);
    for i in 1..256 {
        let mut v: u8 = match used(i) {
            Some(d) => (b(d, LOADED).read() != 0 && b(d, DISABLE_DYNAMIC).read() != 0) as u8,
            None => 0,
        };
        save_data_to_work_buffer(GPtr::from_raw(&raw mut v), 1);
    }
    1
}

/// `CIplStore::Load()` (0x5D54A0, cdecl, AL = 1): the stored count is read and ignored.
#[hook(addr = 0x5D54A0, conv = cdecl, name = "CIplStore::Load", expect = "83 EC 08 53 56")]
pub fn load() -> u8 {
    let mut n: i32 = 0;
    load_data_from_work_buffer(GPtr::from_raw((&raw mut n).cast()), 4);
    for i in 1..256 {
        let d = at(i);
        let mut v: u8 = 0;
        load_data_from_work_buffer(GPtr::from_raw(&raw mut v), 1);
        if d.is_null() {
            continue;
        }
        if v != 0 {
            request_ipl_and_ignore_call(i);
        } else if b(d, LOADED).read() != 0 && b(d, DISABLE_DYNAMIC).read() != 0 {
            remove_ipl_and_ignore_call(i);
        }
    }
    1
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_ADD_IPLS_NEEDED_AT_POSN,
    HOOK_CLEAR_IPLS_NEEDED_AT_POSN,
    HOOK_SET_IF_INTERIOR_IPL_IS_REQUIRED,
    HOOK_SET_IF_IPL_IS_REQUIRED,
    HOOK_SET_IF_IPL_IS_REQUIRED_REDUCED_BB,
    HOOK_SET_IPLS_REQUIRED,
    HOOK_GET_NEW_IPL_ENTITY_INDEX_ARRAY,
    HOOK_GET_IPL_ENTITY_INDEX_ARRAY,
    HOOK_GET_IPL_NAME,
    HOOK_SET_IS_INTERIOR,
    HOOK_FIND_IPL_SLOT,
    HOOK_REMOVE_IPL,
    HOOK_GET_BOUNDING_BOX,
    HOOK_INCLUDE_ENTITY,
    HOOK_ENABLE_DYNAMIC_STREAMING,
    HOOK_SETUP_RELATED_IPLS,
    HOOK_REMOVE_RELATED_IPLS,
    HOOK_LOAD_IPLS,
    HOOK_ENSURE_IPLS_ARE_IN_MEMORY,
    HOOK_REQUEST_IPLS,
    HOOK_HAVE_IPLS_LOADED,
    HOOK_REMOVE_ALL_IPLS,
    HOOK_LOAD_ALL_REMAINING_IPLS,
    HOOK_REQUEST_IPL_AND_IGNORE,
    HOOK_REMOVE_IPL_AND_IGNORE,
    HOOK_REMOVE_IPL_WHEN_FAR_AWAY,
    HOOK_ADD_IPL_SLOT,
    HOOK_REMOVE_IPL_SLOT,
    HOOK_LOAD_IPL_BOUNDING_BOX,
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_LOAD_IPL,
    HOOK_SAVE,
    HOOK_LOAD,
];
