# POP unattributed functions (group P3) - full specs

ThePaths = CPathFind at 0x96F050. CPathFind fields (gr PathFind.h, offsets confirmed by the bytes): +0x804 m_pPathNodes[] (CPathNode*,
stride 0x1C), +0xA44 m_pNodeLinks[] (CNodeAddress* = dword {u16 area, u16 node}), +0xC84 m_pPathIntersections[] (byte per link),
+0x10C4 m_anNumVehicleNodes[], +0x11E4 m_anNumPedNodes[]. CPathNode: +0x08/+0x0A/+0x0C i16 compressed pos (x8), +0x10 i16
m_wBaseLinkId, +0x12 u16 m_wAreaId, +0x14 u16 m_wNodeId (dword at +0x12 = its CNodeAddress), +0x16 u8 m_nPathWidth,
+0x18 byte: bits 0-3 m_nNumLinks, bit 5 (0x20) m_isSwitchedOff; +0x1A byte bits 0-3 m_nSpawnProbability.
`_rand` 0x821B1E (0..0x7FFF), `_ftol` 0x821B40 (truncation), CPathNode::GetPosition 0x420A10 (thiscall (CVector* out) -> EAX = out).

## 0x421A50 CCarCtrl::sub_421A50 (proposed: CCarCtrl::WouldCollideAtThisAngle)  (catalogue: unknown/REA, port level 1)
- Name evidence: only caller CCarCtrl::WeaveForOtherCar 0x426350 (2 sites, left and right angle sweeps, see CCarCtrl.md 0x426350
  step 6/7), which advances the candidate angle while this returns true. Pure function (no globals except one constant).
- Convention / signature: `__cdecl bool (float angle [esp+4], CVector* P0 [+8], CVector* Wv [+0xC], CVector* P1 [+0x10],
  CVector* Lv [+0x14], CVector* Q0 [+0x18], CVector* Wc [+0x1C], CVector* Q1 [+0x20], CVector* Lc [+0x24], float mvX [+0x28],
  float mvY [+0x2C], float speed [+0x30], bool swap [+0x34] (only the low byte is read, 0x421A57))`, AL result (0/1, upper EAX
  bits garbage), plain `ret`. Argument slots +0x1C..+0x34 are reused as scratch (no caller relies on them).
  Caller argument meaning (WeaveForOtherCar): P0/Wv = own vehicle front-edge start/extent, P1/Lv = own side-edge start/extent,
  Q0/Wc and Q1/Lc = the same for the other vehicle; mv = other's move speed; speed = own 2D speed; swap = own box narrower.
- Callees: CCollision::Test2DLineAgainst2DLine 0x4138D0 (cdecl (s1x, s1y, d1x, d1y, s2x, s2y, d2x, d2y) -> AL; segment = start +
  delta), 16 sites; 0x40FE30 = `CVector operator+` (cdecl (CVector* out, const CVector* a, const CVector* b) -> EAX = out,
  out = (a.x+b.x, a.y+b.y, a.z+b.z), components computed z, y, x and stored x, y, z), 1 site.
