# CPed (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5DFDF0 CPed::PositionAttachedPed  (catalogue: specified/REA, gr plugin::Call wrapper, port level 4)
- Convention: `__thiscall void()`, plain `ret`. SEH frame for two stack CMatrix objects (M at esp+0x20, R at esp+0x74, both with
  `m_pAttachMatrix = 0, m_bOwnsAttachedMatrix = 0` set inline; destroyed with ~CMatrix 0x59ACD0 at the end).
  Callers: CCam::Process_M16_1stPerson 0x5105C0, CWorld::Process 0x5684A0, CPed::AttachPedToEntity 0x5E7CB0 (3).
- Fields: `m_pAttachedTo` +0xFC; `m_vecTurretOffset` +0x76C (CVector); +0x778 u16 attach direction (gr `m_fTurretAngleA`, really a
  0..3 mode); +0x77C f32 heading limit (gr `m_fTurretAngleB`); `m_fCurrentRotation` +0x558; `m_nPedType` +0x598; `m_vecMoveSpeed`
  +0x44, `m_vecTurnSpeed` +0x50; flags1 +0x46C; `m_standingOnEntity` +0x568. Constants: -900.0f 0x86C2FC, 1000.0f 0x858C4C,
  pi/2 0x858FE4, pi 0x858CB8, 2pi 0x858CBC, -pi 0x858CC0.
- Behaviour:
  1. `att = m_pAttachedTo`; null -> return.
  2. If `att->m_nModelIndex (+0x22, i16) == 544` (FIRELA) and `att->+0x69C` (CAutomobile `m_aCarNodes[21]`, CAR_MISC_B = ladder frame)
     != 0 and `m_vecTurretOffset.z < -900.0f`: `M.Attach(RwFrameGetLTM(frame), false)` then `M.Detach()` (copy);
     `off = m_vecTurretOffset; off.z += 1000.0f` (f32).
     Else: if `att->m_matrix` is null: `att->AllocateMatrix()` + `att->m_placement.UpdateMatrix(att->m_matrix)`; `M = *att->m_matrix`;
     `off = m_vecTurretOffset` (0x5DFE3D-0x5DFEEC).
  3. `w = Multiply3x3(M, off)` (0x59C790, returns a temp vector); `M.pos.z += w.z; M.pos.y += w.y; M.pos.x += w.x` (that order).
  4. `h = atan2(-M.forward.x, M.forward.y)` (fpatan; stored f32) (0x5DFF2F-0x5DFF3B).
  5. If `m_nPedType` is 0 or 1 (player): skip to 6. Otherwise:
     `t = h` adjusted by mode (+0x778): 1 -> `h + pi/2`, 2 -> `h + pi`, 3 -> `h - pi/2`, else h (stored f32); `t = LimitRadianAngle(t)`;
     `m_fCurrentRotation = LimitRadianAngle(m_fCurrentRotation)` (fst); `d = m_fCurrentRotation - t`; `if (d > pi) d -= 2pi; else if (d < -pi) d += 2pi`;
     `lim = +0x77C`: if `d > lim`: `m_fCurrentRotation = t + lim`; else if `d < -lim`: `m_fCurrentRotation = t - lim` (NaN: no change);
     then always `m_fCurrentRotation = LimitRadianAngle(m_fCurrentRotation)` (0x5DFF4E-0x5E0025).
  6. `R.SetRotateZ(m_fCurrentRotation - h)`; `M *= R` (0x411A80); `this->SetMatrix(M)` (CPlaceable::SetMatrix 0x54F610).
  7. If `att` is a vehicle or object (type bits `[+0x36] & 7` = 2 or 4): copy `att->m_vecMoveSpeed` and `att->m_vecTurnSpeed` into this.
  8. `flags1 |= 1` (bIsStanding); `m_standingOnEntity = 0`.
- gta-reversed: wrapper only (`Ped.cpp:1361`).

## 0x5E0540 CPool<CPed,CCopPed>::CreateAtRef (proposed; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: only caller `CPed::operator new(size, int32 poolRef)` 0x5E4730; body equals gr `CPool::CreateAtRef`
  (`Core/Pool.h:231`). Pool layout: +0x00 m_Storage, +0x04 m_SlotState (u8 per slot: bit7 IsEmpty, bits0-6 Ref),
  +0x08 m_Size, +0x0C m_LastFreeSlot (int32).
- Convention: `__thiscall void(int32 ref [esp+4])`, ECX = pool, `ret 4`. Preserves EBX/ESI.
- Behaviour:
  1. `idx = ref >> 8` (arithmetic `sar`, 0x5E054A).
  2. `slot[idx] &= 0x7F` (clear IsEmpty, 0x5E054D).
  3. `slot[idx] = slot[idx] ^ ((slot[idx] ^ (u8)ref) & 0x7F)` = keep bit7, low 7 bits := `ref & 0x7F` (0x5E055A-0x5E0564).
  4. `m_LastFreeSlot = 0`; then while `(int8)slot[m_LastFreeSlot] >= 0` (not empty): `m_LastFreeSlot++`
     (0x5E0569-0x5E0583; the field is re-read and re-written each iteration). No upper bound: if no slot is empty
     the scan runs past the slot array (original behaviour; never happens in practice).
- gta-reversed: same (asserts only).

## 0x5E0820 CPed::PositionPedOutOfCollision  (catalogue: specified/REA, gr plugin wrapper, port level 22)
- Convention: `__thiscall bool(int32 exitDoor [esp+4], CVehicle* veh [esp+8], bool findClosestNode [esp+0xC])`, `ret 0xC`, return AL. The first two
  argument slots are reused as scratch (side offset, "found" flag). Callers: CEventKnockOffBike::SetPedSafePosition 0x4B4AC0, vt_86ee94::ProcessPed 0x6477F0 (2),
  CTaskSimpleCarSetPedOut::PositionPedOutOfCollision 0x6479B0 (3), CTaskSimpleCarSetPedSlowDraggedOut::ProcessPed 0x64C220.
- Notation: `B = veh->GetColModel()` bounding box (+0x0 min.x, +0x4 min.y, +0x8 min.z, +0xC max.x, +0x10 max.y, +0x14 max.z); `V` = vehicle position (copy),
  `O` = this ped's original position (copy); `m = veh->m_matrix` (right +0, fwd +0x10, up +0x20; used without null checks). TRY(c): write c into this ped's
  position directly (m_matrix->pos if the ped has a matrix, else placement), then if `!CPhysical::CheckCollision()` (0x54D920) and
  `CWorld::GetIsLineOfSightClear(V, c, 1, 0, 0, 1, 0, 0, 0)` (0x56A490): found -> step 7. Candidate components are formed in x87 and stored f32.
- Behaviour:
  1. If `veh == null`: `veh = m_pVehicle (+0x58C)`; if still null return false. If `F3 & 1` (bDonePositionOutOfCollision) return true (0x5E139E).
  2. `CWorld::pIgnoreEntity = veh`; save `m_nFlags & 1` (uses collision); `m_vecMoveSpeed = 0`; `physicalFlags |= 0x10000`; `m_nFlags &= ~1`.
  3. Door candidates: if `veh->IsOnItsSide()` and `m_nVehicleClass != 9`: TRY(V + (0, 0, B.max.x + 1.0f)) (SetPosn 0x4241C0). Else if `exitDoor != 0`:
     TRY(`CCarEnterExit::GetPositionToOpenCarDoor(veh, exitDoor)` 0x64E740); if not found and the vehicle is a bike and exitDoor is 10 or 11:
     TRY(GetPositionToOpenCarDoor(veh, exitDoor == 11 ? 9 : 8)).
  4. Side offset: `h = max(B.max.z, -B.min.z)` (`B.max.z > -B.min.z` picks max.z); `side = |m.right.z| * h + (B.min.x - 0.355f)` (0x86C300); if exitDoor
     is 8 or 9: `side = B.max.x + 0.355f`.
  5. If not found, in order (each a TRY):
     a. `k = side - ((O.z-V.z)*m.right.z + (O.y-V.y)*m.right.y + (O.x-V.x)*m.right.x)`; `c = O + m.right * k`.
     b. for i = 0..3: `t = i * ((B.max.y - B.min.y) * 0.33333334f) + B.min.y`; `c = (V + m.right*side) + m.fwd*t`.
     c. `c = V + m.right * (B.min.x - 0.355f)`; d. `c = V + m.right * (B.max.x + 0.355f)`.
     e. `c = (V - m.right*side) + m.fwd*B.min.y`; f. `c = (V - m.right*side) + m.fwd*B.max.y`.
     g. only if `m_nVehicleClass == 0`: `c = V + m.up * B.max.z`, `c.z += 1.0f`.
  6. Not found: restore (`pIgnoreEntity = 0`, `physicalFlags &= ~0x10000`, uses-collision bit restored). If `!findClosestNode` return false.
     `n1 = ThePaths.FindNodeClosestToCoors(V, 1, 999999.88f, 0, 0, 0, 0, 0)`, `n2 = ...(V, 0, 999999.88f, ...)` (0x44F460, results as CNodeAddress).
     If n1 valid (area != 0xFFFF): `c = node(n1).GetPosition()` (0x420A10; nodes at dword[0x96F854 + area*4] + index*0x1C). If n2 valid: `p2 = node(n2) pos`;
     compare 2D distances of c and p2 from V (sqrt of squared sums) and take p2 when it is nearer (when n1 was invalid, c still holds the last
     tried candidate). If neither valid return false. `CPedPlacement::FindZCoorForPed(&c)` (0x616920); SetPosn(c); heading of the vehicle
     (`atan2(-m.fwd.x, m.fwd.y)` or veh placement heading) applied with SetRotateZOnly / placement heading; `F3 |= 1`; this move/turn speed = 0;
     veh turn/move speed = 0 then `veh->m_vecMoveSpeed.z += 0.02f`; return true.
  7. Found: restore as in 6; `F3 |= 1`; this `m_vecMoveSpeed = m_vecTurnSpeed = 0`; unless (veh is a bike and `!findClosestNode`):
     `veh->m_vecMoveSpeed = (0, 0, 0 - 0.05f)`, `veh->m_vecTurnSpeed = 0`. Return true.
- Open question: tie/NaN handling of the n1/n2 distance compare (0x5E12BB `fcompp; test ah,0x41; jne`) - port the exact flag test.

## 0x5E17E0 CPed::SortPeds  (catalogue: specified/REA (called from gta-reversed only), port level 0)
- Convention: `__thiscall void(CPed** list [esp+4], int32 lo [esp+8], int32 hi [esp+0xC])`, `ret 0xC`. Recursive (only caller is
  itself at 0x5E192A; gr wrapper `Ped.cpp:1673` calls it). Preserves EBX/EBP/ESI/EDI. The `list` and `lo` argument slots are
  reused as scratch (pivot distance f32, new lo) - not observable.
- Position helper used everywhere below: `pos(e) = e->m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &e->m_placement.pos (+0x4)`.
  `dist(p) = sqrt((dz*dz + dy*dy) + dx*dx)` with `dx = this.x - p.x`, `dy = this.y - p.y`, `dz = this.z - p.z` (each `fld this;
  fsub other`), squares summed in that order (z first) (e.g. 0x5E182D-0x5E184D). `this` position is re-read for every distance.
- Behaviour (Hoare quicksort, ascending by distance to `this`):
  1. if `lo >= hi` (signed) return (0x5E17EB).
  2. outer loop (0x5E1804): `i = lo; j = hi; pivot = (float)dist(list[(lo + hi) / 2])` (signed division truncating toward 0:
     `cdq; sub eax, edx; sar 1`); pivot is rounded to f32 (stored, 0x5E1855).
  3. partition loop (0x5E1860):
     - `while (dist(list[i]) < pivot) i++` (x87 value vs f32 pivot; `test ah,5; jp` exits on >=, ==, NaN; 0x5E18A7-0x5E18B3).
     - `while (dist(list[j]) > pivot) j--` (`test ah,0x41; jne` exits on <, ==, NaN; 0x5E18F9-0x5E1905).
     - if `i <= j`: swap list[i], list[j]; `i++; j--` (0x5E190B-0x5E1918); if still `i <= j` repeat 3; else go to 4.
       If `i > j` before the swap (0x5E1909 `jg`): go to 4 without swapping.
  4. `SortPeds(list, lo, j)` (recursive, 0x5E1921-0x5E192A).
  5. if `i < hi`: `lo = i` and repeat from 2 (tail iteration, 0x5E192F-0x5E1937); else return.
- Edge cases: no index bounds checks (the scans rely on the pivot being in range); NaN distances stop both scans.
- gta-reversed: only a call wrapper (no C++ body).

