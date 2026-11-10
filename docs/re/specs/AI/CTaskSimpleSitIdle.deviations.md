# CTaskSimpleSitIdle: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

Layout used (gr TaskSimpleSitIdle.h): +0x8 m_bSitAfterStep (u8), +0xC m_durationMs, +0x10 m_anim, +0x14 timer start, +0x18
timer interval, +0x1C timer started (u8), +0x1D timer "stopped/reset" (u8). Inline RNG: `rand` 0x821B1E, `_ftol` 0x821B40.

## 0x631260 CTaskSimpleSitIdle::StartAnim  (gr_deviations = 1)
Original bytes (`__thiscall void (this, CPed* ped [esp+4])`, `ret 4`): timer start = m_snTimeInMilliseconds, interval = m_durationMs
(+0xC), started = 1 (0x631263-0x631272); `m_anim = CAnimManager::BlendAnimation(ped->m_pRwClump (+0x18), grp, anim, 256.0f)` 0x4D4610
with (grp, anim) = m_bSitAfterStep ? (0x35, 0x136) : (0, 0x96) (0x631276-0x6312A2); if m_anim != 0:
`m_anim->m_fSpeed (+0x24) = (float)rand() * 3.0518509e-05f [0x858C7C] * 0.6f [0x858CC8] + 0.9f [0x858C20]` (fild, fmul, fmul, fadd,
fstp; 0x6312B1-0x6312D3).
Row TaskSimpleSitIdle.cpp:44 (`m_anim->m_Speed = CGeneral::GetRandomNumberInRange(0.9f, 1.5f)`):
- Original: inline `rand()*(1/32767)*0.6 + 0.9` (0x6312B1-0x6312CD). gr's float helper computes `1.5*t + 0.9*(1-t)` (common.h:197 lerp):
  equal in exact arithmetic, different rounding.
- Verdict: CONFIRMED (rounding level). Class: keep-original.

## 0x637A60 CTaskSimpleSitIdle::ProcessPed  (gr_deviations = 3)
Original bytes (`__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`), in order:
  1. if m_anim (+0x10) == 0 StartAnim(ped) (0x637A66-0x637A74).
  2. if `!g_ikChainMan.IsLooking(ped)` (0x6181A0) AND `ftol((rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 100.0f [0x858628]) < 5`
     (0x637A8C-0x637AB2):
     a. `closest = ped->m_pIntelligence (+0x47C)->m_pedScanner (+0x124).GetClosestPedInRange()` 0x5FFF20 (0x637AB8-0x637AC4).
     b. `lookTime = 3000 - ftol((rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * -5000.0f [0x86E630])` = 3000 + trunc(r*5000/32768)
        in [3000, 7999] (0x637ACB-0x637AF3); converted to float and kept as f32 `fLook` (0x637AFB-0x637AFF).
     c. if closest != 0 (0x637B03): `d = closest.pos - ped.pos` (f32 per component); `f = ped forward` (CPlaceable::GetTopDirection
        0x41CCB0 = matrix +0x10); `dot = (d.z*f.z + d.y*f.y) + d.x*f.x` (0x637B4D-0x637B63); if `dot <= 0.2f [0x858CC4]`
        or NaN (`test ah,0x41; jne`) -> go to d. Else `if (CGeneral::GetRandomNumberInRange(0, 100) 0x407180 < 25)` (0x637B72-0x637B81)
        `g_ikChainMan.LookAt("TaskSitIdle" 0x86E624, ped, closest, ftol(fLook), 5 (BONE_HEAD), nullptr, 0, 0.25f, 750, 3, 0)`
        0x618970 (0x637B83-0x637CC6) and skip d; otherwise fall to d.
     d. Random point (0x637BA7-0x637CC6): `p = ped.pos`; `s = (float)rand() * 3.0518509e-05f [0x858C7C] * 4.0f [0x858B90] + 1.0f
        [0x858624]`; `p += forward * s` (per component); `s2 = (float)rand() * 3.0518509e-05f [0x858C7C] * 8.0f [0x859000] - 4.0f
        [0x858B90]`; `p += right * s2` (GetRightDirection 0x41CC70); `LookAt("TaskSitIdle", ped, nullptr, ftol(fLook), -1, &p, 0,
        0.25f, 750, 3, 0)`.
  3. Timer (0x637CCB-0x637D28): if started (+0x1C): if +0x1D set, restart (start = now, +0x1D = 0); if `now >= start + interval`
     (unsigned `cmp; sbb; inc`): if IsLooking(ped) `AbortLookAt(ped, 750)` 0x618280; return 1. Return 0.
Row TaskSimpleSitIdle.cpp:74 (`lookTime = CGeneral::GetRandomNumberInRange(3000, 5000)`):
- Original: range 3000..7999 via an inlined helper with factor -5000 (0x637ACB-0x637AF3); gr gives 3000..4999 with a different formula.
- Verdict: CONFIRMED (range AND formula differ). Class: keep-original.
Additional difference in the same block (no row): gr draws the 25% chance (`RandomBool(25.f)`) BEFORE the dot-product test; the
original calls 0x407180 only after the dot test passed (0x637B65-0x637B76), so RNG consumption differs. gr's `<= 0.2f` lets NaN pass;
the original rejects NaN. CONFIRMED, keep-original.
Rows TaskSimpleSitIdle.cpp:109 / :110 (`GetRandomNumberInRange(1.f, 5.f)` / `(-4.f, 4.f)`):
- Original: inline `r*(1/32767)*4 + 1` and `r*(1/32767)*8 - 4` (0x637BCA-0x637BEA, 0x637C33-0x637C53); the forward offset is drawn and
  added before the right offset is drawn. gr's lerp `max*t + min*(1-t)` rounds differently.
- Verdict: CONFIRMED (rounding level). Class: keep-original.
