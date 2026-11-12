//! WRITER-NATIVE (batch `native-32`): `CGarage::SlideDoorOpen` (0x44A660) and `CGarage::FindDoorsWithGarage`
//! (0x449FF0), reached when CJ walks up to his garage door. Ported from the 1.0 US bytes; x87 arithmetic follows
//! the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x};

orig! {
    /// `CAEDoorAudioEntity::AddAudioEvent(eAudioEvents, CVector& pos, float volume, float speed)` (0x4DC860)
    fn door_audio_add_event(a: GPtr<u8>, ev: u32, pos: GPtr<u8>, vol: f32, speed: f32) = thiscall 0x4DC860 => unported;
    /// `CGarage::FindDoorsWithGarage(CObject** door1, CObject** door2)` (0x449FF0)
    fn find_doors_c(gar: GPtr<u8>, d1: GPtr<u8>, d2: GPtr<u8>) = thiscall 0x449FF0 => find_doors_with_garage;
}

/// `CGarages::aGarages` (0x96C048, 0xD8 bytes each)
const GARAGES: u32 = 0x96_C048;
/// `CPools::ms_pObjectPool` (0xB7449C)
const OBJECT_POOL: usize = 0xB7_449C;

/// `void CGarage::FindDoorsWithGarage(CObject** door1, CObject** door2)` (0x449FF0, `ret 8`).
///
/// Both outputs null, best distances 99999.9. The garage centre: `x = f32(f32(f32(dirA.x * width/2) + base.x) +
/// f32(dirB.x * height/2))`, `y = f32((dirB.y * height/2) + ((dirA.y * width/2) + base.y))` (y kept on the x87
/// stack). Every used pool object (high to low index) whose garage index (+0x147) is this garage's: its 2D distance
/// to the centre; the nearest becomes door 1 (the previous door 1 moves to door 2), otherwise it replaces a farther
/// or missing door 2. The original keeps the distance in the door-1 argument slot.
#[hook(addr = 0x449FF0, conv = thiscall, name = "FUN_00449ff0",
       expect = "83 EC 20 8B 44 24 28",
       unverified = "native-32", route = "L2", hazards = "x87-transcendental,heap-writes,stack-residue",
       recs = "grove-walk", note = "fsqrt; the door-1 argument slot holds the last distance")]
pub fn find_doors_with_garage(this: GPtr<u8>, d1: GPtr<u8>, d2: GPtr<u8>) {
    wr::<u32>(d2, 0, 0);
    wr::<u32>(d1, 0, 0);
    let f = |o: isize| rd::<f32>(this, o);
    let hw = x(f(0x20)).fmul_m(0.5);
    let t1 = x(f(0xC)).fmul(hw).to_f32();
    let cx = x(t1).fadd_m(f(0)).to_f32();
    let cy80 = x(f(0x10)).fmul(hw).fadd_m(f(4));
    let hh = x(f(0x24)).fmul_m(0.5);
    let t2 = x(f(0x14)).fmul(hh).to_f32();
    let cx = x(t2).fadd_m(cx).to_f32();
    let cy = x(f(0x18)).fmul(hh).fadd(cy80).to_f32();
    let idx = ((this.addr() as u32).wrapping_sub(GARAGES) as i32 / 0xD8) as u8;
    let mut best1 = x(f32::from_bits(0x47C3_4FF3));
    let mut best2 = f32::from_bits(0x47C3_4FF3);
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).cast::<u8>().read() & 0x80 != 0 {
            continue;
        }
        let obj = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x19C);
        if obj.is_null() || rd::<u8>(obj, 0x147) != idx {
            continue;
        }
        let m = rd::<GPtr<u8>>(obj, 0x14);
        let p = if m.is_null() { obj.byte_add(4) } else { m.byte_add(0x30) };
        let dx = x(cx).fsub_m(rd::<f32>(p, 0));
        let dy = x(cy).fsub_m(rd::<f32>(p, 4));
        let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let cur1 = rd::<GPtr<u8>>(d1, 0);
        if cur1.is_null() {
            best1 = x(dist);
            wr::<GPtr<u8>>(d1, 0, obj);
        } else if !x(dist).fcom(best1).test_parity(0x05) {
            best2 = best1.to_f32();
            wr::<GPtr<u8>>(d2, 0, cur1);
            best1 = x(dist);
            wr::<GPtr<u8>>(d1, 0, obj);
        } else if rd::<u32>(d2, 0) == 0 || !x(dist).fcom(x(best2)).test_parity(0x05) {
            wr::<GPtr<u8>>(d2, 0, obj);
            best2 = dist;
        }
    }
}

/// `bool CGarage::SlideDoorOpen()` (0x44A660).
///
/// The door position +0x38 advances by `timestep * [0x8599AC]` for the garage types 0x2C / 0x2D, else
/// `[0x8599A8]`. At 1 or more (ordered): clamped to 1, the door-open-finished sound (0xA0) at the first door of
/// `FindDoorsWithGarage`, true. Otherwise the door-moving sound (0x9F) there, false.
#[hook(addr = 0x44A660, conv = thiscall, name = "FUN_0044a660",
       expect = "D9 05 5C CB B7 00",
       unverified = "native-32", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn slide_door_open(this: GPtr<u8>) -> u8 {
    let ts = x(g::<f32>(0xB7_CB5C).read());
    let ty = rd::<u8>(this, 0x4C);
    let k = if ty == 0x2C || ty == 0x2D { g::<f32>(0x85_99AC).read() } else { g::<f32>(0x85_99A8).read() };
    wr::<f32>(this, 0x38, ts.fmul_m(k).fadd_m(rd::<f32>(this, 0x38)).to_f32());
    let mut doors = [0u32; 2];
    let d1: GPtr<u8> = GPtr::from_raw(doors.as_mut_ptr()).cast();
    let d2 = d1.byte_add(4);
    let opened = !x(rd::<f32>(this, 0x38)).fcom(x(1.0)).test_any(0x01);
    if opened {
        wr::<u32>(this, 0x38, 0x3F80_0000);
    }
    find_doors_c(this, d1, d2);
    let door = rd::<GPtr<u8>>(d1, 0);
    if !door.is_null() {
        let m = rd::<GPtr<u8>>(door, 0x14);
        let p = if m.is_null() { door.byte_add(4) } else { m.byte_add(0x30) };
        door_audio_add_event(this.byte_add(0x50), if opened { 0xA0 } else { 0x9F }, p, 0.0, 1.0);
    }
    u8::from(opened)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIND_DOORS_WITH_GARAGE, HOOK_SLIDE_DOOR_OPEN];
