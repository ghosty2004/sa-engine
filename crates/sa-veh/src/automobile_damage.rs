//! WRITER-VEH-BIG (batch `vehbig-1`): `CAutomobile::VehicleDamage`, the collision / weapon damage
//! entry of every four-wheeled vehicle. Ported from the 1.0 US bytes; gta-reversed as a structural
//! guide only. Where they differ, the bytes win (noted on the hook).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::world::common::{find_player_ped, find_player_vehicle};

use crate::common::{ftol32, g, rand_at, rd, wr, x};

orig! {
    /// `CBike::DamageKnockOffRider(CVehicle*, float, uint16, CEntity*, CVector&, CVector&)` (0x6B5A10, cdecl)
    fn damage_knock_off_rider(v: GPtr<u8>, dmg: u32, comp: u32, e: GPtr<u8>, coors: GPtr<u8>, dir: GPtr<u8>) -> u8 = cdecl 0x6B5A10 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, hidden return pointer)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => sa_world::world::mvpa_math::scalar_mul_vector;
    /// `DotProduct(const RwV3d*, const RwV3d*)` (0x40FDB0), ST0
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => dot_product_native;
    /// `CStats::GetPercentageProgress()` (0x5591E0), ST0
    fn get_percentage_progress() -> f64 = cdecl 0x5591E0 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30)
    fn set_wanted_level_no_drop(p: GPtr<u8>, level: i32) = thiscall 0x609F30 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake(int16, uint8, uint32)` (0x53F920)
    fn start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => sa_world::world::entity::register_reference;
    /// `CDamageManager::GetLightStatus(eLights)` (0x6C2130)
    fn get_light_status(dm: GPtr<u8>, light: u32) -> u32 = thiscall 0x6C2130 => crate::damage::get_light_status;
    /// `CAutomobile::dmgDrawCarCollidingParticles(const CVector&, float, eWeaponType)` (0x6A6DC0)
    fn dmg_draw_car_colliding_particles(v: GPtr<u8>, coors: GPtr<u8>, intensity: f32, weapon: u32) = thiscall 0x6A6DC0 => unported;
    /// `CVehicle::CanVehicleBeDamaged(CEntity*, eWeaponType, bool*)` (0x6D1280)
    fn can_vehicle_be_damaged(v: GPtr<u8>, e: GPtr<u8>, weapon: u32, out: GPtr<u8>) -> u8 = thiscall 0x6D1280 => unported;
    /// `CDamageManager::ApplyDamage(CAutomobile*, tComponent, float, float)` (0x6C24B0)
    fn apply_damage(dm: GPtr<u8>, car: GPtr<u8>, comp: i32, intensity: f32, mult: f32) -> u8 = thiscall 0x6C24B0 => crate::damage::apply_damage;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => sa_world::world::mvpa_math::vector_sub;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `CDamageManager::GetPanelStatus(int32)` (0x6C2180)
    fn get_panel_status(dm: GPtr<u8>, panel: u32) -> u32 = thiscall 0x6C2180 => crate::damage::get_panel_status;
    /// `CVector::Difference(const CVector&, const CVector&)` (0x40FE00, thiscall on the output): `a - b`
    fn vector_difference(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FE00 => unported;
    /// `CEventVehicleDamageCollision::CEventVehicleDamageCollision(CVehicle*, CVehicle*, eWeaponType)` (0x6A0620)
    fn event_vehicle_damage_collision_ctor(ev: GPtr<u8>, v: GPtr<u8>, other: GPtr<u8>, weapon: u32) -> GPtr<u8> = thiscall 0x6A0620 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CVehicle::ReactToVehicleDamage(CPed*)` (0x6D5490)
    fn react_to_vehicle_damage(v: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6D5490 => unported;
    /// `CEventVehicleDamage` base destructor body (0x4B1990), after the vtable store 0x8710D0
    fn event_vehicle_damage_dtor(ev: GPtr<u8>) = thiscall 0x4B1990 => unported;
    /// `CAEVehicleAudioEntity::GetVehicleTypeForAudio()` (0x4F4F00): int16 in AX
    fn get_vehicle_type_for_audio(a: GPtr<u8>) -> u32 = thiscall 0x4F4F00 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0)
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CVehicle::PickRandomPassenger()` (0x6D2A10)
    fn pick_random_passenger(v: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D2A10 => crate::vehicle2::pick_random_passenger;
    /// `CInterestingEvents::Add(EType, CEntity*)` (0x602590) on `g_InterestingEvents` (0xC0B058)
    fn interesting_events_add(ie: GPtr<u8>, kind: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CAutomobile::BlowUpCar(CEntity*, bool)` (vtable +0xA4)
    fn v_blow_up_car(v: GPtr<u8>, culprit: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn add_audio_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CDamageManager::GetEngineStatus()` (0x6C22C0)
    fn get_engine_status(dm: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => crate::damage::get_engine_status;
    /// `CDamageManager::SetEngineStatus(uint32)` (0x6C22A0)
    fn set_engine_status(dm: GPtr<u8>, s: u32) = thiscall 0x6C22A0 => crate::damage::set_engine_status;
}

/// Phases B/C side of `DotProduct` 0x40FDB0 (sa-math's port of the same sum order).
pub fn dot_product_native(a: GPtr<u8>, b: GPtr<u8>) -> f64 {
    let v = |p: GPtr<u8>| sa_math::vector::CVector::new(rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    sa_math::vector::dot_product_x87(&v(a), &v(b)).to_f64()
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;

fn ld(p: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(p, off))
}

fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

fn gf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `(a.z*b.z + a.y*b.y) + a.x*b.x` with `a` at `p + off` (the inlined dot products).
fn dot_at(p: GPtr<u8>, off: isize, v: GPtr<u8>) -> X87 {
    ld(p, off + 8)
        .fmul_m(rd::<f32>(v, 8))
        .fadd(ld(p, off + 4).fmul_m(rd::<f32>(v, 4)))
        .fadd(ld(p, off).fmul_m(rd::<f32>(v, 0)))
}

fn etype(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// `CAutomobile::VehicleDamage(float damageIntensity, eVehicleCollisionComponent component,
/// CEntity* damager, CVector* coors, CVector* dir, eWeaponType weapon)` (0x6A7650, `ret 0x18`).
///
/// Nothing unless +0x42A bit 5 (can be damaged). Intensity 0 means "the last collision": intensity,
/// component, damager, point (+0xEC) and direction (+0xE0) come from the physical, the threshold is
/// `(mass * (1/1500)) * 25` and the factor 1.0 (else 25 and 0.333). An upside-down collision of a
/// car the player is not in: returns for +0x868 bit 3, status 8, submerged; else health -=
/// `timeStep * 4` (not below 0) unless wrecked. Returns for intensity 0, +0x40 bit 20, or a bike
/// rider knock-off. Head-on (forward . dir < -g 0x8D31C8) physics-status cars without +0x4A4 == 2
/// and intensity / mass > g 0x8D31C4 get temp action 0x13 for 4000 ms; with the global 0xB72914 set
/// and |forward . dir| > g, a vehicle damager scales by 0.333; a ped damager in a vehicle (+0x46C
/// bit 0) driving into the car reduces it by `dot * mass` (not below 0). The explicit path asks
/// `CanVehicleBeDamaged` instead. Then: buildings with `up . dir > 0.6` do nothing; non-player cars
/// with +0x40 bit 22 take damage only from the player (ped or vehicle); the player's car takes half
/// at 100% progress; the towing / towed vehicle does nothing. Above the threshold (not wrecked):
/// law enforcers rammed by a faster player car (> 0.1) give wanted level 1, the player's car
/// shakes the pad (`f = min((i / mass) * 800 + 100, 250)`, `40000 / f` ms), a vehicle damager is
/// recorded (+0x508 = 0x31, +0x50C), the light statuses are snapshotted, the player's last-bump
/// time is set, colliding particles above `|v|^2 > 0.0004`, and (upright or `|v|^2 > 0.3`) the
/// component damage: components 1/2/5..10/19 damage panels / doors 5/6/7/8/9/0xA/0xB/0xC/0xF, the
/// front bumper (3) adds bonnet / doors / windscreen by side offset, direction and mass ratios (one
/// `rand()` at 0x6A7D76 for the windscreen), the rear bumper (4) the boot / rear doors for vans.
/// Health then drops by `(i - threshold) * collisionDamageMultiplier * 0.6 * factor` (x7 for 0x1AC hit
/// by the player, x30 for 0x1D1/0x1F5/0x234, x15 from a Rhino), scaled 0.5 / 0.1667 (player's car,
/// +0x429 bit 7 = mission vehicle) or 0.0833 / 0.6667 (hit by the player) / 0.25, with the collision
/// event, the drivers' and a random passenger's speech and the interesting event above 5; a car
/// that drops to <= 0 from a positive health keeps 1.0 unless a Rhino (not the player's car) or
/// the cheat 0x969164 with the player's car as damager blows it up. A light that broke plays audio
/// event 0x5C; below 250 health the engine status rises to 225 (fire starter recorded at +0x928,
/// a passenger says 0x21); model 0x1A8 gets engine status 200 below 400 or 100 below 600.
///
/// The original keeps scratch values in its own argument slots (intensity, component, damager,
/// coors, dir); the port uses locals.
#[hook(addr = 0x6a7650, conv = thiscall, name = "CAutomobile::VehicleDamage", this = "CAutomobile",
       expect = "6A FF 68 D8 7F 84 00",
       unverified = "vehbig-1", route = "split", hazards = "stack-residue,heap-writes",
       recs = "driving,car-ai",
       note = "rand site 0x6A7D76; own argument slots rewritten (damage, component, coors, dir, damager); StartShake's frequency dword carries the damager slot's upper bytes in the original (zero-extended here); the collision event lives on the Rust stack (0x28 bytes, as the original frame)")]
