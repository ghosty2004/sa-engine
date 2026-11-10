# CRunningScript::ProcessCommands1100To1199 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` = `CPool<CVehicle>::GetAtRef(h)` 0x4048E0 (ECX =
CPools::ms_pVehiclePool [0xB74494]); `obj(h)` = `CPool<CObject>::GetAtRef(h)` 0x465040 (ECX = ms_pObjectPool [0xB7449C],
stride 0x19C); `GetRef` vehicle 0x424160, ped 0x4442D0 (ECX = ms_pPedPool [0xB74490]). Pool GetAtRef returns null when the
slot flag byte != h & 0xFF. "no null check" = the original dereferences the result directly.
CVehicle fields: +0x390 m_autoPilot (+0x2B m_nTempAction = veh+0x3BB, +0x2C m_nTempActionTime = veh+0x3BC,
+0x4B m_nCarCtrlFlags = veh+0x3DB, bit 3 bStayInFastLane), +0x45E u16 (random route seed), +0x460 m_pDriver,
+0x490 m_pFire, +0x4C0 m_fHealth, +0x508 m_nLastWeaponDamageType (u8), +0x590 m_nVehicleType (0 AUTOMOBILE, 9 BIKE),
+0x5A0 CDamageManager (CAutomobile), +0x65C/+0x65D CBike wheel status bytes (front/rear).

## 0x48A320 CRunningScript::ProcessCommands1100To1199  (catalogue: specified/REA, port level 18)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. **No SEH frame**: `sub esp,0x84`,
  push ebx/ebp/esi, this in ESI (0x48A329), push edi. `eax = command - 0x44D` (0x48A332), `cmp eax,0x62; ja 0x48B406`
  (0x48A338/0x48A33C): 1101..1199 dispatched; 1100 (044C) underflows and reaches the default. Two-level table:
  `movzx eax, byte [eax+0x48B4D8]` (0x48A342), `jmp [eax*4+0x48B418]` (0x48A349).
- Default 0x48B406: pop edi/esi/ebp, `or al,0xFF`, pop ebx, `add esp,0x84; ret 4` -> -1. Cases return 0 (`xor al,al` copies,
  e.g. 0x48B3A2, 0x48ABCB).
- Opcodes: 63 with a case: 33 gta-reversed `handler`, 12 `nop` (044D, 0450, 0451, 045D, 0469, 0481, 0482, 0493, 049C, 049E,
  049F, 04A1 all share 0x48B3F7 = `xor al,al` + epilogue: no parameters are collected, same as gr), 18 `unregistered`
  specified below.
- Default-reaching (37, all gr `unimplemented`): 044C, 044E, 044F, 0452, 0455, 0456, 045E, 045F, 0461, 0462, 046A, 046B, 046F,
  0478, 0479, 047B..047F, 0483, 0486, 0487, 048D, 048E, 0490, 0492, 0497..049B, 049D, 04A0, 04A8, 04AA, 04AC. No gr handler
  falls to default here.

### 0x48B100 04A4 COMMAND_IS_INT_LVAR_EQUAL_TO_CONSTANT  (gr: unregistered, used: 10/712/342)
- Parameters: `p = GetPointerToScriptVariable(1)` (0x48B104), then CollectParameters(1) (0x48B10F).
- Behaviour: UpdateCompareFlag(`*(int32*)p == SP0`) (0x48B114..0x48B124). Return 0.

### 0x48B0DC 04A3 COMMAND_IS_INT_VAR_EQUAL_TO_CONSTANT  (gr: unregistered, used: 17/250/401)
- Parameters: `p = GetPointerToScriptVariable(2)` (0x48B0E0), then CollectParameters(1) (0x48B0EB).
- Behaviour: `al = (*(int32*)p == SP0)` (0x48B0F0..0x48B0F8), jump to 0x48ABC3: UpdateCompareFlag(eax) (only AL meaningful),
  return 0.

### 0x48A974 046C COMMAND_GET_DRIVER_OF_CAR  (gr: unregistered, used: 0/66/14)
- CollectParameters(1). `d = veh(SP0)->m_pDriver` ([+0x460], 0x48A98E, no null check on the vehicle). `SP0 = d ? PedPool.GetRef(d)
  : -1` (0x48A99F / 0x48A9C1), StoreParameters(1). Return 0.

### 0x48A350 0453 COMMAND_SET_OBJECT_ROTATION  (gr: unregistered, used: 0/94/25)
- Parameters: CollectParameters(4): SP0 = object handle, SP1/SP2/SP3 = x/y/z angles in degrees.
- Behaviour: `o = obj(SP0)` (0x48A366, no null check). `CWorld::Remove(o)` (0x563280, cdecl; its stack slot is not popped and is
  reused as the third float argument below). Angles computed in this order: `rz = SP3 * [0x8595EC]=0.017453292f`,
  `ry = SP2 * 0.017453292f`, `rx = SP1 * 0.017453292f` (each fmul then fstp f32, 0x48A373..0x48A3A4);
  `CPlaceable::SetOrientation(o, rx, ry, rz)` (0x439A80 thiscall, ret 0xC); `CEntity::UpdateRwMatrix(o)` (0x446F90);
  `CEntity::UpdateRwFrame(o)` (0x532B00); `CWorld::Add(o)` (0x563220; `add esp,4` 0x48A3C0). Return 0.

