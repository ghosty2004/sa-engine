# WORLD static initialisers and atexit destructors

These run from the CRT's static-initialiser table before `WinMain` (`__init_*`) and from the CRT exit
path (`__dtor_*`, registered with `_atexit`). In phase A they have already run by the time any hook is
installed; a port matters only for phase B/C start-up and shutdown. None has callers in game code: each
`__init_*` is reached from the CRT initialiser table, each `__dtor_*` only through its `_atexit`
registration (catalogue "callers" = the `fnref` from the matching `__init_*`).

Common CRT helpers (catalogue state `excluded:crt`, call them through `orig!` in phase A, Rust
equivalents in phase B):

- `0x821d1e _atexit` — `__cdecl int atexit(void (*fn)(void))`; every caller does `push fn; call; pop ecx`
  and ignores the result.
- `0x821d30 eh vector constructor iterator` — `__stdcall (void* base, u32 size, i32 count,
  ctor (__thiscall), dtor (__thiscall))`, callee pops 20 bytes. Calls `ctor(base + i*size)` for
  `i = 0 .. count-1` in ascending order; on an exception during construction it destroys the already
  built elements with `dtor` (irrelevant for a port: no ctor below can throw).
- `0x821e02 eh vector destructor iterator` — `__stdcall (void* base, u32 size, i32 count, dtor)`, callee
  pops 16 bytes. Calls `dtor(base + i*size)` for `i = count-1 down to 0` (reverse order).

Unless stated otherwise every function below is `__cdecl void (void)`, plain `ret`, no locals.

## 0x849b20 __init_g_96a718  (catalogue: specified/REA, port level 0)
- `[0x96a718] (f32) = [0x96a714] (f32) * [0x85862c] = 10.0f` — `fld; fmul; fstp`, x87 with 24-bit PC.
- `0x96a714`/`0x96a718` are unnamed `.data` floats just after `CDarkel::Status` (no gr name). The
  initial `.data` value of `0x96a714` decides the result (read it from the image; it is 0.0 in the
  memory-mapped image => result 0.0f unless another earlier initialiser writes `0x96a714`).
- Open question: which object owns 0x96a714/0x96a718 (no gr `StaticRef`); does not affect behaviour.

## 0x849b40 __init_g_96a71c  (catalogue: specified/REA, port level 0)
- `[0x96a71c] (f32) = [0x858630] (f32 0.277778 = 0x3e8e38eb) / [0x8594fc] (f32 50.0)` — `fld; fdiv; fstp`.
  Compute as f32 division (24-bit PC; both operands f32, result rounded once to f32).

## 0x849ce0 __init_CGameLogic__SavedWeapons  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CGameLogic::SavedWeapons = 0x96a9b8, size 0x1c, count 13, ctor 0x441e00, dtor 0x4411a0)`,
  then `_atexit(0x856110)`.
- `0x441e00` (CWeapon default ctor, `__thiscall`, returns this in EAX): `byte[+0x14] = 0; byte[+0x15] = 0;
  dword[+0x18] = 0`. Nothing else is written (type/state/ammo fields keep their `.bss` zero).
- `0x4411a0` (CWeapon dtor): a bare `ret`.

