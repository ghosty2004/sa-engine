# CTaskComplexSequence: gta-reversed deviation notes (AI, group I2)
Layout: +0xC m_CurrentTaskIndex i32, +0x10 m_Tasks[8] (CTask*), +0x30 m_RepeatMode i32, +0x34 m_RepeatedTimes i32.

## 0x638A40 CTaskComplexSequence::CreateNextSubTask (virtual)  (gr_deviations = 3)
- Original bytes: `return CreateNextSubTask(ped, &m_CurrentTaskIndex (+0xC), &m_RepeatedTimes (+0x34))` 0x632C70, `ret 4`.
  0x632C70 (`__thiscall CTask* (this, CPed*, int32* idx, int32* count)`, `ret 0xC`): `++*idx`; if `m_RepeatMode == 0`: return
  `(*idx != 8 && m_Tasks[*idx]) ? m_Tasks[*idx]->Clone() : 0` (vtbl+4). Else: if `*idx == 8 || !m_Tasks[*idx]`: `*idx = 0; ++*count`;
  then if `m_RepeatMode != 1 && *count == m_RepeatMode` return 0; return `m_Tasks[*idx]->Clone()` (0x632C9D-0x632CD2).
- Rows cpp:115/116 (notes about the useless `if`): Verdict CONFIRMED - the test is not useless: any m_RepeatMode > 1 means "repeat the
  sequence that many times" (the count is compared after each wrap). Class: keep-original.
- Row cpp:130 (NOTSA_UNREACHABLE default for modes other than 0/1): Verdict CONFIRMED (the original handles them as a repeat count).
  Class: keep-original (script SET_SEQUENCE_TO_REPEAT with a count must work).
