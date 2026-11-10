# CTaskComplexCarDrive: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x642FA0 CTaskComplexCarDrive::CreateSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH): EAX is zeroed first (0x642FBC).
type == 0x2CD (717) -> `new(0xC)` CTaskComplex with vtbl 0x86EA14 (CTaskComplexEnterAnyCarAsDriver, 0x6430EB); type > 0x2CD:
0x340 (832) -> `new(0x34) CTaskSimpleCreateCarAndGetIn(&ped.pos (matrix +0x30 or placement +4), m_DesiredCarModel (+0x14))` 0x6493E0;
0x390 (912) -> `CTaskComplexWander::GetWanderTaskByPedType(ped)` 0x673D00; 0x516 (1302, TASK_FINISHED) -> 0; type < 0x2CD:
`type - 0x2BC` in 0..0xE through byte table 0x6431F0 / dword table 0x6431DC: 0x2BC (700) -> `new(0x50)
CTaskComplexEnterCarAsPassenger(m_Veh (+0xC), 0, 0)` 0x640340; 0x2BD -> `new(0x50) CTaskComplexEnterCarAsDriver(m_Veh)` 0x6402F0;
0x2C5 (709) -> `this->vtbl[0x2C/4]()` (SetUpCar) then `new(0x60) CTaskSimpleCarDrive(m_Veh, 0, 0)` 0x63C340; 0x2CA (714) ->
`new(0x14) CTaskComplexLeaveAnyCar(0, 1, 0)` 0x421150; every other value (table entry 4 = 0x643140, `ja` 0x642FDA, and the
fall-through at 0x64313C) -> return 0 (EAX still 0). Allocation failure -> 0.
Row TaskComplexCarDrive.cpp:187 (`default: NOTSA_UNREACHABLE();`):
- Original: unknown task types return nullptr (0x643140).
- Verdict: CONFIRMED (gr aborts; the original returns 0). Class: keep-original.

## 0x644E20 CTaskComplexCarDrive::CreateNextSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`): EDI = 0 (0x644E2A); `t = m_pSubTask->GetTaskType()` (vtbl+0x10,
no null check); `t - 0x2BC` above 0xDA (unsigned, 0x644E34-0x644E39) -> return EDI = 0 (0x644FFB); otherwise byte table 0x645024 /
dword table 0x645004; the case blocks match gr's cases (cpp:68-106); unlisted entries go to 0x644FFB (return 0).
Row TaskComplexCarDrive.cpp:108 (`default: NOTSA_UNREACHABLE();`):
- Original: returns nullptr for any other subtask type (0x644E39 -> 0x644FFB).
- Verdict: CONFIRMED. Class: keep-original.
