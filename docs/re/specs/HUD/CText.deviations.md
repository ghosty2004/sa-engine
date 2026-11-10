# CText (HUD) - gta-reversed deviation notes (group H1)

Source: `re/gta-reversed/source/game_sa/Text/Text.cpp`. TheText 0xC1B340.

## 0x69FBD0 CText::GetNameOfLoadedMissionText  (gr_deviations = 1)
Original: `__thiscall void (char* out [esp+4])`, `ret 4`: unbounded `strcpy(out, this->m_szMissionName (+0x24))`
(0x69FBD4-0x69FBE8). Row Text.cpp:396 (`notsa::string_copy(outStr, m_szMissionName, std::size(m_szMissionName))`):
- Original bytes: byte copy including the terminator, no bound.
- Verdict: REFUTED (same result for the NUL-terminated 8-byte name). Class: harmless.

## 0x69FBF0 CText::LoadMissionText  (gr_deviations = 1)
Row Text.cpp:253 (`NOTSA_UNREACHABLE("Index of the mission {} is not defined.")`):
- Original bytes 0x69FC6E-0x69FCDB: loop `i < m_MissionTextOffsets.size (+0xA8C, u16)` comparing `strlen(entry.name) ==
  strlen(mission)` (u16) and `strncmp(entry.name, mission, len) == 0` (0x8214D0); if no entry matches (or the table is
  empty) it jumps to 0x69FF02 and returns normally (messages cleared, mission text unloaded, name zeroed).
- Verdict: CONFIRMED (gr aborts, original returns). Class: keep-original.
Found in passing: the GXT file name is built by `switch (m_nPrefsLanguage (byte 0xBA67CC))` 0..4 -> `sprintf(buf,
"AMERICAN.GXT" ... "SPANISH.GXT")` (table 0x69FF0C, strings 0x870FA0..0x870F70); for other values no sprintf happens and
the uninitialised stack buffer is passed to OpenFile (0x69FCF8 `ja 0x69FD3E`). Original bug, keep.

## 0x6A01A0 CText::Load  (gr_deviations = 2)
Row Text.cpp:166 (`assert(file)`): original reads from the OpenFile result without a check (0x6A0228-0x6A0237).
Verdict: REFUTED (debug assert only). Class: harmless.
Row Text.cpp:183 (todo: magic 0x58000 for CMissionTextOffsets::Load): original passes `push 0x58000` (0x6A02C0) to
CMissionTextOffsets::Load 0x69F670 (ECX = this+0x12C), then sets `m_bIsMissionTextOffsetsLoaded (+0x21) = 1`.
Verdict: REFUTED (gr uses the same value). Class: harmless.
