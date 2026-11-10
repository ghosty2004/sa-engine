# CTaskComplexInAirAndLand (AI) - RE-SPEC-3 group I3

Vtable 0x8704AC.

## 0x678CB0 CTaskComplexInAirAndLand::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0xF0; ret` -> 240 (TASK_COMPLEX_IN_AIR_AND_LAND). Slot 4. gr matches.

## 0x67CC10 CTaskComplexInAirAndLand::`scalar deleting destructor'  (catalogue name CTaskComplexInAirAndLand::~CTaskComplexInAirAndLand)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x8704AC. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x678CC0 (0x67CC13): `vtbl = 0x8704AC; jmp 0x61A3D0` (~CTaskComplex). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x67CC18, call at 0x67CC20; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
