//! `CPed::MakeTyresMuddySectorList` (PEDHIT: a vehicle rolling over a dead ped gets bloody tyres and a bump).
//! Ported from the 1.0 US bytes (0x6AE0D0..0x6AE833, spec `docs/re/specs/PED/CPed.md`); gta-reversed only wraps the
//! original (`.Reversed = false`).
//!
//! The spec's "uninitialised `bike` local" is initialised: the prologue stores EBX (0) to `[esp+0xC]` before
//! `push esi; push edi`, which is the `[esp+0x14]` the body reads as `bike`.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CWorld::ms_nCurrentScanCode` (u16)
const SCAN_CODE: usize = 0xB7CD78;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

orig! {
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, returns out)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CLocalisation::Blood()` (0x56D230)
    fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn add_audio_event(a: GPtr<u8>, ev: u32, vol: u32) = thiscall 0x4F6420 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x4ABBA0; raw float words)
    fn physical_apply_move_force(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x4ABBA0 => unported;
    /// `FUN_0065da70(CVector force, CVector point)` (0x65DA70): re-pushes its vectors to `ApplyTurnForce` (raw float words)
    fn apply_turn_force_65da70(p: GPtr<u8>, fx: u32, fy: u32, fz: u32, px: u32, py: u32, pz: u32) = thiscall 0x65DA70 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake(int16 time, uint8 freq, uint32 delay)` (0x53F920)
    fn pad_start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
}

/// `|a - b| < lim` as the original tests it: the difference negated when below 0 (`test ah, 5; jp`), then
/// `jp` past the body unless strictly less (NaN fails).
fn within(a: f32, b: f32, lim: f32) -> bool {
    let d = x(a).fsub_m(b);
    let d = if d.fcom(x(0.0)).test_parity(5) { d } else { x(a).fsub_m(b).fchs() };
    !d.fcom(x(lim)).test_parity(5)
}

/// `min(v, 1.0f)` as `fld 1.0; fcomp st(1); test ah, 5; jp` keeps it: 1.0 only when `1.0 < v` (NaN keeps v).
fn cap1(v: X87) -> X87 {
    if x(1.0).fcom(v).test_parity(5) { v } else { x(1.0) }
}

/// The vehicle's collision model (`ms_modelInfoPtrs[(int16)+0x22]->+0x14`).
fn col_model(v: GPtr<u8>) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * i32::from(rd::<i16>(v, 0x22)) as usize).read();
    rd::<GPtr<u8>>(mi, 0x14)
}

/// The shared wheel test: `W = matrix * o`; `|W.z - ped.z| < 2` and `(W.y - ped.y)² + (W.x - ped.x)² < 1`.
fn wheel_on_ped(ped: GPtr<u8>, v: GPtr<u8>, o: &mut [f32; 3]) -> Option<[f32; 3]> {
    let mut w = [0f32; 3];
    matrix_mul_vec(lp(&mut w).cast(), rd::<GPtr<u8>>(v, 0x14), lp(o).cast());
    let pp = vec3(ent_pos(ped));
    if !within(w[2], pp[2], 2.0) {
        return None;
    }
    let dy = x(w[1]).fsub_m(pp[1]);
    let dx = x(w[0]).fsub_m(pp[0]);
    if dy.fmul(dy).fadd(dx.fmul(dx)).fcom(x(1.0)).test_parity(5) {
        return None;
    }
    Some(w)
}

/// The push on the vehicle at the wheel: `ApplyMoveForce((0, 0, up))`, then 0x65DA70 with `(0, 0, turn)` at
/// `W - vehicle position`, and the pad shake (300, 70) when it is the player's vehicle.
fn push_vehicle(v: GPtr<u8>, w: [f32; 3], up: f32, turn: impl FnOnce() -> f32) {
    physical_apply_move_force(v, 0, 0, up.to_bits());
    let t = turn();
    let p = vec3(ent_pos(v));
    let d = [f32_of(x(w[0]).fsub_m(p[0])), f32_of(x(w[1]).fsub_m(p[1])), f32_of(x(w[2]).fsub_m(p[2]))];
    apply_turn_force_65da70(v, 0, 0, t.to_bits(), d[0].to_bits(), d[1].to_bits(), d[2].to_bits());
    if v == find_player_vehicle(-1, 0) {
        pad_start_shake(get_pad(0), 300, 70, 0);
    }
}

