# CWaterCannon (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/WaterCannon.cpp` (pin 20d8d2a). Layout: +0 m_nId, +4 section counter (int16 in the
original), +8 creation time, +0xC section points[32] (CVector), +0x18C section move speeds[32], +0x30C used[32] (u8).

## 0x7295E0 CWaterCannon::PushPeds  (gr_deviations = 2)
Original: `__thiscall void ()` (ECX = CWaterCannon*), plain `ret`; has an SEH frame (CEventHitByWaterCannon local).
Row WaterCannon.cpp:71 (FIX_BUGS: gr starts the box at +/-FLT_MAX).
- Original bytes: `min = (10000, 10000, 10000)`, `max = (-10000, -10000, -10000)` (imm 0x461C4000 / 0xC61C4000, 0x729605-0x72962D).
  For k = 0..31 with `m_abUsed[k]` (+0x30C+k): per axis `if (!(min < p)) min = p` (0x72964B, so NaN p is taken) and
  `if (!(max > p)) max = p` (0x729660).
- Verdict: CONFIRMED (CAT-14 spot check agrees). Class: keep-original (+/-10000 seeds).
Row WaterCannon.cpp:91 (pool-iteration `GetPedPool()->GetAllValid()`).
- Original bytes: ped pool walked from slot size-1 DOWN to 0 (0x7296E7-0x729B03), stride 0x7C4. Per ped the original adds an
  event and two particles, so the visiting order is observable (event/particle order, FX pool order).
- Verdict: CONFIRMED. Class: keep-original (descending order).
Found in passing (all CONFIRMED, keep-original; gr's unhooked-but-installed code is wrong here):
- Ped filter: the original requires `ped->m_nFlags (+0x1C) & 1` (CEntity bUsesCollision; `test b[esi+0x1C],1; je skip`, 0x729719).
  gr instead skips peds with `physicalFlags.bMakeMassTwiceAsBig` (+0x40 bit 0): different field and inverted sense.
- Box test is strict on all axes: `min.x < p.x < max.x`, same for y, z (0x729730-0x7297C8; equal or NaN fails).
- Inner loop: only sections with `m_abUsed[k]` are tested (0x7297D0); gr tests all 32 sections, used or not.
  Distance: `((s.z-p.z)^2 + (s.y-p.y)^2) + (s.x-p.x)^2 < 5.0f [0x858C80]` (strict).
- Hit: `CEventHitByWaterCannon(&sectionPoint[k], &sectionMoveSpeed[k])` (0x4B1290) added with `ped->m_pIntelligence (+0x47C)
  ->m_eventGroup (+0x68).Add(&ev, false)` (0x4AB420); `ped->+0x46C &= ~3`; `ApplyMoveForce(CVector(0, 0, CTimer::ms_fTimeStep))`.
  Then per axis x (+0x44 / speed+0) and y (+0x48 / speed+4): `a = (0.1f [0x872A28] * speed - ped.mv) * 0.1f [0x872A2C]`;
  `if (a * speed > 0.0f) ped.mv = a + ped.mv` (0x72992F `fadd [esi+0x44]`). gr writes `applyable + secMoveSpeed` - wrong
  term (the original adds the ped's own move speed). Particles: FxPrtMult_c(1,1,1,0.6,0.75,0,0.2); g_fx.m_SmokeII3expand
  ([0xA9AE24])->AddParticle(pedPos, mv*0.3f, 0, mult, -1, 1.2, 0.6, 0) then (pedPos, (mv.x*-0.3f, mv.y*-0.3f, mv.z*-0.3f + 0.5f), ...)
  (0.3f = 0x858C24, -0.3f = 0x858EE4); then break (at most one section per ped).

## 0x72A280 CWaterCannon::Update_OncePerFrame  (gr_deviations = 1)
Original: `__thiscall void (int16 index [esp+4])`, `ret 4`.
Row WaterCannon.cpp:44 ("Originally done in a separate loop, but we do it here").
- Original bytes: loop 1 (0x72A2D0-0x72A330), k = 0..31 with `m_abUsed[k]`: `speed.z = speed.z - ms_fTimeStep * 0.004f [0x859CE0]`
  (stored), then `p += (ts*speed.x, ts*speed.y, ts*speed.z)` (all three products computed before the adds). Loop 2
  (0x72A340-0x72A375), k = 0..31 with `m_abUsed[k]`: `gFireManager.ExtinguishPointWithWater(p, 2.0f, 0.5f)` (0x5394C0).
- Verdict: REFUTED - the extinguish calls see the same section positions in the same ascending order; the extinguish does not
  touch the cannon, so merging the loops is equivalent. Class: harmless.
Found in passing (CONFIRMED, keep-original): (a) gr computes `GetTimeStep() / 250.0f`, the original multiplies by 0.004f (f32
constant, not exactly 1/250): last-bit differences. (b) Section counter: `+4` is handled as int16 (`movsx`/`mov w`,
0x72A29B-0x72A2AF): `n = (int16)(((int32)n + 1) % 32)` (signed remainder), written back as 16 bits; gr uses an int32 field.
(c) `if (now > m_nCreationTime + 150)` unsigned, then `m_abUsed[n] = 0`; PushPeds when `((frameCounter + index) & 3) == 0`;
if no section is used, `m_nId (+0) = 0` (scan uses an int16 counter) - these match gr.
