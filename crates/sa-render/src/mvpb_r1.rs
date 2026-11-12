//! WRITER-MVP-B (batch `mvpb-render-1`): RENDER functions on the boot → menu → in-world path that had
//! no port (`CGamma::SetGamma`, `CDraw::CalculateAspectRatio`, `CPlantMgr::SetPlantFriendlyFlagInAtomicMI`,
//! `CSprite2d::Draw(8 floats, CRGBA)`, `CClock::Update`, `CSkidmarks::Update`,
//! `CRealTimeShadowManager::ReturnRealTimeShadow`, `CCoronas::RegisterCorona` (both overloads)).
//!
//! Ported from the 1.0 US bytes; gta-reversed (`Gamma.cpp`, `Draw.cpp`, `PlantMgr.cpp`, `Sprite2d.cpp`,
//! `Clock.cpp`, `Skidmarks.cpp`, `RealTimeShadowManager.cpp`, `Coronas.cpp`) as reading aid. x87
//! arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

fn g<T: GameData>(a: u32) -> GPtr<T> {
    GPtr::from_addr(a as usize)
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

orig! {
    /// `RwD3D9GetCurrentD3DDevice()` (0x7F9D50)
    fn rw_d3d9_get_current_d3d_device() -> GPtr<GPtr<u32>> = cdecl 0x7F9D50 => unported;
    /// `IDirect3DDevice9::GetGammaRamp(iSwapChain, D3DGAMMARAMP*)` (COM slot 0x58)
    fn d3d_get_gamma_ramp(dev: GPtr<GPtr<u32>>, swap_chain: u32, ramp: GPtr<u8>) = stdcall vslot 0x58 => unported;
    /// `IDirect3DDevice9::SetGammaRamp(iSwapChain, flags, const D3DGAMMARAMP*)` (COM slot 0x54)
    fn d3d_set_gamma_ramp(dev: GPtr<GPtr<u32>>, swap_chain: u32, flags: u32, ramp: GPtr<u8>) = stdcall vslot 0x54 => unported;
    /// CRT `pow(double, double)` (0x8220F0)
    fn crt_pow(x: f64, y: f64) -> f64 = cdecl 0x8220F0 => unported;
    /// `SurfaceInfos_c::CreatesPlants(surface)` (0x55E9D0, bool in AL)
    fn surface_creates_plants(this: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E9D0 => unported;
    /// `SurfaceInfos_c::CreatesObjects(surface)` (0x55E9F0, bool in AL)
    fn surface_creates_objects(this: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E9F0 => unported;
    /// `CSprite2d::SetVertices(x1, y1, x2, y2, x3, y3, x4, y4, c1, c2, c3, c4)` (0x727590)
    #[allow(clippy::too_many_arguments)]
    fn sprite2d_set_vertices(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x727590 => unported;
    /// `RwRenderStateSet(state, value)` (RwEngineInstance slot 0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// 0x734E90: the 2D primitive draw used by `CSprite2d` (`RwIm2DRenderPrimitive(type, verts, n)`)
    fn im2d_render_primitive(prim: u32, verts: GPtr<u8>, n: u32) -> i32 = cdecl 0x734E90 => unported;
    /// `CStats::IncrementStat(uint16 stat, float value)` (0x55C180; stat pushed as a dword)
    fn stats_increment_stat(stat: u32, value: u32) = cdecl 0x55C180 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(this: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0)
    fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, result through a hidden pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(this: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

orig! {
    /// `CCoronas::RegisterCorona(id, attachTo, r, g, b, a, const CVector& pos, size, range, RwTexture*
    /// tex, flare, refl, los, trail, normalAngle, neonFade, pullTowardsCam, fullBrightAtStart,
    /// fadeSpeed, onlyFromBelow, reflectionDelay)` (0x6FC180; every argument a dword slot)
    #[allow(clippy::too_many_arguments)]
    fn register_corona_o(id: u32, attach: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, range: u32, tex: u32, flare: u32, refl: u32, los: u32, trail: u32, normal_angle: u32, neon_fade: u32, pull: u32, full_bright: u32, fade_speed: u32, only_from_below: u32, refl_delay: u32) = cdecl 0x6FC180 => crate::mvpb_r1::register_corona_type;
}

// ------------------------------------------------------------------ CGamma ------------------------

/// `CGamma::SetGamma(float gamma, bool force)` (0x747200, thiscall, `ret 8`): `gamma *= 0.7` (stored
/// back as f32). Nothing unless `m_bInitialised` (+0). `m_fCurrentGamma` (+0xC) = gamma; when gamma
/// equals `m_fTargetGamma` (+0x8, ordered compare) only `force` continues, else the target is set.
/// With a D3D device: `v = (1.0 - gamma) + 0.2` (f32); the first time (`0xC920C8`) the current ramp
/// is saved (`GetGammaRamp(0, 0xC8D4C8)`); then for i in 0..256 the three channels of the ramp at
/// 0xC8DAC8 (R), 0xC8DCC8 (G), 0xC8DEC8 (B) get `ftol(pow((i + 1) * (1/256), v) * 65535.0)` (the
/// product rounded on the x87 and stored as a double, `pow` from the CRT), and
/// `SetGammaRamp(0, D3DSGR_CALIBRATE, 0xC8DAC8)`.
#[hook(addr = 0x747200, conv = thiscall, name = "CGamma::SetGamma", this = "CGamma", expect = "55 8B EC 83 E4 F8",
       route = "split", hazards = "none",
       note = "D3D device calls (gamma ramp); frontend/menu settings only")]
pub fn set_gamma(this: GPtr<u8>, gamma: f32, force: u32) {
    let gm = X87::from_f32(gamma).fmul_m(f(0x3F33_3333)).to_f32();
    if this.cast::<u32>().read() == 0 {
        return;
    }
    let target = this.byte_add(8).cast::<f32>();
    // `test ah,0x44; jnp` taken (odd parity) = ordered equal
    let equal = !X87::from_f32(gm).fcom(X87::from_f32(target.read())).test_parity(0x44);
    this.byte_add(0xC).cast::<f32>().write(gm);
    if equal {
        if force as u8 == 0 {
            return;
        }
    } else {
        target.write(gm);
    }
    let dev = rw_d3d9_get_current_d3d_device();
    if dev.is_null() {
        return;
    }
    let v = X87::one().fsub_m(gm).fadd_m(f(0x3E4C_CCCD)).to_f32();
    let saved = g::<u8>(0xC920C8);
    if saved.read() == 0 {
        saved.write(1);
        d3d_get_gamma_ramp(rw_d3d9_get_current_d3d_device(), 0, g(0xC8D4C8));
    }
    let mut i = 0i32;
    while i < 0x100 {
        let x = X87::from_i32(i + 1).fmul_m(f(0x3B80_0000)).to_f64();
        let p = crt_pow(x, X87::from_f32(v).to_f64());
        let r = X87::from_f64_exact(p).fmul(X87::from_f64_exact(65535.0)).ftol() as u16;
        g::<u16>(0xC8DEC8).wrapping_index(i).write(r);
        g::<u16>(0xC8DCC8).wrapping_index(i).write(r);
        g::<u16>(0xC8DAC8).wrapping_index(i).write(r);
        i += 1;
    }
    d3d_set_gamma_ramp(dev, 0, 1, g(0xC8DAC8));
}

// ------------------------------------------------------------------ CDraw -------------------------

/// `CDraw::CalculateAspectRatio()` (0x6FF420): `ms_fAspectRatio` (0xC3EFA4) = 16/9 (0x862CE4) when the
/// menu's widescreen flag (0xBA6793) is set, else 5/4 (0x8595F0) when `TheCamera.m_bWideScreenOn`
/// (0xB6F065), else 4/3 (0x85F09C); the value stays in ST0 (`fst`).
#[hook(addr = 0x6FF420, conv = cdecl, name = "CDraw::CalculateAspectRatio", expect = "A0 93 67 BA 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn calculate_aspect_ratio() -> X87 {
    let bits = if g::<u8>(0xBA6793).read() != 0 {
        0x3FE3_8E39
    } else if g::<u8>(0xB6F065).read() != 0 {
        0x3FA0_0000
    } else {
        0x3FAA_AAAB
    };
    let v = X87::from_f32(f(bits));
    g::<f32>(0xC3EFA4).write(v.to_f32());
    v
}

// ------------------------------------------------------------------ CPlantMgr ---------------------

/// `CPlantMgr::SetPlantFriendlyFlagInAtomicMI(CAtomicModelInfo*)` (0x5DB650, cdecl, bool in AL): clears
/// bit 1 of byte +0x13 (`bAtomicFlag` "plant friendly"); with a col model (`m_pColModel` +0x14, its
/// `m_pColData` +0x2C) and a non-zero `m_nNumTriangles` (int16 +0x4), the first triangle (8 bytes each
/// at `m_pTriangles` +0x18, surface byte +6) whose surface `CreatesPlants` or `CreatesObjects` sets the
/// bit again and returns true; else false.
#[hook(addr = 0x5DB650, conv = cdecl, name = "CPlantMgr::SetPlantFriendlyFlagInAtomicMI", expect = "8B 44 24 04 80 60 13 FD",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "writes the model info's flag byte +0x13 (model info table)")]
pub fn set_plant_friendly_flag_in_atomic_mi(mi: GPtr<u8>) -> bool {
    let flags = mi.byte_add(0x13);
    flags.write(flags.read() & 0xFD);
    let data = mi.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x2C).cast::<GPtr<u8>>().read();
    if data.is_null() {
        return false;
    }
    let n = data.byte_add(4).cast::<i16>().read() as i32;
    if n == 0 {
        return false;
    }
    let mut i = 0;
    while i < n {
        let tri = data.byte_add(0x18).cast::<GPtr<u8>>().read().byte_add(i as isize * 8);
        let s = tri.byte_add(6).read() as u32;
        if surface_creates_plants(g(0xB79538), s) != 0 || surface_creates_objects(g(0xB79538), tri.byte_add(6).read() as u32) != 0 {
            flags.write(flags.read() | 2);
            return true;
        }
        i += 1;
    }
    false
}

// ------------------------------------------------------------------ CSprite2d ---------------------

/// `CSprite2d::Draw(float x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA& c)` (0x728520, thiscall, `ret
/// 0x24`): `SetVertices(x1..y4, c, c, c, c)`, texture raster state = `m_pTexture->raster` (NULL without
/// a texture), the 4-vertex triangle fan (`0x734E90(5, maVertices 0xC80468, 4)`), raster state NULL.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x728520, conv = thiscall, name = "CSprite2d::Draw-ffffffffCRGBA", this = "CSprite2d", expect = "8B 44 24 24 8B 54 24 18",
       route = "L2", hazards = "none")]
