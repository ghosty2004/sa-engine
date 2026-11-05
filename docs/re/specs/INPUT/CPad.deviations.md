# CPad: gta-reversed deviation notes (INPUT)

Source: `re/gta-reversed/source/game_sa/Pad.cpp` (pin 20d8d2a). CPad layout: see CPad.md.
`0x821B40` = CRT `_ftol2` (truncation toward zero).

## 0x53F9A0 CPad::StartShake_Distance  (gr_deviations = 1)
Row Pad.cpp:548 ("originally 3x float instead of CVector").
- Original bytes: `__thiscall(int16 time [esp+4], uint8 freq [esp+8], float x [+0xC], float y [+0x10], float z [+0x14])`,
  `ret 0x14`. Returns at once if `FrontEndMenuManager.m_PrefsUseVibration (0xBA6768) == 0` or `CCutsceneMgr::ms_running
  (0xB5F851) != 0`. `c = TheCamera.m_matrix (0xB6F03C) ? &m_matrix->pos (+0x30) : &placement.pos (0xB6F02C)`;
  `d = sqrt(((c.z - z)*(c.z - z) + (c.y - y)*(c.y - y)) + (c.x - x)*(c.x - x))` (0x53F9E5-0x53FA1C, z term first);
  proceed only if `d < 70.0f` [0x858CE0] (`test ah,5; jp`: NaN and >= skip). If `freq == 0`: `ShakeDur (+0x10C) = 0;
  ShakeFreq (+0x110) = 0`. Else if `time > ShakeDur` (signed 16-bit, `jle`): `ShakeDur = time; ShakeFreq = freq`.
