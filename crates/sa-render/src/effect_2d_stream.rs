//! `C2dEffect::Rwt2dEffectPluginDataChunkReadCallBack` (STD-6, batch `std6-render-1`). Ported from the
//! 1.0 US bytes (0x6F9FD0; its tail goes through the protection chunk 0x407EAF, which loads the
//! geometry argument).
//!
//! The plugin data (`geometry + [0xC3A1E0]`) is one `CMemoryMgr::Malloc` block: a count, then
//! `C2dEffect` entries of 0x40 bytes (+0 position, +0xC type byte, +0x10.. the type's fields).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CTxdStore::FindTxdSlot(name)` (0x731850), `PushCurrentTxd` (0x7316A0), `SetCurrentTxd` (0x7319C0),
    /// `PopCurrentTxd` (0x7316B0)
    fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => unported;
    fn push_current_txd() = cdecl 0x7316A0 => unported;
    fn set_current_txd(slot: i32) = cdecl 0x7319C0 => unported;
    fn pop_current_txd() = cdecl 0x7316B0 => unported;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0), `RwStreamSkip(stream, n)` (0x7ECD00)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    fn stream_skip(s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ECD00 => unported;
    /// `CMemoryMgr::Malloc(size, hint)` (0x72F420)
    fn mgr_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `RwTextureRead(name, mask)` (0x7F3AC0)
    fn texture_read(name: GPtr<u8>, mask: u32) -> u32 = cdecl 0x7F3AC0 => unported;
    /// CRT `strncpy` (0x821F40)
    fn crt_strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
}

/// `C2dEffect::ms_nTxdSlot` (.data, -1) and the geometry plugin offset.
const TXD_SLOT: usize = 0x8D4948;
const PLUGIN_OFFSET: usize = 0xC3A1E0;

fn w8(p: GPtr<u8>, o: isize, v: u8) {
    p.byte_add(o).write(v)
}

fn w16(p: GPtr<u8>, o: isize, v: u16) {
    p.byte_add(o).cast::<u16>().write(v)
}

fn w32(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(v)
}

fn d32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// `C2dEffect::Rwt2dEffectPluginDataChunkReadCallBack(stream, length, geometry, offset, size)`
/// (0x6F9FD0): the "particle" TXD slot is looked up once; a count of 0 returns. Otherwise per effect:
/// position, type (stored as a byte) and chunk size; a size the type does not accept is skipped
/// (`RwStreamSkip`) and the entry dropped (the count shrinks, the slot is reused). Lights (0): 0x50 /
/// 0x4C byte records (colour, distances, sizes, flags, shadow data; the 0x4C form gets look direction
/// 0, 0, 100), corona and shadow textures read from the particle TXD; particles (1, 0x18): the name;
/// type 7 (0x58): five dwords, a word and a 0x40-byte block copied to a new allocation (+0x28, +0x2C
/// = 0); type 6 (0x2C or 0x28; the shorter record leaves its last bytes at 0 and 0x18): dwords, flags
/// and an 8-byte name; type 9 (0xC), type 8 (4), type 10 (0x28), type 3 (0x38: dwords, bytes and an
/// 8-byte name); type 4 reads nothing; other types neither read nor skip. The block's count is the
/// number kept. Returns the stream.
#[hook(addr = 0x6F9FD0, conv = cdecl, name = "C2dEffect::Rwt2dEffectPluginDataChunkReadCallBack", expect = "A1 48 49 8D 00 81 EC 7C 01 00 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes,uninit-locals", recs = "new-game",
       note = "the original reads each record into fixed stack buffers without bounds; the 0x28-byte type-6 record leaves one byte of stack residue (here 0)")]
