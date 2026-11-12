//! WRITER-WORLD (batch `world5-2`): more `CCheat` handlers (money/armour/health, parachute,
//! adrenaline, blow up cars, Elvis lives, drive-by, the special-cheat reset). Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, xi};

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CCheat::HealthCheat()` (0x438D60)
    fn health_cheat() = cdecl 0x438D60 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn load_all_requested_models(only_priority: u32) = cdecl 0x40EA10 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool likeUnlimited)` (0x5E6080)
    fn ped_give_weapon(ped: GPtr<u8>, ty: i32, ammo: u32, unlimited: u32) -> i32 = thiscall 0x5E6080 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280)
    fn ped_set_current_weapon(ped: GPtr<u8>, ty: i32) = thiscall 0x5E6280 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32 ammo)` (0x5E89B0)
    fn ped_give_delayed_weapon(ped: GPtr<u8>, ty: i32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CPickups::GivePlayerGoodiesWithPickUpMI(int16 model, int32 player)` (0x4564F0)
    fn give_player_goodies(model: u32, player: i32) -> u8 = cdecl 0x4564F0 => unported;
    /// virtual slot +0xA4 (`CVehicle::BlowUpCar(CEntity*, bool)`)
    fn v_blow_up_car(v: GPtr<u8>, culprit: u32, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CStreaming::StreamPedsIntoRandomSlots(int32 models[8])` (0x40BDA0)
    fn stream_peds_into_random_slots(m: GPtr<u8>) = cdecl 0x40BDA0 => unported;
    /// `CCheat::BeachPartyCheat()` (0x439230)
    fn beach_party_cheat() = cdecl 0x439230 => unported;
    /// `CCheat::FunhouseCheat()` (0x439720)
    fn funhouse_cheat() = cdecl 0x439720 => unported;
    /// `CCheat::CountrysideInvasionCheat()` (0x439F60)
    fn countryside_invasion_cheat() = cdecl 0x439F60 => unported;
    /// `CCheat::EverybodyAttacksPlayerCheat()` (0x439C70)
    fn everybody_attacks_player_cheat() = cdecl 0x439C70 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    fn intelligence_get_task_jet_pack(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleJetPack::CTaskSimpleJetPack(const CVector*, float, int32, CEntity*)` (0x67B4E0)
    fn task_jetpack_ctor(t: GPtr<u8>, pos: u32, h: u32, a: u32, e: u32) -> GPtr<u8> = thiscall 0x67B4E0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// Flips a cheat flag byte (`sete`): the new value.
fn toggle(addr: usize) -> u8 {
    let v = u8::from(g::<u8>(addr).read() == 0);
    g::<u8>(addr).write(v);
    v
}

/// `CCheat::MoneyArmourHealthCheat()` (0x438E40): +250000 money for the focus player's slot
/// (`Players[focus].m_nMoney`, +0xB8), the player ped's armour = player 0's max armour (0xB7CEE8,
/// not the focus slot), then the health cheat (tail jump to 0x438D60).
#[hook(addr = 0x438E40, conv = cdecl, name = "CCheat::MoneyArmourHealthCheat",
       expect = "51 0F B6 05 74 CD B7 00",
       unverified = "world5-2", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn money_armour_health_cheat() {
    let slot = usize::from(g::<u8>(0xB7_CD74).read());
    let money = g::<i32>(0xB7_CE50 + slot * 0x190);
    money.write(money.read().wrapping_add(250_000));
    let armour = xi(i32::from(g::<u8>(0xB7_CEE8).read())).to_f32();
    wr::<f32>(find_player_ped(-1), 0x548, armour);
    health_cheat();
}

/// `CCheat::ParachuteCheat()` (0x4395B0): loads the parachute model (0x173, mission priority),
/// gives the player a parachute (weapon 46, no ammo, "like unlimited") and selects it.
#[hook(addr = 0x4395B0, conv = cdecl, name = "CCheat::ParachuteCheat",
       expect = "6A 02 68 73 01 00 00",
       unverified = "world5-2", route = "split", hazards = "heap-writes", recs = "world-cheats",
       note = "loads a model synchronously (streaming file I/O)")]
pub fn parachute_cheat() {
    request_model(0x173, 2);
    load_all_requested_models(0);
    ped_give_weapon(find_player_ped(-1), 0x2E, 0, 1);
    ped_set_current_weapon(find_player_ped(-1), 0x2E);
}

/// `CCheat::AdrenalineCheat()` (0x439880): toggles the cheat flag (0x969177); when turned on, the
/// adrenaline pickup's effect (`GivePlayerGoodiesWithPickUpMI(MI_PICKUP_ADRENALINE, 0)`).
#[hook(addr = 0x439880, conv = cdecl, name = "CCheat::AdrenalineCheat",
       expect = "8A 15 77 91 96 00",
       unverified = "world5-2", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn adrenaline_cheat() {
    if toggle(0x96_9177) != 0 {
        give_player_goodies(u32::from(g::<u16>(0x8C_D5BC).read()), 0);
    }
}

/// `CCheat::BlowUpCarsCheat()` (0x439D80): every vehicle of the pool (0xB74494, slots descending,
/// 0xA18 bytes each, free slots skipped) gets `BlowUpCar(nullptr, false)`.
#[hook(addr = 0x439D80, conv = cdecl, name = "CCheat::BlowUpCarsCheat",
       expect = "53 8B 1D 94 44 B7 00",
       unverified = "world5-2", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "blows up every vehicle (explosions, fires, damage events)")]
pub fn blow_up_cars_cheat() {
    let pool = g::<GPtr<u8>>(0xB7_4494).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag >= 0 {
            let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
            if !v.is_null() {
                v_blow_up_car(v, 0, 0);
            }
        }
    }
}

