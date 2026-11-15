//! FRONTIER-2 lane C: this crate's phase B natives of bodies outside the catalogue (CAT-12 tail targets).

/// Every lane C `#[native]` of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[crate::pathfind::NATIVE_DTOR_977BE8_BODY_NATIVE];

/// `CPathFind::StoreDetachedNodeInfoPed(...)` (0x44D2E0, thiscall, `ret 0x38`: 14 stack arguments; a no-op in 1.0 US, 3
/// bytes, not hookable; FRONTIER-2 turn 2). Called by `CFileLoader::LoadPedPathNode`; the arguments are not read, so the
/// native takes `this` only, as its one Rust caller's declaration (sa-stream's file loader).
#[sa_core::native(addr = 0x44D2E0, conv = thiscall, name = "CPathFind::StoreDetachedNodeInfoPed")]
pub fn store_detached_node_info_ped(_this: sa_core::mem::GPtr<u8>) {}

/// Every FRONTIER-2 turn 2 lane C `#[native]` of this crate.
pub const NATIVES2: &[sa_core::abi::NativeReg] = &[NATIVE_STORE_DETACHED_NODE_INFO_PED];
