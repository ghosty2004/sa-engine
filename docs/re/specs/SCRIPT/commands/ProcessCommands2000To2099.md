# CRunningScript::ProcessCommands2000To2099 (SCRIPT) - opcode specs

Notation (shared by all ProcessCommands* spec files):
- `this` = CRunningScript* (EDI in the case blocks unless stated). `SP[i]` = ScriptParams[i] at 0xA43C78 + 4*i (i32 or f32 as used).
- `Collect(n)` = CRunningScript::CollectParameters 0x464080 (thiscall, n). `Store(n)` = StoreParameters 0x464370. `Cmp(b)` =
  UpdateCompareFlag 0x4859D0 (thiscall, bool pushed as a dword whose low byte is the flag). `ReadLabel(buf,8)` =
  ReadTextLabelFromScript 0x463D50. `VarPtr()` = GetPointerToScriptVariable 0x464790. `CollectNoPC()` =
  CollectNextParameterWithoutIncreasingPC 0x464250.
- `PED(h)` = CPedPool::GetAtRef 0x404910 on [0xB74490] CPools::ms_pPedPool (null when the slot flag byte != (h & 0xFF));
  `VEH(h)` = 0x4048E0 (catalogue FUN_004048e0 = CVehiclePool::GetAtRef, stride 0xA18, continues through the .text stub
  0x40210A, same semantics) on [0xB74494]; `OBJ(h)` = 0x465040 (FUN_00465040 = CObjectPool::GetAtRef, stride 0x19C) on
  [0xB7449C]. `PEDREF(p)` = 0x4442D0 (jmp stub to .HOODLUM 0x156CD40: ((p - base) / 0x7C4) << 8 | flags[idx], i.e.
  CPool<CPed>::GetRef).
- `ftol` = 0x821B40 (MSVC _ftol2: truncates ST0 toward zero, result EDX:EAX). `CRGBA(r,g,b,a)` = 0x7170C0 (thiscall ctor, ret 0x10).
- AL return 0 = OR_CONTINUE unless the case says otherwise (gr OpcodeResult: 0 continue, 1 wait, -1 interrupt).
- Memory stores are written `[0xADDR] <- value`. SEH: in switches that have an SEH frame (see the dispatcher), task-constructing cases set the frame state slot around `new` + ctor and back to -1; this only matters for exception unwinding.
- Vtable slots (index = offset/4): CEntity 0 dtor, 1 Add(rect), 2 Add, 3 Remove, 4 SetIsStatic, 5 SetModelIndex, 6
  SetModelIndexNoCreate, 7 CreateRwObject, 8 DeleteRwObject, 9 GetBoundRect, 10 ProcessControl, 11 ProcessCollision, 12
  ProcessShift, 13 TestCollision, 14 Teleport, 15 SpecialEntityPreCollisionStuff, 16 SpecialEntityCalcCollisionSteps, 17 PreRender,
  18 Render, 19 SetupLighting, 20 RemoveLighting, 21 FlagToDestroyWhenNextProcessed, 22 ProcessEntityCollision (CPhysical);
  CEntity slot 4 (+0x10) SetIsStatic, slot 14 (+0x38) Teleport(CVector, bool). CPhysical +0x44 m_vecMoveSpeed, +0x50 m_vecTurnSpeed, +0x8C m_fMass, +0x90 m_fTurnMass, +0xA4 m_vecCentreOfMass; CEntity +0x1C flags dword (bit 0x04 bIsStatic). CVehicle +0x38C handling flags (0x20000 = hydraulics installed), +0x590 m_nVehicleType, +0x594 m_nVehicleSubType (0 automobile, 3 heli, 4 plane). CObject +0x140 object flags dword.

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- Vector helpers (reading aids; call them via `orig!` or reproduce exactly): Multiply3x3 0x59C790 (cdecl `(CVector* out, CMatrix*
  m, CVector* v)`, returns out), VectorSub 0x40FE60 (cdecl `(out, a, b)`: out = a - b), CVector::CrossProduct 0x59C730 (cdecl
  `(out, a, b)`), 0x406DA0 = CVector::SquaredMagnitude (thiscall, ST0 = (x*x + y*y) + z*z), 0x40FEF0 = CVector::operator*=(float)
  (each component `k * c`), 0x411A00 = CVector::operator+=(const CVector&) (each `a.c + this.c`), 0x4119D0 = operator/(CVector, float)
  (cdecl `(out, v, f)`: `k = 1.0f/f` stored f32 first, then `out = (v.x*k, v.y*k, v.z*k)` - multiply by the reciprocal, not a
  division), CVector::Normalise 0x59C910, CVector::Magnitude 0x4082C0.
