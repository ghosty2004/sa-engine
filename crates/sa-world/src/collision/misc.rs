//! Small `CCollision` / `CTempColModels` helpers around the camera and the temporary col models.
//! Ported from the 1.0 US bytes; gta-reversed `Collision.cpp`/`TempColModels.cpp` as reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::{GBool, GPtr};
use sa_core::{hook, orig};

sa_core::globals! {
    list GLOBALS;
    /// `CCollision::bCamCollideWithPeds`
    pub static CAM_COLLIDE_WITH_PEDS @ 0x8A5B17: GBool = GBool::TRUE;
    /// `gnTop` (camera collision sector rect, inclusive)
    pub static GN_TOP @ 0x965590: i32 = 0;
    /// `gnRight`
    pub static GN_RIGHT @ 0x965594: i32 = 0;
    /// `gnBottom`
    pub static GN_BOTTOM @ 0x965598: i32 = 0;
    /// `gnLeft`
    pub static GN_LEFT @ 0x96559C: i32 = 0;
    /// `gpMadeInvisibleEntities[10]`
    pub static MADE_INVISIBLE_ENTITIES @ 0x9655A0: [GPtr<u8>; 10] = [GPtr::NULL; 10];
    /// `gNumEntitiesSetInvisible`
    pub static NUM_ENTITIES_SET_INVISIBLE @ 0x9655DC: i32 = 0;
}

orig! {
    /// `CCollision::CheckCameraCollisionPeds(int32 x, int32 y, const CVector& pos, const CVector& dir,
    /// float& t)` (0x415320)
    pub fn check_camera_collision_peds(x: i32, y: i32, pos: GPtr<f32>, dir: GPtr<f32>, t: GPtr<f32>) -> u8
        = cdecl 0x415320 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571A00 => unported;
    /// `CColModel::RemoveCollisionVolumes` (0x40F9E0)
    pub fn col_model_remove_collision_volumes(cm: GPtr<u8>) = thiscall 0x40F9E0 => col_model_rcv_port;
}

pub fn col_model_rcv_port(cm: GPtr<u8>) {
    crate::collision::data::col_model_remove_collision_volumes(cm.cast())
}

/// `CCollision::CheckPeds(const CVector& pos, const CVector& dir, float& t)` (stub 0x4154A0 →
/// 0x1567650, AL): false when `bCamCollideWithPeds` is clear; otherwise `CheckCameraCollisionPeds` for
/// every sector `x` in `gnLeft..=gnRight`, `y` in `gnBottom..=gnTop` (rows outer, bounds re-read
/// once), true when any call returns true.
#[hook(addr = 0x4154A0, conv = cdecl, name = "CCollision::CheckPeds", expect = "E9 AB 21 15 01")]
pub fn check_peds(pos: GPtr<f32>, dir: GPtr<f32>, t: GPtr<f32>) -> u8 {
    if !CAM_COLLIDE_WITH_PEDS.get().get() {
        return 0;
    }
    let (right, left, bottom, top) = (GN_RIGHT.get(), GN_LEFT.get(), GN_BOTTOM.get(), GN_TOP.get());
    let mut found = 0u8;
    let mut y = bottom;
    while y <= top {
        let mut x = left;
        while x <= right {
            if check_camera_collision_peds(x, y, pos, dir, t) != 0 {
                found = 1;
            }
            x += 1;
        }
        y += 1;
    }
    found
}

/// `CCollision::ResetMadeInvisibleObjects` (stub 0x415540 → 0x156D280): every non-null entry of the
/// first `gNumEntitiesSetInvisible` (re-read each step) gets flag 0x80 (+0x1C), its reference cleaned
/// up and the slot cleared; then the count is 0.
#[hook(addr = 0x415540, conv = cdecl, name = "CCollision::ResetMadeInvisibleObjects", expect = "E9 3B 7D 15 01")]
pub fn reset_made_invisible_objects() {
    let arr = MADE_INVISIBLE_ENTITIES.ptr().cast::<GPtr<u8>>();
    let mut i = 0;
    while i < NUM_ENTITIES_SET_INVISIBLE.get() {
        let slot = arr.wrapping_index(i);
        let e = slot.read();
        if !e.is_null() {
            let f = e.byte_add(0x1C).cast::<u32>();
            f.write(f.read() | 0x80);
            let e = slot.read();
            if !e.is_null() {
                clean_up_old_reference(e, slot);
            }
            slot.write(GPtr::NULL);
        }
        i += 1;
    }
    NUM_ENTITIES_SET_INVISIBLE.set(0);
}

