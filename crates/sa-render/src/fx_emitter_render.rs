//! `FxEmitterBP_c::Render` (0x4A2C40, batch `writero-render-1`): the particle emitter's per-frame
//! renderer (gta-reversed has it unreversed). Ported from the 1.0 US bytes; the x87 stack values the
//! original keeps across instructions are modelled with `sa_math::x87::X87` (full width), every `fstp dword` is a
//! `to_f32`. Verified by L2 (VERIFIER-10, docs/verify/verdicts-gen2b.tsv).
//!
//! Per particle the original keeps its locals in one 0x1F8-byte frame; the ones that live across
//! iterations are kept here too: the render-info block (`FxInfoManager_c::ProcessRenderInfo` rewrites
//! only its defaults, the rest stays from the previous particle), the texture raster local (read
//! unwritten when an animated texture id is not 1..4) and the `brightness` argument slot, which the
//! original overwrites with the particle's brightness byte (byte +0x35 ≤ 100) and then keeps for the
//! following particles (original behaviour kept). Locals the original reads before any write (the
//! first iteration's raster local, render-info words no info wrote) start as zero here.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::{g, rd, wr, x, xi};

/// `g_fxMan` (`FxManager_c`) and its smoke system pointer `[0xA9AE20]`; `m_bHeatHazeEnabled` 0xA9AF38.
const FX_MAN: usize = 0xA9AE80;
const FX_SMOKE_SYSTEM: usize = 0xA9AE20;
const HEAT_HAZE_ENABLED: usize = 0xA9AF38;
/// The emitter blend-function table (`RwBlendFunction`, int32 per blend id).
const BLEND_FUNCTIONS: usize = 0x8A6230;
/// `.rdata` constants: 1/255, 1.0, 0.0, 0.5, 255.0, 2pi, the rand scale, the brightness-byte scale,
/// the bright-colour scale, the rotation-to-angle scale, the trail time-step scale.
const C_INV255: usize = 0x859A3C;
const C_ONE: usize = 0x858624;
const C_ZERO: usize = 0x858B50;
const C_HALF: usize = 0x858B8C;
const C_255: usize = 0x859AAC;
const C_TWO_PI: usize = 0x859E2C;
const C_RAND: usize = 0x858C7C;
const C_BRIGHT_BYTE: usize = 0x858C58;
const C_BRIGHT_COLOUR: usize = 0x858B88;
const C_ROT_ANGLE: usize = 0x85A7BC;
const C_TRAIL_STEP: usize = 0x858B38;
/// `CTimer::ms_fTimeStep`; the camera-relative position subtracted by trail mode 2 (0xB6FA14..1C).
const TIME_STEP: usize = 0xB7CB5C;
const TRAIL_CAM: usize = 0xB6FA14;

/// `RenderInfo_t` byte size (written by `ProcessRenderInfo` and the infos it calls).
const INFO_SIZE: usize = 0xA4;

