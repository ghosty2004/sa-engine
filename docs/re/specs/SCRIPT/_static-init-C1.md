# _static-init-C1 (SCRIPT) — RE-SPEC-REST

## 0x84A720 __init_CTheScripts__MissionCleanUp  (catalogue: specified/REA, port level 0)
- `__cdecl` static initialiser (CRT init table), `ret`. Inlined CMissionCleanup ctor for CTheScripts::MissionCleanUp (0xA90850): for the
  75 (0x4B) entries: `type (+0) = 0; handle (+4) = 0` (EAX from 0xA90854, step 8). m_Count (+0x258) is NOT written (stays .bss 0).
  No atexit registration.

## 0x84A740 __init_CTheScripts__ScriptsForBrains  (catalogue: specified/REA, port level 2)
- Static initialiser: `ECX = &CTheScripts::ScriptsForBrains (0xA90CF0)`; tail-jumps to the CScriptsForBrains ctor 0x46B260 (which calls
  Init 0x46A8C0). No atexit registration.

## 0x84A750 __init_CTheScripts__IntroTextLines  (catalogue: specified/REA, port level 0)
- Static initialiser: `eh vector constructor iterator` 0x821D30 (stdcall: arr, elemSize, count, ctor, dtor) on CTheScripts::IntroTextLines
  0xA913E8, elemSize 0x44, count 0x60, ctor 0x463750, dtor 0x463760; then `atexit(__dtor_CTheScripts__IntroTextLines 0x856190)` 0x821D1E
  (cdecl, `pop ecx`).

## 0x84A780 __init_CTheScripts__IntroRectangles  (catalogue: specified/REA, port level 0)
- Static initialiser: 0x821D30 on CTheScripts::IntroRectangles 0xA92D68, elemSize 0x3C, count 0x80, ctor 0x463770, dtor 0x463780; then
  `atexit(0x8561B0)`.

## 0x84A7B0 __init_CTheScripts__ScriptSprites  (catalogue: specified/REA, port level 0)
- Static initialiser: 0x821D30 on CTheScripts::ScriptSprites 0xA94B68, elemSize 4, count 0x80, ctor CSprite2d::CSprite2d 0x727230,
  dtor CSprite2d::~CSprite2d 0x7281E0; then `atexit(0x856170)`.

## 0x84A860 __init_g_a95158  (catalogue: specified/REA, port level 0)
- Static initialiser: `[0xA95158] (f32) = [0xA95154] (f32) * [0x85862C]=10.0f` (fld/fmul/fstp). 0xA95154 is written by the earlier
  initialiser 0x84A840, so the CRT init order (0x84A840 before 0x84A860) matters. Both globals sit after CTheScripts::ScriptSearchLightArray
  (+0x3EC / +0x3F0 per rwsets); 0xA95158 is also referenced by CTheScripts::Init 0x468D50. Meaning UNCLEAR (a search-light constant).

## 0x8545B0 __init_CScripted2dEffects__ms_userLists  (catalogue: specified/REA, port level 0)
- Static initialiser: 0x821D30 on CScripted2dEffects::ms_userLists 0xC3A200, elemSize 0x24, count 0x40, ctor 0x6FA6D0, dtor 0x6F9E00; then
  `atexit(0x856A90)`.

## 0x8545E0 __init_CScripted2dEffects__ms_effectPairs  (catalogue: specified/REA, port level 0)
- Static initialiser: 0x821D30 on CScripted2dEffects::ms_effectPairs 0xC3BB00, elemSize 0x94, count 0x40, ctor 0x6FA790, dtor 0x6F9E60;
  then `atexit(0x856AB0)`.

## 0x856170 __dtor_CTheScripts__ScriptSprites  (catalogue: specified/REA, port level 0)
- atexit destructor: `eh vector destructor iterator` 0x821E02 (stdcall: arr, elemSize, count, dtor; descending) on 0xA94B68, 4, 0x80,
  CSprite2d::~CSprite2d 0x7281E0.

## 0x856190 __dtor_CTheScripts__IntroTextLines  (catalogue: specified/REA, port level 0)
- atexit destructor: 0x821E02 on 0xA913E8, 0x44, 0x60, dtor 0x463760.

## 0x8561B0 __dtor_CTheScripts__IntroRectangles  (catalogue: specified/REA, port level 0)
- atexit destructor: 0x821E02 on 0xA92D68, 0x3C, 0x80, dtor 0x463780.

## 0x856A90 __dtor_CScripted2dEffects__ms_userLists  (catalogue: specified/REA, port level 0)
- atexit destructor: 0x821E02 on 0xC3A200, 0x24, 0x40, dtor 0x6F9E00.

## 0x856AB0 __dtor_CScripted2dEffects__ms_effectPairs  (catalogue: specified/REA, port level 0)
- atexit destructor: 0x821E02 on 0xC3BB00, 0x94, 0x40, dtor 0x6F9E60.