## 0x5E1B10 CPed::UpdatePosition  (catalogue: specified/REA, gr `.Reversed = false` (plugin wrapper), port level 2)
- Convention: `__thiscall void()`, plain `ret`. Caller: CPed::ProcessControl 0x5E8CD0.
- Helpers (all small game functions, port inline): 0x406D70 `CVector::operator-=` (x, y, z in order); 0x406DA0 `CVector::SquaredMagnitude`
  (thiscall, ST0 = `(x*x + y*y) + z*z`); 0x40FE30 `operator+(out, a, b)` (computes z, y, x sums, stores x, y, z); 0x40FE90 `operator*(out, float s, v)`;
  0x40FEC0 `operator*(out, v, float s)`; 0x411A00 `CVector::operator+=`; CVector::CrossProduct 0x59C730 (cdecl(out, a, b)).
- Fields: `m_fCurrentRotation` +0x558, `m_fAimingRotation`/heading goal +0x55C, `m_vecAnimMovingShift` +0x550 (2D), `m_standingOnEntity` +0x568,
  `field_56C` (contact point, CVector), `field_578` (contact normal, CVector), `m_vecMoveSpeed` +0x44, `m_nContactSurface` +0xBA.
  Locals: shift `(sx, sy)`.
- Behaviour:
  1. If `CReplay::Mode` (0xA43088) == 1 return.
  2. `ROT` := if `m_matrix` then `m_matrix->SetRotateZOnly(m_fCurrentRotation)` (0x59B020) else `m_placement.m_fHeading (+0x10) = m_fCurrentRotation`.
  3. If NOT standing (`flags1 & 1` clear): if `m_pIntelligence->GetTaskSwim()` or `->GetTaskJetPack()` or (`intel+0x10` primary task slot 3 is non-null
     with `GetTaskType() == 108`): ROT. Return in all cases.
  4. Standing: if `m_pAttachedTo` return. ROT.
  5. If `s = m_standingOnEntity`:
     - if `m_nPedType` not 0/1 and s is a vehicle with `m_nVehicleClass (+0x590) == 5` (boat): `p = pos - (0,0,1.0f)` (z stored f32) then `p -= s.pos`;
       `V = CrossProduct(s->m_vecTurnSpeed (+0x50), p) + s->m_vecMoveSpeed (+0x44)`; `V += (-(s->m_vecTurnSpeed.SquaredMagnitude()) * p) * CTimer::ms_fTimeStep`
       (each step through the helpers above, with f32 temporaries; -1.0f at 0x858C1C); `m_vecMoveSpeed.z = V.z` (0x5E1C09-0x5E1CDC).
     - else `V = s->GetSpeed(field_56C)` (CPhysical::GetSpeed 0x542CE0, thiscall(out, CVector by value)).
     - `sx = (V.x + animShift.x) - moveSpeed.x`; `sy = (V.y + animShift.y) - moveSpeed.y`;
       `m_fCurrentRotation = ts * s->m_vecTurnSpeed.z (+0x58) + m_fCurrentRotation`; `+0x55C = ts * s->turnSpeed.z + +0x55C` (0x5E1D1C-0x5E1D68).
  6. Else if `g_surfaceInfos.IsSteepSlope(m_nContactSurface)` (0x55E770) and (`field_578.x != 0` or `field_578.y != 0`; NaN counts as != 0):
     `n = (field_578.x, field_578.y)`, `n.Normalise()` (0x44E480); `m_vecMoveSpeed = (0, 0, -0.001f)`; `sx = n.x * 0.02f + animShift.x`;
     `sy = n.y * 0.02f + animShift.y`; `d = n.y*sy + n.x*sx`; if `d < 0`: `sx = sx - n.x*d; sy = sy - d*n.y` (0x5E1D8D-0x5E1E46).
     Else: `sx = animShift.x - moveSpeed.x; sy = animShift.y - moveSpeed.y`.
  7. Clamp (re-read `s = m_standingOnEntity`):
     - if `s && !s->m_pAttachedTo && (!(s->physicalFlags & 4) || (s->physicalFlags & 8))`: `len = sqrt(sy*sy + sx*sx)`;
       limit: if s is a vehicle: `m_nPedState == 54` -> `ts * 0.002f` (0x858F44); else if class 9 (bike): `s->m_vecMoveSpeed.SquaredMagnitude() > 0.04f`
       (0x863244) -> `ts * 0.0002f` (0x86C304), otherwise no clamp; else if class != 0 -> no clamp; class 0 -> `ts * 0.01f` (0x858C58).
       Not a vehicle -> `ts * 0.01f`. If `len > limit`: `r = limit / len; sx *= r; sy *= r` (0x5E1E6A-0x5E1F7B).
     - else if `flags3 (+0x474) & 0x100000` (bFallenDown) and `s == null`: `len = sqrt(sy*sy + sx*sx)`, `limit = ts * 0.01f` (0x86C28C); same scaling.
  8. `m_vecMoveSpeed.x += sx; m_vecMoveSpeed.y += sy` (fld shift; fadd field; fstp).

## 0x5E2530 CPed::ProcessEntityCollision  (catalogue: specified/REA, gr hook commented out (plugin wrapper), port level 9)
Instruction-level spec (1199 instructions; extents 0x5E2530-0x5E32BD and 0x5E32C0-0x5E3629). Locals `F+x` = `[esp+x]` with esp as it is after
`push edi` (0x5E2585); from 0x5E2691 on the body runs with EBX pushed, so raw operands there read `[esp+x+4]` (normalised here).
- Convention: virtual `__thiscall int32(CEntity* e [F+0x10C], CColPoint* colPoints [F+0x110])`, `ret 8`; ESI = this (CPed*), EBX = e from
  0x5E2692. Returns the number of sphere collision points `n` (EDI at 0x5E360C; 0 from the early exit 0x5E268A). SEH frame (handler 0x83D946,
  state dword F+0x104). Callers: CPhysical::ProcessShiftSectorList 0x546670 and ProcessCollisionSectorList_SimpleCar 0x54CFF0 (virtual).
- Callees: CPedIntelligence::GetTaskUseGun 0x600F70 (3 calls), CPlaceable::GetHeading 0x441DB0 (ST0), CPlaceable::SetHeading 0x43E0C0,
  CPlaceable::AllocateMatrix 0x54F560, CSimpleTransform::UpdateMatrix 0x54F1B0, CEntity::GetColModel 0x535300,
  CCollision::ProcessColModels 0x4185C0 (cdecl(const CMatrix& a, CColModel& cmA, const CMatrix& b, CColModel& cmB, CColPoint* sphPts,
  CColPoint* linePts, float* lineDists, bool ignoreLines) -> int), CEntity::RegisterReference 0x571B70, SurfaceInfos_c::IsSteepSlope 0x55E770 /
  IsSoftLanding 0x55E690 (thiscall on g_surfaceInfos 0xB79538, (u8 surface)), RpAnimBlendClumpGetAssociation(clump, animId) 0x4D68B0 (cdecl),
  CPed::GetLocalDirection 0x5DEF60, CEventDamage ctor 0x4AD830 / dtor 0x4AD960 / AffectsPed 0x4B35A0, CPedDamageResponseCalculator ctor 0x4AD3F0 /
  dtor 0x4AD420 / ComputeDamageResponse 0x4B5AC0, CEventGroup::Add 0x4AB420, CEntity::GetIsTypePhysical 0x4DA030 (type 2..4),
  CColPoint::operator= 0x40FC80 (thunk to 0x15637E0: thiscall(dest, const CColPoint& src), copies 0x2C bytes field-wise, `ret 4`),
  CVector::Normalise 0x59C910, CMatrix::SetRotateZOnly 0x59B020, CPhysical::AddCollisionRecord 0x543490.
- Globals (R): CTimer::ms_fTimeStep 0xB7CB5C, CTimer::m_snTimeInMilliseconds 0xB7CB84, CModelInfo::ms_modelInfoPtrs 0xA9B0C8, TheCamera active cam
  byte 0xB6F081, cam mode u16 at 0xB6F1A8 + 0x238*i, cam front x/y at 0xB6F32C/0xB6F330 + 0x238*i, CTempColModels::ms_colModelPed2 0x968E20
  (col-data pointer dword 0x968E4C), CCustomBuildingDNPipeline::m_fDNBalanceParam 0x8D12C0, .data 0x8D21E4 (0.1f).
  Constants: 0x86C330 -0.15f, 0x858624 1.0f, 0x858B3C 3.0f, 0x86C32C -0.867f, 0x858F10 0.033333335f, 0x86C328 99999.992f, 0x86C290 0.33f,
  0x86C294 -0.25f, 0x858CE8 1.5f, 0x86C324 0.375f, 0x858B50 0.0f, 0x86C298 100.0f, 0x86C29C 400.0f, 0x86C2A0 -0.6f, 0x858C58 0.01f, 0x86C310 -0.01f,
  0x858C1C -1.0f, 0x86C320 -0.016f, 0x858B1C 0.1f, 0x86C318 -1000.0 (f64), 0x86C314 -0.99f, 0x858CDC 0.001f, 0x858C28 0.05f, 0x858F9C 0.35f.
- Ped fields: +0x10 placement heading, +0x14 m_matrix, +0x18 clump, +0x1C m_nFlags (bit0 bUsesCollision, bit1 bCollisionProcessed, bit4 bIsStuck,
  bit12 0x1000 bHasHitWall), +0x22 model, +0x40 physical flags (mask 0x19000 = bits 12/15/16, gr: bSkipLineCol, bProcessingShift,
  bForceHitReturnFalse; byte +0x42 bit0 = bit 16), +0x44 moveSpeed, +0xBA m_nContactSurface, +0xFC m_pAttachedTo, +0x128 m_pEntityIgnoredCollision,
  +0x12C m_fContactSurfaceBrightness, flag dwords F1 +0x46C (bit0 bIsStanding, bit1 bWasStanding), F2 +0x470 (0x2 bOnBoat, 0x20 bHitSteepSlope,
  0x80 bTryingToReachDryLand, 0x200000 bHeadStuckInCollision, 0x8000000 bPushedAlongByCar), F3 +0x474 (0x8000 bTestForBlockedPositions,
  0x10000/0x20000/0x40000/0x80000 bRightArmBlocked/bLeftArmBlocked/bDuckRightArmBlocked/bMidriffBlockedForJump, 0x100000 bFallenDown),
  F4 +0x478 (0x100: enables the above-head line; gr's bitfield count names this bit bDruggedUp and puts bCheckColAboveHead at 0x40 - the gr
  layout of this dword is unverified), +0x47C intelligence, +0x530 m_nPedState, +0x568 standing-on entity (ref), +0x56C..0x574 offset on it,
  +0x578..0x580 ground normal, +0x584 contact entity (ref), +0x588 ceiling z (99999.992f = none), +0x58C m_pVehicle, +0x598 m_nPedType.
  Col model (CColModel): +0x8 bbox.min.z, +0x14 bbox.max.z, +0x24 sphere radius, +0x2C m_pColData; col data: +6 u8 numLines, +8 spheres
  (stride 0x14: centre +0, radius +0xC), +0x10 lines (stride 0x20: start +0, end +0x10). CColPoint (0x2C): point +0, normal +0x10, surfaceA +0x20,
  pieceA +0x21, lightingA +0x22, surfaceB +0x23, pieceB +0x24, lightingB +0x25 (low nibble day, high nibble night), depth +0x28.
- Locals: F+0xE u8 useLines -> later "steep normal seen"; F+0xF u8 bBoat -> later "e is not a ped"; F+0x10 0.94f (dead) -> head top;
  F+0x14 `k = ts * -0.15f`; F+0x18 n; F+0x1C ignoreLines; F+0x38 cm; F+0x3F bIsStanding copy; F+0x40/44/48 contact normal (init (0,0,1));
  F+0x4C saved heading (init -1001.0f); F+0x50 cd; F+0x54/58 lineDists[2] (init 1.0f); F+0x5C old z; F+0x60 linePts[0], F+0x8C linePts[1];
  F+0xB8 CEventDamage; F+0x24 CPedDamageResponseCalculator (overlaps the relative-velocity temporaries).

### Block A - col model choice (0x5E254B-0x5E268C)
`k = ts * -0.15f` (F+0x14). `cm = ms_modelInfoPtrs[model]->m_pColModel` (+0x14), `cd = cm->m_pColData`. If `F3 & 0x8000` (sign of byte +0x475):
if `!(m_nFlags & 1)` -> 0x5E2684. Else, for ped type 0/1 only: if `mode[activeCam] == 53` and `GetTaskUseGun()` and its `+0x30` weapon info and
`(info+0x18 >> 1) & 1`: `F+0x4C = GetHeading()`; `SetHeading(atan2(-front.x, front.y))` (fld y; fld x; fchs; fxch; fpatan). For every ped
reaching here (also non-players): `cm = &ms_colModelPed2`, `cd = *(0x968E4C)`. 0x5E267E: if `!(m_nFlags & 1) && !(byte+0x42 & 1)` return 0.

