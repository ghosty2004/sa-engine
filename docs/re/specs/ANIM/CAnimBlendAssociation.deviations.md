# CAnimBlendAssociation: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4CE9B0 CAnimBlendAssociation::CAnimBlendAssociation()  (gr_deviations = 1)
Row AnimBlendAssociation.cpp:45 (notsa: `m_pCallbackFunc = nullptr; m_pCallbackData = nullptr;`).
- Original bytes (0x4CE9B0-0x4CE9ED, returns this in EAX): vtable +0x0 = 0x85C6D0; +0xE m_AnimGroupId = -1 (w);
  +0x10 m_BlendNodes = 0; +0x14 m_BlendHier = 0; +0x18 m_BlendAmount = 1.0f; +0x1C m_BlendDelta = 0; +0x20 m_CurrentTime = 0;
  +0x24 m_Speed = 1.0f; +0x28 m_TimeStep = 0; +0x2C m_AnimId = -1 (w); +0x2E m_Flags = 0 (w); +0x30 m_nCallbackType = 0;
  then +0x4/+0x8 (m_Link next/prev) = 0. +0xC m_NumBlendNodes and +0x34 m_pCallbackFunc / +0x38 m_pCallbackData are NOT written.
- Verdict: CONFIRMED (gr additionally nulls the callback pointers; the original leaves them uninitialised).
- Class: harmless (callback type is NONE, so the pointers are never read before SetCallback/SetFinishCallback writes them).
  A 1:1 port may leave them untouched; zeroing them is not observable.
