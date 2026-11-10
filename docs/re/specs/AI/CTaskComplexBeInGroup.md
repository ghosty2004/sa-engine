# CTaskComplexBeInGroup (AI) - RE-SPEC-3 group I3

## 0x638A80 CTaskComplexBeInGroup::`scalar deleting destructor'  (catalogue name CTaskComplexBeInGroup::~CTaskComplexBeInGroup)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86E22C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x632EA0 (0x638A83): `vtbl = 0x86E22C; jmp 0x61A3D0` (~CTaskComplex: deletes m_pSubTask via its slot 0 with flags 1). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x638A88, call at 0x638A90; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
