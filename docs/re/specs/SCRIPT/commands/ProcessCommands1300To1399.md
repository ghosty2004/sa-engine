# CRunningScript::ProcessCommands1300To1399 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Pool helpers (`__thiscall` on the pool pointer, `ret 4`):
0x4048E0 CPool<CVehicle>::GetAt(handle) (stride 0xA18; null when `flags[handle>>8] != (u8)handle`), 0x404910
CPedPool::GetAtRef, 0x465040 CPool<CObject>::GetAt (same body shape, stride 0x19C), 0x424160 (jmp to .HOODLUM
0x15667F0) CPool<CVehicle>::GetRef(veh) = `((veh - objects)/0xA18 << 8) + flags[idx]`. Pools: ped [0xB74490],
vehicle [0xB74494], object [0xB7449C]. "-100 ground rule": `fcomp z,[0x859014] = -100.0; fnstsw; test ah,0x41; jp`
skips when `z > -100` or NaN, so the ground lookup runs when `z <= -100.0`.

## 0x48CDD0 CRunningScript::ProcessCommands1300To1399  (catalogue: specified/REA, port level 28)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x48CDE3; command at [esp+4]), `ret 4`. No SEH frame:
  `sub esp,0x68; push ebx; push ebp; push esi; push edi` (0x48CDD4..0x48CDE2). `eax = command + 0xFFFFFAEB`
  (= command - 0x515, 0x48CDDA); `cmp eax,0x60; ja 0x48DD27` (0x48CDDF/0x48CDE5); byte index table
  `movzx eax, byte [eax + 0x48DDD4]` (0x48CDEB); jump table `jmp [eax*4 + 0x48DD34]` (0x48CDF2, 40 entries).
  Opcodes 0x514 (below the base) and > 0x575 reach the default. The command-argument stack slot [esp+0x7C] (after the
  prologue) is reused by several cases as the local bool for UpdateCompareFlag.
- Default 0x48DD27: `pop edi; pop esi; pop ebp; or al,0xFF; pop ebx; add esp,0x68; ret 4` -> 0xFF.
- Return-0 epilogue (inlined in every case; shared copies 0x48DD1B, 0x48DAA9, 0x48DB31):
  `xor al,al; pop edi; pop esi; pop ebp; pop ebx; add esp,0x68; ret 4`. Shared tails: 0x48CEC6 = `push 1;
  StoreParameters(1); return 0`; 0x48D077 / 0x48DA9D = `UpdateCompareFlag((bool)[esp+0x7C]); return 0`
  (0x48D070 `jne 0x48D077; mov byte [esp+0x7C],1` sets the flag on equality first). No case returns 1.
- Opcodes (100 in range 0x514..0x577): 55 with a case; gta-reversed handler: 20 (0x51A, 0x51B, 0x526, 0x52C, 0x547,
  0x54A, 0x54C, 0x54E, 0x555, 0x55D..0x561, 0x563, 0x568, 0x56C, 0x56D, 0x572, 0x575; not re-specified); gr nop: 16.
  Specified below: 19 (0517, 0518, 0519, 051C, 051D, 051E, 053E, 053F, 0541, 054F, 0550, 0564, 0565, 0566, 056A, 056E,
  0570, 0574, and the shared 0x48DD1B case containing 0521).
  Default-reaching (45, all gr unimplemented -> -1 with no params read, same as the original): 0514, 0516, 051F..0520,
  0527..052B, 052D..053D, 0542..0544, 0546, 0549, 0553..0554, 0558..055A, 055C, 0562, 0567, 056B, 056F, 0571, 0573,
  0576..0577. No gr `handler` opcode reaches the default.

### 0x48DD1B shared "return 0" case: 0515, 0521, 0522, 0523, 0524, 0525, 0540, 0545, 0548, 054B, 054D, 0551, 0552, 0556, 0557, 055B, 0569
- Behaviour: `xor al,al` + epilogue (0x48DD1B..0x48DD24): no parameter read, AL = 0.
- gta-reversed: the 16 others are `REGISTER_COMMAND_NOP(cmd)` without argument types (no params read, return 0):
  same as the original. 0521 COMMAND_IS_CHAR_DROWNING_IN_WATER (gr: unimplemented, CharacterCommands.cpp:4434, used
  0/0/0) returns OR_INTERRUPT (-1) in gr: DEVIATION (return value only; unused by scripts). Port: return 0.

