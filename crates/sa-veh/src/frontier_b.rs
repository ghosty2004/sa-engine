//! FRONTIER-2 lane B (batch `frontier2-b3`): vehicle functions on the phase B frontier: `CQuadBike`'s constructor,
//! `CAutomobile::SetupDamageAfterLoad`, `BreakTowLink`, `GetTowBarPos`, `CTrain::GetDooorAngleOpenRatioU32` and
//! `CHeli::TestSniperCollision`. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{automobile_ctor, automobile_setup_suspension_lines, car_set_panel_damage as set_bumper_damage};
use crate::common::{f32_of, g, rand_at, rd, wr, x, xi};
use crate::mvpd_v2::ride_anim_data_ctor;
use sa_core::bindings::matrix_mul_vec;
use sa_world::bindings::{
    automobile_set_panel_damage, dist_to_line, entity_clean_up_old_reference, find_player_ped, set_door_damage,
};
use sa_rwcore::bindings::frame_get_ltm;
use sa_stream::bindings::ped_is_player;

orig! {
    /// `CVehicle::GetDooorAngleOpenRatio(eDoors)` (vtable +0x78, ST0)
    fn v_get_door_angle_open_ratio(v: GPtr<u8>, door: u32) -> f64 = thiscall vslot 0x78 => unported;
    /// `CVehicle::BlowUpCar(CEntity*, bool)` (vtable +0xA4)
    fn v_blow_up_car(v: GPtr<u8>, damager: GPtr<u8>, hide: u32) = thiscall vslot 0xA4 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8).
fn model_info_ptr(model: i32) -> GPtr<u8> {
    g::<GPtr<u8>>(0xA9_B0C8 + 4 * model as usize).read()
}

/// `CQuadBike::CQuadBike(int32 modelIndex, eVehicleCreatedBy)` (0x6CE370, thiscall, `ret 8`; the entry jumps
/// through a protection chunk that pushes the SEH handler): the `CAutomobile` constructor (no suspension setup),
/// the quad vtable 0x871AE8, the ride-anim data (+0x98C) constructed with the anim group 10, the handling pointer
/// +0x988 = `0xC3491C + 64 * (byte +0x4A of the model info)`, subtype +0x594 = 2, +0x9AC..+0x9B4 = 0, +0x9A8 =
/// 1.0, `CAutomobile::SetupSuspensionLines`, bit 0 of +0x9B8 cleared, +0x494 = 0. Returns `this`. (SEH frame not
/// reproduced.)
#[hook(addr = 0x6CE370, conv = thiscall, name = "CQuadBike::Constructor", expect = "6A FF E9 B1 9C D3 FF",
       this = "CQuadBike", unverified = "frontier2-b3", route = "L2", hazards = "heap-writes")]
pub fn quad_bike_constructor(this: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> {
    automobile_ctor(this, mi as u32, created_by, 0);
    wr::<u32>(this, 0, 0x87_1AE8);
    ride_anim_data_ctor(this.byte_add(0x98C));
    let info = model_info_ptr(mi);
    wr::<u32>(this, 0x98C, 0xA);
    wr::<u32>(this, 0x988, (u32::from(rd::<u8>(info, 0x4A)) << 6).wrapping_add(0xC3_491C));
    wr::<u32>(this, 0x594, 2);
    wr::<u32>(this, 0x9AC, 0);
    wr::<u32>(this, 0x9B0, 0);
    wr::<u32>(this, 0x9B4, 0);
    wr::<u32>(this, 0x9A8, 0x3F80_0000);
    automobile_setup_suspension_lines(this);
    wr::<u8>(this, 0x9B8, rd::<u8>(this, 0x9B8) & 0xFE);
    wr::<u32>(this, 0x494, 0);
    this
}

/// `CAutomobile::SetupDamageAfterLoad()` (0x6B3E90, virtual): each present part gets its damage state re-applied
/// (`flying` false): bumpers 5 (+0x678) and 6 (+0x67C), doors 0..5 (+0x688, +0x68C, +0x670, +0x668, +0x674,
/// +0x66C, interleaved as the original orders them), panels 0 (+0x684) and 1 (+0x680).
#[hook(addr = 0x6B3E90, conv = thiscall, name = "CAutomobile::SetupDamageAfterLoad", expect = "56 8B F1 8B 86 78 06 00 00",
       this = "CAutomobile", unverified = "frontier2-b3", route = "L2", hazards = "heap-writes")]
