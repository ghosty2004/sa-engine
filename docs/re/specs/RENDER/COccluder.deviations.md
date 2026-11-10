# COccluder (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Occluder.cpp` (pin 20d8d2a). Globals: OccluderCoorsValid 0xC73CB0 (bool[]), OccluderCoors 0xC798E0
(CVector[]), OccluderCoorsOnScreen 0xC79950 (CVector[]), TheCamera.m_mViewMatrix at 0xB6FA2C.

## 0x71E130 COccluder::ProcessLineSegment  (gr_deviations = 2)
Original: `__stdcall`-like `bool (int32 idxFrom [esp+4], int32 idxTo [esp+8], CActiveOccluder* [esp+0xC])`, `ret 0xC`; entry goes through a
.text detour (0x71E135 -> 0x401759 loads `cl = OccluderCoorsValid[idxFrom]`).
Rows Occluder.cpp:181 ("BUG/NOTE/TODO", deviates=no) and :182 ("Originally they used `idxFrom` and `idxTo` for both calculations").
- Original bytes: if neither index is valid return 1 (0x71E13D-0x71E156). For the FROM point, if not valid (0x71E184-0x71E25B):
  `zA = view(C[idxFrom]).z - 1.1f` (CMatrix::operator* 0x59C890 with 0xB6FA2C; [0x858F14] = 0x3F8CCCCD) stored f32;
  `zB = |view(C[idxTo]).z - 1.1f|`; `t = zB / (|zA| + zB)` (0x71E1CC-0x71E1D6: `fabs; fld zA; fabs; fadd st1; fdivp`); `g = 1.0f - t`;
  `pt = (t * C[idxFrom]) + (g * C[idxTo])` per component (g*C[to] stored f32 first; x: 0x71E1E0-0x71E20A); then
  `CalcScreenCoors(pt, &out, ...)` 0x71DA00; on failure return 1 (0x71E563). For the TO point (0x71E28F onward) the original performs
  the IDENTICAL computation again - same zA from C[idxFrom], zB from C[idxTo], same t and same weights (C[idxTo] times g at 0x71E2EA).
- Verdict :182: CONFIRMED - gr swaps the roles for the second point (`lerp(C[other], C[i], zB/(zA+zB))` with i = idxTo). Both formulas
  describe the same near-plane intersection mathematically, so only rounding differs (and gr's `lerp` form differs from
  `t*C[from] + (1-t)*C[to]`). Class: keep-original (compute the second point exactly like the first).
- Verdict :181: REFUTED as a separate item (it is the header of the same comment). Class: harmless.
