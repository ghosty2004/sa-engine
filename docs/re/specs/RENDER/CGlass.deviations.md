# CGlass (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Glass.cpp` (pin 20d8d2a). Compressed collision vertices are decoded by 0x411590
(`v = (int16 x, y, z) * 0.0078125f` [0x858B88] = 1/128).

## 0x71ACF0 CGlass::CalcAlphaWithNormal  (gr_deviations = 3)
Original: `__cdecl uint8 (const CVector* normal [esp+4])`, plain `ret`; result = low byte of `_ftol` (0x821B40) in AL (EAX full ftol).
F = the camera forward vector read directly from 0xB6F9AC/0xB6F9B0/0xB6F9B4 (copied to locals, 0x71ACF3-0x71AD17); N = *normal.
Exact x87 sequence: `d = (F.y*N.y + F.z*N.z) + F.x*N.x` (0x71AD10-0x71AD2A); `k = d + d` (0x71AD2C); `ky = k*N.y` -> f32,
`kz = k*N.z` -> f32 (0x71AD37/0x71AD40); `rx = F.x - k*N.x` (in ST, 0x71AD44); `ry = F.y - ky`, `rz = F.z - kz` (in ST);
`s = (rx + ry) * 0.57f - rz * 0.57f` (0x71AD5A-0x71AD6C; f32 [0x872728] = 0x3F11EB85); `p = s*s*s*s*s*s` (five sequential `fmul st(1)`,
0x71AD72-0x71AD7C); `return (uint8)_ftol(p * 235.0f [0x872944] + 20.0f [0x858BA4])` (0x71AD7E-0x71AD8A).
- Row Glass.cpp:485 (comment "Original code uses a hardcoded value of 0.57 ..."). Original bytes: 0.57f at 0x71AD5E and 0x71AD66.
  Verdict: REFUTED (the comment describes the original correctly; it is not itself a difference). Class: harmless.
- Row Glass.cpp:488 (`LightDir = IsFixBugs() ? (1,1,-1).Normalized() : (1,1,-1) * 0.57f`). Verdict: CONFIRMED for the FixBugs branch
  (0.57735 instead of 0.57). Class: keep-original.
- Row Glass.cpp:492 (no clamp to 0 before the power). Original bytes: no compare between 0x71AD6E and 0x71AD7E. Verdict: REFUTED
  (gr matches). Class: harmless.
Found in passing (CONFIRMED, keep-original, rounding-level): gr computes `reflect.Dot(LightDir)` (= rx*0.57 + ry*0.57 + rz*(-0.57)) and
`std::pow(specular, 6.0f)`; the original factors `(rx+ry)*0.57 - rz*0.57` and multiplies six times; gr's DotProduct for d sums
`x + y + z`, the original `(y + z) + x`. Ports must use the original order. Open question: gr reads `TheCamera.GetForward()`
(CPlaceable::m_matrix->forward); the original reads the fixed address 0xB6F9AC (0x71ACF3) - whether these are the same storage was
not checked here.

## 0x71BC40 CGlass::WindowRespondsToCollision  (gr_deviations = 1)
Original: `__cdecl void (CEntity* entity, float damage, CVector moveSpeed, CVector point, bool max1PaneSection)`.
Row Glass.cpp:209 ("R* used `Mag2D`, but `SqMag2D` is a better choice"; gr: `rng::max_element(verticesOS, {}, (v0 - v).SquaredMagnitude2D())`).
- Original bytes (0x71BD69-0x71BDD9): vertices v0..v3 decoded to locals (v0 at [esp+0x40], v1 [esp+0x4C], v2 [esp+0x58], v3 [esp+0x64]).
  `best = sqrt(dy1*dy1 + dx1*dx1)` with `dx1 = v1.x - v0.x`, `dy1 = v1.y - v0.y` (stored f32 at 0x71BD97), `idx = 1` (0x71BD6D);
  for k = 2..3: `d = sqrt(dyk*dyk + dxk*dxk)` (same form, 0x71BDA0-0x71BDB7); if `d > best` (strict; `test ah,0x41; jne` skips on
  <=, equal and NaN, 0x71BDBD-0x71BDC6): `best = d` (f32), `idx = k`. v0 itself is never a candidate.
