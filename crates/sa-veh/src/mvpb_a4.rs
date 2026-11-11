//! WRITER-MVP-B (helper A, batch `mvpb-veh-4`): vehicle functions on the MVP call path. Ported from
//! the 1.0 US bytes; gta-reversed and docs/re/specs/VEH as reading aids. Where they differ, the bytes
//! win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, rd, wr, x, xi};

orig! {
    /// `CStats::GetPercentageProgress()` (0x5591E0, ST0)
    fn get_percentage_progress() -> f64 = cdecl 0x5591E0 => unported;
    /// `CBike::DamageKnockOffRider(CVehicle*, float, uint16, CEntity*, const CVector&, const CVector&)` (0x6B5A10)
    fn damage_knock_off_rider(v: GPtr<u8>, intensity: u32, piece: u32, damager: GPtr<u8>, posn: GPtr<u8>, dir: GPtr<u8>) = cdecl 0x6B5A10 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30)
    fn player_set_wanted_level_no_drop(p: GPtr<u8>, level: i32) = thiscall 0x609F30 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0), the float as its bits
    fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: u32, a4: u32, a5: u32, a6: u32) = thiscall 0x5EFFE0 => unported;
}

fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// `a > b` (`test ah, 0x41` + `jne` not taken).
fn gt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// `a < b` (`test ah, 5` + `jp` not taken).
fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

fn fpv() -> GPtr<u8> {
    sa_world::world::common::find_player_vehicle(-1, 0)
}

/// `CBike::VehicleDamage(float intensity, uint16 component, CEntity*, CVector*, CVector*, eWeaponType)`
/// (0x6B8EC0, `ret 0x18`; docs/re/specs/VEH/CBike.md). Only collision damage: returns when
/// `intensity > 0` (f64 compare), `m_fDamageIntensity` (+0xD8) < 1 or `bCanBeDamaged` (+0x42A bit 5)
/// is clear. `d` = the raw intensity (halved for a player-status bike when the game progress is not
/// below 100); side stand cleared when d > 20; `DamageKnockOffRider(this, +0xD8, +0xF8, +0xDC, &+0xEC,
/// &+0xE0)`; a vehicle damager becomes the last damage entity (weapon 49, reference registered). Then
/// nothing more when collision-proof, or the damager is a building with `impact . up > 0.6`. When `d >
/// 25` and not wrecked: a law-enforcer bike hit by the moving player vehicle (not abandoned, not slower
/// than this, speed > 0.1) gives wanted level 1; `d2 = (d - 25) * handling +0xC8`; when > 0: the driver
/// says 0x42 for d2 > 5 against a driven vehicle (unless the player is this bike and the damager is a
/// mission vehicle); health -= d2 * (player bike: 1/6 or 0.5 by bTakeLessDamage; else 1/12, 2/3 when hit
/// by the player vehicle, or 0.25), clamped back to 1 when it crosses 1 from above. Finally below 250
/// health the engine catches fire once (+0x614 bit 6, `m_BlowUpTimer` = 0, `m_Damager` = the damager,
/// reference registered).
#[hook(addr = 0x6B8EC0, conv = thiscall, name = "CBike::VehicleDamage", this = "CBike", expect = "51 D9 44 24 08 56",
       unverified = "mvpb-veh-4", route = "L2", hazards = "heap-writes",
       note = "the original keeps d/d2 in its intensity argument slot (entry esp+4); the port uses a local")]
