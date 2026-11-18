//! The spinning pickup effects of `CPickups`: `DoMoneyEffects` (PEDHIT: the money a ped killed by a car drops spins),
//! `DoCollectableEffects` (snapshots, horseshoes, oysters, tags: a grey glow and a turn) and `DoMineEffects` (a red
//! glow and a turn). Ported from the 1.0 US bytes; gta-reversed `Pickups.cpp` (`SetRotateZOnly`) as a reading aid.

use sa_core::abi::HookReg;
use sa_core::field::ptr_of;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{fbits, lt};

use super::common::{cam_pos, entity_pos, f32_of, ftol32, g, rd, wr, x, xi};

use crate::bindings::{placeable_allocate_matrix, register_corona, store_static_shadow};
use sa_stream::bindings::simple_transform_update_matrix;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7_CB84;
/// `gpShadowExplosionTex`
const SHADOW_EXPLOSION_TEX: usize = 0xC4_03F4;

/// `CPlaceable::GetMatrix()` as the original inlines it: the matrix, allocated and filled from the placement when
/// missing.
fn get_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// The inlined `SetRotateZOnly` the three effects end with: `a = (time & mask) * k` (the `jge` that adds 2^32 to a
/// negative value is never taken), `c = f32(fcos a)`, `s = f32(fsin a)`; the matrix becomes a rotation about z:
/// right = (c, s, 0), forward = (-s, c, 0), up = (0, 0, 1), each field written through its own inlined
/// `GetMatrix()`. The position is kept.
fn spin_about_z(e: GPtr<u8>, mask: u32, k: f32) {
    let t = (g::<u32>(TIME_MS).read() & mask) as i32;
    let a = xi(t).fmul_m(k);
    let c = f32_of(a.fcos());
    let s = f32_of(a.fsin());
    wr::<f32>(get_matrix(e), 0x0, c);
    wr::<f32>(get_matrix(e), 0x4, s);
    wr::<u32>(get_matrix(e), 0x8, 0);
    wr::<f32>(get_matrix(e), 0x10, f32_of(x(s).fchs()));
    wr::<f32>(get_matrix(e), 0x14, c);
    wr::<u32>(get_matrix(e), 0x18, 0);
    wr::<u32>(get_matrix(e), 0x20, 0);
    wr::<u32>(get_matrix(e), 0x24, 0);
    wr::<f32>(get_matrix(e), 0x28, 1.0);
}

/// The glow of [`pickups_do_collectable_effects`] / [`pickups_do_mine_effects`]: the entity position is copied (bits),
/// `d = sqrt((dz² + dy²) + dx²)` of `camera - pos` (extended). Within `radius` (`fcom; test ah, 5; jp`), the shade
/// `t = u8(ftol(((fsin(((time + entity) & mask) * k) + 1) * 0.5 * (radius - d)) * inv_radius * max))` lights an
/// additive explosion-texture static shadow (2, 0, 0, -2; z distance 4, scale 1, draw distance 40) and a torus corona
/// (size 0.6, `range`, pull 1.5, fade 15), both coloured `(t, t, t)` when `grey`, else `(t, 0, 0)`.
#[allow(clippy::too_many_arguments)]
fn glow(e: GPtr<u8>, radius: u32, mask: u32, k: u32, inv_radius: u32, max: u32, range: f32, grey: bool) {
    let src = entity_pos(e);
    let mut pos = [rd::<u32>(src, 0), rd::<u32>(src, 4), rd::<u32>(src, 8)];
    let cam = cam_pos().cast::<u8>();
    let dx = x(rd::<f32>(cam, 0)).fsub_m(f32::from_bits(pos[0]));
    let dy = x(rd::<f32>(cam, 4)).fsub_m(f32::from_bits(pos[1]));
    let dz = x(rd::<f32>(cam, 8)).fsub_m(f32::from_bits(pos[2]));
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    if !lt(d, x(fbits(radius))) {
        return;
    }
    let i = (g::<u32>(TIME_MS).read().wrapping_add(e.addr() as u32) & mask) as i32;
    let a = xi(i).fmul_m(fbits(k)).fsin().fadd_m(1.0).fmul_m(0.5);
    let t = ftol32(a.fmul(x(fbits(radius)).fsub(d)).fmul_m(fbits(inv_radius)).fmul_m(fbits(max))) as u8 as u32;
    let (gb, b) = if grey { (t, t) } else { (0, 0) };
    let id = e.addr() as u32;
    let tex = g::<u32>(SHADOW_EXPLOSION_TEX).read();
    let pp = ptr_of(&mut pos);
    store_static_shadow(id, 2, tex, pp, 2.0f32.to_bits(), 0, 0, (-2.0f32).to_bits(), 0, t, gb, b, 4.0f32.to_bits(),
        1.0f32.to_bits(), 40.0f32.to_bits(), 0, 0);
    register_corona(id, GPtr::NULL, t, gb, b, 0xFF, pp, 0.6f32.to_bits(), range.to_bits(), 9, 0, 0, 0, 0, 0, 0,
        1.5f32.to_bits(), 0, 15.0f32.to_bits(), 0, 0);
}

