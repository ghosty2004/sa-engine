# CPedModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable 0x85BDC0 (gr PedModelInfo.cpp:6). Size 0x44. Field: +0x34 `CColModel* m_pHitColModel`.
Store `CModelInfo::ms_pedModelInfoStore` (0xB478F8, element 0x44, count 0x116).

## 0x4C57A0 CPedModelInfo::CPedModelInfo  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall CPedModelInfo* (this)`; plain `ret`; EAX = this. Preserves ESI.
- Callees: `orig!(0x4C4A60 CBaseModelInfo::CBaseModelInfo)` (sets vptr 0x85BB9C, u16 +0x8 m_nRefCount = 0,
  i16 +0xA m_nTxdIndex = -1; returns this).
- Callers: used as element constructor by the store initialiser 0x4C67D0 (via `eh vector constructor iterator`).
- Behaviour: CBaseModelInfo ctor; `vptr = 0x85BDC0`; `*(u32*)(this+0x34) = 0`; return this. No other field
  is written (the CClumpModelInfo part has an empty ctor).
- gta-reversed: PedModelInfo.h:48 `CPedModelInfo() : CClumpModelInfo(), m_pHitColModel(nullptr) {}` - matches.
- Open questions: none.

## 0x4C62F0 CPedModelInfo::~CPedModelInfo  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this)`; plain `ret`. Sets up an SEH/C++ EH frame (handler 0x83BC28, state
  variable set to 0 at 0x4C6319) only to protect the base-destructor call; no observable effect.
- Callees: `orig!(0x40F700 CColModel::~CColModel)` (thiscall), `orig!(0x40FC40 CColModel::operator delete)`
  (cdecl). Unwind funclet target 0x4C5750 (~CClumpModelInfo) only on exception.
- Behaviour:
  1. `vptr = 0x85BDC0;`
  2. `col = this->m_pHitColModel` (+0x34); if `col != NULL`: `col->~CColModel(); CColModel::operator delete(col);`
     (m_pHitColModel is NOT cleared).
  3. `vptr = 0x85BB9C` (inlined ~CClumpModelInfo -> ~CBaseModelInfo chain).
- Callers: 0x4C6350 (scalar deleting dtor), store initialiser 0x4C67D0 (as element dtor for EH unwinding),
  atexit destructor 0x8562B0 (via 0x4C6390).
- gta-reversed: PedModelInfo.h:49 `~CPedModelInfo() { delete m_pHitColModel; }` - same semantics.
- Open questions: none.

## 0x4C6350 CPedModelInfo::`scalar deleting destructor' (catalogue: CPedModelInfo::vfunc_0, specified/REA, port level 6)
- Convention: `__thiscall CPedModelInfo* (this, uint32 flags)`; `ret 4`; EAX = this.
- Behaviour: `~CPedModelInfo(this)` (0x4C62F0); if `(u8)flags & 1`: `operator delete(this)` (0x8214BD);
  return this. Name evidence: slot-0 pattern (CBaseModelInfo.md). Open questions: none.
