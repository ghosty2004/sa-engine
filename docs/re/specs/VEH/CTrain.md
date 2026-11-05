# CTrain (VEH) - full specs (writer group E)

Layout (gr `Entity/Vehicle/Train.h`, plugin-sdk `CTrain.h` offsets; size 0x6AC): +0x5A0 `m_nNodeIndex` i16,
+0x5A4 `m_fTrainSpeed`, +0x5A8 `m_fCurrentRailDistance`, +0x5AC `m_fLength`, +0x5B0 `m_fTrainGas`, +0x5B4
`m_fTrainBrake`, +0x5B8 `m_nTrainFlags` u16 (bit0 b01, bit1 bStoppedAtStation, bit2 bPassengersCanEnterAndLeave,
bit3 0x08 bIsFrontCarriage, bit4 0x10 bIsLastCarriage, bit5 0x20 bMissionTrain, bit6 0x40 bClockwiseDirection,
bit7 0x80 bStopsAtStations, bit8 bNotOnARailRoad, bit9 bForceSlowDown, bit10 bIsStreakModel), +0x5BC
`m_nTimeWhenStoppedAtStation`, +0x5C0 `m_nTrackId` (byte), +0x5C4 `m_nTimeWhenCreated`, +0x5C8 field_5C8 (u16),
+0x5CA `m_nPassengersGenerationState` u8, +0x5CB nibbles (low = gr `m_nNumPassengersToLeave`, high = gr
`m_nNumPassengersToEnter`), +0x5CC `m_pTemporaryPassenger`, +0x5D0 `m_pPrevCarriage`, +0x5D4 `m_pNextCarriage`,
+0x5D8 `m_aDoors[6]` (CDoor, stride 0x18: +0 openAngle, +4 closedAngle, +8 dirn u16, +0xA axis u8, +0xB state,
+0xC angle), +0x668 `m_aTrainNodes[17]` (RwFrame*).

Globals: `pTrackNodes[4]` 0xC38024 (CTrainNode*), `NumTrackNodes[4]` 0xC38014 (i32), `arrTotalTrackLength[4]`
0xC37FEC (f32), `StationDist[6]` 0xC38034 (f32), `CTrain::aStationCoors[6]` 0x8D48F8 (CVector), train configs
0x8D44F8 (16 types x 16 int32 model ids, 0-terminated; see CreateMissionTrain), GenTrain_* 0xC37FFC..0xC3800C,
`bDisableRandomTrains` 0xC38010, 0xC38064 (i32 config counter), 0xC38068 (u8 "player near a station").
CTrainNode: see CTrainNode.md (positions are int16 * 0.125f, distance u16 * (1/3)).
Common constants: 0x858C48 = 0.125f, 0x859040 = 0.33333334f (an f32 multiply, never a division by 3),
0x858B50 = 0.0f.

Virtual slot names (CTrain vtable 0x872370, same slot order as CVehicle 0x871E80, names from gr Vehicle.h):
vfunc_0 = deleting destructor, vfunc_14 = Teleport, vfunc_17 = PreRender, vfunc_18 = Render, vfunc_27 = OpenDoor,
vfunc_29 = GetDooorAngleOpenRatioU32, vfunc_30 = GetDooorAngleOpenRatio(eDoors), vfunc_45 = IsRoomForPedToLeaveCar.

## 0x6F54D0 CTrain::~CTrain (catalogue: unnamed, proposed name)  (catalogue: unknown/REA, port level 15)
- Name evidence: only caller is the deleting destructor CTrain::vfunc_0 0x6F6300; stores the CTrain vtable
  0x872370 and ends in CVehicle::~CVehicle.
- Convention: `__thiscall void (CTrain* this = ECX)`, plain `ret`. SEH frame (state 0 during the audio call).
- Behaviour: `this->vtable = 0x872370`; `m_vehicleAudio (this+0x138).Terminate()` (CAEVehicleAudioEntity::Terminate
  0x4FB8C0); `CVehicle::~CVehicle(this)` (0x6E2B40). Nothing else (no carriage unlinking).

## 0x6F55C0 CTrain::vfunc_18 = Render  (catalogue: specified/REA, port level 7)
- Tail jump to CVehicle::Render 0x6D0E60 (ECX unchanged). Equivalent to not overriding.

## 0x6F5570 CTrain::vfunc_17 = PreRender  (catalogue: specified/REA, port level 14)
- Convention: thiscall, no args, plain `ret`.
- Behaviour: `CVehicle::PreRender()` (0x6D6480, non-virtual). If `flags & 0x08` (front carriage):
  `DoVehicleLights(*m_matrix, model == 449 (0x1C1, tram) ? 5 : 0x25)`; else if `flags & 0x10` (last carriage):
  `DoVehicleLights(*m_matrix, 0x15)`. DoVehicleLights 0x6E1A60 is thiscall `(CMatrix&, uint32 flags)`, ret 8;
  the matrix pointer is passed without a null check.

## 0x6F59C0 CTrain::vfunc_30 = GetDooorAngleOpenRatio(eDoors)  (catalogue: specified/REA, port level 1)
- Convention: thiscall `(int32 door [esp+4])`, returns ST0, `ret 4`.
- Behaviour: tail-like call `CDoor::GetAngleOpenRatio(&m_aDoors[door])` (0x6F47E0, ECX = this + 0x5D8 + door*0x18);
  no bounds check.

