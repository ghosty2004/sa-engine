//! WRITER-MVP-C batch `mvpc-world-1`: the `CWorld` line / sphere tests on the boot/loading path
//! (`ProcessVerticalLine` and its sector variants, the sector parts of `GetIsLineOfSightClear` and
//! `ProcessLineOfSight`, `TestSphereAgainstWorld`). Ported from the 1.0 US bytes (SecuROM fragments
//! inlined; gta-reversed `World.cpp` a reading aid only). Unverified (docs/verify/HOWTO.md).
//!
//! Entities: +0x14 matrix, +0x4 placement, +0x1C flags (bit 0 bUsesCollision), +0x22 i16 model index,
//! +0x2C u16 scan code, +0x36 type (low 3 bits: 2 vehicle, 3 ped). Sector lists: node +0 item, +4 next.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr};
use super::cworld::{CURRENT_SCAN_CODE, IGNORE_ENTITY, INCLUDE_BIKERS, INCLUDE_DEAD_PEDS, REPEAT_SECTORS, SECTORS};
use crate::collision::fpu::Fpu;
use crate::collision::mvpc_coll1::local;
use crate::physical::base::floor_f64;

/// `CWorld::ms_fWeaponSpreadRate`... the float at 0xB7CD64 the line-of-sight sector code saves,
/// clears and restores
const SPREAD_RATE: usize = 0xB7CD64;
/// `CWorld::ms_nTotalCrossings`... the dword at 0xB7CD60 the vertical-line and line-of-sight lists add
/// `CCollision::ms_iProcessLineNumCrossings` to
const TOTAL_CROSSINGS: usize = 0xB7CD60;
/// `CCollision::ms_iProcessLineNumCrossings`
const NUM_CROSSINGS: usize = 0x9655D0;
/// `CWorld::m_aTempColPts` (0xB9B250)
const TEMP_COL_PTS: u32 = 0xB9B250;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CCollision::ProcessVerticalLine(...)` (0x417BF0, AL)
    #[allow(clippy::too_many_arguments)]
    fn coll_process_vertical_line(line: GPtr<u8>, m: GPtr<u8>, model: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>,
        see: u32, shoot: u32, poly: GPtr<u8>) -> u8 = cdecl 0x417BF0 => unported;
    /// `CCollision::TestLineOfSight(...)` (0x417730, AL)
    fn coll_test_line_of_sight(line: GPtr<u8>, m: GPtr<u8>, model: GPtr<u8>, see: u32, shoot: u32) -> u8 = cdecl 0x417730 => unported;
    /// `CWorld::CameraToIgnoreThisObject(CEntity*)` (0x563F40, AL)
    fn camera_to_ignore_this_object(e: GPtr<u8>) -> u8 = cdecl 0x563F40 => unported;
    /// `CColLine::CColLine(const CVector&, const CVector&)` (0x40EF50, `ret 8`)
    fn col_line_ctor(l: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x40EF50 => unported;
    /// `CWorld::ProcessVerticalLineSector(...)` (0x564500, AL)
    #[allow(clippy::too_many_arguments)]
    fn process_vertical_line_sector_call(s: GPtr<u8>, r: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, out: GPtr<u8>,
        b: u8, v: u8, p: u8, o: u8, d: u8, see: u8, poly: GPtr<u8>) -> u8 = cdecl 0x564500 => process_vertical_line_sector;
    /// `CWorld::ProcessVerticalLineSectorList(...)` (0x5632B0, AL)
    fn process_vertical_line_sector_list_call(l: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>,
        out: GPtr<u8>, see: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5632B0 => process_vertical_line_sector_list;
    /// `CWorld::GetIsLineOfSightSectorListClear(...)` (0x564970, AL)
    fn sector_list_clear_call(l: GPtr<u8>, line: GPtr<u8>, see: u8, cam: u8) -> u8 = cdecl 0x564970 => get_is_line_of_sight_sector_list_clear;
    /// `CWorld::ProcessLineOfSightSectorList(...)` (0x566EE0, AL)
    #[allow(clippy::too_many_arguments)]
    fn los_sector_list(l: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>, out: GPtr<u8>, see: u32, cam: u32,
        shoot: u32) -> u8 = cdecl 0x566EE0 => unported;
    /// `CWorld::TestSphereAgainstSectorList(...)` (0x566140, `CVector` by value)
    fn sphere_sector_list_call(l: GPtr<u8>, x: f32, y: f32, z: f32, r: f32, ignore: GPtr<u8>, cam: u8) -> GPtr<u8>
        = cdecl 0x566140 => test_sphere_against_sector_list;
    /// `CWorld::GetIsLineOfSightSectorClear(...)` (0x568AD0, AL)
    #[allow(clippy::too_many_arguments)]
    fn get_is_line_of_sight_sector_clear_call(s: GPtr<u8>, r: GPtr<u8>, line: GPtr<u8>, b: u8, v: u8, p: u8, o: u8, d: u8,
        see: u8, cam: u8) -> u8 = cdecl 0x568AD0 => get_is_line_of_sight_sector_clear;
    /// `CWorld::ProcessLineOfSightSector(...)` (0x56B5E0, AL)
    #[allow(clippy::too_many_arguments)]
    fn process_line_of_sight_sector_call(s: GPtr<u8>, r: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>,
        out: GPtr<u8>, b: u8, v: u8, p: u8, o: u8, d: u8, see: u8, cam: u8, shoot: u8) -> u8
        = cdecl 0x56B5E0 => process_line_of_sight_sector;
    /// `CWorld::FindObjectsKindaCollidingSectorList(...)` (0x565000)
    fn kinda_colliding_list_call(l: GPtr<u8>, c: GPtr<u8>, r: f32, two_d: u8, count: GPtr<u8>, max: i16, out: GPtr<u8>)
        = cdecl 0x565000 => find_objects_kinda_colliding_sector_list;
    /// `CWorld::FindObjectsOfTypeInRangeSectorList(...)` (0x5635C0)
    #[allow(clippy::too_many_arguments)]
    fn objects_of_type_in_range_list(model: i32, l: GPtr<u8>, p: GPtr<u8>, r: f32, two_d: u8, count: GPtr<u8>, max: i16,
        out: GPtr<u8>) = cdecl 0x5635C0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470; the port is unhookable)
    fn clear_scan_codes() = cdecl 0x563470 => super::cworld::clear_scan_codes;
    /// `CColModel::CColModel()` (0x40FB60)
    fn col_model_ctor(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CColModel::~CColModel()` (0x40F700)
    fn col_model_dtor(m: GPtr<u8>) = thiscall 0x40F700 => unported;
    /// `CCollisionData::CCollisionData()` (0x40F030)
    fn col_data_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x40F030 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40, `ret 0xC`)
    fn matrix_set_translate(m: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x59AF40 => unported;
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534250, `ret 4`, returns `out`)
    fn get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x534250 => unported;
    /// `CCollision::ProcessColModels(...)` (0x4185C0, count of colpoints)
    #[allow(clippy::too_many_arguments)]
    fn process_col_models(m1: GPtr<u8>, c1: GPtr<u8>, m2: GPtr<u8>, c2: GPtr<u8>, cps: u32, lcps: u32, ld: u32,
        b: u32) -> i32 = cdecl 0x4185C0 => unported;
}

