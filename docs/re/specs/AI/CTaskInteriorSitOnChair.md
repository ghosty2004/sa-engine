# CTaskInteriorSitOnChair (AI) - RE-SPEC-FINAL (helper B)

## 0x675C30 CTaskInteriorSitOnChair::CTaskInteriorSitOnChair  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this, a1 [esp+4], a2 [esp+8], uint8 a3 [esp+0xC])`, `ret 0xC`. Callers: Clone 0x675CF0, CTaskInteriorUseInfo::CreateNextSubTask 0x676880.
- Behaviour: CTaskSimple ctor 0x61A390; `vtbl = 0x870314` (slot 4 returns TASK_INTERIOR_SIT_ON_CHAIR); `+0x1C = 0; +0x20 = 0; +0x24 = 0 (u8); +0x25 = 0 (u8)` (CTaskTimer); `+8 = a1; +0x14 = 0 (m_pAnim); +0x29 = 0; +0x28 = 0; +0x2A = 0 (u8s); +0xC = a2; +0x10 = (u8)a3; +0x18 = -1`; return this.

## 0x675C90 CTaskInteriorSitOnChair::~CTaskInteriorSitOnChair  (catalogue: unknown/REA, port level 1)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame. Caller: 0x676D10 (scalar deleting dtor).
- Behaviour: `vtbl = 0x870314`; `if +0x14 (m_pAnim)` -> `m_pAnim->SetDeleteCallback(0x4D1390 (empty), NULL)` 0x4CEBC0; CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x675DD0 CTaskInteriorSitOnChair::sub_675DD0 (proposed: CTaskInteriorSitOnChair::FinishAnimCB)  (catalogue: unknown/REA, port level 0)
- `static void __cdecl (CAnimBlendAssociation* assoc [esp+4], void* data [esp+8] = CTaskInteriorSitOnChair*)`, plain `ret`. Referenced 5 times by vt_870314::ProcessPed 0x676D30.
- Behaviour: `id = (int32)(int16)assoc->m_AnimId (+0x2C)`; `task->+0x18 = id`; `if id == 0x158` -> `assoc->m_BlendDelta (+0x1C) = -1000.0f`, `task->+0x29 = 1`. Then `if task->+0x2A (abort requested) != 0 && task->+0x18 == 0x157` -> same two stores. Finally `task->+0x14 (m_pAnim) = 0`.
