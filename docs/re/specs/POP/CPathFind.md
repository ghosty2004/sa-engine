# CPathFind (POP) - full specs (group P4)

ThePaths = 0x96F050 (all functions below are `__thiscall` on it unless stated). Fields (gr PathFind.h): +0x004 m_pathFindHashTable[512],
+0x804 m_pPathNodes[72] (CPathNode*), +0x924 m_pNaviNodes[72] (CCarPathLink*, stride 0xE), +0xA44 m_pNodeLinks[72] (CNodeAddress*),
+0xB64 m_pLinkLengths[72] (u8*), +0xC84 m_pPathIntersections[72] (u8*), +0xDA4 m_pNaviLinks[64] (u16 link address: area = v >> 10,
index = v & 0x3FF), +0xFA4 m_anNumNodes[72], +0x10C4 m_anNumVehicleNodes[72], +0x11E4 m_anNumPedNodes[72], +0x3544 hash-table count.
CNodeAddress = dword: low word area (0xFFFF = invalid), high word node index. CPathNode (0x1C): +0x00 m_next, +0x04 m_prev, +0x08/+0x0A/+0x0C
pos x/y/z (i16, world = v * 0.125f [0x858C48]), +0x0E m_totalDistFromOrigin (i16, 0x7FFE = not visited), +0x10 m_wBaseLinkId (i16),
+0x12 area (u16), +0x14 node (u16), +0x16 m_nPathWidth (u8), +0x17 m_nFloodFill (u8), +0x18 byte0 (bits 0-3 m_nNumLinks, bit 5
m_isSwitchedOff, bit 7 m_bWaterNode), +0x19 byte1 (bit 1 temporary "already returned" mark, bit 2 m_bDontWander), +0x1A byte2 (bits 4-7
m_nBehaviourType). CCarPathLink (0xE): +0x04/+0x06 attachedTo (area, node), +0x0B lanes (bits 0-2 same direction, bits 3-5 opposite).
Region of a coordinate = `clamp(ftol((c - -3000.0f [0x859A90]) * 0.0013333333f [0x859A8C]), 0, 7)` (signed; see FindRegionForCoors
0x44D830 in CPathFind.deviations.md); region id = rx + 8*ry. `_ftol` 0x821B40, `_rand` 0x821B1E. gta-reversed: `plugin::Call*` for all
functions here except DoPathSearch (unhooked C++, see its section).

## 0x44ECA0 CPathFind::GeneratePedCreationCoors_Interior  (catalogue: specified/REA, port level 9)
- Convention / signature: `__thiscall bool (float x [+4], float y [+8], CVector* outCoords [+0xC], CNodeAddress* unused1 [+0x10],
  CNodeAddress* unused2 [+0x14], float* outOrientation [+0x18])`, returns AL, `ret 0x18`. The arg slot of outCoords is reused as the
  FindGroundZFor3DCoord `bool* found` target (0x44EF26; the pointer itself was saved in EBX first).
