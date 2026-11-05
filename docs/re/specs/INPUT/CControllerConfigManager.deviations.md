# CControllerConfigManager: gta-reversed deviation notes (INPUT)

Source: `re/gta-reversed/source/game_sa/ControllerConfigManager.cpp` (pin 20d8d2a). `ControlsManager` = 0xB70198.
Layout (gr header, size 0x12E4): +0x000 `m_bJoyJustInitialised`, +0x004 `m_OldJoyState` (DIJOYSTATE2, 0x110),
+0x114 `m_NewJoyState`, +0x224 `m_ControllerActionName[59]` (char[40] each, stride 0x28), +0xB5C `m_ButtonStates[17]`,
+0xB70 `m_Actions[59]` (stride 0x20; `Keys[type]` stride 8: +0 `m_uiActionInitiator` u32, +4 `m_uiSetOrder` u32),
+0x12D0 `m_bStickL_X_Rgh_Lft_MovementBothDown[4]`, +0x12D4 `m_bStickL_Up_Dwn_...[4]`, +0x12D8 `m_bStickR_X_...[4]`,
+0x12DC `m_bStickR_Up_Dwn_...[4]`, +0x12E0 `m_MouseFoundInitSet`. Address of `m_Actions[a].Keys[t]` = this + 0xB70 + a*0x20 + t*8.
Controller types: 0 KEYBOARD, 1 OPTIONAL_EXTRA_KEY, 2 MOUSE, 3 JOY_STICK. `rsNULL` = 0x420.
Action ids used below (gr enum): 0 PED_FIRE_WEAPON, 1 PED_FIRE_WEAPON_ALT, 10 VEHICLE_ENTER_EXIT, 14 PED_LOOKBEHIND,
18 VEHICLE_FIRE_WEAPON, 20-23 VEHICLE_STEER{LEFT,RIGHT,UP,DOWN}, 28 VEHICLE_RADIO_TRACK_SKIP, 36 VEHICLE_LOOKBEHIND,
37 VEHICLE_MOUSELOOK, 38-41 VEHICLE_TURRET{LEFT,RIGHT,UP,DOWN}, 44 PED_CENTER_CAMERA_BEHIND_PLAYER, 45 PED_LOCK_TARGET,
46 NETWORK_TALK, 58 SWITCH_CAM_DEBUG_MENU.

## 0x52D260 CControllerConfigManager::InitialiseControllerActionNameArray  (gr_deviations = 1)
Row ControllerConfigManager.cpp:751 (`if (notsa::IsFixBugs())` copies "VEHICLE_RADIO_TRACK_SKIP").
- Original bytes: 55 copies, each `Common::AsciiToGxtChar(asciiName, localBuf)` (0x718600) then
  `CMessages::StringCopy(&m_ControllerActionName[slot], localBuf, 0x28)` (0x69DB70). Destination slots written
  (0x52D27D ... 0x52DA17): every action except 28 (VEHICLE_RADIO_TRACK_SKIP, offset 0x684), 53 (NUM_OF_1ST_PERSON_ACTIONS)
  and 58 (SWITCH_CAM_DEBUG_MENU); those three keep whatever they held. Slot 45 (PED_LOCK_TARGET, `edi = this+0x92C`,
  0x52D2E6) is written twice with the same string (0x52D2ED and again at 0x52D3B6). String spellings in the binary:
  slot 0 "PED_FIREWEAPON", 1 "PED_FIREWEAPON_ALT", 18 "VEHICLE_FIREWEAPON", 19 "VEHICLE_FIREWEAPON_ALT", 17 "SNEAK_ABOUT".
- Verdict: CONFIRMED (vanilla never fills slot 28; gr with FixBugs does). gr's vanilla path matches.
- Class: keep-original.

