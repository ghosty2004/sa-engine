//! WRITER-CLIMB (batch `climb-1`): `CTaskSimpleClimb::ScanToGrab` (0x67FD30) and
//! `ScanToGrabSectorList<CPtrListSingleLink<CPhysical*>>` (0x67DE10), reached by every player jump through
//! `TestForClimb`. Both entries are SecuROM stubs (`jmp` into the 0x401000 fragment area, which jumps back); the
//! hooks sit on the stubs. Ported from the 1.0 US bytes; gta-reversed (`TaskSimpleClimb.cpp`) as a reading aid.
//! x87 arithmetic follows the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimpleClimb::CreateColModel()` (0x67A890)
    fn climb_create_col_model() -> u8 = cdecl 0x67A890 => unported;
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CTaskSimpleClimb::ScanToGrabSectorList<...>(list, ped, pos&, angle&, surface&, launch, standUp, vault)`
    /// (0x67DE10, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn scan_sector_list_c(list: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surface: GPtr<u8>, launch: u32, stand_up: u32, vault: u32) -> GPtr<u8> = cdecl 0x67DE10 => climb_scan_to_grab_sector_list;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix Invert(const CMatrix&)` (0x59BDD0, cdecl, hidden result first)
    fn matrix_invert(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BDD0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// CRT `_atexit` (0x821D1E)
    fn crt_atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn intel_get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CStats::GetFatAndMuscleModifier(int32)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier(i: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534250, returns `&out`)
    fn entity_get_bound_centre(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x534250 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CCollision::ProcessColModels(const CMatrix&, CColModel&, const CMatrix&, CColModel&, CColPoint*,
    /// CColPoint*, float*, bool)` (0x4185C0)
    #[allow(clippy::too_many_arguments)]
    fn collision_process_col_models(m1: GPtr<u8>, c1: GPtr<u8>, m2: GPtr<u8>, c2: GPtr<u8>, cps: GPtr<u8>, lcps: u32, dists: u32, b: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, hidden result first)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(m: GPtr<u8>, o: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::SetTranslateOnly(float, float, float)` (0x59AF80)
    fn matrix_set_translate_only(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF80 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `SurfaceInfos_c::CanClimb(surface)` (0x55EA10) on `g_surfaceInfos` (0xB79538)
    fn surface_can_climb(s: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55EA10 => unported;
}

/// `CTaskSimpleClimb::ms_ClimbColModel` (0xC19518; bound centre +0x18, radius +0x24, col data +0x2C),
/// `ms_StandUpColModel` (0xC19548), `ms_VaultColModel` (0xC19578), `ms_FindEdgeColModel` (0xC195A8)
const CLIMB_COL: usize = 0xC1_9518;
const STAND_UP_COL: usize = 0xC1_9548;
const VAULT_COL: usize = 0xC1_9578;
const FIND_EDGE_COL: usize = 0xC1_95A8;
/// The function-static `CMatrix` of `ScanToGrabSectorList` (0xC195E8) and its init guard (0xC19630, bit 0);
/// its `atexit` destructor is 0x8569A0.
const EDGE_MATRIX: usize = 0xC1_95E8;
const EDGE_MATRIX_GUARD: usize = 0xC1_9630;
/// `CWorld::m_aTempColPts` (0xB9ACD0): point +0, normal +0x10, piece A +0x21, surface B +0x23.
const COL_PTS: usize = 0xB9_ACD0;
/// `CWorld::ms_nCurrentScanCode` (0xB7CD78)
const SCAN_CODE: usize = 0xB7_CD78;
/// `CWorld::ms_aSectors` (0xB7D0B8, 120 x 120, 8 bytes) and `ms_aRepeatSectors` (0xB992B8, 16 x 16, 12 bytes:
/// vehicles +0, peds +4, objects +8)
const SECTORS: usize = 0xB7_D0B8;
const REPEAT_SECTORS: usize = 0xB9_92B8;

fn c(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..3 {
        wr::<u32>(dst, 4 * k, rd::<u32>(src, 4 * k));
    }
}

fn buf(v: &mut [u32]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// Restores a boat's sphere count (word at `colData +0`), saved as a signed word (`-1`: nothing saved).
fn restore_spheres(e: GPtr<u8>, saved: i16) {
    if rd::<u8>(e, 0x36) & 7 == 2 && saved > -1 {
        let col = entity_get_col_model(e);
        wr::<i16>(rd::<GPtr<u8>>(col, 0x2C), 0, saved);
    }
}

/// The hit normal (0xB9ACE0) leaves the near-flat window on this axis: `v > [0x858C28]` (ordered) or
/// `v < [0x85901C]`.
fn out_of_window(v: f32) -> bool {
    !x(v).fcom(x(c(0x85_8C28))).test_any(0x41) || !x(v).fcom(x(c(0x85_901C))).test_parity(0x05)
}

/// Both normal axes x / y inside the near-flat window.
fn flat_hit() -> bool {
    !out_of_window(c(COL_PTS + 0x10)) && !out_of_window(c(COL_PTS + 0x14))
}

/// The facing test of a steep hit: `d = hit - pedPos`; when `d.y n.y + d.x n.x + n.z [0x858B50] < [0x858B50]`
/// the normal's x / y (stored) and z (kept in ST0) are scaled by [0x858C1C]; passes when `n . forward >
/// [0x858C24]` (ordered). Returns the angle `atan2(-n.x, n.y)` (scaled values) on a pass.
fn facing_angle(ped: GPtr<u8>) -> Option<f32> {
    let mut n = [c(COL_PTS + 0x10), c(COL_PTS + 0x14), c(COL_PTS + 0x18)];
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let mut d = [0u32; 3];
    let dp = buf(&mut d);
    let r = vector_sub(dp, GPtr::from_addr(COL_PTS), pos_ptr(ped));
    copy3(dp, r);
    let (dx, dy) = (f32::from_bits(d[0]), f32::from_bits(d[1]));
    let k50 = c(0x85_8B50);
    let t = x(dy).fmul_m(n[1]).fadd(x(dx).fmul_m(n[0])).fadd(x(n[2]).fmul_m(k50));
    let mut nz = x(n[2]);
    if !t.fcom(x(k50)).test_parity(0x05) {
        let s = c(0x85_8C1C);
        n[0] = x(n[0]).fmul_m(s).to_f32();
        n[1] = x(n[1]).fmul_m(s).to_f32();
        nz = nz.fmul_m(s);
    }
    let dot = x(n[1])
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(nz.fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(n[0]).fmul_m(rd::<f32>(m, 0x10)));
    if dot.fcom(x(c(0x85_8C24))).test_any(0x41) {
        return None;
    }
    Some(x(n[0]).fchs().fpatan(x(n[1])).to_f32())
}

/// `CPhysical* CTaskSimpleClimb::ScanToGrabSectorList<CPtrListSingleLink<CPhysical*>>(list, CPed* ped, CVector&
/// climbPos, float& climbAngle, uint8& surface, bool launch, bool standUp, bool vault)` (0x67DE10, cdecl; used for
/// the building, object and vehicle lists alike).
///
/// First call: the function-static `CMatrix` at 0xC195E8 is constructed (attach fields zeroed, `atexit`
/// 0x8569A0). `swimming` = `GetTaskSwim() != null`; the player (+0x480) when not swimming gets `muscle =
/// ftol(GetFatAndMuscleModifier(0))`, else 0. The scan model is the climb model (stand-up / vault models for those
/// scans), its bound centre through the ped matrix. Every list entity not yet scanned (scan code +0x2C) that
/// uses collision (+0x1C bit 0) and is a building, an object (+0x1C bit 2 / bit 18 / +0x40 bit 2, without +0x40
/// bit 6) or a vehicle (stand-up / vault scans; a swimming ped and a boat (+0x594 == 5); a train (6) moving
/// slower than [0x858B1C]) whose bound spheres overlap (strictly) is tested with `ProcessColModels` (a boat's
/// spheres are hidden meanwhile and restored). On contact: stand-up / vault scans return the entity; sphere
/// piece 0 returns 1 ("blocked"); pieces 1..3, a hit too low (not above the climb point and in front, `rel .
/// forward >= [0x858B50]`), an unclimbable surface, pieces 4 / 10 / 16 and the strength-limited 5 / 11 / 17,
/// 6 / 12 / 18 are rejected: 1 for pieces 0..4 and 10, otherwise the next entity. A kept hit becomes the grab
/// point (flat normal: the ped's heading +0x558; steep: the facing test), then the find-edge model at the hit
/// point refines it (flat: position and surface only). Returns the last entity grabbed.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x67DE10, conv = cdecl, name = "CTaskSimpleClimb::ScanToGrabSectorList<CPtrListSingleLink<CPhysical*>>",
       expect = "E9 D2 44 D8 FF 83 EC 68",
       unverified = "climb-1", route = "L2", hazards = "x87-transcendental,heap-writes,entry-protection-read",
       recs = "grove-walk", note = "SecuROM stub entry; fpatan; temporarily zeroes a boat's sphere count")]
pub fn climb_scan_to_grab_sector_list(list: GPtr<u8>, ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surface: GPtr<u8>, launch: u32, stand_up: u32, vault: u32) -> GPtr<u8> {
    let guard = g::<u32>(EDGE_MATRIX_GUARD);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
        g::<u32>(EDGE_MATRIX + 0x40).write(0);
        g::<u32>(EDGE_MATRIX + 0x44).write(0);
        crt_atexit(0x85_69A0);
    }
    let (launch, stand_up, vault) = (launch as u8 != 0, stand_up as u8 != 0, vault as u8 != 0);
    let mut result = GPtr::<u8>::NULL;
    let swimming = !intel_get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null();
    let mut muscle = 0i32;
    if rd::<u32>(ped, 0x480) != 0 && intel_get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        muscle = ftol32(X87::from_f64_exact(stats_get_fat_and_muscle_modifier(0)));
    }
    let model = if stand_up {
        STAND_UP_COL
    } else if vault {
        VAULT_COL
    } else {
        CLIMB_COL
    };
    let model_p: GPtr<u8> = GPtr::from_addr(model);
    let mut centre = [0u32; 3];
    let centre_p = buf(&mut centre);
    let r = matrix_mul_vec(centre_p, rd::<GPtr<u8>>(ped, 0x14), model_p.byte_add(0x18));
    copy3(centre_p, r);
    let radius = rd::<f32>(model_p, 0x24);
    let cpts: GPtr<u8> = GPtr::from_addr(COL_PTS);
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let code = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) == code {
            continue;
        }
        wr::<u16>(e, 0x2C, code);
        if rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        let ty = rd::<u8>(e, 0x36) & 7;
        let mut take = ty == 1;
        if ty == 4 {
            let f = rd::<u32>(e, 0x1C);
            if (f & 4 != 0 || f & 0x4_0000 != 0 || rd::<u8>(e, 0x40) & 4 != 0) && rd::<u8>(e, 0x40) & 0x40 == 0 {
                take = true;
            }
        } else if ty == 2 {
            if stand_up || vault || (swimming && rd::<u32>(e, 0x594) == 5) {
                take = true;
            } else if rd::<u32>(e, 0x594) == 6 {
                let mag = X87::from_f64_exact(vector_magnitude(e.byte_add(0x44)));
                take = !mag.fcom(x(c(0x85_8B1C))).test_parity(0x05);
            }
        }
        if !take {
            continue;
        }
        let mut bc = [0u32; 3];
        let bcp = entity_get_bound_centre(e, buf(&mut bc));
        let dx = x(f32::from_bits(centre[0])).fsub_m(rd::<f32>(bcp, 0));
        let dy = x(f32::from_bits(centre[1])).fsub_m(rd::<f32>(bcp, 4));
        let dz = x(f32::from_bits(centre[2])).fsub_m(rd::<f32>(bcp, 8));
        let dist = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy)).fsqrt();
        let info = g::<GPtr<u8>>(0xA9_B0C8).wrapping_index(i32::from(rd::<i16>(e, 0x22))).read();
        let sum = x(rd::<f32>(rd::<GPtr<u8>>(info, 0x14), 0x24)).fadd_m(radius);
        if sum.fcom(dist).test_any(0x41) {
            continue;
        }
        let mut saved: i16 = -1;
        if ty == 2 && rd::<u32>(e, 0x594) == 5 {
            saved = rd::<i16>(rd::<GPtr<u8>>(entity_get_col_model(e), 0x2C), 0);
            wr::<i16>(rd::<GPtr<u8>>(entity_get_col_model(e), 0x2C), 0, 0);
        }
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let em = rd::<GPtr<u8>>(e, 0x14);
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        if collision_process_col_models(pm, model_p, em, entity_get_col_model(e), cpts, 0, 0, 0) > 0 {
            if stand_up || vault {
                restore_spheres(e, saved);
                return e;
            }
            let rel = [
                x(c(COL_PTS)).fsub_m(rd::<f32>(pos, 0)).to_f32(),
                x(c(COL_PTS + 4)).fsub_m(rd::<f32>(pos, 4)).to_f32(),
                x(c(COL_PTS + 8)).fsub_m(rd::<f32>(pos, 8)).to_f32(),
            ];
            let piece = rd::<u8>(cpts, 0x21);
            if piece == 0 {
                restore_spheres(e, saved);
                return GPtr::from_addr(1);
            }
            let mut reject = matches!(piece, 1..=3);
            if !reject && !x(c(COL_PTS + 8)).fcom(x(rd::<f32>(pos, 8))).test_parity(0x41) {
                let dot = x(rel[2])
                    .fmul_m(rd::<f32>(pm, 0x18))
                    .fadd(x(rel[1]).fmul_m(rd::<f32>(pm, 0x14)))
                    .fadd(x(rel[0]).fmul_m(rd::<f32>(pm, 0x10)));
                reject = !dot.fcom(x(c(0x85_8B50))).c0();
            }
            if !reject {
                reject = surface_can_climb(GPtr::from_addr(0xB7_9538), u32::from(rd::<u8>(cpts, 0x23))) == 0
                    || matches!(piece, 4 | 0xA | 0x10);
            }
            if !reject && !(launch && muscle < 1) {
                reject = matches!(piece, 5 | 0xB | 0x11);
                if !reject && muscle >= 1 && !(launch && muscle < 2) {
                    reject = matches!(piece, 6 | 0xC | 0x12);
                }
            }
            if reject {
                restore_spheres(e, saved);
                if matches!(piece, 0..=4 | 0xA) {
                    return GPtr::from_addr(1);
                }
                continue;
            }
            // the climb model's hit
            if flat_hit() {
                copy3(pos, cpts);
                wr::<u32>(angle, 0, rd::<u32>(ped, 0x558));
                wr::<u8>(surface, 0, rd::<u8>(cpts, 0x23));
                result = e;
            } else if let Some(a) = facing_angle(ped) {
                copy3(pos, cpts);
                wr::<f32>(angle, 0, a);
                wr::<u8>(surface, 0, rd::<u8>(cpts, 0x23));
                result = e;
            }
            // the find-edge model at the hit point
            let edge: GPtr<u8> = GPtr::from_addr(EDGE_MATRIX);
            matrix_assign(edge, rd::<GPtr<u8>>(ped, 0x14));
            matrix_set_translate_only(edge, rd::<u32>(cpts, 0), rd::<u32>(cpts, 4), rd::<u32>(cpts, 8));
            let ecol = entity_get_col_model(e);
            let emat = placeable_get_matrix(e);
            if collision_process_col_models(edge, GPtr::from_addr(FIND_EDGE_COL), emat, ecol, cpts, 0, 0, 0) > 0 {
                if flat_hit() {
                    copy3(pos, cpts);
                    wr::<u8>(surface, 0, rd::<u8>(cpts, 0x23));
                } else if let Some(a) = facing_angle(ped) {
                    copy3(pos, cpts);
                    result = e;
                    wr::<f32>(angle, 0, a);
                    wr::<u8>(surface, 0, rd::<u8>(cpts, 0x23));
                }
            }
        }
        restore_spheres(e, saved);
    }
    result
}

