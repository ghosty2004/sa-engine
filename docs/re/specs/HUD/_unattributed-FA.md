# _unattributed-FA (HUD) - RE-SPEC-FINAL group A

## 0x718660 GxtCharStrcpy  (catalogue: specified/REA, port level 0)
- `__cdecl GxtChar* (GxtChar* dst [esp+4], const GxtChar* src [esp+8])`, plain `ret`, EAX = dst. Copies bytes (GxtChar = char here) until
  the 0 terminator, then writes the terminator. Callers CStats::BuildStatLine 0x559230, ConstructStatLine 0x55A780 (5).
- gta-reversed: GxtCharStrcpy - same. Open questions: none.

## 0x718690 GxtCharStrlen  (catalogue: unknown/REA, port level 0)
- `__cdecl int32 (const GxtChar* s [esp+4])`, plain `ret`: number of bytes before the first 0. gta-reversed: same. Open questions: none.
