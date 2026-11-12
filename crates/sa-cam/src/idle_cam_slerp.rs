//! WRITER-NATIVE (batch `native-18`): `CIdleCam::ProcessSlerp` (0x5179E0), reached once the player stands
//! idle long enough for the idle camera. Ported from the 1.0 US bytes; gta-reversed (`IdleCam.cpp`) as a
//! reading aid (the bytes win: the look-at position is always computed). x87 arithmetic follows the original's
//! instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CIdleCam::GetLookAtPositionOnTarget(CEntity*, CVector*)` (0x50EAE0)
    fn get_look_at_position_on_target(this: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>) = thiscall 0x50EAE0 => crate::idlecam::get_look_at_position_on_target;
    /// `CGeneral::GetATanOfXY(float x, float y)` (0x53CC70, ST0)
    fn general_get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    #[allow(clippy::too_many_arguments)]
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
}

const PI: u32 = 0x4049_0FDB;
const TWO_PI: u32 = 0x40C9_0FDB;
const NEG_PI: u32 = 0xC049_0FDB;

fn atan(x_: f32, y: f32) -> X87 {
    X87::from_f64_exact(general_get_atan_of_xy(x_, y))
}

/// `(heading = atan(v.x, v.y) + pi, pitch = atan(|v.xy|, v.z))`, both rounded to f32 as the original stores them,
/// except that the second call's pitch is returned on the x87 stack (`keep_pitch`).
fn angles(v: [f32; 3]) -> (f32, X87) {
    let heading = atan(v[0], v[1]).fadd_m(f32::from_bits(PI)).to_f32();
    let r = x(v[0]).fmul_m(v[0]).fadd(x(v[1]).fmul_m(v[1])).fsqrt().to_f32();
    (heading, atan(r, v[2]))
}

/// `to` moved by 2 pi towards `from` when `to - from` leaves [-pi, pi] (ordered compares).
fn unwrap(to: X87, from: f32) -> X87 {
    let d = to.fsub_m(from);
    if !d.fcom(x(f32::from_bits(PI))).test_any(0x41) {
        to.fsub_m(f32::from_bits(TWO_PI))
    } else if !d.fcom(x(f32::from_bits(NEG_PI))).test_parity(0x05) {
        to.fadd_m(f32::from_bits(TWO_PI))
    } else {
        to
    }
}

/// `float CIdleCam::ProcessSlerp(float& outX, float& outZ)` (0x5179E0, `ret 8`, ST0).
///
/// `GetLookAtPositionOnTarget(target +0, &lookAt)`, replaced by the last idle position (+0x18) once the LOS
/// counter (+0x54) reaches its limit (+0x50). Heading / pitch of `slerpFrom (+4) - camSource` and `lookAt -
/// camSource` (camera +0x98, source +0x19C); the target angles are unwrapped by 2 pi towards the start ones.
/// `t = (time - slerpStart (+0x34)) / duration (+0x14)`, at most 1 (ordered); `s = (sin((270 - 180 t) deg) + 1)
/// / 2`; `outX = pitch0 + (pitch1 - pitch0) s`, `outZ = heading0 + (heading1 - heading0) s`. Returns `t` (the
/// f32 the original keeps in its local).
#[hook(addr = 0x5179E0, conv = thiscall, name = "CIdleCam::ProcessSlerp", this = "CIdleCam",
       expect = "A1 84 CB B7 00 DB 05 84 CB B7 00",
       unverified = "native-18", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "fsin, fsqrt; GetATanOfXY x4")]
