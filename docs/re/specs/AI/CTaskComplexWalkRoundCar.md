# CTaskComplexWalkRoundCar (AI) - RE-SPEC-3 group I4

Layout (gr TaskComplexWalkRoundCar.h, size 0x58) with byte-level corrections from these functions: +0xC m_MoveState (u8), +0xD flags
(bit0 m_bIsPedGoingForCarDoor, bit1 m_bFirstSubTaskNeedsToBeCreated, bits 2..5 = result of ComputeRouteRoundEntityBoundingBox, i.e. the
direction going round the car), +0xE low nibble (signed 4-bit) = direction passed IN to the route computation (forced direction),
+0x10 m_TargetPt, +0x1C m_Veh, +0x20 m_Route (CPointRoute*: +0 count, +4 points[]), +0x24 m_Timer (start, +0x28 interval, +0x2C started),
+0x30 m_VehPos, +0x3C m_VehMatFwd, +0x48 m_VehMatRight, +0x54 m_EnterCarStartTime. (gr places both nibbles in +0xE; the bytes write the
computed direction to +0xD bits 2..5 and read the forced direction from +0xE bits 0..3.) None of the three is reversed in gr
(TaskComplexWalkRoundCar.cpp:15/23/24 `.Reversed = false`, plugin calls): port from these bytes.

## 0x654370 CTaskComplexWalkRoundCar::ControlSubTask  (catalogue: specified/REA, port level 13)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Callees: CPed::IsPlayer 0x5DF8F0, CCarEnterExit::IsPlayerToQuitCarEnter 0x64F240,
  CTaskTimer::IsOutOfTime 0x420E30; virtual CreateFirstSubTask (slot 9), GetTaskType (slot 4), MakeAbortable (slot 6).
- Behaviour:
  1. If flags & 2: clear bit 1, return `this->CreateFirstSubTask(ped)` (vtbl+0x24) (0x654373-0x654388).
  2. `next = 200; quit = 0`. If `ped->IsPlayer() && m_Veh && m_EnterCarStartTime != -1 && IsPlayerToQuitCarEnter(ped, m_Veh,
     m_EnterCarStartTime, m_pSubTask)` (cdecl) and `m_pSubTask->GetTaskType() == 0x389` (905): `quit = 1; next = 0x516` (0x65439F-0x6543D9).
     Otherwise if `m_pSubTask->GetTaskType() == 0x389` and `m_Timer.IsOutOfTime()`: `next = 0x516` (0x6543DD-0x6543F8).
  3. If m_Veh == 0 -> step 5. `p = vehicle position` (matrix +0x30 or placement +4); `dx = VehPos.x - p.x`, dy, dz;
     `s = (dz*dz + dy*dy) + dx*dx`; if `s > 0.0625f [0x86F140]` -> step 5 (`test ah,0x41; je`; NaN stays) (0x654408-0x65444B).
     `f = m_Veh->matrix->forward` (no null check): if `(f.z*VF.z + f.y*VF.y) + f.x*VF.x < 0.9f [0x86F144]` -> step 5 (`test ah,5; jnp`:
     strictly less; NaN stays) (0x65444D-0x65446E). `r = matrix->right`: if `(VR.z*r.z + VR.y*r.y) + r.x*VR.x < 0.9f` -> step 5 (0x654470-0x654490).
  4. If `next == 200` return m_pSubTask (0x654492-0x654498 -> 0x6544DF); else step 5.
  5. If `m_pSubTask->MakeAbortable(ped, 1, 0)` (0x65449A-0x6544A4): if quit: `t = ped->m_pIntelligence (+0x47C) dword +0x10` (task manager
     primary slot 3); if t and `t->GetTaskType() == 0x2BD` (701) `t->MakeAbortable(ped, 0, 0)`; return 0 (0x6544AB-0x6544D9). If not
     abortable return m_pSubTask. (`next` only distinguishes 200 from "change"; its value 0x516 is never used otherwise.)