/// `if (ms_nCurrentScanCode >= 0xFFFF) { ClearScanCodes(); ms_nCurrentScanCode = 1; } else ++` (the
/// inlined `SetNextScanCode`; `cmp word, -1; jae`).
fn next_scan_code() {
    let sc = CURRENT_SCAN_CODE.get();
    if sc == 0xFFFF {
        clear_scan_codes();
        CURRENT_SCAN_CODE.set(1);
    } else {
        CURRENT_SCAN_CODE.set(sc + 1);
    }
}

/// `_ftol(floor((double)(v * 0.02f + 60.0f)))` of an x87 value.
fn sector_coord(v: X87) -> i32 {
    let s = v.fmul_m(g::<f32>(0x858B38).read()).fadd_m(g::<f32>(0x858B34).read());
    X87::from_f64_exact(floor_f64(s.to_f64())).ftol() as u32 as i32
}

/// `v <= 0 → 0` (`test; setle; dec; and`), then `>= 0x77 → 0x77`.
fn clamp_sector(v: i32) -> i32 {
    let v = if v <= 0 { 0 } else { v };
    if v >= 0x77 { 0x77 } else { v }
}

fn sector_at(cx: i32, cy: i32) -> GPtr<u8> {
    g::<u8>(SECTORS.wrapping_add(((cy * 0x78 + cx) * 8) as usize))
}

fn repeat_at(x: i32, y: i32) -> GPtr<u8> {
    g::<u8>(REPEAT_SECTORS.wrapping_add(((((y & 0xF) << 4) + (x & 0xF)) * 12) as usize))
}

/// The entity's matrix, allocated and filled from its placement first when it has none.
fn ensure_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `fld a; fcomp [b]; test ah, 5; jp` not taken: `a < b` (f32, NaN false).
fn less(a: f32, b: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(a);
    f.fcomp_m(b);
    !f.jp(5)
}

fn add_crossings() {
    let t = g::<u32>(TOTAL_CROSSINGS);
    t.write(t.read().wrapping_add(g::<u32>(NUM_CROSSINGS).read()));
}

/// `CWorld::ProcessVerticalLine(const CVector& origin, float distance, CColPoint& colPoint,
/// CEntity*& outEntity, bool buildings, bool vehicles, bool peds, bool objects, bool dummies,
/// bool doSeeThroughCheck, CStoredCollPoly* poly)` (0x5674E0, AL). Next scan code (the
/// `nop; jmp` fragment holds the compare), `ms_nTotalCrossings = 0`, sector x/y of the origin
/// (`floor(v * 0.02 + 60)`), the line from the origin to `(x, y, distance)`, and only that one sector
/// (clamped) and its repeat sector through `ProcessVerticalLineSector`.
/// Stack: the clamped x is stored into the `origin` argument slot (S+4).
#[hook(addr = 0x5674E0, conv = cdecl, name = "CWorld::ProcessVerticalLine", expect = "83 EC 2C 90 90 90 90",
       unverified = "mvpc-world-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "stores the clamped sector x into its origin argument slot (S+4)")]
#[allow(clippy::too_many_arguments)]
pub fn process_vertical_line(origin: GPtr<u8>, distance: f32, cp: GPtr<u8>, out: GPtr<u8>, b: u8, v: u8, p: u8,
    o: u8, d: u8, see: u8, poly: GPtr<u8>) -> u8 {
    next_scan_code();
    g::<u32>(TOTAL_CROSSINGS).write(0);
    let sx = sector_coord(X87::from_f32(rd::<f32>(origin, 0)));
    let sy = sector_coord(X87::from_f32(rd::<f32>(origin, 4)));
    let mut end = [rd::<u32>(origin, 0), rd::<u32>(origin, 4), distance.to_bits()];
    let cx = clamp_sector(sx);
    let cy = clamp_sector(sy);
    let mut lb = [0u32; 8];
    let line = col_line_ctor(local(&mut lb), origin, local(&mut end));
    process_vertical_line_sector_call(sector_at(cx, cy), repeat_at(sx, sy), line, cp, out, b, v, p, o, d, see, poly)
}

