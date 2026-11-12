//! PORT-VEH-4 (batch `veh4-3`): `CAutomobile::BlowUpCar`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, rd, wr, x, xi};

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(n: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `FUN_007322f0(RpClump*, uint32)` (0x7322F0, the clump's atomic flags)
    fn clump_set_flags_7322f0(clump: GPtr<u8>, flags: u32) = cdecl 0x7322F0 => unported;
    /// `CDamageManager::FuckCarCompletely(bool)` (0x6C25D0)
    fn damage_fuck_car_completely(d: GPtr<u8>, b: u32) = thiscall 0x6C25D0 => unported;
    /// `CAutomobile::SetBumperDamage(ePanels, bool)` (0x6B1350)
    fn set_bumper_damage(v: GPtr<u8>, panel: u32, b: u32) = thiscall 0x6B1350 => unported;
    /// `CAutomobile::SetDoorDamage(eDoors, bool)` (0x6B1600)
    fn set_door_damage(v: GPtr<u8>, door: u32, b: u32) = thiscall 0x6B1600 => unported;
    /// `CAutomobile::SpawnFlyingComponent(int32, uint32)` (0x6A8580)
    fn spawn_flying_component(v: GPtr<u8>, node: i32, kind: u32) -> GPtr<u8> = thiscall 0x6A8580 => crate::automobile_flying_component::automobile_spawn_flying_component;
    /// `RwFrameForAllObjects(RwFrame*, RwObjectCallBack, void*)` (0x7F1200)
    fn frame_for_all_objects(f: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1200 => unported;
    /// `CCamera::CamShake(float, float x, float y, float z)` (0x50A9F0) on `TheCamera`
    fn cam_shake(cam: GPtr<u8>, strength: f32, px: u32, py: u32, pz: u32) = thiscall 0x50A9F0 => unported;
    /// `CVehicle::KillPedsInVehicle()` (0x6D1C80)
    fn kill_peds_in_vehicle(v: GPtr<u8>) = thiscall 0x6D1C80 => unported;
    /// `CVehicle::KillPedsGettingInVehicle()` (0x6D82F0)
    fn kill_peds_getting_in_vehicle(v: GPtr<u8>) = thiscall 0x6D82F0 => unported;
    /// `CVehicle::ChangeLawEnforcerState(bool)` (0x6D2330)
    fn change_law_enforcer_state(v: GPtr<u8>, on: u32) = thiscall 0x6D2330 => unported;
    /// `CFireManager::StartFire(CEntity*, CEntity*, float, uint8, uint32, int8)` (0x53A050) on `gFireManager` (0xB71F80)
    fn start_fire(mgr: GPtr<u8>, target: GPtr<u8>, creator: GPtr<u8>, size: f32, a: u32, time: u32, generation: u32) -> GPtr<u8> = thiscall 0x53A050 => unported;
    /// `CDarkel::RegisterCarBlownUpByPlayer(CVehicle*, int32)` (0x43DF20)
    fn darkel_register_car_blown_up(v: GPtr<u8>, player: i32) = cdecl 0x43DF20 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CEntity* creator, eExplosionType, CVector, uint32
    /// lifetime, bool makeSound, float camShake, bool invisible)` (0x736A50)
    fn add_explosion(victim: GPtr<u8>, creator: GPtr<u8>, ty: u32, x: u32, y: u32, z: u32, life: u32, sound: u32, shake: u32, invisible: u32) -> u8 = cdecl 0x736A50 => unported;
}

/// `rand()` as the original call returning to `site` calls it, times 1/RAND_MAX (0x858C7C).
fn rand_unit(site: u32) -> X87 {
    xi(sa_core::orig::at_site(site, sa_core::crt::rand)).fmul_m(f32::from_bits(0x3800_0100))
}

/// `CAutomobile::BlowUpCar(CEntity* damager, bool hideExplosion)` (0x6B3780).
///
/// Only when the car can be damaged (+0x42A bit 5). Blown up by the player (ped or vehicle): +20 to the
/// focus player's +0x140 counter, +10 to its +0x148 float, stat 0x7D += `rand() % 6000 + 4000`. The news
/// chopper (488) clears 0xB7CB8C. Status 5 (wrecked), `ms.z += 0.13`, flag +0x40 bit 29, blow-up time
/// now, the clump flags 0x4000, the damage manager wrecked; except for the RC Tiger/Bandit (564/441) the
/// bumpers and doors are damaged, the bonnet (node 5) flies off and its atomic is hidden. Passengers
/// count bits, health and +0x4DE cleared; a 0.4 camera shake; occupants and peds getting in killed; the
/// siren/law flags cleared (decrementing the counters at 0x9690A8/0x9690AC); law enforcer state off; a
/// fire (size 0.8, 7 s) started by the damager; the player's Darkel kill registered. The explosion (type
/// 4, or 5 for the RC cars) goes at a random point of the col-model box (`[-k, k]` of max x along right,
/// of max y along forward, `-(u + 0.5) * max z` vertically; k = 0.75 for automobiles and quads, else
/// 0.1) with the given hide flag, no lifetime, sound, camera shake -1.
#[hook(addr = 0x6B3780, conv = thiscall, name = "CAutomobile::BlowUpCar", this = "CAutomobile",
       expect = "83 EC 24 53 55 56 8B F1",
       unverified = "veh4-3", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x6B37E4 (stat), 0x6B3A41/0x6B3AB3/0x6B3B0F (explosion point); spawns the bonnet, starts a fire and an explosion; the hide flag dword is passed on as the caller pushed it")]
pub fn automobile_blow_up_car(this: GPtr<u8>, damager: GPtr<u8>, hide: u32) {
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    if damager == find_player_ped(-1) || damager == find_player_vehicle(-1, 0) {
        let pi = 0xB7_CD98 + 0x190 * usize::from(g::<u8>(0xB7_CD74).read());
        let money = g::<i32>(pi + 0x140);
        money.write(money.read().wrapping_add(20));
        let f = g::<f32>(pi + 0x148);
        f.write(f32_of(x(f.read()).fadd_m(10.0)));
        let r = sa_core::orig::at_site(0x6B_37E4, sa_core::crt::rand);
        let v = r % 6000 + 4000;
        stats_increment_stat(0x7D, f32_of(xi(v)));
    }
    if rd::<i16>(this, 0x22) == 0x1E8 {
        g::<u8>(0xB7_CB8C).write(0);
    }
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x28);
    wr::<f32>(this, 0x4C, f32_of(x(rd::<f32>(this, 0x4C)).fadd_m(f32::from_bits(0x3E05_1EB8))));
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x2000_0000);
    wr::<u32>(this, 0x4D8, g::<u32>(0xB7_CB84).read());
    clump_set_flags_7322f0(rd::<GPtr<u8>>(this, 0x18), 0x4000);
    damage_fuck_car_completely(this.byte_add(0x5A0), 0);
    let model = rd::<i16>(this, 0x22);
    let rc = model == 0x234 || model == 0x1B9;
    if !rc {
        set_bumper_damage(this, 5, 0);
        set_bumper_damage(this, 6, 0);
        for door in 0..6 {
            set_door_damage(this, door, 0);
        }
        spawn_flying_component(this, 5, 1);
        let mut atomic = [0u32; 1];
        frame_for_all_objects(rd::<GPtr<u8>>(this, 0x65C), 0x6A_0750, GPtr::from_raw(&mut atomic as *mut [u32; 1]).cast());
        if atomic[0] != 0 {
            wr::<u8>(GPtr::from_addr(atomic[0] as usize), 2, 0);
        }
    }
    wr::<u8>(this, 0x4A8, rd::<u8>(this, 0x4A8) & 0xF8);
    wr::<u32>(this, 0x4C0, 0);
    wr::<u16>(this, 0x4DE, 0);
    let pos = || {
        let m = rd::<GPtr<u8>>(this, 0x14);
        if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) }
    };
    let p = pos();
    cam_shake(GPtr::from_addr(0xB6_F028), f32::from_bits(0x3ECC_CCCD), rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
    kill_peds_in_vehicle(this);
    kill_peds_getting_in_vehicle(this);
    wr::<u8>(this, 0x4A8, rd::<u8>(this, 0x4A8) & 0xE7);
    wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xAF);
    wr::<u8>(this, 0x42D, rd::<u8>(this, 0x42D) & 0x7F);
    wr::<u8>(this, 0x868, rd::<u8>(this, 0x868) & 0xFE);
    if rd::<u8>(this, 0x428) & 2 != 0 {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xFD);
        let c = g::<i32>(0x96_90A8);
        c.write(c.read().wrapping_sub(1));
    }
    if rd::<u8>(this, 0x428) & 4 != 0 {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xFB);
        let c = g::<i32>(0x96_90AC);
        c.write(c.read().wrapping_sub(1));
    }
    change_law_enforcer_state(this, 0);
    start_fire(GPtr::from_addr(0xB7_1F80), this, damager, f32::from_bits(0x3F4C_CCCD), 1, 7000, 0);
    darkel_register_car_blown_up(this, 0);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = pos();
    let (mut px, mut py, mut pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let sub = rd::<u32>(this, 0x594);
    let k = if sub == 0 || sub == 2 { 0.75f32 } else { f32::from_bits(0x3DCC_CCCD) };
    let lo = f32_of(x(k).fchs());
    let pick = |site: u32, o: isize| {
        let u = f32_of(rand_unit(site));
        let col = entity_get_col_model(this);
        x(k).fsub_m(lo).fmul_m(u).fadd_m(lo).fmul_m(rd::<f32>(col, o))
    };
    let v = pick(0x6B_3A41, 0xC);
    let dy = f32_of(v.fmul_m(rd::<f32>(m, 4)));
    let dz = f32_of(v.fmul_m(rd::<f32>(m, 8)));
    px = f32_of(v.fmul_m(rd::<f32>(m, 0)).fadd_m(px));
    py = f32_of(x(py).fadd_m(dy));
    pz = f32_of(x(dz).fadd_m(pz));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let v = pick(0x6B_3AB3, 0x10);
    let fy = f32_of(v.fmul_m(rd::<f32>(m, 0x14)));
    let fz = f32_of(v.fmul_m(rd::<f32>(m, 0x18)));
    px = f32_of(v.fmul_m(rd::<f32>(m, 0x10)).fadd_m(px));
    py = f32_of(x(fy).fadd_m(py));
    let u3 = f32_of(rand_unit(0x6B_3B0F));
    let col = entity_get_col_model(this);
    pz = f32_of(x(fz).fadd_m(pz).fsub(x(u3).fadd_m(0.5).fmul_m(rd::<f32>(col, 0x14))));
    let ty = if rc { 5 } else { 4 };
    add_explosion(this, damager, ty, px.to_bits(), py.to_bits(), pz.to_bits(), 0, 1, 0xBF80_0000, hide);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_BLOW_UP_CAR];
