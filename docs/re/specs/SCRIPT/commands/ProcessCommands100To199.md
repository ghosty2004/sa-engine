# CRunningScript::ProcessCommands100To199 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` = `CPool<CVehicle>::GetAtRef(h)` 0x4048E0 (thiscall,
ECX = CPools::ms_pVehiclePool [0xB74494], stride 0xA18, null when the slot flag byte != h & 0xFF). `GetRef` 0x424160 =
CPool<CVehicle>::GetRef (.HOODLUM 0x15667F0: `((p - base) / 0xA18) << 8 | flags[idx]`).
CVehicle fields: +0x14 m_matrix, +0x04 placement pos, +0x36 type(bits0-2)/status(bits3-7), +0x384 m_pHandlingData,
+0x390 m_autoPilot (CAutoPilot: +0x1C m_nTimeToStartMission = veh+0x3AC, +0x2A m_nCarMission = veh+0x3BA,
+0x40 m_nCruiseSpeed u8 = veh+0x3D0). Mission values: 0 NONE, 1 CRUISE, 8 GOTOCOORDINATES, 9 GOTOCOORDINATES_STRAIGHTLINE,
0x39 = 57 PLANE_CRASH_AND_BURN, 0x3A = 58 HELI_CRASH_AND_BURN.
Helpers: 0x41BDD0 = CVehicle::SetEngineOn(bool) (thiscall; body .HOODLUM 0x1567620: if bEngineBroken (byte [veh+0x42D] & 2)
then clear bEngineOn (bit 4 of [veh+0x428]) else bEngineOn = arg). 0x463490 = CAutoPilot::SetCarMission(u8) (thiscall on
&veh->m_autoPilot; body 0x1565620: `if (m != 0x39 && m != 0x3A) m_nCarMission = arg`). 0x821B40 = MSVC `__ftol2`
(truncation toward zero, result EDX:EAX).
"Keep crash missions" below = the inlined pattern `if (mission != 0x39 && mission != 0x3A) mission = X`.

## 0x466DE0 CRunningScript::ProcessCommands100To199  (catalogue: specified/REA, port level 27)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x837D7C,
  0x466DE0..0x466DF2), `sub esp,0x38`, push esi/edi; this in ESI (0x466E04). `eax = command - 0x64` (0x466DFD),
  `cmp eax,0x60; ja 0x46834F` (0x466E00/0x466E06): only 100..196 are dispatched, 197..199 (00C5..00C7) always reach the
  default. Single-level table `jmp [eax*4+0x468364]` (0x466E0C).
- Default 0x46834F: `or al,0xFF`, restore fs:[0] from [esp+0x40], pop edi/esi, `add esp,0x44; ret 4` -> returns -1.
  Cases end with their own copy of the epilogue (`xor al,al` ... `ret 4`) -> 0 = continue.
- Opcodes: 82 with a case: 70 gta-reversed `handler` (not re-specified), 12 `unregistered` specified below (0098, 00A5..00AB,
  00AD, 00AF, 00B0, 00B1).
- Default-reaching (18): 009C, 009D, 009E, 009F, 00A2, 00AC, 00B2..00B9, 00BD, 00C7 (gr `unimplemented`) and
  **00C5 COMMAND_RETURN_TRUE, 00C6 COMMAND_RETURN_FALSE**, which gta-reversed implements as handlers
  (Commands/BasicCommands.cpp:382-383) but which in the exe are above the range check (0x466E00 `cmp eax,0x60`) and return
  -1 from the default (a gta-reversed deviation: a port of the original must return the default value, not set the
  compare flag).

### 0x467AB3 00A5 COMMAND_CREATE_CAR  (gr: unregistered, used: 6/956/29)
- Parameters: CollectParameters(4) (0x467AB7): SP0 = model id (int), SP1/SP2/SP3 = x/y/z (f32). Output: SP0 = vehicle handle.
- Behaviour: `veh = CCarCtrl::CreateCarForScript(SP0, CVector(SP1,SP2,SP3) by value, (bool)this->m_UsesMissionCleanup)`
  (0x431F80, cdecl, 5 dwords: model, x, y, z, byte [esi+0xC6] zero-extended, 0x467AD3..0x467B01, `add esp,0x14`).
  `SP0 = GetRef(veh)` (0x467B10, ECX = vehicle pool), shared tail 0x468294: `StoreParameters(1)`, return 0.
- Edge: no null check before GetRef (CreateCarForScript result passed directly, 0x467B0F).
- gta-reversed: absent. Open questions: none.

