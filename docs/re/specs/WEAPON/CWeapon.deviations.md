# CWeapon (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Weapon.cpp` (pin 20d8d2a). RNG: the original either calls CGeneral::GetRandomNumberInRange
(int 0x407180 / float 0x41BD90) or inlines `(float)rand() * 3.0518509e-05f` (1/32767, no `& 0xFFFF`); gr's helpers differ
(docs/re/cat14-gr-deviations.md finding 1).

## 0x73A3E0 CWeapon::AddGunshell  (gr_deviations = 2)
Original: `__thiscall void (CEntity* creator [esp+4], CVector* pos [+8], CVector2D* dir [+0xC], float size [+0x10])`, `ret 0x10`.
Row Weapon.cpp:142 (original-comment "originally squared").
- Original bytes: `d = TheCamera.GetPosition() - creator->GetPosition()` (camera matrix 0xB6F03C ? +0x30 : placement 0xB6F02C);
  `q = (d.z*d.z + d.x*d.x) + d.y*d.y` (0x73A432-0x73A440); return if `q > 100.0f [0x858628]` (`test ah,0x41; je`;
  NaN does not return).
- Verdict: REFUTED - gr also compares the squared distance against `sq(10.0f)` = 100 and returns on `>`, NaN passes in both.
  Only the summation order differs (gr x+y+z, original (z+x)+y), which can change the last bit at the boundary;
  port uses the original order. Class: harmless (keep the original sum order).
Row Weapon.cpp:154 (rng-helper `CGeneral::GetRandomNumberInRange(0.4f, 1.6f)` for the velocity z).
- Original bytes: `vel = (dir.x * 1.0f, dir.y * 1.0f, z)` with
  `z = (float)rand() * 3.0518509e-05f [0x858C7C] * 0.06f [0x859934] * 20.0f [0x858BA4] + 0.4f [0x862D50]`
  (0x73A476-0x73A49F; raw `rand()` 0x821B1E, no `& 0xFFFF`, scale 1/32767, three separate f32 multiplies). Then the original
  calls `CGeneral::GetRandomNumberInRange(-20, 20)` (int, 0x407180 at 0x73A4A3) and DISCARDS the result - one extra
  `rand()` per gunshell. FxPrtMult_c(0.5, 0.5, 0.5, 1.0, size, 1.0, 1.0) (0x4AB290); for m_Type 25/27 (SHOTGUN/SPAS12) r,g,b =
  0.6, 0.1, 0.1. `g_fx.m_GunShell` ([0xA9AE14])->AddParticle(pos, &vel, 0.0f, &mult, -1.0f, 1.2f, 0.6f, false) (0x4AA440).
- Verdict: CONFIRMED - different formula (rounding/scale) and gr omits the discarded second RNG call, which shifts the
  global rand() sequence for everything after a gunshell. Class: keep-original (both rand() calls, original formula).

## 0x73A530 CWeapon::GenerateDamageEvent  (gr_deviations = 1)
Row Weapon.cpp:216 (fix-bugs-runtime: `(!notsa::IsFixBugs() || CWeaponInfo::TypeIsWeapon(weaponType)) && GetWeaponInfo(weaponType)->m_nWeaponFire == MELEE`).
- Original bytes (after ComputeDamageResponse 0x73A6E9): `wi = CWeaponInfo::GetWeaponInfo(weaponType, 1)` (0x743C60) is called
  UNCONDITIONALLY for any weaponType (0x73A6EE-0x73A6F1); then the anim branch is taken iff
  `(wi->m_nWeaponFire (+0) == 0 /*MELEE*/ || (weaponType == 0x36 /*WEAPON_FALL*/ && creator && (creator->+0x36 & 7) == 4 /*object*/))`
  and `!(victim->+0x46C & 0x100)` (bInVehicle) (0x73A6FB-0x73A72A; the vehicle test comes after the type tests, a pure reordering
  versus gr). Only the evaluation order differs otherwise (no side effects in these tests).
- Verdict: CONFIRMED when gr runs with fix-bugs on (it then skips the GetWeaponInfo lookup for non-weapon damage types and
  treats them as non-melee); with fix-bugs off gr matches. Class: keep-original - always evaluate
  `GetWeaponInfo(type, 1)->m_nWeaponFire` (for types such as WEAPON_FALL it reads whatever aWeaponInfo entry
  GetWeaponInfo returns).

