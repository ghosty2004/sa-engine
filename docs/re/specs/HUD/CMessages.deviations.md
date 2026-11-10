# CMessages (HUD) - gta-reversed deviation notes (group H1)

Source: `re/gta-reversed/source/game_sa/Messages.cpp`.

## 0x69DC50 CMessages::CutString  (gr_deviations = 1)
Row Messages.cpp:402 (`NOTSA_UNREACHABLE(); // unused`):
- Original bytes 0x69DC50-0x69DCCF (`__cdecl int16 (int16 width [esp+4], GxtChar* str [esp+8], GxtChar** lines [esp+0xC])`,
  plain `ret`, result in AX; no callers in the catalogue): `len = (int16)strlen(str)`; `n = 0`, `pos = 0` (int16);
  if `len <= 0` return 0. Loop while `n < 8`: `lines[n] = str + pos`; `n++`; if `pos + width >= len` stop; `pos += width`;
  while `str[pos] != ' '` `pos--` (no lower bound); `str[pos] = 0`; `pos++`; continue while `pos < len`. Return n.
- Verdict: CONFIRMED (gr has no implementation). Class: harmless (no caller); a port may implement the bytes above.

## 0x69DE90 CMessages::InsertNumberInString  (gr_deviations = 1)
Original: `__cdecl void (const GxtChar* str, int32 n1..n6, GxtChar* dst)` (dst at [esp+0x20]), plain `ret`.
Row Messages.cpp:440 (comment: sprintf + AsciiToGxtChar replaced by std::format, only "~1~" handled):
- Original bytes 0x69DE90-0x69E02B: if `str == 0`: `*dst = 0`, return. `c = 0` (u8 counter); `sprintf(num, "%d" (0x859508),
  n1)`; `nlen = strlen(num)`; `AsciiToGxtChar(num, gnum)`. `L = strlen(str)`; `out = 0`; for (i = 0; i < L; ): if
  `str[i] == '~' && str[i+1] == '1' && str[i+2] == '~'`: `i += 3`; copy `nlen` bytes of gnum to `dst + out`, `out += nlen`;
  `c++`; c = 1..5 -> `sprintf(num, "%d", n2..n6)` (table 0x69E02C); c >= 6 -> no sprintf (the last number, n6, is reused);
  then `nlen = strlen(num)`, `AsciiToGxtChar(num, gnum)`. Otherwise `dst[out++] = str[i++]`. Finally `dst[out] = 0`.
- Verdict: CONFIRMED for strings with more than six "~1~" markers (original repeats n6; gr indexes past its 6-element
  array). For up to six markers the output is identical. Class: keep-original.

## 0x69E040 CMessages::InsertStringInString  (gr_deviations = 1)
Original: `__cdecl void (GxtChar* target [esp+4], const GxtChar* repl [esp+8])`, plain `ret`; 400-byte local buffer.
Row Messages.cpp:453 (`notsa::string_copy(buf, target, sizeof(buf))` + StringReplace("~1~")):
- Original bytes 0x69E040-0x69E150: return if either pointer is null. `lt = strlen(target)`, `lr = strlen(repl)`,
  `total = (u16)(lt + lr)`; `in = 0`, `out = 0` (u16). While `out < total`: if `target[in] == '~' && target[in+1] == 'a' &&
  target[in+2] == '~'` (0x69E0A8-0x69E0B6, marker "~a~", 0x61): `in += 3`, append all lr bytes of repl to buf; else
  `buf[out++] = target[in++]`. `buf[out] = 0`. Then copy `total` bytes (not out+1) from buf to target and, if `total <
  0x190`, zero-fill target[total .. 399] (rep stos, 0x69E127-0x69E144). The scan reads up to `lr` bytes past target's
  terminator when there is no marker (they are copied but follow the copied NUL).
- Verdict: CONFIRMED - gr searches "~1~" instead of "~a~" (strings with ~a~ are not substituted in gr), does not
  zero-fill target up to 400 bytes, and bounds the copy. Class: keep-original.
