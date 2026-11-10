# CPedGroupIntelligence: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x5FC4A0 CPedGroupIntelligence::Process  (gr_deviations = 2)
Relevant original bytes (this: +4 m_CurrentEvent, +8 m_HighestPriorityEvent, +0x290 m_PrimaryTaskAllocator, +0x294
m_EventResponseTaskAllocator): when the highest-priority event becomes current (0x5FC72F-0x5FC762): `if (m_CurrentEvent) m_CurrentEvent->
vtbl[0](1)`; `m_CurrentEvent = m_HighestPriorityEvent; m_HighestPriorityEvent = 0`; `if (m_EventResponseTaskAllocator)
m_EventResponseTaskAllocator->vtbl[0](1)`; `m_EventResponseTaskAllocator = 0`; `m_EventResponseTaskAllocator = ComputeEventResponseTasks()`
0x5FC440. Later (0x5FC7A7-0x5FC7C1): if `m_PrimaryTaskAllocator == 0 && m_CurrentEvent != 0 && m_EventResponseTaskAllocator != 0`
`m_EventResponseTaskAllocator->vtbl[2](this)` (ProcessGroup), result ignored.
Row PedGroupIntelligence.cpp:192 (`if (notsa::bugfixes::GenericUB) { if (m_PrimaryTaskAllocator == m_EventResponseTaskAllocator)
m_PrimaryTaskAllocator = nullptr; }`):
- Original: no such check; the response allocator is deleted while m_PrimaryTaskAllocator may still point to it (0x5FC73C-0x5FC753), and
  the dangling primary allocator is then used at 0x5FC768 (`ProcessGroup` through a freed object).
- Verdict: CONFIRMED (gr fixes a use-after-free; the original bug must be kept or, if a port cannot reproduce freed-memory behaviour,
  recorded in the divergence ledger). Class: keep-original.
Row PedGroupIntelligence.cpp:216 (`NOTE: The original doesn't use the return value`):
- Original: the return value of `ProcessGroup` at 0x5FC7C1 is ignored, exactly as gr does.
- Verdict: REFUTED (gr matches the bytes; the comment documents it). Class: harmless.
