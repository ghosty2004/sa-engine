# CBoat: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Boat.cpp` (pin 20d8d2a).

## 0x6F2710 CBoat::FillBoatList  (gr_deviations = 3)
Convention: `__cdecl void ()` (static), plain `ret`. Writes `CBoat::apFrameWakeGeneratingBoats` 0xC27994 (CBoat*[4]).
Original structure (all addresses in 0x6F2710-0x6F2937):
- Clear the 4 slots. Camera position `c` = `TheCamera.m_matrix ? &m_matrix->pos : &TheCamera.placement.pos`
  (0xB6F03C / 0xB6F02C), x and y copied to locals. Camera direction `d` = 2D `(fx, fy)` of
  `TheCamera.m_mCameraMatrix.forward` (0xB6F9AC / 0xB6F9B0, CCamera +0x984/+0x988; m_mCameraMatrix at +0x974 per
  plugin-sdk VALIDATE_OFFSET). `len = sqrt(fy*fy + fx*fx)` (0x6F2769-0x6F2773); if `len > 0` (`test ah,0x41; jne`
  skips on <=, NaN): `inv = 1/len; fx *= inv; fy *= inv` (kept in x87 registers, never stored); else d stays
  unnormalised.
- Pool loop: `i` from `pool->size` down to 1, slot `i-1` (descending, 0x6F27B2-0x6F27C9, flag byte & 0x80 = free),
  stride 0xA18. Skip null, `m_nVehicleType (+0x590) != 5` (boat), `m_NumWakeCoords (+0x644, u16) == 0`.
- `dx = f32(pos.x - c.x)`, `dy = f32(pos.y - c.y)`; `dot = dy*fy + dx*fx` (y term first, 0x6F2817-0x6F282A).
  `if (dot > 100.0f) skip` (0x6F282C-0x6F2837); `if (dot < -15.0f) skip` (0x6F283D-0x6F2848). NaN passes both.
- `d2 = f32(dy*dy + dx*dx)` (from the stored f32 dx/dy, 0x6F284E-0x6F2860); `if (d2 > 6400.0f) skip` (0x859AEC,
  `test ah,0x41; je`; NaN passes).
- `count` (16-bit, signed compare): if `count < 4`: `slots[count++] = boat`.
  Else: `best = -1, min = 999999.875f` (0x497423FE); for k = 0..3: `e = slots[k]` position, `s = (ey-cy)^2 + (ex-cx)^2`
  (y term first, x87, not rounded); `if (s < min) { min = f32(s); best = k; }` (`test ah,5; jp` keeps on >= / NaN).
  Then `if (best != -1 && (d2 < min || (boat->byte[0x36] & 0xF8) == 0 /*STATUS_PLAYER*/)) slots[best] = boat`
  (0x6F28FB-0x6F2919; `d2 < min` false on NaN).

Row Boat.cpp:454 (pool-iteration, `GetAllValid()` ascending).
- Original bytes: descending slot order (above). The first four accepted boats fill the list, so the order decides
  which boats are kept.
- Verdict: CONFIRMED.
- Class: keep-original (iterate from the last slot down).

Row Boat.cpp:473 ("Originally squared dist, compared to 6400.0F"; gr `DistanceBetweenPoints2D > 80`).
- Original bytes: `d2 = dy*dy + dx*dx` rounded to f32 and compared with 6400.0f (0x6F284E-0x6F286F).
- Verdict: CONFIRMED (sqrt then compare with 80 can differ at the boundary; gr also sums x first).
- Class: keep-original.

Row Boat.cpp:490 ("Originally squared dist", replacement search).
- Original bytes: squared 2D distances for the stored boats, compared with the squared distance of the new boat
  (0x6F28A1-0x6F2905). gr compares sqrt distances; the min-search starts at 999999.99f (same f32 999999.875).
- Verdict: CONFIRMED (monotonic, so it differs only through rounding: squared values are not rounded to f32 before
  the compare in the original, and gr rounds the sqrt). 
- Class: keep-original.

Found in passing (CONFIRMED, keep-original): gr normalises `TheCamera.m_mCameraMatrix.GetForward()` as a 3D CVector
(`Normalise()`, includes z, and sets x = 1 when the length is 0). The original normalises only the x/y pair and leaves
it unchanged when the 2D length is not > 0 (0x6F2756-0x6F2790). The dot product therefore differs whenever the camera
forward vector has a z component.
