//! The skin pipeline's vertex-shader begin callback 0x7CB190 (batch `samp-rw-2`), ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `RwRenderStateGet(RwRenderState, void*)`: slot +0x24 of `RwEngineInstance` (0xC97B24)
    fn rw_render_state_get(state: u32, out: GPtr<u32>) -> u32 = cdecl table 0xC97B24 0x24 => unported;
}

/// 0x7CB190 (catalogue: missed by Ghidra, data-code-immediate), proposed `_rpD3D9SkinVertexShaderBeginCallBack(void*
/// object, RwUInt32 type, _rpD3D9VertexShaderDescriptor* desc)` (cdecl): `_rpD3D9SkinPipelineInit` (0x7CB240) stores
/// it as the begin callback of the skin pipeline's vertex-shader node. The descriptor dword := 0; byte 1 := the
/// skin's byte +0x1C (through `object +0x18`) shifted left by 4; byte 2 gets 0x10 / 0x60 / 0x80 for the skin flags
/// (+8) 0x08 / 0x10 / 0x40. When render state 0xE reads non-zero, render state 0x10 picks byte 3's low two bits: 1 →
/// bit 0 set, bit 1 cleared; 2 → bit 1 set, bit 0 cleared; 3 → both set; otherwise unchanged. The original reads the
/// two render states into its own argument slots (`type`, then `object`), so a failing read leaves the argument.
#[hook(addr = 0x7CB190, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 56",
       unverified = "samp-rw-2", route = "L2", hazards = "none", recs = "anim-input",
       note = "writes its own argument slots in the original (no caller reads them)")]
pub fn skin_vertex_shader_begin_callback(object: GPtr<u8>, ty: u32, desc: GPtr<u8>) {
    desc.cast::<u32>().write(0);
    let skin = object.byte_add(0x18).cast::<GPtr<u8>>().read();
    let flags = skin.byte_add(8).cast::<u32>().read();
    let b1 = desc.byte_add(1);
    let al = skin.byte_add(0x1C).read() << 4;
    b1.write(((b1.read() ^ al) & 0xF) ^ al);
    let b2 = desc.byte_add(2);
    if flags & 8 != 0 {
        b2.write(b2.read() | 0x10);
    }
    if flags & 0x10 != 0 {
        b2.write(b2.read() | 0x60);
    }
    if flags & 0x40 != 0 {
        b2.write(b2.read() | 0x80);
    }
    // the slots start as the arguments: a failing RwRenderStateGet leaves them
    let mut v = [ty; 1];
    rw_render_state_get(0xE, GPtr::from_raw(v.as_mut_ptr()));
    if v[0] == 0 {
        return;
    }
    let mut m = [object.addr() as u32; 1];
    rw_render_state_get(0x10, GPtr::from_raw(m.as_mut_ptr()));
    let b3 = desc.byte_add(3);
    match m[0] {
        1 => b3.write((b3.read() & 0xFD) | 1),
        2 => b3.write((b3.read() & 0xFE) | 2),
        3 => b3.write(b3.read() | 3),
        _ => {}
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SKIN_VERTEX_SHADER_BEGIN_CALLBACK];