## 0x73A8D0 CWeapon::LaserScopeDot  (gr_deviations = 1)
Row Weapon.cpp:275 (notsa-unreachable: gr body is `/* UNUSED */ NOTSA_UNREACHABLE();`).
- Original bytes: a complete function, `__thiscall bool (CVector* outCoord [esp+4], float* outSize [+8])` (this = CWeapon*),
  AL result, `ret 8`. No catalogued callers (catalogue: 0), so it is dead in 1.0 US. Body:
  `range = CWeaponInfo::GetWeaponInfo(this->m_Type, 1)->+8` (0x743C60, skill 1); with `cam = TheCamera.m_aCams[m_nActiveCam]`,
  `A = cam+0x190` (0xB6F32C + n*0x238), `B = cam+0x19C` (0xB6F338): `P = A*0.5f + B` (per component, 0x73A8F8-0x73A955);
  `d = A` normalised (CVector::Normalise 0x59C910); `E = d*range + P`; `CWorld::ProcessLineOfSight(P, E, colPoint, entity(=0),
  buildings 1, vehicles 1, peds 1, objects 1, dummies 0, seeThrough 0, camera 0, shootThrough 0)` (0x56BA00); on hit
  `CalcScreenCoors(colPoint.pos, &o, &w, &h, true, true)`; if that succeeds: `*outCoord = o`, `*outSize = w * 0.05f [0x858C28]`,
  `CCoronas::RegisterCorona(id = (u32)this + 7, null, 0x80, 0, 0, 0xFF, colPoint.pos, 1.2f, 50.0f, tex null, flare 0, refl 1,
  checkObstacles 0, 1, normalAngle 0.0f, longDistance 0, nearClip 1.5f, fadeState 0, fadeSpeed 15.0f, onlyFromBelow 0, delay 0)`
  (0x6FC580, texture overload) and return true; otherwise return false.
- Verdict: CONFIRMED (gr has no implementation; the original does real work). Class: harmless in practice (no callers);
  a port that keeps the function must implement the original body above.

## 0x73B550 CWeapon::DoBulletImpact  (gr_deviations = 7)
Row Weapon.cpp:517 (`#ifdef FIX_BUGS` block 517-521; original = the `#else` branch, 0x73B6D0).
- Original bytes (0x73B6BB-0x73B6C7; EBP = firedByPlayer, EBX = firedBy, ESI = victim): continue into the stats/skill block iff
  `(firedByPlayer != null && victim != firedBy) || (firedBy->m_nStatus (+0x36 >> 3) == 0 /*STATUS_PLAYER*/)`
  (`test b[ebx+0x36],0xF8`; firedBy is dereferenced without a null check in the second half), i.e. gr's `#else` expression.
  Inside: victim type 3, 2 or 4 -> `a = GetStatValue(0x80)`
  (stored f32), `b = GetStatValue(0x7E)`; if `!(b < a)` (C0 clear) `IncrementStat(0x80, 1.0f)` (0x73B6DE-0x73B716).
- Verdict: CONFIRMED (the FIX_BUGS build regroups the condition: with firedByPlayer == null and a STATUS_PLAYER shooter gr
  skips the block, the original enters it). Class: keep-original (`#else` form).
Rows Weapon.cpp:522, :530, :545, :596, :692, :708 (notsa::contains / notsa::coalesce helpers).
- Original bytes: :522 = type compares 3/2/4 (0x73B6CD-0x73B6DC); :545 and :596 = `13 <= cp.pieceB <= 16` (unsigned byte
  compares at 0x73B7F6-0x73B7FF and 0x73BD2A-0x73BD37); :692 = pedType compares 4 / 5 (0x73B886-0x73B88E); :708 = weapon type
  compares 0x19 / 0x1B (0x73BA1B-0x73BA23); :530 is a null-coalesce of `m_pTargetedObject` with victim (pure expression).
- Verdict (all six): REFUTED - the helpers express the same predicates as the original compares. Class: harmless.