pub fn automobile_setup_damage_after_load(this: GPtr<u8>) {
    let has = |o: isize| rd::<u32>(this, o) != 0;
    if has(0x678) {
        set_bumper_damage(this, 5, 0);
    }
    if has(0x688) {
        set_door_damage(this, 0, 0);
    }
    if has(0x67C) {
        set_bumper_damage(this, 6, 0);
    }
    for (o, door) in [(0x68C, 1), (0x670, 2), (0x668, 3), (0x674, 4), (0x66C, 5)] {
        if has(o) {
            set_door_damage(this, door, 0);
        }
    }
    if has(0x684) {
        automobile_set_panel_damage(this, 0, 0);
    }
    if has(0x680) {
        automobile_set_panel_damage(this, 1, 0);
    }
}

/// `CAutomobile::BreakTowLink()` (0x6A4400, virtual, AL): the tractor link (+0x4C4) and its back link (+0x4C8 of the
/// tractor) cleaned up and cleared; then, only for a trailer status (byte +0x36 >> 3 of 10 or 11): with a driver
/// (+0x460) the status becomes 0 for a player and 3 otherwise; without one 5 (wrecked) when the health (+0x4C0) is
/// below 1.0, else 4 (abandoned); AL = 1. Other statuses: AL = 0.
#[hook(addr = 0x6A4400, conv = thiscall, name = "CAutomobile::BreakTowLink", expect = "56 8B F1 8B 86 C4 04 00 00",
       this = "CAutomobile", unverified = "frontier2-b3", route = "L2", hazards = "none")]
pub fn automobile_break_tow_link(this: GPtr<u8>) -> u8 {
    let link = this.byte_add(0x4C4);
    let tractor: GPtr<u8> = rd(link, 0);
    if !tractor.is_null() {
        let back = tractor.byte_add(0x4C8);
        let t: GPtr<u8> = rd(back, 0);
        if !t.is_null() {
            entity_clean_up_old_reference(t, back);
            wr::<u32>(rd::<GPtr<u8>>(link, 0), 0x4C8, 0);
        }
        entity_clean_up_old_reference(rd(link, 0), link);
        wr::<u32>(link, 0, 0);
    }
    let st = rd::<u8>(this, 0x36);
    if st >> 3 != 0xA && st >> 3 != 0xB {
        return 0;
    }
    let driver: GPtr<u8> = rd(this, 0x460);
    if !driver.is_null() {
        if ped_is_player(driver) != 0 {
            wr::<u8>(this, 0x36, rd::<u8>(this, 0x36) & 7);
        } else {
            wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x18);
        }
        return 1;
    }
    let below = !x(rd::<f32>(this, 0x4C0)).fcom(x(g::<f32>(0x85_8624).read())).test_parity(5);
    wr::<u8>(this, 0x36, (st & 7) | if below { 0x28 } else { 0x20 });
    1
}

/// `CTrain::GetDooorAngleOpenRatioU32(uint32 door)` (0x6F5930, virtual, ST0): the door numbers 8, 9, 10, 11 map to
/// the doors 3, 5, 2, 4 of `GetDooorAngleOpenRatio(eDoors)` (virtual +0x78, a tail jump); anything else gives 0.0.
#[hook(addr = 0x6F5930, conv = thiscall, name = "CTrain::GetDooorAngleOpenRatioU32", expect = "8B 44 24 04 D9 05 50 8B 85 00",
       this = "CTrain", unverified = "frontier2-b3", route = "L2", hazards = "none")]
