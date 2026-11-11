//! PORT-RENDER: `CBulletTraces` (static class; `CBulletTrace` 0x2C bytes). Ported from the 1.0 US
//! bytes; gta-reversed `BulletTraces.cpp` and `docs/re/specs/RENDER/CBulletTraces.deviations.md` as
//! reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CBulletTraces::aTraces[16]` (CBulletTrace, 0x2C bytes: start, end, +0x18 exists, +0x1C creation
    /// time, +0x20 lifetime, +0x24 radius, +0x28 transparency)
    pub static TRACES @ 0xC7C748: [u8; 16 * 0x2C] = [0; 16 * 0x2C];
}

/// `CBulletTraces::Update()` (0x723FB0): every existing trace whose age `TIME_MS - created` (unsigned)
/// reached its lifetime stops existing (the loop unrolled 8 times, `TIME_MS` read once).
#[hook(addr = 0x723FB0, conv = cdecl, name = "CBulletTraces::Update", expect = "8B 0D 84 CB B7 00")]
pub fn update() {
    let now = TIME_MS.get();
    let base = TRACES.ptr().cast::<u8>();
    for i in 0..16 {
        let t = base.byte_add(i * 0x2C);
        if rd::<u8>(t, 0x18) != 0 && now.wrapping_sub(rd::<u32>(t, 0x1C)) >= rd::<u32>(t, 0x20) {
            wr::<u8>(t, 0x18, 0);
        }
    }
}

/// `TheCamera.m_nActiveCam` (uint8) and `TheCamera.m_aCams[0].m_nMode` (uint16; `CCam` 0x238 bytes).
const ACTIVE_CAM: usize = 0xB6F081;
const CAM0_MODE: usize = 0xB6F1A8;

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(this: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `CBulletTraces::AddTrace(CVector* start, CVector* end, float radius, uint32 lifeTime, uint8
    /// transparency)` (0x723750); the radius as a raw dword, the byte as its pushed dword
    pub fn add_trace(start: GPtr<f32>, end: GPtr<f32>, radius: u32, life: u32, alpha: u32) = cdecl 0x723750 => unported;
}

/// `CBulletTraces::Init()` (0x721D50): no trace exists.
#[hook(addr = 0x721D50, conv = cdecl, name = "CBulletTraces::Init", expect = "B8 60 C7 C7 00")]
pub fn init() {
    let base = TRACES.ptr().cast::<u8>();
    for i in 0..16 {
        wr::<u8>(base.byte_add(i * 0x2C), 0x18, 0);
    }
}

/// `rand() * (1 / 32767)` as `fild` + `fmul` (the call attributed to its original return address).
fn rand_unit(ret: u32) -> X87 {
    xi(sa_core::orig::at_site(ret, sa_core::crt::rand)).fmul_m(k(0x3800_0100))
}

