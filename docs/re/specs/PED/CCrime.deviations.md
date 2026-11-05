# CCrime: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x532010 CCrime::ReportCrime  (gr_deviations = 3)
Original bytes (`__cdecl void(eCrimeType crime [esp+4], CEntity* victim [esp+8], CPed* by [esp+0xC])`):
  1. if `by == null` or `by->m_nPedType (+0x598)` not 0/1 return (0x532018-0x532034).
  2. `isPedCriminal = victim && victim is a ped && CPedType::PoliceDontCareAboutCrimesAgainstPedType(victim->m_nPedType)` (0x53203A-0x53205A).
  3. if `crime == 2` (DAMAGED_PED): **no null check on victim** (`mov cl, [edi+0x36]` at 0x53206B dereferences it); if victim is a ped &&
     `IsPedPointerValid(victim)` && `by->GetWantedLevel() == 0` (0x41BE60, on `by`) && `victim->flags1 (+0x46C) & 0x800000`
     (bBeingChasedByPolice): if victim state (+0x530) is 54 or 55 return; else `CMessages::AddBigMessage(TheText.Get("GOODBOY"), 5000, 0)`
     (no null check) and **`CWorld::Players[PlayerInFocus].m_nMoney (+0xB8) += 50`** (no ped-type check); return.
     Otherwise (or for other crimes) continue at 4; `crime == 0` returns here (0x5320FA).
  4. `pos = by` position (copy). If `FindPlayerPed(-1)->m_pPlayerData->m_pWanted->m_fMultiplier (+0x14) >= 0` (`test ah,1`: < 0 and NaN skip):
     `if (CWanted::WorkOutPolicePresence(pos, CCrime::FindImmediateDetectionRange(crime)) != 0 || (crime in {4,5,13,14} && CLocalisation::GermanGame()))`
     -> `FindPlayerPed(-1)`'s wanted: `RegisterCrime_Immediately(crime, &pos, victim, isPedCriminal)`, then `CWanted::SetWantedLevelNoDrop(1)` (0x562570);
     else `RegisterCrime(crime, &pos, victim, isPedCriminal)` (0x562410). No victim check in this step.
  5. `crime == 3` -> `FindPlayerPed(-1)->CPlayerPed::SetWantedLevelNoDrop(1)` (0x609F30); `crime == 5 || crime == 19` -> `(2)`.
Row Crime.cpp:46 ("Vanilla bug here" on `bBeingChasedByPolice`): Verdict REFUTED (gr tests the same bit 0x800000). Class: harmless.
Row Crime.cpp:67 (`notsa::contains` German-crime set, deviates=no): Verdict REFUTED for the set {4,5,13,14}; note the original evaluates
  WorkOutPolicePresence FIRST and only then the German test, gr short-circuits in the opposite order (equivalent only if
  WorkOutPolicePresence has no side effects - not verified here). Class: harmless.
Row Crime.cpp:70 (SetWantedLevelNoDrop(1) "never know if this is a bug"): Verdict REFUTED (the original does the same call). Class: keep-original.
Additional gr differences (not catalogued), keep-original: (a) gr checks `pVictim` before the DAMAGED_PED branch and in step 4 (original:
none / null deref in step 3); (b) gr pays the $50 only for PED_TYPE_PLAYER1 into the committer's PlayerInfo and null-checks the text;
the original always pays Players[PlayerInFocus]; (c) gr uses the committer's CWanted everywhere, the original always uses
FindPlayerPed(-1)'s (player in focus), including the final step where it calls CPlayerPed::SetWantedLevelNoDrop rather than CWanted's.