## 0x52DA30 CControllerConfigManager::StoreMouseButtonState  (gr_deviations = 1)
Row ControllerConfigManager.cpp:829 (`default: NOTSA_UNREACHABLE();`).
- Original bytes: `__thiscall` (ECX unused), args `button` [esp+4], `state` (byte) [esp+8], `ret 8`. Compare chain
  1 -> 0xB73404 (left), 2 -> 0xB73406 (middle), 3 -> 0xB73405 (right), 4 -> 0xB73407 (wheel up), 5 -> 0xB73408 (wheel down),
  6 -> 0xB73409 (X1), 7 -> 0xB7340A (X2) (bytes of `CPad::TempMouseControllerState`). Any other value: falls to
  `ret 8` at 0x52DAAC, nothing written.
- Verdict: CONFIRMED (gr aborts / UB on other values; the original is a no-op). Mapping for 1-7 matches gr.
- Class: keep-original (silently ignore).

## 0x52DAB0 CControllerConfigManager::UpdateJoyInConfigMenus_ButtonDown  (gr_deviations = 1)
Row ControllerConfigManager.cpp:856 (`if ((button != JOYBUTTON_TWELVE && padNumber != 1) || IsFixBugs())`).
- Original bytes: `__thiscall(button [esp+4], padNumber [esp+8])`, `ret 8`. `if (button == 0) return` (0x52DAB5, checked
  before GetPad); `pad = CPad::GetPad(padNumber)`; `if (!pad) return` (dead). Then `switch (button - 5)` (table 0x52DBBC,
  12 entries), writing 0xFF to `pad->PCTempJoyState` (+0xA8): 5 LeftShoulder2 (+0xB2), 6 RightShoulder2 (+0xB6),
  7 LeftShoulder1 (+0xB0), 8 RightShoulder1 (+0xB4), 9 Select (+0xC2), 10 ShockButtonL (+0xCC), 11 ShockButtonR (+0xCE),
  **12 Start (+0xC0) only if `padNumber == 1`** (0x52DB0B `cmp esi,1; jne`), 13 DPadUp (+0xB8), 14 DPadRight (+0xBE),
  15 DPadDown (+0xBA), 16 DPadLeft (+0xBC). Then, for every button value (0x52DB56), if `AllValidWinJoys` pad-1
  `wVendorID (0xC9214C) == 0x3427 && wProductID (0xC92150) == 0x1190`: buttons 1..4 -> Triangle (+0xC6), Circle (+0xCA),
  Cross (+0xC8), Square (+0xC4) (table 0x52DBEC); else 1..4 -> Circle, Cross, Square, Triangle (table 0x52DBFC).
  Buttons 0 / > 16: nothing.
- Verdict: CONFIRMED. gr (vanilla) writes nothing at all for `padNumber == 1` and never writes Start; the original writes
  every button for both pads and Start only for pad 1. gr (FixBugs) also writes Start for pad 0.
- Class: keep-original.

## 0x52DC20 CControllerConfigManager::UpdateJoyInConfigMenus_ButtonUp  (gr_deviations = 1)
Row ControllerConfigManager.cpp:868 (same condition as above).
- Original bytes: identical structure to 0x52DAB0 with the value 0 (`xor esi,esi`, stores `si`); tables 0x52DD30
  (12 entries, same mapping), 0x52DD60 / 0x52DD70 (same vendor-specific mapping); Start (+0xC0) cleared only when
  `padNumber == 1` (0x52DC79).
- Verdict: CONFIRMED (same as ButtonDown).
- Class: keep-original.

## 0x52F2A0 CControllerConfigManager::GetIsKeyBlank  (gr_deviations = 1)
Row ControllerConfigManager.cpp:1014 (`default: NOTSA_UNREACHABLE();`).
- Original bytes: `__thiscall(key [esp+4], type [esp+8])`, `ret 8`, result AL. `type > 3` (unsigned) -> AL = 1 (0x52F2A7 ->
  0x52F2CC). Table 0x52F2D4 = {0x52F2B0, 0x52F2B0, 0x52F2BF, 0x52F2BF}: types 0/1 -> `key == 0x420`; types 2/3 -> `key == 0`.
  (Bodies 0x52F2B0-0x52F2C9 lie outside the catalogue extent but belong to this function.)
