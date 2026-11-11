# CTaskSimpleArrestPed (AI) - RE-SPEC-AI-2 group J3

## 0x68B680 CTaskSimpleArrestPed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x44C` (0x68B680); `ret` -> returns 1100 (TASK_SIMPLE_ARREST_PED). Vtable: slot 4 (+0x10) of vtable 0x870984 (CTaskSimpleArrestPed).

## 0x68CF40 CTaskSimpleArrestPed::~CTaskSimpleArrestPed  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleArrestPed::~CTaskSimpleArrestPed 0x68B690 (0x68CF43); if `flags & 1` (0x68CF48) calls CTask::operator delete 0x61A5B0 (this) (0x68CF50, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870984 (CTaskSimpleArrestPed).

## 0x68B620 CTaskSimpleArrestPed::CTaskSimpleArrestPed  (catalogue: specified/REA, port level 2)
- `__thiscall this (CPed* ped [esp+4])`, `ret 4`, SEH frame. CTaskSimple::CTaskSimple() 0x61A390; vtbl = 0x870984; +8 m_pPed = ped;
  +0xC = 0 (u8 finished); +0x10 m_pAnim = 0; if ped != NULL: ped->RegisterReference(&this->+8) 0x571B70. Returns this.

## 0x68B690 CTaskSimpleArrestPed::~CTaskSimpleArrestPed  (catalogue: specified/REA, port level 3)
- `__thiscall void ()`, plain `ret`, SEH frame. vtbl = 0x870984. If +0x10 (m_pAnim) != NULL: SetFinishCallback(0x4D1390, NULL)
  0x4CEBE0. If +8 (m_pPed) != NULL: CleanUpOldReference(&this->+8) 0x571A00 (the pointer itself is not cleared).
  Then if m_pAnim (+0x10, re-read) != NULL and m_pPed != NULL and !m_pPed->IsPlayer() 0x5DF8F0 and (m_pPed+0x46C & 0x100) (in vehicle)
  and m_pPed+0x530 (ped state) == 0x3F (63, PEDSTATE_ARRESTED): m_pPed->SetPedState(0x32 = 50, PEDSTATE_DRIVING) 0x5E4500.
  Finally CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x68B7D0 CTaskSimpleArrestPed::FinishAnimArrestPedCB  (catalogue: specified/REA, port level 0)
- `__cdecl void (CAnimBlendAssociation* anim, void* data [esp+8])`: data+0xC (u8) = 1; data+0x10 (m_pAnim) = 0.

## 0x68CD10 CTaskSimpleArrestPed::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x14) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskSimpleArrestPed::CTaskSimpleArrestPed(this+8) 0x68B620; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
