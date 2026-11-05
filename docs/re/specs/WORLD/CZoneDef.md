# CZoneDef (WORLD)

Layout (0x10 bytes, all int16): `m_cornerX` +0x0, `m_cornerY` +0x2, `m_vec1X` +0x4, `m_vec1Y` +0x6,
`m_vec2X` +0x8, `m_vec2Y` +0xa, `m_minZ` +0xc, `m_maxZ` +0xe (gta-reversed CullZones.h).

## 0x72d850 CZoneDef::IsPointWithin  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall bool (this = ECX: const CZoneDef*, CVector p BY VALUE)`:
  `p.x [esp+4]`, `p.y [esp+8]`, `p.z [esp+0xc]` (12 bytes on the stack), **`ret 0xc`**. Return AL.
  The port's ABI must take the vector by value, not by reference.
- Callers: 4 (`CCullZones::FindAttributesForCoors` 0x72d970, `FindTunnelAttributesForCoors` 0x72d9f0,
  `FindMirrorAttributesForCoors` 0x72da70, `FindZoneWithStairsAttributeForPlayer` 0x72dad0). Callees: none.
- Globals: `0x858b50` f32 0.0.
- Behaviour (int16 fields are sign-extended to int32 and loaded with `fild`):
  1. `if !((float)minZ < p.z) return false` (`fcomp`, `test ah,5; jp`: fails on `minZ > z`, `minZ == z`,
     unordered).
  2. `if !((float)maxZ > p.z) return false` (`test ah,0x41; jne`: fails on `maxZ < z`, `==`, unordered).
  3. `dx = p.x - (float)cornerX` (`fild; fsubr`), stored as f32. `dy = p.y - (float)cornerY`, stored as f32.
  4. `t1 = (float)vec1X * dx + (float)vec1Y * dy` (two products, then `faddp`; kept in an x87 register,
     not stored). `if t1 < 0.0f (or NaN) return false` (`fcom [0x858b50]; test ah,1`).
     `s1 = vec1X*vec1X + vec1Y*vec1Y` in **int32** (`imul`, wrapping), `fild s1`.
     `if !(t1 <= s1) return false` (`fcompp` with ST0 = t1; `test ah,0x41; jp`: fails on `t1 > s1` or
     unordered).
  5. Same with `t2 = (float)vec2X * dx + (float)vec2Y * dy` and `s2 = vec2X*vec2X + vec2Y*vec2Y`.
  6. return true.
- Precision: `dx`, `dy` are rounded to f32 memory; products and sum are x87 with 24-bit PC (round to
  f32 significand at each op); the compare is against the register value.
- gta-reversed: inline in CullZones.h:99 (not hooked). Same logic, but takes `const CVector&` (ABI
  differs from the original `ret 0xc`), and computes `sq(..)` sums as int (same values). Behaviour
  equivalent; the signature is the difference.
- Open questions: none.