- Model id rule ("UsedObjectArray"): `if (m < 0) m = *(i32*)(0xA44B88 - m*0x1C)` = CTheScripts::UsedObjectArray[-m].index
  (array base 0xA44B70, stride 0x1C).

## 0x472310 CRunningScript::ProcessCommands2000To2099  (catalogue: specified/REA, port level 11)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x47233F), command id at [esp+4] kept in EBP (`mov ebp,[esp+0x1B0]` at 0x47232D),
  `ret 4`. SEH frame `push -1; push 0x83821C; push fs:[0]`, `sub esp,0x198`, push EBX/EBP/ESI/EDI. In cases: command id also at
  `[esp+0x1B8]`, SEH state slot `[esp+0x1B0]`. Range: `eax = id - 0x7D0` (0x472335), `cmp eax,0x63; ja 0x474372`,
  `jmp [eax*4 + 0x474390]` (100 dwords).
- Default 0x474372: `or al,0xFF` (AL = -1). Tail 0x474374: `nop; nop; jmp 0x4016AD` -> .text stub `mov ecx,[esp+0x1A8]; jmp 0x47437B`
  -> `pop edi/esi/ebp; fs:[0] <- ecx; pop ebx; add esp,0x1A4; ret 4`. Cases end `xor al,al; jmp 0x474374`. Shared compare tails:
  0x473F33 (`add esp,4; if al: scratch [esp+0x10] = 1; Cmp(scratch)`), 0x472DDF (`Cmp(scratch)`), 0x473F4B (`xor al,al`).
- Table: 100 entries; 8 to the default (07D8 07D9 0802 082B-082F), 24 to the empty case 0x47436E (`xor al,al`, AL = 0, no parameters:
  07D1 07D2 07D4 07DC 07E2 07E3 07E9 07EA-07ED 07F4 0805 0807-0809 080C 080D 080F 0813 081B 0821 0824 0832; gr nop except 07E9 080C
  080D 0821), 68 real cases. gta-reversed handler 20, nop 22 (not re-specified). Specified below: 46 unregistered + 4 unimplemented
  with a case.
- Default-reaching (all gr unimplemented -> -1, same as the exe): 07D8 07D9 0802 082B 082C 082D 082E 082F.
- gta-reversed deviation: 07E9 HAS_CHAR_SPOTTED_CAR, 080C GET_CHAR_BREATH, 080D SET_CHAR_BREATH, 0821 IS_3D_COORD_IN_ZONE are gr
  `unimplemented` (-1) but run the empty case here (AL = 0, parameters not read).

### 0x4730A8 07FB COMMAND_SWITCH_ENTRY_EXIT  (gr: unregistered, used: 14 / 172 / 38)
- `ReadLabel(name, 8)`; `Collect(1)`; `CEntryExitManager::SetEnabledByName(name, SP0 != 0)` 0x43F9B0 (cdecl; two call copies with
  `push 1` / `push 0`).

### 0x472845 07E0 COMMAND_SET_BLIP_AS_FRIENDLY  (gr: unregistered, used: 0 / 342 / 3)
- `Collect(2)`; `CRadar::SetBlipFriendly(SP0, (u8)SP1)` 0x583EB0 (cdecl).

