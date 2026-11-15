//! `CWaterCannon` / `CWaterCannons` (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `WaterCannon(s).cpp`, `docs/re/specs/WEAPON/CWaterCannon*.md`).
//!
//! `CWaterCannon` (0x3CC): +0 id (vehicle), +4 current section (int16), +8 last update time, +0xC
//! section points (32 x CVector), +0x18C section velocities (32 x CVector), +0x30C section active
//! flags (32 bytes), +0x32C `CAEWaterCannonAudioEntity` (vtable at +0x32C, +0x33C ..., +0x3A8 byte).
//! `CWaterCannons::aCannons[3]` at 0xC80740; the render index list (18 x uint16) at 0xC80700.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const CANNON_SIZE: usize = 0x3CC;
pub const NUM_CANNONS: usize = 3;
const _: () = assert!(CANNON_SIZE == sa_core::layouts::size::CWaterCannon);

sa_core::globals! {
    list GLOBALS;
    /// `CWaterCannon::m_auRenderIndices[18]`
    pub static RENDER_INDICES @ 0xC80700: [u16; 18] = [0; 18];
    /// `CWaterCannons::aCannons[3]`
    pub static CANNONS @ 0xC80740: [u8; NUM_CANNONS * CANNON_SIZE] = [0; NUM_CANNONS * CANNON_SIZE];
}

pub use crate::bindings::extinguish_point_with_water;
use crate::bindings::event_hit_by_water_cannon_dtor;
use sa_math::fpu::{gt, lt};
use sa_math::vector::CVector;
use sa_world::bindings::{event_group_add, fx_add_particle, fx_prt_mult_ctor, physical_apply_move_force_5429f0};
use sa_core::input::rw_render_state_set;
use sa_world::bindings::{process_line_of_sight, rw_im3d_end, rw_im3d_render_indexed_primitive, rw_im3d_transform};
orig! {
    /// `CAEWaterCannonAudioEntity::Initialise(CWaterCannon*)` (0x503060, thiscall)
    pub fn audio_initialise(ae: GPtr<u8>, wc: GPtr<u8>) = thiscall 0x503060 => unported;
    /// `CAEWaterCannonAudioEntity::Service()` (0x5030D0, thiscall)
    pub fn audio_service(ae: GPtr<u8>) = thiscall 0x5030D0 => unported;
    /// `CAEWaterCannonAudioEntity::~CAEWaterCannonAudioEntity()` (0x728AB0, thiscall)
    pub fn audio_dtor(ae: GPtr<u8>) = thiscall 0x728AB0 => unported;
    /// `CWaterCannon::Init()` (0x728B40)
    pub fn init_call(this: GPtr<u8>) = thiscall 0x728B40 => init;
    /// `CWaterCannon::Update_NewInput(CVector*, CVector*)` (0x728C20)
    pub fn update_new_input_call(this: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>) = thiscall 0x728C20 => update_new_input;
    /// `CWaterCannon::PushPeds()` (0x7295E0)
    pub fn push_peds_call(this: GPtr<u8>) = thiscall 0x7295E0 => push_peds;
    /// `CEventHitByWaterCannon::CEventHitByWaterCannon(const CVector& point, const CVector& velocity)` (0x4B1290)
    fn event_hit_by_water_cannon_ctor(e: GPtr<u8>, point: GPtr<u8>, velocity: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1290 => unported;
    /// `CWaterCannon::Render()` (0x728DA0)
    pub fn render_call(this: GPtr<u8>) = thiscall 0x728DA0 => render;
    /// `CAEWaterCannonAudioEntity::ClearSplashInfo()` (0x502F20)
    fn wc_clear_splash_info(ae: GPtr<u8>) = thiscall 0x502F20 => unported;
    /// `CAEWaterCannonAudioEntity::SetSplashInfo(CVector pos, float velocity)` (0x502F50, `ret 0x10`)
    fn wc_set_splash_info(ae: GPtr<u8>, x: u32, y: u32, z: u32, v: f32) = thiscall 0x502F50 => unported;
    /// `CWaterCannon::Update_OncePerFrame(int16 index)` (0x72A280)
    pub fn update_once_per_frame_call(this: GPtr<u8>, i: i32) = thiscall 0x72A280 => unported;
}

pub fn cannon(i: usize) -> GPtr<u8> {
    CANNONS.ptr().cast::<u8>().byte_add((i * CANNON_SIZE) as isize)
}

/// `CWaterCannon::CWaterCannon()` (0x728B10, thiscall): the audio entity's +0x10 = 0, its vtable
/// 0x872A60, +0x3A8 = 0; returns `this`.
#[hook(addr = 0x728B10, conv = thiscall, name = "CWaterCannon::CWaterCannon", expect = "8B C1 33 C9 89 88 3C 03 00 00", this = "CWaterCannon")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wu(this, 0x33C, 0);
    wu(this, 0x32C, 0x872A60);
    wb(this, 0x3A8, 0);
    this
}

