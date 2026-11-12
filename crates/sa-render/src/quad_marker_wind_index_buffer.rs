//! Small render helpers SA:MP executes (batch `samp-render-2`): `CPostEffects::DrawQuad`, `CCheckpoints::DeleteCP`,
//! `CWindModifiers::RegisterOne`, `D3DIndexDataBuffer::Resize` and `RwTexDictionaryFindHashNamedTexture`. Ported
//! from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{k, rd, wr};

orig! {
    /// `RwRenderStateSet(RwRenderState, void*)`: slot +0x20 of `RwEngineInstance` (0xC97B24)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm2DRenderPrimitive(RwPrimitiveType, RwIm2DVertex*, int32)` (0x734E90: `jmp [RwEngineInstance + 0x30]`)
    fn rw_im2d_render_primitive(ty: u32, verts: u32, n: u32) -> u32 = cdecl 0x734E90 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u32> = cdecl 0x82119A => unported;
    /// CRT `operator delete` (0x8214BD)
    fn operator_delete(p: GPtr<u32>) = cdecl 0x8214BD => unported;
    /// `IUnknown::Release` (COM vtable slot 2, stdcall)
    fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
}

fn gp<T: sa_core::mem::GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

/// `CPostEffects::ms_imf` quad vertices (4 `RwIm2DVertex`, 0x1C each) at 0xC401D8 and the near-screen z
/// (`RwIm2DGetNearScreenZ` value) at 0xC40150.
const QUAD: usize = 0xC4_01D8;
const NEAR_Z: usize = 0xC4_0150;

/// `CPostEffects::DrawQuad(float x, float y, float w, float h, uint8 r, uint8 g, uint8 b, uint8 a, RwRaster*)`
/// (0x700EC0, cdecl): `RwRenderStateSet(rwRENDERSTATETEXTURERASTER, raster)` (its two arguments stay on the stack
/// until the end); the four vertices (x, y), (x + w, y), (x, y + h), (x + w, y + h) (sums on the x87 stack, stored
/// as f32), each with z = the near-screen z and the colour `a << 24 | r << 16 | g << 8 | b`;
/// `RwIm2DRenderPrimitive(rwPRIMTYPETRISTRIP, verts, 4)`.
#[hook(addr = 0x700EC0, conv = cdecl, name = "CPostEffects::DrawQuad", expect = "8B 44 24 24 8B 0D 24 7B C9 00",
       unverified = "samp-render-2", route = "L2", hazards = "none", recs = "air-water")]
pub fn post_effects_draw_quad(x: f32, y: f32, w: f32, h: f32, r: u32, g: u32, b: u32, a: u32, raster: u32) {
    rw_render_state_set(1, raster);
    let q = gp::<u8>(QUAD);
    let xw = X87::from_f32(x).fadd_m(w).to_f32();
    let yh = X87::from_f32(y).fadd_m(h).to_f32();
    let z = gp::<u32>(NEAR_Z).read();
    let col = ((((a & 0xFF) << 8 | (r & 0xFF)) << 8 | (g & 0xFF)) << 8) | (b & 0xFF);
    for (i, (vx, vy)) in [(x, y), (xw, y), (x, yh), (xw, yh)].into_iter().enumerate() {
        let v = q.byte_add(0x1C * i as isize);
        wr::<f32>(v, 0, vx);
        wr::<f32>(v, 4, vy);
        wr::<u32>(v, 8, z);
        wr::<u32>(v, 0x10, col);
    }
    rw_im2d_render_primitive(4, QUAD as u32, 4);
}

/// `CCheckpoints::m_aCheckPtArray[32]` (0x38 each) at 0xC7F158: +0 type (word), +2 active flag, +4 id.
const CHECKPOINTS: usize = 0xC7_F158;

/// `CCheckpoints::DeleteCP(uint32 id, uint16 type)` (0x722FC0, cdecl): every active checkpoint with this id and
/// type (16-bit compare) is cleared: active := 0, the type word := 0x101, id := 0.
#[hook(addr = 0x722FC0, conv = cdecl, name = "CCheckpoints::DeleteCP", expect = "66 8B 54 24 08",
       unverified = "samp-render-2", route = "L2", hazards = "none", recs = "world-cheats")]