/// `CWorld::ProcessVerticalLineSector(CSector&, CRepeatSector&, const CColLine&, CColPoint&,
/// CEntity*& outEntity, bool buildings, bool vehicles, bool peds, bool objects, bool dummies,
/// bool doSeeThroughCheck, CStoredCollPoly* poly)` (0x564500, AL): minimum 1.0; buildings
/// (sector +0), vehicles / peds / objects (repeat +0 / +4 / +8), dummies (sector +4) through
/// `ProcessVerticalLineSectorList`; true when the minimum dropped below 1.0.
#[hook(addr = 0x564500, conv = cdecl, name = "CWorld::ProcessVerticalLineSector", expect = "51 8A 44 24 1C 84 C0 53",
       unverified = "mvpc-world-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
#[allow(clippy::too_many_arguments)]
pub fn process_vertical_line_sector(s: GPtr<u8>, r: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, out: GPtr<u8>, b: u8,
    v: u8, p: u8, o: u8, d: u8, see: u8, poly: GPtr<u8>) -> u8 {
    let mut minb = [0x3F80_0000u32];
    let min = local(&mut minb);
    let run = |list: GPtr<u8>| {
        process_vertical_line_sector_list_call(list, line, cp, min, out, see, poly);
    };
    if b != 0 {
        run(s);
    }
    if v != 0 {
        run(r);
    }
    if p != 0 {
        run(r.byte_add(4));
    }
    if o != 0 {
        run(r.byte_add(8));
    }
    if d != 0 {
        run(s.byte_add(4));
    }
    less(f32::from_bits(minb[0]), g::<f32>(0x858624).read()) as u8
}

/// `CWorld::ProcessVerticalLineSectorList(CPtrListDoubleLink&, const CColLine&, CColPoint&,
/// float& minDist, CEntity*& outEntity, bool doSeeThroughCheck, CStoredCollPoly* poly)` (0x5632B0,
/// AL). A local minimum starts at `*minDist`; every entity not yet scanned, using collision and not
/// `pIgnoreEntity` gets the scan code, a matrix if it has none, and
/// `CCollision::ProcessVerticalLine(line, matrix, colModel, colPoint, localMin, poly, false, see)`
/// (the original passes the poly pointer as the sixth and the see-through flag as the eighth
/// argument); a hit stores the entity and adds `ms_iProcessLineNumCrossings` to the total. True (and
/// `*minDist` updated) when the local minimum ends below `*minDist`.
#[hook(addr = 0x5632B0, conv = cdecl, name = "CWorld::ProcessVerticalLineSectorList<CPtrListDoubleLink<CPhysical*>>",
       expect = "51 8B 54 24 08 8B 4C 24", unverified = "mvpc-world-1", route = "L2", hazards = "none",
       recs = "grove-walk,driving", note = "argument order of the CCollision::ProcessVerticalLine call kept: (..., poly, 0, see)")]
pub fn process_vertical_line_sector_list(list: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>, out: GPtr<u8>,
    see: u8, poly: GPtr<u8>) -> u8 {
    let mut lmb = [rd::<u32>(min, 0)];
    let lm = local(&mut lmb);
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 || e == IGNORE_ENTITY.get() {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let m = ensure_matrix(e);
        let model = get_col_model(e);
        if coll_process_vertical_line(line, m, model, cp, lm, poly.addr() as u32, 0, GPtr::<u8>::from_addr(see as usize))
            != 0
        {
            wr::<GPtr<u8>>(out, 0, e);
            add_crossings();
        }
    }
    let v = rd::<u32>(lm, 0);
    if less(f32::from_bits(v), rd::<f32>(min, 0)) {
        wr::<u32>(min, 0, v);
        return 1;
    }
    0
}

/// `CWorld::GetIsLineOfSightSectorClear(CSector&, CRepeatSector&, const CColLine&, bool buildings,
/// bool vehicles, bool peds, bool objects, bool dummies, bool doSeeThroughCheck,
/// bool doIgnoreCameraCheck)` (0x568AD0, AL): buildings (sector +0), vehicles (repeat +0), peds
/// (repeat +4), objects (repeat +8, with the camera-ignore flag), dummies (sector +4) through
/// `GetIsLineOfSightSectorListClear`; false at the first blocked list.
#[hook(addr = 0x568AD0, conv = cdecl, name = "CWorld::GetIsLineOfSightSectorClear", expect = "8A 44 24 10 84 C0 55 8B",
       unverified = "mvpc-world-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
#[allow(clippy::too_many_arguments)]
pub fn get_is_line_of_sight_sector_clear(s: GPtr<u8>, r: GPtr<u8>, line: GPtr<u8>, b: u8, v: u8, p: u8, o: u8, d: u8,
    see: u8, cam: u8) -> u8 {
    if b != 0 && sector_list_clear_call(s, line, see, 0) == 0 {
        return 0;
    }
    if v != 0 && sector_list_clear_call(r, line, see, 0) == 0 {
        return 0;
    }
    if p != 0 && sector_list_clear_call(r.byte_add(4), line, see, 0) == 0 {
        return 0;
    }
    if o != 0 && sector_list_clear_call(r.byte_add(8), line, see, cam) == 0 {
        return 0;
    }
    if d != 0 && sector_list_clear_call(s.byte_add(4), line, see, 0) == 0 {
        return 0;
    }
    1
}

