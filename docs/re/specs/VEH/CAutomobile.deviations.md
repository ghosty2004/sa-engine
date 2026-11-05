# CAutomobile: gta-reversed deviation notes (VEH, group C)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Automobile.cpp` (pin 20d8d2a). Layout and helper notes: see
CAutomobile.md (group C header). RNG: the original either calls CGeneral::GetRandomNumberInRange<float> 0x41BD90
(`(max - min) * (rand() * 3.0518509e-05f) + min`) or inlines `(rand() & 0xFFFF) * (1/32768.f) * k` with `_ftol`;
gr's helpers use `lerp` (docs/re/cat14-gr-deviations.md finding 1).

## 0x6A2390 CAutomobile::TellHeliToGoToCoors  (gr_deviations = 1)
Convention: `__thiscall void (this, f32 x, f32 y, f32 z, f32 altitudeMin, f32 altitudeMax)`, `ret 0x14`.
Row Automobile.cpp:3910 (todo "Is dis some inlined function?" on `m_fAircraftGoToHeading -= TWO_PI`).
- Original bytes: stores `x,y,z` to +0x3EC/+0x3F0/+0x3F4 (destination), `altitudeMin` -> +0x9B0
  (CHeli::m_fMinAltitude), `altitudeMax` -> +0x9A8 (m_fMaxAltitude), mission byte +0x3BA = 0x14, cruise speed +0x3D0
  = 100, status = 3 (`(byte[0x36] & 7) | 0x18`). Then `if (m_fAircraftGoToHeading (+0x8A8) == 0.0f)` (`test ah,0x44;
  jp` skips on != or NaN): `h = GetATanOfXY(m->forward.x, m->forward.y) + 3.1415927f [0x858CB8]`, stored f32 (fst);
  `if (h > 6.2831855f [0x858CBC])`: reload h as f32, then `do h -= 6.2831855f; while (h > 6.2831855f)` in x87
  (0x6A2430-0x6A2441), store f32. Nothing else: there is no final `-= TWO_PI`.
- Verdict: CONFIRMED. gr subtracts TWO_PI once more after the wrap (and also when no wrap was needed), giving a heading
  in (-2pi, 0]; the original keeps it in (0, 2pi] (just atan + pi when that is <= 2pi).
- Class: keep-original.

## 0x6A2530 CAutomobile::SetRandomDamage  (gr_deviations = 6)
Full behaviour: CAutomobile.md. Rows Automobile.cpp:3956 ("Bugfix shit"), :3962 (`#ifdef FIX_BUGS`
`GetRandomNumberInRange(x, y + 1)`), :3963/:3965 (rng-helper in FixedRandom), :3999 (`NOTSA_UNREACHABLE` in
GetNthUnpicked), :4003 (rng-helper `GetRandomNumberInRange(0, numToDmg - i)`).
- Original bytes: counts at 0x6A2571-0x6A2638 (`_ftol(R*6)`, `1 - _ftol(R*-3)`, `_ftol(R*4)`, and for
  bRemoveStuff == false `_ftol(R)` = 0 then panels = 1); per-pick `_ftol(R * (6 - i))` (0x6A265E-0x6A2677) and
  `_ftol(R * remaining)`, remaining = 4, 3, ... (0x6A2742-0x6A274C, 0x6A27D0); scans fall out to index 6 / 7
  instead of aborting (0x6A2690-0x6A26A1, 0x6A2760-0x6A2771).
- Verdict :3956/:3962: CONFIRMED (FIX_BUGS widens every count range by one; the original ranges are the ones above).
- Verdict :3963/:3965/:4003: CONFIRMED (helper formula; and :4003 also uses the wrong upper bound `numToDmg - i`
  instead of the remaining-candidate count - a gr logic error independent of FIX_BUGS).
- Verdict :3999: CONFIRMED (original: no abort, index 6/7 used), unreachable with the original counts.
- Class: keep-original (all).
Found in passing (CONFIRMED, keep-original): panels are set to status 1 (`push 1` at 0x6A27AD), gr uses
DAMSTATE_DAMAGED (2).

## 0x6A27F0 CAutomobile::SetTotalDamage  (gr_deviations = 2)
Convention: `__thiscall void (CAutomobile* this, bool randomness [esp+4])`, `ret 4`.
Rows Automobile.cpp:4052 and :4074 (rng-helper `!randomness || GetRandomNumberInRange(0, 3)`, doors and panels).
- Original bytes: per door d = 0..5 with a non-null `m_aCarNodes[GetCarNodeIndexFromDoor(d)]`: if randomness and
  `_ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f * 3.0f [0x858B3C]) == 0` (0x6A2822-0x6A2847): `SetDoorStatus(d, 4)`,
  `SetComponentVisibility(frame, 0)`; else if `GetDoorStatus(d) == 0` and the damageable-mask bit is set:
  `SetDoorStatus(d, 2)`, `SetComponentVisibility(frame, 2)`. Panels p = 0..6 except 2 and 3, same pattern with
  status 3 / visibility 0 on the roll (0x6A28E7-0x6A291C), status 1 / visibility 2 otherwise. One rand() per present
  component when randomness is set.