## 0x6544F0 CTaskComplexWalkRoundCar::ComputeRouteRoundSmallCar  (catalogue: specified/REA, port level 10)
- `__thiscall float (this, CPed* ped [esp+4])`, `ret 4`, result in ST0. Callers: ComputeRouteRoundBigCar 0x656BB0, ComputeRoute 0x657B80
  (tail), CreateFirstSubTask 0x658200. Callees (CPedGeometryAnalyser, cdecl): ComputeEntityBoundingBoxCorners 0x5F3650 (pedZ, veh, corners[4]),
  ComputeEntityBoundingBoxPlanes 0x5F3660 (pedZ, veh, normals[4] (CVector, body +0x2C), dists[4] (float, body +0x1C)), ComputeEntityHitSide
  0x5F3BC0 (ped, veh), ComputeEntityHitSide 0x5F3AC0 (const CVector* point, veh), ComputeRouteRoundEntityBoundingBox 0x5F6110.
- Behaviour:
  1. `z = ped position.z` (matrix +0x38 or placement +0xC); corners and planes of m_Veh at height z (0x6544FE-0x654546; the corners are not
     used afterwards).
  2. `side = ComputeEntityHitSide(ped, m_Veh)`; `ret = side == -1 ? 0.0 : (n[side].z*p.z + n[side].y*p.y) + n[side].x*p.x + d[side]`
     (p = ped position; stored f32; 0x65454B-0x654592).
  3. If flags & 1 (going for the door) (0x65459A): test m_TargetPt t against the 4 planes in order k = 0..3: `s_k = dot + d[k]` with dot
     order `(n.x*t.x + n.z*t.z) + n.y*t.y` for k = 0 and `(n.y*t.y + n.x*t.x) + n.z*t.z` for k = 1..3 (stored f32 s[k]); stop at the
     first `s_k >= 0` (`test ah,1; je`; NaN counts as < 0 and continues). If all four are < 0 (point inside the box):
     `side = ComputeEntityHitSide(&m_TargetPt, m_Veh)`; if != -1: `k = -s[side] + 0.05f [0x858C28]`; `m_TargetPt += n[side] * k`
     (products k*n.x, k*n.y, k*n.z, each then added to the stored component) (0x654677-0x6546CC).
  4. `dirIn = sign-extended low nibble of byte +0xE`; `r = ComputeRouteRoundEntityBoundingBox(ped, m_Veh, &m_TargetPt, m_Route, dirIn)`
     (AL); `byte +0xD = (byte +0xD & ~0x3C) | ((r << 2) & 0x3C)` (0x6546D0-0x654704). Return `ret` (f32 from step 2, `fld` at 0x6546F2).

## 0x658200 CTaskComplexWalkRoundCar::CreateFirstSubTask  (catalogue: specified/REA, port level 12)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH (handler 0x843A93). Local CTaskSimpleStandStill (body +0x2C).
- Callees: ComputeRouteRoundBigCar 0x656BB0, ComputeRouteRoundSmallCar 0x6544F0, CVector::NormaliseAndMag 0x59C970, CPed::IsPlayer,
  CTaskManager::FindTaskByType 0x6817D0, CreateRouteTask 0x6542E0, CTaskSimpleStandStill ctor 0x62F310 / ProcessPed 0x62F370 / dtor 0x62F360,
  CGeneral::GetRadianAngleBetweenPoints 0x53CBE0, CGeneral::LimitRadianAngle 0x53CB50, CTaskSimpleAchieveHeading ctor 0x667E20.
