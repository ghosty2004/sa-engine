# CPedIntelligence: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x6020D0 CPedIntelligence::LookAtInterestingEntities  (gr_deviations = 3)
Original bytes (`__thiscall void (this)`, frame 0x101C via `_alloca_probe` 0x822070): return if `m_pPed->flags (+0x474) & 0x400000`
(0x6020E0); return unless one of `m_apInterestingEntities[0..2]` (+0x288) is non-null (0x602100-0x60210C); return if
`g_ikChainMan.IsLooking(ped)` (0x6181A0) or `!ped->GetIsOnScreen()` (0x534540) or `CGeneral::GetRandomNumberInRange(0, 100)` 0x407180
`!= 50` (0x60213B-0x60214A). `count = -1` (int16); `CWorld::FindObjectsInRange(&ped.pos, 15.0f, 0, &count, 0x400, list, 0, 1, 1, 1, 0)`
0x564A20. Compact the entities with `IsInterestingEntity` 0x6020A0 into a list (the compacted array is addressed 3 dwords below the
result buffer, 0x6021B5 vs 0x6021A5; since the write index never exceeds the read index no unread entry is overwritten - same result as
gr's in-place compaction). If none, return. `idx = GetRandomNumberInRange(0, n)` 0x407180 (0x6021CC), then `time =
GetRandomNumberInRange(3000, 5000)` 0x407180 (0x6021DD); `g_ikChainMan.LookAt("InterestingEntities" 0x86CD60, ped, list[idx], time, -1,
&{0,0,0}, 0, 0.25f, 500, 3, 0)` 0x618970.
Rows PedIntelligence.cpp:783 (`GetRandomNumberInRange(0, 100) != 50`), :804 (`(0, interestingEntityCount)`), :805 (`(3000, 5000)`):
- Original: each is a call to the original helper 0x407180 (`min + ftol((rand() & 0xFFFF) * (1/32768) * (max - min))`), at 0x60213F,
  0x6021CC, 0x6021DD in the same order as gr. gr's inline helper uses a different formula (cat14 finding 1).
- Verdict: CONFIRMED (x3). Class: keep-original.