/// `CWaterCannon::Init()` (0x728B40, thiscall): id 0, section 0, time now, the 32 section flags 0,
/// the 18 render indices, `m_audio.Initialise(this)`.
#[hook(addr = 0x728B40, conv = thiscall, name = "CWaterCannon::Init", expect = "8B D1 C7 02 00 00 00 00", this = "CWaterCannon")]
pub fn init(this: GPtr<u8>) {
    wu(this, 0, 0);
    wh(this, 4, 0);
    wu(this, 8, time_ms());
    for k in 0..8 {
        wu(this, 0x30C + k * 4, 0);
    }
    RENDER_INDICES.set([0, 1, 2, 1, 3, 2, 4, 5, 6, 5, 7, 6, 8, 9, 10, 9, 11, 10]);
    audio_initialise(this.byte_add(0x32C), this);
}

/// The body of `Update_NewInput` (also inlined by `CWaterCannons::UpdateOne`).
fn new_input(this: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>) {
    let s = i32::from(rh(this, 4) as i16);
    wv_bits(this.byte_add(((s + 1) * 0xC) as isize), 0, rv_bits(pos, 0));
    let s = i32::from(rh(this, 4) as i16);
    wv_bits(this.byte_add(((s + 0x21) * 0xC) as isize), 0, rv_bits(vel, 0));
    let s = i32::from(rh(this, 4) as i16);
    wb(this, (s + 0x30C) as isize, 1);
}

/// `CWaterCannon::Update_NewInput(CVector* pos, CVector* vel)` (0x728C20, thiscall `ret 8`): the
/// current section's point, velocity and active flag.
#[hook(addr = 0x728C20, conv = thiscall, name = "CWaterCannon::Update_NewInput", expect = "0F BF 41 04 40", this = "CWaterCannon")]
pub fn update_new_input(this: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>) {
    new_input(this, pos, vel);
}

/// `CWaterCannons::Init()` (0x728C80): `Init` on the three cannons.
#[hook(addr = 0x728C80, conv = cdecl, name = "CWaterCannons::Init", expect = "56 57 BE 40 07 C8 00")]
pub fn cannons_init() {
    for i in 0..NUM_CANNONS {
        init_call(cannon(i));
    }
}

/// `CWaterCannons::UpdateOne(uint32 id, CVector* pos, CVector* vel)` (0x728CB0): the cannon with this
/// id gets the input (inlined `Update_NewInput`); else the first free one is `Init`ed, takes the id
/// and the input; else nothing.
#[hook(addr = 0x728CB0, conv = cdecl, name = "CWaterCannons::UpdateOne", expect = "56 57 8B 7C 24 0C")]
pub fn cannons_update_one(id: u32, pos: GPtr<u8>, vel: GPtr<u8>) {
    for i in 0..NUM_CANNONS {
        if ru(cannon(i), 0) == id {
            new_input(cannon(i), pos, vel);
            return;
        }
    }
    for i in 0..NUM_CANNONS {
        if ru(cannon(i), 0) == 0 {
            let c = cannon(i);
            init_call(c);
            wu(c, 0, id);
            update_new_input_call(c, pos, vel);
            return;
        }
    }
}