### 0x473831 0812 COMMAND_TASK_PLAY_ANIM_NON_INTERRUPTABLE  (gr: unregistered, used: 37 / 288 / 229)
- `this->PlayAnimScriptCommand(commandId)` 0x470150 (EBP pushed). AL = 0.

### 0x472FD4 07F7 COMMAND_SET_OBJECT_COLLISION_DAMAGE_EFFECT  (gr: unregistered, used: 7 / 71 / 0)
- `Collect(2)`; `o = OBJ(SP0)` (unchecked). If `SP1 != 0`: if `o[+0x145] != 0`: `o[+0x144] <- o[+0x145]`, `o[+0x145] <- 0`.
  Else: if `o[+0x144] != 0`: `o[+0x145] <- o[+0x144]`, `o[+0x144] <- 0` (byte swap-out/in of the collision damage effect, saved in
  +0x145).

### 0x473840 0814 COMMAND_ADD_STUNT_JUMP  (gr: unregistered, used: 0 / 70 / 0)
- `Collect(16)`: start box centre SP0..SP2 + half size SP3..SP5; landing box centre SP6..SP8 + half size SP9..SP11; camera position
  SP12..SP14; reward SP15 (i32).
- Box 1 (0x473849-0x4738F6): `maxX = SP3 + SP0`, `maxY = SP4 + SP1`, `maxZ = SP5 + SP2`, `minX = SP0 - SP3`, `minY = SP1 - SP4`,
  `minZ = SP2 - SP5` (each x87 add/sub of two f32 values, stored f32; computed in the order maxX, maxY, maxZ, minX, minY, minZ);
  `CBoundingBox::SetMinMax(&box1, min, max)` 0x470100 (thiscall, two CVectors by value).
- Box 2 (0x4738FB-0x4739A8): same with SP6..SP11 (`maxX = SP9 + SP6` ... `minZ = SP8 - SP11`); `SetMinMax(&box2, min, max)`.
- `CStuntJumpManager::AddOne(&box1, &box2, &CVector(SP12, SP13, SP14), SP15)` 0x49CB40 (cdecl).

### 0x47286C 07E1 COMMAND_TASK_SWIM_TO_COORD  (gr: unregistered, used: 0 / 7 / 0)
- `Collect(4)`: SP0 ped (or -1), SP1..SP3 target.
- If `SP0 != -1` and `p = PED(SP0)` is non-null and `p->m_pIntelligence->GetTaskSwim()` 0x601070 is non-null: write the target into
  that swim task at +0x14 (x, y, z; GetTaskSwim is called twice) and return AL = 0 (no new task).
- Else: `t = new(0x64) CTaskSimpleSwim(&target, null)` 0x688930 (SEH state 0); `GivePedScriptedTask(SP0, t, commandId)` (EBP).

### 0x473C38 0817 COMMAND_TASK_FOLLOW_PATROL_ROUTE  (gr: unregistered, used: 0 / 46 / 0)
- `Collect(3)`: SP0 ped, SP1 move state, SP2 mode. `t = new(0x30)`; ctor `FUN_00674930(t, SP1, &route 0xC18DB8, SP2, [0x86FD08]
  (0.5f), [0x86FD0C] (5.0f))` (proposed name CTaskComplexFollowPatrolRoute::CTaskComplexFollowPatrolRoute(moveState, route, mode,
  targetRadius, slowDownDist); evidence: script patrol route global 0xC18DB8 and the opcode); SEH state 6;
  `GivePedScriptedTask(SP0, t, commandId [esp+0x1B8])`.

### 0x472677 07DB COMMAND_SET_CAR_ROTATION_VELOCITY  (gr: unregistered, used: 0 / 45 / 0)
- `Collect(4)`; `v = VEH(SP0)` (unchecked). `w = (ts*SP1, ts*SP2, ts*SP3)` with `ts = [0xB7CB5C] CTimer::ms_fTimeStep` (fld ts; fmul SP;
  stored f32). `d = Multiply3x3(v->m_matrix, &w)`.
