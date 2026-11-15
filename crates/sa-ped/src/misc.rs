//! Small `CPlayerInfo` / `CPlayerPedData` / `CCopPed` / `CPlayerSkin` / `CPedPlacement` helpers and the
//! PED static initialiser of `PlayerClothes` (PORT-WEAPON-PED). Ported from the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// `CPlayerSkin::m_txdSlot`
    pub static SKIN_TXD_SLOT @ 0xC3F03C: i32 = 0;
}

/// `CTheScripts::FailCurrentMission` / `CGameLogic` restart globals written by
/// `PlayerFailedCriticalMission` (byte 0x96A8B0 = 3, time 0x96A8AC)
const MISSION_FAILED_STATE: usize = 0x96A8B0;
const MISSION_FAILED_TIME: usize = 0x96A8AC;
/// `CStreaming::ms_aInfoForModel[MODEL_GUN_PARA].m_nLoadState`
const PARACHUTE_LOAD_STATE: usize = 0x8E69CC;

pub use sa_core::bindings::crt_free_8214bd as crt_free;
pub use sa_pop::bindings::end_gang_war;
pub use sa_stream::bindings::txd_add_ref;
pub use sa_core::bindings::txd_create;
pub use sa_world::bindings::take_remote_controlled_car_from_player;
pub use sa_core::bindings::add_big_message;
pub use sa_core::bindings::add_txd_slot as txd_add_slot;
pub use sa_world::bindings::entity_clean_up_old_reference as clean_up_old_reference;
pub use sa_world::bindings::entity_register_reference as register_reference;
pub use crate::bindings::ped_give_weapon;
pub use sa_core::bindings::text_get;
pub use sa_stream::bindings::remove_txd_slot as txd_remove_slot;
pub use sa_world::bindings::world_test_sphere_against_world as test_sphere_against_world;
orig! {
    /// `CDarkel::ResetOnPlayerDeath` (0x43DC10)
    pub fn darkel_reset_on_player_death() = cdecl 0x43DC10 => unported;
    /// `CVehicle::BlowUpCar(CEntity*, bool)` (vtable slot 0xA4)
    pub fn vehicle_blow_up_car(v: GPtr<u8>, by: GPtr<u8>, b: u32) = thiscall vslot 0xA4 => unported;
    /// the police-scanner destructor of a `CWanted` (0x56F200: `this + 0x21C`, then
    /// `CAEPoliceScannerAudioEntity::~CAEPoliceScannerAudioEntity`)
    pub fn wanted_scanner_destructor(w: GPtr<u8>) = thiscall 0x56F200 => unported;
    /// `CPlayerPedData::~CPlayerPedData` (0x571840)
    pub fn ped_data_destructor_call(d: GPtr<u8>) = thiscall 0x571840 => ped_data_destructor;
}

/// `CPlayerInfo::KillPlayer` (0x56E580): once (state 0 → 1): `CDarkel::ResetOnPlayerDeath`, the "DEAD"
/// big message (4000 ms, style 2), stat 0x87 += 1, `CGangWars::EndGangWar(false)`.
#[hook(addr = 0x56E580, conv = thiscall, name = "CPlayerInfo::KillPlayer", expect = "8A 81 DC 00 00 00", this = "CPlayerInfo")]
pub fn kill_player(this: GPtr<u8>) {
    if rd::<u8>(this, 0xDC) != 0 {
        return;
    }
    wr::<u8>(this, 0xDC, 1);
    darkel_reset_on_player_death();
    add_big_message(GPtr::from_addr((GPtr::<u8>::addr(text_get(GPtr::from_addr(0xC1B340), GPtr::from_addr(0x865020))) as u32) as usize), 0xFA0, 2);
    increment_stat(0x87, 1.0);
    end_gang_war(0);
}

/// `CPlayerInfo::ArrestPlayer` (0x56E5D0): once (state 0 → 2): +0x154 = 0, `ResetOnPlayerDeath`, stat
/// 0x85 += 1, `EndGangWar(false)`.
#[hook(addr = 0x56E5D0, conv = thiscall, name = "CPlayerInfo::ArrestPlayer", expect = "8A 81 DC 00 00 00", this = "CPlayerInfo")]
pub fn arrest_player(this: GPtr<u8>) {
    if rd::<u8>(this, 0xDC) != 0 {
        return;
    }
    wr::<u8>(this, 0xDC, 2);
    wr::<u8>(this, 0x154, 0);
    darkel_reset_on_player_death();
    increment_stat(0x85, 1.0);
    end_gang_war(0);
}

/// `CPlayerInfo::PlayerFailedCriticalMission` (0x56E830): once (state 0 → 3): the mission-failed state and
/// time globals, tail call `ResetOnPlayerDeath`.
#[hook(addr = 0x56E830, conv = thiscall, name = "CPlayerInfo::PlayerFailedCriticalMission", expect = "8A 81 DC 00 00 00", this = "CPlayerInfo")]
pub fn player_failed_critical_mission(this: GPtr<u8>) {
    if rd::<u8>(this, 0xDC) != 0 {
        return;
    }
    wr::<u8>(this, 0xDC, 3);
    g::<u8>(MISSION_FAILED_STATE).write(3);
    g::<u32>(MISSION_FAILED_TIME).write(sa_core::timer::TIME_MS.get());
    darkel_reset_on_player_death();
}

