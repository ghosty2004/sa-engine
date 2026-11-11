# CTaskComplexPartnerShove (AI) - RE-SPEC-AI-2 group J2

## 0x681E20 CTaskComplexPartnerShove::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4B9` (0x681E20); `ret` -> returns 1209 (TASK_COMPLEX_PARTNER_SHOVE). Vtable: slot 4 (+0x10) of vtable 0x870800 (CTaskComplexPartnerShove).

## 0x684FE0 CTaskComplexPartnerShove::~CTaskComplexPartnerShove  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00684750 0x684750 (0x684FE3); if `flags & 1` (0x684FE8) calls CTask::operator delete 0x61A5B0 (this) (0x684FF0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870800 (CTaskComplexPartnerShove).
