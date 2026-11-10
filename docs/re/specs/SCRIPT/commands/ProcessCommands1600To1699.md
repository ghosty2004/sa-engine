# CRunningScript::ProcessCommands1600To1699 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Pool helpers (`__thiscall`, `ret 4`, null for stale handles):
0x4048E0 CPool<CVehicle>::GetAt [0xB74494], 0x404910 CPedPool::GetAtRef [0xB74490], 0x465040 CPool<CObject>::GetAt
[0xB7449C], 0x4442D0 (jmp to .HOODLUM 0x156CD40) CPedPool::GetRef(ped). "-100 ground rule" (fcom form): `z <= -100.0`
([0x859014] = -100.0) -> `z = CWorld::FindGroundZForCoord(x, y)` 0x569660. "Task give": `CTask::operator new(size)`
0x61A5A0, EH state [esp+0xF4] = k during the ctor, task = 0 if new fails, EH -1, then
`this->GivePedScriptedTask(P0, task, command)` 0x465C20 (command = EBP). "Deg2rad" = `* [0x8595EC] = 0.017453292` (x87, f32
store). "Fx slot": `i = CTheScripts::GetActualScriptThingIndex(h, 1)` 0x4839A0; `fx = [0xA44114 + i*8]`
(CTheScripts::ScriptEffectSystemArray[i].m_pFxSystem); skip when `i < 0` or `fx == 0`.

## 0x493FE0 CRunningScript::ProcessCommands1600To1699  (catalogue: specified/REA, port level 31)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x49400F; command at [esp+4] copied to EBP at 0x493FFD), `ret 4`. SEH frame
  `push -1; push 0x838B93; push fs:[0]; mov fs:[0],esp` (0x493FE0..0x493FEE), `sub esp,0xDC`, `push ebx; push ebp; push esi;
  push edi`. `eax = command - 0x642` (0x494005); `cmp eax,0x61; ja 0x495DE1` (0x49400B/0x494011); direct jump table
  `jmp [eax*4 + 0x495E00]` (0x494017, 98 entries). 0x640/0x641 (below base) reach the default.
- Common exit 0x495DE3: `nop; nop; jmp 0x4059EC` (loads the saved SEH link into ECX, then 0x495DEA) -> `pop edi; pop esi;
  pop ebp; mov fs:[0],ecx; pop ebx; add esp,0xE8; ret 4`. Default 0x495DE1 `or al,0xFF` -> 0xFF. Shared return-0: 0x495DDD,
  0x495DA9. Shared store-1 tail 0x495DD4 (`StoreParameters(1); return 0`). No case returns 1.
- Opcodes (100): 87 with a case; gta-reversed handler: 18, nop: 16 (not re-specified). Specified below: 53 (43 unregistered +
  067D + 9 unimplemented in the shared 0x495DDD case).
  Default-reaching (13, gr unimplemented, same -1 behaviour): 0640, 0641, 0644, 0645, 0649, 0658, 0690..0696. No gr `handler`
  opcode reaches the default.

### 0x495DDD shared "return 0": 064A, 064D, 0651, 0659, 065A, 0666, 066F, 0670, 0671, 0675, 0678, 068F, 069C, 069D, 069E, 069F, 06A0, 06A1
- No parameter read, AL = 0. The 9 gr nops (`REGISTER_COMMAND_NOP(cmd)`, no args) match. 065A, 0666, 0678, 069C..06A1 (gr:
  unimplemented, CharacterCommands.cpp:4459..4465, CameraCommands.cpp:341..345, used 0/0/0): gr returns -1. DEVIATION in the
  return value only.

### 0x494EA4 0674 COMMAND_CUSTOM_PLATE_FOR_NEXT_CAR  (gr: unregistered, used: 42/319/1)
- `CollectParameters(1)` [0] model; `ReadTextLabelFromScript(buf = esp+0x10, 9)`. For i = 0..7: if `buf[i] == '_'` (0x5F) or
  `buf[i] == 0` -> `buf[i] = ' '` (0x494EC0..0x494ED5); `buf[8] = 0` (0x494EE5).