### 0x467B1A 00A6 COMMAND_DELETE_CAR  (gr: unregistered, used: 3/511/10)
- Parameters: CollectParameters(1) (0x467B1E): SP0 = vehicle handle.
- Behaviour: `veh = veh(SP0)` (0x467B30). If non-null (0x467B39): `CWorld::Remove(veh)` 0x563280 (cdecl),
  `CWorld::RemoveReferencesToDeletedObject(veh)` 0x565510 (cdecl), then `veh->vtable[0](1)` (scalar deleting destructor,
  0x467B47..0x467B50). Then, regardless of whether veh was found, if `this->m_UsesMissionCleanup` (0x467B52):
  `CTheScripts::MissionCleanUp.RemoveEntityFromList(SP0, 1 /*vehicle*/)` (0x4654B0, ECX = 0xA90850, 0x467B5C..0x467B69).
  Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467B83 00A7 COMMAND_CAR_GOTO_COORDINATES  (gr: unregistered, used: 0/57/0)
- Parameters: CollectParameters(4) (0x467B87): SP0 = vehicle handle, SP1/SP2/SP3 = x/y/z (f32).
- Behaviour:
  1. `v = veh(SP0)` (0x467B99; no null check, crashes on a dead handle).
  2. `z = SP3` (f32 local [esp+0x50], 0x467BAA). `fcomp [0x859014]=-100.0` with `test ah,0x41; jp skip` (0x467BAE..0x467BC8):
     if `z <= -100.0f` (ordered; NaN skips) -> `z = CWorld::FindGroundZForCoord(SP1, SP2)` (0x569660, cdecl, ST0 stored f32).
  3. `z = GetDistanceFromCentreOfMassToBaseOfModel(v)` (0x536BE0, thiscall, ST0) `+ z` (0x467BE3), stored f32 into the
     target vector (SP1, SP2, z) at [esp+0x34].
  4. `ok = CCarCtrl::JoinCarWithRoadSystemGotoCoors(v, &target, false, false)` (0x42F870, cdecl 4 args, 0x467BEF..0x467C05).
  5. If ok: keep crash missions, else mission = 9 (0x467C17..0x467C1F); if !ok: keep crash missions, else mission = 8
     (0x467C28..0x467C30).
  6. `[v+0x36] = ([v+0x36] & 7) | 0x18` (status = 3 STATUS_PHYSICS, 0x467C37..0x467C42).
  7. `v->SetEngineOn(true)` (0x467C45).
  8. `cruise = [v+0x3D0]; if (cruise <= 1) cruise = 1` (unsigned compare, 0x467C4A..0x467C5C).
  9. `[v+0x3AC] = CTimer::m_snTimeInMilliseconds` [0xB7CB84] (0x467C62). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467C83 00A8 COMMAND_CAR_WANDER_RANDOMLY  (gr: unregistered, used: 0/15/0)
- Parameters: CollectParameters(1): SP0 = vehicle handle.
- Behaviour: `v = veh(SP0)` (0x467C99, no null check); `CCarCtrl::JoinCarWithRoadSystem(v)` 0x42F5A0 (cdecl, 0x467CA1);
  keep crash missions else mission = 1 CRUISE (0x467CA6..0x467CB7); `v->SetEngineOn(true)` (0x467CC2); cruise = max(cruise,1)
  as in 00A7 step 8 (0x467CC7..0x467CD9); `[v+0x3AC] = CTimer::m_snTimeInMilliseconds` (0x467CDF). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467CFF 00A9 COMMAND_CAR_SET_IDLE  (gr: unregistered, used: 0/44/1)
- Parameters: CollectParameters(1): SP0 = vehicle handle.
- Behaviour: `v = veh(SP0)` (0x467D15, no null check); keep crash missions else mission = 0 NONE ([v+0x390+0x2A],
  0x467D1A..0x467D2F). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467D48 00AA COMMAND_GET_CAR_COORDINATES  (gr: unregistered, used: 7/234/6)
- Parameters: CollectParameters(1): SP0 = vehicle handle. Output SP0..SP2 = x, y, z.
- Behaviour: `v = veh(SP0)` (0x467D5E, no null check); `pos = v->m_matrix ? &m_matrix->pos (+0x30) : &v->placement.pos (+4)`
  (0x467D63..0x467D6F); SP0/SP1/SP2 = pos.x/y/z copied as raw dwords (0x467D72..0x467D96); StoreParameters(3) (0x467D9B).
  Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467DB5 00AB COMMAND_SET_CAR_COORDINATES  (gr: unregistered, used: 3/310/5)
