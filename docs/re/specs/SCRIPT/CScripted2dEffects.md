# CScripted2dEffects (SCRIPT) — RE-SPEC-REST

## 0x6FA6F0 CScripted2dEffects::Init  (catalogue: specified/REA, port level 1)
- `__cdecl`, no args, `ret`. Caller: CTheScripts::Init 0x468D50. Callee: 0x6F9EB0 (thiscall, ECX = &ms_effectPairs[i], stride 0x94;
  catalogue unnamed — the effect-pair initialiser).
- Behaviour: for i = 0 .. 63 (0x40): `ms_activated[i] (0xC3A1A0, u8) = 0`; `ScriptReferenceIndex[i] (0xC3A120, u16) = 1`;
  `ms_effectSequenceTaskIDs[i] (0xC3A020, i32) = -1`; userLists entry (0xC3A200 + 0x24*i): 4 dwords at +0 = -1 and 4 dwords at +0x10 = -1,
  byte +0x20 = 0; `effectPairs[i].init()` 0x6F9EB0; `ms_useAgainFlags[i] (0xC39FE0, u8) = 0`; `ms_radii[i] (0xC39EE0, f32) = -1.0f`
  (immediate 0xBF800000).