- If `v[+0x1C] & 4` (static): `v->SetIsStatic(false)` (slot 4) and `CPhysical::AddToMovingList(v)` 0x542800.
- `CPhysical::SetTurnSpeed(v, d.x*0.02f, d.y*0.02f, d.z*0.02f)` 0x45AFB0 (jmp stub to .HOODLUM 0x15658A0; thiscall 3 floats, writes
  +0x50) with `[0x858B38] = 0.02` (each product stored f32; z computed first, then y, then x).

### 0x473F8A 0823 COMMAND_TASK_GREET_PARTNER  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(4)`: SP0 ped A, SP1 ped B, SP2 distance multiplier (f32), SP3 handshake type. `a = PED(SP0)`, `b = PED(SP1)`.
- `t1 = new(0x78) CTaskComplexPartnerGreet("COMMAND_TASK_GRE..." [0x859E8C], b, leadSpeaker 1, SP2, SP3, CVector(0,0,0))` 0x684210
  (SEH 7); `GivePedScriptedTask(SP0, t1, commandId)`.
- `t2 = new(0x78) CTaskComplexPartnerGreet([0x859E8C], a, 0, SP2, SP3, CVector(0,0,0))` (SEH 8); `GivePedScriptedTask(SP1, t2,
  commandId)` (SP0/SP1 re-read from ScriptParams).

### 0x4740DC 0825 COMMAND_SET_HELI_BLADES_FULL_SPEED  (gr: unregistered, used: 0 / 35 / 0)
- `Collect(1)`; `v = VEH(SP0)` (unchecked); `s = v->m_nVehicleSubType (+0x594)`; if `s == 3` (heli): `v[+0x84C] <- 0.22f`
  (imm 0x3E6147AE); else if `s == 4` (plane): `v[+0x9C4] <- 0.18f` (imm 0x3E3851EC).

### 0x474128 0826 COMMAND_DISPLAY_HUD  (gr: unregistered, used: 0 / 38 / 8)
- `Collect(1)`; `[0xA444A0] CTheScripts::bDisplayHud <- (SP0 != 0)`.

### 0x4723C6 07D5 COMMAND_APPLY_FORCE_TO_CAR  (gr: unregistered, used: 2 / 22 / 0)
- `Collect(7)`: SP0 vehicle, F = (SP1, SP2, SP3) force, P = (SP4, SP5, SP6) offset. `v = VEH(SP0)` (unchecked); `com = &v->m_vecCentreOfMass`.
- `P += Multiply3x3(v->m_matrix, com)` (operator+=, 0x47243C).
- `n = F; n.Normalise()`.
- `r = P - Multiply3x3(v->m_matrix, com)` (recomputed; VectorSub). `c = CrossProduct(r, n)`.
- `k = 1.0f / (c.SquaredMagnitude() / v->m_fTurnMass + 1.0f / v->m_fMass)` (x87: `sq / turnMass`, then `1.0 / mass`
  ([0x858624] = 1.0), `faddp` = `(sq/turnMass) + (1/mass)`, `fdivr` 1.0; stored f32); `F *= k` (0x40FEF0).
- `CPhysical::ApplyForce(v, F, P, true)` 0x542B50 (thiscall, two CVectors by value + bool).

### 0x473350 0803 COMMAND_DOES_CAR_HAVE_HYDRAULICS  (gr: unregistered, used: 1 / 21 / 0)
- `Collect(1)`; `v = VEH(SP0)` (unchecked); `Cmp(v->m_nVehicleSubType == 0 && (v[+0x38C] & 0x20000))`.