- Verdict (both rows): CONFIRMED. Original removes the part when `rand() & 0xFFFF < 10923` (probability ~1/3);
  gr removes it when `(int)lerp(0, 2, rand()/32767.f) == 0`, i.e. rand() <= 16383 (~1/2). Control flow, statuses and
  visibility values otherwise match gr.
- Class: keep-original.

## 0x6A29C0 CAutomobile::ProcessControlCollisionCheck  (gr_deviations = 1)
Convention: `__thiscall void (CAutomobile* this, bool applySpeed [esp+4])`, virtual slot 23, `ret 4`.
Row Automobile.cpp:1890 ("Save original matrix (We need it later)").
- Original bytes: local `CMatrix saved(*m_matrix)` (0x59BCF0 copy ctor, 0x6A29E0-0x6A29E5); clear entity flag 0x10
  (bIsStuck); `SkipPhysics()`; physicalFlags &= ~0x9000; `m_fMovingSpeed (+0xD4) = 0`; compressions[0..3] = 1.0f.
  applySpeed: `ApplyMoveSpeed(); ApplyTurnSpeed();` then `n = 0; while (CheckCollision() && n++ < 5)`
  `{ *m_matrix = saved; ApplyMoveSpeed(); ApplyTurnSpeed(); }` (16-bit counter compare `cx < 5` before the increment,
  0x6A2A50-0x6A2A7E: at most 6 CheckCollision calls). Else: save flag bit 0 (bUsesCollision), clear it,
  `CheckCollision()`, restore bit 0. Finally flags: clear 0x10, set 0x20 (bIsInSafePosition); destroy `saved`.
