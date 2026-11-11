# CTaskComplexFollowPedFootsteps (AI) - RE-SPEC-AI-2 group J2

## 0x694EC0 CTaskComplexFollowPedFootsteps::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3A8` (0x694EC0); `ret` -> returns 936 (TASK_COMPLEX_FOLLOW_PED_FOOTSTEPS). Vtable: slot 4 (+0x10) of vtable 0x870CC0 (CTaskComplexFollowPedFootsteps).

## 0x694ED0 CTaskComplexFollowPedFootsteps::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)`: forwards to `this->m_pSubTask` (+0x8, no null check, 0x694ED0) virtual slot +0x18 (= MakeAbortable) via tail-jump 0x694ED5 with the same stack args, so returns `m_pSubTask->MakeAbortable(ped, priority, event)`. Vtable: slot 6 (+0x18) of vtable 0x870CC0 (CTaskComplexFollowPedFootsteps).

## 0x696260 CTaskComplexFollowPedFootsteps::~CTaskComplexFollowPedFootsteps  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00695dc0 0x695DC0 (0x696263); if `flags & 1` (0x696268) calls CTask::operator delete 0x61A5B0 (this) (0x696270, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870CC0 (CTaskComplexFollowPedFootsteps).
