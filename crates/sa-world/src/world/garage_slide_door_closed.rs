//! WRITER-CLIMB (batch `climb-6`): `CGarage::SlideDoorClosed` (0x44A750), reached when CJ walks away from his
//! garage (the counterpart of WRITER-NATIVE's `SlideDoorOpen`). Ported from the 1.0 US bytes; x87 arithmetic follows
//! the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x};

orig! {
    /// `CAEDoorAudioEntity::AddAudioEvent(eAudioEvents, CVector& pos, float volume, float speed)` (0x4DC860)
    fn door_audio_add_event(a: GPtr<u8>, ev: u32, pos: GPtr<u8>, vol: f32, speed: f32) = thiscall 0x4DC860 => unported;
    /// `CGarage::FindDoorsWithGarage(CObject** door1, CObject** door2)` (0x449FF0)
    fn find_doors_with_garage(gar: GPtr<u8>, d1: GPtr<u8>, d2: GPtr<u8>) = thiscall 0x449FF0 => unported;
}

/// `bool CGarage::SlideDoorClosed()` (0x44A750).
///
/// The door position +0x38 falls by `timestep * [0x8599B4]` for the garage types 0x2C / 0x2D, else `[0x8599B0]`
/// (as `pos - step * k`). At 0 or below (ordered): set to 0, the door-close-finished sound (0xA2) at the first door
/// of `FindDoorsWithGarage`, true. Otherwise the door-moving sound (0xA1) there, false.
#[hook(addr = 0x44A750, conv = thiscall, name = "FUN_0044a750",
       expect = "D9 05 5C CB B7 00 83 EC 08",
       unverified = "climb-6", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn slide_door_closed(this: GPtr<u8>) -> u8 {
    let ts = x(g::<f32>(0xB7_CB5C).read());
    let ty = rd::<u8>(this, 0x4C);
    let k = if ty == 0x2C || ty == 0x2D { g::<f32>(0x85_99B4).read() } else { g::<f32>(0x85_99B0).read() };
    wr::<f32>(this, 0x38, x(rd::<f32>(this, 0x38)).fsub(ts.fmul_m(k)).to_f32());
    let mut doors = [0u32; 2];
    let d1: GPtr<u8> = GPtr::from_raw(doors.as_mut_ptr()).cast();
    let closed = !x(rd::<f32>(this, 0x38)).fcom(x(g::<f32>(0x85_8B50).read())).test_parity(0x41);
    if closed {
        wr::<u32>(this, 0x38, 0);
    }
    find_doors_with_garage(this, d1, d1.byte_add(4));
    let door = rd::<GPtr<u8>>(d1, 0);
    if !door.is_null() {
        let m = rd::<GPtr<u8>>(door, 0x14);
        let p = if m.is_null() { door.byte_add(4) } else { m.byte_add(0x30) };
        door_audio_add_event(this.byte_add(0x50), if closed { 0xA2 } else { 0xA1 }, p, 0.0, 1.0);
    }
    u8::from(closed)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SLIDE_DOOR_CLOSED];
