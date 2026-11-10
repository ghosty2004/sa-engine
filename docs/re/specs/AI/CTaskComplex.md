# CTaskComplex and complex-task deleting destructors (AI) - RE-SPEC-3 group I1

CTask vtable slots (gr Task.h / TaskComplex.h): 0 dtor, 1 Clone, 2 GetSubTask, 3 IsSimple, 4 GetTaskType, 5 StopTimer,
6 MakeAbortable(ped, priority, event), 7 SetSubTask, 8 CreateNextSubTask, 9 CreateFirstSubTask, 10 ControlSubTask.
Layout: +0 vtbl, +4 m_Parent (CTaskComplex*), +8 m_pSubTask (CTask*). CTaskComplex vtable 0x86D4CC; its complete dtor is
0x61A3D0 (SEH; `vtbl = 0x86D4CC`; `if (m_pSubTask) m_pSubTask->vtbl[0](1)`; then CTask vtbl).
`CTask::operator delete` 0x61A5B0 (cdecl): inlined CPool::Delete on `CPools::ms_pTaskPool` [0xB744A8] with stride 0x80:
`idx = (obj - objects) >> 7` (arithmetic `sar`, not a divide), `byteMap[idx] |= 0x80`, `if (idx < firstFree) firstFree = idx`.

## 0x421190 CTaskComplex::GetSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* ()`: `return m_pSubTask` (+8). Slot 2, shared by all complex tasks. gr matches.

## 0x4211A0 CTaskComplex::IsSimple  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`: `xor al,al` -> false. Slot 3. gr matches.

## 0x4211B0 CTaskComplex::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)` (3 stack args): tail-jumps to
  `m_pSubTask->MakeAbortable(...)` (vtbl+0x18) with ECX replaced and the caller's stack args reused; the callee's `ret 0xC` returns.
  No null check of m_pSubTask (crashes if null). Slot 6 of many complex tasks that do not override it.
- gta-reversed: not a CTaskComplex method in gr (each task overrides; several gr tasks write `return m_pSubTask->MakeAbortable(...)`)
  - behaviour identical.

## 0x61A430 CTaskComplex::SetSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CTask* sub [esp+4])`, `ret 4`. Slot 7.
- Behaviour: `old = m_pSubTask`; if `old == sub` return without doing anything (0x61A43B-0x61A43D; note: the parent pointer of
  `sub` is NOT (re)set in that case); if old != 0 `old->vtbl[0](1)` (deleting dtor, 0x61A443-0x61A447); `m_pSubTask = sub`
  (0x61A44B); if sub != 0 `sub->m_Parent (+4) = this` (0x61A450).
- gta-reversed: TaskComplex.h:28 inline - same logic (checks equality first). Matches.

## Deleting destructors (catalogue name `CTaskComplex::~CTaskComplex`; NAME CORRECTION: each is the
## `scalar deleting destructor' of the class whose vtable slot 0 holds it)
All eleven share one shape: `__thiscall this (uint32 flags [esp+4])`, `ret 4`: call the class's complete-object dtor (thiscall),
then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`), return this (EAX). Port: implement as the
class's Drop + pool free. Port level per catalogue (3 unless noted).

| address | vtable (slot 0) | proposed name | complete dtor called |
|---|---|---|---|
| 0x61C0D0 | 0x86D5DC | CTaskComplexPlayHandSignalAnim::`scalar deleting destructor' (level 12) | 0x61BDF0 |
| 0x629060 | 0x86D9F0 | CTaskComplexRoadRage::`scalar deleting destructor' | 0x622110 |
| 0x637D90 | 0x86DF60 | CTaskComplexSitDownThenIdleThenStandUp::`scalar deleting destructor' | 0x6314A0 |
| 0x6385B0 | 0x86E134 | CTaskComplexUseAttractor::`scalar deleting destructor' | 0x632720 |
| 0x6434F0 | 0x86EA14 | CTaskComplexEnterAnyCarAsDriver::`scalar deleting destructor' | 0x63D100 (`vtbl = 0x86EA14`; jmp 0x61A3D0 = ~CTaskComplex; proposed ~CTaskComplexEnterAnyCarAsDriver) |
| 0x656E50 | 0x86F334 | CTaskComplexWalkRoundBuildingAttempt::`scalar deleting destructor' | 0x654950 |
| 0x657B60 | 0x86F308 | CTaskComplexWalkRoundCar::`scalar deleting destructor' | 0x656B00 |
| 0x65D260 | 0x86F744 | CTaskComplexSmartFleePoint::`scalar deleting destructor' | 0x65BDB0 |
| 0x662350 | 0x86F8FC | CTaskComplexGangLeader::`scalar deleting destructor' | 0x65DF30 |
| 0x6644C0 | 0x86FA8C | CTaskComplexSignalAtPed::`scalar deleting destructor' | 0x660AB0 |
| 0x684760 | 0x8706CC | CTaskComplexWalkAlongsidePed::`scalar deleting destructor' | 0x6832C0 |
| 0x68DAB0 | 0x870A00 | CTaskComplexKillCriminal::`scalar deleting destructor' (level 4) | 0x68BF30 |

Instruction evidence (same offsets in each): call dtor at +3, `test byte [esp+8],1` at +8, `call 0x61A5B0` at +0x10, `ret 4` at +0x1B.
gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
