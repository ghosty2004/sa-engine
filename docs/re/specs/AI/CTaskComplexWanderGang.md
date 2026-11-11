# CTaskComplexWanderGang (AI) - RE-SPEC-3 group I3

Vtable 0x8700D4 (gr TaskComplexWanderGang.h, size 0x38). CTaskComplexWander part (size 0x28): +0xC m_nMoveState (i32), +0x10 m_nDir (u8),
+0x14 m_fTargetRadius (f32), +0x18 m_LastNode, +0x1C m_NextNode (CNodeAddress {u16 area, u16 node}), +0x20 m_nLastUpdateDirFrameCount,
+0x24 flags (bit0 m_bWanderSensibly, bit1 m_bNewDir, bit2 m_bNewNodes, bit3 m_bAllNodesBlocked). Own: +0x28 m_TaskTimer (CTaskTimer:
+0x28 start, +0x2C interval, +0x30 started, +0x31 stopped), +0x34 m_NextScanTime.

## 0x66F5C0 CTaskComplexWanderGang::CTaskComplexWanderGang  (catalogue: specified/REA, port level 1)
- `__thiscall this (eMoveState moveState [esp+4], uint8 dir [esp+8], uint32 scanTime [esp+0xC], bool bWanderSensibly [esp+0x10],
  float targetRadius [esp+0x14])`, `ret 0x14`, returns this. 7 callers (CGangWars::TellGangMembersTo 0x444530, group task allocators, ...).
- Behaviour (CTaskComplexWander ctor INLINED, not called): `CTaskComplex::CTaskComplex(this)` 0x61A3B0; `+0xC = moveState`; byte `+0x10 = dir`;
  `+0x14 = targetRadius` (raw dword copy); WORD `+0x18 = 0xFFFF` and WORD `+0x1C = 0xFFFF` (only the area halves; the node halves at +0x1A/+0x1E
  are NOT written, 0x66F5E6-0x66F5EA); `+0x20 = 0`; `vtbl = 0x8700D4`; `+0x24 = (+0x24 & 0xF0) | (bWanderSensibly & 1)` (read-modify-write of
  the old byte: bits 4-7 keep pool garbage, 0x66F5EE-0x66F604); timer `+0x28 = 0, +0x2C = 0, +0x30 = 0, +0x31 = 0`; `+0x34 = scanTime`;
  then `+0x2C = scanTime`, `+0x28 = CTimer::m_snTimeInMilliseconds [0xB7CB84]`, `+0x30 = 1` (timer started, 0x66F613-0x66F626).
- gta-reversed: TaskComplexWanderGang.cpp:5 calls the CTaskComplexWander ctor (which may initialise more, e.g. full CNodeAddress); verify the
  port initialises exactly the bytes above (node halves left as-is).

## 0x66F630 CTaskComplexWanderGang::GetWanderType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,4; ret` -> 4 (WANDER_TYPE_GANG). Slot 11 of 0x8700D4. gr matches.

## 0x66F640 CTaskComplexWanderGang::ScanForStuff  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, CPed* ped [esp+4])`: `ret 4` only (no-op). Slot 12. gr matches.

## 0x66F650 CTaskComplexWanderGang::`scalar deleting destructor'  (catalogue name CTaskComplexWanderGang::~CTaskComplexWanderGang)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x8700D4. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x66F670 (0x66F653): `vtbl = 0x86FE84` (the CTaskComplexWander vtable; CTaskComplexWanderGang has no dtor of its own) `; jmp 0x61A3D0` (~CTaskComplex). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x66F658, call at 0x66F660; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## 0x66F670 CTaskComplexWanderGang::~CTaskComplexWanderGang  (catalogue: unknown/REA, port level 2)
- Convention: `__thiscall void (this)`. Body: `vtable = 0x86FE84` (the CTaskComplexWander base vtable - WanderGang has nothing of its own to
  destroy), then tail-jump to CTaskComplex::~CTaskComplex 0x61A3D0. Byte-for-byte the same work as 0x669D20 (a second compiler copy).
- gta-reversed: `= default` - matches. Open questions: none.
