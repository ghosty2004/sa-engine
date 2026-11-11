# CTaskComplexDriveToPoint (AI) - RE-SPEC-AI-2 group J2

Vtable 0x86E9DC, derives from CTaskComplexCarDrive (ctor 0x63C940: +0xC m_pVehicle, +0x10 m_fCruiseSpeed f32, +0x14 m_nCarModelIndexToCreate i32,
+0x18 m_nCarDrivingStyle u8, +0x1D/+0x1E/+0x1F saved autopilot bytes, +0x20 m_bSavedVehicleBehavior u8). Own fields (gr TaskComplexDriveToPoint.h,
size 0x3C): +0x24 m_Point CVector, +0x30 field_30 i32, +0x34 m_Radius f32, +0x38 u8.

## 0x63CE00 CTaskComplexDriveToPoint::CTaskComplexDriveToPoint  (catalogue: specified/REA, port level 1)
- `__thiscall (this, CVehicle* veh [esp+4], const CVector* point [+8], f32 speed [+0xC], i32 a4 [+0x10], i32 model [+0x14], f32 radius [+0x18],
  i32 drivingStyle [+0x1C])`, `ret 0x1C`, returns this.
- `CTaskComplexCarDrive::CTaskComplexCarDrive(veh, speed, model, drivingStyle)` 0x63C940 (pushes 0x63CE09-0x63CE16); vtbl = 0x86E9DC; +0x24 = *point
  (3 dwords); +0x30 = a4; +0x34 = radius (raw dword); +0x38 = 0 (0x63CE22-0x63CE49).

## 0x63CE60 CTaskComplexDriveToPoint::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2C6` (0x63CE60); `ret` -> returns 710 (TASK_COMPLEX_CAR_DRIVE_TO_POINT). Vtable: slot 4 (+0x10) of vtable 0x86E9DC.

## 0x63CE80 CTaskComplexDriveToPoint::CreateSubTaskCannotGetInCar  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped [esp+4])` (ped unused), `ret 4`, SEH frame.
- `p = CTask::operator new(0x34)`; if p: `CTaskComplexGoToPointAnyMeans(6 /*move state*/, &m_Point (+0x24), radius = f32 global [0xC18DB4], model = +0x14)`
  0x66B720 (pushes 0x63CEB3-0x63CEC2; 0xC18DB4 is .bss in the image, value set at runtime). Returns p, or 0 when allocation failed.

## 0x63CF00 CTaskComplexDriveToPoint::SetUpCar  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. veh = +0xC (no null check).
- Save: `+0x1D = veh->+0x3B9`, `+0x1E = veh->+0x3BA`, `+0x1F = veh->+0x3D0` (bytes, 0x63CF10-0x63CF2D); `+0x20 = 1`.
- If `+0x10 (cruise speed) > 0.0 [0x858B50]` (strict, `test ah,0x41; jne`; NaN skips): `veh->+0x3D0 = (u8)_ftol(+0x10)` 0x821B40 (truncation).
- `veh->+0x3B9 = (u8)+0x18` (driving style), `veh->+0x3AC = CTimer::m_snTimeInMilliseconds` 0xB7CB84 (0x63CF44-0x63CF59).
- gr row TaskComplexDriveToPoint.cpp:31 (assert): REFUTED - the bytes have no check (harmless debug assert).

## 0x63DDE0 CTaskComplexDriveToPoint::Clone  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* ()`, plain `ret`, SEH: `p = new(0x3C)`; if p: `CTaskComplexDriveToPoint(+0xC, &+0x24, +0x10, +0x30, +0x14, +0x34, +0x18)` 0x63CE00
  (all passed as raw dwords, 0x63DE13-0x63DE2E); returns p or 0. gr TaskComplexDriveToPoint.h:27 matches (field_30 = arg4).

## 0x643280 CTaskComplexDriveToPoint::~CTaskComplexDriveToPoint  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0063ce70 0x63CE70 (0x643283); if `flags & 1` (0x643288) calls CTask::operator delete 0x61A5B0 (this) (0x643290, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E9DC.

## 0x6432A0 CTaskComplexDriveToPoint::IsTargetBlocked  (catalogue: specified/REA, port level 0)
- Overload `__thiscall bool (this, CPed* ped [esp+4], CEntity** list [+8], int32 count [+0xC])`, `ret 0xC`. Uses CModelInfo::ms_modelInfoPtrs
  0xA9B0C8; "R(e)" = `ms_modelInfoPtrs[e->m_nModelIndex (+0x22, i16)]->m_pColModel (+0x14)->+0x24` (bounding sphere radius); positions via
  matrix +0x30 or placement +0x4.