orig! {
    /// `RwRenderStateSet(state, value)` (`RwEngineInstance` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `List_c::GetNumItems()` (0x4A8EC0, `mov eax, [ecx+8]`)
    fn list_get_num_items(this: GPtr<u8>) -> u32 = thiscall 0x4A8EC0 => sa_core::mvpb_app::list_get_num_items;
    /// `FxEmitterBP_c::RenderHeatHaze(RwCamera*, uint32 txdHash, float brightness)` (0x4A1940)
    fn render_heat_haze_call(this: GPtr<u8>, camera: u32, txd: u32, brightness: u32) = thiscall 0x4A1940 => fx_emitter_bp_render_heat_haze;
    /// `Fx_c::RenderBegin(RwRaster*, RwMatrix*, uint32 flags)` (0x4A13B0)
    fn render_begin(raster: u32, ltm: u32, flags: u32) = cdecl 0x4A13B0 => crate::fx::fx_render_begin;
    /// `Fx_c::RenderEnd()` (0x4A1600)
    fn render_end() = cdecl 0x4A1600 => crate::mvpa_render3::fx_render_end;
    /// `Fx_c::RenderAddTri_` (0x4A1410): three positions, three UVs, three RGBA colours (int dwords)
    fn render_add_tri(x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32, u0: u32, v0: u32, u1: u32, v1: u32, u2: u32, v2: u32, r0: u32, g0: u32, b0: u32, a0: u32, r1: u32, g1: u32, b1: u32, a1: u32, r2: u32, g2: u32, b2: u32, a2: u32) = cdecl 0x4A1410 => crate::mvpc_fx2::fx_render_add_tri;
    /// `FxManager_c::FxRwMatrixCreate()` (0x4A9440) / `FxRwMatrixDestroy(RwMatrix*)` (0x4A9460)
    fn matrix_create(this: GPtr<u8>) -> u32 = thiscall 0x4A9440 => crate::fx::fx_manager_fx_rw_matrix_create;
    fn matrix_destroy(this: GPtr<u8>, m: u32) = thiscall 0x4A9460 => crate::fx::fx_manager_fx_rw_matrix_destroy;
    /// `FxSystem_c::GetCompositeMatrix(RwMatrix*)` (0x4AA8C0)
    fn get_composite_matrix(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AA8C0 => crate::fx::fx_system_get_composite_matrix;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RwV3dNormalize(out, in)` (0x7ED9B0), ST0 length (discarded by every call here)
    fn normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwV3dLength(v)` (0x7EDAC0), ST0
    fn length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
    /// `FxInfoManager_c::ProcessRenderInfo(currTime, lifeRatio, timeStep, totalTime, useConst, out)` (0x4A4A80)
    fn process_render_info(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, out: GPtr<u8>) = thiscall 0x4A4A80 => crate::mvpc_fx1::info_manager_process_render_info;
    /// `FxPrtMult_c::FxPrtMult_c()` (0x4AB270)
    fn prt_mult_ctor(this: u32) -> u32 = thiscall 0x4AB270 => crate::generated::generated_setters::g_fx_prt_mult_c_4ab270;
    /// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float time, FxPrtMult_c*, float zRot, float lightMult, float limit, uint8 local)` (0x4AA440)
    fn add_particle(this: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, zrot: u32, light_mult: f32, limit: f32, local: u32) = thiscall 0x4AA440 => crate::mvpb_r1::fx_system_add_particle;
    /// `Fx_c::RotateVecAboutVec(out, in, axis, angle)` (0x4A1780)
    fn rotate_vec_about_vec(out: GPtr<u8>, inp: GPtr<u8>, axis: GPtr<u8>, angle: u32) = cdecl 0x4A1780 => crate::mvpc_fx1::fx_rotate_vec_about_vec;
}

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn fv(p: GPtr<u8>, off: isize) -> [f32; 3] {
    [rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8)]
}

fn ptr(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr().cast::<u8>())
}

/// `fld v; fcomp 0.0; test ah, 0x44; jp`: false only for a component equal to 0.0 (not NaN).
fn is_zero(v: f32) -> bool {
    x(v).fcom(x(gf(C_ZERO))) == Cmp::Equal
}

/// The smoke colour component clamp: `v = c * (1/255) + rb`; 1.0 when `1.0 < v` (full width), else
/// `f32(v)`, then 0.0 when `0.0 > f32(v)`.
fn smoke_component(c: f32, rb: f32) -> f32 {
    let v = x(c).fmul_m(gf(C_INV255)).fadd_m(rb);
    if !x(gf(C_ONE)).fcom(v).test_parity(5) {
        return f32::from_bits(0x3F80_0000);
    }
    let f = v.to_f32();
    if !x(gf(C_ZERO)).fcom(x(f)).test_any(0x41) { 0.0 } else { f }
}

/// The two cross products every orientation path ends with: `B = V x A`, `C = B x A` (the original's
/// operand order and rounding: B.z stays full width for C.x).
fn cross_basis(v: [f32; 3], a: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let bx = x(v[2]).fmul_m(a[1]).fsub(x(v[1]).fmul_m(a[2])).to_f32();
    let by = x(v[0]).fmul_m(a[2]).fsub(x(v[2]).fmul_m(a[0])).to_f32();
    let bz_w = x(v[1]).fmul_m(a[0]).fsub(x(v[0]).fmul_m(a[1]));
    let bz = bz_w.to_f32();
    let cx = bz_w.fmul_m(a[1]).fsub(x(by).fmul_m(a[2])).to_f32();
    let cy = x(bx).fmul_m(a[2]).fsub(x(bz).fmul_m(a[0])).to_f32();
    let cz = x(by).fmul_m(a[0]).fsub(x(bx).fmul_m(a[1])).to_f32();
    ([bx, by, bz], [cx, cy, cz])
}

/// Writes right = `b`, up = `a`, at = `c` into the RwMatrix `m`.
fn set_basis(m: GPtr<u8>, b: [f32; 3], a: [f32; 3], c: [f32; 3]) {
    for i in 0..3isize {
        wr::<f32>(m, 4 * i, b[i as usize]);
        wr::<f32>(m, 0x10 + 4 * i, a[i as usize]);
        wr::<f32>(m, 0x20 + 4 * i, c[i as usize]);
    }
}