### Block B - line/sphere set-up (0x5E2691-0x5E27CB)
`bBoat = e is vehicle && e+0x590 == 5`. If `physFlags & 0x19000 || m_pAttachedTo || e is a ped`: `cd.numLines = 0; cm.radius = 1.0f;
cm.max.z = 0.95f; cm.min.z = -1.0f` (0x5E27B2). Else: set `m_nFlags |= 2` if clear; `line0.start.z = 0; line0.end.z = -1.0f`; useLines = 1;
if bWasStanding: `line0.end.z = k + line0.end.z`; `top = sph[2].radius + sph[2].centre.z` (F+0x10). If `F4 & 0x100`:
`t = (sph[0].centre.z - sph[0].radius) + 1.0f`; `line1.start.z = top - t` (via storing top then `fld; fsub st(1)`); `line1.end.z = t + top`;
`numLines = 2; cm.radius = line1.end.z; cm.max.z = line1.end.z; cm.min.z = line0.end.z`. Else: `numLines = 1; cm.max.z = 0.95f;
cm.radius = |line0.end.z|; cm.min.z = line0.end.z`.

### Block C - collision test (0x5E27CB-0x5E2856)
`ignoreLines = (m_nFlags & 0x10) && (e is a building || byte e+0x40 & 4)`. If `e->m_matrix == null`: AllocateMatrix, UpdateMatrix(placement).
`n = ProcessColModels(*ped.m_matrix, *cm, *e.m_matrix, *e.GetColModel(), colPoints, &F+0x60, &F+0x54, ignoreLines)`; `oldZ = ped.m_matrix.pos.z`
(F+0x5C). If !useLines -> block F.

### Block D - line hits (0x5E285C-0x5E3133)
Restore: `cd.numLines = 0; cm.radius = 1.0f; cm.min.z = -1.0f; cm.max.z = 0.95f`. If `!(lineDist0 < 1.0f)` -> block E.
`P = linePts[0]` (point F+0x60/64/68, normal F+0x70/74/78, surfaceB F+0x83, lightingB F+0x85). F+0x3F = bIsStanding.
1. If bIsStanding and `!(P.z + 1.0f > pos.z)` (pos = ped matrix): if !bBoat or `!(P.z + 3.0f > pos.z)` -> step 9 (skip 2-8).
2. Steep scan: F+0xE = 0; for every sphere point i < n (unrolled x4, then remainder): if `colPoints[i].normal.z < -0.867f` F+0xE = 1.
3. Brightness: `D = (lightingB & 0xF) * 0.033333335f`, `N = (lightingB >> 4) * 0.033333335f` (fild of int), `B = m_fDNBalanceParam`;
   `L = (1.0f - B) * D + B * N` (x87: `(1-B)*D` first, then `B*N`, `faddp`). Players (type 0/1): `a = ts * 0.1f` (.data 0x8D21E4);
   `+0x12C = L * a + (1.0f - a) * +0x12C`. Others: `+0x12C = L`.
4. Standing-on: if bIsStanding, or e is not a vehicle/object (types 1, 3, 5): `+0x584 = e; e->RegisterReference(&+0x584); F2 &= ~0x82`.
   Else (vehicle/object): `+0x568 = e; RegisterReference(&+0x568)`; `+0x56C = (P.x - e.x, P.y - e.y, P.z - e.z)` (e pos via matrix or placement;
   computed z, y, x); `+0x584 = e; RegisterReference(&+0x584)`; F2 bit 0x2 = (e is a boat); if e is a vehicle: `+0x12C = (*(+0x568))->+0x12C`.
5. Ceiling: if `F4 & 0x100 && lineDist1 < 1.0f && linePts[1].point.z < +0x588` and (e type 0/1/>=5, or `byte e+0x40 & 8`): `+0x588 = linePts[1].z`.
6. `hs = F2 bit 21`. If hs: if `P.z + 1.0f < pos.z` -> step 7; else if e is a building and `(N.y*v.y + N.z*v.z) + N.x*v.x > 0.0f` (N = P.normal,
   v = ped moveSpeed) -> step 7; else -> step 8. If !hs -> step 7.
7. If F+0xE (steep normal) -> step 8. Ceiling test 0x5E2C0A `fcomp 99999.992f; test ah,5; jp`: when `+0x588 >= 99999.992f` or NaN (no
   ceiling): `pos.z = P.z + 1.0f` and clear F2 bit 21 if set. Otherwise (`+0x588 < 99999.992f`):
   `z = P.z + 1.0f`; if `P.z + top + 1.0f > +0x588`: contact normal F+0x40 = N; `c = +0x588 - top`; if `c < z` then `z = c`. `pos.z = z`.
   -> step 8b.
8. (rejected) if hs and e is not a vehicle: contact normal F+0x40 = N.
8b. `+0x578 = N; +0xBA = surfaceB`; if IsSteepSlope(surfaceB): F2 |= 0x20.
9. Landing damage (0x5E2D06): `hThr = 0.33f, vThr = -0.25f`; if `+0x530 == 1`: `hThr = 0.33f + 0.33f`, `vThr = -0.25f * 1.5f`.
   `assoc = RpAnimBlendClumpGetAssociation(clump, 120)`. `rel = ped.moveSpeed`; if e is vehicle/ped/object: `rel -= e.moveSpeed` (x, y, z).
   If bWasStanding -> 9b. `h = sqrt(rel.y*rel.y + rel.x*rel.x)` (F+0x10). Damage variant A iff ((`h > hThr` && !(F2 & 0x8000000)) || `rel.z < vThr`)
   && `+0x128 != e` && `+0x58C != e`; else -> 9b.
   Variant A: `vOff = -0.25f`, `hOff = 0.25f`; if IsSoftLanding(surfaceB): `hOff = 0.375f, vOff = -0.25f * 1.5f`.
   `hd = h - hOff; if (0.0f > hd) hd = 0.0f; hd *= 100.0f; vd = vOff - rel.z; if (0.0f > vd) vd = 0.0f; vd *= 400.0f; dmg = vd + hd` (F+0x10);
   if `rel.z < -0.6f`: dmg = 500.0f. `dir = 2` (byte F+0x14); if `rel.x > 0.01f || rel.x < -0.01f || rel.y > 0.01f || rel.y < -0.01f`:
   `dir = GetLocalDirection(&(rel.x * -1.0f, rel.y * -1.0f))`.
   **0x5E2F3F**: `ev = CEventDamage(e, m_snTimeInMilliseconds, 54 /*WEAPON_FALL*/, 3 /*PED_PIECE_TORSO*/, (uint8)dir, false, false)`.
   If `ev.AffectsPed(ped)`: `calc = CPedDamageResponseCalculator(e, dmg, 54, 3, false)` (0x5E2F74); `ComputeDamageResponse(ped, &ev+0x38, true)`;
   `intel.eventGroup(+0x68).Add(&ev, false)`; destroy calc. Destroy ev. -> 10.
   9b (0x5E2FB9): variant B iff `!bWasStanding && assoc && rel.z < ts * -0.016f && +0x58C != e`:
   **0x5E3008**: `ev = CEventDamage(e, time, 54, 3, 2, false, false)`; if AffectsPed: `calc = (e, 15.0f, 54, 3, false)`, ComputeDamageResponse(ped,
   &resp, true), Add(&ev, false), destroy calc; destroy ev.
10. `F1 |= 1` (bIsStanding); `moveSpeed.z = 0`. If `oldZ + 0.1f < pos.z` and `cd.numLines == 0` (always true here) and ped type 0/1:
    ensure e matrix; `n = ProcessColModels(*ped.m_matrix, *cm, *e.m_matrix, *e.GetColModel(), colPoints, null, null, false)`. -> block F.

### Block E - no ground line (0x5E3138-0x5E31BF)
If `F4 & 0x100 && lineDist1 < 1.0f && linePts[1].z < +0x588` and (!e.GetIsTypePhysical() || `byte e+0x40 & 8`): `+0x588 = linePts[1].z`;
if bIsStanding and `top + pos.z > linePts[1].z` (pos via matrix/placement; written through m_matrix): `m_matrix.pos.z = linePts[1].z - top`.
Always: `F2 &= ~2` (bOnBoat).

### Block F - sphere points (0x5E31C6-0x5E35B4)
`i = 0` (F+0x38), `last = n - 1` (F+0x14), `p = colPoints` (F+0x10). While `i < n`:
- If `F3 & 0x8000 && m_nFlags & 1 && p.pieceA > 2`: `notPed = 1, c1 = 1, c2 = 1`; if e is a ped: `notPed = 0`; if `e.F3 & 0x100000` c1 = 0, else if
  `e.F1 & 0x4000000` (bIsDucking) c2 = 0. Piece 6 && c1 && c2: F3 |= 0x10000; piece 5 && c1 && c2: |= 0x20000; piece 8 && c1: |= 0x40000;
  piece 4 && notPed: |= 0x80000 (first match only). Then **remove the point**: if `i < last`: `colPoints[j] = colPoints[j+1]` for j = i..last-1
  (operator=), then `i--`, `p -= 0x2C`. `n--`, `last--`.
- Else if e is a vehicle with `+0x594 == 6` (train) and `byte e+0x40 & 4` and `p.normal.z < 0.0f`: `p.normal.z = 0; Normalise(p.normal)`.
- `i++`, `p += 0x2C`.
After the loop (0x5E3366): if `F+0x4C > -1000.0` (f64): `m_matrix ? m_matrix->SetRotateZOnly(F+0x4C) : (placement heading = F+0x4C)`.
If (e is a building || `e.m_nFlags & 4` || `e.m_nFlags & 0x40000`) && bWasStanding && n > 0, for each point (normal Nn = p.normal):
- if `Nn.z < -0.99f` && `p.pieceA == 2` && ped type 0/1: `h = sqrt(v.x*v.x + v.y*v.y)` (v = ped moveSpeed); `if (h < 0.001f) h = 0.001f`;
  `q = -1.0f / h`; `t = (q*v.x, q*v.y, Nn.z)`; `m_matrix.pos.z -= 0.05f`; F2 |= 0x200020.
