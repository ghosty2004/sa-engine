//! `CPlaceName` and `CZone::GetTranslatedName` (PORT-HUD-2): the zone name shown on the HUD and on the
//! map. Ported from the 1.0 US bytes; gta-reversed `PlaceName.cpp` / `Zone.cpp` as reading aid.
//!
//! `CPlaceName` (8 bytes): +0 `CZone*`, +4 `uint16` additional timer.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `TheText`
const THE_TEXT: usize = 0xC1B340;
/// `CWorld::PlayerInFocus` and `CWorld::Players[i].m_pPed` (stride 0x190)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS_PED: usize = 0xB7CD98;
/// `CGame::currArea`
const CURR_AREA: usize = 0xB72914;

orig! {
    /// `CText::Get(const char*)` (0x6A0050)
    pub fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CEntryExitManager::GetPositionRelativeToOutsideWorld(CVector&)` (0x43F150)
    pub fn position_relative_to_outside_world(pos: GPtr<u8>) = cdecl 0x43F150 => unported;
    /// `CTheZones::FindSmallestZoneForPosition(const CVector&, bool)` (0x572360)
    pub fn find_smallest_zone_for_position(pos: GPtr<u8>, info: u32) -> GPtr<u8> = cdecl 0x572360 => unported;
    /// `CZone::GetTranslatedName()` (0x5720C0)
    pub fn get_translated_name_call(zone: GPtr<u8>) -> GPtr<u8> = thiscall 0x5720C0 => get_translated_name;
    /// `CHud::SetZoneName(const GxtChar*, bool)` (0x588BB0)
    pub fn set_zone_name(name: u32, force: u32) = cdecl 0x588BB0 => unported;
}

/// `CZone::GetTranslatedName()` (0x5720C0): `TheText.Get(m_szTextKey)` (the key at +8).
#[hook(addr = 0x5720C0, conv = thiscall, name = "CZone::GetTranslatedName", expect = "83 C1 08 51 B9 40 B3 C1 00")]
pub fn get_translated_name(this: GPtr<u8>) -> GPtr<u8> {
    text_get(g(THE_TEXT), this.byte_add(8))
}

/// The focused player's position as `CPlaceName` reads it: the vehicle's when the ped is in one (flag
/// bit 8 of +0x46C), otherwise the ped's, passed through `GetPositionRelativeToOutsideWorld`. Raw dwords.
fn player_position() -> [u32; 3] {
    let i = usize::from(g::<u8>(PLAYER_IN_FOCUS).read());
    let ped: GPtr<u8> = g::<GPtr<u8>>(PLAYERS_PED + i * 0x190).read();
    let in_vehicle = rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let e: GPtr<u8> = if in_vehicle { rd(ped, 0x58C) } else { ped };
    let m: GPtr<u8> = rd(e, 0x14);
    let p = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    let mut v = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    if !in_vehicle {
        position_relative_to_outside_world(GPtr::from_raw(v.as_mut_ptr()).cast());
    }
    v
}

/// `CPlaceName::GetForMap(float x, float y)` (0x571D90, `ret 8`): the translated name of the smallest
/// zone at (x, y, player z) (no null check: a point outside every zone passes a null zone on, as the
/// original).
#[hook(addr = 0x571D90, conv = stdcall, name = "CPlaceName::GetForMap", expect = "0F B6 05 74 CD B7 00")]
pub fn get_for_map(px: u32, py: u32) -> GPtr<u8> {
    let p = player_position();
    let mut v = [px, py, p[2]];
    let zone = find_smallest_zone_for_position(GPtr::from_raw(v.as_mut_ptr()).cast(), 0);
    get_translated_name_call(zone)
}

/// `CPlaceName::Process()` (0x571F20): finds the smallest zone at the focused player's position; a
/// different zone (outside interiors) or one with a different label (8 bytes at +8) restarts the 250
/// frame timer, otherwise the timer counts down. The zone's translated name goes to
/// `CHud::SetZoneName(name, false)`.
///
/// Without a zone the original passes an uninitialised stack dword as the name (`[esp]` after `pop esi`
/// at 0x572016: the lowest dword of its 0x10-byte local area, `entry ESP - 0x10`, which it never writes;
/// only stored by `SetZoneName` outside interiors when no zone name is shown). The port passes that
/// dword as it was at entry (`peek`, PORT-HUD-3); entered without its hook (a direct Rust call) it
/// passes 0.
#[hook(addr = 0x571F20, conv = thiscall, name = "CPlaceName::Process", expect = "0F B6 05 74 CD B7 00", peek = 0x10)]
pub fn process(this: GPtr<u8>) {
    let mut v = player_position();
    let zone = find_smallest_zone_for_position(GPtr::from_raw(v.as_mut_ptr()).cast(), 0);
    if zone.is_null() {
        wr::<u32>(this, 0, 0);
        wr::<u16>(this, 4, 0);
    }
    let cur: GPtr<u8> = rd(this, 0);
    let mut restart = true;
    if (zone.addr() == cur.addr() || g::<u32>(CURR_AREA).read() != 0) && !cur.is_null() {
        restart = false;
    }
    if restart
        && !zone.is_null()
        && !cur.is_null()
        && rd::<u32>(zone, 8) == rd::<u32>(cur, 8)
        && rd::<u32>(zone, 0xC) == rd::<u32>(cur, 0xC)
    {
        restart = false;
    }
    if restart {
        wr::<u32>(this, 0, zone.addr() as u32);
        wr::<u16>(this, 4, 250);
    } else {
        let t = rd::<u16>(this, 4);
        if t > 0 {
            wr::<u16>(this, 4, t - 1);
        }
    }
    let z: GPtr<u8> = rd(this, 0);
    if z.is_null() {
        set_zone_name(sa_core::abi::peek_stack(0x571F20, -0x10).unwrap_or(0), 0);
    } else {
        let name = get_translated_name_call(z);
        set_zone_name(name.addr() as u32, 0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GET_TRANSLATED_NAME, HOOK_GET_FOR_MAP, HOOK_PROCESS];
