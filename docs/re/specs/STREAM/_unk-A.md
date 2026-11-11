# STREAM: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x406450 CdStreamGetLastPosn (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__cdecl u32 ()`, EAX = [0x8e4898] gLastCdStreamPosn (0x1567540). Caller CStreaming::RequestModelStream.
- gta-reversed: `CdStreamGetLastPosn()` (CdStreamInfo.h:94).

## 0x44d2d0 empty stub (ret 0x30) (proposed)  (catalogue: unknown/REA, port level 0)
- `ret 0x30` only: callee-cleaned 12-dword argument list, no effect. Port: no-op with the same signature.

## 0x44d2e0 empty stub (ret 0x38) (proposed)  (catalogue: unknown/REA, port level 0)
- `ret 0x38` only (14 dwords). Port: no-op.

## 0x4c4a60 CBaseModelInfo::CBaseModelInfo (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall CBaseModelInfo* ()`, EAX = this: `[+0] = 0x85bb9c`; `word[+8] = 0` (m_nRefCount); `word[+0xa] = 0xffff` (m_nTxdIndex = -1). Nothing else initialised (gr ctor initialises more, Models/BaseModelInfo.cpp - port keeps original).

## 0x4c5540 CAtomicModelInfo::CAtomicModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall`: `CBaseModelInfo::CBaseModelInfo(this)` 0x4c4a60; `[this] = 0x85bbf0`; EAX = this.

## 0x4c55a0 CDamageAtomicModelInfo::CDamageAtomicModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- As 0x4c5540 with vtable 0x85bc30 (no CAtomicModelInfo ctor call; the base ctor + final vtable only).

## 0x4c55e0 CLodAtomicModelInfo::CLodAtomicModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- `CBaseModelInfo::CBaseModelInfo(this)`; `word[+0x20] = 0; word[+0x22] = 0`; `[this] = 0x85bc70`; EAX = this.

## 0x4c5640 CTimeModelInfo::CTimeModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- `CBaseModelInfo::CBaseModelInfo(this)`; `[this] = 0x85bcb0`; `word[+0x22] = 0xffff`; EAX = this.

## 0x4c5690 CLodTimeModelInfo::CLodTimeModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- `CBaseModelInfo::CBaseModelInfo(this)`; `word[+0x20] = 0; word[+0x22] = 0`; `[this] = 0x85bcf0`; `word[+0x26] = 0xffff`; EAX = this.

## 0x4c56f0 CClumpModelInfo::CClumpModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- As 0x4c5540 with vtable 0x85bd30.

## 0x4c5760 CWeaponModelInfo::CWeaponModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- As 0x4c5540 with vtable 0x85bd78.

## 0x4c5cb0 CStore<CAtomicModelInfo,14000>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CStore* ()`, EAX = this: `eh_vector_constructor_iterator(this+4, 0x20, 0x36b0 (14000), ctor 0x4c5540, dtor 0x4c5590)` 0x821d30; then `[this] = 0` (m_nCount, after the array).

## 0x4c5d10 CStore<CDamageAtomicModelInfo,70>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x24, count 0x46 (70), ctor 0x4c55a0, dtor 0x4c55d0.

## 0x4c5d70 CStore<CLodAtomicModelInfo,1>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x24, count 1, ctor 0x4c55e0, dtor 0x4c5620.

## 0x4c5dd0 CStore<CTimeModelInfo,169>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x24, count 0xa9 (169), ctor 0x4c5640, dtor 0x4c5680.

## 0x4c5e30 CStore<CLodTimeModelInfo,1>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x28, count 1, ctor 0x4c5690, dtor 0x4c56e0.

## 0x4c5e90 CStore<CWeaponModelInfo,51>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x28, count 0x33 (51), ctor 0x4c5760, dtor 0x4c5790.

## 0x4c5ef0 CStore<CClumpModelInfo,92>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x24, count 0x5c (92), ctor 0x4c56f0, dtor 0x4c5750.

## 0x4c5f50 CStore<CVehicleModelInfo,212>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x308, count 0xd4 (212), ctor 0x4c75e0, dtor 0x4c5920.

## 0x4c67d0 CStore<CPedModelInfo,278>::CStore (proposed)  (catalogue: unknown/REA, port level 0)
- As 0x4c5cb0 with element size 0x44, count 0x116 (278), ctor 0x4c57a0, dtor 0x4c62f0.

## 0x4c73f0 UpgradePosnDesc::UpgradePosnDesc (trivial) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = this, no stores (0x4c73f0).

## 0x4c7400 UpgradePosnDesc::~UpgradePosnDesc (trivial) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x4c7400). Port: no-op.

## 0x4c7410 CVehicleModelInfo::CVehicleStructure::~CVehicleStructure (proposed)  (catalogue: unknown/REA, port level 7)
- `__thiscall void ()`.
- `for (i = 0; i < (i8)m_nNumExtras [+0x30c]; ++i)` (count re-read and sign-extended each iteration, 0x4c7439): `a = m_apExtras[i] (+0x2f4 + 4i)`; `f = a->object.parent ([a+4])`; `RpAtomicDestroy(a)` 0x749dc0; `RwFrameDestroy(f)` 0x7f05a0.
- `eh_vector_destructor_iterator(this+0xb4, 0x20, 0x12, 0x4c7400)` 0x821e02 (m_aUpgrades, trivial dtor). Pointers are not cleared.

## 0x4c75e0 CVehicleModelInfo::CVehicleModelInfo (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall CVehicleModelInfo* ()`, EAX = this.
- `CBaseModelInfo::CBaseModelInfo(this)` 0x4c4a60; `[+0x5c] = 0` (m_pVehicleStruct); `byte[+0x2d0] = 0`; `byte[+0x4e] = 0`; `[this] = 0x85c5c8`; `[+0x304] = -1`; `rep stosd` 9 dwords of -1 at +0x2d6 (unaligned; 18 i16 upgrade ids = -1); `[+0x2fa] = -1`, `[+0x2fe] = -1` (four more i16 = -1).
- Open question: names of bytes +0x4e and +0x2d0 (0x4c75ee/0x4c75f4).

## 0x537290 CloneAtomicToClumpCB (proposed)  (catalogue: specified/REA, port level 7)
- `__cdecl RpAtomic* (RpAtomic* atomic, void* clump)`: `c = RpAtomicClone(atomic)` 0x749e60; `RpAtomicSetFrame(c, *(RwFrame**)(atomic->parent [+4] + 0xa0))` 0x74bf20; `RpClumpAddAtomic(clump, c)` 0x74a490; return atomic. No null checks.
- Open question: what the frame plugin slot +0xa0 holds (frame -> frame link used by LoadClumpFile) (0x5372a1).
