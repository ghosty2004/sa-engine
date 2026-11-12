//! `CVehicle::ProcessOpenDoor` (0x6D56C0, batch `std14-veh-2`): the door angle following the enter / exit
//! animations. Ported from the 1.0 US bytes (gta-reversed as a reading aid; its door enum default and the per-group
//! tables differ, the bytes are followed). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, x};

orig! {
    /// `CVehicle::IsDoorMissing(eDoors)` (vtable +0x98)
    fn v_is_door_missing(v: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x98 => unported;
    /// `CVehicle::GetDooorAngleOpenRatio(eDoors)` (vtable +0x78, ST0)
    fn v_door_angle_open_ratio(v: GPtr<u8>, door: u32) -> f64 = thiscall vslot 0x78 => unported;
    /// `CVehicle::OpenDoor(CPed*, int component, eDoors, float ratio, bool playSound)` (vtable +0x6C)
    fn v_open_door(v: GPtr<u8>, ped: GPtr<u8>, component: u32, door: u32, ratio: u32, sound: u32) = thiscall vslot 0x6C => unported;
    /// `CVehicleAnimGroupData::GetInOutTimings(AssocGroupId, eInOutTimingMode, float*, float*)` (0x645630)
    fn get_in_out_timings(group: u32, mode: u32, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x645630 => unported;
}

fn kf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}

/// The animation-group cases that use the fixed timings (`group - 0x58` in the byte tables 0x6D5C8C / 0x6D5CB0 /
/// 0x6D5CD4).
fn fixed_group(group: u32, t1: bool) -> bool {
    let k = group.wrapping_sub(0x58);
    if t1 {
        matches!(k, 0 | 1 | 2 | 15 | 16 | 19 | 20 | 22 | 25)
    } else {
        matches!(k, 0 | 1 | 2 | 14 | 15 | 19 | 20 | 22 | 25)
    }
}

/// `CVehicle::ProcessOpenDoor(CPed*, uint32 component, uint32 animGroup, uint32 animId, float time)` (0x6D56C0,
/// thiscall, `ret 0x14`). The door (components 8 / 9 / 10 / 11 → doors 3 / 5 / 2 / 4; any other component passes
/// the time's bits, as the original) is left alone when missing. By the animation (0x163 + k):
/// opening (k 0..=3, 18..=21, 23, 24, 29..=33) and closing (k 12..=15, 25..=28) cases: inside the window the ratio
/// is interpolated (for opening only when larger than the current ratio, for closing only when smaller), before /
/// after it the door is set shut or open. Locals the original reads without setting (the default animation groups
/// of k 29..=31) are the pushed `this` (the first local) and the caller's argument slots.
#[hook(addr = 0x6D56C0, conv = thiscall, name = "CVehicle::ProcessOpenDoor", expect = "51 53 8B 5C 24 10 56 8D 43 F8",
       unverified = "std14-veh-2", route = "L2", hazards = "stack-residue,uninit-locals", recs = "enter-car",
       note = "the interpolated ratio is stored over the time argument's slot; default groups of the last cases read the pushed `this` and argument slots as floats")]
