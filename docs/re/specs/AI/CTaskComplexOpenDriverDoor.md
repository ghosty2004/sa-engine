# CTaskComplexOpenDriverDoor (AI) - RE-SPEC-AI-2 group J2

## 0x6403C0 CTaskComplexOpenDriverDoor::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2D2` (0x6403C0); `ret` -> returns 722 (TASK_COMPLEX_CAR_OPEN_DRIVER_DOOR). Vtable: slot 4 (+0x10) of vtable 0x86EB0C.

## 0x643CA0 CTaskComplexOpenDriverDoor::~CTaskComplexOpenDriverDoor  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006403d0 0x6403D0 (0x643CA3); if `flags & 1` (0x643CA8) calls CTask::operator delete 0x61A5B0 (this) (0x643CB0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86EB0C.

## 0x643870 CTaskComplexOpenDriverDoor::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x50) 0x61A5A0 (cdecl); if p == NULL the constructor is skipped and EAX = 0 (see the trailing stores). Else p->CTaskComplexEnterCar::CTaskComplexEnterCar(this+0xC, 1, 1, 0, 0) 0x63A220; vtbl = 0x86EB0C (CTaskComplexOpenDriverDoor); p+0x1C = 0xA. Then outside the NULL check: p+0x24 = this+0x24 (same NULL-write bug as 0x6437F0); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