- `mi = ms_modelInfoPtrs[P0]` (0xA9B0C8); if mi != 0 and `mi->vtable[4]()` (GetModelType, +0x10) == 6 (vehicle) and `mi->+0x24`
  != 0: `CVehicleModelInfo::SetCustomCarPlateText(mi, buf)` 0x4C8980. Return 0.

### 0x494402 0650 COMMAND_KILL_FX_SYSTEM / 0x49439F 064F COMMAND_PLAY_AND_KILL_FX_SYSTEM  (gr: unregistered, used: 0/128/13, 0/20/0)
- `CollectParameters(1)` [0] handle h. Fx slot; `FxSystem_c::Kill` 0x4AA3F0 resp. `PlayAndKill` 0x4AA3D0 (thiscall on fx). Then
  (shared 0x4943D7) `CTheScripts::RemoveScriptEffectSystem(h)` 0x492FD0; if `this->m_UsesMissionCleanup`:
  `MissionCleanUp (0xA90850).RemoveEntityFromList(h, 4)` 0x4654B0. Return 0. (Invalid slot -> return 0, nothing removed.)

### 0x494323 064C COMMAND_PLAY_FX_SYSTEM / 0x494361 064E COMMAND_STOP_FX_SYSTEM  (gr: unregistered, used: 0/68/5, 0/25/9)
- `CollectParameters(1)`; fx slot; `FxSystem_c::Play` 0x4AA2F0 resp. `Stop` 0x4AA390. Return 0.

### 0x494228 064B COMMAND_CREATE_FX_SYSTEM  (gr: unregistered, used: 0/55/2)
- `ReadTextLabelFromScript(name = esp+0x4C, 32)`; `CollectParameters(4)`: [0..2] pos, [3] flag; discarded
  `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 1)` (0x494249). -100 ground rule (0x494254..0x49427F).
- `fx = g_fxMan (0xA9AE80).CreateFxSystem(name, &pos, nullptr, P3 != 0)` 0x4A9BE0 (0x4942C7). If fx == 0: `ScriptParams[0] = -1`;
  StoreParameters(1). Else `h = CTheScripts::AddScriptEffectSystem(fx)` 0x492F90; `ScriptParams[0] = h`; if m_UsesMissionCleanup
  `AddEntityToList(h, 4)` 0x4637E0; StoreParameters(1). Return 0.

### 0x4949F9 0669 COMMAND_CREATE_FX_SYSTEM_ON_CHAR, 0x494B70 066B ..._ON_CAR, 0x494BEC 066D ..._ON_OBJECT  (gr: unregistered, used: 0/29/2, 0/3/0, 0/21/0)
- `ReadTextLabelFromScript(name, 32)`; `CollectParameters(5)`: [0] entity, [1..3] offset, [4] flag; discarded
  GetActualScriptThingIndex(next, 1). `e = PedPool.GetAtRef / VehiclePool.GetAt / ObjectPool.GetAt(P0)`; join 0x494C26.
- -100 ground rule applied to the OFFSET (`z = P3`, ground lookup at (P1, P2), 0x494C32..0x494C55). Local `off = (P1, P2, z)`.
  If `e->m_pRwObject (+0x18) == 0` `e->vtable[7]()` (+0x1C, CreateRwObject). `m = CEntity::GetModellingMatrix(e)` 0x46A2D0; if 0
  -> `ScriptParams[0] = -1`, StoreParameters(1) (0x494A54). Else `fx = g_fxMan.CreateFxSystem(name, &off, m, P4 != 0)` 0x4A9BE0
  (0x494CAE / 0x494A4A). Common tail 0x494D85: as 064B (fx 0 -> -1; else AddScriptEffectSystem, cleanup list type 4, store 1).
- Edge: no null check on e.