- Verdict: REFUTED (by-value CVector and three floats have the same stack layout; gr's logic matches). Note: gr's
  `Magnitude()` sums x, y, z in that order, the original z, y, x; with f32-precision intermediates the sum can differ in the
  last bit near the 70.0 threshold. Port must keep the z, y, x order.
- Class: harmless (keep the summation order).

## 0x53F3C0 CPad::UpdateMouse  (gr_deviations = 1)
Row Pad.cpp:281 (`#ifndef NOTSA_USE_SDL3`, lines 281-320 = the DirectInput branch).
- Original bytes (`__thiscall`, this = pad, plain `ret`):
  1. `if (!IsForegroundApp())` return (0x53F3C6).
  2. `if (PsGlobal->diMouse (0xC8CFA4) == 0)`: `diMouseInit(!m_bMenuActive (0xBA67A4) && IsVideoModeExclusive())`
     (IsVideoModeExclusive only called when the menu is inactive); then if diMouse is **still** null return, otherwise
     **continue in the same call** (0x53F402-0x53F40C).
  3. `if (GetMouseState(&ms) < 0)` return (0x53F417, DIMOUSESTATE2 local: lX +0, lY +4, lZ +8, rgbButtons +0xC).
  4. `mx = my = 1`; if menu inactive: `mx = (byte 0xBA6744 bInvertMouseX) ? -1 : 1`, `my = (byte 0xBA6745) ? -1 : 1`.
  5. Temp state (0xB73404): `amount.x (+0xC) = (float)(lX*mx)`, `amount.y (+0x10) = (float)(lY*my)`,
     **`m_fWheelMoved (+8) = (float)lZ`** (0x53F49E-0x53F4B1), buttons stored as **`rgbButtons[i] & 0x80`** (value 0x80, not 1):
     left (+0) = b[0], right (+1) = b[1], middle (+2) = b[2], X1 (+5) = b[3], X2 (+6) = b[4]; wheelUp (+3) = lZ > 0,
     wheelDown (+4) = lZ < 0 (both cleared first).
  6. `OldMouseControllerState (0xB7342C) = NewMouseControllerState (0xB73418)`; `New = Temp` (5 dwords each).
  7. `if (New.CheckForInput())` (0x53F270) `this->LastTimeTouched (+0x120) = CTimer::m_snTimeInMilliseconds`.
- Verdict: CONFIRMED. gr's non-SDL3 branch differs in three points: (a) it stores button flags as 0/1 instead of 0/0x80;
  (b) it never writes `m_fWheelMoved`; (c) after `diMouseInit` it returns without reading the mouse in that frame.
  The SDL3 branch is a different implementation altogether.
- Class: keep-original.

## 0x746A10 CPad::ProcessPad  (gr_deviations = 1)
Row Pad.cpp:325 (`#ifndef NOTSA_USE_SDL3`, lines 325-421 = the DirectInput branch).
- Original bytes (`__cdecl int32(ePadID padID [esp+4])`, returns 0 in EAX on every path, plain `ret`):
  1. `dev = padID == 0 ? &PsGlobal->diDevice1 (RsGlobal.ps 0xC17054 +0x20) : padID == 1 ? &...diDevice2 (+0x24) : nullptr`.
     For other padIDs the pointer stays 0 and `*dev` is read from address 0 (0x746A3A; would fault; never happens).
     `if (*dev == 0)` return.
  2. `Poll` (vtable +0x64); if < 0: `Acquire` (+0x1C) repeated while it returns 0x8007001E (DIERR_INPUTLOST); if < 0 return;
     `Poll` again; if < 0 return. `GetDeviceState(0x110, &js)` (+0x24); if < 0 return.
  3. `if (ControlsManager.m_bJoyJustInitialised == 1)`: `Old = js; New = js; flag = 0` else `Old = New; New = js`
     (rep movsd 0x44 dwords; Old 0xB7019C, New 0xB702AC).
  4. `RsPadEventHandler(0x20, &padID)` (0x746AF6).
  5. If `*dev != 0` (re-read): `lx (global 0xC8CFB8) = (float)js.lX * 0.0005f [0x859978]`, `ly (0xC8CFBC) = (float)js.lY * 0.0005f`;
     if `(uint16)js.rgdwPOV[0] != 0xFFFF`: `a = (float)(uint32)rgdwPOV[0] * 0.01f [0x858C58] * 0.017453292f [0x8595EC]`
     (unsigned fix-up +2^32 [0x858C54] when negative), `lx = sin(a)`, `ly = cos(a) * -1.0f`.
     If `AllValidWinJoys.JoyStickNum[padID].bZRotPresent (0xC9214A + padID*16) && .bZAxisPresent (0xC92149 + padID*16)`:
     `rx (0xC8CFB0) = (float)js.lZ * 0.0005f`, `ry (0xC8CFB4) = (float)js.lRz * 0.0005f`.
     The four stick values are **globals in PsGlobal (+0x28..+0x34)**, not locals: values not updated this call persist.
  6. **`padID` slot := `(CPad::padNumber (0xB73400) != 0)`**, then `RsPadEventHandler(0x20, &slot)`,
     `RsPadEventHandler(0x1F, &slot)`, `pad = CPad::GetPad(CPad::padNumber != 0)` (0x746BA6-0x746BEC).
  7. For each axis (order lx, ly, rx, ry): `if (fabs(v) > 0.30000001192092896 [f64 0x874AF8])` (NaN skips):
     if the invert flag is set, `v = -v` **written back to the global**; `w = _ftol2(v * 128.0f [0x858BF4])` (low 16 bits);
     lx: invert 0xBA67F9, swap 0xBA67FD: !swap -> PCTempJoyState.LeftStickX (+0xA8) else LeftStickY (+0xAA);
     ly: invert 0xBA67FA, same swap: !swap -> +0xAA else +0xA8; rx: invert 0xBA67FB, swap 0xBA67FE: !swap -> RightStickX
     (+0xAC) else RightStickY (+0xAE); ry: invert 0xBA67FC: !swap -> +0xAE else +0xAC.
- Verdict: CONFIRMED. gr's DirectInput branch differs: (a) stick values are zero-initialised locals (original: persistent
  globals; with no Z axes the right stick keeps the last value, and with "invert" set its sign flips every call because the
  negation is written back); (b) gr divides by 2000.0f instead of multiplying by 0.0005f (can differ by 1 ulp);
  (c) gr passes `padID` to the 2nd/3rd event handler calls and to GetPad, the original uses `CPad::padNumber != 0`;
  (d) gr skips stick processing when the device became null, the original still runs steps 6-7 with the stored globals;
  (e) gr returns void, the original returns 0; (f) gr's `WIN_FCHECK` on GetDeviceState vs a silent return. The order
  `pov * 0.01f * deg2rad` vs gr's `DegreesToRadians(pov / 100.0f)` can also round differently.
- Class: keep-original.
- Open question: event ids 0x20 / 0x1F correspond to gr's rsPADBUTTONUP / rsPADBUTTONDOWN only if gr's RsEvent numbering
  is right; not checked against `Rs::RsPadEventHandler` 0x6195A0.
