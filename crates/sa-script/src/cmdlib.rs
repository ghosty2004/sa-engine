//! Shared pieces of the partial switch ports (PORT-SCRIPT-2, ADR-002): `ScriptParams` access, the
//! fallback to the original body for unported opcodes, and `orig!` declarations used by several
//! switches.
//!
//! A partial switch is a `#[hook(..., fallback)]` whose `match` has one arm per ported opcode and a
//! last arm `_ => fallback(<fn>_original, NAME, s, command)`. In phase A the fallback runs the
//! original switch body through sa-hook's trampoline (the original dispatches the opcode itself, so
//! unported cases and the default -1 behave exactly as before); in phases B/C it panics naming the
//! switch and the opcode. Every switch module lists its Rust cases in `PORTED` (checked by the
//! `hooks` tests); a switch is fully ported only when `PORTED` covers every case of its jump table.

use sa_core::mem::GPtr;
use sa_core::orig;

use crate::vm::{self, Scr};

pub use crate::vm::{
    collect_parameters_call as collect, get_pointer_to_script_variable_call as var_ptr, param,
    store_parameters_call as store, update_compare_flag_call as flag,
};

/// Runs the original switch body for an opcode without a Rust case (phase A), or panics naming the
/// switch and the opcode (phases B/C: no original body).
#[inline]
pub fn fallback(orig_body: fn(Scr, u32) -> i8, name: &str, s: Scr, command: u32) -> i8 {
    if !sa_core::orig::PHASE_A {
        panic!("{name}: opcode {command:04X} is not ported (phases B/C have no original body)");
    }
    orig_body(s, command)
}

/// `ScriptParams[i]` as a raw dword.
#[inline]
pub fn sp(i: i32) -> u32 {
    param(i).read()
}

/// `ScriptParams[i]` as an int.
#[inline]
pub fn spi(i: i32) -> i32 {
    param(i).read() as i32
}

/// `ScriptParams[i]` as a float.
#[inline]
pub fn spf(i: i32) -> f32 {
    f32::from_bits(param(i).read())
}

/// `ScriptParams[i] = v`.
#[inline]
pub fn set_sp(i: i32, v: u32) {
    param(i).write(v);
}

/// A float constant by its bits (`.rdata` values are written as bit patterns).
#[inline]
pub fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `UpdateCompareFlag(b)` with a 0/1 byte (the `setcc` forms; the upper bytes of the pushed dword
/// are stale stack bytes the callee ignores).
#[inline]
pub fn set_flag(s: Scr, b: bool) {
    flag(s, u32::from(b));
}

/// `fld dword; fst dword`: the value an x87 load/store round trip leaves (a signalling NaN comes back
/// quiet; every other value unchanged).
#[inline]
pub fn fld_fst(v: f32) -> f32 {
    if v.is_nan() {
        f32::from_bits(v.to_bits() | 0x0040_0000)
    } else {
        v
    }
}

/// `.rdata` 0x859014: -100.0f (the "find the ground" z threshold of the CREATE_*/blip commands).
pub const K_MINUS_100: u32 = 0xC2C8_0000;

/// The short-range coordinate blip commands (04CE ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD 0x48BC0A: type
/// 4, colour 5; 0570 ADD_SHORT_RANGE_SPRITE_BLIP_FOR_CONTACT_POINT 0x48DB3D: type 5, colour 2):
/// `CollectParameters(4)` (x, y, z, sprite); z = ground z when `z <= -100` (ordered, `fld; fst slot;
/// fcomp`, then `fstp slot`); the discarded `GetActualBlipArrayIndex(CollectNextParameter...)`;
/// `h = SetShortRangeCoordBlip(type, {x, y, z}, colour, 3 /*BOTH*/, this->m_szName (+8))`;
/// `SetBlipSprite(h, SP3)`; `SP0 = h`; `StoreParameters(1)`.
pub fn short_range_coord_blip(s: Scr, ty: i32, colour: i32) {
    collect(s, 4);
    let mut z = fld_fst(spf(2));
    let y = spf(1);
    let x = spf(0);
    if z <= f(K_MINUS_100) {
        z = sa_math::x87::X87::from_f64_exact(find_ground_z_for_coord(x, y)).to_f32();
    }
    get_actual_blip_array_index(collect_next_parameter_without_increasing_pc(s));
    let h = set_short_range_coord_blip(ty, x, y, z, colour, 3, s.byte_add(8));
    set_blip_sprite(h, spi(3));
    set_sp(0, h as u32);
    store(s, 1);
}

/// `CRunningScript::m_UsesMissionCleanup` (byte +0xC6).
#[inline]
pub fn uses_mission_cleanup(s: Scr) -> bool {
    s.byte_add(vm::USES_MISSION_CLEANUP).read() != 0
}

/// `CPools::ms_pPedPool` 0xB74490 (owned by sa-core's pools; read by address as the original does).
pub const PED_POOL: usize = 0xB74490;
/// `CPools::ms_pVehiclePool` 0xB74494.
pub const VEHICLE_POOL: usize = 0xB74494;
/// `CPools::ms_pObjectPool` 0xB7449C.
pub const OBJECT_POOL: usize = 0xB7449C;
/// `CTheScripts::MissionCleanUp` 0xA90850.
pub const MISSION_CLEANUP: usize = 0xA90850;
/// `CWorld::Players[0]` (`CPlayerInfo`, 0x190 bytes each) 0xB7CD98; `m_pPed` is at +0.
pub const PLAYERS: usize = 0xB7CD98;
/// `TheCamera` 0xB6F028.
pub const THE_CAMERA: usize = 0xB6F028;

/// The pool pointer stored at `cell`.
#[inline]
pub fn pool(cell: usize) -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(cell).read()
}

