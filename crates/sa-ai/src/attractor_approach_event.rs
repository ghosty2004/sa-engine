//! `CPedAttractorManager::IsApproachable` (0x5EA220) and the `CEventAttractor` constructor (0x4AF350): the ambient
//! peds' attractor scan (batch `std11-ai-1`). Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it
//! differs, the bytes are followed). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CVector::FromMultiply(const CMatrix&, const CVector&)` (0x59C670, thiscall on the result, `ret 8`)
    fn vector_from_multiply(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x59C670 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790, cdecl)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CPedGeometryAnalyser::IsWanderPathClear(const CVector&, const CVector&, float, int32)` (0x5F2F70)
    fn is_wander_path_clear(a: GPtr<u8>, b: GPtr<u8>, max_height_diff: f32, max_steps: i32) -> u32 = cdecl 0x5F2F70 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, thiscall, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) with a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// `CPedAttractorManager::IsApproachable(C2dEffectPedAttractor* fx, const CMatrix& mat, int32 slot, CPed* ped)`
/// (0x5EA220, cdecl). A shelter attractor (type +0x34 == 4): `CWorld::GetIsLineOfSightClear(ped position, mat * fx
/// position, buildings only)`. Otherwise, x87: C = `Multiply3x3(mat, fx +0x28)` (the forward direction, stored f32),
/// D = `mat * fx position`, P = the ped's position; fails unless `(C.z*P.z + C.x*P.x + C.y*P.y) - (D.x*C.x + (D.z*C.z
/// + D.y*C.y))` is above [0x858B50]; E = (f32(P.x - D.x), f32(P.y - D.y), 0) normalised; fails unless `(E.y*C.y +
/// C.z*E.z) + E.x*C.x` is above [0x86C534]; then AL = `IsWanderPathClear(P, D, 2.0, 0) == 4`.
///
/// Returns AL; EAX's upper bytes are the FPU status / pointer residue of the original (not reproduced).
#[hook(addr = 0x5EA220, conv = cdecl, name = "CPedAttractorManager::IsApproachable", expect = "83 EC 30 56 8B 74 24 38",
       unverified = "std11-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn is_approachable(fx: GPtr<u8>, mat: GPtr<u8>, _slot: i32, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(fx, 0x34) == 4 {
        let mut d = [0f32; 3];
        vector_from_multiply(vp(&mut d), mat, fx);
        return world_get_is_line_of_sight_clear(pos(ped), vp(&mut d), 1, 0, 0, 0, 0, 0, 0);
    }
    let mut a = [rd::<f32>(fx, 0x28), rd::<f32>(fx, 0x2C), rd::<f32>(fx, 0x30)];
    let mut b = [0f32; 3];
    let r = multiply3x3(vp(&mut b), mat, vp(&mut a));
    let c = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let mut d = [0f32; 3];
    vector_from_multiply(vp(&mut d), mat, fx);
    let neg = x(d[2]).fmul_m(c[2]).fadd(x(d[1]).fmul_m(c[1])).fadd(x(d[0]).fmul_m(c[0])).fchs();
    let p = pos(ped);
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let s = x(c[2]).fmul_m(pz).fadd(x(c[0]).fmul_m(px)).fadd(x(c[1]).fmul_m(py)).fadd(neg);
    if s.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
        return 0;
    }
    let mut e = [f32_of(x(px).fsub_m(d[0])), f32_of(x(py).fsub_m(d[1])), 0.0];
    vector_normalise(vp(&mut e));
    let dot = x(e[1]).fmul_m(c[1]).fadd(x(c[2]).fmul_m(e[2])).fadd(x(e[0]).fmul_m(c[0]));
    if dot.fcom(x(g::<f32>(0x86C534).read())).test_any(0x41) {
        return 0;
    }
    u8::from(is_wander_path_clear(p, vp(&mut d), 2.0, 0) == 4)
}

/// `CEventAttractor::CEventAttractor(C2dEffectPedAttractor* fx, CEntity* entity, bool avoid_looking)` (0x4AF350,
/// thiscall, `ret 0xC`): the `CEventEditableResponse` base inline (+4 = 0, byte +8 = 0, byte +0xC = 1, word +0xE =
/// 200, word +0x10 = 0xFFFF), +0x14 = fx, vtable 0x85B1B8, +0x18 = entity, byte +0x1C = avoid_looking; a set entity
/// registers the reference at +0x18. Returns `this`.
#[hook(addr = 0x4AF350, conv = thiscall, name = "CEventAttractor::Constructor", expect = "6A FF 68 48 9E 83 00",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_attractor_ctor(this: GPtr<u8>, fx: GPtr<u8>, entity: GPtr<u8>, avoid: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<GPtr<u8>>(this, 0x14, fx);
    wr::<u32>(this, 0, 0x85B1B8);
    wr::<GPtr<u8>>(this, 0x18, entity);
    wr::<u8>(this, 0x1C, avoid as u8);
    if !entity.is_null() {
        entity_register_reference(entity, this.byte_add(0x18));
    }
    this
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_APPROACHABLE, HOOK_EVENT_ATTRACTOR_CTOR];

