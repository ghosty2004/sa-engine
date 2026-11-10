# CTaskSimpleCarDrive: gta-reversed deviation notes (AI, group I2)

## 0x63C500 CTaskSimpleCarDrive::TriggerIK  (gr_deviations = 1)
Row TaskSimpleCarDrive.cpp:79 (rng-helper: `if (g_ikChainMan.IsLooking(ped) || CGeneral::GetRandomNumberInRange(0, 100) >= 5) return;`
placed before the mission switch).
- Original bytes: if `m_pVehicle (+8) == 0` return; `mission = veh->m_autoPilot.m_nCarMission (+0x3BA, u8)`; only for mission in
  {2, 3, 4, 5, 6} (player ram/block missions, 0x63C514-0x63C537) or {0xF, 0x10} (ram-car missions) does it test
  `g_ikChainMan.IsLooking(ped)` and then draw `CGeneral::GetRandomNumberInRange(0, 100)` 0x407180 (`>= 5` -> return), e.g.
  0x63C549-0x63C56F for the ram-car branch; any other mission returns before IsLooking and before drawing.
- Verdict: CONFIRMED, two differences: (1) gr's GetRandomNumberInRange formula differs from 0x407180 (cat14 finding 1); (2) gr
  consumes a rand() on every call, the original only for those seven missions - RNG stream divergence. Class: keep-original.
