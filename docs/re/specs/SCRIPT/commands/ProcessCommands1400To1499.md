# CRunningScript::ProcessCommands1400To1499 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` / `ped(h)` / `obj(h)` = pool GetAtRef 0x4048E0 / 0x404910 /
0x465040 (ECX = CPools::ms_pVehiclePool 0xB74494 / ms_pPedPool 0xB74490 / ms_pObjectPool 0xB7449C), null when the slot flag
byte != h & 0xFF; results are never null-checked unless stated. `this` = ESI, **EBP = command id** (0x48EABD; some cases reuse
EBP and reload the id from [esp+0x458]).
Task pattern ("give T(args)"): `t = CTask::operator new(size)` 0x61A5A0; if non-null construct `T(args)` (SEH state [esp+0x450]
set to a per-case value around the ctor, then -1); null allocation -> `t = null`, ctor skipped; then
`GivePedScriptedTask(SP0, t, command)` (0x465C20, thiscall ECX = this). Where the case passes the ped handle from another slot it is
stated. "Sequence add" = `CTaskSequences::ms_taskSequence[CTaskSequences::ms_iActiveSequence].AddTask(t)` (0x632D10, ECX = 0xC178F0 +
index*0x40). The index is loaded through the .HOODLUM-obfuscated stub 0x409AD7 (`pushfd; clc; ecx = [0x15608E0] + [0x4EF1C5] (+CF=0);
ecx = [ecx]; popfd; jmp 0x48FF03`): the computed address is 0x8D2E98 = ms_iActiveSequence, i.e. plain `mov ecx,[0x8D2E98]`; no range check.
Time convention used by several tasks: `-1 -> 20000` (0x4E20), `-2 -> 0x7FFFFFFF` or `-1` as stated.
All cases return AL = 0 through 0x490C11 unless stated.

## 0x48EAA0 CRunningScript::ProcessCommands1400To1499  (catalogue: specified/REA, port level 10)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x838880),
  `sub esp,0x438`, push ebx/ebp/esi/edi; `ebp = command` (0x48EABD), `esi = this` (0x48EACF). `eax = command - 0x57B` (0x48EAC5),
  `cmp eax,0x60; ja 0x490C0F` (0x48EACB/0x48EAD1): 1403..1499 dispatched; 1400..1402 (0578..057A) reach the default.
  Single table `jmp [eax*4+0x490C2C]` (0x48EAD7).
- Default 0x490C0F: `or al,0xFF`; epilogue 0x490C11: restore fs:[0] from [esp+0x448], pop, `add esp,0x444; ret 4` -> -1.
- Opcodes: 82 with a case: 13 gta-reversed `handler`, 21 `nop`, 1 `unimplemented` (05D5 SET_CHAR_IN_DISGUISE, shares 0x490C0B
  `xor al,al` with 19 of the nops), 47 `unregistered` specified below.
  **gr-nop deviations:** 0596 IS_PLAYER_IN_SHORTCUT_TAXI (case 0x48EE39) does CollectParameters(1) and falls into 059A, i.e.
  UpdateCompareFlag(false); 05B6 SAVE_STRING_TO_DEBUG_FILE (case 0x48FAEC) copies 128 bytes from `m_IP` into a local buffer and
  advances `m_IP` by 128 (0x48FAF3..0x48FB05). A port must keep these operand/flag side effects.
- Default-reaching (18, gr `unimplemented`): 0578, 0579, 057A, 057C, 0580, 0584, 0586, 0589, 058B, 058F, 0590, 0593, 0598, 0599,
  05AB, 05AC, 05AF, 05B4.