## 0x73C1F0 CWeapon::TakePhotograph  (gr_deviations = 5)
Original: `__thiscall bool (CEntity* owner [esp+4], CVector* point [esp+8])`, `ret 8`. If point: `g_fxMan.CreateFxSystem("camflash"
(0x872C24), point, 0, 0)` + PlayAndKill. Return false unless the active cam mode (i16 0xB6F1A8 + n*0x238) == 0x2E. Then
CPickups::PictureTaken, `bPhotographHasBeenTaken (0xC8A7C0) = 1`, `ms_bTakePhoto (0xC8A7C1) = 1`, `CStats::IncrementStat(0xA6, 1.0f)`,
ped loop, object loop, return true (AL = 1).
Row Weapon.cpp:768 (NOTSA optimisation: camera matrix/pos cached).
- Original bytes: re-reads `TheCamera` pos (matrix 0xB6F03C ? +0x30 : 0xB6F02C) at every use and the vector at 0xB6F9AC..0xB6F9B4
  (doubled with `fadd st0,st0`) for the LOS start `camPos + 2*v`. Nothing writes the camera inside the function, so caching gives
  the same values.
- Verdict: REFUTED. Class: harmless.
Row Weapon.cpp:773 (NOTSA: squared magnitude).
- Original bytes: `dist = sqrt(((dz*dz) + dy*dy) + dx*dx)`; candidate only if `dist < 125.0f [0x865038]` (0x73C2EC peds, 0x73C599 objects;
  NaN fails). gr compares `distSq >= 125^2`; the rounded sqrt can equal 125.0f for distSq just below 15625, so the boundary differs.
- Verdict: CONFIRMED (boundary only). Class: keep-original (use sqrt and `< 125.0f`).
Row Weapon.cpp:783 (bug-note about the 10%/90% border).
- Original bytes: accept iff `0.1f*W < x` and `x < 0.9f*W` and `0.1f*H < y` and `y < 0.9f*H` (fild W/H, f32 0.1f 0x858B1C, 0.9f 0x858C20;
  0x73C335-0x73C39F); gr's rejection test is the same condition.
- Verdict: REFUTED (gr matches; the note is a remark). Class: harmless (keep-original).
Rows Weapon.cpp:815 / :834 (pool iteration ascending).
- Original bytes: ped pool from slot size-1 down to 0 (0x73C279-0x73C4E2), object pool likewise (0x73C4F9-0x73C6F2). Each entity
  only gets its own flag set (`ped+0x470 |= 0x40000` bHasBeenPhotographed, `obj+0x140 |= 0x1000` bIsPhotographed), so the order
  is not observable.
- Verdict: REFUTED for both. Class: harmless.
Found in passing (CONFIRMED, gr LOGIC ERROR, keep-original): for objects the original uses the same `dist < 125.0f` acceptance
as for peds (0x73C599 `jp` skips when not less). gr writes `if (!IsPosInRange(objPos) || ...) continue;`, i.e. it skips objects
CLOSER than 125 and photographs only far ones - inverted. Also for both loops the original sets the flag when
`!ProcessLineOfSight(...) || hitEntity == entity` (LOS flags 1,1,1,1,1,1,0,0; peds: target = head (GetBonePosition(5, false)) +
normalise(camPos - head) * 1.5f [0x858CE8]); this matches gr.

## 0x73C710 CWeapon::SetUpPelletCol  (gr_deviations = 3)
Rows Weapon.cpp:853 (assert), :874 and :875 (rng-helper, `GetRandomNumberInRange(-PI, PI)` and
`GetRandomNumberInRange(0.f, depth * 0.8f)`).
- Original bytes: `depth = |colPoint.pos - point| * CWorld::fWeaponSpreadRate [0xB7CD64] * PELLET_COL_SCALE_RATIO_MULT
  [0x8D6128] = 1.3f` (0x73C7BB-0x73C7E8, stored f32 at [esp+0x60]). Line 0 = (0,-depth,0)-(0,depth,0). For
  i = 1..numPellets-1 (0x73C835-0x73C904), precomputed: `lo = depth * 0.2f [0x858CC4]` (f32), `w = depth - lo` (f32),
  `y0 = depth * -2.0f [0x858B18]`, `y1 = depth + depth`. Per pellet:
  `angle = (float)rand() * 3.0518509e-05f [0x858C7C] * 6.2831855f [0x858CBC] - 3.1415927f [0x858CB8]` (stored f32; the CRT
  `rand()` 0x821B1E value is used directly, NO `& 0xFFFF`, scale 1/32767 not 1/32768); then
  `spread = (float)rand() * 3.0518509e-05f * w + lo` (x87, kept in ST); `x = cos(angle) * spread`, `z = sin(angle) * spread`
  (x87 fcos/fsin on the f32 angle); line = (x, y0, z)-(x, y1, z). The two rand() calls are in this order.
