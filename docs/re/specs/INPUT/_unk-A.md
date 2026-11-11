# INPUT: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x5404a0 CPad::CycleCameraModeUpJustDown (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall bool ()`.
- `switch (word[this+0x10a])` (Mode, unsigned `> 3` -> AL = 0 only, upper EAX = mode): table 0x5404dc = {0x5404b3, 0x5404c1, 0x5404b3, 0x5404b3}.
- Modes 0,2,3: `return NewState.Select [+0x1a] != 0 && OldState.Select [+0x4a] == 0` (EAX = 1/0). Mode 1: same with DPadUp `+0x10` / `+0x40`.
- Open question: gr name (left unnamed in gr Pad.h:424); CPad::DisablePlayerControls is NOT checked here (0x5404a0).

## 0x540e80 CPad mouse/pad look helper X (sub_540e80) (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall i16 (void)` (this = CPad*), result in AX via tail-jumped `_ftol` 0x821b40 (upper EAX = high bits of the ftol result) or `xor ax,ax` (upper EAX stale).
- `s = (float)(i16)[0xb7345c]` (CPad::Pads[0].NewState.RightStickX, sign-extended, fild, stored f32 at 0x540e90). `a = |s|` (`fcomp 0.0f [0x858b50]; test ah,5; jp` skips `fchs` when s >= 0 or NaN, 0x540ea3).
- If `a > 100.0f [0x858628]` (`test ah,0x41; jne` skips on <= / NaN, 0x540eb2): `if (word[this+0x10e] != 0 || word[this+0x26] == 0)` go to the pad branch (0x540eca); else (+0x10e == 0 and +0x26 != 0) go to the mouse branch.
  - Pad branch: `if (s > 0.0f)` (`test ah,0x41`, 0x540eda) return `_ftol((s - 100.0f [0x858628]) + 50.0f [0x858b40])`; else return `_ftol((s + 100.0f) - 50.0f)`.
- Mouse branch (0x540f07): `if (!CCam::Using3rdPersonMouseCam(&TheCamera.m_aCams[0] 0xb6f19c))` 0x50a850 return AX = 0. Else `if (|s| <= 50.0f [0x858b40] or NaN)` return AX = 0; else `if (s > 0.0f)` return `_ftol((s - 50.0f) * 0.5f [0x858b8c])`, else `_ftol((s + 50.0f) * 0.5f)`.
- Open question: gr field names of +0x10e and +0x26 (CPad DisablePlayerControls? / NewState.ShockButtonR?) (0x540eb4/0x540ebe); the condition reads odd (control disabled -> stick still used) but is what the bytes do.

## 0x540f80 CPad mouse/pad look helper Y (sub_540f80) (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall i16 (void)`; integer arithmetic version of 0x540e80 on the Y stick.
- `v = (i16)[0xb7345e]`; `if (CPad::bInvertLook4Pad [0xb73402]) v = -v` (16-bit negate, 0x540f9c); `e = (i32)v`.
- If `|e| > 100` (signed, 0x540fb1): same +0x10e / +0x26 test as 0x540e80; pad branch returns `v > 0 ? e - 50 : e + 50` in EAX (integer, 0x540fd2/0x540fd9).
- Mouse branch (0x540fdf): `if (!Using3rdPersonMouseCam(0xb6f19c))` return AX = 0; `if (|e| <= 50)` return AX = 0; else `v > 0 ? _ftol((float)(e - 50) * 0.5f [0x858b8c]) : _ftol((float)(e + 50) * 0.5f)` (fild of the int, 0x541008/0x541022).
- Open question: as 0x540e80 (0x540fb6).
