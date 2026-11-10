# CTaskSimpleStandStill (AI) - RE-SPEC-3 group I3

Vtable 0x86DD2C (gr TaskSimpleStandStill.h, size 0x20): +8 m_nTime (i32), +0xC m_timer (CTaskTimer: +0xC start, +0x10 interval, +0x14 started,
+0x15 stopped), +0x18 m_bLooped (u8), +0x19 m_bUseAnimIdleStance (u8), +0x1C m_fBlendData (f32).

## 0x62F350 CTaskSimpleStandStill::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0xCB; ret` -> 203 (TASK_SIMPLE_STAND_STILL). Slot 4. gr matches.

## 0x635CF0 CTaskSimpleStandStill::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (const this)`, plain `ret`, SEH frame. Slot 1.
- Behaviour: `p = CTask::operator new(0x20)` 0x61A5A0 (0x635D0F); if p == 0 return 0 (0x635D84-0x635D8A). Else the ctor is INLINED: source
  fields are read first (+0x19, +0x1C, +0x18, +8, 0x635D27-0x635D30), `CTaskSimple::CTaskSimple(p)` 0x61A390, then `p+8 = m_nTime`,
  `vtbl = 0x86DD2C`, timer `+0xC = 0, +0x10 = 0, +0x14 = 0, +0x15 = 0`, `+0x18 = m_bLooped`, `+0x19 = m_bUseAnimIdleStance`,
  `+0x1C = m_fBlendData` (raw dword) (0x635D52-0x635D6E). Returns p. The source timer state is NOT copied.
- gta-reversed: TaskSimpleStandStill.h:28 `new CTaskSimpleStandStill(m_nTime, m_bLooped, m_bUseAnimIdleStance, m_fBlendData)` - matches.

## 0x637480 CTaskSimpleStandStill::`scalar deleting destructor'  (catalogue name CTaskSimpleStandStill::~CTaskSimpleStandStill)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x86DD2C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x62F360 (0x637483): `vtbl = 0x86DD2C; jmp 0x61A3A0` (~CTaskSimple). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x637488, call at 0x637490; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