## 0x6F5930 CTrain::vfunc_29 = GetDooorAngleOpenRatioU32  (catalogue: specified/REA, port level 0)
- Convention: thiscall `(uint32 nodeId [esp+4])`, returns ST0, `ret 4`.
- Behaviour: `fld 0.0f` first. Switch on `nodeId - 8` (unsigned, <= 10; tables 0x6F5998 dwords / 0x6F59B0 bytes):
  8 -> 3, 9 -> 5, 10 -> 2, 11 -> 4: pop the 0.0, overwrite the argument slot with that eDoors value and
  `jmp [vtable + 0x78]` (slot 30, i.e. a tail call to GetDooorAngleOpenRatio(eDoors) with the same return
  address). 18 -> pop and reload 0.0f, return. Any other value -> return 0.0f.

## 0x6F59E0 CTrain::FindCoorsFromPositionOnTrack  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (float railDistance [esp+4], int32 trackId [esp+8], CVector* out [esp+0xC])`.
  The trackId slot is reused as a float temporary. No callers recorded (script / unreferenced).
- Behaviour: `n = NumTrackNodes[trackId]`, `nodes = pTrackNodes[trackId]`; if `n <= 0` return (out untouched).
  For `i = 0 .. n-1`: `next = (i + 1) % n` (signed idiv);
  `a = railDistance - (float)nodes[i].dist * 0.33333334f` (fild of the zero-extended u16, fmul, fsubr; stored f32);
  `b = (float)nodes[next].dist * 0.33333334f - railDistance` (stored f32);
  if `a < 0` or `b < 0` (NaN counts as < 0) continue. Else (0x6F5A78):
  `X = Pi.x*b + Pn.x*a`, `Y = Pi.y*b + Pn.y*a`, `Z = Pi.z*b + Pn.z*a` where P = node position (int16 * 0.125f);
  the products `Pn.x*a`, `Pi.x*b`, `Pi.z*b` and the sums X, Y are stored to f32 temporaries, the others stay in
  x87 registers; `inv = 1.0f / (b + a)` (`fdivr [0x858624]`); `out = (X*inv, Y*inv, Z*inv)`; return.
- Edge cases: for the last node, next = 0 whose distance is 0, so `b = -railDistance` and the segment matches only
  for `railDistance <= 0`; positions beyond the last node are not found (out untouched).

## 0x6F5BA0 CTrain::FindMaximumSpeedToStopAtStations  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall bool (CTrain* this, float* outSpeed [esp+4])`, AL, `ret 4`.
- Callers: DoTrainGenerationAndRemoval 0x6F7900, ProcessControl 0x6F86A0.
- Constants: 0x858B8C = 0.5f, 0x858F40 = -0.5f, 0x858A10 = 40.0f, 0x858B58 = 500.0f, 0x858F44 = 0.002f,
  0x858624 = 1.0f, 0x858B40 = 50.0f, 0x858C80 = 5.0f.
- Behaviour:
  1. `*outSpeed = 50.0f`; `minDist = 10000.0f`. If `m_nTrackId != 0` return false.
  2. `hi = arrTotalTrackLength[0] * 0.5f`, `lo = arrTotalTrackLength[0] * -0.5f` (kept on the x87 stack);
     `cw = (flags >> 6) & 1`.
  3. For each station `s` (0..5, pointer loop to 0xC3804C): `d = StationDist[s] - m_fCurrentRailDistance`;
     `d = cw ? d + 40.0f : d - 40.0f`; `while (d > hi) d -= total;` `while (d < lo) d += total;` (total re-read from
     memory each time; NaN exits both loops). If cw: candidate only if `d > 0`; else candidate only if `d < 0`,
     then `d = -d`. If candidate and `d < minDist`: `minDist = d`.
  4. If `minDist < 500.0f`: `*outSpeed = (1.0f - (500.0f - minDist) * 0.002f) * 50.0f` (order: fld 500, fsub,
     fmul, fsubr 1.0, fmul 50); else `*outSpeed = 100000.0f`.
  5. Return `minDist < 5.0f`.

## 0x6F5CF0 CTrain::TrainHitStuff<CPtrListSingleLink<CPhysical*>>  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl void (CPtrListSingleLink* list [esp+4], CEntity* train [esp+8])`. A fragment lives at
  0x406E85 (`or eax, 0x800; jmp 0x6F5D15`), split code only.
- Callers: MarkSurroundingEntitiesForCollisionWithTrain 0x6F6640 (3 sites).
- Behaviour: for each node (`next` read before processing): `e = node->item`; skip if `e == train`.
  `e->m_nPhysicalFlags (+0x40) |= 0x800` (bProcessCollisionEvenIfStationary) - done for EVERY entity of the list
  except the train, before the type test. Then only if `(e->m_nType & 7) == 4` (object) and
  (`e->m_nFlags (+0x1C) & 0x4` (m_bIsStatic) or `& 0x40000` (m_bIsStaticWaitingForCollision)) and the model
  (sign-extended i16) equals MI_OBJECTFORMAGNOCRANE1/2/3 (u16 globals 0x8CD6C8/CC/D0, zero-extended):
  `e->SetIsStatic(false)` (vtable +0x10), `e->AddToMovingList()` (0x542800), `e->m_nFakePhysics (+0xB8) = 0`.
- gta-reversed: absent (`.Reversed = false`, raw call).

## 0x6F5E70 CTrain::FindCaboose  (catalogue: specified/REA, port level 0)
- `__cdecl CTrain* (CTrain* train)`: follow `m_pNextCarriage` (+0x5D4) until null; return the last carriage
  (returns `train` itself if it has no next). No null check on `train`. Clobbers ECX.

