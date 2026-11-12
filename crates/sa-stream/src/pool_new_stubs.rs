//! Three `CPool<T>::New()` instances whose entries are 5-byte stubs into `.HOODLUM` (batch `mvpl-stream-1`):
//! `CPool<IplDef>` (0x34-byte slots), `CPool<ColDef>` (0x2C) and the stunt jump pool (0x44). Ported from the 1.0 US
//! bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}
fn wr<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}

/// The shared body (thiscall; objects +0, flags +4, size +8, firstFree +0xC). `firstFree` advances, stored at each
/// step, to the next slot whose flag has bit 7 (free) set; reaching `size` (read once at entry) the first time stores
/// 0 and goes on from slot 0, the second time stores 0 and returns NULL. The found flag loses bit 7, then its low 7
/// bits (the id) are incremented (`b ^ (((b + 1) ^ b) & 0x7F)`, two read-modify-writes as in the original); returns
/// `objects + firstFree * elem`.
fn pool_new_bytes(this: GPtr<u8>, elem: i32) -> GPtr<u8> {
    let size = rd::<i32>(this, 8);
    let mut wrapped = false;
    loop {
        let i = rd::<i32>(this, 0xC).wrapping_add(1);
        wr::<i32>(this, 0xC, i);
        if i == size {
            wr::<i32>(this, 0xC, 0);
            if wrapped {
                return GPtr::NULL;
            }
            wrapped = true;
        }
        let i = rd::<i32>(this, 0xC);
        if rd::<GPtr<u8>>(this, 4).byte_add(i as isize).read() & 0x80 != 0 {
            break;
        }
    }
    let f = rd::<GPtr<u8>>(this, 4).byte_add(rd::<i32>(this, 0xC) as isize);
    f.write(f.read() & 0x7F);
    let f = rd::<GPtr<u8>>(this, 4).byte_add(rd::<i32>(this, 0xC) as isize);
    let b = f.read();
    f.write(b ^ ((b.wrapping_add(1) ^ b) & 0x7F));
    let i = rd::<i32>(this, 0xC);
    rd::<GPtr<u8>>(this, 0).byte_add(i.wrapping_mul(elem) as isize)
}

/// `CPool<IplDef>::New()` (stub 0x4059B0 → 0x15654E0, thiscall): 0x34-byte slots; see [`pool_new_bytes`].
#[hook(addr = 0x4059b0, conv = thiscall, name = "thunk_FUN_015654e0", expect = "E9 2B FB 15 01",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the pool's flag array (heap)")]
pub fn ipl_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    pool_new_bytes(this, 0x34)
}

/// `CPool<ColDef>::New()` (stub 0x411030 → 0x156FBB0, thiscall): 0x2C-byte slots; see [`pool_new_bytes`].
#[hook(addr = 0x411030, conv = thiscall, name = "thunk_FUN_0156fbb0", expect = "E9 7B EB 15 01",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the pool's flag array (heap)")]
pub fn col_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    pool_new_bytes(this, 0x2C)
}

/// The stunt jump pool's `New()` (stub 0x49C9E0 → 0x1566280, thiscall; `CStuntJumpManager::AddOne` / `Load`): 0x44-byte
/// slots; see [`pool_new_bytes`].
#[hook(addr = 0x49c9e0, conv = thiscall, name = "thunk_FUN_01566280", expect = "E9 9B 98 0C 01",
       unverified = "mvpl-stream-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the pool's flag array (heap)")]
pub fn stunt_jump_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    pool_new_bytes(this, 0x44)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IPL_POOL_NEW, HOOK_COL_POOL_NEW, HOOK_STUNT_JUMP_POOL_NEW];
