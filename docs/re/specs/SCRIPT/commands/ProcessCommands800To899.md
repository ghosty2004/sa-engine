# CRunningScript::ProcessCommands800To899 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` = `CPool<CVehicle>::GetAtRef(h)` 0x4048E0 (ECX =
CPools::ms_pVehiclePool [0xB74494]); `obj(h)` = `CPool<CObject>::GetAtRef(h)` 0x465040 (ECX = ms_pObjectPool [0xB7449C]).
GetAtRef returns null when the slot flag byte != h & 0xFF; "no null check" = result dereferenced directly.
Entity flags dword +0x1C: bit 0 bUsesCollision, bit 7 bIsVisible, bit 9 (0x200) bRenderDamaged. Position =
`m_matrix (+0x14) ? m_matrix->pos (+0x30) : placement.pos (+0x04)`. Model info: `CModelInfo::ms_modelInfoPtrs[ (int16)[e+0x22] ]`
(0xA9B0C8), `+0x14` = CColModel* whose CBox bounding box is at +0 (min) / +0xC (max).
Used-object model: for a negative model id `m`, `*(int32*)(0xA44B88 + (-m)*0x1C)` (CTheScripts::UsedObjectArray[-m].nModelIndex).

## 0x481300 CRunningScript::ProcessCommands800To899  (catalogue: specified/REA, port level 26)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x8383A8,
  0x481300..0x48130E), `sub esp,0x1EC`, push ebx/ebp/esi/edi. **EDI = 0 for the whole function** (0x48131F; many cases compare
  against / push EDI as zero). this in ESI (0x481321). A function-scope `CMatrix` lives at [esp+0x9C]: its attach fields
  (+0x40 m_pAttachMatrix [esp+0xDC], +0x44 m_bOwnsAttachedMatrix [esp+0xE0]) are zeroed at entry (0x481323/0x48132A) and the
  SEH state [esp+0x204] = 0 (0x481340); every exit runs `~CMatrix` 0x59ACD0 on it after setting the state to -1.
  `eax = command - 0x321` (`add eax,0xFFFFFCDF`, 0x481338), `cmp eax,0x62; ja 0x4835CC` (0x48133D/0x481347): 801..899
  dispatched; 800 (0320) reaches the default. Two-level table `movzx eax, byte [eax+0x4836B8]` (0x48134D),
  `jmp [eax*4+0x483600]` (0x481354).
- Common return-0 tail 0x4835B1: `nop; nop; jmp 0x4039D5` (.text stub: `lea ecx,[esp+0x9C]; jmp 0x4835B8`), state = -1
  (0x4835B8), `~CMatrix(&local)` (0x4835C3), `xor al,al`, epilogue 0x4835E5 (restore fs:[0] from [esp+0x1FC], pop, `add esp,0x1F8;
  ret 4`). Default 0x4835CC: same destructor, `or al,0xFF` -> -1.
- Opcodes: 48 with a case: 29 gta-reversed `handler`, 2 `nop` (037E, 0383) and 2 `unimplemented` (0329 HAS_RESPRAY_HAPPENED,
  032C SET_CAR_RAM_CAR) whose case IS the common tail 0x4835B1 (no parameters collected, return 0), 15 `unregistered` specified below.
- Default-reaching (52): 0320, 0322, 0324, 0328, 032D..032F, 0333, 0334, 0336, 033A..033D, 0346, 0347, 034A..034C, 0351..0355,
  0357..035B, 035E, 0365, 0367..0369, 036B, 036C, 0372, 0374, 0375, 0377..037D, 037F, 0380 (gr `unimplemented`) and
  **036E, 036F, 0370, 0371 COMMAND_PRINT_WITH_3/4/5/6_NUMBERS_BIG**, which gta-reversed implements as handlers but which in the
  exe have no case (byte table entry -> default): the original returns -1 for them (gr deviation; ports keep -1).

