# CRunningScript::ProcessCommands400To499 (SCRIPT) - opcode specs

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
  CVehicle slot 50 (+0xC8) = Fix.

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).

## 0x47D210 CRunningScript::ProcessCommands400To499  (catalogue: specified/REA, port level 32)
- Dispatcher: `__thiscall` (this = ECX, copied to ESI at 0x47D22E; in this switch `this` is ESI, not EDI), command id at [esp+4],
  `ret 4`. SEH frame `push -1; push 0x838329; push fs:[0]` (0x47D210-0x47D21D), `fs:[0] <- esp` (0x47D222), `sub esp,0x58`,
  push EBX/ESI/EDI. EAX keeps the command id through the dispatch (`mov eax,[esp+0x10]` at 0x47D21E).
  Range check: `ecx = id - 0x190` (0x47D230), `cmp ecx,0x63; ja 0x47DF40` (0x47D236/0x47D23A); two-level table:
  `movzx ecx, byte [ecx + 0x47DFE0]` (100-byte index table, 0x47D240), `jmp [ecx*4 + 0x47DF58]` (34 dwords, 0x47D247).
  Index byte 0x21 maps to 0x47DF40 = default.
- No common tail: every case ends with its own inlined epilogue `mov ecx,[esp+0x64]; fs:[0] <- ecx; pop edi; pop esi; pop ebx;
  add esp,0x64; ret 4` with AL = 0 (`xor al,al` before it). Default 0x47DF40: same epilogue with `or al,0xFF` (AL = -1, 0x47DF46).
  The local `[esp+0x74]` (= the dword at the caller's argument slot, i.e. the command-id argument reused as a bool scratch) is used
  by the compare cases.
- Opcodes with a case: 46 (byte table entries != 0x21). gta-reversed handler: 17 (not re-specified). Specified below: 19 unregistered
  (0190 0191 01AB 01AC 01AD 01AE 01AF 01B0 01B5 01B6 01B7 01BD 01C0 01C1 01C3 01E9 01EA 01EB 01EC) + 10 unimplemented with a case
  (01A1-01AA).
- Default-reaching (54, all gr `unimplemented`, so gr also returns -1 = same as the exe): 0192-01A0, 01B1, 01B3, 01B8, 01BA, 01BE,
  01BF, 01C6, 01C9-01E2, 01E6, 01ED, 01EE, 01EF, 01F1, 01F2.
- gta-reversed deviation: 01A1-01AA are `unimplemented` in gr (CharacterCommands.cpp:4351-4360) but have a real case here (below);
  gr returns -1 without collecting parameters. Port follows the exe.

### 0x47D2E9 01AD COMMAND_LOCATE_CAR_2D, 01AE LOCATE_STOPPED_CAR_2D, 01AF LOCATE_CAR_3D, 01B0 LOCATE_STOPPED_CAR_3D  (gr: unregistered, used: 01AD 2/207/0, 01AE 0/4/0, 01AF 1/157/0, 01B0 0/18/1)
- `this->LocateCarCommand(commandId)` 0x487A20 (thiscall, EAX = command id pushed at 0x47D2E9; separate function with its own spec,
  it collects 6 (2D) or 8 (3D) parameters itself and sets the compare flag). AL = 0.

### 0x47D75A 01C3 COMMAND_MARK_CAR_AS_NO_LONGER_NEEDED  (gr: unregistered, used: 4 / 578 / 18)
- `Collect(1)`: SP0 vehicle handle. `CTheScripts::CleanUpThisVehicle(VEH(SP0))` 0x486670 (cdecl; pointer may be null, the callee
  is passed it unchecked). Then if `this->m_UsesMissionCleanup` (+0xC6, 0x47D77A) != 0:
  `MissionCleanUp.RemoveEntityFromList(SP0, 1 /*vehicle*/)` 0x4654B0 (push 1 at 0x47D787, handle re-read from SP0 at 0x47D7EC).

### 0x47D69E 01BD COMMAND_GET_GAME_TIMER  (gr: unregistered, used: 19 / 1048 / 50)
- No collect. `SP0 <- [0xB7CB84] CTimer::m_snTimeInMilliseconds`; jumps to the shared store tail 0x47DD97 (`SP0 <- eax; Store(1)`,
  inside 01EA). AL = 0.

### 0x47DDBB 01EB COMMAND_SET_CAR_DENSITY_MULTIPLIER  (gr: unregistered, used: 1 / 239 / 5)
- `Collect(1)`; `[0x8A5B20] CCarCtrl::CarDensityMultiplier <- SP0` (raw f32 bits, 0x47DDCA).

### 0x47D45C 01B6 COMMAND_FORCE_WEATHER_NOW  (gr: unregistered, used: 1 / 65 / 0)
- `Collect(1)`; `CWeather::ForceWeatherNow((u16)SP0)` 0x72A4F0 (cdecl, low 16 bits zero-extended at 0x47D467).

### 0x47D42B 01B5 COMMAND_FORCE_WEATHER  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(1)`; `CWeather::ForceWeather((u16)SP0)` 0x72A4E0 (cdecl, 0x47D436).

### 0x47D48D 01B7 COMMAND_RELEASE_WEATHER  (gr: unregistered, used: 1 / 54 / 0)
- No parameters. `CWeather::ReleaseWeather()` 0x72A510.

### 0x47D6CD 01C1 COMMAND_IS_CAR_STOPPED  (gr: unregistered, used: 1 / 69 / 8)
- `Collect(1)`; `b = CTheScripts::IsVehicleStopped(VEH(SP0))` 0x4861F0 (cdecl; null passed through unchecked);
  scratch byte `[esp+0x74] <- 0`, set to 1 if `b` (0x47D6E9-0x47D6FA); `Cmp(scratch)`.

### 0x47DDE6 01EC COMMAND_SET_CAR_HEAVY  (gr: unregistered, used: 0 / 49 / 0)
- `Collect(2)`: SP0 vehicle handle, SP1 flag. `v = VEH(SP0)`; if null -> AL = 0 (0x47DE03 -> 0x47DE84).
- If `SP1 != 0` (0x47DE0B): `v->m_nPhysicalFlags (+0x40) |= 1` (bMakeMassTwiceAsBig, 0x47DE12); `h = v->m_pHandlingData (+0x384)`;
  `v->m_fMass (+0x8C) = h->m_fMass (+4) * [0x858B3C] (3.0f)`; `v->m_fTurnMass (+0x90) = h->m_fTurnMass (+0xC) * [0x858C80] (5.0f)`;
  `v->m_fBuoyancyConstant (+0xA0) = h->m_fBuoyancyConstant (+0x24) + itself` (`fadd st0,st0`, i.e. x2) (0x47DE1E-0x47DE41).
  Each product is computed in x87 from f32 loads and stored f32.
- Else (0x47DE5D): flags `&= ~1`; copy (integer moves, bit-exact) `+0x8C <- h+4`, `+0x90 <- h+0xC`, `+0xA0 <- h+0x24`.

### 0x47D24E 0190 COMMAND_ADD_UPSIDEDOWN_CAR_CHECK  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(1)`; `CUpsideDownCarCheck::AddCarToCheck(&CTheScripts::UpsideDownCars 0xA4892C, SP0)` 0x4638D0 (thiscall).

### 0x47D27E 0191 COMMAND_REMOVE_UPSIDEDOWN_CAR_CHECK  (gr: unregistered, used: 0 / 0 / 0)
- `Collect(1)`; `CUpsideDownCarCheck::RemoveCarFromCheck(&UpsideDownCars 0xA4892C, SP0)` 0x463910.

### 0x47D2CB 01AB COMMAND_IS_CAR_STOPPED_IN_AREA_2D, 01AC IS_CAR_STOPPED_IN_AREA_3D  (gr: unregistered, used: 01AB 0/4/0, 01AC 0/8/0)
- `this->CarInAreaCheckCommand(commandId)` 0x488EC0 (thiscall, EAX = id pushed at 0x47D2CB; separate function: collects 6 / 8
  parameters, sets the compare flag). AL = 0.

### 0x47D2AD 01A1-01AA COMMAND_IS_CHAR_[STOPPED_]IN_AREA_[ON_FOOT_|IN_CAR_]2D/3D  (gr: unimplemented, used: 01A6 0/3/2, 01A7 0/1/0, others 0)
- Shared by 01A1, 01A2, 01A3, 01A4, 01A5, 01A6, 01A7, 01A8, 01A9, 01AA: `this->CharInAreaCheckCommand(commandId)` 0x488B50
  (thiscall, EAX = id pushed at 0x47D2AD; separate function: collects 6 (2D) or 8 (3D) parameters, sets the compare flag). AL = 0.
- gta-reversed: `unimplemented` (returns -1 without reading parameters) - deviation, see dispatcher.

### 0x47D6A8 01C0 COMMAND_STORE_WANTED_LEVEL  (gr: unregistered, used: 1 / 1 / 0)
- `Collect(1)`: SP0 player index. `w = CPlayerPed::GetWantedLevel(CWorld::Players[SP0].m_pPed)` 0x41BE60 (thiscall; ped read from
  0xB7CD98 + SP0*0x190, unchecked); jump to the shared tail 0x47DD97: `SP0 <- w; Store(1)`.

### 0x47DD2F 01E9 COMMAND_GET_NUMBER_OF_PASSENGERS  (gr: unregistered, used: 0 / 10 / 0)
- `Collect(1)`; `SP0 <- (u8) VEH(SP0)->m_nNumPassengers (+0x484)` (movzx, 0x47DD49; no null check - original crash on a bad
  handle); `Store(1)`.

### 0x47DD75 01EA COMMAND_GET_MAXIMUM_NUMBER_OF_PASSENGERS  (gr: unregistered, used: 0 / 14 / 2)
- `Collect(1)`; `SP0 <- (u8) VEH(SP0)->m_nMaxPassengers (+0x488)` (0x47DD90, no null check); `Store(1)` (0x47DD97 is the shared
  store tail used by 01BD and 01C0).

- gta-reversed: none of the unregistered opcodes above has gr code. Open questions: none.
