# CWorld: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x4072a0 CWorld::GetRepeatSector  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.h:411` (original-comment): "Original code uses `&` instead of `%` to also clear the sign bit." gr then implements it as `static_cast<size_t>(std::abs(coord)) % 16`.
  - Original bytes: body at 0x156c6b0 (jmp stub 0x4072a0). `eax = y & 0xF` (0x156c6b8), `eax <<= 4`, `ecx = x & 0xF` (0x156c6be), `eax += ecx`, result `&ms_aRepeatSectors[0xb992b8] + eax*12` (0x156c6c3/0x156c6c6). Index = `(y & 15) * 16 + (x & 15)`, a two's-complement mask.
  - Verdict: CONFIRMED. The comment describes the original correctly, but gr's replacement is not equivalent for negative coordinates: `-1 & 15 = 15` in the original, `abs(-1) % 16 = 1` in gr (and `abs(INT_MIN)` is UB in gr). Negative sector coordinates (positions below world origin by repeat-sector size) select a different repeat sector.
  - Class: keep-original. Port: `((y & 15) << 4) + (x & 15)` on the raw i32 bit pattern, element stride 12 bytes (CRepeatSector = 3 list heads).

## 0x563a80 CWorld::CallOffChaseForAreaSectorListVehicles  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
Args (cdecl, bytes): list +4 (slot reused as the local "sphere hit" bool, 0x563b8e), minX +8, minY +0xc, maxX +0x10, maxY +0x14, biggerMinX +0x18, biggerMinY +0x1c, biggerMaxX +0x20, biggerMaxY +0x24.
- gr `source/game_sa/World.cpp:551` (original-comment): "R* used a bool variable and then, after the loop they did this and didn't break after the sphere has been found." gr uses `std::ranges::any_of` (stops at the first hit).
  - Original bytes: `hit = 0` (0x563b8e); loop over all `numSpheres` (int16 at colData+0, re-read every iteration, signed, 0x563c2c-0x563c36), for each: `c = CMatrix::operator*(veh->m_matrix, &sphere.center)` (0x59c890; sphere stride 0x14, radius at +0xc); `hit = 1` if `c.x + r > minX && c.x - r < maxX && c.y + r > minY && c.y - r < maxY` (strict, NaN fails; 0x563bcd-0x563c27). After the loop, `if (hit)` adjust the speed (0x563c3c).
  - Verdict: REFUTED. The extra iterations only call the pure matrix*vector helper; the result is the same boolean.
  - Class: harmless.
- Found in passing:
  1. Position filter bounds (keep-original, CONFIRMED): the original tests the vehicle position against the "bigger" arguments, `pos.x > biggerMinX` (0x563ade-0x563ae9), `pos.x < biggerMaxX` (0x563af9-0x563b04), `pos.y > biggerMinY` (0x563b14-0x563b20), `pos.y < biggerMaxY` (0x563b32-0x563b3e), all strict. gr tests against minX/minY/maxX/maxY (World.cpp:506). Same result only when the caller passes identical rectangles.
  2. No null check on `colData` (`mov ebp,[eax+0x2c]; cmp word [ebp],0`, 0x563b98-0x563b9d): the original dereferences it; gr skips when null (harmless unless a vehicle without col data reaches this code).
  3. Speed clamp (matches gr except NaN position): `mid = (minX + maxX) * 0.5f [0x858b8c]`; if `pos.x > mid` (ordered) `speed.x = (0.0 > speed.x) ? 0.0 : speed.x`, else (including NaN pos) `speed.x = (0.0 < speed.x) ? 0.0 : speed.x` (0x563c55-0x563c96); same for y with minY/maxY (0x563ca1-0x563ce1). gr sends a NaN position to the `max` branch, the original to the `min` branch.
  - Filters before the spheres (match gr): scan code, not FindPlayerVehicle(-1, 0) (0x563ac1), `byte [veh+0x428] & 1` law enforcer, car mission `byte [veh+0x3ba]` in {2,3,4,5,0x3d,0x3c}; then `byte [veh+0x3bb] = 1`, `dword [veh+0x3bc] = CTimer::m_snTimeInMilliseconds + 2000` (0x563b73-0x563b88).

