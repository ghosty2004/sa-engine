# HUD static initialisers / atexit destructors (group H1)

CRT helpers (excluded CRT, call through `orig!` or reproduce with Rust statics): `_atexit` 0x821D1E (cdecl, 1 arg, caller pops),
`eh vector constructor iterator` 0x821D30 (`__stdcall (void* arr, u32 elemSize, u32 count, ctor, dtor)`, callee pops),
`eh vector destructor iterator` 0x821E02 (`__stdcall (void* arr, u32 elemSize, u32 count, dtor)`). Trivial element
ctor/dtor used for CRGBA arrays: 0x463360 (`mov eax, ecx; ret` = CRGBA default ctor, no-op returning this) and 0x43CE70
(`ret` = no-op dtor). All initialisers are `__cdecl void ()`, no callers (called from the CRT initialiser table).

## 0x84ED80 __init_g_ba672c  (catalogue: specified/REA, port level 0)
- `fld [0x8651A8] (0.005f)`; `fmul [0x858620] (0.0625f)`; `fstp dword [0xBA672C]` (0x84ED80-0x84ED8C). Stores the f32
  product 0.0003125 (= 1/3200, rounded to f32). No atexit.
- gta-reversed: MenuManager_Input.cpp:916 comments the global as "minimum mouse sensitivity = 1.0f/3200.0f" (no
  StaticRef name). Port: a static f32 initialised to `0.005f * 0.0625f` computed in f32 (exactly what the x87 stores).

## 0x84EE40 __init_FrontEndMenuManager  (catalogue: specified/REA, port level 7)
- `ecx = 0xBA6748 (FrontEndMenuManager)`; `call CMenuManager::Constructor 0x574350` (0x84EE45); then
  `_atexit(__dtor_FrontEndMenuManager 0x856560)` (0x84EE4A-0x84EE54, caller pops).

## 0x84F0A0 __init_CRadar__ArrowBlipColour  (catalogue: specified/REA, port level 0)
- `eh vector constructor iterator(0xBA86D4 CRadar::ArrowBlipColour, size 4, count 6, ctor 0x463360, dtor 0x43CE70)`
  (0x84F0A0-0x84F0B3): 6 CRGBA, no-op construction (memory stays zero from .bss). Then `_atexit(0x856570)`.

## 0x84F0D0 __init_CRadar__RadarBlipSprites  (catalogue: specified/REA, port level 0)
- `eh vector constructor iterator(0xBAA250 CRadar::RadarBlipSprites, size 4, count 0x40 = 64, CSprite2d::Constructor
  0x727230, CSprite2d::Destructor 0x7281E0)` (0x84F0D0-0x84F0E3); `_atexit(0x856590)`.

## 0x84F1C0 __init_CHud__Sprites  (catalogue: specified/REA, port level 0)
- `eh vector constructor iterator(0xBAB1FC CHud::Sprites, 4, 6, 0x727230, 0x7281E0)` (0x84F1C0-0x84F1D3); `_atexit(0x8565B0)`.

## 0x84F1F0 __init_HudColour  (catalogue: specified/REA, port level 1)
- `ecx = 0xBAB22C (HudColour)`; `call CHudColours::Constructor 0x58FDA0`; `_atexit(__dtor_HudColour 0x8565D0)`.

## 0x854DC0 __init_CFont__Sprite  (catalogue: specified/REA, port level 0)
- `eh vector constructor iterator(0xC71AD0 CFont::Sprite, 4, 2, 0x727230, 0x7281E0)`; `_atexit(0x856B40)`.

## 0x854DF0 __init_CFont__ButtonSprite  (catalogue: specified/REA, port level 0)
- `eh vector constructor iterator(0xC71AD8 CFont::ButtonSprite, 4, 0xF = 15, 0x727230, 0x7281E0)`; `_atexit(0x856B60)`.

## 0x856560 __dtor_FrontEndMenuManager  (catalogue: specified/REA, port level 16)
- `ecx = 0xBA6748; jmp CMenuManager::Destructor 0x579440` (tail call). Registered by 0x84EE40.

## 0x856570 __dtor_CRadar__ArrowBlipColour  (catalogue: specified/REA, port level 0)
- `eh vector destructor iterator(0xBA86D4, 4, 6, 0x43CE70 no-op)`; behaviourally a no-op.

## 0x856590 __dtor_CRadar__RadarBlipSprites  (catalogue: specified/REA, port level 0)
- `eh vector destructor iterator(0xBAA250, 4, 64, CSprite2d::Destructor 0x7281E0)` (destroys elements last to first, CRT).

## 0x8565B0 __dtor_CHud__Sprites  (catalogue: specified/REA, port level 0)
- `eh vector destructor iterator(0xBAB1FC, 4, 6, 0x7281E0)`.

## 0x8565D0 __dtor_HudColour  (catalogue: specified/REA, port level 0)
- `ecx = 0xBAB22C; jmp 0x58FD90`. 0x58FD90 (CHudColours destructor, not in this assignment) is
  `eh vector destructor iterator(ecx, 4, 0xF = 15, 0x43CE70)`, i.e. a no-op over the 15 CRGBA entries.

## 0x856B40 __dtor_CFont__Sprite  (catalogue: specified/REA, port level 0)
- `eh vector destructor iterator(0xC71AD0, 4, 2, 0x7281E0)`.

## 0x856B60 __dtor_CFont__ButtonSprite  (catalogue: specified/REA, port level 0)
- `eh vector destructor iterator(0xC71AD8, 4, 15, 0x7281E0)`.

gta-reversed: none of these exist as functions (C++ static construction of the `StaticRef` objects); open questions: none.
