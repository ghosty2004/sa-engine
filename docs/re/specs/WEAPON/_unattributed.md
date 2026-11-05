# WEAPON - functions assigned to WEAPON whose class lies elsewhere

These were assigned to the WEAPON group by caller; their owning classes (CWaterLevel, CPlayerInfo) are outside WEAPON.

## 0x56EF90 CPlayerInfo crosshair helper (proposed: CPlayerCrossHair::Render(int32 playerIdx))  (catalogue: unknown/REA, port level 2)
- Evidence: single caller CWeaponEffects::Render 0x742CF0 at 0x743C03, in a loop `for (i = 0; i < 2; i++)` with
  ECX = 0xB7CEF0 + i*0x190 = `&CWorld::Players[i] (+0x158)` = CPlayerInfo::m_nCrosshairActivated, followed by
  m_vecCrosshairTarget (CVector2D in -1..1, +0x15C/+0x160). The object is gr's `{ m_nCrosshairActivated; m_vecCrosshairTarget }`
  part of CPlayerInfo (owner class outside WEAPON, hence `_unattributed`).
- Convention: `__thiscall void (int32 playerIdx [esp+4])` (ECX = &CPlayerInfo::m_nCrosshairActivated), `ret 4`.
  Preserves EBX/EBP/ESI/EDI.
- Callees: `RwEngineInstance->fpRenderStateSet` (call `[[0xC97B24]+0x20]`, cdecl (state, value)),
  CSprite::RenderOneXLUSprite_Rotate_Aspect 0x70D490 (cdecl, 12 dwords, gr signature `(CVector pos, CVector2D size, uint8 r,
  uint8 g, uint8 b, int16 intensity, float rz, float rotation, uint8 alpha)`).
- Globals: history tables (file statics, no gr names), each `[2][5]` of 4-byte entries, row stride 0x14:
  `T` 0xB9B8F8 (times, u32), `Y` 0xB9B920 (f32), `X` 0xB9B948 (f32). `0x8CDF1C` (i32 .data, initial value 8, never written by
  any catalogued function) = index into `gpCoronaTexture` 0xC3E000. `RsGlobal.maximumWidth` 0xC17044 / `maximumHeight`
  0xC17048 (i32). `CTimer::m_snTimeInMilliseconds` 0xB7CB84.
- Constants: 0x858C54 4294967296.0f (dead), 0x865034 0.0061359233f (2*pi/1024), 0x858CC4 0.2f, 0x858624 1.0f, 0x85862C 10.0f,
  0x858BA4 20.0f, 0x858FE4 1.5707964f, 0x859AB0 0.7853982f, 0x858B8C 0.5f.
- Behaviour:
  1. `if (*(u8*)this == 0) return` (byte test of the activated flag, 0x56EF96) - nothing else is done (no state changes).
  2. RenderStateSet in order: (8 ZWRITEENABLE, 0), (6 ZTESTENABLE, 0), (0xC VERTEXALPHAENABLE, 1), (0xA SRCBLEND, 5),
     (0xB DESTBLEND, 8), (1 TEXTURERASTER, `gpCoronaTexture[*(i32*)0x8CDF1C]->raster` (dword at texture+0)).
  3. Colour: `playerIdx != 0` -> (r, g, b) = (0x32, 0, 0xFF); else (0, 0xFF, 0x32). Kept as u32 in EBX/EBP/[esp+0x2C].
  4. History shift (0x56F040-0x56F06C), k = 3, 2, 1, 0 (descending), with `o = playerIdx*0x14`:
     `X[idx][k+1] = X[idx][k]`; `Y[idx][k+1] = Y[idx][k]`; `T[idx][k+1] = *(u32*)(0xB9B920 + 4*k)` - the time slot is filled
     from a FIXED address independent of playerIdx: the raw bits of `Y[0][k]` (for idx 0 read before Y[0][k] is overwritten
     in a later iteration, i.e. the old Y[0][k]). Original bug (should be `T[idx][k]`): keep. Then
     `X[idx][0] = this->+4` (x), `Y[idx][0] = this->+8` (y), `T[idx][0] = CTimer::m_snTimeInMilliseconds` (raw dword copies).
  5. For j = 0..4 (history entry, `p = o + 4*j`):
     - `pulse = sin((float)(T[idx][j] & 0x3FF) * 0.0061359233f) * 0.2f + 1.0f` (fild int, fmul f32, x87 `fsin`, fmul, fadd;
       stored f32). So for j >= 1 the phase comes from float bit patterns (step 4).
     - For m = 0..2: `r = (float)m * 10.0f + 20.0f` (stored f32);
       for k = 0..3: `a = (float)k * 1.5707964f + 0.7853982f` (x87, not stored);
         `y = cos(a) * r * pulse + ((Y[idx][j] + 1.0f) * (int)maximumHeight * 0.5f)` (fadd, fimul, fmul; faddp);
         `x = sin(a) * r * pulse + ((X[idx][j] + 1.0f) * (int)maximumWidth * 0.5f)`; both stored f32 as args.
         `RenderOneXLUSprite_Rotate_Aspect(pos = (x, y, 100.0f), size = (15.0f, 15.0f), r, g, b, intensity 255,
         rz = 0.01f (0x3C23D70A), rotation = 0.0f (dword 0), alpha 255)`.
     - After each j: `r >>= 1; g >>= 1; b >>= 1` (logical shifts of the u32 values).
  6. RenderStateSet (8 ZWRITEENABLE, 1), (6 ZTESTENABLE, 1). Blend/vertex-alpha/raster are not restored.
