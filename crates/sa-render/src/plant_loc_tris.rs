//! WRITER-RENDER-BIG-2: `CPlantMgr::_ProcessEntryCollisionDataSections_AddLocTris` (0x5DC8B0). Ported
//! from the 1.0 US bytes; gta-reversed `PlantMgr.cpp` as reading aid. x87 arithmetic follows the
//! original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use sa_world::collision::data::CdPtr;
use sa_world::collision::surface::SiPtr;

use crate::common::{g, jp, k, rd, wr, x};

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `CCollisionData::GetTrianglePoint(CVector&, int)` (0x40f5e0)
    fn get_triangle_point(cd: CdPtr, out: GPtr<CVector>, idx: i32) = thiscall 0x40f5e0 => sa_world::collision::data::get_triangle_point;
    /// `CPlaceable::AllocateMatrix()` (0x54f560)
    fn allocate_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x54f560 => sa_world::world::entity::allocate_matrix;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54f1b0)
    fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54f1b0 => crate::mvpd_unk::simple_transform_update_matrix;
    /// `TransformPoints(RwV3d* out, int count, const CMatrix&, const RwV3d* in)` (0x54eef0)
    fn transform_points(out: GPtr<u8>, count: u32, m: GPtr<u8>, inp: GPtr<u8>) = cdecl 0x54eef0 => crate::mvpb_r1::transform_points;
    /// `SurfaceInfos_c::CreatesPlants(id)` (0x55e9d0)
    fn creates_plants(si: SiPtr, id: u32) -> u32 = thiscall 0x55e9d0 => sa_world::collision::surface::creates_plants;
    /// `SurfaceInfos_c::CreatesObjects(id)` (0x55e9f0)
    fn creates_objects(si: SiPtr, id: u32) -> u32 = thiscall 0x55e9f0 => sa_world::collision::surface::creates_objects;
    /// `CPlantLocTri::Add(const CVector&, const CVector&, const CVector&, uint8 surface, uint8 light,
    /// bool plants, bool objects)` (0x5dc290; dwords as pushed)
    fn loc_tri_add(this: GPtr<u8>, v0: GPtr<u8>, v1: GPtr<u8>, v2: GPtr<u8>, surface: u32, light: u32, plants: u32, objects: u32) -> u32 = thiscall 0x5dc290 => unported;
    /// `ProcObjectMan_c::ProcessTriangleAdded(CPlantLocTri*)` (0x5a3f20)
    fn process_triangle_added(this: GPtr<u8>, tri: GPtr<u8>) -> u8 = thiscall 0x5a3f20 => unported;
    /// `CPlantLocTri::Release()` (0x5db6d0)
    fn loc_tri_release(this: GPtr<u8>) = thiscall 0x5db6d0 => crate::mvpb_r1::plant_loc_tri_release;
}

/// `CPlantMgr::m_UnusedLocTriListHead`, `g_surfaceInfos`, `g_procObjMan`.
const UNUSED_LOC_TRI_HEAD: usize = 0xc0_3984;
const SURFACE_INFOS: usize = 0xb7_9538;
const PROC_OBJ_MAN: usize = 0xbb_7cb0;

/// `CPlantMgr::_ProcessEntryCollisionDataSections_AddLocTris(CPlantColEntEntry* entry, const CVector&
/// centre, int32 skipMask, int32 start, int32 end)` (0x5DC8B0). Nothing without collision data. For
/// each triangle `start..=end` (skipped unless the mask is 0xFAFAFAFA or `i & 7`, when it already has a
/// loc tri, or when no unused loc tri is left): its points through `TransformPoints` with the entity
/// matrix (allocated when missing) into a separate buffer; kept when any of `centre - p` for the three
/// points, the centroid (`(v2 + (v1 + v0)) / 3`) and the edge midpoints (`(d_a + d_b) * 0.5`) has a
/// squared length (`(z² + y²) + x²`) below 10000 (the first point's difference stays on the x87
/// stack, the others are stored as f32). Then, when the surface creates plants or objects, the first
/// unused loc tri `Add(v0, v1, v2, surface, light, plants, objects)`; on success it is the entry's
/// object for the triangle; when it creates objects, `ProcessTriangleAdded` either sets its bit 2 or,
/// for a loc tri without plants, releases it again (slot cleared).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x5dc8b0, conv = cdecl, name = "CPlantMgr::_ProcessEntryCollisionDataSections_AddLocTris", expect = "81 EC E8 00 00 00",
       unverified = "renderbig2-6", route = "L2", hazards = "heap-writes,uninit-locals",
       recs = "grove-walk",
       note = "the plants / objects flags are pushed as dwords whose upper bytes are stack residue (Add reads bit 0); writes the entry's object array and loc-tri lists")]
