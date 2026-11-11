# MATH static initialisers — RE-SPEC-REST

## 0x84F8C0 __init_g_bb3dfc  (catalogue: specified/REA, port level 0)
- `__cdecl`, no args, plain `ret`; a CRT static-initialiser entry (no callers; reached through the initialiser table).
- Behaviour: `[0xBB3DFC] (f32 global, .bss) = [0x858630]=0.277778f / [0x866D8C]=50.0f` — `fld; fdiv; fstp` (0x84F8C0-0x84F8CC),
  rounded to f32 (= 0.0055555...). 0.277778 is the km/h → m/s factor (1/3.6) as an f32; the global is that factor per 50 fps
  frame. No direct reader found in catalog/rwsets.csv (only this writer and an indexed write from 0x59AC90, the maths-table initialiser; gr Maths.cpp:6 notes the sin table starts at 0xBB3E00, right after this dword). Purpose UNCLEAR; keep the value bit-exact.
- No atexit registration.
