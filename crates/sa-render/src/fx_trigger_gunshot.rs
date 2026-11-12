//! `Fx_c::TriggerGunshot` (batch `samp-render-3`), ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, k, rd};

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `Multiply3x3(CVector* out, const CVector& v, const CMatrix& m)` (0x59C810): the transposed product; returns `out`
    fn multiply3x3_transposed(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `FxManager_c::FxRwMatrixCreate()` (0x4A9440) / `FxRwMatrixDestroy(RwMatrix*)` (0x4A9460)
    fn fx_rw_matrix_create(fx: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9440 => unported;
    fn fx_rw_matrix_destroy(fx: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4A9460 => unported;
    /// `Fx_c::CreateMatFromVec(RwMatrix* out, CVector* origin, CVector* direction)` (0x49E950, thiscall)
    fn fx_create_mat_from_vec(this: GPtr<u8>, out: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>) = thiscall 0x49E950 => unported;
    /// `FxManager_c::CreateFxSystem(const char* name, CVector* point, RwMatrix* parent, bool)` (0x4A9BE0, thiscall)
    fn fx_create_system(fx: GPtr<u8>, name: u32, point: GPtr<u8>, parent: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::CopyParentMatrix()` (0x4AA890) / `PlayAndKill()` (0x4AA3D0)
    fn fx_system_copy_parent_matrix(s: GPtr<u8>) = thiscall 0x4AA890 => unported;
    fn fx_system_play_and_kill(s: GPtr<u8>) = thiscall 0x4AA3D0 => unported;
}

/// `g_fxMan` (0xA9AE80); the effect names "gunflash" (0x85A728) and "gunsmoke" (0x85A71C).
const FX_MAN: usize = 0xA9_AE80;
const GUNFLASH: u32 = 0x85_A728;
const GUNSMOKE: u32 = 0x85_A71C;

/// `Fx_c::TriggerGunshot(CEntity* entity, CVector& origin, CVector& target, bool doGunflash)` (0x4A0DE0, thiscall,
/// `ret 0x10`): nothing unless the squared distance from the camera, `(dz² + dx²) + dy²` (x87), is at most 1265.0f
/// (0x85A6E8; NaN passes). With an entity: the offset `origin - entity position` (f32 per axis) is turned into the
/// entity's frame by `Multiply3x3(offset, matrix)` (transposed; the matrix allocated and updated from the placement
/// when missing), and the parent matrix is the entity's RW object frame matrix (`rwObject->frame + 0x10`; no RW
/// object: nothing). Without one: a matrix from `FxRwMatrixCreate`, `CreateMatFromVec(matrix, origin, target)`,
/// the point (0, 0, 0). With a parent: with `doGunflash` a "gunflash" system, then a "gunsmoke" system at the point
/// (`CreateFxSystem(name, &point, parent, false)`), each, when created, `CopyParentMatrix()` (no entity only) and
/// `PlayAndKill()`. The created matrix is destroyed at the end.
#[hook(addr = 0x4A0DE0, conv = thiscall, name = "Fx_c::TriggerGunshot", expect = "A1 3C F0 B6 00",
       unverified = "samp-render-3", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "creates FX systems from the FX pools")]
pub fn fx_trigger_gunshot(this: GPtr<u8>, entity: GPtr<u8>, origin: GPtr<u8>, target: GPtr<u8>, gunflash: u32) {
    let m = g::<u32>(0xB6_F03C).read();
    let cam = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { g::<u8>(0xB6_F02C) };
    let dx = X87::from_f32(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(origin, 0));
    let dy = X87::from_f32(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(origin, 4));
    let dz = X87::from_f32(rd::<f32>(cam, 8)).fsub_m(rd::<f32>(origin, 8));
    let sq = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if !sq.fcom(X87::from_f32(k(0x441C_4000))).test_any(0x41) {
        return;
    }
    let fx = g::<u8>(FX_MAN);
    let mut point = [0f32; 3];
    let mut created = GPtr::<u8>::NULL;
    let parent = if !entity.is_null() {
        let mat = rd::<GPtr<u8>>(entity, 0x14);
        let pos = if mat.is_null() { entity.byte_add(4) } else { mat.byte_add(0x30) };
        let mut v = [0f32; 3];
        for (i, o) in v.iter_mut().enumerate() {
            let off = 4 * i as isize;
            *o = X87::from_f32(rd::<f32>(origin, off)).fsub_m(rd::<f32>(pos, off)).to_f32();
        }
        if mat.is_null() {
            placeable_allocate_matrix(entity);
            simple_transform_update_matrix(entity.byte_add(4), rd::<GPtr<u8>>(entity, 0x14));
        }
        let mut out = [0f32; 3];
        let r = multiply3x3_transposed(GPtr::from_raw(out.as_mut_ptr()).cast(), GPtr::from_raw(v.as_mut_ptr()).cast(),
            rd::<GPtr<u8>>(entity, 0x14));
        point = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        let rw = rd::<GPtr<u8>>(entity, 0x18);
        if rw.is_null() {
            return;
        }
        rd::<GPtr<u8>>(rw, 4).byte_add(0x10)
    } else {
        created = fx_rw_matrix_create(fx);
        fx_create_mat_from_vec(this, created, origin, target);
        created
    };
    if !parent.is_null() {
        let pp = GPtr::from_raw(point.as_mut_ptr()).cast::<u8>();
        let names: &[u32] = if gunflash as u8 != 0 { &[GUNFLASH, GUNSMOKE] } else { &[GUNSMOKE] };
        for &name in names {
            let s = fx_create_system(fx, name, pp, parent, 0);
            if !s.is_null() {
                if entity.is_null() {
                    fx_system_copy_parent_matrix(s);
                }
                fx_system_play_and_kill(s);
            }
        }
    }
    if !created.is_null() {
        fx_rw_matrix_destroy(fx, created);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FX_TRIGGER_GUNSHOT];