### 0x494BAC 066C COMMAND_CREATE_FX_SYSTEM_ON_CAR_WITH_DIRECTION / 0x494CB8 066E ..._ON_OBJECT_WITH_DIRECTION  (gr: unregistered, used: 0/6/0, 0/0/1)
- `ReadTextLabelFromScript(name, 32)`; `CollectParameters(8)`: [0] entity, [1..3] offset, [4..6] direction, [7] flag; discarded
  GetActualScriptThingIndex. Entity from vehicle (066C) / object (066E) pool; join 0x494CF3. Ground rule on the offset z as above.
- `Fx_c::CreateMatFromVec(g_fx (0xA9AE00), &mat (esp+0xA8), &off, &dir)` 0x49E950 (0x494B2B); ensure RwObject; `m =
  GetModellingMatrix(e)` (0 -> -1); `fx = g_fxMan.CreateFxSystem(name, &mat, m, P7 != 0)` 0x4A9BB0 (matrix overload, 0x494D80).
  Common tail 0x494D85. Return 0.

### 0x4948EF 0668 COMMAND_TASK_SHOOT_AT_COORD  (gr: unregistered, used: 0/96/1)
- `CollectParameters(5)`: [0] ped, [1..3] target, [4] duration. `v = (P1,P2,P3)` local.
- If `P0 != -1`: `ped = PedPool.GetAtRef(P0)`; `w = ped->m_aWeapons[(i8)ped->+0x718].m_nType` (+0x5A0 + slot*0x1C);
  `info = CWeaponInfo::GetWeaponInfo(w, 1)` 0x743C60; if `(info->+0x18 >> 8) & 1` (throw weapon): task give (0x1C, EH 3)
  `CTaskSimpleThrowControl(nullptr, &v)` 0x61F8B0; if that task != 0 -> give it and return.
- Otherwise task give (0x3C, EH 4) `CTaskSimpleGunControl(nullptr, &v, nullptr, 3, 5, P4)` 0x61F3F0. Return 0.

### 0x494F85 0677 COMMAND_TASK_CHAT_WITH_CHAR  (gr: unregistered, used: 0/86/6)
- `CollectParameters(4)`: [0] ped, [1] partner, [2] leader flag, [3] flag. `partner = PedPool.GetAtRef(P1)`; `n = (P3 != 0) ? -1 : 4`.
- Task give (0x78, EH 8) `CTaskComplexPartnerChat("COMMAND_TASK_CHAT_WITH_CHAR" (0x85A3B8), partner, P2 != 0, 0.5f, n, 1, 1,
  CVector(0,0,0))` 0x684290 (pushes 0x494FEE..0x49502F); given to P0. Return 0.

### 0x4954E2 0682 COMMAND_DETACH_OBJECT / 0x495657 0684 COMMAND_DETACH_CAR  (gr: unregistered, used: 0/18/10, 0/11/1)
- `CollectParameters(5)`: [0] entity, [1] x, [2] y, [3] z, [4] flag. `e = ObjectPool / VehiclePool.GetAt(P0)`; `ny = P2 * [0x858C1C]
  = -1.0` (f32). If e == 0 or `e->m_pAttachedTo (+0xFC) == 0` return 0. Else `e->DettachEntityFromEntity(P1 * deg2rad, ny * deg2rad,
  P3 (raw), P4 != 0)` 0x5442F0 (thiscall; 0x495537..0x49556F). Return 0.

### 0x495425 0681 COMMAND_ATTACH_OBJECT_TO_CAR, 0x495BAF 069A ..._TO_OBJECT, 0x495C6A 069B ..._TO_CHAR  (gr: unregistered, used: 0/129/1, 0/11/7, 0/1/5)
- `CollectParameters(8)`: [0] object, [1] target, [2..4] offset, [5..7] rotation (degrees). `obj = ObjectPool.GetAt(P0)`; target
  from vehicle / object / ped pool (P1). `obj->AttachEntityToEntity(target, CVector(P2,P3,P4), CVector(P5*d2r, P6*d2r, P7*d2r))`
  0x54D570 (vector overload, both by value; deg2rad products stored f32). No null checks. Return 0.

