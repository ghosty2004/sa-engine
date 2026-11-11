# CRunningScript (SCRIPT) — RE-SPEC-REST

## 0x465AA0 CRunningScript::ShutdownThisScript  (catalogue: specified/REA, port level 3)
- `__thiscall` (ECX = CRunningScript* → ESI; gr RunningScript.h offsets: +0x10 m_BaseIP, +0x3C m_LocalVars[0], +0xC4 m_IsActive,
  +0xC7 m_IsExternal, +0xC9 i8 m_ExternalType, +0xDC m_ThisMustBeTheOnlyMissionRunning), no args, plain `ret`; SEH frame (for the task
  `new`). Callers: ProcessCommands0To99 0x465E60, ProcessCommands2200To2299 0x474900, ProcessCommands1100To1199 0x48A320.
  Callees: CStreamedScripts::GetStreamedScriptWithThisStartAddress 0x470910 (thiscall on CTheScripts::StreamedScripts 0xA47B60, arg
  u8* → index), CTask::operator new 0x61A5A0, CTaskSimpleFinishBrain ctor 0x463640 (stub → 0x156F250: CTaskSimple ctor 0x61A390 then
  vtable 0x859D20), CScriptedBrainTaskStore::SetTask 0x635720 (cdecl (CPed*, CTask*)).
- Behaviour:
  1. `m_IsActive = 0` (0x465AC2).
  2. If m_IsExternal: `i = GetStreamedScriptWithThisStartAddress(m_BaseIP)`; `StreamedScripts.entry[i] (0xA47B64 + 32*i, u8) -= 1`
     (the script's reference count).
  3. Jump table 0x465BFC on `m_ExternalType` (movsx; > 5 unsigned → return): 0, 2, 3, 5 → ped (0x465AFF); 1, 4 → object (0x465B9D).
     `h = m_ThisMustBeTheOnlyMissionRunning ? CTheScripts::LocalVariablesForCurrentMission[0] ([0xA48960]) : m_LocalVars[0]`.
  4. Ped: GetAt(h) on ped pool [0xB74490] (size 0x7C4); NULL → return. `ped+0x474 &= ~0x800000` (bHasAScriptBrain). If m_ExternalType == 5:
     `t = new(8) CTaskSimpleFinishBrain` (NULL if the pool is full); `SetTask(ped, t)`.
  5. Object: GetAt(h) on object pool [0xB7449C] (0x19C); if found: `obj+0x140 = (obj+0x140 & ~0x200000) | 0x100000`.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:78); behaviour from bytes.

## 0x470150 CRunningScript::PlayAnimScriptCommand(int32 commandId)  (catalogue: specified/REA, port level 10)
- `__thiscall` (ECX = this → ESI), commandId [esp+4] (seen at [esp+0x5C]), `ret 4`; SEH frame. Stub → .HOODLUM body 0x1569460
  (+ 0x1569650-0x1569710). Commands: 0605 TASK_PLAY_ANIM (case 0x4913F5), 0812 TASK_PLAY_ANIM_NON_INTERRUPTABLE (0x473831), 088A
  TASK_PLAY_ANIM_WITH_FLAGS (0x471E0C), 0A1A TASK_PLAY_ANIM_SECONDARY (0x47BB08). Callees: CollectParameters 0x464080,
  ReadTextLabelFromScript 0x463D50 (args (char* buf, u8 size)), CTask::operator new 0x61A5A0, CTaskSimpleRunNamedAnim ctor 0x61A990
  (gr signature (animName, groupName, u32 flags, f32 blend, u32 endTime, bool dontInterrupt, bool runInSequence, bool offsetPed,
  bool holdLastFrame)), CTaskSimpleAffectSecondaryBehaviour ctor 0x691270 ((bool add, int type, CTask*)), GivePedScriptedTask
  0x465C20 ((handle, CTask*, commandId)). ScriptParams at 0xA43C78 (SP[i] = 0xA43C78 + 4i).
