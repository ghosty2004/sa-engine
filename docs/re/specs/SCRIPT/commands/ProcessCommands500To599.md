# CRunningScript::ProcessCommands500To599 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` = `CPool<CVehicle>::GetAtRef(h)` 0x4048E0 (thiscall, ECX =
CPools::ms_pVehiclePool [0xB74494], stride 0xA18, null when the slot flag byte != h & 0xFF). `__ftol2` = 0x821B40
(truncation toward zero). "no null check" = the original dereferences the pool result directly (crash on a dead handle).
CVehicle fields: +0x14 m_matrix (up vector at matrix+0x20..+0x28), +0x434/+0x435 m_nPrimaryColor/m_nSecondaryColor,
+0x4C0 m_fHealth (f32), +0x4F8 m_nDoorLock.

## 0x47E090 CRunningScript::ProcessCommands500To599  (catalogue: specified/REA, port level 18)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x838353,
  0x47E090..0x47E09E), `sub esp,0x70`, push ebx/esi/edi; this in ESI (0x47E0AA). `eax = command - 0x1F4` (0x47E0B3),
  `cmp eax,0x62; ja 0x47F235` (0x47E0B9/0x47E0BD): 500..598 dispatched, 599 (0257) reaches default. Two-level table:
  `movzx eax, byte [eax+0x47F304]` (0x47E0C3), `jmp [eax*4+0x47F24C]` (0x47E0CA).
- Default 0x47F235: restore fs:[0] from [esp+0x7C], pop edi/esi, `or al,0xFF`, pop ebx, `add esp,0x7C; ret 4` -> -1.
  Cases return 0 through their own epilogue copies (`xor al,al`) or the shared one at 0x47F028.
