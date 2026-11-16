//! PORT-CAM-3 (helper B): the unnamed helpers of `CCam::Process_Cam_TwoPlayer` (0x525E50): the
//! "both players visible from a point" test 0x513220 and the two-player source / front / target
//! 0x5132D0. Ported from the 1.0 US bytes; `docs/re/specs/CAM/_unattributed-B.md` as reading aid
//! (gta-reversed has neither).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use crate::common::{Fpu, g, k, rd, wr};

/// `CWorld::Players[0].m_pPed`, `CWorld::Players[1].m_pPed`, `gCurCamColVars`.
const PLAYER0_PED: usize = 0xB7CD98;
const PLAYER1_PED: usize = 0xB7CF28;
const CUR_CAM_COL_VARS: usize = 0x8CCB80;
/// 0.67 (0x8631C8), 7.0 (0x858F48), 0.25 (0x858C84), 0.5 (0x858B8C), 1.0 (0x858624), 0.1 (0x858B1C).
const K_0_67: u32 = 0x3F2B_851F;
const K_7: u32 = 0x40E0_0000;
const K_0_25: u32 = 0x3E80_0000;
const K_HALF: u32 = 0x3F00_0000;
const K_ONE: u32 = 0x3F80_0000;
const K_0_1: u32 = 0x3DCC_CCCD;

pub use sa_core::bindings::vector_normalise;
pub use sa_world::bindings::process_line_of_sight;

use sa_world::world::common::entity_pos as pos_of;

/// `FUN_00513220(CVector point)` (0x513220, `ret 0xC`, AL; the vector by value; the entry fragment
/// 0x40110D loads `ECX = Players[0].m_pPed`, so the caller's ECX is never read). `gCurCamColVars = 5`
/// (the byte only); 0 when the line from `point` to player 0's ped is blocked (buildings, see-through,
/// camera-ignore), else whether the line to player 1's ped is clear.
#[hook(addr = 0x513220, conv = stdcall, name = "FUN_00513220", expect = "83 EC 30 90 E9 E4 DE EE FF")]
pub fn two_player_point_sees_both(px: u32, py: u32, pz: u32) -> u8 {
    let mut point = [px, py, pz];
    let pp = GPtr::<u32>::from_raw(point.as_mut_ptr()).cast::<u8>();
    let mut hit: u32 = 0;
    let mut cp = [0u32; 11];
    let hp = GPtr::<u32>::from_raw(&mut hit).cast::<u8>();
    let cpp = GPtr::<u32>::from_raw(cp.as_mut_ptr()).cast::<u8>();
    g::<u8>(CUR_CAM_COL_VARS).write(5);
    let p0 = pos_of(g::<GPtr<u8>>(PLAYER0_PED).read());
    if process_line_of_sight(pp, p0, cpp, hp, 1, 0, 0, 0, 0, 1, 1, 0) != 0 {
        return 0;
    }
    let p1 = pos_of(g::<GPtr<u8>>(PLAYER1_PED).read());
    u8::from(process_line_of_sight(pp, p1, cpp, hp, 1, 0, 0, 0, 0, 1, 1, 0) == 0)
}

