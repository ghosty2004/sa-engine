//! WRITER-MVP-D (helper VEH, batch `mvpd-veh-3`): `CBike` water handling. Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, g, rd, wr, x};

orig! {
    /// `cBuoyancy::ProcessBuoyancy(CPhysical*, float, CVector*, CVector*)` (0x6C3EF0) on `mod_Buoyancy` (0xC1C890)
    fn buoyancy_process(b: GPtr<u8>, e: GPtr<u8>, buoyancy: u32, centre: GPtr<u8>, force: GPtr<u8>) -> u8 = thiscall 0x6C3EF0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(p: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(p: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::HandlePlayerBreath(bool, float)` (0x60A8D0), the float as bits
    fn player_handle_breath(ped: GPtr<u8>, b: u32, f: u32) = thiscall 0x60A8D0 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(CVehicle*, CVector*, CVector*, float, float, uint8, uint8, int32, CPed*, bool, bool)` (0x4AFCF0)
    fn kob_ctor(e: GPtr<u8>, veh: GPtr<u8>, speed: GPtr<u8>, impact: GPtr<u8>, dmg: u32, a5: u32, kind: u32, dir: u32, time: u32, ped: u32, driver: u32, force: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CEventKnockOffBike::~CEventKnockOffBike()` (0x4AFE70)
    fn kob_dtor(e: GPtr<u8>) = thiscall 0x4AFE70 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)` (0x4AD3F0)
    fn damage_calc_ctor(c: GPtr<u8>, e: GPtr<u8>, dmg: u32, wt: u32, piece: u32, speak: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420)
    fn damage_calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32, eWeaponType, ePedPieceTypes, uint8, bool, bool)` (0x4AD830)
    fn event_damage_ctor(e: GPtr<u8>, src: GPtr<u8>, time: u32, wt: u32, piece: u32, dir: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0)
    fn event_damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool)` (0x4B5AC0)
    fn damage_calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) = thiscall 0x4B5AC0 => unported;
}

const TIME_STEP: usize = 0xB7_CB5C;

/// One of the four +0x710.. compressions is below 1.0 (`test ah, 5; jnp`).
fn any_compressed(this: GPtr<u8>) -> bool {
    (0..4).any(|k| !x(rd::<f32>(this, 0x710 + 4 * k)).fcom(x(1.0)).test_parity(0x05))
}

/// The per-occupant reaction of `CBike::ProcessBuoyancy` (driver: `is_driver` 1, the `+0x428` bit 4
/// cleared before the knock-off event is destroyed).
fn occupant_in_water(this: GPtr<u8>, ped: GPtr<u8>, is_driver: bool) {
    wr::<u32>(ped, 0x40, rd::<u32>(ped, 0x40) | 0x800_0000);
    if is_driver {
        if ped_is_player(ped) == 0 && rd::<u8>(this, 0x614) & 4 != 0 {
            return;
        }
        if ped_is_player(ped) != 0 {
            player_handle_breath(ped, 1, 0x3F80_0000);
        }
    } else {
        if ped_is_player(ped) != 0 {
            player_handle_breath(ped, 1, 0x3F80_0000);
        }
        if ped_is_player(ped) == 0 && rd::<u8>(this, 0x614) & 4 != 0 {
            return;
        }
    }
    let group = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68);
    if !any_compressed(this) {
        let mut ev = [0u32; 0x20];
        let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
        kob_ctor(
            evp,
            this,
            this.byte_add(0x44),
            this.byte_add(0xE0),
            rd::<u32>(this, 0xD8),
            0,
            0x35,
            0,
            0,
            0,
            u32::from(is_driver),
            0,
        );
        event_group_add(group(ped), evp, 0);
        if is_driver {
            wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
        }
        kob_dtor(evp);
        return;
    }
    if ped_is_player(ped) != 0 {
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

/// `CBike::ProcessBuoyancy()` (0x6B5FB0).
///
/// `mod_Buoyancy.ProcessBuoyancy(this, +0xA0, &centre, &force)`; false clears +0x42B bit 6 and the
/// in-water / touching-water flags (+0x40 bits 8, 27). Else touching water is set, `ApplyMoveForce
/// (force)`, `ApplyTurnForce(force, centre)`; `imm = force.z / (max(timeStep, 0.01) * mass * 0.008)`,
/// times `(mass * 0.008 / buoyancy) * 1.05` when `mass * 0.008 > buoyancy`, times 1.5 with +0x40
/// bit 0; move and turn speed are multiplied by `pow(max(1 - imm * 0.05, 0.5), timeStep)` (CRT
/// `_CIpow`). In water (imm > 0.8, or imm > 0.4 with a wheel compression exactly 1.0): +0x42B bit 6
/// and +0x40 bit 8 set, move z clamped to >= -0.1, then the driver and each passenger get the
/// touching-water ped flag, player breath, and (unless a non-player on a bike with +0x614 bit 2) a
/// `CEventKnockOffBike` (weapon 0x35) when no wheel is compressed, or a drowning `CEventDamage`
/// (piece 3, damage = the time step) for non-players; no driver clears +0x428 bit 4. Not in water
/// clears +0x40 bit 8 and +0x42B bit 6.
#[hook(addr = 0x6B5FB0, conv = thiscall, name = "CBike::ProcessBuoyancy", this = "CBike", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "mvpd-veh-3", route = "L2", hazards = "x87-transcendental",
       recs = "driving",
       note = "pow through orig::x87_binary(0x822130); event objects on the Rust stack (sizes assumed <= 0x80)")]
