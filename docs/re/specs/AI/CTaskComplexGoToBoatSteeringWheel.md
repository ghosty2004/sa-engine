# CTaskComplexGoToBoatSteeringWheel (AI) - RE-SPEC-AI-2 group J2

## 0x6490F0 CTaskComplexGoToBoatSteeringWheel::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x33E` (0x6490F0); `ret` -> returns 830 (TASK_COMPLEX_GO_TO_BOAT_STEERING_WHEEL). Vtable: slot 4 (+0x10) of vtable 0x86F018 (CTaskComplexGoToBoatSteeringWheel).

## 0x64CD80 CTaskComplexGoToBoatSteeringWheel::~CTaskComplexGoToBoatSteeringWheel  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGoToBoatSteeringWheel::Destructor 0x649100 (0x64CD83); if `flags & 1` (0x64CD88) calls CTask::operator delete 0x61A5B0 (this) (0x64CD90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F018 (CTaskComplexGoToBoatSteeringWheel).

## 0x649160 CTaskComplexGoToBoatSteeringWheel::ComputeTargetPos  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (this)`, plain `ret`, `sub esp,0x18`. Fields: +0xC CVector m_vecTargetPos (out), +0x18 CVehicle*.
- v = vehicle; mi = CModelInfo::ms_modelInfoPtrs[(int16)v+0x22] (0xA9B0C8); s = mi+0x5C (vehicle struct pointer); src = (mi+0x3C ==
  5) ? s+0 : s+0x30 (dummy position; 5 = boat type); L = *src (3 dwords). t = Multiply3x3(&tmp, v->m_matrix (+0x14), &L) 0x59C790
  (cdecl, returns &tmp; no NULL check on the matrix); L = t. p = matrix ? &matrix->pos (+0x30) : &v->placement.pos (+4).
  +0xC = L.x + p.x; +0x10 = L.y + p.y; +0x14 = L.z + p.z (each fld/fadd/fstp f32, stored after all three are computed in the order
  x, y, z).
- gta-reversed: not hooked at this address.

## 0x64CDA0 CTaskComplexGoToBoatSteeringWheel::CreateSubTask  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`, SEH frame.
- 0xCB (203, STAND_STILL): new(0x20) CTaskSimpleStandStill(-1, 0, 0, 8.0f) 0x62F310 (NULL on failure).
- 0x387 (903, GO_TO_POINT_AND_STAND_STILL): this->ComputeTargetPos() 0x649160 (before allocating); new(0x28)
  CTaskComplexGoToPointAndStandStill(4 (move state), &this->+0xC, [0x86FC84]=0.5f, [0x86FC88]=2.0f (raw dwords), 0, 0) 0x668120.
- 0x516 and everything else: NULL.
- gta-reversed: not hooked at this address.
