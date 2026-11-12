//! SAVE-1: `CAutomobile::Save` / `Load` and `CVehicleSaveStructure::Construct` / `Extract` (gta_sa.exe 1.0
//! US, from the bytes; docs/arch/save-port.md). `CVehicle::Save` / `Load` are in `vehicle2.rs`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::rd;

orig! {
    /// `CVehicle::Save()` (0x5D4760)
    fn vehicle_save(v: GPtr<u8>) -> u8 = thiscall 0x5D4760 => crate::vehicle2::save;
    /// `CVehicle::Load()` (0x5D2900)
    fn vehicle_load(v: GPtr<u8>) -> u8 = thiscall 0x5D2900 => crate::vehicle2::load;
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => sa_core::save::save_data_to_work_buffer;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => sa_core::save::load_data_from_work_buffer;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(m: GPtr<u8>, from: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    fn placeable_set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
}

/// `CAutomobile::Save()` (0x5D47E0, thiscall): `CVehicle::Save()`, then the damage manager (0x18 bytes at
/// +0x5A0). AL = 1.
#[hook(addr = 0x5D47E0, conv = thiscall, name = "CAutomobile::Save", expect = "56 8B F1 E8 78 FF FF FF", this = "CAutomobile",
       unverified = "save-veh-1", route = "split", hazards = "none", recs = "save-load")]
pub fn automobile_save(this: GPtr<u8>) -> u8 {
    vehicle_save(this);
    save_data(this.byte_add(0x5A0), 0x18);
    1
}

/// `CAutomobile::Load()` (0x5D2980, thiscall): `CVehicle::Load()`, then the damage manager read back. AL = 1.
#[hook(addr = 0x5D2980, conv = thiscall, name = "CAutomobile::Load", expect = "56 8B F1 E8 78 FF FF FF", this = "CAutomobile",
       unverified = "save-veh-1", route = "split", hazards = "none", recs = "save-load")]
pub fn automobile_load(this: GPtr<u8>) -> u8 {
    vehicle_load(this);
    load_data(this.byte_add(0x5A0), 0x18);
    1
}

/// The (structure offset, vehicle offset, size) fields after the matrix, in the bytes' order.
const FIELDS: [(isize, isize, u8); 11] = [
    (0x49, 0x434, 1),
    (0x4A, 0x435, 1),
    (0x4B, 0x436, 1),
    (0x4C, 0x437, 1),
    (0x4E, 0x45C, 2),
    (0x50, 0x488, 1),
    (0x54, 0x494, 4),
    (0x58, 0x49C, 4),
    (0x5C, 0x4A0, 4),
    (0x60, 0x428, 4),
    (0x64, 0x42C, 4),
];

fn copy(dst: GPtr<u8>, src: GPtr<u8>, n: u8) {
    match n {
        1 => dst.write(src.read()),
        2 => dst.cast::<u16>().write(src.cast::<u16>().read()),
        _ => dst.cast::<u32>().write(src.cast::<u32>().read()),
    }
}

/// `CVehicleSaveStructure::Construct(CVehicle*)` (0x5D2330, thiscall, `ret 4`): the matrix by `CMatrix::operator=`
/// from the vehicle's matrix (+0x14), the colours (+0x434..+0x437), +0x45C word, +0x488 byte, the dwords +0x494,
/// +0x49C, +0x4A0, +0x428, +0x42C.
#[hook(addr = 0x5D2330, conv = thiscall, name = "CVehicleSaveStructure::Construct", expect = "56 57 8B 7C 24 0C 8B 47 14",
       unverified = "save-veh-1", route = "split", hazards = "none", recs = "save-load")]
pub fn save_structure_construct(this: GPtr<u8>, v: GPtr<u8>) {
    matrix_assign(this, rd::<GPtr<u8>>(v, 0x14));
    for (s, o, n) in FIELDS {
        copy(this.byte_add(s), v.byte_add(o), n);
    }
}

/// `CVehicleSaveStructure::Extract(CVehicle*)` (0x5D23B0, thiscall, `ret 4`): `vehicle->SetMatrix(*this)`, then the
/// fields of `Construct` copied back in the same order.
#[hook(addr = 0x5D23B0, conv = thiscall, name = "CVehicleSaveStructure::Extract", expect = "56 57 8B 7C 24 0C 8B F1",
       unverified = "save-veh-1", route = "split", hazards = "none", recs = "save-load")]
pub fn save_structure_extract(this: GPtr<u8>, v: GPtr<u8>) {
    placeable_set_matrix(v, this);
    for (s, o, n) in FIELDS {
        copy(v.byte_add(o), this.byte_add(s), n);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_AUTOMOBILE_SAVE,
    HOOK_AUTOMOBILE_LOAD,
    HOOK_SAVE_STRUCTURE_CONSTRUCT,
    HOOK_SAVE_STRUCTURE_EXTRACT,
];