/// `FUN_005132d0(float heading, CVector* outSource, CVector* outFront, CVector* outTarget)` (0x5132D0,
/// thiscall on a CCam, `ret 0x10`). With A = alpha (+0xAC), H = heading, p0 / p1 the player peds'
/// positions (re-evaluated at each use): `*outFront = (-(cos H cos A), -(sin H cos A), sin A)`;
/// `v = normalise(front.xy, 0)`; `R = |p0 - p1| * 0.67 + 7` (`(z^2 + y^2) + x^2`);
/// `w = normalise(p0.xy - p1.xy, 0)`; `t = 0.5 - ((w.y v.y + w.x v.x) + v.z w.z) * 0.25`;
/// `*outTarget = t p0 + (1 - t) p1` (x, y with the f32-stored `(1-t) p1` terms, z with `t p0.z` stored);
/// `*outSource = target - R * front`, then `outSource.z += R * 0.1`.
#[hook(addr = 0x5132D0, conv = thiscall, name = "FUN_005132d0", expect = "83 EC 24 D9 81 AC 00 00 00", this = "CCam")]
pub fn two_player_vectors(
    this: GPtr<u8>,
    heading: u32,
    out_source: GPtr<u8>,
    out_front: GPtr<u8>,
    out_target: GPtr<u8>,
) {
    let h = f32::from_bits(heading);
    let mut f = Fpu::new();
    let mut v: [f32; 3] = [0.0; 3];
    let vp = GPtr::<f32>::from_raw(v.as_mut_ptr()).cast::<u8>();
    f.fld(rd::<f32>(this, 0xAC));
    fcos(&mut f);
    wr::<u32>(vp, 8, 0);
    f.fld(h);
    fcos(&mut f);
    f.fmul_st(1);
    f.fchs();
    let fx = f.fstp();
    f.fld(h);
    fsin(&mut f);
    f.fmul_st(1);
    f.fchs();
    let fy = f.fstp();
    f.fstp_st(0);
    f.fld(rd::<f32>(this, 0xAC));
    fsin(&mut f);
    wr::<f32>(out_front, 0, fx);
    wr::<f32>(out_front, 4, fy);
    let fz = f.fstp();
    wr::<f32>(out_front, 8, fz);
    wr::<u32>(vp, 0, rd::<u32>(out_front, 0));
    wr::<u32>(vp, 4, rd::<u32>(out_front, 4));
    vector_normalise(vp.cast::<[f32;3]>());
    let ped0 = || g::<GPtr<u8>>(PLAYER0_PED).read();
    let ped1 = || g::<GPtr<u8>>(PLAYER1_PED).read();
    let (a, c) = (pos_of(ped0()), pos_of(ped1()));
    for o in [0, 4, 8] {
        f.fld(rd::<f32>(a, o));
        f.fsub_m(rd::<f32>(c, o));
    }
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fmul_m(k(K_0_67));
    f.fadd_m(k(K_7));
    let r = f.fstp();
    let (a, c) = (pos_of(ped0()), pos_of(ped1()));
    let mut w: [f32; 3] = [0.0; 3];
    let wp = GPtr::<f32>::from_raw(w.as_mut_ptr()).cast::<u8>();
    f.fld(rd::<f32>(a, 4));
    f.fsub_m(rd::<f32>(c, 4));
    f.fld(rd::<f32>(a, 0));
    f.fsub_m(rd::<f32>(c, 0));
    wr::<u32>(wp, 8, 0);
    wr::<f32>(wp, 0, f.fstp());
    wr::<f32>(wp, 4, f.fstp());
    vector_normalise(wp.cast::<[f32;3]>());
    f.fld(rd::<f32>(wp, 4));
    f.fmul_m(rd::<f32>(vp, 4));
    f.fld(rd::<f32>(wp, 0));
    f.fmul_m(rd::<f32>(vp, 0));
    f.faddp(1);
    f.fld(rd::<f32>(vp, 8));
    f.fmul_m(rd::<f32>(wp, 8));
    f.faddp(1);
    f.fmul_m(k(K_0_25));
    f.fsubr_m(k(K_HALF));
    let t = f.fstp();
    let c = pos_of(ped1());
    f.fld(k(K_ONE));
    f.fsub_m(t);
    f.fld_st(0);
    f.fmul_m(rd::<f32>(c, 0));
    let s1x = f.fstp();
    f.fld_st(0);
    f.fmul_m(rd::<f32>(c, 4));
    let s1y = f.fstp();
    f.fmul_m(rd::<f32>(c, 8));
    let a = pos_of(ped0());
    f.fld(t);
    f.fmul_m(rd::<f32>(a, 0));
    f.fld(t);
    f.fmul_m(rd::<f32>(a, 4));
    f.fld(t);
    f.fmul_m(rd::<f32>(a, 8));
    let t0z = f.fstp();
    f.fxch(1);
    f.fadd_m(s1x);
    wr::<f32>(out_target, 0, f.fstp());
    f.fadd_m(s1y);
    let ty = f.fstp();
    f.fld(t0z);
    f.fadd_st(1);
    wr::<f32>(out_target, 4, ty);
    let tz = f.fstp();
    f.fstp_st(0);
    wr::<f32>(out_target, 8, tz);
    f.fld(r);
    f.fmul_m(rd::<f32>(out_front, 0));
    f.fld(r);
    f.fmul_m(rd::<f32>(out_front, 4));
    f.fld(r);
    f.fmul_m(rd::<f32>(out_front, 8));
    let rfz = f.fstp();
    f.fld(rd::<f32>(out_target, 0));
    f.fsub_st(2);
    let sx = f.fstp();
    f.fld(rd::<f32>(out_target, 4));
    f.fsub_st(1);
    let sy = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(rd::<f32>(out_target, 8));
    f.fsub_m(rfz);
    wr::<f32>(out_source, 0, sx);
    let sz = f.fstp();
    f.fld(r);
    wr::<f32>(out_source, 4, sy);
    f.fmul_m(k(K_0_1));
    wr::<f32>(out_source, 8, sz);
    f.fadd_m(rd::<f32>(out_source, 8));
    wr::<f32>(out_source, 8, f.fstp());
}

fn fsin(f: &mut Fpu) {
    let v = f.pop();
    f.push(v.fsin());
}

fn fcos(f: &mut Fpu) {
    let v = f.pop();
    f.push(v.fcos());
}

use sa_world::bindings::weapon_find_nearest_target_entity_with_screen_coors;

/// `CDraw::ms_fAspectRatio`
const ASPECT_RATIO: usize = 0xC3_EFA4;
/// 0.5 * pi / 180 (0x8631D4)
const K_HALF_DEG2RAD: u32 = 0x3C0E_FA35;

