# CSetPiece (WORLD) - full specs

Layout (gr `SetPiece.h`, size 0x20): +0x0C `m_SpawnCoord1` (int16 x, int16 y; world units * 4), +0x10 `m_SpawnCoord2`,
+0x14 `m_TargetCoord1`, +0x18 `m_TargetCoord2`. Constant 0x858C84 = 0.25f.

All four getters are identical apart from the field offset:
- Convention: `__thiscall CVector2D* (CSetPiece* this = ECX, CVector2D* out [esp+4])` - the CVector2D return value is
  returned through the hidden pointer; EAX = out on return; `ret 4`. Callers: CSetPiece::Update 0x499A80 only.
- Behaviour (exact x87 order): `t_y = (float)(int32)(int16)field.y` (movsx, fild) `* 0.25f`; `t_x = (float)(int16)field.x * 0.25f`;
  `out->x = t_x` (fstp first), `out->y = t_y`. All values are exact in f32 (int16 * 0.25).
- gr: inline getters (`FixedVector2D<int16, 4.0f>` conversion) - same result.

## 0x4997A0 CSetPiece::GetSpawnCoord1  (catalogue: specified/REA, port level 0)
- Field +0x0C (x at +0x0C, y at +0x0E).

## 0x4997E0 CSetPiece::GetSpawnCoord2  (catalogue: specified/REA, port level 0)
- Field +0x10 (x at +0x10, y at +0x12).

## 0x499820 CSetPiece::GetTargetCoord1  (catalogue: specified/REA, port level 0)
- Field +0x14 (x at +0x14, y at +0x16).

## 0x499860 CSetPiece::GetTargetCoord2  (catalogue: specified/REA, port level 0)
- Field +0x18 (x at +0x18, y at +0x1A).
