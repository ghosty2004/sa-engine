# RwStream: write path, chunk header, typed writes (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwStream.md` (read-only) covers open/close/read/skip/find-chunk.

## Layout
- RwStream: +0x0 type (1 FILE, 2 FILENAME, 3 MEMORY, 4 CUSTOM; jump table 0x7ecce4 indexed by `type-1`, `cmp eax,3; ja`, 0x7ecb3d),
  +0x4 accessType, +0xc FILE* (file types) or memory position, +0x10 memory nSize, +0x14 memory block / custom sfnwrite, +0x1c custom
  data (decompile of 0x7ecb30, instruction addresses below).
- RwGlobals: +0xd4 `fileFuncs.rwfwrite(buf, 1, n, fp)` (0x7ecb63), +0x134 malloc (0x7ecbab), +0x13c realloc (0x7ecc26).
- Stream module data at engine + `[0xc9792c]`: +0x0 stream freelist (0x7ec7eb); `[0xc97930]` module open count (0x7ec801).
- Error codes (`RwErrorSet({1, _rwerror(code, ...)})`): 0x8000001c write failed (0x7ecb70), 0x80000013 out of memory with the byte
  count as argument (0x7ecbc0, 0x7ecc41), 0x8000001a read failed (0x7ed4bf), 0xe bad stream type (default case 0x7eccbb).

## 0x7ec7e0 cb_STREAMMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- `void* (void* instance, RwInt32, RwInt32)`: if module+0x0 freelist != NULL `RwFreeListDestroy` it (the pointer is NOT cleared,
  0x7ec7f3); `[0xc97930]--`. Returns instance.
- librw: absent.

## 0x7ecb30 RwStreamWrite  (catalogue: specified/REA, port level 1)
- `RwStream* (RwStream* s, const void* buf, RwUInt32 n)`.
- FILE / FILENAME: `w = rwfwrite(buf, 1, n, s+0xc)` (+0xd4, 0x7ecb63); `w == n` -> s; else error 0x8000001c, NULL.
- MEMORY: if block (+0x14) == NULL: `block = malloc(0x200, 0x30404)`; NULL -> error {0x80000013, 0x200}, NULL; `nSize (+0x10) = 0x200`.
  If `nSize - pos (+0xc) < n` (unsigned, 0x7ecc01): `newSize = nSize + (n >= 0x200 ? n : 0x200)` (0x7ecc05-0x7ecc11);
  `p = realloc(block, newSize, 0x1030404)` (+0x13c); NULL -> error {0x80000013, newSize - nSize}, NULL (old block kept); else block = p,
  nSize = newSize. `memcpy(block + pos, buf, n)` (dwords then bytes); `pos += n`. Returns s. (+0x8 is not updated.)
- CUSTOM: `r = sfnwrite (+0x14)(data (+0x1c), buf, n)` (0x7ecca8); returns s if r != 0 else NULL (no error set).
- Other types: error 0xe, NULL.
- librw: base.cpp `StreamFile::write8` / `StreamMemory::write8` differ: librw memory streams never grow (writes are clipped at the
  fixed capacity and the stream goes to EOF), no custom streams, no error codes; returns the byte count.

## 0x7ed270 _rwStreamWriteVersionedChunkHeader  (catalogue: specified/REA, port level 2)
- `RwStream* (RwStream* s, RwInt32 type, RwInt32 size, RwUInt32 version, RwUInt32 build)`: local `{type, size, libId}` with
  `libId = (((version + 0x10000) << 14) & 0xffc00000) | ((version & 0x3f) << 16) | (build & 0xffff)` (0x7ed28b-0x7ed2ae);
  returns `RwStreamWrite(s, &hdr, 0xc)` (raw, little-endian).
- librw: base.cpp `writeChunkHeader` matches (librw uses the same libraryID encoding via `libraryIDPack`).

## 0x7ed3d0 RwStreamWriteReal  (catalogue: specified/REA, port level 2)
- `RwStream* (RwStream* s, const RwReal* p, RwUInt32 bytes)`: while `bytes != 0`: `k = min(bytes, 0x100)` (unsigned, 0x7ed3e5); copy k
  bytes into a 0x100-byte local buffer; `RwStreamWrite(s, buf, k)`; NULL -> return NULL (0x7ed427); `p += k; bytes -= k`. Returns s.
  No byte swapping (little-endian host build); bytes == 0 returns s.
- librw: base.cpp `Stream::writeF32` matches in effect (librw writes directly).

## 0x7ed460 RwStreamWriteInt32  (catalogue: specified/REA, port level 2)
- `RwStream* (RwStream* s, const RwInt32* p, RwUInt32 bytes)`: `RwStreamWrite(s, p, bytes)` and returns s REGARDLESS of the result
  (0x7ed478). So callers testing for NULL only catch `s == NULL`.
- librw: base.cpp `Stream::writeI32(val)` differs: takes a value, returns the write result.

## 0x7ed480 RwStreamWriteInt16  (catalogue: specified/REA, port level 2)
- Same as 0x7ed460 (byte-identical shape): `RwStreamWrite(s, p, bytes)`; returns s regardless.
- librw: base.cpp `Stream::writeI16(val)` differs as above.

## 0x7ed4a0 RwStreamReadInt16  (catalogue: specified/REA, port level 2)
- `RwStream* (RwStream* s, RwInt16* p, RwUInt32 bytes)`: `r = RwStreamRead(s, p, bytes)` 0x7ec9d0; `r != 0` -> s (a short read
  counts as success, 0x7ed4bd); `r == 0` -> error 0x8000001a, NULL.
- librw: base.cpp `Stream::readI16` differs: returns the value, no error.