- Verdict: REFUTED (gr's `for (tr = 0; CheckCollision() && tr < 5; tr++)` has the same call sequence and count).
- Class: harmless.

## 0x6A32B0 CAutomobile::BurstTyre  (gr_deviations = 2)
Full behaviour: CAutomobile.md. Rows Automobile.cpp:2521 and :2523 (rng-helper `GetRandomNumberInRange(-force, force)`).
- Original bytes: calls 0x41BD90 with (-[0x8D3168], [0x8D3168]) at 0x6A337C and 0x6A33C7; result multiplied by the
  mass / turn mass before the vector multiply (0x6A3381, 0x6A33CC).
- Verdict: CONFIRMED (both): different interpolation formula and multiplication order.
- Class: keep-original.

## 0x6A44D0 CAutomobile::PopBootUsingPhysics  (gr_deviations = 2 rows, 1 counted)
Convention: `__thiscall void (CAutomobile* this)`, plain `ret`.
Rows Automobile.cpp:4499 ("UNUSED - Likely inlined?") and :4505 ("Original code: ... SetDoorStatus(BOOT, OPENED);
[[fallthrough]]").
- Original bytes: `s = m_damageManager.GetDoorStatus(1 /*BOOT*/)` (0x6C2230). s == 0 (OK):
  `SetDoorStatus(1, 1 /*OPENED*/)` then falls through to `SetDoorStatus(1, 3 /*OPENED_DAMAGED*/)` (0x6A44FA-0x6A450B).
  s == 2 (DAMAGED): `SetDoorStatus(1, 3)`. Any other s: no status change. In every case
  `m_doors[1].m_angVel (+0x5E4) = -2.0f`.
- gr: DAMAGED -> return without touching m_angVel; every other status (OK, OPENED, OPENED_DAMAGED, NOTPRESENT) ->
  SetDoorStatus(BOOT, OPENED_DAMAGED); then m_angVel = -2.
- Verdict :4499: REFUTED (comment only; no callers in the catalogue).
- Verdict :4505: CONFIRMED - gr has the switch cases swapped: the original sets OPENED_DAMAGED only for OK and DAMAGED
  and sets `m_angVel = -2` for every status, including DAMAGED; for OPENED / OPENED_DAMAGED / NOTPRESENT it leaves the
  status alone. Also the intermediate SetDoorStatus(BOOT, OPENED) call for OK is omitted in gr (SetDoorStatus may have
  side effects, see CDamageManager::SetDoorStatus 0x6C21C0).
- Class: keep-original.

## 0x6A47F0 CAutomobile::DoBurstAndSoftGroundRatios  (gr_deviations = 1)
Convention: `__thiscall void (CAutomobile* this)`, virtual slot 52 (+0xD0), plain `ret` (ebp frame, 8-aligned).
Row Automobile.cpp:2718 (rng-helper `GetRandomNumberInRange(0, int32(speedToFwdRatio * 40.0f) + 98)`).
- Original bytes: `ratio = fabs((moveSpeed.z*fwd.z + moveSpeed.y*fwd.y) + moveSpeed.x*fwd.x)` stored f32
  (0x6A4802-0x6A4839). For a wheel with status 1 (BURST) (0x6A486B): `a = (float)(rand() & 0xFFFF) * 3.0517578e-05f`
  kept on the x87 stack; `n = (u16)_ftol(ratio * 40.0f [0x858A10]) + 98` (`movzx eax, ax` keeps only the low 16 bits
  of the truncated product, 0x6A4888-0x6A489A); `val = _ftol(a * n)` (`fimul`, 0x6A48A1-0x6A48A5). If `val < 100`:
  `comp[i] = (line[i] - spring[i]) / line[i] * 0.25f + comp[i]` (fld [+0x888]; fsub [+0x878]; fdiv [+0x888]; fmul
  0.25f [0x8710B0]; fadd comp; fst), and if that is > 1.0f, `comp[i] = 1.0f` (0x6A48B3-0x6A48DF).
- Verdict: CONFIRMED (gr: `(int)lerp(0, n-1, rand()/32767.f)`; original `trunc((rand&0xFFFF)/32768 * n)`; the burst
  roll fires for different rand() values. gr also uses a 32-bit `int32(ratio*40)` where the original keeps 16 bits).
- Class: keep-original.

## 0x6A65D0 CAutomobile::SetupSuspensionLines  (gr_deviations = 3 rows, 1 counted)
Convention: `__thiscall void (CAutomobile* this)`, virtual slot 48, plain `ret`. Locals 0x5C. CColLine stride 0x20
(start +0x00, end +0x10).
Row Automobile.cpp:2568 (`m_wheelPosition[i] = wheelPos.z; // Originally set earlier in the loop`).
- Original bytes: `m_wheelPosition[i] (+0x838) = wheelPos.z` right after GetWheelPosn and the quad x-offset
  (0x6A6685-0x6A668D), before z is modified. Same value as gr.
- Verdict: REFUTED. Class: harmless.
Row :2593 ("This was probably inlined?", bbox min.z adjust).
- Original bytes: `if (lines[0].end.z < cm->bbox.min.z) cm->bbox.min.z = lines[0].end.z` (0x6A681C-0x6A6833, `jp` keeps on
  >= / NaN). Same as gr. Verdict: REFUTED. Class: harmless.
Row :2599 ("most likely inlined", bounding radius).
- Original bytes: `r = (|min| > |max|) ? |min| : |max|` (each `sqrt((x*x + y*y) + z*z)` in x87, ties/NaN -> |max|),
  then `if (r > radius) radius = f32(r)` (0x6A6836-0x6A68B4). gr `std::max({radius, |min|, |max|})` gives the same
  value except for NaN ordering and that gr rounds each magnitude to f32 before comparing.
- Verdict: REFUTED (no observable difference for finite data). Class: harmless.

Found in passing (CONFIRMED, keep-original):
1. Line allocation is keyed on the pointer: `if (cd->m_pLines (+0x10) == nullptr)` -> `m_nNumLines = (model == 0x1B0 RHINO)
   ? 12 : 4`, `m_pLines = CMemoryMgr::Malloc(numLines * 0x20)`, `hadToAllocate = true` (0x6A65F6-0x6A6629). gr tests
   `!cd.m_nNumLines` and calls `AllocateLines`.
2. Line end z is computed from the already rounded start z: `start.z = f32(z + upper)`;
   `end.z = f32(((lower - upper) - size*0.5f) + start.z)` (0x6A66D8-0x6A670B). gr: `z + lower - size/2`.
3. Heights above road (0x6A676D-0x6A67E3): `front = (1 - 1/(force*4)) * spring[0] + (mi->front*0.5f - lines[0].start.z)`;
   `rear = (1 - 1/(force*4)) * spring[3] (+0x884) + (mi->rear*0.5f - lines[3].start.z (+0x68))`: the original uses wheel 3
   (rear right) for the rear height and adds the bracketed term last. gr uses wheel 1 (REAR_LEFT) and evaluates
   `(a*spring + size/2) - start.z`.
4. Quad x offset: `if (x > 0) x += 0.15f else x -= 0.15f` (NaN subtracts); gr `x <= 0 ? -= : +=` (NaN adds).
5. `GetVehicleAppearance()` is called once or twice (0x6A675C-0x6A6768) with the result unused; gr omits the calls
   (harmless if the callee has no side effects).

## 0x6A7650 CAutomobile::VehicleDamage  (gr_deviations = 6 counted, 11 rows)
Convention: `__thiscall void (CAutomobile* this, f32 damageIntensity, u16 collisionComponent, CEntity* damager,
CVector* collisionCoors, CVector* collisionDirection, eWeaponType weapon)`, virtual slot 56 (+0xE0), `ret 0x18`.
Rows :2808, :2814, :3003 (asserts), :2937 and :3180 (NOTSA_LOG_DEBUG): REFUTED, harmless (no code in the original).

Row Automobile.cpp:2944 ("R* used magnitude, but squared magnitude is more suitable").
- Original bytes (0x6A79F8-0x6A7A54), law enforcer, damager == FindPlayerVehicle(), status != 4: `a = |this->moveSpeed|`
  (CVector::Magnitude 0x4082C0, stored f32); `b = |playerVeh->moveSpeed|`; continue only if NOT `b < a` (NaN fails);
  then `|playerVeh->moveSpeed| > 0.1f` [0x858B1C] (FindPlayerVehicle and Magnitude called again);
  `FindPlayerPed(-1)->SetWantedLevelNoDrop(1)`. FindPlayerVehicle(-1, false) is called 4 times in this block.
- Verdict: CONFIRMED (gr compares squared magnitudes against `sq(0.1f)`; boundary/rounding differ). Class: keep-original.

Rows :2996 (`#ifdef FIX_BUGS` divide by half the bbox width), :3001 (comment), :3002 (bug-note).
- Original bytes: the side factor is `dot(coll - pos, right) / GetColModel()->bbox.max.x` (`fdiv [eax+0xC]` at
  0x6A7C89 and 0x6A7EDE), i.e. gr's `#else` branch.
- Verdict :2996: CONFIRMED (gr's FIX_BUGS divides by `(max.x - min.x) / 2`). :3002: CONFIRMED (same block).
  :3001: REFUTED (comment only).
- Class: keep-original (divide by max.x).

Row :3092 (NOTSA `case DEFAULT / WING_LR / WING_RR: break`).
- Original bytes: `switch (component - 1)` with byte index table 0x6A84E8 and jump table 0x6A84B8 (0x6A7BA9-0x6A7BC1);
  values > 0x12 and the unused entries go to 0x6A8044 (no damage). Same as gr's explicit no-op cases.
- Verdict: REFUTED. Class: harmless.

Row :3148 (`default: NOTSA_UNREACHABLE()` in the driver crash speech switch).
- Original bytes (0x6A8227-0x6A8254): `t = m_vehicleAudio.GetVehicleTypeForAudio()` (sign-extended 16-bit);
  `t == 0 -> Say(0x43)`, `t == 1 -> Say(0x42)`, any other value -> `Say(0x44)`; Say args `(id, 0, 0.66f, 0, 0, 0)`.
- Verdict: CONFIRMED (gr aborts on values other than its three cases; the original treats every other value as the
  third case). Class: keep-original.

## 0x6A9680 CAutomobile::ProcessHarvester  (gr_deviations = 5)
Convention: `__thiscall void (CAutomobile* this)`, plain `ret`. `t(n)` below = `(float)rand() * 3.0518509e-05f`
[0x858C7C] for the n-th rand() call in this block (rand() not masked).
Rows Automobile.cpp:5379 / :5380 (`velocity.x/.y += GetRandomNumberInRange(0.05f, -0.05f)`).
- Original bytes (0x6A9A0E-0x6A9A81): `vel = f32(forward * -0.1f)` (z first); `vel.x = f32((t(1) * 0.1f + vel.x) -
  0.05f)`, `vel.y = f32((t(2) * 0.1f + vel.y) - 0.05f)` (0.1f [0x858B1C], 0.05f [0x858C28]), i.e. an added term
  `0.1*t - 0.05` in [-0.05, 0.05], increasing with t.
- gr: `0.05*(1-t) - 0.05*t` = `0.05 - 0.1*t` (mirrored mapping, lerp rounding).
- Verdict: CONFIRMED (both). Class: keep-original.
Rows :5405 / :5406 / :5407 (limb turn speed `GetRandomNumberInRange(0.12f, -0.04f)` x3).
- Original bytes (0x6A9B25-0x6A9B9A), only when a limb object was created: `a = f32(t(3))`, then
  `A = f32(a * 0.08f - 0.04f)`, `B = f32(t(4) * 0.08f - 0.04f)`, `C = f32(t(5) * 0.08f - 0.04f)` (0.08f [0x859018],
  0.04f [0x858CEC]); the call 0x45AFB0 (stores its three args to +0x50/+0x54/+0x58 = m_vecTurnSpeed) receives
  `(C, B, A)`: turnSpeed.x comes from the LAST rand() call, .z from the first. Range [-0.04, 0.04].
- gr: each component `lerp(0.12, -0.04, t)` = `0.12 - 0.16*t` (range [-0.04, 0.12]), x from the first call.
- Verdict: CONFIRMED (all three: wrong range constants in gr, reversed component order, different formula).
- Class: keep-original.

## 0x6A9D70 CAutomobile::ProcessSwingingDoor  (gr_deviations = 1)
Convention: `__thiscall void (CAutomobile* this, eCarNodes node [esp+4], eDoors door [esp+8])`, `ret 8`.
Row Automobile.cpp:5496 (todo: `door.m_doorState == DAMSTATE_OPENED` compares eDoorState with ePanelDamageState).
- Original bytes: bonnet detach test is `door == 0 && byte[+0x5C3] (m_doors[0].m_doorState) == 1` (0x6AA017-0x6AA026)
  and `(v.z*f.z + v.y*f.y) + v.x*f.x > 0.4f` [0x858EE8] (0x6AA02C-0x6AA052). Value 1 is DOOR_HIT_MAX_END; gr's
  DAMSTATE_OPENED is also 1.
- Verdict: REFUTED (same numeric test). Class: harmless.

Found in passing (CONFIRMED, keep-original; gr logic error) - velocity of the detached bonnet (0x6AA096-0x6AA165):
`r = rand()`; with `u = 0.5f * up`, `rt = 0.1f * right`, `m = 0.4f * moveSpeed` (each via 0x40FE90, f32 per
component): if `(r & 1) != 0`: `obj->m_vecMoveSpeed = (m + rt) + u`; else `obj->m_vecMoveSpeed = (m - rt) + u`
(VectorSub 0x40FE60 then 0x40FE30). So the random bit flips the sign of the sideways term and the up term is always
added. gr adds `0.1*right` always and flips the sign of the up term instead. Then
`obj->ApplyTurnForce(10.0f * up, forward)` as in gr. Products are computed in the order u, rt, m (one rand() call
before them).

## 0x6ACE70 CAutomobile::ProcessEntityCollision  (gr_deviations = 7)
Convention: `__thiscall int32 (CAutomobile* this, CEntity* entity [esp+4], CColPoint* outColPoints [esp+8])`,
`ret 8`. Virtual slot 22. Locals 0x88 (saved compressions at esp+0x58, touch distances at esp+0x28).
Original order (for reference): status != SIMPLE -> vehicleFlags bit 0x01 of byte +0x42B (bVehicleColProcessed);
`tcd = GetColModel()->m_pColData` (no null check, `m_nNumLines` byte read at 0x6ACE96); copy compressions; RHINO copies
door 4/5 fields; zero `tcd->m_nNumLines` when physicalFlags & 0x9000, entity is a ped (type 3) or (model == 0xFFFE
and entity type 2); hide triangles when `m_pTractor (+0x4C4) == entity || m_pTrailer (+0x4C8) == entity`
(0x6ACFBC-0x6AD001); `numColPts = status == GHOST(12) ? 0 : CCollision::ProcessColModels(...)` (0x4185C0, lines
output to aAutomobileColPoints 0xC1BFF8); restore triangles; wheels; forklift filter; collision records; extra
wheel colpoints.

Rows Automobile.cpp:1697 / :1698 (FIX_BUGS block `if (!tcd || !ocd) return 0;`), :1699 and :1702 (comments).
- Original bytes: no early return. `tcd->m_nNumLines` is read before anything else (0x6ACE96), so a null `tcd` faults.
  `ocd` is only dereferenced in the tow case (0x6ACFE7-0x6AD001: `entity->GetColModel()->m_pColData->m_nNumTriangles`
  read and set to 0), which faults when it is null - the edge case gr's comment describes. Otherwise ProcessColModels
  handles missing data.
- Verdict :1697/:1698: CONFIRMED (gr returns 0 early). :1699: REFUTED (comment only). :1702: CONFIRMED (describes the
  original crash; keep the original access pattern).
- Class: keep-original (the null cases do not occur for streamed vehicles).

Row :1723 (`NOTSA_UNREACHABLE` in the RHINO extra-wheel lambda `switch (idx % 4)`).
- Original bytes: the eight values are plain dword copies of door 4/5 fields +0x00, +0x04, +0x0C, +0x10 (0x6ACEFD-0x6ACF8C)
  and are written back with `min()` after the wheel loop (0x6AD24A-0x6AD372). No switch exists.
- Verdict: REFUTED. Class: harmless.

Row :1741 ("Later reset back to original value", `tcd->m_nNumLines = 0`).
- Original bytes: the count is saved at 0x6ACE96 and restored only in the branch where `tcd->m_nNumLines == 0` after the
  collision call (0x6AD08F-0x6AD0A0), which also skips the wheel loop. gr does the same in its `else` branch.
- Verdict: REFUTED. Class: harmless.

Row :1876 (`#ifdef FIX_BUGS numColPts++; break;` in the extra wheel colpoint loop).
- Original bytes (0x6AD657-0x6AD66F): after filling the colpoint, `if (numColPts < 31) { numColPts++; next slot }`;
  the loop continues with the next wheel. Each of the 4 wheels can add one colpoint. gr with FIX_BUGS adds at most
  one and stops; gr without it never increments.
- Verdict: CONFIRMED. Class: keep-original.

Found in passing (CONFIRMED, keep-original):
1. Forklift filter (model 0x212, entity type 4 object, `numColPts > 0`, `m_wMiscComponentAngle == 0` and
   `m_wMiscComponentAnglePrev == 0`, 0x6AD378-0x6AD42D): the original removes entries with surfaceTypeA == 0x41 from
   `outColPoints` (the caller's array, `[esp+0xA0]`), shifting the tail down one CColPoint at a time (operator=
   0x40FC80) and decrementing numColPts. gr filters `aAutomobileColPoints` (the suspension-line array) instead.
2. Wheel entity reference: the original stores `m_apWheelCollisionEntity[i] = entity` and calls
   `entity->RegisterReference(&slot)` (0x6AD14E-0x6AD150) without CleanUpOldReference on the previous pointer; gr uses
   ChangeEntityReference (cleans the old one).
3. Extra wheel colpoint depth (0x6AD589-0x6AD630): `delta = ((compNow - comp[i]) * line[i]) - g_C1C214` (f32 at
   0xC1C214, right after aAutomobileColPoints; no writer in the catalogue, so 0.0); `d = min(0.2f [0x8D31FC], delta)`
   (0.2 taken when 0.2 < delta); `s = |(v.z*f.z + v.y*f.y) + v.x*f.x|`, `s = (s > 0.3f) ? 0.3f : s`;
   `depth = (((|(n.z*u.z + n.x*u.x) + u.y*n.y| * s) * d) * 3.3333333f [0x859F60]) * handling->m_fSuspensionHighSpdComDamp`.
   gr divides by 0.3f at the end and multiplies in a different order (different rounding). The delta test
   (`> 0.1f` [0x8D3200]) runs before GetWheelStatus in the original; gr tests the wheel status first (no side effect).

## 0x6AD690 CAutomobile::ProcessControlInputs  (gr_deviations = 1 counted, 3 rows)
Convention: `__thiscall void (CAutomobile* this, u8 playerNum [esp+4])`, virtual slot 24, `ret 4`.
Row Automobile.cpp:1991 (notsa `dyn_cast_if_present<CTaskSimpleGangDriveBy>`): not checked in detail (helper only).
Verdict: REFUTED (marker is a helper, no behavioural claim). Class: harmless.

Row :2046 (bug-note on `m_f2ndSteerAngle * 0.9f`).
- Original bytes (0x6ADD28-0x6ADD65), when handling flags (+0xD0) & 0x40 (HB_REARWHEEL_STEER): handbrake (vehicleFlags
  byte +0x428 & 0x20) -> `m_f2ndSteerAngle = [0x8D3208]*m_f2ndSteerAngle - (1.0f - [0x8D3208]) * steer` (x87:
  `fld k; fmul 2nd; fld 1; fsub k; fmul steer; fsubp`), with `steer` the f32 just stored to +0x494 and k = 0.9f
  (data); else `m_f2ndSteerAngle = [0x8D3204] * m_f2ndSteerAngle` (k = 0.9f, data).
- Verdict for the noted else branch: REFUTED (gr keeps `* 0.9f`). Class: keep-original (read the two data constants).
- Found in passing (CONFIRMED, keep-original): gr's handbrake branch is `lerp(m_f2ndSteerAngle, -m_fSteerAngle,
  0.9f)` = `0.1*2nd - 0.9*steer`; the original keeps 90 % of the old value: `0.9*2nd - 0.1*steer`.

Row :2051 (rng-helper `GetRandomNumberInRange(0, 10)` for comedy controls).
- Original bytes: if vehicleFlags byte +0x429 & 0x10 (bComedyControls): `r = rand() % 10` (`cdq; idiv 10`,
  0x6ADD74-0x6ADD7F). State byte +0x51A: 0 -> r < 2 -> 1, 2 <= r < 4 -> 2 (0x6ADE89-0x6ADEA5); 1 -> steer += 0.05f
  [0x858C28]; 2 -> steer -= 0.05f; for states 1/2 the state is reset to 0 when r < 2; any other state value: no change.
- Verdict: CONFIRMED (`rand() % 10` vs gr's lerp helper). Class: keep-original.

Found in passing (CONFIRMED, keep-original): the steer angle (0x6ADCDB-0x6ADD22) is `s = raw*raw; if (raw < 0) s = -s;`
stored to the global f32 0xC1C218 (written here, read nowhere else in this function), then
`m_fSteerAngle = (handling->m_fSteeringLock * 0.017453292f [0x8595EC]) * s`. gr computes
`DegreesToRadians(lock * copysign(pow(raw, 2), raw))` (other multiplication order, no global write).
Found in passing (CONFIRMED, keep-original): when bComedyControls is clear, the original jumps to 0x6ADDBC and
writes `m_comedyControlState = 0` every call (`je 0x6ADDBC` at 0x6ADD72); gr leaves the state untouched.

## 0x6AF110 CAutomobile::BlowUpCarsInPath  (gr_deviations = 2)
Convention: `__thiscall void (CAutomobile* this)`, plain `ret`.
Original (0x6AF110-0x6AF1C8): `if (!(sqrt((vx*vx + vy*vy) + vz*vz) > 0.1f [0x858B1C])) return;` (`test ah,0x41; jne`,
NaN returns); `if ((autoFlags (+0x868) & 0x20) == 0) return;` (0x6AF145-0x6AF14C); then for i = 0 ..
`m_nNumEntitiesCollided (+0xB9, re-read each iteration) - 1`: `e = m_apCollidedEntities[i] (+0xBC)`; skip null, type
!= 2, model 0x1B0 (RHINO), physicalFlags & 0x20000000 (bRenderScorched); if `this == FindPlayerVehicle(-1, false)`:
`CCrime::ReportCrime(0x11, e, FindPlayerPed(-1))`; `e->BlowUpCar(this, false)` (virtual slot 41, +0xA4).

Row Automobile.cpp:5764 ("Originally `Magnitude() < 0.1f`").
- Original bytes: sqrt magnitude compared with 0.1f (0x6AF12C-0x6AF13F). gr compares the squared magnitude with
  `sq(0.1f)`, which differs at the boundary (rounding of 0.1f*0.1f and of the sqrt).
- Verdict: CONFIRMED. Class: keep-original.
Row :5774 ("original code checks" for null entities).
- Original bytes: `test eax, eax; je` at 0x6AF164. gr checks too.
- Verdict: REFUTED. Class: harmless.

Found in passing (CONFIRMED, keep-original; gr logic error): the flag test is inverted in gr. The original continues
only when `autoFlags.bTankExplodesCars` is SET (`test byte [edi+0x868], 0x20; je return`); gr returns when it is set.

## 0x6B0690 CAutomobile::DoHeliDustEffect  (gr_deviations = 3)
Convention: `__thiscall void (CAutomobile* this, f32 timeConstMult [esp+4], f32 fxMaxZMult [esp+8])`, `ret 8`.
Original order: subtype 3 (HELI) needs `m_fHeliRotorSpeed (+0x84C) > 0.1125f` [0x87125C], subtype 4 (PLANE) passes,
anything else -> kill (0x6B06AC-0x6B06CE). Then, before any distance test: `CWorld::ProcessVerticalLine(pos, -1000.0f,
cp, &ent, true, 0, 0, 0, 0, 0, 0)` (0x6B0705, result ignored), `isSand = IsSand(cp.surfaceTypeB)`,
`groundZ = cp.point.z`; `level = -1000.0f`; if `GetWaterLevel(pos.x, pos.y, pos.z, &level, false, nullptr)` and
`level > groundZ` (0x6B0772-0x6B077F): `groundZ = level`, `underWater = true`. `maxZ = (model 0x1F5 or 0x1D1 ? 3.0f :
30.0f) * fxMaxZMult` (f32). Only now the camera test: `dist2(pos, camPos) >= 2500.0f` (or NaN) -> kill
(0x6B07B5-0x6B0808); `pos.z - groundZ >= maxZ` (or NaN) -> kill.

Row Automobile.cpp:5959 (bugfixes::GenericUB: `isHitPosUnderWater = water && ((!hasHitGround && GenericUB) || level >
ground.z)`).
- Original bytes: `underWater = water && level > cp.point.z`; the ProcessVerticalLine result is never tested, so when
  the line misses, `cp.point.z` is whatever the stack holds (the CColPoint local is not initialised).
- Verdict: CONFIRMED. Class: keep-original (open question below).
Row :5961 (GenericUB: kill when neither ground nor water was found).
- Original bytes: no such test (0x6B076E-0x6B078E).
- Verdict: CONFIRMED. Class: keep-original.
Row :6035 ("Above code originally (kinda)": EnablePrim sequence).
- Original bytes (0x6B09EA-0x6B0A4C): underWater -> EnablePrim(0,0), (1,0), (2,1); else sand -> (0,0), (1,1), (2,0);
  else (0,1), (1,0), (2,0). gr's three calls produce the same values in the same order.
- Verdict: REFUTED. Class: harmless.

Found in passing (CONFIRMED, keep-original): gr moves the 50 m camera-distance early-out before ProcessVerticalLine /
GetWaterLevel. The original always runs both queries first (ProcessVerticalLine advances the world scan code), so
a port must keep the original call order.
Open question: value of `cp.point.z` at 0x6B0712 (`[esp+0x8C]` after the call) when ProcessVerticalLine finds
nothing; the original reads uninitialised stack there.

## 0x6B0A90 CAutomobile::Constructor  (gr_deviations = 1 counted, 2 rows)
Row Automobile.cpp:257 (rng-helper `DoorRotation[i] = GetRandomNumberInRange(-0.15f, 0.15f)`).
- Original bytes (0x6B0E8E-0x6B0EE0, 6 iterations over +0x8CC..+0x8E0): inlined float range with min = `-[0x870FD8]`
  (fchs of 0.15f) and max = `[0x870FD8]`: `v = (max - min) * ((float)rand() * 3.0518509e-05f [0x858C7C]) + min`
  (x87: `fild rand; fmul k; fld max; fsub min; fmul st1; fadd min`, stored f32; rand() not masked).
- Verdict: CONFIRMED (gr lerp `max*t + min*(1-t)` rounds differently; same rand() consumption: 6 calls).
- Class: keep-original (the field is unused, but the 6 rand() calls must be kept for RNG parity).
Row :365 ("Probably an inlined function here ... redundant check (model == -2)").
- Original bytes: `cmp ax, 0xFFFE; jne` at 0x6B11D9 then again `cmp word [esi+0x22], -2; jne` at 0x6B1227 after
  `GetFirstObject(m_aCarNodes[5])->flags(+2) = 0`. Same effect as gr's single case.
- Verdict: REFUTED. Class: harmless.
Checked in passing: the hydraulics roll `(rand() & 3) == 0` (0x6B0BC4-0x6B0BCB) equals gr's
`GetRandomNumber() % 4 == 0` for rand() >= 0 (REFUTED, harmless).

## 0x6B1350 CAutomobile::SetBumperDamage  (gr_deviations = 1 counted, 3 rows)
Convention: `__thiscall void (CAutomobile* this, ePanels panel [esp+4], bool withoutVisualEffect [esp+8])`, `ret 8`.
The body starts with an obfuscation hop (0x6B135C `jmp 0x409B1D` -> `lea ecx, [esi+0x5A0]; jmp 0x6B1361`).
Original order: `s = m_damageManager.GetPanelStatus(panel)` (before the node lookup), `node =
GetCarNodeIndexFromPanel(panel)`; return if `m_aCarNodes[node]` is null or the damageable-mask bit is clear.
- s == 2: return if handling model flags & 0x10000000 (bBouncePanels). Scan `m_panels` (+0x6AC, stride 0x20, frame id
  i16 at +0): `for (i = 0; i <= 3; i++)`: frame id 0xFFFF -> use slot i; == node -> return. Falling out -> return.
  Free slot: `m_panels[i].SetPanel(node, 0, (float)rand() * 3.0518509e-05f * -0.3f - 0.2f)` (0x6B13F4-0x6B1424;
  `fild; fmul [0x858C7C]; fmul [0x858EE4] = -0.3f; fsub [0x858CC4] = 0.2f`), then return.
- s == 1: `SetComponentVisibility(frame, 2)`. s == 3: if !withoutVisualEffect `SpawnFlyingComponent(node, 0)`; then
  `SetComponentVisibility(m_aCarNodes[node], 0)`.

Row Automobile.cpp:6052 (R* log) and :6062 (bug-note on the bBouncePanels name): REFUTED (log only; gr keeps the test
`!bBouncePanels` as the original). Class: harmless.
Row :6064 (rng-helper `SetPanel(nodeIdx, 0, GetRandomNumberInRange(-0.5f, -0.2f))`).
- Original bytes: `-0.2f - 0.3f*t`, t = rand()/32767 (equivalent to the original helper with min = -0.2, max = -0.5).
  gr's call with (-0.5, -0.2) yields `-0.5 + 0.3*t` (mirrored mapping) and uses the lerp formula.
- Verdict: CONFIRMED. Class: keep-original.

Found in passing (CONFIRMED, keep-original): the panel scan runs i = 0..3 (`cmp edi, 3; jle`, 0x6B13E8), one past
`m_panels[3]`; i = 3 reads the word at +0x70C (start of `m_swingingChassis`, i.e. the low half of its m_openAngle) and,
if that word is 0xFFFF, calls SetPanel on +0x70C (overwriting the chassis door). gr's CheckIfExistsGetFree stops at 3
panels. The same scan appears in SetPanelDamage 0x6B1480.

## 0x6B1480 CAutomobile::SetPanelDamage  (gr_deviations = 1 counted, 2 rows)
Convention: `__thiscall void (CAutomobile* this, ePanels panel [esp+4], bool createWindowGlass [esp+8])`, `ret 8`.
Original order as in SetBumperDamage (status first, then node, null/mask returns). s == 2: return if bBouncePanels;
scan i = 0..3 (same out-of-range 4th slot at +0x70C); match -> skip SetPanel; free slot and node not in {0x12, 0x0F,
0x0E} -> `SetPanel(node, 1, GetRandomNumberInRange<float>(-0.2f, -0.5f))` (0x41BD90, pushed max = -0.5f then
min = -0.2f); in every s == 2 case that did not return: `SetComponentVisibility(frame, 2)`. s == 1: if panel == 4
`m_vehicleAudio.AddAudioEvent(0x5B, 0.0f)`; visibility 2. s == 3: if !createWindowGlass `SpawnFlyingComponent(node, 5)`,
else if panel == 4 `CGlass::CarWindscreenShatters(this, false)` (cdecl, 2 args); visibility 0.

Row Automobile.cpp:6098 (bug-note on bBouncePanels): REFUTED (gr returns on the flag like the original). harmless.
Row :6108 (rng-helper `GetRandomNumberInRange(-0.2f, -0.5f)`).
- Original bytes: 0x41BD90 with min = -0.2f, max = -0.5f: `(max - min) * t + min`.
- Verdict: CONFIRMED (formula only; argument order matches). Class: keep-original.
Found in passing: same 4-slot panel scan as SetBumperDamage (0x6B1514 `cmp edi, 3; jle`). CONFIRMED, keep-original.

## 0x6B1880 CAutomobile::ProcessControl  (gr_deviations = 1)
Convention: `__thiscall void (CAutomobile* this)`, virtual slot 10, plain `ret` (2210 instructions; only the row's
block was checked).
Row Automobile.cpp:1071 (rng-helper `position.z = GetRandomNumberInRange(0.0f, CWeather::Earthquake * 0.5f)`).
- Original bytes (0x6B3635-0x6B36C2): `if (CWeather::Earthquake [0xC81340] > 0.0f)`: copy the position (matrix pos or
  placement), `h = f32(Earthquake * 0.5f)`, `z = ((float)rand() * 3.0518509e-05f) * h + pos.z` (x87, rand() not
  masked), then write x, y back unchanged and z (to `m_matrix->pos` if the matrix exists, else placement +0x4..+0xC).
- Verdict: CONFIRMED - two differences: (1) gr assigns the random value to z instead of adding it to the current z
  (gr logic error: the car would be teleported to height 0..E/2); (2) the helper formula.
- Class: keep-original (`z += t * (E * 0.5f)`).

