# CRunningScript::ProcessCommands600To699 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Extra CRunningScript fields used here: +0x08 m_szName
(char[8]), +0xC6 m_UsesMissionCleanup, +0xC7 m_IsExternal, +0xC9 m_ExternalType (i8).
Pool helpers (inlined-call style, `__thiscall` on the pool, `ret 4`): 0x4048E0 = CPool<CVehicle>::GetAt(handle)
(`idx = handle >> 8` (sar); returns `objects + idx*0xA18` when `flags[idx] == (u8)handle`, else 0);
0x404910 CPedPool::GetAtRef; 0x465070 (jmp into .HOODLUM 0x1561010) = CPool<CObject>::GetRef(obj)
(`idx = (obj - objects) / 0x19C`; returns `(idx << 8) + flags[idx]`).

## 0x47F370 CRunningScript::ProcessCommands600To699  (catalogue: specified/REA, port level 15)
- Dispatcher: `__thiscall` (this = ECX, copied to ESI at 0x47F383; command at [esp+4]), `ret 4`. No SEH frame:
  `sub esp,0x2C; push ebx; push ebp; push esi; push edi` (0x47F374..0x47F382). `eax = command + 0xFFFFFD6D`
  (= command - 0x293, 0x47F37A); `cmp eax,0x26; ja 0x47F9AD` (0x47F37F/0x47F385); byte index table
  `movzx eax, byte [eax + 0x47FA04]` (0x47F38B), jump table `jmp [eax*4 + 0x47F9BC]` (0x47F392, 18 entries).
  Opcodes below 0x293 wrap to a huge unsigned value and also reach the default.
- Default 0x47F9AD: `pop edi; pop esi; pop ebp; or al,0xFF; pop ebx; add esp,0x2C; ret 4` -> 0xFF.
- Common epilogue (each case inlines it; shared copy at 0x47F9A1): `xor al,al; pop edi; pop esi; pop ebp; pop ebx;
  add esp,0x2C; ret 4` -> 0. No case of this switch returns 1.
- Opcodes (100 in range 600..699 = 0x258..0x2BB): 18 with a case; gta-reversed handler: 6 (0297, 0298, 02A0, 02A9,
  02AB, 02AC; not re-specified); specified below: 12 (0293, 0294, 0296, 0299, 029B, 02A3, 02A7, 02A8, 02AA, 02B9,
  02A4/02A5 shared). Default-reaching (82, all gr unimplemented, both return -1 without reading params -> no
  difference): 0258..0292, 0295, 029A, 029C..029F, 02A1..02A2, 02A6, 02AD..02B8, 02BA..02BB. No gr `handler`
  opcode reaches the default here.

### 0x47F684 02A3 COMMAND_SWITCH_WIDESCREEN  (gr: unregistered, used: 37/754/34)
- Parameters: `CollectParameters(1)` (0x47F688). ScriptParams[0] = int on/off.
- Behaviour: ECX = &TheCamera 0xB6F028 (0x47F694); if ScriptParams[0] != 0 (0x47F692 test, 0x47F699 je) call
  CCamera::SetWideScreenOn 0x50C140 (0x47F69B), else CCamera::SetWideScreenOff 0x50C150 (0x47F6AC). Return 0.
- gta-reversed: absent (unregistered). No open questions.

### 0x47F4F1 029B COMMAND_CREATE_OBJECT_NO_OFFSET  (gr: unregistered, used: 22/477/71)
- Parameters: `CollectParameters(4)` (0x47F4F5): [0] model (int), [1] x, [2] y, [3] z (f32). Output: 1 value
  (StoreParameters(1) at 0x47F608) = object handle.
