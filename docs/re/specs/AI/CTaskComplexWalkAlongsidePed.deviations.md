# CTaskComplexWalkAlongsidePed: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x685000 CTaskComplexWalkAlongsidePed::CreateSubTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH): EAX = 0 (0x68501C); cases
0x2C0 (704, 0x68501E), 0x38B (907, 0x685072), 0xCB (203, 0x685107); any other value -> `jne 0x685150` and returns EAX = 0; allocation
failures -> 0x68514E (`xor eax,eax`).
Row TaskComplexWalkAlongsidePed.cpp:64 (`default: NOTSA_UNREACHABLE("Invalid TaskType {}", tt);`):
- Original: returns nullptr. Verdict: CONFIRMED. Class: keep-original.

## 0x685170 CTaskComplexWalkAlongsidePed::ControlSubTask  (gr_deviations = 3; 2 notsa-helper rows deviates=no)
Rows cpp:105 (`notsa::contains({PEDMOVE_WALK, PEDMOVE_RUN, PEDMOVE_SPRINT}, ...)`) and cpp:114 (`notsa::dyn_cast_if_present<...>`):
helper syntax only. Verdict: REFUTED. Class: harmless.
Row cpp:148 (`NOTSA_UNREACHABLE("Never say never"); // 0x6854BF`), inside `if (d2 >= sq(0.25f))`:
- Original bytes: `d2` ([esp+0x18]) is compared with 0.0625f [0x858620] twice. First (0x685400-0x68540F, `test ah,5; jp`): `d2 < 0.0625`
  -> decrease own walk anim / increase target's (0x685411-0x685459: `max(0.85f, s - 0.0125f [0x85A5C4])` via 0x420800 and
  `min(1.25f, s + 0.0125f)` via 0x404330). Second (0x68545E-0x68546D, `test ah,0x41; jne`): `d2 > 0.0625` -> increase own / decrease
  target's (0x68546F-0x6854B7). Remaining cases - `d2 == 0.0625f` exactly OR d2 is NaN - reach 0x6854BC:
  `old = pedWalkAnim->m_fSpeed (+0x24)`; `ped->SetMoveAnimSpeed(pedWalkAnim)` 0x5DEC10; `new = anim speed`; if `approxEqual(old, new,
  0.013f (imm 0x3C54FDF4))` 0x4EEA80 the speed is set back to `old`; else `old <= new` (`fcomp new; test ah,0x41; jne`; NaN also takes this
  branch) ? `old + 0.0125f` : `old - 0.0125f` (0x6854E4-0x68550C). Then the same for the target ped's walk anim ([esp+0x1c]) with
  `m_TargetPed (+0xC)->SetMoveAnimSpeed` if that anim exists (0x68550F-0x68556D); "anim speed changed" flag = 1 (0x685570).
- Verdict: CONFIRMED - gr aborts; the original nudges both walk anims towards the SetMoveAnimSpeed value. Reachable for NaN distances
  and exact equality. Class: keep-original.
