# CTaskComplexAvoidOtherPedWhileWandering (AI) - RE-SPEC-3 group I3

## 0x66F680 CTaskComplexAvoidOtherPedWhileWandering::`scalar deleting destructor'  (catalogue name CTaskComplexAvoidOtherPedWhileWandering::~CTaskComplexAvoidOtherPedWhileWandering)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86FEC4. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x66A1D0 (0x66F683): SEH; `vtbl = 0x86FEC4`; if [+0xC] (m_OtherPed) != 0 `CEntity::CleanUpOldReference([+0xC], this+0xC)` 0x571A00; then ~CTaskComplex 0x61A3D0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x66F688, call at 0x66F690; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
