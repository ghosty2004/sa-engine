# CGarages: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x4479a0 CGarages::IsCarSprayable  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Garages.cpp:280` (original-comment, in a TODO block): "In the source it compares against -2 ... original code also disallow artict1 to be sprayed (tested)"; gr then lists `case eModelID::MODEL_ARTICT1:`.
  - Original bytes: body 0x156b1c0. `if (veh->IsLawEnforcementVehicle() [0x6d2370]) return false`; `if (dword [veh+0x594] == 10 /*BMX*/) return false` (0x156b1d0); model `movsx [veh+0x22]`: returns false for 431 0x1af BUS (0x156b1e4), 407 0x197 FIRETRUK (0x156b1eb), 416 AMBULAN (`sub 9`, 0x156b1ed), 437 0x1b5 COACH (0x156b1f2) and -2 (0x156b1f9); otherwise true.
  - Verdict: CONFIRMED. The original compares the model against -2, not 435 (ARTICT1). In this function ARTICT1 is sprayable; the "tested" remark in gr must come from another check on the garage path. Port: `model == -2` (never matches a real model; effectively dead).
  - Class: keep-original.

## 0x447c40 CGarages::SetTargetCarForMissionGarage  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Garages.cpp:339` (fix-bugs-ifdef, lines 339-343): with FIX_BUGS gr calls `m_pTargetCar->CleanUpOldReference(&m_pTargetCar)` before clearing the target.
  - Original bytes: body 0x1560bd0. `vehicle == null` path (0x1560bd8-0x1560be9): `aGarages[id].m_pTargetCar (0x96c088 + id*0xd8) = 0`, return. No CleanUpOldReference call. Non-null path: store the pointer, `CEntity::RegisterReference(&m_pTargetCar)` (0x571b70), then `if (byte [garage+0xd] (door state) == 5) = 0` (0x1560c04-0x1560c0d). `garageId` is sign-extended from int16.
  - Verdict: CONFIRMED. The original leaves the vehicle's reference entry pointing at `m_pTargetCar` (it is nulled by the garage, but the reference stays registered).
  - Class: keep-original.

## 0x448b60 CGarages::Init_AfterRestart  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Garages.cpp:78` (todo): "TODO: Seems like inlined?" on the `aCarsInSafeHouse[..][..].Init()` loop.
  - Original bytes: the CStoredCar init is inlined as stores of `word 0` to `m_wModelIndex` (+0x12) of each stored car: outer loop over the car slot (`edx = 0x96abea`, += 0x40, until 0x96acea: 4 slots), inner loop over 20 safehouses (`eax += 0x100`, 20 iterations) (0x448b9f-0x448bd6). Garage loop before it: `for (u32 i = 0; i < NumGarages (0x96c024); i++) { type(+0x4c) = originalType(+0x4f); InitDoorsAtStart (0x447600); m_GarageAudio(+0x50).Reset() (0x4dc6b0); }`, NumGarages re-read each iteration; then `NoResprays (0x96c008) = 0`.
  - Verdict: REFUTED (the comment is right: it is inlined; gr's `Init()` only zeroes m_wModelIndex, and the loop order of independent stores does not matter).
  - Class: harmless.

## 0x44c8c0 CGarages::Update  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Garages.cpp:118` (original-comment): "Originally they've clearly used some kind of `abs` macro ..."; gr: `if (abs(left - cam.x) < 40 && abs(front - cam.y) < 40) TidyUpGarageClose(); else TidyUpGarage();`.
  - Original bytes: `d = cam.x - garage.m_fLeftCoord (+0x28)`; `a = (d < 0) ? -d : d` (ordered less picks `fchs`, NaN keeps d) (0x44c978-0x44c9b0); `if (a > 40.0f [0x858a10])` (ordered greater: `test ah,0x41; je`) -> TidyUpGarage (0x449c50); same with `cam.y - m_fFrontCoord (+0x30)` (0x44c9cf-0x44ca1b); otherwise TidyUpGarageClose (0x449d10). Both are tail jumps.
  - Verdict: CONFIRMED. The original treats `a == 40` and NaN as "close" (`!(a > 40)`), gr requires `a < 40`.
  - Class: keep-original.
- Found in passing (keep-original, CONFIRMED):
  1. Round-robin index. The original increments `LastUpdatedGarageId` (0x96ea78) FIRST, wraps it to 0 when `>= 50` (unsigned), and then checks the garage at the NEW index (0x44c938-0x44c952). gr checks `aGarages[LastUpdatedGarageId++]`, i.e. the old index.
  2. Garage loop: the original calls `CGarage::Update(i)` (0x44aa50) only for garages with `m_nType (+0x4c) != 0` (0x44c906-0x44c90b); gr calls it for all 50.
  3. Frame test: `(CTimer::m_FrameCounter & 0xF) == 12` (byte compare), same as gr's `% 16 != 12` for the unsigned counter. Entry: `CReplay::Mode == 1` or `CGameLogic::IsCoopGameGoingOn()` -> return; `bCamShouldBeOutside = 0`, `pOldToGarageWeAreIn = TheCamera.m_pToGarageWeAreIn (0xb6f844)`, both camera garage pointers (0xb6f844, 0xb6f848) = 0.