- else: `h = sqrt(Nn.y*Nn.y + Nn.x*Nn.x)`; unless `h == 0.0f` (`test ah,0x44; jnp`): `q = 1.0f / h`, `t = (Nn.x*q, Nn.y*q, Nn.z)`.
- `Normalise(t)`; `p.normal = t`; if IsSteepSlope(p.surfaceB): F2 |= 0x20.
If `F+0x48 < 1.0f` (contact normal replaced in block D): append point `colPoints[n]`: `normal = Normalise(F+0x40, F+0x44, 0)`;
`s = normal * 0.35f`; `point = (pos.x - s.x, pos.y - s.y, pos.z - s.z)` (pos via matrix/placement); `n++`; F2 |= 0x20. Only point and normal
are written (surface/piece/depth keep whatever the caller's array held).

### Block G - records (0x5E35B4-0x5E3626)
If `n > 0 || lineDist0 < 1.0f`: `this->AddCollisionRecord(e)`; unless e is a building `e->AddCollisionRecord(this)`. If `n > 0` and (e building ||
`e.m_nFlags & 4` || `e.m_nFlags & 0x40000`): `m_nFlags |= 0x1000`. Return n.

- Oddities: F+0x10's initial 0.94f is never read; the `cd.numLines != 0` test at 0x5E30CD is dead; m_matrix is dereferenced without null checks
  (0x5E2840, 0x5E2C74, 0x5E31BC, 0x5E3457); the appended col point carries stale surface data; removal does not re-check the shifted point when
  `i == last`.
- gta-reversed: hook commented out, plugin::CallMethodAndReturn wrapper (Ped.cpp:3984); no C++ to compare.
- Open questions: none affecting the transcription; the gr names of F4 bit 0x100 and physical-flag bit 16 are unverified (see fields).

## 0x5E3630 sub_5e3630 (proposed: DoFootLanded helper "AddFootSplashParticles"; catalogue: unnamed, unknown/REA, port level 3)
- Evidence: only caller CPed::DoFootLanded 0x5E5380; emits particles through `g_fx` slot +0x30 (= `Fx_c::m_Splash`, 13th FxSystem_c*
  of gr `Fx/Fx.h`).
- **Convention (non-standard, register args)**: `EBX = CEntity* ped`, `EDI = const CVector* pos`, `EAX = int32 count`; no stack arguments,
  plain `ret`; preserves ESI (pushed); clobbers EAX/ECX/EDX. Callers must load these registers explicitly (a port gives it a normal
  signature `fn(ped, pos, count)` and the caller side in DoFootLanded passes them).
- Behaviour:
  1. if `!ped->GetIsOnScreen()` (0x534540) return.
  2. `cam` = TheCamera position (`m_matrix` 0xB6F03C ? matrix+0x30 : placement 0xB6F02C). `dx = pos.x - cam.x`, `dy = pos.y - cam.y`;
     if `dy*dy + dx*dx > 100.0f` return (`test ah,0x41; je`: NaN continues) (0x5E3658-0x5E367B).
  3. `mult = FxPrtMult_c(1.0f, 1.0f, 1.0f, 0.1f, 0.15f, 0.0f, 0.15f)` (0x4AB290).
  4. repeat `count` times (`count <= 0`: none; loop `dec; jne`): `p = *pos`;
     `p.x = ((float)rand() * 3.0518509e-05f * 0.2f + p.x) - 0.1f`; `p.y = ((float)rand() * 3.0518509e-05f * 0.2f + p.y) - 0.1f` (x87, two rand()
     calls in that order; constants 0x858C7C, 0x858CC4, 0x858B1C); `v0 = (0,0,0)`; `v1 = ped->m_matrix->forward (+0x10) * 1.5f` (each component
     fmul 1.5f 0x858CE8; ped->m_matrix used without null check);
     `m_Splash->AddParticle(&p, &v0, 0.0f, &mult, -1.0f, 1.2f, 0.6f, 0)` then `m_Splash->AddParticle(&p, &v1, 0.0f, &mult, -1.0f, 1.2f, 0.6f, 0)`
     (FxSystem_c* re-read from 0xA9AE30 for each call; 0x4AA440). Two particles per iteration, the first with zero velocity.
- gta-reversed: no counterpart (DoFootLanded is a plugin::Call wrapper there).

## 0x5E37C0 sub_5e37c0 (proposed: DoFootLanded helper "AddFootDustParticles"; catalogue: unnamed, unknown/REA, port level 3)
- Evidence: only caller CPed::DoFootLanded 0x5E5380; checks `SurfaceInfos_c::ProducesFootDust`; uses `g_fx` slot +0x24 (10th
  FxSystem_c* of gr `Fx_c` = `m_SmokeII3expand`).
- **Convention (non-standard, register args)**: `ESI = CPed* ped`, `EBX = const CVector* pos`, `EAX = int32 count`; no stack args,
  plain `ret`; preserves EDI.
- Behaviour:
  1. `!ped->GetIsOnScreen()` -> return; camera 2D squared distance `dy*dy + dx*dx > 100.0f` -> return (same code as 0x5E3630).
  2. `mult = FxPrtMult_c(1, 1, 1, 0.1f, 0.15f, 0, 0.15f)`.
  3. if `!g_surfaceInfos.ProducesFootDust(ped->m_nContactSurface (+0xBA, u8))` (0x55E8B0) return.
  4. repeat `count` times: `p = *pos`; `p.x = (rand()*c*0.2f + p.x) - 0.1f`; `p.y = (rand()*c*0.2f + p.y) - 0.1f`;
     `v.x = rand()*c*0.3f - 0.15f`; `v.y = rand()*c*0.3f - 0.15f`; `v.z = 0` (c = 3.0518509e-05f; 0.3f 0x858C24, 0.15f 0x858FCC; four rand()
     calls in this order, all made even when the particle is skipped); if `byte [0xB72C70] == 0`:
     `m_SmokeII3expand->AddParticle(&p, &v, 0.0f, &mult, -1.0f, 1.2f, 0.6f, 0)`.
- Open question: name/meaning of the byte global 0xB72C70 (catalogue only "gHorZ+0x4"; it suppresses the dust particle) - 0x5E38F4.

## 0x5E3C30 CPed::SpecialEntityPreCollisionStuff  (catalogue: specified/REA, gr plugin::CallMethod wrapper, port level 2)
- Convention: virtual `__thiscall void(CPhysical* e [esp+4], bool ignoreStuck [esp+8], bool& collisionDisabled [esp+0xC],
  bool& collidedEntityCollisionIgnored [esp+0x10], bool& collidedEntityUnableToMove [esp+0x14], bool& thisOrCollidedEntityStuck [esp+0x18])`,
  `ret 0x18`. Out-flags are only ever set to 1, never cleared. Caller: CPhysical::ProcessShiftSectorList 0x546670 (virtual, 4 sites).
- Notation: `SKIP` = `this->physicalFlags (+0x40) |= 0x1000` (bSkipLineCol) then TAIL; `TAIL` = if `m_nPedType` is 0 or 1 and
  `m_pIntelligence->GetTaskClimb()` (0x601180): `physicalFlags |= 0x1000`; return. `stuck(x)` = `x->m_nFlags (+0x1C) & 0x10` (m_bIsStuck).
- Behaviour (first matching rule wins):
  1. e is a vehicle (type bits 2) and `this->flags2 (+0x470) & 0x80000000` (bKnockedOffBike) and `m_pVehicle (+0x58C) == e`:
     `collisionDisabled = 1`; TAIL.
  2. `this->m_pEntityIgnoredCollision (+0x128) == e` or `e->+0x128 == this`: `collidedEntityCollisionIgnored = 1`; then SKIP unless
     (`flags2 & 0x10` (bKnockedUpIntoAir) and not `flags2 & 0x80000000`), in which case TAIL.
  3. `this->m_pAttachedTo (+0xFC) == e` or `e->m_pAttachedTo == this`: `collisionDisabled = 1`; TAIL.
  4. both `this->m_pAttachedTo` and `e->m_pAttachedTo` non-null: `collisionDisabled = 1`; TAIL.
  5. `f = e->physicalFlags`. If `f & 0x20` (bDisableMoveForce): if `f & 4` (bDisableCollisionForce) or `f & 0x40000000`: `collidedEntityUnableToMove = 1`;
     else if ignoreStuck: `collidedEntityCollisionIgnored = 1`; else if `stuck(this) || stuck(e)`: `thisOrCollidedEntityStuck = 1`. Then SKIP.
  6. Else if `f & 0xC0` (bInfiniteMass | bDisableZ): if ignoreStuck `collidedEntityCollisionIgnored = 1`; else if `stuck(this) || stuck(e)`
     `thisOrCollidedEntityStuck = 1`. SKIP.
  7. Else if e is an object (type 4):
     - if `e->m_nObjectFlags (+0x140) & 0x100` (bIsLampPost) and `e->m_matrix->up.z < 0.66f` (0x85BA50): `collidedEntityCollisionIgnored = 1`; SKIP.
     - if `e->m_nModelIndex == 342` and `e->m_matrix->pos.z < this->m_matrix->pos.z` (matrices used directly): `collidedEntityCollisionIgnored = 1`; SKIP.
     - if `e->m_pObjectInfo (+0x160)->m_fUprootLimit (+0x14) > 0` or `f & 4`: if `|e->m_vecMoveSpeed.x| < 0.001f && |.y| < 0.001f && |.z| < 0.001f`
       (0x858CDC; each strict) `collidedEntityUnableToMove = 1`, TAIL. Otherwise (or if neither condition): if `stuck(e)`
       `collidedEntityUnableToMove = 1`; TAIL.
  8. Else (vehicle / ped / other): if `e->m_nModelIndex` is 441, 564 or 594 (RC Bandit, RC Tiger, RC Cam): `collidedEntityCollisionIgnored = 1`; SKIP.
     Else if `stuck(e)`: `collidedEntityUnableToMove = 1`. TAIL.

## 0x5E3E90 CPed::SpecialEntityCalcCollisionSteps  (catalogue: specified/REA, gr hook `.Reversed = false`, port level 0)
- Convention: virtual `__thiscall uint8(bool& a [ebp+8], bool& b [ebp+0xC])`, `ret 8`; return AL (EAX is the full `_ftol2`
  result on the step paths, `mov al, 1` on the early paths with the rest of EAX = fnstsw/garbage). **Neither argument is
  read or written** (the base 0x403EA0 also ignores them). EBP frame with 8-byte stack alignment.
- Caller: CPhysical::ProcessCollision 0x54DFB0 (virtual). Callees: CRT `ceil` 0x823820 (double on stack -> ST0; identified by its
  x87 fallback using control word 0x1B3F = round up), `_ftol2` 0x821B40.
- Globals: `CTimer::ms_fTimeStep` 0xB7CB5C. Constants: 0.09f 0x85B324, 6.6666665f 0x866FBC, 4.0f 0x858B90, 3.3333333f 0x859F60,
  2.0f 0x858CA0, 5.0f 0x858C80.
- Fields: `m_pAttachedTo` +0xFC, `m_vecMoveSpeed` +0x44/+0x48/+0x4C, `m_fElasticity` +0x9C (f32), `m_pPlayerData` +0x480,
  `m_standingOnEntity` +0x568.
- Behaviour:
  1. if `m_pAttachedTo != null` return 1 (0x5E3EA4).
  2. `pd = m_pPlayerData`. If pd == null: `s2 = ((x*x + y*y) + z*z) * ts * ts` (x87; two separate fmul by the f32 timestep);
     if `s2 < 0.09f` or NaN (`test dl(=1), ah` = C0) return 1 (0x5E3EB4-0x5E3EE9).
  3. `v = sqrt((x*x + y*y) + z*z) * ts` (x87, unrounded) (0x5E3EEF-0x5E3F15).
  4. if pd == null (0x5E3F1B): `n = _ftol2(ceil((double)(v * 5.0f)))` (0x5E3FA8-0x5E3FB9); then `m_fElasticity = m_fElasticity +
     m_fElasticity` (doubled, f32 store, 0x5E3FC8-0x5E3FD0); return n (low byte).
  5. else if `m_standingOnEntity != null`: `k = v * 6.6666665f` (kept as f32 copy `kf`); if `ceil((double)k) < 4.0f`: n = 4; else
     `n = _ftol2(ceil((double)kf))` (ceil recomputed from the f32-rounded copy; equal and NaN cases also take this branch:
     `test ah,5; jp`) (0x5E3F21-0x5E3F71).
     else: same with `k = v * 3.3333333f` and minimum 2 (0x5E3F73-0x5E3FA6).
     Return n (no elasticity change for players).
- Oddities: the elasticity doubling for non-player peds happens on every call (cumulative unless reset elsewhere) - original
  behaviour, keep. A NaN speed gives `_ftol2(NaN)` = 0x80000000 -> AL = 0 steps (caller divides the time step by it).
- gta-reversed: `plugin::CallMethodAndReturn` wrapper only (`Ped.cpp:3747`), hook commented out.

## 0x5E4C50 CPed::CalculateNewVelocity  (catalogue: specified/REA, gr `.Reversed = false` (wrapper), port level 2)
- Convention: `__thiscall void()`, plain `ret`. Caller: CPed::ProcessControl 0x5E8CD0.
- Callees: CGeneral::LimitRadianAngle 0x53CB50 (cdecl(float) -> ST0), SurfaceInfos_c::IsStairs 0x55E7B0, CPedIntelligence::GetTaskUseGun 0x600F70 /
  GetTaskFighting 0x600F30, CRT `_CIasin` 0x821E70 (ST0 -> ST0; identified: `atan2(x, sqrt((1+x)(1-x)))`), CRT `_CIpow` 0x822130 (ST1^ST0).
- Fields: +0x558 m_fCurrentRotation (cur), +0x55C m_fAimingRotation (goal), +0x560 m_fHeadingChangeRate, +0x564 m_fMoveAnim (turn accumulator, "MA"),
  +0x530 m_nPedState, +0x534 m_nMoveState, +0x598 m_nPedType, m_pedIK at +0x50C: +0x518 m_fSlopePitch, +0x520 m_fSlopeRoll, +0x524 m_fBodyRoll,
  +0x528 m_nFlags (bit 3 bSlopePitch); +0x578/+0x57C/+0x580 contact normal N (gr field_578); +0x4D8/+0x4DC m_vecAnimMovingShiftLocal (lx, ly);
  +0x550/+0x554 m_vecAnimMovingShift (out). ts = `CTimer::ms_fTimeStep` (0xB7CB5C). NPC = `m_nPedType` not 0/1.
- Part 1 - heading (skipped entirely when `flags1 & 0x600` (bIsInTheAir | bIsLanding) or state is 54, 55 or 63; then go to Part 2):
  1. `S = (m_fHeadingChangeRate * 0.017453292f [0x8595EC]) * ts` (stored f32). `cur = LimitRadianAngle(cur)` (stored to the ped);
     `g = LimitRadianAngle(goal)` (local only, the ped's goal is not written).
  2. if `cur + pi < g`: `g -= 2pi`; else if `cur - pi > g`: `g += 2pi` (x87, unrounded g). `D = g - cur` (stored f32).
  3. Player: `MA = 1.0f`. NPC: if `D >= 0` (NaN goes to the next test) and `MA < 0`: `MA = 0.1f` (dword copy of 0x86C284); else if `D < 0` and
     `MA > 0`: `MA = -0.1f`.
  4. `t = |MA| * S`; `turned = 0`:
     - if `D > t`: `cur = t + cur`; `MA = ts * 0.1f [0x86C288] + MA`; if `MA > 1.0f` `MA = 1.0f`; turned = 1.
     - else if `-1.0f * t > D`: `cur = cur - t`; `MA = MA - ts * 0.1f`; if `MA < -1.0f` `MA = -1.0f`; turned = 1.
     - else if NPC and `0.1f * S < |D|`: `cur = D * 0.5f + cur`; `MA = MA * 0.5f`.
     - else: `cur = D + cur`; `MA = |D| / S`; if `0.1f > MA` then `MA = 0.1f` (NaN kept).
  5. If NPC and `m_nMoveState` is 1 or 0 and turned and `!GetTaskUseGun()` and `!GetTaskFighting()`: `m_nMoveState = (D > 0) ? 2 : 3`.
     Else if `m_nMoveState` is 2 or 3: `m_nMoveState = 1`.
  6. `m_fBodyRoll = 0`.
- Part 2 - slope (0x5E4F3D), `m = m_matrix` (no null check): `NF = (N.z*m.fwd.z + N.y*m.fwd.y) + N.x*m.fwd.x`;
  `NR = (N.z*m.right.z + N.y*m.right.y) + N.x*m.right.x` (both stored f32). `clamp1(v)` = -1 if `-1 > v`; else 1 if `1 < v`; else v (NaN -> v).
  - If bSlopePitch set: `k = 0`;
    a. state 54 -> ROLL then PITCH.
    b. else if `flags3 (+0x474) & 0x100000` (bFallenDown) -> ROLL then PITCH.
    c. else if `m_nMoveState >= 4` and `!IsStairs(m_nContactSurface)` and `!(flags (+0x478) & 0x4000)` (bPedHitWallLastFrame): `m_fSlopeRoll = 0`; PITCH.
    d. else (decay): if `pitch != 0` or `roll != 0` (exact): `k = pow(0.9f [0x86C280], ts)` (f32). `pitch = |pitch| > 0.01f ? pitch * k : 0`;
       `roll = |roll| > 0.02f ? roll * k : 0`.
    ROLL: `m_fSlopeRoll = asin(clamp1(NR))`. PITCH: `m_fSlopePitch = asin(clamp1(NF)) * (1.0f - 0.75f) + 0.75f * m_fSlopePitch` (0x86C278 = 0.75f;
    `1-0.75` computed in x87).
  - Else (bSlopePitch clear): if `m_nMoveState >= 4` and NPC: if `|roll| > 0.02f` then `roll *= pow(0.9f, ts)` (stored) - and then
    **unconditionally `roll = 0`** (0x5E519F follows the multiply), so the multiply is dead. Otherwise roll unchanged.
- Part 3 - anim shift (0x5E51A5):
  `a = 1.0f - NF*NF`, `if (0 > a) a = 0`, `sf = sqrt(a)` (stored f32); `b = 1.0f - NR*NR`, `if (0 > b) b = 0`; `animShift = (0, 0)`;
  `v = sf * ly`; `animShift.x = v*m.fwd.x + 0`; `animShift.y = v*m.fwd.y + 0`; `w = sqrt(b) * lx`; `animShift.x += w*m.right.x`; `animShift.y += w*m.right.y`
  (matrix re-read; products stored through f32 temporaries as in the bytes).
  If `ts < 0.01f` and `!CTimer::bSlowMotionActive` (0xB7CB88): `animShift *= 0.01f` (both components). Else `r = 1.0f / ts`; `animShift *= r`.

## 0x5E5380 CPed::DoFootLanded  (catalogue: specified/REA, gr plugin::Call wrapper, port level 9)
- Convention: `__thiscall void(bool leftFoot [esp+4], uint8 makeWobble [esp+8])`, `ret 8`. Caller: CPed::PlayFootSteps 0x5E57F0 (2).
- Behaviour:
  1. Return unless `flags3 (+0x474) & 0x400` (bCalledPreRender) is set and `m_nFlags (+0x1C) & 0x800000` (m_bDontUpdateHierarchy) is clear.
  2. `B = GetBonePosition(leftFoot ? 43 : 53, false)` (0x5E4280, thiscall(out, bone, bool)). `F = m_matrix->forward`, `R = m_matrix->right` (copies;
     matrix used without null check). `P.x = F.x*0.2f + B.x; P.y = B.y + F.y*0.2f; P.z = F.z*0.2f + (B.z - 0.1f)` (0.2f 0x858CC4, 0.1f 0x858B1C).
  3. If `flags1 & 0x10000000` (bDoBloodyFootprints) and `CLocalisation::Blood()`: `CShadows::AddPermanentShadow(1, gpBloodPoolTex (0xC40400), &P,
     F.x*0.26f, F.y*0.26f, R.x*0.14f, R.y*0.14f, 255, 200, 0, 0, 4.0f, 3000, 1.0f)` (0x706F60; 0.26f 0x86C338, 0.14f 0x86C33C);
     then `if ((uint32)+0x750 > 20) +0x750 -= 20; else { +0x750 = 0; flags1 &= ~0x10000000; }` (counter at +0x750, plugin-sdk name m_nDeathTime).
  4. If `g_surfaceInfos.LeavesFootsteps(m_nContactSurface)` and `sqrt((cam.y-pos.y)^2 + (cam.x-pos.x)^2) < 10.0f` (2D, camera = TheCamera matrix pos
     or placement): `AddPermanentShadow(1, gpShadowPedTex (0xC403E4), &P, F.x*-0.26f, F.y*-0.26f, R.x*-0.1f, R.y*-0.1f, 120, 250, 250, 50, 4.0f,
     isPlayer ? 5000 : 2000, 1.0f)` (isPlayer = pedType 0/1; -0.26f 0x86C334, -0.1f 0x858EF4).
  5. If `CWeather::Rain > 0.1f` and `!CCullZones::CamNoRain()` and `!CCullZones::PlayerNoRain()` and `CGame::currArea == 0`:
     `sub_5E3630(EBX = this, EDI = &P, EAX = 4)` with a dummy stack value 0.15f pushed and popped by the caller (the callee ignores it).
  6. Always: `sub_5E37C0(ESI = this, EBX = &P, EAX = 4)` (dummy stack value 0 pushed/popped).
  7. If `makeWobble`: `m_Wobble (+0x758) = 6.2831855f`; `m_WobbleSpeed (+0x75C) = (shift.x*shift.x + shift.y*shift.y) * 20.0f + 0.4f`
     (shift = m_vecAnimMovingShift +0x550).
  8. Splash: if `physicalFlags (+0x40) & 0x100` (submerged): `pos = this position`; `CWaterLevel::GetWaterLevel(pos.x, pos.y, pos.z + 1.5f, &w, true, 0)`
     (0x6EB690, cdecl; return ignored); `S = (ts*moveSpeed.x*2 + pos.x, ts*moveSpeed.y*2 + pos.y, w)` (`a + a` doubling in x87); if NOT
     `pos.z - 0.4f > w` return (0x5E570E-0x5E5721). Else if `g_surfaceInfos.IsShallowWater(m_nContactSurface)`: `S = P` with `S.z = 0.3f + P.z`
     (0.3f from .data 0x8D21EC), `top = GetTopDirection()` (0x41CCB0); `S += top * k` with k = .data 0x8D21E8 (-0.2f) (`S.x = top.x*k + S.x`, ...).
     Else return.
     Then `g_fx.TriggerFootSplash(&S)` (0x4A1150) and `m_pedAudio (+0x138).AddAudioEvent(54, 0.0f, 1.0f, null, 39, 0, 0)` (0x4E2BB0).

## 0x5E57F0 CPed::PlayFootSteps  (catalogue: specified/REA, gr `.Reversed = false` (plugin wrapper + commented draft), port level 10)
- Convention: `__thiscall void()`, plain `ret`. SEH frame (stack CEventSoundQuiet). Body split: the common exit jumps to 0x4072B1 and back to 0x5E5E6F.
  Caller: CPed::ProcessControl 0x5E8CD0.
- Notation: assoc fields: +0x14 m_pHierarchy (+0x10 m_fTotalTime), +0x18 m_fBlendAmount, +0x20 m_fCurrentTime (ct), +0x28 m_fTimeStep (dt),
  +0x2C m_nAnimId (i16), +0x2E m_nFlags (u16). `AE(e, vol, spd)` = `m_pedAudio (+0x138).AddAudioEvent(e, vol, spd, 0, 0, 0, 0)` (0x4E2BB0), only
  called when the byte at +0x1B4 (inside m_pedAudio) is non-zero. ducking = `flags1 & 0x04000000`. LANDING = step 9.
- Behaviour:
  1. `first = RpAnimBlendClumpGetFirstAssociation(m_pRwClump)` (0x4D15E0, no null check); `wrs = first->m_nAnimId in {0, 1, 2}` (byte).
     `skater = (m_pStats (+0x59C) == CPedStats::ms_apPedStats + 0x820)` (stat index 40).
  2. if `flags1 & 0x10000000` (bDoBloodyFootprints) and `0 < +0x750 < 300` (unsigned): `+0x750 -= 1`; if it becomes 0 clear that flag bit.
  3. if not standing (`flags1 & 1` clear) return.
  4. Loop over all associations from `first` (next via 0x4D6AB0): if `flags & 0x100`: `walkTotal += blend; walk = assoc` (last such assoc wins);
     else if `!(flags & 0x400)` and `animId != 223` and (`flags & 0x10` or !ducking): `idleTotal += blend` (x87 adds, f32 stores).
     If `walk == null` or NOT `walkTotal > 0.5f` or NOT `idleTotal < 1.0f` -> LANDING.
  5. `T = walk->m_pHierarchy->m_fTotalTime`; `minT = T * 0.06666667f` (0x863E0C, x87); `maxT = T * 0.5f + minT` (stored f32); if ducking:
     `minT += 0.2f`, `maxT += 0.2f`.
  6. Skater: `K = (walk->m_nAnimId == 0) ? 0.53333336f : 0.33333334f`; `g = GetAdhesionGroup(m_nContactSurface)`:
     g == 3: if `rand() & 0x7F` then `m_vecAnimMovingShiftLocal (+0x4D8/+0x4DC) *= 0.5f`; `mult = 0.5f`. g == 4: if `rand() & 0x3F` then shiftLocal *= 0.2f;
     LANDING. g == 5: shiftLocal *= 0.3f; LANDING. Else `mult = 1.0f`.
     If `ct > 0` and `ct - dt <= 0`: `AE(56, log10(mult) * 20.0f, walk->m_nAnimId != 0 ? 1.0f : 0.75f)` (log10 via `fldlg2; fyl2x`); LANDING.
     Else if `mult > 0.2f` and `ct > K` and `ct - dt <= K`: `AE(57, same vol, same speed)`. LANDING.
  7. First foot: if `minT <= ct` and `ct - dt < minT`:
     a. if `IsPlayer()` (0x5DF8F0) and `m_pPlayerData`: `bal = pd->m_pPedClothesDesc->GetIsWearingBalaclava()`; `ms = m_nMoveState`:
        ms == 7 -> vol = bal ? 65 : 55; ms in {5, 6}: `r = pd->m_fMoveBlendRatio (+0x14)`: if `r >= 2.0f` -> vol = bal ? 55 : 45; else if bal and `r > 1.1f`
        -> vol = (r - 1)*20 + 30; else if `r > 1.5f` -> vol = (r - 1)*15 + 30; else no event; computed vols must be > 0. Other ms: no event.
        If a vol was chosen: stack `CEventSoundQuiet(this, vol, -1, (0,0,0))` (0x5E05B0) added with `GetEventGlobalGroup()->Add(&ev, 0)`; destroy (0x5DEA00).
     b. (vol, spd) = ducking ? (-18, 0.8) : ms == 6 ? (-6, 1.1) : ms == 7 ? (0, 1.2) : (-12, 0.9); if not ducking and `m_nAnimGroup == 69`:
        `vol -= 6.0f; spd -= 0.1f`. `AE(54, vol, spd)`. `DoFootLanded(true, wrs)`. LANDING.
  8. Second foot (also reached when the first test fails): if `ct >= maxT` and `ct - dt < maxT`: same (vol, spd) selection; `AE(55, vol, spd)`;
     `DoFootLanded(false, wrs)` (no CEventSoundQuiet for this foot). LANDING.
  9. LANDING: if `flags1 & 0x400` (bIsLanding): `t = m_TaskMgr.GetSimplestActiveTask()` (0x6819D0, no null check); if `t->GetTaskType() == 242`:
     if `CTaskSimpleLand::RightFootLanded(t)` -> `DoFootLanded(false, true)`; else if `LeftFootLanded(t)` -> `DoFootLanded(true, true)`.
- gta-reversed: only a commented-out draft; it differs from the bytes (e.g. idle sum uses `partial || bIsDucking`, the original `partial || !ducking`;
  skater K values swapped; landing tests RightFootLanded twice). Port from the bytes.

## 0x5E88E0 CPed::CanWeRunAndFireWithWeapon  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall bool()`, return AL (bit 0 of the flags, upper EAX bits = flags >> 1), plain `ret`. Caller: CCamera::CamControl 0x527FA0.
- Behaviour: `t = m_aWeapons[(int8)m_nActiveWeaponSlot (+0x718)].m_Type` (+0x5A0 + slot*0x1C); `info = CWeaponInfo::GetWeaponInfo(t,
  this->GetWeaponSkill(t) /*0x5E3B60, thiscall*/)` (0x743C60, cdecl); `return (info->m_nFlags (+0x18) >> 1) & 1` (bit 1 = `bAimWithArm` in gr `WeaponInfo.h:57`; the
  0x5E88FF-0x5E8907).

## 0x5E8C80 CPed::vfunc_0 = scalar deleting destructor  (catalogue: specified/REA, port level 16)
- Convention: virtual slot 0, `__thiscall CPed*(uint8 flags [esp+4])`, `ret 4`, returns this in EAX.
- Behaviour: `CPed::Destructor` 0x5E8620. If `flags & 1`, inlined `CPools::ms_pPedPool->Delete(this)` (pool 0xB74490: +0 storage,
  +4 slot-state bytes, +0xC first-free index): `idx = (int32)(this - storage) / 0x7C4` (signed magic multiply by 0x41EE7CA7, `sar 9`
  of EDX, plus the sign bit: truncating division; exact for in-pool pointers) (0x5E8C96-0x5E8CAE); `slot[idx] |= 0x80`;
  `if (idx < firstFree) firstFree = idx` (signed) (0x5E8CB3-0x5E8CBE). No destructor-only path difference: when the flag is clear
  only the destructor runs.

## 0x5E8CD0 CPed::ProcessControl  (catalogue: specified/REA, gr hook commented out (plugin wrapper), port level 31)
- Convention: virtual `__thiscall void()`, plain `ret`. Callers: CCivilianPed / CCopPed / CEmergencyPed / CPlayerPed ::ProcessControl.
- Notation: F1 = +0x46C, F2 = +0x470, F3 = +0x474, F4 = +0x478 (ped flag dwords; names from gr bitfield order); `W = m_aWeapons[(int8)+0x718]`;
  `dtMs = _ftol2(ts * 0.02f * 1000.0f)` (x87). RETURN = plain return (skips everything below, including the final weapon-audio service).
- Behaviour (in order):
  1. `m_pedAudio (+0x138).Service()` (0x4E2EE0).
  2. Players (type 0/1) with `W.type == 18` (molotov): if `F3 & 2` (bDontRender) or entity flag bit 7 clear (`byte +0x1C` sign, m_bIsVisible) or
     (`physicalFlags & 0x100` and `m_pIntelligence->GetTaskSwim()`) or `m_pWeaponObject (+0x4F4) == null`: if `W.m_FxSystem (+0x18)`:
     `g_fxMan.DestroyFxSystem(fx)`, clear. Otherwise, if `W.m_FxSystem == null`: `h = GetAnimHierarchyFromSkinClump(clump)`;
     `m = RpHAnimHierarchyGetMatrixArray(h) + RpHAnimIDGetIndex(h, 24) * 64`; `fx = g_fxMan.CreateFxSystem("molotov_flame", &(0,0,0), m, false)`
     (0x4A9BE0); store; if fx: `SetLocalParticles(1)`, `CopyParentMatrix()`, `Play()`.
  3. If `F4 & 0x200000` (bUsedForReplay) RETURN. If `F3 & 0x1000` (bPartOfAttackWave): RETURN if `CGame::currArea != 0` or
     `CWorld::FindPlayerCoors(&tmp, -1)->z > 950.0f` (0x858F4C).
  4. If `((u8)byte[+0x20] + CTimer::m_FrameCounter) & 0x1F == 0`: `CEntity::PruneReferences()` (0x571A90).
  5. Alpha: `a = GetClumpAlpha(clump)`; if `F2 & 8` (bFadeOut): `a -= 8`, if negative `a = 0`; else if `a < 255`: `a += 16`, if `> 255` a = 255.
     `SetClumpAlpha(clump, a)`.
  6. If `F2 & 0x80000000` (bKnockedOffBike) and (`F1 & 1` or `F2 & 0x80000`) and `m_standingOnEntity == null` and `!(F2 & 0x200000)` and
     `+0x588 == 99999.992f` (dword compare with 0x47C34FFF) and `m_vecMoveSpeed.SquaredMagnitude() < 0.01f`:
     if `m_pVehicle`: if `!CCollision::ProcessColModels(this->m_matrix, thisColModel, veh->m_matrix, vehColModel, colPoints 0xC092A8, 0, 0, 0)`
     (0x4185C0; col models via ms_modelInfoPtrs[model]->+0x14): clear F2 bit 31 and `m_pEntityIgnoredCollision (+0x128) = 0`. Else (no vehicle):
     clear F2 bit 31.
  7. If `F2 & 0x200000` (bHeadStuckInCollision) and `F4 & 0x100` (bCheckColAboveHead) and `pos.z + 1.5f < +0x588`: clear F2 bit 21.
  8. Reset per-frame flags: `F1 &= ~0x10002`; `F2 &= ~0x80000` (value kept in a local `f2`); `physicalFlags &= ~0x100`; `F3 &= ~0x80000001`;
     `F4 &= ~0x104100`; `+0x588 = 99999.992f`.
  9. Gun flash fade, for (alpha, prog) = (+0x504, +0x506) then (+0x508, +0x50A) (int16): if `alpha > 0`: `t = dtMs`; if `(uint32)(int32)alpha >
     (uint32)(prog * t)`: `alpha -= t * prog` (16-bit) else `alpha = 0`.
  10. If not standing: `F2 = f2 & ~0x200000`.
  11. If `m_nCreatedBy (+0x484) == 2` and `this != FindPlayerPed(-1)` and the player's group membership `IsMember(this)`: `F4 &= ~0x100`.
  12. `ProcessBuoyancy()` (0x5E1FA0).
  13. If `m_pPlayerData`: if `physicalFlags & 0x100` and `pd->m_nWaterCoverPerc (+0x8E, u8) > 50`: if `(int8)pd->m_nWetness (+0x8C) < 100` it++;
      else if `(int8)m_nWetness > 0` it--.
  14. If standing and `!CWorld::bForceProcessControl` and `s = m_standingOnEntity` and (`s->m_nFlags & 0x20` or (s is a vehicle and `s->+0x594 == 6`)):
      `m_nFlags |= 0x40` (m_bWasPostponed); RETURN.
  15. `m_pIntelligence->ProcessFirst()` (0x6073A0).
  16. If not standing and `m_vecMoveSpeed.z > 0.25` (f64 compare): if `m_pPlayerData` -> `moveSpeed.z = 0.25f`; else
      `moveSpeed = moveSpeed * pow(0.95 (f64 0x86C3C8), ts)` (via operator*(v, float) 0x40FEC0; f32 store).
  17. If NPC and standing and `moveSpeed == (0,0,0)` (exact, each component) and `m_nMoveState` is 1 or 0 and `m_vecAnimMovingShiftLocal == (0,0)`
      and `m_nPedState != 41` and `!(F1 & 0x200)` (bIsInTheAir) and `m_standingOnEntity == null`: `CPhysical::SkipPhysics()` (0x5433B0);
      else `CPhysical::ProcessControl()` (0x5485E0).
  18. `RequestDelayedWeapon()` (0x5E8910); `PlayFootSteps()`; `F3 &= ~0x108000` (bTestForBlockedPositions, bFallenDown); `m_pIntelligence->Process()`
      (0x608260); if `m_nPedState != 55` `CalculateNewVelocity()`; `UpdatePosition()`; virtual slot 23 (`vtable+0x5C`, SetMoveAnim).
  19. `F3 &= ~0xF0000` (arm/midriff blocked bits).
  20. Blood drips: if (`F1 & 0x40000` (bPedIsBleeding) or `field_72F` != 0) and `CLocalisation::Blood()` and not in vehicle: if `field_72F` then it--;
      if `(CTimer::m_FrameCounter & 3) == 0` and `VectorSub(pedPos, camPos).SquaredMagnitude() < 2500.0f`: `size = (rand() & 0x7F) * 0.0015f + 0.15f`;
      `p.x = ((rand() & 0x7F) - 64) * 0.007f + m_matrix->pos.x`; `p.y = ((rand() & 0x7F) - 64) * 0.007f + m_matrix->pos.y`; `p.z = m_matrix->pos.z + 1.0f`;
      `time = (rand() & 0xFFF) + 2000` (rand order: size, x, y, time; fild of ints; constants 0x863E38, 0x858FCC, 0x86C3C0);
      `CShadows::AddPermanentShadow(1, gpBloodPoolTex, &p, size, 0, 0, -size, 255, 200, 0, 0, 4.0f, time, 1.0f)`.
  21. If `F1 & 0x100`: if `m_pVehicle == null` clear that bit (no pedcount call); else `CPopulation::UpdatePedCount(this, 1)`. Else `UpdatePedCount(this, 0)`.
  22. If `((u16)word[+0x20] + m_FrameCounter) & 0x3FFF == 0` and `F4 & 0x400` (bDruggedUp): `Say(0x51, 0, 1.0f, 0, 0, 0)`.
  23. If `W.type == 9` (chainsaw) and `m_nPedState != 16` and not in vehicle and `!GetTaskSwim()`: `m_weaponAudio (+0x394).AddAudioEvent(153)`.
      `m_weaponAudio.Service()` (0x4E6AE0).

## 0x5F0360 CPed::KillPedWithCar  (catalogue: specified/REA, gr plugin wrapper, port level 17)
Instruction-level spec (1139 instructions, 0x5F0360-0x5F1309). Local names: `F+x` = `[esp+x]` measured with esp as it is after the
prologue's `push edi` (0x5F0386); raw operands inside push sequences are shifted by the pushed bytes (already normalised here).
- Convention: `__thiscall void(CVehicle* car [F+0xE8], float damageIntensity [F+0xEC], bool playDeadAnim [F+0xF0])`, `ret 0xC`.
  SEH frame (handler 0x83DE00; state dword F+0xE0: 0..3 during the two damage constructions, -1 after). this = ESI (CPed*), car = EDI
  throughout. Callers: CPhysical::ProcessCollisionSectorList 0x54BA60 (2), CCollisionEventScanner::ScanForCollisionEvents 0x604500 (3).
- Callees (via `orig!`): CTaskManager::GetSimplestActiveTask 0x6819D0, task vfunc +0x10 GetTaskType, CTaskManager::FindActiveTaskByType 0x681740,
  CPed::IsPlayer 0x5DF8F0, CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, false)), CVector::Magnitude 0x4082C0, `_ftol2` 0x821B40 (truncating
  float->int64, low dword used), CPad::GetPad 0x53FB70, CPad::StartShake 0x53F920, CPed::GetLocalDirection 0x5DEF60 (thiscall(const CVector2D*) -> int 0..3),
  CEntity::GetColModel 0x535300, CVector::DotProduct 0x40FDB0 (cdecl(a*, b*) -> ST0), _rand 0x821B1E, CVector::Normalise 0x59C910,
  CAutomobile::RemoveBonnetInPedCollision 0x6AA200 (-> CObject* or null), CPhysical::ApplyTurnForce 0x542A50 (force, point by value),
  CLocalisation::KnockDownPeds 0x56D290, CAEVehicleAudioEntity::AddAudioEvent 0x4F6420 (thiscall(int id, float vol), `ret 8`),
  CCrime::ReportCrime 0x532010 (cdecl(type, victim, criminal)), CPedDamageResponseCalculator ctor 0x4AD3F0 (thiscall(entity, float damage, weapon,
  piece, bool speak), `ret 0x14`) / dtor 0x4AD420, CEventDamage ctor 0x4AD830 (thiscall(source, startTime, weapon, piece, uint8 direction,
  bool jumpedOutOfMovingCar, bool pedInVehicle), `ret 0x1C`) / dtor 0x4AD960, CEventDamage::AffectsPed 0x4B35A0, ComputeDamageResponse 0x4B5AC0
  (thiscall(ped, CPedDamageResponse*, bool speak), `ret 0xC`), CAnimManager::BlendAnimation 0x4D4610 (cdecl(clump, group, id, float blend)),
  CEventGroup::Add 0x4AB420 (thiscall(event*, bool), `ret 8`), CPed::Say 0x5EFFE0 (6 args, `ret 0x18`), CPhysical::ApplyForce 0x542B50
  (force, point by value, bool), helper vector functions below.