### 0x48A723 0460 COMMAND_SET_INTERPOLATION_PARAMETERS  (gr: unregistered, used: 0/24/1)
- CollectParameters(2): SP0 = f32, SP1 = time (int).
- `TheCamera.SetParametersForScriptInterpolation(SP0, [0x858628]=100.0f - SP0, (uint32)SP1)` (0x50C030, thiscall ECX = 0xB6F028;
  fld 100.0, fsub SP0, fstp f32, 0x48A72C..0x48A749). Return 0.

### 0x48A8A8 0466 COMMAND_SET_CAR_STAY_IN_FAST_LANE  (gr: unregistered, used: 0/5/0)
- CollectParameters(2). `v = veh(SP0)` (no null check). If `SP1 != 0`: `[v+0x3DB] |= 0x08` (0x48A8D2) else `[v+0x3DB] &= 0xF7`
  (0x48A8EA) (bStayInFastLane). Return 0.

### 0x48A93B 0468 COMMAND_CLEAR_CAR_LAST_WEAPON_DAMAGE  (gr: unregistered, used: 0/2/0)
- CollectParameters(1). `v = veh(SP0)`; if non-null `[v+0x508] = 0xFF` (0x48A95E); null -> 0x48B3A2. Return 0.

### 0x48AB30 0477 COMMAND_SET_CAR_TEMP_ACTION  (gr: unregistered, used: 0/34/0)
- CollectParameters(3). `v = veh(SP0)` (no null check); `[v+0x3BB] = (u8)SP1` (0x48AB50); `[v+0x3BC] = CTimer::m_snTimeInMilliseconds
  [0xB7CB84] + SP2` (0x48AB5C..0x48AB64). Return 0.

### 0x48AC3E 0484 COMMAND_GET_REMOTE_CONTROLLED_CAR  (gr: unregistered, used: 0/2/0)
- CollectParameters(1): SP0 = player index. `rc = CWorld::Players[SP0].m_pRemoteVehicle` (`[0xB7CD98 + SP0*0x190 + 0xB0]`,
  0x48AC4C..0x48AC57; index not range-checked). `SP0 = rc ? VehPool.GetRef(rc) : -1` (0x48AC6C / shared 0x48A9C1),
  StoreParameters(1) (0x48A9CB). Return 0.

### 0x48AC7B 0485 COMMAND_IS_PC_VERSION  (gr: unregistered, used: 0/0/0)
- No parameters. UpdateCompareFlag(true) (0x48AC7B..0x48AC7F). Return 0.

### 0x48AD30 048A COMMAND_SET_ENABLE_RC_DETONATE  (gr: unregistered, used: 0/3/0)
- CollectParameters(1). `CVehicle::bDisableRemoteDetonation [0xC1CC00] = (SP0 == 0)` (0x48AD3E..0x48AD43). Return 0.

### 0x48AD58 048B COMMAND_SET_CAR_RANDOM_ROUTE_SEED  (gr: unregistered, used: 0/3/0)
- CollectParameters(2). `*(uint16*)(veh(SP0) + 0x45E) = (uint16)SP1` (0x48AD72/0x48AD79, no null check). Return 0.
- Open question: gr field name of CVehicle+0x45E (u16 random seed used by the path code) not cross-checked (0x48AD79).

### 0x48AD8F 048C COMMAND_IS_ANY_PICKUP_AT_COORDS  (gr: unregistered, used: 0/8/0)
- CollectParameters(3): SP0/SP1/SP2 = x/y/z.
- Behaviour: `found = 0` (bl, 0x48ADAE). Loop `p = &CPickups::aPickUps[0]` (0x9788C0, stride 0x20) for 620 entries
  (`ebp += 0x20` while `< 0x4D80`, 0x48AE37..0x48AE43); an iteration starting with `found != 0` exits immediately (0x48ADD0).
  For each `p` with `m_nPickupType` byte [p+0x1C] != 0 (0x48ADD4): `pos = CPickup::GetPosn(p)` (0x4549A0, thiscall, returns
  CVector by hidden pointer); `d = (pos.x - SP0, pos.y - SP1, pos.z - SP2)` (each fsub then fstp f32, 0x48ADE7..0x48AE17);
  `m = CVector::Magnitude(&d)` (0x4082C0, ST0); `fcomp [0x858B8C]=0.5f`, `test ah,5; jp` (0x48AE28..0x48AE33): `found = 1` only
  when `m < 0.5f` (ordered). UpdateCompareFlag(found) (0x48AE55). Return 0.
