//! PORT-CAM-3 (helper B): the unnamed helpers of `CCam::Process_Cam_TwoPlayer` (0x525E50): the
//! "both players visible from a point" test 0x513220 and the two-player source / front / target
//! 0x5132D0. Ported from the 1.0 US bytes; `docs/re/specs/CAM/_unattributed-B.md` as reading aid
//! (gta-reversed has neither).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use crate::common::{Fpu, g, k, rd, wr};

/// `CWorld::Players[0].m_pPed`, `CWorld::Players[1].m_pPed`, `gCurCamColVars`.
pub(crate) const PLAYER0_PED: usize = 0xB7CD98;
pub(crate) const PLAYER1_PED: usize = 0xB7CF28;
pub(crate) const CUR_CAM_COL_VARS: usize = 0x8CCB80;
/// 0.67 (0x8631C8), 7.0 (0x858F48), 0.25 (0x858C84), 0.5 (0x858B8C), 1.0 (0x858624), 0.1 (0x858B1C).
const K_0_67: u32 = 0x3F2B_851F;
const K_7: u32 = 0x40E0_0000;
const K_0_25: u32 = 0x3E80_0000;
const K_HALF: u32 = 0x3F00_0000;
const K_ONE: u32 = 0x3F80_0000;
const K_0_1: u32 = 0x3DCC_CCCD;

pub use sa_core::bindings::vector_normalise;
pub use sa_world::bindings::process_line_of_sight;

pub(crate) use sa_world::world::common::entity_pos as pos_of;

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

/// 80.0 (0x8CCCB8 holds the 0.4 rad tilt), 0.005 (0x858B4C), 0.0 (0x858B50), 10.0 (0x85862C), 30.0 (0x858CA4),
/// 0.75 (0x858F34), 0.45 (0x8631D0).
const K_80: u32 = 0x42A0_0000;
const TOPDOWN_TILT: usize = 0x8C_CCB8;
const K_0_005: u32 = 0x3BA3_D70A;
const K_10: u32 = 0x4120_0000;
const K_30: u32 = 0x41F0_0000;
const K_0_75: u32 = 0x3F40_0000;
const K_0_45: u32 = 0x3EE6_6666;
const TIME_STEP: usize = 0xB7_CB5C;

/// `CCam::Process_Cam_TwoPlayer_Separate_Cars_TopDown()` (0x513BE0, thiscall, no arguments; unlike
/// gta-reversed, the time step is multiplied by 0.005 and up does not come from the source). FOV 80.
/// `e_i` = player i's ped's +0x58C vehicle, else the ped
/// (`FindPlayerEntity` inlined, no in-vehicle flag test). `c = (p1 + p0) * 0.5` (x stored as f32, y and z kept on the
/// x87 stack, z's sum rounded first), `d = |p0 - p1|` (`(z^2 + y^2) + x^2`). The height target m: both players in a
/// vehicle 1.0, one 0.75, none 0.45; `diff = m - h` (+0xD8), `s = f32(timeStep * 0.005)`: `h = m` when `|diff| < s`,
/// else `h -/+ s` by the sign of `f32(diff)`. `R = max(d + 10, 30) * h`; with `a = f32(sin 0.4)`, `b = f32(-cos 0.4)`:
/// source = `(f32(c.x - 0 * R), c.y - f32(a R), c.z - f32(b R))`, front = `(0, a, b)`, up = `(0, f32(-b), a)`,
/// +0x148 = source.
#[hook(addr = 0x513BE0, conv = thiscall, name = "CCam::Process_Cam_TwoPlayer_Separate_Cars_TopDown", this = "CCam",
       expect = "83 EC 2C 53 C7 81 B4 00 00 00 00 00 A0 42",
       unverified = "frontier2-b29", route = "L2", hazards = "x87-transcendental", note = "FSIN / FCOS of 0.4")]