- Parameters: CollectParameters(4): SP0 = vehicle handle, SP1/SP2/SP3 = x/y/z.
- Behaviour: `CCarCtrl::SetCoordsOfScriptCar(veh(SP0), SP1, SP2, SP3, false, true)` (0x4342A0, cdecl 6 args pushed
  1, 0, SP3, SP2, SP1, veh at 0x467DCF..0x467DE7; `add esp,0x18`). No null check here (callee receives null). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467E05 00AD COMMAND_SET_CAR_CRUISE_SPEED  (gr: unregistered, used: 0/175/9)
- Parameters: CollectParameters(2): SP0 = vehicle handle, SP1 = speed (f32).
- Behaviour:
  1. `v = veh(SP0)` (0x467E1B, no null check).
  2. `c = (uint8)__ftol2(SP1)` (0x467E20..0x467E28, truncation; only the low byte is kept), written to `[v+0x3D0]` (0x467E30).
  3. `lim = handling->m_transmissionData.m_MaxFlatVelocity ([[v+0x384]+0x88]) * [0x858B34]=60.0f`, stored as f32 temp
     (0x467E44..0x467E50).
  4. `fcom` of `(float)(int)c` (fild, 0x467E40) against `lim`; `test ah,5; jnp keep` (0x467E54..0x467E5D): keep `c` only when
     `c < lim` (ordered); otherwise (>=, or NaN) take `lim` (0x467E5F/0x467E61).
  5. `[v+0x3D0] = (uint8)__ftol2(result)` (0x467E65/0x467E6A). Return 0.
  So cruise = min((uint8)trunc(SP1), trunc(maxFlatVel*60)) in the low byte (a limit >= 256 wraps).
- gta-reversed: absent. Open questions: none.

### 0x467EC1 00AF COMMAND_SET_CAR_MISSION  (gr: unregistered, used: 0/18/2)
- Parameters: CollectParameters(2): SP0 = vehicle handle, SP1 = mission (low byte used, 0x467EDD).
- Behaviour: `v = veh(SP0)` (0x467ED6, no null check); `v->m_autoPilot.SetCarMission((u8)SP1)` (0x463490, ECX = v+0x390;
  keeps crash missions); `[v+0x3AC] = CTimer::m_snTimeInMilliseconds` (0x467EF1/0x467EFB); `v->SetEngineOn(true)`
  (0x467F01). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467F1B 00B0 COMMAND_IS_CAR_IN_AREA_2D  (gr: unregistered, used: 0/34/5)
- Parameters: CollectParameters(6): SP0 = vehicle handle, SP1..SP4 = x1, y1, x2, y2 (f32), SP5 = highlight flag.
- Behaviour: `v = veh(SP0)` (0x467F30, no null check); UpdateCompareFlag(`CPlaceable::IsWithinArea(v, SP1, SP2, SP3, SP4) != 0`)
  (0x54F200 thiscall 4 floats, 0x467F35..0x467F68). If `SP5 != 0` (0x467F6D): `CTheScripts::HighlightImportantArea(id,
  SP1, SP2, SP3, SP4, -100.0f)` with `id = (uint32)this + (uint32)this->m_IP` (0x467F98..0x467F9D) and the height pushed as
  immediate 0xC2C80000 (= -100.0f, 0x467F8B) (0x485E00, cdecl 6 args). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x467FBD 00B1 COMMAND_IS_CAR_IN_AREA_3D  (gr: unregistered, used: 0/11/0)
- Parameters: CollectParameters(8): SP0 = vehicle handle, SP1..SP6 = x1, y1, z1, x2, y2, z2 (f32), SP7 = highlight flag.
- Behaviour: `v = veh(SP0)` (no null check); UpdateCompareFlag(`IsWithinArea(v, SP1..SP6) != 0`) (0x54F2B0, thiscall 6 floats,
  0x467FD8..0x468019). If `SP7 != 0` (0x46801E): `h = (SP6 + SP3) * [0x858B8C]=0.5f` (fld SP6, fadd SP3, fmul, stored f32,
  0x468027..0x46804B); `HighlightImportantArea(id = this + m_IP, SP1, SP2, SP4, SP5, h)` (0x46805E). Return 0 (0x468066).
- gta-reversed: absent. Open questions: none.

### 0x4674E4 0098 COMMAND_GENERATE_RANDOM_FLOAT  (gr: unregistered, used: 0/0/0)
- Parameters: `p = GetPointerToScriptVariable(2)` (0x4674E8), no CollectParameters.
- Behaviour: calls `_rand` 0x821B1E four times (0x4674EF..0x4674FE); the first three results are discarded; the fourth `r`
  gives `*p = (float)r * [0x859D5C]=1.52587890625e-05f` (= 1/65536; fild, fmul, fstp f32, 0x467503..0x467513). Since
  rand() <= 0x7FFF the result is in [0, 0.5) (original oddity, not 0..1). Return 0.
- gta-reversed: absent. Open questions: none.