- Callers: CPopulation::PopulateInterior 0x616470. Callees: FindRegionForCoors 0x44D830, `rand`, CWorld::FindGroundZFor3DCoord 0x5696C0.
- Behaviour:
  1. `r = FindRegionForCoors(x, y)`. Repeat 200 times (counter `cmp 0xC8; jl`, 0x44EF90):
     a. If `m_pPathNodes[r] == NULL` or `m_anNumPedNodes[r] == 0`: next attempt (no rand consumed).
     b. `idx = ((rand() >> 6) (arithmetic) % m_anNumPedNodes[r]) + m_anNumVehicleNodes[r]` (signed `idiv`, 0x44ECF3); N = nodes[idx].
     c. `nx = N.x*0.125f`, `ny = N.y*0.125f`; continue only if `(ny - y)*(ny - y) + (nx - x)*(nx - x) < 2500.0f [0x8598B0]` (y term first,
        0x44ED3D-0x44ED6E; NaN fails) and `N.z*0.125f > 900.0f [0x858978]` (0x44ED8E, NaN fails) and numLinks != 0.
     d. For link l = 0 .. numLinks-1: `a = m_pNodeLinks[r][N.base + l]`; skip if `a.area >= 0x40` (unsigned, 0x44EDD0) or not loaded;
        L = node(a). Up to 5 tries (0x44EF65):
        - `t = (float)(rand() & 0xFF) * 0.00390625f [0x859AA0]`; `*outOrientation = t` (the orientation output receives the
          interpolation factor, 0x44EE25).
        - `s = 1.0f - t` (f32 0x44EEC5). `P.x = N.x*0.125*s + L.x*0.125*t`, `P.y = N.y*0.125*s + L.y*0.125*t`, `P.z = N.z*0.125*s + L.z*0.125*t`
          (each product stored f32, then the two products added, 0x44EE6D-0x44EF01); `*outCoords = P`.
        - `z = FindGroundZFor3DCoord(P.x, P.y, P.z + 2.0f [0x858CA0], &found, NULL)`; if found and `|z - outCoords->z| < 3.0f [0x858B3C]`:
          `outCoords->z = z` (f32), return 1.
  2. Return 0.
- gta-reversed: `plugin::CallMethodAndReturn` (PathFind.cpp:257). Open questions: none.