### Ped tasks
| case | opcode | name (used) | behaviour |
|---|---|---|---|
| 0x48FB0E | 05B9 | TASK_PAUSE (0/217/18) | CollectParameters(2); give `CTaskSimplePause(SP1)` (0x48E750, size 0x18). |
| 0x48FC9D | 05BB | TASK_FALL_AND_GET_UP (0/8/0) | CollectParameters(3); give `CTaskComplexFallAndGetUp(SP1, SP2)` (0x678700, size 0x18). |
| 0x48FCE7 | 05BC | TASK_JUMP (0/16/0) | CollectParameters(2); `t = new(0x14) CTaskComplexJump(0)` (0x67A030); `byte [t+0x10] = (SP1 != 0)` (0x48FD2D, written even when t is null - original oddity); give t. |
| 0x48FD4A | 05BD | TASK_TIRED (0/5/0) | CollectParameters(2); give `CTaskSimpleTired(SP1)` (0x630F20, size 0x1C). |
| 0x48FD8D | 05BE | TASK_DIE (0/63/3) | CollectParameters(1); give `CTaskComplexDie(0, 0, 15, 4.0f /*imm 0x40800000*/, 0, 0, 0, 0, 0)` (0x630040, size 0x28; weapon 0, anim group 0, anim id 15, blend 4.0). |
| 0x490143 | 05C3 | TASK_COWER | CollectParameters(1); give `CTaskSimpleCower()` (0x48DE70, size 0x20). |
| 0x49017F | 05C4 | TASK_HANDS_UP | CollectParameters(2); `time = SP1 == -1 ? 20000 : SP1 == -2 ? 0x7FFFFFFF : SP1`; give `CTaskSimpleHandsUp(time)` (0x48E970, size 0x34). |
| 0x49029B | 05C7 | TASK_USE_ATM | CollectParameters(1); give `CTaskSimpleUseAtm()` (0x48DFE0, size 0x20). |
| 0x4902D7 | 05C8 | TASK_SCRATCH_HEAD | CollectParameters(1); give task built by 0x48DF30 (.HOODLUM 0x1562470; names-extra hint "ScratchHead": CTaskSimpleScratchHead ctor), size 0x20. |
| 0x490313 | 05C9 | TASK_LOOK_ABOUT | CollectParameters(2); time mapping as 05C4; give `CTaskSimpleLookAbout(time)` (0x48E0A0, size 0x34). |
| 0x49090B | 05D4 | TASK_ACHIEVE_HEADING | CollectParameters(2); give `CTaskSimpleAchieveHeading(SP1 * [0x8595EC]=0.017453292f, [0x86FC7C]=0.5f, [0x86FC80]=0.2f)` (0x667E20, size 0x18). |
| 0x490554 | 05CD | TASK_LEAVE_CAR | CollectParameters(2); give `CTaskComplexLeaveCar(veh(SP1), 0, 0, true, false)` (0x63B8C0, size 0x34). |
| 0x4905AC | 05CF | TASK_LEAVE_CAR_AND_FLEE | CollectParameters(5); give `CTaskComplexLeaveCarAndFlee(veh(SP1), CVector(SP2,SP3,SP4), 0, 0, false)` (0x63BF90, size 0x28). |
| 0x490762 | 05D2 | TASK_CAR_DRIVE_WANDER | CollectParameters(4); `v = SP1 < 0 ? null : veh(SP1)`; give `CTaskComplexCarDriveWander(v, SP3 /*driving style*/, SP2 /*speed*/)` (0x63CB10, size 0x24). |
| 0x490A55 | 05D9 | TASK_GOTO_CHAR | CollectParameters(4); `time = SP2 == -2 ? -1 : SP2 == -1 ? 50000 : SP2`; give `CTaskComplexSeekEntity<Standard>(ped(SP1), time, 1000, SP3, [0x859E30]=2.0f, [0x859E34]=2.0f, true, true)` (0x46AC10, size 0x4C). |
| 0x490AE6 | 05DA | TASK_FLEE_POINT | CollectParameters(6); give `CTaskComplexFleePoint(CVector(SP1,SP2,SP3), true, SP4 /*safe distance*/, SP5 /*time*/)` (0x65B390, size 0x3C). |
| 0x490B7E | 05DB | TASK_FLEE_CHAR | CollectParameters(4); give `CTaskComplexFleeEntity(ped(SP1), true, (float)SP2 /*fild: integer operand*/, SP3, [0x86F664] (int 1000), [0x86F668]=1.0f)` (0x65B930, size 0x3C). |
| 0x4909D4 | 05D8 | TASK_FOLLOW_POINT_ROUTE | CollectParameters(3); give `CTaskComplexFollowPointRoute(SP1 /*move state*/, route 0xC18D50, SP2 /*mode*/, [0x86FC98]=0.5f, [0x86FC9C]=5.0f, false, true, true)` (0x671510, size 0x3C). |
| 0x490981 | 05D6 | FLUSH_ROUTE | `dword [0xC18D50] = 0` (point count of the script CPointRoute). |
| 0x490992 | 05D7 | EXTEND_ROUTE | CollectParameters(3); `0x48E180(&route 0xC18D50, &CVector(SP0,SP1,SP2))` (thiscall; .HOODLUM 0x1569990; proposed CPointRoute::AddRoutePoint). |

