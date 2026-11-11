# CTaskComplexKillPedFromBoat (AI) - RE-SPEC-AI-2 group J2

## 0x622820 CTaskComplexKillPedFromBoat::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x408` (0x622820); `ret` -> returns 1032 (TASK_KILL_PED_FROM_BOAT). Vtable: slot 4 (+0x10) of vtable 0x86DA98 (CTaskComplexKillPedFromBoat).

## 0x6298E0 CTaskComplexKillPedFromBoat::~CTaskComplexKillPedFromBoat  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexKillPedFromBoat::Destructor 0x622830 (0x6298E3); if `flags & 1` (0x6298E8) calls CTask::operator delete 0x61A5B0 (this) (0x6298F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DA98 (CTaskComplexKillPedFromBoat).

## 0x622890 CTaskComplexKillPedFromBoat::CreateNextSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4] unused)`, `ret 4`, SEH frame. p = CTask::operator new(0x20) 0x61A5A0; if NULL return
  NULL; else CTaskSimpleStandStill::CTaskSimpleStandStill(0, 1, 0, 8.0f (imm 0x41000000)) 0x62F310 (time 0, looped true, holdLast
  false, blend 8.0) and return p.

## 0x622900 CTaskComplexKillPedFromBoat::CreateFirstSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame. First ped+0x46C |= 0x200000 (ped flag bit 21 of the first flag
  dword) - done before the allocation. Then identical to 0x622890: new(0x20) + CTaskSimpleStandStill(0, 1, 0, 8.0f) 0x62F310, NULL on
  allocation failure.

## 0x6238A0 CTaskComplexKillPedFromBoat::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x10) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexKillPedFromBoat::CTaskComplexKillPedFromBoat(this+0xC) 0x6227C0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x622980 CTaskComplexKillPedFromBoat::ControlSubTask  (catalogue: specified/REA, port level 6)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame. Default result = m_pSubTask (+8) (EBX).
- Callees: CPed::IsPlayer 0x5DF8F0, CAudioEngine::SayPedless 0x5073C0 (this = AudioEngine 0xB6BC90), CWorld::FindPlayerWanted(-1)
  0x56E230, CTask::operator new 0x61A5A0, CTaskSimpleStandStill ctor 0x62F310, CTaskComplexKillPedOnFoot ctor 0x620E30.
- Fields: +8 m_pSubTask, +0xC target ped. ped+0x598 m_nPedType (6 = COP), ped+0x46C flags (0x100 = in vehicle), ped+0x58C vehicle,
  vehicle+0x1B8 (u8, compared with 3), ped+0x40 (physical flags, bit 0x100), wanted+0x2C m_nWantedLevel.
- Behaviour: t = m_pSubTask->GetTaskType() (vtable +0x10).
  - t == 0x3E8 (1000, KILL_PED_ON_FOOT):
    1. Speech (0x6229BE-0x622A32): if ped type == 6 and target != NULL and target->IsPlayer(): if target in vehicle and target+0x58C
       != NULL: if vehicle+0x1B8 == 3: SayPedless(0x34, 0xAD, ped, 0, 1.0f, 0, 0, 0); (otherwise no speech). Else (not in a vehicle or
       vehicle NULL): if target+0x40 & 0x100: SayPedless(0x34, 0xAF, ped, 0, 1.0f, 0, 0, 0).
    2. If ped type == 6 and target != NULL and target->IsPlayer() and FindPlayerWanted(-1)->m_nWantedLevel == 0 and
       m_pSubTask->MakeAbortable(ped, 1, NULL) (vtable +0x18): return new(0x20) CTaskSimpleStandStill(0, 1, 0, 8.0f (imm 0x41000000))
       (NULL if the allocation fails). Otherwise return m_pSubTask.
  - t == 0xCB (203, SIMPLE_STAND_STILL): if ped type == 6 and target != NULL and target->IsPlayer() and wanted level != 0 and
    m_pSubTask->MakeAbortable(ped, 1, NULL): return new(0x38) CTaskComplexKillPedOnFoot(target, -1, 0, 0, 0, 1) (NULL on failure);
    otherwise m_pSubTask.
  - any other type: return m_pSubTask.
- gta-reversed: not hooked at this address (catalogue source REA).