### 0x4835B1 0329 COMMAND_HAS_RESPRAY_HAPPENED, 032C COMMAND_SET_CAR_RAM_CAR (+ gr-nop 037E, 0383)  (gr: unimplemented, used: 0/0/0)
- No parameters read, compare flag untouched, return 0 (common tail). gta-reversed: `unimplemented` stubs (VehicleCommands.cpp:172/173).

### 0x482C67 035C COMMAND_PLACE_OBJECT_RELATIVE_TO_CAR  (gr: unregistered, used: 0/399/0)
- CollectParameters(5): SP0 = object, SP1 = vehicle, SP2/SP3/SP4 = offset.
- `CPhysical::PlacePhysicalRelativeToOtherPhysical(veh(SP1), obj(SP0), CVector(SP2,SP3,SP4))` (0x546DB0, cdecl 5 dwords,
  `add esp,0x14`; neither handle null-checked). Return 0.

### 0x482339 034E COMMAND_SLIDE_OBJECT  (gr: unregistered, used: 2/71/2)
- Parameters: CollectParameters(8): SP0 = object, SP1/SP2/SP3 = target x/y/z, SP4/SP5/SP6 = step x/y/z, SP7 = collision check flag.
- Behaviour (o = obj(SP0), no null check; p = copy of o's position, 0x482355..0x48238C; t = (SP1,SP2,SP3) copied to locals,
  tz at [esp+0x1C]):
  1. If `p.x == t.x && p.y == t.y && p.z == t.z` (each `fcomp; test ah,0x44; jp` = ordered equality, 0x482388..0x4823C0):
     UpdateCompareFlag(true), return 0 (0x4823C2..0x4823D3).
  2. `dx = p.x - t.x`, `dy = p.y - t.y` (stored f32), `dz = p.z - t.z` (stored f32) (0x4823D8..0x482411). Per axis
     (x: 0x482415..0x482464 using ST0 = dx; y: 0x482468..0x4824BF; z: 0x4824C3..0x48251A), with step s:
     - if `d < 0` (`test ah,1`): if `-d > s` (fchs; fcomp s; `test ah,0x41; jne` -> not taken only when strictly greater):
       `p.a = s + p.a`; else `p.a = t.a`.
     - else (d >= 0 or NaN): if `d > s`: `p.a = p.a - s`; else `p.a = t.a`.
     (equality, and NaN comparisons, snap to the target.)
  3. If `SP7 != 0` (0x48251E): local matrix = `*o->m_matrix` (CMatrix::operator= 0x59BBC0, the object's matrix pointer is not
     null-checked), its pos = new p (0x48253B..0x482555); bounding box corners and min/max as in 034D step 4 (0x48255C..0x4829D2);
     `CWorld::FindObjectsIntersectingAngledCollisionBox(colModel->bbox, local, p, minX, minY, maxX, maxY, &count, 2, nullptr,
     false, true, true, false, false)` (0x568FF0, 15 dwords); if `(int16)count > 0` -> UpdateCompareFlag(true), return 0
     (0x482A26 -> 0x4823C2); the object is NOT moved.
  4. `o->vtable[0x38/4 = 14](CVector p, false)` (CEntity::Teleport(CVector, bool), 0x482A2C..0x482A4B).
  5. UpdateCompareFlag(`p.x == t.x && p.y == t.y && p.z == t.z`) (ordered equality, 0x482A4E..0x482A8C). Return 0.
- Oddity: the result byte is stored into the low byte of the tz local ([esp+0x1C]) after its last read (0x482A74); harmless.
- gta-reversed: absent. Open questions: see 034D (corner order).

