//! `CCarGenerator` and `CTheCarGenerators` (PORT-POP), the leaf part. Ported from the 1.0 US bytes;
//! gta-reversed `CarGenerator.cpp`, `TheCarGenerators.cpp` as reading aids.
//!
//! `CCarGenerator` (0x20): +0 model (i16), +2/+3 colours, +4/+6/+8 position * 8 (i16), +0xA heading
//! (u8, `angle * 0.711111`), +0xB alarm, +0xC door lock, +0xD flags (bit 1 force spawn, bit 4 ignore
//! the population limit, bit 0 cleared), +0xE/+0x10 min/max delay, +0x14 next generation time, +0x18
//! vehicle handle, +0x1A generate count, +0x1C IPL id, +0x1D in use.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;

use crate::common::{ftol32, rd, wr, x};

pub const NUM_CAR_GENERATORS: usize = 500;
pub const CAR_GENERATOR_SIZE: usize = 0x20;

sa_core::globals! {
    list GLOBALS;
    /// `CTheCarGenerators::GenerateEvenIfPlayerIsCloseCounter` (+0) and `ProcessCounter` (+1)
    pub static COUNTERS @ 0xC279D0: [u8; 2] = [0; 2];
    /// `CTheCarGenerators::NumOfCarGenerators`
    pub static NUM_OF_CAR_GENERATORS @ 0xC279D4: u32 = 0;
    /// `CTheCarGenerators::m_SpecialPlateHandler`: 15 entries of 0x10 (+0 car generator index, +4 text)
    pub static SPECIAL_PLATES @ 0xC279D8: [u8; 0xF0] = [0; 0xF0];
    /// `m_SpecialPlateHandler.m_nCount`
    pub static SPECIAL_PLATE_COUNT @ 0xC27AC8: u32 = 0;
    /// `CTheCarGenerators::CarGeneratorArray`
    pub static CAR_GENERATORS @ 0xC27AD0: [u8; NUM_CAR_GENERATORS * CAR_GENERATOR_SIZE] = [0; NUM_CAR_GENERATORS * CAR_GENERATOR_SIZE];
}

fn generator(i: i32) -> GPtr<u8> {
    CAR_GENERATORS
        .ptr()
        .cast::<u8>()
        .byte_add((i as isize) * CAR_GENERATOR_SIZE as isize)
}

/// `CCarGenerator::SwitchOff()` (0x6F2E30): generate count 0.
#[hook(addr = 0x6F2E30, conv = thiscall, name = "CCarGenerator::SwitchOff", expect = "66 C7 41 1A 00 00", this = "CCarGenerator")]
pub fn switch_off(this: GPtr<u8>) {
    wr::<u16>(this, 0x1A, 0);
}

/// `CCarGenerator::SwitchOn()` (0x6F32C0): generate count 0xFFFF, next time = now + 4.
#[hook(addr = 0x6F32C0, conv = thiscall, name = "CCarGenerator::SwitchOn", expect = "66 C7 41 1A FF FF", this = "CCarGenerator")]
pub fn switch_on(this: GPtr<u8>) {
    wr::<u16>(this, 0x1A, 0xFFFF);
    wr(this, 0x14, TIME_MS.get().wrapping_add(4));
}

/// `CCarGenerator::Setup(...)` (0x6F2E50, thiscall, 14 stack words).
#[hook(addr = 0x6F2E50, conv = thiscall, name = "CCarGenerator::Setup", expect = "D9 44 24 04 56", this = "CCarGenerator")]
#[allow(clippy::too_many_arguments)]
pub fn setup(
    this: GPtr<u8>,
    px: f32,
    py: f32,
    pz: f32,
    angle: f32,
    model: i32,
    c1: i32,
    c2: i32,
    force: u32,
    alarm: u32,
    door: u32,
    min_delay: u32,
    max_delay: u32,
    ipl: u32,
    ignore_limit: u32,
) {
    let eight = f32::from_bits(0x4100_0000);
    let vx = ftol32(x(px).fmul_m(eight));
    let vy = ftol32(x(py).fmul_m(eight));
    wr(this, 0x4, vx as u16);
    let vz = ftol32(x(pz).fmul_m(eight));
    wr(this, 0x6, vy as u16);
    let va = ftol32(x(angle).fmul_m(f32::from_bits(0x3F36_0B60)));
    wr(this, 0x8, vz as u16);
    wr(this, 0xA, va as u8);
    wr(this, 0x0, model as u16);
    wr::<u8>(this, 0x2, if c1 as u16 == 0xFFFF { 0xFF } else { c1 as u8 });
    wr::<u8>(this, 0x3, if c2 as u16 == 0xFFFF { 0xFF } else { c2 as u8 });
    wr(this, 0xE, min_delay as u16);
    wr(this, 0xC, door as u8);
    wr(this, 0xB, alarm as u8);
    wr(this, 0x10, max_delay as u16);
    let old = rd::<u8>(this, 0xD);
    let f = ((((ignore_limit as u8) & 1) << 3 | (force as u8 & 1)) << 1) | (old & 0xE5);
    wr(this, 0xD, f);
    wr::<u16>(this, 0x18, 0xFFFF);
    wr(this, 0xD, f & 0xFE);
    wr(this, 0x14, TIME_MS.get().wrapping_add(1));
    wr::<u16>(this, 0x1A, 0);
    wr(this, 0x1C, ipl as u8);
    wr::<u8>(this, 0x1D, 1);
}