/// `CWaterCannons::Render()` (0x729B30): `Render` on every cannon with an id.
#[hook(addr = 0x729B30, conv = cdecl, name = "CWaterCannons::Render", expect = "56 57 BE 40 07 C8 00")]
pub fn cannons_render() {
    for i in 0..NUM_CANNONS {
        if ru(cannon(i), 0) != 0 {
            render_call(cannon(i));
        }
    }
}

/// `CWaterCannons::Update()` (0x72A3C0): every cannon's audio `Service`, then `Update_OncePerFrame(i)`
/// for those with an id.
#[hook(addr = 0x72A3C0, conv = cdecl, name = "CWaterCannons::Update", expect = "56 57 33 FF BE 40 07 C8 00")]
pub fn cannons_update() {
    for i in 0..NUM_CANNONS {
        let c = cannon(i);
        audio_service(c.byte_add(0x32C));
        if ru(c, 0) != 0 {
            update_once_per_frame_call(c, i as i32);
        }
    }
}

/// `CWaterCannon::Update_OncePerFrame(int16 index)` (0x72A280, thiscall `ret 4`): after 150 ms since
/// the last input the next section (mod 32) is cleared (the time is not refreshed here); every active
/// section falls (`vz -= ts*0.004`) and moves (`pos += v*ts`); then each active section puts out fires
/// (`gFireManager.ExtinguishPointWithWater(pos, 2.0, 0.5)`); every 4th frame (by `index + frame`)
/// `PushPeds`; with no active section the id becomes 0.
#[hook(addr = 0x72A280, conv = thiscall, name = "CWaterCannon::Update_OncePerFrame", expect = "83 EC 0C 53 55", this = "CWaterCannon")]
pub fn update_once_per_frame(this: GPtr<u8>, index: i32) {
    if time_ms() > ru(this, 8).wrapping_add(0x96) {
        let s = i32::from(rh(this, 4) as i16) + 1;
        let s = s % 32; // `and 0x8000001F` + the sign fix-up: the C remainder
        wh(this, 4, s as u16);
        wb(this, (i32::from(s as i16) + 0x30C) as isize, 0);
    }
    let ts = time_step();
    for k in 0..32isize {
        if rb(this, 0x30C + k) == 0 {
            continue;
        }
        let p = this.byte_add(0xC + k * 0xC);
        let v = this.byte_add(0x18C + k * 0xC);
        wf(v, 8, x(ts).fmul_m(0.004).fsubr_m(rf(v, 8)).to_f32());
        let dx = x(ts).fmul_m(rf(v, 0));
        let dy = x(ts).fmul_m(rf(v, 4));
        let dz = x(ts).fmul_m(rf(v, 8)).to_f32();
        wf(p, 0, dx.fadd_m(rf(p, 0)).to_f32());
        wf(p, 4, dy.fadd_m(rf(p, 4)).to_f32());
        wf(p, 8, x(dz).fadd_m(rf(p, 8)).to_f32());
    }
    for k in 0..32isize {
        if rb(this, 0x30C + k) == 0 {
            continue;
        }
        let p = rv(this.byte_add(0xC + k * 0xC), 0);
        extinguish_point_with_water(g(crate::fire::FIRE_MANAGER), p[0], p[1], p[2], 2.0, 0.5);
    }
    if (index as u32).wrapping_add(frame_counter()) & 3 == 0 {
        push_peds_call(this);
    }
    if (0..32isize).all(|k| rb(this, 0x30C + k) == 0) {
        wu(this, 0, 0);
    }
}

/// `CWaterCannon::~CWaterCannon()` (0x728B30, thiscall): `m_audio.~CAEWaterCannonAudioEntity()` (tail
/// jump with ECX = this + 0x32C).
#[hook(addr = 0x728B30, conv = thiscall, name = "CWaterCannon::~CWaterCannon", expect = "81 C1 2C 03 00 00", this = "CWaterCannon")]
pub fn destructor(this: GPtr<u8>) {
    audio_dtor(this.byte_add(0x32C));
}

