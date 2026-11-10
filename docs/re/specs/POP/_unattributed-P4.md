# POP unattributed functions (group P4)

## 0x450FB0 CPathFind::sub_450FB0 (proposed: CPathFind::AddInteriorNodeLink)  (catalogue: unknown/REA, port level 1)
Evidence: only caller CPathFind::CompleteNewInterior 0x452270 (call at 0x452644, args `(slot + 0x40, nodeIdx, slot + 0x40,
ConnectsToGiven entry, slot = [0x96EF84], &linkCounter)`); all tables are ThePaths (0x96F050) arrays indexed by interior area 64 + slot;
it writes one node link, its length and intersection byte and bumps the node's link count, so it is the per-link helper of
CompleteNewInterior. Not present in gta-reversed.
- Convention / signature: `__cdecl void (int32 areaA [esp+4], int8 nodeA [esp+8] (dword slot, read as sign-extended byte), int32 areaB
  [esp+0xC], int32 nodeB [esp+0x10], int32 slot [esp+0x14], int32* linkCounter [esp+0x18])`, plain `ret`. ECX unused.
- Callees: CPathNode::GetPosition 0x420A10, `_ftol` 0x821B40 (x2).
- Globals (ThePaths arrays at +0x100, i.e. index 64 + slot): m_pNodeLinks 0x96FB94, m_pLinkLengths 0x96FCB4, m_pPathIntersections 0x96FDD4,
  m_pPathNodes 0x96F954; m_pPathNodes 0x96F854 (indexed by areaA / areaB).
- Behaviour:
  1. `k = *linkCounter`; `m_pNodeLinks[64+slot][k] = (area = (u16)areaB, node = (u16)nodeB)` (0x450FD2 / 0x450FDE).
  2. `B = &m_pPathNodes[areaB][nodeB]`, `A = &m_pPathNodes[areaA][(int8)nodeA]`.
     `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `d = A*0.125 - B*0.125` (positions from the i16 fields; dx, Ax, Ay, Az stored f32;
     0x451005-0x4510A3). If `d > 255.0f [0x859AAC]` (0x4510B4 `test ah,0x41; jne`), v = 255.0f, else v = d (recomputed identically).
     `len = (u8)ftol(v)`; if `len < 1`: len = 1. Otherwise the value is computed once more the same way (again clamped to 255.0f; the
     unclamped path uses `A.GetPosition() - B.GetPosition()`, x/y/z differences summed as `(dz*dz + dy*dy) + dx*dx`) and `len = (u8)ftol`
     of it (same result as the first).
  3. `m_pLinkLengths[64+slot][k] = len`; `m_pPathIntersections[64+slot][k] = 0`; node `m_pPathNodes[64+slot][(int8)nodeA]`: numLinks
     nibble of `+0x18` += 1 (4-bit wrap, other bits kept, 0x4512B1-0x4512BE); `*linkCounter = k + 1`.
- Edge cases: the node whose link count is bumped is addressed with area 64+slot, the position with areaA (the only caller passes
  areaA = 64 + slot). A distance whose truncation is 0 (or NaN) gives length 1.
- gta-reversed: absent. Open questions: none.
