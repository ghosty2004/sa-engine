# CTaskComplexWalkRoundObject (AI) - RE-SPEC-3 group I2
Vtable 0x86F364, type 0x1FE (510), size 0x54 (gr header says 0x34 - the bytes use fields up to +0x53). Layout: +0xC m_moveState,
+0x10 m_targetPoint CVector, +0x1C m_object CEntity* (registered ref), +0x20 m_pointRoute CPointRoute* (+0 count i32, +4 points[8]
CVector), +0x24 CTaskTimer {+0x24 start, +0x28 interval, +0x2C started, +0x2D stopped}, +0x30 object position snapshot, +0x3C object
forward snapshot (matrix +0x10), +0x48 object right snapshot (matrix +0). Helpers: 0x655140 CreateRouteTask(ped), 0x655290
CreateSubTask(type, ped), 0x6551D0 ComputeRouteAndGetHitSideDistance (see _unattributed-I1.md), `CPointRoute::AddPoint(const CVector*)`
0x48E180 (thunk to 0x1569990: if count < 8 append and return 1).

## 0x6550C0 CTaskComplexWalkRoundObject::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x1FE.

## 0x657200 CTaskComplexWalkRoundObject::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- call 0x6550D0; `if (flags & 1) CTask::operator delete`; return this; `ret 4`.

## 0x657220 CTaskComplexWalkRoundObject::CreateNextSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH. If m_object == 0 return 0. `t = m_pSubTask->GetTaskType()`:
  - 0x387 (GO_TO_POINT_AND_STAND_STILL): return `CreateRouteTask(ped)`.
  - 0x386 (ACHIEVE_HEADING): if `m_pointRoute->count != 0` return CreateRouteTask(ped). Else: `S = CPedGeometryAnalyser::
    ComputeClosestSurfacePoint(ped, m_object, &S)` 0x5F3B70; `a = ComputeEntityHitSide(&S, m_object)` 0x5F3AC0; `b =
    ComputeEntityHitSide(&m_targetPoint, m_object)`. If a == b: `m_pointRoute->AddPoint(&S)`; `S = ComputeClosestSurfacePoint(&m_targetPoint,
    m_object, &S)` 0x5F36F0; `AddPoint(&S)`; return CreateRouteTask(ped). Else: `p = new(0x28)`; return p ?
    `CTaskComplexGoToPointAndStandStill(p, 4, &S, 0.5f [0x86FC84], 2.0f [0x86FC88], false, false)` : 0.
  - 0xCB, 0x389 and any other type: return 0.

## 0x657380 CTaskComplexWalkRoundObject::CreateFirstSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH.
- If m_object == 0 return 0. `m_pointRoute->count = 0`; `d0 = ComputeRouteAndGetHitSideDistance(ped)` (f32); if the route is still empty
  return 0.
- Timer: start = now, interval = (m_moveState == 4) ? 8000 : 4000, started = 1. Snapshots: +0x30 = object position (matrix ? +0x30 :
  +4); for the two orientation snapshots the object's matrix is created on demand (`if (!m_matrix) { AllocateMatrix() 0x54F560;
  m_placement.UpdateMatrix(m_matrix) 0x54F1B0 }`, done twice): +0x3C = matrix forward, +0x48 = matrix right.
- `v = route.points[0] - pedPos` (z, y, x computed; stored f32); `m = v.NormaliseAndMag()` 0x59C970 (f32, kept in the ped arg slot);
  `dot = (v.z*f.z + v.y*f.y) + v.x*f.x` with f = ped matrix forward (not null-checked); `lim = 2.0f [0x858CA0]` (moveState 4), `4.0f
  [0x858B90]` (6) or `6.0f [0x858B44]` (other).
- If `m > lim` and `d0 > lim` and `dot > 0.0f [0x86F420]` (all strict): return CreateRouteTask(ped).
- Else: `a = CGeneral::LimitRadianAngle(CGeneral::GetRadianAngleBetweenPoints(pt.x - P.x, pt.y - P.y, 0, 0))` (0x53CBE0, 0x53CB50;
  differences recomputed from the route point and the ped position, stored f32); `p = new(0x18)`; return p ?
  `CTaskSimpleAchieveHeading(p, a, 1.0f, 0.2f)` : 0.

## 0x6575F0 CTaskComplexWalkRoundObject::ControlSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. QUIT = `if (m_pSubTask->MakeAbortable(ped, 1, nullptr)) return CreateSubTask(0xCB, ped)
  0x655290; else return m_pSubTask`.
- If m_object == 0 -> QUIT. If subtask type == 0x389 and the timer is started: (if stopped: restart at now) if `now >= start + interval`
  (unsigned) -> QUIT. If `ped->intel->+0x274 > [0x86C938] (30)` -> QUIT. If subtask type == 0xCB return m_pSubTask.
- Ensure the object matrix (AllocateMatrix/UpdateMatrix as above). `d = snapPos - objPos`; if `(d.z*d.z + d.y*d.y) + d.x*d.x > 0.0625f
  [0x86F15C]` (strict) -> QUIT. `c1 = (mat.fwd.z*snapFwd.z + mat.fwd.y*snapFwd.y) + snapFwd.x*mat.fwd.x`; if `c1 < 0.9f [0x86F160]` -> QUIT.
  `c2 = DotProduct(&snapRight, &mat.right)` 0x40FDB0; if `c2 < 0.9f` -> QUIT. Else return m_pSubTask.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x655C10 CTaskComplexWalkRoundObject::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x54)` 0x61A5A0 (cdecl, task pool, 0x655C2B); if p == 0 return 0 (0x655C41); else `CTaskComplexWalkRoundObject::Constructor` 0x655020 on p with args (this+0xC, &this+0x10, this+0x1C) (0x655C51) and return its EAX (the ctor returns p).