pub fn process_two_player_separate_cars_top_down(this: GPtr<u8>) {
    use sa_math::fpu::{f32_of, x};
    wr::<u32>(this, 0xB4, K_80);
    let entity = |slot: usize| {
        let ped = g::<GPtr<u8>>(slot).read();
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        (veh, if veh.is_null() { ped } else { veh })
    };
    let (v0, e0) = entity(PLAYER0_PED);
    let (v1, e1) = entity(PLAYER1_PED);
    let (q0, q1) = (pos_of(e0), pos_of(e1));
    let sx = x(rd::<f32>(q1, 0)).fadd_m(rd::<f32>(q0, 0));
    let sy = x(rd::<f32>(q0, 4)).fadd_m(rd::<f32>(q1, 4));
    let sz = f32_of(x(rd::<f32>(q0, 8)).fadd_m(rd::<f32>(q1, 8)));
    let cx = f32_of(sx.fmul_m(k(K_HALF)));
    let cy = sy.fmul_m(k(K_HALF));
    let cz = x(sz).fmul_m(k(K_HALF));
    let dx = x(rd::<f32>(q0, 0)).fsub_m(rd::<f32>(q1, 0));
    let dy = x(rd::<f32>(q0, 4)).fsub_m(rd::<f32>(q1, 4));
    let dz = x(rd::<f32>(q0, 8)).fsub_m(rd::<f32>(q1, 8));
    let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let m = match (v0.is_null(), v1.is_null()) {
        (false, false) => K_ONE,
        (true, true) => K_0_45,
        _ => K_0_75,
    };
    let m = x(k(m));
    let diff = m.fsub_m(rd::<f32>(this, 0xD8));
    let diff32 = f32_of(diff);
    let s = f32_of(x(g::<f32>(TIME_STEP).read()).fmul_m(k(K_0_005)));
    let abs = if diff.fcom(x(0.0)).test_parity(5) { diff } else { diff.fchs() };
    let h = if !abs.fcom(x(s)).test_parity(5) {
        m
    } else if x(diff32).fcom(x(0.0)).test_parity(5) {
        x(s).fadd_m(rd::<f32>(this, 0xD8))
    } else {
        x(rd::<f32>(this, 0xD8)).fsub_m(s)
    };
    wr::<f32>(this, 0xD8, f32_of(h));
    let r = dist.fadd_m(k(K_10));
    let r = if x(k(K_30)).fcom(r).test_any(0x41) { r } else { x(k(K_30)) };
    let r = r.fmul_m(rd::<f32>(this, 0xD8));
    let a = f32_of(x(g::<f32>(TOPDOWN_TILT).read()).fsin());
    let b = f32_of(x(g::<f32>(TOPDOWN_TILT).read()).fcos().fchs());
    let ar = f32_of(x(a).fmul(r));
    let br = f32_of(x(b).fmul(r));
    let src = [f32_of(x(cx).fsub(x(0.0).fmul(r))), f32_of(cy.fsub_m(ar)), f32_of(cz.fsub_m(br))];
    let up_y = f32_of(x(b).fchs());
    for (i, v) in src.iter().enumerate() {
        let o = 4 * i as isize;
        wr::<f32>(this, 0x19C + o, *v);
        wr::<f32>(this, 0x148 + o, *v);
    }
    wr::<u32>(this, 0x190, 0);
    wr::<f32>(this, 0x194, a);
    wr::<f32>(this, 0x198, b);
    wr::<u32>(this, 0x1B4, 0);
    wr::<f32>(this, 0x1B8, up_y);
    wr::<f32>(this, 0x1BC, a);
}

/// 6.0 (0x858B44), pi (0x858CB8), -0.13 (0x8631CC); the blend rate 0.04 is .data (0x8CCCB4, read at run time);
/// `CModelInfo::ms_modelInfoPtrs`.
const K_6: u32 = 0x40C0_0000;
const K_PI: u32 = 0x4049_0FDB;
const K_M0_13: u32 = 0xBE05_1EB8;
const SEPARATE_BLEND_RATE: usize = 0x8C_CCB4;
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;

/// An original function's stack frame, addressed by the offset from ESP after its register pushes (temporary
/// pushes inside the body folded in), so locals passed by address keep the bytes' layout.
pub(crate) struct Frame<const N: usize>(pub(crate) [u32; N]);

impl<const N: usize> Frame<N> {
    pub(crate) fn p(&mut self, o: usize) -> GPtr<u8> {
        GPtr::<u32>::from_raw(self.0.as_mut_ptr()).cast::<u8>().byte_add(o as isize)
    }
    pub(crate) fn f(&self, o: usize) -> f32 {
        f32::from_bits(self.0[o / 4])
    }
    pub(crate) fn s(&mut self, o: usize, v: f32) {
        self.0[o / 4] = v.to_bits();
    }
    pub(crate) fn u(&mut self, o: usize, v: u32) {
        self.0[o / 4] = v;
    }
    pub(crate) fn copy_from(&mut self, o: usize, src: GPtr<u8>) {
        for i in 0..3 {
            self.0[o / 4 + i] = rd::<u32>(src, 4 * i as isize);
        }
    }
    pub(crate) fn normalise(&mut self, o: usize) {
        vector_normalise(self.p(o).cast::<[f32; 3]>());
    }
}