## 0x849e90 __init_CGarages__aGarages  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CGarages::aGarages = 0x96c048, size 0xd8, count 50, CGarage::CGarage 0x4470e0,
  CGarage::~CGarage 0x447110)`, then `_atexit(0x856130)`.

## 0x84a260 __init_g_97d64c  (catalogue: specified/REA, port level 0)
- `[0x97d64c] (f32) = [0x858624] (1.0f) / [0x859b00] (50.0f)` (= 0.02f), `fld; fdiv; fstp`. Global is
  unnamed (near `CollectPickupBuffer+0x8`).

## 0x84a280 __init_g_97d650  (catalogue: specified/REA, port level 0)
- `[0x97d650] (f32) = [0x858628] (100.0f) / [0x859afc] (3.6f = 0x40666666)`, `fld; fdiv; fstp`.
  Global is unnamed (near `CollectPickupBuffer+0xc`).

## 0x84a540 __init_CReplay__PlayerInfo  (catalogue: specified/REA, port level 4)
- `CPlayerInfo::Constructor(this = &CReplay::PlayerInfo = 0xa430b0)` (`0x571920`, `__thiscall`), then
  `_atexit(0x856160)`.

## 0x84b1c0 __init_g_a95834  (catalogue: specified/REA, port level 0)
- Inlined `CSetPiece::CSetPiece()` over `CSetPieces::aSetPieces` (0xa95818, 210 = 0xd2 entries, stride
  0x20). Loop: `p = 0xa95818; n = 210; do { byte[p+0x1c] = 0 (m_nType = SETPIECE_NONE);
  dword[p+0] = 0 (m_nLastGenerationTime); p += 0x20; } while (--n != 0)`. Write order inside an
  element: +0x1c first, then +0. No other field is touched (rects/coords keep `.bss` zero).
- No `_atexit` (trivial destructor). The catalogue name refers to 0xa95834 = element 0 + 0x1c.

## 0x84b2a0 __init_gStoredClothesState  (catalogue: specified/REA, port level 1)
- Tail call: `ecx = &gStoredClothesState (0xa9a810); jmp CPedClothesDesc::Constructor (0x5a8020)`.
  No `_atexit` (trivial destructor). EAX = whatever the ctor returns (this).

## 0x84e300 __init_gMatrixList  (catalogue: specified/REA, port level 1)
- `0x54f440(this = &gMatrixList = 0xb74288)` then `_atexit(0x856490)`.
- `0x54f440` (CMatrixLinkList ctor, `__thiscall`, returns this): writes 0 to dwords at
  +0x40, +0x44, +0x94, +0x98, +0xe8, +0xec, +0x13c, +0x140, +0x190, +0x194, +0x1e4, +0x1e8, +0x1f8 in this
  order. These are the `m_pAttachMatrix` / `m_bOwnsAttachedMatrix` pairs (+0x40/+0x44 of a CMatrix) of the
  six CMatrixLink members at +0x0, +0x54, +0xa8, +0xfc, +0x150, +0x1a4 (inlined CMatrix default ctor), and
  `+0x1f8` (the links-array pointer).

## 0x84e980 __init_CWorld__Players  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CWorld::Players = 0xb7cd98, size 0x190, count 2, CPlayerInfo::Constructor 0x571920,
  CPlayerInfo::~CPlayerInfo 0x45b110)`, then `_atexit(0x856500)`.

## 0x84e9b0 __init_CWorld__ms_aSectors  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CWorld::ms_aSectors = 0xb7d0b8, size 8, count 0x3840 = 14400, ctor 0x564040,
  dtor 0x563130)`, then `_atexit(0x856520)`.
- `0x564040` (CSector ctor): `dword[+0] = 0; dword[+4] = 0` (both list heads), returns this.
- `0x563130` (CSector dtor): `CPtrListDoubleLink::Flush(this+4)` (0x552470), then
  `CPtrListSingleLink::Flush(this+0)` (0x552400) (SEH frame only for unwinding).

## 0x84e9e0 __init_CWorld__ms_aRepeatSectors  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CWorld::ms_aRepeatSectors = 0xb992b8, size 0xc, count 256, ctor 0x563190, dtor 0x5631b0)`,
  then `_atexit(0x856540)`.
- `0x563190` (CRepeatSector ctor): `eh_vector_ctor(this, 4, 3, 0x563180, 0x5630f0)`, returns this.
  `0x563180` = `dword[this] = 0` (CPtrListDoubleLink ctor). `0x5630f0` = `jmp CPtrListDoubleLink::Flush`.
- `0x5631b0` (CRepeatSector dtor): `eh_vector_dtor(this, 4, 3, 0x5630f0)` (flushes lists 2, 1, 0).