/// `CPlayerInfo::BlowUpRCBuggy(bool)` (0x56EA30, `ret 4`): a remote vehicle not flagged +0x1C bit 11 is
/// taken from the player; with the flag it is blown up (`BlowUpCar(focus player's ped, false)`).
#[hook(addr = 0x56EA30, conv = thiscall, name = "CPlayerInfo::BlowUpRCBuggy", expect = "56 8B F1 8B 86 B0 00 00 00", this = "CPlayerInfo")]
pub fn blow_up_rc_buggy(this: GPtr<u8>, explode: u32) {
    let v = rd::<GPtr<u8>>(this, 0xB0);
    if v.is_null() || rd::<u32>(v, 0x1C) & 0x800 != 0 {
        return;
    }
    take_remote_controlled_car_from_player(explode);
    if explode as u8 == 0 {
        return;
    }
    let focus = usize::from(g::<u8>(PLAYER_IN_FOCUS).read());
    let ped = g::<GPtr<u8>>(PLAYERS + focus * PLAYER_INFO_SIZE).read();
    vehicle_blow_up_car(rd::<GPtr<u8>>(this, 0xB0), ped, 0);
}

/// `CPlayerInfo::GivePlayerParachute` (0x56EC40): with the parachute timer running and the model loaded
/// (load state 1): `GiveWeapon(0x2E, 1, true)` and the saved weapon set to the parachute.
#[hook(addr = 0x56EC40, conv = thiscall, name = "CPlayerInfo::GivePlayerParachute", expect = "56 8B F1 8B 86 8C 01 00 00", this = "CPlayerInfo")]
pub fn give_player_parachute(this: GPtr<u8>) {
    if rd::<u32>(this, 0x18C) == 0 || g::<u8>(PARACHUTE_LOAD_STATE).read() != 1 {
        return;
    }
    ped_give_weapon(rd::<GPtr<u8>>(this, 0), 0x2E, 1, 1);
    wr::<i32>(rd::<GPtr<u8>>(this, 0), 0x70C, 0x2E);
}

/// `CPlayerPedData::DeAllocateData` (0x56F890): destroys and frees the wanted (its police-scanner entity
/// destructor first) and frees the clothes; both pointers cleared.
#[hook(addr = 0x56F890, conv = thiscall, name = "CPlayerPedData::DeAllocateData", expect = "56 8B F1 57 8B 3E", this = "CPlayerPedData")]
pub fn ped_data_deallocate(this: GPtr<u8>) {
    let w = rd::<GPtr<u8>>(this, 0);
    if !w.is_null() {
        wanted_scanner_destructor(w);
        crt_free(w);
    }
    wr::<u32>(this, 0, 0);
    let c = rd::<GPtr<u8>>(this, 4);
    if !c.is_null() {
        crt_free(c);
    }
    wr::<u32>(this, 4, 0);
}

/// `CPlayerPedData::~CPlayerPedData` (0x571840): as `DeAllocateData`, each pointer cleared only when set.
#[hook(addr = 0x571840, conv = thiscall, name = "CPlayerPedData::Destructor", expect = "56 8B F1 57 8B 3E", this = "CPlayerPedData")]
pub fn ped_data_destructor(this: GPtr<u8>) {
    let w = rd::<GPtr<u8>>(this, 0);
    if !w.is_null() {
        wanted_scanner_destructor(w);
        crt_free(w);
        wr::<u32>(this, 0, 0);
    }
    let c = rd::<GPtr<u8>>(this, 4);
    if !c.is_null() {
        crt_free(c);
        wr::<u32>(this, 4, 0);
    }
}

/// `CPlayerInfo::~CPlayerInfo` (0x45B110): the player data destructor at +4 (tail jump).
#[hook(addr = 0x45B110, conv = thiscall, name = "CPlayerInfo::~CPlayerInfo", expect = "83 C1 04 E9 28 67 11 00", this = "CPlayerInfo")]
pub fn player_info_destructor(this: GPtr<u8>) {
    ped_data_destructor_call(this.byte_add(4));
}

fn criminal(this: GPtr<u8>, i: i32) -> GPtr<GPtr<u8>> {
    this.byte_add(0x7AC + (i as isize).wrapping_mul(4)).cast()
}

/// `CCopPed::RemoveCriminalToKill(CPed*, int32 i)` (0x5DE040, `ret 8`): clears criminal slot `i`.
#[hook(addr = 0x5DE040, conv = thiscall, name = "CCopPed::RemoveCriminalToKill", expect = "8B 44 24 08 56", this = "CCopPed")]
pub fn remove_criminal_to_kill(this: GPtr<u8>, _ped: GPtr<u8>, i: i32) {
    let r = criminal(this, i);
    let e = r.read();
    if !e.is_null() {
        clean_up_old_reference(e, r.cast::<u8>());
        r.write(GPtr::NULL);
    }
}