pub fn bike_vehicle_damage(this: GPtr<u8>, intensity: f32, _component: u32, _damager: GPtr<u8>, _coors: GPtr<u8>, _dir: GPtr<u8>, _weapon: u32) {
    if gt(x(intensity), x(0.0)) {
        return;
    }
    if lt(x(rd::<f32>(this, 0xD8)), x(1.0)) {
        return;
    }
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    let mut d = rd::<f32>(this, 0xD8);
    if rd::<u8>(this, 0x36) & 0xF8 == 0 && !lt(st0(get_percentage_progress()), x(100.0)) {
        d = f32_of(x(d).fmul_m(0.5));
    }
    let cl = rd::<u8>(this, 0x614);
    if cl & 0x10 != 0 && gt(x(d), x(20.0)) {
        wr::<u8>(this, 0x614, cl & 0xEF);
    }
    damage_knock_off_rider(
        this,
        rd::<u32>(this, 0xD8),
        u32::from(rd::<u16>(this, 0xF8)),
        rd::<GPtr<u8>>(this, 0xDC),
        this.byte_add(0xEC),
        this.byte_add(0xE0),
    );
    let dmg = rd::<GPtr<u8>>(this, 0xDC);
    if !dmg.is_null() && rd::<u8>(dmg, 0x36) & 7 == 2 {
        wr::<u8>(this, 0x508, 0x31);
        wr::<GPtr<u8>>(this, 0x50C, dmg);
        entity_register_reference(dmg, this.byte_add(0x50C));
    }
    if rd::<u32>(this, 0x40) & 0x10_0000 != 0 {
        return;
    }
    let dmg = rd::<GPtr<u8>>(this, 0xDC);
    if !dmg.is_null() && rd::<u8>(dmg, 0x36) & 7 == 1 {
        let up = rd::<GPtr<u8>>(this, 0x14).byte_add(0x20);
        if gt(st0(vector_dot_product(this.byte_add(0xE0), up)), x(f32::from_bits(0x3F19_999A))) {
            return;
        }
    }
    if gt(x(d), x(25.0)) && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        if rd::<u8>(this, 0x428) & 1 != 0
            && !fpv().is_null()
            && rd::<GPtr<u8>>(this, 0xDC) == fpv()
            && rd::<u8>(this, 0x36) & 0xF8 != 0x20
        {
            let pv = fpv();
            let mine = f32_of(st0(vector_magnitude(this.byte_add(0x44))));
            if !lt(st0(vector_magnitude(pv.byte_add(0x44))), x(mine))
                && gt(st0(vector_magnitude(fpv().byte_add(0x44))), x(f32::from_bits(0x3DCC_CCCD)))
            {
                player_set_wanted_level_no_drop(find_player_ped(-1), 1);
            }
        }
        let d2x = x(d).fsub_m(25.0).fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0xC8));
        let d2 = f32_of(d2x);
        if gt(d2x, x(0.0)) {
            if gt(x(d2), x(5.0)) && !rd::<GPtr<u8>>(this, 0x460).is_null() {
                let e = rd::<GPtr<u8>>(this, 0xDC);
                if !e.is_null()
                    && rd::<u8>(e, 0x36) & 7 == 2
                    && !(fpv() == this && rd::<u8>(e, 0x4A4) == 2)
                    && !rd::<GPtr<u8>>(e, 0x460).is_null()
                {
                    ped_say(rd::<GPtr<u8>>(this, 0x460), 0x42, 0, 0x3F80_0000, 0, 0, 0);
                }
            }
            let old = rd::<f32>(this, 0x4C0);
            let less = rd::<u8>(this, 0x429) & 0x80 != 0;
            let k = if fpv() == this {
                if less { x(d2).fmul_m(f32::from_bits(0x3E2A_AAAB)) } else { x(d2).fmul_m(0.5) }
            } else if less {
                x(d2).fmul_m(f32::from_bits(0x3DAA_AAAB))
            } else {
                let e = rd::<GPtr<u8>>(this, 0xDC);
                if !e.is_null() && e == fpv() {
                    x(d2).fmul_m(f32::from_bits(0x3F2A_AAAB))
                } else {
                    x(d2).fmul_m(0.25)
                }
            };
            wr::<f32>(this, 0x4C0, f32_of(k.fsubr_m(rd::<f32>(this, 0x4C0))));
            if x(rd::<f32>(this, 0x4C0)).fcom(x(1.0)).test_parity(0x41) {
                // greater or unordered: no clamp
            } else if gt(x(old), x(1.0)) {
                wr::<f32>(this, 0x4C0, 1.0);
            }
        }
    }
    if lt(x(rd::<f32>(this, 0x4C0)), x(250.0)) {
        let al = rd::<u8>(this, 0x614);
        if al & 0x40 == 0 {
            let e = rd::<GPtr<u8>>(this, 0xDC);
            wr::<u8>(this, 0x614, al | 0x40);
            wr::<u32>(this, 0x7BC, 0);
            wr::<GPtr<u8>>(this, 0x800, e);
            if !e.is_null() {
                entity_register_reference(e, this.byte_add(0x800));
            }
        }
    }
}