/// `CWorld::GetIsLineOfSightSectorListClear(CPtrListDoubleLink&, const CColLine&,
/// bool doSeeThroughCheck, bool doCameraIgnoreCheck)` (0x564970, AL; the scan code load sits in a
/// `nop; jmp` fragment). Every entity not yet scanned and using collision gets the scan code (before
/// the `pIgnoreEntity` and camera-ignore tests), a matrix if it has none, and
/// `CCollision::TestLineOfSight(line, matrix, colModel, see, false)`; a hit → false.
#[hook(addr = 0x564970, conv = cdecl, name = "CWorld::GetIsLineOfSightSectorListClear<CPtrListDoubleLink<CPhysical*>>",
       expect = "8B 44 24 04 53 55 8B 28", unverified = "mvpc-world-1", route = "L2", hazards = "none",
       recs = "grove-walk,driving")]
pub fn get_is_line_of_sight_sector_list_clear(list: GPtr<u8>, line: GPtr<u8>, see: u8, cam: u8) -> u8 {
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        if e == IGNORE_ENTITY.get() {
            continue;
        }
        if cam != 0 && camera_to_ignore_this_object(e) != 0 {
            continue;
        }
        let m = ensure_matrix(e);
        let model = get_col_model(e);
        if coll_test_line_of_sight(line, m, model, see as u32, 0) != 0 {
            return 0;
        }
    }
    1
}

/// `CWorld::ProcessLineOfSightSector(CSector&, CRepeatSector&, const CColLine&, CColPoint&,
/// float& minDist, CEntity*& outEntity, bool buildings, bool vehicles, bool peds, bool objects,
/// bool dummies, bool doSeeThroughCheck, bool doCameraIgnoreCheck, bool doShootThroughCheck)`
/// (0x56B5E0, AL). Saves `bIncludeDeadPeds` (0xB7CD71, its load in a `nop; jmp` fragment) and
/// `bIncludeBikers` (0xB7CD6F) as booleans and the float at 0xB7CD64, clears all three; buildings
/// (sector +0); the saved float is put back only when it is `> 0`; vehicles (repeat +0); peds
/// (repeat +4) with the two flags set again when they were set, cleared afterwards; objects (repeat +8,
/// with the camera-ignore flag); dummies (sector +4); every list through
/// `ProcessLineOfSightSectorList(list, line, colPoint, localMin, outEntity, see, cam-or-false, shoot)`.
/// The two flags are restored; true (and `*minDist` updated) when the local minimum is below it.
#[hook(addr = 0x56B5E0, conv = cdecl, name = "CWorld::ProcessLineOfSightSector", expect = "83 EC 0C 8B 44 24 20 8B",
       unverified = "mvpc-world-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
#[allow(clippy::too_many_arguments)]
pub fn process_line_of_sight_sector(s: GPtr<u8>, r: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>, out: GPtr<u8>,
    b: u8, v: u8, p: u8, o: u8, d: u8, see: u8, cam: u8, shoot: u8) -> u8 {
    let mut lmb = [rd::<u32>(min, 0)];
    let lm = local(&mut lmb);
    let dead = INCLUDE_DEAD_PEDS.get() != 0;
    let bikers = INCLUDE_BIKERS.get() != 0;
    let spread = g::<u32>(SPREAD_RATE).read();
    INCLUDE_DEAD_PEDS.set(0);
    INCLUDE_BIKERS.set(0);
    g::<u32>(SPREAD_RATE).write(0);
    let (see, shoot) = (see as u32, shoot as u32);
    if b != 0 {
        los_sector_list(s, line, cp, lm, out, see, 0, shoot);
    }
    let mut f = Fpu::new();
    f.fld(f32::from_bits(spread));
    f.fcomp_m(g::<f32>(0x858B50).read());
    if !f.jne(0x41) {
        g::<u32>(SPREAD_RATE).write(spread);
    }
    if v != 0 {
        los_sector_list(r, line, cp, lm, out, see, 0, shoot);
    }
    if p != 0 {
        if dead {
            INCLUDE_DEAD_PEDS.set(1);
        }
        if bikers {
            INCLUDE_BIKERS.set(1);
        }
        los_sector_list(r.byte_add(4), line, cp, lm, out, see, 0, shoot);
        INCLUDE_DEAD_PEDS.set(0);
        INCLUDE_BIKERS.set(0);
    }
    if o != 0 {
        los_sector_list(r.byte_add(8), line, cp, lm, out, see, cam as u32, shoot);
    }
    if d != 0 {
        los_sector_list(s.byte_add(4), line, cp, lm, out, see, 0, shoot);
    }
    INCLUDE_BIKERS.set(bikers as u8);
    INCLUDE_DEAD_PEDS.set(dead as u8);
    let lv = rd::<u32>(lm, 0);
    if less(f32::from_bits(lv), rd::<f32>(min, 0)) {
        wr::<u32>(min, 0, lv);
        return 1;
    }
    0
}

/// `CWorld::TestSphereAgainstWorld(CVector centre, float radius, CEntity* ignoreEntity, bool buildings,
/// bool vehicles, bool peds, bool objects, bool dummies, bool doCameraIgnoreCheck)` (0x569E20): the
/// sector rect of `centre ± radius` (x min, y min, x max, y max, each `floor(v * 0.02 + 60)` of the
/// unrounded x87 sum), the next scan code, then for y = min..=max (outer) and x = min..=max (inner) the
/// clamped sector and the repeat sector of (x, y): buildings (sector +0), vehicles / peds / objects
/// (repeat +0 / +4 / +8; objects with the camera-ignore flag), dummies (sector +4) through
/// `TestSphereAgainstSectorList`; the first entity found is returned, else null.
#[hook(addr = 0x569E20, conv = cdecl, name = "CWorld::TestSphereAgainstWorld", expect = "83 EC 1C D9 44 24 20 53",
       unverified = "mvpc-world-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
