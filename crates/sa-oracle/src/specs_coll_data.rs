//! L1 specs for PORT-COLL `sa_world::collision::data` (see `docs/arch/coll-port.md`): the two pure
//! `CColTrianglePlane` functions. The hooked entries run on the host on a copy of the scratch image.
//! `Set`'s callees `CrossProduct` 0x59C730, `CVector::Normalise` 0x59C910 and `_ftol` 0x821B40 run as
//! original code under Unicorn and as their Rust ports (`orig!` host paths) on the host.

use crate::call::Conv::*;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_world::collision::data;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// Triangle indices (region 1, words 0..1) in 0..4 (region 0 holds 4 compressed vertices); the
/// vertices are small integers a third of the time (degenerate and axis-aligned triangles).
fn fix_tri(v: &mut [u32], lay: &Layout) {
    let w = &lay.word_slot[1];
    let (s0, s1) = (w[0].unwrap(), w[1].unwrap());
    let a = v[s0] & 3;
    let b = (v[s0] >> 16) & 3;
    let c = v[s1] & 3;
    v[s0] = a | (b << 16);
    v[s1] = (v[s1] & 0xFFFF_0000) | c;
    let wv = &lay.word_slot[0];
    let seed = v[wv[0].unwrap()];
    if seed.is_multiple_of(3) {
        for (k, slot) in wv.iter().enumerate() {
            let x = v[slot.unwrap()];
            let lo = ((x as i32 % 5) as i16 as u16 as u32) * (k as u32 % 3 + 1);
            let hi = (((x >> 8) as i32 % 5) as i16 as u16 as u32) << 16;
            v[slot.unwrap()] = (lo & 0xFFFF) | hi;
        }
    }
}

pub fn all() -> Vec<Spec> {
    use sa_core::pools::COL_MODEL_POOL;
    vec![
        // `CColModel::operator new/delete`: the CORE pool spec builders (pool global patched to scratch)
        crate::specs_core::op_new_spec(
            "CColModelOperatorNew",
            "CColModel::operator new",
            0x40FC30,
            &[0x40FB80],
            0xB744A4,
            0x30,
            &COL_MODEL_POOL,
            data::col_model_operator_new,
        ),
        crate::specs_core::op_delete_spec(
            "CColModelOperatorDelete",
            "CColModel::operator delete",
            0x40FC40,
            0xB744A4,
            0x30,
            &COL_MODEL_POOL,
            data::col_model_operator_delete,
        ),
        b(
            "CColTrianglePlaneGetNormal",
            "CColTrianglePlane::GetNormal-Out",
            0x411610,
            Thiscall,
            Void,
        )
        .regions(vec![raw(3), raw(3)])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| {
            data::get_normal(tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
        b(
            "CColTrianglePlaneSet",
            "CColTrianglePlane::Set",
            0x411660,
            Thiscall,
            Void,
        )
        .regions(vec![raw(6), raw(2), raw(3)])
        .args(vec![A::P(2), A::P(0), A::P(1)])
        .callees(&[0x59C730, 0x59C910, 0x821B40])
        .fix(|v, l| fix_tri(v, l))
        .rust(|c| {
            data::triangle_plane_set(tp(c, 2), tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
    ]
}