- Helpers (all exact x87 orders, verified from their bytes): 0x406DA0 `SquaredMagnitude` (thiscall, ST0 = (x*x + y*y) + z*z);
  0x40FE30 `out = a + b` (cdecl(out*, a*, b*), per component `a.c + b.c`); 0x40FE60 `out = a - b`; 0x40FE90 `out = f * v` (cdecl(out*, float f, v*),
  per component `f * v.c`); 0x40FEC0 `out = v * f` (cdecl(out*, v*, float f), `f * v.c`); 0x406D70 `this -= v` (thiscall, `this.c - v.c`);
  0x411A00 `this += v` (thiscall, computes `v.c + this.c`); **0x404330 = min(float a, float b)** (cdecl, ST0): `fld a; fcomp b; test ah,5; jp -> b`,
  i.e. returns a iff a < b, otherwise b (also b when a == b or either is NaN). Confirmed.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84 (R). Constants: 0x866FC0 0.0225f, 0x858CCC 12.0f, 0x858624 1.0f, 0x86C688 200000.0f,
  0x858A14 80.0f, 0x859F80 250.0f, 0x858B50 0.0f, 0x862CD0 0.99f, 0x862E1C 0.85f, 0x858B1C 0.1f, 0x858FDC -0.2f, 0x858B8C 0.5f, 0x858C84 0.25f,
  0x8631F8 0.002 (f64), 0x86C680 1.5 (f64), 0x858CC4 0.2f, 0x859000 8.0f, 0x858B90 4.0f, 0x8599D0 16.0f, 0x85A310 1.0 (f64), 0x858C9C -0.8f,
  0x858B3C 3.0f, 0x858B44 6.0f, 0x85862C 10.0f, .data 0x8D22A0 1600.0f, 0x8D22A4 0.25f, 0x8D22A8 -0.5f, 0x8D22AC -0.75f (.data: may be patched).
