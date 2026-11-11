# COLL: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

Generated from the bytes by the RE-SPEC-UNK lead (decoder checked by hand on samples). All `__init_*`
functions are entries of the CRT `__xc` initialiser table 0x8A4004 (called once, in table-slot order, by the
CRT before `WinMain`; `__cdecl void(void)`, plain `ret`, no arguments, no return value). `__dtor_*` functions
are registered with `_atexit` (0x821D1E) and run at process exit in reverse registration order. CRT helpers:
`eh vector constructor iterator` 0x821D30 (stdcall: arr, elemSize, count, ctor, dtor; ascending); `eh vector destructor iterator` 0x821E02 (stdcall: arr, elemSize, count, dtor; descending); 0x8214BD operator delete (cdecl).

## Header-constant initialisers (table)

About 225 translation units include a header that defines six float constants which the compiler could not
fold, so every such TU gets its own copies, computed by six `__xc` initialisers (one per constant, in this
order): `a * 0.0625` (a = 0.005), `1 / b` (b = 50.0), `100 / c` (c = 3.6), `1 / (b * b)`, `previous * 10`,
`0.277778 / b`. The operands a, b, c are TU-local .rdata copies (addresses differ per TU), 0.0625, 1.0,
100.0, 10.0 and 0.277778 are shared .rdata constants. A few other initialisers of the same shape subtract
two globals. Instruction shapes: `fld a; fmul b; fstp g` / `fld a; fdiv b; fstp g` / `fld a; fsub b; fstp g`
/ `fld a; fmul b; fdivr c; fstp g` (= c / (a * b)), then `ret`.

Port rules: compute in x87 order (left operand loaded first; `fdivr c` = c / ST0) and store as f32. The x87
precision control at CRT-init time is the process default (53-bit) because D3D has not yet lowered it to
24-bit; for every row below the f32 result was computed with both a 53-bit and a 24-bit model and they agree
unless the row says DIFFERS. Most target globals are never read by any code (the "readers" column comes from
a scan of every code byte for the absolute address outside the initialisers; "dead store" = no occurrence):
a port only needs them for memory-image parity, but must keep the initialisation order because some rows
read a global set by an earlier row (shown as `g[...] (= value, set by an earlier initialiser)`).
The subsystem of a row is the TU guess described in the classification evidence (low/medium confidence);
the behaviour does not depend on it.

