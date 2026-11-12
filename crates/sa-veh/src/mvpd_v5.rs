//! WRITER-MVP-D (helper VEH, batch `mvpd-veh-5`): `CAutomobile` water handling. Ported from the 1.0
//! US bytes; gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, g, model_info, rand_at, rd, wr, x};

orig! {
    /// `CTimeCycle::GetAmbientRed_Obj()` (0x560360, ST0)
    fn get_ambient_red_obj() -> f64 = cdecl 0x560360 => unported;
    /// `CTimeCycle::GetAmbientGreen_Obj()` (0x560370, ST0)
    fn get_ambient_green_obj() -> f64 = cdecl 0x560370 => unported;
    /// `CTimeCycle::GetAmbientBlue_Obj()` (0x560380, ST0)
    fn get_ambient_blue_obj() -> f64 = cdecl 0x560380 => unported;
    /// `cBuoyancy::ProcessBuoyancy(CPhysical*, float, CVector*, CVector*)` (0x6C3EF0) on `mod_Buoyancy` (0xC1C890)
    fn buoyancy_process(b: GPtr<u8>, e: GPtr<u8>, buoyancy: u32, centre: GPtr<u8>, force: GPtr<u8>) -> u8 = thiscall 0x6C3EF0 => unported;
    /// `CVehicle::BlowUpCar(CEntity*, bool)` (virtual, vtable +0xA4)
    fn blow_up_car_virtual(v: GPtr<u8>, damager: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(p: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(p: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0, hidden result pointer)
    fn get_speed(p: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => sa_world::physical::forces::get_speed;
    /// `operator/(const CVector&, float)` (0x4119D0, hidden result pointer)
    fn vector_div(out: GPtr<u8>, v: GPtr<u8>, k: f32) -> GPtr<u8> = cdecl 0x4119D0 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0)
    fn vector_mul_assign(v: GPtr<u8>, k: f32) = thiscall 0x40FEF0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => sa_world::collision::data::normalise_port;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `SurfaceInfos_c::IsWater(uint32)` (0x55E710)
    fn surface_is_water(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E710 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::HandlePlayerBreath(bool, float)` (0x60A8D0), the float as bits
    fn player_handle_breath(ped: GPtr<u8>, b: u32, f: u32) = thiscall 0x60A8D0 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(...)` (0x4AFCF0)
    fn kob_ctor(e: GPtr<u8>, veh: GPtr<u8>, speed: GPtr<u8>, impact: GPtr<u8>, dmg: u32, a5: u32, kind: u32, dir: u32, time: u32, ped: u32, driver: u32, force: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CEventKnockOffBike::~CEventKnockOffBike()` (0x4AFE70)
    fn kob_dtor(e: GPtr<u8>) = thiscall 0x4AFE70 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(...)` (0x4AD3F0)
    fn damage_calc_ctor(c: GPtr<u8>, e: GPtr<u8>, dmg: u32, wt: u32, piece: u32, speak: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420)
    fn damage_calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CEventDamage::CEventDamage(...)` (0x4AD830)
    fn event_damage_ctor(e: GPtr<u8>, src: GPtr<u8>, time: u32, wt: u32, piece: u32, dir: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0)
    fn event_damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool)` (0x4B5AC0)
    fn damage_calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) = thiscall 0x4B5AC0 => unported;
}

const TIME_STEP: usize = 0xB7_CB5C;

fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

fn gt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// All four `m_fWheelsSuspensionCompression` (+0x7D4..) are exactly 1.0 (`test ah, 0x44; jp`).
fn all_extended(this: GPtr<u8>) -> bool {
    (0..4).all(|k| !x(rd::<f32>(this, 0x7D4 + 4 * k)).fcom(x(1.0)).test_parity(0x44))
}

/// The occupant reaction of `CAutomobile::ProcessBuoyancy` (`is_driver`: +0x428 bit 4 cleared
/// before the knock-off event is destroyed).
fn occupant(this: GPtr<u8>, ped: GPtr<u8>, is_driver: bool) {
    wr::<u32>(ped, 0x40, rd::<u32>(ped, 0x40) | 0x800_0000);
    if ped_is_player(ped) == 0 && rd::<u8>(this, 0x868) & 4 != 0 {
        return;
    }
    let group = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68);
    if rd::<u32>(this, 0x594) == 2 && all_extended(this) {
        let mut v = CVector::new(
            f32_of(x(rd::<f32>(this, 0x44)).fchs()),
            f32_of(x(rd::<f32>(this, 0x48)).fchs()),
            f32_of(x(rd::<f32>(this, 0x4C)).fchs()),
        );
        normalise(GPtr::from_raw(&mut v));
        let f = f32_of(X87::from_f64_exact(vector_magnitude(this.byte_add(0x44))).fmul_m(rd::<f32>(this, 0x8C)));
        let mut ev = [0u32; 0x20];
        let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
        kob_ctor(evp, this, this.byte_add(0x44), GPtr::from_raw(&mut v).cast(), f.to_bits(), 0, 0x35, 0, 0, 0, 0, 0);
        event_group_add(group(ped), evp, 0);
        if ped_is_player(ped) != 0 {
            player_handle_breath(ped, 1, 0x3F80_0000);
        }
        if is_driver {
            wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
        }
        kob_dtor(evp);
        return;
    }
    if ped_is_player(ped) != 0 {
        player_handle_breath(ped, 1, 0x3F80_0000);
        return;
    }
    let mut calc = [0u32; 8];
    let cp = GPtr::from_raw(calc.as_mut_ptr()).cast::<u8>();
    damage_calc_ctor(cp, this, g::<u32>(TIME_STEP).read(), 0x35, 3, 0);
    let mut ev = [0u32; 0x20];
    let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_damage_ctor(evp, this, g::<u32>(0xB7_CB84).read(), 0x35, 3, 0, 0, 1);
    if event_damage_affects_ped(evp, ped) != 0 {
        damage_calc_compute_damage_response(cp, ped, evp.byte_add(0x38), 1);
    } else {
        wr::<u8>(evp, 0x42, 1);
    }
    event_group_add(group(ped), evp, 0);
    event_damage_dtor(evp);
    damage_calc_dtor(cp);
}

fn clear_in_water(this: GPtr<u8>) {
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FEFF);
    wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xBF);
}