- Constants: `0x858628 = 100.0` (f32).
- Behaviour (frame S = esp after the 4 pushes; locals named below):
  1. `vx = (mvX - cos(angle) * speed) * 100.0f`, `vy = (mvY - sin(angle) * speed) * 100.0f` (0x421A53-0x421A8B: `fld angle; fcos;
     fmul speed; fsubr mvX`, then `fsin` chain, each `fmul [0x858628]`, stored f32 to S+0x1C / S+0x20; fcos/fsin on the x87 value of
     the f32 angle).
  2. Role selection (0x421A8F):
     - swap == 0 (0x421AF3): lineA = (P0, Wv) [edi, esi], lineB = (P1, Lv) [ebp, ebx], rect1 = (Q0, Wc), rect2 = (Q1, Lc).
     - swap != 0 (0x421A91-0x421AF1): `vx = -vx`, `vy = -vy` (fchs, stored f32), `vz = 0`; lineA = (Q0, Wc), lineB = (Q1, Lc),
       rect1 = (P0, Wv), rect2 = (P1, Lv).
     (vz is never read; in the non-swapped path S+0x24 is left uninitialised, harmless.)
  3. Rect1 with origin O = *rect1.origin (copied, 0x421B07-0x421B1F) and extent W = *rect1.extent; corners (each stored f32):
     `C1 = (W.x + O.x, O.y + W.y)` (0x421B1D-0x421B2F, note operand order), `A = (vx + O.x, vy + O.y)` (0x421B33-0x421B47),
     `B = (C1.x + vx, C1.y + vy)` (0x421B4B-0x421B5F).
     Edges tested, each as (start, delta = end - start, deltas stored f32): e1 = O -> C1, e2 = C1 -> A, e3 = A -> B, e4 = B -> O.
     Calls, returning 1 at the first true (0x421BAB): lineA vs e1 (0x421B9F), e2 (0x421BF5), e3 (0x421C3B), e4 (0x421C85); then lineB
     vs e1 (0x421CBB), e2 (0x421CEE), e3 (0x421D21), e4 (0x421D54). The line is always the FIRST segment argument
     (`Test2DLineAgainst2DLine(line.start.x, line.start.y, line.ext.x, line.ext.y, edge.start.x, edge.start.y, edge.d.x, edge.d.y)`).
  4. Rect2: O' = *rect2.origin (copied), `R = O' + W'` via 0x40FE30 (0x421D89; W' = *rect2.extent), `A' = (vx + O'.x, vy + O'.y)`,
     `B' = (vx + R.x, vy + R.y)` (0x421D8E-0x421DD2). Edges: e1' = O' -> R, e2' = R -> A', e3' = A' -> B', e4' = B' -> O'.
     lineA vs e1' (0x421E12), e2' (0x421E62), e3' (0x421EAC), e4' (0x421EF6); lineB vs e1' (0x421F2C), e2' (0x421F5F), e3' (0x421F92),
     e4' (0x421FC5); the last call's result is returned directly (`setne al`, 0x421FD2).
  5. Returns 0 only if all 16 tests are false.
- Original bug (keep): the "swept rectangle" of each extent segment O..O+W moved by v is traced as O -> O+W -> O+v -> O+W+v -> O, i.e.
  a self-intersecting bow-tie: two edges are the W-parallel sides, the other two are the DIAGONALS (O+W -> O+v and O+W+v -> O); the
  two v-parallel sides are never tested. Reproduce the 16 calls and their argument values exactly.
- gta-reversed: absent (no function at this address). Open questions: none.

