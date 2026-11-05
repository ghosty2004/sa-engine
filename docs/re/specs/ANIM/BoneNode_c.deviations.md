# BoneNode_c: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x617080 BoneNode_c::QuatToEuler  (gr_deviations = 3)
Original bytes (`__cdecl void(const RtQuat* q [esp+4], CVector* out [esp+8])`, plain `ret`; q = {x +0, y +4, z +8, w +0xC}).
All arithmetic in x87 (PC 24-bit); `L*` are f32 stack temporaries (rounded on store).
  1. `L0 = (1 - 2yy) - 2zz` (0x617087-0x6170A7, 2yy computed as `y*y` then `st+st`); `Lc = 2zw + 2xy` (0x6170AD-0x6170BE);
     `v = 2zx - 2wy` (kept in ST, 0x6170C2-0x6170D1); `L1c = 2zy + 2wx` (0x6170D3-0x6170E4); `L20 = (1 - 2xx) - 2yy` (0x6170E8-0x6170F8);
     `n = -v` (0x6170FE).
  2. `c = sqrt(1 - n*n)` stored f32 into the q-argument slot (0x617102-0x61710E).
  3. `out.y = atan2(n, c) * 180.0f [0x85A994] * 0.31830987f [0x86D2B4]` (fpatan, two f32 multiplies, 0x617112-0x617126).
  4. Gimbal test on the unrounded `n`: `n == 1.0f` (0x617129-0x617134, `test ah,0x44; jnp`) or `n == -1.0f` (0x617136-0x617141).
     NaN goes to the general case.
     - Gimbal (0x61718C): `g = (1 - 2xx) - 2zz`; `h = -(2zy - 2wx)`; `out.x = atan2(h, g)*180*(1/pi)`;
       `out.z = atan2(0.0, 1.0)*180*(1/pi)` (= +0.0; f64 constants 0x859EF8 / 0x85A310).
     - General (0x617143): `r = 1.0f / c`; `out.x = atan2(L1c*r, r*L20)*180*(1/pi)`; `out.z = atan2(Lc*r, r*L0)*180*(1/pi)`.
       (fpatan operand order: ST1 = first, ST0 = second argument.)
Row BoneNode.cpp:100 ("original code:" block, then a rewritten implementation):
- Verdict: CONFIRMED - the commented block matches the bytes, but gr's live code tests `approxEqual(sinPitch, 1.f, 0.0001f)`
  (one-sided with tolerance; sinPitch = v = -n), so it misses `n == +1` and takes the gimbal path for |v-1| < 1e-4; the
  original tests `n == +-1` exactly. Class: keep-original.
Row BoneNode.cpp:119 (`out.y = RadiansToDegrees(asin(-sinPitch))` "simplified from atan2"):
- Verdict: CONFIRMED - mathematically equal for |n| <= 1, but not bit-identical (original: atan2 with an f32-rounded sqrt and two
  separate f32 multiplies 180 then 1/pi). Class: keep-original.
Row BoneNode.cpp:121 (division by c skipped):
- Verdict: CONFIRMED (numerical) - the scaling by `1/c` cancels in atan2 except for rounding, and for |n| > 1 (c = NaN) where the
  original yields NaN for out.x/out.z and gr finite values. Class: keep-original.

## 0x617530 BoneNode_c::ClampLimitsDefault  (gr_deviations = 1)
Row BoneNode.cpp:168 (`if (const auto* info = GetBoneInfo())  // BUGFIX`).
- Original bytes (`__thiscall void(bool limitX [esp+4], bool limitY [esp+8], bool limitZ [esp+0xC])`, `ret 0xC`):
  if `s_TestSkipClampDefault` (byte 0x8D2BD0, initialised to 1 in .data) != 0: return (0x617530-0x61753D). Otherwise
  find `i` with `(int16)ms_boneInfos[i].BoneTag == this->m_BoneTag (+0x8, int32)` over the 32 entries of
  `BoneNodeManager_c::ms_boneInfos` 0x8D26D0 (stride 0x28, loop ends at 0x8D2BD0); not found -> `i = -1` (0x617562), no check.
  Copy the 0x28-byte entry `ms_boneInfos[i]` to the stack (`rep movsd`, 0x61756B-0x61757E) - for i = -1 that reads
  0x8D26A8..0x8D26CF (the 40 bytes before the table). Then: limitX -> `m_LimitMax.x (+0x88) = m_LimitMin.x (+0x7C) = PoseRots.x`
  (entry+4); limitY -> `+0x8C = +0x80 = entry+8`; limitZ -> `+0x90 = +0x84 = entry+0xC` (raw dword copies).
- Verdict: CONFIRMED (gr skips the stores when the tag is unknown; the original uses entry -1). With the default flag value 1 the
  body never runs, so it is unobservable unless the byte is changed.
- Class: keep-original (index -1 must read the bytes preceding ms_boneInfos: dwords 0x36,0x36,0x36,0x37,... in the shipped .data).

## 0x617650 BoneNode_c::Limit  (gr_deviations = 1)
Original bytes (`__thiscall void(float blend [esp+4] (unused))`, `ret 4`; angles a = {x,y,z} f32 locals; m_LimitMin +0x7C/+0x80/+0x84,
m_LimitMax +0x88/+0x8C/+0x90, m_BoneTag +0x8):
  1. `QuatToEuler(&m_Orientation (+0x10), &a)` (0x617080).
  2. x: `if (a.x < min.x) a.x = min.x` (`test ah,5; jp`: NaN/>= keep); then `if (a.x > max.x) a.x = max.x` (`test ah,0x41; jne`)
     (0x617665-0x617694). Same for y (0x617698-0x6176D0). Order "min first, then max": when min > max the result is max.
  3. `lo = min.z` (x87), `hi = max.z`. If `m_BoneTag == 5` (BONE_HEAD): `e = ms_boneInfos[i]` with i = first index whose int16 tag == 5
     (-1 if none, no check; 0x6176EC-0x617704) copied to the stack; `f = 1.0f - fabs(a.x) * 0.022222223f [0x86D2B8]` (x87);
     if `0.0f > f` then f = 0 (NaN kept); `lo = (lo - e.PoseRots.z) * f + e.PoseRots.z`; `hi = (hi - e.PoseRots.z) * f + e.PoseRots.z`
     (hi stored f32; lo stays in x87) (0x617707-0x617767).
  4. `if (a.z < lo) a.z = lo` (f32 store of the x87 value); `if (a.z > hi) a.z = hi` (0x617769-0x617791).
  5. `EulerToQuat(&a, &m_Orientation)` (0x6171F0).
Row BoneNode.cpp:195 (BUGFIX: check info for BONE_HEAD):
- Verdict: CONFIRMED in code (the original has no null path and would read entry -1), but unobservable: tag 5 is entry 5 of the
  shipped table. Class: keep-original.
- Additional numeric differences (not catalogued): gr computes `|x| / -45.0f + 1.0f` (division) where the original multiplies by
  0.022222223f and subtracts from 1 - results can differ in the last bit; gr uses `std::clamp` (returns lo when lo > hi and v < lo),
  the original returns hi in that case. Keep-original.