- Verdict: CONFIRMED (edge cases only): gr includes v0 (distance 0) and compares squared distances. Results differ (a) when all of
  v1..v3 coincide with v0 in 2D (original idx 1, gr idx 0) and (b) when two different squared distances round to the same f32 sqrt
  (original keeps the earlier vertex, gr takes the larger square). Class: keep-original (use sqrt, start at v1, strict `>`).

## 0x71C2B0 CGlass::CarWindscreenShatters  (gr_deviations = 2)
Row Glass.cpp:134 (`float minRightFwdDotSum = FLT_MAX; // todo: FLT_MAX is not NOTSA ?`, loop `i = 0..5`).
- Original bytes (0x71C8FB-0x71C983): six dot products per axis are stored in A[6] at [esp+0x78] (dots with the vector at
  [esp+0x24]) and B[6] at [esp+0x9C] (dots with the vector at [esp+0x18]); each dot is `(v.x*d.x + v.z*d.z) + v.y*d.y`
  (e.g. 0x71C8D8-0x71C8F2). There is no FLT_MAX: `minSum = A[0] + B[0]` (0x71C8FB-0x71C901), `maxB = B[0]` (0x71C905), `maxA = A[0]`
  (kept in ST0, 0x71C909), `idx = 0`; then for i = 1..5 (16-bit counter `cmp dx,6`, 0x71C962): `s = A[i] + B[i]`; if `s < minSum`
  (strict, `test ah,5; jp`; NaN fails) `minSum = s, idx = i`; if `B[i] > maxB` (strict) `maxB = B[i]`; if `maxA < A[i]` `maxA = A[i]`.
  Extents: `eB = maxB - B[idx]` (0x71C968-0x71C97B), `eA = maxA - A[idx]` (0x71C97F).
- Verdict: REFUTED for finite values (FLT_MAX with strict `<` always takes element 0 first, giving the same index; gr's max_element
  returns the same values). Differs only if A[0]+B[0] is NaN or >= FLT_MAX (original idx 0, gr may pick another). Class: harmless
  (port follows the original form). Found in passing: gr's DotProduct sums `x + y + z`, the original `(x + z) + y` - CONFIRMED
  rounding-level difference, keep-original.
Row Glass.cpp:144 (`assert(0 <= idx < 6)`): debug only. Verdict: REFUTED. Class: harmless.

## 0x71CF50 CGlass::BreakGlassPhysically  (gr_deviations = 1)
Row Glass.cpp:529 (`for (auto& object : GetObjectPool()->GetAllValid())`, ascending).
- Original bytes: the object pool (CPools::ms_pObjectPool 0xB7449C; +0 objects, +4 flag bytes, +8 size) is walked DOWNWARDS:
  `ebx = size` (0x71CF68), each iteration `--ebx` (0x71CFC3), object = base + ebx*0x19C (0x71CFAC/0x71CFC4), skipped if
  `flags[ebx] & 0x80` (free, 0x71CFC9). Checks in order: type == 4 (object, 0x71CFEB), modelinfo vfunc +4 (AsAtomicModelInfoPtr)
  non-NULL (0x71D001), `(mi->+0x12 & 0x7800)` is 0x2000 or 0x2800 (glass, 0x71D00C-0x71D01E), `+0x1C & 1` (uses collision, 0x71D024).
  The early-out `now < LastColCheckMS + 1000 && now >= LastColCheckMS` (unsigned, 0x71CF6D-0x71CF7E) matches gr.
- Verdict: CONFIRMED - iteration order is reversed in gr. Because the function returns at the first matching unbroken pane and
  updates LastColCheckMS per hit, which object breaks first depends on the order. Class: keep-original (iterate from the highest slot
  down).