## 0x564050 CWorld::ShutDown  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/World.cpp:687` (original-comment): "This code is mostly similar to the original".
  - Original bytes: deletion passes in the same order as gr: LOD lists (0xb99eb8 .. 0xb9acc8, 0x56405a-0x5640a2), repeat sectors (vehicles, peds, objects; 0xb992b8 .. 0xb99eb8, 0x5640b0-0x56416b), sectors (buildings, dummies; up to 0xb992b8, 0x564180-0x564217). Each entity is removed with `vtbl[+0xc]` (Remove), `CPhysical::RemoveFromMovingList` for physical types, then `vtbl[0](1)`. The sector loop flushes its two lists right away (`CPtrListSingleLink::Flush` 0x552400, `CPtrListDoubleLink::Flush` 0x552470). Then the "not empty" checks: `sprintf(gString 0xb71670, fmt, x, y)` (0x821bb5; x = i % 120, y = i / 120 via `idiv 0x78`) + Flush for non-empty lists, then flush `ms_listMovingEntityPtrs` (0xb9acc8) and `ms_listObjectsWithControlCode` (0xb9accc), then `Players[0..1].m_PlayerData.DeAllocateData()` (0x56f890, stride 0x190).
  - Verdict: REFUTED for the comment (no behavioural content).
  - Class: harmless.
- Found in passing (keep-original, CONFIRMED): the original calls `CIplStore::Shutdown()` (0x405fa0) first (0x564055); gr's CWorld::ShutDown does not.

## 0x565000 CWorld::FindObjectsKindaCollidingSectorList<CPtrListDoubleLink<CPhysical*>>  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:988` (notsa): `if (*outCount >= maxCount) return; // NOTSA` at the top of each iteration.
- gr `World.cpp:1010` (notsa): "`outCount` checked above already"; gr then stores and increments unconditionally.
  - Original bytes: no early exit. For every node (`entity = node[0]`, `node = node[4]` read first, 0x565020-0x56502d): skip if scan code current; stamp scan code (`[e+0x2c] = ms_nCurrentScanCode`, 0x56503d); `c = GetBoundCentre()` (0x534250); `d = point - c`; 2D: `sqrt(dy*dy + dx*dx)`, 3D: `sqrt((dz*dz + dy*dy) + dx*dx)` (0x565056-0x565084); `R = colModel->boundRadius (+0x24) + radius`; skip unless `d < R` (ordered, `test ah,5; jp`, 0x56509f-0x5650a6); then `if (*outCount < maxCount)` (signed int16, `jge` skip, 0x5650a8-0x5650b0) `{ if (out) out[*outCount] = e; ++*outCount; }`.
  - Verdict (both rows): CONFIRMED. When the output is full the original keeps walking the list, so it still stamps the scan code on (and evaluates) every remaining entity; gr returns and leaves them unstamped, which changes what later sector lists of the same query (and other users of the same scan code) see.
  - Class: keep-original.