/// `CAutomobile::ProcessBuoyancy()` (0x6A8C00).
///
/// Calls the three `CTimeCycle::GetAmbient*_Obj` (results dropped) and one `rand` (dropped), then
/// `mod_Buoyancy.ProcessBuoyancy(this, +0xA0, &centre, &force)`. False: touching/in-water flags and
/// +0x42B bit 6 cleared, the buoyancy constant reset from handling +0x24, and for each compressed
/// wheel on a water surface `GetSpeed(colPoint + up * 0.3 - pos)` (result unused). True: a plane
/// subtype 4 falling faster than 1 blows up (damager itself); touching water; `imm` and the
/// `pow(max(1 - imm*0.05, 0.5), timeStep)` damping as for bikes (skipped for an upright Vortex that
/// is not drowning); amphibious handling (0x2000000) with +0x84C > 0.15: the Sea Sparrow / Leviathan
/// (0x1BF/0x1A1) float when imm > 3, others get `ApplyMoveForce(force * -2 / max(imm*8, 1))`, turn
/// speed *= the damping and float when imm > 0.9 (else return); floating clears +0x84C. Then
/// `ApplyMoveForce(force)`, `ApplyTurnForce(force, centre)`. Not in water: helicopter-boats with imm
/// < 3 and up.z > -0.5 (or imm < 0.6), a player car on water (cheat / Vortex) upright and not
/// drowning, or `imm < 0.6`, or `0.6 < imm < 1` with no wheel fully extended. In water: drowning
/// flags, move z >= -0.1, the buoyancy constant sinks by `mass * 8e-6` while above `mass * 0.0064`,
/// +0x428 bit 4 cleared below `mass * 0.008`, then the driver and passengers (knock-off for subtype
/// 2 with all wheels extended, else breath for players or a drowning `CEventDamage`).
#[hook(addr = 0x6A8C00, conv = thiscall, name = "CAutomobile::ProcessBuoyancy", this = "CAutomobile", expect = "6A FF 64 A1 00 00 00 00",
       route = "L2", hazards = "x87-transcendental",
       recs = "driving",
       note = "pow through orig::x87_binary(0x822130); rand site 0x6A8C3B (value dropped); epilogue through the protection fragment 0x408C75; event objects on the Rust stack")]
