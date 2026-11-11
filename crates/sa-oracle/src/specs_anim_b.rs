//! L1 specs for PORT-ANIM-INPUT fork D (`sa_anim::{bones, ik, pedik, vehgroup}`; see
//! `docs/arch/anim-input-port.md`): the pure leaves of `BoneNode_c`, `IKChain_c`, `CPedIK` and
//! `CVehicleAnimGroup`. The hooked entries run on the host on a copy of the scratch image; the
//! `ms_boneInfos` table is sa-anim's host global (initialised from the exe image, which Unicorn
//! maps too). FPATAN / FSIN / FCOS functions are routed to L2 (Unicorn unfaithful; the native
//! backend decides them when it is available).

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, FPATAN_REASON, Inputs, RetKind::*, Route, Spec};
use sa_anim::{bones, ik, pedik, vehgroup};
use sa_core::mem::GPtr;

const FPATAN: Route = Route::L2 {
    reason: FPATAN_REASON,
};
/// `ms_boneInfos` with the 0x28 bytes before it and the three test flags (0x8D2BD0..0x8D2BD2).
const BONE_INFOS: (u32, u32) = (0x8D26A8, 0x8D2BD3);
/// The CRT's `__sse2_available` (as the game has it at runtime: 1).
const SSE2_AVAILABLE: u32 = 0xC9C400;
const FSIN: Route = Route::L2 {
    reason: crate::math_specs::FSIN_REASON,
};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0xB0_4E_D0;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// A bone tag: 75 % one of the 32 tags of `ms_boneInfos`, else small or random.
fn g_tag(r: &mut Rng) -> u32 {
    match r.below(8) {
        0..=5 => i32::from(bones::bone_info(r.below(32) as i32).cast::<i16>().read()) as u32,
        6 => r.below(64),
        _ => r.next_u32(),
    }
}

/// A `CVehicleAnimGroup` (0x94): the group bytes and flags raw, timings and door offsets floats.
fn veh_group_region() -> Region {
    let mut w = vec![W::R; 3];
    w.extend([W::F; 34]);
    Region {
        words: w,
        kind: Kind::Any,
    }
}

fn g_axis(r: &mut Rng) -> u32 {
    if r.chance(85) { r.below(3) } else { r.next_u32() }
}

