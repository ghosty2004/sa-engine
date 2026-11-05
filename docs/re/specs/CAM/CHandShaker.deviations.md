# CHandShaker: gta-reversed deviation notes (CAM)

Source: `re/gta-reversed/source/game_sa/HandShaker.cpp`. Layout: see CHandShaker.md (m_ang +0x00, m_lim +0x0C,
m_motion +0x18, m_vel +0x24, m_slow +0x30, m_resultMat +0x3C, scaleMin +0x84, scaleMax +0x88, twitchFreq +0x8C,
twitchVel +0x90). `0x821B1E` = CRT `rand()` (0..32767), `0x821B40` = CRT `_ftol2` (truncation).
Constants: 0x858C7C = 3.0518509e-05f (1/32767), 0x858B14 = 3.0517578e-05f (1/32768), 0x858B50 = 0.0f,
0x858C1C = -1.0f, 0x858B38 = 0.02f, 0x858C4C = 1000.0f, 0x858C54 = 4294967296.0f, 0x85A6EC = 0.03f.

## 0x50D930 CHandShaker::Process  (gr_deviations = 2)
Convention: `__thiscall void (CHandShaker* this = ECX, float degree [esp+4])`, `ret 4` (0x50DD64).

Original algorithm (needed to judge both rows):
1. Scales (0x50D936-0x50D97D): `d = scaleMax - scaleMin` (x87); for i = x,y,z in that order:
   `scale[i] = fabs(ang[i] / lim[i]) * d + scaleMin` (stored f32).
2. Slow factor per axis (x 0x50D97F, y 0x50D9C8, z 0x50DA13): `if ((ang[i] > 0 && vel[i] > 0) || (ang[i] < 0 && vel[i] < 0))
   scale[i] *= slow[i]`. Strict compares against 0.0f; zero or NaN in either never multiplies.
3. Random motion (0x50DA5E-0x50DB2F): three `rand()` calls, NOT masked, each `r = (float)rand() * (1/32767)`:
   call 1 -> z, call 2 -> y, call 3 -> x. `m[i] = r_i * motion[i] * scale[i]` (fmul order exactly so). Then for
   x, y, z: `if (ang[i] > 0) m[i] *= -1.0f` (0x50DAD4/0x50DAED/0x50DB07). Then `vel.x += m.x; vel.y += m.y; vel.z += m.z`.
4. `t = (float)(uint32)_ftol2(ms_fTimeStepNonClipped [0xB7CB58] * 0.02f * 1000.0f) * 0.03f` (0x50DB32-0x50DB61; the
   4294967296.0f add at 0x50DB55 is the unsigned fix-up for a negative int).
   `n = _ftol2((float)twitchFreq * t)` (fild of the i32 at +0x8C, 0x50DB65-0x50DB6F).
   `k = _ftol2((float)(rand() & 0xFFFF) * (1/32768) [0x858B14] * (int)(n - 1) [fimul])` (0x50DB76-0x50DB97);
   twitch happens iff `k + 1 == 2`, i.e. `k == 1` (0x50DB9C-0x50DBA1).
5. Twitch (0x50DBA7-0x50DC71): with `lo = -twitchVel`, `hi = twitchVel`: three unmasked `rand()` calls,
   `r = rand() * (1/32767)`; call 1 -> z, call 2 -> y, call 3 -> x; each component `= (hi - lo) * r + lo`;
   then `vel += (x, y, z)` (x first in the stores).
6. `ang[i] = t * vel[i] + ang[i]` for x, y, z (0x50DC74-0x50DCAC).
7. Clamp per axis (x 0x50DCAF, y 0x50DCDC, z 0x50DD07): `if (ang > lim) ang = lim; else if (ang < -lim) ang = -lim`
   (NaN: neither branch).
8. `m_resultMat.SetRotate(ang.x * degree, ang.y * degree, ang.z * degree)` (CMatrix::SetRotate(float,float,float)
   0x59B120, thiscall ECX = this+0x3C, args pushed z, y, x).

Row HandShaker.cpp:45 (rng-helper, `float motion = CGeneral::GetRandomNumberInRange(0.f, 1.f) * m_motion[i] * scale`).
- Original bytes: step 3 above. The value itself equals gr's (`lerp(0,1,t)` = `1*t + 0*(1-t)` = t with t =
  rand() * (1/32767), the same constant 0x858C7C, unmasked rand), but the original draws the three numbers in the
  order z, y, x (rand call at 0x50DA65 feeds `motion.z`, 0x50DA83 feeds y, 0x50DAA1 feeds x), while gr's loop
  draws x, y, z. Also gr applies the slow factor using `signbit(ang) == signbit(vel)`, which differs from step 2 when
  either value is zero (e.g. right after Reset `ang = 0`: gr multiplies by `m_slow` whenever `vel >= +0`; the
  original never does).
- Verdict: CONFIRMED (axis/rand order and the zero-sign condition; formula itself is equal).
- Class: keep-original.

Row HandShaker.cpp:55 (rng-helper, `GetRandomNumberInRange((size_t)((float)m_twitchFreq * twitchT)) == 1`).
- Original bytes: step 4 above: `_ftol2((rand() & 0xFFFF) * (1/32768) * (n - 1)) == 1` (0x50DB7B mask,
  0x50DB8D 1/32768, 0x50DB93 fimul n-1). gr computes `(size_t)lerp(0, n - 1, rand() / 32767.f)`, i.e.
  `(n-1)*t + 0*(1-t)` with t = rand()/32767 - different divisor (cat14 finding 1), so for some rand values the result
  differs. In addition gr's `twitchT` uses `ms_fTimeStepNonClipped / 50.0f * 1000.0f` while the original multiplies by
  0.02f (0x858B38) then 1000.0f; 0.02f is not exactly 1/50, so the truncated millisecond count can differ by one at
  boundaries. The twitch vector (step 5) is also drawn in z, y, x order with `(hi - lo) * r + lo`; gr's
  `CVector::Random(-v, v)` draws x, y, z with `hi * t + lo * (1 - t)`.
- Verdict: CONFIRMED.
- Class: keep-original.