- Fields: ped +0x14 m_matrix (pos +0x30), +0x4 placement pos (fallback when m_matrix null), +0x18 m_pRwClump, +0x1C m_nFlags (bit 12 0x1000
  bHasHitWall), +0x44 m_vecMoveSpeed, +0xE8 m_vecLastCollisionImpactVelocity.z, +0x128 m_pEntityIgnoredCollision, +0x46C/+0x470 ped flag dwords
  (0x46C bit0 bIsStanding, bit8 bInVehicle; 0x470 bit4 0x10 bKnockedUpIntoAir), +0x47C m_pIntelligence (task manager at +4, event group at
  +0x68, byte +0x260), +0x584 contact entity, +0x58C m_pVehicle, +0x598 m_nPedType. Car: +0x14 matrix (right +0x0, forward +0x10, up +0x20,
  pos +0x30), +0x44 m_vecMoveSpeed, +0x8C m_fMass, +0x138 audio entity, +0x384 handling (byte +0xCC: model flags, bit 3 0x8 = IS_BIG per gr enum),
  +0x460 m_pDriver, +0x590 vehicle class (0 automobile, 5 boat, 9 bike), +0x594 subtype (4 plane, 6 train). Col model: +0x4 bbox.min.y,
  +0xC max.x, +0x10 max.y, +0x14 max.z.
