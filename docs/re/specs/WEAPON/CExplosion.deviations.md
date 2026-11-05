# CExplosion (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Explosion.cpp` (pin 20d8d2a).

## 0x736A50 CExplosion::AddExplosion  (gr_deviations = 8)
All random numbers here are inlined: `t = (float)rand() * 3.0518509e-05f [0x858C7C]` (raw CRT rand 0x821B1E, no `& 0xFFFF`,
scale 1/32767), then f32 multiplies/adds in the order given. gr's float GetRandomNumberInRange differs in formula and
rounding (cat14 finding 1).
Rows Explosion.cpp:183, :184, :185, :187, :188 (fuel direction / offset / speed).
- Original bytes (0x736B50-0x736C17), for i = 0..2 (ebp = +0x64 + 4i offsets, ebx = +0x44 + 12i directions): if `i != 0` and
  `rand() >= 0x3FFF`: `fuelOffset[i] = 0` (gr's `>= RAND_MAX/2` = 16383, same). Else, in this order:
  `dir.x = (t + t) - 1.0f` (fadd st0,st0, 0x736B7F), `dir.y = (t + t) - 1.0f`, `dir.z = t * 0.8f + 0.2f`,
  `fuelOffset = t * 1.5f + 0.5f`, `fuelSpeed (+0x70 + 4i) = t * 10.0f + 20.0f` (one rand() each).
- Verdict: CONFIRMED for all five (different formula; same call order). Class: keep-original.
Row Explosion.cpp:200 (original-comment "Originally most likely a separate function").
- Original bytes: the FX-and-sound code is inline. Verdict: REFUTED (structure remark only). Class: harmless.
Row Explosion.cpp:377 (fire position offset `GetRandomNumberInRange(-4.0f, 4.0f)` for x and y).
- Original bytes (0x7372D4-0x737341): for fire k = 0 the position is the explosion position UNCHANGED (no rand() calls,
  `test ebx,ebx; je 0x737345`); only for k >= 1: `x = t*8.0f + pos.x - 4.0f`, `y = t*8.0f + pos.y - 4.0f` (fmul 8, fadd pos, fsub 4).
- Verdict: CONFIRMED - beyond the formula, gr offsets (and draws two random numbers for) every fire including the first.
  Class: keep-original.
Row Explosion.cpp:381 (StartFire burn time `(uint32)(GetRandomNumberInRange(5600.f, 12600.f) * 0.4f)`).
- Original bytes (0x737386-0x7373AF): `time = _ftol2(t * 0.40000004f [0x872820] * 7000.0f [0x872B6C] + 5600.0f [0x872B68])`,
  i.e. 5600 + t*2800 (range 5600..8400). gr computes (5600 + t*7000) * 0.4 = 2240..5040. Then
  `gFireManager.StartFire(firePos, 0.8f, (k != 0), m_pCreator (+0x18), time, 3, 1)` (0x539F00; third arg is unused).
- Verdict: CONFIRMED (different value range, not only rounding). Class: keep-original.
Found in passing (CONFIRMED, keep-original): fire count (switch on type, table 0x737610): type 1: `(rand() - 2) & 3`;
types 2, 3, 9: `(rand() + 1) & 3`. gr uses `% 4`, which gives a negative count (no fires) for rand() in {0, 1} in the
molotov case where the original gets 2 or 3. Ground check `FindGroundZFor3DCoord(x, y, z + 3.0f, &hit, null)` and
`|z - pos.z| < 10.0f` match gr.

## 0x737620 CExplosion::Update  (gr_deviations = 3)
Here the original calls the real helpers: `CGeneral::GetRandomNumberInRange(int, int)` 0x407180 and the float version 0x41BD90
(`min + (max - min) * t`, see cat14 finding 1); gr's helpers compute different values for the same rand().
Row Explosion.cpp:448 (aircraft: `GetRandomNumberInRange(0, 100) < 5`).
- Original bytes (0x7378D7): `CGeneral::GetRandomNumberInRange(0, 100)` (0x407180) compared `< 5`.
- Verdict: CONFIRMED (helper formula). Class: keep-original (call 0x407180 semantics).
Rows Explosion.cpp:473 and :475 (car/boat creeping fire offset).
- Original bytes (0x73770C-0x7377D8): if `m_pVictim (this-0x14 rel. ESI)` and `(rand() & 0x1F) == 0`:
  `a = GRN(-0.5f, 0.5f)` (first call), `b = GRN(-0.5f, 0.5f)` (second call); `v = (b, a, 0)` - the FIRST value goes to y
  (stored at [esp+0x18], the second at [esp+0x14] = x); `v.Normalise()`; `v *= GRN(1.0f, 2.0f)` (0x40FEF0);
  `v += m_pVictim->GetPosition()` (0x411A00: the VICTIM's position, not the explosion position); `v.z += 2.0f`;
  `CCreepingFire::TryToStartFireAtCoors(v, 0, 1, 0, 10.0f)` (0x53A450).
- Verdict (both): CONFIRMED - helper formula, plus three logic differences in gr: x/y order of the two random values,
  base position (gr: `exp.m_vecPosition`, original: victim position) and the missing `+2.0f` on z. Class: keep-original.