pub fn idle_cam_process_slerp(this: GPtr<u8>, out_x: GPtr<f32>, out_z: GPtr<f32>) -> X87 {
    let now = g::<u32>(0xB7_CB84).read();
    let mut tf = X87::from_i32(now as i32);
    if (now as i32) < 0 {
        tf = tf.fadd_m(f32::from_bits(0x4F80_0000));
    }
    let begin = tf.to_f32();
    let mut look = [0f32; 3];
    get_look_at_position_on_target(this, rd::<GPtr<u8>>(this, 0), GPtr::from_raw(look.as_mut_ptr()).cast());
    if rd::<i32>(this, 0x54) >= rd::<i32>(this, 0x50) {
        look = [rd::<f32>(this, 0x18), rd::<f32>(this, 0x1C), rd::<f32>(this, 0x20)];
    }
    let cam = rd::<GPtr<u8>>(this, 0x98);
    let src = [rd::<f32>(cam, 0x19C), rd::<f32>(cam, 0x1A0), rd::<f32>(cam, 0x1A4)];
    let v1 = [
        x(rd::<f32>(this, 4)).fsub_m(src[0]).to_f32(),
        x(rd::<f32>(this, 8)).fsub_m(src[1]).to_f32(),
        x(rd::<f32>(this, 0xC)).fsub_m(src[2]).to_f32(),
    ];
    let v2 = [
        x(look[0]).fsub_m(src[0]).to_f32(),
        x(look[1]).fsub_m(src[1]).to_f32(),
        x(look[2]).fsub_m(src[2]).to_f32(),
    ];
    let (a1, b1) = angles(v1);
    let b1 = b1.to_f32();
    let a2 = atan(v2[0], v2[1]).fadd_m(f32::from_bits(PI)).to_f32();
    let r2 = x(v2[1]).fmul_m(v2[1]).fadd(x(v2[0]).fmul_m(v2[0])).fsqrt().to_f32();
    let b2 = unwrap(atan(r2, v2[2]), b1);
    let a2 = unwrap(x(a2), a1);
    let tx = x(begin).fsub_m(rd::<f32>(this, 0x34)).fdiv_m(rd::<f32>(this, 0x14));
    let mut t = tx.to_f32();
    if !tx.fcom(x(1.0)).test_any(0x41) {
        t = 1.0;
    }
    let s = x(t)
        .fmul_m(180.0)
        .fsubr_m(270.0)
        .fmul_m(f32::from_bits(0x3C8E_FA35))
        .fsin()
        .fadd_m(1.0)
        .fmul_m(0.5);
    out_x.write(b2.fsub_m(b1).fmul(s).fadd_m(b1).to_f32());
    out_z.write(a2.fsub_m(a1).fmul(s).fadd_m(a1).to_f32());
    x(t)
}

/// `void CIdleCam::ProcessFOVZoom(float slerpTime)` (0x517BF0, `ret 4`).
///
/// With a target (+0): the camera source's distance to its look-at position; a ped target in state 0x16 / 5
/// halves the nearest zoom (+0x68) and wants a zoom-in, flagging +0x78 when closer than 8; any target farther than
/// +0x48 wants a zoom-in. Once the slerp is done (`slerpTime >= 1`, ordered): a wanted zoom-in more than +0x7C after
/// the last one (+0x80) checks the line of sight (ignoring the target) when there is a target; a +0x54 counter above
/// 10 turns zoomed-in (2) into zooming-out (1); zoomed-out (3) with +0x79 clear and a clear view starts zooming in
/// (0) to the nearest zoom. Without a wanted zoom-in, zoomed-in starts zooming out to the farthest (+0x6C). A new
/// zoom records from = current FOV (+0x5C), to (+0x60), start time (+0x64). Zoomed-in stamps +0x80; flag +0x78 with
/// zoomed-in forces zooming out. Then by state: zooming in/out move +0x70 by `(sin((270 - 180 t) deg) + 1) / 2`
/// over +0x74 until within 1 degree (then zoomed in with +0x79 set / zoomed out), zoomed in/out hold the nearest /
/// farthest zoom. The camera's FOV (+0x98 -> +0xB4) = +0x70.
#[hook(addr = 0x517BF0, conv = thiscall, name = "CIdleCam::ProcessFOVZoom", this = "CIdleCam",
       expect = "A1 84 CB B7 00 DB 05 84 CB B7 00",
       unverified = "native-18", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "fsin, fsqrt; reached after ~2 minutes of standing still")]
