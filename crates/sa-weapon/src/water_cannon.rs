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
    /// `CFireManager::ExtinguishPointWithWater(CVector, float, float)` (0x5394C0, thiscall)
    pub fn extinguish_point_with_water(man: GPtr<u8>, x: f32, y: f32, z: f32, r: f32, w: f32) -> u8 = thiscall 0x5394C0 => crate::fire::extinguish_point_with_water;
    /// `CWaterCannon::PushPeds()` (0x7295E0)
    pub fn push_peds_call(this: GPtr<u8>) = thiscall 0x7295E0 => unported;
    /// `CWaterCannon::Render()` (0x728DA0)
    pub fn render_call(this: GPtr<u8>) = thiscall 0x728DA0 => unported;
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

pub const HOOKS: &[HookReg] = &[
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
