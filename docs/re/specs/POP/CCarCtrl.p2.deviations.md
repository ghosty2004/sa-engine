# CCarCtrl (POP) - gta-reversed deviation notes, part 2 (group P2)

Source: `re/gta-reversed/source/game_sa/CarCtrl.cpp`. Vehicle pool: `CPools::ms_pVehiclePool` 0xB74494 -> CPool {+0 m_pObjects,
+4 m_byteMap (flags, bit 7 = free), +8 m_nSize}; vehicle stride 0xA18. See CCarCtrl.deviations.md (P1) for the other functions.

## 0x42C250 CCarCtrl::IsAnyoneParking  (gr_deviations = 1)
Original: entry `jmp 0x15636D0` (.text detour stub, body at 0x15636D0). `__cdecl bool ()`, AL. `i = pool->m_nSize`; while `i != 0`:
`i--`; skip if `byteMap[i] & 0x80` (free, `js`) or `base + i*0xA18 == 0`; if `veh->+0x3BA` (m_autoPilot.m_nCarMission, u8) is 0x2D, 0x2E,
0x31 or 0x32 return 1 (0x1563701-0x1563715). Return 0 after slot 0. Slots are walked from the LAST down to 0.
Row:
- CarCtrl.cpp:569 `GetAllValid()` (ascending). Original descending (0x15636EC-0x1563719). REFUTED: pure predicate, no side effects,
  the result does not depend on the order. harmless. gr's case labels PARK_PERPENDICULAR 45, PARK_PARALLEL 46, *_2 49/50 match
  0x2D/0x2E/0x31/0x32.

## 0x42CD10 CCarCtrl::RemoveDistantCars  (gr_deviations = 4)
Original: `__cdecl void ()`. `edi = pool` (also saved at [esp+8], restored at 0x42CE1D because edi is reused), `i = pool->m_nSize`
(read once, 0x42CD1B). Loop while `i != 0` (0x42CE21): `i--`; `byteMap` (`pool->+4`, re-read every iteration 0x42CD34) `[i] & 0x80` ->
skip; `v = pool->m_pObjects (re-read) + i*0xA18`, skip if 0. Then:
1. `CCarCtrl::PossiblyRemoveVehicle(v)` 0x424F80 (may delete `v`).
2. `if (!(v->+0x42A & 0x10)) continue` (vehicleFlags bCreateRoadBlockPeds; READ FROM THE SLOT EVEN IF STEP 1 DELETED IT).
3. `c = FindPlayerCentreOfWorld(PlayerInFocus 0xB7CD74)` 0x56E250, copied; `P = v->GetPosition()` (matrix +0x30 or placement +4).
   `d = sqrt((P.y - c.y)*(P.y - c.y) + (P.x - c.x)*(P.x - c.x))` (0x42CDAC-0x42CDCC: 2D distance, y term first, no z).
   `if (!(d < 54.5f [0x858FF4])) continue` (`test ah,5; jp`, NaN skips).
4. `CRoadBlocks::GenerateRoadBlockPedsForCar(v, v->+0x4AD (u8, zero-extended), IsLawEnforcementVehicle(v) 0x6D2370 ? 6 : 7)` 0x461170
   (cdecl, 3 args); then `v->+0x42A &= ~0x10`.
Rows:
- CarCtrl.cpp:802 bugfix `CCarCtrl_RemoveDistantCars_UseAfterFree` (separate removal pass). Original: single pass, flag read after
  PossiblyRemoveVehicle (0x42CD55 -> 0x42CD5A). CONFIRMED. keep-original (with the fix off gr also does a single pass, see :812).
- CarCtrl.cpp:803 `GetAllValid()` (ascending) in the fix pass. CONFIRMED: original descending (0x42CD37-0x42CD3C). keep-original:
  order matters because PossiblyRemoveVehicle's decisions read global counters / the pool, and RoadBlock ped creation allocates pool slots.
- CarCtrl.cpp:811 `GetAllValid()` (ascending) main loop. CONFIRMED, same reason. keep-original. Note also that the original
  evaluates the free flag for a slot only once, before PossiblyRemoveVehicle; a vehicle deleted in step 1 is still processed by
  steps 2-4 using the stale slot memory (gr's range adaptor also re-checks nothing).
- CarCtrl.cpp:812 fix-off path (single pass). CONFIRMED only in direction (ascending vs descending). keep-original.
Found in passing (CONFIRMED, keep-original): gr uses `DistanceBetweenPoints(FindPlayerCentreOfWorld(), veh.GetPosition())` (3D,
Vector.h:274); the original distance is 2D (x,y only, 0x42CDAC-0x42CDCC). gr's `>= 54.5f -> continue` lets NaN through; the
original skips on NaN. Ped type 6 = COP, 7 = GANG1 (matches gr).
