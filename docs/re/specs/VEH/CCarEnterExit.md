# CCarEnterExit (VEH) - full specs (writer group E)

All functions are static (`__cdecl`, caller cleans). Offsets used throughout:
- CVehicle: +0x14 `m_matrix` (CMatrix*, null -> use placement), +0x04 placement pos, +0x10 placement heading,
  +0x22 `m_nModelIndex` (i16), +0x384 `m_pHandlingData`, +0x460 `m_pDriver`, +0x464 `m_apPassengers[8]`,
  +0x486 `m_nGettingInFlags` (u8), +0x488 `m_nMaxPassengers` (u8), +0x490 `m_pFire`, +0x590 `m_nVehicleType`
  (gr `m_nVehicleType`, plugin-sdk `m_nVehicleClass`), +0x594 `m_nVehicleSubType`, +0xFC `m_pAttachedTo` (CPhysical).
  CAutomobile +0x66C / +0x674 = `m_aCarNodes[9]` (CAR_DOOR_RR) / `[11]` (CAR_DOOR_LR).
- tHandlingData: +0xCC `m_nModelFlags` (bit 3 = 0x8 IS_BIG, bit 9 = 0x200 TANDEM_SEATS, bit 15 = 0x8000
  FORCE_DOOR_CHECK), +0xD4 `m_fSeatOffsetDistance` (f32), +0xDE `m_nAnimGroup` (u8; AssocGroupId = 88 + value).