/// `FxEmitterBP_c::Render(RwCamera* camera, uint32 txdHash, float brightness, bool heatHaze)`
/// (0x4A2C40, thiscall, `ret 0x10`).
///
/// - `heatHaze`: only `RenderHeatHaze(camera, txdHash, brightness)` when byte +0x3D is set.
/// - An info of type 0x4008 (heat haze) among the manager's infos (+0x2C count, +0x30 array): only
///   `g_fxMan.m_bHeatHazeEnabled = 1` when the particle list (+0x20) is not empty.
/// - Else, with particles: render states 0xC = alpha flag (byte +7), 0xA / 0xB = the blend table
///   entries of bytes +5 / +6 (without alpha: entries 1 / 0); `RenderBegin(texture0 raster, 0, 1)`.
///   Per particle: its position (local-to-system particles: through `GetCompositeMatrix` of a temporary
///   FX matrix); `ProcessRenderInfo(system time, f32(life / total life), 0, system length, 0, info)`;
///   a smoke type > -1 adds a smoke particle (random brightness from the direct `rand()` at 0x4A2E64,
///   colour clamped to [0, 1]) to the smoke system `[0xA9AE20]`. The orientation matrix (a temporary
///   FX matrix): trail modes 1 / 2 (velocity trail; mode 2 against the camera motion 0xB6FA14), the
///   direction info (velocity or fixed direction, against the camera-to-particle vector), the flat
///   matrix or the camera matrix; then the particle rotation (byte +0x36 * 2 when not 0xFF, wrapped
///   to [0, 2pi) with full-width loops) rotates the right vector about the at vector. Size, colour
///   (range / bright modes, byte multipliers +0x2C..+0x2F, brightness unless self-lit) and texture
///   (animated id 1 → texture 0, 2..4 → texture 1..3 or texture 0) follow; a texture change flushes
///   (`RenderEnd` + `RenderBegin`). Two triangles go to `RenderAddTri_`.
/// - `RenderEnd()`.
#[hook(addr = 0x4A2C40, conv = thiscall, name = "FxEmitterBP_c::Render", expect = "81 EC E8 01 00 00",
       route = "L2", hazards = "uninit-locals,heap-writes",
       recs = "grove-walk,driving",
       note = "direct rand() at 0x4A2E64; writes particle +0x38 (rotation); x87-heavy (full-width intermediates)")]
pub fn fx_emitter_bp_render(this: GPtr<u8>, camera: u32, txd: u32, brightness: u32, heat: u32) {
    let n = rd::<i32>(this, 0x2C);
    let infos = rd::<u32>(this, 0x30);
    let mut haze = false;
    for i in 0..n.max(0) {
        let info = g::<u32>(infos as usize + 4 * i as usize).read();
        if rd::<u16>(g::<u8>(info as usize), 4) == 0x4008 {
            haze = true;
            break;
        }
    }
    if heat as u8 != 0 {
        if rd::<u8>(this, 0x3D) != 0 {
            render_heat_haze_call(this, camera, txd, brightness);
        }
        return;
    }
    let list = this.byte_add(0x20);
    if haze {
        if list_get_num_items(list) as i32 > 0 {
            g::<u8>(HEAT_HAZE_ENABLED).write(1);
        }
        return;
    }
    if list_get_num_items(list) == 0 {
        return;
    }
    let tex0 = || rd::<u32>(g::<u8>(rd::<u32>(this, 0xC) as usize), 0);
    let mut cur_raster = tex0();
    if list_get_num_items(list) == 0 {
        return;
    }
    let alpha = rd::<u8>(this, 7);
    rw_render_state_set(0xC, alpha as u32);
    let blend = |i: isize| g::<u32>((BLEND_FUNCTIONS as isize + 4 * i) as usize).read();
    if alpha != 0 {
        rw_render_state_set(0xA, blend(rd::<i8>(this, 5) as isize));
        rw_render_state_set(0xB, blend(rd::<i8>(this, 6) as isize));
    } else {
        rw_render_state_set(0xA, blend(1));
        rw_render_state_set(0xB, blend(0));
    }
    render_begin(cur_raster, 0, 1);
    let cam_m: GPtr<u8> = g(rd::<u32>(g::<u8>(camera as usize), 4) as usize + 0x10);
    let mut prt = rd::<u32>(list, 0);
    let mut st = Frame { bright: f32::from_bits(brightness), raster: 0, info: [0; INFO_SIZE / 4], mult: [0; 7] };
    while prt != 0 {
        let p: GPtr<u8> = g(prt as usize);
        render_particle(this, p, cam_m, &mut cur_raster, &mut st, false);
        prt = rd::<u32>(p, 4);
    }
    render_end();
}