### 0x48CFA5 0519 COMMAND_FREEZE_CAR_POSITION  (gr: unregistered, used: 3/261/13)
- Parameters: `CollectParameters(2)` (0x48CFA9): [0] vehicle handle, [1] int on/off.
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x48CFBB; no null check). `f = veh->m_nPhysicalFlags (+0x40)`.
  If P1 != 0 (0x48CFC7/0x48CFCC): `f |= 0x200C` (bits 2, 3, 13; gr names bDisableCollisionForce, bCollidable,
  bDontApplySpeed) stored (0x48CFD5); `CPhysical::SkipPhysics` 0x5433B0 (0x48CFD8); `CPhysical::SetVelocity(CVector(0,0,0))`
  0x441130 (3 pushed zeros, 0x48CFE5); 0x45AFB0 (jmp stub to .HOODLUM 0x15658A0, writes the CVector argument to this+0x50,
  `ret 0xC`) = CPhysical::SetTurnSpeed(CVector(0,0,0)) (0x48CFF2). Else: `f &= 0xFFFFDFF3` (0x48D003) stored. Return 0.
- Edge: invalid handle -> null dereference (original crash). gta-reversed: absent.

### 0x48D621 0550 COMMAND_FREEZE_OBJECT_POSITION  (gr: unregistered, used: 6/80/11)
- Parameters: `CollectParameters(2)` (0x48D625): [0] object handle, [1] int.
- Behaviour: `obj = ObjectPool.GetAt(P0)` 0x465040 (0x48D637; no null check). If P1 != 0: `obj->+0x40 |= 0x2004`
  (bits 2, 13; 0x48D649) else `&= 0xFFFFDFFB` (0x48D65E). Unlike 0519, bit 3 (0x8) is not touched and no velocity is
  cleared. Return 0. gta-reversed: absent.

### 0x48DB3D 0570 COMMAND_ADD_SHORT_RANGE_SPRITE_BLIP_FOR_CONTACT_POINT  (gr: unregistered, used: 34/75/0)
- Parameters: `CollectParameters(4)` (0x48DB41): [0] x, [1] y, [2] z (f32), [3] sprite (int);
  `CollectNextParameterWithoutIncreasingPC()` 0x464250 (0x48DB86, reads the output variable's current value);
  output StoreParameters(1) (0x48DBF3).
- Behaviour: -100 ground rule (0x48DB55..0x48DB6E): `z = CWorld::FindGroundZForCoord(x, y)` 0x569660 (0x48DB78, no
  offset added). `CRadar::GetActualBlipArrayIndex(old output value)` 0x582870 (0x48DB8C) - result discarded (EAX
  overwritten at 0x48DB9C). `h = CRadar::SetShortRangeCoordBlip(5 /*BLIP_CONTACT_POINT*/, CVector(x,y,z), 2 /*colour*/,
  3 /*BLIP_DISPLAY_BOTH*/, this->m_szName (this+8))` 0x583920 (pushes 0x48DBAA..0x48DBCD, call 0x48DBD2);
  `CRadar::SetBlipSprite(h, P3)` 0x583D70 (0x48DBE1); `add esp,0x24`; `ScriptParams[0] = h`; StoreParameters(1). Return 0.
- gta-reversed: absent.

### 0x48D356 053F COMMAND_SET_CAN_BURST_CAR_TYRES  (gr: unregistered, used: 0/99/0)
- Parameters: `CollectParameters(2)` (0x48D35A): [0] vehicle handle, [1] int.
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x48D36B, no null check); byte `veh+0x42B` (vehicleFlags 0x428, bit 31 =
  gr bTyresDontBurst): P1 != 0 -> `&= 0x7F` (0x48D380), else `|= 0x80` (0x48D395). Return 0. gta-reversed: absent.

### 0x48DAF9 056E COMMAND_DOES_VEHICLE_EXIST  (gr: unregistered, used: 8/75/5)
- Parameters: `CollectParameters(1)` (0x48DAFD): [0] vehicle handle.
- Behaviour: `[esp+0x7C] = 0` (0x48DB0F); if `VehiclePool.GetAt(P0)` (0x48DB17) != 0 set it to 1 (0x48DB20);
  `UpdateCompareFlag(flag)` 0x4859D0 (0x48DB2C). Return 0. gta-reversed: absent.

