# CTaskSimplePlayHandSignalAnim: gta-reversed deviation notes (AI, group I2)

## 0x61AF60 CTaskSimplePlayHandSignalAnim::StartAnim  (gr_deviations = 1)
Row TaskSimplePlayHandSignalAnim.cpp:57 (rng-helper: `animId = GetRandomNumberInRange(ANIM_ID_GSIGN1, ANIM_ID_GSIGN5 + 1)`).
- Original bytes (0x61AF7B-0x61AFA9): `r = _rand() & 0xFFFF` 0x821B1E; `animId = 0x140 - _ftol(((float)r * 3.0517578e-05f [0x858B14]) *
  -5.0f [0x858FC8])` -> 320..324 (`_ftol` truncates toward zero, so the negative product gives 0..-4). It is drawn first, before the
  holding / pointing / weapon tests.
- Verdict: CONFIRMED (gr's GetRandomNumberInRange uses a different formula, cat14 finding 1; the result distribution differs).
  Class: keep-original.
