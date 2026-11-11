# CTaskComplexKillPedOnFoot (AI) - RE-SPEC-AI-2 group J2

## 0x620EE0 CTaskComplexKillPedOnFoot::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3E8` (0x620EE0); `ret` -> returns 1000 (TASK_COMPLEX_KILL_PED_ON_FOOT). Vtable: slot 4 (+0x10) of vtable 0x86D894 (CTaskComplexKillPedOnFoot).

## 0x625E20 CTaskComplexKillPedOnFoot::~CTaskComplexKillPedOnFoot  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00620ef0 0x620EF0 (0x625E23); if `flags & 1` (0x625E28) calls CTask::operator delete 0x61A5B0 (this) (0x625E30, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D894 (CTaskComplexKillPedOnFoot).

## 0x626A80 CTaskComplexKillPedOnFoot::~CTaskComplexKillPedOnFoot  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexKillPedOnFootStealth::Destructor 0x620FF0 (0x626A83); if `flags & 1` (0x626A88) calls CTask::operator delete 0x61A5B0 (this) (0x626A90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D8C0 (CTaskComplexKillPedOnFootStealth).

## 0x623370 CTaskComplexKillPedOnFoot::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x38) 0x61A5A0 (cdecl); if p == NULL the constructor is skipped and EAX = 0 (see the trailing stores). Else p->CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(this+0x10, this+0x28, this+0x14, this+0x18, this+0x1C, (int32)(int8)this+0x20) 0x620E30 (args in push order reversed: first arg this+0x10). Then (OUTSIDE the NULL check, 0x6233C7) p+0xC bit 4 (0x10) = this+0xC bit 4, then p+0xC bit 5 (0x20) = this+0xC bit 5 (xor/and/xor merges). Original bug: if operator new returned NULL, EAX = 0 and these two byte writes hit address 0xC (crash) - keep; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x625E40 CTaskComplexKillPedOnFoot::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)`, `ret 0xC`. First ped+0x474 &= 0xFFBFFFFF (clear ped flag
  bit 22 of the third flag dword). If m_pSubTask (+8) == NULL return 1; else return m_pSubTask->MakeAbortable(ped, priority, event)
  (vtable +0x18).

## 0x625E70 CTaskComplexKillPedOnFoot::CreateSubTask  (catalogue: specified/REA, port level 10)
- Convention: `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH frame (`sub esp,0x20`; the local
  is a CTaskSimpleStandStill used by case 202). Entry 0x625E70 is `nop; jmp 0x40A0F8` (relocated `mov eax, fs:[0]`), back at 0x625E76.
- Callers: ControlSubTask 0x626260, CreateNextSubTask 0x62B150, CreateFirstSubTask 0x62B490, CTaskComplexKillPedOnFootStealth::
  CreateNextSubTask 0x62B790.
- Fields: +0xC flags byte (bits 0x10, 0x20, 0x40), +0x10 target ped, +0x14/+0x18/+0x1C/+0x20 (armed sub task parameters), +0x2C
  timer start, +0x30 timer interval, +0x34 timer started (u8).
- Behaviour by taskType (all allocations via CTask::operator new 0x61A5A0; a failed allocation yields NULL unless stated):
  - 0xCA (202, SIMPLE_PAUSE): local StandStill(0, 0, 0, 8.0f) 0x62F310; local.ProcessPed(ped) 0x62F370 (result ignored); if (+0xC &
    0x20) and (+0xC & 0x40): new(0x18) CTaskSimplePause(10000) 0x48E750, else new(0x18) CTaskSimplePause(2000); destroy the local
    (0x62F360); return the pause task.
  - 0xC8 (200): new(8) CTaskSimpleLeaveGroup() 0x463540.
  - 0xCB (203, STAND_STILL): new(0x20) StandStill(0, 0, 0, 8.0f).
  - 0x2BF (703, DRAG_PED_FROM_CAR): new(0x54) CTaskComplexDragPedFromCar(target, 0) 0x640430; then (also when NULL) +0x2C = now,
    +0x30 = 2000, +0x34 = 1.
  - 0x2C0 (704, LEAVE_CAR): new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle (+0x58C), 0, 0, 1, 1) 0x63B8C0.
  - 0x2CF (719, CAR_DRIVE_TIMED): new(0x70) CTaskSimpleCarDriveTimed(ped->m_pVehicle, 2000) 0x5FF940.
  - 0x3E9 (1001, MELEE): new(0x38) CTaskComplexKillPedOnFootMelee(target) 0x621000; then +0x2C = now, +0x30 = 2000, +0x34 = 1.
  - 0x3EA (1002, ARMED): new(0x5C) CTaskComplexKillPedOnFootArmed(target, +0x14, +0x18, +0x1C, (int32)(int8)+0x20) 0x621190; then,
    OUTSIDE the NULL check, result+0x35 = (+0xC >> 4) & 1 (original bug: NULL result -> write to address 0x35); +0xC &= 0xEF;
    +0x2C = now, +0x30 = 2000, +0x34 = 1.
  - 0x3EB (1003, DESTROY_CAR): new(0x20) CTaskComplexDestroyCar(target->m_pVehicle (+0x58C), 0, 0, 0) 0x621C00.
  - 0x4BA (1210, SIGNAL_AT_PED): if target == FindPlayerPed(0): ped->Say(0x103, 0, 1.0f, 0, 0, 0) 0x5EFFE0; else if
    CPedGroups::GetPedsGroup(target) 0x5F7E80 != NULL and (group+8).GetLeader() 0x5F69A0 == FindPlayerPed(0): no speech; else
    ped->Say(0x102, 0, 1.0f, 0, 0, 0). Then r = CGeneral::GetRandomNumberInRange(0, 1500) 0x407180 (the original int version, see
    cat14 finding 1); new(0x18) CTaskComplexSignalAtPed(target, r, 1) 0x660A30.
  - 0x516 (1302, FINISHED): ped+0x474 &= 0xFFBFFFFF; return NULL.
  - anything else: NULL.
  "now" = CTimer::m_snTimeInMilliseconds 0xB7CB84.
- gta-reversed: plugin::CallMethodAndReturn stub (TaskComplexKillPedOnFoot.cpp:60).
