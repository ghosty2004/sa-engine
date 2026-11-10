# CTaskComplexEnterCarAsDriver (AI) - RE-SPEC-3 group I3

Vtable 0x86EAAC (derives from CTaskComplexEnterCar, vtable 0x86E6F0).

## 0x640320 CTaskComplexEnterCarAsDriver::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x2BD; ret` -> 701 (TASK_COMPLEX_ENTER_CAR_AS_DRIVER). Slot 4 of 0x86EAAC. gr matches.

## 0x643C60 CTaskComplexEnterCarAsDriver::`scalar deleting destructor'  (catalogue name CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver)  (catalogue: specified/REA, port level 4)
- NAME CORRECTION: slot 0 of vtable 0x86EAAC. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x640330 (0x643C63): `vtbl = 0x86EAAC; jmp 0x63DFA0` (CTaskComplexEnterCar::~CTaskComplexEnterCar: `vtbl = 0x86E6F0`, cleans the [+0xC] vehicle reference, ...; its SEH prologue is obfuscated by the no-CD build via 0x401148, no game-visible effect). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x643C68, call at 0x643C70; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
