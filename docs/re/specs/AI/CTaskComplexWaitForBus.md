# CTaskComplexWaitForBus (AI) - RE-SPEC-AI-2 group J3

## 0x632920 CTaskComplexWaitForBus::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xED` (0x632920); `ret` -> returns 237 (TASK_COMPLEX_WAIT_FOR_BUS). Vtable: slot 4 (+0x10) of vtable 0x86E18C (CTaskComplexWaitForBus).

## 0x632A40 CTaskComplexWaitForBus::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x632A40); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86E18C (CTaskComplexWaitForBus).

## 0x636A60 CTaskComplexWaitForBus::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x10) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplex::CTaskComplex() 0x61A3B0; vtbl = 0x86E18C (CTaskComplexWaitForBus); p+0xC = 0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x638940 CTaskComplexWaitForBus::CreateFirstSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`, SEH frame. p = new(0xC) 0x61A5A0; if NULL return NULL; CTaskSimple::CTaskSimple
  0x61A390; vtbl = 0x86E1B8 (an inline-constructed simple task, gr CTaskSimpleWaitUntilPedIsOutCar-like class with one field; class
  of vtable 0x86E1B8 UNCLEAR); p+8 = 0; return p.
- Open question: identity of the class with vtable 0x86E1B8 (0x63897A).

## 0x632940 CTaskComplexWaitForBus::CreateNextSubTask  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame. Fields: +8 m_pSubTask, +0xC m_bus (CVehicle*).
- t = m_pSubTask->GetTaskType() (vtable +0x10):
  - 0xEE (238, SIMPLE_WAIT_FOR_BUS): m_bus = m_pSubTask+8 (the wait task's bus); return new(0x50) CTaskComplexEnterCarAsPassenger(m_bus,
    0, 0) 0x640340 (NULL if the allocation fails).
  - 0x2BC (700, ENTER_CAR_AS_PASSENGER): ped+0x46C |= 0x20000000; ped+0x1C (entity flags) |= 0x800 (m_bRemoveFromWorld). p =
    FindPlayerPed(-1) 0x56E210. If p != NULL and m_bus->IsPassenger(p) 0x6D1BD0 -> fare; else (including p == NULL!) if
    m_bus->IsDriver(p) 0x6D1C40 -> fare. fare: p->m_pPlayerData (+0x480)->+0x80 += 5 (m_nBusFaresCollected). Return NULL.
  - other: return NULL.
- Original quirk: with no player ped, IsDriver(NULL) is still called (0x6329A5 jumps to 0x6329B4); if it returns true the fare update
  dereferences NULL+0x480 - keep.
- gta-reversed (TaskComplexWaitForBus.cpp CreateNextSubTask, not hooked here): skips IsDriver when the player is NULL and uses
  NOTSA_UNREACHABLE for other types (the original returns NULL). Otherwise equivalent.
