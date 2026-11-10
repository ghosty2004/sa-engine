# CWeather (RENDER) - full specs

Globals: `CWeather::WeatherRegion` 0xC81314 (i16, eWeatherRegion: 0 DEFAULT, 1 LA, 2 SF, 3 LV, 4 DESERT). `TheCamera` 0xB6F028
(+0x04 placement position 0xB6F02C, +0x14 m_matrix 0xB6F03C; matrix pos at +0x30).

## 0x72A640 CWeather::UpdateWeatherRegion  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (CVector* pos [esp+4])`, plain `ret`. pos may be null.
- Callers: CWeather::SetWeatherToAppropriateTypeNow 0x72A790, CWeather::Update 0x72B850 (2 sites). Callees: none.
- Globals: reads TheCamera.m_matrix 0xB6F03C, TheCamera placement pos 0xB6F02C; writes WeatherRegion 0xC81314 (word stores).
- Constants: 0x858C4C = 1000.0f, 0x872A8C = 910.0f, 0x872A88 = -850.0f, 0x872A84 = 1280.0f, 0x872A80 = -1430.0f,
  0x872A7C = -580.0f, 0x872A78 = 1430.0f, 0x859F80 = 250.0f, 0x859A94 = 3000.0f, 0x859A90 = -3000.0f.
- Behaviour: P = camera position (`m_matrix ? m_matrix+0x30 : &placement.pos`, 0x72A640-0x72A656) copied to a local; if
  `pos != null` the local is overwritten with *pos (0x72A670). Only x, y are used. Compares are `fcomp` against f32 memory;
  "x > c" = `test ah,0x41; jne` (NaN false), "x < c" = `test ah,5; jp` (NaN false).
  1. `x > 1000 && y > 910` -> region 3 (0x72A686-0x72A6A8).
  2. else `x > -850 && x < 1000 && y > 1280` -> 4 (0x72A6B5-0x72A6E8).
  3. else `x < -1430 && y > -580 && y < 1430` -> 2 (0x72A6F5-0x72A728).
  4. else `x > 250 && x < 3000 && y > -3000` -> store 1 (0x72A76C, BEFORE the last test), then if `!(y < -850)` store 0
     (0x72A775-0x72A782); if the earlier tests of step 4 fail -> 0.
- Edge cases: in step 4 WeatherRegion is transiently 1 before being set to 0 (single thread, harmless).
- gta-reversed: UpdateWeatherRegion -> FindWeatherRegion (Weather.cpp:687-706): same tests in the same order; the hook is
  commented out ("bad", Weather.cpp:37). No behavioural difference found. Open questions: none.