/// A sector coordinate: `v * 0.02 + 60` ([0x858B38] / [0x858B34]) stored as a double, CRT `floor`, `_ftol`.
fn sector_of(v: X87) -> i32 {
    let t = v.fmul_m(c(0x85_8B38)).fadd_m(c(0x85_8B34));
    ftol32(X87::from_f64_exact(sa_world::physical::base::floor_f64(t.to_f64())))
}

/// Writes a position through the ped's matrix (+0x30) or its placement (+4).
fn set_pos(ped: GPtr<u8>, v: [u32; 3]) {
    let d = pos_ptr(ped);
    for (k, w) in v.iter().enumerate() {
        wr::<u32>(d, 4 * k as isize, *w);
    }
}

/// `CEntity* CTaskSimpleClimb::ScanToGrab(CPed* ped, CVector& climbPos, float& climbAngle, uint8& surface, bool
/// launch, bool standUp, bool vault, CVector* pedPosition)` (0x67FD30, cdecl).
///
/// Creates the climb models when missing ([0xC19544]). With `pedPosition` the ped is moved there for the scan
/// (through `fld` / `fstp`). `climbPos = pos + forward * [0x85862C]`. Over the sectors the climb model's bound
/// sphere touches (after `AdvanceCurrentScanCode`): buildings, then repeat-sector objects, else vehicles; a 1
/// ("blocked") from either returns null at once (without restoring the position); a hit returns at once for
/// stand-up / vault scans (position restored), else is remembered (the later one wins). The position is
/// restored; a remembered vehicle, ped or object turns `climbPos` into its local space (`Invert(matrix) *
/// climbPos`, allocating the matrix when missing) and subtracts its heading `atan2(-fwd.x, fwd.y)` from
/// `climbAngle`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x67FD30, conv = cdecl, name = "CTaskSimpleClimb::ScanToGrab",
       expect = "90 E9 06 13 D8 FF",
       unverified = "climb-1", route = "L2", hazards = "x87-transcendental,heap-writes,entry-protection-read",
       recs = "grove-walk", note = "SecuROM stub entry; floor / _ftol sector bounds; fpatan")]
