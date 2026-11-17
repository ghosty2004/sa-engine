//! FRONTIER-2 turn 10, lane B (batch `frontier2-b23`): the vehicles' guns, `CVehicle::FireFixedMachineGuns`
//! (0x73DF00). Ported from the 1.0 US bytes with docs/re/specs/VEH/CVehicle.md as reading aid (gta-reversed has no
//! body).

use sa_core::abi::HookReg;
use sa_core::bindings::matrix_mul_vec;
use sa_core::field::{gf, gr32, ptr_of as vp};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, rand_at, rd, wr, x, xi};
use crate::mvpd_v1::read_vec;

use crate::bindings::weapon_fire_one_instant_hit_round;
use sa_world::bindings::report_weapon_event;

orig! {
    /// `CWeapon::DoTankDoomAiming(CEntity* vehicle, CEntity* owner, CVector* start, CVector* end)` (0x73D1E0, cdecl)
    fn do_tank_doom_aiming(v: GPtr<u8>, owner: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) = cdecl 0x73D1E0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7_CB84;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6_BC90;

/// `((rand() & 0xFF) - 128)` as the bytes load it (`fild` of the int).
fn spread(site: u32) -> X87 {
    xi((rand_at(site) & 0xFF) - 0x80)
}

/// One fixed gun: `start = matrix * offset`, `end = start + (dx, dy, 0)` (the caller's sum), the rand() spread
/// (x from the second call, y from the first, z from the third; x / y f32 offsets, z unrounded), the aim
/// correction and the shot.
fn fire_gun(this: GPtr<u8>, offset: [f32; 3], end_xy: impl Fn(CVector) -> (f32, f32), sites: [u32; 3]) {
    let mut v = CVector::new(offset[0], offset[1], offset[2]);
    let mut out = CVector::default();
    let mut start = read_vec(matrix_mul_vec(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(&mut v)));
    let (ex, ey) = end_xy(start);
    let r1 = rand_at(sites[0]);
    let r2 = rand_at(sites[1]);
    let k = gf(0x85_9F00);
    let ox = f32_of(xi((r2 & 0xFF) - 0x80).fmul_m(k));
    let oy = f32_of(xi((r1 & 0xFF) - 0x80).fmul_m(k));
    let oz = spread(sites[2]).fmul_m(gf(0x85_8B38));
    let mut end = CVector::new(f32_of(x(ox).fadd_m(ex)), f32_of(x(ey).fadd_m(oy)), f32_of(x(start.z).fadd(oz)));
    do_tank_doom_aiming(this, rd::<GPtr<u8>>(this, 0x460), vp(&mut start), vp(&mut end));
    weapon_fire_one_instant_hit_round(vp(&mut start).cast(), vp(&mut end).cast(), 15);
}

/// `CVehicle::FireFixedMachineGuns()` (0x73DF00, thiscall, plain `ret`).
///
/// Not before `m_nGunFiringTime (+0x4D4) + 150` (unsigned). The 2D forward direction `(fx, fy) / max(|(fx, fy)|,
/// 0.1)` (stored f32) times 60 is the reach: the right gun (2.0, 2.5, 1.0) adds the unrounded products, the left
/// gun (-2.0, 2.5, 1.0) their f32 stores. Each gun: three rand() spreads (x / y * 0.015, z * 0.02),
/// `CWeapon::DoTankDoomAiming(this, driver, start, end)` and `FireOneInstantHitRound(start, end, 15)`. Then
/// `ReportWeaponEvent(0x95, 31, this)`; the clip (+0x4AB) empties after 20 shots: 20 again and a 1400 ms pause.
#[hook(addr = 0x73DF00, conv = thiscall, name = "CVehicle::FireFixedMachineGuns", this = "CVehicle",
       expect = "8B 15 84 CB B7 00", unverified = "frontier2-b23", route = "L2", hazards = "none",
       note = "consumes six rand() values")]
pub fn fire_fixed_machine_guns(this: GPtr<u8>) {
    let now = gr32(TIME_MS);
    if now <= rd::<u32>(this, 0x4D4).wrapping_add(0x96) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let (fx, fy) = (x(rd::<f32>(m, 0x10)), x(rd::<f32>(m, 0x14)));
    let mut len = fy.fmul(fy).fadd(fx.fmul(fx)).fsqrt();
    let tenth = x(gf(0x85_8B1C));
    // fld 0.1; fcomp st(1); `test ah, 0x41; jne` keeps the length when 0.1 <= it (or NaN)
    if !tenth.fcom(len).test_any(0x41) {
        len = tenth;
    }
    let inv = len.fdivr_m(1.0);
    wr::<u32>(this, 0x4D4, now);
    let dx = f32_of(inv.fmul(fx));
    let dy = f32_of(inv.fmul(fy));
    let sixty = gf(0x85_8B34);
    let dx60 = x(dx).fmul_m(sixty);
    let dy60 = x(dy).fmul_m(sixty);
    let (dx60f, dy60f) = (f32_of(dx60), f32_of(dy60));
    fire_gun(
        this,
        [2.0, 2.5, 1.0],
        |s| (f32_of(dx60.fadd_m(s.x)), f32_of(x(s.y).fadd(dy60))),
        [0x73_E015, 0x73_E01C, 0x73_E05B],
    );
    fire_gun(
        this,
        [-2.0, 2.5, 1.0],
        |s| (f32_of(x(dx60f).fadd_m(s.x)), f32_of(x(dy60f).fadd_m(s.y))),
        [0x73_E153, 0x73_E15A, 0x73_E199],
    );
    report_weapon_event(GPtr::from_addr(AUDIO_ENGINE), 0x95, 0x1F, this.addr() as u32);
    let clip = rd::<u8>(this, 0x4AB).wrapping_sub(1);
    wr::<u8>(this, 0x4AB, clip);
    if clip == 0 {
        wr::<u8>(this, 0x4AB, 0x14);
        wr::<u32>(this, 0x4D4, gr32(TIME_MS).wrapping_add(0x578));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIRE_FIXED_MACHINE_GUNS];