### 0x49557B 0683 COMMAND_ATTACH_CAR_TO_CAR  (gr: unregistered, used: 0/9/0)
- `CollectParameters(8)`; `a = GetAt(P0)`, `b = GetAt(P1)`; rotation `P5..P7 * deg2rad` (f32). If `P2 <= [0x85A3B4] = -999.9`
  (`fcomp; test ah,0x41; jp`, 0x4955F6..0x495601): `a->AttachEntityToEntity(b, nullptr, nullptr)` 0x54D690 (quaternion overload);
  else `a->AttachEntityToEntity(b, CVector(P2,P3,P4), CVector(rx,ry,rz))` 0x54D570. Return 0.

### 0x494E34 0673 COMMAND_TASK_DIVE_AND_GET_UP  (gr: unregistered, used: 0/7/0)
- `CollectParameters(4)`: [0] ped, [1] dx, [2] dy, [3] time. `v = (P1, P2, 0)`; `CVector::Normalise(&v)` 0x59C910. Task give
  (0x24, EH 6) `CTaskComplexEvasiveDiveAndGetUp(nullptr, P3, &v, 1)` 0x6536B0. Return 0.

### 0x494DC6 0672 COMMAND_TASK_DESTROY_CAR / 0x494F1D 0676 COMMAND_TASK_SHUFFLE_TO_NEXT_CAR_SEAT  (gr: unregistered, used: 0/37/0, 0/35/1)
- `CollectParameters(2)`; `veh = GetAt(P1)`. 0672: task give (0x20, EH 5) `CTaskComplexDestroyCar(veh, 0, 0, 0)` 0x621C00. 0676: task
  give (0x2C, EH 7) `CTaskComplexShuffleSeats(veh)` 0x63D130. Return 0.

### 0x494868 0667 COMMAND_TASK_AIM_GUN_AT_COORD  (gr: unregistered, used: 0/24/27)
- `CollectParameters(5)`: [0] ped, [1..3] target, [4] duration. Task give (0x3C, EH 2) `CTaskSimpleGunControl(nullptr, &CVector(P1,P2,P3),
  nullptr, 0, 500, P4)` 0x61F3F0. Return 0.

### 0x49578F 0688 COMMAND_TASK_TOGGLE_PED_THREAT_SCANNER  (gr: unregistered, used: 0/43/1)
- `CollectParameters(4)`: [0] ped or -1, [1..3] bools.
- `P0 != -1`: `ped = GetAtRef(P0)`; stack task `t(P1 != 0, P2 != 0, P3 != 0)` (ctor 0x492D10 at esp+0x6C, EH 9);
  `t.ProcessPed(ped)` 0x6337A0; vtable set to 0x85A334 and CTask destructor 0x61A3A0; return 0.
- `P0 == -1`: `new(0xC)` + same ctor (EH 10); `CTaskSequences::ms_taskSequence[ms_iActiveSequence [0x8D2E98]]` (0xC178F0 + seq*0x40)
  `.AddTask(task)` 0x632D10. No check that a sequence is open (seq = -1 indexes before the array: original bug). Return 0.

### 0x4944E9 0655 COMMAND_TASK_LOOK_AT_OBJECT  (gr: unregistered, used: 0/7/0)
- `CollectParameters(3)`: [0] ped or -1, [1] object, [2] time (-1 -> 20000, -2 -> 0x7FFFFFFF). `obj = ObjectPool.GetAt(P1)`.
- `P0 != -1`: `g_ikChainMan (0xC15448).LookAt("COMMAND_TASK_LOOK_AT_OBJECT" (0x85A3D4), GetAtRef(P0), obj, time, -1, nullptr, 0, 0.25f,
  500, 6, 1)` 0x618970. Return 0.
- `P0 == -1`: `new(0x30)` (EH 1) `0x634440(obj, time, -1, CVector(0,0,0), 1, 0.25f, 1000, 3)` (look-at trigger task ctor); then (0x4941CE)
  appended to the active sequence as in 0688. Return 0.