- CVehicleModelInfo (from `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8): +0x3C `m_nVehicleType` (5 = boat),
  +0x4C `m_nNumDoors` (i8), +0x5C `m_pVehicleStruct`; vehicle struct `m_avDummyPos` at +0: dummy 0 at +0x00,
  dummy 4 (PED_FRONTSEAT) at +0x30, dummy 5 (PED_BACKSEAT) at +0x3C. "Front seat dummy" below means
  `boat (mi+0x3C == 5) ? dummy0 : dummy4`; "back seat dummy" is always dummy 5.
- Matrix: right +0x00, forward +0x10, up +0x20, pos +0x30.
- Door ids are car node ids: 8 = RF, 9 = RR, 10 = LF, 11 = LR, 18 = bike/quad "driver" side.
- `Multiply3x3` 0x59C790 = cdecl `CVector* (CVector* out, const CMatrix* m, const CVector* v)` (rotation only);
  `CMatrix::operator*` 0x59C890 = cdecl `CVector* (CVector* out, const CMatrix* m, const CVector* v)` (rotation +
  translation). Both return `out` in EAX and are pure.
- `CVehicleAnimGroup::ComputeAnimDoorOffsets` 0x6E3D10: thiscall on `&CVehicleAnimGroupData::m_vehicleAnimGroups
  [m_nAnimGroup]` (0xC1CDC0 + group*0x94), args `(CVector* out /*hidden*/, int32 doorOffsetIndex)`, callee-cleaned.

## 0x64E740 CCarEnterExit::GetPositionToOpenCarDoor  (catalogue: specified/REA, port level 6)
- Convention: `__cdecl CVector* (CVector* ret [esp+4], const CVehicle* vehicle [esp+8], int32 doorId [esp+0xC])`,
  RVO hidden pointer, returns `ret` in EAX (0x64EC74), plain `ret`. Locals 0x30. The `vehicle` argument slot is
  reused as a float local ("xOff").
- Callers (13): CPed::PositionPedOutOfCollision, CTaskComplexEnterCar / CTaskComplexShuffleSeats
  ::CreateTaskUtilityLineUpPedWithCar, CTaskSimpleCarAlign, CTaskComplexGoToCarDoorAndStandStill (x3),
  IsRoomForPedToLeaveCar, GetNearestCarPassengerDoor (6), GetNearestCarDoor (3).
- Callees: ComputeAnimDoorOffsets 0x6E3D10 (x3), Multiply3x3 0x59C790 (x2), CMatrix::operator* 0x59C890 (x2),
  CBike::GetCorrectedWorldDoorPosition 0x6BF230, virtual `vehicle->GetHeightAboveRoad()` (vtable +0xD4, slot 53,
  result in ST0).
- Constants: 0x858EF0 = 0.95f, 0x858C1C = -1.0f.
- Behaviour. `mi = ms_modelInfoPtrs[vehicle->m_nModelIndex]`, `h = vehicle->m_pHandlingData`.
  **A. Car path** (taken unless `vehicle->m_nVehicleType == 9` or `h->m_nModelFlags & 0x200`, 0x64E756-0x64E775):
  1. `xOff = (h->m_nAnimGroup == 0x65 && (doorId == 11 || doorId == 9)) ? 0.0f : h->m_fSeatOffsetDistance`
     (0x64E77B-0x64E7A2). NOTE: the raw byte is compared with 0x65 (101 = ANIM_GROUP_VANCARANIMS as an
     AssocGroupId); `m_nAnimGroup` holds 0..29, so this test is never true with valid data (original bug: the
     group base 88 is not subtracted). Keep it.
  2. Offset index: doorId 10 or 8 -> 0; 11 or 9 -> 1; 18 -> 2; any other doorId -> the raw 32-bit pattern of
     `xOff` (0x64E7D1, `mov eax,[esp+0x48]`; original bug, only reachable with an invalid door id).
     `C = ComputeAnimDoorOffsets(group h->m_nAnimGroup, index)`.
  3. Seat point A (copied as raw dwords) and x adjustment, switch on doorId (table 0x64EC7C):
     - 10 (LF, 0x64E81E): A = front seat dummy; `A.x = -(A.x + xOff)` (fld, fadd, fchs); `t = C.x`.
     - 8 (RF, 0x64E851): A = front seat dummy; `A.x = A.x + xOff`; `t = -C.x`.
     - 11 (LR, 0x64E873): A = back seat dummy; `A.x = -(A.x + xOff)`; `t = C.x`.
     - 9 (RR, 0x64E89D): A = back seat dummy; `A.x = A.x + xOff`; `t = -C.x`.
     - default (incl. 18, 0x64E8CB): A = front seat dummy; C = (0,0,0); `t = C.x` (= 0).
  4. `B.x = A.x - t; B.y = A.y - C.y; B.z = A.z - C.z` (0x64E91F-0x64E960; each stored to f32).
  5. If `vehicle->m_nVehicleSubType == 1` (MTRUCK), or (`h->m_nModelFlags & 0x8` and subtype != 4 (PLANE)):
     `B.z = 0.95f - vehicle->GetHeightAboveRoad()` (`fsubr [0x858EF0]`, 0x64E97A-0x64E98A).
  6. `B = Multiply3x3(vehicle->m_matrix, B)` (no null check on m_matrix, 0x64E99C). Then
     `B += vehicle position` (matrix pos if m_matrix else placement pos; x, y, z in that order, 0x64E9CB-0x64EA04).
  7. A second `Multiply3x3(&tmp, m_matrix, &A)` is called (0x64EA08) and its result is discarded (dead call; no
     side effects). Return B.
  **B. Bike / tandem path** (0x64EA16):
  - doorId == 18: `D = ComputeAnimDoorOffsets(group, 2)`; A = front seat dummy;
    `B = (A.x + (-D.x), D.y + A.y, A.z + (-D.z))` (0x64EA50-0x64EAC7; note y is ADDED, x and z subtracted);
    return `CMatrix::operator*(m_matrix, B)` (full transform, 0x64EAD8; no null check).
  - else: `D = ComputeAnimDoorOffsets(group, 0)`, `C = D`. A = back seat dummy if doorId is 11 or 9, else front seat
    dummy (0x64EB14-0x64EB6C).
    - If `vehicle->m_nVehicleType == 9` (bike): if doorId is 9 or 8, `C.x = D.x * -1.0f` (0x64EB7C). Return
      `CBike::GetCorrectedWorldDoorPosition(vehicle /*ECX*/, &ret, C /*by value*/, A /*by value*/)` (0x64EBB3;
      thiscall with hidden return pointer and two 12-byte by-value vectors pushed: first pushed = A (last param),
      second = C; callee cleans 0x1C bytes).
    - Else (tandem car): `s = h->m_fSeatOffsetDistance`; if doorId is 9 or 8: `C.x = D.x * -1.0f; A.x = A.x + s`,
      else `A.x = A.x - s` (0x64EBC1-0x64EBF3). `B = A - C` (x, y, z) and return `CMatrix::operator*(m_matrix, B)`.
  - The result is copied into `*ret` (0x64EC5B-0x64EC71).
- Edge cases: m_matrix is dereferenced without a null check in every path. Door id 18 on cars takes the
  default case (front seat dummy, zero anim offset). Unknown door ids on cars index the anim-offset table with
  float bits (see A.2) - ComputeAnimDoorOffsets then reads out of range; a port should reproduce by passing the
  same integer.
- gta-reversed: `plugin::CallAndReturn` wrapper only (CarEnterExit.cpp:387, unreversed).
- Open questions: none affecting behaviour for valid door ids.

## 0x64F240 CCarEnterExit::IsPlayerToQuitCarEnter  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl bool (const CPed* ped [esp+4], const CVehicle* vehicle [esp+8], int32 startTime [esp+0xC],
  CTask* task [esp+0x10])`, result AL, plain `ret`. The `ped` argument slot is reused for the float `angle`,
  `vehicle` and `task` slots for temporaries.
- Callers: CTaskComplexEnterCar::ControlSubTask (2 sites), CTaskComplexGoToBoatSteeringWheel::ControlSubTask,
  CTaskComplexWalkRoundCar::ControlSubTask.
- Callees: CPlayerPed::GetPadFromPlayer 0x609560 (thiscall ped -> CPad*), `task->GetTaskType()` (vtable +0x10),
  CPad::MeleeAttackJustDown 0x540390 (thiscall, arg 0), CPad::GetPedWalkUpDown 0x53FD30 / GetPedWalkLeftRight
  0x53FC90 (thiscall -> AX int16), CGeneral::GetRadianAngleBetweenPoints 0x53CBE0 (cdecl 4 floats -> ST0).
- Globals: `CTimer::m_snTimeInMilliseconds` 0xB7CB84, `TheCamera.m_fOrientation` 0xB6F178 (f32),
  0x8D2ED8 (f32 `.data`, image value 500.0f; gr labels the neighbour 0x8D2ED4 = 30.f). Constants: 0x858B50 = 0.0f,
  0x858FE4 = 1.5707964f, 0x858CB8 = 3.1415927f, 0x858CBC = 6.2831855f, 0x858CC0 = -3.1415927f, 0x858C54 = 4294967296.0f,
  0x858B88 = 0.0078125f, 0x858F34 = 0.75f, 0x859AB0 = 0.7853982f.
- Fields: ped +0x558 `m_fCurrentRotation`, +0x14/+0x04 position; vehicle +0x14 matrix, +0x10 heading,
  +0x490 m_pFire; CPad +0x10E `DisablePlayerControls` (u16).
- Behaviour:
  1. `pad = ped->GetPadFromPlayer()`; `angle = ped->m_fCurrentRotation`; `jack = false`.
  2. If `task != null`: `type = task->GetTaskType()` and classify (0x64F275-0x64F2F1, jump table 0x64F51C/0x64F528):
     - "compute, jack = false": 203, 208, 801, 805, 807, 808, 812, 816, 829
       (gr names: STAND_STILL, FALL_AND_GET_UP, CAR_ALIGN, CAR_CLOSE_DOOR_FROM_INSIDE, CAR_GET_IN, CAR_SHUFFLE,
       CAR_SET_PED_IN_AS_DRIVER, CAR_SET_PED_OUT, WAIT_UNTIL_PED_OUT_CAR);
     - "compute, jack = true": 704, 802, 803, 804, 817, 820 (COMPLEX_LEAVE_CAR, CAR_OPEN_DOOR_FROM_OUTSIDE,
       CAR_OPEN_LOCKED_DOOR_FROM_OUTSIDE, BIKE_PICK_UP, CAR_QUICK_DRAG_PED_OUT, CAR_SLOW_DRAG_PED_OUT);
     - any other type: skip to step 3 (angle stays the ped rotation).
     "compute":
     a. `d = (ped.pos - veh.pos)`; `dot = d.x*R.x + (d.y*R.y + d.z*R.z)` with R = `vehicle->m_matrix->right`
        (x87 order: z-term, + y-term, + x-term; 0x64F2FB-0x64F319). m_matrix is dereferenced without null check
        (the vehicle position itself does handle a null matrix).
     b. `hdg = m_matrix ? atan2(-forward.x, forward.y) /*fpatan*/ : placement heading`;
        `angle = (dot > 0) ? hdg + 1.5707964f : hdg - 1.5707964f` (NaN or <= 0 -> minus; stored f32).
     c. If `m_matrix->up.z < 0` (again no null check): `angle += pi`; if `angle > pi`, `angle -= 2pi`.
     d. Normalise once: if `angle > pi`: `angle -= 2pi`; else if `angle < -pi`: `angle += 2pi`
        (0x64F39F-0x64F3D7).
  3. If `vehicle->m_pFire != null` -> return true (0x64F3DB).
  4. If `pad->DisablePlayerControls != 0` -> return false.
  5. If `jack`: if `pad->MeleeAttackJustDown(0)` -> return true; else go to 7 (no time check).
  6. Else: `elapsed = (uint32)(now - startTime)` converted to float (fild + 2^32 fix-up when negative);
     if `elapsed <= [0x8D2ED8]` (or NaN) -> return false.
  7. `ud = (float)GetPedWalkUpDown()`, `lr = (float)GetPedWalkLeftRight()` (both int16, sign-extended).
     `a = GetRadianAngleBetweenPoints(0.0f, 0.0f, -lr, ud) - TheCamera.m_fOrientation`.
     `m = sqrt(ud*ud + lr*lr) * 0.0078125f` (ud*ud computed first, 0x64F489-0x64F4A0).
     If `a > angle + pi`: `a -= 2pi`; else if `a < angle - pi`: `a += 2pi` (0x64F4A6-0x64F4E2).
     If `m <= 0.75f` (or NaN) -> return false.
     Return `fabs(a - angle) > 0.7853982f` (NaN -> false).
- Edge cases: the angle normalisation in 2c/2d is one step only. The elapsed-time test treats time as unsigned.
- gta-reversed: `plugin::CallAndReturn` only (CarEnterExit.cpp:508).
- Open questions: task type names come from gr `eTaskType.h` enumeration order (numeric values above are exact).

## 0x64F540 CCarEnterExit::MakeUndraggedPassengerPedsLeaveCar  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl void (const CVehicle* vehicle [esp+4], const CPed* draggedPed [esp+8], const CPed* ped
  [esp+0xC])`. SEH frame for the temporary event.
- Callers: none recorded (gr declares it; possibly unused).
- Callees: ComputeTargetDoorToExit 0x64F110 (cdecl (vehicle, ped) -> int32), 0x4AEAC0 (unnamed in the catalogue =
  `CEventPedEnteredMyVehicle::CEventPedEnteredMyVehicle(CPed*, CVehicle*, int32 door)`, thiscall, ret 0xC; vtable
  0x85AFE8, size 0x20, fields +0x14 ped, +0x18 vehicle, +0x1C door; registers references on both),
  CEventGroup::Add 0x4AB420 (thiscall on `passenger->m_pIntelligence + 0x68`, args (CEvent*, false)), 0x4AEB90
  (the event's destructor).
- Behaviour: for `i = 0; i < vehicle->m_nMaxPassengers; i++` (u8 re-read every iteration, signed int compare):
  `p = vehicle->m_apPassengers[i]`; skip if `p == null`, `p == draggedPed`, or `p` flag dword +0x470 bit 0x00800000
  is set (CPed flag #55, gr `bStayInCarOnJack`). Otherwise: `door = ComputeTargetDoorToExit(vehicle, p)`; build
  `CEventPedEnteredMyVehicle(ped, vehicle, door)` on the stack; `p->m_pIntelligence->m_eventGroup.Add(&ev, false)`;
  destroy `ev`.
- gta-reversed: `plugin::Call` only (CarEnterExit.cpp:603). Name note: the event constructor 0x4AEAC0 and
  destructor 0x4AEB90 are unnamed in the catalogue; the vtable/size/field layout matches gr's
  `CEventPedEnteredMyVehicle` (Events/EventPedEnteredMyVehicle.h, size 0x20).

## 0x6504C0 CCarEnterExit::IsRoomForPedToLeaveCar  (catalogue: specified/REA, port level 8)
- Convention: `__cdecl bool (const CVehicle* vehicle [esp+4], int32 doorId [esp+8], const CVector* offset
  [esp+0xC])` (offset may be null), result AL, plain `ret`. Locals 0x88. Offsets below are relative to entry ESP (E).
- Callers: CTaskComplexKillPedOnFoot (2 fns), CTaskComplexLeaveCar::CreateNextSubTask,
  CTaskSimpleCarSetPedSlowDraggedOut::ProcessPed, GetNearestCarPassengerDoor (6), GetNearestCarDoor (10),
  CTaskComplexArrestPed::ControlSubTask.
- Callees: CMatrix::operator* 0x59C890, GetPositionToOpenCarDoor 0x64E740, Multiply3x3 0x59C790,
  CWorld::GetIsLineOfSightClear 0x56A490, CWorld::TestSphereAgainstWorld 0x569E20, CWorld::ProcessVerticalLine
  0x5674E0 (x2).
- Constants: 0x858B50 = 0.0f, 0x858B8C = 0.5f, 0x858CC4 = 0.2f, 0x858F9C = 0.35f, 0x858CC8 = 0.6f, 0x858624 = 1.0f;
  immediates 0.35f (0x3EB33333), 1000.0f, -1000.0f.
- Behaviour:
  0. Locals: `losEnd` (E-0x60) = (0,0,0); a CColPoint at E-0x2C whose point.x/.y/.z are zeroed (E-0x2C, E-0x28,
     E-0x24); `hitEntity` (E-0x48) = null. `mi = ms_modelInfoPtrs[model]`.
  1. Seat point S (raw dword copy):
     - Car (type != 9 and no TANDEM flag): doorId 8 -> front seat dummy; 9 -> back seat dummy; 10 -> front seat
       dummy with `S.x = -S.x`; 11 -> back seat dummy with `S.x = -S.x`; any other doorId -> return false
       (0x650544 -> 0x650947).
     - Bike/tandem: doorId 11 or 9 -> back seat dummy, else front seat dummy; then `S.x = -S.x` if doorId is 10 or 11.
  2. `W = CMatrix::operator*(m_matrix, S)` (world seat point). `D = GetPositionToOpenCarDoor(vehicle, doorId)`.
  3. If `offset != null`: `P = *offset`; if doorId is 8 or 9: `P.x = -P.x`; `R = Multiply3x3(m_matrix, P)`;
     `D.x = R.x + D.x; D.y = R.y + D.y; D.z = R.z + D.z`.
  4. If `m_matrix->up.z < 0` (upside down): `W.z += 0.5f; D.z += 0.5f`.
  5. `dx = D.x - W.x` (stored f32), `dy = D.y - W.y` (kept in ST0).
     - Bike (type == 9, 0x65074D): `L = (D.x, D.y, D.z + 0.2f)` and `D.z = (D.z + 0.2f) + 0.35f` (D.z is modified;
       two f32 roundings).
     - Else (0x650781): `len = sqrt(dy*dy + dx*dx)`; `k = (0.35f + len) / len` (fdivrp); `L.x = W.x + dx*k`,
       `L.y = W.y + dy*k`; a value `W.z + k*0.0f` is computed and thrown away; `L.z = D.z` (0x6507EB).
       If `len == 0`, k = +inf and `0*inf` = NaN in L.x/L.y.
  6. If `!CWorld::GetIsLineOfSightClear(W, L, buildings=1, vehicles=0, peds=0, objects=1, dummies=0,
     seeThrough=0, ignoreSomeObjects=0)` -> return false.
  7. `hit = CWorld::TestSphereAgainstWorld(D /*by value*/, 0.35f, vehicle /*ignore*/, buildings = (type != 6 /*train*/),
     vehicles=1, peds=0, objects=1, dummies=0, cameraIgnore=0)`. If `hit != null` and not
     (`hit->m_nModelIndex == 608 && vehicle->m_nModelIndex == 577`) and `hit != vehicle->m_pAttachedTo` -> return false.
     (The `buildings` flag is pushed as the whole dword at E-0x70 whose low byte was just set; only the bool matters.)
  8. `ProcessVerticalLine(D, 1000.0f, colPoint, hitEntity, buildings=1, vehicles=0, peds=0, objects=1,
     dummies=0, seeThrough=0, storedPoly=null)` (upward). `ceilZ = colPoint.point.z` (read whether or not it hit).
     If it hit and `ceilZ > D.z` and `D.z + 0.6f > ceilZ` -> return false.
  9. Ground: if `type == 5` (boat) or model is 460, 539, 447 or 417 (0x1CC, 0x21B, 0x1BF, 0x1A1): `g = colPoint.point.z
     - 1.0f` (current colPoint z, i.e. the upward result). Else: `ProcessVerticalLine(D, -1000.0f, ...same flags)`;
     if it did not hit -> return false; `g = colPoint.point.z`.
  10. If `ceilZ == 0.0f` -> return true. Else return `ceilZ >= g` (NaN -> false) (0x65095C-0x65097A).
- Edge cases: m_matrix is never null-checked. The "no ceiling" test is `ceilZ == 0.0f`, relying on the zeroed
  colPoint when the upward line misses (or a ceiling exactly at z = 0). For boats/the four sea planes step 10
  is effectively always true (`ceilZ >= ceilZ - 1`).
- gta-reversed: `plugin::CallAndReturn` only (CarEnterExit.cpp:513).
- Open questions: whether ProcessVerticalLine 0x5674E0 writes colPoint on a miss (affects `ceilZ` in steps 8-10)
  - must be taken from that function's spec.

## 0x650BB0 CCarEnterExit::GetNearestCarPassengerDoor  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl bool (const CPed* ped [esp+4], const CVehicle* vehicle [esp+8], CVector* outPos [esp+0xC],
  int32* outDoor [esp+0x10], bool checkOccupied [esp+0x14], bool checkEnterable [esp+0x18], bool checkRoom
  [esp+0x1C])`, result AL (= `found`, kept in BL). Locals 0x48. Code oddity: the function jumps out to 0x401896
  (`mov ecx,[esi+0x384]; jmp 0x650BBF`) and back - a split fragment, no behavioural effect.