/// `CTempColModels::Shutdown` (stub 0x41B360 → 0x156CBC0): `RemoveCollisionVolumes` of the twelve
/// single temp models (0x968A00, 0x968DF0, ... 0x968FD0, as listed) and then of the 20 models of the
/// array at 0x968A30.
#[hook(addr = 0x41B360, conv = cdecl, name = "CTempColModels::Shutdown", expect = "E9 5B 18 15 01")]
pub fn temp_col_models_shutdown() {
    const SINGLE: [usize; 12] = [
        0x968A00, 0x968DF0, 0x968E20, 0x968E50, 0x968E80, 0x968EB0, 0x968EE0, 0x968F10, 0x968F40, 0x968F70,
        0x968FA0, 0x968FD0,
    ];
    for a in SINGLE {
        col_model_remove_collision_volumes(temp_model(a));
    }
    for i in 0..20 {
        col_model_remove_collision_volumes(temp_model(0x968A30 + 0x30 * i));
    }
}

/// A `CTempColModels` static col model (fixed address in phase A; phases B/C need the module's own
/// storage, which comes with `CTempColModels::Initialise`).
fn temp_model(addr: usize) -> GPtr<u8> {
    GPtr::from_addr(addr)
}

orig! {
    /// `CSphere::Set(float radius, const CVector& center)` (stub 0x40FCF0, `ret 8`)
    pub fn sphere_set_call(s: GPtr<u8>, radius: u32, center: GPtr<u32>) = thiscall 0x40FCF0 => sphere_set;
    /// `CBox::Set(const CVector& min, const CVector& max)` (stub 0x40EDE0, `ret 8`)
    pub fn box_set_call(b: GPtr<u8>, min: GPtr<u32>, max: GPtr<u32>) = thiscall 0x40EDE0 => box_set;
    /// `CColSphere::Set(float radius, const CVector& center, uint8 material, uint8 piece, uint8 lighting)`
    /// (0x40FD10, `ret 0x14`; the bytes pushed as dwords)
    pub fn col_sphere_set_call(s: GPtr<u8>, radius: u32, center: GPtr<u32>, material: u32, piece: u32, light: u32)
        = thiscall 0x40FD10 => col_sphere_set;
    /// `CColLine::Set(const CVector& start, const CVector& end)` (stub 0x40EF10, `ret 8`)
    pub fn col_line_set_call(l: GPtr<u8>, start: GPtr<u32>, end: GPtr<u32>) = thiscall 0x40EF10 => col_line_set;
    /// `CColModel::AllocateData(numSpheres, numBoxes, numLines, numVertices, numTriangles, disks)` (0x40F870,
    /// `ret 0x18`; the counts pushed as dwords)
    pub fn col_model_allocate_data_call(cm: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32)
        = thiscall 0x40F870 => unported;
}

fn copy3(dst: GPtr<u8>, src: GPtr<u32>) {
    for i in 0..3 {
        dst.byte_add(4 * i).cast::<u32>().write(src.wrapping_index(i as i32).read());
    }
}

/// `CSphere::Set(float radius, const CVector& center)` (stub 0x40FCF0 → 0x1567080, thiscall): +0xC = radius,
/// +0..+8 = center (dword copies).
#[hook(addr = 0x40FCF0, conv = thiscall, name = "CSphere::Set", expect = "E9 8B 73 15 01",
       unverified = "std4-data-2", route = "L1-pure", hazards = "none", recs = "boot")]