- Behaviour:
  1. `b13 = 0; keepInterrupt = 1` (BL). `CollectParameters(1)`; `pedHandle = SP[0]`. `ReadTextLabelFromScript(animName[24], 0x18)`;
     `ReadTextLabelFromScript(group[16], 0x10)`.
  2. By command: 0x605 → `CollectParameters(6)`; 0x812 → `BL = 0`, `CollectParameters(6)`; 0xA1A → `CollectParameters(6)`;
     0x88A → `CollectParameters(8)` (the 8 is pushed through a protection idiom at 0x15694D0: [0x1560540] - [0x659622] = 8),
     `BL = (SP[6] != 0)`, `b13 = (SP[7] != 0)`; any other id → no further parameters.
  3. `blend = SP[0]` (f32 bits), `time = SP[5]` (i32). Flags: `f = 0x10`; `if (SP[1] != 0 || (time > 0 && SP[4] == 0)) f = 0x12`;
     `if SP[2]: f |= 0x40`; `if SP[3]: f |= 0x80`; `if SP[4] == 0: f |= 8`; `if cmd == 0xA1A: f |= 0x400` (0x1569513-0x1569570).
  4. `inSeq = (CTaskSequences::ms_iActiveSequence [0x8D2E98] >= 0)`. `t = new(0x64)`.
  5. If `time > 0`: if t != NULL: `ctor(t, animName, group, f, blend, time, BL == 0, inSeq, b13 != 0, X)` reached through a
     protection dispatcher (0x15695C0 → jmp 0xCB73E0 → back at 0x15695FD) that computes the LAST argument X.
     OPEN QUESTION (0x15695C0-0x1569628): the value of X (holdLastFrame) in the timed branch was not resolved statically.
     Else (time <= 0): if t != NULL: `ctor(t, animName, group, f, blend, -1 (0xFFFFFFFF), BL == 0, inSeq, b13 != 0, 0)` (the 0 is
     computed by an idiom at 0x1569675: ~[0x1560548] ^ [0x6DBA32] = 0).
  6. If cmd == 0xA1A: `w = new(0x14)`; `t = w ? CTaskSimpleAffectSecondaryBehaviour(w, 1, 4, t) : NULL`.
  7. `GivePedScriptedTask(pedHandle, t, cmd)`.
- Note: dontInterrupt = (BL == 0): TASK_PLAY_ANIM_WITH_FLAGS passes `SP[6] == 0` (inverted), 0812 passes 1, the others 0.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:50), body is `plugin::CallMethod`.

## 0x486D80 CRunningScript::LocateCharCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this, saved at [esp+0x2C]), commandId [esp+4], `ret 4`. Caller: ProcessCommands200To299 0x469390 (commands
  00EC-0103, LOCATE_[STOPPED_]CHAR/PLAYER_{ANY_MEANS,ON_FOOT,IN_CAR}[_CHAR]_{2D,3D}). Callees: CollectParameters 0x464080,
  CTheScripts::IsPedStopped 0x486110 (cdecl (CPed*)), UpdateCompareFlag 0x4859D0, CTheScripts::HighlightImportantArea 0x485E00
  (cdecl (id, x1, y1, x2, y2, z)), CTheScripts::DrawDebugSquare 0x486840 (dead, see below).
