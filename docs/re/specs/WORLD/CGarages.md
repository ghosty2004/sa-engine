# CGarages (WORLD) - full specs, writer E (RE-SPEC-1)

Globals (gta-reversed `Garages.h` StaticRef addresses):
- 0x96C048 `CGarages::aGarages` = CGarage[50], stride 0xD8; end address 0x96EA78 (the loops compare the
  pointer against 0x96EA78, which the catalogue labels `LastUpdatedGarageId`; it is just the array end).
- 0x96C024 i32 `NumGarages`; 0x96ABD8 `aCarsInSafeHouse` = CStoredCar[20][4] (0x40 per car, 0x100 per
  safehouse); 0x96BFE4 u8 `bCamShouldBeOutside`.
- CGarage layout: see CSaveGarage.md (header) and CGarage.md. CStoredCar (0x40): +0x00 CVector pos,
  +0x12 u16 m_wModelIndex, +0x3C/+0x3D/+0x3E packed forward x/y/z.
- x87 notes: precision control is 24 bit in game; "f32" below means a value loaded from / stored to a
  32-bit float. Comparison idioms: `fcomp b; fnstsw; test ah,5; jp X` jumps to X when NOT (st0 < b)
  (NaN included); `test ah,0x41; jne X` jumps when st0 <= b or unordered; `test ah,0x41; je X` jumps when
  st0 > b (ordered).

## 0x4471E0 CGarages::AddOne  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl int32 AddOne(float x1, float y1, float z1, float frontX, float frontY, float x2,
  float y2, float z2, uint8 type, uint32 unused, const char* name, uint32 argFlags)`. After the `push ecx`
  (4-byte temp at [esp]) the args are at [esp+8 .. +0x34]: x1 +8, y1 +0xC, z1 +0x10, frontX +0x14,
  frontY +0x18, x2 +0x1C, y2 +0x20, z2 +0x24, type +0x28 (low byte), unused +0x2C (never read), name
  +0x30, argFlags +0x34 (low byte). Returns EAX = index of the new garage (old NumGarages). Plain `ret`.
- Callers: 1 (CFileLoader::LoadGarage 0x5B4530). Callees: `_strncpy` 0x821F40 (CRT).
- Globals: R/W NumGarages 0x96C024; W fields of aGarages[NumGarages] (g = 0x96C048 + NumGarages*0xD8).
- Behaviour (min/max helpers: `mn(a,b) = (a < b) ? a : b`, `mx(a,b) = (a > b) ? a : b`, both pick the
  SECOND operand when the compare is unordered; all operands are f32 loaded from the stack):
  1. `D = (frontX + x2) - x1` computed in that order and stored as f32 to the temp.
     `g.m_fLeftCoord (+0x28) = mn(mn(mn(x1, frontX), x2), D)`.
     `g.m_fRightCoord (+0x2C) = mx(mx(mx(x1, frontX), x2), D)` (D reused from the temp).
  2. `D = (frontY + y2) - y1` -> temp. `g.m_fFrontCoord (+0x30) = mn(mn(mn(y1, frontY), y2), D)`,
     `g.m_fBackCoord (+0x34) = mx(mx(mx(y1, frontY), y2), D)`.
     (The code recomputes the inner mn/mx chain from memory instead of reusing it; same values.)
  3. `g.m_vPosn = (x1, y1, z1)` (raw dword copies).
     `g.dirA.x (+0x0C) = frontX - x1`, `g.dirA.y (+0x10) = frontY - y1`, `g.dirB.x (+0x14) = x2 - x1`,
     `g.dirB.y (+0x18) = y2 - y1` (each stored f32). `g.m_fTopZ (+0x1C) = z2` (raw copy).
  4. `w = sqrt(dirA.y*dirA.y + dirA.x*dirA.x)` (operands reloaded from the stored f32; sum order
     `dy^2 + dx^2`, x87 fsqrt) -> `g.m_fWidth (+0x20) = w` (fst).
     `g.dirA.x = dirA.x / w` (w taken from the register), store; `g.dirA.y = dirA.y / g.m_fWidth` (w
     reloaded from memory).
  5. Same for dirB: `h = sqrt(dirB.y^2 + dirB.x^2)` -> `g.m_fHeight (+0x24)`; `dirB.x /= h` (register),
     `dirB.y /= g.m_fHeight` (memory).
  6. `g.m_nType (+0x4C) = type`, `g.m_nOriginalType (+0x4F) = type`.
  7. `strncpy(g.m_anName (+0x44), name, 7)`.
  8. Flags byte g+0x4E (read after strncpy, NumGarages re-read): bit 0x08 (gr m_bDoorOpensUp) = argFlags&1,
     bit 0x10 (m_bDoorGoesIn) = argFlags&2, bit 0x20 (m_bCameraFollowsPlayer) = argFlags&4; other bits kept.
  9. `EAX = NumGarages; NumGarages = NumGarages + 1`.
- Edge cases: no bound check against 50. m_anName[7] is never written (relies on the zeroed .bss / earlier
  content; strncpy pads with zeros up to 7 only). m_nDoorState, m_fDoorPosition, m_nTimeToOpen,
  m_pTargetCar and the other flag bits are NOT initialised here (Init_AfterRestart/InitDoorsAtStart do it).
  Degenerate rectangles give w or h = 0 and division by zero (inf/NaN directions). The 4th corner D is
  rounded to f32 before the comparisons.
- gta-reversed: Garages.cpp:279, `plugin::Call` only (not reversed); gr declares a void return, the bytes
  return the index in EAX. gr comment flag meanings agree with the bits above.
- Open questions: none.

## 0x448650 CGarages::CameraShouldBeOutside  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool (void)`, returns AL, plain `ret`. Body in .HOODLUM at 0x1565080.
- Callers: CCamera::CamControl 0x527FA0 (2 sites). Callees: none.
- Behaviour: `al = *(u8*)0x96BFE4` (bCamShouldBeOutside). Upper EAX bits are not cleared.
- gta-reversed: inline getter in Garages.h; same.

