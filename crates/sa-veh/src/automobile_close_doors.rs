//! SCRIPT-5 (batches `script5-2`, `script5-9`): `CAutomobile::CloseAllDoors` (0508 CLOSE_ALL_CAR_DOORS),
//! `CVehicle::AddPassenger` (01C8 CREATE_CHAR_AS_PASSENGER, 0430). Ported from the
//! 1.0 US bytes (gta-reversed as a reading aid; the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

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

orig! {
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// The six-float `CPhysical::ApplyTurnForce(fx, fy, fz, px, py, pz)` wrapper (0x65DA70, `ret 0x18`; ported in mvpb_a1)
    fn apply_turn_force_6(e: GPtr<u8>, fx: f32, fy: f32, fz: f32, px: f32, py: f32, pz: f32) = thiscall 0x65DA70 => unported;
    /// `CEntity::RegisterReference(CEntity** slot)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `.rdata` -0.1f (0x858EF4), -0.02f (0x85904C), -0.2f (0x858FDC)
const K_M0_1: u32 = 0xBDCC_CCCD;
const K_M0_02: u32 = 0xBCA3_D70A;
const K_M0_2: u32 = 0xBE4C_CCCD;

fn f(p: GPtr<u8>, off: isize) -> f32 {
    p.byte_add(off).cast::<f32>().read()
}


/// `CVehicle::AddPassenger(CPed*)` (0x6D13A0, thiscall, `ret 4`; the automatic-seat overload).
///
/// First a nudge from the ped's mass (+0x8C): for vehicle type +0x590 == 9 `ApplyTurnForce(k * up, -0.1f *
/// forward)` with `k = mass * -0.02f` kept on the x87 stack (each product stored as f32); otherwise the six-float
/// wrapper 0x65DA70 with force `(0, 0, f32(mass * -0.2f))` at `(f32(ped.x - v.x), f32(ped.y - v.y), 0)` (matrix
/// positions, no NULL checks). Then the first empty passenger slot (+0x464, up to the byte +0x488, re-read each
/// step) takes the ped: `RegisterReference(&slot)`, byte +0x484 (passenger count) + 1, AL = 1; no free slot → AL = 0.
#[hook(addr = 0x6d13a0, conv = thiscall, name = "CVehicle::AddPassenger-Auto-Seat", expect = "83 EC 18 56 8B F1 83 BE",
       this = "CVehicle", unverified = "script5-9", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vehicle_add_passenger_auto(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let mass = f(ped, 0x8C);
    if this.byte_add(0x590).cast::<u32>().read() == 9 {
        let m = this.byte_add(0x14).cast::<GPtr<u8>>().read();
        let k01 = f32::from_bits(K_M0_1);
        let point = CVector {
            x: X87::from_f32(f(m, 0x10)).fmul_m(k01).to_f32(),
            y: X87::from_f32(f(m, 0x14)).fmul_m(k01).to_f32(),
            z: X87::from_f32(f(m, 0x18)).fmul_m(k01).to_f32(),
        };
        let k = X87::from_f32(mass).fmul_m(f32::from_bits(K_M0_02));
        let force = CVector {
            x: k.fmul_m(f(m, 0x20)).to_f32(),
            y: k.fmul_m(f(m, 0x24)).to_f32(),
            z: k.fmul_m(f(m, 0x28)).to_f32(),
        };
        apply_turn_force(this, force, point);
    } else {
        let pm = ped.byte_add(0x14).cast::<GPtr<u8>>().read();
        let vm = this.byte_add(0x14).cast::<GPtr<u8>>().read();
        let dy = X87::from_f32(f(pm, 0x34)).fsub_m(f(vm, 0x34)).to_f32();
        let dx = X87::from_f32(f(pm, 0x30)).fsub_m(f(vm, 0x30)).to_f32();
        let fz = X87::from_f32(mass).fmul_m(f32::from_bits(K_M0_2)).to_f32();
        apply_turn_force_6(this, 0.0, 0.0, fz, dx, dy, 0.0);
    }
    let mut i = 0u32;
    while i < u32::from(this.byte_add(0x488).read()) {
        let slot = this.byte_add(0x464 + 4 * i as isize);
        if slot.cast::<u32>().read() == 0 {
            slot.cast::<u32>().write(ped.addr() as u32);
            entity_register_reference(ped, slot);
            let n = this.byte_add(0x484);
            n.write(n.read().wrapping_add(1));
            return 1;
        }
        i += 1;
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_CLOSE_ALL_DOORS, HOOK_VEHICLE_ADD_PASSENGER_AUTO];
