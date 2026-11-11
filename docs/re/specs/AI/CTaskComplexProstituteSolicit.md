# CTaskComplexProstituteSolicit (AI) - RE-SPEC-AI-2 group J3

## 0x661AE0 CTaskComplexProstituteSolicit::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x51D` (0x661AE0); `ret` -> returns 1309 (TASK_COMPLEX_PROSTITUTE_SOLICIT). Vtable: slot 4 (+0x10) of vtable 0x86FB88.

## 0x664AA0 CTaskComplexProstituteSolicit::~CTaskComplexProstituteSolicit  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexProstituteSolicit::~CTaskComplexProstituteSolicit 0x661AF0 (0x664AA3); if `flags & 1` (0x664AA8) calls CTask::operator delete 0x61A5B0 (this) (0x664AB0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FB88.

## 0x661AF0 CTaskComplexProstituteSolicit::~CTaskComplexProstituteSolicit  (catalogue: specified/REA, port level 2)
- `__thiscall void ()`, plain `ret`, SEH frame. vtbl = 0x86FB88. p = CWorld::FindPlayerPed(-1) 0x56E210; if p != NULL:
  pd = p->m_pPlayerData (+0x480); if pd+0xA4 (gr m_pPlayerProstitute? entity ref) != NULL: CleanUpOldReference(&pd->+0xA4) 0x571A00;
  pd+0xA4 = 0; then if (u16)this+0x2C & 0x100: this+0x2C &= 0xFEFF (word store). Finally CTaskComplex::~CTaskComplex 0x61A3D0.
  (The flag clear only happens when the player ped exists.)

## 0x661B80 CTaskComplexProstituteSolicit::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)`, `ret 0xC`. r = m_pSubTask (+8)->MakeAbortable(ped,
  priority, event) (vtable +0x18); if r (AL) != 0: byte this+0x2D &= 0xFE (clears bit 8 of the +0x2C word). Returns r (AL).

## 0x661D30 CTaskComplexProstituteSolicit::GetRidOfPlayerProstitute  (catalogue: specified/REA, port level 2)
- `__cdecl void ()` (static). p = CWorld::FindPlayerPed(-1) 0x56E210 (no NULL check); e = p->m_pPlayerData (+0x480)->+0xA4 (player's
  prostitute ped); if e != NULL: t = e->m_pIntelligence (+0x47C)->FindTaskByType(0x51D = 1309, TASK_COMPLEX_PROSTITUTE_SOLICIT)
  0x600EE0; if t != NULL: byte t+0x2C |= 4; word t+0x2A = 0.

## 0x6622F0 CTaskComplexProstituteSolicit::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x30) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexProstituteSolicit::CTaskComplexProstituteSolicit(this+0xC) 0x661A60; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x661BB0 CTaskComplexProstituteSolicit::IsTaskValid  (catalogue: specified/REA, port level 1)
- `static bool __cdecl (CPed* prostitute [esp+4], CPed* client [esp+8])`, plain `ret`. Callers: CreateFirstSubTask 0x6666A0, CreateNextSubTask 0x666780, ControlSubTask 0x6669D0, CTaskComplexWanderProstitute::ScanForStuff 0x672700.
- Returns AL = 1 only if all of the following hold, tested in this order (any failure -> 0):
  1. `client == CWorld::FindPlayerPed(-1)` and client != NULL.
  2. `client->bInVehicle` (+0x46C & 0x100; the flag dword is read in a .HOODLUM detour 0x40160B); `veh = client->m_pVehicle (+0x58C) != NULL`; `!(client->+0x474 & 0x40)` (bIsBeingArrested).
  3. `client->m_pPlayerData (+0x480)->m_pCurrentProstitutePed (+0xA4)` is NULL or == prostitute.
  4. `CVehicle::GetVehicleAppearance(veh) == 1` 0x6D1080; `!CVehicle::IsUpsideDown(veh)` 0x6D1D90; `!CVehicle::IsOnItsSide(veh)` 0x6D1DD0.
  5. `CTaskManager::GetSimplestActiveTask(&client->m_pIntelligence->m_TaskMgr)->GetTaskType() == 0x2C5` (TASK_SIMPLE_CAR_DRIVE).
  6. `veh->m_pDriver (+0x460) == client`.
  7. `pv = prostitute->m_pVehicle`: if pv != NULL it must equal veh and `veh->m_nNumPassengers (+0x484) == 1`; if pv == NULL then `veh->m_nNumPassengers == 0`.
  8. `veh->m_nMaxPassengers (+0x488) >= 1` (unsigned); `!((veh->m_pHandlingData (+0x384)->+0xCC >> 9) & 1)` (handling flag bit 9).
  9. `VectorSub(&d, clientPos, prostitutePos)` 0x40FE60; `m = 0x406DA0(&d)` (CVector::Magnitude, ST0); `fcomp [0x86F840]=100.0f; test ah, 0x41; je fail`: fails only when `m > 100` (equal or NaN pass).
  10. `!CTheScripts::IsPlayerOnAMission()` 0x464D50 and `!CGameLogic::IsCoopGameGoingOn()` 0x441390.

