# CRunningScript::ProcessCommands1000To1099 (SCRIPT) - opcode specs

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
- Memory stores are written `[0xADDR] <- value`. SEH: every switch has a frame; a case that constructs a task sets the state slot to 0 around the ctor and back to -1.
- Vtable slots (index = offset/4): CEntity 0 dtor, 1 Add(rect), 2 Add, 3 Remove, 4 SetIsStatic, 5 SetModelIndex, 6
  SetModelIndexNoCreate, 7 CreateRwObject, 8 DeleteRwObject, 9 GetBoundRect, 10 ProcessControl, 11 ProcessCollision, 12
  ProcessShift, 13 TestCollision, 14 Teleport, 15 SpecialEntityPreCollisionStuff, 16 SpecialEntityCalcCollisionSteps, 17 PreRender,
  18 Render, 19 SetupLighting, 20 RemoveLighting, 21 FlagToDestroyWhenNextProcessed, 22 ProcessEntityCollision (CPhysical);
  CVehicle slot 50 (+0xC8) = Fix.

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- CVehicle fields used here: +0x14 m_matrix, +0x22 m_nModelIndex (i16), +0x3DB m_autoPilot.m_nCarCtrlFlags (bit 0x04
  bAvoidLevelTransitions), +0x42A vehicle-flags byte 2 (bit 0x20, gr bCanBeDamaged), +0x434/+0x435 m_nPrimaryColor /
  m_nSecondaryColor (u8), +0x460 m_pDriver, +0x464 m_apPassengers[8], +0x488 m_nMaxPassengers (u8), +0x590 m_nVehicleType
  (0 = VEHICLE_TYPE_AUTOMOBILE), CAutomobile +0x868 autoFlags (bit 0x08 bNotDamagedUpsideDown).

