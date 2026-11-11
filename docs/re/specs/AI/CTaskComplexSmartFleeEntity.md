# CTaskComplexSmartFleeEntity (AI) - RE-SPEC-3 group I3

## 0x65D280 CTaskComplexSmartFleeEntity::`scalar deleting destructor'  (catalogue name CTaskComplexSmartFleeEntity::~CTaskComplexSmartFleeEntity)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86F784. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x65C4D0 (0x65D283): SEH; `vtbl = 0x86F784`; if [+0xC] (m_Entity) != 0 CleanUpOldReference; then ~CTaskComplex 0x61A3D0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x65D288, call at 0x65D290; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## 0x65C530 CTaskComplexSmartFleeEntity::CreateSubTask  (catalogue: specified/REA, port level 3)
(Added by the lead; the I3 writer left it unwritten.) Layout: +0xC m_fleeFrom, +0x10 m_pos, +0x1C m_time, +0x20 m_safeDistance,
+0x24 m_pedScream, +0x28 m_posCheckPeriod, +0x34 m_posCheckTimer.
- `__thiscall CTask* (this, int32 type)`, `ret 4`, SEH (handler push obfuscated at 0x401D31: `not`/`xor` of two dwords; no effect).
- 0xCB: `p = new(0x20)`; if p: `ms = _ftol(((float)(_rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 50.0f [0x858B40])` (drawn after the
  allocation) and `CTaskSimpleStandStill(p, ms, false, false, 8.0f)`; return p / 0.
- 0x38E (SMART_FLEE_POINT): timer start = now, interval = m_posCheckPeriod, started = 1; `m_pos = m_fleeFrom position` (no null check);
  `p = new(0x44)`; return p ? `CTaskComplexSmartFleePoint(p, &m_pos, (u8)m_pedScream, m_safeDistance, m_time)` 0x65BD20 : 0.
- 0x516 and other types: 0.
- gta-reversed (cpp:137): the 0xCB case is commented out as "dead code" - it exists in the original (whether any caller passes 0xCB
  was not checked: open question); the `default: NOTSA_UNREACHABLE()` row: Verdict CONFIRMED - the original returns 0.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x65D2C0 CTaskComplexSmartFleeEntity::~CTaskComplexSmartFleeEntity  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x65CD10 CTaskComplexFleeAnyMeans::Destructor (0x65D2C3); if `flags & 1` (0x65D2C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x65D2D0); returns this (0x65D2D8). Proposed name: `CTaskComplexFleeAnyMeans::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexFleeAnyMeans::Destructor).