pub fn checkpoints_delete_cp(id: u32, ty: u32) {
    for i in 0..32 {
        let c = gp::<u8>(CHECKPOINTS + 0x38 * i);
        if rd::<u8>(c, 2) != 0 && rd::<u32>(c, 4) == id && rd::<u16>(c, 0) == ty as u16 {
            wr::<u8>(c, 2, 0);
            wr::<u16>(c, 0, 0x101);
            wr::<u32>(c, 4, 0);
        }
    }
}

/// `CWindModifiers::Number` (0xC81450) and `Array[16]` (0x14 each: position, +0xC and +0x10 the two other
/// arguments) at 0xC81458; `TheCamera`'s matrix pointer 0xB6F03C and placement position 0xB6F02C.
const WIND_NUMBER: usize = 0xC8_1450;
const WIND_ARRAY: usize = 0xC8_1458;

/// `CWindModifiers::RegisterOne(CVector pos, int32 type, float power)` (0x72C8B0, cdecl): with fewer than 16
/// registered, the distance from the camera (`sqrt(dz*dz + dy*dy + dx*dx)` on the x87 stack, `fsqrt`) is compared with
/// 200.0f (0x858A48); only when it is below (ordered) the entry is appended.
#[hook(addr = 0x72C8B0, conv = cdecl, name = "CWindModifiers::RegisterOne", expect = "8B 0D 50 14 C8 00",
       unverified = "samp-render-2", route = "L2", hazards = "x87-transcendental", recs = "air-water",
       note = "FSQRT")]
pub fn wind_modifiers_register_one(px: f32, py: f32, pz: f32, ty: u32, power: u32) {
    let n = gp::<i32>(WIND_NUMBER).read();
    if n >= 0x10 {
        return;
    }
    let m = gp::<u32>(0xB6_F03C).read();
    let cam = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { gp::<u8>(0xB6_F02C) };
    let dx = X87::from_f32(px).fsub_m(rd::<f32>(cam, 0));
    let dy = X87::from_f32(py).fsub_m(rd::<f32>(cam, 4));
    let dz = X87::from_f32(pz).fsub_m(rd::<f32>(cam, 8));
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    if d.fcom(X87::from_f32(k(0x4348_0000))).test_parity(5) {
        return;
    }
    let e = gp::<u8>(WIND_ARRAY + 0x14 * n as usize);
    wr::<f32>(e, 0, px);
    wr::<f32>(e, 4, py);
    wr::<f32>(e, 8, pz);
    wr::<u32>(e, 0xC, ty);
    wr::<u32>(e, 0x10, power);
    gp::<i32>(WIND_NUMBER).write(n + 1);
}

/// `D3DIndexDataBuffer::Resize(int32 size)` (0x730330, thiscall, `ret 4`): unless `size` is the capacity (+8): a
/// new array of `size` pointers (`operator new`); the first `min(count, size)` stored buffers (+0x14 array, +0x10
/// count, re-read per step) are copied; the ones from `size` to `count` are `Release`d; the old array is freed and
/// replaced; capacity := `size` (the count is left as it was).
#[hook(addr = 0x730330, conv = thiscall, name = "D3DIndexDataBuffer::Resize", expect = "53 8B 5C 24 08",
       unverified = "samp-render-2", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "allocates and frees the CRT heap; releases COM index buffers")]
pub fn d3d_index_data_buffer_resize(this: GPtr<u8>, size: i32) {
    if size == rd::<i32>(this, 8) {
        return;
    }
    let arr = operator_new((size as u32).wrapping_mul(4));
    let mut i = 0i32;
    while i < rd::<i32>(this, 0x10) && i < size {
        arr.wrapping_index(i).write(rd::<GPtr<u32>>(this, 0x14).wrapping_index(i).read());
        i += 1;
    }
    let mut j = size;
    while j < rd::<i32>(this, 0x10) {
        let ib = GPtr::<u8>::from_addr(rd::<GPtr<u32>>(this, 0x14).wrapping_index(j).read() as usize);
        com_release(ib);
        j += 1;
    }
    operator_delete(rd::<GPtr<u32>>(this, 0x14));
    wr(this, 0x14, arr);
    wr::<i32>(this, 8, size);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_POST_EFFECTS_DRAW_QUAD,
    HOOK_CHECKPOINTS_DELETE_CP,
    HOOK_WIND_MODIFIERS_REGISTER_ONE,
    HOOK_D3D_INDEX_DATA_BUFFER_RESIZE,
];
