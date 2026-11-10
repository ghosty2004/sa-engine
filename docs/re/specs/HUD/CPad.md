# CPad (HUD group H1) - full specs

All six are static `__cdecl bool ()` with plain `ret`, returning AL only (EAX upper bytes keep whatever the loaded byte
left: `mov al, [..]` leaves EAX[8..31] unchanged). Pattern "just pressed": `New != 0 && Old == 0` -> AL = 1, else AL = 0.
gta-reversed: inline Pad.h macros KEY_IS_PRESSED / MOUSE_IS_PRESSED (Pad.h:39/45) with the same semantics. No callees.
CMouseControllerState (gr): +0 lmb, +1 rmb, +2 mmb, +3 wheelUp, +4 wheelDown, +5 bmx1, +6 bmx2 (bytes);
`CPad::NewMouseControllerState` 0xB73418, `CPad::OldMouseControllerState` 0xB7342C.

## 0x572DB0 CPad::IsEscJustPressed  (catalogue: specified/REA, port level 0)
- `NewKeyState.esc` (i16 at 0xB733A8) != 0 (0x572DB0) and `OldKeyState.esc` (i16 at 0xB73138) == 0 (0x572DBA).
  Caller: CMenuManager::DrawBackground 0x57B750 (3 sites). gr Pad.h:268. Open questions: none.

## 0x572E70 CPad::IsMouseRButtonPressed  (catalogue: specified/REA, port level 0)
- Byte 0xB73419 (New.rmb) != 0 and byte 0xB7342D (Old.rmb) == 0 (0x572E70/0x572E79). Caller:
  CMenuManager::AdditionalOptionInput 0x5773D0. gr Pad.h:340.

## 0x57C400 CPad::IsMouseWheelUpPressed  (catalogue: specified/REA, port level 0)
- Byte 0xB7341B (New.wheelUp) != 0 and byte 0xB7342F == 0. Callers: CMenuManager::CheckRedefineControlInput 0x57E4D0,
  CMenuManager::UserInput 0x57FD70. gr Pad.h:342.

## 0x57C420 CPad::IsMouseWheelDownPressed  (catalogue: specified/REA, port level 0)
- Byte 0xB7341C != 0 and byte 0xB73430 == 0. Callers: 0x57E4D0, 0x57FD70. gr Pad.h:343.

## 0x57C440 CPad::IsMouseBmx1Pressed  (catalogue: specified/REA, port level 0)
- Byte 0xB7341D != 0 and byte 0xB73431 == 0. Caller: 0x57E4D0. gr Pad.h:344.

## 0x57C460 CPad::IsMouseBmx2Pressed  (catalogue: specified/REA, port level 0)
- Byte 0xB7341E != 0 and byte 0xB73432 == 0. Caller: 0x57E4D0. gr Pad.h:345.