pub fn sprite2d_draw_quad(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, c: GPtr<u8>) {
    sprite2d_set_vertices(a0, a1, a2, a3, a4, a5, a6, a7, c, c, c, c);
    let tex = this.cast::<GPtr<u32>>().read();
    if tex.is_null() {
        rw_render_state_set(1, 0);
    } else {
        rw_render_state_set(1, tex.read());
    }
    im2d_render_primitive(5, g(0xC80468), 4);
    rw_render_state_set(1, 0);
}

// ------------------------------------------------------------------ CClock ------------------------

/// `CClock::Update()` (0x52CF10): when `CTimer::m_snTimeInMilliseconds - ms_nLastClockTick` (signed)
/// exceeds `ms_nMillisecondsPerGameMinute`, or the fast-clock cheat (0x96913B) is on, and neither
/// freeze cheat (0x969167, 0x969168) is on: minutes+1, `LastClockTick += msPerMinute` (= now with the
/// fast clock); at 60 minutes the hour, at 24 hours the weekday (7 → 1), day of month, `CStats::
/// IncrementStat(0x86, 1.0)` and the month by the days table (0x8CCF23, 12 → 1). Always:
/// `ms_nGameClockSeconds = (now - LastClockTick) * 60 / msPerMinute` (unsigned, 16-bit store).
#[hook(addr = 0x52CF10, conv = cdecl, name = "CClock::Update", expect = "56 8B 35 58 01 B7 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clock_update() {
    let last = g::<u32>(0xB70158);
    let now = g::<u32>(0xB7CB84).read();
    let per_min = g::<u32>(0xB7015C).read();
    let fast = g::<u8>(0x96913B).read();
    let due = (now.wrapping_sub(last.read()) as i32) > per_min as i32 || fast != 0;
    if due && g::<u8>(0x969167).read() == 0 && g::<u8>(0x969168).read() == 0 {
        let minutes = g::<u8>(0xB70152);
        minutes.write(minutes.read().wrapping_add(1));
        last.write(last.read().wrapping_add(per_min));
        if fast != 0 {
            last.write(now);
        }
        if minutes.read() >= 60 {
            let hours = g::<u8>(0xB70153);
            let h = hours.read().wrapping_add(1);
            minutes.write(0);
            hours.write(h);
            if h >= 24 {
                let weekday = g::<u8>(0xB7014E);
                let wd = weekday.read();
                hours.write(0);
                let day = g::<u8>(0xB70154);
                day.write(day.read().wrapping_add(1));
                if wd == 7 {
                    weekday.write(1);
                } else {
                    weekday.write(wd.wrapping_add(1));
                }
                stats_increment_stat(0x86, 0x3F80_0000);
                let month = g::<u8>(0xB70155);
                let m = month.read();
                if day.read() >= g::<u8>(0x8CCF23 + m as u32).read() {
                    let m2 = m.wrapping_add(1);
                    day.write(1);
                    month.write(m2);
                    if m2 > 12 {
                        month.write(1);
                    }
                }
            }
        }
    }
    let secs = g::<u32>(0xB7CB84).read().wrapping_sub(last.read()).wrapping_mul(60) / g::<u32>(0xB7015C).read();
    g::<u16>(0xB70150).write(secs as u16);
}

// ------------------------------------------------------------------ CSkidmarks --------------------

/// `CSkidmarks::Update()` (0x7205C0): for each of the 32 skidmarks (0x158 bytes from 0xC79BF0; state
/// byte +0xE, `m_bActive` +0xF, int16 `m_nNumPieces` +0xC, fade-start +0x0, disappear +0x4): state 1
/// (`JUST_UPDATED`) not active this frame → state 2 (`FADING`) with fade/disappear times now + 2.5/5 s
/// (< 4 pieces), 5/10 s (< 9) or 10/20 s; state 2 past the disappear time (unsigned) → 0. Every
/// skidmark's `m_bActive` is cleared.
#[hook(addr = 0x7205C0, conv = cdecl, name = "CSkidmarks::Update", expect = "8B 15 84 CB B7 00 B8 FE",
       route = "L2", hazards = "none", recs = "driving")]
pub fn skidmarks_update() {
    let now = g::<u32>(0xB7CB84).read();
    let mut s = 0xC79BFEu32;
    while s < 0xC7C6FE {
        let state = g::<u8>(s);
        match state.read() {
            1 => {
                if g::<u8>(s + 1).read() == 0 {
                    let n = g::<i16>(s - 2).read();
                    state.write(2);
                    let (fade, gone) = if n < 4 {
                        (2500, 5000)
                    } else if n < 9 {
                        (5000, 10000)
                    } else {
                        (10000, 20000)
                    };
                    g::<u32>(s - 0xE).write(now.wrapping_add(fade));
                    g::<u32>(s - 0xA).write(now.wrapping_add(gone));
                }
            }
            2 if now > g::<u32>(s - 0xA).read() => {
                state.write(0);
            }
            _ => {}
        }
        g::<u8>(s + 1).write(0);
        s += 0x158;
    }
}

// ------------------------------------------------------------------ CRealTimeShadowManager --------

/// `CRealTimeShadowManager::ReturnRealTimeShadow(CRealTimeShadow*)` (0x705B30, thiscall, `ret 4`): when
/// the manager is initialised (+0), the shadow's owner (`m_pOwner` +0) gets `m_pShadowData` (+0x134) =
/// NULL and the shadow's owner is cleared.
#[hook(addr = 0x705B30, conv = thiscall, name = "CRealTimeShadowManager::ReturnRealTimeShadow", this = "CRealTimeShadowManager", expect = "80 39 00 74 16",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "writes the owner entity's +0x134 and the shadow's +0 (pool/heap)")]
pub fn return_real_time_shadow(this: GPtr<u8>, shadow: GPtr<u8>) {
    if this.read() == 0 {
        return;
    }
    let owner = shadow.cast::<GPtr<u8>>().read();
    owner.byte_add(0x134).cast::<u32>().write(0);
    shadow.cast::<u32>().write(0);
}

// ------------------------------------------------------------------ CCoronas ----------------------

const CORONAS: u32 = 0xC3E058;
const NUM_CORONAS: u32 = 0xC3E038;

/// `CCoronas::RegisterCorona(..., eCoronaType type, ...)` (0x6FC580, cdecl): replaces the type argument
/// with `gpCoronaTexture[type]` (0xC3E000) and jumps to the texture overload (0x6FC180). The port calls
/// it with the same arguments; the original leaves the texture pointer in the caller's argument slot.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x6FC580, conv = cdecl, name = "CCoronas::RegisterCorona-texture", expect = "0F B6 54 24 28",
       unverified = "mvpb-render-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the original tail-jumps after rewriting its 10th argument slot; the port passes a fresh copy")]
pub fn register_corona_by_type(id: u32, attach: GPtr<u8>, r: u32, gg: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, range: u32, kind: u32, flare: u32, refl: u32, los: u32, trail: u32, normal_angle: u32, neon_fade: u32, pull: u32, full_bright: u32, fade_speed: u32, only_from_below: u32, refl_delay: u32) {
    let tex = g::<u32>(0xC3E000).wrapping_index((kind as u8) as i32).read();
    register_corona_o(id, attach, r, gg, b, a, pos, size, range, tex, flare, refl, los, trail, normal_angle, neon_fade, pull, full_bright, fade_speed, only_from_below, refl_delay);
}

/// `CCoronas::RegisterCorona(id, attachTo, r, g, b, a, const CVector& pos, size, range, RwTexture*, ...)`
/// (0x6FC180, cdecl). World position: with `attachTo`, `*attachTo->GetMatrix() * pos` (the matrix
/// allocated and updated from the placement when missing), else `pos`. Nothing when `range^2 <` the 2D
/// squared distance to the camera (x87, ordered). With `neonFade`: nothing when the 3D distance (FSQRT)
/// is < 35; below 50 the alpha becomes `ftol((d - 35) * a * (1/15))`. The corona with this id (first
/// of the 64, 0x3C bytes at 0xC3E058, id +0xC) is updated; if missing, a zero alpha returns, else the
/// first free slot (none: return) is initialised (`m_FadedIntensity` +0x30 = 255 with
/// `fullBrightAtStart` else 0, flags +0x34 |= 2, +0x36 &= ~4, +0x35 = 1, id, `NumCoronas`++). An
/// existing corona with faded intensity 0 and alpha 0 is freed (`id = 0`, `NumCoronas`--). Update:
/// colour, **`m_vPosn` = the unmodified `pos` argument** (original behaviour: the transformed position
/// is used only for the range tests), size, normal angle, range, texture, flare, reflection, the LOS bit,
/// `+0x31 = 1`, pull towards camera, fade speed, flag bits (neon fade, only from below, reflection
/// delay), and the attached entity (+0x38, `RegisterReference`) or NULL.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x6FC180, conv = cdecl, name = "CCoronas::RegisterCorona-type", expect = "83 EC 1C 55 56 8B 74 24",
       unverified = "mvpb-render-1", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk", note = "FSQRT distance (neonFade path); RegisterReference writes the entity's reference list")]
