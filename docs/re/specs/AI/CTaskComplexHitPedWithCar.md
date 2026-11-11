# CTaskComplexHitPedWithCar (AI) - RE-SPEC-AI-2 group J2

## 0x653A20 CTaskComplexHitPedWithCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1F9` (0x653A20); `ret` -> returns 505 (TASK_COMPLEX_HIT_PED_WITH_CAR). Vtable: slot 4 (+0x10) of vtable 0x86F294 (CTaskComplexHitPedWithCar).

## 0x653A90 CTaskComplexHitPedWithCar::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x653A90); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86F294 (CTaskComplexHitPedWithCar).

## 0x653AE0 CTaskComplexHitPedWithCar::HitHurtsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, result AL.
- t = ped->IsPlayer() 0x5DF8F0 ? [0x86F124]=10.0f : [0x86F11C]=6.0f. `fld t; fcomp this+0x10 (f32, impact speed); test ah,5; jp`:
  if t >= speed is NOT the case... precisely: return 1 when t < speed (C0 set alone -> PF=0 -> fallthrough to 0x653B0B).
  If t >= speed or NaN (PF=1) continue: if ped+0xE8 (f32, gr m_fHealth? field at +0xE8 - UNCLEAR name) < -0.8f [0x858C9C] (`test ah,5;
  jp` taken when NOT less / NaN) and this+0x10 > 3.0f [0x858B3C] (`test ah,0x41; je`) -> return 1; otherwise return 0.
- Open question: meaning of ped+0xE8 (0x653B12; CPhysical area, possibly a velocity component); the comparison is as stated.

## 0x6559B0 CTaskComplexHitPedWithCar::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x28) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexHitPedWithCar::CTaskComplexHitPedWithCar(this+0xC, this+0x10) 0x6539A0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x657AF0 CTaskComplexHitPedWithCar::CreateNextSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`. t = m_pSubTask (+8)->GetTaskType() (vtable +0x10):
  - t in {0x1F6 502 EVASIVE_STEP, 0xC8 200 NONE, 0xD0 208 FALL_AND_GET_UP, 0x19E 414 HIT_BEHIND, 0x1FA 506 KILL_PED_WITH_CAR}:
    return this->CreateSubTask(0x516 TASK_FINISHED) 0x6560E0;
  - t == 0x1FB (507 HURT_PED_WITH_CAR): if m_pSubTask+0x10 (u8) == 0 return CreateSubTask(0xD0 FALL_AND_GET_UP); else NULL;
  - otherwise NULL.

## 0x653B40 CTaskComplexHitPedWithCar::ComputeEvasiveStepMoveDir  (catalogue: specified/REA, port level 7)
- Convention: static, `__cdecl CVector* (CVector* out [esp+4] (hidden return), X a2 [esp+8], CVehicle* vehicle [esp+0xC])`, plain
  `ret`, returns `out` in EAX. ECX unused.
- side = CPedGeometryAnalyser::ComputeEntityHitSide(a2, vehicle) 0x5F3BC0 (cdecl, args in that order). Jump table 0x653BF8:
  0 -> d = matrix->forward (+0x10); 1 -> d = -matrix->right (+0); 2 -> d = -matrix->forward; 3 -> d = matrix->right (matrix =
  vehicle+0x14, no NULL check). Negation is `fmul [0x858C1C]=-1.0f` per component, stored f32. side > 3 (unsigned): d is the
  UNINITIALISED stack local (original bug, keep). *out = d.
- gta-reversed: not hooked at this address.