Script usage (main/missions/streamed) of every opcode in this file: 0587 0/137/2, 058C 1/0/0, 0594 0/2/0, 0595 0/18/2, 059A 0/0/0, 059C 0/1/0, 059D 0/0/2, 059E 0/0/11, 05A1 0/1/0, 05A2 0/4/0, 05A3 0/6/0, 05A4 1/10/3, 05A5 0/19/0, 05A6 0/2/10, 05A7 0/2/0, 05A8 0/10/0, 05B0 0/8/0, 05B9 0/217/18, 05BA 6/101/23, 05BB 0/8/0, 05BC 0/16/0, 05BD 0/5/0, 05BE 0/63/3, 05BF 0/164/10, 05C0 0/12/0, 05C1 0/3/0, 05C2 0/4/0, 05C3 0/1/1, 05C4 0/17/17, 05C5 0/31/25, 05C7 0/1/0, 05C8 0/34/0, 05C9 0/24/4, 05CA 0/108/2, 05CB 0/115/8, 05CD 1/101/4, 05CF 0/6/0, 05D1 0/322/19, 05D2 0/47/13, 05D3 23/931/24, 05D4 0/269/16, 05D6 0/188/10, 05D7 0/393/34, 05D8 0/120/10, 05D9 0/45/2, 05DA 0/17/0, 05DB 0/17/0, 05D5 0/0/0.
Most used: 05D3 GO_STRAIGHT_TO_COORD 23/931/24, 05D7 EXTEND_ROUTE 0/393/34, 05D1 0/322/19, 05D4 0/269/16, 05B9 0/217/18.

### 0x48FB82 05BA COMMAND_TASK_STAND_STILL  (gr: unregistered, used: 6/101/23)
- CollectParameters(2). `t = new(0x20)`; ctor `CTaskSimpleStandStill(time, looped, holdLastFrame, 8.0f /*imm 0x41000000*/)` (0x62F310):
  SP1 == -1 -> `(20000 [0x86DB24], false, false)`; SP1 == -2 -> `(999999 (0xF423F), true, false)`; else `(SP1, false, false)`.
  GivePedScriptedTask(SP0, t, command).

### 0x48FE0F 05BF COMMAND_TASK_LOOK_AT_CHAR / 0x48FF19 05C0 COMMAND_TASK_LOOK_AT_VEHICLE  (gr: unregistered, used: 0/164/10, 0/12/0)
- CollectParameters(3): SP0 = ped or -1 (sequence), SP1 = target ped (05BF) / vehicle (05C0), SP2 = time.
  `time = SP2 == -1 ? 20000 : SP2 == -2 ? (05BF: -1, 05C0: 0x7FFFFFFF) : SP2`. `target = ped(SP1)` / `veh(SP1)`.
