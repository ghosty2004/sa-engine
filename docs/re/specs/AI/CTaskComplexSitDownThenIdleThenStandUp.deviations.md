# CTaskComplexSitDownThenIdleThenStandUp: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x637DB0 CTaskComplexSitDownThenIdleThenStandUp::CreateNextSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH): EDI = 0 (0x637DCF); `t = m_pSubTask->GetTaskType()`;
`t - 0xCB` above 0x13 (unsigned) -> return 0 (0x637DDC -> 0x637F20); byte table 0x637F4C = {0, 4 x16, 1, 2, 3}, dword table
0x637F38 = {0x637DF0, 0x637ED9, 0x637F0E, 0x637F1E, 0x637F20}: t = 0xCB (203) -> 0x637DF0, 0xDC (220) -> 0x637ED9, 0xDD (221) ->
0x637F0E, 0xDE (222) -> 0x637F1E (`xor edi,edi`: returns 0), every other value in range -> 0x637F20 (returns 0).
Row TaskComplexSitDownThenIdleThenStandUp.cpp:94 (`NOTSA_UNREACHABLE();`):
- Original: any other subtask type returns nullptr. Verdict: CONFIRMED. Class: keep-original.
