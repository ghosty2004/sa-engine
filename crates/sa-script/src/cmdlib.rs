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

/// Phase B exploration aid (never a port): a host handler sa-game installs for `SA_GAME_SCRIPT_SKIP`; it
/// may consume an unported opcode (log it, skip its parameters) and return the switch result.
pub static UNPORTED_OPCODE_HANDLER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Phase B exploration aid (never a port): a host `fn(Scr, u32)` sa-game installs for `SA_GAME_SCRIPT_TRACE`;
/// called with the script and the command word before each command is dispatched.
pub static COMMAND_TRACE_HANDLER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Runs the original switch body for an opcode without a Rust case (phase A), or panics naming the
/// switch and the opcode (phases B/C: no original body; unless the exploration handler consumes it).
#[inline]
pub fn fallback(orig_body: fn(Scr, u32) -> i8, name: &str, s: Scr, command: u32) -> i8 {
    if !sa_core::orig::PHASE_A {
        let h = UNPORTED_OPCODE_HANDLER.load(core::sync::atomic::Ordering::Relaxed);
        if h != 0 {
            // SAFETY: only ever set to a `fn(Scr, u32) -> Option<i8>` by sa-game.
            #[allow(unsafe_code)]
            let f: fn(Scr, u32) -> Option<i8> = unsafe { core::mem::transmute(h) };
            if let Some(r) = f(s, command) {
                return r;
            }
        }
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

/// The zone-info commands' common start (STD-7): `ReadTextLabelFromScript(buf, 8)` (the stack bytes after the
/// 8 are left over in the original, zero here), `CollectParameters(n)`, `z = CTheZones::FindZoneByLabel(buf, 2)`
/// (AX, signed); for `z >= 0` the zone's info record `0xBA1DF0 + 0x11 * zones[z].extraIndex` (word 0xBA37B4 +
/// 0x20 z), else `None`.
pub fn zone_info_by_label(s: Scr, n: u32) -> Option<GPtr<u8>> {
    let mut buf = [0u8; 16];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    collect(s, n);
    let z = find_zone_by_label(b, 2) as i16;
    if z < 0 {
        return None;
    }
    let extra = GPtr::<u16>::from_addr(0xBA37B4 + ((i32::from(z) << 5) as usize)).read();
    Some(GPtr::from_addr(0xBA1DF0 + usize::from(extra) * 0x11))
}

/// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(&CTheScripts::StreamedScripts, (u16)SP0)` (AX)
/// sign-extended into `SP0`, as the streamed-script brain commands start (STD-7).
pub fn streamed_script_index_to_sp0() -> i32 {
    let i = i32::from(get_proper_index_from_index_used_by_script(GPtr::from_addr(STREAMED_SCRIPTS), sp(0) & 0xFFFF) as i16);
    set_sp(0, i as u32);
    i
}

/// `CTheScripts::StreamedScripts` and `CTheScripts::ScriptsForBrains`
pub const STREAMED_SCRIPTS: usize = 0xA47B60;
pub const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;

/// 07D3 / 0884 (STD-7): `CollectParameters(1)`, `SP0 = proper index`, `ReadTextLabelFromScript(buf, 8)`,
/// `AddNewStreamedScriptBrainForCodeUse((u16)SP0, buf, ty)`.
pub fn register_script_brain_for_code_use(s: Scr, ty: u32) {
    collect(s, 1);
    streamed_script_index_to_sp0();
    let mut buf = [0u8; 16];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    add_new_streamed_script_brain_for_code_use(GPtr::from_addr(SCRIPTS_FOR_BRAINS), sp(0) & 0xFFFF, b, ty);
}

orig! {
    /// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(int16)` (0x470810), AX
    pub fn get_proper_index_from_index_used_by_script(m: GPtr<u8>, i: u32) -> u16 = thiscall 0x470810 => unported;
    /// `CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse(int16 index, const char* name, int8 type)` (0x46A9C0)
    pub fn add_new_streamed_script_brain_for_code_use(b: GPtr<u8>, index: u32, name: GPtr<u8>, ty: u32) = thiscall 0x46A9C0 => unported;
    /// `CScriptsForBrains::AddNewScriptBrain(int16 index, int16 model, uint16 priority, int8 type, int8 b,
    /// float radius)` (0x46A930)
    pub fn add_new_script_brain(b: GPtr<u8>, index: u32, model: u32, prio: u32, ty: u32, a: u32, radius: u32) = thiscall 0x46A930 => unported;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    pub fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
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
