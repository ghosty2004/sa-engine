# CRunningScript::ProcessCommands300To399 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Pool helper 0x4048E0 = CPool<CVehicle>::GetAt(handle) on
[0xB74494] (`__thiscall`, `ret 4`, null for a stale handle). "-100 ground rule": `fcomp z,[0x859014] = -100.0;
fnstsw; test ah,0x41; jp skip` -> `z = CWorld::FindGroundZForCoord(x, y)` 0x569660 when `z <= -100.0` (NaN and
`z > -100` keep z). Blip creators call `CRadar::GetActualBlipArrayIndex(CollectNextParameterWithoutIncreasingPC())`
(0x582870, 0x464250) first and discard the result (reads the output variable's old value; m_IP not advanced).

## 0x47C100 CRunningScript::ProcessCommands300To399  (catalogue: specified/REA, port level 13)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x47C111; command at [esp+4]), `ret 4`. No SEH frame:
  `sub esp,0x7C; push esi; push edi` (0x47C104..0x47C110). `eax = command + 0xFFFFFEC9` (= command - 0x137,
  0x47C108); `cmp eax,0x58; ja 0x47D0EA` (0x47C10D/0x47C113); byte index table `movzx eax, byte [eax + 0x47D1B4]`
  (0x47C119); jump table `jmp [eax*4 + 0x47D0F4]` (0x47C120, 48 entries). Opcodes 0x12C..0x136 (below base) wrap
  and reach the default.
- Default 0x47D0EA: `pop edi; or al,0xFF; pop esi; add esp,0x7C; ret 4` -> 0xFF.
- Return-0 epilogue (inlined per case; shared copy 0x47D0E0): `xor al,al; pop edi; pop esi; add esp,0x7C; ret 4`.
  Local bool for UpdateCompareFlag 0x4859D0 lives at [esp+0xC]. No case returns 1.
- Opcodes (100 in range 0x12C..0x18F): 50 with a case; gta-reversed handler: 17 (014C, 0154, 0158, 0159, 015A, 015B,
  015F, 0160, 016A, 016F, 0172, 0173, 0176, 0177, 0179, 017B, 0184), nop: 2 (018D, 018E) - not re-specified.
  Specified below: 31 (27 unregistered + 4 unimplemented sharing 0x47D0E0).
  Default-reaching (50): 012C..0136, 0138..014A, 014D, 0150, 0152..0153, 0155..0157, 015C, 015E, 0163, 0170..0171,
  0178, 017A, 017C..017F, 0182..0183 (0178 gr `unsupported` -> NotImplemented, others gr unimplemented; all -1 with
  no params read in both). No gr `handler` opcode reaches the default.

### 0x47C660 0164 COMMAND_REMOVE_BLIP  (gr: unregistered, used: 78/3185/34)
- `CollectParameters(1)` (0x47C664); `CRadar::ClearBlip(P0)` 0x587CE0 (cdecl, 0x47C670). Return 0.

### 0x47C831 016B COMMAND_GET_FADING_STATUS  (gr: unregistered, used: 21/1078/58)
- No params. `flag = TheCamera (0xB6F028).GetFading()` 0x50ADE0 `!= 0` (0x47C836..0x47C83D);
  `UpdateCompareFlag(flag)` (0x47C84B). Return 0.

### 0x47CC64 0175 COMMAND_SET_CAR_HEADING  (gr: unregistered, used: 3/997/25)
- Parameters: `CollectParameters(2)` (0x47CC68): [0] vehicle handle, [1] heading in degrees (f32).
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x47CC79, no null check). x87, all in ST0 (no intermediate f32 store):
  `a = P1`; `fcom [0x858B50] = 0.0; test ah,5; jp` (0x47CC84..0x47CC91): if `a < 0` then `a += [0x859E2C] = 360.0`;
  `fcom 360.0; test ah,0x41; jne` (0x47CC99..0x47CCA4): if `a > 360` then `a -= 360.0`; `a *= [0x8595EC] = 0.017453292`
  (0x47CCAC); stored f32 as the argument (0x47CCB5). `veh->SetHeading(a)` 0x43E0C0 (0x47CCB8); `CEntity::UpdateRwMatrix`
  0x446F90 (0x47CCBF). Return 0. NaN skips both corrections. Exactly 0 and 360 are kept.

