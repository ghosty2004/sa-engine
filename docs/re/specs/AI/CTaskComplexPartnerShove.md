# CTaskComplexPartnerShove (AI) - RE-SPEC-AI-2 group J2

## 0x681E20 CTaskComplexPartnerShove::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4B9` (0x681E20); `ret` -> returns 1209 (TASK_COMPLEX_PARTNER_SHOVE). Vtable: slot 4 (+0x10) of vtable 0x870800 (CTaskComplexPartnerShove).

## 0x684FE0 CTaskComplexPartnerShove::~CTaskComplexPartnerShove  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00684750 0x684750 (0x684FE3); if `flags & 1` (0x684FE8) calls CTask::operator delete 0x61A5B0 (this) (0x684FF0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870800 (CTaskComplexPartnerShove).

## 0x684ED0 CTaskComplexPartnerShove::Clone  (catalogue: specified/REA, port level 4)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x74) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexPartnerShove::CTaskComplexPartnerShove(&this->+0x14 (name), this+0x38, (u8)this+0x58, this+0x3C, (int32)(int8)this+0x5B, CVector this+0x40 by value (12 bytes)) 0x6846F0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x683120 CTaskComplexPartnerShove::GetPartnerSequence  (catalogue: specified/REA, port level 3)
- `__thiscall CTaskComplexSequence* (this)`, plain `ret`; SEH frame. Caller: CTaskComplexPartner::CreateNextSubTask 0x683AD0.
- Behaviour: `seq = new(0x40) CTaskComplexSequence()` (NULL on failure, then AddTask on NULL as in 0x682440); `seq->AddTask(new(0x28) CTaskComplexTurnToFaceEntityOrCoord(partner (+0x38), 0.5f [0x86FC7C], 0.001f (0x3A83126F)))`; then `seq->AddTask(new(0x20) CTaskSimpleRunAnim(0, (+0x58 != 0) ? 0xA4 : 0x8E, 4.0f, false))`; return seq.
