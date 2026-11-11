# CTaskComplexCarDrive (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x63C9C0 CTaskComplexCarDrive::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 725 (0x2D5) (0x63C9C0).

## 0x642F80 CTaskComplexCarDrive::~CTaskComplexCarDrive  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63CA40 CTaskComplexCarDrive::Destructor (0x642F83); if `flags & 1` (0x642F88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x642F90); returns this (0x642F98).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x63C840 CTaskComplexCarDrive::sub_63C840 (proposed: CTaskSimpleCarDrive::FinishCloseDoorAnimCB)  (catalogue: unknown/REA, port level 1)
- Name proposal: anim finish callback of CTaskSimpleCarDrive (installed by 0x642700, which is called from CTaskSimpleCarDrive::ProcessPed 0x644470); data = CTaskSimpleCarDrive* (+8 m_pVehicle, +0xC anim, +0x5C flags).
- `static void __cdecl (CAnimBlendAssociation* assoc, void* data [esp+8])`, plain `ret`.
- Behaviour: `data->+0x5C &= 0xDF` (clear bit 5, 0x63C845); `data->+0xC = 0` (0x63C84E); `veh = data->+8`; if NULL return. `CVehicle::ClearGettingOutFlags(veh, 1)` 0x6D3060 (0x63C859). `if veh->m_pDriver (+0x460)`: `grp = CVehicleAnimGroupData::m_vehicleAnimGroups[veh->m_pHandlingData (+0x384)->+0xDE (u8 anim group)]` (0xC1CDC0 + idx*0x94) `.GetGroup(0x182)` 0x6E3B00; `veh->vfunc[+0x70](m_pDriver, 0xA, grp, 0x182, 1.0f)` (thiscall, float pushed as 0x3F800000; 0x63C8A7) - by the argument shape this is CVehicle::ProcessOpenDoor(ped, door 10, animGroup, animId 386, time 1.0).