pub fn register_corona_type(id: u32, attach: GPtr<u8>, r: u32, gg: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, range: u32, tex: u32, flare: u32, refl: u32, los: u32, _trail: u32, normal_angle: u32, neon_fade: u32, pull: u32, full_bright: u32, fade_speed: u32, only_from_below: u32, refl_delay: u32) {
    let world = if !attach.is_null() {
        if attach.byte_add(0x14).cast::<u32>().read() == 0 {
            placeable_allocate_matrix(attach);
            simple_transform_update_matrix(attach.byte_add(4), attach.byte_add(0x14).cast::<GPtr<u8>>().read());
        }
        let mut tmp = [0f32; 3];
        let res = matrix_mul_vec(GPtr::from_raw(tmp.as_mut_ptr()).cast(), attach.byte_add(0x14).cast::<GPtr<u8>>().read(), pos);
        res.cast::<[f32; 3]>().read()
    } else {
        pos.cast::<[f32; 3]>().read()
    };
    let cam_mat = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cam_mat.is_null() { g::<f32>(0xB6F02C) } else { cam_mat.byte_add(0x30).cast::<f32>() };
    let cx = || X87::from_f32(cam.read());
    let cy = || X87::from_f32(cam.wrapping_index(1).read());
    let cz = || X87::from_f32(cam.wrapping_index(2).read());
    let dy = cy().fsub_m(world[1]);
    let dy2 = dy.fmul(cy().fsub_m(world[1]));
    let dx = cx().fsub_m(world[0]);
    let dx2 = dx.fmul(cx().fsub_m(world[0]));
    let d2 = dy2.fadd(dx2);
    let rr = X87::from_f32(f(range)).fmul_m(f(range));
    // fcompp: ST0 = range^2 vs ST1 = d2; `test ah,5; jnp` leaves when range^2 < d2
    if !rr.fcom(d2).test_parity(0x05) {
        return;
    }
    let mut alpha = a as u8;
    if neon_fade as u8 != 0 {
        let ex = cx().fsub_m(world[0]);
        let ey = cy().fsub_m(world[1]);
        let ez = cz().fsub_m(world[2]);
        let d = ez.fmul(ez).fadd(ey.fmul(ey)).fadd(ex.fmul(ex)).fsqrt();
        let d32 = d.to_f32();
        if !d.fcom(X87::from_f32(f(0x420C_0000))).test_parity(0x05) {
            return;
        }
        if !X87::from_f32(d32).fcom(X87::from_f32(f(0x4248_0000))).test_parity(0x05) {
            let v = X87::from_f32(d32)
                .fsub_m(f(0x420C_0000))
                .fmul(X87::from_i32(alpha as i32))
                .fmul_m(f(0x3D88_8889));
            alpha = v.ftol() as u8;
        }
    }
    let slot = |i: u32| CORONAS + i * 0x3C;
    let mut i = 0u32;
    while i < 0x40 && g::<u32>(slot(i) + 0xC).read() != id {
        i += 1;
    }
    let c;
    if i != 0x40 {
        c = slot(i);
        if g::<u8>(c + 0x30).read() == 0 && alpha == 0 {
            g::<u32>(NUM_CORONAS).write(g::<u32>(NUM_CORONAS).read().wrapping_sub(1));
            g::<u32>(c + 0xC).write(0);
            return;
        }
    } else {
        if alpha == 0 {
            return;
        }
        let mut j = 0u32;
        while j < 0x40 && g::<u32>(slot(j) + 0xC).read() != 0 {
            j += 1;
        }
        if j == 0x40 {
            return;
        }
        c = slot(j);
        g::<u8>(c + 0x30).write(if full_bright as u8 != 0 { 0xFF } else { 0 });
        g::<u8>(c + 0x34).write(g::<u8>(c + 0x34).read() | 2);
        g::<u8>(c + 0x36).write(g::<u8>(c + 0x36).read() & 0xFB);
        let n = g::<u32>(NUM_CORONAS).read().wrapping_add(1);
        g::<u8>(c + 0x35).write(1);
        g::<u32>(c + 0xC).write(id);
        g::<u32>(NUM_CORONAS).write(n);
    }
    g::<u8>(c + 0x2C).write(r as u8);
    g::<u8>(c + 0x2D).write(gg as u8);
    g::<u8>(c + 0x2E).write(b as u8);
    g::<u8>(c + 0x2F).write(alpha);
    let p = pos.cast::<u32>();
    g::<u32>(c).write(p.read());
    g::<u32>(c + 4).write(p.wrapping_index(1).read());
    g::<u32>(c + 8).write(p.wrapping_index(2).read());
    g::<u32>(c + 0x14).write(size);
    g::<u32>(c + 0x18).write(normal_angle);
    g::<u32>(c + 0x1C).write(range);
    g::<u32>(c + 0x10).write(tex);
    g::<u8>(c + 0x32).write(flare as u8);
    g::<u8>(c + 0x33).write(refl as u8);
    let fl = g::<u8>(c + 0x34).read();
    g::<u8>(c + 0x31).write(1);
    g::<u8>(c + 0x34).write(fl ^ ((fl ^ los as u8) & 1));
    g::<u32>(c + 0x20).write(pull);
    g::<u32>(c + 0x28).write(fade_speed);
    let bits = ((((refl_delay as u8) & 1) << 2 | (only_from_below as u8 & 1)) << 1)
        | (g::<u8>(c + 0x36).read() & 0xF4)
        | (neon_fade as u8 & 1);
    g::<u8>(c + 0x36).write(bits);
    if !attach.is_null() {
        g::<u8>(c + 0x36).write(bits | 0x10);
        g::<GPtr<u8>>(c + 0x38).write(attach);
        entity_register_reference(attach, g(c + 0x38));
    } else {
        g::<u8>(c + 0x36).write(bits & 0xEF);
        g::<u32>(c + 0x38).write(0);
    }
}

// ------------------------------------------------------------------ CShadows ----------------------

/// `CShadows::AffectColourWithLighting(uint8 shadowType, uint8 lighting, uint8 r, uint8 g, uint8 b,
/// uint8* outR, uint8* outG, uint8* outB)` (0x707850, cdecl). Multiplier 1.0 for shadow type 2; else
/// `min((1 - W) * 0.6 + 0.4, ((lighting & 15)/30 * (1 - W) + (lighting >> 4)/30 * W) * 0.7 + 0.3)` with
/// `W` = the float at 0x8D12C0 (the second term rounded to f32, the first kept on the x87; ordered
/// `a < b` keeps a). Each output = `ftol(channel * multiplier)` (low byte).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x707850, conv = cdecl, name = "CShadows::AffectColourWithLighting", expect = "8A 44 24 04 D9 05 24 86 85 00",
       unverified = "mvpb-render-1", route = "L2", hazards = "none")]
pub fn affect_colour_with_lighting(kind: u32, lighting: u32, r: u32, gg: u32, b: u32, out_r: GPtr<u8>, out_g: GPtr<u8>, out_b: GPtr<u8>) {
    let one = X87::from_f32(f(0x3F80_0000));
    let mut m = one;
    if kind as u8 != 2 {
        let w = g::<f32>(0x8D12C0).read();
        let a = one.fsub_m(w).fmul_m(f(0x3F19_999A)).fadd_m(f(0x3ECC_CCCD));
        let wx = X87::from_f32(w);
        let lo = X87::from_i32((lighting as u8 & 0xF) as i32).fmul_m(f(0x3D08_8889)).fmul(one.fsub(wx));
        let hi = X87::from_i32((lighting as u8 >> 4) as i32).fmul_m(f(0x3D08_8889)).fmul(wx);
        let v = lo.fadd(hi).fmul_m(f(0x3F33_3333)).fadd_m(f(0x3E99_999A)).to_f32();
        // `fcom v; test ah,5; jnp` keeps a when a < v (ordered)
        m = if !a.fcom(X87::from_f32(v)).test_parity(0x05) { a } else { X87::from_f32(v) };
    }
    out_r.write(X87::from_i32(r as u8 as i32).fmul(m).ftol() as u8);
    out_g.write(X87::from_i32(gg as u8 as i32).fmul(m).ftol() as u8);
    out_b.write(X87::from_i32(b as u8 as i32).fmul(m).ftol() as u8);
}

// ------------------------------------------------------------------ CMovingThings / CPlaneTrails --

