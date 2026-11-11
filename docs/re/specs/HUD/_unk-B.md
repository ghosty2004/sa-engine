# HUD: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x69F7E0 GxtCharToAscii (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr GxtChar.cpp:13 (`NOTSA -- Derived from GxtCharToAscii (0x69F7E0)`); callers CStats::BuildStatLine (5), CStats::ConstructStatLine (2), CMenuManager::SaveStatsToFile (9), CText::Load.
- Convention: `__cdecl char* (const GxtChar* src [esp+4], u8 start [esp+8])`, returns the static buffer 0xC1BDD0 in EAX. Part of the loop test is relocated to .HOODLUM stub 0x4012BB (`cmp ecx, 0xFF; jmp 0x69F806`, plain code).
- Behaviour: `p = src; if (start != 0) p = src + start;` `i = 0; if (p) { while (i < 255 /*signed*/ && *p != 0) { c = *p;` map: `c < 0x80 -> c`; `0x80..0x83 -> c+0x40`; `0x84..0x8D -> c+0x42`; `0x8E..0x91 -> c+0x44`; `0x92..0x95 -> c+0x47`; `0x96..0x9A -> c+0x49`; `0x9B..0xA4 -> c+0x4B`; `0xA5..0xA8 -> c+0x4D`; `0xA9..0xCC -> c+0x50`; `0xCD -> 0xD1`; `0xCE -> 0xF1`; `0xCF -> 0xBF`; `0xD0..0xFF -> 0x23 ('#')` (u8 arithmetic, wraps); `buf[i] = mapped; p++; i++; } }` (the loop also tests `p != 0` after increment, never false in practice); `buf[i] = 0; return buf;`
- Edge: output truncated at 255 characters; the buffer is shared between calls. GxtChar is 8-bit here (byte loads). Open questions: none.
