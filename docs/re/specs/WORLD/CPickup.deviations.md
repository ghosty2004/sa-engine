# CPickup: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x4549a0 CPickup::GetPosn  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Pickup.h:72` (notsa): `CVector2D GetPosn2D() const { return GetPosn(); }; // NOTSA`.
  - Original bytes: 0x4549a0 is GetPosn itself (returns CVector by hidden pointer, `ret 4`): decompresses the int16 triple at +0x10/+0x12/+0x14 with `fild` and `fmul 0.125 [0x858c48]` (z, y, x loaded in that order, 0x4549a1-0x4549d5), stores x,y,z to the result (0x4549db-0x4549e0).
  - Verdict: REFUTED. The flagged line is a gr-only 2D helper declared next to GetPosn; GetPosn in gr (`return m_vecPos` via CompressedLargeVector, scale 1/8) matches the bytes.
  - Class: harmless.

## 0x455540 CPickup::FindStringForTextIndex  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Pickup.cpp:72` (notsa-unreachable): `default: NOTSA_UNREACHABLE();` (cases CAN_BUY=1 "PROP_3", CANT_BUY=2 "PROP_4", CANCEL=0 "FESZ_CA").
  - Original bytes: body 0x1565750. `eax = index; dec; je` -> 1 returns 0x859b3c "PROP_3" (0x1565766); `dec; je` -> 2 returns 0x859b34 "PROP_4" (0x1565760); every other value (0 and anything else) returns 0x859b44 "FESZ_CA" (0x156575a).
  - Verdict: CONFIRMED. The original has no unreachable path: any index other than 1 or 2 yields "FESZ_CA".
  - Class: keep-original (only reachable with an out-of-range index; the 3-bit flag field allows 3..7).

## 0x457410 CPickup::Update  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 3, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Pickup.cpp:282` (pool-iteration): nautical mine (inactive): `for (auto& vehicle : GetVehiclePool()->GetAllValid()) if (IsSphereTouchingVehicle(objPos, 2.0f)) isAnyVehicleTouching = true;`
- gr `Pickup.cpp:302` (pool-iteration): armed mine: same with radius 1.5f setting `explode`.
  - Original bytes: both loops are DESCENDING over the vehicle pool (0xb74494; `dec ebp` first, 0x4583c9 and 0x458503), skip free/null, `CVehicle::IsSphereTouchingVehicle(veh, pos.x, pos.y, pos.z, 1.5f / 2.0f)` (0x6d84d0, position components passed by value from the object's matrix or placement); armed mine: `explode = (CTimer::m_snTimeInMilliseconds > m_nRegenerationTime (+0xc))` (unsigned, 0x45838e-0x458398) OR any touch.
  - Verdict (both): CONFIRMED order differs, but the loop only ORs the result of a side-effect-free test, so the outcome is identical.
  - Class: harmless.
- gr `Pickup.cpp:559` (original-comment): `FindPlayerInfo().m_nMoney += m_nAmmo; // originally player 0` (PICKUP_MONEY / MONEY_DOESNTDISAPPEAR).
  - Original bytes: `add dword [0xb7ce50], edx` with `edx = pickup->m_nAmmo (+8)` (0x458054-0x45805b): always `Players[0].m_nMoney`. (Asset revenue, gr line 565, uses the player index as gr does: `Players[playerId]`, 0x4580a4.)
  - Verdict: CONFIRMED. The original pays player 0; gr pays `Players[PlayerInFocus]`.
  - Class: keep-original.
