# RENDER static initialisers / atexit destructors (group R1)

All `__init_*` are `__cdecl void()` CRT initialisers (plain `ret`) run from the C++ initialiser table before WinMain; all
`__dtor_*` are `__cdecl void()` atexit callbacks. CRT helpers (excluded:crt): `_atexit` 0x821D1E (cdecl (fn), the caller pops with
`pop ecx`), `eh vector constructor iterator` 0x821D30 (__stdcall (ptr, size, count, ctor, dtor), callee pops 0x14),
`eh vector destructor iterator` 0x821E02 (__stdcall (ptr, size, count, dtor); destroys elements from last to first),
`_ftol2` 0x821B40 (ST0 -> EDX:EAX, truncation toward zero, FPU control word untouched).
Global state at the time these run: .data/.bss as in the image (RsGlobal 0xC17040 is still all zero; CWeather::OldWeatherType
0xC81320 / NewWeatherType 0xC8131C are 0).

## 0x84E7B0 __init_CTimeCycle__m_BelowHorizonGrey  (catalogue: specified/REA, port level 1)
- `CRGBA::CRGBA(r, g, b, a)` 0x7170C0 (thiscall, `ret 0x10`, stores bytes +0..+3 = r, g, b, a) on `CTimeCycle::m_BelowHorizonGrey`
  0xB7CB10 with (0x46, 0x46, 0x46, 0xFF) (pushes 0x84E7B0-0x84E7B9: a first, then b, g, r); then `atexit(0x8564B0)`.
- Result: 0xB7CB10 = {70, 70, 70, 255}. gr: StaticRef only (TimeCycle.h:128), no initial value.

## 0x84E7E0 __init_g_b7cb20  (catalogue: specified/REA, port level 0)
- `word [0xB7CB20] = word [0xC81320]` (CWeather::OldWeatherType, i16) (0x84E7E0/0x84E7E6). No atexit.
- 0xB7CB20 is the i16 "last old weather type" cache written by CTimeCycle::CalcColoursForPoint 0x5603D0 at 0x5605B3. Proposed name:
  `CTimeCycle::CalcColoursForPoint::s_LastOldWeatherType`. Value at startup: 0 (copy of a zero global).

## 0x84E7F0 __init_g_b7cb24  (catalogue: specified/REA, port level 0)
- `word [0xB7CB24] = word [0xC8131C]` (CWeather::NewWeatherType) (0x84E7F0/0x84E7F6). No atexit. Companion cache of 0xB7CB20,
  written at 0x5605BA. Proposed name: `CTimeCycle::CalcColoursForPoint::s_LastNewWeatherType`.

## 0x8508C0 __init_g_c03970  (catalogue: specified/REA, port level 0)
- Zero the four dwords 0xC03970, 0xC03974, 0xC03978, 0xC0397C (0x8508C2-0x8508D6), then `atexit(0x856700)`.
- 0xC03970 is `RpAtomic** PlantModelsTab[4]` of CGrassRenderer (SetPlantModelsTab 0x5DACC0 writes `[idx*4 + 0xC03970]`,
  idx < 4; GetPlantModelsTab 0x5DACE0 reads it). Proposed name: `CGrassRenderer::PlantModelsTab`.

## 0x850910 __init_CPlantMgr__m_AmbientColor  (catalogue: specified/REA, port level 1)
- `CRGBA::CRGBA(0xFF, 0xFF, 0xFF, 0xFF)` 0x7170C0 on 0xC03A44 (CPlantMgr::m_AmbientColor), then `atexit(0x856710)`.

## 0x850940 __init_CPlantMgr__m_LocTrisTab  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(0xC03A48, 0x54, 0x100, ctor 0x5DB2B0, dtor 0x5DB2C0)` (CPlantLocTri[256]; ctor 0x5DB2B0 = `mov eax, ecx; ret`,
  dtor 0x5DB2C0 = `ret`: both no-ops), then `atexit(0x856720)`. gr MAX_NUM_PLANT_TRIANGLES = 256, size 0x54: same.