### 0x48DC3F 0574 COMMAND_FREEZE_CAR_POSITION_AND_DONT_LOAD_COLLISION  (gr: unregistered, used: 0/45/0)
- Parameters: `CollectParameters(2)` (0x48DC43): [0] vehicle handle, [1] int.
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x48DC55, no null check). If P1 != 0: `veh->+0x40 |= 0x2004` (0x48DC65);
  if `this->m_UsesMissionCleanup (+0xC6)` (0x48DC6C): `CWorld::Remove(veh)` 0x563280 (0x48DC77), `veh->+0x1C |= 0x40000`
  (CEntity flags bit 18, gr m_bIsStaticWaitingForCollision; 0x48DC7F), `CWorld::Add(veh)` 0x563220 (0x48DC89).
  Else (P1 == 0): `veh->+0x40 &= 0xFFFFDFFB` (0x48DC9D); byte `veh->m_nFakePhysics (+0xB8) = 0` (0x48DCA4). Return 0.
- Edge: for non-mission scripts (+0xC6 == 0) the "freeze" only sets the physical flags. gta-reversed: absent.

### 0x48D0D2 051C COMMAND_HAS_CAR_BEEN_DAMAGED_BY_CHAR  (gr: unregistered, used: 0/36/0)
- Parameters: `CollectParameters(2)` (0x48D0D6): [0] vehicle handle, [1] ped handle or -1 (any ped).
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x48D0E8). If `P1 != -1` (0x48D0F3) `ped = PedPool.GetAtRef(P1)` 0x404910
  (0x48D101), else EAX = the stale [esp+0x7C] (unused on that path). `res = 0` (0x48D10E). If `veh == 0` (0x48D113) or
  `d = veh->m_pLastDamageEntity (+0x50C)` == 0 (0x48D121) -> UpdateCompareFlag(0).
  If P1 == -1 (0x48D12A): `res = ((d->+0x36 & 7) == 3)` (entity type PED; 0x48D14F..0x48D156 then 0x48D070).
  Else: `if d == ped: res = 1` (0x48D12C..0x48D130); then if `ped->m_nPedFlags (+0x46C) & 0x100` (bInVehicle, 0x48D135
  `test dh,1`) and `d == ped->m_pVehicle (+0x58C)` (0x48D144) -> `res = 1`. `UpdateCompareFlag(res)` (0x48D07E). Return 0.
- Edge: an invalid ped handle gives `ped = 0` and 0x48D135 reads [0x46C] -> crash (only when the car has a damager).
  gta-reversed: absent.

### 0x48D15B 051D COMMAND_HAS_CAR_BEEN_DAMAGED_BY_CAR  (gr: unregistered, used: 0/8/1)
- Parameters: `CollectParameters(2)` (0x48D15F): [0] vehicle handle, [1] other vehicle handle or -1.
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x48D173); `other = P1 != -1 ? VehiclePool.GetAt(P1) : <stale>` (0x48D17E..
  0x48D18F). `res = 0`; if veh == 0 or `d = veh->+0x50C` == 0 -> false (0x48D19A/0x48D1A8). If P1 == -1:
  `res = ((d->+0x36 & 7) == 2)` (VEHICLE; 0x48D1BA..0x48D1C1); else `res = (d == other)` (0x48D1B3). UpdateCompareFlag(res).
  Return 0. An invalid P1 gives other = 0 -> false. gta-reversed: absent.

### 0x48CDF9 0517 COMMAND_CREATE_LOCKED_PROPERTY_PICKUP  (gr: unregistered, used: 0/26/0)
### 0x48CEDB 0518 COMMAND_CREATE_FORSALE_PROPERTY_PICKUP  (gr: unregistered, used: 26/6/0)
- Parameters: 0517: `CollectParameters(3)` (0x48CDFD) [0] x, [1] y, [2] z; 0518: `CollectParameters(4)` (0x48CEDF), plus
  [3] price (int). Then `ReadTextLabelFromScript(buf = esp+0x20, 8)` (0x48CE4C / 0x48CF2F), then
  `CollectNextParameterWithoutIncreasingPC()` (0x48CE62 / 0x48CF45). Output StoreParameters(1) (0x48CECA).