### 0x47CF4E 018A COMMAND_ADD_BLIP_FOR_COORD  (gr: unregistered, used: 9/648/9)
- Parameters: `CollectParameters(3)` (0x47CF52): [0] x, [1] y, [2] z; output StoreParameters(1) (0x47CFFC).
- Behaviour: -100 ground rule (0x47CF66..0x47CF89). Discarded GetActualBlipArrayIndex (0x47CF97/0x47CF9D).
  `h = CRadar::SetCoordBlip(4 /*BLIP_COORD*/, CVector(x,y,z), 5 /*colour*/, 3 /*BLIP_DISPLAY_BOTH*/, this->m_szName
  (this+8))` 0x583820 (pushes 0x47CFB8..0x47CFDB); `CRadar::ChangeBlipScale(h, 3)` 0x583CC0 (0x47CFEA); `add esp,0x24`;
  `ScriptParams[0] = h` (0x47CFF6); StoreParameters(1). Return 0.

### 0x47CEBF 0186 COMMAND_ADD_BLIP_FOR_CAR, 0x47CEFD 0187 COMMAND_ADD_BLIP_FOR_CHAR, 0x47CF25 0188 COMMAND_ADD_BLIP_FOR_OBJECT  (gr: unregistered, used: 0/366/5, 0/626/3, 0/239/1)
- Parameters: `CollectParameters(1)` (0x47CEC3 / 0x47CF01 / 0x47CF29): [0] entity handle; discarded
  GetActualBlipArrayIndex (0x47CECA..0x47CED0 / 0x47CF08..0x47CF0E / 0x47CF30..0x47CF36); output StoreParameters(1).
- Behaviour: `h = CRadar::SetEntityBlip(type, P0, colour, 3 /*BOTH*/, this->m_szName)` 0x5839A0 (call site 0x47CEE6,
  shared): 0186 type 1 (CAR), colour 0 (0x47CEE1/0x47CEE4); 0187 type 2 (CHAR), colour 1 (0x47CF1E/0x47CF21);
  0188 type 3 (OBJECT), colour 6 (0x47CF47/0x47CF4A). Then `CRadar::ChangeBlipScale(h, 3)` (0x47CEF0), `add esp,0x20`,
  jump to 0x47CFF2: `ScriptParams[0] = h`; StoreParameters(1). Return 0.

### 0x47D00B 018B COMMAND_CHANGE_BLIP_DISPLAY  (gr: unregistered, used: 37/522/1)
- `CollectParameters(2)` (0x47D00F); `CRadar::ChangeBlipDisplay(P0, P1)` 0x583D20 (cdecl, 0x47D022). Return 0.

### 0x47C78C 0169 COMMAND_SET_FADING_COLOUR  (gr: unregistered, used: 8/244/5)
- `CollectParameters(3)` (0x47C790); `TheCamera.SetFadeColour((u8)P0, (u8)P1, (u8)P2)` 0x50BF00 (low bytes,
  zero-extended, 0x47C795..0x47C7B4). Return 0.

### 0x47C169 014B COMMAND_CREATE_CAR_GENERATOR  (gr: unregistered, used: 4/195/0)
- Parameters: `CollectParameters(12)` (0x47C16D): [0] x, [1] y, [2] z, [3] heading (f32), [4] model, [5] colour1,
  [6] colour2, [7] forceSpawn, [8] alarm chance, [9] door-lock chance, [10] min delay, [11] max delay. Output 1.
- Behaviour: `fld z; fcomp [0x859014] = -100.0; test ah,0x41; jne` (0x47C187..0x47C19E): only when `z > -100.0`
  (not equal, not NaN), `z = z + [0x859F00] = 0.015` (x87, stored f32 at 0x47C1AA). No ground lookup here.
  `h = CTheCarGenerators::CreateCarGenerator(CVector(x,y,z), P3, P4, (u16)P5, (u16)P6, (u8)P7, (u8)P8, (u8)P9,
  (u16)P10, (u16)P11, 0 /*iplId*/, 1 /*ignorePopulationLimit*/)` 0x6F31A0 (14 dwords, `add esp,0x38` at 0x47C214;
  the narrow values are loaded zero-extended with `xor reg; mov reg16/8`). `ScriptParams[0] = h`; StoreParameters(1). Return 0.

