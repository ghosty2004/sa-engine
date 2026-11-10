# CAEAudioChannel (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Hardware/AEAudioChannel.cpp` (pin 20d8d2a).

## 0x4D7A10 CAEAudioChannel::Lost  (gr_deviations = 1)
Original: `__thiscall bool (this = ECX)`, returns AL = 1, plain `ret`. Called (tail) from UpdateStatus 0x4D7BD0.
Row AEAudioChannel.cpp:208 ("BUG: Infinite loop if we don't restore").
- Original bytes: EDI = `Sleep` import (IAT 0x8580C4). Loop at 0x4D7A20: `hr = buf(+8)->Restore()` (vtable +0x50);
  if `hr == 0x88780096` (DSERR_BUFFERLOST) `Sleep(10)` (0x4D7A30); then `hr = buf->Restore()` AGAIN (0x4D7A3A); if
  `hr == 0x88780096` repeat from 0x4D7A20. Return `al = 1`.
- Verdict: REFUTED - the original has the same unbounded loop (a buffer that never restores spins forever), so the gr
  comment describes original behaviour. Side note (not in gr): the original calls Restore() at least twice even when
  the first call succeeds (0x4D7A26 and 0x4D7A3A), gr calls it once. Class: harmless (port keeps the two calls).