/// `CPickups::DoMoneyEffects(CEntity*)` (0x454E80, cdecl): the spin of [`spin_about_z`] with `time & 0x7FF` and
/// `k = 0.0030566407f` (0x859B0C).
#[hook(addr = 0x454E80, conv = cdecl, name = "CPickups::DoMoneyEffects", expect = "83 EC 08 A1 84 CB B7 00",
       unverified = "pedhit-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "driving",
       note = "FSIN / FCOS of the spin angle (sa_math::x87 models)")]
pub fn pickups_do_money_effects(e: GPtr<u8>) {
    spin_about_z(e, 0x7FF, fbits(0x3B48_51EC));
}

/// `CPickups::DoCollectableEffects(CEntity*)` (0x455E20, cdecl): within 14 of the camera a grey [`glow`]
/// (`(time + entity) & 0x7FF`, k 0x859B68, 1/14 0x859B50, max 255, corona range 40), then the spin of
/// [`spin_about_z`] with `time & 0xFFF`, `k` 0x859B64.
#[hook(addr = 0x455E20, conv = cdecl, name = "CPickups::DoCollectableEffects", expect = "83 EC 10 56 8B 74 24 18",
       unverified = "frontier2-a19", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "collectable pickups (CEntity::PreRender of a collectable object); FSIN / FCOS / FSQRT")]
pub fn pickups_do_collectable_effects(e: GPtr<u8>) {
    glow(e, 0x4160_0000, 0x7FF, 0x3B48_F5C3, 0x3D92_4925, 0x437F_0000, 40.0, true);
    spin_about_z(e, 0xFFF, fbits(0x3AC8_51EC));
}

/// `CPickups::DoMineEffects(CEntity*)` (0x4560E0, cdecl): within 20 of the camera a red [`glow`]
/// (`(time + entity) & 0x1FF`, k 0x859B70, 1/20 0x858C28, max 64, corona range 60: the bytes push 60, not
/// gta-reversed's 40), then the spin of [`spin_about_z`] with `time & 0x3FF`, `k` 0x859B6C.
#[hook(addr = 0x4560E0, conv = cdecl, name = "CPickups::DoMineEffects", expect = "83 EC 10 56 8B 74 24 18",
       unverified = "frontier2-a19", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "land mine pickups (CEntity::PreRender of a mine object); FSIN / FCOS / FSQRT")]
pub fn pickups_do_mine_effects(e: GPtr<u8>) {
    glow(e, 0x41A0_0000, 0x1FF, 0x3C48_F5C3, 0x3D4C_CCCD, 0x4280_0000, 60.0, false);
    spin_about_z(e, 0x3FF, fbits(0x3BC8_51EC));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] =
    &[HOOK_PICKUPS_DO_MONEY_EFFECTS, HOOK_PICKUPS_DO_COLLECTABLE_EFFECTS, HOOK_PICKUPS_DO_MINE_EFFECTS];