### 0x47C2F1 0151 COMMAND_CLEAR_ONSCREEN_COUNTER, 0x47C2D2 014F COMMAND_CLEAR_ONSCREEN_TIMER  (gr: unregistered, used: 4/178/35, 1/129/3)
- Parameters: `GetIndexOfGlobalVariable()` 0x464700 (0x47C2F3 / 0x47C2D4; returns the u16 global-variable offset).
- Behaviour: `CUserDisplay::OnscnTimer (0xBA1788).ClearCounter((u16)idx)` 0x44CE80 (0x47C301) resp.
  `.ClearClock((u16)idx)` 0x44CE60 (0x47C2E2). Return 0.

### 0x47CBE1 0174 COMMAND_GET_CAR_HEADING  (gr: unregistered, used: 2/79/3)
- Parameters: `CollectParameters(1)` (0x47CBE5); output StoreParameters(1) (0x47CC55).
- Behaviour: `veh = VehiclePool.GetAt(P0)` (no null check); `h = veh->GetHeading()` 0x441DB0 (ST0) `* [0x859878] = 57.295776`
  (0x47CC03), stored f32 at 0x47CC09. `fcomp [0x858B50] = 0.0; test ah,5; jp`: if `h < 0` (0x47CC1A) `h = h + 360.0`
  ([0x859E2C], stored f32). Reload; `fcomp 360.0; test ah,0x41; jne`: if `h > 360` `h = h - 360.0` (stored f32,
  0x47CC3D). `ScriptParams[0] = h`. Return 0. Note each step round-trips through an f32 local (unlike 0175).

### 0x47C764 0168 COMMAND_CHANGE_BLIP_SCALE  (gr: unregistered, used: 0/266/0)
- `CollectParameters(2)`; `CRadar::ChangeBlipScale(P0, P1)` 0x583CC0 (0x47C77A). Return 0.

### 0x47C682 0165 COMMAND_CHANGE_BLIP_COLOUR  (gr: unregistered, used: 0/111/2)
- `CollectParameters(2)`; `CRadar::ChangeBlipColour(P0, P1)` 0x583AB0 (0x47C698). Return 0.

### 0x47D034 018C COMMAND_ADD_ONE_OFF_SOUND  (gr: unregistered, used: 0/67/4)
- `CollectParameters(4)` (0x47D038): [0] x, [1] y, [2] z (f32), [3] sound event id. Local `CVector v(P0, P1, P2)`
  (at esp+0x30); `AudioEngine (0xB6BC90).ReportMissionAudioEvent((u16)P3, &v)` 0x507340 (CVector& overload, 0x47D06E).
  Return 0.

### 0x47C518 015D COMMAND_SET_TIME_SCALE  (gr: unregistered, used: 0/45/0)
- `CollectParameters(1)`; `CTimer::ms_fTimeScale [0xB7CB64] = P0` (raw 32-bit copy, 0x47C527). Return 0.

### 0x47CE71 0185 COMMAND_IS_CAR_HEALTH_GREATER  (gr: unregistered, used: 0/31/0)
- `CollectParameters(2)` (0x47CE75): [0] vehicle, [1] int health. `veh = VehiclePool.GetAt(P0)` (no null check).
  `flag = 1`; `fild P1; fcomp veh->m_fHealth (+0x4C0); test ah,5; jnp` (0x47CE8C..0x47CEA2): flag stays 1 only when
  `(float)P1 < health`; equal, greater or NaN -> 0. `UpdateCompareFlag(flag)`. Return 0.

### 0x47C95E 016E COMMAND_OVERRIDE_NEXT_RESTART  (gr: unregistered, used: 0/4/0)
### 0x47C85A 016C COMMAND_ADD_HOSPITAL_RESTART  (gr: unregistered, used: 0/8/0)
### 0x47C8DC 016D COMMAND_ADD_POLICE_RESTART  (gr: unregistered, used: 0/7/0)
- Parameters: 016E `CollectParameters(4)` (0x47C962): [0] x, [1] y, [2] z, [3] heading; 016C/016D `CollectParameters(5)`
  (0x47C85E / 0x47C8E0): same plus [4] town id (int).
