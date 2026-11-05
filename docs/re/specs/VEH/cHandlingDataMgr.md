# cHandlingDataMgr (VEH) - full specs

Layout (gr `cHandlingDataMgr.h`, size 0xC624): +0x00 `field_0`, +0x04 `fWheelFriction`, +0x08 `field_8`, +0x0C
`field_C`, +0x10 `field_10` (f32), +0x14 `m_aVehicleHandling[210]` (tHandlingData, 0xE0), then bike, flying and boat
arrays. The global instance `gHandlingDataMgr` is at 0xC2B9C8 (0xC2B9DC - 0x14).

## 0x5BF3D0 cHandlingDataMgr::Initialise  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (cHandlingDataMgr* this)`, plain `ret`.
- Callers: none in the catalogue (CGame init path calls LoadHandlingData directly at 0x5BFA90).
  Callees: cHandlingDataMgr::LoadHandlingData 0x5BD830.
- Behaviour: `LoadHandlingData()`; then dword stores (after the load): +0x00 = 0.1f (0x3DCCCCCD), +0x04 = 0.9f
  (0x3F666666), +0x08 = 1.0f, +0x0C = 0.8f (0x3F4CCCCD), +0x10 = 0.98f (0x3F7AE148) (0x5BF3D8-0x5BF3F3).
- gta-reversed: C++ exists (cHandlingDataMgr.cpp:233) and matches.