/// `CWaterCannon::PushPeds()` (0x7295E0, thiscall; FRONTIER-2 part 4, the fire truck of the crossing run). The box of
/// the active sections (from ±10000; a bound takes the section's coordinate unless it is already beyond it, so NaN
/// coordinates are taken). Every ped of the pool (from the last slot down) that uses collision (+0x1C bit 0) and lies
/// strictly inside the box: the first active section within `(dz² + dy²) + dx²` < 5 of it gives it a
/// `CEventHitByWaterCannon(point, velocity)` (a stack event added to its event group, then destroyed), clears its
/// standing flags (+0x46C bits 0, 1), `ApplyMoveForce(0, 0, timestep)`, and for x and y `d = (0.1 * v - move) * 0.1`
/// added to the move speed when `d * v` > 0; then two `FxSystem_c::AddParticle` of the water-splash system (0xA9AE24)
/// at its position with `FxPrtMult_c(1, 1, 1, 0.6, 0.75, 0, 0.2)`, velocities `0.3 * move` and `-0.3 * move + (0, 0,
/// 0.5)`, (-1, 1.2, 0.6, 0). Only one section per ped (the loop index is set to 32).
#[hook(addr = 0x7295E0, conv = thiscall, name = "CWaterCannon::PushPeds", expect = "6A FF 68 58 8D 84 00", this = "CWaterCannon",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
pub fn push_peds(this: GPtr<u8>) {
    let (mut min, mut max) = ([10000.0f32; 3], [-10000.0f32; 3]);
    for k in 0..32isize {
        if rb(this, 0x30C + k) == 0 {
            continue;
        }
        let p = this.byte_add(0xC + k * 0xC);
        for a in 0..3 {
            let c = rf(p, 4 * a as isize);
            if !lt(x(min[a]), x(c)) {
                min[a] = c;
            }
            if !gt(x(max[a]), x(c)) {
                max[a] = c;
            }
        }
    }
    let pool = g::<GPtr<u8>>(0xB7_4490).read();
    let mut i = ru(pool, 8);
    while i != 0 {
        i -= 1;
        if (rb(rp(pool, 4), i as isize) as i8) < 0 {
            continue;
        }
        let ped = rp(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() || rb(ped, 0x1C) & 1 == 0 {
            continue;
        }
        let inside = (0..3).all(|a| {
            let c = rf(pos_ptr(ped), 4 * a as isize);
            gt(x(c), x(min[a])) && lt(x(c), x(max[a]))
        });
        if !inside {
            continue;
        }
        for k in 0..32isize {
            if rb(this, 0x30C + k) == 0 {
                continue;
            }
            let s = this.byte_add(0xC + k * 0xC);
            let pp = pos_ptr(ped);
            let dz = x(rf(s, 8)).fsub_m(rf(pp, 8));
            let dy = x(rf(s, 4)).fsub_m(rf(pp, 4));
            let dx = x(rf(s, 0)).fsub_m(rf(pp, 0));
            if !lt(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)), x(5.0)) {
                continue;
            }
            let v = this.byte_add(0x18C + k * 0xC);
            let mut ev = [0u32; 0x11];
            let evp = lp(&mut ev).cast::<u8>();
            event_hit_by_water_cannon_ctor(evp, s, v);
            event_group_add(rp(ped, 0x47C).byte_add(0x68), evp, 0);
            wu(ped, 0x46C, ru(ped, 0x46C) & 0xFFFF_FFFC);
            physical_apply_move_force_5429f0(ped, CVector::new(0.0, 0.0, time_step()));
            for a in 0..2isize {
                let d = x(0.1).fmul_m(rf(v, 4 * a)).fsub_m(rf(ped, 0x44 + 4 * a)).fmul_m(0.1);
                if gt(d.fmul_m(rf(v, 4 * a)), x(0.0)) {
                    wf(ped, 0x44 + 4 * a, d.fadd_m(rf(ped, 0x44 + 4 * a)).to_f32());
                }
            }
            let pp = pos_ptr(ped);
            let mut pos = [rf(pp, 0), rf(pp, 4), rf(pp, 8)];
            let (mx, my, mz) = (rf(ped, 0x44), rf(ped, 0x48), rf(ped, 0x4C));
            let mut v1 = [x(mx).fmul_m(0.3).to_f32(), x(my).fmul_m(0.3).to_f32(), x(mz).fmul_m(0.3).to_f32()];
            let mut v2 = [
                x(mx).fmul_m(-0.3).to_f32(),
                x(my).fmul_m(-0.3).to_f32(),
                x(mz).fmul_m(-0.3).fadd_m(0.5).to_f32(),
            ];
            let mut mult = [0u32; 7];
            let mp = lp(&mut mult).cast::<u8>();
            fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, 0.6, 0.75, 0.0, 0.2);
            let fx = g::<GPtr<u8>>(0xA9_AE24).read();
            fx_add_particle(fx, lp(&mut pos).cast(), lp(&mut v1).cast(), 0, mp, 0xBF80_0000, 1.2, 0.6, 0);
            fx_add_particle(fx, lp(&mut pos).cast(), lp(&mut v2).cast(), 0, mp, 0xBF80_0000, 1.2, 0.6, 0);
            event_hit_by_water_cannon_dtor(evp);
            break;
        }
    }
}