orig! {
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0) on `g_surfaceInfos` 0xB79538
    fn get_adhesion_group(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E5C0 => unported;
}

/// CRT `floor` (0x8219F0, catalogue-excluded) of the double the original stores with `fstp qword`.
fn x87_floor(v: X87) -> X87 {
    X87::from_f64_exact(sa_stream::modelinfo::vehicle::floor_f64(v.to_f64()))
}

/// `(lineLen[l] - suspLen[l]) / lineLen[l]` (+0x780 / +0x770).
fn compression(this: GPtr<u8>, l: isize) -> X87 {
    x(rd::<f32>(this, 0x780 + 4 * l))
        .fsub_m(rd::<f32>(this, 0x770 + 4 * l))
        .fdiv_m(rd::<f32>(this, 0x780 + 4 * l))
}

/// `CBike::DoBurstAndSoftGroundRatios()` (0x6B6950; docs/re/specs/VEH/CBike.md, checked against the
/// bytes). `spd = |moveSpeed . forward|` (z, y, x order) as f32; per wheel i (lines a, b = 0, 1 / 2, 3):
/// missing (+0x65C[i] == 2) -> both ratios (+0x710) 1; burst (1) -> `rand()` (site 0x6B69F8) decides
/// `ftol(R * ((u16) ftol(spd * 40) + 98)) < 100`, then both ratios `min1(t + ratio)` with `t` = 0.2 *
/// the compression of line a; otherwise, when a line with ratio < 1 is on rail track (surface B 0xB2,
/// colpoints +0x683 stride 0x2C): a wheel-rotation bump test (`per = 1 / (1.5 / (wheelSize * 0.5)
/// [* spd / 0.3 when spd > 0.3])`, `f1 = frac(per * rot)`, `f2 = frac((timeStep * angVel + rot) * per)`,
/// bump on a wrap) lowers both ratios by 0.3 * the compression of line a (floored at 0.2). Those
/// three cases clear the lines' soft flags (a rail-test miss does not). Then every soft line with ratio
/// < 1 on a sand adhesion group (4), model not 0x1B0: `ratio += k * (u * s)` with k = 0.25 / 0.1
/// (offroad 2) / 0.15 (offroad), `s = max(1 - spd / 0.3 * 0.7 - WetRoads * 0.7, 0.4)`, capped at 1.
#[hook(addr = 0x6B6950, conv = thiscall, name = "CBike::DoBurstAndSoftGroundRatios", this = "CBike", expect = "55 8B EC 83 E4 F8 83 EC 18",
       unverified = "mvpb-veh-4", route = "L2", hazards = "heap-writes")]
