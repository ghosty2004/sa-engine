# CTaskComplexUseMobilePhone (AI) - RE-SPEC-AI-2 group J3

## 0x6348D0 CTaskComplexUseMobilePhone::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x640` (0x6348D0); `ret` -> returns 1600 (TASK_COMPLEX_USE_MOBILE_PHONE). Vtable: slot 4 (+0x10) of vtable 0x86E454.

## 0x639660 CTaskComplexUseMobilePhone::~CTaskComplexUseMobilePhone  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006348e0 0x6348E0 (0x639663); if `flags & 1` (0x639668) calls CTask::operator delete 0x61A5B0 (this) (0x639670, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E454.

## 0x636FD0 CTaskComplexUseMobilePhone::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x20) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplex::CTaskComplex() 0x61A3B0; p+0xC = this+0xC; vtbl = 0x86E454 (CTaskComplexUseMobilePhone); p+0x10 = 0; p+0x14 = 0; p+0x18 = 0 (u8); p+0x19 = 0 (u8); p+0x1C = 0 (u8); p+0x1D = 0 (u8); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x634930 CTaskComplexUseMobilePhone::MakeAbortable  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`, AL.
- Fields: +8 m_pSubTask, +0xC duration, +0x10 timer start, +0x14 timer interval, +0x18 timer started (u8), +0x1C u8 flag.
- Behaviour:
  1. priority == 2: r = m_pSubTask->MakeAbortable(ped, 2, event) (vtable +0x18). If r: ped+0x478 &= ~2; ped+0x474 &= 0xFFBFFFFF;
     if ped+0x70C (saved weapon) != 0x37: 0x6348F0(ped) (see below). Return r.
  2. Else if m_pSubTask->GetTaskType() == 0xCA (202, SIMPLE_PAUSE): ped+0x478 &= ~2; ped+0x474 &= 0xFFBFFFFF; return 1.
  3. Else: if event != NULL: if event->GetEventType() (vtable +4) == 0x3A (EVENT_ON_FIRE) and ped->IsPlayer(): return 1 (nothing
     changed); otherwise +0x1C = 1. (event == NULL: +0x1C not touched.) Then restart the timer: +0x10 = m_snTimeInMilliseconds,
     +0x14 = +0xC, +0x18 = 1. If m_pSubTask->GetTaskType() == 0x643 (1603, SIMPLE_PHONE_OUT): m_pSubTask->MakeAbortable(ped,
     priority, event) (result ignored). Return 0.
- 0x6348F0 (thiscall `void (CPed* ped)`, `ret 4`, unnamed; proposed `CTaskComplexUseMobilePhone::RestorePedWeapon`):
  ped->RemoveWeaponModel(0x14A = 330, the phone model) 0x5E3990; ped->SetCurrentWeapon(ped+0x70C) 0x5E6280; ped+0x70C = 0x37;
  if ped+0x480 (player data) != NULL: pd+0x85 = 0 (u8).
- gta-reversed: not hooked at this address.

## 0x634A60 CTaskComplexUseMobilePhone::CreateNextSubTask  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame. RunAnim(...) below = new(0x20)
  CTaskSimpleRunAnim(group, anim, blend, taskType, name, holdLastFrame) 0x61A900 (NULL on allocation failure).
- t = m_pSubTask (+8)->GetTaskType():
  - 0x641 (1601, PHONE_TALK): return RunAnim(0, 0x92, 4.0f, 0x643, "PhoneOut" [0x86E48C], 0).
  - 0xCA (202, PAUSE): +0x1C = 0; +0x18 = 0 (timer stopped); if +0x1D == 0: return this->vtable[+0x24](ped) (CreateFirstSubTask, slot
    9); else: ped+0x478 &= ~2; ped+0x474 &= 0xFFBFFFFF; return NULL.
  - 0x642 (1602, PHONE_IN): if +0xC (duration) >= 0: +0x10 = m_snTimeInMilliseconds, +0x14 = +0xC, +0x18 = 1. Return RunAnim(0, 0x93,
    4.0f, 0x641, "PhoneChat" [0x86E480], 0).
  - 0x643 (1603, PHONE_OUT): if +0x1D == 0 and +0x1C != 0: return new(0x18) CTaskSimplePause(1000) 0x48E750 (NULL on failure).
    Else: ped+0x478 &= ~2; ped+0x474 &= 0xFFBFFFFF; return NULL.
  - other: NULL.