#[allow(clippy::too_many_arguments)]
pub fn test_sphere_against_world(x: f32, y: f32, z: f32, radius: f32, ignore: GPtr<u8>, b: u8, v: u8, p: u8, o: u8, d: u8,
    cam: u8) -> GPtr<u8> {
    let x0 = sector_coord(X87::from_f32(x).fsub_m(radius));
    let y0 = sector_coord(X87::from_f32(y).fsub_m(radius));
    let x1 = sector_coord(X87::from_f32(x).fadd_m(radius));
    let y1 = sector_coord(X87::from_f32(y).fadd_m(radius));
    next_scan_code();
    let mut sy = y0;
    while sy <= y1 {
        let mut sx = x0;
        while sx <= x1 {
            let sec = sector_at(clamp_sector(sx), clamp_sector(sy));
            let rep = repeat_at(sx, sy);
            let lists = [(b, sec, 0u8), (v, rep, 0), (p, rep.byte_add(4), 0), (o, rep.byte_add(8), cam),
                (d, sec.byte_add(4), 0)];
            for (flag, l, c) in lists {
                if flag != 0 {
                    let e = sphere_sector_list_call(l, x, y, z, radius, ignore, c);
                    if !e.is_null() {
                        return e;
                    }
                }
            }
            sx += 1;
        }
        sy += 1;
    }
    GPtr::NULL
}

/// `CWorld::TestSphereAgainstSectorList(CPtrListDoubleLink&, CVector centre, float radius,
/// CEntity* ignoreEntity, bool doCameraIgnoreCheck)` (0x566140; SEH frame, its first push in a
/// `.HOODLUM` fragment). Builds a local sphere `CColModel` (constructor, then box `-r..r`, bounding
/// sphere at 0 with radius r, its `CCollisionData` (constructed) with one sphere at 0 of radius r) and
/// a local matrix (attachment fields cleared, `SetTranslate(centre)`). Every entity not yet scanned
/// gets the scan code; skipped when it is `ignoreEntity`, does not use collision or (with the flag)
/// the camera ignores it. `dist = |centre - GetBoundCentre()|` (`(dx*dx + dy*dy) + dz*dz`, `fsqrt`);
/// unless `dist < radius + model's colModel radius` (the model info's col model +0x24) it is skipped;
/// else `ProcessColModels(local matrix, local model, matrix, GetColModel(), m_aTempColPts, 0, 0, 0)`
/// non-zero → that entity. (For vehicles the original also reads +0x590 and tests it against 0 and 4
/// at once, which never passes.) The local model's data pointer is cleared before its destructor runs;
/// the matrix destructor follows.
#[hook(addr = 0x566140, conv = cdecl, name = "CWorld::TestSphereAgainstSectorList<CPtrListDoubleLink<CPhysical*>>",
       expect = "6A FF E9 70 40 EA FF", unverified = "mvpc-world-1", route = "L2", hazards = "none",
       recs = "grove-walk,driving", note = "local CColModel/CCollisionData/CMatrix live in the Rust frame")]
pub fn test_sphere_against_sector_list(list: GPtr<u8>, x: f32, y: f32, z: f32, radius: f32, ignore: GPtr<u8>, cam: u8)
    -> GPtr<u8> {
    let mut modelb = [0u32; 0x30 / 4];
    let mut datab = [0u32; 0x30 / 4];
    let mut sphb = [0u32; 0x14 / 4];
    let mut matb = [0u32; 0x48 / 4];
    let mut bcb = [0u32; 3];
    let model = local(&mut modelb);
    let data = local(&mut datab);
    let sph = local(&mut sphb);
    let mat = local(&mut matb);
    wr::<u32>(mat, 0x40, 0);
    wr::<u32>(mat, 0x44, 0);
    col_model_ctor(model);
    col_data_ctor(data);
    let nr = -radius;
    let r = radius.to_bits();
    for i in 0..3 {
        wr::<f32>(model, 4 * i, nr);
        wr::<u32>(model, 0xC + 4 * i, r);
        wr::<u32>(model, 0x18 + 4 * i, 0);
        wr::<u32>(sph, 4 * i, 0);
    }
    wr::<u32>(model, 0x24, r);
    wr::<GPtr<u8>>(model, 0x2C, data);
    wr::<u32>(sph, 0xC, r);
    wr::<GPtr<u8>>(data, 8, sph);
    wr::<u16>(data, 0, 1);
    matrix_set_translate(mat, x, y, z);
    let mut found = GPtr::NULL;
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        if e == ignore || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        if cam != 0 && camera_to_ignore_this_object(e) != 0 {
            continue;
        }
        let bc = get_bound_centre(e, local(&mut bcb));
        let mut f = Fpu::new();
        f.fld(z);
        f.fsub_m(rd::<f32>(bc, 8));
        f.fld(y);
        f.fsub_m(rd::<f32>(bc, 4));
        f.fld(x);
        f.fsub_m(rd::<f32>(bc, 0));
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(e, 0x22) as isize as usize).read();
        let cm = rd::<GPtr<u8>>(mi, 0x14);
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fstp_st(3);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fsqrt();
        f.fld(rd::<f32>(cm, 0x24));
        f.fld(radius);
        f.fadd_st(1);
        f.fxch(1);
        f.fxch(2);
        f.fcompp();
        if f.jp(5) {
            continue;
        }
        let m = ensure_matrix(e);
        let em = get_col_model(e);
        if process_col_models(mat, model, m, em, TEMP_COL_PTS, 0, 0, 0) != 0 {
            found = e;
            break;
        }
        if rd::<u8>(e, 0x36) & 7 == 2 {
            // `cmp [e+0x590], 0; jne next; cmp eax, 4; jne next`: never passes
            let _ = rd::<u32>(e, 0x590);
        }
    }
    wr::<u32>(model, 0x2C, 0);
    col_model_dtor(model);
    matrix_dtor(mat);
    found
}

