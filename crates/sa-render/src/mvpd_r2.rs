//! WRITER-MVP-D (batch `mvpd-render-2`): `CBirds` (spawn and per-frame update) on the in-world path.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Birds.cpp` as reading aid. x87 arithmetic follows the
//! original's instruction order on the `sa_math::x87::X87` model; every direct `call rand` runs under
//! `orig::at_site` with the original return address. Bird layout (0x44 bytes, 6 at 0xC6A8B0,
//! `CBirds::NumberOfBirds` 0xC6A8A4): position +0, velocity +0xC, target velocity +0x18, angle +0x24,
//! next update time +0x28, wing period +0x2C, wing speed +0x30, max distance +0x34, mode +0x38,
//! colours +0x39..+0x3E, active +0x3F, curving +0x40.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jp, k, rd, wr, x};

orig! {
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90, ST0)
    fn random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(this: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(x, y, z, float* out, bool, CVector*)` (0x6E8580, bool in AL)
    fn water_level_no_waves(px: u32, py: u32, pz: u32, out: GPtr<u8>, a: u32, b: u32) -> u8 = cdecl 0x6E8580 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, 7 bools)` (0x56A490, bool in AL)
    #[allow(clippy::too_many_arguments)]
    fn is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CBirds::CreateNumberOfBirds` (0x711EF0)
    #[allow(clippy::too_many_arguments)]
    fn create_number_of_birds_o(sx: u32, sy: u32, sz: u32, tx: u32, ty: u32, tz: u32, count: i32, ty_: i32, check: u32) = cdecl 0x711EF0 => crate::mvpd_r2::create_number_of_birds;
}

const BIRDS: usize = 0xC6A8B0;
const NUM_BIRDS: usize = 0xC6A8A4;

fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `CBirds::CreateNumberOfBirds(CVector start, CVector target, int32 count, eBirdsBiome type, bool
/// checkObstacles)` (0x711EF0, cdecl). Max distance by type (0: 45, 1: 80, else 40). For each of
/// `count` birds with a free slot: position = start; `d = normalise(target - pos)`; the target becomes
/// `pos + d * maxDist * 2.4` (written into the target argument slots); with `checkObstacles` a blocked
/// `GetIsLineOfSightClear(pos, target, buildings)` skips the bird (its position stays written). Then
/// `r = rand() % 31`, angle `atan2(d.x, d.y)`, period `1000 - 12 r`, the slot active (mode 1, no
/// curving, next update 0), the position offset by the per-index tables 0x8D5250/68/80, speed
/// `r * 0.02 + 4 | 3 | 5` (type 0 | 1 | 2) along `d` into both velocities and type-specific colours and
/// wing speed from further `rand()` calls; `NumberOfBirds + 1` (also for unknown types).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x711EF0, conv = cdecl, name = "CBirds::CreateNumberOfBirds", expect = "8B 44 24 20 83 EC 5C",
       unverified = "mvpd-render-2", route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "grove-walk", note = "FPATAN angle; writes the scaled target into its own argument slots; rand sites")]