- Behaviour:
  1. `m_Route->count = 0`; `hit = (m_Veh byte +0x429 & 4) ? ComputeRouteRoundBigCar(ped) : ComputeRouteRoundSmallCar(ped)` (f32 store)
     (0x65821B-0x658249). If `m_Route->count == 0` return 0.
  2. Timer: `ms = (m_MoveState == 4) ? 20000 : 15000`; if `m_Veh[+0x4C8] != 0 || m_Veh[+0x4C4] != 0` `ms = ftol((float)ms * 4.0f [0x858B90])`;
     `m_Timer = {start = m_snTimeInMilliseconds, interval = ms, started = 1}` (0x658267-0x6582B1).
  3. `m_VehPos = vehicle position`; `m_VehMatFwd = m_Veh->matrix->forward (+0x10)`; `m_VehMatRight = matrix->right (+0)` (no null check of
     the matrix for the two vectors; 0x6582B5-0x658304).
  4. `d = route.points[0] - ped position` (components computed z, y, x; stored f32); `dist = d.NormaliseAndMag()`; `dot = (d.z*pf.z +
     d.y*pf.y) + d.x*pf.x` with pf = ped matrix forward (no null check) (0x658307-0x658365).
  5. `thr = m_MoveState == 4 ? 2.0f : m_MoveState == 6 ? 4.0f : 6.0f` (immediates, 0x658369-0x658381).
  6. If ped->IsPlayer(): `t = taskMgr.FindTaskByType(3, 0x2BD)`; if t `m_EnterCarStartTime = t[+0x4C]` (0x658389-0x6583B0).
  7. If `!(m_Veh byte +0x429 & 4)` and `dist > thr` and `hit > thr` and `dot > 0.0f [0x86F424]` (each `test ah,0x41; jne`: NaN fails) ->
     return `CreateRouteTask(ped)` (0x6583B3-0x6583F1).
  8. Else: local `CTaskSimpleStandStill(0, 0, 0, 8.0f)`; `local.ProcessPed(ped)`; `a = LimitRadianAngle(GetRadianAngleBetweenPoints(
     route.points[0].x - ped.x, route.points[0].y - ped.y, 0, 0))` (differences stored f32); `t = new(0x18) CTaskSimpleAchieveHeading(a,
     1.0f, 0.1f)` (0 on failure); destroy local; return t (0x658409-0x6584C5).
- Open questions: meaning of vehicle +0x4C4/+0x4C8 (0x65827F) and of byte +0x429 bit 2 (big-vehicle flag per gr GoingForDoor/bIsBig).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x656BB0 CTaskComplexWalkRoundCar::ComputeRouteRoundBigCar  (catalogue: specified/REA, port level 11)
- `__thiscall float (this, CPed* ped [esp+4])`, `ret 4`; always returns 0.0f in ST0 (`fld [0x858B50]` 0x656E16, left on the FPU stack). Fields: +0x10 target point (CVector), +0x1C vehicle, +0x20 CPointRoute* (u32 count + CVector[8]). "Add(p)" = inline CPointRoute::AddPoint (append if count < 8, else drop). "Surf*" calls run with `CPedGeometryAnalyser::ms_fPedNominalRadius` ([0x8D22B0]) temporarily set to 0.7f (0x3F333333) and restored to the saved value afterwards (save/set/restore around each pair).
- Behaviour:
  1. `s1 = CPedGeometryAnalyser::ComputeEntityHitSide(ped, veh)` 0x5F3BC0; `s2 = ComputeEntityHitSide(&target, veh)` 0x5F3AC0.
  2. Same side (`s1 == s2`): `A = ComputeClosestSurfacePoint(ped, veh)` 0x5F3B70; `B = ComputeClosestSurfacePoint(&target, veh)` 0x5F36F0 (radius 0.7 for both); `Add(A)`; then (common end) `Add(B)`.
  3. Different sides: `C = ComputeClosestSurfacePoint(ped, veh)`, `D = ComputeClosestSurfacePoint(&target, veh)` (radius 0.7). Save the ped position `P` and the target `T`; move the ped to C (`m_matrix->pos = C` if the ped has a matrix - y and z via fld/fstp, x raw - else `m_placement.pos = C`) and set `target = D`; `CTaskComplexWalkRoundCar::ComputeRouteRoundSmallCar(this, ped)` 0x6544F0 (ST0 result discarded); restore `target = T` and the ped position `P` (same matrix/placement choice). `tmp = *route` (CPointRoute copy 0x63A040 into a local); `route->count = 0`; `Add(C)`; `Add(tmp.pts[i])` for i = 0 .. tmp.count-1; `E = ComputeClosestSurfacePoint(&target, veh)` (radius 0.7); then (common end) `Add(E)`.
