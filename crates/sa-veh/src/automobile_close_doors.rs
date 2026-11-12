//! SCRIPT-5 (batch `script5-2`): `CAutomobile::CloseAllDoors`, reached by the script command CLOSE_ALL_CAR_DOORS
//! (0508) in the Grove Street missions. Ported from the 1.0 US bytes (gta-reversed as a reading aid; the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// Virtual `CVehicle::IsDoorMissing(eDoors)` (vtable +0x98, AL)
    fn vcall_is_door_missing(f: u32, v: GPtr<u8>, door: u32) -> u8 = thiscall dyn => unported;
    /// Virtual `CVehicle::OpenDoor(CPed*, int32 component, eDoors, float ratio, bool playSound)` (vtable +0x6C)
    fn vcall_open_door(f: u32, v: GPtr<u8>, ped: u32, component: u32, door: u32, ratio: u32, sound: u32) = thiscall dyn => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

/// The vtable slot at `+off` of `v`, read from the vehicle each time.
fn slot(v: GPtr<u8>, off: isize) -> u32 {
    v.cast::<GPtr<u8>>().read().byte_add(off).cast::<u32>().read()
}

/// `CAutomobile::CloseAllDoors()` (0x6A4520, thiscall).
///
/// The model info (`ms_modelInfoPtrs[(int16) +0x22]`) is read first. For each door in order, unless
/// `IsDoorMissing(door)`, `OpenDoor(NULL, component, door, 0.0f, true)`: front left (2, component 10); then, while
/// the model's door count (signed byte +0x4C) is above 1, front right (3, 8); above 2, rear left (4, 11) and rear right
/// (5, 9).
#[hook(addr = 0x6a4520, conv = thiscall, name = "CAutomobile::CloseAllDoors", expect = "56 8B F1 0F BF 46 22",
       this = "CAutomobile", unverified = "script5-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn automobile_close_all_doors(this: GPtr<u8>) {
    let id = this.byte_add(0x22).cast::<i16>().read();
    let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS).byte_add(4 * id as isize).read();
    let close = |door: u32, component: u32| {
        if vcall_is_door_missing(slot(this, 0x98), this, door) == 0 {
            vcall_open_door(slot(this, 0x6C), this, 0, component, door, 0, 1);
        }
    };
    close(2, 0xA);
    let doors = mi.byte_add(0x4C).cast::<i8>().read();
    if doors <= 1 {
        return;
    }
    close(3, 8);
    if mi.byte_add(0x4C).cast::<i8>().read() <= 2 {
        return;
    }
    close(4, 0xB);
    close(5, 9);
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_CLOSE_ALL_DOORS];