### 0x4732B1 07FF COMMAND_SET_CAR_HYDRAULICS  (gr: unregistered, used: 0 / 14 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); `m = (u16) [0x8CD76C] MI_HYDRAULICS`; `SP1 != 0 ? CVehicle::AddVehicleUpgrade(v, m)`
  0x6E3290 `: CVehicle::RemoveVehicleUpgrade(v, m)` 0x6DF930.

### 0x472EBC 07F5 COMMAND_CONTROL_CAR_HYDRAULICS  (gr: unregistered, used: 1 / 16 / 0)
- `Collect(5)`; `v = VEH(SP0)` (unchecked). If `(i8) v->m_vehicleSpecialColIndex (+0x48B) < 0`: `v->GetSpecialColModel()` 0x6DF3D0.
  `i = (i8) v[+0x48B]`; if `i <= -1` -> done. Else (re-reading +0x48B before each store) `[0xC1CB78 + i*0x28] <- SP1`,
  `[0xC1CB7C + i*0x28] <- SP2`, `[0xC1CB80 + i*0x28] <- SP3`, `[0xC1CB84 + i*0x28] <- SP4` (f32 via fld/fstp; special hydraulic data
  table of CVehicle, stride 0x28).

### 0x472C63 07F0 COMMAND_HAS_OBJECT_OF_TYPE_BEEN_SMASHED  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(5)`: centre x SP0, y SP1, z SP2 (if `z <= -100.0f` [0x859014]: `z = FindGroundZForCoord(x, y)` 0x569660), radius SP3,
  model SP4 (UsedObjectArray rule).
- `pool = [0xB7449C]`; `found = 0`; for `i = pool->m_nSize (+8) - 1` down to 0, stopping once found: `o = CObjectPool::GetAt(pool, i)`
  0x404870; if `o` and `o[+0x140] & 0x400` and `(i16) o->m_nModelIndex == model`: `d = |centre - o->GetPosition()|` (matrix pos or
  placement pos; VectorSub + Magnitude); if `d <= radius` (fcomp + `test ah,0x41; jp`: less or equal passes, NaN fails) `found = 1`.
  `Cmp(found)`.

### 0x472510 07D6 COMMAND_IS_INT_LVAR_EQUAL_TO_INT_VAR  (gr: unregistered, used: 5 / 27 / 28)
- `a = this->GetPointerToScriptVariable(1)`, `b = this->GetPointerToScriptVariable(2)` (0x464790, argument values 1 then 2);
  `Cmp(*b == *a)` (i32).

### 0x47253A 07D7 COMMAND_IS_FLOAT_LVAR_EQUAL_TO_FLOAT_VAR  (gr: unregistered, used: 0)
- Same two pointers; `Cmp(*(f32*)b == *(f32*)a)` (fcomp + `test ah,0x44; jnp`: exact equality, NaN -> false).

### 0x473617 080A COMMAND_GET_LEVEL_DESIGN_COORDS_FOR_OBJECT  (gr: unregistered, used: 4 / 0 / 34)
- `Collect(2)`; `o = OBJ(SP0)` (unchecked); `p = CEntity::FindTriggerPointCoors(o, &tmp, SP1)` 0x533380; `SP0..SP2 <- p`; `Store(3)`
  (shared tail 0x47377A).

### 0x472F57 07F6 COMMAND_GET_GROUP_SIZE  (gr: unregistered, used: 0 / 5 / 2)
- `Collect(1)`; `g = GetActualScriptThingIndex(SP0, 8)`. If `0 <= g < 8`: `m = &ms_groups[g].m_groupMembership (0xC09928 + g*0x2D4)`;
  `SP0 <- (m->GetLeader() != null)` (neg/sbb/neg = 0/1), `SP1 <- m->CountMembersExcludingLeader()`. Else `SP0 <- 0, SP1 <- 0`. `Store(2)`.

### 0x4731CE 07FD COMMAND_DOES_GROUP_EXIST  (gr: unregistered, used: 0 / 13 / 2)
- `Collect(1)`; `Cmp(0 <= GetActualScriptThingIndex(SP0, 8) < 8)` (only the handle is validated; ms_activeGroups is not read).