- :853 assert: no code in the original; not behavioural. Verdict: REFUTED. Class: harmless.
- :874 Verdict: CONFIRMED - gr's float helper is `lerp(min, max, t)`; the original inlines `t*(2pi) - pi` with
  t = rand()*(1/32767) (rounding differs). Class: keep-original.
- :875 Verdict: CONFIRMED and larger than a rounding difference: the original spread range is `[0.2*depth, depth]`
  (`t*(depth - 0.2depth) + 0.2depth`), gr uses `[0, 0.8*depth]`. Class: keep-original.

## 0x73CDC0 CWeapon::DoDoomAiming  (gr_deviations = 3)
Original: `__cdecl void (CEntity* owner [esp+4], CVector* start [+8], CVector* end [+0xC])`, plain `ret`.
Row Weapon.cpp:972 (notsa "todo: add check owner->GetIsTypePed()").
- Original bytes: `CPed::CanSeeEntity(owner, entity, 0.3926991f)` (0x5E0730, ECX = owner, imm 0x3EC90FDB, 0x73CE58-0x73CE60) is
  called with no type check of owner either.
- Verdict: REFUTED (gr matches; the comment proposes a check neither has). Class: harmless.
- Found in passing (CONFIRMED, gr LOGIC ERROR, keep-original): the original SKIPS the entity when CanSeeEntity returns
  false (`test al,al; je 0x73D093` at 0x73CE65), i.e. only visible entities are candidates. gr's
  `if (entity == owner || owner->AsPed()->CanSeeEntity(entity, PI / 8.f)) continue;` skips the visible ones - inverted.
