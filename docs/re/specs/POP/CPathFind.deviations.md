# CPathFind (POP) - gta-reversed deviation notes (group P4)

Source: `re/gta-reversed/source/game_sa/PathFind.cpp` / `PathFind.h` (pin 20d8d2a). ThePaths = 0x96F050. CPathFind fields used: +0x004
m_pathFindHashTable[512] (CPathNode* buckets), +0x804 m_pPathNodes[72], +0x924 m_pNaviNodes[72], +0xA44 m_pNodeLinks[72], +0xB64
m_pLinkLengths[72], +0xC84 m_pPathIntersections[72], +0xDA4 m_pNaviLinks[64], +0xEA4 m_aTempNodes[64], +0x3544 hash-table node count,
+0x3548 m_interiorIDs[8], +0x3568 m_nNumNodeSwitches, +0x356C m_aNodeSwitches[64] (stride 0x1C: xMin, xMax, yMin, yMax, zMin, zMax f32,
+0x18 isOff u8, +0x19 isCars u8), +0x3C6C m_loadAreaRequestPending. CPathNode (0x1C): +0x00 m_next, +0x04 m_prev, +0x0E
m_totalDistFromOrigin i16, +0x10 m_wBaseLinkId i16, +0x12 m_wAreaId u16, +0x18 flags byte 0 (bits 0-3 m_nNumLinks, bit 5 m_isSwitchedOff),
+0x19 flags byte 1 (bit 0 m_isSwitchedOffOriginal), +0x1A flags byte 2 (bits 4-7 m_nBehaviourType).

## 0x44D080 CPathFind::Init  (gr_deviations = 2)
Original (stub `jmp 0x1560DC0`, `__thiscall`, plain `ret`): `[0x96F02C] = 0` (static NumTempExternalNodes), `this+0x3568 = 0`,
`byte this+0x3C6C = 0`; loop 72 times (esi = 0x48, eax = this+0x924 + 4i): zero the dwords at eax-0x120 (+0x804), eax (+0x924), +0x120
(+0xA44), +0x240 (+0xB64), +0x360 (+0xC84), +0x480 (+0xDA4), +0x580 (+0xEA4) (0x1560DE1-0x1560E0B); then `rep stosd` 8 x 0xFFFFFFFF at
this+0x3548 (m_interiorIDs).
Rows PathFind.cpp:130 and :131 ("BUG: Out of array bounds write, same as in original code"):
- Original bytes: the two arrays at +0xDA4 and +0xEA4 have 64 entries but are written for i = 0..71 (0x1560DFB, 0x1560E01), i.e. the
  writes for i >= 64 land at +0xEA4..+0xEC0 (first 8 entries of m_aTempNodes, which are then written again) and +0xFA4..+0xFC0
  (m_anNumNodes[0..7], gr PathFind.h:203, which are therefore zeroed).
- Verdict: REFUTED (gr reproduces the same out-of-bounds writes). Class: keep-original (a port must also zero m_anNumNodes[0..7]).

## 0x44D1E0 CPathFind::AddNodeToList  (gr_deviations = 1)
Original: `__thiscall void (CPathNode* node [esp+4], int32 dist [esp+8])`, `ret 8`. `b = (int16)(dist & 0x1FF)` (`movsx ax`);
`slot = &this->m_pathFindHashTable[b]` (this+4+4b); `node->m_next = *slot; node->m_prev = slot` (pointer to the bucket, cast); if `*slot`:
`(*slot)->m_prev = node`; `*slot = node`; `node->+0xE = (int16)dist`; `this+0x3544 += 1`.
Row PathFind.cpp:1283 ("Prevent bugs from overflow"):
- Original bytes: 0x44D20C stores the low 16 bits of dist with no check; gr's line is an `assert` (no release-build effect).
- Verdict: REFUTED (assert only). Class: harmless (port: plain truncating store, no check).

## 0x44D830 CPathFind::FindRegionForCoors  (gr_deviations = 1)
Original: `__thiscall` (ECX unused and clobbered) `int32 (float x [esp+4], float y [esp+8])`, `ret 8`.
`rx = ftol((x - -3000.0f [0x859A90]) * 0.0013333333f [0x859A8C])`; `rx = rx <= 0 ? 0 : rx` (signed, 0x44D848-0x44D84E); `rx = min(rx, 7)`;
same for y; return `rx + ry * 8`.
Row PathFind.h:486 (NOTSA: CVector2D instead of 2 floats):
- Original bytes: as above. gr's `RegionValueOf` computes `(uint32)((p + 3000.f) / 750.f)` then clamps to 0..7: a division instead of
  the multiply by 0.0013333333f (different rounding at region borders) and an UNSIGNED conversion, so a coordinate below -3000 gives a large
  value clamped to 7 instead of 0.
- Verdict: CONFIRMED (the parameter type itself is harmless; the formula differs). Class: keep-original.

