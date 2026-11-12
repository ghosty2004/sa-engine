//! WRITER-WORLD-2 (batch `world6-1`): world queries whose bodies continue in a no-CD protection chunk
//! of `.text` 0x401000.., decoded by hand from the exe image:
//!
//! - `CWorld::FindObjectsIntersectingAngledCollisionBoxSectorList`: chunk 0x407507 is `mov ax,
//!   [0xB7CD78]; jmp 0x565228` (the scan code load, moved out of the loop body);
//! - `CSetPiece::TryToGenerateCopPed`: the entry is `push -1; jmp 0x40193F`; the chunk pushes
//!   `[0x4B4704] ^ [0x1560694]` (the SEH handler of the frame) and jumps back to 0x499697. The
//!   protection reads the entry's dword at 0x499692 as a key (`xor edx, [0x499692]` at 0x4015AA):
//!   sa-hook's keycells `READERS` keep it reading the original dword;
//! - `FUN_00449220` (a `CGarage` method called from `CGarage::Update`): chunk 0x40209A loads
//!   `[[0x1560718] + [0x5E7CBD]]` = `[0x1F1D6EAB + 0xE199D5E9]` = `[0xB74494]`, the vehicle pool
//!   (`CPools::ms_pVehiclePool`), and jumps back to 0x44922B.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use super::common::{g, rd, wr, x};

orig! {
    /// `Multiply3x3(CVector* out, const CVector&, const CMatrix&)` (0x59C810): `v * M` (transposed)
    fn multiply3x3_vm(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `CCollision::TestSphereBox(const CColSphere&, const CColBox&)` (0x4120C0)
    fn test_sphere_box(s: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x4120C0 => unported;
    /// `CWorld::ProcessVerticalLine(...)` (0x5674E0)
    fn process_vertical_line(pos: GPtr<u8>, len: f32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CWorld::FindObjectsKindaColliding(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)` (0x568B80)
    fn find_objects_kinda_colliding(pos: GPtr<u8>, radius: f32, b2d: u32, count: GPtr<u8>, max: u32, out: u32, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x568B80 => unported;
    /// `CPed::operator new(uint32)` (0x5E4720)
    fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCopPed::CCopPed(eCopType)` (0x5DDC60)
    fn cop_ped_ctor(p: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x5DDC60 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CGarage::IsEntityTouching3D(CEntity*)` (0x448EE0)
    fn garage_is_entity_touching_3d(gr: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x448EE0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890): the result through the hidden pointer
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CGarage::IsPointInsideGarage(CVector, float margin)` (0x4487D0)
    fn garage_is_point_inside(gr: GPtr<u8>, p: CVector, margin: f32) -> u8 = thiscall 0x4487D0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vec_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// `CWorld::ms_nCurrentScanCode` (0xB7CD78), `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8),
/// `CTimer::ms_fTimeStep` (0xB7CB5C), `CPools::ms_pVehiclePool` (0xB74494).
const SCAN_CODE: usize = 0xB7_CD78;
const MODEL_INFOS: usize = 0xA9_B0C8;
const TIME_STEP: usize = 0xB7_CB5C;
const VEHICLE_POOL: usize = 0xB7_4494;

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `GetPosition()`.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CWorld::FindObjectsIntersectingAngledCollisionBoxSectorList(CPtrListDoubleLink& list, const
/// CColBox& box, const CMatrix& transform, const CVector& point, int16* count, int16 maxCount,
/// CEntity** out)` (0x565200): every entity of the list not yet visited this scan (+0x2C != the scan
/// code; then marked) whose bound sphere (radius of its model's collision +0x24, centre `(pos -
/// point) * transform` transposed) touches `box` is counted; while `*count < maxCount` it is
/// appended to `out` (when not null) and the count incremented.
#[hook(addr = 0x565200, conv = cdecl, name = "CWorld::FindObjectsIntersectingAngledCollisionBoxSectorList<CPtrListDoubleLink<CPhysical*>>",
       expect = "8B 44 24 04 83 EC 20",
       unverified = "world6-1", route = "L2", hazards = "none",
       note = "the scan code is loaded in protection chunk 0x407507; the sphere's material bytes are not written (TestSphereBox does not read them)")]
pub fn find_objects_intersecting_angled_collision_box_sector_list(list: GPtr<u8>, bx: GPtr<u8>, transform: GPtr<u8>,
                                                                   point: GPtr<u8>, count: GPtr<i16>, max: i16,
                                                                   out: GPtr<GPtr<u8>>) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let code = g::<u16>(SCAN_CODE).read();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == code {
            continue;
        }
        wr::<u16>(e, 0x2C, code);
        let pos = ent_pos(e);
        let mi = rd::<i16>(e, 0x22) as isize;
        let col = rd::<GPtr<u8>>(g::<GPtr<u8>>((MODEL_INFOS as isize + mi * 4) as usize).read(), 0x14);
        let mut d = [
            x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(point, 0)).to_f32(),
            x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(point, 4)).to_f32(),
            x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(point, 8)).to_f32(),
        ];
        let mut sphere = [0u32; 5];
        sphere[3] = rd::<u32>(col, 0x24);
        let mut c = [0u32; 3];
        multiply3x3_vm(p(&mut c), p(&mut d), transform);
        sphere[..3].copy_from_slice(&c);
        if test_sphere_box(p(&mut sphere), bx) != 0 {
            let n = count.read();
            if n < max {
                if !out.is_null() {
                    out.wrapping_index(i32::from(n)).write(e);
                }
                count.write(n.wrapping_add(1));
            }
        }
    }
}