orig! {
    /// `CPlaneTrails::Update()` (0x7174F0)
    fn plane_trails_update_o() = cdecl 0x7174F0 => crate::mvpb_r1::plane_trails_update;
    /// `CEscalators::Update()` (0x718580)
    fn escalators_update() = cdecl 0x718580 => unported;
    /// `CPlaneTrail::RegisterPoint(CVector)` (0x7172D0, the vector by value)
    fn plane_trail_register_point_o(this: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x7172D0 => crate::mvpb_r1::plane_trail_register_point;
    /// `CCoronas::RegisterCorona(..., eCoronaType, ...)` (0x6FC580)
    #[allow(clippy::too_many_arguments)]
    fn register_corona_by_type_o(id: u32, attach: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, range: u32, kind: u32, flare: u32, refl: u32, los: u32, trail: u32, normal_angle: u32, neon_fade: u32, pull: u32, full_bright: u32, fade_speed: u32, only_from_below: u32, refl_delay: u32) = cdecl 0x6FC580 => crate::mvpb_r1::register_corona_by_type;
    /// `CCoronas::UpdateCoronaCoors(id, const CVector&, range, normalAngle)` (0x6FC4D0)
    fn update_corona_coors_o(id: u32, pos: GPtr<u8>, range: u32, normal_angle: u32) = cdecl 0x6FC4D0 => crate::mvpb_r1::update_corona_coors;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(this: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CCollision::DistToLine(const CVector* lineStart, const CVector* lineEnd, const CVector* point)`
    /// (0x417610, float in ST0)
    fn collision_dist_to_line(a: GPtr<u8>, b: GPtr<u8>, p: GPtr<u8>) -> f32 = cdecl 0x417610 => unported;
}

/// `CMovingThings::Update()` (0x7185B0): `CPlaneTrails::Update()`, then (tail jump) `CEscalators::Update()`.
#[hook(addr = 0x7185B0, conv = cdecl, name = "CMovingThings::Update", expect = "E8 3B EF FF FF",
       unverified = "mvpb-render-1", route = "L3-only", hazards = "none", recs = "grove-walk")]
pub fn moving_things_update() {
    plane_trails_update_o();
    escalators_update();
}

/// `CPlaneTrail::RegisterPoint(CVector pt)` (0x7172D0, thiscall, `ret 0xC`; its first load sits in the
/// protection chunk 0x407485): 16 points (+0x0, 12 bytes) and times (+0xC0). When `time[0]` is set and
/// more than 2000 ms old (unsigned), every entry moves one slot back (15..1 from 14..0). `point[0] = pt`;
/// `time[0] = now` after a shift or when it was 0.
#[hook(addr = 0x7172D0, conv = thiscall, name = "CPlaneTrail::RegisterPoint", this = "CPlaneTrail", expect = "51 90 E9 AE 01 CF FF",
       route = "L2", hazards = "entry-protection-read",
       recs = "grove-walk", note = "the protection reads the entry bytes at +2 (code-data-reads 0x1562826)")]
pub fn plane_trail_register_point(this: GPtr<u8>, x: u32, y: u32, z: u32) {
    let times = this.byte_add(0xC0).cast::<u32>();
    let t0 = times.read();
    let mut shifted = false;
    if t0 != 0 && g::<u32>(0xB7CB84).read().wrapping_sub(t0) > 2000 {
        shifted = true;
        let pts = this.cast::<[u32; 3]>();
        let mut k = 15;
        while k >= 1 {
            pts.wrapping_index(k).write(pts.wrapping_index(k - 1).read());
            times.wrapping_index(k).write(times.wrapping_index(k - 1).read());
            k -= 1;
        }
    }
    let p = this.cast::<u32>();
    p.write(x);
    p.wrapping_index(1).write(y);
    p.wrapping_index(2).write(z);
    if shifted || times.read() == 0 {
        times.write(g::<u32>(0xB7CB84).read());
    }
}

/// One plane trail of `CPlaneTrails::Update`: the point `(trig1(t) * k1, trig2(t) * k2, z)` with `t =
/// (now & 0x1FFFF) * (pi/65536)` (FSIN/FCOS of the x87 product, results rounded to f32).
fn trail_point(cos_first: bool, k1: u32, k2: u32, z: u32) -> [u32; 3] {
    let now = g::<u32>(0xB7CB84).read();
    let t = X87::from_i32((now & 0x1FFFF) as i32).fmul_m(f(0x3849_0FDB));
    let (a, b) = if cos_first { (t.fcos(), t.fsin()) } else { (t.fsin(), t.fcos()) };
    [a.fmul_m(f(k1)).to_f32().to_bits(), b.fmul_m(f(k2)).to_f32().to_bits(), z]
}

/// `CPlaneTrails::Update()` (0x7174F0): three trails (0xC713A8, 0xC714A8, 0xC715A8) get a new point
/// (`(sin t*2590, cos t*2200, 550)`, `(sin t*2000, cos t*-2600, 500)`, `(cos t*2100, sin t*1300, 600)`).
/// Outside the hours 7..=22, 7..=21 and 6..=20 respectively the trail's corona (ids 0x65/0x66/0x67;
/// red, yellow-white (255,255,128), white) is registered (`RegisterCorona` type 1, size 5, range 2000,
/// pull 1.5, fade speed 15) when bit 9 of `now` (+0, +0x15E, +0xC8) is set, else only its coordinates
/// are updated (`UpdateCoronaCoors(id, pos, 2000, 0)`).
#[hook(addr = 0x7174F0, conv = cdecl, name = "CPlaneTrails::Update", expect = "83 EC 1C 53 56 8B 35 84",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSIN/FCOS of the clock")]
#[allow(clippy::type_complexity)]
pub fn plane_trails_update() {
    let trails: [(u32, bool, u32, u32, u32, u8, u8, u32, u32, [u32; 3]); 3] = [
        (0xC713A8, false, 0x4521_E000, 0x4509_8000, 0x4409_8000, 22, 7, 0, 0x65, [0xFF, 0, 0]),
        (0xC714A8, false, 0x44FA_0000, 0xC522_8000, 0x43FA_0000, 21, 7, 0x15E, 0x66, [0xFF, 0xFF, 0x80]),
        (0xC715A8, true, 0x4503_4000, 0x44A2_8000, 0x4416_0000, 20, 6, 0xC8, 0x67, [0xFF, 0xFF, 0xFF]),
    ];
    for (this, cos_first, k1, k2, z, hi, lo, add, id, rgb) in trails {
        let mut pos = trail_point(cos_first, k1, k2, z);
        plane_trail_register_point_o(g(this), pos[0], pos[1], pos[2]);
        let hours = g::<u8>(0xB70153).read();
        if hours > hi || hours < lo {
            let p = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
            if g::<u32>(0xB7CB84).read().wrapping_add(add) & 0x200 != 0 {
                register_corona_by_type_o(
                    id, GPtr::NULL, rgb[0], rgb[1], rgb[2], 0xFF, p, 0x40A0_0000, 0x44FA_0000, 1, 0, 0, 0, 0, 0, 0,
                    0x3FC0_0000, 0, 0x4170_0000, 0, 0,
                );
            } else {
                update_corona_coors_o(id, p, 0x44FA_0000, 0);
            }
        }
    }
}

// ------------------------------------------------------------------ CCoronas (coords) -------------

/// `CCoronas::UpdateCoronaCoors(uint32 id, const CVector& pos, float range, float normalAngle)`
/// (0x6FC4D0; the camera matrix pointer is read through the protection chunk 0x4038C2, which decodes
/// `TheCamera.m_pMat` 0xB6F03C): nothing when `range^2 <` the 2D squared camera distance (x87,
/// ordered); else the corona with this id (if any) gets `m_vPosn = pos` and `m_fNormalAngle`.
#[hook(addr = 0x6FC4D0, conv = cdecl, name = "CCoronas::UpdateCoronaCoors", expect = "E9 ED 73 D0 FF 85 C0",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn update_corona_coors(id: u32, pos: GPtr<u8>, range: u32, normal_angle: u32) {
    let range = f(range);
    let m = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if m.is_null() { g::<f32>(0xB6F02C) } else { m.byte_add(0x30).cast::<f32>() };
    let p = pos.cast::<f32>();
    let cy = || X87::from_f32(cam.wrapping_index(1).read());
    let cx = || X87::from_f32(cam.read());
    let dy2 = cy().fsub_m(p.wrapping_index(1).read()).fmul(cy().fsub_m(p.wrapping_index(1).read()));
    let dx2 = cx().fsub_m(p.read()).fmul(cx().fsub_m(p.read()));
    let d2 = dy2.fadd(dx2);
    if !X87::from_f32(range).fmul_m(range).fcom(d2).test_parity(0x05) {
        return;
    }
    let mut i = 0u32;
    while i < 0x40 && g::<u32>(CORONAS + i * 0x3C + 0xC).read() != id {
        i += 1;
    }
    if i == 0x40 {
        return;
    }
    let c = CORONAS + i * 0x3C;
    let v = pos.cast::<u32>();
    g::<u32>(c).write(v.read());
    g::<u32>(c + 4).write(v.wrapping_index(1).read());
    g::<u32>(c + 8).write(v.wrapping_index(2).read());
    g::<u32>(c + 0x18).write(normal_angle);
}

// ------------------------------------------------------------------ CRopes ------------------------

/// `CRopes::FindRope(uint32 id)` (0x556000; the array base load sits in the chunk 0x40602F): the index of
/// the first of the 8 ropes (0x328 bytes from 0xB76BB8) with `m_nType` (+0x25) set and `m_nId` (+0) ==
/// id, else -1.
#[hook(addr = 0x556000, conv = cdecl, name = "CRopes::FindRope", expect = "8B 54 24 04 33 C0",
       route = "L2", hazards = "none")]
pub fn ropes_find_rope(id: u32) -> i32 {
    let mut i = 0;
    let mut r = 0xB76BB8u32;
    while r < 0xB784F8 {
        if g::<u8>(r + 0x25).read() != 0 && g::<u32>(r).read() == id {
            return i;
        }
        r += 0x328;
        i += 1;
    }
    -1
}

// ------------------------------------------------------------------ CWeather (tunnels) ------------

/// `CWeather::UpdateInTunnelness()` (0x72B630): target 0 unless the camera cull-zone flags (0xC87ABC)
/// have bit 0x2000. In a tunnel: target 1.0 and distance 100; the line from the camera position `P`
/// (z = 0) to `P + normalise(forward with z = 0) * 100` (forward = the matrix's +0x10 row, or
/// `(-sin h, cos h)` of the placement heading 0xB6F038 via FSIN/FCOS) is measured to the two fixed points
/// (85, -1020, 0) and (1683, -1956, 0) (set once, flags 0xC8143C bits 0/1) with
/// `CCollision::DistToLine`; the distance takes each result not above it (ordered); target =
/// `min(1.0, dist * 0.01)` (1.0 kept when `1.0 < v`). `CWeather::InTunnelness` (0xC81334) moves towards
/// the target by `timeStep * 0.01` (f32), snapping when `|target - cur| < step`.
#[hook(addr = 0x72B630, conv = cdecl, name = "CWeather::UpdateInTunnelness", expect = "83 EC 20 A1 BC 7A C8 00",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSIN/FCOS only without a camera matrix")]
pub fn update_in_tunnelness() {
    let mut target = 0.0f32;
    if g::<u32>(0xC87ABC).read() & 0x2000 != 0 {
        target = 1.0;
        let mut dist = f(0x42C8_0000);
        let m = g::<GPtr<u8>>(0xB6F03C).read();
        let cam = if m.is_null() { g::<[f32; 3]>(0xB6F02C) } else { m.byte_add(0x30).cast::<[f32; 3]>() };
        let mut p = cam.read();
        p[2] = 0.0;
        let mut fw = if !m.is_null() {
            m.byte_add(0x10).cast::<[f32; 3]>().read()
        } else {
            let h = X87::from_f32(g::<f32>(0xB6F038).read());
            [h.fsin().fchs().to_f32(), h.fcos().to_f32(), 0.0]
        };
        fw[2] = 0.0;
        vector_normalise(GPtr::from_raw(fw.as_mut_ptr()).cast());
        let k100 = f(0x42C8_0000);
        let zk = X87::from_f32(fw[2]).fmul_m(k100).to_f32();
        let mut q = [
            X87::from_f32(fw[0]).fmul_m(k100).fadd_m(p[0]).to_f32(),
            X87::from_f32(fw[1]).fmul_m(k100).fadd_m(p[1]).to_f32(),
            X87::from_f32(zk).fadd_m(p[2]).to_f32(),
        ];
        let flags = g::<u32>(0xC8143C);
        let fl = flags.read();
        if fl & 1 == 0 {
            flags.write(fl | 1);
            g::<[u32; 3]>(0xC81430).write([0x42AA_0000, 0xC47F_0000, 0]);
        }
        let fl = flags.read();
        if fl & 2 == 0 {
            flags.write(fl | 2);
            g::<[u32; 3]>(0xC81424).write([0x44D2_6000, 0xC4F4_8000, 0]);
        }
        let pp = GPtr::from_raw(p.as_mut_ptr()).cast::<u8>();
        let qq = GPtr::from_raw(q.as_mut_ptr()).cast::<u8>();
        let d1 = collision_dist_to_line(pp, qq, g(0xC81430));
        if X87::from_f32(k100).fcom(X87::from_f32(d1)).test_parity(0x05) {
            dist = d1;
        }
        let d2 = collision_dist_to_line(pp, qq, g(0xC81424));
        if X87::from_f32(dist).fcom(X87::from_f32(d2)).test_parity(0x05) {
            dist = d2;
        }
        let v = X87::from_f32(dist).fmul_m(f(0x3C23_D70A));
        if X87::one().fcom(v).test_parity(0x05) {
            target = v.to_f32();
        }
    }
    let cur = g::<f32>(0xC81334);
    let diff = X87::from_f32(target).fsub_m(cur.read());
    let step = X87::from_f32(g::<f32>(0xB7CB5C).read()).fmul_m(f(0x3C23_D70A)).to_f32();
    if !diff.fabs().fcom(X87::from_f32(step)).test_parity(0x05) {
        cur.write(target);
    } else if !diff.fcom(X87::from_f32(0.0)).test_parity(0x05) {
        cur.write(X87::from_f32(cur.read()).fsub_m(step).to_f32());
    } else {
        cur.write(X87::from_f32(cur.read()).fadd_m(step).to_f32());
    }
}

// ------------------------------------------------------------------ CCoronas::Update -------------

