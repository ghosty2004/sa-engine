//! WRITER-MVP-D (batch `mvpd-d3d-1`): the D3D resource recycling of `D3DResourceSystem` (texture and
//! index buffer pools that `RwD3D9` destroys into and creates from) and the small vector helper
//! `CVector::operator*=(float)` (0x40FEF0) the shadow code calls. `TidyUpD3DTextures` (0x7305E0) and
//! `TidyUpD3DIndexBuffers` (0x730740) are ported in sa-core (`game.rs`) by another writer.
//!
//! Ported from the 1.0 US bytes; gta-reversed `D3DResourceSystem.cpp` / `D3DTextureBuffer.cpp` /
//! `D3DIndexDataBuffer.cpp` as reading aids. COM calls go through their vtable slots (stdcall).
//! Layouts: `D3DTextureBuffer` (0x1C: format +0, size +4, levels flag +8, capacity +0xC, counter +0x10,
//! count +0x14, array +0x18), 17 of them at 0xC87C68 (`TextureBuffer`, count at 0xC87C60, round-robin
//! index 0xC87FE0); `D3DIndexDataBuffer` (0x18: +8 capacity, +0xC counter, +0x10 count, +0x14 array),
//! 16 size buckets at 0xC87E48 (cursor 0xC87FE4) and one large-size buffer at 0xC87FC8.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, k, rd, wr, x};

orig! {
    /// `IUnknown::Release` (COM slot 8)
    fn com_release(o: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `IDirect3DIndexBuffer9::GetDesc(D3DINDEXBUFFER_DESC*)` (COM slot 0x34)
    fn ib_get_desc(o: GPtr<u8>, desc: GPtr<u8>) -> u32 = stdcall vslot 0x34 => unported;
    /// `IDirect3DTexture9::GetLevelCount()` (COM slot 0x34)
    fn tex_get_level_count(o: GPtr<u8>) -> u32 = stdcall vslot 0x34 => unported;
    /// `IDirect3DTexture9::GetLevelDesc(level, D3DSURFACE_DESC*)` (COM slot 0x44)
    fn tex_get_level_desc(o: GPtr<u8>, level: u32, desc: GPtr<u8>) -> u32 = stdcall vslot 0x44 => unported;
    /// `IDirect3DDevice9::CreateIndexBuffer(len, usage, fmt, pool, IDirect3DIndexBuffer9**, HANDLE*)` (COM slot 0x6C)
    #[allow(clippy::too_many_arguments)]
    fn dev_create_index_buffer(dev: GPtr<u8>, len: u32, usage: u32, fmt: u32, pool: u32, out: GPtr<u8>, h: u32) -> u32 = stdcall vslot 0x6C => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `D3DTextureBuffer::Push(IDirect3DTexture9*)` (0x72FFF0)
    fn tex_buffer_push(this: GPtr<u8>, t: GPtr<u8>) = thiscall 0x72FFF0 => unported;
    /// `D3DTextureBuffer::Resize(int32)` (0x730020)
    fn tex_buffer_resize(this: GPtr<u8>, n: u32) = thiscall 0x730020 => unported;
    /// `D3DTextureBuffer::PushWithoutIncreasingCounter(IDirect3DTexture9*)` (0x730AD0)
    fn tex_buffer_push_nc(this: GPtr<u8>, t: GPtr<u8>) = thiscall 0x730AD0 => unported;
    /// `D3DIndexDataBuffer::Resize(int32)` (0x730330)
    fn idx_buffer_resize(this: GPtr<u8>, n: u32) = thiscall 0x730330 => unported;
    /// `D3DTextureBuffer::Setup` (0x72FE80)
    fn tex_buffer_setup_o(this: GPtr<u8>, fmt: u32, size: u32, levels: u32, cap: u32) -> u32 = thiscall 0x72FE80 => crate::mvpd_d3d::tex_buffer_setup;
    /// `D3DIndexDataBuffer::Pop(uint32)` (0x730270)
    fn idx_buffer_pop_o(this: GPtr<u8>, size: u32) -> GPtr<u8> = thiscall 0x730270 => crate::mvpd_d3d::idx_buffer_pop;
}

/// `D3DResourceSystem::m_bUseD3DResourceBuffering` (u8)
const USE_BUFFERING: usize = 0x8D6084;
const TEX_BUFFERS: usize = 0xC87C68;
const NUM_TEX_BUFFERS: usize = 0xC87C60;
const IDX_BUCKETS: usize = 0xC87E48;
const IDX_LARGE: usize = 0xC87FC8;

fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `D3DTextureBuffer::Setup(format, size, levels, capacity)` (0x72FE80, thiscall, `ret 0x10`): +0
/// format, +4 size, +8 levels; array (+0x18) = `operator new(capacity * 4)`, +0xC capacity, +0x14 and
/// +0x10 zero. Returns 0 (EAX).
#[hook(addr = 0x72FE80, conv = thiscall, name = "D3DTextureBuffer::Setup", this = "D3DTextureBuffer", expect = "8B 44 24 04 8B 54 24 0C",
       unverified = "mvpd-d3d-1", route = "L2", hazards = "heap-writes")]