## 0x6F5E90 CTrain::FindEngine  (catalogue: specified/REA, port level 0)
- `__cdecl CTrain* (CTrain* train)`: same as FindCaboose but following `m_pPrevCarriage` (+0x5D0).

## 0x6F5EB0 CTrain::FindCarriage  (catalogue: specified/REA, port level 0)
- `__cdecl CTrain* (CTrain* train, uint8 carriage)`: if `carriage == 0` return train. Else follow
  `m_pNextCarriage` `carriage` times (u8 counter, unsigned compare); if a null is met return null.

## 0x6F5EE0 CTrain::vfunc_45 = IsRoomForPedToLeaveCar  (catalogue: specified/REA, port level 0)
- thiscall `(uint32, CVector*)`: returns AL = 1, `ret 8`.

## 0x6F5F00 CTrain::FindNextStationPositionInDirection  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (bool clockwise [esp+4], float railDistance [esp+8], float* outStationDist [esp+0xC],
  int32* outStationIndex [esp+0x10])`. (gr names the last two `distanceToStation` / `numStations`; the bytes
  store the station's rail distance and the station index - name correction.)
- Behaviour: `i` = first station index (0..5) with `StationDist[i] > railDistance` (strict; NaN never), else 0.
  If `!clockwise`: `i -= 1; if (i < 0) i += 6`. Then if `fabs(railDistance - StationDist[i]) < 100.0f`
  (0x858628): `i = clockwise ? i + 1 : i - 1`; `if (i < 0) i += 6`; `if (i >= 6) i = 0`.
  `*outStationIndex = i; *outStationDist = StationDist[i]` (raw dword copy).

## 0x6F6030 CTrain::Constructor  (catalogue: specified/REA, port level 15)
- Convention: `__thiscall CTrain* (CTrain* this, int32 modelIndex [esp+4], eVehicleCreatedBy createdBy [esp+8])`,
  returns this, `ret 8`. SEH frame (unwind -> CVehicle::~CVehicle).
- Callers: CCarCtrl::GetNewVehicleDependingOnCarModel, CReplay::PlayBackThisFrameInterpolation, CPools::LoadVehiclePool,
  CCarGenerator::DoInternalProcessing, CreateMissionTrain.
- Behaviour (byte order):
  1. `CVehicle::CVehicle(this, createdBy)` (0x6D5F10); vtable = 0x872370; zero 0x24 dwords at +0x5D8 (all 6 doors).
  2. `mi = ms_modelInfoPtrs[modelIndex]`; `m_nVehicleSubType = m_nVehicleType = 6`;
     `m_pHandlingData = 0xC2B9DC + mi->m_nHandlingId (u8 +0x4A) * 0xE0`; `m_nHandlingFlags (+0x38C) = handling+0xD0`.
  3. `CVehicle::SetModelIndex(modelIndex)` (0x6D6A40, direct call); zero `m_aTrainNodes` (17 dwords);
     `CClumpModelInfo::FillFrameArray(m_pRwClump, m_aTrainNodes)`.
  4. Doors 2 (+0x608) and 3 (+0x620): `dirn = 0x14`, `axis = 2`; if model == 570 (0x23A, streakc): open = 1.25f,
     closed = 0.25f for both; else door 2 open = -1.2566371f (0xBFA0D97C), door 3 open = +1.2566371f, closed = 0.
  5. Flags low byte: `(b & 0xD9) | 0x81` (sets b01 and bStopsAtStations, clears bits 1, 2, 5; bits 3, 4, 6
     (front, last, clockwise) are NOT written). High byte: `(b & 0xFA) | 0x02` (sets bForceSlowDown, clears
     bNotOnARailRoad and bIsStreakModel, other bits kept).
  6. `+0x5CA = 0`; `+0x5CB low nibble = rand() & 3` (first rand); `high nibble = (rand() & 3) + 1` (second rand).
  7. `m_nPhysicalFlags |= 0x20000` (bDisableSimpleCollision); `m_pTemporaryPassenger = null`;
     `m_nMaxPassengers = 5`; `m_nFlags |= 1` (bUsesCollision); `m_nTimeWhenCreated = now`; field_5C8 = 0;
     `m_nTrackId = 0`; rail distance = 0; train speed = 0; `m_nTimeWhenStoppedAtStation = 0`.
  8. `mi->ChooseVehicleColour(&+0x434, &+0x435, &+0x436, &+0x437, 1)` (0x4C8500, ret 0x14).
  9. Physics from handling: `m_fMass = h+4`, `m_fTurnMass = h+0xC`, `m_vecCentreOfMass = h+0x14`,
     `m_fElasticity = 0.05f`, `m_fBuoyancyConstant = h+0x24`; `m_fAirResistance = (h->m_fDragMult (+0x10) > 0.01f)
     ? h->m_fDragMult / 1000.0f * 0.5f : h->m_fDragMult` (fdiv then fmul).
  10. `m_nPhysicalFlags = (f & ~0x8) | 0x4`; `m_nFlags |= 0x80000000` (m_bTunnelTransition); prev = next = null;
      status = 6 (STATUS_TRAIN_MOVING, `(b & 7) | 0x30`); `m_autoPilot.m_speed (+0x3C8) = 0`;
      `m_autoPilot.m_nCruiseSpeed (+0x3D0) = 0`; `m_vehicleAudio.Initialise(this)`.
- gta-reversed (Train.cpp:77, unreachable after the plugin call): differs from the bytes in (a) setting
  bClockwiseDirection/bIsLastCarriage/bIsFrontCarriage = true (bytes leave them), (b) the passenger nibbles
  (gr: high = first rand & 3, low = second rand & 3 + 1; bytes: low = first rand & 3, high = second rand & 3 + 1),
  (c) gr sets `bAddMovingCollisionSpeed` where the bytes set physical bit 2 and clear bit 3, (d) gr's
  `GetDefaultAirResistance()` - bytes inline the 0.01 threshold formula.

## 0x6F6300 CTrain::vfunc_0 = deleting destructor  (catalogue: specified/REA, port level 16)
- thiscall `(uint8 flags [esp+4])`, `ret 4`: `CTrain::~CTrain()` (0x6F54D0); if `flags & 1`:
  `CVehicle::operator delete(this)` (0x6E2D90, pool delete); return this.

## 0x6F6370 CTrain::vfunc_27 = OpenDoor  (catalogue: specified/REA, port level 9)
- Convention: thiscall `(CPed* ped [esp+4], int32 nodeId [esp+8], eDoors door [esp+0xC], float ratio [esp+0x10],
  bool playSound [esp+0x14])`, `ret 0x14`. SEH frame (CMatrix and event locals).
- Behaviour:
  1. `door == 2` -> node = 1; `door == 3` -> node = 2; any other door -> return. The `nodeId` argument is ignored
     (overwritten). `frame = m_aTrainNodes[node]`; if null return.
  2. `CMatrix mat(RwFrameGetMatrix(frame) /*frame+0x10*/, false)` (0x59C050); `pos0 = mat.pos`; `rot = (0,0,0)`.
  3. `d = &m_aDoors[door]`; `wasClosed = d->IsClosed()` (0x6F4800); if so
     `RwFrameForAllObjects(frame, CVehicleModelInfo::ClearAtomicFlagCB, 0x400)`.
  4. `d->Open(ratio)` (0x6F4790).
  5. If wasClosed: if `d->closedAngle != d->angle` (NaN counts as different):
     `RwFrameForAllObjects(frame, SetAtomicFlagCB, 0x400)`, and if `ped`: event
     `CEventSoundQuiet(ped, 60.0f, -1, CVector(0,0,0))` added to `GetEventGlobalGroup()` (Add(&ev, false)), then
     destroyed (0x5DEA00). If not wasClosed: if `ratio == 0.0f` and `ped`: same event with volume 120.0f.
     `playSound` is not read.
  6. If model == 570 (streakc): `y = (d->openAngle - d->closedAngle) * ratio + d->closedAngle`;
     `mat.SetTranslate(pos0.x, y, pos0.z)` (0x59AF40). Else: `rot[d->axis] = d->angle`;
     `mat.SetRotate(rot.x, rot.y, rot.z)` (0x59B120); `mat.pos += pos0` (x, y, z).
  7. `mat.UpdateRW()`; `~CMatrix`.

## 0x6F6850 CTrain::RemoveRandomPassenger  (catalogue: specified/REA, port level 28)
- Convention: thiscall, no args, plain `ret`. SEH frame for the task allocation.
- Callees: FindPlayerPed 0x56E210, GetTaskType (vtable +0x10), CleanUpOldReference 0x571A00, GetRandomNumberInRange
  0x407180, rand, CPopulation::AddPedInCar 0x613A00, RegisterReference 0x571B70, CTask::operator new 0x61A5A0,
  CTaskComplexLeaveCarAndWander ctor 0x63BDD0 (ret 0x10), CTaskManager::SetTask 0x681AF0 (ret 0xC).
- Behaviour:
  1. Return if `CReplay::Mode (0xA43088) == 1`.
  2. `pl = FindPlayerPed(-1)`; return if `pl->m_pVehicle (+0x58C) == this`.
  3. `t = pl->m_pIntelligence->m_TaskMgr.m_aPrimaryTasks[3]` (intel+0x10); if `t` and its type is 701 or 700
     (ENTER_CAR_AS_DRIVER / AS_PASSENGER) and `t+0xC (target vehicle) == this` -> return.
  4. `cw = (flags >> 6) & 1`. `pl = FindPlayerPed(-1)` again; if `pl` bInVehicle (+0x46C & 0x100) and
     `pl->m_pVehicle == this`: if cw and `m_pDriver == pl` return; if !cw and `m_pDriver != pl` return.
     (Unreachable after step 2, kept for fidelity.)
  5. Return if low nibble of +0x5CB is 0.
  6. If `m_pTemporaryPassenger`: return if it is bInVehicle; else `CleanUpOldReference(&m_pTemporaryPassenger)`
     and set it to null.
  7. `rating = GetRandomNumberInRange(0, 12)` (original formula, see CStreaming.deviations.md); `if (rating == 12)
     rating = 11`; `if ((rand() & 0xFFFF) < 100) rating = 25`.
  8. `ped = AddPedInCar(this, cw == 0, rating, 0, false, false)`; return if null.
  9. `ped flags +0x478 |= 0x40000` (flag #114 bJustGotOffTrain); low nibble of +0x5CB -= 1 (high nibble kept);
     `m_pTemporaryPassenger = ped; ped->RegisterReference(&m_pTemporaryPassenger)`.
  10. `task = new CTaskComplexLeaveCarAndWander(this, 0, 0, true)` (null if allocation fails);
      `ped->m_pIntelligence->m_TaskMgr.SetTask(task, 3, false)`.
- gta-reversed: raw call only.

## 0x6F6BD0 CTrain::FindClosestTrackNode  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl int32 (CVector posn [esp+4..+0xC], int32* outTrackId [esp+0x10])`, EAX.
- Behaviour: `best = 99999.898f` (0x47C34FF3); result register initialised from an UNINITIALISED stack local
  (0x6F6BDB `mov edi,[esp+0x14]`). For track t = 0..3, node i = 0..NumTrackNodes[t]-1 (count re-read each
  iteration): `dx = posn.x - P.x` (stored f32), `dy = posn.y - P.y`, `dz = posn.z - P.z` (registers);
  `d = sqrt(dx*dx + (dy*dy + dz*dz))` (fsqrt); if `d < best` (strict): `best = d; result = i; *outTrackId = t`.
  Return result. If nothing is closer than 99999.9 the return value is stack garbage and outTrackId untouched.

