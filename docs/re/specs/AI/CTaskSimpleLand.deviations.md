# CTaskSimpleLand (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x67D380 CTaskSimpleLand::ProcessPed  (gr_deviations = 1, plus 1 bug-note row)
- TaskSimpleLand.cpp:68 `assert(m_pAnim); // NOTSA`: Original player block 0x67D4D7-0x67D526: `if (ped->IsPlayer())` 0x5DF8F0: `pad = GetPadFromPlayer(ped)` 0x609560
  is called FIRST, then `if (ped->m_nMoveState (+0x534) == 7 && CPad::GetSprint(pad) 0x5407A0) m_pAnim(+8)->m_fSpeed (+0x24) = 2.0f`; then
  `if (m_nAnimId (+0xC) != 10) m_pAnim->m_fSpeed = CStats::GetFatAndMuscleModifier(2) 0x559AF0 * oldSpeed` (old speed read before the call, x87 `fmul`).
  No null check of m_pAnim. Verdict: REFUTED (an assert only; GetPadFromPlayer evaluated before the move-state test is harmless). Class: harmless.
- TaskSimpleLand.cpp:70 `// possible bug, m_anim can be null here`: the original dereferences m_pAnim unconditionally (0x67D4FB, 0x67D50B) - gr keeps it.
  Verdict: REFUTED (gr matches the original, bug kept). Class: keep-original.