pub fn do_burst_and_soft_ground_ratios(this: GPtr<u8>) {
    let mi = crate::common::model_info(rd::<i16>(this, 0x22) as i32);
    let fwd = rd::<GPtr<u8>>(this, 0x14).byte_add(0x10);
    let spd = f32_of(
        x(rd::<f32>(this, 0x4C))
            .fmul_m(rd::<f32>(fwd, 8))
            .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(fwd, 4)))
            .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(fwd, 0)))
            .fabs(),
    );
    let mut soft = [1u8; 4];
    let ratio = |l: isize| rd::<f32>(this, 0x710 + 4 * l);
    let set_ratio = |l: isize, v: X87| wr::<f32>(this, 0x710 + 4 * l, f32_of(v));
    let one = x(1.0);
    for i in 0..2isize {
        let (a, b) = if i == 0 { (0, 1) } else { (2, 3) };
        let status = rd::<u8>(this, 0x65C + i);
        if status == 2 {
            set_ratio(a, one);
            set_ratio(b, one);
        } else if status == 1 {
            let r = crate::common::rand_frac(crate::common::rand_at(0x6B69F8));
            let n = crate::common::ftol32(x(spd).fmul_m(40.0));
            let m = (n as u32 & 0xFFFF) as i32 + 0x62;
            let q = crate::common::ftol32(r.fmul(xi(m)));
            if q < 100 {
                let t = compression(this, a).fmul_m(f32::from_bits(0x3E4C_CCCD));
                let v = t.fadd_m(ratio(a));
                set_ratio(a, if lt(v, one) { v } else { one });
                let v = t.fadd_m(ratio(b));
                set_ratio(b, if lt(v, one) { v } else { one });
            }
        } else {
            let rail = |l: isize| lt(x(ratio(l)), one) && rd::<u8>(this, 0x683 + l * 0x2C) == 0xB2;
            if !rail(a) && !rail(b) {
                continue;
            }
            let w = rd::<f32>(mi, if i == 0 { 0x40 } else { 0x44 });
            let mut v = x(w).fmul_m(0.5).fdivr_m(1.5);
            let c03 = f32::from_bits(0x3E99_999A);
            if gt(x(spd), x(c03)) {
                v = v.fmul(x(spd).fdiv_m(c03));
            }
            let per = f32_of(x(1.0).fdiv(v));
            let rot = rd::<f32>(this, 0x750 + 4 * i);
            let x1 = x(per).fmul_m(rot);
            let f1 = f32_of(x87_floor(x1).fsubr_m(f32_of(x1)));
            let x2 = x(g_f32(0xB7CB5C)).fmul_m(rd::<f32>(this, 0x758 + 4 * i)).fadd_m(rot).fmul_m(per);
            let f2 = x87_floor(x2).fsubr_m(f32_of(x2));
            let av = x(rd::<f32>(this, 0x758 + 4 * i));
            let bump = (gt(av, x(0.0)) && lt(f2, x(f1))) || (lt(av, x(0.0)) && gt(f2, x(f1)));
            if bump {
                let t = compression(this, a).fmul_m(f32::from_bits(0x3E99_999A));
                let lo = x(f32::from_bits(0x3E4C_CCCD));
                let v = x(ratio(a)).fsub(t);
                set_ratio(a, if gt(v, lo) { v } else { lo });
                let v = t.fsubr_m(ratio(b));
                set_ratio(b, if gt(v, lo) { v } else { lo });
            }
        }
        soft[a as usize] = 0;
        soft[b as usize] = 0;
    }
    for j in 0..4isize {
        if soft[j as usize] == 0 || !lt(x(ratio(j)), one) {
            continue;
        }
        let surf = u32::from(rd::<u8>(this, 0x683 + j * 0x2C));
        if get_adhesion_group(GPtr::from_addr(0xB79538), surf) != 4 || rd::<u16>(this, 0x22) == 0x1B0 {
            continue;
        }
        let flags = rd::<u32>(this, 0x38C);
        let k = if flags & 0x20_0000 != 0 {
            f32::from_bits(0x3DCC_CCCD)
        } else if flags & 0x10_0000 != 0 {
            f32::from_bits(0x3E19_999A)
        } else {
            0.25
        };
        let c07 = f32::from_bits(0x3F33_3333);
        let mut s = x(spd)
            .fdiv_m(f32::from_bits(0x3E99_999A))
            .fmul_m(c07)
            .fsubr_m(1.0)
            .fsub(x(g_f32(0xC81308)).fmul_m(c07));
        let c04 = x(f32::from_bits(0x3ECC_CCCD));
        if gt(c04, s) {
            s = c04;
        }
        let u = compression(this, j);
        let v = x(k).fmul(u.fmul(s)).fadd_m(ratio(j));
        set_ratio(j, v);
        if gt(v, one) {
            wr::<u32>(this, 0x710 + 4 * j, 0x3F80_0000);
        }
    }
}

fn g_f32(addr: usize) -> f32 {
    crate::common::g::<f32>(addr).read()
}

pub const HOOKS: &[HookReg] = &[HOOK_BIKE_VEHICLE_DAMAGE, HOOK_DO_BURST_AND_SOFT_GROUND_RATIOS];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