pub fn train_get_door_angle_open_ratio_u32(this: GPtr<u8>, door: u32) -> f64 {
    let d = match door {
        8 => 3,
        9 => 5,
        0xA => 2,
        0xB => 4,
        _ => return f64::from(g::<f32>(0x85_8B50).read()),
    };
    v_get_door_angle_open_ratio(this, d)
}

/// `CAutomobile::GetTowBarPos(CVector& out, bool always, CVehicle* trailer)` (0x6AF250, virtual, `ret 0xC`, AL).
/// The tow truck (525) and tractor (531): `out = matrix * (0, k + colModel.min.y, z - [+0x898])` with k = -1.05 for
/// the tow truck (none when the trailer is a real trailer, subtype 11) and -0.6 for the tractor (none for a trailer
/// other than model 610), z = `(1 - arm (+0x86C) / 0x8D313C) * 0.5 + 0.5`. Models 514, 515, 403, 591, model 552
/// pulling 611 and models 485, 583, 606, 607 pulling 606, 607 or 608: the position of the tow hitch frame (+0x698)
/// when present. Otherwise, with `always`: `(0, -0.5 + min.y, 0.5 - [+0x898])` through the matrix; else false.
#[hook(addr = 0x6AF250, conv = thiscall, name = "CAutomobile::GetTowBarPos", expect = "66 8B 41 22 83 EC 0C",
       this = "CAutomobile", unverified = "frontier2-b3", route = "L2", hazards = "none")]
pub fn automobile_get_tow_bar_pos(this: GPtr<u8>, out: GPtr<u8>, always: u32, trailer: GPtr<u8>) -> u8 {
    let model = rd::<i16>(this, 0x22);
    let min_y = || {
        let col: GPtr<u8> = rd(model_info_ptr(i32::from(rd::<i16>(this, 0x22))), 0x14);
        rd::<f32>(col, 4)
    };
    let finish = |z: sa_math::x87::X87| {
        wr(out, 8, z.fsub_m(rd(this, 0x898)).to_f32());
        let mut tmp = [0u32; 3];
        let tp: GPtr<u8> = GPtr::from_raw(tmp.as_mut_ptr()).cast();
        let r = matrix_mul_vec(tp, rd(this, 0x14), out);
        for o in [0, 4, 8] {
            wr::<u32>(out, o, rd(r, o));
        }
        1u8
    };
    let arm_z = || {
        let a = xi(i32::from(rd::<u16>(this, 0x86C))).fdiv(xi(i32::from(g::<u16>(0x8D_313C).read())));
        a.fsubr_m(1.0).fmul_m(g::<f32>(0x87_10AC).read()).fadd_m(0.5)
    };
    if model == 0x20D || model == 0x213 {
        let k = if model == 0x213 {
            if !trailer.is_null() && rd::<u32>(trailer, 0x594) == 0xB && rd::<i16>(trailer, 0x22) != 0x262 {
                return 0;
            }
            g::<f32>(0x87_10A8).read()
        } else {
            if !trailer.is_null() && rd::<u32>(trailer, 0x594) == 0xB {
                return 0;
            }
            g::<f32>(0x87_10A0).read()
        };
        wr::<u32>(out, 0, 0);
        wr(out, 4, x(k).fadd_m(min_y()).to_f32());
        return finish(arm_z());
    }
    let tm = |t: GPtr<u8>| rd::<i16>(t, 0x22);
    let hitch = matches!(model, 0x202 | 0x203 | 0x193 | 0x24F)
        || (model == 0x228 && !trailer.is_null() && tm(trailer) == 0x263)
        || (matches!(model, 0x1E5 | 0x247 | 0x25E | 0x25F)
            && !trailer.is_null()
            && matches!(tm(trailer), 0x25E..=0x260));
    if hitch {
        let frame: GPtr<u8> = rd(this, 0x698);
        if !frame.is_null() {
            let ltm = frame_get_ltm(frame);
            for o in [0, 4, 8] {
                wr::<u32>(out, o, rd(ltm, 0x30 + o));
            }
            return 1;
        }
    }
    if always as u8 == 0 {
        return 0;
    }
    wr::<u32>(out, 0, 0);
    wr(out, 4, x(g::<f32>(0x87_109C).read()).fadd_m(min_y()).to_f32());
    finish(x(g::<f32>(0x85_8B8C).read()))
}