/// The 12 `RxObjSpace3DVertex` of the water cannon's strips (0xC80550, 0x24 bytes each: position +0, colour +0x18).
const WC_VERTICES: usize = 0xC8_0550;

fn wc_vertex(v: usize, k: usize, value: X87) {
    g::<f32>(WC_VERTICES + 0x24 * v + 4 * k).write(value.to_f32());
}

/// `rand() / 32767` (`fild; fmul [0x858C7C]`) as a float, the call's return address `site`.
fn rand_unit(site: u32) -> f32 {
    xi(sa_core::crt::rand_at(site)).fmul_m(sa_core::field::gf(0x85_8C7C)).to_f32()
}

/// A vector of three `rand_unit` (x, y, z), normalised and scaled by 0.05 (`[0x858C28]`).
fn random_axis(sites: [u32; 3]) -> [f32; 3] {
    let mut v = [rand_unit(sites[0]), rand_unit(sites[1]), rand_unit(sites[2])];
    sa_core::bindings::vector_normalise(lp(&mut v).cast());
    v.map(|c| x(c).fmul_m(0.05).to_f32())
}

/// The four vertices `a - d`, `a + d`, `b - d`, `b + d` from `base` (x and y of `d` extended, z a float).
fn wc_strip(base: usize, a: GPtr<u8>, b: GPtr<u8>, d0: X87, d1: X87, d2: f32) {
    wc_vertex(base, 0, x(rf(a, 0)).fsub(d0));
    wc_vertex(base, 1, x(rf(a, 4)).fsub(d1));
    wc_vertex(base, 2, x(rf(a, 8)).fsub_m(d2));
    wc_vertex(base + 1, 0, d0.fadd_m(rf(a, 0)));
    wc_vertex(base + 1, 1, d1.fadd_m(rf(a, 4)));
    wc_vertex(base + 1, 2, x(d2).fadd_m(rf(a, 8)));
    wc_vertex(base + 2, 0, x(rf(b, 0)).fsub(d0));
    wc_vertex(base + 2, 1, x(rf(b, 4)).fsub(d1));
    wc_vertex(base + 2, 2, x(rf(b, 8)).fsub_m(d2));
    wc_vertex(base + 3, 0, d0.fadd_m(rf(b, 0)));
    wc_vertex(base + 3, 1, d1.fadd_m(rf(b, 4)));
    wc_vertex(base + 3, 2, x(d2).fadd_m(rf(b, 8)));
}

