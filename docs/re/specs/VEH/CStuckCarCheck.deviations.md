# CStuckCarCheck: gta-reversed deviation notes (VEH, writer group E)

Source: `re/gta-reversed/source/game_sa/StuckCarCheck.cpp` (pin 20d8d2a).

## 0x465970 CStuckCarCheck::AddCarToCheck  (gr_deviations = 1)
Row StuckCarCheck.cpp:35 (`// NOTSA optimization`, the early `return` inside the free-slot loop).
- Original bytes (`.HOODLUM` 0x15627A0): `__thiscall (this, int32 handle, float radius, uint32 time, bool warpCar,
  bool stuck, bool flipped, bool inWater, int8 nodes)`, `ret 0x20`. Vehicle lookup = `CPools::ms_pVehiclePool`
  0xB74494: `if (byteMap[handle >> 8] != (uint8)handle) return;` (signed `sar`, low-byte compare, 0x15627AF-0x15627B7),
  `veh = objects + (handle >> 8) * 0xA18`. Loop `i = 0..15` (u16 counter): the first slot with
  `CarHandle < 0` (signed `jl`, 0x15627DD) is taken and the loop ends; if none, return (0x15627E6).
  Fill: CarHandle = handle; LastPos = vehicle position (matrix pos or placement); LastChecked =
  `CTimer::m_snTimeInMilliseconds` (read through the obfuscated pointer `[0x1560494] + [0x7172D2] + 1` =
  0xB7CB84, 0x156281E-0x156282C); StuckRadius, CheckTime; +0x1D warpCar, +0x1E stuck, +0x1F flipped, +0x1C = 0,
  +0x20 inWater, +0x21 nodes.
- Verdict: REFUTED for the marked line (the original also stops at the first free slot, so the "optimization"
  is not a behavioural difference). One real but harmless difference: the original treats any negative
  CarHandle as free (`< 0`), gr only `== -1`; handles stored there are -1 or valid (>= 0), so no observable effect.
- Class: harmless (a port may use `< 0` to match the bytes exactly).