## 0x84ea10 __init_CWorld__ms_aLodPtrLists  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CWorld::ms_aLodPtrLists = 0xb99eb8, size 4, count 0x384 = 900, ctor 0x43e3f0,
  dtor 0x43e1a0)`, then `_atexit(0x8564c0)`.
- `0x43e3f0`: `dword[this] = 0`, returns this. `0x43e1a0`: `jmp CPtrListSingleLink::Flush (0x552400)`.

## 0x84ea40 __init_CWorld__ms_listMovingEntityPtrs  (catalogue: specified/REA, port level 0)
- Only `_atexit(0x8564e0)`. The list head itself is not written (the compiler folded the zeroing ctor
  into `.bss` zero-init).

## 0x84ea50 __init_CWorld__ms_listObjectsWithControlCode  (catalogue: specified/REA, port level 0)
- Only `_atexit(0x8564f0)` (same pattern as above).

## 0x84f300 __init_g_furnitureMan  (catalogue: specified/REA, port level 2)
- `FurnitureManager_c::FurnitureManager_c(this = &g_furnitureMan = 0xbab380)` (0x591260), then
  `_atexit(0x856640)`.

## 0x84f320 __init_FurnitureManager_c__g_subGroupStore  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&FurnitureManager_c::g_subGroupStore = 0xbad3f8, size 0x1c, count 0x80 = 128,
  FurnitureSubGroup_c::FurnitureSubGroup_c 0x590e20, FurnitureSubGroup_c::~FurnitureSubGroup_c 0x590e70)`,
  then `_atexit(0x856600)`.

## 0x84f350 __init_g_bae1f8  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(0xbae1f8, size 0x14, count 0x100 = 256, Furniture_c::Furniture_c 0x590ec0,
  Furniture_c::~Furniture_c 0x590ed0)`, then `_atexit(0x856620)`.
- 0xbae1f8 has no gr name; it is an array of 256 `Furniture_c` (0x14 bytes each), most likely the
  furniture item store of FurnitureManager_c (proposed: `FurnitureManager_c::g_furnitureStore`; name not
  load-bearing).

## 0x84f740 __init_g_interiorMan  (catalogue: specified/REA, port level 2)
- `InteriorManager_c::InteriorManager_c(this = &g_interiorMan = 0xbaf670)` (0x5984c0), then
  `_atexit(0x856650)`.

## 0x84fa80 __init_g_breakMan  (catalogue: specified/REA, port level 1)
- `0x59e620(this = &g_breakMan = 0xbb4240)` (BreakManager_c ctor), then `_atexit(0x856660)`.
- `0x59e620`: `eh_vector_ctor(this, size 0x20, count 0x40 = 64, BreakObject_c::BreakObject_c 0x59d170,
  BreakObject_c dtor 0x59d180)`, returns this. `0x59d180` is a bare `ret`. See
  `docs/re/specs/WORLD/BreakObject_c.md` for 0x59d170.

## 0x853f10 __init_g_waterCreatureMan  (catalogue: specified/REA, port level 2)
- `0x6e3eb0(this = &g_waterCreatureMan = 0xc1df30)` (WaterCreatureManager_c ctor), then `_atexit(0x856a80)`.
- `0x6e3eb0`: `eh_vector_ctor(this, size 0x34, count 0x80 = 128, ctor 0x6e3e30, dtor 0x6e3e50)`
  (WaterCreature_c array), then `0x4a8dd0(this + 0x1a00)` and `0x4a8dd0(this + 0x1a0c)` (two list
  objects, ctor 0x4a8dd0 `__thiscall`); returns this.

## 0x854d50 __init_CEscalators__aEscalators  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(&CEscalators::aEscalators = 0xc6e9a8, size 0x150, count 0x20 = 32,
  CEscalator::Constructor 0x717110, CEscalator::Destructor 0x717130)`, then `_atexit(0x856b20)`.