## 0x448660 CGarages::GivePlayerDetonator  (catalogue: specified/REA, port level 14)
- Convention: `__cdecl void (void)`, plain `ret`. Body in .HOODLUM at 0x1569B90.
- Callers: CGarage::Update 0x44AA50 (bomb shop, type 4). Callees: CWorld::FindPlayerPed 0x56E210,
  CWeaponInfo::GetWeaponInfo 0x743C60, CPed::GiveWeapon 0x5E6080.
- Behaviour:
  1. `ped = FindPlayerPed(-1)`.
  2. `slot = GetWeaponInfo(40 /*WEAPON_DETONATOR*/, 1 /*skill STD*/)->m_nSlot` (i32 at info+0x14);
     GetWeaponInfo is cdecl (caller pops 0xC together with the FindPlayerPed arg).
  3. `ped->GiveWeapon(40, 1 /*ammo*/, 1 /*likeUnused*/)` (thiscall).
  4. `*(u32*)(ped + 0x5A4 + slot*0x1C) = 0` (m_aWeapons[slot].m_nState = WEAPONSTATE_READY; weapons at
     +0x5A0, stride 0x1C).
  5. `ped->m_pPlayerData (+0x480)->m_nChosenWeapon (+0x20) = (u8)slot`.
  6. `if (*(i32*)(ped+0x70C) != 55) *(i32*)(ped+0x70C) = 40` (m_nSavedWeapon; 55 = WEAPON_UNIDENTIFIED).
- gta-reversed: Garages.cpp:263 (reversed); same semantics.
- Open questions: none.

## 0x448890 CGarages::StopCarFromBlowingUp  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void (CAutomobile* car = [esp+4])`, plain `ret`.
- Callers: none in the exe (dead). Callees: CDamageManager::GetEngineStatus 0x6C22C0 (2 sites),
  CDamageManager::SetEngineStatus 0x6C22A0 (2 sites); both thiscall on `car + 0x5A0` (m_damageManager).