pub fn effect_2d_stream_read(s: GPtr<u8>, _len: i32, geom: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if GPtr::<i32>::from_addr(TXD_SLOT).read() == -1 {
        GPtr::<i32>::from_addr(TXD_SLOT).write(find_txd_slot(GPtr::from_addr(0x85C648)));
    }
    let mut n = 0u32;
    stream_read(s, GPtr::from_raw(&mut n as *mut u32).cast(), 4);
    if n == 0 {
        return s;
    }
    let buf = mgr_malloc((n << 6).wrapping_add(4), 0);
    let mut i = 0u32;
    let mut obj = buf.byte_add(4);
    while i < n {
        stream_read(s, obj, 0xC);
        let mut ty = 0u32;
        let mut size = 0u32;
        stream_read(s, GPtr::from_raw(&mut ty as *mut u32).cast(), 4);
        w8(obj, 0xC, ty as u8);
        stream_read(s, GPtr::from_raw(&mut size as *mut u32).cast(), 4);
        let mut rec = [0u8; 0x100];
        let rp = GPtr::from_raw(rec.as_mut_ptr());
        let mut skip = false;
        match ty as u8 {
            0 => {
                if size == 0x50 || size == 0x4C {
                    stream_read(s, rp, size);
                    for k in 0..5 {
                        w32(obj, 0x10 + k * 4, d32(&rec, k as usize * 4));
                    }
                    w8(obj, 0x27, rec[0x15]);
                    w8(obj, 0x28, rec[0x16]);
                    w8(obj, 0x26, rec[0x14]);
                    w8(obj, 0x29, rec[0x17]);
                    w16(obj, 0x24, ((rec[0x4A] as u16) << 8).wrapping_add(rec[0x18] as u16));
                    w8(obj, 0x2A, rec[0x49]);
                    if size == 0x50 {
                        w8(obj, 0x2B, rec[0x4B]);
                        w8(obj, 0x2C, rec[0x4C]);
                        w8(obj, 0x2D, rec[0x4D]);
                    } else {
                        w8(obj, 0x2B, 0);
                        w8(obj, 0x2C, 0);
                        w8(obj, 0x2D, 0x64);
                    }
                    push_current_txd();
                    set_current_txd(GPtr::<i32>::from_addr(TXD_SLOT).read());
                    w32(obj, 0x30, texture_read(GPtr::from_raw(rec[0x19..].as_mut_ptr()), 0));
                    w32(obj, 0x34, texture_read(GPtr::from_raw(rec[0x31..].as_mut_ptr()), 0));
                    pop_current_txd();
                } else {
                    skip = true;
                }
            }
            1 => {
                if size == 0x18 {
                    stream_read(s, rp, 0x18);
                    let mut k = 0;
                    loop {
                        let c = rec[k];
                        w8(obj, 0x10 + k as isize, c);
                        if c == 0 {
                            break;
                        }
                        k += 1;
                    }
                } else {
                    skip = true;
                }
            }
            7 => {
                if size == 0x58 {
                    stream_read(s, rp, 0x58);
                    for k in 0..5 {
                        w32(obj, 0x10 + k * 4, d32(&rec, k as usize * 4));
                    }
                    w16(obj, 0x24, u16::from_le_bytes([rec[0x14], rec[0x15]]));
                    w32(obj, 0x2C, 0);
                    let blk = mgr_malloc(0x40, 0);
                    w32(obj, 0x28, blk.addr() as u32);
                    for k in 0..0x40 {
                        w8(blk, k as isize, rec[0x16 + k]);
                    }
                } else {
                    skip = true;
                }
            }
            6 => {
                rec[0x28] = 0;
                rec[0x29] = 0x18;
                if size == 0x2C || size == 0x28 {
                    stream_read(s, rp, size);
                    for k in 0..7 {
                        w32(obj, 0x10 + k * 4, d32(&rec, k as usize * 4));
                    }
                    w16(obj, 0x2C, u16::from_le_bytes([rec[0x1C], rec[0x1D]]));
                    w8(obj, 0x2E, rec[0x1E]);
                    w8(obj, 0x2F, rec[0x1F]);
                    crt_strncpy(obj.byte_add(0x30), rp.byte_add(0x20), 8);
                    w8(obj, 0x38, rec[0x28]);
                    w8(obj, 0x39, rec[0x29]);
                    w8(obj, 0x3A, rec[0x2A]);
                } else {
                    skip = true;
                }
            }
            9 => {
                if size == 0xC {
                    stream_read(s, rp, 0xC);
                    w32(obj, 0x10, d32(&rec, 0));
                    w32(obj, 0x14, d32(&rec, 4));
                    w8(obj, 0x18, rec[8]);
                } else {
                    skip = true;
                }
            }
            4 => {}
            8 => {
                if size == 4 {
                    stream_read(s, rp, 4);
                    w32(obj, 0x10, d32(&rec, 0));
                } else {
                    skip = true;
                }
            }
            0xA => {
                if size == 0x28 {
                    stream_read(s, rp, 0x28);
                    w8(obj, 0x34, rec[0x24]);
                    for k in 0..8 {
                        w32(obj, 0x10 + k * 4, d32(&rec, k as usize * 4));
                    }
                    w32(obj, 0x30, d32(&rec, 0x20));
                } else {
                    skip = true;
                }
            }
            3 => {
                if size == 0x38 {
                    stream_read(s, rp, 0x38);
                    for k in 0..8 {
                        w32(obj, 0x10 + k * 4, d32(&rec, 4 + k as usize * 4));
                    }
                    w8(obj, 0x35, rec[0x30]);
                    w32(obj, 0x30, d32(&rec, 0x24));
                    w8(obj, 0x34, rec[0]);
                    w8(obj, 0x36, rec[0x34]);
                    w8(obj, 0x37, rec[0x36]);
                    crt_strncpy(obj.byte_add(0x38), rp.byte_add(0x28), 8);
                } else {
                    skip = true;
                }
            }
            _ => {}
        }
        if skip {
            stream_skip(s, size);
            n -= 1;
        } else {
            i += 1;
            obj = obj.byte_add(0x40);
        }
    }
    buf.cast::<u32>().write(n);
    geom.byte_add(GPtr::<i32>::from_addr(PLUGIN_OFFSET).read() as isize).cast::<u32>().write(buf.addr() as u32);
    s
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EFFECT_2D_STREAM_READ];