/// `CBulletTraces::AddTrace(CVector* start, CVector* end, eWeaponType, CEntity* from)` (0x726AF0, the
/// wrapper): a shot from the player (ped, or the player's vehicle) while the active camera is in one of
/// the first-person / aiming modes 0x22, 7, 0x2E, 8, 0x33, 0x2A, 0x27, 0x28, 0x34, 0x2D adds nothing
/// unless the player vehicle's (else the ped's) speed `sqrt((z*z + y*y) + x*x)` is >= 0.05 (or NaN).
/// Otherwise the trace starts at a random point `rand/32767 * len` along the normalised direction and
/// has the length `rand/32767 * 3 + 2` when that is >= the remaining length (or NaN), else a second such
/// random length; `AddTrace(start, end, 0.01, 300, 70)`. The original stores the length into its
/// `from` argument slot (stack only; a local here).
#[hook(addr = 0x726AF0, conv = cdecl, name = "CBulletTraces::AddTrace-Wrapper", expect = "83 EC 34 56 6A FF")]
pub fn add_trace_wrapper(start: GPtr<f32>, end: GPtr<f32>, _weapon: u32, from: GPtr<u8>) {
    let from_player = from == find_player_ped(-1)
        || (!find_player_vehicle(-1, 0).is_null() && find_player_vehicle(-1, 0) == from);
    if from_player {
        let cam = i32::from(g::<u8>(ACTIVE_CAM).read());
        let mode = g::<u8>(CAM0_MODE)
            .byte_add(cam as isize * 0x238)
            .cast::<u16>()
            .read();
        if matches!(
            mode,
            0x22 | 7 | 0x2E | 8 | 0x33 | 0x2A | 0x27 | 0x28 | 0x34 | 0x2D
        ) {
            let e = if find_player_vehicle(-1, 0).is_null() {
                find_player_ped(-1)
            } else {
                find_player_vehicle(-1, 0)
            };
            let (vx, vy, vz) = (rd::<f32>(e, 0x44), rd::<f32>(e, 0x48), rd::<f32>(e, 0x4C));
            let s = x(vz)
                .fmul_m(vz)
                .fadd(x(vy).fmul_m(vy))
                .fadd(x(vx).fmul_m(vx))
                .fsqrt();
            // `fcomp 0.05` + `test ah, 5` + `jnp`: no trace when the speed is below 0.05.
            if !s.fcom(x(k(0x3D4C_CCCD))).test_parity(5) {
                return;
            }
        }
    }
    let c = |p: GPtr<f32>, i: i32| p.wrapping_index(i).read();
    let mut d = [
        x(c(end, 0)).fsub_m(c(start, 0)).to_f32(),
        x(c(end, 1)).fsub_m(c(start, 1)).to_f32(),
        0.0,
    ];
    let dz = x(c(end, 2)).fsub_m(c(start, 2));
    d[2] = dz.to_f32();
    let len = dz
        .fmul_m(d[2])
        .fadd(x(d[1]).fmul_m(d[1]))
        .fadd(x(d[0]).fmul_m(d[0]))
        .fsqrt()
        .to_f32();
    vector_normalise(GPtr::from_raw(d.as_mut_ptr()));
    let s = rand_unit(0x726C3C).fmul_m(len);
    let dxs = x(d[0]).fmul(s);
    let dys = x(d[1]).fmul(s);
    let dzs = x(d[2]).fmul(s).to_f32();
    let mut p = [0f32; 6];
    p[0] = dxs.fadd_m(c(start, 0)).to_f32();
    p[1] = dys.fadd_m(c(start, 1)).to_f32();
    p[2] = x(dzs).fadd_m(c(start, 2)).to_f32();
    let rest = x(len).fsub(s).to_f32();
    let v = rand_unit(0x726C8F).fmul_m(k(0x4040_0000)).fadd_m(k(0x4000_0000));
    // `fcomp rest` + `test ah, 5` + `jp`: the remaining length unless the random one is shorter.
    let l = if v.fcom(x(rest)).test_parity(5) {
        x(rest)
    } else {
        rand_unit(0x726CB9).fmul_m(k(0x4040_0000)).fadd_m(k(0x4000_0000))
    };
    let dxl = x(d[0]).fmul(l);
    let dyl = x(d[1]).fmul(l).to_f32();
    let dzl = x(d[2]).fmul(l).to_f32();
    p[3] = dxl.fadd_m(p[0]).to_f32();
    p[4] = x(dyl).fadd_m(p[1]).to_f32();
    p[5] = x(dzl).fadd_m(p[2]).to_f32();
    let pp = GPtr::<f32>::from_raw(p.as_mut_ptr());
    add_trace(pp, pp.wrapping_index(3), 0x3C23_D70A, 0x12C, 0x46);
}

orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(RxObjSpace3DVertex*, uint32 count, RwMatrix* ltm, uint32 flags)` (0x7EF450)
    pub fn rw_im3d_transform(verts: GPtr<u8>, count: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    pub fn rw_im3d_render_indexed_primitive(ty: u32, indices: GPtr<u16>, count: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `CVector::CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730)
    pub fn cross_product(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C730 => unported;
}

/// `TheCamera.m_pMat` and `TheCamera.m_placement.m_vPosn`.
const CAMERA_MATRIX: usize = 0xB6F03C;
const CAMERA_PLACEMENT_POS: usize = 0xB6F02C;

/// An unsigned dword as `fild` + (`fadd 2^32` when negative as a signed value).
fn u32_to_x87(v: u32) -> X87 {
    let f = xi(v as i32);
    if (v as i32) < 0 {
        f.fadd_m(k(0x4F80_0000))
    } else {
        f
    }
}