- gta-reversed: not hooked at this address.

## 0x634C10 CTaskComplexUseMobilePhone::CreateFirstSubTask  (catalogue: specified/REA, port level 15)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame, local CTaskSimpleStandStill at [esp+8] (0x24 bytes).
- Behaviour:
  1. ped+0x478 |= 2; ped+0x474 |= 0x400000. If ped->IsPlayer(): CPlayerPed::ClearWeaponTarget 0x609C80.
  2. If ped+0x70C (saved weapon) == 0x37: ped+0x70C = weapon type of the active slot (ped+0x5A0 + 0x1C*(int8)ped+0x718);
     ped->SetCurrentWeapon(0) 0x5E6280 (weapon type 0 = unarmed).
  3. If ped+0x480 (player data) != NULL: pd+0x85 = 1. If ped->IsPlayer(): TheCamera.ClearPlayerWeaponMode() 0x50AB10.
  4. Local stand-still inline-constructed: CTaskSimple ctor 0x61A390; vtbl = 0x86DD2C; +8 = 0; +0xC = 0; +0x10 = 0; +0x14 = 0 (u8);
     +0x15 = 0; +0x18 = 0; +0x19 = 0; +0x1C = 8.0f (imm 0x41000000). local.ProcessPed(ped) 0x62F370 (result ignored).
  5. r = new(0x20) CTaskSimpleRunAnim(0, 0x91, 4.0f, 0x642, "PhoneIn" [0x86E498], 0) 0x61A900 (NULL on failure).
  6. Destroy the local (vtbl = 0x86DD2C again, CTaskSimple::~CTaskSimple 0x61A3A0). Return r.
- gta-reversed: not hooked at this address.

## 0x634D60 CTaskComplexUseMobilePhone::ControlSubTask  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame. Result defaults to m_pSubTask (+8, read at entry).
- Behaviour:
  1. pd = ped+0x480; if pd != NULL: pd+0x85 = 1.
  2. If m_pSubTask->GetTaskType() != 0xCA (PAUSE):
     a = RpAnimBlendClumpGetAssociation(ped clump +0x18, 0x91) 0x4D68B0 (phone-in), b = ...(clump, 0x92) (phone-out).
     - a != NULL: if a+0x20 (current time) >= 0.85f [0x862E1C] (`test ah,1; jne` skips when less or NaN): ped->AddWeaponModel(0x14A)
       0x5E5ED0.
     - else b != NULL: if b.time >= [0x86E4A0]=1.566 (f64 constant, f32 operand) and (b.time - b+0x28 (time step)) < 1.566 (`fsubr`
       then `fcomp qword`, `test ah,5; jp`): 0x6348F0(ped) (remove phone, restore weapon - see MakeAbortable 0x634930).
     - else (neither anim): if CTaskTimer::IsOutOfTime(this+0x10) 0x420E30: result = new(0x20) CTaskSimpleRunAnim(0, 0x92, 4.0f, 0x643,
       "PhoneOut" [0x86E48C], 0) (NULL on failure).
  3. If pd != NULL: pd+0x84 = 1; if pd+0x18 (f32) < 0.0f (`test ah,5; jp` skips when >= 0 or NaN): pd+0x18 = 0. t =
     ped->m_pIntelligence (+0x47C)->FindTaskByType(0 = TASK_SIMPLE_PLAYER_ON_FOOT) 0x600EE0; if t: t->PlayerControlZelda(ped, 1)
     0x6883D0. pd+0x84 = 0.
  4. Return result.
- gta-reversed: not hooked at this address.