/// `CPed::MakeTyresMuddySectorList<CPtrListSingleLink<CPhysical*>>(list)` (0x6AE0D0, thiscall, `ret 4`; caller
/// `DeadPedMakesTyresBloody` 0x6B4200). Despite the name: the wheels of moving vehicles within 10 units that roll
/// over this ped get bloody (`CLocalisation::Blood()`, audio event 120) and the vehicle a small push. `car` /
/// `bike` (both 0 at first) are taken from vehicles of class 0 / 9; any other class keeps the previous vehicle's
/// (original behaviour) while its own speed is tested.
#[hook(addr = 0x6AE0D0, conv = thiscall, name = "CPed::MakeTyresMuddySectorList<CPtrListSingleLink<CPhysical*>>", this = "CPed",
       expect = "81 EC 84 00 00 00 8B 84 24 88 00 00 00",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "vehicles of a class other than 0 / 9 reuse the previous car / bike (original behaviour)")]
pub fn ped_make_tyres_muddy_sector_list(this: GPtr<u8>, list: GPtr<u8>) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    let (mut car, mut bike) = (GPtr::<u8>::NULL, GPtr::<u8>::NULL);
    while !node.is_null() {
        let v = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let scan = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(v, 0x2C) == scan {
            continue;
        }
        wr::<u16>(v, 0x2C, scan);
        let pp = vec3(ent_pos(this));
        let vp = vec3(ent_pos(v));
        if !within(pp[0], vp[0], 10.0) || !within(pp[1], vp[1], 10.0) {
            continue;
        }
        match rd::<i32>(v, 0x590) {
            0 => (car, bike) = (v, GPtr::NULL),
            9 => (car, bike) = (GPtr::NULL, v),
            _ => {}
        }
        let (sx, sy) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)));
        if sx.fmul(sx).fadd(sy.fmul(sy)).fcom(x(0.05)).test_any(0x41) {
            continue;
        }
        if !car.is_null() {
            for w in 0..4isize {
                if rd::<u8>(car, 0x820 + w) != 0 || x(rd::<f32>(car, 0x7D4 + 4 * w)).fcom(x(1.0)).test_parity(5) {
                    continue;
                }
                let cm = col_model(car);
                let (max_x, max_y, min_y) = (rd::<f32>(cm, 0xC), rd::<f32>(cm, 0x10), rd::<f32>(cm, 0x4));
                // w0 / w1 load max.x with fld / fchs, w2 / w3 copy it as a dword; the y values go through fld / fstp
                let mut o = match w {
                    0 => [f32_of(x(max_x).fchs()), f32_of(x(max_y)), 0.0],
                    1 => [f32_of(x(max_x).fchs()), f32_of(x(min_y)), 0.0],
                    2 => [max_x, f32_of(x(max_y)), 0.0],
                    _ => [max_x, f32_of(x(min_y)), 0.0],
                };
                let Some(wp) = wheel_on_ped(this, car, &mut o) else { continue };
                if localisation_blood() != 0 {
                    wr::<u8>(car, 0x820 + w, 1);
                    add_audio_event(car.byte_add(0x138), 0x78, 0);
                }
                if x(rd::<f32>(car, 0x8C)).fcom(x(500.0)).test_any(0x41) {
                    continue;
                }
                let up = f32_of(cap1(x(rd::<f32>(this, 0x8C)).fmul_m(0.001)).fmul_m(50.0));
                push_vehicle(car, wp, up, || f32_of(cap1(x(rd::<f32>(this, 0x90)).fmul_m(0.0005)).fmul_m(50.0)));
            }
        } else if !bike.is_null() {
            for w in 0..2isize {
                if rd::<u8>(bike, 0x74C + w) != 0 || x(rd::<f32>(bike, 0x710 + 8 * w)).fcom(x(1.0)).test_parity(5) {
                    continue;
                }
                let cm = col_model(bike);
                let y = rd::<f32>(cm, if w == 0 { 0x10 } else { 0x4 });
                let mut o = [0.0, f32_of(x(y).fmul_m(0.8)), 0.0];
                let Some(wp) = wheel_on_ped(this, bike, &mut o) else { continue };
                if localisation_blood() != 0 {
                    wr::<u8>(bike, 0x74C + w, 1);
                    add_audio_event(bike.byte_add(0x138), 0x78, 0);
                }
                if x(rd::<f32>(bike, 0x8C)).fcom(x(100.0)).test_any(0x41) {
                    continue;
                }
                push_vehicle(bike, wp, 10.0, || 10.0);
            }
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PED_MAKE_TYRES_MUDDY_SECTOR_LIST];
