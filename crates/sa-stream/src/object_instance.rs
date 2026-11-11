//! STD-5: `CFileLoader::LoadObjectInstance(CFileObjectInstance*, const char*)` (0x538090), the IPL
//! `inst` entry → entity, ported from the 1.0 US bytes (gta-reversed `FileLoader.cpp` as the reading
//! aid; the bytes win). The SEH frame of the `new` calls is not reproduced.
//!
//! `CFileObjectInstance`: +0 position, +0xC rotation (x, y, z, w), +0x1C model id, +0x20 flags (low
//! byte = area; 0x100 redundant stream, 0x200 don't stream, 0x400 underwater, 0x800 tunnel, 0x1000
//! tunnel transition), +0x24 LOD index. Entity: +4 the simple transform (position, heading +0x10), +0x14
//! matrix, +0x1C flags, +0x22 model index, +0x2F area, +0x30 LOD index.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CDummy::operator new(size)` (0x5326E0)
    fn dummy_new(size: u32) -> GPtr<u8> = cdecl 0x5326E0 => unported;
    /// `CDummyObject::CDummyObject()` (0x59EA00)
    fn dummy_object_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x59EA00 => unported;
    /// `CBuilding::operator new(size)` (0x404090)
    fn building_new(size: u32) -> GPtr<u8> = cdecl 0x404090 => unported;
    /// `CAnimatedBuilding::CAnimatedBuilding()` (0x537A10)
    fn animated_building_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x537A10 => unported;
    /// `CBuilding::CBuilding()` (0x403E00)
    fn building_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x403E00 => unported;
    /// virtual `CEntity::SetModelIndexNoCreate(uint32)` (+0x18)
    fn set_model_index_no_create(this: GPtr<u8>, model: u32) = thiscall vslot 0x18 => unported;
    /// virtual `CEntity::GetBoundRect()` (+0x24, the CRect returned through a hidden pointer)
    fn get_bound_rect(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// virtual `CBaseModelInfo::GetModelType()` (+0x10, AL)
    fn model_type(this: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CGlass::IsObjectGlass(CEntity*)` (0x46A760)
    fn is_object_glass(e: GPtr<u8>) -> u8 = cdecl 0x46A760 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn set_rotate_z_only(m: GPtr<u8>, angle: f32) = thiscall 0x59B020 => unported;
    /// `CPlaceable::AllocateStaticMatrix()` (0x54F4C0)
    fn allocate_static_matrix(this: GPtr<u8>) = thiscall 0x54F4C0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(this: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix::SetRotate(const CQuaternion&)` (0x59BBF0)
    fn set_rotate_quat(m: GPtr<u8>, q: GPtr<u8>) = thiscall 0x59BBF0 => unported;
    /// `CObject::SetMatrixForTrainCrossing(CMatrix*, float)` (0x59F200)
    fn set_matrix_for_train_crossing(m: GPtr<u8>, angle: u32) = cdecl 0x59F200 => unported;
    /// `CColStore::GetBoundingBox(int32)` (0x410800)
    fn col_store_get_bounding_box(slot: u32) -> GPtr<u8> = cdecl 0x410800 => unported;
    /// `CRect::Restrict(const CRect&)` (0x404200)
    fn rect_restrict(this: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> = thiscall 0x404200 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `_CIacos`.
const CI_ACOS: usize = 0x822380;

fn rd32(p: GPtr<u8>, o: isize) -> u32 {
    p.byte_add(o).cast::<u32>().read()
}
fn rdf(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}
fn wr32(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(v)
}
fn or_flags(e: GPtr<u8>, bits: u32) {
    wr32(e, 0x1C, rd32(e, 0x1C) | bits);
}
fn model_info(id: u32) -> GPtr<u8> {
    GPtr::from_addr(GPtr::<u32>::from_addr(MODEL_INFO_PTRS + 4 * id as usize).read() as usize)
}
fn matrix(e: GPtr<u8>) -> GPtr<u8> {
    GPtr::from_addr(rd32(e, 0x14) as usize)
}
/// The matrix, allocated (and set from the simple transform) when the entity has none.
fn ensure_matrix(e: GPtr<u8>) {
    if rd32(e, 0x14) == 0 {
        allocate_matrix(e);
        update_matrix(e.byte_add(4), matrix(e));
    }
}

/// `CFileLoader::LoadObjectInstance(CFileObjectInstance*, const char*)` (0x538090): NULL without a model
/// info. A model with object data (+0x10 != -1) makes a `CDummyObject` (invisible when it is glass that
/// is not of glass type 2: flags +0x12 & 0x7800 != 0x2800); else a `CAnimatedBuilding` (clump model with
/// an anim blend, byte +0x13 bit 0) or a `CBuilding` (don't-cast-shadows from model flag 0x10, invisible
/// below draw distance 2). Rotation: when |x| > 0.05, |y| > 0.05, or (don't stream, x != 0, y != 0) the
/// negated quaternion through a static matrix; else the heading `acos(w) * (z < 0 ? 2 : -2)` (matrix
/// `SetRotateZOnly` or the transform's heading). Position, the flags, area, LOD index; the train
/// crossing model gets its static matrix set; the collision model restricts its slot's area to the
/// bound rect (or clears uses-collision without volumes) and marks the entity underwater when its box
/// bottom is below 0.
#[hook(addr = 0x538090, conv = cdecl, name = "CFileLoader::LoadObjectInstance-inst", expect = "6A FF 68 31 C9 83 00 64 A1 00",
       unverified = "std5-stream-1", route = "L2", hazards = "heap-writes,x87-transcendental", recs = "new-game")]
pub fn load_object_instance(inst: GPtr<u8>, _name: GPtr<u8>) -> GPtr<u8> {
    let id = rd32(inst, 0x1C);
    let mi = model_info(id);
    if mi.is_null() {
        return GPtr::NULL;
    }
    let e;
    if mi.byte_add(0x10).cast::<i16>().read() != -1 {
        let p = dummy_new(0x38);
        e = if p.is_null() { p } else { dummy_object_ctor(p) };
        set_model_index_no_create(e, rd32(inst, 0x1C));
        if is_object_glass(e) != 0 {
            let mi2 = model_info(e.byte_add(0x22).cast::<i16>().read() as i32 as u32);
            if u32::from(mi2.byte_add(0x12).cast::<u16>().read()) & 0x7800 != 0x2800 {
                wr32(e, 0x1C, rd32(e, 0x1C) & !0x80);
            }
        }
    } else {
        let animated = model_type(mi) == 5 && mi.byte_add(0x13).read() & 1 != 0;
        let p = building_new(0x38);
        e = if p.is_null() {
            p
        } else if animated {
            animated_building_ctor(p)
        } else {
            building_ctor(p)
        };
        set_model_index_no_create(e, rd32(inst, 0x1C));
        if mi.byte_add(0x12).read() & 0x10 != 0 {
            or_flags(e, 0x10000);
        }
        let dd = rdf(mi, 0x18);
        if dd < GPtr::<f32>::from_addr(0x858CA0).read() {
            wr32(e, 0x1C, rd32(e, 0x1C) & !0x80);
        }
    }
    let lim = GPtr::<f32>::from_addr(0x858C28).read();
    let zero = GPtr::<f32>::from_addr(0x858B50).read();
    let (rx, ry) = (rdf(inst, 0xC), rdf(inst, 0x10));
    // `fabs; fcomp; test ah, 0x41; je` = greater (ordered)
    let quat = X87::from_f32(rx).fabs().to_f64() > f64::from(lim)
        || X87::from_f32(ry).fabs().to_f64() > f64::from(lim)
        || (rd32(inst, 0x20) & 0x200 != 0 && rx != zero && ry != zero);
    if quat {
        for o in [0xC, 0x10, 0x14] {
            inst.byte_add(o).cast::<f32>().write(X87::from_f32(rdf(inst, o)).fchs().to_f32());
        }
        allocate_static_matrix(e);
        let mut q = [rd32(inst, 0xC), rd32(inst, 0x10), rd32(inst, 0x14), X87::from_f32(rdf(inst, 0x18)).to_f32().to_bits()];
        ensure_matrix(e);
        set_rotate_quat(matrix(e), GPtr::from_raw(q.as_mut_ptr()).cast());
    } else {
        let a = X87::from_f64_exact(sa_core::orig::x87_unary(CI_ACOS, f64::from(rdf(inst, 0x18))));
        let rz = rdf(inst, 0x14);
        let h = if rz < zero { a.fdouble() } else { a.fmul_m(GPtr::<f32>::from_addr(0x858B18).read()) }.to_f32();
        if rd32(e, 0x14) != 0 {
            set_rotate_z_only(matrix(e), h);
        } else {
            e.byte_add(0x10).cast::<f32>().write(h);
        }
    }
    let pos = if rd32(e, 0x14) != 0 { matrix(e).byte_add(0x30) } else { e.byte_add(4) };
    for k in 0..3isize {
        pos.byte_add(4 * k).cast::<f32>().write(X87::from_f32(rdf(inst, 4 * k)).to_f32());
    }
    let fl = rd32(inst, 0x20);
    if fl & 0x400 != 0 {
        or_flags(e, 0x10_0000);
    }
    if fl & 0x800 != 0 {
        or_flags(e, 0x4000_0000);
    }
    if fl & 0x1000 != 0 {
        or_flags(e, 0x8000_0000);
    }
    if fl & 0x100 != 0 {
        or_flags(e, 0x2000_0000);
    }
    e.byte_add(0x2F).write(fl as u8);
    wr32(e, 0x30, rd32(inst, 0x24));
    if rd32(inst, 0x1C) == u32::from(GPtr::<u16>::from_addr(0x8CD6B4).read()) {
        ensure_matrix(e);
        allocate_static_matrix(e);
        ensure_matrix(e);
        set_matrix_for_train_crossing(matrix(e), 0x3FAC_E9CC);
    }
    let cm = GPtr::<u8>::from_addr(rd32(model_info(e.byte_add(0x22).cast::<i16>().read() as i32 as u32), 0x14) as usize);
    if !cm.is_null() {
        if cm.byte_add(0x29).read() & 1 == 0 {
            wr32(e, 0x1C, rd32(e, 0x1C) & !1);
        } else {
            let slot = cm.byte_add(0x28).read();
            if slot != 0 {
                let mut rect = [0u32; 4];
                let r = get_bound_rect(e, GPtr::from_raw(rect.as_mut_ptr()).cast());
                let bb = col_store_get_bounding_box(u32::from(slot));
                rect_restrict(bb, r);
            }
        }
        let pos = if rd32(e, 0x14) != 0 { matrix(e).byte_add(0x30) } else { e.byte_add(4) };
        let mcm = GPtr::<u8>::from_addr(rd32(mi, 0x14) as usize);
        if X87::from_f32(rdf(mcm, 8)).fadd_m(rdf(pos, 8)).to_f64() < f64::from(zero) {
            or_flags(e, 0x10_0000);
        }
    }
    e
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_LOAD_OBJECT_INSTANCE];
