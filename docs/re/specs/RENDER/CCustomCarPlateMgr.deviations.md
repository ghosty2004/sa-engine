# CCustomCarPlateMgr (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/CustomCarPlateMgr.cpp` (pin 20d8d2a). `rand` 0x821B1E; `_ftol2` 0x821B40 (truncation).
Globals: pCharsetLockedData 0xC3EF78, CurrentLicensePlateMaterial 0xC3EF7C, CurrentLicensePlateType 0xC3EF80 (u8).

## 0x6FD5B0 CCustomCarPlateMgr::GeneratePlateText  (gr_deviations = 5)
Convention: `__cdecl bool (char* out [esp+4], i32 length [esp+8])`, AL, plain `ret`. Letter L() = `'A' - ftol((float)(rand() & 0xFFFF)
* 3.0517578e-05f [0x858B14] * -23.0f [0x872574])` = 'A'..'W'; digit D() = `'0' - ftol(... * -9.0f [0x872570])` = '0'..'8' (the
0x407180 formula, inlined, with (max - min) negated). Sequence: `length < 4` (signed `cmp ebx,4; jge`, 0x6FD5B5) -> return 0;
out[0] = L (0x6FD5C0-0x6FD5EB), out[1] = L, out[2] = D, out[3] = D; then i = 4 and while i < length (signed; first test 0x6FD66D
`jle`, loop test 0x6FD706): out[i] = ' ', out[i+1] = D, out[i+2] = L, out[i+3] = L, i += 4. Return 1. No terminator is written, and
up to 3 bytes past `length` are written when length % 4 != 0 (same in gr).
- Rows :404 (rbf MissingLettersAndDigits, letters) and :410 (rbf, digits). Original: letters A..W (0x6FD5D8 -23.0f), digits 0..8
  (0x6FD62F -9.0f). gr with the fix on draws A..Z / 0..9. Verdict: CONFIRMED. Class: keep-original.
- Rows :405 / :406 / :410 (rng-helper: gr `GetRandomNumberInRange('A', 'X')` / `('0', '9')` with the fix off). Original uses the
  `min + ftol((rand() & 0xFFFF) * (1/32768) * (max - min))` form (inlined, above); gr's helper is `lerp(min, max-1, rand()/32767.0f)`,
  a different value for the same rand(). CONFIRMED (all three). Class: keep-original.
- Found in passing: the original length test is signed (a length >= 0x80000000 returns 0); gr's `uint32 length < 4` is unsigned.
  CONFIRMED, harmless (no caller passes such lengths).

## 0x6FD7C0 CCustomCarPlateMgr::GetCharacterPositionInCharSet  (gr_deviations = 1)
Row CustomCarPlateMgr.cpp:20 (commented-out hook: "Incompatible with original function (As it uses __usercall and passes the first
arg in `al`)").
- Original bytes: non-standard convention - the character in AL (`movzx eax, al`, 0x6FD7C0), out-pointers `u32* col [esp+4]`,
  `u32* row [esp+8]`, plain `ret` (caller pops, 0x6FDDF9 `add esp, 8`). Index `c - 0x20`, `cmp eax, 0x5A; ja default` (0x6FD7C6),
  byte table 0x6FDB98 (0x5B entries) -> dword jump table 0x6FDB00 (38 targets). Decoded mapping: 'A'..'Z' (and 'a'..'z') ->
  (col, row) = (k % 4, k / 4) for k = 0..25; '0' (2,6), '1' (3,6), '2'..'5' (0..3, 7), '6'..'9' (0..3, 8); every other character
  (including ' ') -> default 0x6FDAE9 = (0, 9).
- Verdict: REFUTED as a behavioural difference - gr's switch returns exactly the same (col, row) for every character; only the calling
  convention differs (gr returns a std::pair). Class: harmless (a port must keep the AL-register convention only if it is called
  from original code; its only caller is 0x6FDD70).

## 0x6FDD70 CCustomCarPlateMgr::RenderLicenseplateTextToRaster  (gr_deviations = 4)
Rows :221 (`// FIX ME`), :223-225 (asserts; deviates=no, harmless, REFUTED).
- Original bytes: `__cdecl bool (const char* text [esp+4], RwRaster* chars [esp+8], void* palette [esp+0xC] (unused), RwRaster* plate
  [esp+0x10])`. `p = RwRasterLock(plate, 0, 5)` (5 = WRITE | NOFETCH, 0x6FDD78-0x6FDD7D); return 0 if p == 0, if
  pCharsetLockedData == 0, if `chars->stride (+0x18) == 0` or `plate->stride == 0` (0x6FDD85-0x6FDDAB) - the three later failures
  return WITHOUT unlocking `plate`. Then for letter = 0..7 (0x6FDE26): col = row = 1 (pre-set, 0x6FDDC4), call 0x6FD7C0 with
  AL = text[letter]; `src = pCharsetLockedData + ((row << 6) + col) << 5` (= row*2048 + col*32 bytes, 0x6FDDF1-0x6FDDFC); copy 16
  rows (EDI = 0x10) of 32 bytes (byte loop, 0x6FDE07-0x6FDE11), src += chars stride, dst += plate stride; dst column += 0x20.
  `RwRasterUnlock(plate)`, return 1.
- Verdict for :221: REFUTED - gr's body is the same algorithm (same offsets row*32*16*4 + col*8*4, same early returns without
  unlock, same lock flags); "FIX ME" marks no behavioural change. Class: harmless.

## 0x6FDFE0 CCustomCarPlateMgr::SetupClumpAfterVehicleUpgrade  (gr_deviations = 2)
Rows :335 (`assert(plateMaterial); // NOTSA: Don't think this should happen?`), :336 (`assert(clump)`, harmless).
- Original bytes: `__cdecl bool (RpClump* clump [esp+4], RpMaterial* material [esp+8], u8 type [esp+0xC])`. If material == 0 ->
  return 0 (0x6FDFE4-0x6FDFEA). Else CurrentLicensePlateType (0xC3EF80) = type, CurrentLicensePlateMaterial (0xC3EF7C) = material,
  `RpClumpForAllAtomics(clump, 0x6FDFC0 AtomicUpgradeSetCarplateTextureCB, 0)` 0x749B70, return 1.
- Verdict: REFUTED - the original also has the null-material early return; the NOTSA note only concerns the debug assert.
  Class: harmless.
