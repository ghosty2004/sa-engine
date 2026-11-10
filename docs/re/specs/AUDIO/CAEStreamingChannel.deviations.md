# CAEStreamingChannel (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Hardware/AEStreamingChannel.cpp` (pin 20d8d2a).

## 0x4F18A0 CAEStreamingChannel::UpdatePlayTime  (gr_deviations = 1)
Row :336 ("Originally a bitwise hack: x &= (x <= 0) - 1"):
- Original bytes: `t = +0x6007C; +0x6007C = t & ((t <= 0) - 1)` (`setle; dec; and`, 0x4F19A3-0x4F19B3) BEFORE
  `+0x60088/+0x6008C = GetCurrentTimeInMS()` (u64, 0x4F19B9-0x4F19C4); then the return value is computed again from the
  stored value as `t & ((t < 0) - 1)` (`setl`, 0x4F19D2-0x4F19D9).
- Verdict: REFUTED - both are `max(0, t)`; the store/time order differs from gr but the two are independent. Class: harmless.