- veh = ped->m_pVehicle (+0x58C); if 0 return 0 (0x6432B2). `Rv = R(veh)`; `dv = veh.pos - m_Point (+0x24)`; `Dv = (dz*dz + dy*dy) + dx*dx` stored
  f32 (0x6432DF-0x64330B).
- For i = 0 .. count-1 (signed, `jle` skip when count <= 0): e = list[i]; skip if null or e == veh. `de = e.pos - m_Point`; `De = (dz*dz + dy*dy) + dx*dx`;
  `Re = R(e)` (stored f32). If `Re*Re > De` (strict, `fcompp; test ah,0x41; jne`): `s = Re + Rv`; if `s*s * 1.5 [0x86C680, f64] > Dv` (strict; f64
  multiply) return 1 (0x64338F-0x6433AE).
- Return 0 after the loop.

## 0x6452C0 CTaskComplexDriveToPoint::IsTargetBlocked  (catalogue: specified/REA, port level 0)
- Overload `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`.
- `d = ped.pos - m_Point`; `D = (dz*dz + dx*dx) + dy*dy` (this summation order, 0x6452E5-0x6452F3); `L = 6.0 [0x86E6A8] * 6.0 [0x86E6A8]`;
  if NOT (`L >= D` or unordered) - i.e. `L < D` - return 0 (`fcompp; test ah,5; jp`, 0x645301-0x645310).
- `if (IsTargetBlocked(ped, intel+0x130, 16))` return 1, else return `IsTargetBlocked(ped, intel+0xE0, 16) != 0` with intel = ped->m_pIntelligence
  (+0x47C) (0x64531D-0x645349; the two 16-entry entity arrays of the ped intelligence, ECX = this passed through).

## 0x645420 CTaskComplexDriveToPoint::Drive  (catalogue: specified/REA, port level 7)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. EBP = m_pSubTask (+0x8) read at entry, EDI = veh (+0xC), EBX = &m_Point (+0x24).
- `d = m_Point - veh.pos`; `dist = sqrt((dz*dz + dy*dy) + dx*dx)` (0x64543B-0x64545F).
- If `dist < m_Radius (+0x34)` (`fcom; test ah,5; jp` -> continue when dist >= radius or NaN): `veh->+0x3BA (autopilot mission) = 0`; `+0x38 = 1`;
  return `CTaskComplexCarDrive::CreateSubTask(0x516 /*TASK_FINISHED*/, ped)` 0x642FA0 (0x645471-0x64548A).
- Else if `dist < 3.0 [0x86E6A4]` (same compare form, 0x645498-0x6454A3) and `veh->+0x3BA == 0`: `+0x38 = 1`; return CreateSubTask(0x516, ped).
- Else: if `veh->+0x3D0 (cruise speed) == 0`: `veh->+0x3D0 = (u8)_ftol(m_fCruiseSpeed +0x10)` 0x821B40. If `IsTargetBlocked(ped)` 0x6452C0: `+0x38 = 1`;
  return CreateSubTask(0x516, ped). Else switch on `+0x30` (unsigned, `cmp 3; ja`, table 0x645594 = {0x64551C, 0x645558, 0x64553A, 0x645576}):
  0 -> `CCarAI::GetCarToGoToCoors(veh, &m_Point, +0x18 (dword), 0)` 0x41CE30; 1 -> GetCarToGoToCoorsAccurate 0x41D0E0; 2 -> GetCarToGoToCoorsStraightLine
  0x41CFB0; 3 -> GetCarToGoToCoorsRacing 0x41D210 (each cdecl, ST0 result popped with `fstp st(0)`); return EBP (the subtask read at entry). Values > 3
  skip the call and also return EBP.
- gr rows: cpp:51 assert (`m_CruiseSpeed < 255`) - REFUTED, harmless (no check in the bytes). cpp:65 `default: NOTSA_UNREACHABLE()` - CONFIRMED
  (original returns m_pSubTask for +0x30 > 3, 0x645513 -> 0x64558B); keep-original. gr's case mapping matches the table (1 = ACCURATE, 2 = STRAIGHT_LINE).