/// `FxEmitterBP_c::RenderHeatHaze(RwCamera* camera, uint32 txdHash, float brightness)` (0x4A1940,
/// thiscall, `ret 0xC`): with particles (+0x20, the count read twice): render states 0xC = 1, 0xA = 5,
/// 0xB = 2, `RenderBegin(texture0 raster, 0, 1)`; every particle as in [`fx_emitter_bp_render`] without
/// smoke and trails, its direction-info velocity used as is, colour bytes r/g/b = 0 (alpha from the
/// render info); then `RenderEnd()`. `txdHash` and `brightness` are not read.
#[hook(addr = 0x4A1940, conv = thiscall, name = "FxEmitterBP_c::RenderHeatHaze", expect = "81 EC 2C 01 00 00",
       route = "L2", hazards = "uninit-locals,heap-writes",
       recs = "grove-walk",
       note = "writes particle +0x38 (rotation); x87-heavy (full-width intermediates)")]
pub fn fx_emitter_bp_render_heat_haze(this: GPtr<u8>, camera: u32, _txd: u32, brightness: u32) {
    let list = this.byte_add(0x20);
    if list_get_num_items(list) == 0 {
        return;
    }
    let mut cur_raster = rd::<u32>(g::<u8>(rd::<u32>(this, 0xC) as usize), 0);
    if list_get_num_items(list) == 0 {
        return;
    }
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 2);
    render_begin(cur_raster, 0, 1);
    let cam_m: GPtr<u8> = g(rd::<u32>(g::<u8>(camera as usize), 4) as usize + 0x10);
    let mut prt = rd::<u32>(list, 0);
    let mut st = Frame { bright: f32::from_bits(brightness), raster: 0, info: [0; INFO_SIZE / 4], mult: [0; 7] };
    while prt != 0 {
        let p: GPtr<u8> = g(prt as usize);
        render_particle(this, p, cam_m, &mut cur_raster, &mut st, true);
        prt = rd::<u32>(p, 4);
    }
    render_end();
}

/// The locals that live across particles (see the module docs).
struct Frame {
    bright: f32,
    raster: u32,
    info: [u32; INFO_SIZE / 4],
    mult: [u32; 7],
}