/// `CWaterCannon::Render()` (0x728DA0, thiscall; FRONTIER-2 part 4, the fire truck of the crossing run). Render states
/// z-write off, z-test on, vertex alpha on, blend src alpha / inv src alpha, fog on, no raster, alpha test greater 0;
/// one `rand()` (0x728E2D, unused); the splash info cleared. From the current section `a` (`m_nSectionsCount % 32`,
/// the C remainder) and `b` = the one before (16-bit wrap), for i = 0..30 (31 strips; the walk moves `a = b`, `b - 1`)
/// until a splash: both sections active and `|b - a|²` < 25 (float differences): on the first such pair the axes —
/// `up = (b - a) x camera forward` (0xB6F9AC) scaled by `0.05 / |up|` (the squares summed x, z, y) and `right`, `fwd`
/// = normalised `(rand, rand, rand) / 32767` times 0.05 (rand sites 0x728FD7.., 0x72904F..); size `S = i² / 32 + 3`,
/// the 12 vertices' colour `(200, 200, 255, ftol(64 - i / 32 * 64))`, the strips `a ∓ up * S`, `b ∓ up * S` (and right,
/// fwd). `ProcessLineOfSight(b, a, buildings, vehicles)`: on a hit, `FxPrtMult_c(1, 1, 1, 0.15, 0.75, 1, 0.2)`, `v =
/// normal * 3` each scaled by `rand / 32767 * 1.6 + 0.2` (sites 0x729391, 0x7293B8, 0x7293DF), two rounds `n` of
/// `AddParticle` on 0xA9AE38 (v) and 0xA9AE24 (0.6 v) at the hit with `n / (unsigned) ftol(timestep * 0.02 * 1000)`,
/// then `SetSplashInfo(hit, |v|)` (z, y, x summed) and the walk stops after this strip. The strip is drawn
/// (`RwIm3DTransform(.., 12, null, 0x10)`, `RwIm3DRenderIndexedPrimitive(3, 0xC80700, 18)`, `RwIm3DEnd`). Afterwards
/// z-write on, vertex alpha off, fog off.
#[hook(addr = 0x728DA0, conv = thiscall, name = "CWaterCannon::Render", expect = "81 EC D0 00 00 00 A1 24 7B C9 00", this = "CWaterCannon",
       unverified = "frontier2-8", route = "L3-only", hazards = "heap-writes",
       note = "rand sites 0x728E2D, 0x728FD7, 0x728FEE, 0x729005, 0x72904F, 0x729066, 0x72907D, 0x729391, 0x7293B8, 0x7293DF; render only")]