## 0x666360 CTaskComplexProstituteSolicit::CreateSubTask  (catalogue: specified/REA, port level 7)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame (local CMatrix; epilogue partly in a .HOODLUM detour 0x401BAF). Fields: +0xC m_pClient, +0x2C (u16) flags. `veh` = `m_pClient->m_pVehicle (+0x58C)` (no NULL checks). Allocation failures -> 0 unless noted.
- By taskType:
  - `0x2C0` (TASK_COMPLEX_LEAVE_CAR): `new(0x34) CTaskComplexLeaveCar(veh, 0, 0, true, false)` 0x63B8C0.
  - `0x2BC` (TASK_COMPLEX_ENTER_CAR_AS_PASSENGER): `new(0x50) CTaskComplexEnterCarAsPassenger(veh, 8, false)` 0x640340.
  - `0xCB` (TASK_SIMPLE_STAND_STILL): `new(0x20) CTaskSimpleStandStill(5000, false, false, 8.0f)`.
  - `0x2D5` (TASK_COMPLEX_CAR_DRIVE): `+0x2C = (+0x2C & 0xFFFD) | 1` (u16) first; `new(0x24) CTaskComplexCarDrive(veh)` 0x63C9D0.
  - `0x398` (TASK_COMPLEX_TURN_TO_FACE_ENTITY): `new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_pClient, [0x86FC7C]=0.5f, [0x86FC80]=0.2f)` 0x66B890.
  - `0x38B` (TASK_COMPLEX_SEEK_ENTITY): local `inv` (CMatrix, attach fields zeroed); `CMatrix::Invert(veh->m_matrix, &inv)` 0x59B920 (cdecl); `A = CCarEnterExit::GetPositionToOpenCarDoor(veh, 10)`, `B = ...(veh, 8)` 0x64E740. With P = ped position: `dA = P - A` (z, y, x order, dA.x stored f32), `dB = P - B`; `qA = (dA.x*dA.x + dA.y*dA.y) + dA.z*dA.z`, `qB = (dB.x*dB.x + dB.y*dB.y) + dB.z*dB.z`; `fcompp` (qB vs qA): if `qB > qA` use door A, else (qB <= qA or NaN) use door B; `off = inv * door` (CMatrix::operator* 0x59C890, vehicle-local offset). `p = new(0x58)`; `EAX = p ? CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>(p, veh, 50000, 1000, [0x86FC2C]=1.0f, [0x86FC28]=2.0f, [0x86FC30]=2.0f, false, false) : 0` 0x661DC0; then `EAX->+0x44..+0x4C = off` (executed also for EAX == 0: original NULL-write bug); `~CMatrix(&inv)` 0x59ACD0; return EAX.
  - anything else -> 0.