- Verdict: CONFIRMED (gr aborts / UB for out-of-range types; the original returns true = "blank").
- Class: keep-original.

## 0x52F550 CControllerConfigManager::GetIsActionAButtonCombo  (gr_deviations = 1)
Row ControllerConfigManager.cpp:317 (comment block "NOT USED IN SA").
- Original bytes: `xor al, al; ret 4` (thiscall, one stack arg). Always false.
- Verdict: REFUTED (gr also returns false; the switch is commented out).
- Class: harmless.

## 0x52F6F0 CControllerConfigManager::InitDefaultControlConfigMouse  (gr_deviations = 4)
Rows :689 (`#ifdef NOTSA_USE_SDL3` forced binding), :718 and :719 (comments about the assert), :720 (`assert(...)`).
- Original bytes: `__thiscall(const CMouseControllerState* setUp [esp+4], bool mouseControls [esp+8])`, `ret 8`;
  `mouseControls` is never read. No write to `m_MouseFoundInitSet` other than `= 1` when `setUp->left (+0)` is set
  (0x52F701). Calls `SetMouseButtonAssociatedWithAction(action, button)` (0x52F590):
  left -> (0,1), (18,1); right (+1) -> (45,3), (37,3); middle (+2) -> (36,2), (14,2); if wheelUp (+3) or wheelDown (+4):
  (3,4), (2,5), (26,4), (27,5), (8,4), (9,5). No assert.
- Verdict :689: CONFIRMED for gr's SDL3 build (binds everything unconditionally); the non-SDL3 path matches. Port follows
  the bytes. Class: keep-original.
- Verdict :718 / :719: REFUTED (comments only). Class: harmless.
- Verdict :720: CONFIRMED (no assert in the original). Class: harmless (a port must not assert).
- Found in passing (CONFIRMED, keep-original): gr starts with `m_MouseFoundInitSet = false;` (ControllerConfigManager.cpp
  ~:695). The original never clears it (0x52F6F0-0x52F6FB): if the left button is not detected, the previous value stays.

## 0x52F7B0 CControllerConfigManager::AffectControllerStateOn_ButtonDown_Driving  (gr_deviations = 1)
Row ControllerConfigManager.cpp:351 (comment "original double ... look behind").
- Original bytes: `__thiscall(key [esp+4], type [esp+8], CControllerState* state [esp+0xC])`, `ret 0xC`. Each test is
  `m_Actions[a].Keys[type].m_uiActionInitiator == key`. Order: 18 -> Circle(+0x22)=0xFF; 19 -> LeftShoulder1(+8);
  **36 -> LeftShoulder2(+0xA) and RightShoulder2(+0xE) both, under one compare (0x52F7E1-0x52F7EE)**; 34 -> +0xA; 35 -> +0xE;
  37 -> m_bVehicleMouseLook(+0x2C); 29 -> ShockButtonL(+0x24); 31 -> RightShoulder1(+0xC); 24 -> Cross(+0x20);
  25 -> Square(+0x1C); sticks (below); 26 -> DPadUp(+0x10); 27 -> DPadDown(+0x12); 28 -> m_bRadioTrackSkip(+0x2E);
  30 -> ShockButtonR(+0x26); turrets (below).