## 0x565300 CWorld::FindMissionEntitiesIntersectingCubeSectorList<CPtrListDoubleLink<CPhysical*>>  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 2, REFUTED 1, UNCLEAR 0).
Original signature (cdecl, bytes): `(list +4, const CVector* cornerA +8 [EDI], const CVector* cornerB +0xc [EBP], int16* outCount +0x10, int16 maxCount +0x14, CEntity** out +0x18, bool vehicles +0x1c, bool peds +0x20, bool objects +0x24)`.
- gr `source/game_sa/World.cpp:1077` (original-comment): "seems like namespaces weren't a thing in C++03". Verdict: REFUTED (remark, no code). Class: harmless.
- gr `World.cpp:1080` (notsa): `const CBoundingBox bb{cornerA, cornerB};` ... `if (bb.IsPointWithin(entity->GetPosition()))`.
  - Original bytes: the test includes the entity's collision bounding-sphere radius `r = ms_modelInfoPtrs[entity->m_nModelIndex]->m_pColModel(+0x14)->radius(+0x24)` (0x56537a-0x565388). Position = matrix (+0x14) ? &matrix->pos (+0x30) : &placement.pos (+4). Six tests, each `fld pos; fadd/fsub r; fcomp corner`, entity skipped when:
    1. `pos.x + r < A.x` (`test ah,1; jne`, 0x56538b-0x565394; NaN also skips),
    2. `pos.x - r > B.x` or unordered (`test ah,0x41; jp`, 0x5653a4-0x5653b1),
    3. `pos.y + r < A.y` (0x5653c1-0x5653cf), 4. `pos.y - r > B.y` (0x5653db-0x5653e9),
    5. `pos.z + r < A.z` (0x5653f5-0x565403), 6. `pos.z - r > B.z` (0x56540f-0x56541d).
    x87: `fld pos.c` (f32), `fadd/fsub r` (f32), compare against f32 corner.
  - Verdict: CONFIRMED. The original is a sphere-vs-box overlap (pos expanded by r on each axis, inclusive bounds), gr tests the bare point and ignores the radius.
  - Class: keep-original.
- gr `World.cpp:1113` (notsa): `else { break; // NOTSA }` inside `if (*outCount < maxCount) { if (outEntities) outEntities[*outCount++] = entity; }`.
  - Original bytes: `if (*outCount < maxCount)` (signed 16-bit, `jge` skip, 0x565423-0x56542b) `{ if (out) out[*outCount] = entity; ++*outCount; }` (0x56542d-0x56543b); when full it just continues with the next node (0x56543e), no break.
  - Verdict: CONFIRMED. Three differences: (a) no break in the original, so scan codes (+0x2c = ms_nCurrentScanCode 0xb7cd78, set at 0x565335 before the filters) are still stamped on the remaining entities of the list; (b) the original increments the count also when `out` is null; (c) gr's `outEntities[*outCount++]` post-increments the pointer, not the count (gr bug), so gr never advances the count. Port the original.
  - Class: keep-original.
- Filters (match gr): vehicles flag: `byte [e+0x4a4] == 2` (0x565341); else peds flag: `byte [e+0x484] == 2` (0x565352); else object type `byte [e+0x13c]` is 2 or 6 (0x56535b-0x565365). Node walk: `entity = node->item [+0]`, `node = node->next [+4]` read before the body (0x56532c).

## 0x565610 CWorld::SetPedsOnFire  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1186` (original-comment): "NOTSA - Originally it was some abs() macro crap"; gr replaces it with an inclusive `CBoundingBox{(x-r, y-r, z-5), (x+r, y+r, z+5)}.IsPointWithin(pos)`.
  - Original bytes: descending ped loop (as gr). Filters: `m_nPedState (+0x530) != 0x37`, `!(dword [ped+0x46c] & 0x100)` (in vehicle), `m_pFire (+0x730) == 0`, `!(physicalFlags (+0x40) & 0x80000)` (fireproof). Then three abs tests, z first: `d = pos.z - z; a = (d < 0) ? -d : d` (`fcomp 0.0; test ah,5; jp` picks `fchs` only when ordered-less; NaN keeps d), require `a < 5.0f [0x858c80]` (strict; `jp` skip, 0x5656a6-0x5656e8). `k = (ped == fireCreator) ? 0.5f : 1.0f`; `lim = k * radius` (stored f32); x: `|pos.x - x| < lim` (0x56570c-0x565755); y: `|pos.y - y| < lim` (0x565765-0x5657a5). All strict.
  - Verdict: CONFIRMED. Boundaries are exclusive in the original (gr inclusive), and the original compares `|p - c| < r` while gr compares against precomputed `c - r` / `c + r` (different rounding).
  - Class: keep-original.