### 0x49590E 068B COMMAND_TASK_EVERYONE_LEAVE_CAR  (gr: unregistered, used: 0/10/0)
- `CollectParameters(1)`; `veh = GetAt(P0)` (no null check).
- Driver (+0x460) != 0: `new(0x14)` (EH 11) `CTaskComplexLeaveAnyCar(0, 1, 0)` 0x421150; `GivePedScriptedTask(PedPool.GetRef(driver), task,
  command)`.
- For i = 0 .. `(u8)veh->m_nMaxPassengers (+0x488)` - 1: p = `veh->m_apPassengers[i]` (+0x464 + 4i); if p: `delay =
  CGeneral::GetRandomNumberInRange(-250, 250)` 0x407180 (original int version; gr's differs, see cat14 finding 1) `+ 500*(i+1)`;
  `new(0x14)` (EH 12) `CTaskComplexLeaveAnyCar(delay, 1, 0)`; give to `GetRef(p)`. Return 0.

### 0x495D27 06A2 COMMAND_GET_CAR_SPEED_VECTOR / 0x495DAD 06A3 COMMAND_GET_CAR_MASS  (gr: unregistered, used: 0/4/0, 0/1/0)
- 06A2: `CollectParameters(1)`; `ScriptParams[0..2] = veh->m_vecMoveSpeed (+0x44) * [0x858B40] = 50.0` (each component x87, f32);
  StoreParameters(3). 06A3: `ScriptParams[0] = veh->m_fMass (+0x8C)`; StoreParameters(1). No null checks. Return 0.

### 0x4956D3 0685 COMMAND_IS_OBJECT_ATTACHED / 0x495719 0686 COMMAND_IS_VEHICLE_ATTACHED  (gr: unregistered, used: 0/28/11, 0/5/1)
- `CollectParameters(1)`; `flag = e != 0 && e->m_pAttachedTo (+0xFC) != 0`; UpdateCompareFlag. Return 0.

### 0x49467B 0657 COMMAND_OPEN_CAR_DOOR  (gr: unregistered, used: 0/44/2)
- `CollectParameters(2)`: [0] vehicle, [1] door. `node = CDamageManager::GetCarNodeIndexFromDoor(P1)` 0x6C26F0. If `veh->vtable[38](P1)`
  (+0x98) returns 0 and `veh->+0x648[node]` (car node frame) != 0: `veh->vtable[27](nullptr, node, P1, 1.0f, 1)` (+0x6C, OpenDoor). Return 0.

### Car door / panel / tyre: 0689, 068A, 0697, 0698, 0699  (gr: unregistered, used: 0/11/0, 0/6/0, 0/6/0, 0/1/0, 0/4/0)
| case | opcode | behaviour (veh = GetAt(P0), no checks) |
|---|---|---|
| 0x495882 | 0689 POP_CAR_DOOR | `CollectParameters(3)`; `CAutomobile::PopDoor(veh, GetCarNodeIndexFromDoor(P1), P1, P2 != 0)` 0x6ADEF0 |
| 0x4958D2 | 068A FIX_CAR_DOOR | `CollectParameters(2)`; `CAutomobile::FixDoor(veh, GetCarNodeIndexFromDoor(P1), P1)` 0x6A35A0 |
| 0x495AF4 | 0697 POP_CAR_PANEL | `CollectParameters(3)`; `CAutomobile::PopPanel(veh, GetCarNodeIndexFromPanel(P1) 0x6C26A0, P1, P2 != 0)` 0x6ADF80 |
| 0x495B44 | 0698 FIX_CAR_PANEL | `CollectParameters(2)`; `CAutomobile::FixPanel(veh, GetCarNodeIndexFromPanel(P1), P1)` 0x6A3670 |
| 0x495B80 | 0699 FIX_CAR_TYRE | `CollectParameters(2)`; `CAutomobile::FixTyre(veh, P1)` 0x6A3580 |

### 0x494767 065C COMMAND_REMOVE_DECISION_MAKER  (gr: unregistered, used: 6/68/19)
- `CollectParameters(1)` [0] handle h; `i = GetActualScriptThingIndex(h, 7)`. If `0 <= i < 20` and
  `ScriptResourceManager.RemoveFromResourceManager(h, 3, this)` 0x470510 (note: the handle, not i): 
  `CDecisionMakerTypesFileLoader::UnloadDecisionMaker(i)` 0x607A70. Then (regardless) if m_UsesMissionCleanup:
  `MissionCleanUp.RemoveEntityFromList(h, 9)`. Return 0.

### 0x4945D5 0656 COMMAND_LIMIT_ANGLE  (gr: unregistered, used: 0/8/0)
- `CollectParameters(1)`; `a = P0` (stored f32). If `a < 0` (`fcom [0x858B50] = 0.0; test ah,5; jp`): loop `a += [0x859E2C] = 360.0`
  while `a < 0` (in ST0, 0x494600..0x494611), store f32. If `a > 360.0` (`jne` skips <=, NaN): loop `a -= 360.0` while `a > 360`
  (0x494630..0x494641), store f32. `ScriptParams[0] = a`; StoreParameters(1). Return 0. (360 itself is kept.)

### 0x4953DD 0680 COMMAND_ADD_PEDTYPE_AS_ATTRACTOR_USER  (gr: unregistered, used: 0/2/0)
- `CollectParameters(2)`; `i = GetActualScriptThingIndex(P0, 6)`; if `0 <= i < 64`: `0x492CB0(this = 0xC3A200 + i*36
  (CScripted2dEffects::ms_userLists[i]), P1)` (jmp to .HOODLUM 0x156E990; adds a ped type). Return 0.

### 0x49539D 067F COMMAND_FORCE_CAR_LIGHTS  (gr: unregistered, used: 0/12/0)
- `CollectParameters(2)`; byte `veh+0x4A8`: bits 3..4 replaced by `(P1 << 3) & 0x18` (0x4953B8..0x4953D0). Return 0.

### 0x4944A1 0654 COMMAND_SET_OBJECT_RENDER_SCORCHED  (gr: unregistered, used: 0/2/0)
- `CollectParameters(2)`; obj +0x40 `|= 0x20000000` if P1 != 0 else `&= 0xDFFFFFFF` (bRenderScorched). Return 0.

### 0x4946E4 065B COMMAND_GET_PICKUP_COORDINATES  (gr: unregistered, used: 0/1/2)
- `CollectParameters(1)`; `i = CPickups::GetActualPickupIndex(P0)` 0x4552A0; result `(0, 0, -100.0)` (imm 0xC2C80000) unless `i != -1`,
  then `CPickups::aPickUps[i] (0x9788C0 + i*32).GetPosn()` 0x4549A0 (sret). `ScriptParams[0..2]`; StoreParameters(3). Return 0.

### 0x4947EC 0661 COMMAND_BREAKPOINT  (gr: unregistered, used: 0/0/0)
- `ReadTextLabelFromScript(buf, 40)` (skips the operand); nothing else. Return 0.

### 0x4952A9 067D COMMAND_ATTACH_CAMERA_TO_CHAR_LOOK_AT_VEHICLE  (gr: unimplemented, CharacterCommands.cpp:4462, used: 0/0/0)
- `CollectParameters(7)`: [0] ped, [1..3] offset, [4] vehicle, [5] f32, [6] switch style (u16). Two local copies of (P1,P2,P3) at esp+0x10
  and esp+0x3C. Joins 0x495170: `TheCamera.TakeControlAttachToEntity(VehiclePool.GetAt(P4), PedPool.GetAtRef(P0), &v1, &v2, P5,
  (u16)P6, 1)` 0x50C910 (pushes 0x4952D4..0x495175). Return 0.
- gta-reversed: unimplemented (returns -1, reads nothing) -> DEVIATION (unused by scripts).
- Open question 0x4952F2: the first vector pointer computes to esp+0x30 (relative to the post-prologue stack), not to either copy; verify
  the exact local before porting (unused opcode).

Open questions: 0x4952F2 (067D vector pointer).
