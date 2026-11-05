# CVehicle: gta-reversed deviation notes (VEH, writer group E)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Vehicle.cpp` (pin 20d8d2a). RNG helpers of the original
(docs/re/cat14-gr-deviations.md finding 1): int 0x407180 `min + _ftol((rand() & 0xFFFF) * (1/32768.f) * (max-min))`,
float 0x41BD90 `(float)rand() * 3.0518509e-05f [0x858C7C] * (max - min) + min` (no 0xFFFF mask). Many sites inline
these formulas. gr's helper (`lerp` with `rand()/32767.f`) gives different values/rounding. Reversible bugfixes
(`notsa::bugfixes::*`) are runtime toggles; "toggle-off" below means the fix disabled.

## 0x6D1610 CVehicle::RemovePassenger  (gr_deviations = 1)
Row Vehicle.cpp:1423 (`assert(m_nNumPassengers > 0); // NOTSA: Sanity check`).
- Original bytes: return if passenger null. Train (`m_nVehicleType == 6`, checked in the split fragment 0x4074C3):
  scan all 8 seats; else scan `i < m_nMaxPassengers` (u8 re-read each step). On a match: `CleanUpOldReference`
  if the slot is non-null, slot = null, `m_nNumPassengers--` (u8 `dec`, wraps 0 -> 255, 0x6D168A).
- Verdict: CONFIRMED (gr asserts where the original silently wraps).
- Class: keep-original (no check).

## 0x6D2600 CVehicle::HeightAboveCeiling  (gr_deviations = 1)
Row Vehicle.cpp:1881 (original-comment about the `(fm - 1) <= 1` condition).
- Original bytes: `__thiscall float (float height, eFlightModel fm)`, ST0, `ret 8`. If `fm == 1 || fm == 2`
  (0x6D2604-0x6D260C): if `height < 500.0f` -> -1.0f; else if `height < 950.0f` -> `height - 500.0f`; else if
  `height < 1500.0f` -> -1.0f; else `(height - 1000.0f) + 500.0f`. Otherwise: `height < 800.0f ? -1.0f : height -
  800.0f`. NaN: in the RC branch NaN fails every "<" test and returns `(NaN - 1000) + 500` (gr returns -1.0f because
  it tests `height >= 500.f`).
- Verdict: REFUTED for the marked line (same condition); only the NaN path differs.
- Class: harmless (keep the "<" tests to be exact).

## 0x6D2A10 CVehicle::PickRandomPassenger  (gr_deviations = 1)
Row Vehicle.cpp:2014 (rng-helper `GetRandomNumberInRange(0u, 8)`).
- Original bytes: `r = _ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 8.0f [0x859000])` (0x6D2A15-0x6D2A33),
  then for `k = 0..7`: seat `(r + k) % 8` (signed modulo idiom), return the first non-null passenger; else null.
