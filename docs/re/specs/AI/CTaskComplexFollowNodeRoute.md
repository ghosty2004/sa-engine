# CTaskComplexFollowNodeRoute (AI) - RE-SPEC-3 group I3

## 0x671730 CTaskComplexFollowNodeRoute::`scalar deleting destructor'  (catalogue name CTaskComplexFollowNodeRoute::~CTaskComplexFollowNodeRoute)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x8700A8. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x66EB70 (0x671733): SEH; `vtbl = 0x8700A8`; if [+0x30] != 0 `CPointRoute::operator delete([+0x30])` 0x41B7D0; if [+0x2C] != 0 `CNodeRoute::operator delete([+0x2C])` 0x41B870 (point route first); then ~CTaskComplex 0x61A3D0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x671738, call at 0x671740; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