/// `CSetPiece::TryToGenerateCopPed(CVector2D pos)` (0x499690, `this` unused): the ground under
/// `pos` (a vertical line from z 1000 of length -1000 against buildings and dummies; hit: z + 0.9,
/// else 1000) must have no vehicle or ped within 0.6 (`FindObjectsKindaColliding`, 3D, count only);
/// then a new city cop (`CCopPed(0)`) is placed there (matrix position) and added to the world.
/// Returns the cop, or null when the spot is taken.
#[hook(addr = 0x499690, conv = thiscall, name = "CSetPiece::TryToGenerateCopPed", this = "CSetPiece",
       expect = "6A FF E9 A8 82 F6 FF",
       unverified = "world6-1", route = "L2", hazards = "entry-protection-read,heap-writes",
       note = "entry push -1; jmp 0x40193F (SEH handler decoded in the chunk); keycells reader 0x4015AA reads the entry dword 0x499692; the count is kept in the x argument slot (initial value: the x bits)")]
pub fn try_to_generate_cop_ped(_this: GPtr<u8>, px: f32, py: f32) -> GPtr<u8> {
    let mut pos = [px, py, 1000.0];
    let mut cp = [0f32; 11];
    let mut ent: u32 = 0;
    if process_vertical_line(p(&mut pos), f32::from_bits(0xC47A_0000), p(&mut cp), p(&mut ent), 1, 0, 0, 0, 1, 0, 0) != 0 {
        pos[2] = x(cp[2]).fadd_m(f32::from_bits(0x3F66_6666)).to_f32();
    }
    let mut count: u32 = px.to_bits();
    find_objects_kinda_colliding(p(&mut pos), f32::from_bits(0x3F19_999A), 0, p(&mut count), 0x10, 0, 0, 1, 1, 0, 0);
    if count as u16 != 0 {
        return GPtr::NULL;
    }
    let mem = ped_new(0x7C4);
    let cop = if mem.is_null() { GPtr::NULL } else { cop_ped_ctor(mem, 0) };
    for (k, v) in pos.iter().enumerate() {
        wr::<f32>(rd::<GPtr<u8>>(cop, 0x14), 0x30 + 4 * k as isize, *v);
    }
    world_add(cop);
    cop
}

/// `FUN_00449220(CEntity* except)` (0x449220, a `CGarage` method called by `CGarage::Update`): for
/// every vehicle of the pool (highest slot first) other than `except` that touches the garage in 3D,
/// the first collision sphere centre (vehicle matrix applied) outside the garage pushes the vehicle
/// away from the garage's xy centre (`(pos - centre)` with z 0, normalised): move speed += `dir *
/// 0.02 * timeStep` (x and z through f32 temporaries, y kept in the register), then the next
/// vehicle.
#[hook(addr = 0x449220, conv = thiscall, name = "FUN_00449220", this = "CGarage",
       expect = "83 EC 48 53 55",
       unverified = "world6-1", route = "L2", hazards = "heap-writes",
       recs = "world-cheats",
       note = "the vehicle pool pointer is decoded in protection chunk 0x40209A (0xB74494)")]
pub fn garage_push_vehicles_out(this: GPtr<u8>, except: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flags = rd::<GPtr<u8>>(pool, 4);
        let flag = rd::<i8>(flags, (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let veh = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if veh.is_null() || veh == except || garage_is_entity_touching_3d(this, veh) == 0 {
            continue;
        }
        let data = rd::<GPtr<u8>>(get_col_model(veh), 0x2C);
        let mut s = 0i32;
        while s < i32::from(rd::<i16>(data, 0)) {
            let centre = rd::<GPtr<u8>>(data, 8).byte_add(s as isize * 0x14);
            let mut out = [0f32; 3];
            let r = matrix_mul_vec(p(&mut out), rd::<GPtr<u8>>(veh, 0x14), centre);
            let pt = CVector::new(rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8));
            if garage_is_point_inside(this, pt, 0.0) == 0 {
                let pos = ent_pos(veh);
                let mut dir = [
                    x(rd::<f32>(this, 0x2C)).fadd_m(rd::<f32>(this, 0x28)).fmul_m(0.5).fsubr_m(rd::<f32>(pos, 0)).to_f32(),
                    x(rd::<f32>(this, 0x34)).fadd_m(rd::<f32>(this, 0x30)).fmul_m(0.5).fsubr_m(rd::<f32>(ent_pos(veh), 4)).to_f32(),
                    0.0,
                ];
                vec_normalise(p(&mut dir));
                let k = f32::from_bits(0x3CA3_D70A);
                let ts = g::<f32>(TIME_STEP).read();
                let a = x(dir[0]).fmul_m(k);
                let b = x(dir[1]).fmul_m(k);
                let c = x(dir[2]).fmul_m(k).to_f32();
                let ax = a.fmul_m(ts).to_f32();
                let by = x(ts).fmul(b);
                let cz = x(c).fmul_m(ts);
                let sx = x(ax).fadd_m(rd::<f32>(veh, 0x44)).to_f32();
                let sy = by.fadd_m(rd::<f32>(veh, 0x48)).to_f32();
                let sz = cz.fadd_m(rd::<f32>(veh, 0x4C)).to_f32();
                wr::<f32>(veh, 0x44, sx);
                wr::<f32>(veh, 0x48, sy);
                wr::<f32>(veh, 0x4C, sz);
                break;
            }
            s += 1;
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_OBJECTS_INTERSECTING_ANGLED_COLLISION_BOX_SECTOR_LIST,
    HOOK_TRY_TO_GENERATE_COP_PED,
    HOOK_GARAGE_PUSH_VEHICLES_OUT,
];