## 0x44DED0 CPathFind::AddInteriorLink  (gr_deviations = 1)
Original (stub `jmp 0x156EC90`, `__stdcall`-like `ret 8`; ECX unused): ConnectsToGiven = 0x96EAC0, rows of 6 int8. For (a, b) then (b, a):
`row = 0x96EAC0 + 6*a`; `i = 0`; if `row[0] >= 0`: repeat { if i >= 6 break; v = row[i+1]; i++ } while v >= 0 (signed bytes);
`row[i] = (int8)b` (0x156ECBB / 0x156ECE4). With a full row i = 6 and the write lands in row a+1's first byte.
Row PathFind.cpp:1227 (original `while (*it >= 0)`):
- Original bytes: stops at the first NEGATIVE byte (any value < 0), capped at index 6. gr's `rng::find(row, -1)` stops only at -1; for a
  full row both write row[6].
- Verdict: CONFIRMED (differs only when a stored entry is negative but not -1, e.g. a node id >= 128 stored as int8: the original then
  overwrites it). Class: keep-original.

## 0x450A60 CPathFind::UpdateStreaming  (gr_deviations = 1)
Row PathFind.cpp:797 (pool-iteration, mission vehicles):
- Original bytes: vehicle pool loop descending (0x450B40-0x450BB2), skipping free slots, `+0x594` subtype 3, 4, 5, 6, 8, and
  `byte +0x4A4 != 2` (not created by MISSION); calls `MarkRegionsForCoors(pos, 300.0f)` (0x44DB60) which only sets ToBeStreamed flags.
- Verdict: REFUTED (the order is not observable). Class: harmless.

## 0x452160 CPathFind::SwitchOffNodeAndNeighbours  (gr_deviations = 1)
Original (stub `jmp 0x15646A0`): `__thiscall void (CPathNode* node [+4], CPathNode** outNext1 [+8], CPathNode** outNext2 [+0xC],
bool switchTo [+0x10], bool backToOriginal [+0x14])`, `ret 0x14`.
1. Set node bit 5 of +0x18: to bit 0 of +0x19 (original state) if backToOriginal, else to `switchTo` (0x15646A0-0x15646D5).
2. `*outNext1 = NULL`; if outNext2: `*outNext2 = NULL`. If `CountNeighboursToBeSwitchedOff(node) >= 3` (0x4504F0) return.
3. For link i = 0 .. numLinks-1 (numLinks re-read each iteration): `addr = m_pNodeLinks[node.areaId][node.baseLink + i]`; skip if
   `m_pPathNodes[addr.area] == NULL`; `L = &nodes[addr.node]`; skip if `L.+0x1A >> 4` is 1 or 2; skip if L is already in the target state
   (backToOriginal: bit 5 of +0x18 == bit 0 of +0x19; else bit 5 == switchTo); skip if `CountNeighboursToBeSwitchedOff(L) >= 3`;
   then if `*outNext1 == NULL` `*outNext1 = L` else `*outNext2 = L` (no NULL check, 0x156478C).
Row PathFind.cpp:1108 (FIX_BUGS `else if (outNext2)`):
- Original bytes: 0x156478C-0x1564790 write through outNext2 unconditionally.
- Verdict: CONFIRMED (gr default build adds the check). Class: keep-original.
Found in passing (CONFIRMED, keep-original): inside the loop the original counts the neighbours of the LINKED node L
(`push esi; call 0x4504F0`, 0x1564778); gr calls `CountNeighboursToBeSwitchedOff(*node)` (the start node) again.

## 0x452C80 CPathFind::SwitchRoadsOffInArea  (gr_deviations = 4)
Original: `__thiscall void (float xMin, float xMax, float yMin, float yMax, float zMin, float zMax, bool switchOff, bool cars,
bool backToOriginal)` (`ret 0x24`). Calls `SwitchRoadsOffInAreaForOneRegion(xMin, xMax, yMin, yMax, zMin, zMax, switchOff, cars, region,
backToOriginal)` (0x452820) for region 0..63; reads the count through the stub 0x401956. Then for i = 0 .. count-1: keep the entry when
`e.xMin < xMin` or `e.yMin < yMin` or `e.zMin < zMin` (`test ah,1`) or `e.xMax > xMax` or `e.yMax > yMax` or `e.zMax > zMax`
(`test ah,0x41; jp`); NaN keeps. Otherwise: for j = i .. count-2 copy entry[i+1] over entry[i] (7 dwords; the same two entries every
time, 0x452D93-0x452DA8), `count--`, `i--` (re-test the same index). Finally if `!backToOriginal && count < 64`: append
(xMin, xMax, yMin, yMax, zMin, zMax, switchOff, cars), count++.
Rows PathFind.cpp:1298 and :1318 (ReversibleBugFix CPathFind_SwitchRoadsOffInArea_StrayAreas):
- Original bytes: no `isCars` comparison, no "same area toggled back" detection, and the shifting loop copies entry[i+1] to entry[i] only
  (0x452D98-0x452D9C use the fixed pointers edx / ebx).
- Verdict: CONFIRMED (bug fix enabled by default in gr). Class: keep-original.
Row PathFind.cpp:1306 (comment inside the bug-fix block): Verdict: CONFIRMED (same block). Class: keep-original.
Row PathFind.cpp:1321 ("R* bug, they messed up with the index"):
- Original bytes: as described; gr's fix-off branch `m_aNodeSwitches[i] = m_aNodeSwitches[i + 1]` matches.
- Verdict: REFUTED. Class: keep-original.