- Opcodes: 52 with a case: 26 gta-reversed `handler`, 2 `nop` (0235 SET_GANG_PED_MODELS 0x47ECC2 = CollectParameters(3) only;
  0236 SET_GANG_CAR_MODEL 0x47ECE1 = CollectParameters(2) only; both match gr's nop), 2 `unimplemented` with a case
  (021C, 0231) and 22 `unregistered`, all specified below.
- Default-reaching (48, gr `unimplemented`, 023A `unsupported`): 01F8, 01FC..0201, 020E..0212, 0218, 0219, 021A, 021D..0220,
  0222, 0225, 0228, 022C..0230, 0232..0234, 0238, 0239, 023A, 023E..0240, 0242, 0243, 0246, 024A..024E, 0251, 0252, 0255, 0257.
  None of them has a gta-reversed handler.
- gta-reversed deviation: **0250 COMMAND_DRAW_LIGHT** is a gr handler (Commands/ObjectCommands.cpp:138/217: reads 6 params and
  calls CPointLights::AddLight) but in the exe it shares the empty case 0x47F21F with 021C/0231: it collects NO parameters
  and returns 0 (so a script using it would desynchronise; no shipped script uses it, script-usage has no row).

### 0x47F21F 021C COMMAND_IS_CAR_IN_MISSION_GARAGE, 0231 COMMAND_SET_SCRIPT_POLICE_HELI_TO_CHASE_CHAR, 0250 COMMAND_DRAW_LIGHT  (gr: unimplemented / unimplemented / handler, used: 0/0/0)
- Parameters: none read. Behaviour: `xor al,al` (0x47F21F), epilogue, return 0. The compare flag is not updated.
- gta-reversed: 021C/0231 `unimplemented` stubs (ScriptCommands.cpp:185/272); 0250 see dispatcher note.

### 0x47E57A 0213 COMMAND_CREATE_PICKUP  (gr: unregistered, used: 30/299/5)
- Parameters: CollectParameters(5) (0x47E57E): SP0 = model (negative = used-object index), SP1 = pickup type (low byte),
  SP2/SP3/SP4 = x/y/z. Output SP0 = pickup handle.
- Behaviour:
  1. `model = SP0`; if `model < 0` (0x47E58B): `model = *(int32*)(0xA44B88 + (-SP0)*0x1C)` (CTheScripts::UsedObjectArray
     [-SP0].nModelIndex; array base 0xA44B70, stride 0x1C, field +0x18; 0x47E58D..0x47E597).
  2. `z = SP4` (f32 [esp+0x18]); `fcomp [0x859014]=-100.0`, `test ah,0x41; jp` (0x47E5AE..0x47E5C4): if `z <= -100.0f`
     (NaN skips): `z = CWorld::FindGroundZForCoord(SP2, SP3) + [0x858B8C]=0.5f` (0x47E5CE/0x47E5D3, stored f32).
  3. `CPickups::GetActualPickupIndex(CollectNextParameterWithoutIncreasingPC())` (0x464250 then 0x4552A0, cdecl): result
     discarded (dead call, EAX overwritten at 0x47E5F8). CollectNextParameterWithoutIncreasingPC reads the operand that
     follows this command without moving m_IP; keep the call for side-effect parity.
  4. `SP0 = CPickups::GenerateNewOne(CVector(SP2, SP3, z), model, (uint8)SP1, 0, 0, false, nullptr)` (0x456F20, cdecl 9 dwords,
     0x47E603..0x47E636), StoreParameters(1) (0x47E647). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x47E6AB 0215 COMMAND_REMOVE_PICKUP  (gr: unregistered, used: 57/121/4)
- CollectParameters(1); `CPickups::RemovePickUp(SP0)` (0x4573D0, cdecl). Return 0.

### 0x47E662 0214 COMMAND_HAS_PICKUP_BEEN_COLLECTED  (gr: unregistered, used: 10/33/4)
- CollectParameters(1); UpdateCompareFlag(`CPickups::IsPickUpPickedUp(SP0) != 0`) (0x454B40, cdecl, 0x47E672..0x47E690). Return 0.

### 0x47E40A 020A COMMAND_LOCK_CAR_DOORS  (gr: unregistered, used: 1/303/17)
- CollectParameters(2): SP0 = vehicle handle, SP1 = lock state. `v = veh(SP0)`; if null return 0 (0x47E427 -> 0x47F028);
  else `[v+0x4F8] = SP1` (dword store, m_nDoorLock, 0x47E433). Return 0.

### 0x47E9E0 0224 COMMAND_SET_CAR_HEALTH  (gr: unregistered, used: 0/238/3)
- CollectParameters(2): SP0 = vehicle handle, SP1 = health (int). `[veh(SP0)+0x4C0] = (float)SP1` (fild, fstp f32 temp,
  0x47E9E9/0x47E9FC, stored 0x47EA0F). No null check. Return 0.

### 0x47EA74 0227 COMMAND_GET_CAR_HEALTH  (gr: unregistered, used: 1/85/4)
- CollectParameters(1). `SP0 = __ftol2([veh(SP0)+0x4C0])` (fld 0x47EA8F, shared tail 0x47EA4B), StoreParameters(1). No null
  check. Return 0.

### 0x47EA97 0229 COMMAND_CHANGE_CAR_COLOUR  (gr: unregistered, used: 1/176/9)
- CollectParameters(3): `v = veh(SP0)` (no null check); `[v+0x434] = (u8)SP1` (0x47EAB8); `[v+0x435] = (u8)SP2` (0x47EAC4).
  Return 0.

### 0x47E0D1 01F4 COMMAND_IS_CAR_UPSIDEDOWN  (gr: unregistered, used: 0/22/1)
- CollectParameters(1). `m = veh(SP0)->m_matrix` (0x47E0EC; neither the vehicle nor the matrix is null-checked).
  `upz = m->up.z` ([m+0x28], copied to [esp+0x40], 0x47E0FB). `fcomp [0x858C24]=0.3f`, `test ah,0x41; jnp` (0x47E106..0x47E11D):
  result = 1 when `upz < 0.3f` or `upz == 0.3f`, 0 when greater or NaN. UpdateCompareFlag(result) (0x47F019 / 0x47E135).
  Return 0.

### 0x47E500 020D COMMAND_IS_CAR_UPRIGHT  (gr: unregistered, used: 2/1/0)
- CollectParameters(1). `upz = veh(SP0)->m_matrix->up.z` (0x47E51A..0x47E528, no null checks). `fcomp [0x858B50]=0.0f`,
  `test ah,1; je` (0x47E530..0x47E54B): result = 0 only when `upz < 0.0f`; true for >= 0 and NaN. UpdateCompareFlag(result).
  Return 0.

### 0x47E44F 020B COMMAND_EXPLODE_CAR  (gr: unregistered, used: 0/81/0)
- CollectParameters(1). `v = veh(SP0)` (no null check); virtual call `v->vtable[0xA4/4 = 41](nullptr, false)`
  (CVehicle::BlowUpCar(CEntity* damager, bool bHideExplosion), 0x47E469..0x47E471). Return 0.

### 0x47E48D 020C COMMAND_ADD_EXPLOSION  (gr: unregistered, used: 0/99/0)
- CollectParameters(4): SP0..SP2 = x/y/z, SP3 = explosion type.
- `CExplosion::AddExplosion(nullptr, nullptr, SP3, CVector(SP0,SP1,SP2), 0 /*lifetime*/, 1 /*usesSound*/,
  -1.0f /*cameraShake, imm 0xBF800000*/, 0 /*isVisible*/)` (0x736A50, cdecl 10 dwords, 0x47E4A7..0x47E4E2). Return 0.

### 0x47E6D9 0216 COMMAND_SET_TAXI_LIGHTS  (gr: unregistered, used: 0/5/0)
- CollectParameters(2). `CAutomobile::SetTaxiLight(veh(SP0), SP1 != 0)` (0x6A3740 thiscall; two copies pushing 1
  (0x47E700) / 0 (0x47E71D)). No null or vehicle-type check. Return 0.

### 0x47E78F 021B COMMAND_SET_TARGET_CAR_FOR_MISSION_GARAGE  (gr: unregistered, used: 0/2/0)
- Parameters: ReadTextLabelFromScript(buf, 8) (0x47E798), then CollectParameters(1) (0x47E7B0): SP0 = vehicle handle or < 0.
- Behaviour: `g = CGarages::GetGarageNumberByName(buf)` (0x447680, cdecl; read BEFORE CollectParameters). If `(int16)g < 0`
  (0x47E7B5 `test di,di; jl`) return 0. If `SP0 < 0` -> `CGarages::SetTargetCarForMissionGarage(g, nullptr)` (0x447C40, cdecl,
  g pushed as full EDI dword); else `SetTargetCarForMissionGarage(g, veh(SP0))` (null possible). Return 0.

### 0x47EDA9 023C COMMAND_LOAD_SPECIAL_CHARACTER  (gr: unregistered, used: 0/149/0)
- Parameters: CollectParameters(1) (slot, 1-based), then ReadTextLabelFromScript(buf, 8) (0x47EDC8).
- Behaviour: `SP0 = SP0 - 1` (written back to ScriptParams, 0x47EDBE/0x47EDC2). Lower-case the 8 label bytes in place: for each
  byte, if `'A' (0x41) <= b <= 'Z' (0x5A)` then `b += 0x20` (unsigned compares, all 8 bytes regardless of the terminator,
  0x47EDD6..0x47EDE6). `CStreaming::RequestSpecialChar(SP0, buf, 0xC)` (0x40B450, cdecl; flags 0xC = KEEP_IN_MEMORY|MISSION_REQUIRED).
  `CTheScripts::ScriptResourceManager.AddToResourceManager(SP0 + 0x122, 2, this)` (0x4704B0, thiscall ECX = 0xA485A8;
  0x122 = 290 = first special-character model id). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x47EE2D 023D COMMAND_HAS_SPECIAL_CHARACTER_LOADED  (gr: unregistered, used: 0/114/0)
- CollectParameters(1). `SP0 = SP0 - 1` (written back, 0x47EE3D). UpdateCompareFlag(`CStreaming::HasSpecialCharLoaded(SP0) != 0`)
  (0x407F00, cdecl; flag byte preset 0 at 0x47EE42, set 1 at 0x47EE56). Return 0.

### 0x47E263 01F9 COMMAND_START_KILL_FRENZY  (gr: unregistered, used: 0/0/0)
- Parameters: ReadTextLabelFromScript(buf, 8) (0x47E26C) then CollectParameters(8) (0x47E2A0).
- Behaviour: `text = (_stricmp(buf, "DUMMY" @0x859EE0) == 0) ? nullptr : TheText.Get(buf)` (0x8229B6; CText::Get 0x6A0050,
  ECX = 0xC1B340). `CDarkel::StartFrenzy(SP0 weapon, SP1 time, (uint16)SP2 kills, SP3 model, text, SP4, SP5, SP6,
  SP7 != 0, false)` (0x43D3B0, cdecl 10 dwords, 0x47E2BC..0x47E2E6). Return 0.

### 0x47E304 01FA COMMAND_READ_KILL_FRENZY_STATUS  (gr: unregistered, used: 0/0/0)
- No inputs. `SP0 = (uint16)CDarkel::ReadStatus()` (0x43D1E0, zero-extended `movzx`, 0x47E309), StoreParameters(1). Return 0.

### 0x47E331 01FB COMMAND_SQRT  (gr: unregistered, used: 2/17/6)
- CollectParameters(1). `SP0 = fsqrt(SP0)` (x87 fsqrt on the f32, stored f32, 0x47E33A..0x47E346; negative input gives the
  default NaN), StoreParameters(1). Return 0.

### 0x47ED00 0237 COMMAND_SET_GANG_WEAPONS  (gr: unregistered, used: 1/1/0)
- CollectParameters(4). `CGangs::SetGangWeapons((uint16)SP0, SP1, SP2, SP3)` (0x5DE550, cdecl). Return 0.

### 0x47EEB8 0244 COMMAND_SET_CUTSCENE_OFFSET  (gr: unregistered, used: 0/0/1)
- CollectParameters(3). Calls 0x47E070 (cdecl, arg = &CVector(SP0,SP1,SP2); body .HOODLUM 0x15688D0 copies the three dwords
  to CCutsceneMgr::ms_cutsceneOffset 0xBC4034/0xBC4038/0xBC403C). Proposed name: CCutsceneMgr::SetCutsceneOffset(const CVector&).
  Return 0.

### 0x47E19B 01F6 COMMAND_CANCEL_OVERRIDE_RESTART  (gr: unregistered, used: 0/2/0)
- No parameters. `CRestart::CancelOverrideRestart()` (0x460800). Return 0.

### 0x47F08E 024F COMMAND_DRAW_CORONA  (gr: unregistered, used: 0/3/0)
- Parameters: CollectParameters(9): SP0/SP1/SP2 = x/y/z, SP3 = size (f32), SP4 = corona type (u8), SP5 = flare type (u8),
  SP6/SP7/SP8 = r/g/b (u8).
- Behaviour:
  1. `z = SP2`; `fcom [0x859014]=-100.0`, `test ah,0x41; jp` (0x47F09D..0x47F0BE): if `z <= -100.0f` -> `z = FindGroundZForCoord(SP0, SP1)`
     (0x47F0CB; no offset added). z stored f32 into the position vector (SP0, SP1, z) at [esp+0x50] (0x47F0D7..0x47F11C).
  2. Corona id (x87, 24-bit precision): `t = (SP1 - [0x859A90](-3000.0f)) * [0x859F1C]=12001.0f` (0x47F0DD/0x47F0E6/0x47F12B);
     `u = __ftol2(t)` (0x47F15A); `fu = (float)(int)u`, plus `[0x858C54]=4294967296.0f` if `u < 0` (unsigned conversion,
     0x47F168..0x47F171); `s = SP0 - (-3000.0f)` (0x47F177/0x47F17B); `id = __ftol2(fu + s) + (uint32)this->m_IP + (uint32)this`
     (`faddp` 0x47F181, 0x47F183..0x47F18B).
  3. `CCoronas::RegisterCorona(id, nullptr, SP6, SP7, SP8, 255, pos, SP3 /*size*/, 450.0f /*range, imm 0x43E10000*/,
     (u8)SP4 /*eCoronaType*/, (u8)SP5 /*flare*/, 1 /*reflType*/, 0 /*LOS*/, 0 /*trails*/, 0.0f /*normalAngle*/, 0 /*neonFade*/,
     1.5f /*pullTowardsCam, imm 0x3FC00000*/, 0 /*fullBrightAtStart*/, 15.0f /*fadeSpeed, imm 0x41700000*/, 0, 0)`
     (0x6FC580, cdecl 21 dwords, `add esp,0x54` 0x47F193). Return 0.
- Name correction: 0x6FC580 is the `eCoronaType` overload (gr Coronas.cpp:114 installs it under the label "texture" but with
  the eCoronaType signature; the catalogue name `RegisterCorona-texture` is misleading).
- gta-reversed: absent. Open questions: none.
