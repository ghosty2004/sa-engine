//! `CObject::ProcessSamSiteBehaviour` (0x5A07D0) and `CObject::Explode` (0x5A1340), batch `frontier2-a14`.
//! Ported from the 1.0 US bytes (gta-reversed `Object.cpp` as a reading aid). Where they differ the bytes
//! win: the SAM site's height test is `pos.z - 2 <= target.z` (x87), the angle difference keeps its
//! extended value for the turn decision while the fire test reads its stored f32; `Explode`'s
//! breakable-object damage point is `(x, z - 1, z)` (the bytes store `z - 1` over the y of the explosion
//! position copy, whose z was never raised: the `+ 0.5` went to a temporary), and its effect offset is
//! rotated by `Multiply3x3` and then moved by the position.

use sa_core::abi::HookReg;
use sa_core::field::ptr_of as p;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::st0;

use super::common::{entity_pos as pos, g, rd, wr, x, xi};
use super::mvpb_world::entity_update_rw_matrix;

use crate::bindings::add_explosion;
use crate::bindings::add_projectile;
use crate::bindings::create_fx_system_bp;
use crate::bindings::find_player_ped;
use crate::bindings::find_player_vehicle;
use crate::bindings::fx_play_and_kill;
use crate::bindings::get_task_jet_pack;
use crate::bindings::get_vehicle_appearance;
use crate::bindings::matrix_copy_ctor;
use crate::bindings::matrix_dtor;
use crate::bindings::multiply3x3;
use crate::bindings::object_damage;
use crate::bindings::physical_add_to_moving_list;
use crate::bindings::vector_add;
use sa_core::bindings::entity_update_rw_frame;
use sa_core::bindings::get_atan_of_xy;
use sa_core::bindings::matrix_mul_vec;
use sa_core::bindings::matrix_set_rotate_z_only;
use sa_core::crt::rand_at;
orig! {
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, v: u8) = thiscall vslot 0x10 => unported;
}

/// `bAircraftCarrierSamSiteDisabled` (0x8D0A24) and `bArea51SamSiteDisabled` (0xBB4A72), bytes.
const SAM_CARRIER_OFF: usize = 0x8D_0A24;
const SAM_AREA51_OFF: usize = 0xBB_4A72;
/// `CTimer::ms_fTimeStep`, `m_snTimeInMilliseconds`, `m_snPreviousTimeInMilliseconds`
const TIME_STEP: usize = 0xB7_CB5C;
const TIME_MS: usize = 0xB7_CB84;
const PREV_TIME_MS: usize = 0xB7_CB78;
/// `g_fxMan`
const FX_MAN: usize = 0xA9_AE80;

/// 0x859948 -1000, 0x858CA0 2, 0x866FC4 2100, 0x858CB8 pi, 0x858CBC 2pi, 0x858CC0 -pi, 0x858C28 1/20,
/// 0x858B50 0, 0x858B4C 1/200, 0x858FE4 pi/2, 0x858BB0 120, 0x858B1C 0.1
const F_M1000: f32 = -1000.0;
const F_2: f32 = 2.0;
const F_2100: f32 = 2100.0;
const F_PI: f32 = f32::from_bits(0x4049_0FDB);
const F_2PI: f32 = f32::from_bits(0x40C9_0FDB);
const F_MPI: f32 = f32::from_bits(0xC049_0FDB);
const F_20TH: f32 = f32::from_bits(0x3D4C_CCCD);
const F_200TH: f32 = f32::from_bits(0x3BA3_D70A);
const F_HALF_PI: f32 = f32::from_bits(0x3FC9_0FDB);
const F_120: f32 = 120.0;
const F_0_1: f32 = f32::from_bits(0x3DCC_CCCD);
/// 0x858B8C 0.5, 0x858624 1.0, 0x85A7D0 0.0002
const F_0_5: f32 = 0.5;
const F_1: f32 = 1.0;
const F_RAND: f32 = f32::from_bits(0x3951_B717);

/// The heading write of `ProcessSamSiteBehaviour` (0x5A0A0A..0x5A0A47): `SetRotateZOnly(h - pi/2)` on the
/// matrix (no null test, as the bytes), the RW matrix, `UpdateRwFrame`.
fn sam_set_heading(this: GPtr<u8>, h: sa_math::x87::X87) {
    matrix_set_rotate_z_only(rd::<GPtr<u8>>(this, 0x14), h.fsub_m(F_HALF_PI).to_f32());
    entity_update_rw_matrix(this);
    entity_update_rw_frame(this);
}

