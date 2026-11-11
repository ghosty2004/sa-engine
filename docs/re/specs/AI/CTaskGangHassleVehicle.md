# CTaskGangHassleVehicle (AI) - RE-SPEC-AI-2 group J3

## 0x65FB50 CTaskGangHassleVehicle::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4B5` (0x65FB50); `ret` -> returns 1205 (TASK_COMPLEX_GANG_HASSLE_VEHICLE). Vtable: slot 4 (+0x10) of vtable 0x86F9D4 (CTaskGangHassleVehicle).

## 0x6637A0 CTaskGangHassleVehicle::~CTaskGangHassleVehicle  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskGangHassleVehicle::Destructor 0x65FB60 (0x6637A3); if `flags & 1` (0x6637A8) calls CTask::operator delete 0x61A5B0 (this) (0x6637B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F9D4 (CTaskGangHassleVehicle).

## 0x65FC00 CTaskGangHassleVehicle::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x44) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskGangHassleVehicle::CTaskGangHassleVehicle(this+0x10, this+0x14, (u8)this+0x18, this+0x1C, this+0x20) 0x65FAC0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