- Verdict: REFUTED (gr's two LOOKBEHIND checks give the same result as the single compare writing both shoulders).
- Class: harmless.
- Found in passing (CONFIRMED, keep-original): left-stick steering does NOT test the opposite value. Original
  (0x52F84D-0x52F8E3): `if (key matches 22 STEERUP) { if (bothDown_LUpDn[type] (+0x12D4+type)) { LeftStickY = 0; flag = 1; }
  else LeftStickY = -128; }`; 23 STEERDOWN same with +128; 20 STEERLEFT / 21 STEERRIGHT on LeftStickX with flag
  +0x12D0+type, -128 / +128. gr's `CheckAndSetStick` also zeroes when `state == -value` (i.e. the opposite key already set
  the axis this frame). In the original the last matching key wins unless the flag is already set. Turrets do test the
  opposite value (0x52F91A-0x52F9C9): 38 TURRETLEFT: `if (RightStickX == 128 || flagRX) { RightStickX = 0; flagRX = 1; }
  else RightStickX = -128`; 39 TURRETRIGHT mirror (test -128, set 128); 40 TURRETUP on RightStickY (test -128, set 128);
  41 TURRETDOWN (test 128, set -128); flags +0x12D8+type / +0x12DC+type. These match gr.

## 0x52FD70 CControllerConfigManager::ClearSettingsAssociatedWithAction  (gr_deviations = 1)
Row ControllerConfigManager.cpp:256 (`default: NOTSA_UNREACHABLE();`).
- Original bytes: `__thiscall(action [esp+4], type [esp+8])`, `ret 8`. Types 0/1: initiator = 0x420, order = 0; types 2/3:
  initiator = 0, order = 0; then `ResetSettingOrder(action)` (0x52F5F0). `type > 3` (0x52FD7B -> 0x52FDF6): only
  `ResetSettingOrder(action)`.
- Verdict: CONFIRMED (gr aborts / UB; the original only re-sorts).
- Class: keep-original.

## 0x530640 CControllerConfigManager::InitDefaultControlConfiguration  (gr_deviations = 3)
Rows :525 (`PED_FIRE_WEAPON_ALT` key `IsFixBugs() ? '0' : '\\'`), :546 (`PED_CENTER_CAMERA_BEHIND_PLAYER`
`IsFixBugs() ? '3' : '#'`), :593 (FixBugs-only VEHICLE_LOOKBEHIND = rsPAD5 and VEHICLE_MOUSELOOK = '*').
- Original bytes: 82 calls of `SetControllerKeyAssociatedWithAction(action, key, type)` (0x530490). 0x53079D:
  `(1, 0x5C '\\', 0)`; 0x5308B5: `(44, 0x23 '#', 0)`; no call for actions 36 / 37 with the keyboard. The function ends
  with an inlined ClearSimButtonPressCheckers (0x530ADB-0x530AF3: clears the 16 bytes +0x12D0..+0x12DF).
- Verdict :525: CONFIRMED (vanilla '\\'). :546: CONFIRMED (vanilla '#'). :593: CONFIRMED (absent in vanilla).
- Class: keep-original for all three.

