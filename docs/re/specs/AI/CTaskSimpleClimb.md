# CTaskSimpleClimb (AI) - RE-SPEC-AI-2 group J3

## 0x67A1C0 CTaskSimpleClimb::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xFE` (0x67A1C0); `ret` -> returns 254 (TASK_SIMPLE_CLIMB). Vtable: slot 4 (+0x10) of vtable 0x87059C (CTaskSimpleClimb).

## 0x67DBC0 CTaskSimpleClimb::~CTaskSimpleClimb  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleClimb::~CTaskSimpleClimb 0x67A1D0 (0x67DBC3); if `flags & 1` (0x67DBC8) calls CTask::operator delete 0x61A5B0 (this) (0x67DBD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x87059C (CTaskSimpleClimb).

## 0x67A1D0 CTaskSimpleClimb::~CTaskSimpleClimb  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, plain `ret`, SEH frame. vtbl = 0x87059C; if +0x24 (m_pClimbEnt) != NULL: CleanUpOldReference(&this->+0x24)
  0x571A00; if +0x2C (m_pAnim) != NULL: m_pAnim->SetDeleteCallback(0x4D1390 DefaultAnimCB, NULL) 0x4CEBC0 (DELETE callback, not
  finish) and +0x2C = 0; CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x67C610 CTaskSimpleClimb::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x30) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskSimpleClimb::CTaskSimpleClimb(this+0x24, &this->+0x18 (CVector*), this+0x14, (u8)this+0xF, (u8)this+0xD, (u8)this+0xB) 0x67A110; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