- Globals: R 0x858FD8 f32 300.0.
- Behaviour:
  1. `*(u32*)(car+0x8E4) = 0` (CAutomobile m_fBurningTimer = 0.0f), done before the health read.
  2. `h = car->m_fHealth (+0x4C0)`; `car->m_fHealth = (h > 300.0f) ? h : 300.0f` (NaN -> 300.0f).
  3. `if (GetEngineStatus() < 275 /*0x113, unsigned*/) SetEngineStatus(275); else
     SetEngineStatus(GetEngineStatus())` (GetEngineStatus called a second time).
- gta-reversed: Garages.cpp:520, plugin::Call plus untested dead code: `m_fHealth <= 300 ? 300 : health`
  keeps NaN where the bytes give 300 (differs for NaN only; keep-original).
- Open questions: none.

## 0x448990 CGarages::IsPointWithinAnyGarage  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl bool (const CVector* point = [esp+4])`, AL, plain `ret`. Body in .HOODLUM
  0x156EF90.
- Callers: CTaskSimpleCarSetPedOut::ProcessPed 0x647D10. Callees: CGarage::IsPointInsideGarage(CVector)
  0x448740 (thiscall, CVector by value: 3 dwords copied onto the stack).
- Behaviour: for g in aGarages[0..50) (all 50, not NumGarages, ascending): if `g.m_nType != 0` and
  `g.IsPointInsideGarage(*point)` -> return true. Return false.
- Register note: the loop keeps the garage pointer in EDX across the call; 0x448740 must preserve EDX.
- gta-reversed: Garages.cpp:398 (`garage.m_nType != INVALID || IsPointInsideGarage`) is WRONG: the bytes
  use AND (skip invalid garages). Keep-original.
- Open questions: none.

## 0x4489F0 CGarages::FindSafeHouseIndexForGarageType  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl int32 (int32 type = [esp+4])` (full dword used), EAX, plain `ret`.
- Callers: CloseHideOutGaragesBeforeSave 0x44A170, CountCarsInHideoutGarage 0x44A210,
  StoreCarInNearestImpoundingGarage 0x44A3C0, CGarage::Update 0x44AA50 (5 sites).
- Behaviour: jump table at 0x448A78 for type-16 in 0..29; out of range -> 0. Mapping:
  17->1, 18->2, 24->3, 25->4, 26->5, 27->6, 28->7, 29->8, 30->9, 31->10, 32->11, 33->12, 34->13, 35->14,
  39->15, 40->16, 41->17, 42->18, 45->19; every other type (including 16 SAFEHOUSE_GANTON, 19..23,
  36..38, 43, 44) -> 0.
- gta-reversed: Garages.cpp:198, plugin::Call followed by a dead switch marked `// wrong`; the dead switch
  is in fact identical to the bytes (gr-deviations row: deviates=no). Port the table above.
- Open questions: none.

## 0x449BA0 CGarages::IsPointInAGarageCameraZone  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl bool (CVector point)` by value: x [esp+4], y [esp+8], z [esp+0xC]; AL; plain `ret`.
- Callers: CCamera::CamControl 0x527FA0 (2 sites). Callees: CGarage::IsPointInsideGarage(CVector, float)
  0x4487D0.
- Behaviour: for g in aGarages[0..50): switch on g.m_nType via tables 0x449C0C/0x449C00: type 0 -> skip;
  types 1..22 -> call; type > 22 -> call (all non-zero types end at the same call). If
  `g.IsPointInsideGarage(point, 0.5f)` -> return true. Return false.
- Register note: ECX (garage pointer), EDX, ESI, EDI (point) are kept across the call; 0x4487D0 must
  preserve ECX and EDX.
- gta-reversed: Garages.cpp:408 `m_nType && IsPointInsideGarage(point, 0.5f)`; matches.
- Open questions: none.

## 0x44A210 CGarages::CountCarsInHideoutGarage  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl int32 (uint8 type)` (movzx of [esp+4]), EAX, plain `ret`. Body .HOODLUM 0x1567330.
- Callers: CGarage::Update. Callees: FindSafeHouseIndexForGarageType 0x4489F0.
- Behaviour: `i = FindSafeHouseIndexForGarageType(type)`; count the 4 cars
  `aCarsInSafeHouse[i][k].m_wModelIndex != 0` (u16 at 0x96ABEA + i*0x100 + k*0x40), k = 0..3; return count.
