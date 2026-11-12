//! PORT-VEH-3 (batch `veh3-3`, SA:MP-executed rows): the sand / soft-ground drag of cars and bikes
//! (`DoSoftGroundResistance`). Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes
//! win: the x87 operand order, the f32 stores of the intermediate vectors and the argument slot used
//! as a temporary are reproduced).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `cSurfaceInfos::GetAdhesionGroup(uint32)` (0x55E5C0) on `g_surfaceInfos` (0xB79538)
    fn get_adhesion_group(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E5C0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
}

/// Some wheel (ratio at `ratio + 4k` below 1.0, i.e. touching) stands on a surface (byte at
/// `surface + 0x2C k`) that `pred` accepts; the wheels are tested in order.
fn any_wheel(this: GPtr<u8>, ratio: isize, surface: isize, mut pred: impl FnMut(u8) -> bool) -> bool {
    (0..4isize).any(|k| {
        !x(rd::<f32>(this, ratio + 4 * k)).fcom(x(1.0)).test_parity(0x05) && pred(rd::<u8>(this, surface + 0x2C * k))
    })
}

fn is_sand(s: u8) -> bool {
    get_adhesion_group(GPtr::from_addr(0xB7_9538), u32::from(s)) == 4
}

/// RC Bandit (441) and Rhino (432) are exempt.
fn exempt(this: GPtr<u8>) -> bool {
    let m = rd::<u16>(this, 0x22);
    m == 0x1B9 || m == 0x1B0
}

/// `(ms.z * v.z + ms.y * v.y) + ms.x * v.x` with the matrix row at `row`.
fn dot_row(this: GPtr<u8>, ms: [X87; 3], row: isize) -> X87 {
    let m = rd::<GPtr<u8>>(this, 0x14);
    ms[2]
        .fmul_m(rd::<f32>(m, row + 8))
        .fadd(ms[1].fmul_m(rd::<f32>(m, row + 4)))
        .fadd(ms[0].fmul_m(rd::<f32>(m, row)))
}

fn move_speed(this: GPtr<u8>) -> [X87; 3] {
    [x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C))]
}

/// The "mud" drag (surface 178): the horizontal move speed (the part along up.z removed) times
/// `-(timestep * mass * 0.003)` (the factor rounded to f32 through the argument slot).
fn mud_drag(this: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let ms = move_speed(this);
    let d = dot_row(this, ms, 0x20);
    let dy = d.fmul_m(rd::<f32>(m, 0x24)).to_f32();
    let dz = d.fmul_m(rd::<f32>(m, 0x28)).to_f32();
    let hx = d.fmul_m(rd::<f32>(m, 0x20)).fsubr_m(rd::<f32>(this, 0x44));
    let hy = x(rd::<f32>(this, 0x48)).fsub_m(dy);
    let hz = x(rd::<f32>(this, 0x4C)).fsub_m(dz).to_f32();
    let k = x(g::<f32>(0xB7_CB5C).read())
        .fmul_m(rd::<f32>(this, 0x8C))
        .fmul_m(f32::from_bits(0x3B44_9BA6))
        .fchs()
        .to_f32();
    let f = CVector::new(hx.fmul_m(k).to_f32(), hy.fmul_m(k).to_f32(), x(hz).fmul_m(k).to_f32());
    apply_move_force(this, f);
}

/// `CAutomobile::DoSoftGroundResistance(uint32& flags)` (0x6A4AF0): on sand (adhesion group 4 under
/// a touching wheel), except RC Bandit / Rhino, the horizontal speed `h` (move speed minus its part
/// along up) is dragged by `F = 0.005 * max(1 - 2 |h| (0.3-scaled), 0.2)` with the direction
/// normalised when `|h| > 0.3` (else `F = 0.005` and +0x868 bit 6), x0.3 / x0.6 for the handling
/// flags 0x200000 / 0x100000, x(1.2 - sandstorm) when the sandstorm (0xC81308) exceeds 0.2; the force
/// `h * -(F * timestep * mass)` and flag 8 in `flags`. Otherwise on surface 178 the mud drag.
#[hook(addr = 0x6A4AF0, conv = thiscall, name = "CAutomobile::DoSoftGroundResistance", this = "CAutomobile",
       expect = "83 EC 28 56 8B F1",
       unverified = "veh3-3", route = "L2", hazards = "x87-transcendental",
       recs = "driving", note = "fsqrt; the mud path keeps its factor in the argument slot")]
