# CTaskSimpleEvasiveDive (AI) - RE-SPEC-AI-2 group J3

## 0x6535C0 CTaskSimpleEvasiveDive::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1F7` (0x6535C0); `ret` -> returns 503 (TASK_SIMPLE_EVASIVE_DIVE). Vtable: slot 4 (+0x10) of vtable 0x86F244.

## 0x655F00 CTaskSimpleEvasiveDive::~CTaskSimpleEvasiveDive  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleEvasiveDive::~CTaskSimpleEvasiveDive 0x6535D0 (0x655F03); if `flags & 1` (0x655F08) calls CTask::operator delete 0x61A5B0 (this) (0x655F10, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F244.

## 0x653560 CTaskSimpleEvasiveDive::CTaskSimpleEvasiveDive  (catalogue: specified/REA, port level 2)
- `__thiscall this (CVehicle* vehicle [esp+4])`, `ret 4`, SEH frame. CTaskSimple::CTaskSimple() 0x61A390; vtbl = 0x86F244;
  +8 m_pVehicle = vehicle; +0xC = 0 (u8, m_bFinished); +0x10 m_pAnim = 0; if vehicle != NULL: CEntity::RegisterReference(&this->+8)
  0x571B70 (thiscall on vehicle). Returns this.

## 0x6535D0 CTaskSimpleEvasiveDive::~CTaskSimpleEvasiveDive  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, plain `ret`, SEH frame. vtbl = 0x86F244; if +8 (m_pVehicle) != NULL: vehicle->CleanUpOldReference(&this->+8)
  0x571A00; if +0x10 (m_pAnim) != NULL: m_pAnim->SetFinishCallback(0x4D1390 DefaultAnimCB, NULL) 0x4CEBE0; CTaskSimple::~CTaskSimple
  0x61A3A0.

## 0x653640 CTaskSimpleEvasiveDive::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`, result AL.
- If priority == 2 (ABORT_PRIORITY_IMMEDIATE): if this+0x10 (m_pAnim) != NULL: m_pAnim+0x1C (blend delta) = -1000.0f (imm 0xC47A0000);
  return 1.
- Else: return 1 only if event != NULL and event->GetEventType() (vtable +4) == 9 (EVENT_DAMAGE) and event+0x14 (source entity) != NULL
  and (source->m_nType (+0x36) & 7) == 2 (vehicle) and event+0x1C (weapon type) is 0x31 (RAMMEDBYCAR) or 0x32 (RUNOVERBYCAR);
  otherwise return 0.

## 0x6536A0 CTaskSimpleEvasiveDive::FinishAnimEvasiveDiveCB  (catalogue: specified/REA, port level 0)
- `__cdecl void (CAnimBlendAssociation* anim [esp+4], void* data [esp+8])` (static anim callback; data = the task). Sets data+0xC
  (u8 m_bFinished) = 1 and data+0x10 (m_pAnim) = 0. anim unused.

## 0x6558E0 CTaskSimpleEvasiveDive::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x14) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskSimpleEvasiveDive::CTaskSimpleEvasiveDive(this+8) 0x653560; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x657AC0 CTaskSimpleEvasiveDive::ProcessPed  (catalogue: specified/REA, port level 14)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, result AL. If +0xC (m_bFinished, u8) != 0 return 1. If +0x10 (m_pAnim) == NULL:
  this->StartAnim(ped) 0x655F20. Return 0.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x655F20 CTaskSimpleEvasiveDive::StartAnim  (catalogue: specified/REA, port level 13)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Caller: CTaskSimpleEvasiveDive::ProcessPed 0x657AC0. Fields: +8 m_pVehicle, +0x10 m_pAnim.
- Behaviour: `CPed::Say(ped, 0x4A, 0, 1.0f, 0, 0, 0)` 0x5EFFE0 (result ignored); `a = CAnimManager::BlendAnimation(ped->m_pRwClump, 0, 0x7F, 8.0f)` 0x4D4610; `m_pAnim = a`; `a->SetFinishCallback(CTaskSimpleEvasiveDive::FinishAnimEvasiveDiveCB 0x6536A0, this)` 0x4CEBE0. Then `if m_pVehicle != NULL && ped->m_nPedType (+0x598) == 6 (PED_TYPE_COP) && m_pVehicle->m_pDriver (+0x460) != NULL && CPed::IsPlayer(m_pVehicle->m_pDriver)`: `CWanted::RegisterCrime_Immediately(CWorld::FindPlayerWanted(-1) 0x56E230, 8, pedPos, ped, false)` 0x562430, then the same with crime type 9 (pedPos = matrix pos or placement, recomputed for each call). Crime ids 8 and 9 as raw values (gr eCrimeType names to be checked in the port's enum).