- SP0 != -1: `g_ikChainMan (0xC15448).LookAt(name, ped(SP0), target, time, boneTag, nullptr, false, 0.25f /*imm 0x3E800000*/, 500, 6,
  true)` (0x618970) with name = "COMMAND_TASK_LOOK_AT_CHAR" (0x85A2EC) / "COMMAND_TASK_LOOK_AT_VEHICLE" (0x85A2CC) and boneTag = 5 (05BF)
  / -1 (05C0). Return 0.
- SP0 == -1: `t = new(0x30) <ctor 0x634440>(target, time, boneTag, CVector(0,0,0), true, 0.25f, 1000, 3)` (unnamed; proposed
  CTaskSimpleTriggerLookAt), sequence add (0x48FEF2..0x48FF0D). Return 0.

### 0x490006 05C1 COMMAND_TASK_SAY / 0x4900B2 05C2 COMMAND_TASK_SHAKE_FIST  (gr: unregistered)
- 05C1: CollectParameters(2); `s = new(0x1C) CTaskSimpleSay(SP1, -1)` (0x48E360). 05C2: CollectParameters(1); `s = new(0x10)
  CTaskSimpleShakeFist()` (0x690B80). Slot k = 2 (say) / 4 (shake fist).
- SP0 != -1: `CTaskManager::SetTaskSecondary(&ped(SP0)->m_pIntelligence->m_TaskMgr ([ped+0x47C] + 4), s, k)` (0x681B60). Return 0.
- SP0 == -1: `t = new(0x14) CTaskSimpleAffectSecondaryBehaviour(true, k, s)` (0x691270) (null on allocation failure), sequence add.

### 0x4901D8 05C5 COMMAND_TASK_DUCK
- CollectParameters(2). `t = new(0x28) CTaskSimpleDuck(0, len, -1)` (0x691FC0) with len = 0 if SP1 == -2, (u16)20000 ([0x870A8C]) if
  SP1 == -1, else (u16)SP1. GivePedScriptedTask(SP0, t, command).

### 0x49036C 05CA COMMAND_TASK_ENTER_CAR_AS_PASSENGER / 0x49046B 05CB COMMAND_TASK_ENTER_CAR_AS_DRIVER
- 05CA: CollectParameters(4): SP0 ped, SP1 vehicle, SP2 time, SP3 seat. `v = veh(SP1)`; `door = SP3 == -1 ? 0 :
  CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(v, SP3)` (0x64F190). `t = new(0x2C) CTaskComplexEnterCarAsPassengerTimed(v, door,
  time, true)` (0x63B030), time = -1 if SP2 == -2, 20000 ([0x86E69C]) if SP2 == -1, else SP2. `[t+0x1C] = dword [0x8D237C]` (t NOT
  null-checked), then `[0x8D237C] = 6`. GivePedScriptedTask(SP0, t, command).
- 05CB: CollectParameters(3). `t = new(0x24) CTaskComplexEnterCarAsDriverTimed(veh(SP1), time)` (0x63AD70), same time mapping
  ([0x86E698] = 20000). `[t+0x14] = [0x8D237C]` (unchecked), `[0x8D237C] = 6`. GivePedScriptedTask(SP0, t, command) (0x4908F6).