## 0x850970 __init_CPlantMgr__m_ColEntCacheTab  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(0xC08E48, 0x14, 0x28, ctor 0x5DB290, dtor 0x5DB2A0)` (CPlantColEntEntry[40]; both no-ops), then `atexit(0x856740)`.
  gr MAX_NUM_PROC_OBJECTS = 40, size 0x14: same.

## 0x854810 __init_CPostEffects__m_fNightVisionSwitchOnFXCount  (catalogue: specified/REA, port level 0)
- `dword [0xC40300] = dword [0x8D50B0]` (copy of CPostEffects::m_fNightVisionSwitchOnFXTime, image value 0x42480000 = 50.0f).

## 0x854820 __init_CPostEffects__m_HeatHazeFXScanSizeX  (catalogue: specified/REA, port level 0)
- `[0xC40304] = (i32)_ftol2(((float)RsGlobal.maximumWidth [0xC17044, fild i32] * [0x859520] (0.0015625f)) * [0x85F114] (24.0f))`
  (0x854820-0x854837; x87 fild, fmul f32, fmul f32, truncation). RsGlobal is still 0 here, so the stored value is 0.
- gr comment: `int32(SCREEN_WIDTH_UNIT * 24.0f)` (PostEffects.h:121) - the same expression; the port must evaluate it at static-init
  time (giving 0), not lazily with the real resolution.

## 0x854840 __init_CPostEffects__m_HeatHazeFXScanSizeY  (catalogue: specified/REA, port level 0)
- `[0xC40308] = _ftol2((float)RsGlobal.maximumHeight [0xC17048] * [0x859524] (0.0022321430f = 1/448) * [0x85F114] (24.0f))`
  (0x854840-0x854857). Value 0 at startup.

## 0x854860 __init_CPostEffects__m_HeatHazeFXRenderSizeX  (catalogue: specified/REA, port level 0)
- `[0xC4030C] = _ftol2((float)[0xC17044] * 0.0015625f [0x859520] * 24.0f [0x85F114])` (0x854860-0x854877). Value 0.

## 0x854880 __init_CPostEffects__m_HeatHazeFXRenderSizeY  (catalogue: specified/REA, port level 0)
- `[0xC40310] = _ftol2((float)[0xC17048] * 0.0022321430f [0x859524] * 24.0f [0x85F114])` (0x854880-0x854897). Value 0.

## 0x8548A0 __init_CPostEffects__m_RadiosityPixelsX  (catalogue: specified/REA, port level 0)
- `dword [0xC40314] = dword [0xC17044]` (raw integer copy of RsGlobal.maximumWidth, 0 at this time; no int->float conversion even
  though gr types m_RadiosityPixelsX as float, PostEffects.h:156).

## 0x8548B0 __init_CPostEffects__m_RadiosityPixelsY  (catalogue: specified/REA, port level 0)
- `dword [0xC40318] = dword [0xC17048]` (raw copy of RsGlobal.maximumHeight, 0).

## 0x854A60 __init_MAX_DISTANCE_PED_SHADOWS_SQR  (catalogue: specified/REA, port level 0)
- `[0xC4B6B0] = [0x8D5240] * [0x8D5240]` (f32 15.0f * 15.0f = 225.0f; fld/fmul/fstp f32, 0x854A60-0x854A6C).
- gr: `constexpr` (Shadows.h:247), i.e. not a writable global; same value.

## 0x854AA0 __init_g_c6a17c  (catalogue: specified/REA, port level 0)
- `[0xC6A17C] = [0x872750] * [0x858620]` = 0.005f * 0.0625f = 0.0003125f (f32 store, 0x854AA0-0x854AAC). No atexit.
- 0xC6A17C is an unnamed f32 just before CStencilShadows::m_StencilShadowObjects 0xC6A198 (gr names 0xC6A178 only). Note: the
  address is also formed (not dereferenced) by CStencilShadows::Init 0x70F9E0 at 0x70FA4D as `0xC6A17C + i*0x1C` (= element[i-1]
  pointer of the 0x1C-stride array); no evidence links that to this float. Name: open (proposed `CStencilShadows::sub_c6a17c`
  constant 0.0003125f).

