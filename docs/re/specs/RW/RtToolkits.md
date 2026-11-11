# RenderWare 3.6 small toolkits (RtBMP, RtDict) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 toolkit code in gta_sa.exe 1.0 US; gta-reversed has thunks only (rtbmp.cpp, rtdict.cpp).
All functions are `__cdecl`. `G` = `RwEngineInstance` (0xc97b24); `G+0x134 rwmalloc`, `G+0x138 rwfree`.

## Layouts used

`RwImage`: +0x18 palette (`RwRGBA*`, 0x7ce17b). `RwError` {+0 pluginID, +4 errorCode}. BMP headers are read into one
0x400-byte stack buffer at [esp+0x64] (after the 4 register pushes; frame `sub esp, 0x454`).
`RtDictSchema` (gr rtdict.h): +0x0 name, +0x4 dictChunkType, +0x8 entryChunkType, +0xc compatibilityVersion, +0x10
`RwSList* dictionaries` (0x7ced58), +0x14 `RtDict* current` (0x7ced61), +0x18.. callbacks.

## 0x7CDF60 RtBMPImageRead  (catalogue: specified/REA, port level 2)
- Convention: `RwImage* (const RwChar* filename)`; returns the image or NULL. Caller CPlayerSkin::GetSkinTexture 0x6ffa10.
  Locals (F = esp after the pushes): image [F+0x10], bytesRead [F+0x14], RLE pair / error struct [F+0x18..], depth
  [F+0x20], stream [F+0x24], offBits [F+0x38], width [F+0x40], height [F+0x44], bitCount (i16) [F+0x4a], compression [F+0x4c].
  Error code used: `RwErrorSet({0x19d, _rwerror(code)})`.
- Behaviour:
  1. `s = RwStreamOpen(2 /*rwSTREAMFILENAME*/, 1 /*rwSTREAMREAD*/, filename)` (0x7cdf76); NULL -> NULL.
  2. `RwStreamRead(s, buf, 4) != 4` -> close, NULL; `(buf & 0xffff) != 0x4d42` ('BM') -> close, NULL (0x7cdfc3).
  3. `RwStreamRead(s, F+0x66, 0xe) != 0xe` -> close, NULL (0x7cdfeb). This reads the rest of BITMAPFILEHEADER plus biSize:
     `offBits = [F+0x6c]`, `biSize = [F+0x70]` (0x7ce010..0x7ce023).
  4. `RwStreamRead(s, F+0x64, biSize - 4) != biSize - 4` -> close, NULL (0x7ce028). No size check: biSize > 0x404 overruns the
     stack buffer. `bytesRead = biSize + 0xe` (0x7ce04c).
  5. biSize == 0xc (core header, 0x7ce04f): width = (i16)[F+0x64], height = (i16)[F+0x66], bitCount = [F+0x6a], compression = 0.
     Else (info header): width = [F+0x64], height = [F+0x68], bitCount = [F+0x6e], compression = [F+0x70], clrUsed = [F+0x80];
     compression == 2 (BI_RLE4) -> RwErrorSet(0x80000009), close, NULL (0x7ce0ac..0x7ce0cf).
     `numColors = clrUsed` if `0 < clrUsed <= (1 << bitCount)` (0x7ce0e4..0x7ce0fa), else `1 << bitCount`; `depth = bitCount`.
  6. Image depth: bitCount 1 or 4 -> 4; 8 -> 8; anything else -> 32 (0x7ce10e..0x7ce129).
     `img = RwImageCreate(width, height, imgDepth)` (0x7ce139); NULL -> close, NULL. `RwImageAllocatePixels(img)` (0x7ce163);
     0 -> RwImageDestroy, close, NULL.
  7. Palette, if `bitCount <= 8` (signed word compare, 0x7ce173): `pal = img->palette`.
     - biSize 0xc: read `3*numColors` bytes into the buffer (0x7ce191; mismatch -> close, NULL, the image LEAKS);
       `bytesRead += 3n`; `pal[i] = {b[3i+2], b[3i+1], b[3i], 0xff}` (RGBA from BGR, 0x7ce1ca..0x7ce1de).
     - biSize 0x28: read `4*numColors` bytes (0x7ce239; mismatch -> close, NULL, leak); `pal[i] = {b[4i+2], b[4i+1], b[4i], 0xff}`
       (0x7ce27a..0x7ce28e).
     - other biSize: RwErrorSet(0x80000009), close, NULL (0x7ce29e; image leaks).
  8. `RwStreamSkip(s, offBits - bytesRead)` (0x7ce1f8); 0 -> RwImageDestroy, close, NULL.
  9. `stride = ((depth*width + 7)/8 + 3) & ~3` (signed division, 0x7ce2be..0x7ce304); `buf = G->rwmalloc(((width + 7) & ~7) *
     ((depth + 7)/8), 0x10000)` (0x7ce307); NULL -> RwErrorSet(0x80000013, size), RwImageDestroy, close, NULL (0x7ce316..0x7ce547).
  10. Pixel data:
      - `compression == 0` or `bitCount == 24` or `bitCount == 32` (0x7ce342..0x7ce361): for `r = 0 .. height-1`:
        `RwStreamRead(s, buf, stride) != stride` -> free buf, RwImageDestroy, close, NULL; `0x7ce5d0(img, height-1-r, buf,
        depth, 0)` (row converter, 0x7ce587).
      - `compression == 1` (BI_RLE8, 0x7ce367): for `row = 0; row < height;`: `x = 0`; loop reading 2-byte pairs (count,
        value) (0x7ce392; short read -> free, destroy, close, NULL):
        count != 0 -> `memset(buf + x, value, count)`, `x += count` (no bound check, 0x7ce3ce/0x7ce3e0);
        count == 0 and value > 2 -> read `value` literal bytes to `buf + x` (0x7ce3f5), `x += value`, and if value is odd
        read one pad byte (0x7ce423); value == 0 (end of line) -> `0x7ce5d0(img, height - row - 1, buf, depth, 1)`, `row++`;
        value == 1 (end of bitmap) -> `row = height - 1`, emit the row the same way (destination row 0), row++ ends the loop;
        value == 2 (delta) -> free, destroy, close, NULL without an error code (0x7ce4e5).
      - other compression: RwErrorSet(0x80000009), free buf, destroy, close, NULL (0x7ce504..0x7ce547).
  11. `G->rwfree(buf)`, `RwStreamClose(s, NULL)`, return img (0x7ce49b..0x7ce4b4).
- librw: bmp.cpp:readBMP differs: librw reads from memory, supports fewer header variants, no RLE8, different palette order
  handling and no leaks on failure paths.

## 0x7CED40 RtDictSchemaInit  (catalogue: specified/REA, port level 2)
- `RtDictSchema* (RtDictSchema* sc)`: `sc->dictionaries = _rwSListCreate(4, sc->dictChunkType | 0x40000)` (0x7ced48..0x7ced58);
  NULL -> return NULL (EAX from the call, 0x7ced60); `sc->current = NULL` (0x7ced61); return sc. Caller: UVAnim engine Open
  0x7cb9b0 with 0x8ded50.
- librw: absent (librw's UVAnimDictionary has no schema object).

## Open questions
- UNCLEAR: exact behaviour of the row converter 0x7ce5d0 `(RwImage*, i32 y, const u8* src, i32 depth, i32 flag)` (outside this
  work list); only its argument values are specified here (0x7ce486, 0x7ce587).
