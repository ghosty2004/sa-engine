# CSprite2d (RENDER) - full specs

Globals: `CSprite2d::maVertices` 0xC80468 (RwIm2DVertex[4], 0x1C each: +0 x, +4 y, +8 z, +0xC rhw, +0x10 emissive colour
(D3DCOLOR 0xAARRGGBB), +0x14 u, +0x18 v), `NearScreenZ` 0xC80460, `RecipNearClip` 0xC80464, `nextBufferIndex` 0xC80458,
`nextBufferVertex` 0xC8045C, `aTempBufferIndices` 0xC4B958 (u16[]), `aRadiosityVertexBuffer` 0xC5F958 (RwIm2DVertex[]).
CRect: +0 left, +4 top, +8 right, +0xC bottom (gr member names; gr's ctor takes (left, bottom, right, top)).
Colour packing used below (CRGBA c, bytes r,g,b,a at +0..+3): `col = (a << 24) | (r << 16) | (g << 8) | b`.

## 0x727FD0 CSprite2d::SetVerticesForSniper  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (const CRect* r [esp+4], const CRGBA* c1 [+8], const CRGBA* c2 [+0xC], const CRGBA* c3 [+0x10],
  const CRGBA* c4 [+0x14])`, plain `ret`. No callers in the original (gr calls it); no callees.
- Behaviour: `z = NearScreenZ + 1e-6f` (0x858C18 = 9.99999997e-07, x87, 0x727FDC), `w = RecipNearClip`; each vertex gets z and w
  stored as f32 (same values for all four):
  - v0 (0xC80468): x = r.left, y = r.bottom, colour c3, u = 0, v = 0 (0x727FE8-0x728039).
  - v1 (0xC80484): x = r.right, y = r.bottom, colour c4, u = 1.0f, v = 0.
  - v2 (0xC804A0): x = r.right, y = r.top, colour c2, u = 1.0f, v = 1.0f.
  - v3 (0xC804BC): x = r.left, y = r.top, colour c1, u = 0, v = 1.0f.
  (Coordinates are copied as dwords; colours packed as above.)
- Note: compared with the regular SetVertices, u/v are mirrored (v = 0 on the "bottom" edge); keep as is.
- gta-reversed: only a raw function-pointer call to 0x727FD0. Open questions: none.

## 0x728150 CSprite2d::OffsetTexCoordForBilinearFiltering  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (float width [esp+4], float height [esp+8])`, plain `ret`.
- Caller: CSprite2d::DrawWithBilinearOffset 0x7283B0.
- Behaviour: `du = 1.0f [0x858624] / (width + width)`, `dv = 1.0f / (height + height)` (x87, kept in registers); then for all four
  maVertices in order v0..v3: `u = u + du` (v0: `fld u; fadd du`; v1, v2, v3: `du + u`), `v = v + dv`; each stored f32
  (0xC8047C/0xC80480, 0xC80498/0xC8049C, 0xC804B4/0xC804B8, 0xC804D0/0xC804D4). Every vertex gets +du/+dv (a half-texel shift of the
  whole quad, not toward the centre).
- gta-reversed: raw call to 0x728150. Open questions: none.

## 0x728200 CSprite2d::AddToBuffer  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (const CRect* r [esp+4], const CRGBA* c [+8], float u1 [+0xC], v1 [+0x10], u2 [+0x14], v2 [+0x18],
  u3 [+0x1C], v3 [+0x20], u4 [+0x24], v4 [+0x28])`, plain `ret` or tail-jump.
- Callers: CFont::PrintChar 0x718A10 (4 sites). Callees: CSprite2d::SetVertices (RwD3D9Vertex overload) 0x727A00 (cdecl, 14 args:
  `(vertices, r, c, c, c, c, u1, v1, u2, v2, u3, v3, u4, v4)`, `add esp,0x38`), CSprite2d::RenderVertexBuffer 0x7273D0 (tail `jmp`).
- Behaviour: `n = nextBufferVertex`; `SetVertices(&aRadiosityVertexBuffer[n] (0xC5F958 + n*0x1C), r, c, c, c, c, u1..v4)`;
  `i = nextBufferIndex`; `idx[i] = n, idx[i+1] = n+1, idx[i+2] = n+2, idx[i+3] = n+3, idx[i+4] = n, idx[i+5] = n+2` (u16 stores
  into 0xC4B958; actual store order i+1, i, i+4, i+2, i+3, i+5); `nextBufferVertex = n + 4`; `nextBufferIndex = i + 6`.
  If `n + 4 > 0x5FC` (signed) or `i + 6 > 0xFFA` (signed): tail-jump to RenderVertexBuffer (it flushes and resets the counters).
- gta-reversed: raw call to 0x728200. Open questions: none.