/// `CCheat::ElvisLivesCheat()` (0x4391D0): toggles the flag (0x969157); when on, streams the
/// three Elvis ped models (0x52, 0x53, 0x54; the other five slots -2) into random ped slots.
#[hook(addr = 0x4391D0, conv = cdecl, name = "CCheat::ElvisLivesCheat",
       expect = "8A 15 57 91 96 00",
       unverified = "world5-2", route = "split", hazards = "none", recs = "world-cheats",
       note = "streams models (file I/O)")]
pub fn elvis_lives_cheat() {
    if toggle(0x96_9157) != 0 {
        let mut models: [i32; 8] = [0x52, 0x53, 0x54, -2, -2, -2, -2, -2];
        stream_peds_into_random_slots(GPtr::from_raw(&mut models as *mut [i32; 8]).cast());
    }
}

/// `CCheat::DrivebyCheat()` (0x4398D0): toggles the flag (0x969179); when on and the player ped's
/// pointer +0x610 is null, gives a delayed micro SMG (weapon 28, 150 rounds) and selects it.
#[hook(addr = 0x4398D0, conv = cdecl, name = "CCheat::DrivebyCheat",
       expect = "8A 15 79 91 96 00",
       unverified = "world5-2", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn driveby_cheat() {
    if toggle(0x96_9179) != 0 && rd::<u32>(find_player_ped(-1), 0x610) == 0 {
        ped_give_delayed_weapon(find_player_ped(-1), 0x1C, 0x96);
        ped_set_current_weapon(find_player_ped(-1), 0x1C);
    }
}

/// `CCheat::HandleSpecialCheats(int32 cheat)` (0x439A10): entering another cheat turns off the
/// mutually exclusive ones: beach party (0x969159) unless cheat 41 (by calling its toggle), the
/// flags 0x96915E / 0x96915F unless 46 / 47 (cleared), funhouse (0x969176) unless 70 and
/// countryside invasion (0x96917B) unless 75 (by calling their toggles).
#[hook(addr = 0x439A10, conv = cdecl, name = "CCheat::HandleSpecialCheats",
       expect = "A0 59 91 96 00",
       unverified = "world5-2", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn handle_special_cheats(cheat: i32) {
    if g::<u8>(0x96_9159).read() != 0 && cheat != 0x29 {
        beach_party_cheat();
    }
    if g::<u8>(0x96_915E).read() != 0 && cheat != 0x2E {
        g::<u8>(0x96_915E).write(0);
    }
    if g::<u8>(0x96_915F).read() != 0 && cheat != 0x2F {
        g::<u8>(0x96_915F).write(0);
    }
    if g::<u8>(0x96_9176).read() != 0 && cheat != 0x46 {
        funhouse_cheat();
    }
    if g::<u8>(0x96_917B).read() != 0 && cheat != 0x4B {
        countryside_invasion_cheat();
    }
}

/// `CCheat::VillagePeopleCheat()` (0x439DD0): toggles the flag (0x969158); when on, streams the five
/// Village People ped models (0x1B, 0xA1, 0x11F, 0x11C, 0xF8; the other three slots -2), turns on
/// "everybody attacks the player" (its toggle) and sets the cheat-active flag 0x969140.
#[hook(addr = 0x439DD0, conv = cdecl, name = "CCheat::VillagePeopleCheat",
       expect = "8A 15 58 91 96 00",
       unverified = "world5-2", route = "split", hazards = "none", recs = "world-cheats",
       note = "streams models (file I/O)")]
pub fn village_people_cheat() {
    if toggle(0x96_9158) != 0 {
        let mut models: [i32; 8] = [0x1B, 0xA1, 0x11F, 0x11C, 0xF8, -2, -2, -2];
        stream_peds_into_random_slots(GPtr::from_raw(&mut models as *mut [i32; 8]).cast());
        everybody_attacks_player_cheat();
        g::<u8>(0x96_9140).write(1);
    }
}

/// `CCheat::JetpackCheat()` (0x439600): unless the player already has a jetpack task, a
/// `CTaskSimpleJetPack(nullptr, 10.0, 0, nullptr)` (null when the task pool is full) goes to the
/// player's event group as a script-command event (index 3), built and destroyed on the stack.
#[hook(addr = 0x439600, conv = cdecl, name = "CCheat::JetpackCheat",
       expect = "64 A1 00 00 00 00",
       unverified = "world5-2", route = "L2", hazards = "heap-writes", recs = "world-cheats",
       note = "allocates a task (task pool) and an event copy in the event group")]
pub fn jetpack_cheat() {
    if !intelligence_get_task_jet_pack(rd::<GPtr<u8>>(find_player_ped(-1), 0x47C)).is_null() {
        return;
    }
    let mem = task_new(0x70);
    let task = if mem.is_null() { mem } else { task_jetpack_ctor(mem, 0, 0x4120_0000, 0, 0) };
    let mut ev = [0u32; 6];
    let ep: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 6]).cast();
    event_script_command_ctor(ep, 3, task, 0);
    let intel = rd::<GPtr<u8>>(find_player_ped(-1), 0x47C);
    event_group_add(intel.byte_add(0x68), ep, 0);
    event_script_command_dtor(ep);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_MONEY_ARMOUR_HEALTH_CHEAT,
    HOOK_PARACHUTE_CHEAT,
    HOOK_ADRENALINE_CHEAT,
    HOOK_BLOW_UP_CARS_CHEAT,
    HOOK_ELVIS_LIVES_CHEAT,
    HOOK_DRIVEBY_CHEAT,
    HOOK_HANDLE_SPECIAL_CHEATS,
    HOOK_VILLAGE_PEOPLE_CHEAT,
    HOOK_JETPACK_CHEAT,
];
