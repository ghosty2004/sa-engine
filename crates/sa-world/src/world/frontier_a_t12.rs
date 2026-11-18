//! FRONTIER-2 turn 12, lane A (batches `frontier2-a19`..): small WORLD ports from the 1.0 US bytes (gta-reversed read
//! as an aid; where it differs the bytes win).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use sa_core::bindings::{cross_product, vector_normalise};
use sa_core::field::ptr_of as p;

use super::common::{f32_of, g, wr, x};
use super::entity::get_matrix;

/// `CWeather::WindDir`
const WIND_DIR: usize = 0xC8_13E0;

/// `CEntity::BuildWindSockMatrix()` (0x532DB0, thiscall): forward = normalised `(-(wind.x + 0.01), -(wind.y + 0.01),
/// 0.1)` (the bytes negate both sums, gta-reversed does not), right = normalised `(0, 0, 1) x forward`, up =
/// `forward x right`; the nine axis floats are written through their own inlined `GetMatrix()`, then the inlined
/// `UpdateRwMatrix` / `UpdateRwFrame`.
#[hook(addr = 0x532DB0, conv = thiscall, name = "CEntity::BuildWindSockMatrix", this = "CEntity",
       expect = "83 EC 30 A1 E0 13 C8 00", unverified = "frontier2-a19", route = "L2", hazards = "heap-writes",
       note = "wind sock objects (CEntity::PreRender)")]
pub fn entity_build_wind_sock_matrix(this: GPtr<u8>) {
    let w = g::<[f32; 3]>(WIND_DIR).read();
    let mut fwd = [f32_of(x(w[0]).fadd_m(0.01).fchs()), f32_of(x(w[1]).fadd_m(0.01).fchs()), 0.1f32];
    vector_normalise(GPtr::from_raw(&mut fwd));
    let mut up0 = [0.0f32, 0.0, 1.0];
    let mut out = [0.0f32; 3];
    let mut right = cross_product(p(&mut out), p(&mut up0), p(&mut fwd)).cast::<[f32; 3]>().read();
    vector_normalise(GPtr::from_raw(&mut right));
    let up = cross_product(p(&mut out), p(&mut fwd), p(&mut right)).cast::<[f32; 3]>().read();
    for (o, v) in [(0, right), (0x10, fwd), (0x20, up)] {
        for (i, f) in v.into_iter().enumerate() {
            wr::<f32>(get_matrix(this), o + 4 * i as isize, f);
        }
    }
    super::mvpd_w1::update_rw_matrix_and_frame(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ENTITY_BUILD_WIND_SOCK_MATRIX];
