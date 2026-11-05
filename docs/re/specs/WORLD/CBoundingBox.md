# CBoundingBox (WORLD)

Layout (CBox): `m_vecMin` CVector at +0x0, `m_vecMax` CVector at +0xc.

## 0x49c300 CBoundingBox::IsPointWithin  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall bool (this = ECX: const CBoundingBox*, const CVector* p [esp+4])`,
  return AL (only AL is set: `mov al,1` / `xor al,al`), `ret 4`.
- Callers: 1 (`CStuntJumpManager::Update` 0x49c490, 2 sites). Callees: none. Globals: none.
- Behaviour: six f32 compares, each `fld p.c; fcomp box.c; fnstsw`, short-circuit in this order; any
  failure returns false:
  1. `p.x >= min.x` (fails on `p.x < min.x` or unordered: `test ah,1`)
  2. `p.y >= min.y`, 3. `p.z >= min.z` (same test)
  4. `p.x <= max.x` (`test ah,0x41; jp` → fails when `p.x > max.x` or unordered)
  5. `p.y <= max.y`, 6. `p.z <= max.z`
  then true. Any NaN coordinate (point or box) → false.
- gta-reversed: provided (Collision/BoundingBox.cpp:4), same order and operators; equivalent.
- Open questions: none.