/// The sectors a line from `o` to `t` crosses, in the order `CWorld::GetIsLineOfSightClear` /
/// `ProcessLineOfSight` visit them (1.0 US bytes): one sector when both ends share it; same x: y from
/// the origin's sector towards the target's (ascending when `oy < ty`, unlike gta-reversed which walks
/// down from the target); same y: likewise in x; else the DDA with `disp = (t.y - o.y) / (t.x - o.x)`
/// (f32) and `endY = sector((float((x + 1 or x) - 60) * 50 - o.x) * disp + o.y)` (x87, PC 24): left to
/// right when `o.x < t.x` (NaN goes right to left), each column from the previous end y to the next,
/// ascending when `start < end`, else descending. `visit(x, y)` false stops the walk (returns false).
fn walk_sectors(o: GPtr<u8>, t: GPtr<u8>, mut visit: impl FnMut(i32, i32) -> bool) -> bool {
    let sc = |v: f32| sector_coord(X87::from_f32(v));
    let ox = sc(rd::<f32>(o, 0));
    let oy = sc(rd::<f32>(o, 4));
    let tx = sc(rd::<f32>(t, 0));
    let ty = sc(rd::<f32>(t, 4));
    let mut column = |x: i32, a: i32, b: i32| -> bool {
        if a < b {
            let mut y = a;
            while y <= b {
                if !visit(x, y) {
                    return false;
                }
                y += 1;
            }
        } else {
            let mut y = a;
            while y >= b {
                if !visit(x, y) {
                    return false;
                }
                y -= 1;
            }
        }
        true
    };
    if ox == tx {
        if oy == ty {
            return visit(ox, oy);
        }
        return column(ox, oy, ty);
    }
    if oy == ty {
        if ox < tx {
            let mut x = ox;
            while x <= tx {
                if !column(x, oy, oy) {
                    return false;
                }
                x += 1;
            }
        } else {
            let mut x = ox;
            while x >= tx {
                if !column(x, oy, oy) {
                    return false;
                }
                x -= 1;
            }
        }
        return true;
    }
    let mut f = Fpu::new();
    f.fld(rd::<f32>(t, 0));
    f.fcomp_m(rd::<f32>(o, 0));
    let right = !f.jne(0x41);
    let dy = X87::from_f32(rd::<f32>(t, 4)).fsub_m(rd::<f32>(o, 4));
    let dx = X87::from_f32(rd::<f32>(t, 0)).fsub_m(rd::<f32>(o, 0));
    let disp = dy.fdiv(dx).to_f32();
    let end_y = |col: i32| -> i32 {
        let v = X87::from_i32(col - 60)
            .fmul_m(g::<f32>(0x858B40).read())
            .fsub_m(rd::<f32>(o, 0))
            .fmul_m(disp)
            .fadd_m(rd::<f32>(o, 4));
        sector_coord(v)
    };
    if right {
        let mut end = end_y(ox + 1);
        if !column(ox, oy, end) {
            return false;
        }
        let mut x = ox + 1;
        while x < tx {
            let start = end;
            end = end_y(x + 1);
            if !column(x, start, end) {
                return false;
            }
            x += 1;
        }
        column(tx, end, ty)
    } else {
        let mut end = end_y(ox);
        if !column(ox, oy, end) {
            return false;
        }
        let mut x = ox - 1;
        while x > tx {
            let start = end;
            end = end_y(x);
            if !column(x, start, end) {
                return false;
            }
            x -= 1;
        }
        column(tx, end, ty)
    }
}

/// `CWorld::GetIsLineOfSightClear(const CVector& origin, const CVector& target, bool buildings,
/// bool vehicles, bool peds, bool objects, bool dummies, bool doSeeThroughCheck,
/// bool doCameraIgnoreCheck)` (0x56A490, AL): next scan code, then every sector of
/// [`walk_sectors`] through `GetIsLineOfSightSectorClear` (a fresh `CColLine(origin, target)` per call,
/// `GetSector` clamped, the repeat sector of the raw x/y); false at the first blocked sector.
/// Stack: sector values are kept in the argument slots (target x in the origin slot, ...).
#[hook(addr = 0x56A490, conv = cdecl, name = "CWorld::GetIsLineOfSightClear", expect = "83 EC 5C 66 81 3D 78 CD B7 00 FF FF",
       unverified = "mvpc-world-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "same-x / same-y cases walk upwards from the origin (gta-reversed walks down from the target); argument slots reused for sector values")]
#[allow(clippy::too_many_arguments)]
pub fn get_is_line_of_sight_clear(o: GPtr<u8>, t: GPtr<u8>, b: u8, v: u8, p: u8, ob: u8, d: u8, see: u8, cam: u8) -> u8 {
    next_scan_code();
    walk_sectors(o, t, |x, y| {
        let mut lb = [0u32; 8];
        let line = col_line_ctor(local(&mut lb), o, t);
        get_is_line_of_sight_sector_clear_call(sector_at(clamp_sector(x), clamp_sector(y)), repeat_at(x, y), line, b, v,
            p, ob, d, see, cam) != 0
    }) as u8
}