pub fn tex_buffer_setup(this: GPtr<u8>, fmt: u32, size: u32, levels: u32, cap: u32) -> u32 {
    wr::<u32>(this, 0, fmt);
    wr::<u32>(this, 4, size);
    wr::<u32>(this, 8, levels);
    let a = operator_new(cap.wrapping_mul(4));
    wr::<u32>(this, 0x18, a.addr() as u32);
    wr::<u32>(this, 0xC, cap);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x10, 0);
    0
}

/// `D3DIndexDataBuffer::Pop(uint32 numIndices)` (0x730270, thiscall, `ret 4`): scans the stored index
/// buffers from the last to the first (`GetDesc`) for the smallest size (unsigned, strictly smaller
/// than the best so far, first 0x7FFFFFFF) of at least `2 * numIndices` bytes (the original keeps that
/// in its argument slot); none → NULL. The chosen one is removed (count - 1; the last entry moves into
/// its slot unless the new count is 0 or the slot is the last) and returned.
#[hook(addr = 0x730270, conv = thiscall, name = "D3DIndexDataBuffer::Pop", this = "D3DIndexDataBuffer", expect = "8B 44 24 04 83 EC 14",
       unverified = "mvpd-d3d-1", route = "L2", hazards = "stack-residue,heap-writes",
       note = "keeps 2 * numIndices in its own argument slot; COM GetDesc")]
pub fn idx_buffer_pop(this: GPtr<u8>, n: u32) -> GPtr<u8> {
    let need = n.wrapping_mul(2);
    let mut i = rd::<i32>(this, 0x10);
    let mut best = 0x7FFF_FFFFu32;
    let mut sel = -1i32;
    while i != 0 {
        let b = rd::<GPtr<u8>>(this, 0x14).cast::<GPtr<u8>>().wrapping_index(i - 1).read();
        i -= 1;
        let mut desc = [0u32; 5];
        ib_get_desc(b, lp(&mut desc));
        if desc[4] < best && desc[4] >= need {
            best = desc[4];
            sel = i;
        }
    }
    if sel == -1 {
        return GPtr::NULL;
    }
    let cnt = rd::<i32>(this, 0x10) - 1;
    let arr = rd::<GPtr<u8>>(this, 0x14).cast::<GPtr<u8>>();
    let res = arr.wrapping_index(sel).read();
    wr::<i32>(this, 0x10, cnt);
    if cnt > 0 && cnt != sel {
        arr.wrapping_index(sel).write(arr.wrapping_index(cnt).read());
    }
    res
}

/// `D3DResourceSystem::CreateIndexBuffer(uint32 numIndices, D3DFORMAT format, IDirect3DIndexBuffer9** out)` (0x7306A0,
/// cdecl): bucket `(numIndices - 1) / 100` (signed), capacity `(bucket + 1) * 100`. Buckets 0..15 take
/// their last stored buffer (+0xC and +0x10 decremented; its `GetDesc` is called and ignored); larger
/// sizes `Pop` from 0xC87FC8. A reused buffer returns 0; otherwise the device (`[0xC97C28]`)
/// `CreateIndexBuffer(capacity * 2, WRITEONLY, INDEX16, MANAGED, out, NULL)` and its HRESULT.
#[hook(addr = 0x7306A0, conv = cdecl, name = "D3DResourceSystem::CreateIndexBuffer", expect = "8B 4C 24 04 83 EC 14",
       route = "split", hazards = "heap-writes",
       note = "creates D3D index buffers (COM state cannot run twice)")]
