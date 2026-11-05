# CWanted: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x561E70 CWanted::SetMaximumWantedLevel  (gr_deviations = 2)
Row Wanted.cpp:158 (assert `level <= WANTED_LEVEL_6`, deviates=no):
- Original bytes: no assert; `cmp eax, 6; ja 0x561F0E` (0x561E74-0x561E77, unsigned) returns without any store.
- Verdict: REFUTED (debug-only check). Class: harmless.
Row Wanted.cpp:169 (`default: NOTSA_UNREACHABLE()`):
- Original bytes: jump table 0x561F10 for 0..6. Each case stores both `MaximumWantedLevel` (0x8CDEE4) and `MaximumChaosLevel`
  (0x8CDEE8): 0 -> (0, 0) (0x561F02), 1 -> (1, 115), 2 -> (2, 365), 3 -> (3, 875), 4 -> (4, 1800), 5 -> (5, 3500),
  6 -> (6, 6900). For level > 6 or negative (unsigned `ja`) nothing is written (0x561F0E `ret`).
- Verdict: CONFIRMED - gr assigns `MaximumWantedLevel = level` before the switch (so an out-of-range level is stored) and then
  hits UB/abort; the original writes neither global.
- Class: keep-original (out-of-range: no change, no abort).

## 0x561FA0 CWanted::NumOfHelisRequired  (gr_deviations = 1)
Row Wanted.cpp:210 (`default: NOTSA_UNREACHABLE(); return 0;`).
- Original bytes: `if (byte[this+0x1E] & 7) return 0` (0x561FA0, PoliceBackOff = bits PoliceBackOff|PoliceBackOffGarage|
  EverybodyBackOff); `lvl = [this+0x2C]`; `lvl == 3 -> 1` (0x561FAC); `lvl < 3` (signed `jle`) -> 0; `lvl <= 6` -> 2;
  `lvl > 6` -> 0 (0x561FB6 `jg` to the return-0 path).
- Verdict: REFUTED for release behaviour of valid levels; the original defines out-of-range (> 6 or negative) as 0, which gr
  only reaches through the unreachable marker (release: `UNREACHABLE_INTRINSIC`, UB).
- Class: keep-original (return 0 for any level outside 3..6 except 3 -> 1).

## 0x562300 CWanted::RemovePursuitCop(CCopPed*, CCopPed**, uint8&)  (gr_deviations = 1)
Row Wanted.cpp:566 (original comment `// R*: delete?`).
- Original bytes (`__cdecl`, args cop [esp+4], array [esp+8], counter& [esp+0xC]): `for (i = 0; i < 10; i++) if (array[i] == cop)
  { array[i] = 0; (*counter)--  /* byte dec, wraps */; return; }` (0x562308-0x562329). No delete of the cop.
- Verdict: REFUTED (comment only; gr loop is identical).
- Class: harmless.

## 0x562470 CWanted::SetWantedLevel  (gr_deviations = 2)
Original bytes (`__thiscall void(int32 level [esp+4])`, `ret 4`): if `CCheat::m_aCheatsActive[65]` (byte 0x969171, I_DO_AS_I_PLEASE) return;
`lvl = level > MaximumWantedLevel (0x8CDEE4) ? Maximum : level` (signed `jle`); inlined ClearQdCrimes: 16 dwords at `this+0x34 + k*0x1C`
= 0 (0x56248D-0x56249F); jump table 0x562518 for 0..6: `m_nChaosLevel (+0x0) =` 0, 70, 200, 570, 1220, 2420, 4620; for `lvl > 6`
(unsigned, includes negative) the chaos level is left unchanged; in every case `UpdateWantedLevel()` (0x561C90) follows.
Row Wanted.cpp:415 (assert): Verdict REFUTED (debug only). Class: harmless.
Row Wanted.cpp:428 (`default: NOTSA_UNREACHABLE()`): Verdict CONFIRMED (out-of-range / negative level: original keeps chaos and still
calls UpdateWantedLevel; gr UB). Class: keep-original.
