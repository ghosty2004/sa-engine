# CTaskComplexGoToCarDoorAndStandStill (AI) - RE-SPEC-3 group I3

## 0x64D290 CTaskComplexGoToCarDoorAndStandStill::`scalar deleting destructor'  (catalogue name CTaskComplexGoToCarDoorAndStandStill::~CTaskComplexGoToCarDoorAndStandStill)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86EC4C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x64A580 (0x64D293): SEH; `vtbl = 0x86EC4C`; if [+0xC] (m_Vehicle) != 0 CleanUpOldReference; if [+0x4C] (m_RouteToDoor) != 0 `CPointRoute::operator delete` 0x41B7D0; then ~CTaskComplex 0x61A3D0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x64D298, call at 0x64D2A0; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
