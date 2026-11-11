# CWorldScan (CALLED-ONLY) — RE-SPEC-REST

CRT helpers: `floor` 0x8219F0 and `ceil` 0x823820 (double in/out, x87; ceil identified by its slow-path control word [0x8E344C]=0x1B3F,
RC = up; floor uses [0x8E312C]=0x173F, RC = down), `_ftol` 0x821B40 (truncation).
Globals: extra-sector count i32 [0xC815B8], extra-sector Y list i32[4] at 0xC81598, X list i32[4] at 0xC815A8.

## 0x72CAE0 CWorldScan::ScanWorld(CVector2D* points, int32 count, void (*scan)(int32 x, int32 y))  (catalogue: specified/REA, port level 1)
- `__cdecl` with an EBP frame (`and esp, -8`); points [ebp+8], count [ebp+0xC] (the slot is rewritten when duplicates are removed),
  scan [ebp+0x10]; void; `ret`. Callers: CRenderer::GetObjectsInFrustum 0x554C60, CRenderer::ScanWorld 0x554FE0 (2 sites),
  CRenderer::RequestObjectsInFrustum 0x555960, CWaterLevel::ScanThroughBlocks 0x6E6D10. gr declares it (Renderer.h:64) but only
  calls the original. Callees: CGeneral::GetATanOfXY 0x53CC70 (cdecl (f32 x, f32 y) → ST0), floor 0x8219F0 (30 sites),
  ceil 0x823820 (4), `_ftol` 0x821B40 (30). Points are in sector units. A protection stub at 0x401512 does the first local store
  (`used[0] = 0`) and jumps back to 0x72CAF9.
- Locals: `used[5]` bytes at [esp+0x38..0x3C] (fixed size 5: more than 5 points overrun the following locals), hull array `H` of
  CVector2D at [esp+0x40] (room for 5), hull count [esp+0x20].
- Phase A, de-duplicate (0x72CAF9-0x72CBA1): for i = 0 .. count-2, for j = i+1 .. count-1: if `p[j].x == p[i].x && p[j].y == p[i].y`
  (exact compares, `test ah, 0x44; jp`): shift p[j+1..count-1] down by one element (dword copies, modifies the CALLER's array),
  `count--`, re-test the same j.
- Phase B, start vertex (0x72CBA3-0x72CC37): `s` = index of the first strict minimum of `p[i].y` (replace only when current min > p.y:
  `fcom; test ah, 0x41; jne`), loop unrolled by 4.
- Phase C, gift wrapping (0x72CC37-0x72CD4A): `used[s] = 1; H[0] = p[s]; hn = 1; prev = 0.0; cur = s`. Repeat: `best = 99999.9f`
  (immediate 0x47C34FF3); `bi` starts as the dword at [esp+0x38] (the used[] bytes — only matters when no candidate exists, i.e.
  count == 1); for every i != cur (used points included): `a = GetATanOfXY(p[i].x - p[cur].x, p[i].y - p[cur].y) - prev`
  (both differences stored to f32 arguments); `while (a < 0) a += [0x858CBC]=6.2831855f`; `while (a >= 6.2831855f) a -= 6.2831855f`
  (`test ah, 1` loops); if `a < best` (C0 only) `best = a; bi = i`. If `used[bi]` → hull closed, stop. Else
  `prev = best + prev` (f32), `H[hn++] = p[bi]; used[bi] = 1; cur = bi`.
- Phase D: if `hn < 3` → return immediately (0x72CD56 → epilogue): the extra-rectangle list is NOT flushed or reset in this case.
- Phase E, rasterise rows (0x72CD5C-0x72D58D): `yMinIdx` = first strict minimum of H[].y, `yMax` = maximum H[].y (updated only when
  strictly greater); `xLo = 9999` (0x270F), `xHi = -9999` (0xFFFFD8F1); `row = _ftol(floor(H[yMinIdx].y))`, `rowEnd = _ftol(floor(yMax))`.
  Two edge walkers start at yMinIdx: the "left" one steps to index-1 (wrapping to hn-1), the "right" one to index+1 (wrapping to 0);
  while the next vertex lies on the same floor-row, it is skipped and xLo = min(xLo, _ftol(floor(v.x))) (left) /
  xHi = max(xHi, _ftol(floor(v.x))) (right) is applied (0x72CEC0-0x72CFBF). Each edge then gets `slope = (v1.x - v0.x) / (v1.y - v0.y)`
  and `x = (ceil(v0.y) - v0.y) * slope + v0.x` (0x72CFC1-0x72D03B; ceil 0x823820); if the left slope < 0, xLo = min(xLo,
  _ftol(floor(xLeft))); if the right slope >= 0, xHi = max(xHi, _ftol(floor(xRight))) (0x72D045-0x72D0D3).
  Row loop (0x72D0F0, while row <= rowEnd): for x = xLo .. xHi inclusive: `scan(x, row)` (cdecl, pushes row then x), then remove the
  FIRST extra-list entry with (X, Y) == (x, row): shift both lists down (rep movsd), `[0xC815B8]--`, and the search ends (ECX forced to 4
  at 0x72D19D). After the row (0x72D1B8-0x72D585): both edges advance by their slope, the next row's xLo/xHi are re-derived from the
  edge x values (floor) and from any vertices crossed inside the row (left walker: index-1, right walker: index+1, with the same
  same-row skipping, slope recomputation and ceil-based re-anchoring as above; the sign of the slope selects whether `x` or `x - slope`
  feeds floor). OPEN QUESTION (port from the bytes 0x72D1B8-0x72D585 instruction by instruction): the exact choice between `x` and
  `x - slope` at 0x72D214-0x72D22A / 0x72D382-0x72D391 / 0x72D3EE-0x72D401 / 0x72D55B-0x72D56A and the vertex-crossing loops
  0x72D240-0x72D2AD, 0x72D330-0x72D36E, 0x72D420-0x72D492, 0x72D510-0x72D551 were read but not reduced to formulas here.
- Phase F (0x72D593-0x72D5C0): for i = 0 while i < [0xC815B8] (re-read each pass): `scan(X[i] (0xC815A8), Y[i] (0xC81598))`; then
  `[0xC815B8] = 0`.
- Floats: all compares are x87 compares of f32 memory against extended values; floor/ceil take doubles (`fstp qword`), results go
  through `_ftol` (truncation of an already integral value).

## 0x72D5E0 CWorldScan::SetExtraRectangleToScan(float minX, float maxX, float minY, float maxY)  (catalogue: specified/REA, port level 0)
- `__cdecl`, f32 args [esp+4], [esp+8], [esp+0xC], [esp+0x10]; `ret`; void. Callers: none in game code.
- Behaviour (sector coordinates, already in sector units):
  `for (x = _ftol(floor(minX)); x < _ftol(ceil(maxX)); x++)` and inside `for (y = _ftol(floor(minY)); y < _ftol(ceil(maxY)); y++)`:
  `n = [0xC815B8]; Y[n] (0xC81598) = y; X[n] (0xC815A8) = x; [0xC815B8] = n + 1`. The loop end values are recomputed (ceil + _ftol) on
  every iteration (0x72D674 / 0x72D690). The count is never reset here and the 4-entry lists are not bounds-checked: a rectangle
  covering more than 4 cells (or repeated calls) overruns into the following globals (original behaviour).
- gta-reversed: `plugin::Call` only.