### 0x472ABF 07E7 COMMAND_TASK_DRIVE_POINT_ROUTE_ADVANCED  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(6)`: SP0 ped, SP1 vehicle (`< 0` -> null else `VEH(SP1)`), SP2 cruise speed (f32), SP3 mode, SP4 car model selector,
  SP5 driving style. `model = SP4 == 0 ? -1 : SP4 == 1 ? 0x19F (415) : SP4`.
- `t = new(0x2C) CTaskComplexDrivePointRoute(veh, &route 0xC18D50 (script point route global), SP2, SP3, model, -1.0f, SP5)` 0x6433E0
  (SEH 1); `GivePedScriptedTask(SP0, t, commandId [esp+0x1B8])`.

### 0x4741AB 0829 COMMAND_TASK_DIE_NAMED_ANIM  (gr: unregistered, used: 0 / 20 / 5)
- `Collect(1)`: ped = SP0 (kept in EBX). `ReadLabel(anim, 0x18)`; `ReadLabel(block, 0x10)`; `Collect(2)`: SP0 blend delta (f32), SP1
  flag. `flags = SP1 ? 0xD0 : 0x10`. `t = new(0x28) CTaskSimpleDie(anim, block, flags, SP0, 1.0f)` 0x62FA60 (SEH 9);
  `GivePedScriptedTask(ped, t, commandId)` (EBP).

### 0x473A01 0815 COMMAND_SET_OBJECT_COORDINATES_AND_VELOCITY  (gr: unregistered, used: 2 / 10 / 6)
- `Collect(4)`; `o = OBJ(SP0)` (unchecked); target `T = (SP1, SP2, z)`, `z = SP3`, if `z <= -100.0f`: `z = FindGroundZForCoord(SP1, SP2)`.
- `pos = o->m_matrix ? matrix pos : placement pos`. `vel = (T - pos) / ts` via 0x4119D0 (`k = 1.0f / ts` with `ts = [0xB7CB5C]`,
  then `vel.c = k * d.c`); `o->m_vecMoveSpeed (+0x44) <- vel`.
- Clamp each of +0x44, +0x48, +0x4C: `if (c < -1.0f) c = -1.0f` ([0x858C1C] = -1.0; `test ah,5; jp`), then `if (c > 1.0f) c = 1.0f`
  ([0x858624] = 1.0; `test ah,0x41; jne`); NaN stays.
- `CTrain::MarkSurroundingEntitiesForCollisionWithTrain(T, 25.0f, o, false)` 0x6F6640 (cdecl). `o->Teleport(T, false)` (slot 14,
  +0x38). `CTheScripts::ClearSpaceForMissionEntity(&T, o)` 0x486B00.

### 0x472576 07DA COMMAND_ADD_TO_CAR_ROTATION_VELOCITY  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(4)`; `v = VEH(SP0)` (unchecked). `w = (SP1 * 0.02f, SP2 * 0.02f, SP3 * 0.02f)` ([0x858B38]; stored f32); `cur = v->m_vecTurnSpeed`
  (copied before); `d = Multiply3x3(v->m_matrix, &w)`; `new = (d.x + cur.x, d.y + cur.y, d.z + cur.z)` (stored f32).
- If static (`+0x1C & 4`): `SetIsStatic(false)`, `AddToMovingList()`. `v->m_vecTurnSpeed (+0x50) <- new`.

### 0x4742AC 0830 COMMAND_SET_POOL_TABLE_COORDS  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(6)`; CWorld::SnookerTableMin (0x8CDF00..0x8CDF08) `<- (SP0, SP1, SP2)`; CWorld::SnookerTableMax (0x8CDEF4..0x8CDEFC)
  `<- (SP3, SP4, SP5)` (raw dword copies; store order: max.z, max.x, max.y, min.z, min.x, min.y).