## 0x6F6CC0 CTrain::FindPositionOnTrackFromCoors  (catalogue: specified/REA, port level 9)
- Convention: thiscall, no args, plain `ret`. Split fragment at 0x403C30 (`movzx eax, byte [esi+0x5C0]`).
- Callers: CVehicleRecording::SaveOrRetrieveDataForThisFrame, SetNewTrainPosition (2 tail calls),
  CreateMissionTrain (2).
- Constants: 0x858CDC = 0.001f, 0x872490 = 1.001f, 0x858B3C = 3.0f, 0x858F10 = 0.033333335f, 0x858624 = 1.0f,
  0x858B8C = 0.5f; `CCustomBuildingDNPipeline::m_fDNBalanceParam` 0x8D12C0.
- Behaviour: `track = m_nTrackId`; return if `NumTrackNodes[track] <= 0`. For `i = 0 .. n-1` (n = NumTrackNodes[track]
  re-read after each iteration): `next = (i + 1) % n`; `Pi`, `Pn` = 2D node positions (int16 * 0.125f, stored f32); `pos` =
  train position (matrix or placement).
  - `L = sqrt((Pi.y-Pn.y)^2 + (Pi.x-Pn.x)^2)` (fsqrt); `dX = Pn.x - Pi.x`, `dY = Pn.y - Pi.y` (stored f32);
    `t = ((pos.y - Pi.y)*dY + (pos.x - Pi.x)*dX) / (L*L)` (stored f32).
  - Skip unless `t > 0.001f` and `t < 1.001f` (NaN skips).
  - `C = (dX*t + Pi.x, dY*t + Pi.y)`; `dist = sqrt((C.x-pos.x)^2 + ((C.y-pos.y)^2 + (t*0.0f)^2))`
    (a `t*0.0f` term is squared and added; 0x6F6E33-0x6F6E75).
  - If `dist < 3.0f`: FOUND (below). Otherwise (dist >= 3 or NaN) the contact-surface brightness is updated and
    the loop continues (0x6F6E90-0x6F6F38): `li = Li(nodes[i])`, `ln = Li(nodes[next])` with
    `Li(node) = (float)(l & 0xF) * 0.033333335f * (1.0f - DN) + (float)(l >> 4) * 0.033333335f * DN`,
    `l = node.GetLightingFromCollision()` (0x6F5F80, thiscall, AL), `DN = m_fDNBalanceParam`;
    `m_fContactSurfaceBrightness (+0x12C) = (ln - li) * t + li`. (Original oddity: the brightness is taken from
    segments the train is NOT on; kept.)
  - FOUND (0x6F6F6D): `r = Di + (Dn - Di) * t` with `Di = nodes[i].dist * 0.33333334f`, `Dn = nodes[next].dist *
    0.33333334f`; `r -= (col->bbox.max.y - col->bbox.min.y) * 0.5f` (col = `ms_modelInfoPtrs[model]->m_pColModel`
    +0x14, max.y +0x10, min.y +0x04); store; if `r < 0`: `r += arrTotalTrackLength[track]`. `m_fCurrentRailDistance
    = r`. Speed: `v = sqrt(mx*mx + (my*my + mz*mz))` of `m_vecMoveSpeed` (+0x44); `fwd = (dX*mx + (dY*my +
    mz*0.0f)) > 0`; `m_fTrainSpeed = (fwd == bClockwiseDirection) ? -v : v`. Return.