/// `CCopPed::ClearCriminalsToKill` (0x5DE070): each of the 5 criminals gets its "time of last cop chase"
/// (+0x54C) = now, +0x470 bit 6 cleared and +0x790 = 1.0; the slot is cleared.
#[hook(addr = 0x5DE070, conv = thiscall, name = "CCopPed::ClearCriminalsToKill", expect = "55 56 8D B1 AC 07 00 00", this = "CCopPed")]
pub fn clear_criminals_to_kill(this: GPtr<u8>) {
    for i in 0..5 {
        let r = criminal(this, i);
        if r.read().is_null() {
            continue;
        }
        wr::<u32>(r.read(), 0x54C, sa_core::timer::TIME_MS.get());
        let c = r.read();
        wr::<u32>(c, 0x470, rd::<u32>(c, 0x470) & 0xFFFF_FFBF);
        wr::<u32>(r.read(), 0x790, 0x3F80_0000);
        let e = r.read();
        if !e.is_null() {
            clean_up_old_reference(e, r.cast::<u8>());
        }
        r.write(GPtr::NULL);
    }
}

/// `CCopPed::SetPartner(CCopPed*)` (0x5DDE80, entry `push esi; nop; jmp 0x403CDD`, `ret 4`)
#[hook(addr = 0x5DDE80, conv = thiscall, name = "CCopPed::SetPartner", expect = "56 90 E9 56 5E E2 FF", this = "CCopPed")]
pub fn set_partner(this: GPtr<u8>, p: GPtr<u8>) {
    let r: GPtr<GPtr<u8>> = this.byte_add(0x7A8).cast();
    let e = r.read();
    if !e.is_null() {
        clean_up_old_reference(e, r.cast::<u8>());
    }
    r.write(p);
    if !p.is_null() {
        register_reference(p, r.cast::<u8>());
    }
}

/// `CPlayerSkin::Initialise` (0x6FF8A0): a "skin" TXD slot, created and referenced.
#[hook(addr = 0x6FF8A0, conv = cdecl, name = "CPlayerSkin::Initialise", expect = "68 BC 25 87 00")]
pub fn skin_initialise() {
    let s = txd_add_slot(GPtr::from_addr(0x8725BC));
    SKIN_TXD_SLOT.set(s);
    txd_create(s);
    txd_add_ref(SKIN_TXD_SLOT.get());
}

/// `CPlayerSkin::Shutdown` (0x6FF8D0)
#[hook(addr = 0x6FF8D0, conv = cdecl, name = "CPlayerSkin::Shutdown", expect = "A1 3C F0 C3 00")]
pub fn skin_shutdown() {
    txd_remove_slot(SKIN_TXD_SLOT.get());
}

/// `CPedPlacement::IsPositionClearOfCars(const CVector*)` (0x6168E0, cdecl): the entity
/// `CWorld::TestSphereAgainstWorld(pos, 0.25, null, false, true, false, false, false, false)` returns.
#[hook(addr = 0x6168E0, conv = cdecl, name = "CPedPlacement::IsPositionClearOfCars-Pos", expect = "8B 44 24 04 8B 10")]
pub fn is_position_clear_of_cars_pos(pos: GPtr<u32>) -> GPtr<u8> {
    test_sphere_against_world(sa_math::vector::CVector::new(f32::from_bits(pos.read()), f32::from_bits(pos.wrapping_index(1).read()), f32::from_bits(pos.wrapping_index(2).read())), f32::from_bits(0x3E80_0000), GPtr::<u8>::addr(GPtr::NULL) as u32, 0, 1, 0, 0, 0, 0)
}

/// `__init_PlayerClothes` (0x84FF30, static initialiser): zeroes `PlayerClothes`.
#[hook(addr = 0x84FF30, conv = cdecl, name = "__init_PlayerClothes", expect = "57 33 C0 B9 0A 00 00 00")]
pub fn init_player_clothes() {
    crate::clothes::PLAYER_CLOTHES.set([0; 30]);
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_KILL_PLAYER,
    HOOK_ARREST_PLAYER,
    HOOK_PLAYER_FAILED_CRITICAL_MISSION,
    HOOK_BLOW_UP_RC_BUGGY,
    HOOK_GIVE_PLAYER_PARACHUTE,
    HOOK_PED_DATA_DEALLOCATE,
    HOOK_PED_DATA_DESTRUCTOR,
    HOOK_PLAYER_INFO_DESTRUCTOR,
    HOOK_REMOVE_CRIMINAL_TO_KILL,
    HOOK_CLEAR_CRIMINALS_TO_KILL,
    HOOK_SET_PARTNER,
    HOOK_SKIN_INITIALISE,
    HOOK_SKIN_SHUTDOWN,
    HOOK_IS_POSITION_CLEAR_OF_CARS_POS,
    HOOK_INIT_PLAYER_CLOTHES,
];
