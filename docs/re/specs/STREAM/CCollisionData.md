# CCollisionData (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x537510 CCollisionData::GetNoOfShadowVertices (proposed name)  (catalogue: unknown/REA, port level 0)
Evidence: `this` (ECX) is a `CCollisionData` (gr Collision/CollisionData.h, size 0x30): +0x20 u32
m_nNumShadowTriangles, +0x2C `CColTriangle*` m_pShadowTriangles (8-byte elements: u16 vA +0, vB +2, vC +4,
u8 material +6, u8 light +7). Both callers (CFileLoader::LoadCollisionModelVer3 0x537CE0 and Ver4 0x537AE0)
do `mov ecx, cd; call 0x537510; mov [cd+0x24], eax` (m_nNumShadowVertices) when the shadow flag is set.
gta-reversed inlines this as `ColHelpers::V3::Header::GetNoOfShdwVerts(cd)` (ColHelpers.h:224).
- Convention: `__thiscall uint32 (CCollisionData* this)`; no stack args; plain `ret`; return EAX.
- Behaviour:
  1. `n = this->m_nNumShadowTriangles (as int32); result = 0;`
  2. If `n == 0`: return 0.
  3. If `n > 0` (signed): `p = m_pShadowTriangles`; repeat n times: for each of vA, vB, vC in that order
     (zero-extended u16): `if (v > result) result = v;` (unsigned compare); `p += 8`.
  4. Return `result + 1`.
- Edge cases: if the count is negative as int32 (>= 0x80000000), the loop is skipped and 1 is returned
  (gr's span-based loop would iterate). Not reachable from valid files.
- gta-reversed: ColHelpers.h:224-234, same result for counts < 2^31.
- Open questions: none.