/// `CColModel::m_boundBox.m_vecMax.z` of the entity's model (`ms_modelInfoPtrs[model]->m_pColModel`).
fn col_max_z(e: GPtr<u8>) -> f32 {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(e, 0x22) as usize).read();
    rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x14)
}

/// `CCam::Process_Cam_TwoPlayer_Separate_Cars()` (0x513510, thiscall, no arguments), with the
/// original's frame and x87 stack kept so every intermediate rounds as the bytes do.
/// FOV 80; V1 / V2 = the two players' peds' +0x58C vehicles (no null checks). `D = normalise(pos V2 - pos
/// V1)`; the points behind each car `A = pos V1 - 6D` and `B = pos V2 + 6D` (z first raised by the col model's max z
/// + 1); `E1 = normalise(pos V2 - A)`, `E2 = normalise(pos V1 - B)`; `N = normalise(normalise(A.y - B.y, B.x - A.x,
/// 0) with z = -0.1)`. With `ang = blend (+0x114) * pi`, `sA = f32(sin ang)`, `w = f32((cos ang + 1) * 0.5)` and
/// `dist = |pos V1 - pos V2|`: source = `(w A + (1 - w) B) - 0.75 * dist * sA * N`, front = `normalise((w E1 + (1 - w)
/// E2) * (1 - sA) + sA N)`, the x87 stack keeping the bytes' unrounded terms. +0x148 = the focused player's (+0x110)
/// car position (V1 when non-zero). up = (0,0,1); `R = normalise(up x front)`, `up = normalise(front x R)`. The
/// focus switches to the other player when the focused car's speed along the flat front is below -0.13 and below
/// the other car's speed along the reversed front; the blend moves by `timeStep * 0.04` towards 1 (player 1 in
/// focus) or 0, clamped.
#[hook(addr = 0x513510, conv = thiscall, name = "CCam::Process_Cam_TwoPlayer_Separate_Cars", this = "CCam",
       expect = "81 EC 94 00 00 00 53 8B D9",
       unverified = "frontier2-b29", route = "L2", hazards = "x87-transcendental", note = "FSIN / FCOS of the blend angle")]