- Found in passing (keep-original, CONFIRMED): the invulnerability test is inverted in gr. Original (0x5657a7-0x5657c7): if `!(physicalFlags & 0x400000)` (NOT bInvulnerable) start the fire; else start it only if `fireCreator == null` or (`fireCreator` type == 3 (ped) and `IsPlayer()`). gr (World.cpp:1197): `bInvulnerable || !fireCreator || creatorIsPlayer`. Fire: `gFireManager.StartFire(ped, fireCreator, 0.8f, 1, 7000, 2)` (0x53a050).

## 0x565800 CWorld::SetPedsChoking  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1212` (original-comment): abs() test replaced by an inclusive bounding box `(x-r, y-r, z-5)..(x+r, y+r, z+5)`.
  - Original bytes: descending ped loop (`dec edi`, 0x56582f). Filters: ped state != 0x37, not in vehicle (`+0x46c & 0x100`), fireproof/fire tests as in gr, created-by `byte [ped+0x484] != 2` (0x56587f). Abs tests in the order z (`|pos.z - z| < 5.0f`, 0x5658a0-0x5658d8), x (`|pos.x - x| < radius`, 0x5658f1-0x565925), y (0x56593f-0x565975), each `a = (d < 0) ? -d : d` then strict `<`. Hit: `dir = VectorSub(pos, (x,y,z))` (0x40fe60), `CPed::GetLocalDirection(&dir.xy)` (0x5def60), `CWeapon::GenerateDamageEvent(ped, gasCreator, 0x11 (teargas), 1, 3 (torso), localDir)` (0x73a530).
  - Verdict: CONFIRMED (strict vs inclusive bounds, and `|p - c| < r` vs `c - r <= p`).
  - Class: keep-original.

## 0x5659f0 CWorld::SetCarsOnFire  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1242` (original-comment): abs() test replaced by an inclusive bounding box.
  - Original bytes: descending vehicle loop (`dec esi`, 0x565a18). Filters: `(byte [veh+0x36] & 0xf8) != 0x28` (status 5 wrecked), `m_pFire (+0x490) == 0`, `!(physicalFlags (+0x40) & 0x80000)` (fireproof) (0x565a31-0x565a55). Abs tests z (`< 5.0f`), x (`< radius`), y (`< radius`), each strict with `a = (d < 0) ? -d : d` (0x565a68-0x565b43). Hit: `gFireManager.StartFire(veh, fireCreator, 0.8f, 1, 7000, 100)`.
  - Verdict: CONFIRMED (strict vs inclusive bounds and different rounding, as in SetPedsOnFire).
  - Class: keep-original.

## 0x565b70 CWorld::SprayPaintWorld  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1290` (original-comment): "Original code has U.B. if `processTagAlphaState` is false, because `newAlpha` isn't assigned a meaningful value"; gr initialises `newAlpha = 0`.
  - Original bytes: the "new alpha" lives in EDI. Before the loop EDI is loaded from the stack slot that holds the object count (`movzx edx,ax; mov [esp+0x14],edx` at 0x565bbf/0x565bc7, then after `push edi` `mov edi,[esp+0x18]` at 0x565bcc reads the same slot). Per tag: `if (processTagAlphaState)` (BL, 0x565c27) `edi = min((int16)TAG_SPRAYING_INCREMENT_VAL[0x8cdef0] + curAlpha, 255)` (signed compare `jg`, 0x565c2f-0x565c48); otherwise EDI is left as is. So with the flag false the first tag gets alpha = (uint8)count (number of objects found, 1..15) and later tags get the same value. `if ((u16)curAlpha < 255 && (u16)edi == 255) painted = 1` (0x565c4a-0x565c57); `CTagManager::SetAlpha(entity, edi)` (0x49cec0, low byte used).
  - Verdict: CONFIRMED. The behaviour is defined by the bytes (not random UB): alpha = object count when the flag is false; gr writes 0. The only known caller passes true, so this is a latent difference.
  - Class: keep-original.