- Behaviour:
  1. `model = P0`; if `model < 0` (0x47F4FF `jge`), `model = *(i32*)(0xA44B88 - model*0x1C)` (0x47F503..0x47F50D) =
     `CTheScripts::UsedObjectArray[-P0].nModelIndex` (array 0xA44B70, stride 0x1C, field +0x18).
  2. `mi = CModelInfo::ms_modelInfoPtrs[model]` (0x47F50F, table 0xA9B0C8); `mi->m_nAlpha (+0xC) = 0xFF` (0x47F519).
  3. `obj = CObject::Create(model, false)` 0x5A1F60 (cdecl, `push 0; push eax`, 0x47F516..0x47F51D). No null check.
  4. `obj->m_nObjectType (+0x13C) = 2` (OBJECT_MISSION) if `this->m_IsExternal (+0xC7) == 0` and
     `this->m_ExternalType (+0xC9) == -1` (0x47F52D..0x47F53A); otherwise `6` (OBJECT_MISSION2, 0x47F543).
  5. `x = P1, y = P2, z = P3`. `fcomp z, [0x859014] = -100.0`; `test ah,0x41; jp` (0x47F55F..0x47F574): when
     `z <= -100.0` (and not NaN) `z = CWorld::FindGroundZForCoord(x, y)` 0x569660 (cdecl, ST0 result stored f32 at
     0x47F581); NaN and `z > -100` keep z.
  6. `obj->SetPosn(x, y, z)` 0x420B80 (thiscall, 0x47F595); `obj->SetOrientation(0, 0, 0)` 0x439A80 (0x47F5A2);
     CEntity::UpdateRwMatrix 0x446F90 (0x47F5A9); CEntity::UpdateRwFrame 0x532B00 (0x47F5B0).
  7. If `mi->vtable[3]()` (+0xC, AsLodAtomicModelInfoPtr; 0x47F5B9) != 0: CEntity::SetupBigBuilding 0x533150 (0x47F5C2).
  8. `CTheScripts::ClearSpaceForMissionEntity(&CVector(x, y, z), obj)` 0x486B00 (cdecl, 0x47F5E5); `CWorld::Add(obj)`
     0x563220 (0x47F5EB); `ScriptParams[0] = ObjectPool->GetRef(obj)` (0x465070 on [0xB7449C], 0x47F5FA/0x47F603);
     StoreParameters(1) 0x464370 (0x47F608).
  9. If `this->m_UsesMissionCleanup (+0xC6)` (0x47F60D; else jump to epilogue 0x47F815):
     `CTheScripts::MissionCleanUp (0xA90850).AddEntityToList(ScriptParams[0], 3 /*object*/)` 0x4637E0 (0x47F629).
     Return 0.
- gta-reversed: absent. Note the z / -100 rule uses `<=` (equality also snaps to ground).

### 0x47F40D 0296 COMMAND_UNLOAD_SPECIAL_CHARACTER  (gr: unregistered, used: 0/177/0)
- Parameters: `CollectParameters(1)` (0x47F411): [0] special-character slot (1-based).
- Behaviour: `s = P0 - 1`, written back to ScriptParams[0] (0x47F41B/0x47F41D).
  `CTheScripts::ScriptResourceManager (0xA485A8).RemoveFromResourceManager(s + 0x122, 2, this)` 0x470510
  (thiscall; pushes `this`, 2, `s + 290` at 0x47F41C..0x47F429). If it returns false (AL == 0, 0x47F434/0x47F436)
  return 0; else `CStreaming::SetMissionDoesntRequireSpecialChar(s)` 0x40B490 (cdecl, 0x47F443). Return 0.
- gta-reversed: absent.