pub fn climb_scan_to_grab(ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surface: GPtr<u8>, launch: u32, stand_up: u32, vault: u32, over: GPtr<u8>) -> GPtr<u8> {
    if g::<u32>(0xC1_9544).read() == 0 {
        climb_create_col_model();
    }
    let p = pos_ptr(ped);
    let saved = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    if !over.is_null() {
        let v = [
            x(rd::<f32>(over, 0)).to_f32().to_bits(),
            x(rd::<f32>(over, 4)).to_f32().to_bits(),
            x(rd::<f32>(over, 8)).to_f32().to_bits(),
        ];
        set_pos(ped, v);
    }
    copy3(pos, pos_ptr(ped));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let ten = c(0x85_862C);
    for k in 0..3isize {
        let v = x(rd::<f32>(m, 0x10 + 4 * k)).fmul_m(ten).fadd_m(rd::<f32>(pos, 4 * k)).to_f32();
        wr::<f32>(pos, 4 * k, v);
    }
    let mut pt = [0u32; 3];
    matrix_mul_vec(buf(&mut pt), rd::<GPtr<u8>>(ped, 0x14), GPtr::from_addr(CLIMB_COL + 0x18));
    let (px, py) = (f32::from_bits(pt[0]), f32::from_bits(pt[1]));
    let r = c(CLIMB_COL + 0x24);
    let start_x = sector_of(x(px).fsub_m(r));
    let start_y = sector_of(x(py).fsub_m(r));
    let end_x = sector_of(x(r).fadd_m(px));
    let end_y = sector_of(x(r).fadd_m(py));
    let code = g::<u16>(SCAN_CODE);
    if code.read() == 0xFFFF {
        world_clear_scan_codes();
        code.write(1);
    } else {
        code.write(code.read() + 1);
    }
    let decides = stand_up as u8 != 0 || vault as u8 != 0;
    let mut collided = GPtr::<u8>::NULL;
    let mut sy = start_y;
    while sy <= end_y {
        let mut sx = start_x;
        while sx <= end_x {
            let cy = sy.clamp(0, 0x77);
            let cx = sx.clamp(0, 0x77);
            let sector: GPtr<u8> = GPtr::from_addr(SECTORS + ((cy * 0x78 + cx) as usize) * 8);
            let rep: GPtr<u8> =
                GPtr::from_addr(REPEAT_SECTORS + ((((sy & 0xF) << 4) + (sx & 0xF)) as usize) * 12);
            let r1 = scan_sector_list_c(sector, ped, pos, angle, surface, launch, stand_up, vault);
            let mut r2 = scan_sector_list_c(rep.byte_add(8), ped, pos, angle, surface, launch, stand_up, vault);
            if r2.is_null() {
                r2 = scan_sector_list_c(rep, ped, pos, angle, surface, launch, stand_up, vault);
            }
            if r1.addr() == 1 || r2.addr() == 1 {
                return GPtr::NULL;
            }
            let ent = if r2.is_null() { r1 } else { r2 };
            if !ent.is_null() {
                if decides {
                    if !over.is_null() {
                        set_pos(ped, saved);
                    }
                    return ent;
                }
                collided = ent;
            }
            sx += 1;
        }
        sy += 1;
    }
    if !over.is_null() {
        set_pos(ped, saved);
    }
    if !collided.is_null() {
        let ty = rd::<u8>(collided, 0x36) & 7;
        if ty > 1 && ty < 5 {
            if rd::<u32>(collided, 0x14) == 0 {
                placeable_allocate_matrix(collided);
                simple_transform_update_matrix(collided.byte_add(4), rd::<GPtr<u8>>(collided, 0x14));
            }
            let mut inv = [0u32; 18];
            let invp = buf(&mut inv);
            let im = matrix_invert(invp, rd::<GPtr<u8>>(collided, 0x14));
            let mut out = [0u32; 3];
            let o = matrix_mul_vec(buf(&mut out), im, pos);
            copy3(pos, o);
            matrix_dtor(invp);
            let cm = rd::<GPtr<u8>>(collided, 0x14);
            let h = if cm.is_null() {
                x(rd::<f32>(collided, 0x10))
            } else {
                x(rd::<f32>(cm, 0x10)).fchs().fpatan(x(rd::<f32>(cm, 0x14)))
            };
            wr::<f32>(angle, 0, x(rd::<f32>(angle, 0)).fsub(h).to_f32());
        }
    }
    collided
}

pub const HOOKS: &[HookReg] = &[HOOK_CLIMB_SCAN_TO_GRAB_SECTOR_LIST, HOOK_CLIMB_SCAN_TO_GRAB];