- gta-reversed: plugin::Call only.
- Open questions: none.

## 0x44A240 CGarages::FindGarageForObject  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl int32 (CObject* obj = [esp+4])` (gr: int16), EAX = garage index or -1, plain `ret`.
- Callers: CObject::ProcessGarageDoorBehaviour 0x44A4D0. Callees: CGarage::IsPointInsideGarage(CVector,
  float) 0x4487D0.
- Globals: R NumGarages (re-read every iteration), aGarages; R f32 0x858B8C = 0.5, 0x858B50 = 0.0.
- Behaviour:
  1. `best = 99999.9f` (0x47C34FF3), `bestIdx = -1`. If NumGarages == 0 return -1.
  2. `mat = obj->m_matrix (+0x14)` read once before the loop; `pos = mat ? &mat->pos (+0x30) : &obj+4`.
  3. For i = 0; (u32)i < NumGarages; i++ (g = &aGarages[i]):
     - if `!g.IsPointInsideGarage(*pos, 7.0f)` continue.
     - Garage centre C (same arithmetic as NeatlyLineUpStoredCars, CGarage.md 0x448330):
       `hw = width*0.5f` (stored f32); `ax = dirA.x*hw` (f32), `ay = dirA.y*hw` (register),
       `az = hw*0.0f` (f32); `C.x = ax + posn.x` (f32); `cy = ay + posn.y` (register);
       `cz = az + posn.z` (register, posn.z first stored f32);
       `hh = height*0.5f`; `bx = dirB.x*hh` (f32), `by = dirB.y*hh` (reg), `bz = hh*0.0f` (f32);
       `C.x = bx + C.x`, `C.y = by + cy`, `C.z = bz + cz` (all stored f32).
     - `d = pos - C` per component (pos reloaded from obj); `dist = sqrt((dz*dz + dy*dy) + dx*dx)`.
     - if `dist < best`: `best = dist` (f32), `bestIdx = i`.
  4. Return bestIdx.
- Register note: the loop keeps ECX (garage) and EDX (obj matrix) across 0x4487D0.
- gta-reversed: plugin::Call only.
- Open questions: none.

## 0x44A3C0 CGarages::StoreCarInNearestImpoundingGarage  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl void (CVehicle* veh = [esp+4])`, plain `ret`.
- Callers: CWorld::ClearCarsFromArea 0x566610, CWorld::ClearExcitingStuffFromArea 0x56A0D0. Callees:
  FindSafeHouseIndexForGarageType 0x4489F0, CStoredCar::StoreCar 0x449760 (thiscall, ret 4).
- Behaviour:
  1. `best = 99999.9f`, `bestIdx = -1`; `n = NumGarages` (read once). If n == 0 return.
  2. For i in 0..n (unsigned): g = aGarages[i]; skip unless 33 <= g.m_nType <= 35 (IMPOUND_LS/SF/LV).
     `p = veh matrix ? matrix->pos : veh+4` (re-evaluated per garage); `dx = p.x - g.posn.x`,
     `dy = p.y - g.posn.y` (g.posn.y first stored to a f32 temp); `dist = sqrt(dy*dy + dx*dx)`
     (garage CORNER m_vPosn, not the centre; 2D). If `dist < best`: best = dist (f32), bestIdx = i.
  3. If bestIdx < 0 return.
  4. `cars = aCarsInSafeHouse[FindSafeHouseIndexForGarageType(aGarages[bestIdx].m_nType)]`.
  5. `c = (cars[0].model != 0) + (cars[1].model != 0) + (cars[2].model != 0)` (slot 3 ignored).
  6. If c == 3: `cars[0] = cars[1]; cars[1] = cars[2];` (0x40-byte rep movsd copies), slot = 2;
     else slot = c.
  7. `cars[slot].StoreCar(veh)`.
- Edge cases: the slot is the COUNT, not the first free slot: with cars[0] empty and cars[1] used, c = 1 and
  cars[1] is overwritten (original quirk, keep). The vehicle is not removed here.
- gta-reversed: plugin::Call only.
- Open questions: none.