- Behaviour: `fld z; fcom -100.0` (0x47C969 / 0x47C8EB / 0x47C96D); if `z <= -100.0` (jp not taken): `fstp st(0)`, then
  `CWorld::FindGroundZForCoord(x, y)` leaves the new z in ST0 (0x47C8A3 / 0x47C925 / 0x47C9A1). ST0 stored f32 as the
  vector's z (0x47C8AF / 0x47C931 / 0x47C9AD). Calls (cdecl, `const CVector&` to a local (x, y, z)):
  016C `CRestart::AddHospitalRestartPoint(v, P3, P4)` 0x460730 (0x47C8CA); 016D `CRestart::AddPoliceRestartPoint(v, P3, P4)`
  0x460780 (0x47C94C); 016E `CRestart::OverrideNextRestart(v, P3)` 0x4607D0 (0x47C9CD). ESI (`this`) is overwritten with P4
  in 016C/016D (0x47C880 / 0x47C902) but not used afterwards. Return 0.

### 0x47C127 0137 COMMAND_IS_CAR_MODEL  (gr: unregistered, used: 6/7/0)
- `CollectParameters(2)`; `veh = VehiclePool.GetAt(P0)` (no null check); `flag = ((i32)(i16)veh->m_nModelIndex (+0x22) == P1)`
  (0x47C142..0x47C14C); UpdateCompareFlag(flag). Return 0.

### 0x47C284 014E COMMAND_DISPLAY_ONSCREEN_TIMER  (gr: unregistered, used: 0/7/0)
- Parameters: `idx = GetIndexOfGlobalVariable()` (0x47C286), then `CollectParameters(1)` (0x47C291): [0] direction.
- Behaviour: `OnscnTimer (0xBA1788).AddClock((u16)idx, nullptr, P0 != 0 ? 1 : 0)` 0x44CD50 (0x47C2AC / 0x47C2C3). Return 0.

### 0x47CE00 0180 COMMAND_DECLARE_MISSION_FLAG  (gr: unregistered, used: 1/0/0)
- Parameters: no CollectParameters; reads the operand by hand: `m_IP += 1` (skips the type byte, assumed to be a global
  variable; 0x47CE00..0x47CE04), `v = *(u16*)m_IP`, `m_IP += 2` (0x47CE07..0x47CE10).
- Behaviour: `CTheScripts::OnAMissionFlag [0xA476AC] = (u32)v` (byte offset of the global, 0x47CE13). Return 0.
- Edge: the type byte is not checked (a non-global operand would be misparsed).

### 0x47C610 0161 COMMAND_ADD_BLIP_FOR_CAR_OLD  (gr: unregistered, used: 0/2/0)
- `CollectParameters(3)` (0x47C614): [0] vehicle handle, [1] colour, [2] display; discarded GetActualBlipArrayIndex
  (0x47C61B/0x47C621). `h = CRadar::SetEntityBlip(1 /*CAR*/, P0, P1, P2, this->m_szName)` 0x5839A0 (0x47C640); no scale call;
  `ScriptParams[0] = h`; StoreParameters(1). Return 0.

### 0x47C6AA 0167 COMMAND_ADD_BLIP_FOR_COORD_OLD  (gr: unregistered, used: 0/1/0)
- `CollectParameters(5)` (0x47C6AE): [0] x, [1] y, [2] z, [3] colour, [4] display. -100 ground rule (0x47C6C2..0x47C6E3).
  Discarded GetActualBlipArrayIndex (0x47C6F7). `h = CRadar::SetCoordBlip(4, CVector(x,y,z), P3, P4, this->m_szName)`
  (0x47C744); no scale call; `ScriptParams[0] = h`; StoreParameters(1). Return 0.

### 0x47D0AC 018F COMMAND_IS_CAR_STUCK_ON_ROOF  (gr: unregistered, used: 0/2/0)
- `CollectParameters(1)`; `flag = CTheScripts::UpsideDownCars (0xA4892C).HasCarBeenUpsideDownForAWhile(P0)` 0x463940
  `!= 0` (0x47D0C6..0x47D0CF); UpdateCompareFlag(flag); falls into 0x47D0E0 (return 0).

### 0x47D0E0 shared "return 0": 0162 ADD_BLIP_FOR_CHAR_OLD, 0166 DIM_BLIP, 0181 DECLARE_MISSION_FLAG_FOR_CONTACT, 0189 ADD_BLIP_FOR_CONTACT_POINT  (gr: unimplemented, used: 0/0/0 each)
- Behaviour: no parameter read; `xor al,al` + epilogue -> 0.
- gta-reversed: NotImplemented returns -1 (CharacterCommands.cpp:4326, ScriptCommands.cpp:143/141/105). DEVIATION in the
  return value only; unused by scripts. Port: return 0.

Open questions: none.
