//! L1 specs for PORT-ANIM-INPUT fork C (`sa_anim::{sequence, hierarchy, node, assoc, clumpdata,
//! group}`; see `docs/arch/anim-input-port.md`): the leaves that only touch `this` and pointer
//! arguments. Functions that follow pointer fields stored inside the objects (frames, sequences,
//! hierarchies, nodes) are not run on the host: their fields are 4 bytes in the original and 8 bytes
//! in the host build, so they are left to L2. `CAnimBlendNode::CalcTheta` calls `_CIacos` and FSIN
//! (not faithful under Unicorn, no native backend in this pass): L2 only.

use crate::call::Conv::*;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_anim::{assoc, clumpdata, group, hierarchy, node, sequence};
use sa_core::mem::GPtr;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "AnimSeqConstructor",
            "CAnimBlendSequence::Constructor",
            0x4D0C10,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(3)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let r = sequence::constructor(tp(c, 0));
            ExpRet::Eax(
                (r.addr() as u32)
                    .wrapping_sub(c.mem.as_ptr() as u32)
                    .wrapping_add(crate::call::SCRATCH),
            )
        }),
        b(
            "AnimSeqSetBoneTag",
            "CAnimBlendSequence::SetBoneTag",
            0x4D0C70,
            Thiscall,
            Void,
        )
        .regions(vec![raw(3)])
        .args(vec![
            A::P(0),
            A::I(|r| if r.chance(30) { u32::MAX } else { r.next_u32() }),
        ])
        .rust(|c| {
            sequence::set_bone_tag(tp(c, 0), c.i(1) as i32);
            ExpRet::Void
        }),
        b(
            "AnimSeqGetDataSize",
            "CAnimBlendSequence::GetDataSize",
            0x4D0C90,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(3)])
        .args(vec![A::P(0), A::I(g_bool)])
        .rust(|c| ExpRet::Eax(sequence::get_data_size(tp(c, 0), c.i(1) as u8))),
        b(
            "AnimHierConstructor",
            "CAnimBlendHierarchy::Constructor",
            0x4CF270,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(6)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let r = hierarchy::constructor(tp(c, 0));
            ExpRet::Eax(
                (r.addr() as u32)
                    .wrapping_sub(c.mem.as_ptr() as u32)
                    .wrapping_add(crate::call::SCRATCH),
            )
        }),
        b("AnimCompressQuaternion", "FUN_004cfa30", 0x4CFA30, Cdecl, Void)
            .regions(vec![raw(2), quat()])
            .args(vec![A::P(0), A::P(1)])
            .callees(&[0x821B40])
            .rust(|c| {
                hierarchy::compress_quaternion(tp(c, 0), tp(c, 1));
                ExpRet::Void
            }),
        b("AnimNodeInit", "CAnimBlendNode::Init", 0x4CFB70, Thiscall, Void)
            .regions(vec![raw(6)])
            .args(vec![A::P(0)])
            .rust(|c| {
                node::init(tp(c, 0));
                ExpRet::Void
            }),
        b(
            "AnimAssocConstructor0",
            "CAnimBlendAssociation::Constructor0-",
            0x4CE9B0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(15)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let r = assoc::constructor0(tp(c, 0));
            ExpRet::Eax(
                (r.addr() as u32)
                    .wrapping_sub(c.mem.as_ptr() as u32)
                    .wrapping_add(crate::call::SCRATCH),
            )
        }),
        b(
            "AnimAssocSetBlendTo",
            "CAnimBlendAssociation::SetBlendTo",
            0x4CEB80,
            Thiscall,
            Void,
        )
        .regions(vec![floats(15)])
        .args(vec![A::P(0), A::F, A::F])
        .rust(|c| {
            let (t, d) = (c.f(1), c.f(2));
            assoc::set_blend_to(tp(c, 0), t, d);
            ExpRet::Void
        }),
        b(
            "AnimAssocSetBlend",
            "CAnimBlendAssociation::SetBlend",
            0x4CEBA0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(15)])
        .args(vec![A::P(0), A::I(g_rand), A::I(g_rand)])
        .rust(|c| {
            let (a, d) = (c.i(1), c.i(2));
            assoc::set_blend(tp(c, 0), a, d);
            ExpRet::Void
        }),
        b(
            "AnimAssocSetDeleteCallback",
            "CAnimBlendAssociation::SetDeleteCallback",
            0x4CEBC0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(15)])
        .args(vec![A::P(0), A::I(g_rand), A::I(g_rand)])
        .rust(|c| {
            let (a, d) = (c.i(1), c.i(2));
            assoc::set_delete_callback(tp(c, 0), a, d);
            ExpRet::Void
        }),
        b(
            "AnimAssocSetFinishCallback",
            "CAnimBlendAssociation::SetFinishCallback",
            0x4CEBE0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(15)])
        .args(vec![A::P(0), A::I(g_rand), A::I(g_rand)])
        .rust(|c| {
            let (a, d) = (c.i(1), c.i(2));
            assoc::set_finish_callback(tp(c, 0), a, d);
            ExpRet::Void
        }),
        b(
            "AnimStaticConstructor1",
            "CAnimBlendStaticAssociation::Constructor1",
            0x4CE940,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(5)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let r = assoc::static_constructor1(tp(c, 0));
            ExpRet::Eax(
                (r.addr() as u32)
                    .wrapping_sub(c.mem.as_ptr() as u32)
                    .wrapping_add(crate::call::SCRATCH),
            )
        }),
        b(
            "AnimClumpDataConstructor",
            "CAnimBlendClumpData::Constructor",
            0x4CF0E0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(5)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let r = clumpdata::constructor(tp(c, 0));
            ExpRet::Eax(
                (r.addr() as u32)
                    .wrapping_sub(c.mem.as_ptr() as u32)
                    .wrapping_add(crate::call::SCRATCH),
            )
        }),
        b(
            "AnimGroupConstructor",
            "CAnimBlendAssocGroup::Constructor",
            0x4CDE70,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(5)])
        .args(vec![A::P(0)])
        .rust(|c| {
            let r = group::constructor(tp(c, 0));
            ExpRet::Eax(
                (r.addr() as u32)
                    .wrapping_sub(c.mem.as_ptr() as u32)
                    .wrapping_add(crate::call::SCRATCH),
            )
        }),
    ]
}