#[allow(clippy::too_many_lines)]
/// One particle of [`fx_emitter_bp_render`] (`haze` = false) or [`fx_emitter_bp_render_heat_haze`]
/// (`haze` = true: no smoke, no trail, no zero-velocity fallback, colour bytes r/g/b = 0, alpha kept).
fn render_particle(this: GPtr<u8>, prt: GPtr<u8>, cam_m: GPtr<u8>, cur_raster: &mut u32, st: &mut Frame, haze: bool) {
    let fxm = g::<u8>(FX_MAN);
    let sys: GPtr<u8> = g(rd::<u32>(prt, 0x28) as usize);
    let raw = fv(prt, 0x10);
    // P: the particle position, through the system's composite matrix for local particles
    let mut pos = raw;
    if rd::<u8>(prt, 0x37) != 0 {
        let m = matrix_create(fxm);
        get_composite_matrix(sys, g(m as usize));
        transform_points(ptr(&mut pos), prt.byte_add(0x10), 1, g(m as usize));
        matrix_destroy(fxm, m);
    }
    let info = GPtr::from_raw(st.info.as_mut_ptr().cast::<u8>());
    let ratio = x(rd::<f32>(prt, 0xC)).fdiv_m(rd::<f32>(prt, 8)).to_f32();
    let len = rd::<u32>(g::<u8>(rd::<u32>(sys, 8) as usize), 0xC);
    process_render_info(this.byte_add(0x2C), rd::<u32>(sys, 0x54), ratio.to_bits(), 0, len, 0, info);

    if !haze && rd::<i8>(info, 0x7B) > -1 {
        smoke(prt, info, st);
    }

    let mb = matrix_create(fxm);
    let m: GPtr<u8> = g(mb as usize);
    for (o, v) in [(0x28, 0x3F80_0000u32), (0x14, 0x3F80_0000), (0, 0x3F80_0000)] {
        wr::<u32>(m, o, v);
    }
    let flags = rd::<u32>(m, 0xC) | 0x20003;
    for o in [0x10, 8, 4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0xC, flags);

    let cam = fv(cam_m, 0x30);
    let trail = if haze { 0 } else { rd::<i32>(info, 0x2C) };
    let tt = rd::<f32>(info, 0x30);
    let mut trail_vec = [0f32; 3];
    let c = if trail > 0 {
        if trail == 2 {
            let a = x(raw[0]).fsub_m(cam[0]);
            let b = x(raw[1]).fsub_m(cam[1]);
            let cz = x(raw[2]).fsub_m(cam[2]).to_f32();
            let t = x(gf(TIME_STEP)).fmul_m(gf(C_TRAIL_STEP));
            let vel = fv(prt, 0x1C);
            let vxt = x(vel[0]).fmul(t);
            let vyt = x(vel[1]).fmul(t).to_f32();
            let vzt = x(vel[2]).fmul(t).to_f32();
            let x1 = x(raw[0]).fsub(vxt);
            let y1 = x(raw[1]).fsub_m(vyt);
            let z1 = x(raw[2]).fsub_m(vzt).to_f32();
            let x2 = x1.fsub_m(gf(TRAIL_CAM)).to_f32();
            let y2 = y1.fsub_m(gf(TRAIL_CAM + 4));
            let z2 = x(z1).fsub_m(gf(TRAIL_CAM + 8)).to_f32();
            let dx = a.fsub_m(x2).to_f32();
            let dy = b.fsub(y2);
            let dz = x(cz).fsub_m(z2);
            trail_vec = [x(dx).fmul_m(tt).to_f32(), dy.fmul_m(tt).to_f32(), dz.fmul_m(tt).to_f32()];
        } else {
            let vel = fv(prt, 0x1C);
            let e1 = x(tt).fmul_m(vel[0]).fsubr_m(raw[0]);
            let e2 = x(tt).fmul_m(vel[1]).fsubr_m(raw[1]);
            let e3 = x(tt).fmul_m(vel[2]).fsubr_m(raw[2]).to_f32();
            trail_vec = [x(raw[0]).fsub(e1).to_f32(), x(raw[1]).fsub(e2).to_f32(), x(raw[2]).fsub_m(e3).to_f32()];
        }
        let mut a = trail_vec;
        if is_zero(a[0]) && is_zero(a[1]) && is_zero(a[2]) {
            a[2] = 1.0;
        } else {
            normalize(ptr(&mut a), ptr(&mut a));
        }
        let mut v = [x(raw[0]).fsub_m(cam[0]).to_f32(), x(raw[1]).fsub_m(cam[1]).to_f32(), x(raw[2]).fsub_m(cam[2]).to_f32()];
        normalize(ptr(&mut v), ptr(&mut v));
        let (b, cc) = cross_basis(v, a);
        set_basis(m, b, a, cc);
        cc
    } else if rd::<u8>(info, 0x79) != 0 {
        let mut a = if rd::<u8>(info, 0x7A) != 0 {
            let mut a = fv(prt, 0x1C);
            if !haze && is_zero(a[0]) && is_zero(a[1]) && is_zero(a[2]) {
                a[2] = 1.0;
            }
            a
        } else {
            fv(info, 0x7C)
        };
        normalize(ptr(&mut a), ptr(&mut a));
        let mut v = [x(pos[0]).fsub_m(cam[0]).to_f32(), x(pos[1]).fsub_m(cam[1]).to_f32(), x(pos[2]).fsub_m(cam[2]).to_f32()];
        normalize(ptr(&mut v), ptr(&mut v));
        let (b, cc) = cross_basis(v, a);
        set_basis(m, b, a, cc);
        cc
    } else {
        let src: GPtr<u8> = if rd::<u8>(info, 0x78) != 0 { info.byte_add(0x38) } else { cam_m };
        for i in 0..16isize {
            wr::<u32>(m, 4 * i, rd::<u32>(src, 4 * i));
        }
        fv(src, 0x20)
    };

    // rotation: byte +0x36 (not 0xFF) * 2, wrapped into [0, 2pi) on the x87 stack
    let b36 = rd::<u8>(prt, 0x36);
    if b36 != 0xFF {
        let v = xi(b36 as i32);
        wr::<f32>(prt, 0x38, v.fadd(v).to_f32());
    }
    let zero = x(gf(C_ZERO));
    let two_pi = gf(C_TWO_PI);
    if !x(rd::<f32>(prt, 0x38)).fcom(zero).test_parity(5) {
        let mut v = x(rd::<f32>(prt, 0x38));
        loop {
            v = v.fadd_m(two_pi);
            if v.fcom(zero).test_parity(5) {
                break;
            }
        }
        wr::<f32>(prt, 0x38, v.to_f32());
    }
    if !x(rd::<f32>(prt, 0x38)).fcom(x(two_pi)).c0() {
        let mut v = x(rd::<f32>(prt, 0x38));
        loop {
            v = v.fsub_m(two_pi);
            if v.fcom(x(two_pi)).c0() {
                break;
            }
        }
        wr::<f32>(prt, 0x38, v.to_f32());
    }
    let rot = rd::<f32>(prt, 0x38);
    let (a, b) = if !x(rot).fcom(zero).test_any(0x41) {
        let mut b = [0f32; 3];
        let mut cc = c;
        let angle = x(rot).fmul_m(gf(C_ROT_ANGLE)).to_f32();
        rotate_vec_about_vec(ptr(&mut b), m, ptr(&mut cc), angle.to_bits());
        let a = [
            x(c[1]).fmul_m(b[2]).fsub(x(c[2]).fmul_m(b[1])).to_f32(),
            x(c[2]).fmul_m(b[0]).fsub(x(c[0]).fmul_m(b[2])).to_f32(),
            x(c[0]).fmul_m(b[1]).fsub(x(c[1]).fmul_m(b[0])).to_f32(),
        ];
        (a, b)
    } else {
        (fv(m, 0x10), fv(m, 0))
    };
    matrix_destroy(fxm, mb);

    // size
    let inv255 = gf(C_INV255);
    let half = gf(C_HALF);
    let sx = xi(rd::<u8>(prt, 0x32) as i32).fmul_m(inv255).fsub_m(half).fmul_m(rd::<f32>(info, 0x14)).fadd_m(rd::<f32>(info, 0xC)).to_f32();
    wr::<f32>(info, 0xC, sx);
    let sy = xi(rd::<u8>(prt, 0x33) as i32).fmul_m(inv255).fsub_m(half).fmul_m(rd::<f32>(info, 0x18)).fadd_m(rd::<f32>(info, 0x10)).to_f32();
    wr::<f32>(info, 0x10, sy);
    let b30 = rd::<u8>(prt, 0x30);
    if b30 != 0xFF {
        let v = xi(b30 as i32).fmul_m(inv255).fmul_m(rd::<f32>(info, 0xC)).to_f32();
        wr::<f32>(info, 0xC, v);
        let v = xi(b30 as i32).fmul_m(inv255).fmul_m(rd::<f32>(info, 0x10)).to_f32();
        wr::<f32>(info, 0x10, v);
    }

    if haze {
        wr::<u8>(info, 0, 0);
        wr::<u8>(info, 1, 0);
        wr::<u8>(info, 2, 0);
    } else {
        colour(prt, info, st);
    }
    texture(this, info, cur_raster, st);
    if trail > 0 {
        let l = length(ptr(&mut trail_vec));
        wr::<f32>(info, 0x1C, l as f32);
        wr::<u32>(info, 0x20, 0);
    }
    emit_quad(info, pos, a, b);
}