pub fn automobile_vehicle_damage(
    this: GPtr<u8>,
    damage: f32,
    component: u32,
    damager_in: GPtr<u8>,
    coors_in: GPtr<u8>,
    dir_in: GPtr<u8>,
    weapon: u32,
) {
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    let zero = X87::zero();
    // L5c: threshold, L58: factor
    let mut threshold = 25.0f32;
    let mut factor = f32::from_bits(0x3EAA_7EFA);
    let mut dmg = damage;
    let (comp, damager, coors, dir);
    if !x(damage).fcom(zero).test_parity(0x44) {
        // collision path
        threshold = ld(this, 0x8C)
            .fmul_m(f32::from_bits(0x3A2E_C33E))
            .fmul_m(25.0)
            .to_f32();
        dmg = rd::<f32>(this, 0xD8);
        comp = u32::from(rd::<u16>(this, 0xF8));
        damager = rd::<GPtr<u8>>(this, 0xDC);
        coors = this.byte_add(0xEC);
        dir = this.byte_add(0xE0);
        factor = 1.0;
        let m = rd::<GPtr<u8>>(this, 0x14);
        if !ge(ld(m, 0x28), zero) && find_player_vehicle(-1, 0) != this {
            if rd::<u8>(this, 0x868) & 8 != 0 {
                return;
            }
            let st = rd::<u8>(this, 0x36) >> 3;
            if st == 8 || rd::<u32>(this, 0x40) & 0x100 != 0 {
                return;
            }
            if st != 5 {
                let mut h = ld(this, 0x4C0).fsub(gf(TIME_STEP).fmul_m(4.0));
                if !le(zero, h) {
                    h = zero;
                }
                wr::<f32>(this, 0x4C0, h.to_f32());
            }
        }
        if !x(dmg).fcom(zero).test_parity(0x44) {
            return;
        }
        if rd::<u32>(this, 0x40) & 0x10_0000 != 0 {
            return;
        }
        if rd::<u32>(this, 0x594) == 2
            && damage_knock_off_rider(this, dmg.to_bits(), comp, damager, coors, dir) != 0
        {
            return;
        }
        // 0x6A77D0: forward . dir (kept on the x87 stack)
        let d = dot_at(m, 0x10, dir);
        if rd::<u8>(this, 0x36) & 0xF8 == 0x18
            && rd::<u8>(this, 0x4A4) != 2
            && !ge(d, gf(0x8D_31C8).fchs())
            && !le(x(dmg).fdiv_m(rd::<f32>(this, 0x8C)), gf(0x8D_31C4))
        {
            wr::<u8>(this, 0x3BB, 0x13);
            wr::<u32>(this, 0x3BC, g::<u32>(0xB7_CB84).read().wrapping_add(4000));
        }
        if g::<u32>(0xB7_2914).read() != 0 && !le(d.fabs(), gf(0x8D_31C8)) {
            if damager.is_null() {
                return automobile_vehicle_damage_tail(
                    this, dmg, comp, damager, coors, dir, weapon, threshold, factor,
                );
            }
            if etype(damager) == 2 {
                dmg = x(dmg).fmul_m(f32::from_bits(0x3EAA_7EFA)).to_f32();
            }
        }
        if damager.is_null() {
            return automobile_vehicle_damage_tail(
                this, dmg, comp, damager, coors, dir, weapon, threshold, factor,
            );
        }
        if etype(damager) == 3 && rd::<u8>(damager, 0x46C) & 1 != 0 {
            let mut out = [0f32; 3];
            let s = scalar_mul_vector(
                GPtr::from_raw(out.as_mut_ptr()).cast(),
                rd::<f32>(damager, 0x4DC),
                rd::<GPtr<u8>>(damager, 0x14).byte_add(0x10),
            );
            let r = X87::from_f64_exact(dot_product(s, dir));
            if !ge(r, zero) {
                let v = r.fmul_m(rd::<f32>(damager, 0x8C)).fadd_m(dmg);
                dmg = if le(zero, v) { v.to_f32() } else { 0.0 };
            }
        }
    } else {
        comp = component;
        (damager, coors, dir) = (damager_in, coors_in, dir_in);
        let mut immune = 0u8;
        if can_vehicle_be_damaged(this, damager, weapon, GPtr::from_raw(&mut immune)) == 0 {
            return;
        }
    }
    // 0x6A78F0
    if !damager.is_null() && etype(damager) == 1 {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let up = ld(m, 0x28)
            .fmul_m(rd::<f32>(dir, 8))
            .fadd(ld(m, 0x24).fmul_m(rd::<f32>(dir, 4)))
            .fadd(ld(dir, 0).fmul_m(rd::<f32>(m, 0x20)));
        if !le(up, c(0x3F19_999A)) {
            return;
        }
    }
    automobile_vehicle_damage_tail(this, dmg, comp, damager, coors, dir, weapon, threshold, factor)
}