## 0x44DA30 CPathFind::sub_44DA30 (proposed: CPathFind::TakeWidthIntoAccountForCoors)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr declares `void CPathFind::TakeWidthIntoAccountForCoors(CNodeAddress, CNodeAddress, uint16 seed, float* fOut1,
  float* fOut2)` (PathFind.h:379, no body, no address); this function has exactly that shape (thiscall on ThePaths, 5 args,
  `ret 0x14`) and its arithmetic is the two-node version of TakeWidthIntoAccountForWandering 0x4509A0 (same 0.00775 factor).
  Caller CPopulation::AddToPopulation 0x614B9B: `(this = 0x96F050, addr1, addr2, rand(), &x, &y)` right after GeneratePedCreationCoors.
- Convention: `__thiscall void (CPathFind* this = ECX, CNodeAddress a [esp+4], CNodeAddress b [+8], int16 seed [+0xC] (read as
  `movsx word`, 0x44DA97), float* pX [+0x10], float* pY [+0x14])`, `ret 0x14`.
- Constants: `0x859A9C = 0.00774999987334013` (f32).
- Behaviour:
  1. Return without writing if `a.area == 0xFFFF` (0x44DA34), `m_pPathNodes[a.area] == null` (0x44DA4A), `b.area == 0xFFFF`
     (0x44DA54) or `m_pPathNodes[b.area] == null` (0x44DA6A), in that order.
  2. A = node a, B = node b. `w = min(A.width, B.width)` (u8 at +0x16; 0x44DA87-0x44DA95: `if (A.w < B.w) w = A.w else w = B.w`).
     `*pX = (float)(((seed & 0xF) - 7) * w) * 0.00775f + *pX` (int multiply, fild of the i32 product, 0x44DA9C-0x44DABF; stored f32).
  3. `w` recomputed the same way (0x44DAC1-0x44DACC); `*pY = (float)((((seed >> 4) & 0xF) - 7) * w) * 0.00775f + *pY` (`sar` on
     the sign-extended seed, then `& 0xF`, 0x44DACE-0x44DAF1).
  The offsets are in [-7, +8] * width * 0.00775 (width is 1/16 m fixed point in gr's comment). The node positions are not used.
- gta-reversed: declaration only, no body. Open questions: none.

## 0x44E790 CPathFind::sub_44E790 (proposed: CPathFind::GeneratePedCreationCoors)  (catalogue: unknown/REA, port level 9)
- Name evidence: gr declares `GeneratePedCreationCoors(float x, float y, float minDist1, float maxDist1, float minDist2, float
  maxDist2, CVector* outCoords, CNodeAddress* outAddress1, CNodeAddress* outAddress2, float* outOrientation, bool bLowTraffic,
  CMatrix* transformMatrix)` (PathFind.h:301, no body): 12 args = this function's `ret 0x30`; thiscall with ECX = 0x96F050 from
  CPopulation::AddToPopulation 0x614B1C, which then uses the two node addresses and calls 0x44DA30. gr's declared return type `void`
  is wrong: the original returns a bool in AL (0x44EC54 / 0x44EC93; the caller tests AL at 0x614B21).
- Convention: `__thiscall bool (CPathFind* this = ECX, float x [esp+4], float y [+8], float minDist1 [+0xC], float maxDist1 [+0x10],
  float minDist2 [+0x14], float maxDist2 [+0x18], CVector* outCoords [+0x1C], CNodeAddress* outAddr1 [+0x20],
  CNodeAddress* outAddr2 [+0x24], float* outT [+0x28], bool allowSwitchedOff [+0x2C] (gr: bLowTraffic; low byte read),
  RwMatrix* camMatrix [+0x30])`, `ret 0x30`. The only caller passes allowSwitchedOff = false and camMatrix = null.
- Callees: `_rand` (per outer iteration, per attempt, plus one per accepted-off-screen test), `_ftol`, CPathFind::FindRegionForCoors
  0x44D830 (thiscall (float x, float y) -> EAX region 0..63), CPathNode::GetPosition 0x420A10 (6 sites), CCamera::IsSphereVisible
  0x420C40 (thiscall on TheCamera 0xB6F028, (const CVector& c, float r, RwMatrix* m) -> AL), CCamera::IsSphereVisible 0x420D40
  (thiscall (const CVector& c, float r) -> AL), CWorld::FindGroundZFor3DCoord 0x5696C0 (cdecl (CVector by value, bool* found,
  CEntity** = null) -> ST0).
- Constants (f32): `0x858B14 = 3.0517578125e-05` (1/32768), `0x858B48 = 15.0`, `0x858CA4 = 30.0`, `0x858C48 = 0.125`,
  `0x859AA0 = 0.00390625`, `0x858624 = 1.0`, `0x858CA0 = 2.0`, `0x858B3C = 3.0`; sphere radius pushed as immediate 0x40000000 (2.0f).
- Behaviour:
  1. `r = _ftol((float)(rand() & 0xFFFF) * (1/32768) * 15.0f)` (0x44E7A0-0x44E7BE; 0..14). `lim = (maxDist1 + 30.0f)^2` (x87 square of
     the sum, stored f32, 0x44E7C3-0x44E7F2). `reg = FindRegionForCoors(x, y)` (0x44E7F8). Only nodes of this one region are used.
  2. Outer loop `i = 0..299` (counter at 0x44EC3D-0x44EC4B, `< 0x12C`):
     a. Skip iteration if `m_pPathNodes[reg] == null` (0x44E819) or `m_anNumPedNodes[reg] == 0` (0x44E828).
     b. `n = (rand() >> 6) % m_anNumPedNodes[reg]` (signed idiv, 0x44E82E-0x44E837); `N1 = m_pPathNodes[reg] + (n +
        m_anNumVehicleNodes[reg]) * 0x1C` (a ped node).
     c. `nx = (float)N1.x16 * 0.125f`, `ny = (float)N1.y16 * 0.125f` (stored f32); `d = (ny - y)*(ny - y) + (nx - x)*(nx - x)` (y term
        first, 0x44E855-0x44E8AD, stored f32 in slot D). Skip unless `d < lim` (`test ah,5; jp`, 0x44E8BA; NaN skips).
     d. Skip unless `(N1.+0x1A & 0xF) > r` (signed, 0x44E8CE). `links = N1.+0x18 & 0xF`; skip if 0 (0x44E8E0).
     e. Link loop `j = 0..links-1` (0x44E900-0x44EC37), link index `k = (int16)N1.m_wBaseLinkId + j`:
        - skip if `m_pPathIntersections[reg][k] & 1` (0x44E90D);
        - `A2 = m_pNodeLinks[reg][k]`; skip if `A2.area >= 0x40` (unsigned 16-bit, 0x44E923) or `m_pPathNodes[A2.area] == null`;
          `N2 = m_pPathNodes[A2.area] + A2.node * 0x1C`;
        - if `(N1.+0x18 & 0x20) || (N2.+0x18 & 0x20)` (either switched off) and `!allowSwitchedOff`: skip (0x44E94F-0x44E964);
        - skip unless `(N2.+0x1A & 0xF) > r` (0x44E978);
        - `D = sqrt(D)` WRITTEN BACK INTO SLOT D (0x44E97E-0x44E98E, see bug below);
        - `d2 = (N2.y - y)*(N2.y - y) + (N2.x - x)*(N2.x - x)` (four GetPosition(N2) calls, 0x44E992-0x44EA04);
        - continue to the attempts if `D < maxDist1` (0x44EA16, `test ah,5; jnp`), else if `sqrt(d2) < maxDist1` (0x44EA26); otherwise
          (incl. NaN) skip the link.
     f. Up to 5 attempts (`a = 0..4`, 0x44EA30, 0x44EC0E-0x44EC1A):
        - `t = (float)(rand() & 0xFF) * 0.00390625f` (0x44EA38-0x44EA59); `*outT = t` (written on EVERY attempt, 0x44EA67).
        - `P2 = GetPosition(N2)`, `tP = (t*P2.x, t*P2.y, t*P2.z)` (each stored f32); `u = 1.0f - t` (stored f32);
          `P1 = GetPosition(N1)`; `pos.x = u*P1.x + tP.x`, `pos.y = u*P1.y + tP.y`, `pos.z = u*P1.z + tP.z` (stored f32,
          0x44EAAF-0x44EAF7).
        - `dist = sqrt((pos.y - y)^2 + (pos.x - x)^2)` (stored f32, 0x44EAFB-0x44EB32).
        - `vis = camMatrix ? TheCamera.IsSphereVisible(pos, 2.0f, camMatrix) (0x420C40) : TheCamera.IsSphereVisible(pos, 2.0f) (0x420D40)`.
        - If vis: accept iff `dist > minDist1` (`test ah,0x41; jne` reject on <=/NaN, 0x44EB77) and `dist < maxDist1` (0x44EB8D).
          Else: accept iff `dist > minDist2` (0x44EB9D) and `dist < maxDist2` (0x44EBAF) and `rand() & 1` (0x44EBB1-0x44EBB8; rand only
          called when both distance tests passed).
        - On accept (0x44EBBA): `*outAddr1 = dword N1.+0x12`, `*outAddr2 = dword N2.+0x12`, `*outCoords = pos` (all written now);
          `g = FindGroundZFor3DCoord((pos.x, pos.y, pos.z + 2.0f), &found, null)` (0x44EBFC). If `!found`: discard g, next attempt.
          If found: if `|g - pos.z| <= 3.0f` (fabs of `g - pos.z`, `test ah,0x41; jne`, 0x44EC73; NaN also counts as <=)
          `outCoords->z = g`, return 1 (0x44EC86); else return 0 IMMEDIATELY (0x44EC75), with outCoords/outAddr already written.
     After 5 failed attempts the next link is tried (0x44EC20).
  3. After 300 outer iterations return 0 (0x44EC54).
- Original bugs (keep):
  - Slot D holds the squared distance of N1, but 0x44E98E overwrites it with its square root, and the link loop re-reads it for every
    link that reaches 0x44E97E: the first such link compares `sqrt(d)`, the second `sqrt(sqrt(d))`, etc. against maxDist1.
  - A ground-height mismatch > 3 aborts the whole search (return 0) instead of trying the next attempt; outputs are already written.
  - FindGroundZFor3DCoord is called with pos.z + 2.0f.
- gta-reversed: declaration only (wrong `void` return). Open questions: none.
