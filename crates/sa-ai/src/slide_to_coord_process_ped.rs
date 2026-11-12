//! `CTaskSimpleSlideToCoord::ProcessPed` (0x66C4E0, batch `std14-ai-2`; reached by attractor users in window runs).
//! Ported from the 1.0 US bytes (gta-reversed's body is unhooked, "untested/review"). Unverified
//! (docs/verify/HOWTO.md).
//!
//! Layout used: +0x64 the slide target, +0x70 the aimed heading, +0x74 the speed, +0x78 flags (0x1 first time, 0x2
//! running the named anim), +0x7C the end time (-1 until started).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimpleRunNamedAnim::ProcessPed(CPed*)` (0x61BF20)
    fn run_named_anim_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x61BF20 => unported;
    /// `CTaskSimpleStandStill(int ms, bool looped, bool useAnimIdle, float blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: u32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleStandStill::ProcessPed(CPed*)` (0x62F370)
    fn stand_still_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x62F370 => unported;
    /// `CTaskSimpleStandStill::~CTaskSimpleStandStill()` (0x62F360)
    fn stand_still_dtor(t: GPtr<u8>) = thiscall 0x62F360 => unported;
    /// `CTask::MakeAbortable(CPed*, int, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, pri: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
}

const TIME_MS: usize = 0xB7CB84;

/// `CTaskSimpleSlideToCoord::ProcessPed(CPed*)` (0x66C4E0, thiscall, `ret 4`, AL). The named anim (flag 0x2) runs
/// first; the end time starts at now + 2000 without the anim, now + 500 once the anim reports done. The first call
/// runs a stack `CTaskSimpleStandStill(20000 (0x86DB24), 0, 0, 8.0)` on the ped, asks a player's default primary task
/// to abort (priority 2) and sets the aimed heading. The ped's local move shift (+0x4D8 right, +0x4DC forward) is
/// the speed-scaled offset to the target projected on the ped's axes, or zero within 0.05 (2D). True once the end
/// time passed, or with the anim done, within 0.05 and the heading within 0.1 of the aim.
#[hook(addr = 0x66C4E0, conv = thiscall, name = "CTaskSimpleSlideToCoord::ProcessPed", expect = "6A FF 68 68 52 84 00 64",
       unverified = "std14-ai-2", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the player's default primary task is used without a null check, as the original")]
pub fn slide_to_coord_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let anim_ok = if rd::<u8>(this, 0x78) & 2 != 0 {
        run_named_anim_process_ped(this, ped) != 0
    } else {
        true
    };
    let now = || g::<u32>(TIME_MS).read();
    if rd::<i32>(this, 0x7C) == -1 {
        if rd::<u8>(this, 0x78) & 2 == 0 {
            wr::<u32>(this, 0x7C, now().wrapping_add(2000));
        } else if anim_ok {
            wr::<u32>(this, 0x7C, now().wrapping_add(500));
        }
    }
    if rd::<u8>(this, 0x78) & 1 != 0 {
        let mut local = [0u32; 8];
        let lp = GPtr::from_raw(local.as_mut_ptr()).cast::<u8>();
        stand_still_ctor(lp, g::<u32>(0x86DB24).read(), 0, 0, 0x4100_0000);
        stand_still_process_ped(lp, ped);
        if ped_is_player(ped) != 0 {
            let t = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x47C), 0x14);
            v_make_abortable(t, ped, 2, GPtr::NULL);
        }
        wr::<u32>(ped, 0x55C, rd::<u32>(this, 0x70));
        wr::<u8>(this, 0x78, rd::<u8>(this, 0x78) & 0xFE);
        stand_still_dtor(lp);
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() {
        ped.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(this, 0x64));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(this, 0x68));
    let sum = dy.fmul(dy).fadd(dx.fmul(dx));
    let k = g::<f32>(0x86FD10).read();
    let kk = x(k).fmul_m(k);
    if sum.fcom(kk).c0() {
        wr::<f32>(ped, 0x4D8, 0.0);
        wr::<f32>(ped, 0x4DC, 0.0);
    } else {
        let s = rd::<f32>(this, 0x74);
        let ax = x(rd::<f32>(this, 0x64))
            .fsub_m(rd::<f32>(pos, 0))
            .fmul_m(s)
            .to_f32();
        let by = x(rd::<f32>(this, 0x68)).fsub_m(rd::<f32>(pos, 4)).fmul_m(s);
        let cz = x(rd::<f32>(this, 0x6C)).fsub_m(rd::<f32>(pos, 8)).to_f32();
        let czs = x(cz).fmul_m(s);
        let czs_f = czs.to_f32();
        let fwd = czs
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(by.fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(ax).fmul_m(rd::<f32>(m, 0x10)));
        let right = x(czs_f)
            .fmul_m(rd::<f32>(m, 8))
            .fadd(by.fmul_m(rd::<f32>(m, 4)))
            .fadd(x(ax).fmul_m(rd::<f32>(m, 0)));
        wr::<f32>(ped, 0x4D8, right.to_f32());
        wr::<f32>(ped, 0x4DC, fwd.to_f32());
    }
    if rd::<u32>(this, 0x7C) < now() {
        return 1;
    }
    if !anim_ok || sum.fcom(kk).test_parity(0x05) {
        return 0;
    }
    let d = x(rd::<f32>(ped, 0x558)).fsub_m(rd::<f32>(ped, 0x55C)).to_f32();
    let a = X87::from_f64_exact(limit_radian_angle(d.to_bits())).fabs();
    u8::from(!a.fcom(x(g::<f32>(0x86FD14).read())).test_parity(0x05))
}

pub const HOOKS: &[HookReg] = &[HOOK_SLIDE_TO_COORD_PROCESS_PED];