pub fn create_number_of_birds(sx: u32, sy: u32, sz: u32, tx: u32, ty: u32, tz: u32, count: i32, kind: i32, check: u32) {
    let max_dist = match kind {
        0 => 0x4234_0000u32,
        1 => 0x42A0_0000,
        _ => 0x4220_0000,
    };
    let md = f32::from_bits(max_dist);
    let mut target: [u32; 3] = [tx, ty, tz];
    let mut i = 0i32;
    while i < count {
        let slot = (0..6isize).find(|&s| rd::<u8>(g::<u8>(BIRDS), s * 0x44 + 0x3F) == 0);
        let Some(slot) = slot else {
            i += 1;
            continue;
        };
        let b = g::<u8>(BIRDS).byte_add(slot * 0x44);
        wr::<u32>(b, 0, sx);
        wr::<u32>(b, 4, sy);
        wr::<u32>(b, 8, sz);
        let mut d: [f32; 3] = [
            x(f32::from_bits(target[0])).fsub_m(rd::<f32>(b, 0)).to_f32(),
            x(f32::from_bits(target[1])).fsub_m(rd::<f32>(b, 4)).to_f32(),
            x(f32::from_bits(target[2])).fsub_m(rd::<f32>(b, 8)).to_f32(),
        ];
        vector_normalise(lp(&mut d));
        let k24 = k(0x4019_999A);
        let zk = x(x(d[2]).fmul_m(md).to_f32()).fmul_m(k24);
        let xk = x(x(d[0]).fmul_m(md).fmul_m(k24).to_f32());
        let yk = x(d[1]).fmul_m(md).fmul_m(k24);
        target[0] = xk.fadd_m(rd::<f32>(b, 0)).to_f32().to_bits();
        target[1] = yk.fadd_m(rd::<f32>(b, 4)).to_f32().to_bits();
        target[2] = zk.fadd_m(rd::<f32>(b, 8)).to_f32().to_bits();
        if check as u8 != 0 && is_line_of_sight_clear(b, lp(&mut target), 1, 0, 0, 0, 0, 0, 0) == 0 {
            i += 1;
            continue;
        }
        let r = rand_at(0x71_204A);
        let angle = x(d[0]).fpatan(x(d[1]));
        let r31 = r % 31;
        wr::<u32>(b, 0x34, max_dist);
        wr::<u8>(b, 0x3F, 1);
        wr::<u8>(b, 0x38, 1);
        wr::<u32>(b, 0x28, 0);
        wr::<u8>(b, 0x40, 0);
        let period = 1000 - r31 * 12;
        wr::<i32>(b, 0x2C, period);
        wr::<f32>(b, 0x24, angle.to_f32());
        let off = |t: usize| g::<f32>(t).byte_add(i as isize * 4).read();
        wr::<f32>(b, 0, x(off(0x8D5250)).fadd_m(rd::<f32>(b, 0)).to_f32());
        wr::<f32>(b, 4, x(off(0x8D5268)).fadd_m(rd::<f32>(b, 4)).to_f32());
        wr::<f32>(b, 8, x(off(0x8D5280)).fadd_m(rd::<f32>(b, 8)).to_f32());
        let set_vel = |base: u32| {
            let s = X87::from_i32(r31).fmul_m(k(0x3CA3_D70A)).fadd_m(k(base));
            let v = [x(d[0]).fmul(s).to_f32(), x(d[1]).fmul(s).to_f32(), x(d[2]).fmul(s).to_f32()];
            for (c, val) in v.iter().enumerate() {
                wr::<f32>(b, 0x18 + c as isize * 4, *val);
                wr::<f32>(b, 0xC + c as isize * 4, *val);
            }
        };
        let k7c = k(0x3800_0100);
        match kind {
            0 => {
                set_vel(0x4080_0000);
                let c1 = ((rand_at(0x71_229E) as u8) & 0x3F).wrapping_add(0x50);
                for o in [0x3B, 0x3A, 0x39] {
                    wr::<u8>(b, o, c1);
                }
                let c2 = ((rand_at(0x71_22B0) as u8) & 0x3F).wrapping_sub(0x4C);
                for o in [0x3E, 0x3D, 0x3C] {
                    wr::<u8>(b, o, c2);
                }
                let rr = rand_at(0x71_22C2);
                wr::<i32>(b, 0x2C, period);
                let w = X87::from_i32(rr).fmul_m(k7c).fmul_m(k(0x3ECC_CCCE)).fadd_m(k(0x3F4C_CCCD));
                wr::<f32>(b, 0x30, w.to_f32());
            }
            1 => {
                wr::<u8>(b, 0x39, 0x1E);
                wr::<u8>(b, 0x3A, 0xF);
                wr::<u8>(b, 0x3B, 0xA);
                wr::<u8>(b, 0x3C, 0x50);
                wr::<u8>(b, 0x3D, 0xF);
                wr::<u8>(b, 0x3E, 0xA);
                set_vel(0x4040_0000);
                let rr = rand_at(0x71_21F2);
                wr::<i32>(b, 0x2C, (0x7D - r31) * 12);
                let w = X87::from_i32(rr).fmul_m(k7c).fmul_m(k(0x3F00_0000)).fadd_m(k(0x4000_0000));
                wr::<f32>(b, 0x30, w.to_f32());
                if rand_at(0x71_2224) & 4 != 0 {
                    wr::<i32>(b, 0x2C, 1_000_000);
                }
                if rand_at(0x71_2234) & 8 != 0 {
                    wr::<u8>(b, 0x40, 1);
                }
            }
            2 => {
                set_vel(0x40A0_0000);
                let c1 = ((rand_at(0x71_212D) as u8) & 0x7F).wrapping_add(0x80);
                for o in [0x3B, 0x3A, 0x39] {
                    wr::<u8>(b, o, c1);
                }
                let c2 = ((rand_at(0x71_213F) as u8) & 0x7F).wrapping_add(0x80);
                for o in [0x3E, 0x3D, 0x3C] {
                    wr::<u8>(b, o, c2);
                }
                let rr = rand_at(0x71_2151);
                wr::<i32>(b, 0x2C, 500 - r31 * 6);
                let w = X87::from_i32(rr).fmul_m(k7c).fmul_m(k(0x3DCC_CCD0)).fadd_m(k(0x3F00_0000));
                wr::<f32>(b, 0x30, w.to_f32());
            }
            _ => {}
        }
        g::<i32>(NUM_BIRDS).write(g::<i32>(NUM_BIRDS).read() + 1);
        i += 1;
    }
}