- Found in passing: the increment is read from the int16 global `TAG_SPRAYING_INCREMENT_VAL` at 0x8cdef0 (.data, initial value 8) on every iteration, not a compile-time constant as in gr (`constexpr int16 SPRAY_ALPHA_CHANGE = 8`). Harmless unless something writes 0x8cdef0. Return values: 2 if any tag reached 255, else 1 if any tag was found (byte [esp+3]), else 0 (0x565c80-0x565ca6). FindObjectsInRange (0x564a20) is called with (posn, radius, false, &count, 15, objects, true, false, false, false, false).

## 0x5660b0 CWorld::UseDetonator  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1396` (pool-iteration): `for (auto& veh : GetVehiclePool()->GetAllValid())` arming remote bombs placed by `creator`.
  - Original bytes: vehicle pool 0xb74494, DESCENDING loop (`dec esi` first, 0x5660d7, until 0, 0x566128), skip free/null; same per-vehicle work as gr, including `creator->RegisterReference(&veh->m_pWhoDetonatedMe)` (0x571b70, 0x566121); then `CProjectileInfo::RemoveDetonatorProjectiles()`.
  - Verdict: CONFIRMED order differs. The state written per vehicle is independent, but RegisterReference can prune references and pops entries off `CReferences::pEmptyList`, so the order in which reference nodes are allocated (and pruned) follows the slot order.
  - Class: keep-original (iterate from size-1 down to 0; low impact).