/// `CWorld::Players[i]` (`CPlayerInfo*`), `i` as the original multiplies it (`imul i, 0x190`).
#[inline]
pub fn player_info(i: u32) -> GPtr<u8> {
    GPtr::from_addr(PLAYERS.wrapping_add(i.wrapping_mul(0x190) as usize))
}

orig! {
    /// `CPool<CPed>::GetAtRef(int32)` (0x404910), ECX = the ped pool
    pub fn ped_at_ref(pool: GPtr<u8>, h: i32) -> GPtr<u8> = thiscall 0x404910 => unported;
    /// `CPool<CVehicle>::GetAtRef(int32)` (0x4048E0), ECX = the vehicle pool
    pub fn vehicle_at_ref(pool: GPtr<u8>, h: i32) -> GPtr<u8> = thiscall 0x4048E0 => unported;
    /// `CPool<CObject>::GetAtRef(int32)` (0x465040), ECX = the object pool
    pub fn object_at_ref(pool: GPtr<u8>, h: i32) -> GPtr<u8> = thiscall 0x465040 => unported;
    /// `CPool<CObject>::GetRef(CObject*)` (0x465070), ECX = the object pool
    pub fn object_get_ref(pool: GPtr<u8>, o: GPtr<u8>) -> i32 = thiscall 0x465070 => unported;
    /// `CPool<CPed>::GetRef(CPed*)` (0x4442D0), ECX = the ped pool
    pub fn ped_get_ref(pool: GPtr<u8>, p: GPtr<u8>) -> i32 = thiscall 0x4442D0 => unported;
    /// `CMissionCleanup::AddEntityToList(int32 handle, uint8 type)` (0x4637E0)
    pub fn mission_cleanup_add(m: GPtr<u8>, h: i32, ty: i32) = thiscall 0x4637E0 => unported;
    /// `CMissionCleanup::RemoveEntityFromList(int32 handle, uint8 type)` (0x4654B0)
    pub fn mission_cleanup_remove(m: GPtr<u8>, h: i32, ty: i32) = thiscall 0x4654B0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::RemoveReferencesToDeletedObject(CEntity*)` (0x565510)
    pub fn world_remove_references_to_deleted_object(e: GPtr<u8>) = cdecl 0x565510 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    pub fn collect_next_parameter_without_increasing_pc(s: Scr) -> i32 = thiscall 0x464250 => unported;
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    pub fn read_text_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => unported;
    /// `CRadar::GetActualBlipArrayIndex(int32)` (0x582870)
    pub fn get_actual_blip_array_index(h: i32) -> i32 = cdecl 0x582870 => unported;
    /// `CRadar::SetShortRangeCoordBlip(eBlipType, CVector, eBlipColour, eBlipDisplay, char*)` (0x583920);
    /// the vector by value (three stack words)
    pub fn set_short_range_coord_blip(ty: i32, x: f32, y: f32, z: f32, colour: i32, display: i32, name: GPtr<u8>) -> i32 = cdecl 0x583920 => unported;
    /// `CRadar::SetBlipSprite(int32 blip, int32 sprite)` (0x583D70)
    pub fn set_blip_sprite(h: i32, sprite: i32) = cdecl 0x583D70 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), ST0 (read as f64: exact)
    pub fn find_ground_z_for_coord(x: f32, y: f32) -> f64 = cdecl 0x569660 => unported;
    /// `CRunningScript::LocateCharCommand(int32 command)` (0x486D80)
    pub fn locate_char_command(s: Scr, command: u32) = thiscall 0x486D80 => unported;
}