/// `CBirds::Update()` (0x712330). Spawning (outside interiors, fewer than 6 birds, hours 6..21, on
/// frame `& 0x1FF == 6`): `n = GetRandomNumberInRange(1, 7 - NumberOfBirds)`; type 0 (camera matrix
/// +0xE4 below 30), else type 1 (weather 0xC81314 == 4, n capped at `3 - NumberOfBirds`) or 2; two
/// discarded `rand()`s; height = camera z + `GetRandomNumberInRange` (4..13 | 15..25 | 2..10) which
/// must exceed 5; distance 45 | 80 | 40; angle `(rand() & 0xFF) * 0.0245` on odd `rand()`, else the
/// camera front's heading `atan2(f.x, f.y)` plus `((rand() & 0xFF) - 128) / 256`; the start
/// `camera + (sin, cos) * distance` must be at least 4 above water (`GetWaterLevelNoWaves`); the birds
/// fly towards `camera + front.xy * 8` at that height (`CreateNumberOfBirds(…, n, type, true)`).
/// Then the bird `FrameCounter % 6` is removed beyond its max distance (2D). Every active bird in mode
/// 1 past its update time: curving birds rotate their target velocity by `timeStep * 0.002`; the
/// velocity eases to the target by `min(timeStep * 0.02 * 0.5, 1)`, the position moves by
/// `velocity * timeStep * 0.02` and the angle becomes `atan2(target.x, target.y)`.
#[hook(addr = 0x712330, conv = cdecl, name = "CBirds::Update", expect = "A1 3C F0 B6 00",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSIN/FCOS/FPATAN; rand sites 0x7123FE, 0x712403, 0x71247E, 0x712487, 0x7124FB")]
pub fn birds_update() {
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let c: [f32; 3] = cam.cast::<[f32; 3]>().read();
    'spawn: {
        if g::<i32>(0xB72914).read() != 0 {
            break 'spawn;
        }
        let nb = g::<i32>(NUM_BIRDS).read();
        if nb >= 6 {
            break 'spawn;
        }
        let hour = g::<u8>(0xB70153).read();
        if hour >= 0x16 || hour <= 5 {
            break 'spawn;
        }
        if g::<u32>(0xB7CB4C).read() & 0x1FF != 6 {
            break 'spawn;
        }
        let mut n = random_int_in_range(1, 7 - nb);
        let kind = if !jp(x(g::<f32>(0xB6F10C).read()), x(k(0x41F0_0000)), 5) {
            0
        } else if g::<u16>(0xC81314).read() == 4 {
            let m = 3 - g::<i32>(NUM_BIRDS).read();
            if n >= m {
                n = m;
            }
            1
        } else {
            2
        };
        if n <= 0 {
            break 'spawn;
        }
        rand_at(0x71_23FE);
        rand_at(0x71_2403);
        let (lo, hi, dist) = match kind {
            0 => (0x4080_0000, 0x4150_0000, 0x4234_0000u32),
            1 => (0x4170_0000, 0x41C8_0000, 0x42A0_0000),
            _ => (0x4000_0000, 0x4120_0000, 0x4220_0000),
        };
        let h = X87::from_f64_exact(random_float_in_range(lo, hi)).fadd_m(c[2]);
        let height = h.to_f32();
        if h.fcom(x(k(0x40A0_0000))).test_any(0x41) {
            break 'spawn;
        }
        let angle = if rand_at(0x71_247E) & 1 != 0 {
            X87::from_i32(rand_at(0x71_2487) & 0xFF).fmul_m(k(0x3CC8_F5C3))
        } else {
            let mut v: [u32; 3] = [g::<u32>(0xB6F9AC).read(), g::<u32>(0xB6F9B0).read(), 0];
            if !jp(x(f32::from_bits(v[0])), X87::zero(), 0x44) {
                v[0] = 0x3C23_D70A;
            }
            vector_normalise(lp(&mut v));
            let a = X87::from_i32((rand_at(0x71_24FB) & 0xFF) - 0x80).fmul_m(k(0x3B80_0000));
            a.fadd(x(f32::from_bits(v[0])).fpatan(x(f32::from_bits(v[1]))))
        };
        let df = f32::from_bits(dist);
        let px = angle.fsin().fmul_m(df).fadd_m(c[0]).to_f32().to_bits();
        let py = angle.fcos().fmul_m(df).fadd_m(c[1]).to_f32().to_bits();
        let mut wl = 0f32;
        if water_level_no_waves(px, py, height.to_bits(), lp(&mut wl), 0, 0) != 0 && jp(x(wl).fadd_m(k(0x4080_0000)), x(height), 5) {
            break 'spawn;
        }
        let mut f: [u32; 3] = [g::<u32>(0xB6F9AC).read(), g::<u32>(0xB6F9B0).read(), 0];
        vector_normalise(lp(&mut f));
        let tx = x(f32::from_bits(f[0])).fmul_m(k(0x4100_0000)).fadd_m(c[0]).to_f32().to_bits();
        let ty = x(f32::from_bits(f[1])).fmul_m(k(0x4100_0000)).fadd_m(c[1]).to_f32().to_bits();
        create_number_of_birds_o(px, py, height.to_bits(), tx, ty, height.to_bits(), n, kind, 1);
    }
    let i = (g::<u32>(0xB7CB4C).read() % 6) as isize;
    let b = g::<u8>(BIRDS).byte_add(i * 0x44);
    if rd::<u8>(b, 0x3F) == 1 {
        let dy = x(rd::<f32>(b, 4)).fsub_m(c[1]);
        let dx = x(rd::<f32>(b, 0)).fsub_m(c[0]);
        let m = x(rd::<f32>(b, 0x34));
        let d2 = dx.fmul(dx).fadd(dy.fmul(dy));
        if !jp(m.fmul(m), d2, 5) {
            g::<i32>(NUM_BIRDS).write(g::<i32>(NUM_BIRDS).read() - 1);
            wr::<u8>(b, 0x3F, 0);
        }
    }
    let now = g::<u32>(0xB7CB84).read();
    for j in 0..6isize {
        let b = g::<u8>(BIRDS).byte_add(j * 0x44);
        if rd::<u8>(b, 0x3F) != 1 || rd::<u8>(b, 0x38) != 1 || now <= rd::<u32>(b, 0x28) {
            continue;
        }
        if rd::<u8>(b, 0x40) != 0 {
            let a = x(g::<f32>(0xB7CB5C).read()).fmul_m(k(0x3B03_126F));
            let (s, co) = (a.fsin(), a.fcos());
            let vx = x(rd::<f32>(b, 0x18));
            let vy = x(rd::<f32>(b, 0x1C));
            wr::<f32>(b, 0x18, co.fmul(vx).fadd(s.fmul(vy)).to_f32());
            wr::<f32>(b, 0x1C, co.fmul(vy).fsub(s.fmul(vx)).to_f32());
        }
        let t = x(g::<f32>(0xB7CB5C).read()).fmul_m(k(0x3CA3_D70A));
        let mut f = x(k(0x3F00_0000)).fmul(t);
        let v: [f32; 3] = [rd::<f32>(b, 0x18), rd::<f32>(b, 0x1C), rd::<f32>(b, 0x20)];
        if !jp(X87::one(), f, 5) {
            f = X87::one();
        }
        let fvx = x(v[0]).fmul(f);
        let fvy = x(v[1]).fmul(f).to_f32();
        let fvz = x(v[2]).fmul(f).to_f32();
        let omf = X87::one().fsub(f);
        let a0 = omf.fmul_m(rd::<f32>(b, 0xC)).to_f32();
        let a1 = omf.fmul_m(rd::<f32>(b, 0x10)).to_f32();
        let a2 = omf.fmul_m(rd::<f32>(b, 0x14)).to_f32();
        wr::<f32>(b, 0xC, x(a0).fadd(fvx).to_f32());
        wr::<f32>(b, 0x10, x(a1).fadd_m(fvy).to_f32());
        wr::<f32>(b, 0x14, x(a2).fadd_m(fvz).to_f32());
        let mx = t.fmul_m(rd::<f32>(b, 0xC));
        let my = t.fmul_m(rd::<f32>(b, 0x10)).to_f32();
        let mz = t.fmul_m(rd::<f32>(b, 0x14)).to_f32();
        wr::<f32>(b, 0, mx.fadd_m(rd::<f32>(b, 0)).to_f32());
        wr::<f32>(b, 4, x(my).fadd_m(rd::<f32>(b, 4)).to_f32());
        wr::<f32>(b, 8, x(mz).fadd_m(rd::<f32>(b, 8)).to_f32());
        wr::<f32>(b, 0x24, x(v[0]).fpatan(x(v[1])).to_f32());
    }
}

