//! PORT-VEH-3 (batch `veh3-1`): the heli/plane go-to-coordinates orders, the no-big-plane zone
//! test and the fixed machine gun trigger. Ported from the 1.0 US bytes; gta-reversed as a reading
//! aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{b, g, rd, wr, x};

orig! {
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70), ST0
    fn get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetCarGunFired()` (0x53FF90): int16 in AX
    fn pad_get_car_gun_fired(pad: GPtr<u8>) -> i16 = thiscall 0x53FF90 => unported;
    /// `CVehicle::FireFixedMachineGuns()` (0x73DF00)
    fn fire_fixed_machine_guns(v: GPtr<u8>) = thiscall 0x73DF00 => unported;
}

/// The common part of the two orders: the target (+0x3EC/+0x3F0/+0x3F4), the altitude (+0x9B0),
/// the creation state (+0x36 bits 3..7) = 3.
fn set_target(this: GPtr<u8>, tx: u32, ty: u32, tz: u32, alt: u32) {
    wr::<u32>(this, 0x3EC, tx);
    wr::<u32>(this, 0x9B0, alt);
    wr::<u32>(this, 0x3F0, ty);
    wr::<u32>(this, 0x3F4, tz);
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x18);
}

/// `fcomp` + `fnstsw` + `test ah, 0x44` + `jp` taken: not equal (or unordered).
fn ne(a: f32, c: f32) -> bool {
    x(a).fcom(x(c)).test_parity(0x44)
}

/// `CAutomobile::TellHeliToGoToCoors(float, float, float, float, float)` (0x6A2390): mission 0x14,
/// cruise speed 100, the target and both altitudes (+0x9B0, +0x9A8); a heli without a forced heading
/// (+0x8A8 == 0) gets `atan(forward) + pi`, reduced by 2pi while above 2pi.
#[hook(addr = 0x6A2390, conv = thiscall, name = "CAutomobile::TellHeliToGoToCoors", this = "CAutomobile",
       expect = "8B 44 24 04 8B 54 24 0C",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn tell_heli_to_go_to_coors(this: GPtr<u8>, tx: u32, ty: u32, tz: u32, alt_min: u32, alt_max: u32) {
    let heading_set = ne(rd::<f32>(this, 0x8A8), 0.0);
    set_target(this, tx, ty, tz, alt_min);
    wr::<u8>(this, 0x3BA, 0x14);
    wr::<u8>(this, 0x3D0, 100);
    wr::<u32>(this, 0x9A8, alt_max);
    if heading_set {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let a = X87::from_f64_exact(get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)))
        .fadd_m(f32::from_bits(0x4049_0FDB));
    wr::<f32>(this, 0x8A8, a.to_f32());
    let two_pi = f32::from_bits(0x40C9_0FDB);
    if a.fcom(x(two_pi)).test_any(0x41) {
        return;
    }
    let mut v = x(rd::<f32>(this, 0x8A8));
    loop {
        v = v.fsub_m(two_pi);
        if v.fcom(x(two_pi)).test_any(0x41) {
            break;
        }
    }
    wr::<f32>(this, 0x8A8, v.to_f32());
}

/// `CAutomobile::TellPlaneToGoToCoors(float, float, float, float, float)` (0x6A2470): mission 0x16,
/// cruise speed 0, the target, altitude +0x9B0 = the 4th argument and +0x9A8 = the larger of z and
/// the 5th; a plane without a heading (+0x8A8 == 0) gets `atan(forward)`; engine flag +0x428 bit 4.
#[hook(addr = 0x6A2470, conv = thiscall, name = "CAutomobile::TellPlaneToGoToCoors", this = "CAutomobile",
       expect = "8B 44 24 04 D9 44 24 14",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn tell_plane_to_go_to_coors(this: GPtr<u8>, tx: u32, ty: u32, tz: u32, alt: u32, height: u32) {
    let low = x(f32::from_bits(height)).fcom(x(f32::from_bits(tz))).test_any(0x41);
    set_target(this, tx, ty, tz, alt);
    wr::<u8>(this, 0x3BA, 0x16);
    wr::<u8>(this, 0x3D0, 0);
    wr::<u32>(this, 0x9A8, if low { tz } else { height });
    if !ne(rd::<f32>(this, 0x8A8), 0.0) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let a = X87::from_f64_exact(get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)));
        wr::<f32>(this, 0x8A8, a.to_f32());
    }
    wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x10);
}

/// `|p.xy - c| < 800` (`fcomp` with `test ah, 5`: below and ordered).
fn within_800(p: GPtr<u8>, cx: f32, cy: f32) -> bool {
    let a = x(rd::<f32>(p, 0)).fsub_m(cx);
    let c = x(rd::<f32>(p, 4)).fsub_m(cy);
    let d = c.fmul(c).fadd(a.fmul(a)).fsqrt();
    !d.fcom(x(800.0)).test_parity(0x05)
}

/// `CPlane::AreWeInNoBigPlaneZone()` (0x6CCBB0): the camera (TheCamera's matrix position, or its
/// placeable position) is within 800 units (2D) of (1522, -1237) or (-1836, 659), the airports.
#[hook(addr = 0x6CCBB0, conv = cdecl, name = "CPlane::AreWeInNoBigPlaneZone",
       expect = "A1 3C F0 B6 00",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn are_we_in_no_big_plane_zone() -> u8 {
    let m = g::<GPtr<u8>>(0xB6_F03C).read();
    let p = if m.is_null() { GPtr::from_addr(0xB6_F02C) } else { m.byte_add(0x30) };
    b(within_800(p, 1522.0, -1237.0) || within_800(p, -1836.0, 659.0))
}

/// `CVehicle::DoFixedMachineGuns()` (0x73F400): only with the active camera's mode (+0x19C of
/// `TheCamera.Cams[ActiveCam]`) 3; the pad is 0, or 1 when the driver's player slot (+0x598) is 1;
/// firing (GetCarGunFired) with +0x42A bit 7 clear fires the guns, otherwise 1400 ms after the last
/// shot (+0x4D4) the ammo/heat byte +0x4AB is reset to 20.
#[hook(addr = 0x73F400, conv = thiscall, name = "CVehicle::DoFixedMachineGuns", this = "CVehicle",
       expect = "0F B6 05 81 F0 B6 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "FireFixedMachineGuns fires bullets (projectiles, effects)")]
pub fn do_fixed_machine_guns(this: GPtr<u8>) {
    let cam = usize::from(g::<u8>(0xB6_F081).read());
    if g::<u32>(0xB6_F1C4 + cam * 0x238).read() != 3 {
        return;
    }
    let mut pad = get_pad(0);
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() && rd::<u32>(driver, 0x598) == 1 {
        pad = get_pad(1);
    }
    if pad_get_car_gun_fired(pad) != 0 && (rd::<u8>(this, 0x42A) as i8) >= 0 {
        fire_fixed_machine_guns(this);
        return;
    }
    if g::<u32>(0xB7_CB84).read() > rd::<u32>(this, 0x4D4).wrapping_add(0x578) {
        wr::<u8>(this, 0x4AB, 0x14);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_TELL_HELI_TO_GO_TO_COORS,
    HOOK_TELL_PLANE_TO_GO_TO_COORS,
    HOOK_ARE_WE_IN_NO_BIG_PLANE_ZONE,
    HOOK_DO_FIXED_MACHINE_GUNS,
];