/// 0x6A7927..: everything after the damager filters (see the hook's docs).
#[allow(clippy::too_many_arguments)]
fn automobile_vehicle_damage_tail(
    this: GPtr<u8>,
    mut dmg: f32,
    comp: u32,
    damager: GPtr<u8>,
    coors: GPtr<u8>,
    dir: GPtr<u8>,
    weapon: u32,
    threshold: f32,
    factor: f32,
) {
    let player_status = rd::<u8>(this, 0x36) & 0xF8 == 0;
    if !player_status {
        if rd::<u32>(this, 0x40) & 0x40_0000 != 0
            && !damager.is_null()
            && damager != find_player_ped(-1)
            && damager != find_player_vehicle(-1, 0)
        {
            return;
        }
    } else if !X87::from_f64_exact(get_percentage_progress())
        .fcom(c(0x42C8_0000))
        .test_any(1)
    {
        dmg = x(dmg).fmul_m(0.5).to_f32();
    }
    if !damager.is_null()
        && (damager == rd::<GPtr<u8>>(this, 0x4C4) || damager == rd::<GPtr<u8>>(this, 0x4C8))
    {
        return;
    }
    if !le(x(dmg), x(threshold)) && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        damage_effects(
            this, &mut dmg, comp, damager, coors, dir, weapon, threshold, factor,
        );
    }
    // 0x6A83DF
    let dm = this.byte_add(0x5A0);
    if !ge(ld(this, 0x4C0), c(0x437A_0000)) {
        if get_engine_status(dm) >= 0xE1 {
            return;
        }
        set_engine_status(dm, 0xE1);
        let e = rd::<GPtr<u8>>(this, 0xDC);
        wr::<u32>(this, 0x8E4, 0);
        wr::<GPtr<u8>>(this, 0x928, e);
        if !e.is_null() {
            register_reference(e, this.byte_add(0x928));
        }
        let p = pick_random_passenger(this);
        if !p.is_null() {
            ped_say(p, 0x21, 0x5DC, 0x3F80_0000, 0, 0, 0);
        }
    } else if rd::<u16>(this, 0x22) == 0x1A8 {
        if !ge(ld(this, 0x4C0), c(0x43C8_0000)) {
            set_engine_status(dm, 0xC8);
        } else if !ge(ld(this, 0x4C0), c(0x4416_0000)) {
            set_engine_status(dm, 0x64);
        }
    }
}

