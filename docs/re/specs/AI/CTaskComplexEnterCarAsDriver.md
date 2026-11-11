# CTaskComplexEnterCarAsDriver (AI) - RE-SPEC-3 group I3

Vtable 0x86EAAC (derives from CTaskComplexEnterCar, vtable 0x86E6F0).

## 0x640320 CTaskComplexEnterCarAsDriver::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x2BD; ret` -> 701 (TASK_COMPLEX_ENTER_CAR_AS_DRIVER). Slot 4 of 0x86EAAC. gr matches.

## 0x643C60 CTaskComplexEnterCarAsDriver::`scalar deleting destructor'  (catalogue name CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver)  (catalogue: specified/REA, port level 4)
- NAME CORRECTION: slot 0 of vtable 0x86EAAC. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x640330 (0x643C63): `vtbl = 0x86EAAC; jmp 0x63DFA0` (CTaskComplexEnterCar::~CTaskComplexEnterCar: `vtbl = 0x86E6F0`, cleans the [+0xC] vehicle reference, ...; its SEH prologue is obfuscated by the no-CD build via 0x401148, no game-visible effect). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x643C68, call at 0x643C70; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x643780 CTaskComplexEnterCarAsDriver::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. Slot 1 of vtable 0x86EAAC.
- Behaviour: `p = CTask::operator new(0x50)`; if p != NULL: `CTaskComplexEnterCar(p, this->+0xC (vehicle), 1, 0, 0, 0)` 0x63A220, `p->vtbl = 0x86EAAC`, EAX = p. Then, OUTSIDE the NULL check (0x6437CC): `EAX->+0x24 = this->+0x24`. Original bug (keep): when operator new returns NULL, EAX = 0 and the store writes to address 0x24 (access violation). Returns EAX.
