# CBirds (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Birds.cpp` (pin 20d8d2a). RNG: `rand` 0x821B1E (returns 0..32767), original integer range
helper 0x407180 (cdecl (min, max) -> `min + ftol((rand() & 0xFFFF) * (1/32768) * (max - min))`), float helper 0x41BD90 (cdecl
(min, max) -> `(max - min) * t + min`); gr's CGeneral::GetRandomNumberInRange differs from both (docs/re/cat14-gr-deviations.md,
finding 1). Constant 0x858C7C = 3.0518509e-05 (f32 1/32767) is the inlined float-RNG scale. CBird fields as written by
CreateNumberOfBirds relative to ESI (bird): +0x0C/+0x18 velocity copies, +0x24 angle, +0x2C wing stillness, +0x30 size,
+0x34 max distance, +0x39..+0x3B body colour, +0x3C..+0x3E wings colour, +0x40 must-do-curves.

## 0x711EF0 CBirds::CreateNumberOfBirds  (gr_deviations = 6)
Row Birds.cpp:80 `assert(i < ...)`: deviates=no, debug only - REFUTED / harmless.
Biome argument is `[esp+0x8C]` at 0x712063 (0 water -> 0x712245, 1 desert -> 0x71217F, 2 normal -> 0x7120D2, anything else ->
0x7122E2). `k = rand() % 31` (idiv, 0x712045-0x71205A) is kept in EDI and [esp+0x14].
- Row :100 (water `m_fSize = GetRandomNumberInRange(0.8F, 1.2F)`). Original 0x7122BD-0x7122DF: inlined
  `size = (float)rand() * 3.0518509e-05f [0x858C7C] * 0.40000004f [0x872820] + 0.8f [0x858C98]` (fild, fmul, fmul, fadd; stored f32).
  Verdict: CONFIRMED (gr lerp form `max*t + min*(1-t)`, t = rand/32767, rounds differently). Class: keep-original.
- Row :107 (desert size 2.0..2.5). Original 0x7121ED-0x71221C: `size = (float)rand() * 3.0518509e-05f [0x858C7C] * 0.5f [0x858B8C]
  + 2.0f [0x858CA0]`. CONFIRMED, keep-original.
- Rows :119 / :120 (normal body / wings colour `GetRandomNumberInRange(128, 256)`). Original 0x712128-0x712149:
  `c = (u8)((rand() & 0x7F) + 0x80)` written to all three bytes, body first (+0x3B, +0x3A, +0x39), then a second rand() for the
  wings (+0x3E, +0x3D, +0x3C). Range 128..255 from the low 7 bits; gr's helper maps rand()/32767 linearly onto 128..255 (different
  value for the same rand()). CONFIRMED, keep-original.
- Row :121 (normal size 0.5..0.6). Original 0x71214C-0x712174 (+ fstp 0x7122DF): `rand() * 3.0518509e-05f * 0.10000002f [0x872824]
  + 0.5f [0x858B8C]`. CONFIRMED, keep-original.
- Found in passing (CONFIRMED, keep-original):
  1. Speed factor: original `(float)k * 0.02f [0x858B38] + base` (fild + fmul, 0x7120D2/0x71218C/0x71224E; base 5.0f 0x858C80,
     3.0f 0x858B3C, 4.0f 0x858B90). gr divides `(float)k / 50.0f` - a different rounding.
  2. Biome > 2: the original jumps to 0x7122E2 and sets no speed, colour, size or wing stillness for that bird beyond the common
     part (it is still counted, `inc [0xC6A8A4]`); gr's `default:` shares the NORMAL case. Only reachable if a caller passes > 2
     (CBirds::Update passes 0..2), so harmless in practice.
  3. Water wings colour `(rand() & 0x3F) - 0x4C` (0x7122AB-0x7122B2) and body `(rand() & 0x3F) + 0x50` (0x712299-0x7122A0) match gr's
     `% 64 - 76` / `% 64 + 80` (rand() >= 0) - REFUTED as differences.

## 0x712330 CBirds::Update  (gr_deviations = 4)
- Row :152 (`iNumBirdsToCreate = GetRandomNumberInRange(size(aBirds) - uiNumberOfBirds) + 1`). Original 0x7123A4-0x7123AE:
  `n = CGeneral::GetRandomNumberInRange(1, 7 - uiNumberOfBirds)` via the original helper 0x407180 (cdecl, args pushed max first).
  Same nominal range [1, 6 - count], different formula. CONFIRMED, keep-original (port calls 0x407180's formula with (1, 7 - count)).
- Rows :173 / :174 / :176 (flight height 4..13 water, 15..25 desert, 2..10 normal). Original calls the float helper 0x41BD90 with
  (4.0f, 13.0f) at 0x712446-0x712450, (15.0f, 25.0f) at 0x71242D-0x712437, (2.0f, 10.0f) at 0x71240D-0x712417, then adds the camera
  z ([esp+0x38], 0x71245D). gr's float helper rounds differently. CONFIRMED (all three), keep-original.
- Found in passing (CONFIRMED gr LOGIC ERROR, keep-original): desert clamp. Original 0x7123D9-0x7123EF:
  `if (n >= 3 - uiNumberOfBirds) n = 3 - uiNumberOfBirds` (at most 3 desert birds in total). gr Birds.cpp:160:
  `iNumBirdsToCreate = std::min(iNumBirdsToCreate, uiNumberOfBirds)` (clamps to the CURRENT count, e.g. 0 birds -> 0 created).
- Found in passing (CONFIRMED, keep-original): water test. Original `fcomp [0x858CA4] 30.0f; test ah,5; jp` (0x7123B3-0x7123C9):
  water when `TheCamera.m_fDistanceToWater (0xB6F10C) < 30.0f`, otherwise (>= 30 or NaN) desert/normal. gr: `> 30.0F` -> not water,
  so exactly 30.0 gives water in gr and not in the original.
- Hours test: original `hours >= 22 -> skip` (0x712381), `hours <= 5 -> skip` (0x712389) = gr ClockHoursInRange(5, 22) (strict both
  sides). Frame test `(FrameCounter & 0x1FF) == 6` (0x712395) = gr. REFUTED as differences.

## 0x712810 CBirds::Render  (gr_deviations = 1)
Row Birds.cpp:345 `NOTSA_UNREACHABLE("CBirds::Render::lambda suppress warning")`.
- Original bytes: vertex loop `esi = 0 .. 7` (`inc esi; cmp esi, 8`, 0x712D3C/0x712D3D); `cmp esi, 7; ja 0x712BCF` (0x712B46) then
  `jmp [esi*4 + 0x712E18]`, table = {0x712B56, 0x712B56, 0x712B83, 0x712B83, 0x712BA6, 0x712BA6, 0x712B68, 0x712B56}: vertices 0, 1, 7
  -> (0, y, 0) body colour; 2, 3 -> (offX1, y, offZ1) body colour; 4, 5 -> (offX1+offX2, y, offZ1+offZ2) wings colour (+0x34..+0x36);
  6 -> (0.1f, y, 0) body colour. The `ja` default is never taken (esi <= 7).
- Verdict: REFUTED - the unreachable marker sits where the original has an unreachable jump-table default; same cases as gr's switch.
  Class: harmless.
