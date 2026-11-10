# CAESoundManager (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Managers/AESoundManager.cpp` (pin 20d8d2a).

## 0x4EFB10 CAESoundManager::RequestNewSound  (gr_deviations = 2)
Original: `__thiscall CAESound* (this; CAESound* src [esp+4])`, `ret 4`. The loop body passes through the .text stub
0x4018E7 (`cmp word [ecx+eax+0x5C],0; jmp 0x4EFB20`) - same behaviour inline.
- Original bytes: find the first `i` in 0..299 (i16) with `m_VirtuallyPlayingSoundList[i].m_IsInUse (+0x58) == 0` (entry at
  this+4+i*0x74); none -> return 0. `dst.operator=(src)` 0x4EF680; if `src.m_PhysicalEntity (+8)`:
  `CEntity::CleanUpOldReference(src.m_PhysicalEntity, &src.m_PhysicalEntity)` 0x571A00, `src.+8 = 0`; `dst.NewVPSLEntry()`
  0x4EF7A0; `AEAudioHardware.RequestVirtualChannelSoundInfo(i, dst.m_SoundID (+2, u16), dst.m_BankSlot (+0, u16))` 0x4D8E60;
  return &dst (the `cmp si,0x12C; jge` after the call can never be taken).
Rows :344 (`#ifdef NOTSA_DEBUG`) and :345 (debug UI module): debug-only bookkeeping. Verdict: REFUTED (no game-state effect).
Class: harmless.