/// `CObject::ProcessSamSiteBehaviour()` (0x5A07D0, thiscall). Broken objects (+0x140 bit 10) do nothing; the
/// carrier's flag (0x8D0A24) applies west of x = -1000, else Area 51's (0xBB4A72). The target is the player's
/// vehicle unless it is a bike or a car (appearance 2 / 1), else the player on a jet pack. A target at most
/// 2 above (`pos.z - 2 <= t.z`) and not north of y = 2100 is tracked: the angle to it minus the heading,
/// wrapped into [-pi, pi] (x87), turns the site by at most `timestep / 20`; closer than 120 in 2D, within 0.1
/// rad and on a new 4-second period of `CTimer` time, a heat-seeking rocket (20) leaves `(0, 2, 4)` in object
/// space along forward + up. Otherwise the site turns by `timestep / 200`.
#[hook(addr = 0x5A07D0, conv = thiscall, name = "CObject::ProcessSamSiteBehaviour", this = "CObject",
       expect = "83 EC 34 53 56 57",
       unverified = "frontier2-a14", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "atan through GetATanOfXY (ST0 kept extended); SAM sites at Area 51 and the carrier")]
pub fn object_process_sam_site_behaviour(this: GPtr<u8>) {
    if rd::<u32>(this, 0x140) & 0x400 != 0 {
        return;
    }
    let me = pos(this);
    let off = if !x(rd::<f32>(me, 0)).fcom(x(F_M1000)).test_parity(0x05) {
        g::<u8>(SAM_CARRIER_OFF).read()
    } else {
        g::<u8>(SAM_AREA51_OFF).read()
    };
    if off != 0 {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let heading = st0(get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14))).to_f32();
    let mut target = GPtr::<u8>::NULL;
    let veh = find_player_vehicle(-1, 0);
    let use_vehicle = !veh.is_null()
        && get_vehicle_appearance(find_player_vehicle(-1, 0)) != 2
        && get_vehicle_appearance(find_player_vehicle(-1, 0)) != 1;
    if use_vehicle {
        target = find_player_vehicle(-1, 0);
    } else if !get_task_jet_pack(rd::<GPtr<u8>>(find_player_ped(-1), 0x47C)).is_null() {
        target = find_player_ped(-1);
    }
    let turn = 'track: {
        if target.is_null() {
            break 'track None;
        }
        let tp = pos(target);
        let me = pos(this);
        if !x(rd::<f32>(me, 8)).fsub_m(F_2).fcom(x(rd::<f32>(tp, 8))).test_any(0x41)
            || !x(rd::<f32>(tp, 4)).fcom(x(F_2100)).test_any(0x41)
        {
            target = GPtr::NULL;
            break 'track None;
        }
        let dx = x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(me, 0)).to_f32();
        let dy = x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(me, 4)).to_f32();
        let a = st0(get_atan_of_xy(dx, dy));
        let angle = a.to_f32();
        let mut diff = a.fsub_m(heading);
        let mut stored = diff.to_f32();
        if !diff.fcom(x(F_PI)).test_any(0x41) {
            while !diff.fcom(x(F_PI)).test_any(0x41) {
                diff = diff.fsub_m(F_2PI);
            }
            stored = diff.to_f32();
        }
        if !diff.fcom(x(F_MPI)).test_parity(0x05) {
            while !diff.fcom(x(F_MPI)).test_parity(0x05) {
                diff = diff.fadd_m(F_2PI);
            }
            stored = diff.to_f32();
        }
        let ts = x(g::<f32>(TIME_STEP).read()).fmul_m(F_20TH).to_f32();
        let abs = if diff.fcom(x(0.0)).test_parity(0x05) { diff } else { diff.fchs() };
        let h = if abs.fcom(x(ts)).test_any(0x41) {
            x(angle)
        } else if diff.fcom(x(0.0)).test_parity(0x05) {
            x(ts).fadd_m(heading)
        } else {
            x(heading).fsub_m(ts)
        };
        Some((h, stored))
    };
    let Some((h, diff)) = turn else {
        sam_set_heading(this, x(g::<f32>(TIME_STEP).read()).fmul_m(F_200TH).fadd_m(heading));
        return;
    };
    sam_set_heading(this, h);
    if target.is_null() {
        return;
    }
    let tp = pos(target);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let me = pos(this);
    let dx = x(rd::<f32>(me, 0)).fsub_m(rd::<f32>(tp, 0));
    let dy = x(rd::<f32>(me, 4)).fsub_m(rd::<f32>(tp, 4));
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    if d.fcom(x(F_120)).test_parity(0x05) {
        return;
    }
    let ad = if x(diff).fcom(x(0.0)).test_parity(0x05) { x(diff) } else { x(diff).fchs() };
    if ad.fcom(x(F_0_1)).test_parity(0x05) {
        return;
    }
    if g::<u32>(TIME_MS).read() / 4000 == g::<u32>(PREV_TIME_MS).read() / 4000 {
        return;
    }
    let mut dir = [0u32; 3];
    vector_add(p(&mut dir), m.byte_add(0x10), m.byte_add(0x20));
    let mut src = [0f32; 3];
    let mut ofs = [0.0f32, 2.0, 4.0];
    matrix_mul_vec(p(&mut src), m, p(&mut ofs));
    add_projectile(this, 20, src[0], src[1], src[2], 0.0, p(&mut dir), target);
}

