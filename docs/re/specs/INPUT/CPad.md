# CPad (INPUT) - full specs

Layout (gr `Pad.h`, `VALIDATE_SIZE(CPad, 0x134)`): +0x00 `NewState` (CControllerState, 0x30 bytes), +0x30 `OldState`,
+0x60 `SteeringLeftRightBuffer` (int16[10]), +0x74 `DrunkDrivingBufferUsed` int32, +0x78/+0xA8/+0xD8 PCTemp{Key,Joy,Mouse}State,
+0x108 `Phase` char, +0x10A `Mode` int16, +0x10C `ShakeDur` int16, +0x10E `DisablePlayerControls` u16, +0x110 `ShakeFreq` u8.
CControllerState (all int16): +0x00 LeftStickX, +0x02 LeftStickY, +0x04 RightStickX, +0x06 RightStickY, +0x08 LeftShoulder1,
+0x0A LeftShoulder2, +0x0C RightShoulder1, +0x0E RightShoulder2, +0x10 DPadUp, +0x12 DPadDown, +0x14 DPadLeft, +0x16 DPadRight,
+0x18 Start, +0x1A Select, +0x1C ButtonSquare, +0x1E ButtonTriangle, +0x20 ButtonCross, +0x22 ButtonCircle, +0x24 ShockButtonL,
+0x26 ShockButtonR, +0x28 m_bChatIndicated, +0x2A m_bPedWalk, +0x2C m_bVehicleMouseLook, +0x2E m_bRadioTrackSkip.
`CPad::Pads` = 0xB73458 (CPad[2], stride 0x134).

"Pressed" helpers below share one shape: `return NewState.X != 0 && OldState.X == 0` (two `cmp word ..., 0`), result
in EAX as 1/0 (`mov eax,1` / `xor eax,eax`, full register), `__thiscall` (this = ECX), plain `ret`.

## 0x4D59E0 CPad::IsCrossPressed  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`. Caller: CPad::WeaponJustDown 0x540250.
- `return NewState.ButtonCross (+0x20) != 0 && OldState.ButtonCross (+0x50) == 0` (0x4D59E0-0x4D59F6), EAX = 1/0.
- gta-reversed: inline in `Pad.h` (`BUTTON_IS_PRESSED(ButtonCross)`), matches. (gr's comment also labels `IsCrossDown`
  with 0x4D59E0; that is wrong, the bytes test the "pressed" edge.)

## 0x53ED60 CPad::~CPad  (catalogue: specified/REA, port level 0)
- `__thiscall void()`: a single `ret` (0x53ED60). Empty destructor; referenced only as the element dtor by
  `__init_CPad__Pads` 0x84E1F0 and `__dtor_CPad__Pads` 0x856460. gr: `~CPad() = default`.

## 0x53EDC0 CPad::IsLeftShoulder1Pressed  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`. `return NewState.LeftShoulder1 (+0x08) != 0 && OldState.LeftShoulder1 (+0x38) == 0`
  (0x53EDC0-0x53EDD6), EAX = 1/0.
- Callers: HornJustDown 0x53FF30 (tail), CarGunJustDown 0x53FFE0, ExitVehicleJustDown 0x540120 (tail), WeaponJustDown 0x540250.
- gta-reversed: inline, matches.

## 0x53EE20 CPad::IsRightShoulder1Pressed  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`. `return NewState.RightShoulder1 (+0x0C) != 0 && OldState.RightShoulder1 (+0x3C) == 0`
  (0x53EE20-0x53EE36), EAX = 1/0. Callers: HornJustDown 0x53FF30 (tail), WeaponJustDown 0x540250.
- gta-reversed: inline, matches.

## 0x53EF40 CPad::IsTrianglePressed  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`. `return NewState.ButtonTriangle (+0x1E) != 0 && OldState.ButtonTriangle (+0x4E) == 0`
  (0x53EF40-0x53EF56), EAX = 1/0. Caller: MeleeAttackJustDown 0x540390.
- gta-reversed: inline, matches.