/// A `BoneNode_c` (0x98): raw list/tag words, float quaternion and position, raw parent/children/
/// world matrix, float limits and speed.
fn bone_region() -> Region {
    let mut w = vec![W::R; 4];
    w.extend([W::F; 7]);
    w.extend([W::R; 20]);
    w.extend([W::F; 7]);
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// Tag word (+8) of the bone in region `r` from [`g_tag`].
fn fix_bone_tag(v: &mut Inputs, l: &Layout, r: usize) {
    let mut g = Rng::new(seed(v));
    let s = l.word_slot[r][2].unwrap();
    v[s] = g_tag(&mut g);
}

/// Quaternions on the gimbal lines (`n = ±1` exactly: (0, ±0.5, 0, 1)) in 10 % of the cases.
fn fix_gimbal(v: &mut Inputs, l: &Layout, r: usize, first: usize) {
    let mut g = Rng::new(seed(v));
    if g.chance(10) {
        let s = |k: usize| l.word_slot[r][first + k].unwrap();
        let y = if g.chance(50) { 0.5f32 } else { -0.5 };
        v[s(0)] = 0f32.to_bits();
        v[s(1)] = y.to_bits();
        v[s(2)] = 0f32.to_bits();
        v[s(3)] = 1f32.to_bits();
    }
}

/// Anim ids around the vehicle range 351..390 (and 377), small ids and others below 1000.
fn g_anim(r: &mut Rng) -> u32 {
    match r.below(10) {
        0..=5 => 351 + r.below(41),
        6 | 7 => r.below(40),
        8 => (r.below(2000) as i32 - 1000) as u32,
        _ => 377,
    }
}

pub fn all() -> Vec<Spec> {
    vec![
        b("BoneEulerToQuat", "BoneNode_c::EulerToQuat", 0x6171F0, Cdecl, Void)
            .regions(vec![vec3(), raw(4)])
            .args(vec![A::P(0), A::P(1)])
            .route(FSIN)
            .rust(|c| {
                bones::euler_to_quat(tp(c, 0), tp(c, 1));
                ExpRet::Void
            }),
        b("BoneGetLimits", "BoneNode_c::GetLimits", 0x616BF0, Thiscall, Void)
            .regions(vec![bone_region(), raw(1), raw(1)])
            .args(vec![A::P(0), A::I(g_axis), A::P(1), A::P(2)])
            .rust(|c| {
                bones::get_limits(tp(c, 0), c.i(1) as i32, tp(c, 1), tp(c, 2));
                ExpRet::Void
            }),
        b("BoneSetLimits", "BoneNode_c::SetLimits", 0x616C50, Thiscall, Void)
            .regions(vec![bone_region()])
            .args(vec![A::P(0), A::I(g_axis), A::F, A::F])
            .rust(|c| {
                let (a, mn, mx) = (c.i(1) as i32, c.i(2), c.i(3));
                bones::set_limits(tp(c, 0), a, mn, mx);
                ExpRet::Void
            }),
        b("BoneGetSpeed", "BoneNode_c::GetSpeed", 0x616CB0, Thiscall, St0)
            .regions(vec![bone_region()])
            .args(vec![A::P(0)])
            .rust(|c| st0(bones::get_speed(tp(c, 0)))),
        b("BoneSetSpeed", "BoneNode_c::SetSpeed", 0x616CC0, Thiscall, Void)
            .regions(vec![bone_region()])
            .args(vec![A::P(0), A::F])
            .rust(|c| {
                let s = c.i(1);
                bones::set_speed(tp(c, 0), s);
                ExpRet::Void
            }),
        b("BoneGetIdFromBoneTag", "BoneNode_c::GetIdFromBoneTag", 0x617050, Cdecl, Eax)
            .args(vec![A::I(g_tag)])
            .reads(vec![BONE_INFOS])
            .rust(|c| ExpRet::Eax(bones::get_id_from_bone_tag(c.i(0) as i32) as u32)),
        b("BoneInitLimits", "BoneNode_c::InitLimits", 0x617490, Thiscall, Void)
            .regions(vec![bone_region()])
            .args(vec![A::P(0)])
            .fix(|v, l| fix_bone_tag(v, l, 0))
            .reads(vec![BONE_INFOS])
            .rust(|c| {
                bones::init_limits(tp(c, 0));
                ExpRet::Void
            }),
        b("BoneClampLimitsDefault", "BoneNode_c::ClampLimitsDefault", 0x617530, Thiscall, Void)
            .regions(vec![bone_region()])
            .args(vec![A::P(0), A::I(g_bool), A::I(g_bool), A::I(g_bool)])
            .fix(|v, l| fix_bone_tag(v, l, 0))
            .reads(vec![BONE_INFOS])
            .rust(|c| {
                let (a, b2, d) = (c.i(1) as u8, c.i(2) as u8, c.i(3) as u8);
                bones::clamp_limits_default(tp(c, 0), a, b2, d);
                ExpRet::Void
            }),
        b("VehAnimGroupCopyAnimGroup", "CVehicleAnimGroup::CopyAnimGroup", 0x5B9E40, Thiscall, Eax)
            .regions(vec![raw(37), raw(37)])
            .args(vec![A::P(0), A::P(1)])
            .rust(|c| ExpRet::Eax(vehgroup::copy_anim_group(tp(c, 0), tp(c, 1)))),
        b("VehAnimGroupInitAnimGroup", "CVehicleAnimGroup::InitAnimGroup", 0x5B9EB0, Thiscall, Eax)
            .regions(vec![raw(37), raw(5), raw(4), raw(4)])
            .args(vec![A::P(0), A::I(g_byte), A::I(g_byte), A::I(g_rand), A::I(g_rand), A::P(1), A::P(2), A::P(3)])
            .rust(|c| {
                let (f, s, af, sf) = (c.i(1) as u8, c.i(2) as u8, c.i(3), c.i(4));
                ExpRet::Eax(vehgroup::init_anim_group(tp(c, 0), f, s, af, sf, tp(c, 1), tp(c, 2), tp(c, 3)))
            }),
        b(
            "VehAnimGroupComputeCriticalBlendTime",
            "CVehicleAnimGroup::ComputeCriticalBlendTime",
            0x6E3C80,
            Thiscall,
            St0,
        )
        .regions(vec![veh_group_region()])
        .args(vec![A::P(0), A::I(|r| loop {
            let a = g_anim(r);
            if a != 377 {
                break a;
            }
        })])
        .rust(|c| {
            let a = c.i(1) as i32;
            st0(vehgroup::compute_critical_blend_time(tp(c, 0), a))
        }),
        b("VehAnimGroupGetGroup", "CVehicleAnimGroup::GetGroup", 0x6E3B00, Thiscall, Eax)
            .regions(vec![veh_group_region()])
            .args(vec![A::P(0), A::I(g_anim)])
            .callees(&[0x822130, 0x821B40, 0x829B99, 0x829B80])
            // the CRT pow takes its SSE2 path (stmxcsr/ldmxcsr), which Unicorn does not run faithfully
            .route(Route::L2 { reason: "CRT pow SSE2 path (stmxcsr): Unicorn unfaithful; decided by the CPU backend / in-game L2" })
            .patch(vec![(SSE2_AVAILABLE, 1u32.to_le_bytes().to_vec())])
            .reads(vec![(SSE2_AVAILABLE, SSE2_AVAILABLE + 4)])
            .rust(|c| {
                let a = c.i(1) as i32;
                ExpRet::Eax(vehgroup::get_group(tp(c, 0), a) as u32)
            }),
        b("IKChainSetBlend", "IKChain_c::SetBlend", 0x617C10, Thiscall, Void)
            .regions(vec![raw(22)])
            .args(vec![A::P(0), A::F])
            .rust(|c| {
                let v = c.i(1);
                ik::set_blend(tp(c, 0), v);
                ExpRet::Void
            }),
        b("IKChainSetOffsetBoneTag", "IKChain_c::SetOffsetBoneTag", 0x617C20, Thiscall, Void)
            .regions(vec![raw(22)])
            .args(vec![A::P(0), A::I(g_tag)])
            .rust(|c| {
                let v = c.i(1) as i32;
                ik::set_offset_bone_tag(tp(c, 0), v);
                ExpRet::Void
            }),
        b("IKChainSetOffsetPos", "IKChain_c::SetOffsetPos", 0x617C30, Thiscall, Void)
            .regions(vec![raw(22)])
            .args(vec![A::P(0), A::F, A::F, A::F])
            .rust(|c| {
                let (x, y, z) = (c.i(1), c.i(2), c.i(3));
                ik::set_offset_pos(tp(c, 0), x, y, z);
                ExpRet::Void
            }),
        b("IKChainUpdateOffset", "IKChain_c::UpdateOffset", 0x617E20, Thiscall, Void)
            .regions(vec![raw(22)])
            .args(vec![A::P(0), A::I(g_tag), A::F, A::F, A::F])
            .rust(|c| {
                let (t, x, y, z) = (c.i(1) as i32, c.i(2), c.i(3), c.i(4));
                ik::update_offset(tp(c, 0), t, x, y, z);
                ExpRet::Void
            }),
        b("IKChainUpdateTarget", "IKChain_c::UpdateTarget", 0x617E50, Thiscall, Void)
            .regions(vec![raw(22)])
            .args(vec![A::P(0), A::I(g_rand)])
            .rust(|c| {
                let v = c.i(1) as u8;
                ik::update_target(tp(c, 0), v);
                ExpRet::Void
            }),
        b("PedIKConstructor", "CPedIK::Constructor", 0x5FD8C0, Thiscall, Eax)
            .regions(vec![raw(8)])
            .args(vec![A::P(0), A::I(g_rand)])
            .rust(|c| {
                let p = c.i(1);
                pedik::constructor(tp(c, 0), GPtr::from_addr(p as usize));
                ExpRet::Eax(c.addr(0))
            }),
        b("PedIKMoveLimb", "CPedIK::MoveLimb-non-normalized", 0x5FDA60, Stdcall, Eax)
            .regions(vec![floats(2), floats(6)])
            .args(vec![A::P(0), A::F, A::F, A::P(1)])
            .rust(|c| {
                let (y, p) = (c.f(1), c.f(2));
                ExpRet::Eax(pedik::move_limb(tp(c, 0), y, p, tp(c, 1)) as u32)
            }),
        b("PedIKMoveLimbNormalized", "CPedIK::MoveLimb-normalized", 0x5FDB60, Stdcall, Eax)
            .regions(vec![floats(2), floats(6)])
            .args(vec![A::P(0), A::F, A::F, A::P(1), A::F])
            .rust(|c| {
                let (y, p, r) = (c.f(1), c.f(2), c.f(4));
                ExpRet::Eax(pedik::move_limb_normalized(tp(c, 0), y, p, tp(c, 1), r) as u32)
            }),
    ]
}

/// Specs of ports whose only differences from the CPU come from `sa_math::x87`'s 53-bit FPATAN model
/// (divergence ledger, pending: "FSIN/FCOS/FPATAN modelled at 53 bits"). Measured with the native
/// backend (i7-14700F, 10^6 generated + 60,000 edge cases): QuatToEuler 19 and Limit 63 cases differ, each
/// in the last bit of one stored angle/quaternion component. Not part of [`all`] (the CPU test would fail
/// until FPATAN is modelled at 64 bits); `sa-oracle` cannot run them by key, rerun them by chaining this
/// list into `all` temporarily.
pub fn ledger_specs() -> Vec<Spec> {
    vec![
        b(
            "BoneQuatToEuler",
            "BoneNode_c::QuatToEuler",
            0x617080,
            Cdecl,
            Void,
        )
        .regions(vec![quat(), raw(3)])
        .args(vec![A::P(0), A::P(1)])
        .route(FPATAN)
        .fix(|v, l| fix_gimbal(v, l, 0, 0))
        .rust(|c| {
            bones::quat_to_euler(tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
        b("BoneLimit", "BoneNode_c::Limit", 0x617650, Thiscall, Void)
            .regions(vec![bone_region()])
            .args(vec![A::P(0), A::F])
            .callees(&[0x617080, 0x6171F0])
            .route(FPATAN)
            .fix(|v, l| {
                fix_bone_tag(v, l, 0);
                // the head (tag 5) path a third of the time
                let mut g = Rng::new(seed(v) ^ 5);
                if g.chance(33) {
                    let s = l.word_slot[0][2].unwrap();
                    v[s] = 5;
                }
            })
            .reads(vec![BONE_INFOS])
            .rust(|c| {
                let bl = c.f(1);
                bones::limit(tp(c, 0), bl);
                ExpRet::Void
            }),
    ]
}