pub fn bike_process_buoyancy(this: GPtr<u8>) {
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
        return;
    }
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x800_0000);
    apply_move_force(this, force);
    apply_turn_force(this, force, centre);
    let ts = g::<f32>(TIME_STEP).read();
    let t = if x(0.01).fcom(x(ts)).test_any(0x41) { x(ts) } else { x(0.01) };
    let mut imm = f32_of(t.fmul_m(rd::<f32>(this, 0x8C)).fmul_m(0.008).fdivr_m(force.z));
    let lim = x(rd::<f32>(this, 0x8C)).fmul_m(0.008);
    if !lim.fcom(x(rd::<f32>(this, 0xA0))).test_any(0x41) {
        imm = f32_of(lim.fdiv_m(rd::<f32>(this, 0xA0)).fmul_m(imm).fmul_m(1.05));
    }
    if rd::<u8>(this, 0x40) & 1 != 0 {
        imm = f32_of(x(imm).fmul_m(1.5));
    }
    let k = x(imm).fmul_m(0.05).fsubr_m(1.0);
    let damp = if x(0.5).fcom(k).test_any(0x41) { f32_of(k) } else { 0.5 };
    let p = X87::from_f64_exact(sa_core::orig::x87_binary(0x82_2130, x(damp).to_f64(), x(ts).to_f64()));
    for off in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr::<f32>(this, off, f32_of(p.fmul_m(rd::<f32>(this, off))));
    }
    let in_water = if !x(imm).fcom(x(0.8)).test_any(0x41) {
        true
    } else if x(imm).fcom(x(0.4)).test_any(0x41) {
        false
    } else {
        (0..4).any(|k| !x(rd::<f32>(this, 0x710 + 4 * k)).fcom(x(1.0)).test_parity(0x44))
    };
    if !in_water {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FEFF);
        wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xBF);
        return;
    }
    wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) | 0x40);
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x100);
    if !x(rd::<f32>(this, 0x4C)).fcom(x(f32::from_bits(0xBDCC_CCCD))).test_parity(0x05) {
        wr::<u32>(this, 0x4C, 0xBDCC_CCCD);
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if driver.is_null() {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
    } else {
        occupant_in_water(this, driver, true);
    }
    let mut i = 0u32;
    while i < u32::from(rd::<u8>(this, 0x488)) {
        let p = rd::<GPtr<u8>>(this, 0x464 + 4 * i as isize);
        if !p.is_null() {
            occupant_in_water(this, p, false);
        }
        i += 1;
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_BIKE_PROCESS_BUOYANCY];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
