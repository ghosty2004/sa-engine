# CFont (HUD) - gta-reversed deviation notes (group H1)

Source: `re/gta-reversed/source/game_sa/Font.cpp`. Globals: `CFont::Sprite` 0xC71AD0 (CSprite2d[2]),
`CFont::ButtonSprite` 0xC71AD8 (CSprite2d[15]).

## 0x7189B0 CFont::Shutdown  (gr_deviations = 2)
Original (0x7189B0-0x718A05, `__cdecl void ()`): `Sprite[0].Delete()` (0x7189B7, CSprite2d::Delete 0x727240),
`Sprite[1].Delete()` (0x7189C1); `RemoveTxdSlot(FindTxdSlot("fonts" 0x86A65C))` (0x7189CB/0x7189D1, CTxdStore::FindTxdSlot
0x731850, CTxdStore::RemoveTxdSlot 0x731CD0, both cdecl, `add esp, 8` after both); then `Delete()` on 4 sprites starting
at 0xC71ADC = ButtonSprite[1] .. ButtonSprite[4] (0x7189D9-0x7189EE, edi = 4); then
`RemoveTxdSlot(FindTxdSlot("ps2btns" 0x86A620))` (0x7189F0-0x7189FB).
Row Font.cpp:134 (`SafeRemoveTxdSlot("fonts")`):
- Original bytes: 0x7189CB-0x7189D1, the FindTxdSlot result is passed to RemoveTxdSlot without a -1 test.
- Verdict: CONFIRMED. Class: keep-original (a missing "fonts" slot means RemoveTxdSlot(-1)).
Row Font.cpp:136 (`SafeRemoveTxdSlot("ps2btns")`):
- Original bytes: 0x7189F0-0x7189FB, same unchecked pair. Verdict: CONFIRMED. Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr deletes all 15 `ButtonSprite` (`rng::for_each(ButtonSprite, ...)`,
Font.cpp:135); the original deletes only ButtonSprite[1..4] (0xC71ADC + 4*i, i = 0..3; ButtonSprite[0] and [5..14]
are not deleted here).

## 0x719490 CFont::SetFontStyle  (gr_deviations = 1)
Original (`__cdecl void (u8 style [esp+4])`, plain `ret`): `switch (style)`: 2 -> `m_FontTextureId (0xC71A94) = 0`,
`m_FontStyle (0xC71A95) = 2` (0x7194BA); 3 -> both = 1 (0x7194AD); any other value -> `m_FontTextureId = style`,
`m_FontStyle = 0` (0x71949F-0x7194A5). (Bytes 0xC71A94/0xC71A95 are gr `m_Details.FontTextureName` /
`m_Details.FontTextureStyle`.)
Row Font.cpp:436 (`default: NOTSA_UNREACHABLE("invalid font style {}", style)`):
- Original bytes: the default path 0x71949F stores `style` itself as the texture id and 0 as the texture style; values 0
  (GOTHIC -> tex 0, style 0) and 1 (SUBTITLES -> tex 1, style 0) also go through this path, so gr's explicit cases give the
  same results for 0..3. For style >= 4 the original stores the raw value (no trap).
- Verdict: CONFIRMED (only for out-of-range input). Class: keep-original (port: default arm stores `style` / 0).

## 0x71A700 CFont::PrintString-null-terminated  (gr_deviations = 3)
Original (`__cdecl void (float x [esp+4], float y [esp+8], const GxtChar* text [esp+0xC])`, plain `ret`):
returns if `text[0] == '*'` (0x71A70A, tested first) or `text[0] == 0` (0x71A712). If `CFont::m_bFontBackground`
(byte 0xC71A7B): copies m_Details.Color r,g,b (bytes 0xC71A60..0xC71A62) to 0xC71AB4..0xC71AB6 (RenderState.Color r,g,b;
alpha byte NOT copied), rect = (+0 1000000.0, +4 -1000000.0, +8 -1000000.0, +0xC 1000000.0) (imm 0x49742400 / 0xC9742400),
`CFont::GetTextRect(&rect, x, y, text)` 0x71A620; if `m_bEnlargeBackgroundBox` (byte 0xC71A7C): `rect[+4] -= 1.0f`,
`rect[+0xC] += 1.0f`, `rect.left -= 1.0f`, `rect.right += 1.0f` (in this order, 1.0f at 0x858624) and
`FrontEndMenuManager.DrawWindow(&rect, 0, 0, m_FontBackgroundColor (by value, dword 0xC71A84), 0, 1)` 0x573EE0; else
`CSprite2d::DrawRect(&rect, &m_FontBackgroundColor 0xC71A84)` 0x727B60; then `m_bFontBackground = 0`. Finally
`CFont::ProcessCurrentString(1, x, y, text)` 0x71A220 (cdecl, `add esp, 0x10`).
Rows Font.cpp:825 (FIX_BUGS long-string bail-out), Font.cpp:827 (NOTSA_UNREACHABLE very long string), Font.cpp:856
(stale RenderState.ContainImages):
- Original bytes: none of these exist in 0x71A700; the gr lines are in the sized overload
  `CFont::PrintString(float, float, const GxtChar*, const GxtChar*, float)` 0x719B40 (Font.cpp:765), which 0x71A700 does not
  call (it reaches it only through ProcessCurrentString). The catalogue attributed them by the overloaded name.
- Verdict: REFUTED for 0x71A700 (x3; the rows belong to 0x719B40). Class: harmless here.
Found in passing: gr (Font.cpp:745) copies the whole `m_Details.Color` into RenderState.Color including alpha; the
original copies r,g,b only (0x71A731-0x71A755). CONFIRMED, keep-original. gr's rect adjustments (`top -= 1; bottom += 1`
in gr's naming of +4/+0xC) match the bytes.