### 0x474322 0833 COMMAND_HAS_OBJECT_BEEN_PHOTOGRAPHED  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(1)`; `o = OBJ(SP0)`; scratch = 0; if `o` and `o[+0x140] & 0x1000`: clear that bit, scratch = 1 (read-and-reset); `Cmp(scratch)`.

### 0x4727B4 07DF COMMAND_REMOVE_OIL_PUDDLES_IN_AREA  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(4)`: x1 SP0, y1 SP1, x2 SP2, y2 SP3; swap to x1 <= x2 then y1 <= y2 (NaN not swapped); `CShadows::RemoveOilInArea(x1, x2, y1,
  y2)` 0x7074F0 (cdecl).

### 0x4736A6 0810 COMMAND_GET_PARKING_NODE_IN_AREA  (gr: unregistered, used: 0 / 1 / 2)
- `Collect(6)`: x1,y1,z1,x2,y2,z2; swap per axis (x, y, z) to min/max; `p = CPathFind::FindParkingNodeInArea(&ThePaths, &tmp, x1, x2, y1,
  y2, z1, z2)` 0x4513F0; `SP0..SP2 <- p`; `Store(3)`.

### 0x472C1B 07EF COMMAND_GET_CITY_FROM_COORDS  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(3)`; `SP0 <- CTheZones::GetLevelFromPosition(&CVector(SP0, SP1, SP2))` 0x572300; `Store(1)` (tail 0x472C4B).

### 0x473E51 081D COMMAND_SET_CAR_ENGINE_BROKEN  (gr: unregistered, used: 0 / 0 / 1)
- `Collect(2)`; `v = VEH(SP0)` (unchecked). If `SP1 != 0`: `v[+0x42D] |= 2` (bEngineBroken), `v[+0x428] &= 0xEF` (engine off). Else
  `v[+0x42D] &= 0xFD`.

### 0x473EA8 081E COMMAND_IS_THIS_MODEL_A_BOAT / 0x473EE1 081F ..._A_PLANE / 0x473F19 0820 ..._A_HELI  (gr: unregistered, used: 0 / 7 / 0 each)
- `Collect(1)`; `Cmp(CModelInfo::IsBoatModel(SP0) 0x4C5A70 / IsPlaneModel 0x4C5B30 / IsHeliModel 0x4C5B00 != 0)`.

### 0x474148 0827 COMMAND_CONNECT_LODS  (gr: unregistered, used: 0 / 7 / 0)
- `Collect(2)`; `a = OBJ(SP0)`, `b = OBJ(SP1)`; `CTheScripts::ScriptConnectLodsFunction(SP0, SP1)` 0x470A20; `CTheScripts::
  AddToListOfConnectedLodObjects(a, b)` 0x470980 (cdecl, cleaned together `add esp,0x10`).

### 0x47418F 0828 COMMAND_SET_MAX_FIRE_GENERATIONS  (gr: unregistered, used: 0 / 15 / 0)
- `Collect(1)`; `[0xB728E0] <- SP0` (gFireManager 0xB71F80 + 0x960, max fire generations).

### 0x472932 07E4 COMMAND_GET_MODEL_DIMENSIONS  (gr: unregistered, used: 2 / 3 / 2)
- `Collect(1)`; `m` = SP0 (UsedObjectArray rule); `cm = ms_modelInfoPtrs[m]->m_pColModel (+0x14)` (no checks); `SP0..SP2 <- cm[+0..+8]`
  (bounding box min), `SP3..SP5 <- cm[+0xC..+0x14]` (max); `Store(6)`.

### 0x47303D 07F8 COMMAND_SET_CAR_FOLLOW_CAR  (gr: unregistered, used: 0 / 5 / 0)
- `Collect(3)`; `CCarAI::TellCarToFollowOtherCar(VEH(SP0), VEH(SP1), SP2 (f32))` 0x41C960 (cdecl; lookups SP1 first, then SP0).