## 0x6666A0 CTaskComplexProstituteSolicit::CreateFirstSubTask  (catalogue: specified/REA, port level 8)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Fields: +0xC m_pClient (CPed*), +0x10 CVector, +0x2C flags.
- Behaviour: `if !CTaskComplexProstituteSolicit::IsTaskValid(ped, m_pClient)` 0x661BB0 (cdecl) -> `+0x2C |= 4` and return 0. Else: `V = m_pClient->m_pVehicle (+0x58C)` position (matrix pos or placement; no NULL check on the vehicle) copied to `this->+0x10`. `m_pClient->m_pPlayerData (+0x480)->m_pCurrentProstitutePed (+0xA4) = ped` (raw store, the old value is NOT cleaned up); `RegisterReference(FindPlayerPed(-1)->m_pPlayerData->+0xA4, &that field)` (0x666718; uses player 1's player data, not the client's - identical when the client is player 1). Then `slot = &m_pClient->m_pPlayerData->m_pLastProstituteShagged (+0xA8)`: `if *slot != ped`: `if *slot` -> CleanUpOldReference(*slot, slot); `*slot = ped`; RegisterReference(ped, slot). Return `CreateSubTask(this, 0x38B TASK_COMPLEX_SEEK_ENTITY, ped)` 0x666360.

## 0x666780 CTaskComplexProstituteSolicit::CreateNextSubTask  (catalogue: specified/REA, port level 8)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Fields: +0xC m_pClient, +0x10 CVector (car position), +0x2C/+0x2D flag bytes. "Create(t)" = `CreateSubTask(this, t, ped)` 0x666360. "LookAt(ms)" = `IKChainManager_c::LookAt(&g_ikChainMan, "TaskProzzy" [0x86FC4C], ped, m_pClient, ms, -1, NULL, false, 0.25f, 500, 3, false)` 0x618970.
- Behaviour: if `m_pClient == NULL` -> return EAX = 0 (EAX holds the NULL pointer). `if !IsTaskValid(ped, m_pClient)` 0x661BB0 -> `+0x2C |= 4` (continues anyway). By `t = m_pSubTask->GetTaskType()`:
  - `0x2C0` (LEAVE_CAR): LookAt(2500 (0x9C4)); return `Create(0x516)`.
  - `0x2BC` (ENTER_CAR_AS_PASSENGER): `CPed::Say(ped, 0xC5, 0, 1.0f, 0, 0, 0)`; `+0x10..+0x18 = m_pClient->m_pVehicle position` (matrix pos or placement, no NULL check); return `Create(0x2D5)`.
  - `0xCB` (STAND_STILL): if `+0x2D & 4`: if `byte [0x96918A] == 0` and `CPlayerPed::GetPlayerInfoForThisPlayerPed(FindPlayerPed(-1))->+0xB8 < 0x14` (signed): `CMessages::ClearMessages(false)` 0x69DCD0; `CMessages::AddMessageQ(TheText.Get("PROS_06" [0x86FC44]), 2000, 1, true)` and `AddMessageQ(TheText.Get("PROS_09" [0x86FC3C]), 3000, 1, true)` (CText::Get 0x6A0050 on TheText 0xC1B340; AddMessageQ 0x69F0B0 cdecl args (text, time, flag 1, flag 1)); then return `Create(0x516)`. If `byte [0x96918A] != 0` or the +0xB8 value is >= 0x14 -> return `Create(0x2BC)`. If `!(+0x2D & 4)` -> return `Create(0x516)`.
  - `0x2D5` (CAR_DRIVE) -> return `Create(0x2C0)`.
  - `0x398` (TURN_TO_FACE_ENTITY): `Say(ped, 0xC0, 0, 1.0f, 0, 0, 0)`; `AddMessageQ(TheText.Get("PROS_04" [0x86FC34]), 5000, 1, true)`; return `Create(0xCB)`.
  - `0x38B` (SEEK_ENTITY): LookAt(5000); return `Create(0x398)`.
  - anything else -> 0.