- Behaviour:
  1. `is3D = (0xFE <= cmd <= 0x103)` (stored as a byte in the argument slot); `CollectParameters(is3D ? 8 : 6)`.
  2. `ped = PedPool.GetAt(SP[0])` ([0xB74490], 0x7C4); NO NULL check (an invalid handle dereferences NULL at 0x486DDF).
     `pos = (ped+0x46C & 0x100 && ped->m_pVehicle (+0x58C)) ? vehicle position : ped position` (matrix pos +0x30 or placement +4).
  3. Stopped variants: `k = cmd - 0xEF`; if k <= 0x14 and byte table 0x4870B0[k] == 0 (cmd 0xEF-0xF1, 0x101-0x103):
     `if !IsPedStopped(ped)` → result 0, skip to 6.
  4. Params: 2D: `x = SP[1], y = SP[2], rx = SP[3], ry = SP[4], marker = SP[5]`; 3D: `x = SP[1], y = SP[2], z = SP[3], rx = SP[4],
     ry = SP[5], rz = SP[6], marker = SP[7]`. `result = 0`. Box test in order (f32 x87 compares; NaN fails every test):
     `(x - rx) <= pos.x` (`test ah, 0x41; jp fail`), `(rx + x) >= pos.x` (`test ah, 1; jne fail`), `(y - ry) <= pos.y`,
     `(ry + y) >= pos.y`; 3D adds `(z - rz) <= pos.z`, `(rz + z) >= pos.z`.
  5. Inside: `k2 = cmd - 0xEC` (<= 0x17), class from byte table 0x4870D8 → jump table 0x4870C8: 0 any means (EC, EF, FE, 101) →
     result 1; 1 on foot (ED, F0, FF, 102) → result 1 only if NOT `ped+0x46C & 0x100`; 2 in car (EE, F1, 100, 103) → result 1 only if
     in vehicle; 3 (F2-FD, handled by other functions) → result 0.
  6. `UpdateCompareFlag(result)`.
  7. If marker != 0: `HighlightImportantArea((int)this + (int)this->m_IP (+0x14), x - rx, y - ry, rx + x, ry + y, is3D ? z :
     -100.0f)` (immediate 0xC2C80000). The CTheScripts::DbgFlag test (u8 [0x859CF8], in .rdata, value 0) guards a DrawDebugSquare call
     for 2D: dead code.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:52), plugin call only.

## 0x487A20 CRunningScript::LocateCarCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this → EDI), commandId [esp+4], `ret 4`. Caller: ProcessCommands400To499 0x47D210 (01AD LOCATE_CAR_2D, 01AE
  LOCATE_STOPPED_CAR_2D, 01AF LOCATE_CAR_3D, 01B0 LOCATE_STOPPED_CAR_3D). Same callees as LocateCharCommand 0x486D80.
- Behaviour: `is3D = (0x1AF <= cmd <= 0x1B0)`; `CollectParameters(is3D ? 8 : 6)`; `v = VehiclePool.GetAt(SP[0])` ([0xB74494], 0xA18;
  NULL on a bad handle and then dereferenced — no check).
  Stopped variants (cmd 0x1AE or 0x1B0, 0x487A80-0x487ACA): `st = (CTimer::ms_fTimeStep [0xB7CB5C] < CTimer::ms_fOldTimeStep [0xB7CB54]) ?
  ms_fTimeStep : ms_fOldTimeStep` (the smaller one; NaN picks the old step); `thr = [0x858C58]=0.01f * st`; if NOT `v.m_fMovingSpeed (+0xD4)
  <= thr` (`fcompp; test ah, 0x41; jnp`) → result 0 and skip the box test.
  Parameters and box test exactly as in LocateCharCommand step 4 (2D: x SP1, y SP2, rx SP3, ry SP4, marker SP5; 3D: x, y, z, rx, ry, rz =
  SP1..SP6, marker SP7), position = `v.m_matrix ? matrix.pos : placement.pos`; inside → result 1 (no on-foot / in-car classes).
  `UpdateCompareFlag(result)`; marker → `HighlightImportantArea((int)this + (int)m_IP, x - rx, y - ry, rx + x, ry + y, is3D ? z : -100.0f)`;
  the DbgFlag [0x859CF8] DrawDebugSquare branch is dead.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:51).

## 0x487D10 CRunningScript::LocateObjectCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this → ESI), commandId [esp+4], `ret 4`. Caller: ProcessCommands1200To1299 0x48B590 (04E5 LOCATE_OBJECT_2D,
  04E6 LOCATE_OBJECT_3D).
- Behaviour: `is3D = (cmd == 0x4E6)`; `CollectParameters(is3D ? 8 : 6)`; `o = ObjectPool.GetAt(SP[0])` ([0xB7449C], 0x19C; NULL dereferenced
  if invalid). No stopped test. Parameters, box test (0x487DC2-0x487E93), UpdateCompareFlag 0x4859D0, marker highlight (2D z = -100.0f,
  immediate at 0x487EEC) and the dead DbgFlag branch exactly as in LocateCarCommand; position from the object's matrix or placement.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:53).

