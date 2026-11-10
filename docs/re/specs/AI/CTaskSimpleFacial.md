# CTaskSimpleFacial (AI) - RE-SPEC-3 group I3

Vtable 0x870AB8.

## 0x690CB0 CTaskSimpleFacial::~CTaskSimpleFacial  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`: `vtbl = 0x870AB8; jmp 0x61A3A0` (~CTaskSimple). gr TaskSimpleFacial.h:39 `= default` - matches.

## 0x692E30 CTaskSimpleFacial::`scalar deleting destructor'  (catalogue name ~CTaskSimpleFacial (second row))  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x870AB8. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x690CB0 (0x692E33): see above. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x692E38, call at 0x692E40; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