- Verdict: CONFIRMED (gr's helper formula differs).
- Class: keep-original.

## 0x6D2D50 CVehicle::AddWheelDirtAndWater  (gr_deviations = 1)
Row Vehicle.cpp:2115 (rng-helper in CreateDryFxForSurface).
- Original bytes: `if (WetRoads <= 0 || GetRandomNumberInRange(WetRoads, 1.01f) <= 0.5f)` with calls to 0x41BD90 at
  0x6D2EBA (dust) and 0x6D2F37 (sand), args (WetRoads 0xC81308, 0x3F8147AE = 1.01f).
- Verdict: CONFIRMED (helper formula). Class: keep-original.

## 0x6D3C70 CVehicle::RemoveWinch  (gr_deviations = 2)
Rows Vehicle.cpp:2561 (comment: original uses `&this[29]`) and :2564 (`NOTSA_UNREACHABLE("Unused function")`).
- Original bytes: `i = CRopes::FindRope((uint32)this + 0xA320)` (0x6D3C73; 0xA320 = 29 * 0x5A0); if `i >= 0`:
  `CRopes::aRopes[i].Remove()` (0x6D3C94, stride 0x328 at 0xB768B8); then `byte +0x4A8 &= 0x9F` (m_ropeType = 0).
  No callers in the catalogue.
- Verdict :2561: CONFIRMED (gr's `GetRopeID()` is `this + 0x1D`, the bytes use `this + 0xA320`).
- Verdict :2564: CONFIRMED (gr aborts; the original runs).
- Class: keep-original for both.

## 0x6D4900 CVehicle::SelectPlaneWeapon  (gr_deviations = 1)
Row Vehicle.cpp:2805 (comment: Tornado moved next to Hunter).
- Original bytes: `__thiscall (bool isFiring, eOrdnanceType type)`, `ret 8`; switch on model 425..576 (byte table
  0x6D4A2C): 425 hunter and 576 tornado (same code), 447 seaspar and 464 rcbaron, 476 rustler, 520 hydra; every
  case first stores `m_nVehicleWeaponInUse (+0x513) = isFiring ? 1 : current`, then hunter/tornado `type == 1 ->
  4`, rustler `type == 1 -> 2`, hydra `type == 1 -> 4, type == 2 -> 3`. Other models: no store.
- Verdict: REFUTED (same results; the intermediate store is not observable).
- Class: harmless.

## 0x6D4D30 CVehicle::FirePlaneGuns  (gr_deviations = 1)
Row Vehicle.cpp:2952 (NOTE: the second gun position is not recalculated in the original).
- Original bytes: `pos = GetPlaneGunsPosition(gunsCycle & 3)` (0x6D4E0D); `shell = m_matrix * pos + moveSpeed *
  timeStep`; `weapon.FireInstantHit(this, &shell, &shell, ...)` (0x6D4E85) BEFORE the FX. If double gun:
  `DoPlaneGunFireFX(&weapon, &pos, &shell, idx*2)` (0x6D4EB1); then the position IS recomputed:
  `pos2 = GetPlaneGunsPosition(idx)` with `pos2.x = -pos2.x` (fchs 0x6D4EDB), `shell2 = m_matrix * pos2 + vel`,
  `FireInstantHit(this, &shell2, &shell2)` (0x6D4F77), `DoPlaneGunFireFX(..., &pos2, &shell2, idx*2 + 1)`.
  Single gun: `DoPlaneGunFireFX(&weapon, &pos, &shell, idx)` after the shot.
- Verdict: CONFIRMED (gr fires both shots from the same point, and calls FX before FireInstantHit; the
  original mirrors x for the second gun and fires before the FX).
- Class: keep-original.

## 0x6D5490 CVehicle::ReactToVehicleDamage  (gr_deviations = 1)
Row Vehicle.cpp:2999 (rng-helper `GetRandomNumberInRange(2000, 5000)`).
- Original bytes: the look duration is drawn at function entry, before any null check:
  `t1 = 2000 - _ftol((rand() & 0xFFFF) * (1/32768.f) * -3000.0f)` (0x6D5496-0x6D54C2). Driver: if passenger 0 exists,
  `rand() < 0x3FFF` (signed, 0x6D54DD) -> look at passenger else at dmgCauser; without passenger -> dmgCauser.
  Then `t2` is drawn the same way (0x6D5539) whether or not there is a passenger; passenger: `rand() < 0x3FFF` ->
  look at the driver, else dmgCauser. LookAt args `("ReactToVhclDam" 0x871E68, ped, target, t, 5, null, false,
  0.25f, 500, 3, false)`.
- Verdict: CONFIRMED (helper formula and RNG call order: original draws the duration first and always draws
  t2; gr draws the coin flip first and only draws durations for existing peds). The coin-flip threshold itself
  (rand >= 16383 -> dmgCauser) matches gr's DoCoinFlip.
- Class: keep-original.

## 0x6D55C0 CVehicle::GetVehicleLightsStatus  (gr_deviations = 1)
Row Vehicle.cpp:3029 (comment: "the rest pretty much follows the original").
- Original bytes: `lights = 0`; `seed = byte(m_nRandomSeed) & 0x3F`; hour > 20 -> 1; hour == 20 and minutes >
  seed -> 1; hour < 6 -> 1; hour == 6 and minutes < seed -> 1; otherwise `thr = (float)(u16)m_nRandomSeed *
  1.9999999e-05f [0x871E78]`; `Foggyness > thr || WetRoads > thr` -> 1. Then (always) if
  `m_fContactSurfaceBrightness < 0.05f && CCullZones::CamNoRain()` -> 1. Finally, if lights and the driver is a gang
  ped (type 7..16) with `m_nRandomSeed & 1` and `CPopCycle::m_bCurrentZoneIsGangArea` -> return 0.
- Verdict: CONFIRMED - gr computes the threshold as `seed / 50000.f` (division) while the bytes multiply by the
  f32 constant 1.9999999e-05f; results can differ in the last bit. The early gang return is equivalent.
- Class: keep-original (multiply by [0x871E78]).

## 0x6D56C0 CVehicle::ProcessOpenDoor  (gr_deviations = 2)
Rows Vehicle.cpp:840 (ROLLOUT default `NOTSA_UNREACHABLE`) and :875 (ROLLDOOR default).
- Original bytes: the function starts with `push ecx`, so the local at [entry-4] holds `this`. ROLLOUT (anims
  384/385, 0x6D59FF): group table 0x6D5CA8/0x6D5CB0 over `animGroup - 88` (0..25): groups 88, 89, 90, 102, 103, 107,
  108, 110, 113 -> start 0.01f, end 0.1f; any other group (or > 113) -> no assignment: start = the raw bits of the
  `animGroup` argument slot reinterpreted as f32, end = the local = `this` pointer bits as f32 (0x6D5A29). ROLLDOOR
  (anim 386, 0x6D5A90, table 0x6D5CCC/0x6D5CD4, same groups) -> constants 0.05f, 0.2f, 0.3f, 0.475f into the slots
  [animGroup], [local], [doorComponentId], [animId]; other groups keep those slots' integer bit patterns.
- Verdict :840: CONFIRMED. Verdict :875: CONFIRMED (gr aborts; the original continues with garbage thresholds).
- Class: keep-original (reinterpret the integer arguments / `this` as f32 thresholds).
- Note: the ROLLDOOR block writes four values (0.05, 0.2, 0.3, 0.475); gr names only three - a port must take the
  comparisons from the bytes (0x6D5ACA onward), not from gr.

## 0x6D6C00 CVehicle::ProcessWheel  (gr_deviations = 3)
Rows Vehicle.cpp:3158 (FIX_BUGS remove statics), :3161 (rbf alreadySkidding), :3189 (rng-helper burst).
- Original bytes: three global bytes: braking 0xC1CDAE, driving 0xC1CDAD, alreadySkidding 0xC1CDAC. braking and
  driving are rewritten on every call (0x6D6C4C-0x6D6C74). alreadySkidding is only ever SET (`= 1` at 0x6D6CAA when
  the incoming wheel state != 0) and never cleared, so it starts 0 (.bss) and stays 1 after the first skid of the
  session. Traction loss (0x6D6FB6-0x6D7008): `t = alreadySkidding ? 1.0f : h->m_fTractionLoss`; if
  `!alreadySkidding && state == 1 (SPINNING) && status in {0, 8}`: `t *= 1 - |gas| * WS_ALREADY_SPINNING_LOSS`;
  then `t = t / sqrt(slipSq) * adhesion`. Burst tyre: `rand() * (1/32767.f) [0x858C7C] * (max - min) + min` with
  `min = -fBurstTyreMod, max = fBurstTyreMod` inlined (0x6D6D76-0x6D6D9C).
- Verdict :3158: CONFIRMED (the static alreadySkidding is observable; braking/driving statics are not).
- Verdict :3161: CONFIRMED. With the toggle off gr uses a constant `true`, which equals the original only after the
  first skid of the session; with the toggle on it resets per call. Additional gr difference (not tagged): when
  `!alreadySkidding` and the state is not SPINNING the original keeps `t = m_fTractionLoss` while gr uses 1.0f;
  and gr computes `trLoss *= adhesion / sqrt(slipSq)` (other rounding order).
- Verdict :3189: CONFIRMED (helper formula).
- Class: keep-original (model 0xC1CDAC as a persistent global).

## 0x6D73B0 CVehicle::ProcessBikeWheel  (gr_deviations = 3)
Rows Vehicle.cpp:3283 (rbf), :3297 (bugfix-comment "doublefix"), :3308 (rng-helper).
- Original bytes: own statics 0xC1CDAF (alreadySkidding, sticky: `= 1` when state != 0 at 0x6D745C, never cleared),
  0xC1CDB0/B1/B2 (rewritten each call). `if (alreadySkidding) adhesion *= m_fTractionLoss` (0x6D7480-0x6D748C);
  skid branch `t = alreadySkidding ? 1.0f : m_fTractionLoss; t = t / sqrt(slipSq) * adhesion` (0x6D77A7-0x6D77CA);
  burst RNG inlined with 0x858C7C at 0x6D7543.
- Verdict :3283: CONFIRMED (same sticky-static issue as ProcessWheel). Verdict :3297: CONFIRMED (gr's "double fix"
  is only equal after the first skid). Verdict :3308: CONFIRMED (helper formula).
- Class: keep-original.

## 0x6D7C90 CVehicle::InflictDamage  (gr_deviations = 1)
Row Vehicle.cpp:3408 (`IncrementStat(STAT_COST_OF_PROPERTY_DAMAGED, GetRandomNumberInRange<float>(5, 25))`).
- Original bytes: `IncrementStat(125, (float)(rand() % 20 + 5))` (0x6D7D63-0x6D7D87: signed idiv by 20, +5, fild) -
  an integer 5..24.
- Verdict: CONFIRMED (gr uses a continuous float in [5, 25]).
- Class: keep-original.

## 0x6D82F0 CVehicle::KillPedsGettingInVehicle  (gr_deviations = 1)
Row Vehicle.cpp:3530 (pool-iteration).
- Original bytes: ped pool walked from the highest slot down (0x6D8330-0x6D8452). Skip if ped flag bInVehicle
  (+0x46C bit 8) OR `CEntity::m_nFlags (+0x1C) bit 0` (m_bUsesCollision) is set (0x6D8360). Task: active
  task 701 else 700 (`FindActiveTaskByType`), target vehicle `+0xC` must be this. Damage: calculator (ped, 1000.0f,
  51, 3, false), event (ped, now, 51, 3, 0, false, bInVehicle).
- Verdict: CONFIRMED (iteration order). Additional gr logic error (not tagged): gr skips `bIsStanding` peds, the
  bytes skip peds with `m_bUsesCollision` set.
- Class: keep-original.

## 0x6DAF00 CVehicle::BladeColSectorList<CPtrListSingleLink<CEntity*>>  (gr_deviations = 3)
Rows Vehicle.cpp:4008 (`assert(rotorType == -3)`), :4016 (`NOTSA_UNREACHABLE("Unknown rotorType")`), :4170 (rng).
- Original bytes: switch `rotorType + 3` (0..6, table 0x6DBA0C) handles all six directions -3..3 except 0;
  rotorType 0 or out of range -> 0x6DB0A9 with the thickness vector left (0,0,0) (zeroed at 0x6DAF38) and the
  rotor-up vector locals not written. Damage timer: `planeRotorDmgTimeMS (0xC1CC1C) = now +
  GetRandomNumberInRange(int)([0x8D34F0], [0x8D34F4])` (0x407180 call at 0x6DB9C4; bounds are `.data` variables,
  image values 150/250).
- Verdict :4008: CONFIRMED (the original supports every direction). Verdict :4016: CONFIRMED (no abort).
  Verdict :4170: CONFIRMED (helper formula).
- Class: keep-original. Open question: the uninitialised rotor-up locals for rotorType 0 (0x6DB0A9 path) - not
  reachable from known callers (gr comment: rotorType is always -3).

## 0x6DBA30 CVehicle::SetComponentRotation  (gr_deviations = 1)
Row Vehicle.cpp:4193 (`default: NOTSA_UNREACHABLE()`).
- Original bytes: `CMatrix m(attach frame+0x10)`; `saved = m.pos`. If `bSetRotate`: axis 2/1/0 -> `SetRotateZ/Y/X
  (angle)` (0x59B0E0/0x59B0A0/0x59B060); other axis -> nothing. Else: `m.pos = m.pos + (-m.pos)` (x, y, z), axis 2/1/0
  -> `RotateZ/Y/X(angle)` (0x59B390/0x59B2C0/0x59B1E0); other -> nothing. Then `m.pos += saved`, `UpdateRW()`.
  So an unknown axis with bSetRotate doubles the translation; without it leaves the matrix as it was.
- Verdict: CONFIRMED (gr aborts). Further note: gr calls `SetRotateXOnly` / `RotateX(angle, true)`, the bytes call
  `SetRotateX` / `RotateX` with the explicit position save/zero/restore shown above - check those CMatrix
  functions before assuming equality.
- Class: keep-original.

## 0x6DD130 CVehicle::DoBoatSplashes  (gr_deviations = 7)
Rows Vehicle.cpp:4592, 4608, 4609, 4610, 4616, 4617, 4618 (rng-helper).
- Original bytes: seven calls to 0x41BD90 in the same order as gr: 0x6DD2E0 (life), 0x6DD4BC, 0x6DD4F8 (0.3, 0.7),
  0x6DD529 (0.8, 1.2), 0x6DD5FC, 0x6DD638 (0.3, 0.7), 0x6DD669 (0.8, 1.2).
- Verdict: CONFIRMED for all seven (helper formula).
- Class: keep-original.

## 0x6DDF60 CVehicle::AddWaterSplashParticles  (gr_deviations = 2)
Rows Vehicle.cpp:4742, 4743 (rng-helper `GetRandomNumberInRange(0.f, 1.f)`).
- Original bytes: `t1 = (float)rand() * 3.0518509e-05f` (0x6DE108-0x6DE11B, stored f32) then `t2` the same
  (0x6DE11F-0x6DE138); `pos = v0 + (v1 - v0) * t1 + (v2 - v1) * t2`. Loop count `1 - _ftol(Rain * -2.0f)`.
- Verdict: CONFIRMED for both (gr's t = rand()/32767.f via lerp; the bytes multiply by the f32 reciprocal).
- Class: keep-original.

## 0x6DE240 CVehicle::AddExhaustParticles  (gr_deviations = 2)
Rows Vehicle.cpp:4795 (static randomFactor), :4816 (`GetRandomNumberInRange(1.0f, 3.0f) * (gas + 1.1f) <= 2.5f`).
- Original bytes: static guard bit 0 of 0xC1CDB8; `[0xC1CDB4] = GetRandomNumberInRange(-1.8f, -0.9f)` (0x41BD90 at
  0x6DE455) on first use. Second site inlined: `r = (float)rand() * 3.0518509e-05f; v = (r + r) + 1.0f` then
  `* (m_GasPedal + 1.1f)` (0x6DE56D-0x6DE58E).
- Verdict :4795: CONFIRMED (helper formula; the one-time static matches). Verdict :4816: CONFIRMED (formula
  `2r + 1` vs gr's lerp).
- Class: keep-original.

## 0x6DE880 CVehicle::AddSingleWheelParticles  (gr_deviations = 4)
Rows Vehicle.cpp:4931 (`NOTSA_UNREACHABLE_CASE(wheelIndex)`), :4948, :4967, :4968 (rng-helper).
- Original bytes: there is no switch on wheelIndex; the bytes only test `wheelIndex & 1` (0x6DEC00) and
  `wheelIndex < 2` (signed, 0x6DEC66, 0x6DEECA, 0x6DF106, 0x6DF339). RNG: 0x41BD90 at 0x6DF26D `(0, |gas| *
  moveSpeed.x * -30)`, 0x6DF28B `(0, |gas| * moveSpeed.y * -30)`, then `rand() % 10000 * 0.0001f * 0.8f` (0x6DF294),
  then colour `GetRandomNumberInRange(0.5f, 1.0f)` (0x6DF2C3; also at 0x6DEE0A, 0x6DF04A).
- Verdict :4931: CONFIRMED (gr aborts for indices outside its case list and classifies wheels 5/6 differently from
  `index & 1` / `index < 2`). Verdicts :4948, :4967, :4968: CONFIRMED (helper formula; call order matches).
- Class: keep-original.

## 0x6E0400 CVehicle::ScanAndMarkTargetForHeatSeekingMissile  (gr_deviations = 2)
Rows Vehicle.cpp:5129 and :5146 (rbf NotProperlyStashedLOSTimestamps).
- Original bytes: `last = (subtype == 4 /*plane*/) ? ((dword[+0x9E4] & 0x7FFFFFFF) == 0) : 0` (0x6E0464-0x6E0477,
  a 0/1 value); LOS test only if `target && now - last > 1000` (unsigned); after the test the plane stores
  `(1 < now)` into the 31-bit time field and the LOS result into bit 31 (0x6E0515-0x6E0532).
- Verdict :5129 and :5146: REFUTED with the toggle off (gr's toggle-off expressions `== 0` and `> 1` are exactly the
  bytes); with the toggle on gr differs.
- Class: keep-original (port must use the toggle-off form).

## 0x6E1A60 CVehicle::DoVehicleLights  (gr_deviations = 3)
Rows Vehicle.cpp:5550, :5555 (notsa step_up_to / step_down_to for the ZR350 pop-ups), :5567 (rbf light RR).
- Original bytes (ZR350 = model 477, status != 5, bEngineOn): up if `bLightsOn || forceOn || IsAlarmActivated
  (0x4F4CE0)`: `p = p + timeStep * 0.01f`, `if (0.6981317f < p) p = target` (NaN keeps NaN), store, return if
  `p < target` (0x6E1C8E-0x6E1CC8). Down: only if `p > 0`: `p = max(0, p - timeStep * 0.01f)` via 0x420800
  (0x6E1C53-0x6E1C83). Light-status calls (0x6E1D02, 0x6E1D31, 0x6E1D56, 0x6E1D83) use lights 1, 0, 3, 3.
- Verdict :5550: REFUTED (same for finite values; std::min returns `target` for a NaN p).
- Verdict :5555: REFUTED (the original skips the update when p <= 0; gr would clamp a negative p to 0 - p is never
  negative).
- Verdict :5567: REFUTED with the toggle off (light 3 used for both rear lights = bytes).
- Class: harmless (5550, 5555), keep-original (5567: do not enable the fix).

## 0x6E2900 CVehicle::FillVehicleWithPeds  (gr_deviations = 1)
Row Vehicle.cpp:5670 (`BUGFIX: Added !m_pDriver`).
- Original bytes: optional afro setup (stat 21 = 1000, clothes "afro", RebuildPlayer). Model = 0 (afro) or 95.
  Loop `slot = -1 .. m_nMaxPassengers-1`: driver slot is skipped if `m_pDriver && m_pDriver == Players[0].m_pPed`
  (0x6E2975-0x6E2981, null-safe). Per slot: if the model is not loaded -> `RequestModel(model, 8)` and continue
  with the next slot (no early return). Else `ped = CPopulation::AddPed(5, model, pos, false)`; the existing
  occupant of the slot is DELETED via its virtual destructor (0x6E29D3 / 0x6E29F4); driver:
  `SetPedInCarDirect(ped, this, 0, true)`, passenger i: `SetPedInCarDirect(ped, this,
  ComputeTargetDoorToEnterAsPassenger(this, i), false)`.
- Verdict: REFUTED for the marked line (the original already handles a null driver). Other differences (not
  tagged): gr nulls the old occupants instead of deleting them, returns early when the model is not loaded,
  compares with `IsPlayer()` instead of Players[0], and passes `true` as the last SetPedInCarDirect argument for
  passengers.
- Class: keep-original.
