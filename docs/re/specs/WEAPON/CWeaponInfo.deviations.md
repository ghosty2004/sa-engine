# CWeaponInfo (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/WeaponInfo.cpp` (pin 20d8d2a).

## 0x743CD0 CWeaponInfo::GetSkillStatIndex  (gr_deviations = 1)
Row WeaponInfo.cpp:364 (notsa-unreachable `default: NOTSA_UNREACHABLE(...)`).
- Original bytes: `__cdecl int32 (eWeaponType wt [esp+4])`, plain `ret`. `if (wt < 22 || wt > 32) return -1`
  (signed compares 0x743CD4/0x743CD9, `or eax,-1` 0x743D0C). `if (wt <= 31) return wt - 22 + 0x45` (0x743CFD-0x743D03,
  i.e. 69..78 = STAT_PISTOL_SKILL..). `wt == 32` (TEC9) -> `6 + 0x45 = 75` (0x743CF4, STAT_MACHINE_PISTOL_SKILL, same
  as MICRO_UZI = 28 -> 75). The `wt == 33` arm (0x743CEB, returns 0x51) and the 0x743D04 arm (returns wt + 0x45) are
  unreachable because wt > 32 was already rejected.
- gr: same values for 22..32; for any other weapon type gr hits NOTSA_UNREACHABLE (debug break / UB) where the
  original returns -1. Callers (CStats::UpdateStatsWhenWeaponHit 0x55CEB0, CHud::DrawVitalStats 0x589650) are expected
  to pass only skill weapons, but the original's -1 is defined behaviour.
- Verdict: CONFIRMED (out-of-range input: original returns -1, gr is unreachable/UB).
- Class: keep-original (return -1 for wt outside 22..32).
