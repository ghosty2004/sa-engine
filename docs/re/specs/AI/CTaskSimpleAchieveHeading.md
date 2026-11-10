# CTaskSimpleAchieveHeading (AI) - RE-SPEC-3 group I3

Vtable 0x86FD74 (gr TaskSimpleAchieveHeading.h, size 0x18): +8 m_DesiredHeadingRad, +0xC m_HeadingChangeRate, +0x10 m_HeadingToleranceRad (f32),
+0x14 bit0 m_IsDoingIK.

## 0x667E20 CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading  (catalogue: specified/REA, port level 1)
- `__thiscall this (float desiredHeading [esp+4], float changeRate [esp+8], float tolerance [esp+0xC])`, `ret 0xC`, returns this. 22 callers.
- Behaviour: `CTaskSimple::CTaskSimple(this)` 0x61A390; `+0x14 &= 0xFE` (clears only bit0; other bits of the pool byte are kept,
  0x667E28-0x667E35); `+8 = desiredHeading`, `vtbl = 0x86FD74`, `+0xC = changeRate`, `+0x10 = tolerance` (raw dword copies, 0x667E3C-0x667E48).
- gta-reversed: TaskSimpleAchieveHeading.cpp:17 - matches (bitfield init clears bit0 only).

## 0x667E70 CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`: `vtbl = 0x86FD74; jmp 0x61A3A0` (~CTaskSimple). gr `= default` - matches.

## 0x66DB60 CTaskSimpleAchieveHeading::`scalar deleting destructor'  (catalogue name ~CTaskSimpleAchieveHeading (second row))  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x86FD74. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x667E70 (0x66DB63): see above. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x66DB68, call at 0x66DB70; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