/// 0x6A79C0..0x6A83DF: above the threshold.
#[allow(clippy::too_many_arguments)]
fn damage_effects(
    this: GPtr<u8>,
    dmg: &mut f32,
    comp: u32,
    damager: GPtr<u8>,
    coors: GPtr<u8>,
    dir: GPtr<u8>,
    weapon: u32,
    threshold: f32,
    factor: f32,
) {
    let zero = X87::zero();
    if rd::<u8>(this, 0x428) & 1 != 0
        && !find_player_vehicle(-1, 0).is_null()
        && damager == find_player_vehicle(-1, 0)
        && rd::<u8>(this, 0x36) & 0xF8 != 0x20
    {
        let pv = find_player_vehicle(-1, 0);
        let mine = X87::from_f64_exact(vector_magnitude(this.byte_add(0x44))).to_f32();
        if !X87::from_f64_exact(vector_magnitude(pv.byte_add(0x44)))
            .fcom(x(mine))
            .test_any(1)
        {
            let pv = find_player_vehicle(-1, 0);
            if !le(
                X87::from_f64_exact(vector_magnitude(pv.byte_add(0x44))),
                c(0x3DCC_CCCD),
            ) {
                let ped = find_player_ped(-1);
                set_wanted_level_no_drop(ped, 1);
            }
        }
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 && !le(x(*dmg), c(0x4248_0000)) {
        let mut f = x(*dmg).fdiv_m(rd::<f32>(this, 0x8C)).fmul_m(800.0).fadd_m(100.0);
        if !ge(c(0x437A_0000), f) {
            f = c(0x437A_0000);
        }
        let freq = ftol32(f) as u8;
        let time = 40000 / i32::from(freq);
        start_shake(get_pad(0), time as u32, u32::from(freq), 2000);
    }
    if !damager.is_null() && etype(damager) == 2 {
        wr::<u8>(this, 0x508, 0x31);
        wr::<GPtr<u8>>(this, 0x50C, damager);
        register_reference(damager, this.byte_add(0x50C));
    }
    let dm = this.byte_add(0x5A0);
    // L3c..L30: the light statuses before the damage
    let mut lights = [0u32; 4];
    for (i, l) in lights.iter_mut().enumerate() {
        *l = get_light_status(dm, i as u32);
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let idx = usize::from(g::<u8>(0xB7_CD74).read());
        g::<u32>(0xB7_CE68 + idx * 0x190).write(g::<u32>(0xB7_CB84).read());
    }
    // Ac: |moveSpeed|^2 as stored
    let (mx, my, mz) = (ld(this, 0x44), ld(this, 0x48), ld(this, 0x4C));
    let ms2 = mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz)).to_f32();
    if !le(x(ms2), c(0x39D1_B717)) {
        let i = x(factor).fmul_m(*dmg).to_f32();
        dmg_draw_car_colliding_particles(this, coors, i, weapon);
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !le(ld(m, 0x28), zero) || !le(x(ms2), c(0x3E99_999A)) {
        component_damage(this, *dmg, comp, coors, dir, weapon);
    }

    // 0x6A8044
    let h = rd::<GPtr<u8>>(this, 0x384);
    let mut d = x(*dmg)
        .fsub_m(threshold)
        .fmul_m(rd::<f32>(h, 0xC8))
        .fmul_m(f32::from_bits(0x3F19_999A))
        .fmul_m(factor)
        .to_f32();
    let model = rd::<u16>(this, 0x22);
    if model == 0x1AC {
        let e = rd::<GPtr<u8>>(this, 0xDC);
        if !e.is_null() && rd::<u8>(e, 0x36) & 0xF8 == 0 {
            d = x(d).fmul_m(7.0).to_f32();
        }
    }
    if model == 0x1D1 || model == 0x1F5 || model == 0x234 {
        d = x(d).fmul_m(30.0).to_f32();
    }
    let culprit = rd::<GPtr<u8>>(this, 0xDC);
    if !culprit.is_null() && rd::<u16>(culprit, 0x22) == 0x1B0 {
        d = x(d).fmul_m(15.0).to_f32();
    }
    *dmg = d;
    if !le(x(d), zero) {
        health_damage(this, d, culprit);
    }
    for (i, &l) in lights.iter().enumerate() {
        if l != 1 && get_light_status(dm, i as u32) == 1 {
            add_audio_event(this.byte_add(0x138), 0x5C, 0.0);
            break;
        }
    }
}

