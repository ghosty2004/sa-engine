# CPathFind (STREAM) - full specs, writer A (RE-SPEC-1)

Both functions are empty leftovers from Vice City, called by CFileLoader::LoadCarPathNode (0x5B4380) with
`ECX = 0x96F050` (`ThePaths`, gta-reversed PathFind.h:618). gta-reversed PathFind.h:278-279 declares
`StoreNodeInfoCar(...)` (17 params) and `StoreNodeInfoPed(...)` (12 params), both `{ /*noop*/ }`.

## 0x44D2C0 CPathFind::StoreNodeInfoCar  (catalogue: unknown/REA, port level 0; proposed name)
Name hint `sub_44d2c0_from_CFileLoader__LoadCarPathNode`. Evidence: `ret 0x44` = 17 dword stack args,
matching gr's 17-parameter `StoreNodeInfoCar`; called with `ECX = &ThePaths` at 0x5B4520.
- Convention: `__thiscall void (CPathFind* this, 17 x 32-bit stack args)`; callee pops 0x44 bytes.
- Behaviour: none (`ret 0x44`). A port must still consume 0x44 bytes of arguments if called through the
  original ABI.
- Open questions: which of the two 17-argument stubs (0x44D2C0 / 0x44D2F0) is `StoreNodeInfoCar` and which
  is `StoreDetachedNodeInfoCar` (by analogy with re3/VC) is not provable from the bytes; both are
  behaviourally identical no-ops, so this does not affect the port.

## 0x44D2F0 CPathFind::StoreDetachedNodeInfoCar  (catalogue: unknown/REA, port level 0; proposed name)
Name hint `sub_44d2f0_from_CFileLoader__LoadCarPathNode`. Evidence: second no-op stub called from the same
function (0x5B44A3, on the other branch of LoadCarPathNode), also `ret 0x44`. Re3 (VC) LoadCarPathNode
calls either `StoreNodeInfoCar` or `StoreDetachedNodeInfoCar`; the name is therefore by analogy (medium).
- Convention: `__thiscall void (CPathFind* this, 17 x 32-bit stack args)`; callee pops 0x44 bytes.
- Behaviour: none (`ret 0x44`).
- Open questions: the exact original name is not provable from the bytes (behaviour is fully known).