orig! {
    /// `CCamera::GetLookDirection()` (0x50AE90)
    fn camera_get_look_direction(this: GPtr<u8>) -> i32 = thiscall 0x50AE90 => unported;
    /// `CRegisteredCorona::Update()` (0x6FABF0)
    fn registered_corona_update_o(this: GPtr<u8>) = thiscall 0x6FABF0 => crate::mvpb_r1::registered_corona_update;
    /// `CRope::Update()` (0x557530)
    fn rope_update(this: GPtr<u8>) = thiscall 0x557530 => unported;
    /// `CGrassRenderer::SetCurrentScanCode(uint16)` (0x5DABB0; pushed as a dword)
    fn grass_set_current_scan_code_o(code: u32) = cdecl 0x5DABB0 => crate::mvpb_r1::grass_set_current_scan_code;
    /// `CGrassRenderer::SetGlobalCameraPos(const CVector&)` (0x5DABC0)
    fn grass_set_global_camera_pos(pos: GPtr<u8>) = cdecl 0x5DABC0 => unported;
    /// `CPlantMgr::UpdateAmbientColor()` (0x5DB310)
    fn plant_update_ambient_color_o() = cdecl 0x5DB310 => crate::mvpb_r1::plant_update_ambient_color;
    /// `CPlantMgr::CalculateWindBending()` (0x5DB3D0, float in ST0)
    fn plant_calculate_wind_bending_o() -> f32 = cdecl 0x5DB3D0 => crate::mvpb_r1::plant_calculate_wind_bending_f32;
    /// `CGrassRenderer::SetGlobalWindBending(float)` (0x5DAC00)
    fn grass_set_global_wind_bending(v: u32) = cdecl 0x5DAC00 => unported;
    /// `CPlantMgr::_ColEntityCache_Update(const CVector&, bool quick)` (0x5DC510)
    fn plant_col_entity_cache_update(pos: GPtr<u8>, quick: u32) = cdecl 0x5DC510 => unported;
    /// `CPlantMgr::_ProcessEntryCollisionDataSections(CPlantColEntEntry*, const CVector&, int)` (0x5DCD80)
    fn plant_process_entry_collision_data_sections(e: GPtr<u8>, pos: GPtr<u8>, n: u32) = cdecl 0x5DCD80 => unported;
    /// `CTimeCycle::GetAmbientRed/Green/Blue()` (floats in ST0)
    fn time_cycle_get_ambient_red() -> f32 = cdecl 0x560330 => unported;
    fn time_cycle_get_ambient_green() -> f32 = cdecl 0x560340 => unported;
    fn time_cycle_get_ambient_blue() -> f32 = cdecl 0x560350 => unported;
}

/// `CCoronas::Update()` (0x6FADF0): `LightsMult` (0x8D4B5C) = `timeStep * 0.03 + LightsMult`, capped at
/// 1.0 (x87, `1.0 < v` → 1.0). The camera state bits (active cam bytes +0x17C/+0x17D/+0x17B → 1/2/4,
/// `GetLookDirection() == 0` → 8) compared with `LastCamLook` (0xC3EF58): changed → `bChangeBrightnessImmediately`
/// (0xC3E034) = 3, else `max(n - 1, 0)`; then `LastCamLook` = bits and every corona with an id gets
/// `CRegisteredCorona::Update()`.
#[hook(addr = 0x6FADF0, conv = cdecl, name = "CCoronas::Update", expect = "D9 05 5C CB B7 00",
       unverified = "mvpb-render-1", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "every corona's Update is a nested call")]
pub fn coronas_update() {
    let lm = g::<f32>(0x8D4B5C);
    let v = X87::from_f32(g::<f32>(0xB7CB5C).read()).fmul_m(f(0x3CF5_C28F)).fadd_m(lm.read());
    if X87::one().fcom(v).test_parity(0x05) {
        lm.write(v.to_f32());
    } else {
        lm.write(1.0);
    }
    let cam = 0xB6F028 + g::<u8>(0xB6F081).read() as u32 * 0x238;
    let mut bits = 0u32;
    if g::<u8>(cam + 0x17C).read() != 0 {
        bits = 1;
    }
    if g::<u8>(cam + 0x17D).read() != 0 {
        bits |= 2;
    }
    if g::<u8>(cam + 0x17B).read() != 0 {
        bits |= 4;
    }
    if camera_get_look_direction(g(0xB6F028)) == 0 {
        bits |= 8;
    }
    let imm = g::<i32>(0xC3E034);
    if bits != g::<u32>(0xC3EF58).read() {
        imm.write(3);
    } else {
        let n = imm.read().wrapping_sub(1);
        imm.write(if n < 0 { 0 } else { n });
    }
    g::<u32>(0xC3EF58).write(bits);
    let mut c = CORONAS;
    while c < 0xC3EF58 {
        if g::<u32>(c + 0xC).read() != 0 {
            registered_corona_update_o(g(c));
        }
        c += 0x3C;
    }
}

/// `CRopes::Update()` (0x558D70): unless replaying, `CRope::Update()` of each of the 8 ropes (0x328 bytes
/// from 0xB768B8) whose type byte (+0x325) is set.
#[hook(addr = 0x558D70, conv = cdecl, name = "CRopes::Update", expect = "80 3D 88 30 A4 00 01",
       unverified = "mvpb-render-1", route = "L3-only", hazards = "none")]
pub fn ropes_update() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    let mut r = 0xB768B8u32;
    while r < 0xB781F8 {
        if g::<u8>(r + 0x325).read() != 0 {
            rope_update(g(r));
        }
        r += 0x328;
    }
}

/// `CGrassRenderer::SetCurrentScanCode(uint16)` (0x5DABB0): stores the code at 0x8D1330.
#[hook(addr = 0x5DABB0, conv = cdecl, name = "CGrassRenderer::SetCurrentScanCode", expect = "66 8B 44 24 04 66 A3 30",
       unverified = "mvpb-render-1", route = "L2", hazards = "none")]
pub fn grass_set_current_scan_code(code: u32) {
    g::<u16>(0x8D1330).write(code as u16);
}

/// `CPlantMgr::Update(const CVector& camPos)` (0x5DCFA0, bool in AL, always true): scan code (u16
/// 0xC03980) + 1 → `SetCurrentScanCode`, `SetGlobalCameraPos(camPos)`, `UpdateAmbientColor`,
/// `SetGlobalWindBending(CalculateWindBending())`; counter 0xC09171 + 1 → `_ColEntityCache_Update(camPos,
/// (counter & 31) != 0)`; counter 0xC09170 (its old value & 7 = the section) + 1, and every entry of the
/// close-entity list (0xC0399C, `next` +0xC) gets `_ProcessEntryCollisionDataSections(entry, camPos,
/// section)`.
#[hook(addr = 0x5DCFA0, conv = cdecl, name = "CPlantMgr::Update", expect = "51 33 C0 66 A1 80 39 C0 00",
       unverified = "mvpb-render-1", route = "L3-only", hazards = "none", recs = "grove-walk")]
pub fn plant_mgr_update(cam: GPtr<u8>) -> bool {
    let sc = g::<u16>(0xC03980);
    let v = sc.read().wrapping_add(1);
    sc.write(v);
    grass_set_current_scan_code_o(v as u32);
    grass_set_global_camera_pos(cam);
    plant_update_ambient_color_o();
    let w = plant_calculate_wind_bending_o();
    grass_set_global_wind_bending(w.to_bits());
    let c1 = g::<u8>(0xC09171);
    let n = c1.read().wrapping_add(1);
    c1.write(n);
    plant_col_entity_cache_update(cam, u32::from(n & 0x1F != 0));
    let c0 = g::<u8>(0xC09170);
    let old = c0.read();
    let section = (old & 7) as u32;
    c0.write(old.wrapping_add(1));
    let mut e = g::<GPtr<u8>>(0xC0399C).read();
    while !e.is_null() {
        plant_process_entry_collision_data_sections(e, cam, section);
        e = e.byte_add(0xC).cast::<GPtr<u8>>().read();
    }
    true
}

/// `CPlantMgr::UpdateAmbientColor()` (0x5DB310): each ambient channel `c` of the time cycle gives
/// `0x40 - ftol(c * 2.5 * -255)` (16-bit compare against 0xFF: above → 0xFF), stored as bytes at
/// 0xC03A44/45/46.
#[hook(addr = 0x5DB310, conv = cdecl, name = "CPlantMgr::UpdateAmbientColor", expect = "83 EC 10 53 E8 17 50 F8 FF",
       unverified = "mvpb-render-1", route = "L2", hazards = "none")]
pub fn plant_update_ambient_color() {
    let r = time_cycle_get_ambient_red();
    let gg = time_cycle_get_ambient_green();
    let b = time_cycle_get_ambient_blue();
    let k1 = f(0x4020_0000);
    let k2 = f(0xC37F_0000);
    let ch = |c: f32| -> u32 {
        let v = X87::from_f32(X87::from_f32(c).fmul_m(k1).to_f32()).fmul_m(k2).ftol() as u32;
        let n = 0x40u32.wrapping_sub(v);
        if n as u16 > 0xFF { 0xFF } else { n }
    };
    let (cr, cg, cb) = (ch(r), ch(gg), ch(b));
    g::<u8>(0xC03A44).write(cr as u8);
    g::<u8>(0xC03A45).write(cg as u8);
    g::<u8>(0xC03A46).write(cb as u8);
}

// ------------------------------------------------------------------ CPlantMgr (wind, entries) -----

orig! {
    /// CRT `rand()` (0x821B1E)
    fn crt_rand() -> i32 = cdecl 0x821B1E => unported;
    /// `CPlantLocTri::Release()` (0x5DB6D0)
    fn plant_loc_tri_release_o(this: GPtr<u8>) = thiscall 0x5DB6D0 => crate::mvpb_r1::plant_loc_tri_release;
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    fn memory_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `CMemoryMgr::Malloc(uint32 size, uint32 hint)` (0x72F420)
    fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(this: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `ProcObjectMan_c::ProcessTriangleRemoved(CPlantLocTri*)` (0x5A3F70)
    fn proc_obj_man_process_triangle_removed(this: GPtr<u8>, tri: GPtr<u8>) = thiscall 0x5A3F70 => unported;
    /// `CPlantSurfPropMgr::GetSurfProperties(uint16)` (0x6F9DE0; the id pushed from `movzx dx, byte`)
    fn plant_surf_get_surf_properties(id: u32) -> GPtr<u16> = cdecl 0x6F9DE0 => unported;
}

/// `CPlantMgr::CalculateWindBending()` (0x5DB3D0, float in ST0): the first call stores a `rand()`
/// (0xC09168, flag bit 0 of 0xC0916C). With `CWeather::Wind` (0xC812F0) `< 0.2`: `sin((now & 0xFFF) *
/// 0.0015332) * 0.005`; `< 0.5`: the same times 0.008 (FSIN of the x87 product); otherwise the gust
/// curve: `i = (rand + now * 8) & 0xFFFF`, `t = (i & 0xFFF) / 4096`, `n = i >> 12`, `((1 - t) * T[n] +
/// t * T[(n + 1) & 15] + 1) * Wind * 0.015` with `T` the 16 floats at 0x8CCF30.
#[hook(addr = 0x5DB3D0, conv = cdecl, name = "CPlantMgr::CalculateWindBending", expect = "51 8A 0D 6C 91 C0 00",
       unverified = "mvpb-render-1", route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "rand() at site 0x5DB3EB on the first call only; ST0 result")]
pub fn plant_calculate_wind_bending() -> X87 {
    wind_bending()
}

/// [`plant_calculate_wind_bending`] rounded to f32 (the `orig!` route of phases B/C).
pub fn plant_calculate_wind_bending_f32() -> f32 {
    wind_bending().to_f32()
}