/// The colour of [`fx_emitter_bp_render`] (0x4A37C0..0x4A3A97): red, green and alpha as f32 locals,
/// blue on the x87 stack; written back as the info's colour bytes (`_ftol`).
fn colour(prt: GPtr<u8>, info: GPtr<u8>, st: &mut Frame) {
    let inv255 = gf(C_INV255);
    let zero = x(gf(C_ZERO));
    let byte = |p: GPtr<u8>, o: isize| xi(rd::<u8>(p, o) as i32);
    let mut red = byte(info, 0).to_f32();
    let mut green = byte(info, 1).to_f32();
    let mut blue = byte(info, 2);
    let mut al = byte(info, 3).to_f32();
    match rd::<i32>(info, 8) {
        1 => {
            red = byte(info, 4).fsub_m(red).fmul(byte(prt, 0x32).fmul_m(inv255)).fadd_m(red).to_f32();
            green = byte(info, 5).fsub_m(green).fmul(byte(prt, 0x33).fmul_m(inv255)).fadd_m(green).to_f32();
            blue = blue.fadd(byte(info, 6).fsub(blue).fmul(byte(prt, 0x34).fmul_m(inv255)));
        }
        2 => {
            let k = byte(prt, 0x32).fmul_m(gf(C_BRIGHT_COLOUR)).fsub_m(gf(C_ONE)).fmul(byte(info, 7));
            red = x(red).fadd(k).to_f32();
            green = x(green).fadd(k).to_f32();
            blue = blue.fadd(k);
            if !zero.fcom(x(red)).test_any(0x41) {
                red = 0.0;
            }
            if !zero.fcom(x(green)).test_any(0x41) {
                green = 0.0;
            }
            if !zero.fcom(blue).test_any(0x41) {
                blue = zero;
            }
            let top = x(gf(C_255));
            if !top.fcom(x(red)).test_parity(5) {
                red = f32::from_bits(0x437F_0000);
            }
            if !top.fcom(x(green)).test_parity(5) {
                green = f32::from_bits(0x437F_0000);
            }
            if !top.fcom(blue).test_parity(5) {
                blue = top;
            }
        }
        _ => {}
    }
    let mul = |o: isize| {
        let b = rd::<u8>(prt, o);
        (b != 0xFF).then(|| xi(b as i32).fmul_m(inv255))
    };
    if let Some(t) = mul(0x2C) {
        red = t.fmul_m(red).to_f32();
    }
    if let Some(t) = mul(0x2D) {
        green = t.fmul_m(green).to_f32();
    }
    if let Some(t) = mul(0x2E) {
        blue = blue.fmul(t);
    }
    if let Some(t) = mul(0x2F) {
        al = t.fmul_m(al).to_f32();
    }
    let (r8, g8, b8) = if rd::<u8>(info, 0x37) != 0 {
        (x(red).ftol() as u8, x(green).ftol() as u8, blue.ftol() as u8)
    } else {
        let b35 = rd::<u8>(prt, 0x35);
        if b35 <= 100 {
            st.bright = xi(b35 as i32).fmul_m(gf(C_BRIGHT_BYTE)).to_f32();
        }
        (x(red).fmul_m(st.bright).ftol() as u8, x(green).fmul_m(st.bright).ftol() as u8, blue.fmul_m(st.bright).ftol() as u8)
    };
    wr::<u8>(info, 0, r8);
    wr::<u8>(info, 1, g8);
    wr::<u8>(info, 2, b8);
    wr::<u8>(info, 3, x(al).ftol() as u8);
}