- Edge cases: for the last node, `Dn = 0`, so r interpolates backwards from Di. If no segment is within 3 m, rail
  distance and speed are unchanged.

## 0x6F7090 CTrain::FindNearestTrain  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl CTrain* (CVector posn [esp+4..], bool mustBeMainTrain [esp+0x10])`, EAX.
- Behaviour: `best = 1.0e7f`, result = null. Vehicle pool walked from the highest slot down (flags byte < 0 =
  free; object = base + i*0xA18). For `m_nVehicleType == 6`: `d = sqrt(dy*dy + dx*dx)` with `dx = v.x - posn.x`,
  `dy = v.y - posn.y` (2D); if `d < best` and (`!mustBeMainTrain` or `flags & 0x08`): `best = d`, result = v.

## 0x6F7190 CTrain::vfunc_14 = Teleport  (catalogue: specified/REA, port level 4)
- Convention: thiscall `(CVector pos [esp+4..0xC] by value, bool clearOrientation [esp+0x10])`, `ret 0x10`.
- Behaviour: `CWorld::Remove(this)`; position = pos (matrix pos if m_matrix else placement); if clearOrientation:
  `SetOrientation(0, 0, 0)` (0x439A80); `m_vecMoveSpeed = m_vecTurnSpeed = (0,0,0)`; `CWorld::Add(this)`.