/// 0x6A80E7..0x6A83AB: the collision event, speech and the health loss.
fn health_damage(this: GPtr<u8>, d: f32, culprit: GPtr<u8>) {
    let five = c(0x40A0_0000);
    if !culprit.is_null()
        && etype(culprit) == 2
        && !rd::<GPtr<u8>>(culprit, 0x460).is_null()
        && !rd::<GPtr<u8>>(this, 0x460).is_null()
        && !le(x(d), five)
    {
        let mut diff = [0f32; 3];
        let dp = GPtr::from_raw(diff.as_mut_ptr()).cast::<u8>();
        vector_difference(dp, culprit.byte_add(0x44), this.byte_add(0x44));
        let v = x(diff[1])
            .fmul_m(rd::<f32>(this, 0xE4))
            .fadd(x(diff[2]).fmul_m(rd::<f32>(this, 0xE8)))
            .fadd(x(diff[0]).fmul_m(rd::<f32>(this, 0xE0)));
        let (veh, other) = if ge(v, X87::zero()) {
            (culprit, this)
        } else {
            (this, culprit)
        };
        // L2c: the event (the frame reserves 0x28 bytes up to the SEH state)
        let mut ev = [0u32; 10];
        let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
        event_vehicle_damage_collision_ctor(evp, veh, other, 0x31);
        let driver = rd::<GPtr<u8>>(veh, 0x460);
        event_group_add(rd::<GPtr<u8>>(driver, 0x47C).byte_add(0x68), evp, 0);
        react_to_vehicle_damage(this, rd::<GPtr<u8>>(veh, 0x460));
        wr::<u32>(evp, 0, 0x87_10D0);
        event_vehicle_damage_dtor(evp);
    }
    if !le(x(d), five) {
        let e = rd::<GPtr<u8>>(this, 0xDC);
        if !rd::<GPtr<u8>>(this, 0x460).is_null()
            && !e.is_null()
            && etype(e) == 2
            && !(find_player_vehicle(-1, 0) == this && rd::<u8>(e, 0x4A4) == 2)
            && !rd::<GPtr<u8>>(e, 0x460).is_null()
        {
            let t = get_vehicle_type_for_audio(this.byte_add(0x138)) as u16 as i16;
            let ctx = match t {
                0 => 0x43,
                1 => 0x42,
                _ => 0x44,
            };
            ped_say(rd::<GPtr<u8>>(this, 0x460), ctx, 0, 0x3F28_F5C3, 0, 0, 0);
        }
        if find_player_vehicle(-1, 0) == this && !e.is_null() {
            let p = pick_random_passenger(this);
            if !p.is_null() {
                let ctx = if etype(e) == 3 { 0x24 } else { 0x1D };
                ped_say(p, ctx, 0, 0x3F80_0000, 0, 0, 0);
            }
        }
        interesting_events_add(GPtr::from_addr(0xC0_B058), 0x11, this);
    }
    // ebx: the health before, as an integer
    let before = ftol32(ld(this, 0x4C0));
    let mission = rd::<u8>(this, 0x429) & 0x80 != 0;
    let loss = if find_player_vehicle(-1, 0) == this {
        x(d).fmul_m(if mission { f32::from_bits(0x3E2A_AAAB) } else { 0.5 })
    } else if mission {
        x(d).fmul_m(f32::from_bits(0x3DAA_AAAB))
    } else {
        let e = rd::<GPtr<u8>>(this, 0xDC);
        if !e.is_null() && e == find_player_vehicle(-1, 0) {
            x(d).fmul_m(f32::from_bits(0x3F2A_AAAB))
        } else {
            x(d).fmul_m(0.25)
        }
    };
    wr::<f32>(this, 0x4C0, loss.fsubr_m(rd::<f32>(this, 0x4C0)).to_f32());
    if g::<u8>(0x96_9164).read() != 0 {
        let e = rd::<GPtr<u8>>(this, 0xDC);
        if !e.is_null() && e == find_player_vehicle(-1, 0) {
            v_blow_up_car(this, rd::<GPtr<u8>>(this, 0xDC), 0);
            return;
        }
    }
    if !ld(this, 0x4C0).fcom(X87::zero()).test_parity(0x41) && (before as i16) > 0 {
        let e = rd::<GPtr<u8>>(this, 0xDC);
        wr::<u32>(this, 0x4C0, 0x3F80_0000);
        if !e.is_null() && rd::<u16>(e, 0x22) == 0x1B0 && find_player_vehicle(-1, 0) != this {
            v_blow_up_car(this, rd::<GPtr<u8>>(this, 0xDC), 0);
        }
    }
}

