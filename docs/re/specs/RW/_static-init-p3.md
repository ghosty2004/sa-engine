# RW-subsystem static initialisers / destructors (RE-SPEC-RW-3 writer B)

Globals: `[0xbc12c0]` playerImg (CDirectory object, gta-reversed name), `[0xc02c10]` f32 global written once at start-up (no name;
it sits right after `CCarFXRenderer::ms_aDirtTextures` 0xc02bd0..0xc02c0f; UNCLEAR owner).

## 0x84fe40 __init_playerImg  (catalogue: specified/REA, port level 1)
- `void (void)`: `CDirectory::CDirectory()` 0x532290 with this (ECX) = 0xbc12c0 (empty ctor); `_atexit(__dtor_playerImg 0x856680)`
  0x821d1e. No return value.
- librw: absent (game code).

## 0x8507a0 __init_g_c02c10  (catalogue: specified/REA, port level 0)
- `void (void)`: x87 `v = 0.005f [0x86bb24] * 0.0625f [0x858620]`, stored as f32 into `[0xc02c10]` (fld f32, fmul f32, fstp f32; 0x8507a0-0x8507ac). Result
  rounds to the f32 nearest 0.0003125. No atexit.
- librw: absent.

## 0x856680 __dtor_playerImg  (catalogue: specified/REA, port level 2)
- `void (void)`: tail-jump to `CDirectory::~CDirectory` 0x5322d0 with ECX = 0xbc12c0 (0x856685).
- librw: absent.