pub fn automobile_process_buoyancy(this: GPtr<u8>) {
    get_ambient_red_obj();
    get_ambient_green_obj();
    get_ambient_blue_obj();
    rand_at(0x6A_8C3B);
    let mut centre = CVector::default();
    let mut force = CVector::default();
    let ok = buoyancy_process(
        GPtr::from_addr(0xC1_C890),
        this,
        rd::<u32>(this, 0xA0),
        GPtr::from_raw(&mut centre).cast(),
        GPtr::from_raw(&mut force).cast(),
    );
    if ok == 0 {
        wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xBF);
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xF7FF_FEFF);
        wr::<u32>(this, 0xA0, rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0x24));
        for i in 0..4isize {
            if !lt(x(rd::<f32>(this, 0x7D4 + 4 * i)), x(1.0)) {
                continue;
            }
            let cpt = this.byte_add(0x724 + 0x2C * i);
            if surface_is_water(GPtr::from_addr(0xB7_9538), u32::from(rd::<u8>(cpt, 0x23))) == 0 {
                continue;
            }
            let m = rd::<GPtr<u8>>(this, 0x14);
            let pos = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
            let k = f32::from_bits(0x3E99_999A);
            let ux = x(rd::<f32>(m, 0x20)).fmul_m(k);
            let uy = x(rd::<f32>(m, 0x24)).fmul_m(k);
            let uz = f32_of(x(rd::<f32>(m, 0x28)).fmul_m(k));
            let px = f32_of(ux.fadd_m(rd::<f32>(cpt, 0)));
            let py = uy.fadd_m(rd::<f32>(cpt, 4));
            let pz = x(uz).fadd_m(rd::<f32>(cpt, 8));
            let p = CVector::new(
                f32_of(x(px).fsub_m(rd::<f32>(pos, 0))),
                f32_of(py.fsub_m(rd::<f32>(pos, 4))),
                f32_of(pz.fsub_m(rd::<f32>(pos, 8))),
            );
            let mut out = CVector::default();
            get_speed(this, GPtr::from_raw(&mut out), p);
        }
        return;
    }
    if rd::<u32>(this, 0x594) == 4 && lt(x(rd::<f32>(this, 0x4C)), x(-1.0)) {
        blow_up_car_virtual(this, this, 0);
    }
    let fl = rd::<u32>(this, 0x40) | 0x800_0000;
    wr::<u32>(this, 0x40, fl);
    let ts = g::<f32>(TIME_STEP).read();
    let t = if x(0.01).fcom(x(ts)).test_any(0x41) { x(ts) } else { x(0.01) };
    let mut imm = f32_of(t.fmul_m(rd::<f32>(this, 0x8C)).fmul_m(0.008).fdivr_m(force.z));
    let lim = x(rd::<f32>(this, 0x8C)).fmul_m(0.008);
    if gt(lim, x(rd::<f32>(this, 0xA0))) {
        imm = f32_of(lim.fdiv_m(rd::<f32>(this, 0xA0)).fmul_m(imm).fmul_m(1.05));
    }
    if fl & 1 != 0 {
        imm = f32_of(x(imm).fmul_m(1.5));
    }
    let k = x(imm).fmul_m(0.05).fsubr_m(1.0);
    let damp = if x(0.5).fcom(k).test_any(0x41) { f32_of(k) } else { 0.5 };
    let p = f32_of(X87::from_f64_exact(sa_core::orig::x87_binary(0x82_2130, x(damp).to_f64(), x(ts).to_f64())));
    let model = rd::<u16>(this, 0x22);
    let upright_vortex = model == 0x21B
        && gt(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28)), x(f32::from_bits(0x3E99_999A)))
        && rd::<u8>(this, 0x42B) & 0x40 == 0;
    if !upright_vortex {
        for off in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
            wr::<f32>(this, off, f32_of(x(p).fmul_m(rd::<f32>(this, off))));
        }
    }
    let heli_boat = |m: u16| m == 0x1BF || m == 0x1A1;
    let mut floating = false;
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x200_0000 != 0 && gt(x(rd::<f32>(this, 0x84C)), x(0.15)) {
        if heli_boat(model) {
            floating = gt(x(imm), x(3.0));
        } else {
            let t8 = x(imm).fmul_m(8.0);
            let c = if x(1.0).fcom(t8).test_any(0x41) { f32_of(t8) } else { 1.0 };
            let mut tmp = [
                f32_of(x(force.x).fmul_m(-2.0)),
                f32_of(x(force.y).fmul_m(-2.0)),
                f32_of(x(force.z).fmul_m(-2.0)),
            ];
            let mut out = [0f32; 3];
            let r = vector_div(GPtr::from_raw(out.as_mut_ptr()).cast(), GPtr::from_raw(tmp.as_mut_ptr()).cast(), c);
            apply_move_force(this, CVector::new(rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)));
            vector_mul_assign(this.byte_add(0x50), p);
            if !gt(x(imm), x(f32::from_bits(0x3F66_6666))) {
                return;
            }
            floating = true;
        }
        if floating {
            wr::<u32>(this, 0x84C, 0);
        }
    }
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x800_0000);
    apply_move_force(this, force);
    apply_turn_force(this, force, centre);
    let model = rd::<u16>(this, 0x22);
    if heli_boat(model) && lt(x(imm), x(3.0)) {
        let upz = x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28));
        if gt(upz, x(-0.5)) || lt(x(imm), x(f32::from_bits(0x3F19_999A))) {
            clear_in_water(this);
            return;
        }
    }
    if (g::<u8>(0x96_9152).read() != 0 || model == 0x21B)
        && rd::<u8>(this, 0x36) & 0xF8 == 0
        && gt(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28)), x(f32::from_bits(0x3E99_999A)))
        && rd::<u8>(this, 0x42B) & 0x40 == 0
    {
        clear_in_water(this);
        return;
    }
    if !floating && lt(x(imm), x(1.0)) {
        if !gt(x(imm), x(f32::from_bits(0x3F19_999A))) {
            clear_in_water(this);
            return;
        }
        let any_full = (0..4).any(|k| !x(rd::<f32>(this, 0x7D4 + 4 * k)).fcom(x(1.0)).test_parity(0x44));
        if !any_full {
            clear_in_water(this);
            return;
        }
    }
    wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) | 0x40);
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x100);
    if lt(x(rd::<f32>(this, 0x4C)), x(f32::from_bits(0xBDCC_CCCD))) {
        wr::<u32>(this, 0x4C, 0xBDCC_CCCD);
    }
    let mass = rd::<f32>(this, 0x8C);
    if lt(x(mass).fmul_m(f32::from_bits(0x3BD1_B718)), x(rd::<f32>(this, 0xA0))) {
        wr::<f32>(this, 0xA0, f32_of(x(mass).fmul_m(f32::from_bits(0x3706_37BE)).fsubr_m(rd::<f32>(this, 0xA0))));
    }
    if lt(x(rd::<f32>(this, 0xA0)), x(rd::<f32>(this, 0x8C)).fmul_m(0.008)) {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() {
        occupant(this, driver, true);
    }
    let mut i = 0u32;
    while i < u32::from(rd::<u8>(this, 0x488)) {
        let p = rd::<GPtr<u8>>(this, 0x464 + 4 * i as isize);
        if !p.is_null() {
            occupant(this, p, false);
        }
        i += 1;
    }
}