pub fn add_loc_tris(entry: GPtr<u8>, centre: GPtr<u8>, mask: i32, start: i32, end: i32) {
    let ent = rd::<GPtr<u8>>(entry, 0);
    let cd = rd::<GPtr<u8>>(get_col_model(ent), 0x2c);
    if cd.is_null() {
        return;
    }
    let mut i = start;
    while i <= end {
        let cur = i;
        i += 1;
        if mask as u32 != 0xfafa_fafa && mask != (cur & 7) {
            continue;
        }
        let objs = rd::<GPtr<u32>>(entry, 4);
        if g::<u32>(objs.addr() + 4 * cur as usize).read() != 0 {
            continue;
        }
        if g::<u32>(UNUSED_LOC_TRI_HEAD).read() == 0 {
            continue;
        }
        let tri = rd::<GPtr<u8>>(cd, 0x18).byte_add(8 * cur as isize);
        let mut pin = [[0f32; 3]; 3];
        for (j, p) in pin.iter_mut().enumerate() {
            get_triangle_point(cd.cast(), GPtr::<f32>::from_raw(p.as_mut_ptr()).cast(), i32::from(rd::<u16>(tri, 2 * j as isize)));
        }
        if rd::<u32>(ent, 0x14) == 0 {
            allocate_matrix(ent);
            simple_transform_update_matrix(ent.byte_add(4), rd::<GPtr<u8>>(ent, 0x14));
        }
        let mut v = [[0f32; 3]; 3];
        transform_points(
            GPtr::<f32>::from_raw(v[0].as_mut_ptr()).cast(),
            3,
            rd::<GPtr<u8>>(ent, 0x14),
            GPtr::<f32>::from_raw(pin[0].as_mut_ptr()).cast(),
        );
        if !near_any(centre, &v) {
            continue;
        }
        let surface = u32::from(rd::<u8>(tri, 6));
        let plants = creates_plants(GPtr::from_addr(SURFACE_INFOS), surface) as u8;
        let objects = creates_objects(GPtr::from_addr(SURFACE_INFOS), surface) as u8;
        if plants == 0 && objects == 0 {
            continue;
        }
        let head = g::<GPtr<u8>>(UNUSED_LOC_TRI_HEAD).read();
        let mut vp = |n: usize| GPtr::<f32>::from_raw(v[n].as_mut_ptr()).cast::<u8>();
        let (p0, p1, p2) = (vp(0), vp(1), vp(2));
        if loc_tri_add(head, p0, p1, p2, surface, u32::from(rd::<u8>(tri, 7)), u32::from(plants), u32::from(objects)) == 0 {
            continue;
        }
        g::<u32>(objs.addr() + 4 * cur as usize).write(head.addr() as u32);
        if rd::<u8>(head, 0x48) & 2 == 0 {
            continue;
        }
        if process_triangle_added(GPtr::from_addr(PROC_OBJ_MAN), head) != 0 {
            wr::<u8>(head, 0x48, rd::<u8>(head, 0x48) | 4);
        } else if rd::<u8>(head, 0x48) & 1 == 0 {
            loc_tri_release(head);
            g::<u32>(rd::<u32>(entry, 4) as usize + 4 * cur as usize).write(0);
        }
    }
}


/// The seven distance tests (0x5DC9BA..0x5DCC9D): true when any squared distance is below 10000.
fn near_any(c: GPtr<u8>, v: &[[f32; 3]; 3]) -> bool {
    let (cx, cy, cz) = (rd::<f32>(c, 0), rd::<f32>(c, 4), rd::<f32>(c, 8));
    // 1/3 (0x859040), 0.5 (0x858B8C), 10000 (0x859AA4)
    let third = k(0x3eaa_aaab);
    let lim = x(k(0x461c_4000));
    let f = |v: X87| v.to_f32();
    let z01 = f(x(v[1][2]).fadd_m(v[0][2]));
    let ccx = f(x(f(x(v[2][0]).fadd(x(v[1][0]).fadd_m(v[0][0])))).fmul_m(third));
    let ccy = f(x(v[2][1]).fadd(x(v[1][1]).fadd_m(v[0][1])).fmul_m(third));
    let d1 = [f(x(cx).fsub_m(v[1][0])), f(x(cy).fsub_m(v[1][1])), f(x(cz).fsub_m(v[1][2]))];
    let d2 = [f(x(cx).fsub_m(v[2][0])), f(x(cy).fsub_m(v[2][1])), f(x(cz).fsub_m(v[2][2]))];
    let dc = [f(x(cx).fsub_m(ccx)), f(x(cy).fsub_m(ccy)), f(x(cz).fsub(x(v[2][2]).fadd_m(z01).fmul_m(third)))];
    let d0 = [x(cx).fsub_m(v[0][0]), x(cy).fsub_m(v[0][1]), x(cz).fsub_m(v[0][2])];
    let m01 = [
        f(x(d1[0]).fadd(d0[0]).fmul_m(0.5)),
        f(x(d1[1]).fadd(d0[1]).fmul_m(0.5)),
        f(x(f(x(d1[2]).fadd(d0[2]))).fmul_m(0.5)),
    ];
    let m12 = [
        f(x(d2[0]).fadd_m(d1[0]).fmul_m(0.5)),
        f(x(d2[1]).fadd_m(d1[1]).fmul_m(0.5)),
        f(x(f(x(d2[2]).fadd_m(d1[2]))).fmul_m(0.5)),
    ];
    let m02 = [
        f(x(d2[0]).fadd(d0[0]).fmul_m(0.5)),
        f(x(d2[1]).fadd(d0[1]).fmul_m(0.5)),
        f(x(f(x(d2[2]).fadd(d0[2]))).fmul_m(0.5)),
    ];
    let lt = |s: X87| !jp(s, lim, 0x5);
    let sq = |d: &[f32; 3]| x(d[2]).fmul_m(d[2]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[0]).fmul_m(d[0]));
    lt(d0[2].fmul(d0[2]).fadd(d0[1].fmul(d0[1])).fadd(d0[0].fmul(d0[0])))
        || lt(sq(&d1))
        || lt(sq(&d2))
        || lt(sq(&dc))
        || lt(sq(&m01))
        || lt(sq(&m12))
        || lt(sq(&m02))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_LOC_TRIS];
