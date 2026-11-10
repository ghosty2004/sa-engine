# CTaskSimpleEvasiveStep (AI) - RE-SPEC-3 group I2
Vtable 0x86F1F4, type 0x1F5 (501), size 0x14. Layout: +8 m_pEntity (registered ref), +0xC m_bFinished u8, +0x10 m_pAnim.

## 0x653160 CTaskSimpleEvasiveStep::CTaskSimpleEvasiveStep  (catalogue: specified/REA, port level 1)
- `__thiscall this (CEntity* e)`, `ret 4`, SEH: `CTaskSimple()` 0x61A390; vtbl = 0x86F1F4; `+8 = e`; `+0xC = 0`; `+0x10 = 0`; if e
  `RegisterReference(e, &+8)`.

## 0x6531C0 CTaskSimpleEvasiveStep::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x1F5.

## 0x6531D0 CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep  (catalogue: specified/REA, port level 1)
- Complete dtor (SEH): vtbl = 0x86F1F4; if m_pAnim `m_pAnim->SetFinishCallback(0x4D1390, nullptr)`; if m_pEntity
  `CleanUpOldReference(e, &+8)`; `~CTaskSimple` 0x61A3A0. (m_pAnim is not blended out here.)

## 0x653290 CTaskSimpleEvasiveStep::FinishAnimEvasiveStepCB  (catalogue: specified/REA, port level 0)
- `__cdecl void (CAnimBlendAssociation*, void* task)`: `task->m_pAnim->m_nFlags (+0x2E) |= 4`; if `m_pAnim->m_fBlendDelta (+0x1C) >= 0.0f`
  (`test ah,1`; NaN counts as less -> unchanged) set it to -4.0f; `task->+0xC = 1`; `task->+0x10 = 0`.

## 0x655E80 CTaskSimpleEvasiveStep::`scalar deleting destructor'  (catalogue: specified/REA, port level 1)
- call 0x6531D0; `if (flags & 1) CTask::operator delete`; return this; `ret 4`.

## 0x655EA0 CTaskSimpleEvasiveStep::StartAnim  (catalogue: specified/REA, port level 9)
- `__thiscall void (this, CPed* ped)`, `ret 4`: `m_pAnim = CAnimManager::BlendAnimation(ped->m_pRwClump, 0, 0x7E, 8.0f)` 0x4D4610;
  `m_pAnim->+0x2E &= ~4`; `m_pAnim->SetFinishCallback(FinishAnimEvasiveStepCB 0x653290, this)`.

## 0x657A60 CTaskSimpleEvasiveStep::ProcessPed  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped)`, `ret 4`: if m_bFinished or m_pEntity == 0 return 1. If m_pAnim == 0: if
  `(m_pEntity->+0x36 & 7) == 2` (entity type vehicle) `ped->Say(0x4A, 0, 1.0f, 0, 0, 0)` 0x5EFFE0; `StartAnim(ped)`. Return 0.