- gta-reversed: absent. Open questions: none (the T-slot source address at 0x56F058 is unambiguous: `mov edi,[edx]`,
  edx = 0xB9B92C - 4*n).

## 0x6E61B0 CWaterLevel::sub_6E61B0 (proposed: CWaterLevel::TestLineAgainstWater)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr lists `TestLineAgainstWater(CVector, CVector, CVector*)` among CWaterLevel's unreversed methods
  (WaterLevel.h:355); the code walks `CWaterLevel::m_BlockPolyInfo` 0xC21B70, `m_PolyCombos` 0xC215F8, `WaterQuads`
  0xC21C90 and `m_aVertices` 0xC22910 and computes the crossing of a segment with z = 0. Class is CWaterLevel (WORLD/water),
  placed here because the function was assigned to WEAPON (its callers are CWeapon::FireInstantHit 0x73FB10, 2 sites).
- Convention: `__cdecl bool (CVector start /*by value [esp+4..+0xC]*/, CVector end /*[+0x10..+0x18]*/, CVector* out [+0x1C])`,
  returns AL (1/0, upper EAX bits garbage), plain `ret`. Preserves EBX/EBP/ESI/EDI. Locals 0x88.
- Callees: `_ftol2` 0x821B40 (truncation toward zero; result EAX) x6 sites.
- Data: `PolyInfo` = u16 (`id = v & 0x3FFF`, `type = v >> 14`: 0 none, 1 quad, 2 triangle, 3 combo);
  m_BlockPolyInfo is 12x12 u16 indexed `[x*12 + y]`; CWaterQuad stride 0xA (`verts[4]` u16 at +0,+2,+4,+6, read as
  signed via movsx); CWaterVertex stride 0x14 (`x` i16 +0, `y` i16 +2). Constants: 0x858F44 = 0.002f, 0x858B44 = 6.0f,
  0x858B50 = 0.0f.
- Behaviour:
  1. Bounds (0x6E61B0-0x6E62A9), each a strict compare with NaN going to the second operand:
     `minZ = (s.z < e.z) ? s.z : e.z`; `minY = (s.y < e.y) ? s.y : e.y`; `minX = (s.x < e.x) ? s.x : e.x`;
     `maxZ = (s.z > e.z) ? s.z : e.z`; `maxY = (s.y > e.y) ? s.y : e.y`; `maxX = (s.x > e.x) ? s.x : e.x`.
  2. Block range, in this evaluation order (each `_ftol2(v * 0.002f + 6.0f)`, fmul then fadd in x87):
     `y0 = blk(minY)`, `x1 = blk(maxX)`, `y1 = blk(maxY)`, `x0 = blk(minX)`.
  3. `for (x = x0; x <= x1; x++) for (y = y0; y <= y1; y++)` (signed compares; inner bound checked before the first
     iteration, 0x6E632A). For each block:
     a. If `x*12 < 0 || x*12 >= 0x90 || y < 0 || y >= 12` (0x6E6332-0x6E6351) -> "open sea" test (step 5).
     b. `info = m_BlockPolyInfo[x*12 + y]`: type 1 -> quad test on `WaterQuads[id]` (step 4), success returns true;
        type 3 -> combo: if `m_PolyCombos[id]` has type 0 skip, else walk `p = &m_PolyCombos[id]` while `type(*p) != 0`:
        entries of type 1 get the quad test (success -> return true), other types are skipped; `p++` (0x6E64FA).
        Types 0 and 2 (single triangle) -> nothing. Triangles are never tested (also inside combos).
  4. Quad test (0x6E6512 single / 0x6E63AF combo, identical math):
     - require `s.z * e.z < 0.0f` (fld s.z, fmul e.z, fcomp 0.0; product computed in x87; NaN/zero -> fail).
     - `dx = e.x - s.x`, `dy = e.y - s.y`, `dz = e.z - s.z` (dz stored f32); `t = |s.z| / (maxZ - minZ)` (stored f32);
       `ax = dx * t` (stored f32), `ay = dy * t`, `az = dz * t` (kept in x87);
       `out->x = ax + s.x`, `out->y = ay + s.y`, `out->z = az + s.z` (each stored f32). The point is written to `*out`
       BEFORE the containment test, so `*out` is overwritten by every candidate, also on failure.
     - containment with `v_i = m_aVertices[quad.verts[i]]` (fild of the i16): `v0.x <= out.x` (0x6E6493, fails on NaN),
       `v1.x >= out.x` (0x6E64B3), `v0.y <= out.y` (0x6E64CB), `v2.y >= out.y` (0x6E64EC). All four -> return true.
  5. Open sea (0x6E6662): require `minZ < 0.0f` and `maxZ > 0.0f` (note: different from the product test of step 4);
     compute `*out` exactly as in step 4 (written), then `if (_ftol2(out.x*0.002f+6.0f) == x && _ftol2(out.y*0.002f+6.0f) == y)`
     return true (x compared first).
  6. Loops exhausted -> return false (AL = 0).
- Edge cases: `*out` is modified even when the result is false (last candidate). `t` divides by `maxZ - minZ`, which is
  non-zero whenever a test passes. Block index uses truncation toward zero, not floor: for v in (-3500, -3000) `v*0.002+6` is in (-1, 0) and truncates to 0, so such points count as grid block 0 (port must truncate).
- gta-reversed: not reversed (declaration only). Open questions: none.