pub fn idle_cam_process_fov_zoom(this: GPtr<u8>, slerp_time: f32) {
    let now = g::<u32>(0xB7_CB84).read();
    let mut tf = X87::from_i32(now as i32);
    if (now as i32) < 0 {
        tf = tf.fadd_m(f32::from_bits(0x4F80_0000));
    }
    let time = tf.to_f32();
    let mut nearest = rd::<f32>(this, 0x68);
    let mut wants = false;
    let target = rd::<GPtr<u8>>(this, 0);
    let mut look = [0f32; 3];
    let lookp: GPtr<u8> = GPtr::from_raw(look.as_mut_ptr()).cast();
    if !target.is_null() {
        get_look_at_position_on_target(this, target, lookp);
        let src = rd::<GPtr<u8>>(this, 0x98).byte_add(0x19C);
        let dx = x(rd::<f32>(src, 0)).fsub_m(rd::<f32>(lookp, 0));
        let dy = x(rd::<f32>(src, 4)).fsub_m(rd::<f32>(lookp, 4));
        let dz = x(rd::<f32>(src, 8)).fsub_m(rd::<f32>(lookp, 8));
        let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        let t = rd::<GPtr<u8>>(this, 0);
        if rd::<u8>(t, 0x36) & 7 == 3 {
            let st = rd::<u32>(t, 0x598);
            if st == 0x16 || st == 5 {
                nearest = x(nearest).fmul_m(0.5).to_f32();
                wants = true;
                if !dist.fcom(x(8.0)).test_parity(0x05) {
                    wr::<u8>(this, 0x78, 1);
                }
            }
        }
        if !dist.fcom(x(rd::<f32>(this, 0x48))).test_any(0x41) {
            wants = true;
        }
    }
    if !x(slerp_time).fcom(x(1.0)).test_any(0x01) {
        let old = rd::<u32>(this, 0x58);
        if wants {
            if !x(time).fsub_m(rd::<f32>(this, 0x80)).fcom(x(rd::<f32>(this, 0x7C))).test_any(0x41) {
                let mut clear = 1u8;
                let t = rd::<GPtr<u8>>(this, 0);
                if !t.is_null() {
                    let saved = g::<u32>(0xB7_CD68).read();
                    g::<u32>(0xB7_CD68).write(t.addr() as u32);
                    get_look_at_position_on_target(this, rd::<GPtr<u8>>(this, 0), lookp);
                    let src = rd::<GPtr<u8>>(this, 0x98).byte_add(0x19C);
                    clear = world_get_is_line_of_sight_clear(src, lookp, 1, 0, 0, 1, 0, 0, 1);
                    g::<u32>(0xB7_CD68).write(saved);
                }
                if rd::<i32>(this, 0x54) > 10 && rd::<u32>(this, 0x58) == 2 {
                    wr::<u32>(this, 0x58, 1);
                }
                if rd::<u32>(this, 0x58) == 3 && rd::<u8>(this, 0x79) == 0 && clear != 0 {
                    wr::<u32>(this, 0x58, 0);
                    wr::<f32>(this, 0x60, nearest);
                    if old != 0 {
                        wr::<f32>(this, 0x64, time);
                        wr::<u32>(this, 0x5C, rd::<u32>(this, 0x70));
                    }
                }
            }
        } else if old == 2 {
            wr::<u32>(this, 0x58, 1);
            wr::<u32>(this, 0x60, rd::<u32>(this, 0x6C));
            wr::<f32>(this, 0x64, time);
            wr::<u32>(this, 0x5C, rd::<u32>(this, 0x70));
        }
    }
    let state = rd::<u32>(this, 0x58);
    if state == 2 {
        wr::<f32>(this, 0x80, time);
    }
    if rd::<u8>(this, 0x78) != 0 && state == 2 {
        wr::<f32>(this, 0x64, time);
        wr::<u32>(this, 0x5C, rd::<u32>(this, 0x70));
        wr::<u32>(this, 0x58, 1);
        wr::<u32>(this, 0x60, rd::<u32>(this, 0x6C));
    }
    let state = rd::<u32>(this, 0x58);
    wr::<u8>(this, 0x78, 0);
    let interpolate = |this: GPtr<u8>| {
        let s = x(time)
            .fsub_m(rd::<f32>(this, 0x64))
            .fdiv_m(rd::<f32>(this, 0x74))
            .fmul_m(180.0)
            .fsubr_m(270.0)
            .fmul_m(f32::from_bits(0x3C8E_FA35))
            .fsin()
            .fadd_m(1.0)
            .fmul_m(0.5);
        let v = x(rd::<f32>(this, 0x60)).fsub_m(rd::<f32>(this, 0x5C)).fmul(s).fadd_m(rd::<f32>(this, 0x5C));
        wr::<f32>(this, 0x70, v.to_f32());
    };
    match state {
        0 => {
            if x(rd::<f32>(this, 0x70)).fsub_m(nearest).fabs().fcom(x(1.0)).test_parity(0x05) {
                interpolate(this);
            } else {
                wr::<u32>(this, 0x58, 2);
                wr::<u8>(this, 0x79, 1);
                wr::<f32>(this, 0x70, nearest);
            }
        }
        1 => {
            if x(rd::<f32>(this, 0x70)).fsub_m(rd::<f32>(this, 0x6C)).fabs().fcom(x(1.0)).test_parity(0x05) {
                interpolate(this);
            } else {
                let f = rd::<u32>(this, 0x6C);
                wr::<u32>(this, 0x58, 3);
                wr::<u32>(this, 0x70, f);
            }
        }
        2 => wr::<f32>(this, 0x70, nearest),
        3 => wr::<u32>(this, 0x70, rd::<u32>(this, 0x6C)),
        _ => {}
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 0x98), 0xB4, rd::<u32>(this, 0x70));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IDLE_CAM_PROCESS_SLERP, HOOK_IDLE_CAM_PROCESS_FOV_ZOOM];
