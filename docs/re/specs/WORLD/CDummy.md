# CDummy (WORLD)

CDummy = CEntity (0x38 bytes) with type DUMMY; vtable 0x8638c0. Pool: `CPools::ms_pDummyPool` 0xb744a0,
element size 0x38. CPool header: `+0 m_pObjects`, `+4 m_byteMap` (u8 per slot, bit 0x80 = free),
`+8 m_nSize`, `+0xc m_nFirstFree`.

Pool delete sequence (used by 0x5326f0 and 0x532770; identical code in other `vfunc_0`s with a different
pool and element size): `idx = (int32)(p - pool->m_pObjects) / 0x38` (signed truncating division:
`imul 0x92492493; add edx,ecx; sar edx,5; add sign`), `pool->m_byteMap[idx] |= 0x80`,
`if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (signed compare).

## 0x532540 CDummy::CDummy  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall CDummy* (this = ECX)`, returns this in EAX, plain `ret`.
- Callers: 3 (`CDummyObject::CDummyObject` 0x59ea00 and 0x59ea20, 0x5de2e0). Callees:
  `CEntity::Constructor` 0x532a90.
- Behaviour: `CEntity::Constructor(this)`; then `b = byte[this+0x36]; b = (b & 0xfd) | 5;
  byte[this+0x36] = b` (CEntityInfo low 3 bits = type ← 5 = ENTITY_TYPE_DUMMY, status bits untouched);
  then `vtable = 0x8638c0` (after the type write).
- gta-reversed: provided (Dummy.cpp:22, `SetTypeDummy()`), equivalent.
- Open questions: none.

## 0x5326f0 CDummy::operator delete  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl void (void* p [esp+4])`, plain `ret` (caller pops).
- Callers: 3 (eh-unwind in `CFileLoader::LoadObjectInstance` 0x538090, `CDummyObject::vfunc_0`
  0x59eb50, 0x5de310). Callees: none. Globals: read `CPools::ms_pDummyPool` 0xb744a0.
- Behaviour: the pool delete sequence above with `p`. No null check (`p == NULL` computes a garbage
  index and writes the byte map out of range — original behaviour, callers never pass NULL).
- gta-reversed: provided (Dummy.cpp:17, `GetDummyPool()->Delete`), equivalent.
- Open questions: none.

## 0x532770 CDummy::vfunc_0 (= scalar deleting destructor `CDummy::~CDummy` + delete)  (catalogue: specified/REA, port level 14)
- Convention / signature: `__thiscall CDummy* (this = ECX, u32 flags [esp+4])`, returns this in EAX,
  `ret 4`. Vtable slot 0 of 0x8638c0; no direct callers.
- Callees: `0x532560` (CDummy::~CDummy: `vtable = 0x8638c0; jmp CEntity::Destructor 0x535e90`).
- Behaviour: `0x532560(this)`; if `flags & 1`: inlined pool delete (sequence above, pool
  `ms_pDummyPool` read after the destructor). Return this.
- gta-reversed: compiler generated; equivalent.
- Open questions: none.
