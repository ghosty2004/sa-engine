# CTrain: gta-reversed deviation notes (VEH, writer group E)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Train.cpp` (pin 20d8d2a). The vehicle pool is
`CPools::ms_pVehiclePool` 0xB74494 (+0 objects, +4 flag bytes, +8 size; slot stride 0xA18; flag byte < 0 = free).

## 0x6F6A20 CTrain::RemoveMissionTrains  (gr_deviations = 1)
Row Train.cpp:350 (`for (auto& vehicle : GetVehiclePool()->GetAllValid())`, pool-iteration).
- Original bytes: slot loop from `size - 1` down to 0 (0x6F6A40-0x6F6A8D: `dec edi` before the test). For each
  used slot with `m_nVehicleType == 6`, not `FindPlayerVehicle(-1, false)`, and `flags & 0x20` (bMissionTrain):
  `CWorld::Remove(v)`, `v->vtable[0](1)` (deleting destructor).
- Verdict: CONFIRMED (gr iterates ascending, the original descending). The filter and actions match.
- Class: keep-original (iterate from the highest slot; deletion order affects which freed objects later carriage
  pointers refer to and the pool free-slot bookkeeping order).

## 0x6F6AA0 CTrain::RemoveAllTrains  (gr_deviations = 1)
Row Train.cpp:376 (pool-iteration).
- Original bytes: descending slot loop (0x6F6AC4-0x6F6B4B). For each train: `inTrain = false`; walk from the
  vehicle itself through `m_pPrevCarriage` (0x6F6AF0-0x6F6B0A) and then again from itself through
  `m_pNextCarriage` (0x6F6B10-0x6F6B2A), setting inTrain when a carriage equals `FindPlayerVehicle(-1, false)`
  (called for every carriage, no early exit). If `!inTrain`: Remove + delete this one vehicle.
- Verdict: CONFIRMED (iteration order). Note that the walks read carriages that may already have been deleted
  earlier in the same loop (freed pool memory); with the original order that memory is the same as what gr would
  read only if the order matches.
- Class: keep-original.

## 0x6F6B60 CTrain::ReleaseMissionTrains  (gr_deviations = 1)
Row Train.cpp:391 (pool-iteration).
- Original bytes: descending slot loop; for each train that is not `FindPlayerVehicle(-1, false)`:
  `flags &= 0xDF` (clear bMissionTrain, 0x6F6BB1).
- Verdict: REFUTED as a behavioural difference (only a flag is cleared; FindPlayerVehicle has no side effects, so
  iteration order is not observable).
- Class: harmless.