## 0x53EF60 CPad::IsCirclePressed  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`. `return NewState.ButtonCircle (+0x22) != 0 && OldState.ButtonCircle (+0x52) == 0`
  (0x53EF60-0x53EF76), EAX = 1/0. Caller: WeaponJustDown 0x540250.
- gta-reversed: inline, matches.

## 0x53F910 CPad::SetDrunkInputDelay  (catalogue: specified/REA, port level 0)
- `__thiscall void(int32 delay [esp+4])`, `ret 4`. `this->DrunkDrivingBufferUsed (+0x74) = delay` (0x53F910-0x53F914, full dword).
- Callers: CMissionCleanup::Process 0x468560, CRunningScript::ProcessCommands1000To1099 0x489500.
- gta-reversed: inline in `Pad.h`, matches.

## 0x53FB70 CPad::GetPad  (catalogue: specified/REA, port level 0)
- `__cdecl CPad*(int32 padNumber [esp+4])`, plain `ret`. `return &Pads[padNumber]` = `padNumber * 0x134 + 0xB73458`
  (0x53FB70-0x53FB7A; `imul` 32-bit, no bounds check). 153 callers.
- gta-reversed: inline `GetPad(int32 = 0)`, matches.

## 0x5404F0 CPad::sub_5404F0  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`. Callers: CCamera::CamControl 0x527FA0 (2 sites).
- Behaviour: `m = (uint16)Mode` (+0x10A, `movzx`). `if (m > 3)` -> `xor al, al` (0x54051A). Jump table 0x540520 =
  {0x54051A, 0x540503, 0x54051A, 0x54051A}: only Mode 1 goes to 0x540503:
  `return NewState.DPadDown (+0x12) != 0 && OldState.DPadDown (+0x42) == 0` (EAX = 1/0). Modes 0, 2, 3 and > 3 return
  with only AL cleared (EAX bits 8-31 still hold the zero-extended Mode; a bool port is equivalent if the CamControl call sites only test AL - not verified).
- Semantics: "DPad-down just pressed while pad Mode == 1".
- gta-reversed: `Pad.h:367` `Mode != 1 ? 0 : IsDPadDownPressed()`, matches (Mode read as int16 in gr, the bytes use
  `movzx`: negative Modes count as > 3 in both, result false either way).

## 0x540530 CPad::sub_540530  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, plain `ret`, result EAX = 1/0. Caller: CCamera::CamControl 0x527FA0.
- Behaviour (`m = (uint16)Mode` +0x10A, 0x540530):
  - `m > 3` -> return 0 (0x54053A -> 0x540579).
  - first jump table 0x540584 = {0x540543, 0x540551, 0x540543, 0x540543}:
    - Mode 0/2/3 (0x540543): if `NewState.Select (+0x1A) != 0 && OldState.Select (+0x4A) == 0` return 1.
    - Mode 1 (0x540551): if `NewState.DPadUp (+0x10) != 0 && OldState.DPadUp (+0x40) == 0` return 1.
  - otherwise second jump table 0x540594 = {0x540579, 0x54056B, 0x540579, 0x540579}: Mode 1 (0x54056B):
    return `NewState.DPadDown (+0x12) != 0 && OldState.DPadDown (+0x42) == 0`; Modes 0/2/3 return 0.
  - Net: Mode 0/2/3 -> IsSelectPressed(); Mode 1 -> IsDPadUpPressed() || IsDPadDownPressed(); else false.
- gta-reversed: `Pad.cpp:1392`, matches (its `sub_5404F0()` fallbacks return false for Modes 0/2/3).

## 0x541D70 CPad::StopPadsShaking  (catalogue: specified/REA, port level 0)
- `__cdecl void()`: a single `ret` (0x541D70). Does nothing.
- Callers: CStreaming::RetryLoadFile 0x4076C0, CPlayerInfo::MakePlayerSafe 0x56E870, CMenuManager::Initialise 0x5744D0,
  WinMain 0x748710.
- gta-reversed: `Pad.cpp:585` loops over both pads calling `StopShaking(i)` (0x53FB50, itself a NOP in gr). Observable
  behaviour identical (nothing happens); a port must simply do nothing.
