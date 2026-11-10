# CTaskSimpleCarDriveTimed (AI) - RE-SPEC-3 group I3

Vtable 0x859E50 (gr TaskSimpleCarDriveTimed.h, size 0x70): CTaskSimpleCarDrive (0x60), +0x60 m_nTime (i32), +0x64 m_nTimer (CTaskTimer:
+0x64 start, +0x68 interval, +0x6C started, +0x6D stopped).

## 0x46F600 CTaskSimpleCarDriveTimed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x2CF; ret` -> 719 (TASK_SIMPLE_CAR_DRIVE_TIMED). Slot 4. gr matches.

## 0x46F610 CTaskSimpleCarDriveTimed::ProcessPed  (catalogue: specified/REA, port level 12)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4` (or tail-jump). Slot 7 of 0x859E50.
- Behaviour:
  1. If `!m_nTimer.m_bStarted` (+0x6C): `start (+0x64) = now [0xB7CB84]; interval (+0x68) = m_nTime (+0x60); started = 1`; then `started` is
     re-tested (always true; the `je 0x46F65F` at 0x46F62F is dead) (0x46F610-0x46F62F).
  2. Inlined IsOutOfTime (0x46F631-0x46F658): if `m_bStopped` (+0x6D): `start = now; stopped = 0`. If `now >= start + interval` (unsigned,
     `cmp; sbb; inc`) return AL = 1 (0x46F65A).
  3. Otherwise tail-jump `CTaskSimpleCarDrive::ProcessPed(this, ped)` 0x644470 (`jmp`, the callee's `ret 4` returns its AL).
- gta-reversed: `plugin::CallMethodAndReturn<0x46F610>` - not reversed; spec from bytes.

## 0x46F670 CTaskSimpleCarDriveTimed::`scalar deleting destructor'  (catalogue name CTaskSimpleCarDriveTimed::~CTaskSimpleCarDriveTimed)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x859E50. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x46F690 (0x46F673): `vtbl = 0x859E50; jmp 0x63C460` (CTaskSimpleCarDrive::~CTaskSimpleCarDrive). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x46F678, call at 0x46F680; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## 0x5FF940 CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed  (catalogue: specified/REA, port level 3)
- `__thiscall this (CVehicle* vehicle [esp+4], int32 time [esp+8])`, `ret 8`, returns this. 11 callers (CPedIntelligence::ClearTasks 0x601420, ...).
- Behaviour: `CTaskSimpleCarDrive::CTaskSimpleCarDrive(this, vehicle, 0, 0)` 0x63C340 (0x5FF946-0x5FF94D); `vtbl = 0x859E50`; `+0x60 = time`;
  timer `+0x64 = 0, +0x68 = 0, +0x6C = 0, +0x6D = 0` (0x5FF956-0x5FF968).
- gta-reversed: TaskSimpleCarDriveTimed.cpp:10 `CTaskSimpleCarDrive(vehicle, nullptr, false)` - matches.