- gta-reversed: absent. Open questions: none.

### 0x48AEDC 0494 COMMAND_GET_POSITION_OF_ANALOGUE_STICKS  (gr: unregistered, used: 1/35/17)
- CollectParameters(1): SP0 = pad index. `pad = CPad::GetPad(SP0)` (0x53FB70, cdecl). SP0..SP3 = sign-extended int16 at
  pad+0, +2, +4, +6 (NewState LeftStickX, LeftStickY, RightStickX, RightStickY; 0x48AEF1..0x48AF19). StoreParameters(4).
  Return 0.

### 0x48AF33 0495 COMMAND_IS_CAR_ON_FIRE  (gr: unregistered, used: 0/27/1)
- CollectParameters(1). `v = veh(SP0)` (no null check). `r = (v->m_pFire [+0x490] != 0)` (0x48AF4F..0x48AF5E).
  If `v->m_nVehicleType [+0x590] == 0` (AUTOMOBILE only, 0x48AF63..0x48AF6B) and `CDamageManager::GetEngineStatus(v+0x5A0) >= 0xE1`
  (0x6C22C0, unsigned `jb`, 0x48AF6D..0x48AF7D) -> `r = 1`. `fcomp [0x859F80]=250.0f` on `v->m_fHealth`, `test ah,5; jp`
  (0x48AF84..0x48AF95): if `health < 250.0f` (ordered) -> `r = 1` (0x48AF9B). UpdateCompareFlag(r) (0x48AFA7 or 0x48A48E). Return 0.

### 0x48AFBB 0496 COMMAND_IS_CAR_TYRE_BURST  (gr: unregistered, used: 0/16/0)
- CollectParameters(2): SP0 = vehicle, SP1 = wheel (0..3, 4 = any). `v = veh(SP0)` (no null check), `r = 0` (0x48AFE3).
- If `v->m_nVehicleType == 9` (BIKE, 0x48AFD6):
  - `SP1 == 4`: `r = 1` if byte [v+0x65C] == 1 (0x48AFEF..0x48AFF8); then `r = 1` if byte [v+0x65D] == 1 (0x48AFFD, tail
    0x48ABB8 -> stub 0x404CF3 `mov byte [esp+0x10],1`).
  - else: `SP1 = 2 -> 0`, `SP1 = 3 -> 1` (written back to ScriptParams[1], 0x48B009..0x48B01C); any other value is used unchanged;
    `r = (byte [v + SP1 + 0x65C] == 1)` (0x48B022). Values outside 0..3 read past the two bike wheel bytes (original bug,
    no range check).
- Else (any non-bike type, including BMX):
  - `SP1 == 4`: for i = 0..3 (0x48B034..0x48B055): if `CDamageManager::GetWheelStatus(v+0x5A0, i) == 1` -> `r = 1` (stores AL).
  - else `r = (GetWheelStatus(v+0x5A0, SP1) == 1)` (0x6C21B0, 0x48B05C..0x48B06B).
  The damage manager is read at +0x5A0 regardless of whether the vehicle is a CAutomobile (boats/planes etc. not excluded).
- UpdateCompareFlag(r) (0x48ABBF..0x48ABC6). Return 0.

### 0x48B070 04A2 COMMAND_HELI_GOTO_COORDS  (gr: unregistered, used: 0/44/0)
- CollectParameters(6). `CAutomobile::TellHeliToGoToCoors(veh(SP0), SP1, SP2, SP3, SP4 /*altitudeMin*/, SP5 /*altitudeMax*/)`
  (0x6A2390 thiscall, 0x48B08B..0x48B0C8; no null or type check). Return 0.

### 0x48B19D 04A6 COMMAND_CREATE_PROTECTION_PICKUP  (gr: unregistered, used: 0/10/1)
- Parameters: CollectParameters(5): SP0/SP1/SP2 = x/y/z, SP3 = revenue limit, SP4 = revenue rate. Output SP0 = pickup handle.
- Behaviour: `z = SP2`; if `z <= -100.0f` ([0x859014], `test ah,0x41; jp`, 0x48B1B6..0x48B1CF): `z = FindGroundZForCoord(SP0, SP1)
  + [0x858B8C]=0.5f` (0x48B1D5/0x48B1DA). Dead call `CPickups::GetActualPickupIndex(CollectNextParameterWithoutIncreasingPC())`
  (0x48B1E9/0x48B1EF, result unused, as in 0213). `SP0 = CPickups::GenerateNewOne(CVector(SP0, SP1, z), MI_PICKUP_REVENUE
  (u16 [0x8CD5E0]), 0x10 /*PICKUP_ASSET_REVENUE*/, SP3 /*ammo*/, SP4 /*moneyPerDay*/, false, nullptr)` (0x456F20,
  0x48B203..0x48B243), StoreParameters(1). Return 0.
- gta-reversed: absent. Open questions: none.