orig! {
    /// `RwRasterDestroy(RwRaster*)` (0x7FB020)
    fn rw_raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CCullZones::FindMirrorAttributesForCoors(CVector)` (0x72DA70; by value)
    fn find_mirror_attributes(px: u32, py: u32, pz: u32) -> GPtr<u8> = cdecl 0x72DA70 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float, const RwMatrix*)` (0x420C40, bool in AL)
    fn camera_is_sphere_visible_m(this: GPtr<u8>, c: GPtr<u8>, r: u32, m: u32) -> u8 = thiscall 0x420C40 => unported;
    /// `CMirrors::CreateBuffer()` (0x7230A0)
    fn mirrors_create_buffer() = cdecl 0x7230A0 => unported;
    /// `CMirrors::BuildCameraMatrixForScreens(CMatrix&)` (0x7266B0)
    fn mirrors_build_camera_matrix_for_screens(m: GPtr<u8>) = cdecl 0x7266B0 => unported;
    /// `CCamera::DealWithMirrorBeforeConstructRenderList(bool, CVector normal, float v, CMatrix*)` (0x50B510)
    #[allow(clippy::too_many_arguments)]
    fn camera_deal_with_mirror(this: GPtr<u8>, active: u32, nx: u32, ny: u32, nz: u32, v: u32, m: GPtr<u8>) = thiscall 0x50B510 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(this: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CCamera::SetCameraUpForMirror()` (0x51A560)
    fn camera_set_up_for_mirror(this: GPtr<u8>) = thiscall 0x51A560 => unported;
    /// `CCamera::RestoreCameraAfterMirror()` (0x51A5A0)
    fn camera_restore_after_mirror(this: GPtr<u8>) = thiscall 0x51A5A0 => unported;
    /// `CStencilShadows::GraphicsHighQuality()` (0x70F9B0, bool in AL)
    fn stencil_graphics_high_quality() -> u8 = cdecl 0x70F9B0 => unported;
    /// `RwCameraClear(RwCamera*, RwRGBA*, mode)` (0x7EE340)
    fn rw_camera_clear(cam: GPtr<u8>, colour: GPtr<u8>, mode: u32) -> GPtr<u8> = cdecl 0x7EE340 => unported;
    /// `RsCameraBeginUpdate(RwCamera*)` (0x619450)
    fn rs_camera_begin_update(cam: GPtr<u8>) -> i32 = cdecl 0x619450 => unported;
    /// `RwCameraEndUpdate(RwCamera*)` (0x7EE180)
    fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `DefinedState()` (0x734650)
    fn defined_state() = cdecl 0x734650 => unported;
    /// `RenderScene()` (0x53DF40)
    fn render_scene() = cdecl 0x53DF40 => unported;
    /// `CVisibilityPlugins::RenderWeaponPedsForPC()` (0x732F30)
    fn render_weapon_peds_for_pc() = cdecl 0x732F30 => unported;
    /// 0x53E8D0 (thiscall on 0xC88224)
    fn fun_53e8d0(this: GPtr<u8>) = thiscall 0x53E8D0 => unported;
}

/// `CMirrors::pBuffer`, `pZBuffer`, `TypeOfMirror`, `MirrorFlags`, `MirrorV`, `MirrorNormal`.
const M_BUFFER: usize = 0xC7C71C;
const M_ZBUFFER: usize = 0xC7C720;
const M_TYPE: usize = 0xC7C724;
const M_FLAGS: usize = 0xC7C618;
const M_V: usize = 0xC7C61C;
const M_NORMAL: usize = 0xC803D8;
const THE_CAMERA: usize = 0xB6F028;
/// `Scene.m_pRwCamera`
const SCENE_CAMERA: usize = 0xC1703C;

fn destroy_mirror_buffers() {
    let b = g::<GPtr<u8>>(M_BUFFER).read();
    if !b.is_null() {
        rw_raster_destroy(b);
        g::<u32>(M_BUFFER).write(0);
    }
    let z = g::<GPtr<u8>>(M_ZBUFFER).read();
    if !z.is_null() {
        rw_raster_destroy(z);
        g::<u32>(M_ZBUFFER).write(0);
    }
}

/// `CMirrors::BeforeConstructRenderList()` (0x726DF0). A pending reset (`[0xC7C729]`) is cleared and drops
/// the buffers, type and flags. No mirror when the player's vehicle is of type 3 or 4 (+0x594) or the
/// camera is in no mirror cull zone (`FindMirrorAttributesForCoors`); screen mirrors (flag 2 at +0x17)
/// need one of the two screens (corner averages of 0x8D5DD8 / 0x8D5E08, radius 8) visible
/// (`IsSphereVisible` with the matrix 0xB6FA74, or 0xB6FABC when `TheCamera` +0x970 got set). Active:
/// `MirrorV` = +0x10, normal = (i8 +0x14..+0x16) * 0.01, flags = +0x17, type 2 (1 when |normal.z| <=
/// 0.7), `CreateBuffer()`. Inactive: type 0, buffers destroyed, flags 0. Then
/// `TheCamera.DealWithMirrorBeforeConstructRenderList(active, normal, V, NULL)`, or with a local matrix
/// from `BuildCameraMatrixForScreens` (destroyed afterwards) for screens or when `[0xC7C72A]` is set.
#[hook(addr = 0x726DF0, conv = cdecl, name = "CMirrors::BeforeConstructRenderList", expect = "6A FF 68 18 8D 84 00",
       route = "split", hazards = "uninit-locals",
       recs = "grove-walk",
       note = "raster create/destroy (RW heap); the active flag is passed as a dword whose upper bytes the original leaves unwritten; SEH frame for the local CMatrix")]
pub fn mirrors_before_construct_render_list() {
    let mut active = 0u32;
    if g::<u8>(0xC7C729).read() != 0 {
        g::<u8>(0xC7C729).write(0);
        destroy_mirror_buffers();
        g::<u32>(M_TYPE).write(0);
        g::<u32>(M_FLAGS).write(0);
    }
    'found: {
        let veh = find_player_vehicle(-1, 0);
        if !veh.is_null() {
            let t = rd::<u32>(veh, 0x594);
            if t == 3 || t == 4 {
                break 'found;
            }
        }
        let cm = g::<GPtr<u8>>(0xB6F03C).read();
        let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
        let attrs = find_mirror_attributes(rd::<u32>(cam, 0), rd::<u32>(cam, 4), rd::<u32>(cam, 8));
        if attrs.is_null() {
            break 'found;
        }
        active = 1;
        if rd::<u8>(attrs, 0x17) & 2 != 0 {
            active = 0;
            for s in 0..2isize {
                let e = g::<u8>(0x8D5DF8).byte_add(s * 0x30);
                let q = k(0x3E80_0000);
                let f = |o: isize| rd::<f32>(e, o);
                let z = x(f(-0x18)).fadd_m(f(-0xC)).fadd_m(f(0xC)).fadd_m(f(0)).fmul_m(q);
                g::<u8>(0xB6F998).write(0);
                let y = x(f(-0x1C)).fadd_m(f(-0x10)).fadd_m(f(8)).fadd_m(f(-4)).fmul_m(q);
                let xx = x(f(-0x20)).fadd_m(f(-0x14)).fadd_m(f(-8)).fadd_m(f(4)).fmul_m(q);
                let mut c: [f32; 3] = [xx.to_f32(), y.to_f32(), z.to_f32()];
                if camera_is_sphere_visible_m(g(THE_CAMERA), lp(&mut c), 0x4100_0000, 0xB6FA74) != 0
                    || (g::<u8>(0xB6F998).read() != 0 && camera_is_sphere_visible_m(g(THE_CAMERA), lp(&mut c), 0x4100_0000, 0xB6FABC) != 0)
                {
                    active = 1;
                }
            }
            if active == 0 {
                break 'found;
            }
        }
        g::<u32>(M_V).write(rd::<u32>(attrs, 0x10));
        let s = |o: isize| X87::from_i32(i32::from(rd::<i8>(attrs, o))).fmul_m(k(0x3C23_D70A));
        let nz = s(0x16);
        let ny = s(0x15);
        let nx = s(0x14);
        g::<f32>(M_NORMAL).write(nx.to_f32());
        g::<f32>(M_NORMAL + 4).write(ny.to_f32());
        g::<f32>(M_NORMAL + 8).write(nz.to_f32());
        g::<i32>(M_FLAGS).write(i32::from(rd::<i8>(attrs, 0x17)));
        let az = if jp(nz, X87::zero(), 5) { nz } else { nz.fchs() };
        g::<u32>(M_TYPE).write(2);
        if az.fcom(x(k(0x3F33_3333))).test_any(0x41) {
            g::<u32>(M_TYPE).write(1);
        }
        mirrors_create_buffer();
        return deal(active);
    }
    g::<u32>(M_TYPE).write(0);
    destroy_mirror_buffers();
    g::<u32>(M_TYPE).write(0);
    g::<u32>(M_FLAGS).write(0);
    deal(active)
}

/// The tail of `BeforeConstructRenderList` (0x727066).
fn deal(active: u32) {
    let n = |o: usize| g::<u32>(M_NORMAL + o).read();
    if g::<u8>(M_FLAGS).read() & 2 == 0 && g::<u8>(0xC7C72A).read() == 0 {
        camera_deal_with_mirror(g(THE_CAMERA), active, n(0), n(4), n(8), g::<u32>(M_V).read(), GPtr::NULL);
        return;
    }
    let mut m = [0u32; 0x12];
    mirrors_build_camera_matrix_for_screens(lp(&mut m));
    camera_deal_with_mirror(g(THE_CAMERA), active, n(0), n(4), n(8), g::<u32>(M_V).read(), lp(&mut m));
    matrix_dtor(lp(&mut m));
}

/// `CMirrors::BeforeMainRender()` (0x727140): with a mirror type, the scene camera renders into the
/// mirror buffers (frame +0x60 / z +0x64 swapped in), `TheCamera.SetCameraUpForMirror()`, clear to
/// (0,0,0,255) with mode 7 (high-quality stencil) or 3; inside `RsCameraBeginUpdate`:
/// `bRenderingReflection` (0xC7C728) set, `DefinedState()`, `RenderScene()`,
/// `RenderWeaponPedsForPC()`, 0x53E8D0 on 0xC88224, flag cleared, `RwCameraEndUpdate`, the camera's
/// buffers restored and `TheCamera.RestoreCameraAfterMirror()` — the restore is skipped when the
/// begin fails (original behaviour).
#[hook(addr = 0x727140, conv = cdecl, name = "CMirrors::BeforeMainRender", expect = "51 A1 24 C7 C7 00",
       unverified = "mvpd-render-2", route = "L3-only", hazards = "none",
       recs = "grove-walk", note = "renders the whole scene into the mirror buffer (nested RenderScene)")]
pub fn mirrors_before_main_render() {
    if g::<u32>(M_TYPE).read() == 0 {
        return;
    }
    let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
    let fb = rd::<u32>(cam, 0x60);
    let zb = rd::<u32>(cam, 0x64);
    wr::<u32>(cam, 0x60, g::<u32>(M_BUFFER).read());
    wr::<u32>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x64, g::<u32>(M_ZBUFFER).read());
    camera_set_up_for_mirror(g(THE_CAMERA));
    let mut col: [u8; 4] = [0, 0, 0, 0xFF];
    let mode = if stencil_graphics_high_quality() != 0 { 7 } else { 3 };
    rw_camera_clear(g::<GPtr<u8>>(SCENE_CAMERA).read(), lp(&mut col), mode);
    if rs_camera_begin_update(g::<GPtr<u8>>(SCENE_CAMERA).read()) == 0 {
        return;
    }
    g::<u8>(0xC7C728).write(1);
    defined_state();
    render_scene();
    render_weapon_peds_for_pc();
    fun_53e8d0(g(0xC88224));
    g::<u8>(0xC7C728).write(0);
    rw_camera_end_update(g::<GPtr<u8>>(SCENE_CAMERA).read());
    wr::<u32>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x60, fb);
    wr::<u32>(g::<GPtr<u8>>(SCENE_CAMERA).read(), 0x64, zb);
    camera_restore_after_mirror(g(THE_CAMERA));
}

orig! {
    /// `CCullZones::CamNoRain()` (0x72DDB0, bool in AL)
    fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0, bool in AL)
    fn player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CCamera::GetLookingLRBFirstPerson()` (0x50AE60, bool in AL)
    fn camera_looking_lrb_first_person(this: GPtr<u8>) -> u8 = thiscall 0x50AE60 => unported;
    /// `CVehicle::CarHasRoof()` (0x6D25D0, bool in AL)
    fn car_has_roof(v: GPtr<u8>) -> u8 = thiscall 0x6D25D0 => unported;
    /// `CWeather::AddSandStormParticles()` (0x72A820)
    fn add_sand_storm_particles() = cdecl 0x72A820 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, angle, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: u32, gg: u32, b: u32, a: u32, size: u32, angle: u32, life: u32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float time, FxPrtMult_c*, float rot, float bright,
    /// float camDist, uint8 local)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32, bright: u32, cam: u32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, 6 bools, CStoredCollPoly*)` (0x5674E0)
    #[allow(clippy::too_many_arguments)]
    fn process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

/// `CWeather::Rain` (f32)
const RAIN: usize = 0xC81324;

/// `CWeather::AddRain()` (0x72A9A0). Nothing under a no-rain cull zone (camera or player), with
/// `[0xC8132C] > 0`, inside an interior (current area or the player ped's area), with the camera above
/// 900, or when looking left/right/back in first person from a car with a roof. The rain flag
/// (0xC81328) is set with a 800-frame hold counter (0x8D5EAC) while `Rain > 0`, then counts down. A
/// wind (0xC812F0) above 1.01 adds sand particles (unless a no-rain zone or `[0xC8132C] > 0`). With `Rain >
/// 0.1` or a non-zero splash level (0xC81410): `ftol(Rain * 5)` drop columns, each at a random
/// angle/radius around the camera (radius up to `max(Rain * 10, 40) / 2`) probed with
/// `ProcessVerticalLine` from z 40 down 40; on a hit `15 - ftol(Rain * -2)` splash particles at the
/// ground (+0.1) jittered by ±15. The splash level eases towards `Rain * 0.2` by 0.0025, is clamped
/// to 0..1 (times 1.0) and drives the alpha of one drizzle particle near the camera (front * 10,
/// jittered by ±20 / ±20 / 7 - 2) moving with the wind (0xC813E0) * 15.
#[hook(addr = 0x72A9A0, conv = cdecl, name = "CWeather::AddRain", expect = "81 EC 84 00 00 00",
       unverified = "mvpd-render-2", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "FSIN/FCOS; particles go into the Fx systems' heap; rand sites 0x72ABD5, 0x72ABDE, 0x72ACDD, 0x72AD04, 0x72AD2B, 0x72AE79, 0x72AEA0, 0x72AEC7")]
