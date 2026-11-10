# CAESmoothFadeThread (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/AESmoothFadeThread.cpp` (pin 20d8d2a). Instance 0xB608D0.

## 0x4EEEC0 CAESmoothFadeThread::Initialise  (gr_deviations = 1)
Original: `__thiscall bool (this = ECX)`, AL result, plain `ret`.
- Original bytes: InitialiseRequestSlots inlined: 0x40 entries of 0x20 bytes from +8: `entry.+0x18 = 0`, `entry.+0 = 0`
  (0x4EEED0-0x4EEED9). `h = CreateThread(NULL, 0, SmoothFadeProc 0x4EEE90, this, 4 (CREATE_SUSPENDED), &this->+4)` (IAT
  0x8580B4); `+0 = h`. If `h == -1` (only that value, 0x4EEEEF): `+0x808 = 0`, return 0. Else `SetThreadPriority(h, 0)`,
  `+0x814 = 0`, `+0x808 = 1`, return 1.
Row AESmoothFadeThread.cpp:39 (`|| !m_threadHandle // NOTSA: nullptr check`):
- Verdict: CONFIRMED - CreateThread reports failure with NULL, which the original does not test (it treats NULL as success
  and calls SetThreadPriority(NULL, 0)). gr also returns void. Class: keep-original (test only -1).