/// `CObject::Explode()` (0x5A1340, thiscall). An object explosion (type 9, 100 ms, sound, no shake value
/// (-1), invisible flag 0) 0.5 above the position, created by the player. Breakable objects (+0x144 200 /
/// 202) take 10000 explosion damage (weapon 51) at `(x, z - 1, z)` along +z (module docs); others without
/// +0x40 bit 2 get a random push of +-0.0256 in x / y and +0.5 in z and leave the static state
/// (`SetIsStatic(false)`, the moving list). The object info's destroy effect (+0x20 == 2) plays at the
/// rotated offset (+0x24) plus the position.
#[hook(addr = 0x5A1340, conv = thiscall, name = "CObject::Explode", this = "CObject",
       expect = "64 A1 00 00 00 00",
       unverified = "frontier2-a14", route = "L2", hazards = "heap-writes",
       note = "the damage point is (x, z - 1, z) as the bytes (gta-reversed: z - 0.5)")]
pub fn object_explode(this: GPtr<u8>) {
    let pp = pos(this);
    let mut v = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
    let z = x(v[2]).fadd_m(F_0_5).to_f32();
    add_explosion(this, find_player_ped(-1), 9, v[0], v[1], z, 100, 1, -1.0, 0);
    let fx = rd::<u8>(this, 0x144);
    if fx == 0xCA || fx == 0xC8 {
        v[1] = x(v[2]).fsub_m(F_1).to_f32();
        let mut dir = [0.0f32, 0.0, 1.0];
        object_damage(this, 10000.0, p(&mut v), p(&mut dir), this, 0x33);
    } else if rd::<u8>(this, 0x40) & 4 == 0 {
        wr::<f32>(this, 0x4C, x(rd::<f32>(this, 0x4C)).fadd_m(F_0_5).to_f32());
        let r = (rand_at(0x5A13EE) & 0xFF) - 0x80;
        wr::<f32>(this, 0x44, xi(r).fmul_m(F_RAND).fadd_m(rd::<f32>(this, 0x44)).to_f32());
        let r = (rand_at(0x5A1411) & 0xFF) - 0x80;
        let f = rd::<u32>(this, 0x1C);
        wr::<f32>(this, 0x48, xi(r).fmul_m(F_RAND).fadd_m(rd::<f32>(this, 0x48)).to_f32());
        if f & 4 != 0 || f & 0x4_0000 != 0 {
            v_set_is_static(this, 0);
            physical_add_to_moving_list(this);
        }
    }
    let info = rd::<GPtr<u8>>(this, 0x160);
    if rd::<u8>(info, 0x20) != 2 {
        return;
    }
    let mut m = [0u32; 0x12];
    matrix_copy_ctor(p(&mut m), rd::<GPtr<u8>>(this, 0x14));
    let info = rd::<GPtr<u8>>(this, 0x160);
    let mut off = [rd::<u32>(info, 0x24), rd::<u32>(info, 0x28), rd::<u32>(info, 0x2C)];
    let mut out = [0f32; 3];
    multiply3x3(p(&mut out), p(&mut m), p(&mut off));
    let pp = pos(this);
    let mut at = [
        x(out[0]).fadd_m(rd::<f32>(pp, 0)).to_f32(),
        x(out[1]).fadd_m(rd::<f32>(pp, 4)).to_f32(),
        x(out[2]).fadd_m(rd::<f32>(pp, 8)).to_f32(),
    ];
    let bp = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x160), 0x30);
    let sys = create_fx_system_bp(g(FX_MAN), bp, p(&mut at), GPtr::NULL, 0);
    if !sys.is_null() {
        fx_play_and_kill(sys);
    }
    matrix_dtor(p(&mut m));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_OBJECT_PROCESS_SAM_SITE_BEHAVIOUR, HOOK_OBJECT_EXPLODE];