sa_core::orig! {
    /// `CCarGenerator::Setup` (0x6F2E50)
    #[allow(clippy::too_many_arguments)]
    pub fn setup_call(this: GPtr<u8>, px: f32, py: f32, pz: f32, angle: f32, model: i32, c1: i32, c2: i32, force: u32, alarm: u32, door: u32, min_delay: u32, max_delay: u32, ipl: u32, ignore_limit: u32) = thiscall 0x6F2E50 => setup;
}

/// `CTheCarGenerators::CreateCarGenerator(...)` (0x6F31A0, cdecl, 14 words): -1 for a model outside
/// 400..630 (other than -1) or when all 500 slots are used; else `Setup` of the first free slot.
#[hook(addr = 0x6F31A0, conv = cdecl, name = "CTheCarGenerators::CreateCarGenerator", expect = "8B 4C 24 14 83 F9 FF")]
#[allow(clippy::too_many_arguments)]
pub fn create_car_generator(
    px: f32,
    py: f32,
    pz: f32,
    angle: f32,
    model: i32,
    c1: i32,
    c2: i32,
    force: u32,
    alarm: u32,
    door: u32,
    min_delay: u32,
    max_delay: u32,
    ipl: u32,
    ignore_limit: u32,
) -> i32 {
    if model != -1 && !(0x190..=0x276).contains(&model) {
        return -1;
    }
    let mut i = 0;
    while i < NUM_CAR_GENERATORS as i32 {
        if rd::<u8>(generator(i), 0x1D) == 0 {
            setup_call(
                generator(i),
                px,
                py,
                pz,
                angle,
                model,
                c1,
                c2,
                force,
                alarm,
                door,
                min_delay,
                max_delay,
                ipl,
                ignore_limit,
            );
            NUM_OF_CAR_GENERATORS.set(NUM_OF_CAR_GENERATORS.get().wrapping_add(1));
            return i;
        }
        i += 1;
    }
    -1
}

/// `CTheCarGenerators::RemoveCarGenerators(uint8 iplId)` (0x6F3240).
#[hook(addr = 0x6F3240, conv = cdecl, name = "CTheCarGenerators::RemoveCarGenerators", expect = "8A 54 24 04 8B 0D D4 79 C2 00")]
pub fn remove_car_generators(ipl: u8) {
    let mut n = NUM_OF_CAR_GENERATORS.get();
    for i in 0..NUM_CAR_GENERATORS as i32 {
        let g = generator(i);
        if rd::<u8>(g, 0x1C) == ipl {
            wr::<u8>(g, 0x1D, 0);
            wr::<u8>(g, 0x1C, 0);
            n = n.wrapping_sub(1);
        }
    }
    NUM_OF_CAR_GENERATORS.set(n);
}

/// `CTheCarGenerators::Init()` (0x6F3270).
#[hook(addr = 0x6F3270, conv = cdecl, name = "CTheCarGenerators::Init", expect = "33 D2 89 15 D4 79 C2 00")]
pub fn init() {
    NUM_OF_CAR_GENERATORS.set(0);
    COUNTERS.set([0; 2]);
    SPECIAL_PLATE_COUNT.set(0);
    let sp = SPECIAL_PLATES.ptr().cast::<u8>();
    for k in 0..15 {
        wr::<i32>(sp, k * 0x10, -1);
        wr::<u8>(sp, k * 0x10 + 4, 0);
    }
    for i in 0..NUM_CAR_GENERATORS as i32 {
        wr::<u8>(generator(i), 0x1D, 0);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SWITCH_OFF,
    HOOK_SWITCH_ON,
    HOOK_SETUP,
    HOOK_CREATE_CAR_GENERATOR,
    HOOK_REMOVE_CAR_GENERATORS,
    HOOK_INIT,
];