pub fn create_index_buffer(n: u32, _format: u32, out: GPtr<u8>) -> u32 {
    let bucket = (n as i32).wrapping_sub(1) / 100;
    let cap = (bucket + 1).wrapping_mul(100);
    if bucket < 0x10 {
        let l = g::<u8>(IDX_BUCKETS).byte_add(bucket as isize * 0x18);
        let c = rd::<i32>(l, 0x10);
        let b = if c != 0 {
            wr::<i32>(l, 0xC, rd::<i32>(l, 0xC) - 1);
            wr::<i32>(l, 0x10, c - 1);
            rd::<GPtr<u8>>(l, 0x14).cast::<GPtr<u8>>().wrapping_index(c - 1).read()
        } else {
            GPtr::NULL
        };
        out.cast::<GPtr<u8>>().write(b);
        if !b.is_null() {
            let mut desc = [0u32; 5];
            ib_get_desc(b, lp(&mut desc));
            return 0;
        }
    } else {
        let b = idx_buffer_pop_o(g(IDX_LARGE), cap as u32);
        out.cast::<GPtr<u8>>().write(b);
        if !b.is_null() {
            return 0;
        }
    }
    let dev = g::<GPtr<u8>>(0xC97C28).read();
    dev_create_index_buffer(dev, (cap as u32).wrapping_mul(2), 8, 0x65, 1, out, 0)
}

/// `fild dword` of an unsigned value as MSVC does it (plus 2^32 when negative).
fn fild_u32(v: u32) -> X87 {
    let r = X87::from_i32(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(0x4F80_0000)) } else { r }
}

/// `D3DResourceSystem::DestroyTexture(IDirect3DTexture9*)` (0x730B70, cdecl): without buffering
/// (`[0x8D6084]`) `Release`. Otherwise with `levels = (GetLevelCount() <= 1)` and level 0's desc:
/// a square texture goes to the first buffer 1..n-1 of equal size (`fild` vs the width as f32,
/// unsigned), format and levels (counter + 1; stored, or `Resize(capacity * 2)` +
/// `PushWithoutIncreasingCounter`). Unmatched with fewer than 17 buffers and square sizes 32..256: a
/// new buffer `Setup(format, width, levels, 16)` + `Push`, buffer count + 1; square above 256 →
/// `Release`. Everything else up to 256×256 goes to buffer 0 (counter + 1; when full, `Resize(capacity
/// * 2)` and stored only if room was made), larger textures are released.
#[hook(addr = 0x730B70, conv = cdecl, name = "D3DResourceSystem::DestroyTexture", expect = "A0 84 60 8D 00",
       route = "split", hazards = "heap-writes",
       note = "COM texture calls and release")]
pub fn destroy_texture(t: GPtr<u8>) {
    if g::<u8>(USE_BUFFERING).read() == 0 {
        com_release(t);
        return;
    }
    let levels = u32::from(tex_get_level_count(t) <= 1);
    let mut desc = [0u32; 8];
    tex_get_level_desc(t, 0, lp(&mut desc));
    let (w, h, fmt) = (desc[6], desc[7], desc[0]);
    let nb = g::<i32>(NUM_TEX_BUFFERS).read();
    if w == h && nb > 1 {
        let wf = fild_u32(w).to_f32();
        for i in 1..nb {
            let b = g::<u8>(TEX_BUFFERS).byte_add(i as isize * 0x1C);
            if X87::from_i32(rd::<i32>(b, 4)).fcom(x(wf)).test_parity(0x44) {
                continue;
            }
            if fmt != rd::<u32>(b, 0) || levels != rd::<u32>(b, 8) {
                continue;
            }
            wr::<i32>(b, 0x10, rd::<i32>(b, 0x10) + 1);
            let c = rd::<i32>(b, 0x14);
            let cap = rd::<i32>(b, 0xC);
            if c < cap {
                rd::<GPtr<u8>>(b, 0x18).cast::<GPtr<u8>>().wrapping_index(c).write(t);
                wr::<i32>(b, 0x14, rd::<i32>(b, 0x14) + 1);
            } else {
                tex_buffer_resize(b, cap.wrapping_mul(2) as u32);
                tex_buffer_push_nc(b, t);
            }
            return;
        }
    }
    if nb < 0x11 && w == h && w >= 0x20 {
        if w > 0x100 {
            com_release(t);
            return;
        }
        tex_buffer_setup_o(g::<u8>(TEX_BUFFERS).byte_add(nb as isize * 0x1C), fmt, w, levels, 0x10);
        let nb2 = g::<i32>(NUM_TEX_BUFFERS).read();
        tex_buffer_push(g::<u8>(TEX_BUFFERS).byte_add(nb2 as isize * 0x1C), t);
        g::<i32>(NUM_TEX_BUFFERS).write(nb2 + 1);
        return;
    }
    if w > 0x100 || h > 0x100 {
        com_release(t);
        return;
    }
    let b0 = g::<u8>(TEX_BUFFERS);
    wr::<i32>(b0, 0x10, rd::<i32>(b0, 0x10) + 1);
    let c = rd::<i32>(b0, 0x14);
    let cap = rd::<i32>(b0, 0xC);
    if c >= cap {
        tex_buffer_resize(b0, cap.wrapping_mul(2) as u32);
        if rd::<i32>(b0, 0x14) >= rd::<i32>(b0, 0xC) {
            return;
        }
    }
    let c = rd::<i32>(b0, 0x14);
    rd::<GPtr<u8>>(b0, 0x18).cast::<GPtr<u8>>().wrapping_index(c).write(t);
    wr::<i32>(b0, 0x14, rd::<i32>(b0, 0x14) + 1);
}