/// `CWorld::ProcessLineOfSight(const CVector& origin, const CVector& target, CColPoint& outColPoint,
/// CEntity*& outEntity, bool buildings, bool vehicles, bool peds, bool objects, bool dummies,
/// bool doSeeThroughCheck, bool doCameraIgnoreCheck, bool doShootThroughCheck)` (0x56BA00): next scan
/// code, `ms_nTotalCrossings = 0`, `outEntity = null`, touch distance 1.0 (kept in the outEntity
/// argument slot), every sector of [`walk_sectors`] through `ProcessLineOfSightSector` (results
/// ignored). One sector: that call's AL is returned; else EAX = `touch < 1.0`.
#[hook(addr = 0x56BA00, conv = cdecl, name = "CWorld::ProcessLineOfSight", expect = "83 EC 60 66 81 3D 78 CD B7 00 FF FF",
       unverified = "mvpc-world-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "touch distance kept in the outEntity argument slot, sector values in the origin/target slots; one-sector path returns the callee's AL")]
#[allow(clippy::too_many_arguments)]
pub fn process_line_of_sight(o: GPtr<u8>, t: GPtr<u8>, cp: GPtr<u8>, out: GPtr<u8>, b: u8, v: u8, p: u8, ob: u8, d: u8,
    see: u8, cam: u8, shoot: u8) -> u8 {
    next_scan_code();
    g::<u32>(TOTAL_CROSSINGS).write(0);
    wr::<u32>(out, 0, 0);
    let mut touchb = [0x3F80_0000u32];
    let touch = local(&mut touchb);
    let sc = |v: f32| sector_coord(X87::from_f32(v));
    let single = sc(rd::<f32>(o, 0)) == sc(rd::<f32>(t, 0)) && sc(rd::<f32>(o, 4)) == sc(rd::<f32>(t, 4));
    let mut last = 0u8;
    walk_sectors(o, t, |x, y| {
        let mut lb = [0u32; 8];
        let line = col_line_ctor(local(&mut lb), o, t);
        last = process_line_of_sight_sector_call(sector_at(clamp_sector(x), clamp_sector(y)), repeat_at(x, y), line, cp,
            touch, out, b, v, p, ob, d, see, cam, shoot);
        true
    });
    if single {
        return last;
    }
    less(f32::from_bits(touchb[0]), g::<f32>(0x858624).read()) as u8
}

/// `CWorld::FindObjectsKindaColliding(const CVector& centre, float radius, bool b2D, int16* outCount,
/// int16 maxCount, CEntity** outEntities, bool buildings, bool vehicles, bool peds, bool objects,
/// bool dummies)` (0x568B80): the sector rect of `centre ± radius` (`x - r`, `y - r`, `r + x`, `r + y`),
/// the next scan code, `*outCount = 0`, then for y (outer) and x (inner) the clamped sector and the
/// repeat sector: buildings, vehicles, peds, objects, dummies through
/// `FindObjectsKindaCollidingSectorList`.
#[hook(addr = 0x568B80, conv = cdecl, name = "CWorld::FindObjectsKindaColliding", expect = "83 EC 18 53 55 56 57",
       unverified = "mvpc-world-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
#[allow(clippy::too_many_arguments)]
pub fn find_objects_kinda_colliding(c: GPtr<u8>, radius: f32, two_d: u8, count: GPtr<u8>, max: i16, out: GPtr<u8>, b: u8,
    v: u8, p: u8, o: u8, d: u8) {
    let cx = rd::<f32>(c, 0);
    let cy = rd::<f32>(c, 4);
    let x0 = sector_coord(X87::from_f32(cx).fsub_m(radius));
    let y0 = sector_coord(X87::from_f32(cy).fsub_m(radius));
    let x1 = sector_coord(X87::from_f32(radius).fadd_m(cx));
    let y1 = sector_coord(X87::from_f32(radius).fadd_m(cy));
    next_scan_code();
    wr::<i16>(count, 0, 0);
    let mut sy = y0;
    while sy <= y1 {
        let mut sx = x0;
        while sx <= x1 {
            let sec = sector_at(clamp_sector(sx), clamp_sector(sy));
            let rep = repeat_at(sx, sy);
            for (flag, l) in [(b, sec), (v, rep), (p, rep.byte_add(4)), (o, rep.byte_add(8)), (d, sec.byte_add(4))] {
                if flag != 0 {
                    kinda_colliding_list_call(l, c, radius, two_d, count, max, out);
                }
            }
            sx += 1;
        }
        sy += 1;
    }
}

/// `CWorld::FindObjectsKindaCollidingSectorList(CPtrListDoubleLink&, const CVector& centre, float radius,
/// bool b2D, int16* outCount, int16 maxCount, CEntity** outEntities)` (0x565000). Every entity not yet
/// scanned gets the scan code (no collision flag test); `d = centre - GetBoundCentre()`, the distance
/// `sqrt(dy*dy + dx*dx)` (2D) or `sqrt((dz*dz + dy*dy) + dx*dx)`; when it is below
/// `colModel radius + radius` (the model info's col model +0x24) and `*outCount < maxCount` (int16), the
/// entity is stored at `outEntities[count]` (when given) and the count incremented.
#[hook(addr = 0x565000, conv = cdecl, name = "CWorld::FindObjectsKindaCollidingSectorList<CPtrListDoubleLink<CPhysical*>>",
       expect = "8B 44 24 04 83 EC 0C 55", unverified = "mvpc-world-1", route = "L2", hazards = "none",
       recs = "grove-walk,driving")]
