# CTaskSimpleSwim (AI) - RE-SPEC-AI-2 group J4

## 0x6889F0 CTaskSimpleSwim::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x518` (0x6889F0); `ret` -> returns 1304 (TASK_SIMPLE_SWIM). Vtable: slot 4 (+0x10) of vtable 0x870920 (CTaskSimpleSwim).

## 0x688A00 CTaskSimpleSwim::~CTaskSimpleSwim  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`, SEH frame: vtbl = 0x870920 (0x688A1D); if `+0x9` (m_bAnimBlockRefAdded) != 0:
  `RemoveAnimBlockRef(((ms_aAnimAssocGroups 0xB4EA34) + 0x58C)->m_pAnimBlock - ms_aAnimBlocks 0xB5D4A0) >> 5)` 0x4D3FD0 (assoc group 71, 0x688A32-0x688A46;
  the flag is not cleared); if +0x38 != 0 `CleanUpOldReference(&+0x38)` 0x571A00; if +0x20 != 0 `CleanUpOldReference(&+0x20)`; CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x68B0E0 CTaskSimpleSwim::~CTaskSimpleSwim  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleSwim::~CTaskSimpleSwim 0x688A00 (0x68B0E3); if `flags & 1` (0x68B0E8) calls CTask::operator delete 0x61A5B0 (this) (0x68B0F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870920 (CTaskSimpleSwim).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x68B050 CTaskSimpleSwim::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. `p = CTask::operator new(0x64)`; if NULL return 0; else `CTaskSimpleSwim::CTaskSimpleSwim(p, &this->+0x14 (const CVector* pos), this->+0x20 (CPed*))` 0x688930; return its EAX.