pub fn sphere_set(this: GPtr<u8>, radius: u32, center: GPtr<u32>) {
    this.byte_add(0xC).cast::<u32>().write(radius);
    copy3(this, center);
}

/// `CBox::Set(const CVector& min, const CVector& max)` (stub 0x40EDE0 → 0x1563AF0, thiscall): +0 = min,
/// +0xC = max (dword copies).
#[hook(addr = 0x40EDE0, conv = thiscall, name = "CBox::Set", expect = "E9 0B 4D 15 01",
       unverified = "std4-data-2", route = "L1-pure", hazards = "none", recs = "boot")]
pub fn box_set(this: GPtr<u8>, min: GPtr<u32>, max: GPtr<u32>) {
    copy3(this, min);
    copy3(this.byte_add(0xC), max);
}

/// `CBox::Recalc()` (0x40EE20, thiscall): per axis, a min (+0, +4, +8) above its max (+0xC, +0x10, +0x14) swaps the
/// two (the max gets the min through `fld`/`fstp`, the min the max's raw dword); NaN compares leave the pair.
#[hook(addr = 0x40EE20, conv = thiscall, name = "CBox::Recalc", expect = "D9 01 D8 59 0C DF E0",
       unverified = "std9-coll-1", route = "L1-pure", hazards = "none", recs = "grove-walk")]
pub fn box_recalc(this: GPtr<u8>) {
    for o in [0isize, 4, 8] {
        let lo = this.byte_add(o).cast::<f32>();
        let hi = this.byte_add(o + 0xC).cast::<f32>();
        let a = sa_math::x87::X87::from_f32(lo.read());
        if !a.fcom(sa_math::x87::X87::from_f32(hi.read())).test_any(0x41) {
            let raw = hi.cast::<u32>().read();
            hi.write(a.to_f32());
            lo.cast::<u32>().write(raw);
        }
    }
}

/// `CColSphere::Set(radius, center, material, piece, lighting)` (0x40FD10, thiscall): the sphere, then the
/// three bytes at +0x10, +0x11, +0x12.
#[hook(addr = 0x40FD10, conv = thiscall, name = "CColSphere::Set", expect = "8B 44 24 04 8B 54 24 08",
       unverified = "std4-data-2", route = "L1-pure", hazards = "none", recs = "boot")]
pub fn col_sphere_set(this: GPtr<u8>, radius: u32, center: GPtr<u32>, material: u32, piece: u32, light: u32) {
    this.byte_add(0xC).cast::<u32>().write(radius);
    copy3(this, center);
    this.byte_add(0x10).write(material as u8);
    this.byte_add(0x11).write(piece as u8);
    this.byte_add(0x12).write(light as u8);
}

/// `CColLine::Set(const CVector& start, const CVector& end)` (stub 0x40EF10 → 0x156D710, thiscall): +0 =
/// start, +0x10 = end (dword copies; the w components are left).
#[hook(addr = 0x40EF10, conv = thiscall, name = "CColLine::Set", expect = "E9 FB E7 15 01",
       unverified = "std4-data-2", route = "L1-pure", hazards = "none", recs = "boot")]
pub fn col_line_set(this: GPtr<u8>, start: GPtr<u32>, end: GPtr<u32>) {
    copy3(this, start);
    copy3(this.byte_add(0x10), end);
}