| address | __xc slot | effect | f32 result | readers |
|---|---|---|---|---|
| 0x849210 | 0x8a4068 | g[0x965568] <- 0.005f [0x858b94] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849230 | 0x8a406c | g[0x96556c] <- 1.0f [0x858624] / 50.0f [0x858b9c] | 0.02 | none (dead store) |
| 0x849250 | 0x8a4070 | g[0x965570] <- 100.0f [0x858628] / 3.6f [0x858b98] | 27.777779 | none (dead store) |
| 0x849270 | 0x8a4074 | g[0x965574] <- 1.0f [0x858624] / (50.0f [0x858b9c] * 50.0f [0x858b9c]) | 0.0004 | none (dead store) |
| 0x849290 | 0x8a4078 | g[0x965578] <- g[0x965574] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8492b0 | 0x8a407c | g[0x96557c] <- 0.277778f [0x858630] / 50.0f [0x858b9c] | 0.00555556 | none (dead store) |
| 0x8492d0 | 0x8a4080 | g[0x9655e8] <- 0.005f [0x858c00] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8492f0 | 0x8a4084 | g[0x9655ec] <- 1.0f [0x858624] / 50.0f [0x858c08] | 0.02 | none (dead store) |
| 0x849310 | 0x8a4088 | g[0x9655f0] <- 100.0f [0x858628] / 3.6f [0x858c04] | 27.777779 | none (dead store) |
| 0x849330 | 0x8a408c | g[0x9655f4] <- 1.0f [0x858624] / (50.0f [0x858c08] * 50.0f [0x858c08]) | 0.0004 | none (dead store) |
| 0x849350 | 0x8a4090 | g[0x9655f8] <- g[0x9655f4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849370 | 0x8a4094 | g[0x9655fc] <- 0.277778f [0x858630] / 50.0f [0x858c08] | 0.00555556 | none (dead store) |
| 0x849450 | 0x8a40b0 | g[0x9689fc] <- 0.005f [0x858c38] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850120 | 0x8a4e3c | g[0xbc408c] <- 0.005f [0x868b0c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

## Other initialisers and destructors

## 0x849470 __init_CTempColModels__ms_colModelBBox  (catalogue: unknown/REA, port level 2)
- ECX = 0x968A00 (CTempColModels::ms_colModelBBox, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x856030). `__cdecl`, plain `ret`.

## 0x849490 __init_CTempColModels__ms_colModelCutObj  (catalogue: unknown/REA, port level 0)
- `eh vector constructor iterator` 0x821D30 (stdcall: arr, elemSize, count, ctor, dtor; ascending)(arr=0x968A30, elemSize=0x30, count=0x14 (20), ctor=0x40FB60 CColModel::CColModel, dtor=0x40F700 CColModel::~CColModel); then _atexit 0x821D1E(0x856040). gr: `std::array<CColModel, MAX_NUM_CUTSCENE_COLMODELS>` at 0x968A30; the array ends at 0x968DF0 = ms_colModelPed1.

## 0x8494c0 __init_CTempColModels__ms_colModelPed1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968DF0 (CTempColModels::ms_colModelPed1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x856060). `__cdecl`, plain `ret`.

## 0x8494e0 __init_CTempColModels__ms_colModelPed2  (catalogue: unknown/REA, port level 2)
- ECX = 0x968E20 (CTempColModels::ms_colModelPed2, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x856070). `__cdecl`, plain `ret`.

## 0x849500 __init_CTempColModels__ms_colModelDoor1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968E50 (CTempColModels::ms_colModelDoor1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x856080). `__cdecl`, plain `ret`.

## 0x849520 __init_CTempColModels__ms_colModelBumper1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968E80 (CTempColModels::ms_colModelBumper1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x856090). `__cdecl`, plain `ret`.

## 0x849540 __init_CTempColModels__ms_colModelPanel1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968EB0 (CTempColModels::ms_colModelPanel1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x8560A0). `__cdecl`, plain `ret`.

## 0x849560 __init_CTempColModels__ms_colModelBonnet1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968EE0 (CTempColModels::ms_colModelBonnet1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x8560B0). `__cdecl`, plain `ret`.

## 0x849580 __init_CTempColModels__ms_colModelBoot1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968F10 (CTempColModels::ms_colModelBoot1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x8560C0). `__cdecl`, plain `ret`.

## 0x8495a0 __init_CTempColModels__ms_colModelWheel1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968F40 (CTempColModels::ms_colModelWheel1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x8560D0). `__cdecl`, plain `ret`.

## 0x8495c0 __init_CTempColModels__ms_colModelBodyPart1  (catalogue: unknown/REA, port level 2)
- ECX = 0x968F70 (CTempColModels::ms_colModelBodyPart1, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x8560E0). `__cdecl`, plain `ret`.

## 0x8495e0 __init_CTempColModels__ms_colModelBodyPart2  (catalogue: unknown/REA, port level 2)
- ECX = 0x968FA0 (CTempColModels::ms_colModelBodyPart2, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x8560F0). `__cdecl`, plain `ret`.

## 0x849600 __init_CTempColModels__ms_colModelWeapon  (catalogue: unknown/REA, port level 2)
- ECX = 0x968FD0 (CTempColModels::ms_colModelWeapon, CColModel, gr StaticRef); call 0x40FB60 CColModel::CColModel; _atexit 0x821D1E(0x856100). `__cdecl`, plain `ret`.

## 0x852010 __init_g_c17854  (catalogue: unknown/REA, port level 1)
- ECX = 0xC17854; tail jump to 0x40F030 CCollisionData::CCollisionData (no atexit registration). The object sits next to `col1` (__init_col1 0x851FF0, AI TU) so it is probably the CCollisionData of that static col model (UNCLEAR).

## 0x855ff0 __dtor_local_static_CMatrix_965960  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0x965960, registered on first use by CCollision::TestLineOfSight (caller list: 0x417730). Body: ECX = 0x965960; tail jump to 0x59ACD0 CMatrix::~CMatrix.

## 0x856000 __dtor_local_static_CMatrix_9659b0  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0x9659B0, registered on first use by CCollision::ProcessLineOfSight (caller list: 0x417950). Body: ECX = 0x9659B0; tail jump to 0x59ACD0 CMatrix::~CMatrix.

## 0x856010 __dtor_local_static_CMatrix_968940  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0x968940, registered on first use by CCollision::ProcessColModels (caller list: 0x4185c0). Body: ECX = 0x968940; tail jump to 0x59ACD0 CMatrix::~CMatrix.

## 0x856020 __dtor_local_static_CMatrix_968988  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0x968988, registered on first use by CCollision::ProcessColModels (caller list: 0x4185c0). Body: ECX = 0x968988; tail jump to 0x59ACD0 CMatrix::~CMatrix.

## 0x856030 __dtor_CTempColModels__ms_colModelBBox  (catalogue: unknown/REA, port level 5)
- ECX = 0x968A00; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x856040 __dtor_CTempColModels__ms_colModelCutObj  (catalogue: unknown/REA, port level 0)
- `eh vector destructor iterator` 0x821E02 (stdcall: arr, elemSize, count, dtor; descending)(arr=0x968A30, elemSize=0x30, count=0x14, dtor=0x40F700 CColModel::~CColModel); `ret`.

## 0x856060 __dtor_CTempColModels__ms_colModelPed1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968DF0; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x856070 __dtor_CTempColModels__ms_colModelPed2  (catalogue: unknown/REA, port level 5)
- ECX = 0x968E20; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x856080 __dtor_CTempColModels__ms_colModelDoor1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968E50; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x856090 __dtor_CTempColModels__ms_colModelBumper1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968E80; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x8560a0 __dtor_CTempColModels__ms_colModelPanel1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968EB0; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x8560b0 __dtor_CTempColModels__ms_colModelBonnet1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968EE0; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x8560c0 __dtor_CTempColModels__ms_colModelBoot1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968F10; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x8560d0 __dtor_CTempColModels__ms_colModelWheel1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968F40; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x8560e0 __dtor_CTempColModels__ms_colModelBodyPart1  (catalogue: unknown/REA, port level 5)
- ECX = 0x968F70; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x8560f0 __dtor_CTempColModels__ms_colModelBodyPart2  (catalogue: unknown/REA, port level 5)
- ECX = 0x968FA0; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.

## 0x856100 __dtor_CTempColModels__ms_colModelWeapon  (catalogue: unknown/REA, port level 5)
- ECX = 0x968FD0; tail jump (`jmp`) to 0x40F700 CColModel::~CColModel. Runs at exit.
