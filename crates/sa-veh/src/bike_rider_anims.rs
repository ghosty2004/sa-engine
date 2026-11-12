//! `CBike::ProcessRiderAnims` (0x6B7280, batch `writerq-veh-1`): the rider's lean / pitch anims, the stop and
//! wheelie anims and the knock-off checks of a bike (or quad) rider. Not reversed in gta-reversed; ported from the
//! 1.0 US bytes. Every x87 value kept on the register stack is an `X87`, every value stored to the frame an `f32`.
//! Unverified (docs/verify/HOWTO.md).
#![allow(
    clippy::unreadable_literal,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names
)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, rand_at, rd, wr, x, xi};

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CPed::GetBikeRidingSkill()` (0x5DF510, ST0)
    fn ped_get_bike_riding_skill(p: GPtr<u8>) -> f64 = thiscall 0x5DF510 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(CVehicle*, CVector* moveSpeed, CVector* impact, float, float, uint8,
    /// uint8, uint16, CPed*, bool, bool)` (0x4AFCF0, `ret 0x2C`)
    fn kob_ctor(e: GPtr<u8>, veh: GPtr<u8>, speed: GPtr<u8>, impact: GPtr<u8>, dmg: u32, a5: u32, kind: u32, dir: u32, time: u32, ped: u32, driver: u32, force: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CEventKnockOffBike::~CEventKnockOffBike()` (0x4AFE70)
    fn kob_dtor(e: GPtr<u8>) = thiscall 0x4AFE70 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn blend_animation(clump: GPtr<u8>, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    fn add_animation(clump: GPtr<u8>, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`)
    fn assoc_set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// `RtQuatRotate(RtQuat*, const RwV3d* axis, RwReal angle, RwOpCombineType)` (0x7EB7C0)
    fn rt_quat_rotate(q: GPtr<u8>, axis: u32, angle: u32, combine: u32) -> GPtr<u8> = cdecl 0x7EB7C0 => unported;
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::ms_fTimeStepNonClipped`
const TIME_STEP_NC: usize = 0xB7CB58;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

fn rf(p: GPtr<u8>, off: isize) -> f32 {
    rd::<f32>(p, off)
}

fn z0() -> X87 {
    X87::zero()
}

/// `fld v; fcomp m; fnstsw; test ah, mask` then `jp` taken.
fn jp(v: f32, m: f32, mask: u8) -> bool {
    x(v).fcom(x(m)).test_parity(mask)
}

/// `fld v; fcomp m; fnstsw; test ah, mask` then `jne` taken.
fn jne(v: f32, m: f32, mask: u8) -> bool {
    x(v).fcom(x(m)).test_any(mask)
}

/// `_CIpow(base, exp)` (0x822130) with base in ST1 and the exponent in ST0.
fn pow(base: f32, exp: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(
        0x822130,
        x(base).to_f64(),
        x(exp).to_f64(),
    ))
}

/// The knock-off events: one `CEventKnockOffBike` (impact (0, 0, 1), type 0x36, direction 2, "is rider" 1) for the
/// rider, and when the vehicle's +0x464 is set and is not the rider, one ("is rider" 0) for that ped; each added to the
/// ped's event group (intelligence +0x68) and destroyed.
fn knock_off(rider: GPtr<u8>, veh: GPtr<u8>) {
    let mut ev = [0u32; 0x40 / 4];
    let mut impact = [0f32, 0.0, 1.0];
    let evp: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    kob_ctor(
        evp,
        veh,
        veh.byte_add(0x44),
        GPtr::from_raw(impact.as_mut_ptr()).cast(),
        0,
        0,
        0x36,
        2,
        0,
        0,
        1,
        0,
    );
    event_group_add(rd::<GPtr<u8>>(rider, 0x47C).byte_add(0x68), evp, 0);
    let other = rd::<GPtr<u8>>(veh, 0x464);
    if !other.is_null() && other != rider {
        let mut ev2 = [0u32; 0x40 / 4];
        let mut impact2 = [0f32, 0.0, 1.0];
        let evp2: GPtr<u8> = GPtr::from_raw(ev2.as_mut_ptr()).cast();
        kob_ctor(
            evp2,
            veh,
            veh.byte_add(0x44),
            GPtr::from_raw(impact2.as_mut_ptr()).cast(),
            0,
            0,
            0x36,
            2,
            0,
            0,
            0,
            0,
        );
        event_group_add(
            rd::<GPtr<u8>>(rd::<GPtr<u8>>(veh, 0x464), 0x47C).byte_add(0x68),
            evp2,
            0,
        );
        kob_dtor(evp2);
    }
    kob_dtor(evp);
}

/// `assoc` blended in with `delta` unless it is fully blended (amount not below 1) or already blending in (delta above 0).
fn blend_in(assoc: GPtr<u8>, clump: GPtr<u8>, group: u32, id: u32, delta: f32) -> GPtr<u8> {
    if !assoc.is_null() && (jp(rf(assoc, 0x18), 1.0, 0x05) || jp(rf(assoc, 0x1C), 0.0, 0x41)) {
        return assoc;
    }
    blend_animation(clump, group, id, delta.to_bits())
}

/// `blend = f32(blend - min(timeStepNC * 0.02 * delta + amount, blend))` (the register value is replaced by `blend`
/// when `blend` is below it).
fn take_blend(assoc: GPtr<u8>, blend: f32) -> f32 {
    let mut v = x(g::<f32>(TIME_STEP_NC).read())
        .fmul_m(0.02)
        .fmul_m(rf(assoc, 0x1C))
        .fadd_m(rf(assoc, 0x18));
    if !x(blend).fcom(v).test_parity(0x05) {
        v = x(blend);
    }
    x(blend).fsub(v).to_f32()
}

/// One lean / pitch anim pair: the positive anim gets `amount` and the time `len * v`, the other amount 0; for `v`
/// below 0 (or unordered) the negative anim gets `amount` and the time `-(len * v)`. Both lose flag bit 0 of +0x2E.
fn set_pair(pos: GPtr<u8>, neg: GPtr<u8>, v: f32, amount: f32) {
    let (on, off, t) = if jne(v, 0.0, 0x01) {
        (neg, pos, x(rf(rd::<GPtr<u8>>(neg, 0x14), 0x10)).fmul_m(v).fchs())
    } else {
        (pos, neg, x(rf(rd::<GPtr<u8>>(pos, 0x14), 0x10)).fmul_m(v))
    };
    wr::<f32>(on, 0x18, amount);
    assoc_set_current_time(on, f32_of(t).to_bits());
    wr::<u8>(on, 0x2E, rd::<u8>(on, 0x2E) & 0xFE);
    wr::<u32>(off, 0x18, 0);
}

/// `CBike::ProcessRiderAnims(CPed* rider, CVehicle* vehicle, CRideAnimData* ride, tBikeHandlingData* handling,
/// int16)` (0x6B7280, cdecl, SEH frame for the stack events). x87.
///
/// Vehicle type (+0x590) 9: bike = vehicle, wheelie state = byte +0x804; type 0: quad = vehicle, state = byte +0x960.
/// Associations 0xC4/0xC5 (lean), 0xC3 (stop), 0xC7/0xC6 (pitch, player only), 0xC8 (back), the first of
/// 0xCA/0xCB/0xCC (still). fwd = f32((ms.z*f.z + ms.y*f.y) + ms.x*f.x) (move speed · forward). Subtype (+0x594) 10 with
/// the 0xC8 or the still anim: byte +0x7B5 = 0. Turn speed |w|² above ((skill + 1) * 0.3)² → knock off.
///
/// No still anim and |fwd| below 0.02: the stop anim blended in (group `ride[0]`, delta 2). Else fwd below 0: with
/// s = (skill + 1) * 3.5 (doubled for subtype 2) times handling +0x8C above fwd, a wheelie state above 0, or forward
/// z (+0x28) below -0.5 within 100 ms of the vehicle's collision time (+0x3C) → knock off. Otherwise a negative steer
/// (+0x49C) with 1.5 * handling +0x8C below fwd blends in the back anim (delta 4); else a player whose fwd is below
/// that marks the lean control -1. Stop / back anims not already fading out get delta -4.
///
/// blend = 1 - min(1, timeStepNC*0.02*delta + amount of the stop anim), then less the same of the still and the back
/// anims (each clamped to blend). Lean target = f32(ride[+8] / handling[+0x18]) clamped to [-1, 1], 0 under the -1
/// control; ride[+0x14] = (1 - p)*target + p*ride[+0x14] with p = 0.86^timeStep (`_CIpow`).
///
/// Pitch control (player with no vehicle +0x464, control not -1): starts at ride[+0x10]; the bike's +0x614 bit 7 is
/// cleared, then set for a wheelie (front wheel ratios +0x730/+0x734 off the ground, forward z above 0, a rear ratio
/// above 0: handling +0x2C - z below half of +0x2C) or a stoppie (rear off, forward z below 0, a front ratio above 0:
/// +0x30 - z above 0.6 * +0x30, the bit only). A quad checks its +0x7F4..+0x800 ratios the same way (wheelie only).
/// A wheelie with +0x2C - z below 0.15 raises the control to at least 0.25; else gas (+0x4A0) above 0.5 at fwd above
/// 0.01 raises it to at least 0.1; else brake (+0x49C) above 0.5 with the control not above 0 and 0.3 * handling
/// +0x88 above fwd lowers it to at most -0.3. With |target| above 0.3 the control is scaled by
/// max(0, 1 - (|t| - 0.3) / (0.56 - 0.3)). ride[+0x18] = (1 - p)*control + p*ride[+0x18], p = 0.89^timeStep (0 for a
/// non-player). Otherwise the control is 0 and the bike's bit 7 is cleared.
///
/// Anim weights: |ride+0x14| above 0.56 or a non-player: lean blend, pitch 0; |ride+0x18| above 0.56: pitch blend, lean
/// 0; else the normalised (lean, pitch) magnitudes (when the length exceeds 0.01; else the raw magnitudes) times blend.
/// Player: pitch anims 0xC7 (forward) / 0xC6 (backward) added when missing and set by `set_pair`; then the lean anims
/// 0xC5 (right) / 0xC4 (left). fwd above 0.3: the rider's frame quaternion ([+0x490] -> +0x10) is rotated about x then
/// y by `rand()`-picked angles in [-6 fwd, 6 fwd] (combine 2), and ped flag 0x4000 of +0x470 is set.
#[hook(addr = 0x6B7280, conv = cdecl, name = "CBike::ProcessRiderAnims", expect = "6A FF 68 99 82 84 00",
       unverified = "writerq-veh-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "driving",
       note = "rand sites 0x6B7EDD / 0x6B7F28; `_CIpow` 0x822130 via x87_binary; knock-off events on the Rust stack")]