- Callers: CTaskComplexGoToCarDoorAndStandStill::ControlSubTask, ::CreateFirstSubTask.
- Callees: IsRoomForPedToLeaveCar 0x6504C0 (always with offset = null), GetPositionToOpenCarDoor 0x64E740.
- Locals: direction vectors d8, d11, d9 (2D, each initialised to (999.0f, 999.0f)); positions P8 (E-0x18), P11
  (E-0x30), P9 (E-0x24). `pedPos` = ped matrix pos or placement pos. "try door N" below means: `if (checkRoom &&
  !IsRoomForPedToLeaveCar(vehicle, N, null)) skip; PN = GetPositionToOpenCarDoor(vehicle, N); dN = (PN.x - pedPos.x,
  PN.y - pedPos.y); found = true`.
- Behaviour:
  1. Coach/bus (`h->m_nAnimGroup + 88` is 103 or 104, 0x650BC6-0x650C07): if `checkEnterable && (gettingIn & 4)`
     -> return false; if `checkRoom && !IsRoom(vehicle, 8, null)` -> return false;
     `*outPos = GetPositionToOpenCarDoor(vehicle, 8)`, `*outDoor = 8`, return true.
  2. Car (type != 9 and no TANDEM flag): door 8: skip if `checkOccupied && m_apPassengers[0]`, or
     `checkEnterable && (gettingIn & 4)`; else try door 8.
  3. Bike/tandem: if `checkOccupied && m_apPassengers[0]` -> skip both doors 11 and 9 (go to 4). Door 11: skip if
     `checkEnterable && (gettingIn & 2)`; else try door 11. Door 9: skip if `checkEnterable && (gettingIn & 8)`;
     else try door 9. NOTE: in this path GetPositionToOpenCarDoor writes its result into the P8 slot (E-0x18)
     before copying it to P11/P9, so P8 ends up holding the last door position computed here (or stays
     uninitialised).
  4. If `mi->m_nNumDoors > 2` (signed):
     - Door 11: skip if `(h->m_nModelFlags & 0x8000) && !(type == 0 && CAutomobile::m_aCarNodes[11] (+0x674))`, or
       `checkOccupied && m_apPassengers[1]`, or `checkEnterable && (gettingIn & 2)`; else try door 11.
     - Door 9: skip if `(flags & 0x8000) && !(type == 0 && m_aCarNodes[9] (+0x66C))`, or `checkOccupied &&
       m_apPassengers[2]`, or `checkEnterable && (gettingIn & 8)`; else try door 9.
     (These may overwrite d11/P11, d9/P9 computed in step 3.)
  5. Selection (always executed, 0x650F92): `*outPos = P8; *outDoor = 8`.
     `s8 = d8.y^2 + d8.x^2`, `s11 = d11.y^2 + d11.x^2`; if `s8 > s11` (strict; NaN keeps 8): `*outDoor = 11,
     *outPos = P11`, best = d11 else best = d8. Then `s9 = d9.y^2 + d9.x^2`, `sb = best.x^2 + best.y^2`; if
     `sb > s9` (strict): `*outDoor = 9, *outPos = P9`. Squares are computed in x87 registers (not stored).
  6. Return `found`.
- Edge cases: outputs are written even when returning false (door 8 and P8, which can be uninitialised or a stale
  bike-path value). Ties keep the earlier door (8, then 11). Untried doors keep distance (999, 999).
- gta-reversed: `plugin::CallAndReturn` only (CarEnterExit.cpp:381).
- Open questions: none.