/// `CHeli::TestSniperCollision(CVector* start, CVector* end)` (0x6C6890, cdecl, AL): only when the shot's z extent
/// is below half its length (`dz < |end - start| * 0.5`, x87); each of the two police / news helis (0xC1C964,
/// 0xC1C968) not flagged +0x40 bit 0x40000 whose pilot point `matrix * (-0.43, 1.49, 1.5)` lies within 0.8 of the
/// shot line is hit: +0x8AC = `(rand() < 0x3FFF) * 0.1 - 0.05`, `BlowUpCar(FindPlayerPed(-1), false)`, +0x9B9 = 0;
/// AL = 1 when any was hit. The original keeps the rand flag in its `end` argument slot.
#[hook(addr = 0x6C6890, conv = cdecl, name = "CHeli::TestSniperCollision", expect = "83 EC 18 53 55",
       unverified = "frontier2-b3", route = "L2", hazards = "heap-writes,stack-residue,x87-transcendental",
       note = "rand site 0x6C6953; the rand flag goes to its own end argument slot")]
pub fn heli_test_sniper_collision(start: GPtr<u8>, end: GPtr<u8>) -> u8 {
    let dx = x(rd(end, 0)).fsub_m(rd(start, 0)).to_f32();
    let dy = x(rd(end, 4)).fsub_m(rd(start, 4)).to_f32();
    let dz = x(rd(end, 8)).fsub_m(rd(start, 8));
    let len = dz.fmul(dz).fadd(x(dx).fmul_m(dx)).fadd(x(dy).fmul_m(dy)).fsqrt().fmul_m(g::<f32>(0x85_8B8C).read());
    if dz.fcom(len).test_parity(5) {
        return 0;
    }
    let mut hit = 0u8;
    for slot in [0xC1_C964usize, 0xC1_C968] {
        let h = g::<GPtr<u8>>(slot).read();
        if h.is_null() || rd::<u32>(h, 0x40) & 0x4_0000 != 0 {
            continue;
        }
        let mut off = [0xBEDC_28F6u32, 0x3FBE_B852, 0x3FC0_0000];
        let mut p = [0u32; 3];
        let pp: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
        matrix_mul_vec(pp, rd(h, 0x14), GPtr::from_raw(off.as_mut_ptr()).cast());
        let d = dist_to_line(start, end, pp);
        if d.fcom(x(g::<f32>(0x85_8C98).read())).test_parity(5) {
            continue;
        }
        let r = i32::from(rand_at(0x6C_6953) < 0x3FFF);
        let h = g::<GPtr<u8>>(slot).read();
        wr(h, 0x8AC, f32_of(xi(r).fmul_m(g::<f32>(0x85_8B1C).read()).fsub_m(g::<f32>(0x85_8C28).read())));
        let h = g::<GPtr<u8>>(slot).read();
        v_blow_up_car(h, find_player_ped(-1), 0);
        wr::<u8>(g::<GPtr<u8>>(slot).read(), 0x9B9, 0);
        hit = 1;
    }
    hit
}

pub const HOOKS: &[HookReg] = &[
    HOOK_QUAD_BIKE_CONSTRUCTOR,
    HOOK_AUTOMOBILE_SETUP_DAMAGE_AFTER_LOAD,
    HOOK_AUTOMOBILE_BREAK_TOW_LINK,
    HOOK_TRAIN_GET_DOOR_ANGLE_OPEN_RATIO_U32,
    HOOK_AUTOMOBILE_GET_TOW_BAR_POS,
    HOOK_HELI_TEST_SNIPER_COLLISION,
];