/// The texture raster (animated id 1 → texture 0, 2..4 → texture 1..3 when set, else texture 0, other
/// ids keep the previous raster); a change flushes the batch (`RenderEnd` + `RenderBegin(raster, 0, 1)`).
fn texture(this: GPtr<u8>, info: GPtr<u8>, cur_raster: &mut u32, st: &mut Frame) {
    let tex_raster = |o: isize| rd::<u32>(g::<u8>(rd::<u32>(this, o) as usize), 0);
    let tex_or_0 = |o: isize| if rd::<u32>(this, o) != 0 { tex_raster(o) } else { tex_raster(0xC) };
    st.raster = if rd::<u8>(info, 0x34) != 0 {
        match rd::<u8>(info, 0x35) {
            1 => tex_raster(0xC),
            2 => tex_or_0(0x10),
            3 => tex_or_0(0x14),
            4 => tex_or_0(0x18),
            _ => st.raster,
        }
    } else {
        tex_raster(0xC)
    };
    if *cur_raster != st.raster {
        render_end();
        *cur_raster = st.raster;
        render_begin(st.raster, 0, 1);
    }
}

/// The two triangles of the particle quad (0x4A3B7D..0x4A3E73): `A` (up) scaled by top / bottom * size
/// Y, `B` (right) by left / right * size X, the sums rounded where the original stores them.
fn emit_quad(info: GPtr<u8>, p: [f32; 3], a: [f32; 3], b: [f32; 3]) {
    let top = rd::<f32>(info, 0x1C);
    let bottom = rd::<f32>(info, 0x20);
    let left = rd::<f32>(info, 0x24);
    let right = rd::<f32>(info, 0x28);
    let sx = rd::<f32>(info, 0xC);
    let sy = rd::<f32>(info, 0x10);
    let ty = x(top).fmul_m(sy);
    let at = [x(a[0]).fmul(ty), x(a[1]).fmul(ty), ty.fmul_m(a[2])];
    let by = x(bottom).fmul_m(sy);
    let ab = [x(a[0]).fmul(by).to_f32(), x(a[1]).fmul(by).to_f32(), by.fmul_m(a[2]).to_f32()];
    let lx = x(left).fmul_m(sx);
    let bl = [x(b[0]).fmul(lx).to_f32(), x(b[1]).fmul(lx).to_f32(), lx.fmul_m(b[2]).to_f32()];
    let rx = x(right).fmul_m(sx);
    let brx = x(b[0]).fmul(rx).to_f32();
    let bry = x(b[1]).fmul(rx).to_f32();
    let brz = rx.fmul_m(b[2]);
    // corner sums kept as f32 locals
    let s1 = [x(brx).fadd_m(ab[0]).to_f32(), x(bry).fadd_m(ab[1]).to_f32(), x(ab[2]).fadd(brz).to_f32()];
    let s0 = [x(bl[0]).fadd(at[0]).to_f32(), x(bl[1]).fadd(at[1]).to_f32(), x(bl[2]).fadd(at[2]).to_f32()];
    let v2 = [
        x(brx).fadd(at[0]).fadd_m(p[0]).to_f32(),
        x(bry).fadd(at[1]).fadd_m(p[1]).to_f32(),
        brz.fadd(at[2]).fadd_m(p[2]).to_f32(),
    ];
    let plus = |s: [f32; 3]| [x(s[0]).fadd_m(p[0]).to_f32(), x(s[1]).fadd_m(p[1]).to_f32(), x(s[2]).fadd_m(p[2]).to_f32()];
    let v0 = plus(s0);
    let v1 = plus(s1);
    let col = [rd::<u8>(info, 0) as u32, rd::<u8>(info, 1) as u32, rd::<u8>(info, 2) as u32, rd::<u8>(info, 3) as u32];
    let one = 0x3F80_0000u32;
    let tri = |q0: [f32; 3], q1: [f32; 3], q2: [f32; 3], uv: [u32; 6]| {
        render_add_tri(
            q0[0].to_bits(), q0[1].to_bits(), q0[2].to_bits(), q1[0].to_bits(), q1[1].to_bits(), q1[2].to_bits(),
            q2[0].to_bits(), q2[1].to_bits(), q2[2].to_bits(), uv[0], uv[1], uv[2], uv[3], uv[4], uv[5],
            col[0], col[1], col[2], col[3], col[0], col[1], col[2], col[3], col[0], col[1], col[2], col[3],
        );
    };
    tri(v0, v1, v2, [0, 0, one, one, one, 0]);
    let w2 = [
        x(bl[0]).fadd_m(ab[0]).fadd_m(p[0]).to_f32(),
        x(bl[1]).fadd_m(ab[1]).fadd_m(p[1]).to_f32(),
        x(bl[2]).fadd_m(ab[2]).fadd_m(p[2]).to_f32(),
    ];
    tri(v1, v0, w2, [one, one, 0, 0, 0, one]);
}

