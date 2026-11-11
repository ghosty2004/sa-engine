//! STD-5: `CPool<T>` constructor instantiations reached by `CGame::Initialise` that had no port. Each
//! is the shared constructor algorithm of [`crate::pools::pool_construct`] (from the 1.0 US bytes:
//! `objects = new[](size * slot)`, `map = new[](size)`, `owns = 1`, `size`, `first_free = -1`, the
//! two read-modify-writes per map byte); only the slot size differs.

use crate::abi::HookReg;
use crate::{hook, orig};
use crate::mem::GPtr;
use crate::pools::{PoolPtr, Slot, call, pool_construct};

/// `CPool<CStuntJump>::CPool(int32 size, const char* name)` (stub 0x49C8F0 → .HOODLUM 0x156FA80,
/// `ret 8`): slot 0x44 (`imul eax, eax, 0x44`), called by `CStuntJumpManager::Init`.
#[hook(addr = 0x49C8F0, conv = thiscall, name = "thunk_FUN_0156fa80", expect = "E9 8B 31 0D 01", this = "CPool<int32>",
       unverified = "std5-pool-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn stunt_jump_pool_ctor(p: PoolPtr<Slot<0x44>>, size: i32, _name: GPtr<u8>) -> PoolPtr<Slot<0x44>> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CEntryExit>::New()` (stub 0x43F7D0 → .HOODLUM 0x15651D0): the shared `New` algorithm, slot
/// 0x3C (`imul eax, eax, 0x3C`).
#[hook(addr = 0x43F7D0, conv = thiscall, name = "thunk_FUN_015651d0", expect = "E9 FB 59 12 01", this = "CPool<int32>",
       unverified = "std5-pool-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn entry_exit_pool_new(p: PoolPtr<Slot<0x3C>>) -> GPtr<Slot<0x3C>> {
    crate::pool::pool_new(p)
}

orig! {
    /// `CPtrNodeSingleLinkPool::New()` (0x552240)
    fn ptr_node_single_pool_new(p: u32) -> GPtr<u8> = thiscall 0x552240 => unported;
    /// `CPtrNodeDoubleLinkPool::New()` (0x5522E0)
    fn ptr_node_double_pool_new(p: u32) -> GPtr<u8> = thiscall 0x5522E0 => unported;
}

/// `CPtrNodeSingleLink::operator new(size)` (0x552380): `[0xB74484]->New()` (tail jump).
#[hook(addr = 0x552380, conv = cdecl, name = "new", expect = "8B 0D 84 44 B7 00 E9",
       unverified = "std5-pool-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn ptr_node_single_new(_size: u32) -> GPtr<u8> {
    ptr_node_single_pool_new(GPtr::<u32>::from_addr(0xB74484).read())
}

/// `CPtrNodeDoubleLink::operator new(size)` (0x5523C0): `[0xB74488]->New()` (tail jump).
#[hook(addr = 0x5523C0, conv = cdecl, name = "new", expect = "8B 0D 88 44 B7 00 E9",
       unverified = "std5-pool-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn ptr_node_double_new(_size: u32) -> GPtr<u8> {
    ptr_node_double_pool_new(GPtr::<u32>::from_addr(0xB74488).read())
}

/// The inlined `CPool::Delete` of the pointer-node `operator delete`s: `map[idx] |= 0x80`, then
/// `first_free = idx` when `idx < first_free` (signed).
fn release(pool: u32, idx: i32) {
    let pool = GPtr::<u8>::from_addr(pool as usize);
    let map = GPtr::<u8>::from_addr(pool.byte_add(4).cast::<u32>().read() as usize).byte_add(idx as isize);
    map.write(map.read() | 0x80);
    let ff = pool.byte_add(0xC).cast::<i32>();
    if idx < ff.read() {
        ff.write(idx);
    }
}

/// `CPtrNodeSingleLink::operator delete(void*)` (0x552390, protected: its chunk 0x409B8D loads the pool
/// `[~[0x15608F0] ^ [0x6C8D62]]` = `[0xB74484]`): index `(p - objects) >> 3` (arithmetic).
#[crate::native(addr = 0x552390, conv = cdecl, name = "CPtrNodeSingleLink::operator delete")]
pub fn ptr_node_single_delete(p: GPtr<u8>) {
    let pool = GPtr::<u32>::from_addr(0xB74484).read();
    let objects = GPtr::<u32>::from_addr(pool as usize).read();
    release(pool, ((p.addr() as u32).wrapping_sub(objects) as i32) >> 3);
}

/// `CPtrNodeDoubleLink::operator delete(void*)` (0x5523D0): index `(p - objects) / 0xC` (signed).
#[hook(addr = 0x5523D0, conv = cdecl, name = "delete", expect = "8B 4C 24 04 56 8B 35 88 44 B7",
       unverified = "std5-pool-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn ptr_node_double_delete(p: GPtr<u8>) {
    let pool = GPtr::<u32>::from_addr(0xB74488).read();
    let objects = GPtr::<u32>::from_addr(pool as usize).read();
    release(pool, ((p.addr() as u32).wrapping_sub(objects) as i32) / 0xC);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_STUNT_JUMP_POOL_CTOR,
    HOOK_ENTRY_EXIT_POOL_NEW,
    HOOK_PTR_NODE_SINGLE_NEW,
    HOOK_PTR_NODE_DOUBLE_NEW,
    HOOK_PTR_NODE_DOUBLE_DELETE,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[crate::abi::NativeReg] = &[NATIVE_PTR_NODE_SINGLE_DELETE];
