# VEH static initialisers / atexit destructors (writer group E)

All functions here are `__cdecl void()` CRT initialisers called from the `__xc` init table (table slot given),
or atexit destructors. The init table is walked in ascending slot order, so an initialiser that reads a global
written by another initialiser depends on that one having run first (noted where it applies). Unless stated
otherwise each one is `fld` / one arithmetic op / `fstp` to an f32 global; the CRT start-up runs with the
default MSVC x87 precision control (53-bit), the game's 24-bit setting is not active yet, so the result is the
correctly rounded f32 of the double-precision intermediate (the operands are exact f32 values).
Constants are read from `.rdata`/`.data` at run time; the values given are those in the 1.0 US image.

## 0x84A300 __init_g_97f634  (catalogue: specified/REA, port level 0)
- Table slot 0x8A4280. `[0x97F634] (f32) = [0x859BC0]=0.005f * [0x858620]=0.0625f` (0x84A300-0x84A30C) = 0.0003125f.
  Global unnamed (catalogue: near `CVehicleRecording::NumPlayBackFiles+4`).

## 0x84A340 __init_g_97f63c  (catalogue: specified/REA, port level 0)
- Table slot 0x8A4288. `[0x97F63C] (f32) = [0x858628]=100.0f / [0x859BC4]=3.6f` (`fdiv`, 0x84A346) ~ 27.7778f
  (km/h -> m/s style factor).

## 0x853500 __init_g_c1bfec  (catalogue: specified/REA, port level 0)
- Table slot 0x8A54BC. `[0xC1BFEC] = [0xC1BFE8] * [0x85862C]=10.0f` (0x853500-0x85350C).
- Depends on initialiser 0x8534E0 (slot 0x8A54B8, earlier) which writes 0xC1BFE8; the image value is 0.

## 0x853540 __init_g_c1c208  (catalogue: specified/REA, port level 0)
- Table slot 0x8A54C4. Builds a CVector at 0xC1C208 as the difference of two CVectors in `.data`:
  `[0xC1C208] = [0x8D3198] - [0x8D318C]`, `[0xC1C20C] = [0x8D319C] - [0x8D3190]`, `[0xC1C210] = [0x8D31A0] - [0x8D3194]`
  (three `fld`/`fsub`/`fstp`, x then y then z, 0x853540-0x853570).
- Image values: 0x8D318C = (0.0, -1.394, 2.296), 0x8D3198 = (0.0, 2.95, 2.97) -> result (0.0, 4.344, 0.674) (f32-rounded).
- gta-reversed: `Automobile.cpp:42` labels 0xC1C208 as `TIGER_GUN_POS(0.0f, 0.5f, 0.2f)` (constexpr). The bytes
  contradict that value (unless the two source vectors are rewritten before the first use, which nothing in this
  initialiser shows). Name/value correction: 0xC1C208 is a computed vector, not (0, 0.5, 0.2). Open question:
  confirm that 0x8D318C/0x8D3198 are not written at run time before this init (rwsets show no writer in VEH).

## 0x853600 __init_g_c1c818  (catalogue: specified/REA, port level 0)
- Table slot 0x8A54D8. `[0xC1C818] = [0xC1C814] * [0x85862C]=10.0f`. Depends on initialiser 0x8535E0
  (slot 0x8A54D4) which writes 0xC1C814.

## 0x853620 __init_g_c1c81c  (catalogue: specified/REA, port level 0)
- Table slot 0x8A54DC. `[0xC1C81C] = [0x858630]=0.277778f / [0x87127C]=50.0f` (~0.00555556f; km/h -> units per
  50 fps frame).

## 0x853C20 __init_g_c1cb28  (catalogue: specified/REA, port level 0)
- Table slot 0x8A559C. `[0xC1CB28] = [0xC1CB24] * [0x85862C]=10.0f`. Depends on initialiser 0x853C00
  (slot 0x8A5598) which writes 0xC1CB24.

## 0x853CE0 __init_g_c1cb54  (catalogue: specified/REA, port level 0)
- Table slot 0x8A55B4. `[0xC1CB54] = [0x858630]=0.277778f / [0x871D48]=50.0f` (same value as 0xC1C81C).

