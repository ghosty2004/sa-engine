# CDecisionSimple (AI) - RE-SPEC-3 group I4

Layout (gr DecisionSimple.h, size 0x34): +0x00 m_anTasks[6] (i32), +0x18 m_afChances[6] (f32, cumulative probabilities in
[0,1]), +0x30 m_nCount (i32).

## 0x6007A0 CDecisionSimple::MakeDecision  (catalogue: specified/REA, port level 0)
- `__thiscall void (CDecisionSimple* this, int32 taskType [esp+4], int16* outTaskType [esp+8], int32* outIndex [esp+0xC])`, `ret 0xC`.
- Callers: CDecision::MakeDecision 0x6040D0 (only). Callees: CRT `rand` 0x821B1E (catalogue excluded; returns 0..0x7FFF).
- Behaviour:
  1. `*outTaskType = 200` (word 0xC8, 0x6007B2); `*outIndex = -1` (0x6007BA).
  2. If `taskType != -1` (0x6007A4/0x6007C1): for i = 0 .. m_nCount-1 (signed, skipped when m_nCount <= 0, 0x6007CA): if
     `m_anTasks[i] == taskType` (full dword compare, 0x6007D0) then `*outTaskType = (int16)m_anTasks[i]` (low word),
     `*outIndex = i`, stop scanning (0x6007DC-0x6007E3).
  3. If `*outTaskType == 200` (re-read as word, 0x6007E6) - i.e. no forced match, or the matched task itself was 200:
     `r = rand()` (0x6007ED); `f = (float)r` (`fild` of the int) `* 3.0518509e-05f [0x858C7C]` (= 1/32767), stored as f32
     (0x6007F9-0x600807). Then for i = 0 .. m_nCount-1 (0x60080B-0x600823): `if (f <= m_afChances[i])` - `fld f; fcomp
     chances[i]; test ah,0x41; jnp` (taken when exactly one of C0/C3 is set: f < c or f == c; NaN/unordered sets both -> not taken)
     -> `*outTaskType = (int16)m_anTasks[i]`, `*outIndex = i`, return (0x60082C-0x600833). If no i matches nothing more is written
     (outputs stay 200 / -1, or the step-2 values if step 2 matched a task of type 200).
- Edge cases: the comparison is against each m_afChances[i] directly (the array must be cumulative; CDecision builds it so).
  m_nCount is not clamped to 6.
- gta-reversed: declared in DecisionSimple.h only (no body or hook in DecisionSimple.cpp); port from these bytes.
  Open questions: none.