- Locals: F+0x1C/20/24 `d` = ped pos - car pos; F+0x13 u8 hitZone; F+0x14 scratch; F+0x18, F+0x28, F+0x2C float scratch; F+0x30 int dir;
  F+0x43 u8 rnd = rand() & 3; F+0x50 weapon (init 49 WEAPON_RAMMEDBYCAR); F+0x60 int animId (init 0xBF = 191 ANIM_ID_NO_ANIMATION_SET);
  F+0x64/68 CVector2D; F+0x70 float animSpeed (init 1.0f); F+0x80 CEventDamage (0x44 bytes; +0x25 flags byte, +0x28 animGroup, +0x2C animId,
  +0x30 blend, +0x34 speed, +0x38 CPedDamageResponse at F+0xB8); F+0xC4 CPedDamageResponseCalculator.

### Block 1 - gates (0x5F0380-0x5F0472)
1. 0x5F038A: `t = intel->taskMgr.GetSimplestActiveTask()`; if t (fetched again at 0x5F039C) and `t->GetTaskType()` is 207 (TASK_SIMPLE_FALL)
   or 212 (TASK_SIMPLE_DIE) -> block X (0x5F12D1); if 218 (TASK_SIMPLE_DEAD) -> return (0x5F12EE).
2. 0x5F03C9: `c = ped+0x584`; if c and `(c+0x36 & 7) == 2` (vehicle): return if `c+0x590 == 5` (boat) or `IsPlayer()`; otherwise continue at
   step 3 (the m_pVehicle test is skipped). Else (c null / not a vehicle): if `ped+0x58C != null && == car`: return if `car+0x590 == 5` or
   `car+0x594 == 4`.
3. 0x5F042B: `t = FindActiveTaskByType(1004)` (0x3EC, gr name TASK_COMPLEX_DESTROY_CAR_MELEE); if t and `t+0x10 == car`: `s = SquaredMagnitude(copy of
   car->moveSpeed)` (copy at F+0x44); return if `s < 0.0225f` (`test ah,5; jnp`, strictly less).

### Block 2 - offset, strength split (0x5F0478-0x5F04E1)
`cp = car->m_matrix ? &mat.pos : &car->placement.pos`, same for ped. `d.z = ped.z - car.z; d.y = ...; d.x = ...` (computed z, y, x; stored
F+0x1C x, F+0x20 y, F+0x24 z). F+0x60 = 0xBF, F+0x70 = 1.0f. Strong path iff `damageIntensity > 12.0f` (0x5F04C7, `test ah,0x41; jne` -> NaN and
equality take the weak path) **and** `!IsPlayer()`; otherwise -> weak path (block 6, 0x5F0E0D).

### Block 3 - strong path set-up and pad shake (0x5F04E7-0x5F05C6)
F+0x13 = 0 (AL of IsPlayer), F+0x43 = `rand() & 3`, F+0x50 = 49. If `car == FindPlayerVehicle(-1, false)`: `inv = 1.0f / car->m_fMass` (F+0x28);
`v = |car.moveSpeed| * inv * 200000.0f + 80.0f`; if `v > 250.0f` use 250.0f else recompute the same expression (second Magnitude call);
`s = (uint8)_ftol2(v)` (F+0x14 low byte); `CPad::GetPad(0)->StartShake((int16)(40000 / s) (signed idiv), s, 0)` (pushed: 0, dword F+0x14 whose
upper bytes are stale, quotient). Then `ped.flags0x46C &= ~1` (bIsStanding = false); F+0x64 = -car.moveSpeed.x, F+0x68 = -car.moveSpeed.y;
`F+0x30 = dir = GetLocalDirection(&F+0x64)`.

### Block 4 - impact geometry, throw, bonnet, runover (0x5F05CA-0x5F0BC2)
`cm = car->GetColModel(); m = car->m_matrix` (not null-checked); F+0x14 = cm.max.x (raw dword).
`R = (d.z*m.right.z + d.y*m.right.y) + d.x*m.right.x` (F+0x18); `U = (d.z*m.up.z + d.y*m.up.y) + d.x*m.up.x` (F+0x28) (x87 order as written).
- 0x5F061C: if `car+0x594 == 6` (train) -> **runover** with hitZone = 2 (enters 0x5F0B6A).
- `f = DotProduct(&car.moveSpeed, &m.forward)`; if `f < 0.0f` -> 0x5F0BAF (no throw: audio 0x77 then damage, F+0x13 = 0, weapon 49, dir raw).
- `a = |R|` (fchs only when `R < 0`); if `a > cm.max.x * 0.99f` (0x5F066F, NaN/equal fail) -> **side hit**: F+0x13 = 4, and = 3 unless
  `R > 0` (0x5F0689); `Fw = (d.z*m.fwd.z + d.y*m.fwd.y) + d.x*m.fwd.x` (F+0x14); `cm2 = GetColModel()`; if `cm2.max.y * 0.85f > |Fw|`
  -> **runover** keeping hitZone (enters 0x5F0B6F) else -> 0x5F0BAF.
- else (front/back, 0x5F06E3): if rnd != 0: if `!(U > 0.1f)` or `rnd <= 1` -> runover hitZone 2 (0x5F0B5F). Then if
  `car->m_pHandling->byte+0xCC & 8` -> runover hitZone 2. Otherwise **bonnet/roof throw**, F+0x13 = 1:
  `maxY = cm.max.y` (F+0x28), `minY = cm.min.y` (F+0x18), `maxZ = cm.max.z` (F+0x2C) (three GetColModel calls), `m = car->m_matrix`, `pos = &m.pos`
  (the placement fallback is dead: m was dereferenced). Then by `m.fwd.z` (+0x18):
  - `fwd.z < -0.2f` (0x5F074D; equal/NaN go on): `top = ((pos + maxZ*up) + minY*fwd)` (0x40FE90/0x40FE30: t1 = minY*fwd, t2 = maxZ*up,
    s = pos + t2, top = s + t1); F+0x2C = top.z; `L = maxY - minY` (F+0x18).
  - else if `fwd.z > 0.1f` (0x5F07DB): `top = (pos + maxZ*up) + maxY*fwd`; F+0x2C = top.z; L = maxY (raw). Then with `pp = ped pos (matrix or
    placement)`: if `(top.z - pp.z) > 0.0f`: `ped.matrix.pos.z = (top.z - pp.z) * 0.5f + ped.matrix.pos.z` (writes through ped->m_matrix
    without a null check, 0x5F089C), then `top.z = (top.z - pp.z') * 0.25f + top.z` with pp re-read after the write.
  - else: `top = pos + maxZ*up`; F+0x2C = top.z; L = `GetColModel()->max.y` (fresh call).
  0x5F0917: `t = L / |car.moveSpeed|` (`fdivr`); `lift = (top.z - pp.z) / t` (F+0x18). `r = rand() & 0xFF`;
  `lift = ((double)r * 0.002 + 1.5) * lift` (x87: fild, fmul f64, fadd f64, fmul f32, fstp f32). `n = Normalise(copy of car.moveSpeed)` (in F+0x1C,
  overwriting d), `w = (lift * 0.2f) * n` (0x40FE90), `ped.moveSpeed = (w.x, w.y, w.z + lift)` (z: `w.z + lift`). Then
  `dir = F+0x30 + 2; if (dir > 3) dir -= 4` (signed).
  0x5F09EB: if `car+0x590 == 0` and `b = RemoveBonnetInPedCollision()` non-null: if `rand() & 1`: `b.moveSpeed = (ped.moveSpeed + 0.1f*m.right)
  + 0.5f*m.up` else `(ped.moveSpeed - 0.1f*m.right) + 0.5f*m.up` (matrix re-read from car+0x14 for each operand); then
  `b->ApplyTurnForce(10.0f * m.up, m.forward)` (force first, point second, both by value; ECX = bonnet). 0x5F0B11: recompute `d = ped pos - car pos`
  (VectorSub, same values as before). If `KnockDownPeds()`: `car.audio.AddAudioEvent(0x7A, 0.0f)`. -> block 5 (no 0x77 event on this path).
- **runover** (0x5F0B5F / 0x5F0B6A / 0x5F0B6F): hitZone = 2 (only at entries 0x5F0B5F/0x5F0B6A), F+0x50 = 50 (WEAPON_RUNOVERBYCAR),
  `ped.moveSpeed = car.moveSpeed * 0.9f` (0x40FEC0), `ped.moveSpeed.z = 0`; if dir (raw GetLocalDirection value) is 1 or 3: dir = 2.
  Falls into 0x5F0BAF.
- 0x5F0BAF: if `KnockDownPeds()`: `AddAudioEvent(0x77, 0.0f)`.