- Open question: identity of the global 0x8D237C (int, initial value 6; looks like a one-shot "move state for the next script
  enter-car task" reset to 6 after each use).

### 0x490649 05D1 COMMAND_TASK_CAR_DRIVE_TO_COORD
- CollectParameters(9): SP0 ped, SP1 vehicle (< 0 -> null), SP2..SP4 point, SP5 speed, SP6 mode, SP7 model flag, SP8 driving style.
  If `SP4 <= -100.0f` ([0x859014], `test ah,0x41; jp`): `z = FindGroundZForCoord(SP2, SP3)` (no offset). `model = SP7 == 0 ? -1 :
  SP7 == 1 ? 0x19F (415) : SP7`. Give `CTaskComplexDriveToPoint(v, CVector(SP2,SP3,z), SP5, SP6, model, -1.0f /*imm 0xBF800000*/,
  SP8)` (0x63CE00, size 0x3C).

### 0x4907CE 05D3 COMMAND_TASK_GO_STRAIGHT_TO_COORD
- CollectParameters(6): SP0 ped, SP1..SP3 point, SP4 move state, SP5 time. Radius `[0x86FC84]=0.5f`, `[0x86FC88]=2.0f`.
  SP5 == -2: `new(0x28) CTaskComplexGoToPointAndStandStill(SP4, pt, 0.5f, 2.0f, false, true)` (0x668120); SP5 == -1:
  `new(0x38) CTaskComplexGoToPointAndStandStillTimed(SP4, pt, 0.5f, 2.0f, 20000 [0x86FC8C])` (0x6685E0); else `...Timed(SP4, pt, 0.5f,
  2.0f, SP5)`. GivePedScriptedTask(SP0, t, command).

### 0x48EBF4 0587 COMMAND_SET_LOAD_COLLISION_FOR_CAR_FLAG  (gr: unregistered, used: 0/137/2)
- CollectParameters(2). `v = veh(SP0)`. If `SP1 != 0`: `[v+0x40] &= ~0x4000` (CPhysical flag bit 14); if `this->m_UsesMissionCleanup`:
  `CWorld::Remove(v)`, `[v+0x1C] |= 0x40000`, `CWorld::Add(v)` (0x48EC2E..0x48EC41). Return 0.
  Else: `[v+0x40] |= 0x4000`; if `[v+0x1C] & 0x40000`: clear it and, if `!CEntity::GetIsStatic(v)` (0x4633E0),
  `CPhysical::AddToMovingList(v)` (0x542800). Return 0.
- Open question: gr names of physical flag 0x4000 (+0x40) and entity flag 0x40000 (+0x1C) (0x48EC19 / 0x48EC37).

### 0x48EDEF 0595 COMMAND_REGISTER_ODDJOB_MISSION_PASSED  (gr: unregistered, used: 0/18/2)
- `CStats::IncrementStat(0x93, 1.0f)`; `v = CStats::GetStatValue(0xB0)` stored (fstp) over the stale 1.0f stack slot, then
  `CStats::IncrementStat(0xB1, v)` (stack-slot reuse, 0x48EDFE..0x48EE14); `CStats::SetStatValue(0xB0, 0.0f)`;
  `CTheScripts::LastMissionPassedTime [0xA476A8] = CTimer::m_snTimeInMilliseconds`. Return 0.

### 0x48EED9 059D COMMAND_SHUFFLE_CARD_DECKS / 0x48EFAD 059E COMMAND_FETCH_NEXT_CARD  (gr: unregistered, used: 0/0/2, 0/0/11)
- 059D: CollectParameters(1): decks n. Zero `CTheScripts::CardStack` (0xA44218, 312 int16, `rep stosd` 0x9C dwords). Local list
  `L[i] = i` for i < 312. `rem = n*52`. For card = 1..52 (ebx), for d = 0..n-1: `k = (int16)__ftol2((float)(rand() & 0xFFFF) *
  [0x858B14]=3.0517578e-05f * rem)` (fild, fmul, fimul by the int remaining count); `CardStack[L[k]] = card`; remove L[k] by shifting
  `L[j] = (j > 310) ? 0 : L[j+1]` for j = k..rem-1; `rem--`. Finally `CardStackPosition [0xA44210] = 0`. Return 0.
- 059E: `p = CardStackPosition`; if `CardStack[p] == 0` then `p = 0`; `SP0 = CardStack[p]` (sign-extended); `p++`; if `p == 312` then
  `p = 0`; store p; StoreParameters(1).

### 0x48F07D 05A1 / 0x48F17E 05A2 / 0x48F461 05A6 object rotation velocity  (gr: unregistered, used: 0/1/0, 0/4/0, 0/2/10)
- 05A1 ADD_TO_OBJECT_ROTATION_VELOCITY: CollectParameters(4). `o = obj(SP0)`; `v = (SP1, SP2, SP3) * [0x858B38]=0.02f` (each fmul, f32);
  `w = Multiply3x3(*o->m_matrix, v)` (0x59C790); `ts = o->m_vecTurnSpeed (+0x50) + w` (per component); if `[o+0x1C] & 4` (bIsStatic):
  `o->vtable[0x10/4](false)` (SetIsStatic) and `AddToMovingList(o)`; `o->m_vecTurnSpeed = ts`.
- 05A2 SET_OBJECT_ROTATION_VELOCITY: CollectParameters(4). `v = (ts*SP1, ts*SP2, ts*SP3)` with `ts = CTimer::ms_fTimeStep [0xB7CB5C]`;
  `w = Multiply3x3(*o->m_matrix, v)`; static handling as 05A1; `0x45AFB0(o, w.x*0.02f, w.y*0.02f, w.z*0.02f)` (.HOODLUM 0x15658A0
  writes the three floats to [o+0x50..0x58]; proposed CPhysical::SetTurnSpeed).
- 05A6 GET_OBJECT_ROTATION_VELOCITY: CollectParameters(1). `r = Multiply3x3(o->m_vecTurnSpeed, *o->m_matrix)` (0x59C810, vector x
  matrix); SP0..SP2 = `r * [0x858B40]=50.0f` each; StoreParameters(3) (tail 0x48F03B).

### 0x48F4E9 05A7 COMMAND_ADD_VELOCITY_RELATIVE_TO_OBJECT_VELOCITY  (gr: unregistered, used: 0/2/0)
- CollectParameters(4). `o = obj(SP0)`; `v = (SP1,SP2,SP3)`; `v *= CTimer::ms_fTimeStep` (0x40FEF0); `v /= 50.0f` (0x411A30: multiplies
  by `[0x858624]=1.0f / 50.0f`). **If `[o+0x1C] & 4` (static) return 0 immediately** (0x48F550). `f = normalised(o->m_vecMoveSpeed
  (+0x44))` (0x59C910); if `f.z == 1.0` ([0x85A310] f64, ordered equality) return 0. `r = normalise(Cross(f, (0,0,1)))`;
  `u = normalise(Cross(r, f))` (0x59C730); `m = o->m_vecMoveSpeed + r*v.x + f*v.y + u*v.z` (operator* 0x40FEC0, operator+= 0x411A00,
  in that order); (dead re-check of the static flag); `o->m_vecMoveSpeed = m`.

### 0x48F73F 05A8 COMMAND_GET_OBJECT_SPEED  (used: 0/10/0)
- CollectParameters(1). `SP0 = CVector::Magnitude(&obj(SP0)->m_vecMoveSpeed) * [0x858B40]=50.0f`; StoreParameters(1).

### 0x48F2A2 05A4 COMMAND_GET_ANGLE_BETWEEN_2D_VECTORS  (gr: unregistered, used: 1/10/3)
- CollectParameters(4): a = (SP0, SP1), b = (SP2, SP3). x87: `dot = (b.y*a.y) + (b.x*a.x)`; `lb = sqrt(b.y*b.y + b.x*b.x)`;
  `la = sqrt(a.y*a.y + a.x*a.x)`; `c = dot / (lb * la)`; `SP0 = acos(c) * [0x859878]=57.295776f` where acos is the CRT x87 intrinsic
  0x822380 (proposed _CIacos, ST0 in/out). StoreParameters(1). Zero-length vectors give NaN.

### 0x48F339 05A5 COMMAND_DO_2D_RECTANGLES_COLLIDE  (gr: unregistered, used: 0/19/0)
- CollectParameters(8): rect1 centre (SP0,SP1) size (SP2,SP3); rect2 centre (SP4,SP5) size (SP6,SP7). Halves use the f64 constant
  `[0x859060]=0.5` on f32 operands. `b1 = SP1 - SP3*0.5`, `t2 = SP5 + SP7*0.5`, `l1 = SP0 - SP2*0.5`, `l2 = SP4 - SP6*0.5`,
  `r1 = SP0 + SP2*0.5`, `r2 = SP4 + SP6*0.5` (stored f32), `t1 = SP1 + SP3*0.5`, `b2 = SP5 - SP7*0.5` (x87).
  flag = 1, cleared when: `b2 > t1` (0x48F402), `b1 > t2` (0x48F416), `r1 < l2` (0x48F42A), `l1 > r2` (0x48F43E) - strict compares,
  equality and NaN keep the flag. UpdateCompareFlag(flag).

### 0x48F830 05B0 COMMAND_GET_2D_LINES_INTERSECT_POINT  (gr: unregistered, used: 0/8/0)
- CollectParameters(8): line 1 (x1,y1)-(x2,y2) = SP0..SP3, line 2 (x3,y3)-(x4,y4) = SP4..SP7. flag = 1.
  1. `dx1 = x1 - x2` (x87, kept); `den1 = (dx1 == 0.0 [0x859EF8 f64]) ? 1e-6f (imm 0x358637BD) : dx1` (memory only; ST0 keeps dx1).
  2. `dx2 = x3 - x4`; if `== 0`: `dx2 = [0x858C18]=1e-6f`.
  3. `dy1 = y1 - y2` (stored f32); `m1 = dy1 / den1`; `m2 = (y3 - y4) / dx2` (stored f32); `c1 = y1 - m1*x1`; `c2 = y3 - m2*x3` (f32).
  4. `det = (-m1) - (-m2)` (f32); `num = (-m1)*c2 - (-m2)*c1` (f32). If `det == 0`: fail.
  5. `X = (dx1 == 0) ? x1 : (c1 - c2) / det`; `Y = (dy1 == 0) ? y1 : num / det`.
  6. Sort each coordinate pair (swap only if first > second): x1/x2, y1/y2, x3/x4, y3/y4.
  7. Fail unless all hold (f64 `[0x85A308]=0.01` margins): `xlo1-0.01 <= X`, `xhi1+0.01 >= X`, `xlo2-0.01 <= X`, `xhi2+0.01 >= X`,
     `ylo1-0.01 <= Y`, `yhi1+0.01 >= Y`, `ylo2-0.01 <= Y`, `yhi2+0.01 >= Y` (NaN fails).
  8. Fail: `X = Y = -1000000.0f` (imm 0xC9742400), flag = 0. UpdateCompareFlag(flag); SP0 = X, SP1 = Y; StoreParameters(2).

### Other simple cases
| case | opcode | name (used) | behaviour |
|---|---|---|---|
| 0x48EDA0 | 058C | GET_PROGRESS_PERCENTAGE (1/0/0) | `SP0 = CStats::GetPercentageProgress()` (0x5591E0, ST0 -> f32); StoreParameters(1). |
| 0x48EDBB | 0594 | SET_VEHICLE_TO_FADE_IN (0/2/0) | CollectParameters(2); `CVisibilityPlugins::SetClumpAlpha(veh(SP0)->m_pRwClump [+0x18], SP1)` (0x732B00). |
| 0x48EE42 | 059A | IS_AUSTRALIAN_GAME (0/0/0) | UpdateCompareFlag(false). Also the tail of 0596. |
| 0x48EE92 | 059C | SET_ONSCREEN_COUNTER_FLASH_WHEN_FIRST_DISPLAYED (0/1/0) | `i = GetIndexOfGlobalVariable()` (0x464700); CollectParameters(1); `CUserDisplay::OnscnTimer (0xBA1788).SetCounterFlashWhenFirstDisplayed((u16)i, SP0 != 0)` (0x44CEB0). |
| 0x48F260 | 05A3 | IS_OBJECT_STATIC (0/6/0) | CollectParameters(1); UpdateCompareFlag(`CEntity::GetIsStatic(obj(SP0))`) (0x4633E0). |