## 0x531140 CControllerConfigManager::AffectPadFromKeyBoard  (gr_deviations = 1)
Row ControllerConfigManager.cpp:1137 (`#ifndef NOTSA_USE_SDL3` around `GTATranslateShiftKey(&keyCode)`).
- Original bytes: `GTATranslateShiftKey(&local, 0)` (0x531155, cdecl, two args, `add esp, 8`) is called once at entry.
- Verdict: REFUTED for the default (non-SDL3) build (the call is there). Class: harmless.
- Found in passing (CONFIRMED, keep-original): the per-action body differs from gr. Original, for each action i = 0..58
  (pointer `p = this + 0xB78 + i*0x20` = &Keys[1]; counter 0x3B, 0x53117E-0x531491); `inMenu = (CPad::padNumber == 0 &&
  !FrontEndMenuManager.m_bMenuActive (0xBA67A4))` computed once (0x53115A-0x53116F):
  1. `k0 = Keys[0].initiator`: if `GetIsKeyboardKeyDown(k0) && inMenu && k0 != 0x420`: inlined ButtonDown(k0, type 0):
     `pad = GetPad(0)`; `driving = FindPlayerVehicle(-1, false) && FindPlayerPed(-1) && ped->m_nPedState (+0x530) == 50
     && pad->DisablePlayerControls (+0x10E) == 0`; `fp = active cam mode` (0xB6F1A8 + activeCam*0x238) is one of
     16, 7, 8, 51, 34, 46; `st = &GetPad(0)->PCTempKeyState (+0x78)`; if pad: driving -> `ButtonDown_Driving(k0, 0, st)`,
     else `ButtonDown_FirstAndThirdPersonOnly(k0, 0, st)` then `fp ? ButtonDown_FirstPersonOnly : ButtonDown_ThirdPersonOnly`;
     after Driving or ThirdPersonOnly (not after FirstPersonOnly): if `Keys[0]` of action 10 (+0xCB0) == k0 ->
     `st->ButtonTriangle = 0xFF`; then `ButtonDown_AllStates(k0, 0, st)`.
  2. same for `k1 = Keys[1].initiator` with type 1 (enter/exit compare at +0xCB8).
  3. key-up: if `!GetIsKeyboardKeyDown(k0)`: if `k0 != 0x420 && GetPad(0) && !m_bMenuActive`: `if (Keys[0] of 46 (+0x1130) == k0)
     st->m_bChatIndicated (+0x28) = 0; if (Keys[0] of 37 (+0x1010) == k0) st->m_bVehicleMouseLook (+0x2C) = 0;` and k1's
     key-up is skipped for this action. Else (k0 down): if `!GetIsKeyboardKeyDown(k1)` and the same guards, the same two
     clears with Keys[1] (+0x1138, +0x1018).
  gr instead calls `AffectControllerStateOn_ButtonUp_All_Player_States` inside the key-DOWN branch (so it clears chat /
  mouse-look while the key is held) and handles both types' key-up symmetrically. The key-up path is not gated by
  `padNumber`.

## 0x531E20 CControllerConfigManager::GetControllerSettingTextWithOrderNumber  (gr_deviations = 1)
Row ControllerConfigManager.cpp:217 (`default: NOTSA_UNREACHABLE();`).
- Original bytes: `__thiscall(action, order)`, `ret 8`. Loop `t = 0..3` over `Keys[t].m_uiSetOrder` (+0xB74 + action*0x20
  + t*8): first match: t 0/1 -> `GetControllerSettingTextKeyBoard(action, t)` (0x52FE10), 2 -> `...TextMouse(action)`
  (0x52F390), 3 -> `...TextJoystick(action)` (0x52F450). No match -> nullptr. The compare chain fall-through (0x531E4F) is
  only reachable for t > 3, which the loop bound excludes.
- Verdict: REFUTED (the default is unreachable in the original too).
- Class: harmless.

## 0x531F20 CControllerConfigManager::ReinitControls  (gr_deviations = 2)
Rows :790 (`#ifdef NOTSA_USE_SDL3` MouseSetUp = {}) and :798 (`#ifdef NOTSA_USE_SDL3` joypad init).
- Original bytes (`__cdecl void()`, works on the global 0xB70198): for `type = 0..3` (outer), `action = 0..58` (inner):
  `ClearSettingsAssociatedWithAction(action, type)` (0x531F30-0x531F46; the entry contains an obfuscation `jmp` to
  0x401182 and back); `InitDefaultControlConfiguration()`; `InitDefaultControlConfigMouse(GetMouseSetUp(&tmp) /*0x53F2D0*/,
  FrontEndMenuManager.m_ControlMethod (0xBA6818) == 0)`; if `AllValidWinJoys.JoyStickNum[0].bJoyAttachedToPort (0xC92148)`:
  `caps.dwSize = 0x2C; PsGlobal->diDevice1 (RsGlobal.ps +0x20)->GetCapabilities(&caps)` (vtable +0xC), result **ignored**,
  then `InitDefaultControlConfigJoyPad(caps.dwButtons)` (0x530B00).
- Verdict :790: REFUTED for the default build (GetMouseSetUp is used); SDL3 build differs. Class: harmless.
- Verdict :798: CONFIRMED (gr's non-SDL3 branch skips the joypad init when GetCapabilities fails; the original always calls
  it with whatever `dwButtons` holds). Class: keep-original.