/// A smoke type > -1 (0x4A2E44): `b` = smoke brightness (+0x98); `rb = f32((rand() * [0x858C7C] * (b -
/// -b) + -b) / 255)`; a fresh `FxPrtMult_c` gets the clamped smoke colour (+0x88..+0x90, see
/// [`smoke_component`]), alpha `f32(+0x94 / 255)`, size +0x9C, +0x14 = 1.0, life +0xA0; then
/// `[0xA9AE20]->AddParticle(the particle's own position, zero velocity, 0, mult, -1.0, 1.2, 0.6, 0)`.
fn smoke(prt: GPtr<u8>, info: GPtr<u8>, st: &mut Frame) {
    let b = rd::<f32>(info, 0x98);
    let neg = x(b).fchs().to_f32();
    let r = sa_core::orig::at_site(0x4A2E64, sa_core::crt::rand);
    let rb = xi(r).fmul_m(gf(C_RAND)).fmul(x(b).fsub_m(neg)).fadd_m(neg).fmul_m(gf(C_INV255)).to_f32();
    let mult = GPtr::from_raw(st.mult.as_mut_ptr().cast::<u8>());
    prt_mult_ctor(mult.addr() as u32);
    let red = smoke_component(rd::<f32>(info, 0x88), rb);
    wr::<f32>(mult, 0, red);
    let green = smoke_component(rd::<f32>(info, 0x8C), rb);
    wr::<f32>(mult, 4, green);
    let blue = smoke_component(rd::<f32>(info, 0x90), rb);
    wr::<f32>(mult, 8, blue);
    wr::<f32>(mult, 0xC, x(rd::<f32>(info, 0x94)).fmul_m(gf(C_INV255)).to_f32());
    wr::<u32>(mult, 0x10, rd::<u32>(info, 0x9C));
    wr::<u32>(mult, 0x14, 0x3F80_0000);
    wr::<u32>(mult, 0x18, rd::<u32>(info, 0xA0));
    let mut pos = fv(prt, 0x10);
    let mut vel = [0f32; 3];
    add_particle(
        g(g::<u32>(FX_SMOKE_SYSTEM).read() as usize),
        ptr(&mut pos),
        ptr(&mut vel),
        0,
        mult,
        0xBF80_0000,
        f32::from_bits(0x3F99_999A),
        f32::from_bits(0x3F19_999A),
        0,
    );
}

pub const HOOKS: &[HookReg] = &[HOOK_FX_EMITTER_BP_RENDER, HOOK_FX_EMITTER_BP_RENDER_HEAT_HAZE];
