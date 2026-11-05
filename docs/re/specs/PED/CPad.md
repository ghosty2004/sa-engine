# CPad (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x53EE60 CPad::IsDPadUpPressed  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = CPad*), return EAX = 0/1 (full register), plain `ret`.
- Callers: CPlayerPed::ProcessControl 0x60EA90.
- Fields: `NewState.DPadUp` (+0x10, int16), `OldState.DPadUp` (+0x40, int16) (CControllerState 0x30 bytes, OldState at +0x30).
- Behaviour: `return NewState.DPadUp != 0 && OldState.DPadUp == 0` (0x53EE60-0x53EE76), i.e. "just pressed".
- gta-reversed: `IsDPadUpPressed()` = `BUTTON_IS_PRESSED(DPadUp)` (`Pad.h:295`), same.