## 0x6F7260 CTrain::IsNextStationAllowed  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl bool (CTrain* train)`, AL.
- Behaviour: `e` = engine (follow m_pPrevCarriage to the end, inline); `FindNextStationPositionInDirection(
  (e->flags >> 6) & 1, e->m_fCurrentRailDistance, &tmp, &idx)`; `lvl = CTheZones::GetLevelFromPosition(
  &aStationCoors[idx])` converted to float (fild); return `CStats::GetStatValue(181 /*STAT_CITY_UNLOCKED*/) + 1.0f >= lvl`
  (`test ah,1` after fcomp: less or unordered -> false).

## 0x6F72F0 CTrain::SkipToNextAllowedStation  (catalogue: specified/REA, port level 21)
- Convention: `__cdecl void (CTrain* train)`.
- Behaviour: `e` = engine (inline walk). `dist = e->m_fCurrentRailDistance`. Loop: `FindNextStationPositionInDirection(
  cw, dist, &dist, &idx)` (the output overwrites `dist`, so each iteration advances one station);
  repeat while `GetStatValue(181) + 1.0f < (float)GetLevelFromPosition(&aStationCoors[idx])`. (Infinite loop if no
  station is allowed - original behaviour.) Then `e->m_fCurrentRailDistance = dist`; if `e->flags & 0x40`:
  `e->m_fCurrentRailDistance = dist - 20.0f; e->m_fTrainSpeed = 0.1f` else `dist + 20.0f`, speed `-0.1f`.
  `CStreaming::LoadScene(&aStationCoors[idx])`; `CStreaming::LoadAllRequestedModels(false)`.
  `d = sqrt(dx*dx + dy*dy)` with `dx = st.x - e.pos.x`, `dy = st.y - e.pos.y` (e's matrix position, NOT updated by
  this function); `CGameLogic::PassTime(_ftol(d * 0.05f + 23.0f))`.

## 0x6F7440 CTrain::InitTrains  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl void ()`.
- Behaviour: `bDisableRandomTrains = false; GenTrain_Status = 0`. For i = 0..3, if `pTrackNodes[i] == null`:
  `ReadAndInterpretTrackFile(name_i, &pTrackNodes[i], &NumTrackNodes[i], &arrTotalTrackLength[i], i)` with names
  "data\\paths\\tracks.dat" (0x8724DC), "tracks3.dat" (0x8724C4), "tracks2.dat" (0x8724AC), "tracks4.dat"
  (0x872494). Then `base = pTrackNodes[0]` (read once); for each station s: `n = FindClosestTrackNode(
  aStationCoors[s], &tmp)` (track id ignored); `StationDist[s] = (float)base[n].dist * 0.33333334f`.
- gta-reversed (Train.cpp:149, unreachable): passes the array bases instead of `&pTrackNodes[i]` etc., uses
  `pTrackNodes[trackId]->m_nDistanceFromStart` (node 0 of the found track) and `/ 3.0f`. The bytes use node `n`
  of TRACK 0 and multiply by 0.33333334f. Keep the bytes.