/// `CTempColModels::Initialise` (0x5BA8C0, cdecl). Straight-line code: the bounding spheres and boxes of the
/// temporary col models (0x968A00 .. 0x968FD0 and the 20 models of 0x968A30), their level bytes, then
/// `AllocateData` of the multi-volume models followed by their spheres (`CColSphere::Set` on
/// `colData->spheres`) and lines, and the last model's box/sphere set directly. Translated mechanically from
/// the 1.0 US bytes (every constant, store and call in the original order; the CVector arguments are the
/// original's stack locals, rebuilt per call).
///
/// Not registered: 0x5BA8C0 is hooked by sa-world mvpe_tempcol.rs (duplicate port left unregistered when STD-5 merged both).
pub fn temp_col_models_initialise() {
    fn at(a: usize) -> GPtr<u8> {
        GPtr::from_addr(a)
    }
    fn v3(v: [u32; 3]) -> [u32; 3] {
        v
    }
    fn rd32(a: u32) -> u32 {
        GPtr::<u32>::from_addr(a as usize).read()
    }
    fn wr32(a: u32, v: u32) {
        GPtr::<u32>::from_addr(a as usize).write(v);
    }
    fn wr8(a: u32, v: u32) {
        GPtr::<u8>::from_addr(a as usize).write(v as u8);
    }
    fn p(v: &[u32; 3]) -> GPtr<u32> {
        GPtr::from_raw(v.as_ptr().cast_mut())
    }
    fn sphere_set(s: GPtr<u8>, r: u32, c: [u32; 3]) {
        sphere_set_call(s, r, p(&c));
    }
    fn box_set(b: GPtr<u8>, min: [u32; 3], max: [u32; 3]) {
        box_set_call(b, p(&min), p(&max));
    }
    fn col_sphere_set(s: GPtr<u8>, r: u32, c: [u32; 3], m: u32, pc: u32, l: u32) {
        col_sphere_set_call(s, r, p(&c), m, pc, l);
    }
    fn col_line_set(l: GPtr<u8>, a: [u32; 3], b: [u32; 3]) {
        col_line_set_call(l, p(&a), p(&b));
    }
    fn col_model_allocate_data(cm: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) {
        col_model_allocate_data_call(cm, a, b, c, d, e, f);
    }
    sphere_set(at(0x968a18), 0x40000000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968a00), v3([0xc0000000, 0xc0000000, 0xc0000000]), v3([0x40000000, 0x40000000, 0x40000000]));
    wr8(0x968a28, 0x0);
    sphere_set(at(0x968a48), 0x40000000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968a30), v3([0xc0000000, 0xc0000000, 0xc0000000]), v3([0x40000000, 0x40000000, 0x40000000]));
    wr8(0x968a58, 0x0);
    sphere_set(at(0x968a78), 0x40000000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968a60), v3([0xc0000000, 0xc0000000, 0xc0000000]), v3([0x40000000, 0x40000000, 0x40000000]));
    wr8(0x968a88, 0x0);
    sphere_set(at(0x968aa8), 0x40000000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968a90), v3([0xc0000000, 0xc0000000, 0xc0000000]), v3([0x40000000, 0x40000000, 0x40000000]));
    wr8(0x968ab8, 0x0);
    sphere_set(at(0x968ad8), 0x40000000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968ac0), v3([0xc0000000, 0xc0000000, 0xc0000000]), v3([0x40000000, 0x40000000, 0x40000000]));
    wr8(0x968ae8, 0x0);
    sphere_set(at(0x968b08), 0x40000000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968af0), v3([0xc0000000, 0xc0000000, 0xc0000000]), v3([0x40000000, 0x40000000, 0x40000000]));
    wr8(0x968b18, 0x0);
    col_model_allocate_data(at(0x968df0), 0x3, 0x0, 0x2, 0x0, 0x0, 0x0);
    let t1 = rd32(0x968e1c);
    let t2 = rd32(t1.wrapping_add(8));
    col_sphere_set(at(t2 as usize), 0x3eb33333, v3([0x0, 0x0, 0xbe4ccccc]), 0x3e, 0x0, 0xff);
    let t3 = rd32(t1.wrapping_add(8));
    let t4 = t3.wrapping_add(0x14);
    col_sphere_set(at(t4 as usize), 0x3eb33333, v3([0x0, 0x0, 0x3e4cccd0]), 0x3e, 0x1, 0xff);
    let t5 = rd32(t1.wrapping_add(8));
    let t6 = t5.wrapping_add(0x28);
    col_sphere_set(at(t6 as usize), 0x3eb33333, v3([0x0, 0x0, 0x3f19999a]), 0x3e, 0x2, 0xff);
    let t7 = rd32(t1.wrapping_add(0x10));
    col_line_set(at(t7 as usize), v3([0x0, 0x0, 0x0]), v3([0x0, 0x0, 0xbf800000]));
    let t8 = rd32(t1.wrapping_add(0x10));
    let t9 = t8.wrapping_add(0x20);
    col_line_set(at(t9 as usize), v3([0x0, 0x0, 0x0]), v3([0x0, 0x0, 0x3f800000]));
    wr8(t1.wrapping_add(6), 0x0);
    sphere_set(at(0x968e08), 0x3f800000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968df0), v3([0xbeb33333, 0xbeb33333, 0xbf800000]), v3([0x3eb33333, 0x3eb33333, 0x3f733333]));
    wr8(0x968e18, 0x0);
    col_model_allocate_data(at(0x968e20), 0x9, 0x0, 0x2, 0x0, 0x0, 0x0);
    let t10 = rd32(0x968e4c);
    let t11 = rd32(t10.wrapping_add(8));
    col_sphere_set(at(t11 as usize), 0x3eb33333, v3([0x0, 0x0, 0xbe4ccccc]), 0x3e, 0x0, 0xff);
    let t12 = rd32(t10.wrapping_add(8));
    let t13 = t12.wrapping_add(0x14);
    col_sphere_set(at(t13 as usize), 0x3eb33333, v3([0x0, 0x0, 0x3e4cccd0]), 0x3e, 0x1, 0xff);
    let t14 = rd32(t10.wrapping_add(8));
    let t15 = t14.wrapping_add(0x28);
    col_sphere_set(at(t15 as usize), 0x3eb33333, v3([0x0, 0x0, 0x3f19999a]), 0x3e, 0x2, 0xff);
    let t16 = rd32(t10.wrapping_add(8));
    let t17 = t16.wrapping_add(0x3c);
    col_sphere_set(at(t17 as usize), 0x3e051eb8, v3([0x3e3851ec, 0x3eb33333, 0x3f400000]), 0x3e, 0x6, 0xff);
    let t18 = rd32(t10.wrapping_add(8));
    let t19 = t18.wrapping_add(0x50);
    col_sphere_set(at(t19 as usize), 0x3e051eb8, v3([0x3e3851ec, 0x3f000000, 0x3f400000]), 0x3e, 0x6, 0xff);
    let t20 = rd32(t10.wrapping_add(8));
    let t21 = t20.wrapping_add(0x64);
    col_sphere_set(at(t21 as usize), 0x3e051eb8, v3([0xbe3851ec, 0x3eb33333, 0x3f400000]), 0x3e, 0x5, 0xff);
    let t22 = rd32(t10.wrapping_add(8));
    let t23 = t22.wrapping_add(0x78);
    col_sphere_set(at(t23 as usize), 0x3e051eb8, v3([0xbe3851ec, 0x3f000000, 0x3f400000]), 0x3e, 0x5, 0xff);
    let t24 = rd32(t10.wrapping_add(8));
    let t25 = t24.wrapping_add(0x8c);
    col_sphere_set(at(t25 as usize), 0x3e4ccccd, v3([0x0, 0x3f000000, 0x3ee66668]), 0x3e, 0x8, 0xff);
    let t26 = rd32(t10.wrapping_add(8));
    let t27 = t26.wrapping_add(0xa0);
    col_sphere_set(at(t27 as usize), 0x3eb33333, v3([0x0, 0x3f0ccccd, 0x3e4cccd0]), 0x3e, 0x4, 0xff);
    let t28 = rd32(t10.wrapping_add(0x10));
    col_line_set(at(t28 as usize), v3([0x0, 0x0, 0x0]), v3([0x0, 0x0, 0xbf800000]));
    let t29 = rd32(t10.wrapping_add(0x10));
    let t30 = t29.wrapping_add(0x20);
    col_line_set(at(t30 as usize), v3([0x0, 0x0, 0x0]), v3([0x0, 0x0, 0x3f800000]));
    wr8(t10.wrapping_add(6), 0x0);
    sphere_set(at(0x968e38), 0x3f800000, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968e20), v3([0xbeb33333, 0xbeb33333, 0xbf800000]), v3([0x3eb33333, 0x3f666666, 0x3f733333]));
    wr8(0x968e48, 0x0);
    col_model_allocate_data(at(0x968e50), 0x3, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t31 = rd32(0x968e7c);
    let t32 = rd32(t31.wrapping_add(8));
    col_sphere_set(at(t32 as usize), 0x3e19999a, v3([0x0, 0xbe800000, 0xbeb33333]), 0x40, 0x0, 0xff);
    let t33 = rd32(t31.wrapping_add(8));
    let t34 = t33.wrapping_add(0x14);
    col_sphere_set(at(t34 as usize), 0x3e19999a, v3([0x0, 0xbf733333, 0xbeb33333]), 0x40, 0x0, 0xff);
    let t35 = rd32(t31.wrapping_add(8));
    let t36 = t35.wrapping_add(0x28);
    col_sphere_set(at(t36 as usize), 0x3e800000, v3([0x0, 0xbf19999a, 0x3e800000]), 0x40, 0x0, 0xff);
    sphere_set(at(0x968e68), 0x3fc00000, v3([0x0, 0xbf19999a, 0x0]));
    box_set(at(0x968e50), v3([0xbe99999a, 0x0, 0xbf19999a]), v3([0x3e99999a, 0xbf99999a, 0x3f19999a]));
    wr8(0x968e78, 0x0);
    col_model_allocate_data(at(0x968e80), 0x4, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t37 = rd32(0x968eac);
    let t38 = rd32(t37.wrapping_add(8));
    col_sphere_set(at(t38 as usize), 0x3e19999a, v3([0x3f59999a, 0xbd4ccccd, 0x0]), 0x40, 0x0, 0xff);
    let t39 = rd32(t37.wrapping_add(8));
    let t40 = t39.wrapping_add(0x14);
    col_sphere_set(at(t40 as usize), 0x3e19999a, v3([0x3ecccccd, 0x3d4ccccd, 0x0]), 0x40, 0x0, 0xff);
    let t41 = rd32(t37.wrapping_add(8));
    let t42 = t41.wrapping_add(0x28);
    col_sphere_set(at(t42 as usize), 0x3e19999a, v3([0xbecccccd, 0x3d4ccccd, 0x0]), 0x40, 0x0, 0xff);
    let t43 = rd32(t37.wrapping_add(8));
    let t44 = t43.wrapping_add(0x3c);
    col_sphere_set(at(t44 as usize), 0x3e19999a, v3([0xbf59999a, 0xbd4ccccd, 0x0]), 0x40, 0x0, 0xff);
    sphere_set(at(0x968e98), 0x400ccccd, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968e80), v3([0xbf99999a, 0xbe99999a, 0xbe4ccccd]), v3([0x3f99999a, 0x3e99999a, 0x3e4ccccd]));
    wr8(0x968ea8, 0x0);
    col_model_allocate_data(at(0x968eb0), 0x4, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t45 = rd32(0x968edc);
    let t46 = rd32(t45.wrapping_add(8));
    col_sphere_set(at(t46 as usize), 0x3e19999a, v3([0x3e19999a, 0x3ee66666, 0x0]), 0x40, 0x0, 0xff);
    let t47 = rd32(t45.wrapping_add(8));
    let t48 = t47.wrapping_add(0x14);
    col_sphere_set(at(t48 as usize), 0x3e19999a, v3([0x3e19999a, 0xbee66666, 0x0]), 0x40, 0x0, 0xff);
    let t49 = rd32(t45.wrapping_add(8));
    let t50 = t49.wrapping_add(0x28);
    col_sphere_set(at(t50 as usize), 0x3e19999a, v3([0xbe19999a, 0xbee66666, 0x0]), 0x40, 0x0, 0xff);
    let t51 = rd32(t45.wrapping_add(8));
    let t52 = t51.wrapping_add(0x3c);
    col_sphere_set(at(t52 as usize), 0x3e19999a, v3([0xbe19999a, 0x3ee66666, 0x0]), 0x40, 0x0, 0xff);
    sphere_set(at(0x968ec8), 0x3fb33333, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968eb0), v3([0xbe99999a, 0xbf19999a, 0xbe19999a]), v3([0x3e99999a, 0x3f19999a, 0x3e19999a]));
    wr8(0x968ed8, 0x0);
    col_model_allocate_data(at(0x968ee0), 0x4, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t53 = rd32(0x968f0c);
    let t54 = rd32(t53.wrapping_add(8));
    col_sphere_set(at(t54 as usize), 0x3e4ccccd, v3([0xbecccccd, 0x3dcccccd, 0x0]), 0x40, 0x0, 0xff);
    let t55 = rd32(t53.wrapping_add(8));
    let t56 = t55.wrapping_add(0x14);
    col_sphere_set(at(t56 as usize), 0x3e4ccccd, v3([0xbecccccd, 0x3f666666, 0x0]), 0x40, 0x0, 0xff);
    let t57 = rd32(t53.wrapping_add(8));
    let t58 = t57.wrapping_add(0x28);
    col_sphere_set(at(t58 as usize), 0x3e4ccccd, v3([0x3ecccccd, 0x3dcccccd, 0x0]), 0x40, 0x0, 0xff);
    let t59 = rd32(t53.wrapping_add(8));
    let t60 = t59.wrapping_add(0x3c);
    col_sphere_set(at(t60 as usize), 0x3e4ccccd, v3([0x3ecccccd, 0x3f666666, 0x0]), 0x40, 0x0, 0xff);
    sphere_set(at(0x968ef8), 0x3fd9999a, v3([0x0, 0x3f000000, 0x0]));
    box_set(at(0x968ee0), v3([0xbf333333, 0xbe4ccccd, 0xbe99999a]), v3([0x3f333333, 0x3f99999a, 0x3e99999a]));
    wr8(0x968f08, 0x0);
    col_model_allocate_data(at(0x968f10), 0x4, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t61 = rd32(0x968f3c);
    let t62 = rd32(t61.wrapping_add(8));
    col_sphere_set(at(t62 as usize), 0x3e4ccccd, v3([0xbecccccd, 0xbdcccccd, 0x0]), 0x40, 0x0, 0xff);
    let t63 = rd32(t61.wrapping_add(8));
    let t64 = t63.wrapping_add(0x14);
    col_sphere_set(at(t64 as usize), 0x3e4ccccd, v3([0xbecccccd, 0xbf19999a, 0x0]), 0x40, 0x0, 0xff);
    let t65 = rd32(t61.wrapping_add(8));
    let t66 = t65.wrapping_add(0x28);
    col_sphere_set(at(t66 as usize), 0x3e4ccccd, v3([0x3ecccccd, 0xbdcccccd, 0x0]), 0x40, 0x0, 0xff);
    let t67 = rd32(t61.wrapping_add(8));
    let t68 = t67.wrapping_add(0x3c);
    col_sphere_set(at(t68 as usize), 0x3e4ccccd, v3([0x3ecccccd, 0xbf19999a, 0x0]), 0x40, 0x0, 0xff);
    sphere_set(at(0x968f28), 0x3fb33333, v3([0x0, 0xbecccccd, 0x0]));
    box_set(at(0x968f10), v3([0xbf333333, 0xbf666666, 0xbe99999a]), v3([0x3f333333, 0x3e4ccccd, 0x3e99999a]));
    wr8(0x968f38, 0x0);
    col_model_allocate_data(at(0x968f40), 0x2, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t69 = rd32(0x968f6c);
    let t70 = rd32(t69.wrapping_add(8));
    col_sphere_set(at(t70 as usize), 0x3eb33333, v3([0xbe99999a, 0x0, 0x0]), 0x3c, 0x0, 0xff);
    let t71 = rd32(t69.wrapping_add(8));
    let t72 = t71.wrapping_add(0x14);
    col_sphere_set(at(t72 as usize), 0x3eb33333, v3([0x3e99999a, 0x0, 0x0]), 0x3c, 0x0, 0xff);
    sphere_set(at(0x968f58), 0x3fb33333, v3([0x0, 0x0, 0x0]));
    box_set(at(0x968f40), v3([0xbf333333, 0xbecccccd, 0xbecccccd]), v3([0x3f333333, 0x3ecccccd, 0x3ecccccd]));
    wr8(0x968f68, 0x0);
    col_model_allocate_data(at(0x968f70), 0x2, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t73 = rd32(0x968f9c);
    let t74 = rd32(t73.wrapping_add(8));
    col_sphere_set(at(t74 as usize), 0x3e4ccccd, v3([0x0, 0x0, 0x0]), 0x3e, 0x0, 0xff);
    let t75 = rd32(t73.wrapping_add(8));
    let t76 = t75.wrapping_add(0x14);
    col_sphere_set(at(t76 as usize), 0x3e4ccccd, v3([0x3f4ccccd, 0x0, 0x0]), 0x3e, 0x0, 0xff);
    sphere_set(at(0x968f88), 0x3f333333, v3([0x3ecccccd, 0x0, 0x0]));
    box_set(at(0x968f70), v3([0xbe99999a, 0xbe99999a, 0xbe99999a]), v3([0x3f8ccccd, 0x3e99999a, 0x3e99999a]));
    wr8(0x968f98, 0x0);
    col_model_allocate_data(at(0x968fa0), 0x2, 0x0, 0x0, 0x0, 0x0, 0x0);
    let t77 = rd32(0x968fcc);
    let t78 = rd32(t77.wrapping_add(8));
    col_sphere_set(at(t78 as usize), 0x3e19999a, v3([0x0, 0x0, 0x0]), 0x3e, 0x0, 0xff);
    let t79 = rd32(t77.wrapping_add(8));
    let t80 = t79.wrapping_add(0x14);
    col_sphere_set(at(t80 as usize), 0x3e19999a, v3([0x3f000000, 0x0, 0x0]), 0x3e, 0x0, 0xff);
    sphere_set(at(0x968fb8), 0x3f000000, v3([0x3e800000, 0x0, 0x0]));
    box_set(at(0x968fa0), v3([0xbe4ccccd, 0xbe4ccccd, 0xbe4ccccd]), v3([0x3f333333, 0x3e4ccccd, 0x3e4ccccd]));
    wr32(0x968fd0, 0xbe800000);
    wr32(0x968fd4, 0xbe800000);
    wr32(0x968fd8, 0xbe800000);
    wr8(0x968fc8, 0x0);
    wr32(0x968ff4, 0x3e800000);
    wr32(0x968fdc, 0x3e800000);
    wr32(0x968fe0, 0x3e800000);
    wr32(0x968fe4, 0x3e800000);
}

/// `CColBox::Set(const CVector& min, const CVector& max, uint8 material, uint8 piece, uint8 lighting)`
/// (stub 0x40EE70, body 0x1567850, `ret 0x14`): min +0, max +0xC, surface bytes +0x18..+0x1A (STD-6).
#[hook(addr = 0x40EE70, conv = thiscall, name = "CColBox::Set", expect = "E9 DB 89 15 01", this = "CColBox",
       unverified = "std6-world-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn col_box_set(this: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>, material: u32, piece: u32, lighting: u32) {
    for i in 0..3isize {
        this.byte_add(i * 4).cast::<u32>().write(min.byte_add(i * 4).cast::<u32>().read());
    }
    for i in 0..3isize {
        this.byte_add(0xC + i * 4).cast::<u32>().write(max.byte_add(i * 4).cast::<u32>().read());
    }
    this.byte_add(0x18).write(material as u8);
    this.byte_add(0x19).write(piece as u8);
    this.byte_add(0x1A).write(lighting as u8);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_COL_BOX_SET,
    HOOK_BOX_RECALC,
    HOOK_CHECK_PEDS,
    HOOK_RESET_MADE_INVISIBLE_OBJECTS,
    HOOK_TEMP_COL_MODELS_SHUTDOWN,
    HOOK_SPHERE_SET,
    HOOK_BOX_SET,
    HOOK_COL_SPHERE_SET,
    HOOK_COL_LINE_SET,
];