## 0x853DC0 __init_CVehicle__m_aSpecialColModel  (catalogue: specified/REA, port level 0)
- Table slot 0x8A55D0. `eh_vector_constructor_iterator(0xC1CC78 /*CVehicle::m_aSpecialColModel*/, 0x30 /*size*/,
  4 /*count*/, CColModel::CColModel 0x40FB60, CColModel::~CColModel 0x40F700)` (pushes at 0x853DC0-0x853DCE, call
  0x821D30, callee-cleaned by the iterator), then `atexit(__dtor_CVehicle__m_aSpecialColModel 0x856A30)` (0x853DDD;
  `pop ecx` cleans the one cdecl arg).

## 0x853DF0 __init_s_TestBladeCol  (catalogue: specified/REA, port level 2)
- Table slot 0x8A55D4. `CColModel::CColModel(&s_TestBladeCol 0xC1CD38)` (thiscall, 0x853DF5), then
  `atexit(__dtor_s_TestBladeCol 0x856A50)`.

## 0x853E10 __init_s_TestBladeColData  (catalogue: specified/REA, port level 1)
- Table slot 0x8A55D8. Tail-jump to `CCollisionData::CCollisionData` 0x40F030 with ECX = &s_TestBladeColData
  0xC1CD68. No atexit registration (CCollisionData has no registered destructor here).

## 0x8540B0 __init_fTimeMult  (catalogue: specified/REA, port level 0)
- Table slot 0x8A562C. `fTimeMult [0xC279CC] = [0x858F08]=1.2f / CBoat::WAKE_LIFETIME [0x8D3940]` (image value
  150.0f -> 0.008f). WAKE_LIFETIME is read from `.data` (a variable, not a constant).

## 0x854350 __init_g_c2b9b4  (catalogue: specified/REA, port level 0)
- Table slot 0x8A5680. `[0xC2B9B4] = 1.0f / ([0x872344]=50.0f * [0x872344]=50.0f)` (`fld; fmul; fdivr [0x858624]=1.0f`)
  = 0.0004f. gr comment (cHandlingDataMgr.cpp:7): acceleration conversion factor.

## 0x854390 __init_g_c2b9bc  (catalogue: specified/REA, port level 0)
- Table slot 0x8A5688. `[0xC2B9BC] = [0x858630]=0.277778f / [0x872344]=50.0f`. gr (cHandlingDataMgr.cpp:10): velocity
  conversion factor.

## 0x8543D0 __init_gHandlingDataMgr  (catalogue: specified/REA, port level 1)
- Table slot 0x8A5690. Two steps, in this order:
  1. 210 (0xD2) calls of `cTransmission::cTransmission` 0x6D0450 (thiscall, ECX = 0xC2BA08 + i*0xE0, i ascending;
     0x8543E0-0x8543EE). That constructor is `mov eax, ecx; ret` (no stores), so the loop has no effect.
  2. `rep stosd` of 0x3189 dwords of 0 from `gHandlingDataMgr` 0xC2B9C8 (0xC624 bytes = sizeof(cHandlingDataMgr)).
- Net effect: gHandlingDataMgr is all zero. No atexit.

## 0x854410 __init_g_c3804c  (catalogue: specified/REA, port level 0)
- Table slot 0x8A5694. `[0xC3804C] = [0x872358]=0.005f * [0x858620]=0.0625f` = 0.0003125f. 0xC3804C lies directly after
  `StationDist[6]` (0xC38034..0xC3804B); it is a separate f32 global (the catalogue's `StationDist+0x18` label is just
  proximity). Note: CTrain::FindMaximumSpeedToStopAtStations uses 0xC3804C as its loop end address, not as data.

## 0x856A30 __dtor_CVehicle__m_aSpecialColModel  (catalogue: specified/REA, port level 0)
- atexit callback registered by 0x853DC0. `eh_vector_destructor_iterator(0xC1CC78, 0x30, 4, CColModel::~CColModel
  0x40F700)` (0x856A3E); destroys the 4 elements in reverse order (CRT iterator semantics).

## 0x856A50 __dtor_s_TestBladeCol  (catalogue: specified/REA, port level 5)
- atexit callback registered by 0x853DF0. Tail-jump to `CColModel::~CColModel` 0x40F700 with ECX = 0xC1CD38.