## 0x44F460 CPathFind::FindNodeClosestToCoors  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall CNodeAddress* (CNodeAddress* out [+4], CVector pos [+8..+0x10], uint8 nodeType [+0x14],
  float maxDistance [+0x18], bool bLowTraffic [+0x1C], int32 unused [+0x20], bool bUnkn [+0x24], bool bBoatsOnly [+0x28],
  bool bIgnoreInterior [+0x2C])`, returns EAX = out, `ret 0x2C`. Arg slot +0x18 is reused for the border distance m and +0x2C as the
  interior loop counter.
- Callers: 16 (CCarCtrl::GenerateCarCreationCoors2, CGangWars::CreateDefendingGroup, FindNthNodeClosestToCoors, DoPathSearch,
  FindNextNodeWandering, script commands, CWorld::RemoveFallenPeds/Cars, ...). Callees: FindNodeClosestInRegion 0x44F2C0 (`__thiscall
  (CNodeAddress* out, int region, CVector pos, uint8 type, float* inoutDist, bool lowTraffic, bool unkn, bool boats, bool unused)`,
  `ret 0x2C`; updates out/dist only for a strictly better node).
  Every region search below is `FindNodeClosestInRegion(&best, region, pos, nodeType, &bestDist, bLowTraffic, bUnkn, bBoatsOnly, bIgnoreInterior)`.
- Behaviour:
  1. `best = 0xFFFF` (area word), `bestDist = maxDistance`. `rx, ry` = regions of pos.x / pos.y (0x44F463-0x44F4D9).
  2. `m = pos.x - (rx*750.0f [0x859A98] - 3000.0f [0x859A94])`; `m = min(m, (rx+1)*750 - 3000 - pos.x)`; then with
     `pos.y - (ry*750 - 3000)` and `(ry+1)*750 - 3000 - pos.y` (each `min` is `test ah,5; jnp`: keep m only if m < candidate, NaN takes the
     candidate). m is the distance to the nearest border of the own region.
  3. Search region `rx + 8*ry`. If `!(bestDist > m)` (0x44F5CE `test ah,0x41; jne`: <= or NaN) skip to step 5.
  4. Rings k = 1..4 (0x44F82B): with xlo = rx-k, xhi = rx+k, ylo = ry-k, yhi = ry+k, search in this order, each region only if its x and
     y are in 0..7: column xlo for y = ylo..yhi; column xhi for y = ylo..yhi; row ylo for x = xlo+1..xhi-1; row yhi for x = xlo+1..xhi-1.
     Then `m += 750.0f` (stored f32); stop rings when `bestDist < m` (0x44F816 `test ah,5; jnp`).
  5. If `(uint8)bIgnoreInterior == 0`: search interior regions 64..71 (u16 counter, `cmp ax,0x48; jb`) with the last argument 0.
  6. `*out = best`; return out.
- Edge cases: the region-search argument `unused` (+0x20) is never read. Ties keep the first node found (strict compare in 0x44F2C0).
- gta-reversed: `plugin::CallMethodAndReturn` (PathFind.cpp:757); gr names the args unk2..unk6. Open questions: none.

## 0x44F8C0 CPathFind::FindNthNodeClosestToCoors  (catalogue: specified/REA, port level 4)
- Convention / signature: `__thiscall CNodeAddress* (CNodeAddress* out [+4], CVector pos [+8], uint8 nodeType [+0x14], float maxDistance
  [+0x18], bool bLowTraffic [+0x1C], bool bUnkn [+0x20], int32 nthNode [+0x24], bool bBoatsOnly [+0x28], bool bIgnoreInterior [+0x2C],
  CNodeAddress* outNode [+0x30])`, returns EAX = out, `ret 0x30`.
- Callers: CGangWars::CreateDefendingGroup 0x4453D0, CStuckCarCheck::Process 0x465680, CPopulation::CreateWaitingCoppers 0x6133F0, script
  commands. Callees: FindNodeClosestToCoors 0x44F460 (x2).
- Behaviour:
  1. For each area a = 0..71 with `m_pPathNodes[a] != NULL`: nodeType 0 -> range [0, m_anNumVehicleNodes[a]); 1 -> [m_anNumVehicleNodes[a],
     m_anNumNodes[a]); other -> the previous range registers (initially both = nthNode, so empty). Clear bit 1 of `+0x19` of every node in
     the range (`and 0xFD`, 0x44F926).
  2. Repeat nthNode times (signed, `jle` when <= 0): `addr = FindNodeClosestToCoors(pos, nodeType, maxDistance, bLowTraffic, bUnkn,
     1, bBoatsOnly, bIgnoreInterior)` (unk3 slot = bUnkn, which that function ignores; unk4 = 1 makes the region search skip marked nodes);
     if outNode: `*outNode = addr`; if `addr.area == 0xFFFF`: `*out = addr`, return out (0x44FA19). Else set bit 1 of the node's `+0x19`.
  3. Final call `FindNodeClosestToCoors(out, pos, nodeType, maxDistance, bLowTraffic, bUnkn, 1, bBoatsOnly, 0)` (interiors always searched
     here, unlike step 2); return out. outNode is not written by this final call. The marks set in step 2 stay set.
- gta-reversed: `plugin::CallMethod` (PathFind.cpp:283). Open questions: none.

## 0x450320 CPathFind::FindNodeOrientationForCarPlacement  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall float (CNodeAddress addr [+4])` -> ST0, `ret 4`. Entry `sub esp,0x28; jmp 0x40A432` (`movzx edx,
  word [esp+0x2C]` = area, back to 0x450328).
- Callers: CStuckCarCheck::Process 0x465680, script commands 0x483BD0 / 0x496E00. Callees: CVector::Normalise 0x59C910.
- Behaviour:
  1. Return 0.0f [0x858B50] if `m_pPathNodes[addr.area] == NULL` or N (= node(addr)) has 0 links.
  2. `l = 0`; for l in 0 .. numLinks-2 (loop only when numLinks-1 > 0): `v = m_pNaviLinks[addr.area][N.base + l]`; if area `v >> 10` is loaded:
     `c = &m_pNaviNodes[v >> 10][v & 0x3FF]`; `lanes = (c.+4 == addr.area && c.+6 == addr.node) ? (c.+0xB & 0x38) : (c.+0xB & 7)`; stop at
     the first l with lanes != 0. If none, l = numLinks-1.
  3. `t = m_pNodeLinks[addr.area][N.base + l]`; return 0.0f if its area is not loaded. T = node(t).
  4. `d = (T.x*0.125 - N.x*0.125, T.y*0.125 - N.y*0.125, 0)` (each f32), `d.Normalise()`; return `atan2(-d.x, d.y) * 57.295776f [0x859878]`
     (`fld d.x; fchs; fld d.y; fpatan`, degrees, result left in ST0).
