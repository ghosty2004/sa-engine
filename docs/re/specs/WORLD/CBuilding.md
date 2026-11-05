# CBuilding (WORLD)

Pool: `CPools::ms_pBuildingPool` 0xb74498, element size 0x38 (pool header layout as in CDummy.md).

## 0x404130 CBuilding::vfunc_0 (scalar deleting destructor)  (catalogue: specified/REA, port level 14)
- Convention / signature: `__thiscall CBuilding* (this = ECX, u32 flags [esp+4])`, returns this, `ret 4`.
  Vtable slot 0; no direct callers.
- Callees: 0x404180 (CBuilding dtor, below).
- Behaviour: `0x404180(this)`; if `flags & 1`: inlined pool delete on `ms_pBuildingPool` (read after
  the dtor): `idx = (int32)(this - pool->m_pObjects) / 0x38` (signed, magic 0x92492493, `sar 5`),
  `byteMap[idx] |= 0x80`, `if (idx < firstFree) firstFree = idx`. Return this.
- Open questions: none.

## 0x404180 CBuilding::~CBuilding (proposed name; catalogue: unknown/REA, port level 13)
- Evidence: single instruction `jmp CEntity::Destructor (0x535e90)`; only caller is the CBuilding scalar
  deleting destructor 0x404130. CBuilding adds no destructor work and does not reset the vtable
  (CEntity's destructor sets its own).
- Convention: `__thiscall void (this = ECX)`, tail call. Open questions: none.