- Behaviour:
  1. -100 ground rule (0x48CE12 / 0x48CEF3): `z = CWorld::FindGroundZForCoord(x, y) + [0x858B8C] = 0.5`
     (`fadd` in x87 at 0x48CE36 / 0x48CF19, stored f32).
  2. `TheText (0xC1B340).Get(buf)` 0x6A0050 (0x48CE5B / 0x48CF3E): the returned GXT string is DISCARDED (EAX overwritten by
     the next call). `CPickups::GetActualPickupIndex(old output value)` 0x4552A0 (0x48CE68 / 0x48CF4B): result discarded.
  3. `h = CPickups::GenerateNewOne(CVector(x,y,z), model, type, ammo, 0 /*moneyPerDay*/, 0 /*isEmpty*/, buf)` 0x456F20
     (call 0x48CEB9, `add esp,0x24`): 0517 model = `(u16)MI_PICKUP_PROPERTY [0x8CD5E8]`, type 0x11, ammo 0;
     0518 model = `(u16)MI_PICKUP_PROPERTY_FORSALE [0x8CD5EC]`, type 0x12, ammo = P3 (price) (0x48CF71..0x48CF8A,
     joins 0517 at 0x48CEB9). The message argument is the raw 8-byte text-label buffer on the stack (char*), not the
     GXT text. `ScriptParams[0] = h` (0x48CEC1); StoreParameters(1). Return 0.
- Oddity: the message pointer is a stack buffer that dies when the case returns; GenerateNewOne must copy it (check
  its spec). gta-reversed: absent.

### 0x48D278 053E COMMAND_GET_RANDOM_CAR_OF_TYPE_IN_AREA_NO_SAVE  (gr: unregistered, used: 0/2/3)
- Parameters: `CollectParameters(5)` (0x48D27C): [0] x1, [1] y1, [2] x2, [3] y2 (f32), [4] model (int; < 0 = any).
  Output StoreParameters(1) via 0x48CEC6.
- Behaviour: `pool = [0xB74494]`, `i = pool->m_nSize (+8)`, `res = -1` (0x48D287/0x48D28A). If `i == 0` store -1.
  Loop (0x48D2B0..0x48D345), slots from `size-1` down to 0: `i--`; exit if `res != -1` (0x48D2B6/0x48D2BD);
  skip if `(i8)flags[i] < 0` (free, 0x48D2C6 `js`); `veh = objects + i*0xA18` (skip if 0); require
  `veh->GetVehicleAppearance()` 0x6D1080 == 1 (AUTOMOBILE) or, by a second call, == 2 (BIKE) (0x48D2D6..0x48D2EA);
  require `(i16)veh->m_nModelIndex (+0x22) == P4` or `P4 < 0` (0x48D2EC..0x48D2FB); require `veh->CanBeDeleted()`
  0x6D1180 (0x48D2FF); require `veh->IsWithinArea(P0, P1, P2, P3)` 0x54F200 (x/y 2D overload, 0x48D325). Then
  `res = VehiclePool.GetRef(veh)` 0x424160 (0x48D335) and byte `veh+0x42A |= 2` (vehicleFlags bit 17, gr
  bHasBeenOwnedByPlayer; 0x48D33C). Loop while `i != 0` (0x48D343). `ScriptParams[0] = res` (0x48D34B). Return 0.
- Note: no mission-cleanup registration (the "NO_SAVE" part); first match from the top of the pool wins.
  gta-reversed: absent.

### 0x48D3AA 0541 COMMAND_FIRE_HUNTER_GUN  (gr: unregistered, used: 0/1/0)
- Parameters: `CollectParameters(1)` (0x48D3AE): [0] vehicle handle.
- Behaviour: `veh = VehiclePool.GetAt(P0)` (0x48D3C0, no null check).
  1. If `CTimer::m_snTimeInMilliseconds [0xB7CB84] <= veh->m_nGunFiringTime (+0x4D4) + 150` (unsigned `jbe`,
     0x48D3D2..0x48D3DA) return 0.
  2. Local `CWeapon w(0x26 /*minigun*/, 5000)` via CWeapon::Constructor 0x73B430 (0x48D3EB). No destructor runs.
  3. `p = CAutomobile::vecHunterGunPos` (0x8D3394 = 0.0, 0x8D3398 = 4.8, 0x8D339C = -1.3; copied to a local).
     `a = veh->m_vecMoveSpeed (+0x44) * CTimer::ms_fTimeStep [0xB7CB5C]` via 0x40FEC0 (CVector*float, sret; computes
     z, y, x products, 0x48D41F); `b = *veh->m_matrix (+0x14) * p` via CMatrix::operator* 0x59C890 (0x48D433);
     `start = b + a` via 0x40FE30 (CVector+CVector, sret, 0x48D441); start copied over the local p.
  4. `w.FireInstantHit(veh, &start, &start, nullptr, nullptr, nullptr, false, true)` 0x73FB10 (0x48D476; both origin and
     muzzle point to the same local).
  5. `w.AddGunshell(veh, start, CVector2D(0.0, 0.1f), 0.025f)` 0x73A3E0 (0x48D49F; immediates 0 and 0x3DCCCCCD, size
     0x3CCCCCCD).
  6. `AudioEngine (0xB6BC90).ReportWeaponEvent(0x95, 0x26, veh)` 0x506F40 (0x48D4B1).
  7. `veh->m_nGunFiringTime = CTimer::m_snTimeInMilliseconds` (0x48D4BC). Return 0.