Row Weapon.cpp:994 (original-comment: "closestDist is FLT_MAX (originally 10 000)").
- Original bytes: `closest = 10000.0f` (imm 0x461C4000, 0x73CE29); after the loop `if (!(closest < 9000.0f [0x872C30])) return`.
  gr's code also initialises `closestDist{ 10'000 }`; the comment is stale.
- Verdict: REFUTED. Class: harmless.
Row Weapon.cpp:1006 (original-comment "-0.8 effectively only -0.5 relative to the original Z").
- Original bytes: `tz = target.pos.z + 0.3f [0x858C24]`; if target is a ped (type 3) and `+0x46C & 0x4000000` (bIsDucking):
  `tz -= 0.8f [0x858C98]` (0x73D18C-0x73D1AC). Then `k = L2 / D2` with `L2 = sqrt((e.y-s.y)^2 + (e.x-s.x)^2)` and
  `D2 = sqrt((T.y-s.y)^2 + (T.x-s.x)^2)`; `end->z = (tz - s.z) * k + s.z` (0x73D1B2-0x73D1C2).
- Verdict: REFUTED (gr does the same arithmetic; the comment is explanatory). Class: harmless.
Other details for the port (bytes): candidates come from `FindObjectsInRange(*start, |*end - *start| ((dz^2+dy^2)+dx^2),
true, &n, 15 (push 0xF at 0x73CDFA; gr passes `objInRange.size()` = 16: CONFIRMED difference, keep 15), list, buildings 0, vehicles 1, peds 1, objects 0, dummies 0)`; status 6, 7 and 5 are skipped; vertical test
`1.5f [0x858CE8] * |owner.z - c.z| < d2D` (strict) and `d3 = sqrt(dz*dz + d2D*d2D)` (gr uses `std::hypot`, which may round
differently: keep sqrt); the LOS check is `ProcessLineOfSight(*start, target pos, cp, ent, 1,0,0,0,0,0,0, shootThrough 1)`,
any hit -> return.

## 0x73E800 CWeapon::FireAreaEffect  (gr_deviations = 1)
Original: `__thiscall bool (CEntity* firer [esp+4], CVector* origin [+8], CEntity* targetEntity [+0xC], CVector* target [+0x10])`.
Row Weapon.cpp:1272 ("Original code used degs instead of radians ... we stick to radians").
- Original bytes: with no targetEntity and no target, and not (firer == FindPlayerPed(-1) && TheCamera.m_aCams[0]
  .Using3rdPersonMouseCam()): `h = firer matrix ? atan2(-m.forward.x (+0x10), m.forward.y (+0x14)) : placement heading (+0x10)`,
  then `h *= 57.295776f [0x859878]` (degrees, stored f32 at 0x73E975 / 0x73E987); at 0x73E990 `r = h * 0.017453216f [0x872C60]`
  - note 0.017453216f is NOT pi/180 (0.017453292f): the round trip scales the angle by 57.295776*0.017453216 = 0.9999986.
  `dir = (-sin(r), cos(r), 0)` (stored f32). If firer is a ped (type 3) with `m_pPlayerData (+0x480)`: `dir.z = -tan(pd->+0x54)`
  (fptan) AND `dir.Normalise()` (0x59C910 at 0x73E9D8). Shot point = `origin + dir`.
- Verdict: CONFIRMED: (a) the angle used differs from gr's pure-radian value by the factor 0.9999986 (plus f32 roundings
  of the degree value); (b) gr does not normalise `dir` after setting z for player peds - a real behavioural difference
  (found while checking this row). Class: keep-original (degree round trip with these two constants; normalise).
- Note: for targetEntity / target the original also calls CGeneral::GetAngleBetweenPoints (0x53CEA0, args (t.x, t.y, o.x, o.y))
  at 0x73E85C and stores the result, but then takes the targeted path (0x73EA1E) where that value is not used - dead, side-effect
  free work; a port may skip it.

## 0x73F910 CWeapon::PickTargetForHeatSeekingMissile  (gr_deviations = 1)
Row Weapon.cpp:1336 (pool-iteration `for (auto& veh : GetVehiclePool()->GetAllValid())`).
- Original bytes: `__cdecl CEntity* (CVector origin [esp+4], CVector dir [+0x10], float distMul [+0x1C], CEntity* ignore [+0x20],
  bool planesPriority [+0x24], CEntity* preferred [+0x28])`, plain `ret`. `point = origin + dir * 5.0f [0x858C80]` (written
  into the origin arg slots, 0x73F913-0x73F969). `best = null`, `bestRating = 99999.898f` (imm 0x47C34FF3, 0x73F93F).
  Vehicle pool walked from slot size-1 DOWN to 0 (0x73F979-0x73FA05; stride 0xA18, byte-map bit 7 = free). Skip
  `veh == ignore`, `!(veh+0x42D & 0x40)` (bVehicleCanBeTargettedByHS), `!(health (+0x4C0) > 0.0f)` (NaN skips). Rating =
  `CWeapon::EvaluateTargetForHeatSeekingMissile(veh, &point, &dir, distMul, planesPriority, preferred)` 0x73E560 (ST0).
  Accept iff `rating >= 0.0f` (0x73F9E7, NaN accepted by this test) and `rating < bestRating` (strict, 0x73F9EC; NaN rejected).
- Verdict: REFUTED for the iteration order as such: gr walks ascending but accepts `rating <= minRating`, so on exact ties
  both keep the highest slot index (original: first visited with strict `<`; gr: last visited with `<=`), and the
  selection is the same provided EvaluateTargetForHeatSeekingMissile 0x73E560 is side-effect free (not checked here; open question). Class: harmless.
- Found in passing (CONFIRMED, keep-original): gr starts from `FLT_MAX`, the original from 99999.898f, so a vehicle whose
  rating is >= 99999.898 is never picked by the original. Also: a NaN rating passes the `>= 0` test in the original
  (C0 clear) and then fails `< best`; in gr NaN fails `>= 0.f`; same outcome.

## 0x73FA20 CWeapon::FireFromCar  (gr_deviations = 1)
Original: `__thiscall bool (CVehicle* veh [esp+4], bool left [+8], bool right [+0xC])`, `ret 0xC`.
Row Weapon.cpp:1361 (fix-bugs-runtime `return notsa::IsFixBugs() ? false : true;` when FireInstantHitFromCar fails).
- Original bytes: `if (!FireInstantHitFromCar(veh, left, right)) goto 0x73FB01` -> returns AL = 1 without touching ammo/state.
- Verdict: CONFIRMED when gr runs with fix-bugs (returns false); otherwise identical. Class: keep-original (return true).
- Found in passing (CONFIRMED, keep-original): the entry test is `(m_State (+4) == 0 || m_State == 1) && m_AmmoInClip (+8) > 0`
  (0x73FA23-0x73FA34): the original also fires while in WEAPONSTATE_FIRING (1); gr rejects every state except READY.
  Rest matches gr: driver `+0x394` weapon audio `AddAudioEvent(0x91)`; unless `*(u8*)0x969178` (infinite-ammo cheat flag):
  `if (clip > 0) clip--`; `if (total < 25000 && total > 0 && ((veh->+0x36 & 0xF8) != 0 || CStats::GetPercentageProgress() < 100.0f)) total--`;
  `m_State = 1`; clip != 0 -> `m_nTimer (+0x10) = now + 1000`; else if total != 0 -> `m_State = 2`,
  `m_nTimer = now + GetWeaponInfo(m_Type, 1)->GetWeaponReloadTime()` (0x743D70); return true.

## 0x741C00 CWeapon::FireM16_1stPerson  (gr_deviations = 2)
Rows Weapon.cpp:1633 and :1634 (rng-helper `(float)CGeneral::GetRandomNumberInRange(-64, 64) * intensity` for the camera angles).
- Original bytes (0x741EA9-0x741F03): `cam = TheCamera.m_aCams[m_nActiveCam]`; first `cam+0xBC (0xB6F258 + n*0x238) +=
  (float)((rand() & 0x7F) - 64) * k` (fild int, fmul by the f32 local k, fadd, fstp), then `cam+0xAC (0xB6F248 + n*0x238) +=
  (float)((rand() & 0x7F) - 64) * k` with a second rand(). k is the per-weapon intensity, `*= 0.3f [0x858C24]` when the player ped is
  ducking or `m_pAttachedTo (+0xFC)` is set (0x741E87-0x741EA5). gr's first statement updates m_fHorizontalAngle and the second
  m_fVerticalAngle, matching this order (gr layout: vertical at +0xAC, horizontal at +0xBC).
- Verdict (both): CONFIRMED - the original value is `(rand() & 0x7F) - 64` (-64..63, uniform over the low 7 bits), not gr's
  `GetRandomNumberInRange(-64, 64)` lerp formula. Class: keep-original.

## 0x742300 CWeapon::Fire  (gr_deviations = 3)
Original: `__thiscall bool (CEntity* firedBy [esp+4], CVector* startPosn [+8], CVector* shotOrigin [+0xC], CEntity* targetEnt [+0x10],
CVector* targetPosn [+0x14], CVector* altPosn [+0x18])`. Weapon-type switch at 0x74245C: `type - 16` > 0x1B -> default 0x742794;
byte table 0x742A30 / jump table 0x742A0C: 16,17,18,39 -> 0x74267F; 22-24, 28-33, 38 -> 0x7424F8 (M16 path); 25-27 -> 0x74247D
(shotguns); 34 -> 0x7424A6 (sniper); 35,36 -> 0x7425AC (rocket launchers); 37,41,42 -> 0x742757; 40 -> 0x74276F; 43 -> 0x742787;
19, 20, 21 -> default.
Row Weapon.cpp:1743 (notsa-helper `contains({MODE_M16_1STPERSON, MODE_HELICANNON_1STPERSON}, m_PlayerWeaponMode.m_nMode)`).
- Original bytes (0x7424F8-0x74251A): firedBy non-null, `firedBy->+0x598 (pedType) == 0`, and `TheCamera.m_PlayerWeaponMode.m_nMode
  (i16 0xB6F858)` is 0x22 or 0x2D -> FireM16_1stPerson(firedBy).
- Verdict: REFUTED (same predicate). Class: harmless.
Row Weapon.cpp:1830 (assert(firedByPed)). Not present in the bytes; not behavioural. Verdict: REFUTED. Class: harmless.
Row Weapon.cpp:1841 (default: NOTSA_UNREACHABLE()).
- Original bytes (0x742794): for unhandled types (16..43 entries with table value 8, i.e. 19, 20, 21, and any type < 16 or > 43) the
  original sets the result register `bl = *(u8*)&targetPosn` (low byte of the targetPosn argument pointer, `mov bl,[esp+0x60]`) and
  continues into the common post-fire code at 0x742798 (taken as "fired" when that byte is non-zero).
- Verdict: CONFIRMED (gr treats the case as unreachable; the original has defined, if odd, behaviour). Class: keep-original -
  a port must evaluate `fired = (uint8)(uintptr_t)targetPosn != 0` for these types (pointer low byte; for a null targetPosn it is
  false). Open question 0x742794: whether any caller reaches Fire with these types (callers not audited here).
