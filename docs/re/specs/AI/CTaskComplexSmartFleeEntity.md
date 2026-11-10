# CTaskComplexSmartFleeEntity (AI) - RE-SPEC-3 group I3

## 0x65D280 CTaskComplexSmartFleeEntity::`scalar deleting destructor'  (catalogue name CTaskComplexSmartFleeEntity::~CTaskComplexSmartFleeEntity)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86F784. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x65C4D0 (0x65D283): SEH; `vtbl = 0x86F784`; if [+0xC] (m_Entity) != 0 CleanUpOldReference; then ~CTaskComplex 0x61A3D0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x65D288, call at 0x65D290; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