## 0x854B60 __init_CStencilShadows__m_StencilShadowObjects  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(0xC6A198, 0x1C, 0x40, CStencilShadowObject::Constructor 0x70F8D0 (`mov eax, ecx; ret`), Destructor 0x70F8E0
  (`ret`))`, then `atexit(0x856AE0)`. 64 elements of 0x1C, matches gr.

## 0x854EE0 __init_CGlass__aGlassPanes  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(0xC71BF8, 0x70, 0x2D, CFallingGlassPane ctor 0x71A8B0, dtor 0x71A8C0)`, then `atexit(0x856BA0)`.
- COUNT IS 45 (0x2D, 0x854EEA). gta-reversed declares `std::array<CFallingGlassPane, 44>` (Glass.h:32) and iterates it with
  range-for in Glass.cpp (lines 34, 346, 373, 509) - gr skips the 45th pane (0xC71BF8 + 44*0x70 = 0xC72AB8). The port must use 45.

## 0x855310 __init_C3dMarkers__m_aMarkerArray  (catalogue: specified/REA, port level 0)
- `eh_vector_ctor(0xC7DD58, 0xA0, 0x20, C3dMarker ctor 0x720F60, dtor 0x720F70)`, then `atexit(0x856BD0)`. 32 x 0xA0, matches gr.

## 0x855510 __init_CWeather__m_WeatherAudioEntity  (catalogue: specified/REA, port level 1)
- `CAEWeatherAudioEntity::CAEWeatherAudioEntity` 0x72A620 on 0xC81360 (thiscall: `[this+0x10] = 0`, vtable `[this] = 0x872A74`,
  `byte [this+0x7C] = 0`; returns this), then `atexit(0x856C10)`.

## 0x8564B0 __dtor_CTimeCycle__m_BelowHorizonGrey  (catalogue: specified/REA, port level 0)
- `ret` (trivial CRGBA destructor).

## 0x856700 __dtor_g_c03970  (catalogue: specified/REA, port level 0)
- `ret`.

## 0x856710 __dtor_CPlantMgr__m_AmbientColor  (catalogue: specified/REA, port level 0)
- `ret`.

## 0x856720 __dtor_CPlantMgr__m_LocTrisTab  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xC03A48, 0x54, 0x100, 0x5DB2C0)` (no-op dtor), `ret`.

## 0x856740 __dtor_CPlantMgr__m_ColEntCacheTab  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xC08E48, 0x14, 0x28, 0x5DB2A0)` (no-op dtor), `ret`.

## 0x856AE0 __dtor_CStencilShadows__m_StencilShadowObjects  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xC6A198, 0x1C, 0x40, 0x70F8E0)` (no-op dtor), `ret`.

## 0x856BA0 __dtor_CGlass__aGlassPanes  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xC71BF8, 0x70, 0x2D, 0x71A8C0)`: for each of the 45 panes (last first) CMatrix::~CMatrix (frees an owned attached
  RwMatrix, see CFallingGlassPane.md).

## 0x856BD0 __dtor_C3dMarkers__m_aMarkerArray  (catalogue: specified/REA, port level 0)
- `eh_vector_dtor(0xC7DD58, 0xA0, 0x20, 0x720F70)`: 32 markers, CMatrix::~CMatrix each.

## 0x856C10 __dtor_CWeather__m_WeatherAudioEntity  (catalogue: specified/REA, port level 2)
- `mov ecx, 0xC81360; jmp 0x72A400`. 0x72A400 (destructor of CAEWeatherAudioEntity, inherited CAEAudioEntity part):
  `[this] = 0x859988` (CAEAudioEntity vtable), then `ecx = this + 8` and tail jump 0x4EF660 (CAESound::~CAESound on the embedded
  m_tempSound: if `[sound+8] != 0` -> `CEntity::CleanUpOldReference(&[sound+8])` 0x571A00 and `[sound+8] = 0`).