/// `CCam::Get_TwoPlayer_AimVector(CVector& out)` (0x513E40, thiscall `ret 4`; FRONTIER-2 turn 8 lane B, batch
/// `frontier2-b18`): the aiming player's ped is player 0's, or player 1's when player 0 sits in a vehicle he does
/// not drive (the entry fragment 0x407E0F loads `Players[0].m_pPed` through `[0x156089C] - [0x727253]` =
/// 0xB7CD98). The nearest target of the cam's screen point (+0x108, +0x10C) within twice the current weapon's
/// target range around the ped wins: `out = pos(target) - source`. Without one the screen point is projected:
/// `t = tan(fov * 0.5deg)`, `A = (t / aspect) * sy`, `R = front x up`, `out = (R*sx)*f32(t) + front - up*A` with
/// the bytes' f32 stores. `out` is normalised; returns the target (or null).
#[hook(addr = 0x513E40, conv = thiscall, name = "CCam::Get_TwoPlayer_AimVector", this = "CCam",
       expect = "83 EC 28 53 55 56 57 90 E9 C2 3F EF FF",
       unverified = "frontier2-b18", route = "L2", hazards = "x87-transcendental",
       note = "FPTAN of the half FOV")]
pub fn get_two_player_aim_vector(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    use sa_core::bindings::{cross_product, get_weapon_info};
    use sa_math::fpu::{f32_of, x};
    use sa_world::bindings::ped_get_weapon_skill;
    let mut ped = g::<GPtr<u8>>(PLAYER0_PED).read();
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if !veh.is_null() && rd::<GPtr<u8>>(veh, 0x460) != ped {
        ped = g::<GPtr<u8>>(PLAYER1_PED).read();
    }
    let slot = isize::from(rd::<i8>(ped, 0x718));
    let wt = rd::<i32>(ped, 0x5A0 + slot * 0x1C);
    let info = get_weapon_info(wt, ped_get_weapon_skill(ped));
    let p = pos_of(ped);
    let range = f32_of(x(rd::<f32>(info, 8)).fadd(x(rd::<f32>(info, 8))));
    let target = weapon_find_nearest_target_entity_with_screen_coors(
        rd::<f32>(this, 0x108),
        rd::<f32>(this, 0x10C),
        range,
        rd::<f32>(p, 0),
        rd::<f32>(p, 4),
        rd::<f32>(p, 8),
        GPtr::NULL,
        GPtr::NULL,
    );
    let v = if !target.is_null() {
        let tp = pos_of(target);
        let mut v = [0f32; 3];
        for (i, c) in v.iter_mut().enumerate() {
            let o = 4 * i as isize;
            *c = f32_of(x(rd::<f32>(tp, o)).fsub_m(rd::<f32>(this, 0x19C + o)));
        }
        v
    } else {
        let mut r = [0f32; 3];
        let front = this.byte_add(0x190);
        let up = this.byte_add(0x1B4);
        cross_product(sa_core::field::v3_ptr(&mut r), front, up);
        let t = x(rd::<f32>(this, 0xB4)).fmul_m(f32::from_bits(K_HALF_DEG2RAD)).fptan().map_or(x(0.0), |(t, _)| t);
        let a = t.fdiv(x(g::<f32>(ASPECT_RATIO).read())).fmul_m(rd::<f32>(this, 0x10C));
        let ux = f32_of(a.fmul_m(rd::<f32>(up, 0)));
        let uy = f32_of(a.fmul_m(rd::<f32>(up, 4)));
        let uz = a.fmul_m(rd::<f32>(up, 8));
        let sx = rd::<f32>(this, 0x108);
        let rx = x(r[0]).fmul_m(sx);
        let ry = f32_of(x(r[1]).fmul_m(sx));
        let rz = f32_of(x(r[2]).fmul_m(sx));
        let tf = f32_of(t);
        let rx = rx.fmul_m(tf);
        let ry = x(ry).fmul_m(tf);
        let rz = f32_of(x(rz).fmul_m(tf));
        let cx = f32_of(rx.fadd_m(rd::<f32>(front, 0)));
        let cy = ry.fadd_m(rd::<f32>(front, 4));
        let cz = f32_of(x(rz).fadd_m(rd::<f32>(front, 8)));
        [f32_of(x(cx).fsub_m(ux)), f32_of(cy.fsub_m(uy)), f32_of(x(cz).fsub(uz))]
    };
    for (i, c) in v.iter().enumerate() {
        wr::<f32>(out, 4 * i as isize, *c);
    }
    vector_normalise(out.cast());
    target
}

pub const HOOKS: &[HookReg] = &[HOOK_TWO_PLAYER_POINT_SEES_BOTH, HOOK_TWO_PLAYER_VECTORS, HOOK_GET_TWO_PLAYER_AIM_VECTOR];
