# CPad (AUDIO group U1 part) - full specs (RE-SPEC-3, group U1)

CPad: +0 CControllerState NewState (0x30 bytes, i16 fields), +0x30 OldState. CControllerState+0x2E = m_bRadioTrackSkip (gr
ControllerState.h:37).

## 0x4E7F20 CPad::IsRadioTrackSkipPressed  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall i32 (this = ECX: CPad*)`, EAX = 0/1, plain `ret`. Caller: CAERadioTrackManager::Service 0x4EB9A0.
- Behaviour: return `NewState.m_bRadioTrackSkip (+0x2E) != 0 && OldState.m_bRadioTrackSkip (+0x5E) == 0` (i16 compares,
  0x4E7F20/0x4E7F27). Full EAX is written (1 or 0).
- gta-reversed: not hooked (no reversed body found); behaviour is a standard "just pressed" test. Open questions: none.
