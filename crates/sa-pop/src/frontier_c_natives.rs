//! FRONTIER-2 lane C: this crate's phase B natives of bodies outside the catalogue (CAT-12 tail targets).

/// `CPathFind::StoreDetachedNodeInfoPed(...)` (0x44D2E0, thiscall, `ret 0x38`: 14 stack arguments; a no-op in 1.0 US, 3
/// bytes, not hookable; FRONTIER-2 turn 2). Called by `CFileLoader::LoadPedPathNode` (sa-stream's file loader) with
/// its 14 stack words, none read.
#[sa_core::native(addr = 0x44D2E0, conv = thiscall, name = "CPathFind::StoreDetachedNodeInfoPed")]
#[allow(clippy::too_many_arguments)]
pub fn store_detached_node_info_ped(_this: u32, _a0: u32, _a1: u32, _a2: u32, _a3: u32, _a4: u32, _a5: u32, _a6: u32, _a7: u32, _a8: u32, _a9: u32, _a10: u32, _a11: u32, _a12: u32, _a13: u32) {}

/// Every lane C `#[native]` of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_STORE_DETACHED_NODE_INFO_PED];