pub fn add_rain() {
    if cam_no_rain() != 0 || player_no_rain() != 0 {
        return;
    }
    if !x(g::<f32>(0xC8132C).read()).fcom(X87::zero()).test_any(0x41) {
        return;
    }
    if g::<i32>(0xB72914).read() != 0 {
        return;
    }
    if !find_player_ped(-1).is_null() && rd::<u8>(find_player_ped(-1), 0x2F) != 0 {
        return;
    }
    let cam_pos = || {
        let cm = g::<GPtr<u8>>(0xB6F03C).read();
        if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) }
    };
    if !x(rd::<f32>(cam_pos(), 8)).fcom(x(k(0x4461_0000))).test_any(0x41) {
        return;
    }
    if camera_looking_lrb_first_person(g(THE_CAMERA)) != 0 {
        let v = find_player_vehicle(-1, 0);
        if !v.is_null() && car_has_roof(v) != 0 {
            return;
        }
    }
    let flag = g::<u32>(0xC81328);
    let hold = g::<i32>(0x8D5EAC);
    if !x(g::<f32>(RAIN).read()).fcom(X87::zero()).test_any(0x41) {
        flag.write(1);
        hold.write(800);
    } else if flag.read() != 0 {
        if hold.read() > 0 {
            hold.write(hold.read() - 1);
        } else {
            flag.write(0);
            hold.write(800);
        }
    }
    if !x(g::<f32>(0xC812F0).read()).fcom(x(k(0x3F81_47AE))).test_any(0x41)
        && cam_no_rain() == 0
        && player_no_rain() == 0
        && !x(g::<f32>(0xC8132C).read()).fcom(X87::zero()).test_parity(0x41)
    {
        add_sand_storm_particles();
    }
    if x(g::<f32>(RAIN).read()).fcom(x(k(0x3DCC_CCCD))).test_any(0x41) && !jp(x(g::<f32>(0xC81410).read()), X87::zero(), 0x44) {
        return;
    }
    let mut cols = X87::from_f32(g::<f32>(RAIN).read()).fmul_m(k(0x40A0_0000)).ftol() as u32 as i32;
    let mut r = x(g::<f32>(RAIN).read()).fmul_m(k(0x4120_0000));
    if !x(k(0x4220_0000)).fcom(r).test_any(0x41) {
        r = x(k(0x4220_0000));
    }
    let splashes = 15 - (x(g::<f32>(RAIN).read()).fmul_m(k(0xC000_0000)).ftol() as u32 as i32);
    if cols > 0 {
        let dist = r.fmul_m(k(0x3F00_0000)).to_f32().to_bits();
        let mut prt = [0u8; 0x40];
        let mut colp = [0u8; 0x2C];
        let mut ent = 0u32;
        while cols != 0 {
            fx_prt_mult_ctor(lp(&mut prt), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3E80_0000, 0x3CA3_D70A, 0, 0x3CF5_C28F);
            let rr = X87::from_f64_exact(random_float_in_range(0, dist));
            let mut vel = [0f32; 3];
            let rad = rr.to_f32();
            let a = rand_at(0x72_ABD5);
            let angle = if a & 1 != 0 {
                X87::from_i32(rand_at(0x72_ABDE) & 0xFF).fmul_m(k(0x3CC8_F5C3))
            } else {
                X87::from_i32((a & 0xFF) - 0x80).fmul_m(k(0x3BCC_CCCD)).fadd_m(g::<f32>(0xB6F178).read())
            };
            let px = angle.fsin().fmul_m(rad).fadd_m(rd::<f32>(cam_pos(), 0)).to_f32();
            let py = angle.fcos().fmul_m(rad).fadd_m(rd::<f32>(cam_pos(), 4)).to_f32();
            let mut p: [f32; 3] = [px, py, 40.0];
            let hit = process_vertical_line(lp(&mut p), 0xC220_0000, lp(&mut colp), lp(&mut ent), 1, 0, 0, 0, 1, 0, 0);
            if hit != 0 {
                let z = x(f32::from_bits(u32::from_le_bytes([colp[8], colp[9], colp[10], colp[11]]))).fadd_m(k(0x3DCC_CCCD)).to_f32();
                let mut n = splashes;
                if n > 0 {
                    loop {
                        let mut q: [f32; 3] = [px, py, z];
                        let jx = X87::from_i32(rand_at(0x72_ACDD)).fmul_m(k(0x3800_0100)).fmul_m(k(0x41F0_0000)).fadd_m(q[0]).fsub_m(k(0x4170_0000));
                        q[0] = jx.to_f32();
                        let jy = X87::from_i32(rand_at(0x72_AD04)).fmul_m(k(0x3800_0100)).fmul_m(k(0x41F0_0000)).fadd_m(q[1]).fsub_m(k(0x4170_0000));
                        q[1] = jy.to_f32();
                        rand_at(0x72_AD2B);
                        fx_add_particle(g::<GPtr<u8>>(0xA9AE30).read(), lp(&mut q), lp(&mut vel), 0, lp(&mut prt), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
                        n -= 1;
                        if n == 0 {
                            break;
                        }
                    }
                }
            }
            cols -= 1;
        }
    }
    let mut v = x(g::<f32>(0xC81410).read());
    let t = x(g::<f32>(RAIN).read()).fmul_m(k(0x3E4C_CCCD));
    if !jp(v, t, 5) {
        v = v.fadd_m(k(0x3B23_D70A));
    }
    if !v.fcom(t).test_any(0x41) {
        v = v.fsub_m(k(0x3B23_D70A));
    }
    if !X87::zero().fcom(v).test_any(0x41) {
        v = X87::zero();
    }
    let w = v.fmul_m(g::<f32>(0x8D5FF0).read());
    if jp(X87::one(), w, 5) {
        g::<f32>(0xC81410).write(w.to_f32());
    } else {
        g::<u32>(0xC81410).write(0x3F80_0000);
    }
    let mut prt = [0u8; 0x40];
    fx_prt_mult_ctor(lp(&mut prt), 0x3F66_6666, 0x3F66_6666, 0x3F80_0000, g::<u32>(0xC81410).read(), 0x3F80_0000, 0, 0x3E4C_CCCD);
    let c = cam_pos();
    let mut p: [f32; 3] = [
        x(g::<f32>(0xB6F9AC).read()).fmul_m(k(0x4120_0000)).fadd_m(rd::<f32>(c, 0)).to_f32(),
        x(g::<f32>(0xB6F9B0).read()).fmul_m(k(0x4120_0000)).fadd_m(rd::<f32>(c, 4)).to_f32(),
        rd::<f32>(c, 8),
    ];
    p[0] = X87::from_i32(rand_at(0x72_AE79)).fmul_m(k(0x3800_0100)).fmul_m(k(0x4220_0000)).fadd_m(p[0]).fsub_m(k(0x41A0_0000)).to_f32();
    p[1] = X87::from_i32(rand_at(0x72_AEA0)).fmul_m(k(0x3800_0100)).fmul_m(k(0x4220_0000)).fadd_m(p[1]).fsub_m(k(0x41A0_0000)).to_f32();
    p[2] = X87::from_i32(rand_at(0x72_AEC7)).fmul_m(k(0x3800_0100)).fmul_m(k(0x40E0_0000)).fadd_m(p[2]).fsub_m(k(0x4000_0000)).to_f32();
    let mut wv: [f32; 3] = [
        x(g::<f32>(0xC813E0).read()).fmul_m(k(0x4170_0000)).to_f32(),
        x(g::<f32>(0xC813E4).read()).fmul_m(k(0x4170_0000)).to_f32(),
        x(g::<f32>(0xC813E8).read()).fmul_m(k(0x4170_0000)).to_f32(),
    ];
    fx_add_particle(g::<GPtr<u8>>(0xA9AE1C).read(), lp(&mut p), lp(&mut wv), 0, lp(&mut prt), 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_RAIN,
    HOOK_CREATE_NUMBER_OF_BIRDS,
    HOOK_BIRDS_UPDATE,
    HOOK_MIRRORS_BEFORE_CONSTRUCT_RENDER_LIST,
    HOOK_MIRRORS_BEFORE_MAIN_RENDER,
];