- gta-reversed: absent. Note `veh->m_matrix` is used without the allocate-if-null pattern.

### 0x48D5E8 054F COMMAND_CLEAR_CAR_LAST_DAMAGE_ENTITY  (gr: unregistered, used: 0/14/1)
- Parameters: `CollectParameters(1)` (0x48D5EC).
- Behaviour: `veh = VehiclePool.GetAt(P0)`; if null return 0 (0x48D605); `veh->m_pLastDamageEntity (+0x50C) = 0`
  (0x48D60B; plain store, no CEntity::CleanUpOldReference on the old pointer). Return 0. gta-reversed: absent.

### 0x48D903 0564 COMMAND_MAKE_HELI_COME_CRASHING_DOWN  (gr: unregistered, used: 0/7/0)
- Parameters: `CollectParameters(1)` (0x48D907).
- Behaviour: `veh = VehiclePool.GetAt(P0)` (no null check); `m = veh->m_autoPilot.m_nCarMission` (byte +0x390+0x2A = +0x3BA,
  0x48D91D). If `m == 0x39` (57 PLANE_CRASH_AND_BURN) or `m == 0x3A` (58 HELI_CRASH_AND_BURN) return 0 (0x48D928/0x48D931);
  else `= 0x3A` (0x48D93A). No vehicle-type check. Return 0. gta-reversed: absent.

### 0x48D94A 0565 COMMAND_ADD_EXPLOSION_NO_SOUND  (gr: unregistered, used: 0/2/0)
- Parameters: `CollectParameters(4)` (0x48D94E): [0] x, [1] y, [2] z (f32), [3] explosion type (int).
- Behaviour: `CExplosion::AddExplosion(nullptr, nullptr, P3, CVector(P0,P1,P2), 0 /*lifetime*/, 0 /*usesSound*/,
  -1.0f /*camShake, imm 0xBF800000*/, 0 /*isVisible*/)` 0x736A50 (pushes 0x48D964..0x48D999, `add esp,0x28`). Return 0.
- gta-reversed: absent.

### 0x48D9B2 0566 COMMAND_SET_OBJECT_AREA_VISIBLE  (gr: unregistered, used: 0/28/0)
- Behaviour: `CollectParameters(2)`; `obj = ObjectPool.GetAt(P0)` 0x465040 (0x48D9C8, no null check);
  `obj->m_nAreaCode (+0x2F) = (u8)P1` (0x48D9CD/0x48D9D3). Return 0. gta-reversed: absent.

### 0x48DA3C 056A COMMAND_WAS_CUTSCENE_SKIPPED  (gr: unregistered, used: 0/8/0)
- Behaviour: no params. `flag = CCutsceneMgr::ms_wasCutsceneSkipped [0xB5F854] != 0` (0x48DA3C..0x48DA48 -> 0x48DA96);
  `UpdateCompareFlag(flag)` (0x48DAA4). Return 0. gta-reversed: absent.

### 0x48D1C6 051E COMMAND_GET_RADIO_CHANNEL  (gr: unregistered, used: 0/2/4)
- Behaviour: no input params. `ScriptParams[0] = (i32)(i8)AudioEngine.GetCurrentRadioStationID() - 1` (0x507040 on
  0xB6BC90, `movsx edx,al; dec edx`, 0x48D1C6..0x48D1D4); StoreParameters(1) (via 0x48CEC6). Return 0.
- gta-reversed: absent.

Open questions: none beyond the null-dereference edge cases noted per opcode (they are original behaviour).