fn wind_bending() -> X87 {
    let fl = g::<u32>(0xC0916C);
    if fl.read() as u8 & 1 == 0 {
        fl.write(fl.read() | 1);
        let r = sa_core::orig::at_site(0x5DB3EB, crt_rand);
        g::<i32>(0xC09168).write(r);
    }
    let wind = g::<f32>(0xC812F0).read();
    let now = g::<u32>(0xB7CB84).read();
    let w = X87::from_f32(wind);
    if !w.fcom(X87::from_f32(f(0x3F00_0000))).test_parity(0x05) {
        let k = if !w.fcom(X87::from_f32(f(0x3E4C_CCCD))).test_parity(0x05) { 0x3BA3_D70A } else { 0x3C03_126F };
        return X87::from_i32((now & 0xFFF) as i32).fmul_m(f(0x3AC8_F5C3)).fsin().fmul_m(f(k));
    }
    let i = (g::<u32>(0xC09168).read().wrapping_add(now.wrapping_mul(8))) & 0xFFFF;
    let n = (i >> 12) & 0xF;
    let t = X87::from_i32((i & 0xFFF) as i32).fmul_m(f(0x3980_0000));
    let a = X87::one().fsub(t).fmul_m(g::<f32>(0x8CCF30 + 4 * n).read());
    let b = t.fmul_m(g::<f32>(0x8CCF30 + 4 * ((n + 1) & 0xF)).read());
    a.fadd(b).fadd_m(1.0).fmul_m(wind).fmul_m(f(0x3C75_C28F))
}

/// Unlinks `node` from the doubly linked list whose head pointer is at `head` (`next` at `nx`, `prev`
/// at `pv`), in the original's store order.
fn list_remove(node: GPtr<u8>, head: GPtr<GPtr<u8>>, nx: isize, pv: isize) {
    let p = |n: GPtr<u8>, o: isize| n.byte_add(o).cast::<GPtr<u8>>();
    let prev = p(node, pv).read();
    if prev.is_null() {
        let next = p(node, nx).read();
        head.write(next);
        if !next.is_null() {
            p(next, pv).write(GPtr::NULL);
        }
    } else {
        let next = p(node, nx).read();
        if next.is_null() {
            p(prev, nx).write(GPtr::NULL);
        } else {
            p(next, pv).write(prev);
            let prev = p(node, pv).read();
            let next = p(node, nx).read();
            p(prev, nx).write(next);
        }
    }
}

/// Inserts `node` at the head of the list at `head`.
fn list_push(node: GPtr<u8>, head: GPtr<GPtr<u8>>, nx: isize, pv: isize) {
    let p = |n: GPtr<u8>, o: isize| n.byte_add(o).cast::<GPtr<u8>>();
    p(node, nx).write(head.read());
    p(node, pv).write(GPtr::NULL);
    head.write(node);
    let next = p(node, nx).read();
    if !next.is_null() {
        p(next, pv).write(node);
    }
}

/// `CPlantColEntEntry::ReleaseEntry()` (0x5DB8A0): releases every non-null loc-tri of `m_Objects` (+4,
/// `m_numTriangles` u16 +8, re-read each step) and frees the array (`m_Objects = 0`, count 0); drops the
/// entity reference (`CleanUpOldReference(&m_Entity)`, `m_Entity = 0`); moves the entry from the close
/// list (0xC0399C) to the free list (0xC03998) (`next` +0xC, `prev` +0x10).
#[hook(addr = 0x5DB8A0, conv = thiscall, name = "CPlantColEntEntry::ReleaseEntry", this = "CPlantColEntEntry", expect = "53 56 8B F1 8B 46 04",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "writes the entry pool (0xC03A50 area) and frees heap; head store via protection chunk 0x40A085")]
pub fn plant_col_ent_entry_release_entry(this: GPtr<u8>) {
    let objs = this.byte_add(4).cast::<GPtr<GPtr<u8>>>();
    if !objs.read().is_null() {
        let mut i = 0i32;
        while i < this.byte_add(8).cast::<u16>().read() as i32 {
            let t = objs.read().wrapping_index(i).read();
            if !t.is_null() {
                plant_loc_tri_release_o(t);
            }
            i += 1;
        }
        memory_free(objs.read().cast());
        objs.write(GPtr::NULL);
        this.byte_add(8).cast::<u16>().write(0);
    }
    let e = this.cast::<GPtr<u8>>().read();
    if !e.is_null() {
        entity_clean_up_old_reference(e, this);
        this.cast::<u32>().write(0);
    }
    list_remove(this, g(0xC0399C), 0xC, 0x10);
    list_push(this, g(0xC03998), 0xC, 0x10);
}

/// `CPlantLocTri::Release()` (0x5DB6D0): `m_V1`.. cleared (+0x40 dword, +0x44 word); with flag 4 (+0x48)
/// `g_procObjMan.ProcessTriangleRemoved(this)`. A tri with flag 2 and without flag 1 leaves the list at
/// 0xC03994, any other the per-surface list `0xC03988[GetSurfProperties(surface +0x46)->m_SlotId]`
/// (`next` +0x4C, `prev` +0x50); both go to the free list 0xC03984; flags &= 0xF8 and the surface byte
/// becomes 0xFE (first case) or 0xFF.
#[hook(addr = 0x5DB6D0, conv = thiscall, name = "CPlantLocTri::Release", this = "CPlantLocTri", expect = "33 C0 56 8B F1 89 46 40",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "writes the loc-tri pool (static array) and its list links")]
pub fn plant_loc_tri_release(this: GPtr<u8>) {
    this.byte_add(0x40).cast::<u32>().write(0);
    this.byte_add(0x44).cast::<u16>().write(0);
    if this.byte_add(0x48).read() & 4 != 0 {
        proc_obj_man_process_triangle_removed(g(0xBB7CB0), this);
    }
    let fl = this.byte_add(0x48).read();
    let surf_byte = if fl & 2 != 0 && fl & 1 == 0 {
        list_remove(this, g(0xC03994), 0x4C, 0x50);
        0xFE
    } else {
        let slot = plant_surf_get_surf_properties(this.byte_add(0x46).read() as u32).read() as u32;
        list_remove(this, g(0xC03988 + slot * 4), 0x4C, 0x50);
        0xFF
    };
    list_push(this, g(0xC03984), 0x4C, 0x50);
    let al = this.byte_add(0x48).read() & 0xF8;
    this.byte_add(0x46).write(surf_byte);
    this.byte_add(0x48).write(al);
}

/// `CPlantColEntEntry::AddEntry(CEntity*)` (0x5DB7D0, thiscall, `ret 4`): `m_Entity = entity` with
/// `RegisterReference`; NULL when the entity's col model has no data or `m_nNumTriangles <= 0`; else
/// `m_numTriangles` (u16 +8) = n, `m_Objects` = `CMemoryMgr::Malloc(n * 4, 0)` zero-filled, and the entry
/// moves from the free list (0xC03998) to the close list (0xC0399C); returns `this`.
#[hook(addr = 0x5DB7D0, conv = thiscall, name = "CPlantColEntEntry::AddEntry", this = "CPlantColEntEntry", expect = "56 57 8B 7C 24 0C",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "allocates and fills the tri array (heap)")]
pub fn plant_col_ent_entry_add_entry(this: GPtr<u8>, entity: GPtr<u8>) -> GPtr<u8> {
    this.cast::<GPtr<u8>>().write(entity);
    entity_register_reference(entity, this);
    let data = entity_get_col_model(entity).byte_add(0x2C).cast::<GPtr<u8>>().read();
    if data.is_null() {
        return GPtr::NULL;
    }
    let n = data.byte_add(4).cast::<i16>().read() as i32;
    if n <= 0 {
        return GPtr::NULL;
    }
    this.byte_add(8).cast::<u16>().write(n as u16);
    let arr = memory_malloc((n * 4) as u32, 0);
    this.byte_add(4).cast::<GPtr<u8>>().write(arr);
    let mut i = 0;
    while i < n {
        this.byte_add(4).cast::<GPtr<u32>>().read().wrapping_index(i).write(0);
        i += 1;
    }
    list_remove(this, g(0xC03998), 0xC, 0x10);
    list_push(this, g(0xC0399C), 0xC, 0x10);
    this
}

// ------------------------------------------------------------------ plant geometry helpers --------

orig! {
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    fn vector_cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CMatrix::CopyToRwMatrix(RwMatrix*) const` (0x59B8B0)
    fn matrix_copy_to_rw_matrix(this: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59B8B0 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwV3dTransformPoint(RwV3d* out, const RwV3d* in, const RwMatrix*)` (0x7EDD60)
    fn rw_v3d_transform_point(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD60 => unported;
    /// `RwV3dTransformVector(RwV3d* out, const RwV3d* in, const RwMatrix*)` (0x7EDDC0)
    fn rw_v3d_transform_vector(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDDC0 => unported;
}

/// `CPlantLocTri::GetPlantDensity()` (0x5DC210; `this` in EAX, the 80-bit result in ST0): half the
/// length of `cross(V2 - V1, V3 - V1)` (differences rounded to f32, `|c| = sqrt(z*z + y*y + x*x)` via
/// FSQRT), returned unrounded.
#[hook(addr = 0x5DC210, conv = reg("args = eax"), name = "CPlantLocTri::GetPlantDensity", expect = "83 EC 24 D9 40 0C",
       unverified = "regconv-render-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk,driving",
       note = "this in EAX (LTCG), 80-bit ST0 result (HARNESS-5 reg conv; was entry_eax in mvpb-render-1)")]
pub fn plant_loc_tri_get_plant_density(this: GPtr<u8>) -> X87 {
    let t = this.cast::<f32>();
    let v = |i: i32| X87::from_f32(t.wrapping_index(i).read());
    let mut a = [
        v(3).fsub_m(t.read()).to_f32(),
        v(4).fsub_m(t.wrapping_index(1).read()).to_f32(),
        v(5).fsub_m(t.wrapping_index(2).read()).to_f32(),
    ];
    let mut b = [
        v(6).fsub_m(t.read()).to_f32(),
        v(7).fsub_m(t.wrapping_index(1).read()).to_f32(),
        v(8).fsub_m(t.wrapping_index(2).read()).to_f32(),
    ];
    let mut c = [0f32; 3];
    vector_cross_product(
        GPtr::from_raw(c.as_mut_ptr()).cast(),
        GPtr::from_raw(a.as_mut_ptr()).cast(),
        GPtr::from_raw(b.as_mut_ptr()).cast(),
    );
    let sq = |x: f32| X87::from_f32(x).fmul_m(x);
    sq(c[2]).fadd(sq(c[1])).fadd(sq(c[0])).fsqrt().fmul_m(f(0x3F00_0000))
}

