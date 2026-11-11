# CTaskComplexFallToDeath: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 groups J2-J4 (helper B).

## 0x679120 CTaskComplexFallToDeath::CreateFirstSubTask  (gr_deviations = 1)
Original bytes: after `ped->ApplyMoveForce(...)` 0x5429F0 (0x6791BA), `d = (u8)+0x20` (m_nFallToDeathDir, `movzx`); `if (d > 3) goto 0x6791F1` (unsigned `ja`,
0x6791C3-0x6791C6) else jump table 0x679260 = {0x6791CF, 0x6791D8, 0x6791E1, 0x6791EA} storing `+0x18 (m_nAnimId)` = 0x1A / 0x1B / 0x18 / 0x19 for d = 0 / 1 / 2 / 3.
Then +0x18 is re-read and range-tested `0 <= id < 4` (0x6791F1-0x6791FB) before use.
Row TaskComplexFallToDeath.cpp:102 (`default: NOTSA_UNREACHABLE("")` in the direction switch):
- Original bytes: d > 3 skips the store and leaves +0x18 unchanged (0x6791C6 -> 0x6791F1); no abort.
- Verdict: CONFIRMED (gr aborts; the original keeps the previous anim id). In practice d comes from CalcFall (0..3), so unreachable from the game's own
  callers. Class: keep-original.

## 0x6796C0 CTaskComplexFallToDeath::CalcFall  (gr_deviations = 1)
Original bytes: in the 2-iteration probe loop, `dir = rnd[i]` (local array at esp+0x58, after the 8 random swaps) is stored to `*outFallDir` (0x67974E)
and dispatched with `cmp 3; ja 0x679811` through table 0x679A8C = {0x67975D: matrix forward (+0x10), 0x679779: right * -1.0 [0x858C1C] per component,
0x6797BA: forward * -1.0 [0x858C1C], 0x6797FB: matrix right (+0x0)} into the local vector at esp+0x10. Then the probe scales it by 1.5 [0x8D2F00] (0x67981A)
and uses 0.375 [0x8D2F08] (0x679820).
Row TaskComplexFallToDeath.cpp:181 (`default: NOTSA_UNREACHABLE()`):
- Original bytes: dir > 3 jumps straight to 0x679811 and uses the local vector as it is (uninitialised on the first iteration). The array only ever holds
  0..3 (filled with 0,1,2,3 and permuted), so the default is unreachable.
- Verdict: CONFIRMED in form (abort vs. stale vector), unreachable in practice. Class: harmless. gr's case mapping (FORWARD=forward, LEFT=-right,
  BACKWARD=-forward, RIGHT=right) matches the table; gr negates while the bytes multiply by -1.0 (same result for all non-NaN values).