### 0x472B88 07E8 COMMAND_IS_RELATIONSHIP_SET  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(3)`; `Cmp((CPedType::GetPedTypeAcquaintances(SP0, SP1) 0x6089D0 & CPedType::GetPedFlag(SP2) 0x608830) != 0)`.

### 0x472A32 07E6 COMMAND_COPY_GROUP_DECISION_MAKER  (gr: unregistered, used: 0 / 1 / 1)
- `Collect(1)`; `src = SP0 == -1 ? -1 : GetActualScriptThingIndex(SP0, 7)`; `GetActualScriptThingIndex(CollectNoPC(), 7)` evaluated
  and discarded. `dm = FUN_006070F0(CDecisionMakerTypes::GetInstance() 0x4684F0, src, 1, (u8) m_UsesMissionCleanup)` (thiscall;
  proposed name CDecisionMakerTypes::CopyDecisionMaker(srcIndex, type 1 = group, forMission); returns the new index).
  `SP0 <- GetNewUniqueScriptThingIndex(dm, 7)`; `Store(1)`; if `m_UsesMissionCleanup`: `MissionCleanUp.AddEntityToList(SP0, 9)`.

### 0x47236A 07D3 COMMAND_REGISTER_SCRIPT_BRAIN_FOR_CODE_USE  (gr: unregistered, used: 1 / 0 / 0)
- `Collect(1)`; `SP0 <- (i32)(i16) CStreamedScripts::GetProperIndexFromIndexUsedByScript(&CTheScripts::StreamedScripts 0xA47B60, (u16)SP0)`
  0x470810; `ReadLabel(name, 8)`; `CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse(&ScriptsForBrains 0xA90CF0, (u16)SP0, name, 3)`
  0x46A9C0.

### 0x47234E 07D0 COMMAND_GET_CURRENT_DAY_OF_WEEK  (gr: unregistered, used: 11 / 0 / 2)
- `SP0 <- (u8) [0xB7014E] CClock::CurrentDay`; `Store(1)`.

### 0x472786 07DE COMMAND_IS_MODEL_IN_CDIMAGE  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(1)`; model rule; scratch = 0; `Cmp(CStreaming::IsObjectInCdImage(m) 0x407800 != 0)` (tail 0x473F33).

### 0x4732FE 0800 COMMAND_IS_2PLAYER_GAME_GOING_ON  (gr: unregistered, used: 0 / 0 / 1)
- `Cmp(CGameLogic::IsCoopGameGoingOn() 0x441390 != 0)`.

### 0x4735FD 0806 COMMAND_GET_TOTAL_NUMBER_OF_PEDS_KILLED_BY_PLAYER  (gr: unregistered, used: 0 / 0 / 2)
- `Collect(1)`; `SP0 <- CDarkel::FindTotalPedsKilledByPlayer(SP0)` 0x43D6E0; `Store(1)`.

### 0x472E4A 07F3 COMMAND_SET_CHECKPOINT_COORDS  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(4)`; `i = GetActualScriptThingIndex(SP0, 3)`; if `i < 0` -> done (no upper bound check). `e = *(void**)(0xA44074 + i*8)`
  (CTheScripts::ScriptCheckpointArray[i] pointer field); if non-null: `CCheckpoints::UpdatePos(e[+4], &CVector(SP1, SP2, SP3))` 0x722900.

### 0x47436E 07E9, 080C, 080D, 0821  (gr: unimplemented, used: 0)
- Empty shared case: AL = 0, no parameters read (deviation vs gr, see dispatcher).

- gta-reversed: none of the unregistered opcodes has gr code. Open questions: proposed names for FUN_00674930 (0817) and FUN_006070F0
  (07E6) need confirmation; meaning of object byte +0x144/+0x145 (07F7) and object flags 0x400 / 0x1000 (07F0, 0833) are not
  named from gr (offsets/bits exact).
