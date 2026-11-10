# CTaskComplexRoadRage: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x629080 CTaskComplexRoadRage::CreateSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`): compare tree on taskType
(0x398, 0x2C0, 0x12E, 0x2BD, 0x38B, 0x3E8, ... at 0x6290A7-0x6292A7); every unmatched value reaches 0x6292AC `xor eax,eax` and returns 0;
allocation failures jump to the same block.
Row TaskComplexRoadRage.cpp:75 (`NOTSA_UNREACHABLE();`):
- Original: returns nullptr (0x6292AC). Verdict: CONFIRMED. Class: keep-original.

## 0x62E270 CTaskComplexRoadRage::CreateNextSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`): returns 0 at once when the value tested at 0x62E279 is 0; otherwise
`t = m_pSubTask->GetTaskType()` and a compare tree with cases 907 (0x38B), 302 (0x12E), 701 (0x2BD), 704 (0x2C0), 920 (0x398),
1000 (0x3E8), 1003 (0x3EB) (0x62E28C-0x62E3A3); unmatched values reach 0x62E3E8 with EDI = 0 and return 0.
Row TaskComplexRoadRage.cpp:119 (`NOTSA_UNREACHABLE_CASE(tt);`):
- Original: returns nullptr (`jne 0x62E3E8` at 0x62E2AE / 0x62E3A3). Verdict: CONFIRMED. Class: keep-original.