pub fn render(this: GPtr<u8>) {
    for (s, v) in [(8, 0), (6, 1), (0xC, 1), (0xA, 5), (0xB, 6), (0xE, 1), (1, 0), (0x1D, 5), (0x1E, 0)] {
        rw_render_state_set(s, v);
    }
    sa_core::crt::rand_at(0x728E2D);
    let s0 = i32::from(rh(this, 4) as i16) % 32;
    let mut a = s0;
    let mut b = i32::from((a - 1) as i16);
    if (b as i16) < 0 {
        b += 32;
    }
    wc_clear_splash_info(this.byte_add(0x32C));
    let mut have_axes = false;
    let mut up = [0f32; 3];
    let mut right = [0f32; 3];
    let mut fwd = [0f32; 3];
    let mut i: i32 = 0;
    loop {
        let (ia, ib) = (i32::from(a as i16) as isize, i32::from(b as i16) as isize);
        if rb(this, 0x30C + ia) != 0 && rb(this, 0x30C + ib) != 0 {
            let pa = this.byte_add(0xC + 12 * ia);
            let pb = this.byte_add(0xC + 12 * ib);
            let mut d = [0f32; 3];
            for (k, c) in d.iter_mut().enumerate() {
                *c = x(rf(pb, 4 * k as isize)).fsub_m(rf(pa, 4 * k as isize)).to_f32();
            }
            let dsq = x(d[0]).fmul_m(d[0]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[2]).fmul_m(d[2]));
            if lt(dsq, x(25.0)) {
                if !have_axes {
                    let mut out = [0f32; 3];
                    let c = sa_core::bindings::cross_product(lp(&mut out).cast(), lp(&mut d).cast(), GPtr::from_addr(0xB6_F9AC));
                    up = [rf(c, 0), rf(c, 4), rf(c, 8)];
                    have_axes = true;
                    let len = x(up[0]).fmul_m(up[0]).fadd(x(up[2]).fmul_m(up[2])).fadd(x(up[1]).fmul_m(up[1])).fsqrt();
                    let k = len.fdivr_m(0.05);
                    up = up.map(|c| x(c).fmul(k).to_f32());
                    right = random_axis([0x72_8FD7, 0x72_8FEE, 0x72_9005]);
                    fwd = random_axis([0x72_904F, 0x72_9066, 0x72_907D]);
                }
                let ii = i32::from(i as i16);
                let size = xi(ii * ii).fmul_m(0.031_25).fadd_m(3.0);
                let alpha = xi(ii).fmul_m(0.031_25).fmul_m(64.0).fsubr_m(64.0).ftol() as u32;
                let colour = (alpha << 24) | 0xC8_C8FF;
                for v in 0..12 {
                    g::<u32>(WC_VERTICES + 0x18 + 0x24 * v).write(colour);
                }
                wc_strip(0, pa, pb, x(up[0]).fmul(size), x(up[1]).fmul(size), x(up[2]).fmul(size).to_f32());
                wc_strip(4, pa, pb, x(right[0]).fmul(size), size.fmul_m(right[1]), size.fmul_m(right[2]).to_f32());
                wc_strip(8, pa, pb, x(fwd[0]).fmul(size), size.fmul_m(fwd[1]), size.fmul_m(fwd[2]).to_f32());
                let mut cp = [0u32; 0x2C / 4];
                let mut hit = [0u32; 1];
                let splash = process_line_of_sight(pb, pa, lp(&mut cp).cast(), lp(&mut hit).cast(), 1, 1, 0, 0, 0, 0, 0, 0) != 0;
                if splash {
                    let mut mult = [0u32; 7];
                    let mp = lp(&mut mult).cast::<u8>();
                    fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, 0.15, 0.75, 1.0, 0.2);
                    let cpp = lp(&mut cp).cast::<u8>();
                    let mut vel = [0f32; 3];
                    for (k, c) in vel.iter_mut().enumerate() {
                        *c = x(rf(cpp, 0x10 + 4 * k as isize)).fmul_m(3.0).to_f32();
                    }
                    for (k, site) in [0x72_9391, 0x72_93B8, 0x72_93DF].into_iter().enumerate() {
                        let r = xi(sa_core::crt::rand_at(site)).fmul_m(sa_core::field::gf(0x85_8C7C)).fmul_m(1.6).fadd_m(0.2);
                        vel[k] = r.fmul_m(vel[k]).to_f32();
                    }
                    let mut vel2 = vel.map(|c| x(c).fmul_m(0.6).to_f32());
                    for n in 0..2 {
                        let t = x(time_step()).fmul_m(0.02).fmul_m(1000.0).ftol() as u32;
                        let tt = X87::from_f64_exact(f64::from(t));
                        let ratio = xi(n).fdiv(tt).to_f32().to_bits();
                        let fx = g::<GPtr<u8>>(0xA9_AE38).read();
                        fx_add_particle(fx, cpp, lp(&mut vel).cast(), ratio, mp, 0xBF80_0000, 1.2, 0.6, 0);
                        let fx = g::<GPtr<u8>>(0xA9_AE24).read();
                        fx_add_particle(fx, cpp, lp(&mut vel2).cast(), ratio, mp, 0xBF80_0000, 1.2, 0.6, 0);
                    }
                    let mag = x(vel[2]).fmul_m(vel[2]).fadd(x(vel[1]).fmul_m(vel[1])).fadd(x(vel[0]).fmul_m(vel[0])).fsqrt();
                    wc_set_splash_info(this.byte_add(0x32C), cp[0], cp[1], cp[2], mag.to_f32());
                }
                if !rw_im3d_transform(GPtr::from_addr(WC_VERTICES), 12, GPtr::NULL, 0x10).is_null() {
                    rw_im3d_render_indexed_primitive(3, RENDER_INDICES.ptr().cast(), 0x12);
                    rw_im3d_end();
                }
                if splash {
                    break;
                }
            }
        }
        i += 1;
        a = b;
        b -= 1;
        if (b as i16) < 0 {
            b += 32;
        }
        if (i as i16) >= 0x1F {
            break;
        }
    }
    rw_render_state_set(8, 1);
    rw_render_state_set(0xC, 0);
    rw_render_state_set(0xE, 0);
}


pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER,
    HOOK_PUSH_PEDS,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_INIT,
    HOOK_UPDATE_NEW_INPUT,
    HOOK_CANNONS_INIT,
    HOOK_CANNONS_UPDATE_ONE,
    HOOK_CANNONS_RENDER,
    HOOK_CANNONS_UPDATE,
    HOOK_UPDATE_ONCE_PER_FRAME,
];
