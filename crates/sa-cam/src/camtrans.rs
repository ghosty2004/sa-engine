//! PORT-CAM-2: camera transitions and shake (`CCamera::StartTransition`, `CCamera::ProcessShake(float)`).
//! Ported from the 1.0 US bytes; gta-reversed `Camera.cpp` and `docs/re/specs/CAM/CCamera.md` as reading
//! aids. Where they differ, the bytes win (see the notes on each function).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_pop::common::get_atan_of_xy;
use sa_world::world::common::find_player_vehicle;
use sa_world::world::cworld::{PLAYER_IN_FOCUS, PLAYER_INFO_SIZE, PLAYERS};

use crate::cam::{cross_product, vector_normalise};
use crate::camctl::{cam_init, clean_up_old_reference, register_reference};
use crate::common::{active_cam, g, jne, jp, k, rd, wr, x};
use crate::handshaker::G_HAND_SHAKER;

sa_core::globals! {
    list GLOBALS;
    /// `StartTransition`: interpolation duration from `MODE_SPECIAL_FIXED_FOR_SYPHON` (int32 1000)
    pub static TRANS_SYPHON_FIXED_DURATION @ 0x8CCCC4: i32 = 1000;
    /// its "stop catching up" fraction (0.8)
    pub static TRANS_SYPHON_FIXED_CATCH_UP @ 0x8CCCC8: f32 = 0.8;
    /// its "stop moving" fraction (0.2)
    pub static TRANS_SYPHON_FIXED_MOVING @ 0x8CCCCC: f32 = 0.2;
    /// `StartTransition`: interpolation duration into `MODE_AIMWEAPON` (int32 400)
    pub static TRANS_AIM_DURATION @ 0x8CCCBC: i32 = 400;
    /// its "stop catching up" fraction (1.0)
    pub static TRANS_AIM_CATCH_UP @ 0x8CCCC0: f32 = 1.0;
    /// its "stop moving" fraction (.bss, 0.0)
    pub static TRANS_AIM_MOVING @ 0xB70044: f32 = 0.0;
    /// `ProcessShake(float)`'s one-time initialisation flag of `gHandShaker[1..5]`
    pub static SHAKE_PARAMS_INITIALISED @ 0xB70048: u8 = 0;
}