orig! {
    /// `operator*(float, const CVector&)` (0x40FE90, hidden result pointer)
    fn vector_scale(out: GPtr<u8>, k: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, hidden result pointer)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVehicle::ProcessControlInputs(uint8 pad)` (virtual, vtable +0x60)
    fn process_control_inputs_virtual(v: GPtr<u8>, pad: u32) = thiscall vslot 0x60 => unported;
    /// `std::min<float>` (0x404330, ST0)
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => unported;
    /// `std::max<float>` (0x420800, ST0)
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, ST0)
    fn get_fat_and_muscle_modifier(a: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CEntity::PruneReferences()` (0x571A90)
    fn prune_references(e: GPtr<u8>) = thiscall 0x571A90 => unported;
    /// `CVehicle::DoDriveByShootings()` (0x741FD0)
    fn do_drive_by_shootings(v: GPtr<u8>) = thiscall 0x741FD0 => unported;
    /// `CBike::DoSoftGroundResistance(uint32&)` (0x6B6D40)
    fn do_soft_ground_resistance(b: GPtr<u8>, flags: GPtr<u32>) = thiscall 0x6B6D40 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::CarGunJustDown()` (0x53FFE0), the result in AX
    fn car_gun_just_down(p: GPtr<u8>) -> u32 = thiscall 0x53FFE0 => unported;
    /// `CVehicle::ActivateBomb()` (0x6D24F0)
    fn activate_bomb(v: GPtr<u8>) = thiscall 0x6D24F0 => unported;
    /// `CCarAI::UpdateCarAI(CVehicle*)` (0x41DA30)
    fn update_car_ai(v: GPtr<u8>) = cdecl 0x41DA30 => unported;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(p: GPtr<u8>) = thiscall 0x5485E0 => unported;
    /// `CCarCtrl::UpdateCarOnRails(CVehicle*)` (0x436540)
    fn update_car_on_rails(v: GPtr<u8>) = cdecl 0x436540 => unported;
    /// `cTransmission::CalculateGearForSimpleCar(float speed, uint8& gear)` (0x6D0530)
    fn calculate_gear_for_simple_car(t: GPtr<u8>, speed: f32, gear: GPtr<u8>) = thiscall 0x6D0530 => unported;
    /// `CVehicle::ProcessWheelRotation(tWheelState, const CVector&, const CVector&, float)` (0x6D1230, ST0)
    fn process_wheel_rotation(v: GPtr<u8>, state: u32, fwd: GPtr<u8>, speed: GPtr<u8>, radius: f32) -> f64 = thiscall 0x6D1230 => unported;
    /// `CVehicle` virtual +0xD8 (`PlayCarHorn`)
    fn play_car_horn_virtual(v: GPtr<u8>) = thiscall vslot 0xD8 => unported;
    /// `CCarCtrl::SteerAICarWithPhysics(CVehicle*)` (0x437C20)
    fn steer_ai_car_with_physics(v: GPtr<u8>) = cdecl 0x437C20 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
}

/// `CBike::ProcessAI(uint32& extraHandlingFlags)` (0x6BC930, `ret 4`).
///
/// Clears +0x3DB bits 0/1. A non-negative +0x424 whose byte at 0x97D6C0 is 0 adds 2 to the flags
/// and returns false. Then by status (+0x36 >> 3): player (0): flags += 2, +0x614 bit 3 cleared;
/// unless the player ped's +0x530 is 0x3D or 0x3A: `ProcessControlInputs(0/1)` (vslot 0x60) for a
/// driver that is player 0/1; the centre of mass y (+0xA8) = bike handling lean factor (+0xC
/// leaning back / +4 forward) * lean (+0x650) + handling +0x18; unless (brake/handbrake, or brake
/// sign for forward lean) with +0x804 set, a wheelie torque `ApplyTurnForce(up * -(timeStep *
/// fatMuscle(11) * ((max(min(0.1, |v|)/0.1, ctl) + ctl) * (bh * turnMass * lean * min(0.1, |v|)) *
/// 0.5)), centreOfMass + fwd)` with ctl the gas (+0x49C) or brake (+0x4A0); `PruneReferences`,
/// drive-bys for the player status, `DoSoftGroundResistance(flags)`. All player paths end with
/// the bomb (`GetPad(0)->CarGunJustDown()` → `ActivateBomb`) and return true. Simple (1): flags
/// += 2. Physics (2): car AI, `CPhysical::ProcessControl`, rails, gear (speed +0x3C8 * 0.02),
/// +0x804/+0x805 = 2 (old +0x805 to +0x806), wheel rotations +0x750/+0x754 += `ProcessWheelRotation
/// (0, fwd, moveSpeed, wheelScale/2) * timeStep`, horn, +0x514 decremented, +0x614 bit 7 and +0x42B
/// bits 0/5 cleared. AI (3/12): car AI, steering, horn, flags += 2, +0x614 bit 7 cleared, a parked
/// bike (+0x42A bit 3) brakes with handbrake, else +0x614 bit 3 cleared. Abandoned (4): brake 0,
/// handbrake when |v|^2 < 0.01 or +0x614 bit 4, gas 0, +0x514 0, flags += 2 with an occupant or
/// +0x42A bit 3 (unless +0x614 bit 4), +0x654/+0x658 0, parked brakes. Wrecked-like (9): handbrake
/// and brake 1 when nearly still, else cleared, steer/gas/+0x514 0, flags += 2. 5: handbrake, brake
/// 0.05, the rest 0. Others: nothing. Returns true.
#[hook(addr = 0x6BC930, conv = thiscall, name = "CBike::ProcessAI", this = "CBike", expect = "83 EC 30 53 55 56 8B F1",
       route = "L2", hazards = "stack-residue",
       recs = "driving",
       note = "the original keeps the wheelie torque / the time step in its own flags-pointer argument slot (it holds the pointer in ebx)")]
pub fn bike_process_ai(this: GPtr<u8>, flags: GPtr<u32>) -> u8 {
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    wr::<u8>(this, 0x3DB, rd::<u8>(this, 0x3DB) & 0xFC);
    let idx = rd::<i8>(this, 0x424);
    if idx >= 0 && g::<u8>((0x97_D6C0 + idx as i32) as usize).read() == 0 {
        flags.write(flags.read().wrapping_add(2));
        return 0;
    }
    let add2 = || flags.write(flags.read().wrapping_add(2));
    match rd::<u8>(this, 0x36) >> 3 {
        0 => {
            add2();
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0xF7);
            let w = rd::<i32>(find_player_ped(-1), 0x530);
            let w2 = if w == 0x3D { w } else { rd::<i32>(find_player_ped(-1), 0x530) };
            if w != 0x3D && w2 != 0x3A {
                player_ai(this, flags);
            }
            if car_gun_just_down(get_pad(0)) as u16 != 0 {
                activate_bomb(this);
            }
            1
        }
        1 => {
            add2();
            1
        }
        2 => {
            update_car_ai(this);
            physical_process_control(this);
            update_car_on_rails(this);
            let speed = f32_of(x(rd::<f32>(this, 0x3C8)).fmul_m(f32::from_bits(0x3CA3_D70A)));
            let old = rd::<u8>(this, 0x805);
            wr::<u8>(this, 0x806, old);
            wr::<u8>(this, 0x804, 2);
            wr::<u8>(this, 0x805, 2);
            calculate_gear_for_simple_car(rd::<GPtr<u8>>(this, 0x384).byte_add(0x2C), speed, this.byte_add(0x4B4));
            for (k, off) in [(0x40, 0x750), (0x44, 0x754)] {
                let ts = g::<f32>(0xB7_CB5C).read();
                let r = f32_of(x(rd::<f32>(mi, k)).fmul_m(0.5));
                let fwd = rd::<GPtr<u8>>(this, 0x14).byte_add(0x10);
                let w = X87::from_f64_exact(process_wheel_rotation(this, 0, fwd, this.byte_add(0x44), r));
                wr::<f32>(this, off, f32_of(w.fmul_m(ts).fadd_m(rd::<f32>(this, off))));
            }
            if rd::<u8>(this, 0x3DB) & 3 != 0 {
                play_car_horn_virtual(this);
            }
            let t = rd::<u32>(this, 0x514);
            if t > 0 {
                wr::<u32>(this, 0x514, t - 1);
            }
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0x7F);
            wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
            1
        }
        3 | 12 => {
            update_car_ai(this);
            steer_ai_car_with_physics(this);
            if rd::<u8>(this, 0x3DB) & 3 != 0 {
                play_car_horn_virtual(this);
            }
            add2();
            let b614 = rd::<u8>(this, 0x614) & 0x7F;
            wr::<u8>(this, 0x614, b614);
            if rd::<u8>(this, 0x42A) & 8 != 0 {
                wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
                wr::<u32>(this, 0x49C, 0);
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
            } else {
                wr::<u8>(this, 0x614, b614 & 0xF7);
            }
            1
        }
        4 => {
            wr::<u32>(this, 0x4A0, 0);
            let still = !X87::from_f64_exact(vector_magnitude_sqr(this.byte_add(0x44))).fcom(x(0.01)).test_parity(0x05);
            let hb = if still || rd::<u8>(this, 0x614) & 0x10 != 0 { rd::<u8>(this, 0x428) | 0x20 } else { rd::<u8>(this, 0x428) & 0xDF };
            wr::<u8>(this, 0x428, hb);
            wr::<u32>(this, 0x49C, 0);
            wr::<u32>(this, 0x514, 0);
            let occupied = rd::<u32>(this, 0x460) != 0 || rd::<u32>(this, 0x464) != 0 || rd::<u8>(this, 0x42A) & 8 != 0;
            if occupied && rd::<u8>(this, 0x614) & 0x10 == 0 {
                add2();
            }
            wr::<u32>(this, 0x654, 0);
            wr::<u32>(this, 0x658, 0);
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0x7F);
            if rd::<u8>(this, 0x42A) & 8 != 0 {
                wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
                wr::<u32>(this, 0x49C, 0);
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
            }
            1
        }
        9 => {
            if !X87::from_f64_exact(vector_magnitude_sqr(this.byte_add(0x44))).fcom(x(0.01)).test_parity(0x05) {
                wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
            } else {
                wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
                wr::<u32>(this, 0x4A0, 0);
            }
            wr::<u32>(this, 0x494, 0);
            wr::<u32>(this, 0x49C, 0);
            wr::<u32>(this, 0x514, 0);
            add2();
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0x7F);
            1
        }
        5 => {
            let hb = rd::<u8>(this, 0x428) | 0x20;
            let b614 = rd::<u8>(this, 0x614) & 0x7F;
            wr::<u32>(this, 0x4A0, 0x3D4C_CCCD);
            wr::<u8>(this, 0x428, hb);
            wr::<u32>(this, 0x494, 0);
            wr::<u32>(this, 0x49C, 0);
            wr::<u32>(this, 0x514, 0);
            wr::<u8>(this, 0x614, b614);
            wr::<u32>(this, 0x654, 0);
            wr::<u32>(this, 0x658, 0);
            1
        }
        _ => 1,
    }
}

