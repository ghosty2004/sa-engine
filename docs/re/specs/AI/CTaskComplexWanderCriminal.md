# CTaskComplexWanderCriminal (AI) - RE-SPEC-3 group I3

Vtable 0x85A23C (gr TaskComplexWanderCriminal.h, size 0x38). CTaskComplexWander base (0x28, see CTaskComplexWanderGang.md for its fields),
+0x28 m_TaskTimer (CTaskTimer: +0x28 start, +0x2C interval, +0x30 started, +0x31 stopped), +0x34 m_nMinNextScanTime (u32).

## 0x48E610 CTaskComplexWanderCriminal::CTaskComplexWanderCriminal  (catalogue: specified/REA, port level 2)
- `__thiscall this (eMoveState moveState [esp+4], uint8 dir [esp+8], bool bWanderSensibly [esp+0xC])`, `ret 0xC`, returns this. Body lives
  in .HOODLUM at 0x156F300 (stub `jmp` at 0x48E610). Callers: script 0x490DB0, CPopulation::AddToPopulation 0x614720, GetWanderTaskByPedType 0x673D00.
- Behaviour: `CTaskComplexWander::CTaskComplexWander(this, moveState, dir, bWanderSensibly, 0.5f [0x86FCB0])` 0x66F450 (target radius pushed as raw
  dword); `vtbl = 0x85A23C`; `+0x28 = 0, +0x2C = 0`, bytes `+0x30 = 0, +0x31 = 0`; `+0x34 = 0` (0x156F31F-0x156F333).
- gta-reversed: TaskComplexWanderCriminal.cpp:5 (default radius 0.5f; timer default-constructed) - matches if CTaskTimer() zeroes and
  m_nMinNextScanTime is zeroed (the original zeroes it).

## 0x48E6F0 CTaskComplexWanderCriminal::GetWanderType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3` (0x48E6F0); `ret` -> returns 3. Vtable: slot 11 (+0x2C) of vtable 0x85A23C.

## 0x48E700 CTaskComplexWanderCriminal::`scalar deleting destructor'  (catalogue name CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal)  (catalogue: specified/REA, port level 4)
- NAME CORRECTION: slot 0 of vtable 0x85A23C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x48E720 (0x48E703): see next section. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x48E708, call at 0x48E710; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## 0x48E720 CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal  (catalogue: specified/REA, port level 3)
- `__thiscall void (this)`: complete-object dtor: `vtbl = 0x85A23C; jmp 0x669D20` (~CTaskComplexWander: split by the no-CD build into
  `jmp 0x40387C` -> `vtbl = 0x86FE84` -> `jmp 0x61A3D0` ~CTaskComplex). gr `= default` - matches.

## 0x66B4F0 CTaskComplexWanderCriminal::LookForCarsToSteal  (catalogue: specified/REA, port level 10)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`, SEH frame (one CEventVehicleToSteal local at [esp+0x14] after the pushes).
  The ped argument slot itself is reused as the `best` variable (`mov [esp+0x34], 0` at 0x66B51B overwrites it; ped is kept in EBX).
- Callees: CCarEnterExit::IsVehicleStealable 0x6510D0 (cdecl (vehicle, ped)); _rand 0x821B1E; CEventVehicleToSteal ctor 0x4AF670 (thiscall, 1 arg)
  / dtor 0x4AF700; CEventGroup::Add 0x4AB420.
- Behaviour:
  1. `intel = ped->m_pIntelligence (+0x47C)`; `best = 0`; `minD = FLT_MAX (0x7F7FFFFF)` (0x66B50E-0x66B51F).
  2. Loop i = 0..15 upward (`inc edi` before the test, `cmp edi,0x10; jl`, 0x66B530-0x66B5A6): `v = intel->vehicleScanner.m_apEntities[i]`
     (+0xE0 + 4i); skip if null or `!IsVehicleStealable(v, ped)`. Positions: `pv = v->m_matrix (+0x14) ? m_matrix + 0x30 : v + 4`,
     `pp` likewise for the ped (0x66B54A-0x66B563). x87 (0x66B566-0x66B58A): `dx = pp.x - pv.x; dy = pp.y - pv.y; dz = pp.z - pv.z;
     d = (dz*dz + dy*dy) + dx*dx` (in that order). `fcom minD; test ah,5; jp skip`: update only if `d < minD` (strict; equal and NaN skip):
     `minD = (f32)d`, `best = v` (0x66B597-0x66B59B).
  3. If best == 0 return (0x66B5AF). Else `this->m_nMinNextScanTime (+0x34) = now [0xB7CB84] + 30000` (0x66B5B1-0x66B5BF) - set before the random
     test. `r = rand()` (NOT masked); `fild r; fmul 3.0518509e-05f [0x858C7C]; fcomp 0.2f [0x858CC4]; test ah,5; jp skip` -> continue only if
     `r * (1/32767) < 0.2f` (0x66B5C2-0x66B5E0).
  4. `CEventVehicleToSteal ev(best)`; `ped->m_pIntelligence->m_eventGroup (+0x68).Add(&ev, false)` (0x66B5E2-0x66B608); destroy ev (0x4AF700).
- gta-reversed: `plugin::CallMethod<0x66B4F0>` - not reversed; spec from bytes.

## 0x670350 CTaskComplexWanderCriminal::ScanForStuff  (catalogue: specified/REA, port level 11)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Slot 12 of 0x85A23C.
- Behaviour:
  1. If `!m_TaskTimer.m_bStarted` (+0x30) (0x670354-0x670359): `start (+0x28) = now; interval (+0x2C) = 50; started = 1`; then
     `t0 = now` (re-read, 0x67036F); `r = rand() & 0xFFFF`; x87 `fild r; fmul 3.0517578e-05f [0x858B14]; fmul -30000.0f [0x870110]`;
     `k = _ftol(...)` (truncation, -29999..0); `m_nMinNextScanTime (+0x34) = t0 - k` (i.e. t0 + 0..29999, 0x670375-0x67039A). Then the started
     flag is re-tested (always 1 here, 0x67039D-0x6703A3).
  2. Inlined CTaskTimer::IsOutOfTime (0x6703A5-0x6703CF): if `m_bStopped` (+0x31): `start = now; stopped = 0`. If `now < start + interval`
     (unsigned, `cmp; sbb; inc`) return.
  3. Timer restart (0x6703D1-0x6703DB): `start = now` (the value read at 0x6703C1), `interval = 50`, `started = 1`.
  4. If `now >= m_nMinNextScanTime` (unsigned, `jb` skips, 0x6703DF-0x6703E7) call `LookForCarsToSteal(ped)` 0x66B4F0.
- gta-reversed: TaskComplexWanderCriminal.cpp:8 is `plugin::CallMethod` followed by dead draft code; the draft uses
  `CGeneral::GetRandomNumberInRange(0.0f, 30000.0f)` (float helper, `30000*t`) where the original inlines `-trunc((r/32768) * -30000)` - row
  TaskComplexWanderCriminal.cpp:19 rng-helper: CONFIRMED deviation in the (unused) draft; port the original.
