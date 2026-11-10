# CTaskComplexFacial (AI) - RE-SPEC-3 group I3

Vtable 0x870ADC.

## 0x690D90 CTaskComplexFacial::~CTaskComplexFacial  (catalogue: specified/REA, port level 2)
- `__thiscall void (this)`: complete-object dtor: `vtbl = 0x870ADC; jmp 0x61A3D0` (~CTaskComplex: `vtbl = 0x86D4CC`, deletes m_pSubTask
  via slot 0 with flags 1, then CTask vtbl). gr TaskComplexFacial.h:18 `= default` - matches.

## 0x692F90 CTaskComplexFacial::`scalar deleting destructor'  (catalogue name ~CTaskComplexFacial (second row))  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x870ADC. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x690D90 (0x692F93): see above. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x692F98, call at 0x692FA0; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