/// `CBulletTraces::Render()` (0x723C10): sets the render states (z-write off, blend 5 / 6, cull 1,
/// vertex alpha on, no raster), then draws every existing trace as six vertices / four triangles (the
/// stack index list 4,1,3, 1,0,3, 0,2,3, 3,2,5). With `t = age / lifetime` (unsigned conversions),
/// `W` = the normalised cross product of the normalised camera-to-start vector and the normalised
/// direction, `A = W * (1 - t) * radius`, `B = dir * len * (1 - t)`, `P = end - B`: the vertices are `P`,
/// `P + A`, `P - A`, `end`, `end + A`, `end - A`, coloured 0x00FFFF80 except the `end` vertex, whose
/// alpha is `_ftol(transparency * (1 - t))` (low byte). The states are restored at the end. Only the
/// positions and colours of the stack vertices are written (the original leaves their normals and UVs
/// as uninitialised stack; zero here, seen only by the D3D stream).
#[hook(addr = 0x723C10, conv = cdecl, name = "CBulletTraces::Render", expect = "81 EC 4C 01 00 00")]
pub fn render() {
    let mut idx: [u16; 12] = [4, 1, 3, 1, 0, 3, 0, 2, 3, 3, 2, 5];
    rw_render_state_set(8, 0);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(0x14, 1);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(1, 0);
    let base = TRACES.ptr().cast::<u8>();
    for i in 0..16 {
        let t = base.byte_add(i * 0x2C);
        if rd::<u8>(t, 0x18) == 0 {
            continue;
        }
        let m = g::<GPtr<u8>>(CAMERA_MATRIX).read();
        let cam = if m.is_null() {
            g::<u8>(CAMERA_PLACEMENT_POS)
        } else {
            m.byte_add(0x30)
        };
        let s = |o: isize| rd::<f32>(t, o);
        let mut v = [
            x(s(0)).fsub_m(rd::<f32>(cam, 0)).to_f32(),
            x(s(4)).fsub_m(rd::<f32>(cam, 4)).to_f32(),
            x(s(8)).fsub_m(rd::<f32>(cam, 8)).to_f32(),
        ];
        let vp = GPtr::<f32>::from_raw(v.as_mut_ptr());
        vector_normalise(vp);
        let mut d = [
            x(s(0xC)).fsub_m(s(0)).to_f32(),
            x(s(0x10)).fsub_m(s(4)).to_f32(),
            0.0,
        ];
        let dz = x(s(0x14)).fsub_m(s(8));
        d[2] = dz.to_f32();
        let len = dz
            .fmul_m(d[2])
            .fadd(x(d[1]).fmul_m(d[1]))
            .fadd(x(d[0]).fmul_m(d[0]))
            .fsqrt()
            .to_f32();
        let dp = GPtr::<f32>::from_raw(d.as_mut_ptr());
        vector_normalise(dp);
        let mut w = [0f32; 3];
        let wp = GPtr::<f32>::from_raw(w.as_mut_ptr());
        cross_product(wp, vp, dp);
        vector_normalise(wp);
        let (d0, d1, d2) = (
            dp.read(),
            dp.wrapping_index(1).read(),
            dp.wrapping_index(2).read(),
        );
        let (w0, w1, w2) = (
            wp.read(),
            wp.wrapping_index(1).read(),
            wp.wrapping_index(2).read(),
        );
        let age = u32_to_x87(TIME_MS.get().wrapping_sub(rd::<u32>(t, 0x1C)));
        let life = u32_to_x87(rd::<u32>(t, 0x20));
        let rest = age.fdiv(life).fsubr_m(k(0x3F80_0000));
        let alpha = ftol32(xi(i32::from(rd::<u8>(t, 0x28))).fmul(rest)) as u8;
        let r = rest.fmul_m(rd::<f32>(t, 0x24));
        let ax = x(w0).fmul(r).to_f32();
        let ay = x(w1).fmul(r).to_f32();
        let az = r.fmul_m(w2);
        let lx = x(d0).fmul_m(len);
        let ly = x(d1).fmul_m(len);
        let lz = x(d2).fmul_m(len).to_f32();
        let bx = lx.fmul(rest).to_f32();
        let by = ly.fmul(rest);
        let bz = x(lz).fmul(rest);
        let (ex, ey, ez) = (s(0xC), s(0x10), s(0x14));
        let px = x(ex).fsub_m(bx).to_f32();
        let py = x(ey).fsub(by).to_f32();
        let pz = x(ez).fsub(bz);
        let pz32 = pz.to_f32();
        let pos: [[f32; 3]; 6] = [
            [px, py, pz32],
            [
                x(ax).fadd_m(px).to_f32(),
                x(ay).fadd_m(py).to_f32(),
                az.fadd(pz).to_f32(),
            ],
            [
                x(px).fsub_m(ax).to_f32(),
                x(py).fsub_m(ay).to_f32(),
                pz.fsub(az).to_f32(),
            ],
            [ex, ey, ez],
            [
                x(ax).fadd_m(ex).to_f32(),
                x(ay).fadd_m(ey).to_f32(),
                az.fadd_m(ez).to_f32(),
            ],
            [
                x(ex).fsub_m(ax).to_f32(),
                x(ey).fsub_m(ay).to_f32(),
                x(ez).fsub(az).to_f32(),
            ],
        ];
        let mut verts = [0u32; 6 * 9];
        for (k, p) in pos.iter().enumerate() {
            verts[k * 9] = p[0].to_bits();
            verts[k * 9 + 1] = p[1].to_bits();
            verts[k * 9 + 2] = p[2].to_bits();
            verts[k * 9 + 6] = if k == 3 {
                u32::from(alpha) << 24 | 0xFF_FF80
            } else {
                0xFF_FF80
            };
        }
        if !rw_im3d_transform(
            GPtr::<u32>::from_raw(verts.as_mut_ptr()).cast(),
            6,
            GPtr::from_addr(0),
            0x10,
        )
        .is_null()
        {
            rw_im3d_render_indexed_primitive(3, GPtr::from_raw(idx.as_mut_ptr()), 0xC);
            rw_im3d_end();
        }
    }
    rw_render_state_set(8, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(0x14, 2);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_UPDATE, HOOK_INIT, HOOK_ADD_TRACE_WRAPPER, HOOK_RENDER];
