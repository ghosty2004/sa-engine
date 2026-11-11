# CTaskLeanOnVehicle (AI) - RE-SPEC-AI-2 group J3

## 0x661020 CTaskLeanOnVehicle::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4C1` (0x661020); `ret` -> returns 1217 (TASK_COMPLEX_LEAN_ON_VEHICLE). Vtable: slot 4 (+0x10) of vtable 0x86FAF4 (CTaskLeanOnVehicle).

## 0x6648A0 CTaskLeanOnVehicle::~CTaskLeanOnVehicle  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskLeanOnVehicle::Destructor 0x661030 (0x6648A3); if `flags & 1` (0x6648A8) calls CTask::operator delete 0x61A5B0 (this) (0x6648B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FAF4 (CTaskLeanOnVehicle).

## 0x6610A0 CTaskLeanOnVehicle::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x2C) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskLeanOnVehicle::CTaskLeanOnVehicle(this+8, this+0xC, (u8)this+0x10) 0x660F90; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
