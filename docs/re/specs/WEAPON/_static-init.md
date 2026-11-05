# WEAPON static initialisers and atexit destructors

All `__init_*` functions are CRT dynamic initialisers (`__cdecl void()`, plain `ret`), run once at startup; all
`__dtor_*` are the atexit callbacks they register (`_atexit` 0x821D1E, followed by `pop ecx`). Array helpers:
`eh_vector_constructor_iterator` 0x821D30 `(void* p, size_t size, int count, ctor, dtor)` and
`eh_vector_destructor_iterator` 0x821E02 `(void* p, size_t size, int count, dtor)` (destroys from the last element down).
Float initialisers: one x87 op on two f32 .rdata constants, result rounded to f32. They run with the CRT-default
x87 precision (53-bit), but a single mul/div of two f32 values gives the same f32 either way here; the exact
resulting bit patterns are given so a port can use constants.

## 0x84DE70 __init_gFireManager  (catalogue: specified/REA, port level 2)
`CFireManager::Constructor` 0x539DA0 on `gFireManager` 0xB71F80 (ECX), then `_atexit(__dtor_gFireManager 0x856420)`.

## 0x855420 __init_CWaterCannons__aCannons  (catalogue: specified/REA, port level 0)
`eh_vector_constructor_iterator(0xC80740 CWaterCannons::aCannons, 0x3CC, 3, CWaterCannon::CWaterCannon 0x728B10,
CWaterCannon::~CWaterCannon 0x728B30)`; `_atexit(0x856BF0)`.

## 0x8559E0 __init_CExplosion__m_ExplosionAudioEntity  (catalogue: specified/REA, port level 1)
`CAEExplosionAudioEntity::CAEExplosionAudioEntity` 0x5074B0 (thiscall, ECX = 0xC888D0 `CExplosion::m_ExplosionAudioEntity`);
`_atexit(0x856C20)`.

## 0x855A00 __init_g_c89190  (catalogue: specified/REA, port level 0)
`*(f32*)0xC89190 = 0.005f [0x872BDC] * 0.0625f [0x858620]` (fld/fmul/fstp) = 0.0003125f, bits 0x39A3D70A.
0xC89190 lies just after `ms_apProjectile` (0xC89110 + 0x80); file-static of ProjectileInfo.cpp (no gr name). catalog/rwsets.csv lists 0x738B20 and 0x739860 as referencing
0xC89190, but in 0x738B20 that is the loop-end immediate `cmp edi, 0xC89190` (end of ms_apProjectile), not a read of
this float; no reader of the value was found in WEAPON.

## 0x855AC0 __init_g_c89678  (catalogue: specified/REA, port level 0)
`*(f32*)0xC89678 = 0.005f [0x872C04] * 0.0625f [0x858620]` = 0.0003125f, bits 0x39A3D70A (a second, separate copy in
ShotInfo.cpp's .data, after `CShotInfo::ms_afRandTable`+0x50; listed as referenced by 0x739B60, probably also an array-end immediate; reader not identified).

## 0x855C20 __init_g_c8a7d8  (catalogue: specified/REA, port level 0)
`*(f32*)0xC8A7D8 = 0.277778f [0x858630] / 50.0f [0x872C18]` (fld/fdiv/fstp) = 0.00555556f, bits 0x3BB60B6A.
catalog/rwsets.csv lists CShotInfo::Update 0x739E60 as referencing the address (as an immediate, likely the end
bound of aShotInfos); no read of the float value was identified.

## 0x855C40 __init_CWeapon__ms_PelletTestCol  (catalogue: specified/REA, port level 2)
`CColModel::CColModel` 0x40FB60 on `CWeapon::ms_PelletTestCol` 0xC8A7DC (ECX); `_atexit(0x856C30)`.

## 0x855D20 __init_gCrossHair  (catalogue: specified/REA, port level 0)
`eh_vector_constructor_iterator(0xC8A838 gCrossHair, 0x2C, 2, CWeaponEffects ctor 0x742A90 (`mov eax,ecx; ret`, no stores),
CWeaponEffects dtor 0x742AA0 (`ret`))`; `_atexit(0x856C40)`. Net effect: nothing is written.

## 0x855D70 __init_g_c8aaa4  (catalogue: specified/REA, port level 0)
`*(f32*)0xC8AAA4 = 1.0f [0x858624] / 50.0f [0x872CE4]` = 0.02f, bits 0x3CA3D70A. File-static after
`CWeaponInfo::ms_WeaponAimOffsets`; listed as referenced by 0x5BF750 (immediate; not checked).

## 0x855E10 __init_CWeaponInfo__aWeaponInfo  (catalogue: specified/REA, port level 0)
`eh_vector_constructor_iterator(0xC8AAB8 CWeaponInfo::aWeaponInfo, 0x70, 80, CWeaponInfo ctor 0x743C30 (`mov eax,ecx; ret`),
dtor 0x743C40 (`ret`))`; `_atexit(0x856C60)`. Net effect: nothing is written.

## 0x856420 __dtor_gFireManager  (catalogue: specified/REA, port level 1)
`mov ecx, 0xB71F80; jmp CFireManager::Destructor 0x538BB0` (tail call).

## 0x856BF0 __dtor_CWaterCannons__aCannons  (catalogue: specified/REA, port level 0)
`eh_vector_destructor_iterator(0xC80740, 0x3CC, 3, CWaterCannon::~CWaterCannon 0x728B30)`.

## 0x856C20 __dtor_CExplosion__m_ExplosionAudioEntity  (catalogue: specified/REA, port level 2)
`mov ecx, 0xC888D0; jmp 0x506B90` = the inlined `CAEAudioEntity` destructor body: `*(ecx) = 0x859988` (CAEAudioEntity
vtable), `ecx += 8; jmp CAESound::~CAESound 0x4EF660` (destroys m_tempSound). CAEExplosionAudioEntity has no own dtor.

## 0x856C30 __dtor_CWeapon__ms_PelletTestCol  (catalogue: specified/REA, port level 5)
`mov ecx, 0xC8A7DC; jmp CColModel::~CColModel 0x40F700`.

## 0x856C40 __dtor_gCrossHair  (catalogue: specified/REA, port level 0)
`eh_vector_destructor_iterator(0xC8A838, 0x2C, 2, 0x742AA0)` (dtor is `ret`, no effect).

## 0x856C60 __dtor_CWeaponInfo__aWeaponInfo  (catalogue: specified/REA, port level 0)
`eh_vector_destructor_iterator(0xC8AAB8, 0x70, 80, 0x743C40)` (dtor is `ret`, no effect).
