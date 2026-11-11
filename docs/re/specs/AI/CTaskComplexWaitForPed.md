# CTaskComplexWaitForPed (AI) - RE-SPEC-FINAL (helper B)

## 0x683340 sub_683340 (proposed: CTaskComplexWaitForPed::CTaskComplexWaitForPed)  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x8706F8 (TASK_COMPLEX_WAIT_FOR_PED); callers vt_8706f8::Clone 0x683950, CTaskComplexBeInCouple::ControlSubTask 0x684930.
- `__thiscall T* (this, CPed* ped [esp+4], int32 a2 [esp+8], int32 a3 [esp+0xC], uint8 a4 [esp+0x10])`, `ret 0x10`; SEH frame.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x8706F8`; timer `+0x1C = 0; +0x20 = 0; +0x24 = 0 (u8); +0x25 = 0 (u8)`; `+0x10 = a2; +0x14 = a3; +0xC = ped; +0x18 = (u8)a4`; `CEntity::RegisterReference(ped, &this->+0xC)` 0x571B70 (not null-checked).

## 0x6833D0 sub_6833d0 (proposed: CTaskComplexWaitForPed::~CTaskComplexWaitForPed (non-deleting))  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x8706F8; caller 0x684780 (scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x8706F8`; `if +0xC (ped)` -> CleanUpOldReference(ped, &this->+0xC) 0x571A00; CTaskComplex::~CTaskComplex 0x61A3D0.