orig! {
    /// `CHandShaker::Process(float intensity)` (0x50D930), the intensity pushed as the dword it is
    pub fn hand_shaker_process(hs: GPtr<u8>, intensity: u32) = thiscall 0x50D930 => unported;
    /// `Multiply3x3(CVector* out, const CVector* v, const CMatrix* m)` (0x59C810, cdecl): `out = (v . right,
    /// v . up, v . at)`
    pub fn multiply3x3(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
}

/// Copies the 12 bytes of a `CVector` dword by dword (raw moves).
fn copy_vec(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..3 {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// `GetATanOfXY(cam.front.x, cam.front.y)` of the active cam (front at cam +0x190), ST0.
fn front_atan(this: GPtr<u8>) -> X87 {
    let cam = active_cam(this);
    X87::from_f64_exact(get_atan_of_xy(rd::<f32>(cam, 0x190), rd::<f32>(cam, 0x194)))
}

/// The inlined `CEntity::ChangeEntityReference(cam.m_pCamTargetEntity (+0x21C), this->m_pTargetEntity
/// (+0x958))`: clean up the old reference (when set) through the slot address taken before the call,
/// store the target, `RegisterReference` on whatever the slot then holds (no null check, as the bytes).
fn change_cam_target_reference(this: GPtr<u8>) {
    let slot = active_cam(this).byte_add(0x21C);
    let old = slot.cast::<GPtr<u8>>().read();
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    active_cam(this)
        .byte_add(0x21C)
        .cast::<GPtr<u8>>()
        .write(rd::<GPtr<u8>>(this, 0x958));
    let slot = active_cam(this).byte_add(0x21C);
    register_reference(slot.cast::<GPtr<u8>>().read(), slot);
}

/// The active cam's fixed-mode vector (+0x154), source (+0x160) and up offset (+0x16C) from the camera's
/// (+0x860 / +0x86C / +0x878), dword by dword.
fn copy_fixed_mode(this: GPtr<u8>) {
    copy_vec(active_cam(this).byte_add(0x154), this.byte_add(0x860));
    copy_vec(active_cam(this).byte_add(0x160), this.byte_add(0x86C));
    copy_vec(active_cam(this).byte_add(0x16C), this.byte_add(0x878));
}

/// Modes for which the target ped is turned to the camera's heading first (0x515236 chain).
const AIM_MODES_TURN_PED: [u16; 12] = [7, 8, 0x33, 0x22, 0x27, 0x28, 0x34, 0x2A, 0x2B, 0x2D, 0x2E, 0x29];

/// `CCamera::StartTransition(int16 newMode)` (0x515200, `ret 4`; entry `movzx eax, [esi+0x59]; nop;
/// jmp 0x40479B`, a protection fragment doing `imul eax, eax, 0x238` and jumping back to 0x51520F).
/// `old` = the active cam's mode at entry (u16 +0xC).
/// 1. `m_bItsOkToLookJustAtThePlayer` (+0x3B) = 0, `m_bUseTransitionBeta` (+0x37) = 0, the "stop moving"
///    / "stop catching up" fractions (+0xC10 / +0xC14) = 0.25 / 0.75. When `old` is one of the aiming
///    modes and the target entity (+0x958, not null-checked) is a ped: its current and aiming rotation
///    (+0x558 / +0x55C) = `GetATanOfXY(front) - pi/2` (both from the unrounded value).
/// 2. Fixed-mode vector / source / up offset copied to the active cam, its target entity reference
///    changed to +0x958, cam +2 = the camera's +0x2C.
/// 3. `switch (newMode)` (table 0x515B80 / 0x515B64): 3 / 0x16: cam +0xC0 = 0; 4: after leaving a garage
///    (+0x28) the horizontal angle (+0xBC) = `atan(front) + pi` and the transition beta (+0x9C) = 0;
///    +0x35 sets +0x1B; with the active cam in mode 0x12: +0x37 = 1 and beta = `atan(front) + 235deg`
///    (the bytes run the [-pi, pi] wrap and the `|a| <= pi/2` test on the constant 0.0 instead of the
///    angle, so the +235deg branch is always taken; kept); 0x12 (`+0x2B == 1`, not after a garage):
///    +0x37 = 1, beta = `atan(front)` (both arms of the mode-0xF test store the same); the aiming modes:
///    horizontal angle = FPATAN of the player vehicle's (else the focused player ped's) matrix up row
///    `atan2(up.x, up.y)` (sa-math f64 model), vertical angle (+0xAC) = 0; 0x1D: vertical angle = 15deg.
/// 4. 4 <-> 0x12 just sets the cam mode; otherwise `CCam::Init` and the mode, keeping the horizontal angle
///    (`fld`/`fstp`: a signalling NaN is quieted).
/// 5. Duration (+0x7C) and fractions by (old, new) (the gr lambda; .data values at 0x8CCCBC..0x8CCCCC and
///    0xB70044 copied as dwords); a flag selects 350 instead of 600 ms for the target-coors duration.
/// 6. Transition state (+0x58), start time (+0x78), +0x57; the interpolation start values (+0x890 /
///    +0x89C / +0x8A8 / +0x118 / +0x11C) from the special-interp values or the active cam; steps 2 (cam
///    +2 and the fixed-mode vectors) and the mode, the reference change and the state / start time again;
///    FOV, vectors and angles saved (+0x170, +0x8B4, +0x8C0, +0x8CC, +0xF8, +0x108, +0x114).
/// 7. Looking at the player (+0x2B): +0xC18 = 0, +0xC1C = 1.0, +0x80 = flag ? 350 : 600; else (with
///    +0x50 == 1 the script values +0xC24..+0xC2C first) +0x80 / +0xC18 / +0xC1C = +0x7C / +0xC10 / +0xC14.
#[hook(addr = 0x515200, conv = thiscall, name = "CCamera::StartTransition", expect = "51 53 56 8B F1", this = "CCamera")]
pub fn start_transition(this: GPtr<u8>, new_mode: u32) {
    let bp = new_mode as u16;
    let old = rd::<u16>(active_cam(this), 0xC);
    wr::<u8>(this, 0x3B, 0);
    wr::<u8>(this, 0x37, 0);
    wr::<u32>(this, 0xC10, 0x3E80_0000); // 0.25
    wr::<u32>(this, 0xC14, 0x3F40_0000); // 0.75
    if AIM_MODES_TURN_PED.contains(&rd::<u16>(active_cam(this), 0xC))
        && rd::<u8>(rd::<GPtr<u8>>(this, 0x958), 0x36) & 7 == 3
    {
        let a = front_atan(this).fsub_m(k(0x3FC9_0FDB)); // pi/2 [0x858FE4]
        wr::<f32>(rd::<GPtr<u8>>(this, 0x958), 0x558, a.to_f32());
        wr::<f32>(rd::<GPtr<u8>>(this, 0x958), 0x55C, a.to_f32());
    }
    copy_vec(active_cam(this).byte_add(0x154), this.byte_add(0x860));
    change_cam_target_reference(this);
    copy_vec(active_cam(this).byte_add(0x160), this.byte_add(0x86C));
    copy_vec(active_cam(this).byte_add(0x16C), this.byte_add(0x878));
    wr::<u8>(active_cam(this), 2, rd::<u8>(this, 0x2C));

    match bp as i16 {
        3 | 0x16 => wr::<u32>(active_cam(this), 0xC0, 0),
        4 => {
            if rd::<u8>(this, 0x28) != 0 {
                let a = front_atan(this).fadd_m(k(0x4049_0FDB)); // pi [0x858CB8]
                wr::<f32>(active_cam(this), 0xBC, a.to_f32());
                wr::<u32>(active_cam(this), 0x9C, 0);
            }
            if rd::<u8>(this, 0x35) != 0 {
                wr::<u8>(this, 0x1B, 1);
            }
            if rd::<u16>(active_cam(this), 0xC) == 0x12 {
                wr::<u8>(this, 0x37, 1);
                let a = front_atan(this);
                let beta = a.fadd_m(wrap_test_on_zero());
                wr::<f32>(active_cam(this), 0x9C, beta.to_f32());
            }
        }
        0x12 => {
            if rd::<u8>(this, 0x2B) == 1 && rd::<u8>(this, 0x28) == 0 {
                wr::<u8>(this, 0x37, 1);
                let a = front_atan(this);
                wr::<f32>(active_cam(this), 0x9C, a.to_f32());
            }
        }
        7 | 8 | 0x22 | 0x27..=0x2B | 0x2D | 0x2E | 0x33 | 0x34 => {
            let a = if !find_player_vehicle(-1, 0).is_null() {
                let m1 = rd::<GPtr<u8>>(find_player_vehicle(-1, 0), 0x14);
                let m2 = rd::<GPtr<u8>>(find_player_vehicle(-1, 0), 0x14);
                x(rd::<f32>(m2, 0x10)).fpatan(x(rd::<f32>(m1, 0x14)))
            } else {
                let id = isize::from(PLAYER_IN_FOCUS.get());
                let ped =
                    g::<GPtr<u8>>(PLAYERS.wrapping_add((id * PLAYER_INFO_SIZE as isize) as usize)).read();
                let m = rd::<GPtr<u8>>(ped, 0x14);
                x(rd::<f32>(m, 0x10)).fpatan(x(rd::<f32>(m, 0x14)))
            };
            wr::<f32>(active_cam(this), 0xBC, a.to_f32());
            wr::<u32>(active_cam(this), 0xAC, 0);
        }
        0x1D => wr::<u32>(active_cam(this), 0xAC, 0x3E86_0A92), // 15deg
        _ => {}
    }

    let horiz = rd::<u32>(active_cam(this), 0xBC);
    let cam = active_cam(this);
    let cur = rd::<u16>(cam, 0xC);
    if (cur == 4 && bp == 0x12) || (cur == 0x12 && bp == 4) {
        wr::<u16>(cam, 0xC, bp);
    } else {
        cam_init(cam);
        wr::<u16>(active_cam(this), 0xC, bp);
        wr::<f32>(active_cam(this), 0xBC, x(f32::from_bits(horiz)).to_f32());
    }

    let short_target_time = durations(this, old, bp);

    wr::<u8>(this, 0x58, 1);
    wr::<u32>(this, 0x78, TIME_MS.get());
    wr::<u8>(this, 0x57, 1);
    if rd::<u8>(this, 0x4F) != 0 {
        copy_vec(this.byte_add(0x890), this.byte_add(0x914));
        copy_vec(this.byte_add(0x89C), this.byte_add(0x920));
        copy_vec(this.byte_add(0x8A8), this.byte_add(0x92C));
        let (a, b) = (rd::<u32>(this, 0x104), rd::<u32>(this, 0x100));
        wr::<u32>(this, 0x11C, b);
        wr::<u32>(this, 0x118, a);
    } else {
        let cam = active_cam(this);
        copy_vec(this.byte_add(0x890), cam.byte_add(0x19C));
        copy_vec(this.byte_add(0x89C), cam.byte_add(0x148));
        copy_vec(this.byte_add(0x8A8), cam.byte_add(0x1B4));
        wr::<u32>(this, 0x11C, rd::<u32>(cam, 0xA4));
        wr::<u32>(this, 0x118, rd::<u32>(cam, 0xA0));
    }
    wr::<u8>(active_cam(this), 2, rd::<u8>(this, 0x2C));
    copy_fixed_mode(this);
    wr::<u16>(active_cam(this), 0xC, bp);
    change_cam_target_reference(this);
    let cam = active_cam(this);
    wr::<u8>(this, 0x58, 1);
    wr::<u32>(this, 0x78, TIME_MS.get());
    wr::<u8>(this, 0x57, 1);
    wr::<u32>(this, 0x170, rd::<u32>(cam, 0xB4));
    copy_vec(this.byte_add(0x8B4), cam.byte_add(0x124));
    copy_vec(this.byte_add(0x8C0), cam.byte_add(0x130));
    copy_vec(this.byte_add(0x8CC), cam.byte_add(0x13C));
    wr::<u32>(this, 0xF8, rd::<u32>(cam, 0x40));
    wr::<u32>(this, 0x108, rd::<u32>(cam, 0x44));
    wr::<u32>(this, 0x114, rd::<u32>(cam, 0x74));

    if rd::<u8>(this, 0x2B) != 0 {
        wr::<u32>(this, 0xC18, 0);
        wr::<u32>(this, 0xC1C, 0x3F80_0000);
        wr::<i32>(this, 0x80, if short_target_time { 350 } else { 600 });
        return;
    }
    if rd::<u8>(this, 0x50) == 1 {
        let (a, b, c) = (
            rd::<u32>(this, 0xC24),
            rd::<u32>(this, 0xC28),
            rd::<u32>(this, 0xC2C),
        );
        wr::<u32>(this, 0xC10, a);
        wr::<u32>(this, 0xC14, b);
        wr::<u32>(this, 0x7C, c);
    }
    let (d, a, b) = (
        rd::<u32>(this, 0x7C),
        rd::<u32>(this, 0xC10),
        rd::<u32>(this, 0xC14),
    );
    wr::<u32>(this, 0x80, d);
    wr::<u32>(this, 0xC18, a);
    wr::<u32>(this, 0xC1C, b);
}

/// The beta offset `StartTransition` adds in mode 4 coming from 0x12 (0x5154CF..0x515554): the [-pi, pi]
/// wrap loops and the `fabs <= pi/2` test run on `fld 0.0` [0x858B50] (not on the angle, which stays in
/// ST1), so the result is always 235deg [0x8631DC]; 55deg [0x8631E0] is unreachable. Literal model.
fn wrap_test_on_zero() -> f32 {
    let pi = x(k(0x4049_0FDB)); // [0x858CB8]
    let two_pi = k(0x40C9_0FDB); // [0x858CBC]
    let mut z = x(k(0)); // [0x858B50]
    if !jne(z, pi, 0x41) {
        loop {
            z = z.fsub_m(two_pi);
            if jne(z, pi, 0x41) {
                break;
            }
        }
    }
    let neg_pi = x(k(0xC049_0FDB)); // [0x858CC0]
    if !jp(z, neg_pi, 0x05) {
        loop {
            z = z.fadd_m(two_pi);
            if jp(z, neg_pi, 0x05) {
                break;
            }
        }
    }
    let abs = X87::from_f64_exact(z.to_f64().abs());
    if jne(abs, x(k(0x3FC9_0FDB)), 0x41) {
        k(0x4083_3FAF) // 235deg
    } else {
        k(0x3F75_BE0C) // 55deg
    }
}

/// `StartTransition`'s duration / fraction choice (0x515675..0x515844). Returns the flag (BL) that
/// selects the 350 ms target-coors duration.
fn durations(this: GPtr<u8>, old: u16, new: u16) -> bool {
    let set = |m: u32, c: u32, d: u32| {
        wr::<u32>(this, 0xC10, m);
        wr::<u32>(this, 0xC14, c);
        wr::<u32>(this, 0x7C, d);
    };
    let bits = |gl: &sa_core::mem::Global<f32>| gl.ptr().cast::<u32>().read();
    let ibits = |gl: &sa_core::mem::Global<i32>| gl.ptr().cast::<u32>().read();
    wr::<u32>(this, 0x7C, 1350);
    if new == 0xB {
        if old == 0x1C {
            wr::<u32>(this, 0x7C, 1800);
            return false;
        }
    } else if new == 0x12 && matches!(old, 0x1C | 4 | 0xB | 0x23 | 0x35) {
        set(0x3DCC_CCCD, 0x3F66_6666, 750); // 0.1, 0.9
        return false;
    }
    if old == 0x23 {
        let (m, c, d) = (
            bits(&TRANS_SYPHON_FIXED_MOVING),
            bits(&TRANS_SYPHON_FIXED_CATCH_UP),
            ibits(&TRANS_SYPHON_FIXED_DURATION),
        );
        set(m, c, d);
        return false;
    }
    if old == 0xF {
        wr::<u32>(this, 0xC10, 0x3D4C_CCCD); // 0.05
        wr::<u32>(this, 0xC14, 0x3F73_3333); // 0.95
        return false;
    }
    let to_ped_list = if rd::<u8>(this, 0x27) != 0 && new == 4 {
        if old == 0x12 {
            wr::<u32>(this, 0x7C, 800);
            wr::<u32>(this, 0xC10, 0x3CA3_D70A); // 0.02
            wr::<u32>(this, 0xC14, 0x3F7A_E148); // 0.98
            return false;
        }
        true
    } else {
        if (new == 0x12 || new == 0x16) && matches!(old, 0x27 | 0x28 | 0x34 | 0x29 | 0x2A | 0x2B | 0x2E) {
            set(0, 0x3F80_0000, 1);
            return false;
        }
        if new == 0x35 {
            let (m, c, d) = (
                bits(&TRANS_AIM_MOVING),
                bits(&TRANS_AIM_CATCH_UP),
                ibits(&TRANS_AIM_DURATION),
            );
            set(m, c, d);
            return true;
        }
        matches!(new, 4 | 0x1C | 0xB | 0x23)
    };
    if to_ped_list && matches!(old, 0x1C | 4 | 0xB | 0x35) {
        set(0x3DCC_CCCD, 0x3F66_6666, 350); // 0.1, 0.9
        return true;
    }
    wr::<u32>(this, 0x7C, 1350);
    false
}

/// `gHandShaker[1..5]` parameters `ProcessShake(float)` sets once: (shaker, `m_lim.z` bits, twitch
/// frequency, twitch velocity bits). Common to all five: `m_lim.xy` 0.02, `m_motion` (0.0002, 0.0002,
/// 0.0001), `m_slow` (1.3, 1.3, 1.4), reaction min / max 0.3 / 1.0. Shaker 0 is untouched.
const SHAKER_INIT: [(isize, u32, u32, u32); 5] = [
    (1, 0x3C23_D70A, 15, 0x3A83_126F),
    (2, 0x3D23_D70A, 20, 0x3A83_126F),
    (3, 0x3C23_D70A, 10, 0x3A03_126F),
    (4, 0x3C23_D70A, 20, 0x3B03_126F),
    (5, 0x3C23_D70A, 2, 0x3B44_9BA6),
];

/// `CCamera::ProcessShake(float)` (0x516560, `ret 4`). The argument is never read. The cam is the active
/// one at entry. Once (byte 0xB70048): the `gHandShaker[1..5]` parameters ([`SHAKER_INIT`], dword stores).
/// Then `hs = &gHandShaker[m_nShakeType (+0xCA0)]` (unchecked), `hs->Process(m_fShakeIntensity (+0xC90))`,
/// `roll = (f32)(hs->m_ang.z * intensity)`; `front = Multiply3x3(front, hs->m_resultMat)`, normalised;
/// `up = ((f32)sin(roll), 0, (f32)cos(roll))` (FSIN / FCOS: sa-math's f64 model); `r = front x up`,
/// normalised, `up = r x front`; `front.x == 0 && front.y == 0` (`test ah,0x44`, NaN no) sets both to
/// 0.0001; again `r = front x up`, normalised, `up = r x front`.
///
/// The original leaves in EAX the last `CrossProduct` result, the address of its own stack local; both
/// callers (0x51A72B, 0x52BA68) ignore it, so the port returns nothing.
#[hook(addr = 0x516560, conv = thiscall, name = "CCamera::ProcessShake-float", expect = "83 EC 28 53 56 8B F1", this = "CCamera")]
pub fn process_shake_float(this: GPtr<u8>, _unused: u32) {
    let cam = active_cam(this);
    if SHAKE_PARAMS_INITIALISED.get() == 0 {
        let base = G_HAND_SHAKER.ptr().cast::<u8>();
        for (i, lim_z, twitch_freq, twitch_vel) in SHAKER_INIT {
            let hs = base.byte_add(i * 0x94);
            for (off, v) in [
                (0x0C, 0x3CA3_D70A), // m_lim 0.02, 0.02, z
                (0x10, 0x3CA3_D70A),
                (0x14, lim_z),
                (0x18, 0x3951_B717), // m_motion 0.0002, 0.0002, 0.0001
                (0x1C, 0x3951_B717),
                (0x20, 0x38D1_B717),
                (0x30, 0x3FA6_6666), // m_slow 1.3, 1.3, 1.4
                (0x34, 0x3FA6_6666),
                (0x38, 0x3FB3_3333),
                (0x84, 0x3E99_999A), // m_scaleReactionMin 0.3
                (0x88, 0x3F80_0000), // m_scaleReactionMax 1.0
                (0x8C, twitch_freq),
                (0x90, twitch_vel),
            ] {
                wr::<u32>(hs, off, v);
            }
        }
        SHAKE_PARAMS_INITIALISED.set(1);
    }
    let ty = rd::<i32>(this, 0xCA0);
    let hs = G_HAND_SHAKER
        .ptr()
        .cast::<u8>()
        .byte_add(ty.wrapping_mul(0x94) as isize);
    hand_shaker_process(hs, rd::<u32>(this, 0xC90));
    let roll = x(rd::<f32>(hs, 8)).fmul_m(rd::<f32>(this, 0xC90)).to_f32();

    let front = cam.byte_add(0x190);
    let up = cam.byte_add(0x1B4);
    let mut tmp = [0u32; 3];
    let tmp_p = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    multiply3x3(tmp_p, front, hs.byte_add(0x3C));
    copy_vec(front, tmp_p);
    vector_normalise(front);
    let s = x(roll).fsin().to_f32();
    wr::<f32>(up, 0, s);
    wr::<u32>(up, 4, 0);
    let c = x(roll).fcos().to_f32();
    wr::<f32>(up, 8, c);

    let mut r = [0u32; 3];
    let r_p = GPtr::from_raw(r.as_mut_ptr()).cast::<u8>();
    let mut out = [0u32; 3];
    let out_p = GPtr::from_raw(out.as_mut_ptr()).cast::<u8>();
    cross_product(r_p, front, up);
    vector_normalise(r_p);
    let res = cross_product(out_p, r_p, front);
    copy_vec(up, res);

    let zero = x(k(0)); // [0x858B50]
    if !x(rd::<f32>(front, 0)).fcom(zero).test_parity(0x44)
        && !x(rd::<f32>(front, 4)).fcom(zero).test_parity(0x44)
    {
        wr::<u32>(front, 0, 0x38D1_B717);
        wr::<u32>(front, 4, 0x38D1_B717);
    }

    let res = cross_product(out_p, front, up);
    copy_vec(r_p, res);
    vector_normalise(r_p);
    let res = cross_product(out_p, r_p, front);
    copy_vec(up, res);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_START_TRANSITION, HOOK_PROCESS_SHAKE_FLOAT];