### 0x481CA5 034D COMMAND_ROTATE_OBJECT  (gr: unregistered, used: 12/28/1)
- Parameters: CollectParameters(4): SP0 = object, SP1 = target heading (degrees, f32), SP2 = step (degrees), SP3 = collision check.
- Behaviour (o = obj(SP0), no null check; flag = 0 at 0x481CF9):
  1. `h = CPlaceable::GetHeading(o) * [0x859878]=57.295776f` (0x441DB0, ST0; 0x481CC4/0x481CC9); if `h < 0` (`test ah,5; jp`,
     ordered) `h += [0x859E2C]=360.0f` (0x481CDC). h stays in ST0 (extended exponent, 24-bit significand).
  2. If `h == SP1` (`fcom; test ah,0x44; jp`, 0x481CF1..0x481D03): flag = 1 -> UpdateCompareFlag, return 0 (0x481D07 -> 0x482323).
  3. `cw = SP1 - h` (stored f32 [esp+0x3C], 0x481D0C..0x481D12); `ccw = h - SP1` (ST0, `fsubr`, 0x481D16..0x481D1A).
     If `cw < 0`: `cw += 360.0f` (stored, 0x481D2D); if `ccw < 0`: `ccw += 360.0f` (ST0, 0x481D48).
  4. If `cw < ccw` (0x481D4E..0x481D59): `new = (cw < SP2) ? SP1 : SP2 + h` (0x481D5D..0x481D82);
     else: `new = (ccw < SP2) ? SP1 : h - SP2` (0x481D86..0x481DA1). All compares ordered `<` (NaN takes the second branch).
     new is not wrapped into 0..360.
  5. If `SP3 != 0` (0x481DA5): `local.SetRotateZ(new * [0x8595EC]=0.017453292f)` (0x59B0E0 on the function's CMatrix); local.pos +=
     o's position (each `fld local; fadd pos; fstp`, 0x481DEF..0x481E1E). Four corners of the model's col bounding box (all at
     z = bbox.min.z per the copies 0x481E25..0x481F0A) are transformed with `operator*(CMatrix const&, CVector const&)` 0x59C890
     (4 calls, 0x481F2A..0x481FBA); minX/minY/maxX/maxY are chains of `(a < b) ? a : b` / `(a > b) ? a : b` over the four
     transformed corners (0x481FBF..0x48229B). `CWorld::FindObjectsIntersectingAngledCollisionBox(bbox, local, pos, minX, minY,
     maxX, maxY, &count, 2, nullptr, false /*buildings*/, true /*vehicles*/, true /*peds*/, false, false)` (0x568FF0,
     0x4822AA..0x4822E1, `add esp,0x3C`). If `(int16)count > 0` (0x4822EF): flag = 1 (obstacle), object not rotated.
  6. Otherwise: `CPlaceable::SetHeading(o, new * 0.017453292f)` (0x43E0C0), `CEntity::UpdateRwMatrix(o)` (0x446F90),
     `CEntity::UpdateRwFrame(o)` (0x532B00); flag = (`new == SP1`, ordered equality, 0x482314..0x482321).
  7. UpdateCompareFlag(flag) (0x48232F). Return 0.
- gta-reversed: absent (re3's VC version has the same structure: corners min, (max.x,min.y), (min.x,max.y), (max.x,max.y)).
- Open question: exact corner composition and the operand order of each min/max compare (ties/NaN) in 0x481E25..0x48229B
  (and the copy in SLIDE_OBJECT 0x48255C..0x4829D2) were read structurally, not instruction by instruction; a port must re-derive
  them from the bytes.

### 0x481678 032B COMMAND_CREATE_PICKUP_WITH_AMMO  (gr: unregistered, used: 14/198/0)
- CollectParameters(6): SP0 = model (negative -> used-object model, 0x481687..0x481690), SP1 = type (u8),
  SP2 = ammo, SP3/SP4/SP5 = x/y/z. Output SP0 = pickup handle.
- `z = SP5`; if `z <= -100.0f` ([0x859014], `test ah,0x41; jp`): `z = FindGroundZForCoord(SP3, SP4) + [0x858B8C]=0.5f`.
  Dead `CPickups::GetActualPickupIndex(CollectNextParameterWithoutIncreasingPC())` (0x4816DC/0x4816E2, result unused).
  `SP0 = CPickups::GenerateNewOne(CVector(SP3, SP4, z), model, (u8)SP1, SP2 /*ammo*/, 0, false, nullptr)` (0x456F20,
  0x4816FD..0x481744), StoreParameters(1). Return 0.

### 0x4818BB 0339 COMMAND_IS_AREA_OCCUPIED  (gr: unregistered, used: 0/6/36)
- CollectParameters(11): SP0..SP2 = corner 1, SP3..SP5 = corner 2, SP6..SP10 = buildings, vehicles, peds, objects, dummies flags.
- Sort per axis, swapping only when the first value is strictly greater (`fcomp; test ah,0x41; jne`, NaN keeps order):
  x (0x4818E9..0x481916), y (0x48191A..0x481935), z on the x87 stack (0x481939..0x48194E). `min = (x_lo, y_lo, z_lo)`,
  `max = (x_hi, y_hi, z_hi)` built at [esp+0x118] / [esp+0x130].
- `CWorld::FindObjectsIntersectingCube(min, max, &count, 2, nullptr, SP6 != 0, SP7 != 0, SP8 != 0, SP9 != 0, SP10 != 0)`
  (0x568DD0, 10 dwords, 0x48199D..0x4819E3). UpdateCompareFlag(`(int16)count > 0`) (0x4819EB..0x4819FE). Return 0.

### 0x48150A 0327 COMMAND_GET_RANDOM_CAR_OF_TYPE_IN_AREA  (gr: unregistered, used: 0/1/0)
- CollectParameters(5): SP0..SP3 = x1, y1, x2, y2, SP4 = model (< 0 = any). Output SP0 = vehicle handle or -1.
- Behaviour: `ref = -1` (0x48151C). Iterate the vehicle pool slots from `size-1` down to 0 (`size = pool+8`, 0x481519..0x481545,
  `ebx--`, loop test 0x4815FD), stopping once `ref != -1` (0x481546). For each used slot (flag byte >= 0, 0x481556) with
  `v = base + i*0xA18`: require `CVehicle::GetVehicleAppearance(v)` (0x6D1080) == 1 or 2 (0x48156E..0x481582); `!(byte [v+0x428] & 1)`
  (not bIsLawEnforcer, 0x481584); `(int16)[v+0x22] == SP4 || SP4 < 0` (0x48158D..0x48159C); `CVehicle::CanBeDeleted(v)` (0x6D1180);
  `IsWithinArea(v, SP0, SP1, SP2, SP3)` (0x54F200). Then `ref = GetRef(v)` (0x424160), `CVehicle::SetVehicleCreatedBy(v, 2)`
  (0x6D5D70, MISSION_VEHICLE); if `this->m_UsesMissionCleanup`: `MissionCleanUp.AddEntityToList(ref, 1)` (0x4637E0, ECX 0xA90850).
  Finally `SP0 = ref`, StoreParameters(1) (0x481605..0x48160F). Return 0.

### 0x482CDB 035D COMMAND_MAKE_OBJECT_TARGETTABLE  (gr: unregistered, used: 0/51/0)
- CollectParameters(2). `o = obj(SP0)`; `ped = CWorld::Players[CWorld::PlayerInFocus].m_pPed` (byte [0xB7CD74] * 0x190 + 0xB7CD98).
  If `SP1 != 0`: `CObject::SetObjectTargettable(o, true)` (0x59F300). Else: `SetObjectTargettable(o, false)`; if
  `ped->m_pTargetedObject [+0x71C] == o` (0x482D29..0x482D37; ped not null-checked): if non-null
  `CEntity::CleanUpOldReference(o, &ped->m_pTargetedObject)` (0x571A00, thiscall ECX = o) and `ped->m_pTargetedObject = nullptr`
  (0x482D47). Return 0.

### 0x482E1D 0360 COMMAND_OPEN_GARAGE / 0x482E60 0361 COMMAND_CLOSE_GARAGE  (gr: unregistered, used: 0/16/0 and 1/23/0)
- ReadTextLabelFromScript(buf, 8); `g = CGarages::GetGarageNumberByName(buf)` (0x447680); if `(int16)g < 0` return 0
  (`cmp ax,di; jl`); else `CGarages::aGarages[(int16)g]` (0x96C048 + g*0xD8) `.OpenThisGarage()` 0x447D50 / `.CloseThisGarage()`
  0x447D70. Return 0.

### 0x482F84 0363 COMMAND_SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE  (gr: unregistered, used: 4/10/0)
- CollectParameters(6): SP0/SP1/SP2 = x/y/z, SP3 = radius, SP4 = model (negative -> used-object model, 0x482FE7..0x482FF4),
  SP5 = visible flag.
- Behaviour: if `z <= -100.0f` ([0x859014], `test ah,0x41; jp`): `z = FindGroundZForCoord(x, y)` (no offset, 0x482FC3).
  `best = SP3 + SP3` (fadd st0,st0, f32 [esp+0x40], 0x482FE3). `count = 0`;
  `CWorld::FindObjectsOfTypeInRange(model, pos, SP3, true /*2D*/, &count, 16, list, true /*buildings*/, false, false,
  true /*objects*/, true /*dummies*/)` (0x564C70, 12 dwords). If `count == 0`: `CWorld::FindLodOfTypeInRange(model, pos, SP3,
  true, &count, 16, list)` (0x564ED0, 7 dwords).
  For i = 0 .. count-1 (u16 counter): `e = list[i]`; `d = |pos(e) - pos|` computed as `dz = e.z - z; dy = e.y - y; dx = e.x - x;
  sqrt((dx*dx + dy*dy) + dz*dz)` (3D, fsqrt; 0x4830D6..0x483108); if `d < best` (ordered): `best = d`, `chosen = e` (0x483115).
  If chosen (0x483127): if `SP5 != 0`: `[e+0x1C] |= 0x81` (visible + uses collision, 0x48313E); if byte [e+0x2E] == 0: scan
  `CTheScripts::InvisibilitySettingArray[0..19]` (0xA449E0, u16 index) for e and zero the first match (0x483153..0x483179).
  Else: `[e+0x1C] &= ~0x81` (0x48318B), `CTheScripts::AddToInvisibilitySwapArray(e, false)` (0x481200, cdecl). Return 0.
- Open question: meaning of the entity byte +0x2E tested at 0x48313B (gr CEntity field name not cross-checked).

### 0x4831F8 0366 COMMAND_HAS_OBJECT_BEEN_DAMAGED  (gr: unregistered, used: 0/43/0)
- CollectParameters(1). UpdateCompareFlag(`f(obj(SP0))`) where 0x46A2F0 (.HOODLUM 0x1569D20, thiscall) returns
  `(flags & 0x200 /*bRenderDamaged*/) || !(flags & 0x80 /*bIsVisible*/)` on dword [e+0x1C]. Proposed name
  `CObject::HasBeenDamaged` (unnamed in the catalogue). No null check. Return 0.

### 0x481435 0323 COMMAND_ANCHOR_BOAT  (gr: unregistered, used: 0/28/0)
- CollectParameters(2). `b = veh(SP0)` (no null or type check). If `SP1 != 0`: `[b+0x5AC] |= 4` else `&= 0xFB` (CBoat m_nBoatFlags
  bit 2 bAnchored, 0x48145D/0x48146B). Return 0.

### 0x481479 0325 COMMAND_START_CAR_FIRE  (gr: unregistered, used: 0/7/0)
- CollectParameters(1). `v = veh(SP0)` (no null check); `pos` = copy of v's position. `SP0 = gFireManager.StartScriptFire(pos, v,
  0.8f /*imm 0x3F4CCCCD*/, 1, 0, 1)` (0x53A270, thiscall ECX = 0xB71F80, 0x4814A5..0x4814CC), StoreParameters(1). Return 0.

### 0x481808 0335 COMMAND_SET_FREE_RESPRAYS  (gr: unregistered, used: 0/3/0)
- CollectParameters(1). `CGarages::RespraysAreFree [0x96C009] = (SP0 != 0)`. Return 0.

### 0x481880 0338 COMMAND_SET_CAR_VISIBLE  (gr: unregistered, used: 0/30/0)
- CollectParameters(2). `v = veh(SP0)` (no null check); `[v+0x1C] = SP1 ? flags | 0x80 : flags & ~0x80` (bIsVisible; stored by the
  shared `mov [eax+0x1C],ecx` at 0x4835AE). Return 0.