- gta-reversed: `plugin::CallMethodAndReturn` (PathFind.cpp:1078). Open questions: none.

## 0x4513F0 CPathFind::FindParkingNodeInArea  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall CVector* (CVector* out [+4], float minX [+8], float maxX [+0xC], float minY [+0x10], float maxY [+0x14],
  float minZ [+0x18], float maxZ [+0x1C])`, returns EAX = out, `ret 0x1C`.
- Callers: CRunningScript::ProcessCommands2000To2099 0x472310. Callees: CPathNode::GetPosition 0x420A10 (thiscall (CVector* out) -> out).
- Globals: static i32 at 0x977B9C (rotating parking index, R/W).
- Behaviour:
  1. count = 0, haveFirst = 0, haveNth = 0. For area 0..63 with nodes != NULL and `m_anNumVehicleNodes[area] > 0`, for each vehicle node N:
     `x = N.x*0.125f`; require `x > minX` (0x45145B `test ah,0x41; jne` rejects <= / NaN) and `x < maxX` (`test ah,5; jp`); same for y
     with minY/maxY and z with minZ/maxZ; require `(N.+0x1A & 0xF0) == 0x20` (behaviour type 2). Then: if count == 0: first =
     N.GetPosition(), haveFirst = 1. If `count == [0x977B9C]`: nth = N.GetPosition(), haveNth = 1. count++.
  2. `s = [0x977B9C] + 1`; `[0x977B9C] = (s < count) ? s : 0` (signed).
  3. `*out = haveFirst ? (haveNth ? nth : first) : (0, 0, 0)`; return out.
- gta-reversed: `plugin::CallMethod` (PathFind.cpp:905). Open questions: none.

## 0x4515D0 CPathFind::DoPathSearch  (catalogue: specified/REA, port level 4)
- Convention / signature: `__thiscall void (uint8 pathType [+4], CVector originPos [+8], CNodeAddress originAddr [+0x14], CVector targetPos
  [+0x18], CNodeAddress* outNodes [+0x24], int16* outCount [+0x28], int32 maxNodes [+0x2C], float* outDistance [+0x30], float
  maxSearchDistance [+0x34], CNodeAddress* targetAddr [+0x38], float maxDepth [+0x3C], bool sameLaneOnly [+0x40], CNodeAddress forbidden
  [+0x44], bool allowWaterTransitions [+0x48], bool forBoats [+0x4C])`, `ret 0x4C`. Frame `push ecx` + 4 registers; several arg slots are
  reused (targetAddr slot = resolved target, originPos.x+... see below).
- Callers: CCarCtrl::PickNextNodeToChaseCar, PickNextNodeToFollowPath, JoinCarWithRoadSystemGotoCoors, ReconsiderRoute (2),
  SteerAICarWithPhysicsFollowPath_Racing, CPathFind::TestCoorsCloseness 0x452000, ComputeRoute 0x452760.
- Callees: FindNodeClosestToCoors 0x44F460 (x2, inline list operations otherwise). Globals: aNodesToBeCleared 0x972CD0 (CNodeAddress[5000]).
- Behaviour:
  1. `this+0x3544 = 0`. Target first: if targetAddr != NULL and `targetAddr->area != 0xFFFF` and loaded: T = *targetAddr; else
     T = FindNodeClosestToCoors(targetPos, pathType, maxSearchDistance, 0, 0, 0, forBoats, 0). If T invalid: fail.
  2. Origin: if `originAddr.area != 0xFFFF` and loaded: O = originAddr; else O = FindNodeClosestToCoors(originPos, same args); invalid: fail.
     "fail" = `*outCount = 0`; if outDistance: `*outDistance = 100000.0f` (imm 0x47C35000); return.
  3. If O == T (area and node words): `*outCount = 0`; if outDistance: `*outDistance = 0` (int 0 = 0.0f); return.
  4. If `O.+0x17 != T.+0x17` (flood fill): fail.
  5. Clear the 512 hash buckets. Inline AddNodeToList(T, 0) (T.+0xE = 0, count++). `aNodesToBeCleared[0] = T.address (+0x12 dword)`,
     cleared = 1, depth = 0, finished = 0.
  6. Loop (bucket b = depth & 0x1FF; the first pass uses b = 0): for each node X in bucket b (following `+0`):
     - if X == O: finished = 1.
     - for link l = 0 .. numLinks(X)-1: `a = m_pNodeLinks[X.area][X.base + l]`; skip if area not loaded; L = node(a); blocked = 0.
       If sameLaneOnly: `v = m_pNaviLinks[X.area][X.base + l]`; if `v >> 10` loaded: c = navi(v); `lanes = (c.+4 == a.area && c.+6 == a.node)
       ? ((c.+0xB >> 3) & 7) : (c.+0xB & 7)`; lanes == 0 -> blocked. If `a == forbidden` (both words) -> blocked.
       If `(L.+0x18 ^ X.+0x18) & 0x80` and !allowWaterTransitions: skip. If blocked: skip.
       `d = X.+0xE + m_pLinkLengths[X.area][X.base + l]` (i16 + u8). If `d < L.+0xE` (signed): if `L.+0xE != 0x7FFE` unlink L
       (`*L.prev = L.next; if L.next: L.next.prev = L.prev; count--`); then if `L.+0xE == 0x7FFE` and cleared < 5000:
       `aNodesToBeCleared[cleared++] = L.address`; link L into bucket `(int16)(d & 0x1FF)` (head insert, prev = bucket address),
       `L.+0xE = (int16)d`, count++.
     - unlink X (count--), continue with `X.next` (read after unlinking).
     After the bucket: depth++; if count == 0 -> end; if `(float)depth > maxDepth` -> end (0x4519A8 `test ah,0x41; je`); if cleared >= 4950
     (0x1356) -> end; if !finished -> next bucket.
     "end" = `*outCount = 0` and go to cleanup (outDistance NOT written).
  7. Finished: `*outCount = 0`; if outDistance: `*outDistance = (float)(int16)O.+0xE`; if outNodes: `outNodes[(*outCount)++] = O.address`.
     Then `X = O`; while `*outCount < maxNodes` (signed) and X != T: for l = 0..numLinks(X)-1: `a = m_pNodeLinks[X.area][X.base + l]`; if
     loaded and `X.+0xE - m_pLinkLengths[X.area][X.base + l] == node(a).+0xE`: `outNodes[(*outCount)++] = a` (no NULL check), X = node(a),
     stop the link loop. If no link matches, X and the count do not change and the loop never ends (original hang; cannot happen with a
     consistent distance field).
  8. Cleanup: for i < cleared: `node(aNodesToBeCleared[i]).+0xE = 0x7FFE` (area assumed loaded).
- gta-reversed: unhooked C++ (PathFind.cpp:297-490, `.Reversed = false`, "sometimes breaks"). Differences from the bytes: gr resolves the
  origin before the target; in the back-trace gr indexes `m_pNodeLinks[area][linkNum]` without the base link id and increments
  outNodesCount twice per step (`for (...; outNodesCount++)` plus the body), and its loop condition is `node == target || count < max`
  (original: `count < max && node != target`). Port from the bytes. Open questions: none.

## 0x451B70 CPathFind::FindNextNodeWandering  (catalogue: specified/REA, port level 4)
- Convention / signature: `__thiscall void (uint8 nodeType [+4], CVector pos [+8], CNodeAddress* origin [+0x14], CNodeAddress* target
  [+0x18], uint8 dir [+0x1C], uint8* outDir [+0x20])`, `ret 0x20`. Arg slots nodeType (+4) and pos.x (+8) are reused for the start address.
- Callers: CTaskComplexWander::UpdatePathNodes 0x669ED0. Callees: CPathNode::GetPosition 0x420A10, FindNodeClosestToCoors 0x44F460.
- Behaviour:
  1. `orig = 0xFFFF` (area word). If origin != NULL and `origin->area != 0xFFFF`: `orig = *origin`; if its area is not loaded:
     `target->area = 0xFFFF`, `*outDir = 0`, return (0x451BAB). Else `P = node(orig).GetPosition()`; `dist2 = (dz*dz + dy*dy) + dx*dx`
     with `d = pos - P`; `r = width*0.125f` (`+0x16`), `r = (7.0f [0x858F48] < r || NaN) ? r : 7.0f` (max); if `sqrt(dist2) <= r` (NaN too)
     start = orig.
  2. Otherwise (no origin, invalid, or too far): `start = FindNodeClosestToCoors(pos, nodeType, 999999.875f, 0, 0, 0, 0, 0)`.
  3. If start invalid or not loaded (0x451FD2): `target->area = 0xFFFF`; start.area = 0xFFFF; go to step 6 (origin NOT written).
  4. N = node(start). `a = (float)dir * 0.7853982f [0x859AB0]`; `s = sin(a)`, `c = cos(a)` (f32); `target->area = 0xFFFF`;
     best = -999999.0f (imm 0xC97423F0). For link l (i16 counter, numLinks re-read): `a = link`; skip if not loaded; L = node(a); skip if
     `!(N.+0x18 & 0x20) && (L.+0x18 & 0x20)` (switched off) or `!(N.+0x19 & 4) && (L.+0x19 & 4)` (don't wander).
     `vx = L.x*0.125 - N.x*0.125`, `vy = L.y*0.125 - N.y*0.125`, `inv = 1.0f / sqrt(vy*vy + vx*vx)`, `nx = inv*vx` (f32), `ny = inv*vy` (f32).
     `score = ny*c + nx*s`; if `score < best` or NaN: skip. Else best = score, `*target = a`, and `*outDir` =
     - nx >= 0: `2|ny| < nx` -> 2; else `2nx < ny` -> 0; else `-2nx > ny` -> 4; else `ny > 0` -> 1 else 3.
     - nx < 0 or NaN: `2|ny| < -nx` -> 6; else `-2nx < ny` -> 0; else `2nx > ny` -> 4; else `ny > 0` -> 7 else 5.
     (every "<"/">" is strict; NaN compares false. -2.0f is 0x858B18.)
  5. `*origin = start` (no NULL check: a NULL origin crashes here, 0x451F3C).
  6. (stub 0x403CFE) If `target->area == 0xFFFF`: `*outDir = 0`, `*target = start`. Then if `*target == orig` (both words), `orig.area != 0xFFFF`
     and loaded: `*target` = the first link of node(orig) whose area is loaded (unchanged if none).
- gta-reversed: `plugin::CallMethod` (PathFind.cpp:292). Open questions: none.

## 0x452760 CPathFind::ComputeRoute  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall void (uint8 nodeType [+4], const CVector* start [+8], const CVector* end [+0xC], const CNodeAddress*
  startAddr [+0x10], CNodeRoute* route [+0x14])`, `ret 0x14`. Stub `jmp 0x156B7D0`. ECX is passed through to DoPathSearch unchanged.
- Callers: CTaskComplexFollowNodeRoute::ComputePathNodes 0x66EFA0 (3). Callees: DoPathSearch 0x4515D0.
- Behaviour: local `count = 0` (dword), local `nodes[8]` with only the area words set to 0xFFFF.
  `DoPathSearch(nodeType, *start, *startAddr, *end, nodes, &count, 8, NULL, 999999.875f, NULL, 999999.875f, 0, forbidden = [0x8A5F44]
  (dword 0x0000FFFF), 0, 0)`. Then `route->count (+0) = 0`; for i < (int16)count: if `route->count < 8`: `route->nodes[route->count++]
  (+4 + 4n) = nodes[i]`.
- gta-reversed: `plugin::CallMethod` (PathFind.cpp:493). Open questions: none.
