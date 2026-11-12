//! PORT-VEH-4 (batch `veh4-2`): `CBoat::PreRender`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, ftol32, g, jne_mask, jp5, rd, wr, x};

orig! {
    /// `CVehicle::PreRender()` (0x6D6480)
    fn vehicle_pre_render(v: GPtr<u8>) = thiscall 0x6D6480 => unported;
    /// `CVehicle::SetComponentRotation(RwFrame*, eRotationAxis, float, bool)` (0x6DBA30)
    fn set_component_rotation(v: GPtr<u8>, frame: GPtr<u8>, axis: u32, angle: f32, set: u32) = thiscall 0x6DBA30 => unported;
    /// `RwFrameForAllObjects(RwFrame*, RwObjectCallBack, void*)` (0x7F1200)
    fn frame_for_all_objects(f: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1200 => unported;
    /// `CVehicle::SetComponentAtomicAlpha(RpAtomic*, int32)` (0x6D2960)
    fn set_component_atomic_alpha(a: GPtr<u8>, alpha: i32) = cdecl 0x6D2960 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `Multiply3x3(const CMatrix&, const CVector&)` (0x59C790, the result through `out`)
    fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CDoor::Process(CVehicle*, CVector& moveSpeed, CVector& turnSpeed, const CVector& offset)` (0x6F4040, AL)
    fn door_process(d: GPtr<u8>, v: GPtr<u8>, ms: GPtr<u8>, ts: GPtr<u8>, off: GPtr<u8>) -> u8 = thiscall 0x6F4040 => unported;
    /// `CMatrix::SetRotate(float x, float y, float z)` (0x59B120)
    fn matrix_set_rotate(m: GPtr<u8>, rx: f32, ry: f32, rz: f32) = thiscall 0x59B120 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `GetFirstObject(RwFrame*)` (0x7348C0)
    fn get_first_object(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// An atomic's render callback (`RpAtomic` +0x48), called with the atomic
    fn call_render_cb(f: u32, atomic: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, RwV3d*, RwMatrix*, bool)` (0x4A9BE0) on `g_fxMan` (0xA9AE80)
    fn create_fx_system(mgr: GPtr<u8>, name: u32, pos: GPtr<u8>, m: GPtr<u8>, ignore_bbox: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::SetLocalParticles(bool)` (0x4AA910)
    fn fx_set_local_particles(fx: GPtr<u8>, b: u32) = thiscall 0x4AA910 => unported;
    /// `FxSystem_c::CopyParentMatrix()` (0x4AA890)
    fn fx_copy_parent_matrix(fx: GPtr<u8>) = thiscall 0x4AA890 => unported;
    /// `FxSystem_c::GetPlayStatus()` (0x4AA900)
    fn fx_get_play_status(fx: GPtr<u8>) -> u32 = thiscall 0x4AA900 => unported;
    /// `FxSystem_c::Stop()` (0x4AA390)
    fn fx_stop(fx: GPtr<u8>) = thiscall 0x4AA390 => unported;
    /// `FxSystem_c::SetMatrix(RwMatrix*)` (0x4AA630)
    fn fx_set_matrix(fx: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4AA630 => unported;
    /// `FxSystem_c::SetConstTime(bool, float)` (0x4AA6C0)
    fn fx_set_const_time(fx: GPtr<u8>, on: u32, t: f32) = thiscall 0x4AA6C0 => unported;
    /// `CVehicle::DoBoatSplashes(float)` (0x6DD130)
    fn do_boat_splashes(v: GPtr<u8>, speed: f32) = thiscall 0x6DD130 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `|v|` of the three floats at `p` as the original sums them (`(x*x + y*y) + z*z`, FSQRT).
fn length(p: GPtr<u8>) -> f32 {
    let (vx, vy, vz) = (x(rd::<f32>(p, 0)), x(rd::<f32>(p, 4)), x(rd::<f32>(p, 8)));
    f32_of(vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt())
}

/// The atomic of a frame's first atomic object, by `GetCurrentAtomicObjectCB` (0x6A0750).
fn frame_atomic(frame: GPtr<u8>) -> GPtr<u8> {
    let mut a = [0u32; 1];
    frame_for_all_objects(frame, 0x6A_0750, buf(&mut a));
    GPtr::from_addr(a[0] as usize)
}

/// `CBoat::PreRender()` (0x6F1180).
///
/// The vehicle pre-render; brightness 0.5; the three rudder frames (+0x5BC/+0x5C8/+0x5CC) turned about z
/// by `-steer`; the propeller angle (+0x5A8) advanced by `timeStep * propSpeed` (wrapped below 2pi); the
/// two propeller pairs (+0x5D0/+0x5D4, +0x5D8/+0x5DC) set about y to `+-2 * angle` / `-+angle` with the
/// fast/slow blur alphas `ftol((1 - min(speed * 5.09, 1)) * 255)` and `max(150 - that, 0)`; the
/// Marquis' (484) boom door processed and rotated on its axis; the radar dish of 430/453/454 (+0x5B4)
/// turned to +0x5A0 (rendered through its atomic's callback while 0x96914B is set) and advanced by
/// `timeStep * 0.02`; the frame speeds (+0x610 = move + friction move, +0x61C = turn + friction turn);
/// the two propeller splash fx (`boat_prop`, +0x628/+0x62C) created at the propeller frames, stopped
/// while the engine is off, else playing with a constant time `min(2 |moveSpeed|, 1)`; the boat
/// splashes by the length of +0x630.
#[hook(addr = 0x6F1180, conv = thiscall, name = "CBoat::PreRender", this = "CBoat",
       expect = "6A FF 68 18 89 84 00",
       unverified = "veh4-2", route = "L2", hazards = "heap-writes",
       note = "creates/plays the propeller FxSystems and writes RW frames; the CMatrix lives on the Rust stack; the SEH frame is not reproduced")]
pub fn boat_pre_render(this: GPtr<u8>) {
    vehicle_pre_render(this);
    wr::<u32>(this, 0x12C, 0x3F00_0000);
    let mut mat = [0u32; 0x12];
    let m = buf(&mut mat);
    for o in [0x5BCisize, 0x5C8, 0x5CC] {
        let a = f32_of(x(rd::<f32>(this, 0x494)).fchs());
        set_component_rotation(this, rd::<GPtr<u8>>(this, o), 2, a, 1);
    }
    let mut f = x(rd::<f32>(this, 0x5A4)).fmul_m(f32::from_bits(0x40A2_F983));
    if !jp5(x(1.0), f) {
        f = x(1.0);
    }
    let alpha = ftol32(x(1.0).fsub(f).fmul_m(255.0));
    let two_pi = f32::from_bits(0x40C9_0FDB);
    let ts = || g::<f32>(0xB7_CB5C).read();
    let a = x(ts()).fmul_m(rd::<f32>(this, 0x5A4)).fadd_m(rd::<f32>(this, 0x5A8));
    wr::<f32>(this, 0x5A8, f32_of(a));
    if !jne_mask(a, x(two_pi), 0x41) {
        let mut v = x(rd::<f32>(this, 0x5A8));
        loop {
            v = v.fsub_m(two_pi);
            if jne_mask(v, x(two_pi), 0x41) {
                break;
            }
        }
        wr::<f32>(this, 0x5A8, f32_of(v));
    }
    for (fast, slow, sign) in [(0x5D0isize, 0x5D4isize, 1.0f32), (0x5D8, 0x5DC, -1.0)] {
        let fr = rd::<GPtr<u8>>(this, fast);
        if !fr.is_null() {
            let ang = f32_of(x(sign).fmul_m(rd::<f32>(this, 0x5A8)).fdouble());
            set_component_rotation(this, fr, 1, ang, 1);
            let at = frame_atomic(rd::<GPtr<u8>>(this, fast));
            if !at.is_null() {
                set_component_atomic_alpha(at, alpha);
            }
        }
        let fr = rd::<GPtr<u8>>(this, slow);
        if !fr.is_null() {
            let ang = f32_of(x(sign).fmul_m(rd::<f32>(this, 0x5A8)).fchs());
            set_component_rotation(this, fr, 1, ang, 1);
            let at = frame_atomic(rd::<GPtr<u8>>(this, slow));
            if !at.is_null() {
                let a2 = 0x96i32.wrapping_sub(alpha);
                set_component_atomic_alpha(at, if a2 < 0 { 0 } else { a2 });
            }
        }
    }
    if rd::<i16>(this, 0x22) == 0x1E4 {
        let fr = rd::<GPtr<u8>>(this, 0x5C0);
        if !fr.is_null() {
            matrix_attach(m, fr.byte_add(0x10), 0);
            let mut saved = [rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), rd::<u32>(m, 0x38)];
            let mut rot = [0u32; 3];
            let mut off = [0u32; 3];
            multiply_3x3(buf(&mut off), rd::<GPtr<u8>>(this, 0x14), buf(&mut saved));
            door_process(this.byte_add(0x5E0), this, this.byte_add(0x610), this.byte_add(0x61C), buf(&mut off));
            let axis = rd::<u8>(this, 0x5EA) as usize;
            let rp = buf(&mut rot);
            wr::<u32>(rp, 4 * axis as isize, rd::<u32>(this, 0x5EC));
            matrix_set_rotate(m, f32::from_bits(rot[0]), f32::from_bits(rot[1]), f32::from_bits(rot[2]));
            for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
                wr::<f32>(m, o, f32_of(x(rd::<f32>(m, o)).fadd_m(f32::from_bits(saved[k]))));
            }
            matrix_update_rw(m);
        }
    }
    let model = rd::<i16>(this, 0x22);
    if model == 0x1AE || model == 0x1C5 || model == 0x1C6 {
        let fr = rd::<GPtr<u8>>(this, 0x5B4);
        if !fr.is_null() {
            set_component_rotation(this, fr, 2, rd::<f32>(this, 0x5A0), 1);
            if g::<u8>(0x96_914B).read() != 0 {
                let first = get_first_object(rd::<GPtr<u8>>(this, 0x5B4));
                let atomic = get_first_object(rd::<GPtr<u8>>(this, 0x5B4));
                call_render_cb(rd::<u32>(first, 0x48), atomic);
            }
        }
        wr::<f32>(this, 0x5A0, f32_of(x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(rd::<f32>(this, 0x5A0))));
    }
    let sum = |a: isize, b: isize| f32_of(x(rd::<f32>(this, a)).fadd_m(rd::<f32>(this, b)));
    let (mz, my, mx) = (sum(0x4C, 0x64), sum(0x48, 0x60), sum(0x5C, 0x44));
    wr::<f32>(this, 0x610, mx);
    wr::<f32>(this, 0x614, my);
    wr::<f32>(this, 0x618, mz);
    let (tz, ty, tx) = (sum(0x58, 0x70), sum(0x54, 0x6C), sum(0x50, 0x68));
    wr::<f32>(this, 0x61C, tx);
    wr::<f32>(this, 0x620, ty);
    wr::<f32>(this, 0x624, tz);
    let speed = length(this.byte_add(0x44));
    for k in 0..2isize {
        let rw = rd::<GPtr<u8>>(this, 0x18);
        let rwm = if rw.is_null() { GPtr::<u8>::NULL } else { rd::<GPtr<u8>>(rw, 4).byte_add(0x10) };
        let slot = 0x628 + 4 * k;
        let frame_o = 0x5D0 + 8 * k;
        if rd::<u32>(this, slot) == 0 {
            if !rwm.is_null() && rd::<u32>(this, frame_o) != 0 {
                let mut off = [0u32; 3];
                let ltm = frame_get_ltm(rd::<GPtr<u8>>(this, frame_o));
                for o in [0x30isize, 0x34, 0x38] {
                    wr::<u32>(rwm, o, rd::<u32>(ltm, o));
                }
                rw_matrix_update(rwm);
                let fx = create_fx_system(GPtr::from_addr(0xA9_AE80), 0x87_22CC, buf(&mut off), rwm, 0);
                wr::<GPtr<u8>>(this, slot, fx);
                if !fx.is_null() {
                    fx_play(fx);
                    fx_set_local_particles(rd::<GPtr<u8>>(this, slot), 1);
                    fx_copy_parent_matrix(rd::<GPtr<u8>>(this, slot));
                }
            }
            if rd::<u32>(this, slot) == 0 {
                continue;
            }
        }
        let fx = rd::<GPtr<u8>>(this, slot);
        if rd::<u32>(this, 0x40) & 0x100 == 0 {
            if fx_get_play_status(fx) == 0 {
                fx_stop(rd::<GPtr<u8>>(this, slot));
            }
            continue;
        }
        if fx_get_play_status(fx) == 1 {
            fx_play(rd::<GPtr<u8>>(this, slot));
        }
        let ltm = frame_get_ltm(rd::<GPtr<u8>>(this, frame_o));
        for o in [0x30isize, 0x34, 0x38] {
            wr::<u32>(rwm, o, rd::<u32>(ltm, o));
        }
        rw_matrix_update(rwm);
        fx_set_matrix(rd::<GPtr<u8>>(this, slot), rwm);
        let mut t = x(speed).fdouble();
        if !jp5(x(1.0), t) {
            t = x(1.0);
        }
        fx_set_const_time(rd::<GPtr<u8>>(this, slot), 1, f32_of(t));
    }
    do_boat_splashes(this, length(this.byte_add(0x630)));
    matrix_dtor(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BOAT_PRE_RENDER];