/// 0x6A7BA9..0x6A8044: the per-component damage (`ebx` collects the extra parts).
fn component_damage(this: GPtr<u8>, dmg: f32, comp: u32, coors: GPtr<u8>, dir: GPtr<u8>, weapon: u32) {
    let dm = this.byte_add(0x5A0);
    let mass = || rd::<f32>(this, 0x8C);
    let apply = |part: i32| {
        let h = rd::<GPtr<u8>>(this, 0x384);
        let mult = rd::<f32>(h, 0xC8);
        apply_damage(dm, this, part, x(dmg).fmul_m(4.0).to_f32(), mult);
    };
    // `g * mass < dmg` (the `fld g; fmul mass; fcomp dmg; test ah, 5; jnp` tests)
    let heavy = |g_addr: usize| !ge(gf(g_addr).fmul_m(mass()), x(dmg));
    let c_m = |v: X87, bits: u32| v.fcom(c(bits));
    match (comp & 0xFFFF).wrapping_sub(1) {
        0 => apply(5),
        1 => apply(6),
        4 => apply(7),
        5 => apply(8),
        6 => apply(9),
        7 => apply(0xA),
        8 => apply(0xB),
        9 => apply(0xC),
        18 => apply(0xF),
        2 => {
            apply(0x10);
            let m = rd::<GPtr<u8>>(this, 0x14);
            // A14: dir . right, A10: (coors - pos) . right / bbox max x
            let side_dir = X87::from_f64_exact(dot_product(dir, m)).to_f32();
            let pos = if m.is_null() {
                this.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            let mut out = [0f32; 3];
            let r = vector_sub(GPtr::from_raw(out.as_mut_ptr()).cast(), coors, pos);
            let off = X87::from_f64_exact(dot_product(r, m)).to_f32();
            let off = x(off).fdiv_m(rd::<f32>(get_col_model(this), 0xC)).to_f32();
            let mut bits = 0u32;
            let o = x(off);
            if !c_m(o, 0x3F33_3333).test_any(0x41)
                || (!le(o, c(0x3F00_0000)) && !ge(x(side_dir), c(0xBF00_0000)) && heavy(0x8D_3158))
            {
                bits = 8;
            } else if !c_m(o, 0xBF33_3333).test_parity(0x05)
                || (!ge(o, c(0xBF00_0000)) && !le(x(side_dir), c(0x3F00_0000)) && heavy(0x8D_3158))
            {
                bits = 4;
            }
            if get_panel_status(dm, 5) >= 1
                && (heavy(0x8D_3150) || ((weapon as i32) < 0x2F && rand_at(0x6A_7D76) & 3 == 0))
            {
                bits |= 1;
            }
            if get_panel_status(dm, 5) >= 2 && heavy(0x8D_3154) {
                bits |= 2;
            }
            if bits & 1 != 0 {
                apply(5);
            }
            if bits & 4 != 0 {
                apply(0xB);
            }
            if bits & 8 != 0 {
                apply(0xC);
            }
            if bits & 2 != 0 {
                apply(0xF);
            }
        }
        3 => {
            apply(0x11);
            let m = rd::<GPtr<u8>>(this, 0x14);
            let pos = if m.is_null() {
                this.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            let mut out = [0f32; 3];
            let r = vector_sub(GPtr::from_raw(out.as_mut_ptr()).cast(), coors, pos);
            let a10 = X87::from_f64_exact(dot_product(r, m)).to_f32();
            let v = x(a10).fdiv_m(rd::<f32>(get_col_model(this), 0xC));
            let van = rd::<u8>(this, 0x429) & 1 != 0;
            let mut bits = 0u32;
            if van && !le(v, c(0x3DCC_CCCD)) && heavy(0x8D_3158) {
                bits = 0x40;
            } else if van && !ge(v, c(0xBDCC_CCCD)) && heavy(0x8D_3158) {
                bits = 0x20;
            }
            if get_panel_status(dm, 6) < 2 {
                bits |= 0x10;
            }
            if bits & 0x10 != 0 {
                apply(6);
            }
            if bits & 0x20 != 0 {
                apply(9);
            }
            if bits & 0x40 != 0 {
                apply(0xA);
            }
        }
        _ => {}
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_VEHICLE_DAMAGE];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
