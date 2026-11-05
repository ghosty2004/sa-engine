# CAnimBlendNode: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4CFB90 CAnimBlendNode::NextKeyFrameNoCalc  (gr_deviations = 1)
Row AnimBlendNode.cpp:109 (`NOTSA_UNREACHABLE("Unused Function")`).
- Reachability: not unused - CAnimBlendNode::UpdateTime 0x4D0160 tail-jumps here (0x4D017F) when the association is playing
  and `m_KFRemainingTime -= assoc->m_TimeStep` drops to <= 0. UpdateTime itself has no static callers (catalog/callgraph.csv), so
  the chain is dead in practice, but gr's reversed UpdateTime would hit the unreachable marker if it were called.
- Original bytes (`__thiscall bool()`, return AL; node layout +0x8 m_KFCurr i16, +0xA m_KFPrev i16, +0xC m_KFRemainingTime f32,
  +0x10 m_Seq, +0x14 m_BlendAssoc; sequence +0x4 flags byte (bit1 HasTranslation), +0x6 m_FramesNum i16, +0x8 m_Frames):
  1. `n = (int16)seq->m_FramesNum`; if `n <= 1` (signed) return false (0x4CFB95-0x4CFBA2).
  2. `looped = false; m_KFPrev = m_KFCurr` (0x4CFBB8). If NOT (`m_KFRemainingTime <= 0.0f`) - i.e. > 0 or NaN - skip to 4
     (`fcomp 0.0f [0x858B50]; test ah,0x41; jp`, 0x4CFBA3-0x4CFBBF).
  3. do { `m_KFCurr = (int16)(m_KFCurr + 1)` (0x4CFBC1-0x4CFBC9);
       if `m_KFCurr >= n` (signed): if `assoc->m_Flags (byte +0x2E) & 2` (LOOPED): `m_KFCurr = 0; looped = true` (0x4CFBDC);
         else: `m_KFCurr = m_KFCurr - 1` (= n-1), `m_KFRemainingTime = 0.0f`, return false (0x4CFC3A-0x4CFC4C; m_KFPrev keeps the
         value from step 2);
       `kf = frames + m_KFCurr * (flags bit1 ? 0x20 : 0x14)` (KeyFrameTrans / KeyFrame, always the UNcompressed layout, even for
       compressed sequences); `m_KFRemainingTime = kf->DeltaTime (+0x10) + m_KFRemainingTime` (fld; fadd; fstp f32, 0x4CFC08-0x4CFC0E);
     } while (`m_KFRemainingTime <= 0.0f`) (`test ah,0x41; jnp` loops on < or ==, NaN exits; 0x4CFC11-0x4CFC1F).
  4. `m_KFPrev = (int16)(m_KFCurr - 1)`; if negative: `m_KFPrev += n` (16-bit, 0x4CFC21-0x4CFC2F). Return `looped` (0x4CFC36).
- Verdict: CONFIRMED (gr has no implementation; the original body above is real code).
- Class: keep-original (port the body as above; it is equivalent to NextKeyFrame 0x4D04A0 without the CalcDeltas call).