## 0x6F55D0 CTrain::ReadAndInterpretTrackFile  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (const char* filename, CTrainNode** nodes, int32* count, float* totalDist,
  int32 skipStations)`. SEH frame.
- Behaviour:
  1. If `*nodes == null`:
     a. `buf = operator new(0xB530)`; `CFileMgr::LoadFile(filename, buf, 0xB530, "rb")`.
     b. Copy the first line (up to '\n') into `gString` (0xB71670), NUL-terminate. If it equals "processed"
        (10-byte compare incl. NUL), copy the next line instead (NUL-terminated).
        `sscanf(gString, "%d", count)`.
     c. `arr = operator new(*count * 10)`; if non-null: array-construct with CTrainNode::CTrainNode 0x6F5370
        (sets byte +9 = 0) via 0x403D60; `*nodes = arr` (null if allocation failed).
     d. For `i = 0 .. *count-1`: copy the next line into gString WITHOUT writing a terminator (stale tail of a
        longer previous line remains); `sscanf(gString, "%f %f %f %d", &x, &y, &z, &station)` (station keeps its
        previous value if not parsed; initially it holds the array pointer). `node.x = (int16)_ftol(x * 8.0f)`,
        same for y, z (0x859000 = 8.0f, truncation). If `skipStations == 0 && station != 0`:
        `aStationCoors[k++] = (x, y, z)` (unquantised floats, no bound check).
     e. `free(buf)` (CRT free of operator-new memory).
  2. Always: `total = 0.0f`; for i = 0..n-1: `nodes[i].dist = (uint16)_ftol(total * 3.0f)` (0x858B3C);
     `total += sqrt((xi-xn)^2 + (yi-yn)^2)` with next = (i+1) % n, coordinates int16 * 0.125f (2D, z ignored,
     last node wraps to node 0). `*totalDist = total`.

## 0x6F7550 CTrain::CreateMissionTrain  (catalogue: specified/REA, port level 28)
- Convention: `__cdecl void (CVector posn [esp+4..0xC], bool clockwise [esp+0x10], uint32 trainType [esp+0x14],
  CTrain** outFirst [esp+0x18], CTrain** outLast [esp+0x1C], int32 nodeIndex [esp+0x20], int32 trackId [esp+0x24],
  bool isMissionTrain [esp+0x28])`. SEH frame. Local array of up to 16 created carriages.
- Train configs (0x8D44F8 + type*0x40): 0 {537,569,569,569,569}, 1 {538,570,570}, 2 {538,570,570},
  3 {537,569,569,569}, 4 {538,570,570}, 5 {538,570,570,570}, 6 {537,569,569}, 7 {538,570,570}, 8 {449,449},
  9 {449}, 10 {537,569}, 11 {538,570,570}, 12 {537,569,569,569,537}, 13 {537,569,569,569,569,569}, 14 {449},
  15 {538}.
- Behaviour:
  1. If `nodeIndex < 0`: `nodeIndex = FindClosestTrackNode(posn, &trackId)`.
  2. `rd = pTrackNodes[trackId][nodeIndex].dist * 0.33333334f`. `prev = null`, `count = 0`.
  3. For each model id m in the config (until 0):
     a. `t = new CTrain(m, 4)` via CVehicle pool `operator new(0x6AC)` 0x6E2D50 (t = null if that fails; then
        crashes). `t->m_matrix->SetTranslate(0, 0, 0)`.
     b. status = 4 (`(b & 7) | 0x20`); `m_nNodeIndex = (int16)nodeIndex`; vehicle flag byte +0x428 |= 8
        (bIsLocked); flags: bMissionTrain = isMissionTrain, bClockwiseDirection = clockwise;
        `m_fCurrentRailDistance = rd`; `m_nTrackId = (uint8)trackId`.
     c. If isMissionTrain: clear bStopsAtStations; if `prev == null`: position = posn (matrix or placement),
        `FindPositionOnTrackFromCoors()`, `m_fLength = 0`.
        If not mission and `prev == null`: `m_fLength = 0` (position NOT set).
        If `prev != null` (both cases): `if (clockwise) { L = len(t); m_fLength = -L; rd = rd - L; }
        else { L = len(prev); m_fLength = L; rd = rd + L; }` with `len(x) = colModel(x).bbox.max.y - min.y`.
        (rd changes after this carriage's distance was stored: carriage 1 has the same distance as carriage 0.)
     d. `arr[count++] = t`. If `prev == null`: set bIsFrontCarriage, `if (outFirst) *outFirst = t`.
        Else: clear bIsFrontCarriage, vehicle flag byte +0x42A |= 2 (bHasBeenOwnedByPlayer), clear prev's
        bIsLastCarriage. Then set bIsLastCarriage; `if (outLast) *outLast = t`.
     e. `t->m_pPrevCarriage = prev`, `if (prev) prev->RegisterReference(&t->m_pPrevCarriage)`;
        `t->m_pNextCarriage = null`; `if (prev) { prev->m_pNextCarriage = t; t->RegisterReference(
        &prev->m_pNextCarriage); }`. `t->ProcessControl()` (vtable +0x28). `prev = t`.
  4. For i = count-1 down to 0: `CWorld::Remove(arr[i]); CWorld::Add(arr[i])`.
  5. `first = *outFirst` (no null check on outFirst). If `first->model != 449`: `AddPedInCar(first, true, -1, 0,
     false, false)`; if `first->m_pDriver`: `driver->m_pIntelligence->SetPedDecisionMakerType(6)`.
  6. `streak = any carriage from first (via next) has model 538 (0x21A)`; for every carriage from first set
     bIsStreakModel (high flag byte bit 2) = streak.
- gta-reversed: raw call only.

## 0x6F7900 CTrain::DoTrainGenerationAndRemoval  (catalogue: specified/REA, port level 29)
- Convention: `__cdecl void ()`. Locals 0x40. Called from CCarCtrl::GenerateRandomCars.
- Constants: 0x858B34 = 60.0f, 0x863994 = 220.0f, 0x858F98 = 170.0f, 0x858B44 = 6.0f, 0x858FD8 = 300.0f,
  0x8724F8 = 2222.0f, 0x8724F4 = -1750.0f. Camera position = TheCamera matrix pos (0xB6F03C) or placement
  (0xB6F02C).
- Behaviour:
  1. Every 3 s (`prevTime / 3000 != now / 3000`, unsigned via magic multiply): `[0xC38068] = 0`; for each station:
     if the 2D distance station -> FindPlayerCoors(-1) is `< 60.0f`: `[0xC38068] = 1`.
  2. `period = flag ? 1 : 950`, `limit = flag ? 100.0f : 70.0f`; return if `prevTime / period == now / period`.
  3. Removal (pool, highest slot first): for each vehicle with `m_nVehicleSubType == 6`, bIsFrontCarriage set and
     bMissionTrain clear: `exists = true`; `far = true`; for each carriage c from it (via next):
     if `c == FindPlayerVehicle(-1, false)` or `sqrt(dx*dx + dy*dy) < 220.0f` (dx, dy = camera - c, stored f32)
     -> far = false. If far: delete the whole chain from this engine (next read first; `CWorld::Remove(c)`;
     `c->vtable[0](1)`).
  4. Return if `bDisableRandomTrains`. Switch `GenTrain_Status`:
     **Status 1** (generate): `cfg = configs[GenTrain_TrainConfig]`; for each model: if
     `ms_aInfoForModel[m].loadState (0x8E4CD0 + m*0x14) != 1`: `RequestModel(m, 8)` and mark not ready. Return if
     not ready. `node = pTrackNodes[GenTrain_Track][GenTrain_GenerationNode]`; if `sqrt(dy*dy + dx*dx) > 60.0f`
     (camera to node, 2D): `CreateMissionTrain((0,0,0), GenTrain_Direction, GenTrain_TrainConfig, &first, null,
     GenTrain_GenerationNode, GenTrain_Track, false)`; cruise: if `GenTrain_Track == 0`: `cs = rand() % 30 + 15`;
     if `sqrt((node.x*0.125f - 2222.0f)^2 + (node.y*0.125f + 1750.0f)^2) < 300.0f`: `cs /= 2` (signed, toward 0);
     else `cs = rand() % 7 + 7`. `FindMaximumSpeedToStopAtStations(first, &maxSp)`; `sp = ((float)cs < maxSp) ?
     (float)cs : maxSp`; `CTrain::SetTrainSpeed(first, sp)` (0x6F5E20); `first->m_autoPilot.m_nCruiseSpeed = (uint8)cs`.
     In both cases (created or too close): `SetModelIsDeletable(m); SetModelTxdIsDeletable(m)` for each config model;
     `GenTrain_Status = 0`.
     **Status 0** (choose a spot): return if `exists`. `GenTrain_Track = 0`; `GenTrain_GenerationNode = rand() %
     NumTrackNodes[0]`; P = node position (x, y, z); `d = sqrt(dx*dx + dy*dy)` (camera 2D).
     - If `d < limit`: if NOT (`P.z + 6.0f > player.z` or `FindTunnelAttributesForCoors(player) & 0x880`) and
       `FindTunnelAttributesForCoors(P) & 0x800` -> go to the track-1 branch. Otherwise:
       `GenTrain_Direction = rand() & 1`; step the node (dir ? -1 with wrap : +1 mod n) while `d < 170.0f`,
       remembering the node before the last step in `prevNode`; (if d >= 170 at entry, prevNode = a stale temp =
       the raw int z of the node - unreachable since limit <= 100). `cfg = ([0xC38064] + 1) % 8` (signed),
       stored to 0xC38064 and GenTrain_TrainConfig; `GenTrain_Status = 1`; if `d > 220.0f`:
       `GenTrain_GenerationNode = prevNode`. Return.
     - Track-1 branch (d >= limit, or the tunnel case): return unless `GenTrain_Status == 0` and
       `CWeather::WeatherRegion (0xC81314, i16) == 2`. `GenTrain_Track = 1`; node = `rand() % NumTrackNodes[1]`;
       return if `d >= limit`. `GenTrain_Direction = rand() & 1`; step while `d < 170.0f`; if `d < 220.0f`:
       `GenTrain_Status = 1`, `GenTrain_TrainConfig = rand() % 2 + 8`.
     Other status values: return.
- gta-reversed: raw call only.

## 0x6F8170 CTrain::AddNearbyPedAsRandomPassenger  (catalogue: specified/REA, port level 10)
- Convention: thiscall, no args, plain `ret`. SEH frame.
- Behaviour:
  1. `pl = FindPlayerPed(-1)`; return if `pl->m_pVehicle == this`; return if the player's primary task (slot 3) is
     700/701 targeting this train (as in RemoveRandomPassenger).
  2. `cw = (flags >> 6) & 1`. Return if low nibble of +0x5CB == high nibble.
  3. If `tmp = m_pTemporaryPassenger`: `j = tmp bJustGotOffTrain`; return unless `j != tmp bInVehicle`; return if
     `tmp->m_pIntelligence->FindTaskByType(707 /*LEAVE_CAR_AND_WANDER*/)`; `CleanUpOldReference(&m_pTemporary...)`;
     if `!j && tmp bInVehicle && tmp->m_pVehicle == this`: `CPopulation::RemovePed(tmp)`; field = null.
  4. `planeD = -(pos.z*R.z + pos.y*R.y + pos.x*R.x)` with R = `m_matrix->right` (no null check).
  5. Ped pool, highest slot first; `best = null`, `bestD = 999999.0f`. Skip peds with: `m_nCreatedBy != 1`,
     `CPedGroups::GetPedsGroup(p)`, `m_pPlayerData`, `m_nPedType == 6` (cop), bInVehicle, bJustGotOffTrain,
     `m_nPedState` 54/55/56, `FindTaskByType(700)`. `d2 = (p.x-t.x)^2 + (p.y-t.y)^2 + 0^2` (VectorSub 0x40FE60 then
     SquaredMagnitude 0x406DA0 on (dx, dy, 0)); skip if `d2 > 625.0f`. `side = Dot(p.pos, R) + planeD`
     (0x40FDB0: z, y, x order); if `!cw` require `side <= 0`, if `cw` require `side >= 0` (NaN passes both).
     If `d2 < bestD`: best = p, bestD = d2.
  6. Return if no best. `GetNearestCarDoor(best, this, &doorPos, &door=0)` (result ignored).
     `inv = Invert(m_matrix)` (0x59B920 into a local CMatrix); `off = inv * doorPos` (0x59C890);
     `off -= R` if `!cw`, else `off += R` (0x406D70 / 0x411A00).
  7. `seek = new CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>(this, 50000, 1000, 1.0f, 2.0f, 2.0f,
     true, true)` (0x661DC0, ret 0x20); `seek+0x44 = off`; `seek+0x50 = 7`. `seq = new CTaskComplexSequence()`;
     `seq.AddTask(seek)`; `seq.AddTask(cw ? new CTaskComplexEnterCarAsPassenger(this, 0, false) :
     new CTaskComplexEnterCarAsDriver(this))`; `best->m_pIntelligence->m_TaskMgr.SetTask(seq, 3, false)`.
  8. Low nibble of +0x5CB += 1; `m_pTemporaryPassenger = best; best->RegisterReference(&m_pTemporaryPassenger)`.
- Edge cases: null task allocations are not handled (writes through null).
- gta-reversed: raw call only.