## 0x489500 CRunningScript::ProcessCommands1000To1099  (catalogue: specified/REA, port level 10)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x489528), command id at [esp+4], `ret 4`. SEH frame `push -1; push 0x838478;
  push fs:[0]`, `fs:[0] <- esp` (0x489512), `sub esp,0x74`, push EBX/EBP/ESI/EDI. Range: `eax = id - 0x3E8` (0x48951F),
  `cmp eax,0x63; ja 0x48A1E6`; `movzx eax, byte [eax + 0x48A2B4]` (100-byte index table), `jmp [eax*4 + 0x48A204]` (44 dwords;
  index 0x2B = default 0x48A1E6, index 0x00 = 0x48A1E2 empty case).
- Common tail: cases end `xor al,al; jmp 0x48A1E8`; default 0x48A1E6 `or al,0xFF` (AL = -1). 0x48A1E8: `nop; nop; jmp 0x40133D`
  -> .text stub `mov ecx,[esp+0x84]; jmp 0x48A1EF` (saved SEH link) -> `pop edi/esi/ebp; fs:[0] <- ecx; pop ebx; add esp,0x80; ret 4`.
  Compare cases use the scratch byte `[esp+0x10]` and the shared tail 0x48A00B (`scratch = 1`) / 0x48A010 (`Cmp(scratch)`),
  or 0x48A0EA / 0x48A0EF (same pattern).
- Opcodes with a case: 51. gta-reversed handler: 15, nop: 8 (not re-specified). Specified below: 27 unregistered (03ED 03F3 03F4
  03F5 03FD 0400 0407 040C 040D 0414 0418 041D 041E 0423 0424 0425 0428 042B 042C 042D 042E 0431 0434 0435 0436 043C 0441) + 1
  unimplemented with a case (03EA).
- Empty case 0x48A1E2 (`xor al,al`, AL = 0, no parameters): 03E8, 03EA, 0409, 040B, 0416, 042F, 043E, 043F, 0440 (all gr nop
  except 03EA).
- Default-reaching (49, all gr `unimplemented` = -1, same as the exe): 03E9 03EC 03F1 03F2 03F6-03FC 03FF 0401-0406 0408 040A
  040E-0413 0415 0419 041B 041C 041F-0422 0426 0427 0429 042A 0437-043B 043D 0442 0443 0444 0447 044A.
- gta-reversed deviation: 03EA SET_GENERATE_CARS_AROUND_CAMERA is gr `unimplemented` (CameraCommands.cpp:326, -1) but the exe runs
  the empty case (AL = 0).

### 0x4898CB 040D COMMAND_CLEAR_MISSION_AUDIO  (gr: unregistered, used: 10 / 1392 / 36)
- `Collect(1)`; `CAudioEngine::ClearMissionAudio(&AudioEngine 0xB6BC90, (u8)SP0 - 1)` 0x5072F0 (slot = low byte minus 1, 8-bit
  wrap: `dec dl` at 0x4898DC, pushed zero-extended).

### 0x489831 0407 COMMAND_GET_OFFSET_FROM_CAR_IN_WORLD_COORDS / 0x48979F 0400 COMMAND_GET_OFFSET_FROM_OBJECT_IN_WORLD_COORDS  (gr: unregistered, used: 0407 0/289/2, 0400 9/88/118)
- `Collect(4)`: SP0 handle, SP1..SP3 offset x,y,z (f32). `e = VEH(SP0)` (0407) / `OBJ(SP0)` (0400), unchecked.
- `r = Multiply3x3(e->m_matrix (+0x14), &offset)` 0x59C790 (cdecl, hidden result pointer first; `add esp,0xC`; 0400 jumps into
  0407's call at 0x489885). The matrix pointer is passed WITHOUT a null check (crash if m_matrix is null).
- `p = e->m_matrix ? &m_matrix->pos (+0x30) : &e->placement.pos (+4)` (0x48989C-0x4898AF / 0x4897F7);
  `r += *p` via CVector::operator+= 0x411A00 (each component `p.c + r.c`, x87 from f32, stored f32).
- `SP0..SP2 <- r.x, r.y, r.z`; `Store(3)`.

### 0x4896E8 03F5 COMMAND_SET_CAR_CAN_BE_DAMAGED  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked). If `SP1 != 0`: `v[+0x42A] |= 0x20`. Else: `CVehicle::ExtinguishCarFire(v)` 0x6D2460,
  then `v[+0x42A] &= 0xDF`.

### 0x489683 03F4 COMMAND_SET_ALL_CARS_CAN_BE_DAMAGED  (gr: unregistered, used: 0 / 34 / 17)
- `Collect(1)`. If `SP0 != 0`: `CWorld::SetAllCarsCanBeDamaged(1)` 0x5668F0. Else: `CWorld::SetAllCarsCanBeDamaged(0)`;
  `p = *CWorld::FindPlayerCoors(&tmp, -1)` 0x56E010 (cdecl, hidden result); `CWorld::ExtinguishAllCarFiresInArea(p, 4000.0f)`
  0x566950 (imm 0x457A0000; cdecl CVector by value + float).

### 0x48954A 03ED COMMAND_SET_UPSIDEDOWN_CAR_NOT_DAMAGED  (gr: unregistered, used: 0 / 20 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked; assumed CAutomobile, no type check); `v[+0x868] |= 8` if `SP1 != 0`, else `&= 0xF7`.

### 0x489B74 041E COMMAND_SET_RADIO_CHANNEL  (gr: unregistered, used: 0 / 55 / 4)
- `Collect(1)`. If `SP0 == -1` -> AL = 0, nothing else (0x489B85). If `SP0 == 11` -> `SP0 <- 12` (written back to ScriptParams[0],
  0x489B95). `CAudioEngine::RetuneRadio(&AudioEngine, (i8)(SP0 + 1))` 0x507E10 (`inc al`, low byte).

### 0x489B53 041D COMMAND_SET_NEAR_CLIP  (gr: unregistered, used: 0 / 46 / 1)
- `Collect(1)`; `CCamera::SetNearClipScript(&TheCamera 0xB6F028, SP0 as f32)` 0x50BF90.

### 0x489C92 042B COMMAND_CLEAR_AREA_OF_CHARS  (gr: unregistered, used: 0 / 9 / 0)
- `Collect(6)`: x1 SP0, y1 SP1, z1 SP2, x2 SP3, y2 SP4, z2 SP5. Per axis (x, then y, then z): `if !(a <= b)` (fcomp + `test ah,0x41;
  jne skip`; NaN does not swap) swap so a <= b (0x489CB6-0x489D2A).
  `CWorld::ClearPedsFromArea(x1, y1, z1, x2, y2, z2)` 0x5667F0 (cdecl 6 floats; push order z2, y2, x2, z1, y1, x1).

### 0x489C1A 0425 COMMAND_CONVERT_METRES_TO_FEET  (gr: unregistered, used: 4 / 0 / 0)
- `Collect(1)`; `SP0 <- SP0 * [0x859F60]` (3.3333333f = 0x40555555; f32 in, f32 out); `Store(1)`. (Factor 3.333 not 3.2808 -
  original.)

### 0x489D82 042D COMMAND_CONVERT_METRES_TO_FEET_INT  (gr: unregistered, used: 2 / 0 / 0)
- `Collect(1)`; `SP0 <- ftol((float)(i32)SP0 * [0x859F60])` (fild, fmul 3.3333333f, _ftol truncation; 0x489D8F-0x489D9B); `Store(1)`.

### 0x489BF3 0424 COMMAND_ARE_MEASUREMENTS_IN_METRES  (gr: unregistered, used: 5 / 1 / 0)
- `Cmp(CLocalisation::Metric() != 0)` 0x56D220 (two copies: `push 1` / `push 0`).

### 0x489E73 0431 COMMAND_IS_CAR_PASSENGER_SEAT_FREE  (gr: unregistered, used: 1 / 17 / 4)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); scratch = 0. If `SP1 < (i32)(u8) v->m_nMaxPassengers` (signed `jge` at 0x489EA2) and
  `v->m_apPassengers[SP1]` (`[v + 0x464 + SP1*4]`) == null -> scratch = 1 (0x48A00B). `Cmp(scratch)`.
- Edge: a negative SP1 passes the bound check and reads before the array (SP1 = -1 reads m_pDriver).

### 0x4898B4 040C COMMAND_IS_GERMAN_GAME  (gr: unregistered, used: 0 / 6 / 0)
- No parameters. scratch = 0; `Cmp(CLocalisation::GermanGame() != 0)` 0x56D200 via 0x48A0EA/0x48A0EF.

### 0x489FB3 0441 COMMAND_GET_CAR_MODEL  (gr: unregistered, used: 5 / 6 / 5)
- `Collect(1)`; `SP0 <- (i32)(i16) VEH(SP0)->m_nModelIndex (+0x22)` (movsx, unchecked); `Store(1)`.

### 0x48963E 03F3 COMMAND_GET_CAR_COLOURS  (gr: unregistered, used: 0 / 5 / 4)
- `Collect(1)`; `v = VEH(SP0)` (unchecked); `SP0 <- (u8) v[+0x434]`, `SP1 <- (u8) v[+0x435]`; `Store(2)`.

### 0x489F94 043C COMMAND_SET_MUSIC_DOES_FADE  (gr: unregistered, used: 0 / 20 / 1)
- `Collect(1)`; `[0xB6F04D] <- (SP0 == 0)` (TheCamera+0x25 = m_bIgnoreFadingStuffForMusic; note the inversion, `sete`).

### 0x489BAE 0423 COMMAND_SET_CAR_TRACTION  (gr: unregistered, used: 0 / 13 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); `t = SP1` (f32, fld/fstp = bit copy for normal values). If `v->m_nVehicleType (+0x590)
  == 0` (automobile): `v[+0x8A0] <- t`; else `v[+0x794] <- t` (0x489BD7). The else branch is meant for bikes (CBike field at +0x794)
  but any non-automobile type (monster truck, quad, heli, plane, boat, trailer...) also gets +0x794 written - original behaviour
  (for CAutomobile subclasses +0x794 lies inside m_wheelColPoint[2]).

### 0x489C48 0428 COMMAND_SET_CAR_AVOID_LEVEL_TRANSITIONS  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(2)`; `v[+0x3DB] |= 4` if `SP1 != 0` else `&= 0xFB` (v = VEH(SP0), unchecked).

