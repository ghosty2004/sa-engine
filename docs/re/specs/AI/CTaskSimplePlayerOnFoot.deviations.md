# CTaskSimplePlayerOnFoot: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x6872C0 CTaskSimplePlayerOnFoot::PlayIdleAnimations  (gr_deviations = 1)
Original bytes (`__thiscall void (this, CPlayerPed* player [esp+4])`, `ret 4`), idle-animation part (0x68739F-0x6874B6):
`RequestModel(m_PlayerIdlesAnimBlockId (+8) + 0x63E7, 2)`; return if the anim block (ms_aAnimBlocks 0xB5D4A0 + 32*id) is not loaded
(+0x10); return if any clump association's hierarchy index (`(hier - 0xB4EA40)/0x18`) lies in [block.firstAnim (+0x14), +count (+0x18))
(0x6873BD-0x68740B); return if `player->flags (+0x46C) & 0xC` (bIsLooking OR bIsRestoringLook, 0x68740D); return unless
`touchDelta - gLastTouchTimeDelta [0xC19664] > 20000` (signed, 0x687424). Then `do i = CGeneral::GetRandomNumberInRange(0, 4) 0x407180
while (i == gLastRandomNumberForIdleAnimationID [0x8D2FEC])` (0x687430-0x687445); `BlendAnimation(clump, 0x31, 0x105 + i, 8.0f)`, flags
(+0x2E) |= 0x200; store gLastTouchTimeDelta and the id; if `CStats::GetStatValue(0x12F) != 0.0f [0x858B50]` (`test ah,0x44; jnp`: NaN
counts as != 0) and `m_snTimeInMilliseconds > 1200000` (unsigned) `player->Say(0x150, 0, 0.2f, 0, 0, 0)` 0x5EFFE0.
Row TaskSimplePlayerOnFoot.cpp:737 (`randomNumber = CGeneral::GetRandomNumberInRange(0, 4)`):
- Original: calls the original helper 0x407180 (0x687434); gr's helper formula differs (cat14 finding 1), so both the chosen anim and the
  number of retries differ for the same rand() sequence.
- Verdict: CONFIRMED. Class: keep-original.
Additional CONFIRMED difference (no row): gr skips only when `bIsLooking && bIsRestoringLook` (cpp:722 `!(a && b)`); the original
skips when EITHER bit is set (`test byte [ebp+0x46C], 0xC; jne`, 0x68740D). gr also tests the flags before scanning the
associations (no side effects; harmless order change).