## 0x856110 __dtor_CGameLogic__SavedWeapons  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0x96a9b8, 0x1c, 13, 0x4411a0)` (dtor is a bare `ret`, so no effect).

## 0x856130 __dtor_CGarages__aGarages  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0x96c048, 0xd8, 50, CGarage::~CGarage 0x447110)`.

## 0x856160 __dtor_CReplay__PlayerInfo  (catalogue: specified/REA, port level 12)
- Tail call: `ecx = 0xa430b0; jmp CPlayerInfo::~CPlayerInfo (0x45b110)`.

## 0x856490 __dtor_gMatrixList  (catalogue: specified/REA, port level 4)
- Tail call: `ecx = 0xb74288; jmp 0x54eb70` (CMatrixLinkList dtor). `0x54eb70` calls the CMatrix dtor
  `0x59acd0` on this+0x1a4, +0x150, +0xfc, +0xa8, +0x54, +0x0 in that order (reverse member order).

## 0x8564c0 __dtor_CWorld__ms_aLodPtrLists  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xb99eb8, 4, 900, 0x43e1a0)` (each = `CPtrListSingleLink::Flush`, element 899 first).

## 0x8564e0 __dtor_CWorld__ms_listMovingEntityPtrs  (catalogue: specified/REA, port level 1)
- Tail call: `ecx = &CWorld::ms_listMovingEntityPtrs (0xb9acc8); jmp CPtrListDoubleLink::Flush (0x552470)`.

## 0x8564f0 __dtor_CWorld__ms_listObjectsWithControlCode  (catalogue: specified/REA, port level 1)
- Tail call: `ecx = 0xb9accc; jmp CPtrListDoubleLink::Flush (0x552470)`.

## 0x856500 __dtor_CWorld__Players  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xb7cd98, 0x190, 2, CPlayerInfo::~CPlayerInfo 0x45b110)` (Players[1] first).

## 0x856520 __dtor_CWorld__ms_aSectors  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xb7d0b8, 8, 14400, 0x563130)`.

## 0x856540 __dtor_CWorld__ms_aRepeatSectors  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xb992b8, 0xc, 256, 0x5631b0)`.

## 0x856600 __dtor_FurnitureManager_c__g_subGroupStore  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xbad3f8, 0x1c, 128, FurnitureSubGroup_c::~FurnitureSubGroup_c 0x590e70)`.

## 0x856620 __dtor_g_bae1f8  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xbae1f8, 0x14, 256, Furniture_c::~Furniture_c 0x590ed0)`.

## 0x856640 __dtor_g_furnitureMan  (catalogue: specified/REA, port level 2)
- Tail call: `ecx = 0xbab380; jmp FurnitureManager_c::~FurnitureManager_c (0x5912e0)`.

## 0x856650 __dtor_g_interiorMan  (catalogue: specified/REA, port level 2)
- Tail call: `ecx = 0xbaf670; jmp InteriorManager_c::~InteriorManager_c (0x598580)`.

## 0x856660 __dtor_g_breakMan  (catalogue: specified/REA, port level 0)
- Tail call: `ecx = 0xbb4240; jmp 0x59e640` (BreakManager_c dtor) =
  `eh_vector_dtor(this, 0x20, 64, 0x59d180)`; `0x59d180` is a bare `ret`, so no observable effect.

## 0x856a80 __dtor_g_waterCreatureMan  (catalogue: specified/REA, port level 2)
- Tail call: `ecx = 0xc1df30; jmp 0x6e3f20` (WaterCreatureManager_c dtor): `0x4a8de0(this + 0x1a0c)`,
  then `0x4a8de0(this + 0x1a00)` (list dtors, reverse order), then
  `eh_vector_dtor(this, 0x34, 128, 0x6e3e50)`.

## 0x856b20 __dtor_CEscalators__aEscalators  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xc6e9a8, 0x150, 32, CEscalator::Destructor 0x717130)`.
