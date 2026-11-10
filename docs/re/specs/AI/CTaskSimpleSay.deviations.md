# CTaskSimpleSay: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x48E360 CTaskSimpleSay::CTaskSimpleSay  (catalogue name: Constructor; gr_deviations = 1)
Original bytes (stub `jmp 0x15655E0`; `__thiscall this (int32 ctx [esp+4], int32 duration [esp+8])`, `ret 8`): CTaskSimple ctor 0x61A390
(`m_Parent = 0`, CTask vtbl); `m_GlobalSpeechContext (+8) = ctx` (full dword); `vtbl = 0x85A1DC`; `m_Duration (+0xC) = duration`;
timer `+0x10 = 0, +0x14 = 0, +0x18 (u8) = 0, +0x19 (u8) = 0`; return this (0x15655E3-0x156560A).
Row TaskSimpleSay.h:18 (NOTSA copy constructor `CTaskSimpleSay(const CTaskSimpleSay& o) : CTaskSimpleSay{o.m_GlobalSpeechContext,
o.m_Duration}`, used by gr's Clone):
- Original: the row is not part of 0x48E360; the ctor itself matches gr (fields and order above). gr's copy ctor delegates to this ctor,
  i.e. it resets the timer exactly as the original Clone would by constructing a new task from (ctx, duration).
- Verdict: REFUTED for this function (no behavioural difference in 0x48E360). Class: harmless.
- Side note: gr's hook wrapper `Constructor(eGlobalSpeechContextS16 sayId, uint32)` (TaskSimpleSay.h:32) declares a 16-bit first
  parameter while the original stores the whole dword at [esp+4]; callers push 32-bit values, so only the upper half of a garbage
  argument could differ. Harmless unless a caller passes a value >= 0x8000.
