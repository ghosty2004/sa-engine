# CTaskGangHassleVehicle (AI) - RE-SPEC-AI-2 group J3

## 0x65FB50 CTaskGangHassleVehicle::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4B5` (0x65FB50); `ret` -> returns 1205 (TASK_COMPLEX_GANG_HASSLE_VEHICLE). Vtable: slot 4 (+0x10) of vtable 0x86F9D4 (CTaskGangHassleVehicle).

## 0x6637A0 CTaskGangHassleVehicle::~CTaskGangHassleVehicle  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskGangHassleVehicle::Destructor 0x65FB60 (0x6637A3); if `flags & 1` (0x6637A8) calls CTask::operator delete 0x61A5B0 (this) (0x6637B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F9D4 (CTaskGangHassleVehicle).

## 0x65FC00 CTaskGangHassleVehicle::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x44) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskGangHassleVehicle::CTaskGangHassleVehicle(this+0x10, this+0x14, (u8)this+0x18, this+0x1C, this+0x20) 0x65FAC0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x61D0B0 CTaskGangHassleVehicle::sub_61D0B0 (proposed: CTaskSimpleFight::FightHitCar)  (catalogue: unknown/REA, port level 15)
- Name correction: this is `CTaskSimpleFight::FightHitCar(CPed* attacker, CVehicle* vehicle, CVector& pos1, CVector& pos2, int16 component, int8 surface)` (gr declares it without an address); it uses the fight-task fields +0x24/+0x25 and the melee combo table. Callers: 0x6240B0 (strike code) and CTaskGangHassleVehicle::ControlSubTask 0x6637C0.
- `__thiscall void (this: CTaskSimpleFight*, CPed* attacker [esp+4], CVehicle* veh [esp+8], CVector* pos1 [esp+0xC], CVector* pos2 [esp+0x10], int16 component [esp+0x14], int8 surface [esp+0x18])`, `ret 0x18`; SEH frame (two local CEventVehicleDamage objects). Argument slots attacker/veh are reused for weaponType and the float damage.
- Behaviour:
  1. `oldHealth = veh->m_fHealth (+0x4C0)`; `dmg = 0x61C740(this, attacker)` (float, ST0, stored f32). `wt = attacker->m_aWeapons[(int8)attacker->m_nActiveWeaponSlot].m_eWeaponType`.
  2. If `wt == 9` (chainsaw): `h = GetAnimHierarchyFromSkinClump(attacker->m_pRwClump)` 0x734A40; `i = RpHAnimIDGetIndex(h, 0x18)` 0x7C51A0; `A = (RpHAnimHierarchyGetMatrixArray(h) 0x7C5120 + i*0x40)->+0x20` (the bone matrix's `at` vector, 3 dwords - not its position; original quirk); `Fx_c::AddSparks(&g_fx 0xA9AE00, pos1, &A, 5.0f, 0x20, A (by value), 1, 0.3f, 1.0f)` 0x49F040 (a zeroed local CVector is prepared but unused); then `veh->vfunc[+0xE0](I, component, attacker, pos1, pos2, 9)` (CVehicle::VehicleDamage) with `I = (veh->m_pHandlingData (+0x384)->+4 (fMass) * dmg) * [0x86D6AC]=0.00075f`. Else: `veh->vfunc[+0xE0](I, component, attacker, pos1, pos2, wt)` with `I = (fMass * dmg) * [0x858C58]=0.01f`.
  3. `CCrime::ReportCrime(0x15, veh, attacker)` 0x532010.
  4. If `veh->m_pDriver (+0x460)`: local `CEventVehicleDamage(veh, attacker, 5)` 0x4B18D0, vtable then set to 0x86D640 (CEventVehicleDamageWeapon); `CEventGroup::Add(&driver->m_pIntelligence->+0x68, &e, false)`; destroyed (vtable 0x86D640 restored, base dtor 0x4B1990). Same for each non-NULL `m_apPassengers[i]` (+0x464 + 4i), i = 0 .. `(u8)veh->m_nMaxPassengers (+0x488)` - 1 (the bound is re-read each iteration).
  5. If `veh->m_fHealth < oldHealth` (`fcomp; test ah, 5; jp` -> only C0): `veh->+0x508 = (u8)wt`; `veh->+0x50C = attacker` (raw store, the old reference is not cleaned up) and `RegisterReference(attacker, &veh->+0x50C)`.
  6. If the attacker's current weapon is 9 (re-read) -> `CAEPedWeaponAudioEntity::AddAudioEvent(&attacker->+0x394, 0x9B)`.
  7. `snd = dword [0xC16F0C + ((int8)this->+0x24 * 0x22 + (int8)this->+0x25) * 4]` (= `m_aComboData[combo - 4].+0x5C[move]`); `CAEPedAudioEntity::AddAudioEvent(&attacker->+0x138, snd, 0.0f, 1.0f, veh, (u8)surface, 0, 0)` 0x4E2BB0.
  8. `Fx_c::AddPunchImpact(&g_fx, pos1, pos2, 4)` 0x49F670.

## 0x65FC80 CTaskGangHassleVehicle::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame. Callers: ControlSubTask 0x6637C0, CreateFirstSubTask 0x664BA0. Fields: +0x10 vehicle, +0x24 CVector offset.
- Behaviour: if `+0x10 == NULL` -> 0. If `m_pSubTask != NULL && m_pSubTask->GetTaskType() == 0x38F` (TASK_COMPLEX_SMART_FLEE_ENTITY) -> 0. If `veh->m_fHealth (+0x4C0) < [0x859F80]=250.0f` (`fcomp; test ah, 5; jp` -> only C0) -> `p = new(0x40)`; NULL -> 0; return `CTaskComplexSmartFleeEntity::CTaskComplexSmartFleeEntity(p, veh, false, 30.0f (0x41F00000), [0x86F674] (int 1000000), [0x86F678] (int 1000), [0xC18CF0] (dword from .data, passed raw as the last float))` 0x65C430. Else: if the subtask exists and its type is 0x4BC (TASK_COMPLEX_GANG_HASSLE_PED) or 0x39D (TASK_COMPLEX_TRACK_ENTITY) -> 0 (GetTaskType called separately for each test); else `p = new(0x58)`; NULL -> 0; return `CTaskComplexTrackEntity::CTaskComplexTrackEntity(p, veh, CVector(+0x24) by value, 1, -1, 10.0f, 40.0f, 1)` 0x65F3B0.

## 0x65FDD0 CTaskGangHassleVehicle::GetTargetHeading  (catalogue: specified/REA, port level 1)
- `__thiscall float (this, unused [esp+4])`, `ret 4`; returns ST0. Caller: ControlSubTask 0x6637C0 (2 sites). Fields: +0x10 vehicle, +0x14 side index.
- Behaviour: `m = vehicle->m_matrix (+0x14)` (no NULL check); locals `F = m->forward (+0x10)`, `R = m->right (+0)`. Select a 2D direction (x, y) by `side`: 0 or 2 -> `R`; 1 or 3 -> `-R` (each component negated with fchs and stored as f32); 4 -> `F`; 5 -> `-F`; any other value -> `R`. Return `CGeneral::LimitRadianAngle(CGeneral::GetRadianAngleBetweenPoints(x, y, 0.0f, 0.0f))` 0x53CBE0 / 0x53CB50 (intermediate stored to an f32 local).

## 0x6641A0 CTaskGangHassleVehicle::CalcTargetOffset  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. Caller: CreateFirstSubTask 0x664BA0. Fields: +0x10 vehicle, +0x14 side index, +0x20 float distance d, +0x24/+0x28/+0x2C target offset (CVector).
- Behaviour: `off = (0, 0, 0)` (stores +0x2C, +0x28, +0x24). `c = CModelInfo::ms_modelInfoPtrs[(int16)veh->m_nModelIndex]->m_pColModel (+0x14)`; locals `mn = c->bbox.min (+0..+8)`, `mx = c->bbox.max (+0xC..+0x14)`. By side: 0 -> `off.x = mn.x - d; off.y = mx.y * [0x858B8C]=0.5f`; 1 -> `off.x = mx.x + d; off.y = mx.y * 0.5f`; 2 -> `off.x = mn.x - d; off.y = mn.y * 0.5f`; 3 -> `off.x = mx.x + d; off.y = mn.y * 0.5f`; 4 -> `off.y = mn.y - d`; 5 -> `off.y = mx.y + d`; other -> nothing. Each result is one fld/fadd-or-fsub/fstp (f32) or fld/fmul/fstp.

## 0x664BA0 CTaskGangHassleVehicle::CreateFirstSubTask  (catalogue: specified/REA, port level 11)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Fields: +0x10 vehicle, +0x14 hassle position id, +0x31 (u8), +0x34 driver (registered reference), +0x38/+0x3C/+0x40 CTaskTimer.
- Behaviour:
  1. If `veh (+0x10) == NULL` -> return 0. `+0x34 = veh->m_pDriver (+0x460)`; if non-NULL -> RegisterReference(driver, &+0x34) 0x571B70 (the old +0x34 value is not cleaned up).
  2. `c = CModelInfo::ms_modelInfoPtrs[veh->m_nModelIndex]->m_pColModel`; `w = c->bbox.max.x - c->bbox.min.x`; `fcomp [0x858B90]=4.0f; test ah, 0x41; je fail` -> return 0 if `w > 4.0` (equal or NaN pass). `l = max.y - min.y`; return 0 if `l > [0x859000]=8.0f`.
  3. `id = CVehicle::GetSpareHasslePosId(veh)` 0x6D3AE0; `+0x14 = id`; if `id == -1` -> return 0. `CVehicle::SetHasslePosId(veh, id, true)` 0x6D3B30. `CalcTargetOffset(this)` 0x6641A0.
  4. `+0x31 = 0`; `CPed::DropEntityThatThisPedIsHolding(ped, true)` 0x5E0360. Timer: `+0x38 = now ([0xB7CB84], read before the call); +0x3C = CGeneral::GetRandomNumberInRange(150000, 250000)` 0x407180 (original int version); `+0x40 = 1`.
  5. If `ped->bInVehicle && ped->m_pVehicle`: `p = new(0x34)`; NULL -> 0; return `CTaskComplexLeaveCar::CTaskComplexLeaveCar(p, ped->m_pVehicle, 0, 0, true, false)` 0x63B8C0. Else return `this->CreateNextSubTask(ped)` (vslot +0x20 = 0x65FC80).