pub fn automobile_do_soft_ground_resistance(this: GPtr<u8>, flags: GPtr<u32>) {
    if !any_wheel(this, 0x7D4, 0x747, is_sand) {
        if any_wheel(this, 0x7D4, 0x747, |s| s == 0xB2) && !exempt(this) {
            mud_drag(this);
        }
        return;
    }
    // the exempt models fall through to the mud test, which exempts them again
    if exempt(this) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let ms = move_speed(this);
    let d = dot_row(this, ms, 0x20);
    let dx = d.fmul_m(rd::<f32>(m, 0x20));
    let dy = d.fmul_m(rd::<f32>(m, 0x24)).to_f32();
    let dz = d.fmul_m(rd::<f32>(m, 0x28)).to_f32();
    let mut h = [
        ms[0].fsub(dx).to_f32(),
        x(rd::<f32>(this, 0x48)).fsub_m(dy).to_f32(),
        0.0,
    ];
    let hz80 = x(rd::<f32>(this, 0x4C)).fsub_m(dz);
    h[2] = hz80.to_f32();
    let sum = hz80.fmul_m(h[2]).fadd(x(h[1]).fmul_m(h[1])).fadd(x(h[0]).fmul_m(h[0]));
    let c = f32::from_bits(0x3E99_999A);
    let mut f = if !sum.fcom(x(c).fmul_m(c)).test_any(0x41) {
        let s = sum.fsqrt();
        let hx = x(h[0]).fmul_m(c);
        let hy = x(h[1]).fmul_m(c);
        let hz = x(h[2]).fmul_m(c).to_f32();
        let inv = x(1.0).fdiv(s);
        let inv32 = inv.to_f32();
        h[0] = inv.fmul(hx).to_f32();
        h[1] = hy.fmul_m(inv32).to_f32();
        h[2] = x(hz).fmul_m(inv32).to_f32();
        let mut v = s.fadd(s).fsubr_m(1.0);
        let lo = f32::from_bits(0x3E4C_CCCD);
        if !x(lo).fcom(v).test_any(0x41) {
            v = x(lo);
        }
        v.fmul_m(f32::from_bits(0x3BA3_D70A))
    } else {
        wr::<u8>(this, 0x868, rd::<u8>(this, 0x868) | 0x40);
        x(f32::from_bits(0x3BA3_D70A))
    };
    let hf = rd::<u32>(this, 0x38C);
    if hf & 0x20_0000 != 0 {
        f = f.fmul_m(f32::from_bits(0x3E99_999A));
    } else if hf & 0x10_0000 != 0 {
        f = f.fmul_m(f32::from_bits(0x3F19_999A));
    }
    let storm = g::<f32>(0xC8_1308).read();
    if !x(storm).fcom(x(f32::from_bits(0x3E4C_CCCD))).test_any(0x41) {
        f = f.fmul(x(f32::from_bits(0x3F99_999A)).fsub_m(storm));
    }
    let tm = x(g::<f32>(0xB7_CB5C).read()).fmul_m(rd::<f32>(this, 0x8C));
    let k = f.fmul(tm).fchs();
    let force = CVector::new(x(h[0]).fmul(k).to_f32(), x(h[1]).fmul(k).to_f32(), x(h[2]).fmul(k).to_f32());
    apply_move_force(this, force);
    flags.write(flags.read() | 8);
}

/// The bike's mud drag (surface 178): as the car's, wheel ratios at +0x710 and surfaces at +0x683.
fn bike_mud(this: GPtr<u8>) {
    mud_drag(this);
}

/// `CBike::DoSoftGroundResistance(uint32& flags)` (0x6B6D40): on sand (adhesion group 4 under a
/// touching wheel) the horizontal speed `h` (move speed minus its part along up); with the gas pedal
/// (+0x49C) above 0.3 `flags += 4` when `|h|^2 < 0.09` and the part along forward is removed too; the
/// force `h * -(timestep * mass * 0.02)` (x with the 80-bit factor, y/z with the f32 one stored in
/// the argument slot). Otherwise on surface 178 the mud drag.
#[hook(addr = 0x6B6D40, conv = thiscall, name = "CBike::DoSoftGroundResistance", this = "CBike",
       expect = "83 EC 18 56 8B F1",
       unverified = "veh3-3", route = "L2", hazards = "none",
       recs = "driving", note = "the factor is kept in the argument slot")]
pub fn bike_do_soft_ground_resistance(this: GPtr<u8>, flags: GPtr<u32>) {
    if !any_wheel(this, 0x710, 0x683, is_sand) {
        if any_wheel(this, 0x710, 0x683, |s| s == 0xB2) {
            bike_mud(this);
        }
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let ms = move_speed(this);
    let d = dot_row(this, ms, 0x20);
    let l4 = d.fmul_m(rd::<f32>(m, 0x20)).to_f32();
    let l8 = d.fmul_m(rd::<f32>(m, 0x24)).to_f32();
    let lc = d.fmul_m(rd::<f32>(m, 0x28)).to_f32();
    let mut hx = x(rd::<f32>(this, 0x44)).fsub_m(l4);
    let mut hy = x(rd::<f32>(this, 0x48)).fsub_m(l8);
    let mut hz = x(rd::<f32>(this, 0x4C)).fsub_m(lc).to_f32();
    if !x(rd::<f32>(this, 0x49C)).fcom(x(f32::from_bits(0x3E99_999A))).test_any(0x41) {
        let sum = x(hz).fmul_m(hz).fadd(hy.fmul(hy)).fadd(hx.fmul(hx));
        let c = f32::from_bits(0x3E99_999A);
        if !x(c).fmul_m(c).fcom(sum).test_any(0x41) {
            flags.write(flags.read().wrapping_add(4));
        }
        let d2 = x(hz)
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(hy.fmul_m(rd::<f32>(m, 0x14)))
            .fadd(hx.fmul_m(rd::<f32>(m, 0x10)));
        let l10 = d2.fmul_m(rd::<f32>(m, 0x10)).to_f32();
        let l14 = d2.fmul_m(rd::<f32>(m, 0x14)).to_f32();
        let dz = d2.fmul_m(rd::<f32>(m, 0x18));
        hx = hx.fsub_m(l10);
        hy = hy.fsub_m(l14);
        hz = x(hz).fsub(dz).to_f32();
    }
    let k = x(g::<f32>(0xB7_CB5C).read())
        .fmul_m(rd::<f32>(this, 0x8C))
        .fmul_m(f32::from_bits(0x3CA3_D70A))
        .fchs();
    let k32 = k.to_f32();
    let force = CVector::new(k.fmul(hx).to_f32(), hy.fmul_m(k32).to_f32(), x(hz).fmul_m(k32).to_f32());
    apply_move_force(this, force);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_DO_SOFT_GROUND_RESISTANCE, HOOK_BIKE_DO_SOFT_GROUND_RESISTANCE];
