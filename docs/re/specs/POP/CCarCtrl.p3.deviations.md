# CCarCtrl (POP) - gta-reversed deviation notes, part 3 (group P3)

Source: `re/gta-reversed/source/game_sa/CarCtrl.cpp`. Vehicle pool `CPools::ms_pVehiclePool` 0xB74494 -> CPool {+0 m_pObjects,
+4 m_byteMap (bit 7 = free), +8 m_nSize}; vehicle stride 0xA18. Parts 1/2: CCarCtrl.deviations.md, CCarCtrl.p2.deviations.md.

## 0x4322B0 CCarCtrl::RemoveCarsIfThePoolGetsFull  (gr_deviations = 1)
Original: `__cdecl void ()`, plain `ret`.
1. Return unless `(CTimer::m_FrameCounter 0xB7CB4C & 7) == 3` (low byte compare, 0x4322B0-0x4322BD).
2. `used` = number of slots with `byteMap[i] & 0x80 == 0` over i = 0..size-1 (0x4322D7-0x4322EA); return if `size - used >= 8`
   (signed, 0x4322F3).
3. `best = null`, `bestDist = 10000000.0f` (immediate 0x4B189680, 0x4322FD). Slots walked from `size-1` DOWN to 0 (0x432320-0x4323E9;
   byteMap re-read each iteration); skip free slots (`js`) and null slot addresses; skip if `v == apCarsToKeep[0]` or `[1]`
   (0x969084, 2 entries, 0x432350-0x432365); skip if `v->+0x428 & 8` (vehicleFlags bIsLocked, 0x432367); skip if `!v->CanBeDeleted()`
   0x6D1180; skip if `CCranes::IsThisCarBeingTargettedByAnyCrane(v)` 0x6F4000.
   `d = cam - vpos` (cam = TheCamera matrix ptr 0xB6F03C ? matrix+0x30 : 0xB6F02C; vpos = v matrix +0x30 or placement +4; camera
   position re-read per vehicle), `dist = sqrt((dz*dz + dx*dx) + dy*dy)` (x87 order, 0x4323A8-0x4323C8). If `dist < bestDist`
   (`fcom; test ah,5; jp`, strict, NaN no, 0x4323D9): `bestDist = dist` (stored f32), `best = v`.
4. If `best`: `CWorld::Remove(best)` 0x563280, then virtual slot 0 (scalar deleting destructor) with arg 1 (0x432409).
Row CarCtrl.cpp:771 `for (auto& veh : GetVehiclePool()->GetAllValid())` (ascending):
- Original bytes: descending walk 0x432320-0x4323E9 with strict `<` (0x4323D9).
- Verdict: CONFIRMED. With equal distances the original keeps the HIGHEST slot index, gr the lowest; a different vehicle is deleted.
- Class: keep-original.
Found in passing (keep-original, rounding/limit only): gr starts with `numeric_limits<float>::max()`, the original with 10000000.0f
(vehicles 1e7 or more from the camera are never chosen); gr's `Magnitude()` sums x*x + y*y + z*z, the original (dz^2 + dx^2) + dy^2.
The rest (free-space test, frame filter, skip conditions and their order, `fClosestDist > dist`) matches gr.

## 0x4341C0 CCarCtrl::GenerateRandomCars  (gr_deviations = 1)
Original: `__cdecl void ()`, plain `ret` / tail jump.
1. If `CCutsceneMgr::ms_running` 0xB5F851: `CountDownToCarsAtStart (0x9690C0, u8) = 2`, return (0x4341C9).
2. Return if `CGangWars::DontCreateCivilians()` 0x4439C0 or `CGame::currArea (0xB72914) != 0` (0x4341E1).
3. If `CGameLogic::LaRiotsActiveHere()` 0x441C10 and `TimeNextMadDriverChaseCreated (0x9690BC) > 480.0f` (`0x85902C = 480.0`,
   `test ah,0x41; jne`, NaN no): `T = CGeneral::GetRandomNumberInRange<float>(240.0f, 480.0f)` (0x41BD90, cdecl, pushes 480 then 240,
   ST0 result, 0x434209).
4. `T = T - ms_fTimeStep * 0.02f` (`0x858B38 = 0.019999999552965164`, x87 `fmul; fsubr`, stored f32, 0x434217-0x434231) - always.
5. If `NumRandomCars (0x969094) < 45` (signed, 0x434237): if `CountDownToCarsAtStart != 0`: decrement it (stored); if it became 0:
   100 x GenerateOneRandomCar 0x430050, then `CTheCarGenerators::GenerateEvenIfPlayerIsCloseCounter (0xC279D0, u8) = 20`.
   Else (countdown 0): GenerateOneRandomCar twice.
6. `CTrain::DoTrainGenerationAndRemoval()` 0x6F7900, `CPlane::DoPlaneGenerationAndRemoval()` 0x6CD2F0.
7. If `(m_snPreviousTimeInMilliseconds 0xB7CB78 ^ m_snTimeInMilliseconds 0xB7CB84) & 0xFFFFF000` tail-jump to
   GenerateEmergencyServicesCar 0x42F9C0 (equivalent to gr's `now/4096 != prev/4096` on u32).
Row CarCtrl.cpp:485 (rng-helper) `TimeNextMadDriverChaseCreated = CGeneral::GetRandomNumberInRange(240.0f, 480.0f)`:
- Original bytes: call 0x41BD90 (original float helper, `(max - min) * t + min`) at 0x434209.
- Verdict: CONFIRMED (gr's `lerp` helper rounds differently, docs/re/cat14-gr-deviations.md finding 1). Class: keep-original
  (port must call the original 0x41BD90 formula).
Everything else matches gr (CanSeeOutSideFromCurrArea == `currArea == 0`).
