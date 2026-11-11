//! PORT-RENDER: `CShadows`, `CStaticShadow`, `CRealTimeShadowManager` / `CRealTimeShadow`. Ported from
//! the 1.0 US bytes; gta-reversed `Shadows.cpp` / `RealTimeShadowManager.cpp` and
//! `docs/re/specs/RENDER/CShadows*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, jne, k, rd, wr, x, xi};
use sa_core::timer::TIME_MS;

sa_core::globals! {
    list GLOBALS;
    /// `CShadows::pEmptyBunchList` (CPolyBunch*)
    pub static EMPTY_BUNCH_LIST @ 0xC403D8: u32 = 0;
    /// `CShadows::aPermanentShadows[48]` (CPermanentShadow, 0x38 bytes each)
    pub static PERMANENT_SHADOWS @ 0xC4AC30: [u8; 48 * 0x38] = [0; 48 * 0x38];
    /// `CShadows::aStaticShadows[48]` (CStaticShadow, 0x40 bytes each)
    pub static STATIC_SHADOWS @ 0xC4A030: [u8; 48 * 0x40] = [0; 48 * 0x40];
}

orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CRealTimeShadow::Create(bool isBlurred, int32 blurPasses, bool drawMoreBlur)` (0x706460)
    pub fn real_time_shadow_create(this: GPtr<u8>, blurred: u32, passes: i32, more_blur: u32) -> u8 = thiscall 0x706460 => unported;
    /// `CShadowCamera::Create(int32 rasterSizePower)` (0x705B60)
    pub fn shadow_camera_create(this: GPtr<u8>, size_power: i32) -> GPtr<u8> = thiscall 0x705B60 => unported;
    /// `RwIm2DRenderLine(verts, numVerts, vert1, vert2)` (`RwEngineInstance` slot 0x28; the thunk
    /// 0x734EC0 jumps there)
    pub fn rw_im2d_render_line(verts: GPtr<u8>, n: i32, v1: i32, v2: i32) -> i32 = cdecl table 0xC97B24 0x28 => unported;
    /// `RwCameraBeginUpdate(RwCamera*)` (0x7EE190)
    pub fn rw_camera_begin_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE190 => unported;
    /// `RwCameraEndUpdate(RwCamera*)` (0x7EE180)
    pub fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `CRealTimeShadow::SetupForThisEntity(CPhysical*)` (0x706520)
    pub fn real_time_shadow_setup_for_this_entity(this: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x706520 => unported;
}

/// 1.0 (0x858624)
const K_ONE: u32 = 0x3F80_0000;
/// 0.5 (0x858B8C)
const K_HALF: u32 = 0x3F00_0000;
/// 0.09 (0x85B324): `GetRealTimeShadow`'s squared vehicle speed limit.
const K_SPEED2: u32 = 0x3DB8_51EC;
/// `CWorld::PlayerInFocus` (u8).
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// `CWorld::Players` (CPlayerInfo, 0x190 bytes; `m_pPed` at +0).
const PLAYERS: usize = 0xB7CD98;

/// `CShadowCamera::MakeGradientRaster()` (0x705D20, thiscall): with a camera (`this+0`) whose frame
/// buffer raster (+0x60) has a height >= 1 (else 0), inside `RwCameraBeginUpdate` (skipped, raster
/// returned, when it fails): render states 1/0, 6/0, 0xC/0, 0xB/1, 0xA/0xA, 7/1; one `RwIm2DRenderLine`
/// per row y from (0, y) to (width - 1, y), z = the device's near screen z (`RwEngineInstance+0x18`),
/// rhw = `1 / camera near plane`, grey colour `c * 0x01010101` (shift-or) with `c = _ftol(v)`, `v` from
/// `(float)[0x8D5218]` stepping by `(float)([0x8D521C] - [0x8D5218]) / height` (f32 slots); then render
/// states 6/1, 0xB/6, 0xA/5, 7/2 and `RwCameraEndUpdate`. Returns the raster. The vertices' u / v
/// (uninitialised stack in the original) are 0 here; no texture is bound (STD-6).
#[hook(addr = 0x705D20, conv = thiscall, name = "CShadowCamera::MakeGradientRaster", expect = "83 EC 54 8B 01", this = "CShadowCamera",
       unverified = "std6-render-2", route = "L3-only", hazards = "none", recs = "new-game",
       note = "draws into the shadow camera raster through RwIm2D")]
pub fn shadow_camera_make_gradient_raster(this: GPtr<u8>) -> GPtr<u8> {
    let cam: GPtr<u8> = rd(this, 0);
    if cam.is_null() {
        return GPtr::NULL;
    }
    let rhw = x(k(K_ONE)).fdiv_m(rd::<f32>(cam, 0x80)).to_f32();
    let raster: GPtr<u8> = rd(cam, 0x60);
    let width: i32 = rd(raster, 0xC);
    let height: i32 = rd(raster, 0x10);
    let engine: GPtr<u8> = g::<GPtr<u8>>(0xC97B24).read();
    let z: f32 = rd(engine, 0x18);
    if height < 1 {
        return GPtr::NULL;
    }
    if rw_camera_begin_update(cam).is_null() {
        return raster;
    }
    for (st, v) in [(1, 0), (6, 0), (0xC, 0), (0xB, 1), (0xA, 0xA), (7, 1)] {
        rw_render_state_set(st, v);
    }
    let lo = g::<i32>(0x8D5218).read();
    let hi = g::<i32>(0x8D521C).read();
    let step = xi(hi.wrapping_sub(lo)).fdiv(xi(height)).to_f32();
    let mut v = xi(lo).to_f32();
    let x1 = xi(width - 1).to_f32();
    for i in 0..height {
        let c = ftol32(x(v)) as u32;
        let colour = (((c << 8 | c) << 8 | c) << 8) | c;
        let y = xi(i).to_f32();
        // Two RwIm2DVertex (x, y, z, rhw, colour, u, v).
        let mut verts = [0u32; 14];
        verts[..5].copy_from_slice(&[0f32.to_bits(), y.to_bits(), z.to_bits(), rhw.to_bits(), colour]);
        verts[7..12].copy_from_slice(&[x1.to_bits(), y.to_bits(), z.to_bits(), rhw.to_bits(), colour]);
        rw_im2d_render_line(GPtr::<u32>::from_raw(verts.as_mut_ptr()).cast::<u8>(), 2, 0, 1);
        v = x(v).fadd_m(step).to_f32();
    }
    for (st, v) in [(6, 1), (0xB, 6), (0xA, 5), (7, 2)] {
        rw_render_state_set(st, v);
    }
    rw_camera_end_update(cam);
    raster
}

/// `CStaticShadow::Free()` (0x707670): the poly bunch chain (+4, linked through +0x54) is prepended
/// to `pEmptyBunchList`; +4 and the id +0 cleared.
#[hook(addr = 0x707670, conv = thiscall, name = "CStaticShadow::Free", expect = "8B 41 04 85 C0", this = "CStaticShadow")]
pub fn static_shadow_free(this: GPtr<u8>) {
    let first: GPtr<u8> = rd(this, 4);
    if !first.is_null() {
        let old = EMPTY_BUNCH_LIST.get();
        EMPTY_BUNCH_LIST.set(first.addr() as u32);
        let mut last = first;
        loop {
            let next: GPtr<u8> = rd(last, 0x54);
            if next.is_null() {
                break;
            }
            last = next;
        }
        wr(last, 0x54, old);
    }
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 0, 0);
}

/// `CShadows::TidyUpShadows()` (0x707770): every permanent shadow's type byte (+0x32) = 0.
#[hook(addr = 0x707770, conv = cdecl, name = "CShadows::TidyUpShadows", expect = "B8 62 AC C4 00")]
pub fn tidy_up_shadows() {
    let base = PERMANENT_SHADOWS.ptr().cast::<u8>();
    for i in 0..48 {
        wr::<u8>(base, i * 0x38 + 0x32, 0);
    }
}

/// The poly bunch chain of a static shadow prepended to `pEmptyBunchList` (inlined `CStaticShadow::Free`
/// body; the caller clears the fields).
fn free_bunches(first: GPtr<u8>) {
    let old = EMPTY_BUNCH_LIST.get();
    EMPTY_BUNCH_LIST.set(first.addr() as u32);
    let mut last = first;
    loop {
        let next: GPtr<u8> = rd(last, 0x54);
        if next.is_null() {
            break;
        }
        last = next;
    }
    wr(last, 0x54, old);
}

/// `CShadows::UpdateStaticShadows()` (0x707F40): every static shadow with polys (+4) that was not
/// rendered this frame (+0x3A == 0) and is either not temporary (+0x3C == 0) or older than 5 s
/// (`TIME_MS > +8 + 5000`, unsigned) is freed (inlined `Free`); the rendered flag +0x3A of every
/// shadow cleared.
#[hook(addr = 0x707F40, conv = cdecl, name = "CShadows::UpdateStaticShadows", expect = "53 B9 34 A0 C4 00")]
pub fn update_static_shadows() {
    let base = STATIC_SHADOWS.ptr().cast::<u8>();
    for i in 0..48 {
        let sh = base.byte_add(i * 0x40);
        if rd::<u32>(sh, 4) != 0
            && rd::<u8>(sh, 0x3A) == 0
            && (rd::<u8>(sh, 0x3C) == 0 || TIME_MS.get() > rd::<u32>(sh, 8).wrapping_add(5000))
        {
            let first: GPtr<u8> = rd(sh, 4);
            if !first.is_null() {
                free_bunches(first);
            }
            wr::<u32>(sh, 4, 0);
            wr::<u32>(sh, 0, 0);
        }
        wr::<u8>(sh, 0x3A, 0);
    }
}

/// `CShadows::SetRenderModeForShadowType(uint8)` (0x707460): source / destination blend per type:
/// 1, 4, 5, 6, 8 = (5, 6) src alpha / inv src alpha; 2 = (2, 2) one / one; 3 = (1, 4) zero / inv src
/// colour; 7 and anything else: nothing (jump table 0x7074CC).
#[hook(addr = 0x707460, conv = cdecl, name = "CShadows::SetRenderModeForShadowType", expect = "0F B6 44 24 04")]
pub fn set_render_mode_for_shadow_type(ty: u32) {
    let (src, dst) = match ty as u8 {
        1 | 4 | 5 | 6 | 8 => (5, 6),
        2 => (2, 2),
        3 => (1, 4),
        _ => return,
    };
    rw_render_state_set(0xA, src);
    rw_render_state_set(0xB, dst);
}

/// `CRealTimeShadowManager::Init()` (0x7067C0): once (`m_bInitialised` +0): 16 `new CRealTimeShadow`
/// (0x4C bytes, constructor inlined: +0 / +8 / +0xC / +0x14 / +0x18 / +0x1C / +0x28 = 0, +0x24 = -1, bytes
/// +4 / +5 / +0x10 / +0x20 = 0; a failed allocation stores null and still calls `Create` on it) into
/// +4.., each `Create(true, 4, true)`; the two shadow cameras +0x44 / +0x4C created with size power 6,
/// the gradient raster of +0x4C made; +0 = 1.
#[hook(addr = 0x7067C0, conv = thiscall, name = "CRealTimeShadowManager::Init", expect = "53 57 8B F9 8A 07", this = "CRealTimeShadowManager")]
pub fn real_time_shadow_manager_init(this: GPtr<u8>) {
    if rd::<u8>(this, 0) != 0 {
        return;
    }
    for i in 0..16 {
        let p = operator_new(0x4C);
        if !p.is_null() {
            for o in [8, 0xC, 0x14, 0x18, 0] {
                wr::<u32>(p, o, 0);
            }
            wr::<i32>(p, 0x24, -1);
            wr::<u32>(p, 0x28, 0);
            wr::<u32>(p, 0x1C, 0);
            for o in [0x10, 0x20, 4, 5] {
                wr::<u8>(p, o, 0);
            }
        }
        wr(this, 4 + i * 4, p);
        real_time_shadow_create(p, 1, 4, 1);
    }
    shadow_camera_create(this.byte_add(0x44), 6);
    shadow_camera_create(this.byte_add(0x4C), 6);
    shadow_camera_make_gradient_raster(this.byte_add(0x4C));
    wr::<u8>(this, 0, 1);
}

/// `CShadows::CalcPedShadowValues(CVector sunPos, float* frontX, float* frontY, float* sideX,
/// float* sideY, float* dispX, float* dispY)` (0x7076C0, cdecl): front = -sun.xy scaled by
/// `(1 + |front|) * (1 / |front|)` (the length from the stored f32s, the factors kept on the x87 stack),
/// side = (-sun.y, sun.x) * (1 / |front|), displacement = -sun.xy (unrounded stack copies); all six
/// halved at the end (load / multiply / store each, in order).
#[hook(addr = 0x7076C0, conv = cdecl, name = "CShadows::CalcPedShadowValues", expect = "8B 44 24 10 D9 44 24 04")]
#[allow(clippy::too_many_arguments)]
pub fn calc_ped_shadow_values(
    sun_x: u32,
    sun_y: u32,
    _sun_z: u32,
    front_x: GPtr<f32>,
    front_y: GPtr<f32>,
    side_x: GPtr<f32>,
    side_y: GPtr<f32>,
    disp_x: GPtr<f32>,
    disp_y: GPtr<f32>,
) {
    let (sx, sy) = (f32::from_bits(sun_x), f32::from_bits(sun_y));
    let nx = x(sx).fchs();
    front_x.write(nx.to_f32());
    let ny = x(sy).fchs();
    front_y.write(ny.to_f32());
    let a = x(front_x.read());
    let len = a.fmul(a).fadd(ny.fmul(ny)).fsqrt();
    let inv = x(k(K_ONE)).fdiv(len);
    let scale = len.fadd_m(k(K_ONE)).fmul(inv);
    front_x.write(scale.fmul_m(front_x.read()).to_f32());
    front_y.write(scale.fmul_m(front_y.read()).to_f32());
    side_x.write(x(sy).fmul(inv).fchs().to_f32());
    side_y.write(inv.fmul_m(sx).to_f32());
    disp_x.write(nx.to_f32());
    disp_y.write(ny.to_f32());
    for p in [front_x, front_y, side_x, side_y, disp_x, disp_y] {
        p.write(x(p.read()).fmul_m(k(K_HALF)).to_f32());
    }
}

/// `CRealTimeShadowManager::GetRealTimeShadow(CPhysical*)` (0x706970): a shadow for the entity unless
/// the manager is not initialised or the player's vehicle moves faster than `0.09` (squared speed,
/// x87 order x, y, z; `test ah, 0x41`: unordered allows). A ped (type 3) whose +0x598 is null skips the
/// speed test and takes shadow 0 (+4); everything else takes the last free one (owner +0 null) of
/// shadows 1..15 (+8..+0x40). The chosen shadow is set up for the entity, stored in the entity's
/// +0x134, flags +4 = 1, +5 = 0. Returns it (or null). gta-reversed has the shadow-0 test inverted.
#[hook(addr = 0x706970, conv = thiscall, name = "CRealTimeShadowManager::GetRealTimeShadow", expect = "53 55 8B 6C 24 0C", this = "CRealTimeShadowManager")]
pub fn get_real_time_shadow(this: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> {
    let mut allow = true;
    let first = rd::<u8>(e, 0x36) & 7 == 3 && rd::<u32>(e, 0x598) == 0;
    if !first {
        let focus = u32::from(g::<u8>(PLAYER_IN_FOCUS).read());
        let ped: GPtr<u8> = g::<GPtr<u8>>(PLAYERS + (focus * 0x190) as usize).read();
        let veh: GPtr<u8> = rd(ped, 0x58C);
        if rd::<u8>(ped, 0x46D) & 1 != 0 && !veh.is_null() {
            let (vx, vy, vz) = (x(rd(veh, 0x44)), x(rd(veh, 0x48)), x(rd(veh, 0x4C)));
            let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
            if !jne(s, x(k(K_SPEED2)), 0x41) {
                allow = false;
            }
        }
    }
    if rd::<u8>(this, 0) == 0 || !allow {
        return GPtr::NULL;
    }
    let mut sh = GPtr::<u8>::NULL;
    if first {
        sh = rd(this, 4);
    } else {
        for i in 1..16 {
            let c: GPtr<u8> = rd(this, 4 + i * 4);
            if rd::<u32>(c, 0) == 0 {
                sh = c;
            }
        }
    }
    if !sh.is_null() {
        real_time_shadow_setup_for_this_entity(sh, e);
        wr(e, 0x134, sh);
        wr::<u8>(sh, 4, 1);
        wr::<u8>(sh, 5, 0);
    }
    sh
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SHADOW_CAMERA_MAKE_GRADIENT_RASTER,
    HOOK_CALC_PED_SHADOW_VALUES,
    HOOK_GET_REAL_TIME_SHADOW,
    HOOK_STATIC_SHADOW_FREE,
    HOOK_TIDY_UP_SHADOWS,
    HOOK_UPDATE_STATIC_SHADOWS,
    HOOK_SET_RENDER_MODE_FOR_SHADOW_TYPE,
    HOOK_REAL_TIME_SHADOW_MANAGER_INIT,
];