/// `TransformPoints(RwV3d* out, int count, const CMatrix& m, const RwV3d* in)` (0x54EEF0, cdecl): `m` is
/// copied into the scratch RwMatrix at `*(0xB72920)` and updated; then `count` points (`dec; jne`, so a
/// count of 0 does nothing) go through `RwV3dTransformPoint(out++, in++, scratch)`.
#[hook(addr = 0x54EEF0, conv = cdecl, name = "TransformPoints", expect = "A1 20 29 B7 00 8B 4C 24",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "writes the scratch RwMatrix (heap pointer at 0xB72920) and the caller's output array")]
pub fn transform_points(out: GPtr<u8>, count: u32, m: GPtr<u8>, inp: GPtr<u8>) {
    matrix_copy_to_rw_matrix(m, g::<GPtr<u8>>(0xB72920).read());
    rw_matrix_update(g::<GPtr<u8>>(0xB72920).read());
    let mut n = count;
    let (mut o, mut i) = (out, inp);
    while n != 0 {
        rw_v3d_transform_point(o, i, g::<GPtr<u8>>(0xB72920).read());
        o = o.byte_add(0xC);
        i = i.byte_add(0xC);
        n = n.wrapping_sub(1);
    }
}

/// `CGrassRenderer::GenPointInTriangle(CVector* out, const CVector* v0, const CVector* v1, const
/// CVector* v2, float r1, float r2)` (0x5DAB00, cdecl): when `r1 + r2 > 1` both become `1 - r`; with
/// `c = 1 - r1 - r2`, `out = c*v0 + r1*v1 + r2*v2` per component (x87, each sum rounded to f32).
#[hook(addr = 0x5DAB00, conv = cdecl, name = "CGrassRenderer::GenPointInTriangle", expect = "D9 44 24 18 D9 44 24 14",
       unverified = "mvpb-render-1", route = "L2", hazards = "none",
       note = "pure function of its pointed-to inputs (no L1 spec yet)")]
pub fn grass_gen_point_in_triangle(out: GPtr<f32>, v0: GPtr<f32>, v1: GPtr<f32>, v2: GPtr<f32>, r1: f32, r2: f32) {
    let one = X87::from_f32(1.0);
    let (mut a, mut b) = (X87::from_f32(r1), X87::from_f32(r2));
    if !a.fadd(b).fcom(one).test_any(0x41) {
        a = one.fsub(a);
        b = one.fsub(b);
    }
    let c = one.fsub(a).fsub(b);
    let mut i = 0;
    while i < 3 {
        let s = c
            .fmul_m(v0.wrapping_index(i).read())
            .fadd(a.fmul_m(v1.wrapping_index(i).read()))
            .fadd(b.fmul_m(v2.wrapping_index(i).read()));
        out.wrapping_index(i).write(s.to_f32());
        i += 1;
    }
}

// ------------------------------------------------------------------ FxSystem_c / CRope -----------

orig! {
    /// `Fx_c::GetFxQuality()` (0x49EA50)
    fn fx_get_fx_quality(this: GPtr<u8>) -> i32 = thiscall 0x49EA50 => unported;
    /// `FxPrim_c::AddParticle(pos, vel, timeSince, mult, zRot, brightness, createLocal)` (vtable slot 0x14)
    #[allow(clippy::too_many_arguments)]
    fn fx_prim_add_particle(prim: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, zrot: u32, bright: u32, local: u32) = thiscall vslot 0x14 => unported;
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CObject::CObject(int modelId, bool create)` (0x5A1D70)
    fn object_ctor(this: GPtr<u8>, model: i32, create: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10)
    fn entity_set_is_static(this: GPtr<u8>, v: u32) = thiscall vslot 0x10 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
}

/// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float timeSince, FxPrtMult_c* mult, float zRot,
/// float lightMult, float lightMultLimit, uint8 createLocal)` (0x4AA440, thiscall, `ret 0x20`): a
/// `rand()` (site 0x4AA450) gives `p = ftol((rand & 0xFFFF) * 2^-15 * 100)`; nothing when the FX quality
/// is 0 and `p < 50`, or 1 and `p < 25`. Brightness 1.0, or `(1 - limit) + lightMult` when `lightMult <
/// limit`; then for `i` below the base system's prim count (`m_pBaseSystem` +8, count +0x1B signed) every
/// enabled (+0xC) prim of this system's own array (`this` +0x78, not the base system's) gets
/// `AddParticle(pos, vel, timeSince, mult, zRot, brightness, createLocal)`. The original keeps its loop
/// counter in the `lightMultLimit` argument slot (stack only).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x4AA440, conv = thiscall, name = "FxSystem_c::AddParticle-v3d", expect = "83 EC 08 56 57 8B F9",
       route = "split", hazards = "heap-writes",
       recs = "grove-walk", note = "rand() at site 0x4AA450; particles go to the FX pools (heap)")]
pub fn fx_system_add_particle(this: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, zrot: u32, light_mult: f32, limit: f32, local: u32) {
    let r = sa_core::orig::at_site(0x4AA450, crt_rand) as u32 & 0xFFFF;
    let p = X87::from_i32(r as i32).fmul_m(f(0x3800_0000)).fmul_m(f(0x42C8_0000)).ftol() as i32;
    let fx = g::<u8>(0xA9AE00);
    if fx_get_fx_quality(fx) == 0 && p < 0x32 {
        return;
    }
    if fx_get_fx_quality(fx) == 1 && p < 0x19 {
        return;
    }
    let mut bright = 1.0f32;
    if !X87::from_f32(light_mult).fcom(X87::from_f32(limit)).test_parity(0x05) {
        bright = X87::one().fsub_m(limit).fadd_m(light_mult).to_f32();
    }
    let base = || this.byte_add(8).cast::<GPtr<u8>>().read();
    let mut i = 0i32;
    while i < base().byte_add(0x1B).cast::<i8>().read() as i32 {
        // the prim array is the system's own (`this` +0x78, 0x4AA4F0..F7), the count the base system's
        let prim = this.byte_add(0x78).cast::<GPtr<GPtr<u8>>>().read().wrapping_index(i).read();
        if prim.byte_add(0xC).read() != 0 {
            fx_prim_add_particle(prim, pos, vel, time, mult, zrot, bright.to_bits(), local);
        }
        i += 1;
    }
}

/// `CRope::CreateHookObjectForRope()` (0x556070): when no hook object exists (+0x314): by rope type
/// (+0x325; 1/4/6/7 → `MI_MAGNOCRANE_HOOK`-style id at 0x8CD700, 2 → 0x8CD6FC, 3 → 0x8CD740, 5 →
/// 0x8CD71C; other types do nothing) a `CObject(model, true)` is created, referenced from +0x314, placed
/// at the first segment position (+0x174), object type 5, `SetIsStatic(false)`, flag 0x2000000 in +0x40,
/// `CWorld::Add`; then +0x318 = 0 and +0x326 = 0.
#[hook(addr = 0x556070, conv = thiscall, name = "CRope::CreateHookObjectForRope", this = "CRope", expect = "6A FF 68 6B CC 83 00",
       unverified = "mvpb-render-1", route = "L2", hazards = "heap-writes",
       note = "creates a CObject (object pool, world lists); SEH frame not modelled")]
pub fn rope_create_hook_object(this: GPtr<u8>) {
    let slot = this.byte_add(0x314).cast::<GPtr<u8>>();
    if !slot.read().is_null() {
        return;
    }
    let mi = match this.byte_add(0x325).read() {
        1 | 4 | 6 | 7 => 0x8CD700,
        2 => 0x8CD6FC,
        3 => 0x8CD740,
        5 => 0x8CD71C,
        _ => return,
    };
    let model = g::<u16>(mi).read() as i32;
    let mem = object_new(0x17C);
    let obj = if mem.is_null() { GPtr::NULL } else { object_ctor(mem, model, 1) };
    slot.write(obj);
    entity_register_reference(obj, slot.cast());
    let o = slot.read();
    let p = |off: isize| X87::from_f32(this.byte_add(off).cast::<f32>().read()).to_f32();
    let (z, y, xx) = (p(0x17C), p(0x178), p(0x174));
    let m = o.byte_add(0x14).cast::<GPtr<u8>>().read();
    if !m.is_null() {
        m.byte_add(0x30).cast::<f32>().write(xx);
        o.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x34).cast::<f32>().write(y);
        o.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x38).cast::<f32>().write(z);
    } else {
        o.byte_add(4).cast::<f32>().write(xx);
        o.byte_add(8).cast::<f32>().write(y);
        o.byte_add(0xC).cast::<f32>().write(z);
    }
    slot.read().byte_add(0x13C).write(5);
    entity_set_is_static(slot.read(), 0);
    let fl = slot.read().byte_add(0x40).cast::<u32>();
    fl.write(fl.read() | 0x0200_0000);
    world_add(slot.read());
    this.byte_add(0x318).cast::<u32>().write(0);
    this.byte_add(0x326).write(0);
}

// ------------------------------------------------------------------ CRegisteredCorona ------------

orig! {
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, bool vehicles,
    /// bool peds, bool objects, bool dummies, bool seeThrough, bool cameraIgnore)` (0x56A490, bool in AL)
    #[allow(clippy::too_many_arguments)]
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32, p7: u32, p8: u32) -> u8 = cdecl 0x56A490 => unported;
}

/// The fade-out step: `ftol(max(faded - timeStep * fadeSpeed, 0))` (x87; `0 <= v` keeps v).
fn corona_fade_out(c: GPtr<u8>) {
    let v = X87::from_i32(c.byte_add(0x30).read() as i32)
        .fsub(X87::from_f32(g::<f32>(0xB7CB5C).read()).fmul_m(c.byte_add(0x28).cast::<f32>().read()));
    let zero = X87::from_f32(0.0);
    let v = if zero.fcom(v).test_any(0x41) { v } else { zero };
    c.byte_add(0x30).write(v.ftol() as u8);
}