/// The player part of `CBike::ProcessAI` between the weapon check and the bomb (0x6BC9DB..0x6BCC1E).
fn player_ai(this: GPtr<u8>, flags: GPtr<u32>) {
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() {
        if g::<GPtr<u8>>(0xB7_CD98).read() == driver {
            process_control_inputs_virtual(this, 0);
        } else if g::<GPtr<u8>>(0xB7_CF28).read() == driver {
            process_control_inputs_virtual(this, 1);
        }
    }
    let h = rd::<GPtr<u8>>(this, 0x384);
    let bh = rd::<GPtr<u8>>(this, 0x63C);
    let lean = rd::<f32>(this, 0x650);
    let back = !x(lean).fcom(x(0.0)).test_parity(0x05);
    let (lean_k, ctl_off, torque_k) = if back { (0xC, 0x49C, 0x10) } else { (4, 0x4A0, 8) };
    wr::<f32>(this, 0xA8, f32_of(x(rd::<f32>(bh, lean_k)).fmul_m(lean).fadd_m(rd::<f32>(h, 0x18))));
    let skip_check = if back {
        x(rd::<f32>(this, 0x4A0)).fcom(x(0.0)).test_parity(0x44) || rd::<u8>(this, 0x428) & 0x20 != 0
    } else {
        !lt(x(rd::<f32>(this, 0x4A0)), x(0.0))
    };
    if !(skip_check && rd::<u8>(this, 0x804) != 0) {
        let tenth = f32::from_bits(0x3DCC_CCCD);
        let m = f32_of(X87::from_f64_exact(vector_magnitude(this.byte_add(0x44))));
        let mn = f32_of(X87::from_f64_exact(min_f(tenth, m)));
        let ctl = rd::<f32>(this, ctl_off);
        let r = f32_of(x(mn).fdiv_m(tenth));
        let g2 = X87::from_f64_exact(max_f(r, ctl)).fadd_m(rd::<f32>(this, ctl_off));
        let a = x(rd::<f32>(bh, torque_k)).fmul_m(rd::<f32>(this, 0x90)).fmul_m(rd::<f32>(this, 0x650)).fmul_m(mn);
        let k = f32_of(g2.fmul(a).fmul_m(0.5));
        let torque = f32_of(X87::from_f64_exact(get_fat_and_muscle_modifier(11)).fmul_m(k));
        let mat = rd::<GPtr<u8>>(this, 0x14);
        let mut point = [0f32; 3];
        let pp = GPtr::from_raw(point.as_mut_ptr()).cast::<u8>();
        vector_add(pp, this.byte_add(0xA4), mat.byte_add(0x10));
        let s = f32_of(x(g::<f32>(0xB7_CB5C).read()).fmul_m(torque).fchs());
        let mut force = [0f32; 3];
        let fp = GPtr::from_raw(force.as_mut_ptr()).cast::<u8>();
        vector_scale(fp, s, mat.byte_add(0x20));
        apply_turn_force(
            this,
            CVector::new(rd::<f32>(fp, 0), rd::<f32>(fp, 4), rd::<f32>(fp, 8)),
            CVector::new(rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)),
        );
    }
    prune_references(this);
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        do_drive_by_shootings(this);
    }
    do_soft_ground_resistance(this, flags);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_AUTOMOBILE_PROCESS_BUOYANCY,
    HOOK_BIKE_PROCESS_AI,
];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