## 0x487420 CRunningScript::LocateCharCarCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this → EBX), commandId [esp+4], `ret 4`. Caller: ProcessCommands500To599 0x47E090 (0202-0204
  LOCATE_CHAR_{ANY_MEANS,ON_FOOT,IN_CAR}_CAR_2D, 0205-0207 the _3D forms). Same callees as LocateCharCommand 0x486D80.
- Behaviour:
  1. `is3D = (0x205 <= cmd <= 0x207)`; `CollectParameters(is3D ? 6 : 5)`. `ped = PedPool.GetAt(SP[0])`, `car = VehiclePool.GetAt(SP[1])`
     (both may be NULL and are dereferenced without checks). `inVeh = (ped+0x46C >> 8) & 1`; `p` = position of `ped->m_pVehicle` when inVeh
     and the vehicle pointer is non-NULL, else the ped's position; `c` = the car's position (matrix pos or placement).
  2. Radii: `rx = SP[2], ry = SP[3]`; 3D: `rz = SP[4], marker = SP[5]`; 2D: `marker = SP[4]`.
  3. Box test (result 0 on any failure): `t = (f32)(c.x - rx)` stored to memory, then `p.x >= t` (`fcomp; test ah, 1; jne fail`);
     `(c.x + rx) >= p.x`; `(c.y - ry) <= p.y` (`test ah, 0x41; jp fail`); `(c.y + ry) >= p.y`; 3D adds `(c.z - rz) <= p.z`,
     `(c.z + rz) >= p.z`. NaN fails every test.
  4. Inside: jump table 0x487704 on `cmd - 0x202` (0..5): any means (0202, 0205) → 1; on foot (0203, 0206) → 1 only if !inVeh; in car
     (0204, 0207) → 1 only if inVeh.
  5. `UpdateCompareFlag(result)`. If marker: `HighlightImportantArea((int)this + (int)m_IP, c.x - rx, c.y - ry, c.x + rx, c.y + ry,
     is3D ? c.z : -100.0f)` (box around the CAR; the first coordinate is the f32 stored in step 3). DbgFlag [0x859CF8] branch dead.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:54). Confidence medium: the highlight argument stack offsets
  (0x487650-0x4876B1) were mapped by hand.

## 0x487720 CRunningScript::LocateCharObjectCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall`, commandId [esp+4], `ret 4`. Caller: ProcessCommands1100To1199 0x48A320 (0471-0473 LOCATE_CHAR_{ANY_MEANS,ON_FOOT,IN_CAR}
  _OBJECT_2D, 0474-0476 the _3D forms).
- Instruction-for-instruction the same code as LocateCharCarCommand 0x487420 (offset +0x300) except: 3D range `0x474 <= cmd <= 0x476`;
  the second entity comes from the object pool (CPools::ms_pObjectPool [0xB7449C], size 0x19C) instead of the vehicle pool; class table
  0x487A04 indexed by `cmd - 0x471` (same layout: any / on foot / in car, twice). Behaviour, box test and highlight as in 0x487420.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:56).

## 0x4870F0 CRunningScript::LocateCharCharCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this → EBX), commandId [esp+4], `ret 4`. Caller: ProcessCommands200To299 0x469390 (00F2-00F4
  LOCATE_CHAR_{ANY_MEANS,ON_FOOT,IN_CAR}_CHAR_2D, 0104-0106 the _3D forms).
- Behaviour: as LocateCharCarCommand 0x487420 with these differences: `is3D = (0x104 <= cmd <= 0x106)`; `CollectParameters(is3D ? 6 : 5)`;
  both SP[0] and SP[1] are ped handles (ped pool [0xB74490], 0x7C4; no NULL checks); BOTH positions use the ped's vehicle position when
  `ped+0x46C & 0x100` and `ped->m_pVehicle (+0x58C)` is non-NULL (first ped 0x487167-0x48719C, second ped 0x4871AF-...), otherwise the ped
  position; the box (radii SP[2], SP[3], 3D SP[4]; marker SP[4] 2D / SP[5] 3D) is centred on the SECOND ped. Class selection: byte table
  0x487408 indexed by `cmd - 0xF2` (<= 0x14) → jump table 0x4873F8: 0 any means (F2, 104) → 1; 1 on foot (F3, 105) → 1 only if the first
  ped is not in a vehicle (EBP = in-vehicle bit); 2 in car (F4, 106) → 1 only if it is; 3 (F5..103, other handlers) → 0.
  Then UpdateCompareFlag, the marker highlight around the second ped (2D z = -100.0f) and the dead DbgFlag branch.
