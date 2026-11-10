# CHud (HUD) - gta-reversed deviation notes (group H1)

Source: `re/gta-reversed/source/game_sa/Hud.cpp`. Notation: `sx = (float)RsGlobal.maximumWidth (i32 0xC17044, fild) *
0.0015625f (0x859520)`, `sy = (float)RsGlobal.maximumHeight (i32 0xC17048, fild) * 0.0022321430f (0x859524)`, `H` = fild of
0xC17048. CRect offsets: +0 left, +4 (gr "top"), +8 right, +0xC (gr "bottom").

## 0x58A330 CHud::DrawRadar  (gr_deviations = 1)
Row Hud.cpp:1161 (`// NOTSA: rects are optimized`, four `Sprites[SPRITE_RADAR_DISC].Draw(rect, black)`):
- Original bytes 0x58A782-0x58AA2D: four independent rect computations, each `CRGBA(0,0,0,255)` built with the CRGBA
  ctor 0x7170C0 and `Sprites[3]` (0xBAB208).Draw(&rect, color) 0x728350; draw ORDER is top-left, top-right, bottom-left,
  bottom-right (gr: TL, BL, TR, BR). Exact expressions (x87, each rect field stored f32; constants 40.0f 0x858A10,
  4.0f 0x858B90, 94.0f 0x866B78, 0.5f 0x858B8C, 104.0f 0x866B70, 76.0f 0x866B74):
  - X "left" pair: A = `40*sx - 4*sx` (0x58A7A3-0x58A7AD: `(40*sx) - (sx*4)`), B = `((sx*4) + 40*sx) + sx*94`
    (0x58A846-0x58A84E). X "right": `(sx*94)*0.5 + 40*sx` (0x58A7E7-0x58A7F3).
  - Y: `T0 = H - sy*104` (stored f32), then +4 = `(sy*76)*0.5 + T0` (0x58A7FB-0x58A80F); upper +0xC = `T0 - sy*4`
    (0x58A7D3-0x58A7DD); lower +0xC = `(sy*4 + T0) + T76` with `T76 = sy*76` stored f32 (0x58A91F-0x58A935).
  - TL: left A, +4 upper, right R, +0xC = T0 - 4sy. TR: left B (138sx), right R (87sx; mirrored, left > right).
    BL: left A, +0xC = lower (about H - 24sy). BR: left B, +0xC = lower.
  - Then tail-jump `CRadar::DrawBlips 0x588050` (0x58AA2D).
  gr computes `SCREEN_STRETCH_X(36.0f)` etc. directly; algebraically equal, but the original's sums of separately rounded
  terms (e.g. `40*sx - 4*sx` vs `36*W/640`) round differently in f32, and the draw order differs.
- Verdict: CONFIRMED (numeric expressions and draw order). Class: keep-original (port the exact expressions above;
  the sub-pixel differences are visible in pixel-exact comparisons).