pub fn process_two_player_separate_cars(this: GPtr<u8>) {
    use sa_core::bindings::cross_product;
    use sa_math::fpu::{f32_of, x};
    wr::<u32>(this, 0xB4, K_80);
    let v1 = rd::<GPtr<u8>>(g::<GPtr<u8>>(PLAYER0_PED).read(), 0x58C);
    let v2 = rd::<GPtr<u8>>(g::<GPtr<u8>>(PLAYER1_PED).read(), 0x58C);
    let mut fr = Frame([0; 0x2A]);
    let mut f = Fpu::new();
    // D at 0x6C
    let (c, a) = (pos_of(v1), pos_of(v2));
    f.fld(rd::<f32>(a, 8));
    f.fsub_m(rd::<f32>(c, 8));
    f.fld(rd::<f32>(a, 4));
    f.fsub_m(rd::<f32>(c, 4));
    f.fld(rd::<f32>(a, 0));
    f.fsub_m(rd::<f32>(c, 0));
    for o in [0x6C, 0x70, 0x74] {
        let v = f.fstp();
        fr.s(o, v);
    }
    fr.normalise(0x6C);
    // A at 0x1C
    fr.copy_from(0x1C, pos_of(v1));
    f.fld(fr.f(0x24));
    f.fadd_m(col_max_z(v1));
    f.fadd_m(k(K_ONE));
    let v = f.fstp();
    fr.s(0x24, v);
    f.fld(fr.f(0x6C));
    f.fmul_m(k(K_6));
    let v = f.fst();
    fr.s(0x38, v);
    f.fld(fr.f(0x70));
    f.fmul_m(k(K_6));
    f.fld_st(0);
    f.fld(fr.f(0x74));
    f.fmul_m(k(K_6));
    let v = f.fst();
    fr.s(0x18, v);
    f.fld(fr.f(0x1C));
    f.fsub_m(fr.f(0x38));
    let v = f.fstp();
    fr.s(0x1C, v);
    f.fld(fr.f(0x20));
    f.fsub_st(2);
    let v = f.fstp();
    fr.s(0x20, v);
    f.fld(fr.f(0x24));
    f.fsub_st(1);
    let v = f.fstp();
    fr.s(0x24, v);
    f.fstp_st(0);
    f.fstp_st(0);
    // B at 0x2C
    fr.copy_from(0x2C, pos_of(v2));
    f.fld(fr.f(0x34));
    f.fadd_m(col_max_z(v2));
    f.fadd_m(k(K_ONE));
    let v = f.fstp();
    fr.s(0x34, v);
    f.fxch(1);
    let v = f.fstp();
    fr.s(0x38, v);
    f.fld(fr.f(0x18));
    f.fld(fr.f(0x38));
    f.fadd_m(fr.f(0x2C));
    let v = f.fstp();
    fr.s(0x2C, v);
    f.fxch(1);
    f.fadd_m(fr.f(0x30));
    let v = f.fstp();
    fr.s(0x30, v);
    f.fadd_m(fr.f(0x34));
    let v = f.fstp();
    fr.s(0x34, v);
    // E1 at 0x78, E2 at 0x84 (through the temporary at 0x0C)
    for (e, from, to) in [(v2, 0x1C, 0x78), (v1, 0x2C, 0x84)] {
        let q = pos_of(e);
        for i in 0..3 {
            let v = f32_of(x(rd::<f32>(q, 4 * i as isize)).fsub_m(fr.f(from + 4 * i)));
            fr.s(0x0C + 4 * i, v);
        }
        for i in 0..3 {
            let v = fr.0[3 + i];
            fr.u(to + 4 * i, v);
        }
        fr.normalise(to);
    }
    // N at 0x60
    fr.u(0x68, 0);
    fr.s(0x60, f32_of(x(fr.f(0x20)).fsub_m(fr.f(0x30))));
    fr.s(0x64, f32_of(x(fr.f(0x2C)).fsub_m(fr.f(0x1C))));
    fr.normalise(0x60);
    fr.u(0x68, 0xBDCC_CCCD);
    fr.normalise(0x60);
    // sA at 0x44, w at 0x28
    f.fld(rd::<f32>(this, 0x114));
    f.fmul_m(k(K_PI));
    f.fld_st(0);
    fsin(&mut f);
    let v = f.fstp();
    fr.s(0x44, v);
    fcos(&mut f);
    f.fadd_m(k(K_ONE));
    f.fmul_m(k(K_HALF));
    let v = f.fstp();
    fr.s(0x28, v);
    // dist at 0x18
    let (c, a) = (pos_of(v2), pos_of(v1));
    f.fld(rd::<f32>(a, 0));
    f.fsub_m(rd::<f32>(c, 0));
    f.fld(rd::<f32>(a, 4));
    f.fsub_m(rd::<f32>(c, 4));
    f.fld(rd::<f32>(a, 8));
    f.fsub_m(rd::<f32>(c, 8));
    f.fld_st(2);
    f.fmulp(3);
    f.fld_st(0);
    f.fmul_st(1);
    f.faddp(3);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(3);
    f.fxch(2);
    f.fsqrt();
    f.fstp_st(2);
    f.fstp_st(0);
    let v = f.fstp();
    fr.s(0x18, v);
    // sA * N, the offset O (0x3C / 0x40, x kept on the stack)
    f.fld(fr.f(0x44));
    f.fmul_m(fr.f(0x60));
    let v = f.fst();
    fr.s(0x38, v);
    f.fld(fr.f(0x44));
    f.fmul_m(fr.f(0x64));
    f.fld_st(0);
    f.fld(fr.f(0x44));
    f.fmul_m(fr.f(0x68));
    let v = f.fst();
    fr.s(0x90, v);
    f.fld(fr.f(0x38));
    f.fmul_m(fr.f(0x18));
    let v = f.fstp();
    fr.s(0x0C, v);
    f.fxch(1);
    f.fmul_m(fr.f(0x18));
    let v = f.fstp();
    fr.s(0x10, v);
    f.fmul_m(fr.f(0x18));
    f.fld(fr.f(0x0C));
    f.fmul_m(k(K_0_75));
    f.fld(fr.f(0x10));
    f.fmul_m(k(K_0_75));
    let v = f.fstp();
    fr.s(0x3C, v);
    f.fxch(1);
    f.fmul_m(k(K_0_75));
    let v = f.fstp();
    fr.s(0x40, v);
    // C = w A + (1 - w) B (0x0C..0x14)
    f.fld(k(K_ONE));
    f.fsub_m(fr.f(0x28));
    let v = f.fst();
    fr.s(0x18, v);
    f.fld(fr.f(0x2C));
    f.fmul_st(1);
    f.fld_st(1);
    f.fmul_m(fr.f(0x30));
    let v = f.fstp();
    fr.s(0x58, v);
    f.fxch(1);
    f.fmul_m(fr.f(0x34));
    let v = f.fstp();
    fr.s(0x5C, v);
    f.fld(fr.f(0x28));
    f.fmul_m(fr.f(0x1C));
    f.fld(fr.f(0x28));
    f.fmul_m(fr.f(0x20));
    let v = f.fstp();
    fr.s(0x30, v);
    f.fld(fr.f(0x28));
    f.fmul_m(fr.f(0x24));
    let v = f.fstp();
    fr.s(0x34, v);
    f.fadd_st(1);
    let v = f.fstp();
    fr.s(0x0C, v);
    f.fstp_st(0);
    f.fld(fr.f(0x30));
    f.fadd_m(fr.f(0x58));
    let v = f.fstp();
    fr.s(0x10, v);
    f.fld(fr.f(0x34));
    f.fadd_m(fr.f(0x5C));
    let v = f.fstp();
    fr.s(0x14, v);
    // source = C - O
    let src = this.byte_add(0x19C);
    f.fld(fr.f(0x0C));
    f.fsub_st(1);
    let v = f.fstp();
    fr.s(0x1C, v);
    wr::<f32>(src, 0, v);
    f.fstp_st(0);
    f.fld(fr.f(0x10));
    f.fsub_m(fr.f(0x3C));
    let v = f.fstp();
    fr.s(0x20, v);
    f.fld(fr.f(0x14));
    f.fsub_m(fr.f(0x40));
    wr::<f32>(src, 4, fr.f(0x20));
    let v = f.fstp();
    fr.s(0x24, v);
    f.fxch(1);
    wr::<f32>(src, 8, v);
    let v = f.fstp();
    fr.s(0x0C, v);
    // front = (w E1 + (1 - w) E2) * (1 - sA) + sA N
    f.fld(fr.f(0x90));
    f.fld(fr.f(0x18));
    f.fld(fr.f(0x84));
    f.fmul_st(1);
    f.fld(fr.f(0x88));
    f.fmul_st(2);
    let v = f.fstp();
    fr.s(0x58, v);
    f.fld(fr.f(0x8C));
    f.fmul_st(2);
    let v = f.fstp();
    fr.s(0x5C, v);
    f.fld(fr.f(0x78));
    f.fmul_m(fr.f(0x28));
    f.fld(fr.f(0x7C));
    f.fmul_m(fr.f(0x28));
    let v = f.fstp();
    fr.s(0x3C, v);
    f.fld(fr.f(0x80));
    f.fmul_m(fr.f(0x28));
    let v = f.fstp();
    fr.s(0x40, v);
    f.fadd_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    f.fld(fr.f(0x3C));
    f.fadd_m(fr.f(0x58));
    f.fld(fr.f(0x40));
    f.fadd_m(fr.f(0x5C));
    let v = f.fstp();
    fr.s(0x5C, v);
    f.fld(k(K_ONE));
    f.fsub_m(fr.f(0x44));
    let v = f.fstp();
    fr.s(0x18, v);
    f.fxch(1);
    f.fmul_m(fr.f(0x18));
    let v = f.fstp();
    fr.s(0x38, v);
    f.fmul_m(fr.f(0x18));
    f.fld(fr.f(0x5C));
    f.fmul_m(fr.f(0x18));
    let v = f.fstp();
    fr.s(0x40, v);
    f.fld(fr.f(0x38));
    f.fadd_m(fr.f(0x0C));
    let v = f.fstp();
    fr.s(0x0C, v);
    f.fadd_st(2);
    let v = f.fstp();
    fr.s(0x10, v);
    f.fld(fr.f(0x40));
    f.fadd_st(1);
    let v = f.fstp();
    fr.s(0x14, v);
    f.fstp_st(0);
    f.fstp_st(0);
    let front = this.byte_add(0x190);
    for i in 0..3 {
        wr::<u32>(front, 4 * i as isize, fr.0[3 + i]);
    }
    vector_normalise(front.cast::<[f32; 3]>());
    let t = pos_of(if rd::<u32>(this, 0x110) != 0 { v1 } else { v2 });
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x148 + o, rd::<u32>(t, o));
    }
    // up = normalise(front x normalise((0, 0, 1) x front))
    let up = this.byte_add(0x1B4);
    fr.u(0x0C, 0);
    fr.u(0x10, 0);
    fr.u(0x14, K_ONE);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<u32>(up, 8, K_ONE);
    vector_normalise(up.cast::<[f32; 3]>());
    cross_product(fr.p(0x94), up, front);
    fr.normalise(0x94);
    let res = cross_product(fr.p(0x38), front, fr.p(0x94));
    for o in [0, 4, 8] {
        wr::<u32>(up, o, rd::<u32>(res, o));
    }
    vector_normalise(up.cast::<[f32; 3]>());
    // the focus switch: H = normalise(front.x, front.y, 0) at 0x48
    let i = rd::<u32>(this, 0x110);
    let player_veh = |n: u32| rd::<GPtr<u8>>(g::<GPtr<u8>>(PLAYER0_PED + n as usize * 0x190).read(), 0x58C);
    let va = player_veh(i);
    let vb = player_veh(i.wrapping_sub(1) & 1);
    fr.copy_from(0x0C, va.byte_add(0x44));
    fr.u(0x50, rd::<u32>(front, 8));
    fr.u(0x48, rd::<u32>(front, 0));
    fr.u(0x4C, rd::<u32>(front, 4));
    fr.u(0x50, 0);
    fr.normalise(0x48);
    f.fld(fr.f(0x0C));
    f.fmul_m(fr.f(0x48));
    f.fld(fr.f(0x14));
    f.fmul_m(fr.f(0x50));
    f.faddp(1);
    f.fld(fr.f(0x4C));
    f.fmul_m(fr.f(0x10));
    f.faddp(1);
    f.fcom_m(k(K_M0_13));
    if f.jp(5) {
        f.fstp_st(0);
    } else {
        f.fld(fr.f(0x48));
        f.fchs();
        f.fld(fr.f(0x4C));
        f.fchs();
        f.fld(fr.f(0x50));
        f.fchs();
        fr.copy_from(0x0C, vb.byte_add(0x44));
        f.fld(fr.f(0x14));
        f.fmul_st(1);
        f.fld(fr.f(0x10));
        f.fmul_st(3);
        f.faddp(1);
        f.fxch(3);
        f.fmul_m(fr.f(0x0C));
        f.faddp(3);
        f.fxch(2);
        f.fxch(1);
        f.fxch(3);
        f.fcompp();
        f.fstp_st(0);
        f.fstp_st(0);
        if !f.jp(5) {
            wr::<u32>(this, 0x110, rd::<u32>(this, 0x110).wrapping_sub(1) & 1);
        }
    }
    // the blend
    f.fld(g::<f32>(TIME_STEP).read());
    f.fmul_m(g::<f32>(SEPARATE_BLEND_RATE).read());
    let blend = if rd::<u32>(this, 0x110) != 0 {
        f.fadd_m(rd::<f32>(this, 0x114));
        f.fcom_m(k(K_ONE));
        if f.jp(5) { k(K_ONE) } else { f.fst() }
    } else {
        f.fsubr_m(rd::<f32>(this, 0x114));
        f.fcom_m(0.0);
        if f.jne(0x41) { 0.0 } else { f.fst() }
    };
    wr::<f32>(this, 0x114, blend);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_TWO_PLAYER_SEPARATE_CARS,
    HOOK_TWO_PLAYER_POINT_SEES_BOTH,
    HOOK_TWO_PLAYER_VECTORS,
    HOOK_GET_TWO_PLAYER_AIM_VECTOR,
    HOOK_PROCESS_TWO_PLAYER_SEPARATE_CARS_TOP_DOWN,
];