- Confidence medium: the box-test block (0x4871D9-0x487304) was not re-traced instruction by instruction; it is assumed to match
  0x487544-0x487615 of LocateCharCarCommand (same shape in the listing). Ports should diff the two blocks.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:55).

## 0x488EC0 CRunningScript::CarInAreaCheckCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this → ESI), commandId [esp+4], `ret 4`. Commands 00B0 IS_CAR_IN_AREA_2D, 00B1 IS_CAR_IN_AREA_3D, 01AB
  IS_CAR_STOPPED_IN_AREA_2D, 01AC IS_CAR_STOPPED_IN_AREA_3D. Callees: CollectParameters 0x464080, CTheScripts::IsVehicleStopped 0x4861F0
  (cdecl (CVehicle*)), UpdateCompareFlag 0x4859D0, HighlightImportantArea 0x485E00, DrawDebugSquare 0x486840 (dead).
- Behaviour:
  1. `is3D = (cmd == 0xB1 || cmd == 0x1AC)`; `CollectParameters(is3D ? 8 : 6)`; `v = VehiclePool.GetAt(SP[0])` ([0xB74494], 0xA18; NULL not
     checked). If `0x1AB <= cmd <= 0x1AC` and `!IsVehicleStopped(v)` → result 0, skip to 4.
  2. Corners: 2D `x1 = SP1, y1 = SP2, x2 = SP3, y2 = SP4, marker = SP5`; 3D `x1 = SP1, y1 = SP2, z1 = SP3, x2 = SP4, y2 = SP5, z2 = SP6,
     marker = SP7`. Normalise (swap the two values, raw dwords) when `z1 > z2` (3D only, 0x488F84), then `x1 > x2`, then `y1 > y2`
     (`fcomp; test ah, 0x41; jne` → no swap for <, = or NaN).
  3. `p` = vehicle position (matrix pos or placement). Result 1 when `p.x >= x1`, `p.x <= x2`, `p.y >= y1`, `p.y <= y2` (and for 3D
     `z1 <= p.z <= z2`), each a separate x87 compare where NaN fails.
  4. `UpdateCompareFlag(result)`. If marker: `HighlightImportantArea((int)this + (int)m_IP, x1, y1, x2, y2, is3D ? (z2 + z1) *
     [0x858B8C]=0.5f : -100.0f)` (normalised corners). DbgFlag [0x859CF8] branch dead.
- Confidence medium: the z comparisons and the separate 2D compare chain (0x489081-0x4890B1) follow the same pattern but were not
  traced one by one.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:57).

## 0x489150 CRunningScript::ObjectInAreaCheckCommand(int32 commandId)  (catalogue: specified/REA, port level 14)
- `__thiscall` (ECX = this → ESI), commandId [esp+4], `ret 4`. Commands 04E9 IS_OBJECT_IN_AREA_2D, 04EA IS_OBJECT_IN_AREA_3D.
- Behaviour: `is3D = (cmd == 0x4EA)`; `CollectParameters(is3D ? 8 : 6)`; `o = ObjectPool.GetAt(SP[0])` ([0xB7449C], 0x19C; NULL not
  checked). No stopped test. Then the same corner layout, min/max normalisation, inclusive area test on the object's position,
  UpdateCompareFlag and marker highlight (`z = (z2 + z1) * 0.5f` for 3D, -100.0f for 2D) as CarInAreaCheckCommand 0x488EC0.
- Confidence medium: only the prologue (0x489150-0x4891B5) was compared instruction by instruction with 0x488EC0; the remainder was
  checked for the same call sequence.
- gta-reversed: hook `.Reversed = false` (RunningScript.cpp:59).
