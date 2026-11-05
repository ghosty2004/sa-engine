# STREAM static initialisers and atexit destructors - writer A (RE-SPEC-1)

All `__init_*` functions are entries of the CRT `__xc` initialiser table (called once by the CRT before
`WinMain`, `__cdecl void(void)`, plain `ret`). All `__dtor_*` functions are registered with `_atexit`
(0x821D1E) by their initialiser and run at process exit in reverse registration order.

CRT helpers (excluded from porting, called via `orig!` or replaced by the Rust runtime equivalent):
- 0x821D30 `eh vector constructor iterator(void* arr, size_t elemSize, int count, ctor, dtor)` (stdcall):
  calls `ctor(arr + i*elemSize)` for i = 0..count-1 ascending; on exception destroys the constructed ones.
- 0x821E02 `eh vector destructor iterator(void* arr, size_t elemSize, int count, dtor)` (stdcall): calls
  `dtor` on elements in DESCENDING order (i = count-1 .. 0).
- 0x821D1E `_atexit(fn)` (cdecl).

## Model-info stores (`CStore<T,N>`: +0x0 i32 count, +0x4 T[N])
Each `__init_CModelInfo__ms_XStore` does `ECX = &store; call storeCtor; _atexit(__dtor_...)`. Each store
constructor (`__thiscall`, plain `ret`, returns EAX = this) does
`eh_vector_constructor_iterator(this+4, elemSize, N, elemCtor, elemDtor); this->count = 0;` (count is set
AFTER the elements are constructed). Each `__dtor_CModelInfo__ms_XStore` is
`mov ecx, &store; jmp helper`, and the helper does
`eh_vector_destructor_iterator(ecx+4, elemSize, N, elemDtor); ret` (count is not touched).

| init (level 1) | dtor (level 0) | store | store ctor | dtor helper | elem size | N | elem ctor | elem dtor |
|---|---|---|---|---|---|---|---|---|
| 0x84BBF0 | 0x856230 | ms_atomicModelInfoStore 0xAAE950 | 0x4C5CB0 | 0x4C5840 | 0x20 | 0x36B0 (14000) | 0x4C5540 | 0x4C5590 ~CAtomicModelInfo |
| 0x84BC10 | 0x856240 | ms_damageAtomicModelInfoStore 0xB1BF58 | 0x4C5D10 | 0x4C5860 | 0x24 | 0x46 (70) | 0x4C55A0 | 0x4C55D0 ~CDamageAtomicModelInfo |
| 0x84BC30 | 0x856250 | ms_lodAtomicModelInfoStore 0xB1C934 | 0x4C5D70 | 0x4C5880 | 0x24 | 1 | 0x4C55E0 | 0x4C5620 ~CLodAtomicModelInfo |
| 0x84BC50 | 0x856260 | ms_timeModelInfoStore 0xB1C960 | 0x4C5DD0 | 0x4C58A0 | 0x24 | 0xA9 (169) | 0x4C5640 | 0x4C5680 ~CTimeModelInfo |
| 0x84BC70 | 0x856270 | ms_lodTimeModelInfoStore 0xB1E128 | 0x4C5E30 | 0x4C58C0 | 0x28 | 1 | 0x4C5690 | 0x4C56E0 ~CLodTimeModelInfo |
| 0x84BC90 | 0x856280 | ms_weaponModelInfoStore 0xB1E158 | 0x4C5E90 | 0x4C58E0 | 0x28 | 0x33 (51) | 0x4C5760 | 0x4C5790 ~CWeaponModelInfo |
| 0x84BCB0 | 0x856290 | ms_clumpModelInfoStore 0xB1E958 | 0x4C5EF0 | 0x4C5900 | 0x24 | 0x5C (92) | 0x4C56F0 | 0x4C5750 ~CClumpModelInfo |
| 0x84BCD0 | 0x8562A0 | ms_vehicleModelInfoStore 0xB1F650 | 0x4C5F50 | 0x4C6370 | 0x308 | 0xD4 (212) | 0x4C75E0 | 0x4C5920 ~CVehicleModelInfo |
| 0x84BCF0 | 0x8562B0 | ms_pedModelInfoStore 0xB478F8 | 0x4C67D0 | 0x4C6390 | 0x44 | 0x116 (278) | 0x4C57A0 CPedModelInfo() | 0x4C62F0 ~CPedModelInfo |

Notes: all element destructors except ~CPedModelInfo only reset the vptr to 0x85BB9C; ~CPedModelInfo also
deletes m_pHitColModel (see CPedModelInfo.md). N values are the gta-reversed `NUM_*_MODEL_INFOS` constants
(ModelInfo.h); the LOD stores really hold a single element in 1.0 US. gta-reversed has no code for these
(static `StaticRef` storage); the Rust port realises them as `globals!` storage initialised the same way.
Below, one short entry per address for the index.

