# FxPrimBP_c (RENDER) - full specs

Layout (FxPrimBP.h, confirmed by the bytes): +0x00 vtable, +0x04 m_Type (i8), +0x05 m_nSrcBlendId, +0x06 m_nDstBlendId,
+0x07 m_bAlphaOn, +0x08 m_pMatrixBuffered (FxBufferedMatrix*, 12 x i16), +0x0C m_apTextures[4], +0x1C m_TxdIndex,
+0x20 m_Particles (TList_c: head, tail, count), +0x2C m_FxInfoManager.

## 0x4A9CF0 FxPrimBP_c::FxPrimBP_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall FxPrimBP_c* (this = ECX)`, plain `ret`, returns this.
- Callers: FxEmitterBP_c::Constructor 0x4A18D0. Callees: 0x4A8DD0 (TList_c ctor: zeroes 3 dwords, thiscall on this+0x20).
- Behaviour: `this->vtable = 0x85A9C4` (0x4A9CF6); `TList_c::ctor(this+0x20)` (+0x20, +0x24, +0x28 = 0); `+0x0C, +0x10, +0x14,
  +0x18 = 0` (the four texture pointers, 0x4A9D03-0x4A9D0C). +0x04..+0x0B, +0x1C and +0x2C.. are not initialised here.
- gta-reversed: FxPrimBP.cpp:15 (`m_apTextures.fill(nullptr)`), matches. Open questions: none.

## 0x5C2010 FxPrimBP_c::Load  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall bool (this = ECX, FILESTREAM file [esp+4], int32 version [+8], FxName32_t* textureNames [+0xC])`,
  `ret 0xC`; always returns AL = 1. Frame 0x1B4 bytes (line[256], field[128], name buffer, 12 floats).
- Callers: FxEmitterBP_c::Load 0x5C25F0. Callees: `ReadLine` thunk 0x4AB250 (-> CFileMgr::ReadLine 0x5389B0, cdecl
  (file, buf, 0x100)), `sscanf` 0x8220AD (CRT, cdecl), FxMemoryPool_c::GetMem 0x4A9CA0 (ECX = 0xA9AF2C), `_ftol` 0x821B40,
  FxInfoManager_c::Load 0x5C0B70 (thiscall on this+0x2C, args (file, version)).
- Constants: 1.0f [0x858624], 0.0f [0x858B50], 32767.0f [0x86B2C0]; formats "%s" [0x85C714], "%s %s" [0x869860],
  "%s %f %f %f %f %f %f %f %f %f %f %f %f" [0x86B2C4], "%s %d" [0x86A6AC].
- Behaviour (each "read" = `ReadLine(file, line, 256)` then `sscanf(line, fmt, ...)`; the sscanf result is ignored):
  1. Read, `sscanf("%s", field)` (FX_PRIM_BASE_DATA: header, 0x5C2031-0x5C2048).
  2. Read, `sscanf("%s %s", field, nameBuf)` (NAME:, discarded, 0x5C205B-0x5C2077).
  3. Read, `sscanf(fmt12, field, &f[0] .. &f[11])` (matrix right, up, at, pos; 0x5C208A-0x5C20E0).
  4. Identity test (0x5C20E5-0x5C21C6), in order f0 == 1, f1 == 0, f2 == 0, f3 == 0, f4 == 1, f5 == 0, f6 == 0, f7 == 0,
     f8 == 1, f9 == 0, f10 == 0, f11 == 0 (each `fcomp; test ah,0x44; jp` -> any "not equal" or NaN leaves the test).
     All equal: `m_pMatrixBuffered = 0` (0x5C21C8). Otherwise (0x5C21D4): `m_pMatrixBuffered = GetMem(0x18, 2)` (result
     not checked) and for k = 0..11 `buf[k] = (i16)ftol(f[k] * 32767.0f)` (truncation; loop of two halves of 6 values,
     0x5C21F0-0x5C227B; the pointer is re-read from this+8 before every store).
  5. Read, `sscanf("%s %s", field, textureNames[0])` (TEXTURE:, 0x5C2281-0x5C22AE).
  6. If `version > 101` (signed, `cmp ebx, 0x65; jle`, 0x5C22BD): read TEXTURE2/3/4 into `textureNames + 0x20`, `+ 0x40`,
     `+ 0x60` with "%s %s".
  7. Read, `sscanf("%s %d", field, &i)`; `m_bAlphaOn (+7) = (u8)i` (0x5C2356-0x5C2397).
  8. Read, `sscanf("%s %d", field, &i)`; `m_nSrcBlendId (+5) = (u8)i` (0x5C23CD).
  9. Read, `sscanf("%s %d", field, &i)`; `m_nDstBlendId (+6) = (u8)i` (0x5C23FC).
  10. `m_FxInfoManager.Load(file, version)`; return 1.
- Edge cases: if a sscanf fails, the uninitialised local int / floats are used as they are (no error path). The
  three byte fields take the low byte of the parsed int.
- gta-reversed: FxPrimBP.cpp:38, hooked `.Reversed = false`, code after the `plugin::Call` is unreachable but matches the
  bytes except: gr `ReadField<bool>`/`ReadField<uint8>` parse helpers (may differ in conversion of values > 255 / != 0/1;
  the original keeps the low byte) and gr's `CopyFromVectors` uses FixedVector conversion (must be `(i16)ftol(v*32767)`).
  Open questions: none.

## 0x4A9D20 FxPrimBP_c::~FxPrimBP_c  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this)`, plain `ret`. SEH frame (handler 0x83990B). Caller: FxEmitterBP_c::~FxEmitterBP_c 0x4A18F0 (tail call).
- Behaviour: `vtable = 0x85A9C4`; for the texture slots in this order: +0xC, +0x10, +0x14, +0x14 again: `if (slot) { RwTextureDestroy(slot)
  0x7F3820; slot = 0; }` (the second +0x14 test always sees 0; slot +0x18 is never destroyed). Then `FxInfoManager_c::~FxInfoManager_c(this+0x2C)`
  0x4A4950; SEH -1; `0x4A8DE0(this+0x20)` (List_c member dtor).
- gr-deviations row (FxPrimBP.cpp:21, "FIX_BUGS: They forgot destroy third texture (texture 3 destroyed twice)"): Original bytes 0x4A9D70-0x4A9D93
  test/destroy +0x14 twice and never touch +0x18. Verdict: CONFIRMED (gr destroys all four, and calls RwTextureDestroy without a null check).
  Class: keep-original (the 4th texture leaks in the original).
- Open questions: none.
