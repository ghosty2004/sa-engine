# CTaskComplexEnterCarAsPassenger (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x640370 CTaskComplexEnterCarAsPassenger::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 700 (0x2BC) (0x640370).

## 0x643C80 CTaskComplexEnterCarAsPassenger::~CTaskComplexEnterCarAsPassenger  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x640380 FUN_00640380 (unnamed complete dtor: stores vtbl 0x86EADC) (0x643C83); if `flags & 1` (0x643C88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x643C90); returns this (0x643C98).

## 0x6437F0 CTaskComplexEnterCarAsPassenger::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x50) 0x61A5A0 (cdecl); if p == NULL the constructor is skipped and EAX = 0 (see the trailing stores). Else p->CTaskComplexEnterCar::CTaskComplexEnterCar(vehicle = this+0xC, 0, 0, 0, (this+0x10 >> 3) & 1) 0x63A220; p+0x1C = this+0x1C; vtbl = 0x86EADC (CTaskComplexEnterCarAsPassenger). Then OUTSIDE the NULL check (0x643851): p+0x24 = this+0x24. Original bug: on allocation failure EAX = 0 and the store hits address 0x24 - keep; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x640380 CTaskComplexEnterCarAsPassenger::~CTaskComplexEnterCarAsPassenger  (catalogue: unknown/REA, port level 3)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86EADC` (0x640380); tail-jump to base dtor CTaskComplexEnterCar::Destructor 0x63DFA0 (0x640386). No other member cleanup.