## 0x84BBF0 __init_CModelInfo__ms_atomicModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_atomicModelInfoStore` (0xAAE950) with its store ctor (table row), then `_atexit(0x856230)`. Open questions: none.

## 0x856230 __dtor_CModelInfo__ms_atomicModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xAAE950; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xAAE950+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BC10 __init_CModelInfo__ms_damageAtomicModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_damageAtomicModelInfoStore` (0xB1BF58) with its store ctor (table row), then `_atexit(0x856240)`. Open questions: none.

## 0x856240 __dtor_CModelInfo__ms_damageAtomicModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1BF58; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1BF58+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BC30 __init_CModelInfo__ms_lodAtomicModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_lodAtomicModelInfoStore` (0xB1C934) with its store ctor (table row), then `_atexit(0x856250)`. Open questions: none.

## 0x856250 __dtor_CModelInfo__ms_lodAtomicModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1C934; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1C934+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BC50 __init_CModelInfo__ms_timeModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_timeModelInfoStore` (0xB1C960) with its store ctor (table row), then `_atexit(0x856260)`. Open questions: none.

## 0x856260 __dtor_CModelInfo__ms_timeModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1C960; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1C960+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BC70 __init_CModelInfo__ms_lodTimeModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_lodTimeModelInfoStore` (0xB1E128) with its store ctor (table row), then `_atexit(0x856270)`. Open questions: none.

## 0x856270 __dtor_CModelInfo__ms_lodTimeModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1E128; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1E128+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BC90 __init_CModelInfo__ms_weaponModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_weaponModelInfoStore` (0xB1E158) with its store ctor (table row), then `_atexit(0x856280)`. Open questions: none.

## 0x856280 __dtor_CModelInfo__ms_weaponModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1E158; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1E158+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BCB0 __init_CModelInfo__ms_clumpModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_clumpModelInfoStore` (0xB1E958) with its store ctor (table row), then `_atexit(0x856290)`. Open questions: none.

## 0x856290 __dtor_CModelInfo__ms_clumpModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1E958; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1E958+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BCD0 __init_CModelInfo__ms_vehicleModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_vehicleModelInfoStore` (0xB1F650) with its store ctor (table row), then `_atexit(0x8562A0)`. Open questions: none.

## 0x8562A0 __dtor_CModelInfo__ms_vehicleModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB1F650; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB1F650+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84BCF0 __init_CModelInfo__ms_pedModelInfoStore  (catalogue: specified/REA, port level 1)
- Constructs `CModelInfo::ms_pedModelInfoStore` (0xB478F8) with its store ctor (table row), then `_atexit(0x8562B0)`. Open questions: none.

## 0x8562B0 __dtor_CModelInfo__ms_pedModelInfoStore  (catalogue: specified/REA, port level 0)
- `mov ecx, 0xB478F8; jmp <dtor helper>` -> `eh_vector_destructor_iterator(0xB478F8+4, elemSize, N, elemDtor)` (table row). Open questions: none.

## 0x84F2B0 __init_g_bab358  (catalogue: specified/REA, port level 0)
- Initialises the unnamed f32 global 0xBAB358 (read only by CLoadingScreen::RenderSplash 0x58FF60; gr has no
  name for it): `fld dword [0x858630] (f32 0x3E8E38EB = 0.277778); fdiv dword [0x866CC0] (f32 50.0);
  fstp dword [0xBAB358]`.
- Result stored: f32 bits 0x3BB60B6A (= 0.00555556). Computed here with exact rational arithmetic; the
  quotient is not near a rounding boundary, so the value is the same under 24-, 53- or 64-bit x87 precision
  control (the CRT runs initialisers before the game sets 24-bit PC). A port may store the constant directly.
- No atexit registration. Open questions: none.

## 0x84F2D0 __init_CLoadingScreen__m_aSplashes  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xBAB35C, 4, 7, 0x727230 CSprite2d::Constructor, 0x7281E0 CSprite2d::Destructor)`
  (`CLoadingScreen::m_aSplashes`, 7 CSprite2d of 4 bytes each, gr LoadingScreen.h:37), then
  `_atexit(0x8565E0)`. Open questions: none.

## 0x8565E0 __dtor_CLoadingScreen__m_aSplashes  (catalogue: specified/REA, port level 0)
- `eh_vector_destructor_iterator(0xBAB35C, 4, 7, 0x7281E0 CSprite2d::Destructor)` (elements destroyed in
  descending order); plain `ret`. Open questions: none.