### Block 5 - crime and damage event, strong path (0x5F0BC7-0x5F0E08)
- If `car->m_pDriver`: `CCrime::ReportCrime(ped.type == 6 ? 11 : 10, ped, driver)` (CRIME_KILL_COP_PED_WITH_CAR / CRIME_KILL_PED_WITH_CAR).
- **0x5F0C00**: `calc(F+0xC4) = CPedDamageResponseCalculator(car, 1000.0f, weapon(F+0x50), 3 /*torso*/, false)`.
- **0x5F0C39**: `ev(F+0x80) = CEventDamage(car, m_snTimeInMilliseconds, weapon, 3, (uint8)dir, false, (ped.flags0x46C >> 8) & 1 /*bInVehicle*/)`
  (pushed dword is `(flags >> 8) & 0xFFFFFF01`; only the low byte is read).
- Anim choice, `rnd2 = rand() & 3` (0x5F0C46), switch on dir (jump table 0x5F130C = {0x5F0C59, 0x5F0C91, 0x5F0C84, 0x5F0CA1}):
  dir 0: `hz==3 && rnd2>1 -> 25 (KO_SPIN_R)`, `hz==4 && rnd2>1 -> 27 (KO_SPIN_L)`, else 24 (KO_SKID_FRONT); dir 1: 22 (KD_LEFT);
  dir 2: `hz==3 && rnd2>1 -> 22`, `hz==4 && rnd2>1 -> 23`, else 26 (KO_SKID_BACK); dir 3: 23 (KD_RIGHT); dir > 3 (unsigned; unreachable): F+0x60 (191).
- If `ev.AffectsPed(ped)`:
  - weapon == 49: `blend = |car.moveSpeed| * 8.0f + 4.0f` (F+0x28; F+0x70 stays 1.0f). Else: `blend = |v| * 12.0f + 4.0f`;
    `speed = |v| * 16.0f + 1.0` (f64 constant, stored f32 to F+0x70) (two Magnitude calls).
  - `calc.ComputeDamageResponse(ped, &ev.damageResponse, true)`; `ev.animGroup = 0; ev.animId = anim; ev.blend = blend; ev.speed = F+0x70`.
  - if playDeadAnim: `a = BlendAnimation(ped.clump, 0, anim, blend); a->m_fSpeed(+0x24) = F+0x70; ev.flags(+0x25) |= 4` (m_bAnimAdded).
  - `intel.eventGroup.Add(&ev, false)`; if `ped+0x128 == null`: `ped+0x128 = car` (no RegisterReference).
  - `ped.flags0x470` bit 0x10 (bKnockedUpIntoAir) = (hitZone == 1); `intel byte +0x260 = 1`.
  Else: `ped.flags0x470 &= ~0x10`.
- `Say(0x156, 0, 1.0f, false, false, false)`; destroy ev, calc (0x5F1132) -> block 7 (0x5F1155).

### Block 6 - weak path (0x5F0E0D-0x5F112A)
- Gate: continue iff `(ped+0xE8 < -0.8f && damageIntensity > 3.0f) || (damageIntensity > 6.0f && (!IsPlayer() || damageIntensity > 10.0f))`
  (0x5F0E13 `jp` skips the first clause for >=, NaN; all other compares fail on NaN/equality); otherwise return.
- `ped.flags0x46C &= ~1`; F+0x64/68 = -car.moveSpeed.xy; `dir = GetLocalDirection(&F+0x64)` (EBP). `dmg = 30.0f`, or 150.0f if
  `IsPlayer() && car+0x594 == 6`.
- **0x5F0ED7**: `calc = CPedDamageResponseCalculator(car, dmg, 49, 3, false)`; **0x5F0F0D**: `ev = CEventDamage(car, time, 49, 3, (uint8)dir, false,
  bInVehicle)`. `anim = dir + 24` (0x5F0F22).
- If `ev.AffectsPed(ped)`: `ComputeDamageResponse(ped, &resp, true)`; ev anim fields = (0, anim, 8.0f, 1.0f); if playDeadAnim:
  `BlendAnimation(clump, 0, anim, 8.0f)->m_fSpeed = 1.0f; ev.flags |= 4`; `eventGroup.Add(&ev, false)`; then `ped+0x128 = car` unless
  `IsPlayer() && !(ped.m_nFlags & 0x1000) && car+0x594 != 6 && !(ped+0xE8 < -0.8f)` (unconditional overwrite, unlike the strong path);
  `intel byte +0x260 = 1`.
- `ped.flags0x470 &= ~0x10`.
- Train (`car+0x594 == 6`): if `ped.m_nFlags & 0x1000`: `ped.moveSpeed = (0,0,0)`. Else `n = Normalise(copy of car.moveSpeed)` (F+0x34);
  `k = (n.y*ped.v.y + n.z*ped.v.z) + n.x*ped.v.x`; `ped.v -= n*k` (0x40FEC0 then 0x406D70); `ped.v += car.moveSpeed * 0.3f` (0x411A00).
  Not a train: if `!(ped.m_nFlags & 0x1000)`: `ped.moveSpeed = car.moveSpeed * 0.75f`.
- `ped.moveSpeed.z = 0`; if `KnockDownPeds()`: `AddAudioEvent(0x79, 0.0f)`; `Say(0x159, 0, 1.0f, false, false, false)`; destroy ev, calc -> block 7.

### Block 7 - force on the car (common tail, 0x5F1155-0x5F12CF)
**ApplyForce is called on the car (ECX = EDI), not on the ped.** `up = car->m_matrix.up`:
`k = (d.y*up.y + d.z*up.z) + d.x*up.x`; `t = k * up`; `d.x = d.x - t.x; d.y -= t.y; d.z -= t.z` (stored back to F+0x1C..24);
`n = Normalise(copy of d)` (F+0x34); `s = (n.y*car.v.y + n.z*car.v.z) + n.x*car.v.x` (kept on the x87 stack);
if `!(ped.flags0x470 & 0x10)`: `n.z = n.z - 0.2f` (after s was computed); `s *= (car+0x590 == 9) ? -0.75f : -0.5f` (F+0x14);
`p = (d.x*0.25f, d.y*0.25f, d.z*0.25f)` (F+0x64..6C); `a = min(car.m_fMass / 1600.0f, 1.0f)` (0x404330(a, b)); `b = min(car.m_fMass, 1600.0f)`;
`k2 = (b * a) * s`; `car->ApplyForce((n.x*k2, n.y*k2, n.z*k2), p, true)`. Return.

### Block X - ignore-collision exit (0x5F12D1)
If `ped+0x128 == null` **or** `(car+0x36 & 0xF8) == 0` (the CAR's status bits = STATUS_PLAYER; earlier spec text said "current ignored entity" -
wrong, EAX is the car argument): `ped+0x128 = car` (no RegisterReference). Return.

- Edge cases / oddities: (1) the strong path writes `ped.matrix.pos.z` through m_matrix without a null check (0x5F089C); (2) StartShake's
  frequency argument is pushed as a full dword with stale upper bytes; (3) runover paths reached from the side-hit test keep hitZone 3/4,
  so `bKnockedUpIntoAir` is only set for the bonnet throw (hitZone 1); (4) the f < 0 (reversing car) path applies damage without changing the
  ped velocity; (5) .data coefficients 0x8D22A0..AC are read at run time (keep as globals).
- gta-reversed: plugin wrapper only (`.Reversed = false`, Ped.cpp:3571); no C++ to compare.
- Open questions: byte `intelligence+0x260` meaning (written 1 at 0x5F0DD7 / 0x5F0FFA); name of handling byte +0xCC bit 3 (IS_BIG per gr enum
  value, field name assumed m_nModelFlags).

## 0x678E80 sub_678e80 (proposed: empty `CTaskSimpleInAir` method taking a CPed*; catalogue: unnamed, unknown/REA, port level 0)
- Call site: CPed::PreRenderAfterTest 0x5E6609-0x5E6611: `push ebp (CPed* this); ecx = intel->GetTaskInAir()` (0x601150); `call 0x678E80`.
- Convention: `__thiscall void(CPed* ped [esp+4])`, ECX = CTaskSimpleInAir*, `ret 4`. Body: `ret 4` only - a no-op.
  The caller does not use EAX afterwards (0x5E6616 `and [ebp+0x528], esi`).
- gta-reversed: `Ped.cpp:2850` only clears `m_pedIK.bSlopePitch` when GetTaskInAir() is non-null and calls nothing - equivalent.
- Open question: the exact method name (an empty stub compiled out of e.g. a debug/roll-pitch routine) is not recoverable from the bytes.

## 0x6AE0D0 CPed::MakeTyresMuddySectorList<CPtrListSingleLink<CPhysical*>>  (catalogue: specified/REA, gr `.Reversed = false`, port level 5)
- Semantics: despite the gr name ("Muddy") this marks vehicle wheels **bloody** when they roll over this (dead) ped; caller
  CPed::DeadPedMakesTyresBloody 0x6B4200.
- Convention: `__thiscall void(CPtrListSingleLink* list [esp+4])`, `ret 4`; this = the ped. The list argument slot is reused for the
  next-node pointer.
- Callees: CMatrix `operator*(const CMatrix&, const CVector&)` 0x59C890 (cdecl(out, m, v)), CLocalisation::Blood 0x56D230,
  CAEVehicleAudioEntity::AddAudioEvent(int, float) 0x4F6420, CPhysical::ApplyMoveForce(CVector by value) 0x4ABBA0, 0x65DA70 (catalogue
  unnamed: by-value wrapper `(CVector force, CVector point)` that re-pushes its arguments, presumably to CPhysical::ApplyTurnForce),
  CWorld::FindPlayerVehicle, CPad::GetPad / StartShake(300, 70, 0).
- Behaviour: `car = 0` (EBX, initialised once before the loop); `bike` = a stack local that is **never initialised**. For each node
  (item +0, next +4):
  1. `v = item`; skip if `v->m_nScanCode (+0x2C) == CWorld::ms_nCurrentScanCode`; else set it.
  2. Skip unless `|ped.x - v.x| < 10.0f` and `|ped.y - v.y| < 10.0f` (abs via sign test; NaN skips).
  3. `cls = v->m_nVehicleClass (+0x590)`: 0 -> `car = v, bike = 0`; 9 -> `car = 0, bike = v`; any other class leaves **both from the
     previous vehicle** (or uninitialised `bike` on the first one) - original bug, keep.
  4. Skip unless `v->m_vecMoveSpeed.x^2 + .y^2 > 0.05f` (0x858C28; uses `v`, not car/bike).
  5. If `car`: for wheel w = 0..3: skip if `car->m_bWheelSkidmarkBloody[w]` (+0x820 + w) or `car->m_fWheelsSuspensionCompression[w]`
     (+0x7D4 + 4w) >= 1.0f. `b = ms_modelInfoPtrs[car->m_nModelIndex]->m_pColModel (+0x14)` bounding box (+0x0 min, +0xC max):
     offset O = w0: (-max.x, max.y, 0); w1: (-max.x, min.y, 0); w2: (max.x, max.y, 0); w3: (max.x, min.y, 0).
     `W = car->m_matrix * O`. Require `|W.z - ped.z| < 2.0f` and `(W.y - ped.y)^2 + (W.x - ped.x)^2 < 1.0f` (y term first).
     If `CLocalisation::Blood()`: `car->m_bWheelSkidmarkBloody[w] = 1`; `car->m_vehicleAudio (+0x138).AddAudioEvent(120, 0.0f)`.
     If `car->m_fMass (+0x8C) > 500.0f`: `car->ApplyMoveForce((0, 0, min(ped->m_fMass * 0.001f, 1.0f) * 50.0f))`;
     `0x65DA70(car, force = (0, 0, min(ped->m_fTurnMass (+0x90) * 0.0005f, 1.0f) * 50.0f), point = W - car.pos)` (min: NaN keeps the product);
     if `car == FindPlayerVehicle(-1, 0)`: `CPad::GetPad(0)->StartShake(300, 70, 0)`.
     (The force part runs even when Blood() is false - then the wheel is not marked and the force repeats on later frames.)
  6. Else if `bike`: for w = 0..1: skip if `bike->m_bWheelBloody[w]` (+0x74C + w) or `bike->m_aWheelRatios[w]` (+0x710 + 8w) >= 1.0f.
     O = w0: (0, max.y * 0.8f, 0); w1: (0, min.y * 0.8f, 0). Same W / distance tests; if Blood(): mark +0x74C+w, audio event 120 on bike+0x138.
     If `bike->m_fMass > 100.0f`: `ApplyMoveForce((0, 0, 10.0f))`; `0x65DA70(bike, (0, 0, 10.0f), W - bike.pos)`; player-vehicle shake as above.