pub fn find_objects_kinda_colliding_sector_list(list: GPtr<u8>, c: GPtr<u8>, radius: f32, two_d: u8, count: GPtr<u8>,
    max: i16, out: GPtr<u8>) {
    let mut bcb = [0u32; 3];
    let mut n = rd::<GPtr<u8>>(list, 0);
    while !n.is_null() {
        let e = rd::<GPtr<u8>>(n, 0);
        let sc = CURRENT_SCAN_CODE.get();
        n = rd::<GPtr<u8>>(n, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let bc = get_bound_centre(e, local(&mut bcb));
        let mut f = Fpu::new();
        f.fld(rd::<f32>(c, 0));
        f.fsub_m(rd::<f32>(bc, 0));
        f.fld(rd::<f32>(c, 4));
        f.fsub_m(rd::<f32>(bc, 4));
        f.fld(rd::<f32>(c, 8));
        f.fsub_m(rd::<f32>(bc, 8));
        if two_d != 0 {
            f.fstp_st(0);
            f.fld_st(0);
            f.fmul_st(1);
            f.fld_st(2);
            f.fmul_st(3);
            f.faddp(1);
            f.fsqrt();
            f.fstp_st(2);
        } else {
            f.fld_st(0);
            f.fmul_st(1);
            f.fld_st(2);
            f.fmul_st(3);
            f.faddp(1);
            f.fld_st(3);
            f.fmul_st(4);
            f.faddp(1);
            f.fsqrt();
            f.fstp_st(3);
            f.fstp_st(0);
        }
        f.fstp_st(0);
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(e, 0x22) as isize as usize).read();
        let cm = rd::<GPtr<u8>>(mi, 0x14);
        f.fld(rd::<f32>(cm, 0x24));
        f.fadd_m(radius);
        f.fxch(1);
        f.fcompp();
        if f.jp(5) {
            continue;
        }
        let k = rd::<i16>(count, 0);
        if k < max {
            if !out.is_null() {
                wr::<GPtr<u8>>(out, 4 * k as isize, e);
            }
            wr::<i16>(count, 0, k.wrapping_add(1));
        }
    }
}

/// `CWorld::FindLodOfTypeInRange(uint32 modelId, const CVector& point, float radius, bool b2D,
/// int16* outCount, int16 maxCount, CEntity** outEntities)` (0x564ED0): the LOD-list rect of
/// `point ± radius` (`_ftol(v * 0.005 + 15)`, no `floor`, no clamping: x0, y0, x1, y1 from `x - r`,
/// `y - r`, `r + x`, `r + y`), the next scan code, `*outCount = 0`, then for every row y0..=y1 the lists
/// `ms_aLodPtrLists[y * 30 + x]` for x0..=x1 through `FindObjectsOfTypeInRangeSectorList`.
/// Stack: x0 is stored into the `point` argument slot (S+8).
#[hook(addr = 0x564ED0, conv = cdecl, name = "CWorld::FindLodOfTypeInRange", expect = "83 EC 0C 53 8B 5C 24 18",
       unverified = "mvpc-world-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "stores the first LOD-list x into its point argument slot (S+8)")]
#[allow(clippy::too_many_arguments)]
pub fn find_lod_of_type_in_range(model: i32, pt: GPtr<u8>, radius: f32, two_d: u8, count: GPtr<u8>, max: i16, out: GPtr<u8>) {
    let k = g::<f32>(0x858B4C).read();
    let c = g::<f32>(0x858B48).read();
    let lod = |v: X87| v.fmul_m(k).fadd_m(c).ftol() as u32 as i32;
    let px = rd::<f32>(pt, 0);
    let py = rd::<f32>(pt, 4);
    let x0 = lod(X87::from_f32(px).fsub_m(radius));
    let y0 = lod(X87::from_f32(py).fsub_m(radius));
    let x1 = lod(X87::from_f32(radius).fadd_m(px));
    let y1 = lod(X87::from_f32(radius).fadd_m(py));
    next_scan_code();
    wr::<i16>(count, 0, 0);
    let mut y = y0;
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            let list = g::<u8>((0xB99EB8u32 as i32).wrapping_add(y.wrapping_mul(30).wrapping_add(x).wrapping_mul(4)) as u32 as usize);
            objects_of_type_in_range_list(model, list, pt, radius, two_d, count, max, out);
            x += 1;
        }
        y += 1;
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_LOD_OF_TYPE_IN_RANGE,
    HOOK_FIND_OBJECTS_KINDA_COLLIDING,
    HOOK_FIND_OBJECTS_KINDA_COLLIDING_SECTOR_LIST,
    HOOK_GET_IS_LINE_OF_SIGHT_CLEAR,
    HOOK_PROCESS_LINE_OF_SIGHT,
    HOOK_PROCESS_VERTICAL_LINE,
    HOOK_PROCESS_VERTICAL_LINE_SECTOR,
    HOOK_PROCESS_VERTICAL_LINE_SECTOR_LIST,
    HOOK_GET_IS_LINE_OF_SIGHT_SECTOR_CLEAR,
    HOOK_GET_IS_LINE_OF_SIGHT_SECTOR_LIST_CLEAR,
    HOOK_PROCESS_LINE_OF_SIGHT_SECTOR,
    HOOK_TEST_SPHERE_AGAINST_WORLD,
    HOOK_TEST_SPHERE_AGAINST_SECTOR_LIST,
];