/// `D3DResourceSystem::DestroyIndexBuffer(IDirect3DIndexBuffer9*)` (0x730D30, cdecl): without
/// buffering `Release`; else by `GetDesc` size the bucket `((size >> 1) - 1) / 100` (unsigned): 0..15
/// that bucket, else the large-size buffer 0xC87FC8 — counter (+0xC) + 1 and the buffer stored, after
/// a `Resize(capacity * 2)` when full (dropped if still full).
#[hook(addr = 0x730D30, conv = cdecl, name = "D3DResourceSystem::DestroyIndexBuffer", expect = "A0 84 60 8D 00",
       route = "split", hazards = "heap-writes",
       note = "COM index buffer calls and release")]
pub fn destroy_index_buffer(ib: GPtr<u8>) {
    if g::<u8>(USE_BUFFERING).read() == 0 {
        com_release(ib);
        return;
    }
    let mut desc = [0u32; 5];
    ib_get_desc(ib, lp(&mut desc));
    let bucket = ((desc[4] >> 1).wrapping_sub(1)) / 100;
    let l = if (bucket as i32) < 0x10 { g::<u8>(IDX_BUCKETS).byte_add(bucket as isize * 0x18) } else { g::<u8>(IDX_LARGE) };
    wr::<i32>(l, 0xC, rd::<i32>(l, 0xC) + 1);
    let cap = rd::<i32>(l, 8);
    if rd::<i32>(l, 0x10) >= cap {
        idx_buffer_resize(l, cap.wrapping_mul(2) as u32);
        if rd::<i32>(l, 0x10) >= rd::<i32>(l, 8) {
            return;
        }
    }
    let c = rd::<i32>(l, 0x10);
    rd::<GPtr<u8>>(l, 0x14).cast::<GPtr<u8>>().wrapping_index(c).write(ib);
    wr::<i32>(l, 0x10, rd::<i32>(l, 0x10) + 1);
}

/// `CVector::operator*=(float)` (0x40FEF0, thiscall, `ret 4`): each component `= s * c` (f32 stores).
/// Returns `this` in EAX.
#[hook(addr = 0x40FEF0, conv = thiscall, name = "FUN_0040fef0", this = "CVector", expect = "D9 44 24 04 8B C1",
       route = "L2", hazards = "none")]
pub fn vector_mul_assign(this: GPtr<u8>, s: f32) -> GPtr<u8> {
    for c in 0..3isize {
        wr::<f32>(this, c * 4, x(s).fmul_m(rd::<f32>(this, c * 4)).to_f32());
    }
    this
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TEX_BUFFER_SETUP,
    HOOK_IDX_BUFFER_POP,
    HOOK_CREATE_INDEX_BUFFER,
    HOOK_DESTROY_TEXTURE,
    HOOK_DESTROY_INDEX_BUFFER,
    HOOK_VECTOR_MUL_ASSIGN,
];