## 0x566610 CWorld::ClearCarsFromArea  (gr_deviations = 1)
Rows checked: 2 (CONFIRMED 0, REFUTED 2, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1568` (notsa, deviates=no): `CBoundingBox box{min, max}` + `IsPointWithin`. REFUTED: the original's six compares are the same inclusive test `min <= p <= max` per axis in the order x, y, z (`test ah,1` / `test ah,0x41; jp`, 0x566695-0x566709; NaN fails), and gr's IsPointWithin is inclusive too (only the evaluation order of the pure compares differs). Class: harmless.
- gr `World.cpp:1569` (pool-iteration): `for (auto& veh : GetVehiclePool()->GetAllValid())`.
  - Original bytes: ASCENDING loop (`inc ebp; cmp ebp,ebx; jl 0x566640`, 0x5667cc-0x5667dd; size cached), skip free/null. Filters in order: `FindPlayerPed(-1)->m_pContactEntity (+0x584) == veh && veh->m_nVehicleType (+0x590) == 5` -> skip; box test; `byte [veh+0x428] & 8` (locked) -> skip; `CVehicle::CanBeDeleted` (0x6d1180). Removal inline: driver (+0x460) -> `CPopulation::RemovePed`, `CleanUpOldReference` if still set, driver = 0; passengers `i < byte [veh+0x488]` (re-read each iteration) at +0x464: `RemovePassenger` then `RemovePed`; `if (CCarCtrl::IsThisVehicleInteresting) CGarages::StoreCarInNearestImpoundingGarage`; `CCarCtrl::RemoveFromInterestingVehicleList`; `CWorld::Remove`; virtual delete `vtbl[0](1)`.
  - Verdict: REFUTED (ascending in the original too).
  - Class: harmless.

## 0x5667f0 CWorld::ClearPedsFromArea  (gr_deviations = 1)
Rows checked: 2 (CONFIRMED 0, REFUTED 2, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1588` (notsa, deviates=no): bounding-box helper. REFUTED: original inclusive per-axis test x, y, z after `!IsPlayer()` (0x5df8f0) and `CanBeDeleted()` (0x5df100), same as gr (0x566860-0x5668c0). Class: harmless.
- gr `World.cpp:1589` (pool-iteration): `for (auto& ped : GetPedPool()->GetAllValid())`.
  - Original bytes: ASCENDING loop over the ped pool (`inc edi; cmp edi,eax; jl 0x566810`, 0x5668cf-0x5668d8; size cached), `CPopulation::RemovePed(ped)` (0x610f20) for matches.
  - Verdict: REFUTED (ascending in the original too).
  - Class: harmless.

## 0x5668f0 CWorld::SetAllCarsCanBeDamaged  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1600` (pool-iteration): `for (auto& veh : GetVehiclePool()->GetAllValid()) veh.vehicleFlags.bCanBeDamaged = enable;`
  - Original bytes: vehicle pool 0xb74494, loop `ecx = 0 .. size-1` ascending (`inc ecx; cmp ecx,ebp; jl`, 0x566933-0x56693c; `jle` guard on size, signed, 0x5668ff), skip free (`js` on flag byte, 0x566917) and null address (0x56691d). Sets bit 0x20 of byte +0x42a to bit 0 of `enable` (`bl = enable << 5; bl ^= b; bl &= 0x20; b ^= bl`, 0x56691f-0x56692d).
  - Verdict: REFUTED. The original also walks ascending; per-entity writes are independent anyway.
  - Class: harmless.

## 0x566950 CWorld::ExtinguishAllCarFiresInArea  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1607` (pool-iteration): `for (auto& veh : GetVehiclePool()->GetAllValid()) if (DistanceBetweenPointsSquared(point, pos) <= sq(radius)) veh.ExtinguishCarFire();`
  - Original bytes: the vehicle pool pointer comes from an obfuscated load (stub 0x401fed). ASCENDING loop (`inc ecx; cmp ecx,edx; jl 0x566980`, 0x566a28-0x566a39; size cached), skip free/null. `d2 = ((p.z-pos.z)*(p.z-pos.z) + (p.y-pos.y)*(p.y-pos.y)) + (p.x-pos.x)*(p.x-pos.x)` (0x5669cf-0x566a00), `r2 = radius*radius`; `fcompp` + `test ah,0x41; jne` skip (0x566a0a-0x566a11): extinguish only if `r2 > d2`.
  - Verdict: REFUTED for the pool-iteration row (ascending in the original too). Class: harmless.
- Found in passing (keep-original, CONFIRMED): the original condition is strict `d2 < r2`; gr uses `<=`. Differs only at exact equality (and both reject NaN).

## 0x566c10 CWorld::StopAllLawEnforcersInTheirTracks  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1631` (pool-iteration): `for (auto& veh : GetVehiclePool()->GetAllValid()) if (bIsLawEnforcer) veh.m_vecMoveSpeed = {};`
  - Original bytes: descending loop (`dec eax` first, 0x566c33; until 0, 0x566c7e), skip `flags & 0x80` (0x566c3a) and null address (0x566c44); if `byte [veh+0x428] & 1` (0x566c46) store 0 to m_vecMoveSpeed x,y,z (+0x44/+0x48/+0x4c) via a zeroed stack temporary (0x566c4f-0x566c7b).
  - Verdict: CONFIRMED order differs (descending), but writes are per entity and independent.
  - Class: harmless.

## 0x566c90 CWorld::FindUnsuspectingTargetCar  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1642` (pool-iteration): `for (auto& veh : GetVehiclePool()->GetAllValid())`, best distance starts at `std::numeric_limits<float>::max()`.
  - Original bytes: descending loop over the vehicle pool (0xb74494): `al = flags[esi-1]; dec esi` (0x566cc0-0x566cc4), skip free (`js`) and null address. Filters in order: `dword [veh+0x594]` (vehicle sub-type) must be 0 = automobile (0x566cde), `byte [veh+0x4a4]` (created-by) must be 1 = random (0x566cec), status `byte [veh+0x36] >> 3` must be 2 or 3 (0x566cf9-0x566d05). 2D distance `sqrt(dy*dy + dx*dx)` with `dx = point.x - pos.x`, `dy = point.y - pos.y` (0x566d18-0x566d2f), stored f32 (`fst [esp+0x14]`); accepted only if `d < best` (ordered; `test ah,5; jp` skip, 0x566d3d). Then `dot = ((pp.z-pos.z)*fwd.z + (pp.y-pos.y)*fwd.y) + (pp.x-pos.x)*fwd.x` with fwd = matrix +0x10..+0x18 read from the matrix pointer without a null check (0x566d4c-0x566d6f); skip if `dot <= 0.0 [0x858b50]` or unordered (`test ah,0x41; jne`, 0x566d79). Accept: best = reload d from [esp+0x14], result = veh.
  - Initial best: `fld 9999.9f [0x864f90]` (0x566c93), not FLT_MAX.
  - Verdict: CONFIRMED. (a) Original iterates descending, so on equal distances the higher slot wins (strict `<` keeps the first found); gr picks the lower slot. (b) Initial best is 9999.9f (cars at 2D distance >= 9999.9 are never returned). Filter order (sub-type before created-by) has no observable effect.
  - Class: keep-original.

## 0x566da0 CWorld::FindUnsuspectingTargetPed  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1674` (pool-iteration): `for (auto& ped : GetPedPool()->GetAllValid())`, best starts at FLT_MAX.
  - Original bytes: ped pool 0xb74490, descending loop (`dec edi` after reading `flags[edi-1]`, 0x566dd3-0x566dd7), skip free/null. Initial best = 9999.9f (`mov [esp+8], 0x461c3f9a`, 0x566db6). Filters in order: `byte [ped+0x484] == 1` (created by game, 0x566df1); `CPed::IsAlive` (0x5e0170); ped type `dword [ped+0x598]`: accepted if `7 <= t <= 16` (signed) or `t == 4` (0x566e0d-0x566e20); active task `CTaskManager::GetActiveTask(&ped->m_pIntelligence(+0x47c)->taskManager(+4))` (0x681720): if non-null and `task->GetTaskType()` (vtable +0x10) `== 0x390` (TASK_COMPLEX_WANDER) the ped is SKIPPED (0x566e3f-0x566e44). Distance `d = sqrt(dy*dy + dx*dx)` with `dx = point.x - pos.x` (0x566e57-0x566e6e); accepted if `d < best` (ordered). `dot = ((pp.z-pos.z)*fwd.z + (pp.y-pos.y)*fwd.y) + (pp.x-pos.x)*fwd.x` with fwd at matrix +0x10 (matrix pointer used without null check); skip if `dot <= 0` or unordered (0x566eae-0x566eb9). Accept: best = d (`fstp` f32), result = ped.
  - Verdict: CONFIRMED. Descending order (ties go to the higher slot) and initial best 9999.9f, as in FindUnsuspectingTargetCar.
  - Class: keep-original.
- Found in passing (keep-original, CONFIRMED):
  1. Task test is inverted in gr. gr (World.cpp:1681-1685) skips peds whose active task is NOT TASK_COMPLEX_WANDER; the original skips peds whose active task IS TASK_COMPLEX_WANDER (912 = 0x390) and accepts peds with no active task or any other task.
  2. Ped-type test. gr: skip if `(t <= GANG1 || t >= GANG10) && t != CIVMALE`, i.e. accepts types 8..15 and 4. Original accepts 7..16 (GANG1..GANG10 inclusive) and 4.

## 0x567750 CWorld::TriggerExplosionSectorList<CPtrListDoubleLink<CPhysical*>>  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/World.cpp:1989` (notsa): for an MI_ATM entity whose creator is the player gr has `assert(0); // NOTSA` and leaves the original statement `m_fHealth -= 2 * radiusProgress2x` commented out.
  - Original bytes: `if (entity->m_nModelIndex == MI_ATM (u16 0x8cd710, zero-extended) && creator (EBP) == FindPlayerPed(-1))` then `obj->m_fHealth (+0x154) = obj->m_fHealth - (L + L)` where L is the f32 stack local `[esp+0xc]` (0x567928-0x567951). L is only written further down (0x567986/0x56798e and 0x567a17/0x567a1f: `min(1.0f, ((radius - dist) / radius) * 2)`), so at this point it holds the value computed for a PREVIOUS entity of this call, or uninitialised stack contents if no earlier entity reached those stores.
  - Verdict: CONFIRMED (the original executes the health change; gr does nothing in release and aborts in debug). Open question: the initial content of `[esp+0xc]` on the first use in a call (address 0x567945) cannot be determined from this function alone.
  - Class: keep-original.
- Found in passing (keep-original, CONFIRMED): the static-object damage call. Original (0x567965-0x5679ad): `f = ((radius - dist) / radius); f += f; L = (f < 1.0f) ? f : 1.0f` (ordered, else 1.0), then `CObject::ObjectDamage(L * 300.0f [0x858fd8], 0, 0, creator, 0x33)` (0x5a0d90). gr (World.cpp:1995) passes the unscaled `min(1, 2*(r-d)/r)` (no `* 300`) and computes `2*(r-d)/r` in a different order.

## 0x569750 CWorld::FindRoofZFor3DCoord  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/World.cpp:2618` (original-comment "R* triggered"): `NOTSA_LOG_DEBUG("THERE IS NO MAP BELOW ...")` when the line hits nothing and `outResult` is null.
  - Original bytes: `ProcessVerticalLine(&(x,y,z), 1000.0f, &cp, &ent, 1,0,0,0,1,0,0)` (0x5674e0). Miss with `outResult == null`: `printf("THERE IS NO MAP BELOW THE FOLLOWING COORS:%f %f %f. (FindGroundZFor3DCoord)\n", (double)x, (double)y, (double)z)` (CRT 0x821982, format 0x864fa0) then return 20.0f [0x858ba4]. Miss with outResult: `*outResult = 0`, return 20.0f. Hit: `*outResult = 1` if non-null, return `cp.m_vecPoint.z` (ST0).
  - Verdict: REFUTED (gr keeps the message as a debug log; the original prints to stdout, which has no game-visible effect).
  - Class: harmless.

## 0x56a0d0 CWorld::ClearExcitingStuffFromArea  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 0, REFUTED 2, UNCLEAR 0).
- gr `source/game_sa/World.cpp:2788` (pool-iteration): ped loop removing non-follower peds within the 2D radius.
- gr `World.cpp:2799` (pool-iteration): vehicle loop removing vehicles without player-group followers.
  - Original bytes: both loops are ASCENDING (`inc edi; ... jl 0x56a100`, 0x56a1b4-0x56a1c1, ped pool 0xb74490; `inc ecx; cmp ecx,eax; jl 0x56a1e6`, 0x56a3c5-0x56a3d6, vehicle pool), with the size cached before each loop. Differences from gr's text are only in evaluation placement: the original calls `CPedGroups::GetPedsGroup(FindPlayerPed(-1))` inside each iteration (0x56a188/0x56a20b) instead of once; both calls are side-effect free. Ped test order: `!IsPlayer`, `CanBeDeleted`, `radius*radius > (dx*dx + dy*dy)` (2D, strict, `test ah,0x41; jne`), not a follower. Vehicle: any passenger (`i < byte [veh+0x488]`, +0x464) that is a follower -> skip, before the boat/contact test.
  - Verdict (both): REFUTED (same ascending order; no behavioural change).
  - Class: harmless.
- Note: the vehicle removal is inline: driver/passengers via `CPopulation::RemovePed`, impound/interesting-list handling, `vtbl[+0xc]` (Remove), `CPhysical::RemoveFromMovingList` when the entity type is 2..4 (0x56a39d-0x56a3ac), `vtbl[0](1)`. Then DeleteAllTempObjectsInArea, gFireManager.ExtinguishPoint, ExtinguishAllCarFiresInArea, CExplosion::RemoveAllExplosionsInArea, optional RemoveAllProjectiles + CShadows::TidyUpShadows, CPickups::RemoveUnnecessaryPickups, as in gr.