/// `CRegisteredCorona::Update()` (0x6FABF0): intensity (+0x2F) 0 when not registered this frame (+0x31);
/// a corona attached (+0x36 bit 4) to a vanished entity (+0x38 NULL) is freed (id 0, `NumCoronas`--).
/// Fade-out (`faded - timeStep * fadeSpeed`, floored at 0) when: the LOS flag (+0x34 bit 0) is set and
/// either the sun (id 2) is blocked by clouds (0xC3E030) or `GetIsLineOfSightClear(pos, camPos, true,
/// false x6)` fails; or the off-screen flag (+0x34 bit 1) is set; or only-from-below (+0x36 bit 1) with
/// the camera above the corona. Otherwise `faded` (+0x30) moves towards the intensity by `timeStep *
/// fadeSpeed` (rounded to f32, clamped at the intensity; equal values change nothing), or jumps to it
/// when `bChangeBrightnessImmediately`; for the sun `LightsMult = max(LightsMult - timeStep * 0.06, 0.6)`.
/// Finally a corona faded to 0 that was not just created is freed; `justCreated` and `registered` are
/// cleared.
#[hook(addr = 0x6FABF0, conv = thiscall, name = "CRegisteredCorona::Update", this = "CRegisteredCorona", expect = "83 EC 10 56 8B F1",
       unverified = "mvpb-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn registered_corona_update(c: GPtr<u8>) {
    if c.byte_add(0x31).read() == 0 {
        c.byte_add(0x2F).write(0);
    }
    if c.byte_add(0x36).read() & 0x10 != 0 && c.byte_add(0x38).cast::<u32>().read() == 0 {
        c.byte_add(0xC).cast::<u32>().write(0);
        g::<u32>(NUM_CORONAS).write(g::<u32>(NUM_CORONAS).read().wrapping_sub(1));
        return;
    }
    let cam_pos = || {
        let m = g::<GPtr<u8>>(0xB6F03C).read();
        if m.is_null() { g::<f32>(0xB6F02C) } else { m.byte_add(0x30).cast::<f32>() }
    };
    let id = c.byte_add(0xC).cast::<u32>();
    let mut fade_out = false;
    if c.byte_add(0x34).read() & 1 != 0 {
        if g::<u8>(0xC3E030).read() != 0 && id.read() == 2 {
            fade_out = true;
        } else {
            let p = cam_pos();
            let mut cp = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
            if world_get_is_line_of_sight_clear(c, GPtr::from_raw(cp.as_mut_ptr()).cast(), 1, 0, 0, 0, 0, 0, 0) == 0 {
                fade_out = true;
            }
        }
    }
    if !fade_out && c.byte_add(0x34).read() & 2 != 0 {
        fade_out = true;
    }
    if !fade_out && c.byte_add(0x36).read() & 2 != 0 {
        let cz = X87::from_f32(cam_pos().wrapping_index(2).read());
        if !cz.fcom(X87::from_f32(c.byte_add(8).cast::<f32>().read())).test_any(0x41) {
            fade_out = true;
        }
    }
    if fade_out {
        corona_fade_out(c);
    } else {
        let faded = c.byte_add(0x30).read();
        let inten = c.byte_add(0x2F).read();
        let step = || X87::from_f32(g::<f32>(0xB7CB5C).read()).fmul_m(c.byte_add(0x28).cast::<f32>().read());
        if inten != faded {
            let bi = X87::from_i32(inten as i32);
            let v = if inten > faded {
                let t = step().fadd(X87::from_i32(faded as i32)).to_f32();
                if !bi.fcom(X87::from_f32(t)).test_parity(0x05) { bi } else { X87::from_f32(t) }
            } else {
                let t = X87::from_i32(faded as i32).fsub(step()).to_f32();
                if !bi.fcom(X87::from_f32(t)).test_any(0x41) { bi } else { X87::from_f32(t) }
            };
            c.byte_add(0x30).write(v.ftol() as u8);
            if g::<u32>(0xC3E034).read() != 0 {
                c.byte_add(0x30).write(inten);
            }
        }
        if id.read() == 2 {
            let lm = g::<f32>(0x8D4B5C);
            let v = X87::from_f32(g::<f32>(0xB7CB5C).read()).fmul_m(f(0x3D75_C28F)).fsubr_m(lm.read());
            if X87::from_f32(f(0x3F19_999A)).fcom(v).test_any(0x41) {
                lm.write(v.to_f32());
            } else {
                lm.write(f(0x3F19_999A));
            }
        }
    }
    if c.byte_add(0x30).read() == 0 && c.byte_add(0x35).read() == 0 {
        id.write(0);
        g::<u32>(NUM_CORONAS).write(g::<u32>(NUM_CORONAS).read().wrapping_sub(1));
    }
    c.byte_add(0x35).write(0);
    c.byte_add(0x31).write(0);
}

// ------------------------------------------------------------------ CCoronas::DoSunAndMoon --------

orig! {
    /// `CGame::CanSeeOutSideFromCurrArea()` (0x53C4A0, bool in AL)
    fn game_can_see_outside_from_curr_area() -> u8 = cdecl 0x53C4A0 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn sprite_calc_screen_coors(inp: GPtr<u8>, out: GPtr<u8>, w: GPtr<u8>, h: GPtr<u8>, a: u32, b: u32) -> u8 = cdecl 0x70CE30 => unported;
}

/// `CCoronas::DoSunAndMoon()` (0x6FC5A0): sun position = `m_VectorToSun[m_CurrentStoredValue]` (12 bytes
/// from 0xB7CA50) times `CCoronas::SunDistance`-style scale (`0xC3EF9C * 0.95`, kept on the x87; each
/// component rounded to f32) plus the camera position. Only when `CanSeeOutSideFromCurrArea`: if the
/// sun vector's z > -0.1 the sun corona (id 1, core colour bytes 0xB7C4D0/D2/D4, size `0xB7C4DC *
/// 2.7335`, range 999999.88, texture 0, pull 1.5, fade 15) is registered, and if z > 0 and the area is
/// 0 also the halo (id 2, colour 0xB7C4D6/D8/DA, size `0xB7C4DC * 6`, flare 1, LOS check). Then
/// `CalcScreenCoors(sunPos, &scr, &w, &h, true, true)` sets `SunScreenX/Y` (0xC3E028/0xC3E02C), or both
/// 1000000.0 when off screen.
#[hook(addr = 0x6FC5A0, conv = cdecl, name = "CCoronas::DoSunAndMoon", expect = "83 EC 20 A1 D0 9F B7 00",
       unverified = "mvpb-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn coronas_do_sun_and_moon() {
    let idx = g::<u32>(0xB79FD0).read();
    let scale = X87::from_f32(g::<f32>(0xC3EF9C).read()).fmul_m(f(0x3F73_3333));
    let v = g::<[f32; 3]>(0xB7CA50 + idx.wrapping_mul(12)).read();
    let s = [
        X87::from_f32(v[0]).fmul(scale).to_f32(),
        X87::from_f32(v[1]).fmul(scale).to_f32(),
        X87::from_f32(v[2]).fmul(scale).to_f32(),
    ];
    let m = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if m.is_null() { g::<f32>(0xB6F02C) } else { m.byte_add(0x30).cast::<f32>() };
    let mut pos = [
        X87::from_f32(s[0]).fadd_m(cam.read()).to_f32(),
        X87::from_f32(s[1]).fadd_m(cam.wrapping_index(1).read()).to_f32(),
        X87::from_f32(s[2]).fadd_m(cam.wrapping_index(2).read()).to_f32(),
    ];
    if game_can_see_outside_from_curr_area() == 0 {
        return;
    }
    let p = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
    let sun_z = |k: u32| {
        let idx = g::<u32>(0xB79FD0).read();
        !X87::from_f32(g::<f32>(0xB7CA58 + idx.wrapping_mul(12)).read())
            .fcom(X87::from_f32(f(k)))
            .test_any(0x41)
    };
    let b = |a: u32| g::<u8>(a).read() as u32;
    if sun_z(0xBDCC_CCCD) {
        let size = X87::from_f32(g::<f32>(0xB7C4DC).read()).fmul_m(f(0x402E_F1AA)).to_f32().to_bits();
        register_corona_o(
            1, GPtr::NULL, b(0xB7C4D0), b(0xB7C4D2), b(0xB7C4D4), 0xFF, p, size, 0x4974_23FE,
            g::<u32>(0xC3E000).read(), 0, 0, 0, 0, 0, 0, 0x3FC0_0000, 0, 0x4170_0000, 0, 0,
        );
        if sun_z(0x0000_0000) && g::<u32>(0xB72914).read() == 0 {
            let size = X87::from_f32(g::<f32>(0xB7C4DC).read()).fmul_m(f(0x40C0_0000)).to_f32().to_bits();
            register_corona_o(
                2, GPtr::NULL, b(0xB7C4D6), b(0xB7C4D8), b(0xB7C4DA), 0xFF, p, size, 0x4974_23FE,
                g::<u32>(0xC3E000).read(), 1, 0, 1, 0, 0, 0, 0x3FC0_0000, 0, 0x4170_0000, 0, 0,
            );
        }
    }
    let mut scr = [0f32; 3];
    let mut w = 0f32;
    let mut h = 0f32;
    let ok = sprite_calc_screen_coors(
        p,
        GPtr::from_raw(scr.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut w as *mut f32).cast(),
        GPtr::from_raw(&mut h as *mut f32).cast(),
        1,
        1,
    );
    if ok != 0 {
        g::<f32>(0xC3E028).write(scr[0]);
        g::<f32>(0xC3E02C).write(scr[1]);
    } else {
        g::<u32>(0xC3E02C).write(0x4974_2400);
        g::<u32>(0xC3E028).write(0x4974_2400);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CORONAS_DO_SUN_AND_MOON,
    HOOK_REGISTERED_CORONA_UPDATE,
    HOOK_FX_SYSTEM_ADD_PARTICLE,
    HOOK_ROPE_CREATE_HOOK_OBJECT,
    HOOK_PLANT_LOC_TRI_GET_PLANT_DENSITY,
    HOOK_TRANSFORM_POINTS,
    HOOK_GRASS_GEN_POINT_IN_TRIANGLE,
    HOOK_PLANT_CALCULATE_WIND_BENDING,
    HOOK_PLANT_COL_ENT_ENTRY_RELEASE_ENTRY,
    HOOK_PLANT_LOC_TRI_RELEASE,
    HOOK_PLANT_COL_ENT_ENTRY_ADD_ENTRY,
    HOOK_CORONAS_UPDATE,
    HOOK_ROPES_UPDATE,
    HOOK_GRASS_SET_CURRENT_SCAN_CODE,
    HOOK_PLANT_MGR_UPDATE,
    HOOK_PLANT_UPDATE_AMBIENT_COLOR,
    HOOK_AFFECT_COLOUR_WITH_LIGHTING,
    HOOK_MOVING_THINGS_UPDATE,
    HOOK_PLANE_TRAIL_REGISTER_POINT,
    HOOK_PLANE_TRAILS_UPDATE,
    HOOK_UPDATE_CORONA_COORS,
    HOOK_ROPES_FIND_ROPE,
    HOOK_UPDATE_IN_TUNNELNESS,
    HOOK_SET_GAMMA,
    HOOK_CALCULATE_ASPECT_RATIO,
    HOOK_SET_PLANT_FRIENDLY_FLAG_IN_ATOMIC_MI,
    HOOK_SPRITE2D_DRAW_QUAD,
    HOOK_CLOCK_UPDATE,
    HOOK_SKIDMARKS_UPDATE,
    HOOK_RETURN_REAL_TIME_SHADOW,
    HOOK_REGISTER_CORONA_BY_TYPE,
    HOOK_REGISTER_CORONA_TYPE,
];

/// Phase B natives of this module.
/// TransformVectors (0x54EEA0) is registered by `crate::transform_vectors` (one registration per address).
pub const NATIVES: &[sa_core::abi::NativeReg] = &[];