pub fn process_open_door(this: GPtr<u8>, ped: GPtr<u8>, component: u32, group: u32, anim: u32, time: u32) {
    let door = match component {
        8 => 3,
        9 => 5,
        10 => 2,
        11 => 4,
        _ => time,
    };
    if v_is_door_missing(this, door) != 0 {
        return;
    }
    let t = x(f32::from_bits(time));
    let open = |ratio: f32, sound: u32| v_open_door(this, ped, component, door, ratio.to_bits(), sound);
    let cur = || X87::from_f64_exact(v_door_angle_open_ratio(this, door));
    let hgroup = u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x384), 0xDE));
    // the stack slots the original uses as locals: L (the pushed ecx = this), the group / component / anim
    // argument slots
    let mut l = this.addr() as u32;
    let mut s_group = group;
    let mut s_comp = component;
    let mut s_anim = anim;
    let lp = |v: &mut u32| GPtr::from_raw(v as *mut u32).cast::<u8>();
    let gt = |a: f32| !t.fcom(x(a)).test_any(0x41); // time > a
    let lt = |a: f32| !t.fcom(x(a)).test_parity(0x05); // time < a
    let k = anim.wrapping_sub(0x163);
    match k {
        0..=3 | 18..=21 => {
            let mode = if k <= 3 { 0 } else { 2 };
            get_in_out_timings(hgroup, mode, lp(&mut s_group), lp(&mut l));
            let (s, e) = (f32::from_bits(s_group), f32::from_bits(l));
            if gt(s) && lt(e) {
                let r = t.fsub_m(s).fdiv(x(e).fsub_m(s)).to_f32();
                if !cur().fcom(x(r)).test_parity(0x05) {
                    open(r, 1);
                }
            } else if gt(e) {
                open(1.0, 1);
            } else if lt(s) {
                open(0.0, if mode == 0 { 0 } else { 1 });
            }
        }
        12..=15 | 25..=28 => {
            let mode = if k <= 15 { 1 } else { 3 };
            get_in_out_timings(hgroup, mode, lp(&mut s_comp), lp(&mut s_anim));
            let (s, e) = (f32::from_bits(s_comp), f32::from_bits(s_anim));
            if gt(s) && lt(e) {
                let r = x(1.0).fsub(t.fsub_m(s).fdiv(x(e).fsub_m(s))).to_f32();
                if mode == 3 || !cur().fcom(x(r)).test_any(0x41) {
                    open(r, 1);
                }
            } else if gt(e) {
                open(0.0, 1);
            } else if lt(s) {
                open(1.0, 1);
            }
        }
        9..=11 => {
            if fixed_group(group, true) {
                open(1.0, 1);
            }
        }
        23 | 24 | 32 | 33 => {
            let (a, b) = (kf(0x858B1C), kf(0x858EE8));
            if gt(a) && lt(b) {
                open(t.fsub_m(a).fmul_m(kf(0x859F60)).to_f32(), 1);
            } else if gt(b) {
                open(1.0, 1);
            } else if lt(a) {
                open(0.0, 1);
            }
        }
        29 | 30 => {
            let e = if fixed_group(group, false) {
                s_group = 0x3C23_D70A;
                kf(0x858B1C)
            } else {
                f32::from_bits(l)
            };
            let s = f32::from_bits(s_group);
            if gt(s) && lt(e) {
                let r = t.fsub_m(s).fdiv(x(e).fsub_m(s)).to_f32();
                if !cur().fcom(x(r)).test_parity(0x05) {
                    open(r, 1);
                }
            } else if gt(e) {
                open(1.0, 1);
            } else if lt(s) {
                open(0.0, 1);
            }
        }
        31 => {
            if fixed_group(group, false) {
                s_group = 0x3D4C_CCCD;
                l = 0x3E4C_CCCD;
                s_comp = 0x3E99_999A;
                s_anim = 0x3EF3_3333;
            }
            let (s1, e1, s2, e2) = (
                f32::from_bits(s_group),
                f32::from_bits(l),
                f32::from_bits(s_comp),
                f32::from_bits(s_anim),
            );
            if lt(s2) && gt(s1) {
                let r = t.fsub_m(s1).fdiv(x(e1).fsub_m(s1)).to_f32();
                if !cur().fcom(x(r)).test_parity(0x05) {
                    open(r, 1);
                }
            } else if gt(s2) && lt(e2) {
                let r = x(1.0).fsub(t.fsub_m(s2).fdiv(x(e2).fsub_m(s2))).to_f32();
                if !cur().fcom(x(r)).test_any(0x41) {
                    open(r, 1);
                }
            } else if gt(e2) {
                open(0.0, 1);
            }
        }
        _ => {}
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_OPEN_DOOR];