pub fn bike_process_rider_anims(
    rider: GPtr<u8>,
    veh: GPtr<u8>,
    ride: GPtr<u8>,
    handling: GPtr<u8>,
    _a5: i32,
) {
    let is_player = ped_is_player(rider) != 0;
    let mut bike = GPtr::<u8>::NULL;
    let mut quad = GPtr::<u8>::NULL;
    let mut wheelie_state: i16 = 0;
    match rd::<u32>(veh, 0x590) {
        9 => {
            bike = veh;
            wheelie_state = i16::from(rd::<u8>(veh, 0x804));
        }
        0 => {
            quad = veh;
            wheelie_state = i16::from(rd::<u8>(veh, 0x960));
        }
        _ => {}
    }
    let clump = || rd::<GPtr<u8>>(rider, 0x18);
    let group = || rd::<u32>(ride, 0);
    let mut blend = 1.0f32;
    let mut control = 0.0f32;
    let a_c4 = clump_get_association(clump(), 0xC4);
    let a_c5 = clump_get_association(clump(), 0xC5);
    let mut a_c3 = clump_get_association(clump(), 0xC3);
    let (mut a_c7, mut a_c6) = (GPtr::<u8>::NULL, GPtr::<u8>::NULL);
    if is_player {
        a_c7 = clump_get_association(clump(), 0xC7);
        a_c6 = clump_get_association(clump(), 0xC6);
    }
    let mut a_c8 = clump_get_association(clump(), 0xC8);
    let mut a_still = clump_get_association(clump(), 0xCA);
    if a_still.is_null() {
        a_still = clump_get_association(clump(), 0xCB);
        if a_still.is_null() {
            a_still = clump_get_association(clump(), 0xCC);
        }
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let fwd = x(rf(veh, 0x4C))
        .fmul_m(rf(m, 0x18))
        .fadd(x(rf(veh, 0x48)).fmul_m(rf(m, 0x14)))
        .fadd(x(rf(veh, 0x44)).fmul_m(rf(m, 0x10)))
        .to_f32();
    if rd::<u32>(veh, 0x594) == 10 && (!a_c8.is_null() || !a_still.is_null()) {
        wr::<u8>(veh, 0x7B5, 0);
    }
    let t = X87::from_f64_exact(ped_get_bike_riding_skill(rider))
        .fadd_m(1.0)
        .fmul_m(0.3);
    let (wx, wy, wz) = (x(rf(veh, 0x50)), x(rf(veh, 0x54)), x(rf(veh, 0x58)));
    let w2 = wx.fmul(wx).fadd(wy.fmul(wy)).fadd(wz.fmul(wz));
    if !t.fmul(t).fcom(w2).test_parity(0x05) {
        knock_off(rider, veh);
    }

    'anims: {
        if a_still.is_null() && !x(fwd).fabs().fcom(x(0.02)).test_parity(0x05) {
            a_c3 = blend_in(a_c3, clump(), group(), 0xC3, 2.0);
            break 'anims;
        }
        if !jp(fwd, 0.0, 0x05) {
            let mut s = X87::from_f64_exact(ped_get_bike_riding_skill(rider))
                .fadd_m(1.0)
                .fmul_m(3.5);
            if rd::<u32>(veh, 0x594) == 2 {
                s = s.fadd(s);
            }
            let h2 = rd::<GPtr<u8>>(veh, 0x384);
            let knock = !s.fmul_m(rf(h2, 0x8C)).fcom(x(fwd)).test_any(0x41)
                && (wheelie_state > 0
                    || (!jp(rf(rd::<GPtr<u8>>(veh, 0x14), 0x28), -0.5, 0x05)
                        && g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(veh, 0x3C)) < 100));
            if knock {
                knock_off(rider, veh);
                break 'anims;
            }
            let back_limit = || x(rf(h2, 0x8C)).fmul(X87::from_f64_exact(1.5));
            if !jp(rf(veh, 0x49C), 0.0, 0x05) && !back_limit().fcom(x(fwd)).test_parity(0x05) {
                a_c8 = blend_in(a_c8, clump(), group(), 0xC8, 4.0);
                break 'anims;
            }
            if is_player && !back_limit().fcom(x(fwd)).test_any(0x41) {
                control = -1.0;
            }
        }
        for a in [a_c3, a_c8] {
            if !a.is_null() && !jne(rf(a, 0x1C), 0.0, 0x01) {
                wr::<u32>(a, 0x1C, 0xC080_0000);
            }
        }
    }

    if !a_c3.is_null() {
        let mut v = x(g::<f32>(TIME_STEP_NC).read())
            .fmul_m(0.02)
            .fmul_m(rf(a_c3, 0x1C))
            .fadd_m(rf(a_c3, 0x18));
        if !X87::one().fcom(v).test_parity(0x05) {
            v = X87::one();
        }
        blend = X87::one().fsub(v).to_f32();
    }
    if !a_still.is_null() {
        blend = take_blend(a_still, blend);
    }
    if !a_c8.is_null() {
        blend = take_blend(a_c8, blend);
    }

    let mut target = x(rf(ride, 8)).fdiv_m(rf(handling, 0x18)).to_f32();
    if !x(control).fcom(x(-1.0)).test_parity(0x44) {
        target = 0.0;
    } else if !jne(target, 1.0, 0x41) {
        target = 1.0;
    } else if !jp(target, -1.0, 0x05) {
        target = -1.0;
    }
    let p = pow(0.86, g::<f32>(TIME_STEP).read());
    let lean = X87::one().fsub(p).fmul_m(target).fadd(p.fmul_m(rf(ride, 0x14)));
    wr::<f32>(ride, 0x14, lean.to_f32());

    if !is_player || !rd::<GPtr<u8>>(veh, 0x464).is_null() {
        control = 0.0;
        if !bike.is_null() {
            wr::<u8>(bike, 0x614, rd::<u8>(bike, 0x614) & 0x7F);
        }
    } else if !jne(control, -1.0, 0x41) {
        control = rf(ride, 0x10);
        // The wheelie height margin left on the register stack for the 0.15 test (None: no wheelie).
        let mut margin: Option<X87> = None;
        if !bike.is_null() {
            let mut cl = rd::<u8>(bike, 0x614) & 0x7F;
            wr::<u8>(bike, 0x614, cl);
            let fz = || rf(rd::<GPtr<u8>>(bike, 0x14), 0x18);
            let pos = |off: isize| !jne(rf(bike, off), 0.0, 0x41);
            let front_up = !jp(rf(bike, 0x730), 0.0, 0x41) && !jp(rf(bike, 0x734), 0.0, 0x41);
            if front_up
                && !jne(fz(), 0.0, 0x41)
                && (pos(0x738) || !x(rf(bike, 0x73C)).fcom(X87::zero()).test_any(0x41))
            {
                let a = x(rf(handling, 0x2C)).fsub_m(fz());
                let half = x(rf(handling, 0x2C)).fmul_m(0.5);
                if !a.fcom(half).test_parity(0x05) {
                    cl |= 0x80;
                    wr::<u8>(bike, 0x614, cl);
                }
                margin = Some(a);
            } else if !jp(rf(bike, 0x738), 0.0, 0x41)
                && !jp(rf(bike, 0x73C), 0.0, 0x41)
                && !jp(fz(), 0.0, 0x05)
                && (pos(0x730) || pos(0x734))
            {
                let a = x(rf(handling, 0x30)).fsub_m(fz());
                let lim = x(rf(handling, 0x30)).fmul_m(0.6);
                if !lim.fcom(a).test_parity(0x05) {
                    wr::<u8>(bike, 0x614, cl | 0x80);
                }
            }
        } else if !quad.is_null() {
            let fz = rf(rd::<GPtr<u8>>(quad, 0x14), 0x18);
            if !jp(rf(quad, 0x7F4), 0.0, 0x41)
                && !jp(rf(quad, 0x7F8), 0.0, 0x41)
                && !jne(fz, 0.0, 0x41)
                && (!jne(rf(quad, 0x7FC), 0.0, 0x41) || !jne(rf(quad, 0x800), 0.0, 0x41))
            {
                margin = Some(x(rf(handling, 0x2C)).fsub_m(fz));
            }
        }
        if margin.is_some_and(|a| !a.fcom(x(0.15)).test_parity(0x05)) {
            if jne(control, 0.25, 0x41) {
                control = 0.25;
            }
        } else if !jne(rf(veh, 0x4A0), 0.5, 0x41) && !jne(fwd, 0.01, 0x41) {
            if jne(control, 0.1, 0x41) {
                control = 0.1;
            }
        } else if !jne(rf(veh, 0x49C), 0.5, 0x41)
            && !jp(control, 0.0, 0x41)
            && !x(rf(rd::<GPtr<u8>>(veh, 0x384), 0x88))
                .fmul_m(0.3)
                .fcom(x(fwd))
                .test_any(0x41)
            && jp(control, -0.3, 0x05)
        {
            control = -0.3;
        }
        let l = x(target).fabs();
        if !l.fcom(x(0.3)).test_any(0x41) {
            let mut f = l.fsub_m(0.3).fdiv(x(0.56).fsub_m(0.3)).fsubr(X87::one());
            if !z0().fcom(f).test_any(0x41) {
                f = z0();
            }
            control = f.fmul_m(control).to_f32();
        }
    }
    if is_player {
        let p = pow(0.89, g::<f32>(TIME_STEP).read());
        wr::<f32>(
            ride,
            0x18,
            X87::one()
                .fsub(p)
                .fmul_m(control)
                .fadd(p.fmul_m(rf(ride, 0x18)))
                .to_f32(),
        );
    } else {
        wr::<u32>(ride, 0x18, 0);
    }

    let (r14, r18) = (rf(ride, 0x14), rf(ride, 0x18));
    // (pitch weight, lean weight) before the blend.
    let (wp, wl) = if !x(r14).fabs().fcom(x(0.56)).test_any(0x41) || !is_player {
        (z0(), X87::one())
    } else if !x(r18).fabs().fcom(x(0.56)).test_any(0x41) {
        (X87::one(), z0())
    } else {
        let b = x(r18);
        let len = x(r14).fmul_m(r14).fadd(b.fmul(b)).fsqrt();
        if len.fcom(x(0.01)).test_any(0x41) {
            (b.fabs(), x(r14).fabs())
        } else {
            let inv = X87::one().fdiv(len).to_f32();
            (b.fmul_m(inv).fabs(), x(inv).fmul_m(r14).fabs())
        }
    };
    let pitch_amount = wp.fmul_m(blend).to_f32();
    let lean_amount = wl.fmul_m(blend).to_f32();
    if is_player {
        if a_c7.is_null() {
            a_c7 = add_animation(clump(), group(), 0xC7);
        }
        if a_c6.is_null() {
            a_c6 = add_animation(clump(), group(), 0xC6);
        }
        set_pair(a_c7, a_c6, rf(ride, 0x18), pitch_amount);
    }
    let a_c4 = if a_c4.is_null() {
        add_animation(clump(), group(), 0xC4)
    } else {
        a_c4
    };
    let a_c5 = if a_c5.is_null() {
        add_animation(clump(), group(), 0xC5)
    } else {
        a_c5
    };
    set_pair(a_c5, a_c4, rf(ride, 0x14), lean_amount);

    if !jne(fwd, 0.3, 0x41) {
        let q = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rider, 0x490), 0x10);
        for (site, axis) in [(0x6B_7EDD_u32, 0x8D232C_u32), (0x6B_7F28, 0x8D2338)] {
            let hi = x(6.0).fmul_m(fwd);
            let hi_f = hi.to_f32();
            let lo = hi.fchs().to_f32();
            let r = xi(rand_at(site)).fmul_m(f32::from_bits(0x3800_0100));
            let angle = x(hi_f).fsub_m(lo).fmul(r).fadd_m(lo).to_f32();
            rt_quat_rotate(q, axis, angle.to_bits(), 2);
        }
        wr::<u32>(rider, 0x470, rd::<u32>(rider, 0x470) | 0x4000);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_BIKE_PROCESS_RIDER_ANIMS];
