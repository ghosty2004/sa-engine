# CTaskComplexCarDriveWander (AI) - RE-SPEC-3 group I2
Vtable 0x86E96C, type 0x2C7, size 0x24. CTaskComplexCarDrive layout (gr, confirmed): +0xC m_Veh, +0x10 m_CruiseSpeed f32, +0x14
m_DesiredCarModel i32, +0x18 m_CarDrivingStyle i32, +0x1C m_bAsDriver, +0x1D m_OriginalDrivingStyle u8, +0x1E m_OriginalMission i8,
+0x1F m_OriginalSpeed u8, +0x20 m_bIsCarSetUp. Vehicle autopilot bytes: +0x3B9 driving style, +0x3BA mission, +0x3D0 cruise speed (u8),
+0x3C8 cruise speed (f32), +0x3AC timer; +0x36 status/type byte; +0x428 / +0x42D flag bytes.

## 0x63CB10 CTaskComplexCarDriveWander::CTaskComplexCarDriveWander  (catalogue: specified/REA, port level 1)
- `__thiscall this (CVehicle* veh, f32 cruise, int32 style)`, `ret 0xC`: `CTaskComplexCarDrive::CTaskComplexCarDrive(this, veh, cruise, -1,
  style)` 0x63C940; vtbl = 0x86E96C; return this.

## 0x63CB40 CTaskComplexCarDriveWander::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x2C7 (711).

## 0x63CB60 CTaskComplexCarDriveWander::SetUpCar  (catalogue: specified/REA, port level 3)
- `__thiscall void ()`. Save `+0x1D = veh->+0x3B9`, `+0x1E = veh->+0x3BA`, `+0x1F = veh->+0x3D0`; `+0x20 = 1`.
- If `!CCarCtrl::bCarIsBeingCreated` [0x9690CC]: `CCarCtrl::JoinCarWithRoadSystem(veh)` 0x42F5A0; `veh->+0x36 = (veh->+0x36 & 7) | 0x18`;
  `veh->+0x3BA = 1` (MISSION_CRUISE); `veh->+0x3D0 = (u8)_ftol(m_CruiseSpeed)`; `veh->+0x3C8 = (f32)(u8)veh->+0x3D0` (fild of the
  zero-extended byte); `veh->+0x3B9 = (u8)m_CarDrivingStyle`.
- Then `veh->+0x428 bit 4 = !(veh->+0x42D & 2)` (set when clear, cleared when set) and `veh->+0x3AC = CTimer::m_snTimeInMilliseconds`.
- gr TaskComplexDriveWander.cpp:13 - compare when porting; the bytes above are authoritative.

## 0x63DD00 CTaskComplexCarDriveWander::Clone  (catalogue: specified/REA, port level 3)
- `p = new(0x24)`; if p: `CTaskComplexCarDrive(p, m_Veh, m_CruiseSpeed (fld/fstp, f32), -1, m_CarDrivingStyle)`; vtbl = 0x86E96C. Return p/0.

## 0x643220 CTaskComplexCarDriveWander::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- call 0x63CB50 (complete dtor); `if (flags & 1) CTask::operator delete`; return this; `ret 4`.

## 0x643240 CTaskComplexCarDriveWander::CreateSubTaskCannotGetInCar  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`: `return CTaskComplexCarDrive::CreateSubTask(this, 0x390, ped)` 0x642FA0
  (0x390 = 912 TASK_COMPLEX_WANDER).

## 0x63CB50 CTaskComplexCarDriveWander::~CTaskComplexCarDriveWander  (catalogue: unknown/REA, port level 3)
- Convention: `__thiscall void (this)`. `vtable = 0x86E96C` (CTaskComplexCarDriveWander, also stored by its ctor 0x63CB10), then tail-jump to
  CTaskComplexCarDrive::~CTaskComplexCarDrive 0x63CA40.
- gta-reversed: `= default` - matches. Open questions: none.