### 0x489731 03FD COMMAND_SET_DRUNK_INPUT_DELAY  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(2)`; `CPad::SetDrunkInputDelay(CPad::GetPad(SP0) 0x53FB70, SP1)` 0x53F910 (thiscall).

### 0x4898F0 0414 COMMAND_SET_FREE_HEALTH_CARE  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(2)`; `CWorld::Players[SP0].m_bFreeHealthCare (+0x152) <- (SP1 != 0)` (0xB7CD98 + SP0*0x190).

### 0x489AAE 0418 COMMAND_SET_OBJECT_DRAW_LAST  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(2)`; `o = OBJ(SP0)` (unchecked); entity flags dword `+0x1C |= 0x4000` if `SP1 != 0` else `&= ~0x4000` (CEntity flag bit
  14, gr bDrawLast).

### 0x489D5B 042C COMMAND_SET_TOTAL_NUMBER_OF_MISSIONS  (gr: unregistered, used: 1 / 0 / 0)
- `Collect(1)`; `CStats::SetStatValue(0x94, (float) SP0)` 0x55A070 (stat 148, gr eStats name not checked; fild -> f32).

### 0x489DB4 042E COMMAND_REGISTER_FASTEST_TIME  (gr: unregistered, used: 0 / 9 / 0)
- `Collect(2)`; `CStats::RegisterFastestTime(SP0, SP1)` 0x55A0B0 (cdecl).

### 0x489F56 0434 COMMAND_START_CREDITS (used 0/1/0) - `CCredits::Start()` 0x7170E0.
### 0x489F62 0435 COMMAND_STOP_CREDITS (used 0/2/0) - `CCredits::Stop()` 0x717100.
### 0x489F6E 0436 COMMAND_ARE_CREDITS_FINISHED (used 0/1/0) - `Cmp([0xC6E97C] CCredits::bCreditsGoing == 0)`.
(All three gr unregistered, no parameters.)

### 0x48A1E2 03EA COMMAND_SET_GENERATE_CARS_AROUND_CAMERA  (gr: unimplemented, used: 0)
- Empty shared case (with the gr-nop opcodes 03E8 0409 040B 0416 042F 043E 043F 0440): AL = 0, no parameters read.

- gta-reversed: none of the unregistered opcodes has gr code. Open questions: 0423 the gr names of +0x8A0 (CAutomobile) and +0x794
  (CBike) were not validated against VALIDATE_OFFSET (offsets are exact).