### 0x47F6BD 02A7 COMMAND_ADD_SPRITE_BLIP_FOR_CONTACT_POINT  (gr: unregistered, used: 36/32/0)
### 0x47F75D 02A8 COMMAND_ADD_SPRITE_BLIP_FOR_COORD  (gr: unregistered, used: 0/53/0)
Two near-identical cases (02A7 jumps into 02A8's tail at 0x47F7FB).
- Parameters: `CollectParameters(4)` (0x47F6C1 / 0x47F761): [0] x, [1] y, [2] z (f32), [3] sprite id (int). Then
  `CollectNextParameterWithoutIncreasingPC()` 0x464250 (0x47F703 / 0x47F7A6) reads the CURRENT value of the output
  variable without advancing m_IP. Output: StoreParameters(1) (0x47F810) = blip handle.
- Behaviour:
  1. `if z <= -100.0` ([0x859014] = -100.0, `test ah,0x41; jp`, 0x47F6D6..0x47F6EF / 0x47F775..0x47F78E; NaN keeps z):
     `z = CWorld::FindGroundZForCoord(x, y)` (0x47F6F5 / 0x47F798).
  2. `CRadar::GetActualBlipArrayIndex(<output variable's old value>)` 0x582870 (0x47F709 / 0x47F7AC); the result in
     EAX is overwritten at 0x47F712 / 0x47F7B5 and never used (vestigial, no side effect beyond the call).
  3. `h = CRadar::SetCoordBlip(type, CVector(x, y, z), colour, 3 /*BLIP_DISPLAY_BOTH*/, this->m_szName (this+8))`
     0x583820 (cdecl, 7 dwords): 02A7 uses type 5 BLIP_CONTACT_POINT, colour 2 (pushes at 0x47F72F `push 2`,
     0x47F747 `push 5`); 02A8 uses type 4 BLIP_COORD, colour 5 (0x47F7D2 `push 5`, 0x47F7EA `push 4`).
  4. `CRadar::SetBlipSprite(h, P3)` 0x583D70 (0x47F7FE; P3 pushed at 0x47F757 / 0x47F7FA); `add esp,0x24` (0x47F803)
     cleans both calls. `ScriptParams[0] = h`; StoreParameters(1). Return 0.
- gta-reversed: absent. Open question: none (the discarded GetActualBlipArrayIndex is a plain call; port it as a call
  or omit it only if proven pure: 0x582870 is not re-checked here, keep the call).

### 0x47F85E 02AA COMMAND_SET_CAR_ONLY_DAMAGED_BY_PLAYER  (gr: unregistered, used: 0/121/2)
- Parameters: `CollectParameters(2)` (0x47F862): [0] vehicle handle, [1] int flag.
- Behaviour: `veh = CPool<CVehicle>::GetAt(P0)` 0x4048E0 on [0xB74494] (0x47F873), then jumps to the shared tail of
  the 02A9 (char) handler at 0x47F83C: `f = veh->m_nPhysicalFlags (+0x40)`; if P1 != 0 `f |= 0x400000` (0x47F849),
  else `f &= 0xFFBFFFFF` (0x47F87A); store at +0x40. Bit 22 = gr `bInvulnerable` (Physical.h). Return 0.
- Edge: no null check; an invalid handle dereferences null+0x40 (original crash). gta-reversed: absent.

### 0x47F4B6 0299 COMMAND_ACTIVATE_GARAGE  (gr: unregistered, used: 1/26/0)
### 0x47F976 02B9 COMMAND_DEACTIVATE_GARAGE  (gr: unregistered, used: 0/27/0)
- Parameters: `ReadTextLabelFromScript(buf = esp+0x18, 8)` 0x463D50 (0x47F4BF / 0x47F97F).
- Behaviour: `n = (i32)(i16)CGarages::GetGarageNumberByName(buf)` 0x447680 (cdecl, `movsx eax,ax` 0x47F4CE / 0x47F98E);
  if `n < 0` (0x47F4D6 / 0x47F996 `jl`) return 0; else `CGarages::ActivateGarage(n)` 0x447CD0 (0x47F4DD) resp.
  `CGarages::DeActivateGarage(n)` 0x447CB0 (0x47F999, then falls through into 0x47F9A1). Return 0.
- gta-reversed: absent.

### 0x47F3B8 0294 COMMAND_SET_CAN_RESPRAY_CAR  (gr: unregistered, used: 0/6/0)
- Parameters: `CollectParameters(2)` (0x47F3BC): [0] vehicle handle, [1] int.
- Behaviour: `veh = CPool<CVehicle>::GetAt(P0)` (0x47F3CE); `b = veh->autoFlags (+0x868, CAutomobile)`; if P1 != 0
  `b &= 0xFD` (0x47F3E3) else `b |= 2` (0x47F3F8); store byte at +0x868. Bit 1 = gr `bShouldNotChangeColour`. Return 0.
- Edge: no null check and no vehicle-type check (+0x868 is written for any vehicle class). gta-reversed: absent.

### 0x47F399 0293 COMMAND_GET_CONTROLLER_MODE  (gr: unregistered, used: 0/12/0)
- Behaviour: `ScriptParams[0] = 0` (0x47F39D); `StoreParameters(1)` (0x47F3A7); return 0. Always reports mode 0.

### 0x47F9A1 02A4 COMMAND_ADD_SPRITE_BLIP_FOR_CAR, 02A5 COMMAND_ADD_SPRITE_BLIP_FOR_CHAR  (gr: unimplemented, used: 0/0/0)
- Behaviour: the jump table sends both to the shared "return 0" epilogue 0x47F9A1 (`xor al,al ... ret 4`): no
  parameter is read, AL = 0.
- gta-reversed: `unimplemented` (ObjectCommands.cpp:288/289) returns OR_INTERRUPT (-1). DEVIATION (return value 0 vs
  -1; no script uses these opcodes). Port keeps the original (return 0, read nothing).
