//! PORT-VEH-4 (batch `veh4-2`): `CBoat::PruneWakeTrail`, `CVehicle::CanPedOpenLocks` and
//! `CVehicle::SetComponentRotation`. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the
//! bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, jne_mask, rd, wr, x};

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::SetRotateX(float)` (0x59B060)
    fn matrix_set_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B060 => unported;
    /// `CMatrix::SetRotateY(float)` (0x59B0A0)
    fn matrix_set_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B0A0 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::RotateX(float)` (0x59B1E0)
    fn matrix_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B1E0 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0)
    fn matrix_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390)
    fn matrix_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B390 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `CBoat::PruneWakeTrail()` (0x6F0E20): the wake points' remaining times (+0x748, 32 floats) count
/// down by the time step; the first non-positive one, or one that would run out (set to 0 then), ends
/// the trail and its index becomes the point count (+0x644, u16).
///
/// Kept as in the original: when all 32 points stay alive the count is not written.
/// A phase B native: the loop branches back to 0x6F0E23, inside the 5 bytes a hook would patch (and a
/// caller keeps ECX across the call).
#[sa_core::native(addr = 0x6F0E20, conv = thiscall, name = "CBoat::PruneWakeTrail")]
pub fn boat_prune_wake_trail(this: GPtr<u8>) {
    let ts = || g::<f32>(0xB7_CB5C).read();
    let mut i: u16 = 0;
    loop {
        let o = 0x748 + 4 * isize::from(i as i16);
        let v = rd::<f32>(this, o);
        if jne_mask(x(v), x(0.0), 0x41) {
            break;
        }
        if jne_mask(x(v), x(ts()), 0x41) {
            wr::<u32>(this, o, 0);
            break;
        }
        wr::<f32>(this, o, f32_of(x(v).fsub_m(ts())));
        i += 1;
        if i >= 0x20 {
            return;
        }
    }
    wr::<u16>(this, 0x644, i);
}

/// `CVehicle::CanPedOpenLocks(const CPed* ped)` (0x6D1E20): false for the door lock states 2, 4, 5
/// and 7 (+0x4F8), and for state 3 (locked for the player) when the ped is a player; else true.
#[hook(addr = 0x6D1E20, conv = thiscall, name = "CVehicle::CanPedOpenLocks", this = "CVehicle",
       expect = "56 8B F1 90 E9 24 FC D2 FF",
       unverified = "veh4-2", route = "L2", hazards = "none",
       note = "the body continues through `nop; jmp 0x401A4D` (a protection fragment that loads the lock state)")]
pub fn vehicle_can_ped_open_locks(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let lock = rd::<u32>(this, 0x4F8);
    if matches!(lock, 2 | 4 | 5 | 7) {
        return 0;
    }
    if ped_is_player(ped) != 0 && rd::<u32>(this, 0x4F8) == 3 {
        return 0;
    }
    1
}

/// `CVehicle::SetComponentRotation(RwFrame* frame, eRotationAxis axis, float angle, bool setRotate)`
/// (0x6DBA30): a `CMatrix` attached to the frame's matrix keeps its position aside; with `setRotate` the
/// rotation is replaced (`SetRotateX/Y/Z` for axis 0/1/2), else the position is zeroed (`pos + -pos`)
/// and the matrix rotated (`RotateX/Y/Z`); the position is added back and the RW matrix updated. A null
/// frame does nothing.
#[hook(addr = 0x6DBA30, conv = thiscall, name = "CVehicle::SetComponentRotation", this = "CVehicle",
       expect = "90 E9 D1 7E D2 FF",
       unverified = "veh4-2", route = "L2", hazards = "heap-writes",
       note = "entry is `nop; jmp 0x403907` (the SEH frame setup in a protection fragment, not reproduced); the CMatrix lives on the Rust stack; writes the frame's RwMatrix")]
pub fn vehicle_set_component_rotation(_this: GPtr<u8>, frame: GPtr<u8>, axis: u32, angle: f32, set: u8) {
    if frame.is_null() {
        return;
    }
    let mut mat = [0u32; 0x12];
    let m: GPtr<u8> = GPtr::from_raw(&mut mat as *mut [u32; 0x12]).cast();
    matrix_attach(m, frame.byte_add(0x10), 0);
    let saved = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
    if set != 0 {
        match axis {
            2 => matrix_set_rotate_z(m, angle),
            1 => matrix_set_rotate_y(m, angle),
            0 => matrix_set_rotate_x(m, angle),
            _ => {}
        }
    } else {
        let neg = [x(saved[0]).fchs(), x(saved[1]).fchs(), x(saved[2]).fchs()];
        for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
            wr::<f32>(m, o, f32_of(x(rd::<f32>(m, o)).fadd(neg[k])));
        }
        match axis {
            2 => matrix_rotate_z(m, angle),
            1 => matrix_rotate_y(m, angle),
            0 => matrix_rotate_x(m, angle),
            _ => {}
        }
    }
    for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
        wr::<f32>(m, o, f32_of(x(rd::<f32>(m, o)).fadd_m(saved[k])));
    }
    matrix_update_rw(m);
    matrix_dtor(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_CAN_PED_OPEN_LOCKS, HOOK_VEHICLE_SET_COMPONENT_ROTATION];

/// This module's phase B natives.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_BOAT_PRUNE_WAKE_TRAIL];
