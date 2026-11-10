# CSprite2d (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Sprite2d.cpp` (pin 20d8d2a).

## 0x728640 CSprite2d::DrawBarChart  (gr_deviations = 1)
Original: `__cdecl void (float x, float y, u16 width, u8 height, float progress, i8 progressAdd, u8 drawPercentage,
u8 drawBlackBorder, CRGBA color, CRGBA addColor)`, plain `ret`, `sub esp,0x28` (frame F below: args at F+0x2C x, +0x30 y,
+0x34 width, +0x38 height, +0x3C progress, +0x40 progressAdd, +0x44 drawPercentage, +0x48 drawBlackBorder, +0x4C color, +0x50 addColor).
Row Sprite2d.cpp:466 (FIX_BUGS: "Because of the progressAdd overflow, the green bar is out of bounds"):
- Original bytes (0x728798-0x728843): `if ((i8)progressAdd != 0)` (byte test 0x72879F): `cur = min(unclampedCurrX, endX)` (computed
  twice, 0x7287A7-0x7287D9); `left = cur - (float)(int)(i8)progressAdd` (`movsx` + `fisub`, 0x7287E1/0x7287F2); if `!(left > x - 1.0f)`
  (`test ah,0x41; je`, equal/NaN -> clamp) `left = x - 1.0f` (x - 1.0f stored f32 first, 0x7287EE); DrawRect 0x727B60 with
  `{left, top = y + height, right = cur, bottom = y}` and colour `CRGBA(addColor.r, addColor.g, addColor.b, color.a)` (CRGBA ctor 0x7170C0).
  So the original is `max(currX - progressAdd, x - 1)`, where a progressAdd >= 0x80 is negative and pushes `left` beyond currX.
- Verdict: CONFIRMED. gr's FIX_BUGS expression `std::max(x - 1.0f, currX - progressAdd < 0 ? 127.0f : progressAdd)` is not even the
  original formula with a fix: it uses `progressAdd` (or 127) as the left coordinate instead of `currX - progressAdd`.
- Class: keep-original (port `max(cur - (int)(i8)progressAdd, x - 1.0f)` with the original compare).
Found in passing (CONFIRMED, keep-original):
- Progress clamp (0x72865C-0x728670): only `if (progress < 0.0f) progress = 0.0f` (`test ah,5; jp`); there is NO upper clamp to 100
  (gr: `std::clamp(progress, 0, 100)`); the bar end is still clamped via endX, but the "%d%%" text and the text position use the
  unclamped value.
- `endX = x + (float)width` (0x728689, stored); `unclampedCurrX = (progress * 0.01f [0x858C58]) * (float)width + x` (0x728693-0x72869F);
  gr computes `x + width * progress / 100.0f` (different rounding). `currX = unclamped < endX ? unclamped : endX` (NaN -> endX).
- Percentage text: `CFont::SetScale(fheight * 0.03f [0x858B10], fheight * 0.04f [0x858CEC])` (0x7289F3-0x728A11); gr uses
  `fheight / 0.04f` for the Y scale (gr bug, 625x too large). Format string 0x872A14 = "%d%%" with `_ftol(progress)`; orientation /
  `textX` logic (`x + 50.0f <= (float)(u16)textX` -> ALIGN_RIGHT 2, else ALIGN_LEFT 1 and `textX += 5`) and `PrintString((float)textX,
  y + 2.0f, ...)` match gr.
- Background rect colour `_ftol(color.c * 0.5f)` (0x728745..), and the first black-border rect `{x, y + 2*H/448, endX, y}`
  (H = RsGlobal.maximumHeight 0xC17048, 0x859524 = 1/448; 0x728852-0x728896) match gr; the remaining border rects were not re-derived
  in this pass.
