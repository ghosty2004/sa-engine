# RwTexture / RwTexDictionary: module close, callbacks, streaming (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwTexture.md` (read-only) has the full TEXTUREMODULE layout; summary of what is used here:

## Layout
- Module data `m = RwEngineInstance + [0xc97b4c]`: +0x00 RwLLLink head of all dictionaries (link at dict+0x10; 0x7f377f,
  0x7f5469), +0x08 texture freelist, +0x0c dictionary freelist (0x7f3d4f, 0x7f3d58), +0x10 current dictionary (0x7f3d80,
  0x7f53e3), +0x14 read callback (0x7f352b), +0x1c mipmapping, +0x20 auto-mipmapping (0x7f35a0, 0x7f35b0), +0x24 heap pointer
  freed at close (0x7f3d0e), +0x28 u16 (0x7f3d3b), +0x2c mipmap generation callback (0x7f3c80, 0x7f3ce5), +0x30 mipmap name
  callback (0x7f3cb0, 0x7f3aab).
- `[0xc97b78]` default dictionary created at open (0x7f3d6b, cleared 0x7f3de8); `[0xc97b50]` module open count (0x7f3e49).
- RwTexDictionary: +0x08 texture list head (`texturesInDict`, link in texture at +0x08, 0x7f3d88, 0x7f53ef), +0x10/+0x14
  `lInInstance` next/prev (0x7f3db7-0x7f3dc6). Registry `[0x8e23e4]`.
- RwTexture: +0x00 raster (0x804655), +0x10 name[32], +0x30 mask[32] (0x8045e8, 0x8045f1), +0x50 filterAddressing (0x80466a).
  Registry `[0x8e23cc]`.
- RwGlobals string functions: +0x100 `rwstrcat(dst, src)` (0x7f40be); memory functions +0x134 malloc, +0x138 free
  (0x7f349f, 0x7f34bb, 0x7f3d17); +0x144/+0x148 freelist alloc/free.
- Device standard functions: +0xac NATIVETEXTUREGETSIZE (slot 25, 0x8049a3), +0xb4 NATIVETEXTUREWRITE (slot 27, 0x804beb),
  +0x68 TEXTURESETRASTER (slot 8, 0x7f35e6).
- Chunk IDs: 1 STRUCT, 6 TEXTURE (0x804621), 0x15 TEXTURENATIVE (0x804bc2); version 0x36003, build 0xffff.
- `.rdata 0x8846d8` = "0123456789abcdef" (16 bytes, hex digit table used by 0x7f4097).

## 0x7f3490 sub_7f3490 (proposed: _rwFreeListAllocNoFreeLists)  (catalogue: unknown/REA, port level 0)
- Evidence: `RwEngineInit` 0x7f3188 stores it into RwGlobals+0x144 (`[0xc97b0c]`, memoryAlloc) when the init flags have bit 0
  (rwENGINEINITNOFREELISTS); otherwise 0x801c30 `_rwFreeListAllocReal` is used (0x7f319f).
- `void* (RwFreeList* fl, RwUInt32 hint)`: replaces the first argument by `*(u32*)fl` (freelist +0 = entry size, 0x7f3499-0x7f349b)
  and tail-jumps to `memoryFuncs.rwmalloc` (+0x134): `return malloc(fl->entrySize, hint)`.
- librw: absent.

## 0x7f34b0 sub_7f34b0 (proposed: _rwFreeListFreeNoFreeLists)  (catalogue: unknown/REA, port level 0)
- Evidence: stored into RwGlobals+0x148 (`[0xc97b10]`, memoryFree) by `RwEngineInit` 0x7f3192 in the same no-freelist branch.
- `RwFreeList* (RwFreeList* fl, void* p)`: `memoryFuncs.rwfree (+0x138)(p)` (0x7f34bb); returns fl (0x7f34c1).
- librw: absent.

## 0x7f3520 RwTextureGetReadCallBack  (catalogue: specified/REA, port level 0)
- `RwTextureCallBackRead (void)`: returns m+0x14 (0x7f352b).
- librw: absent (librw texture.cpp has the find/read callbacks as globals with no getter of this name).

## 0x7f3590 _rwTextureSetAutoMipMapState  (catalogue: specified/REA, port level 0)
- `RwBool (RwBool enable)`: m+0x1c = enable AND m+0x20 = enable (both, 0x7f35a0, 0x7f35b0); returns 1.
- librw: absent.

## 0x7f35c0 _rwTextureGetAutoMipMapState  (catalogue: specified/REA, port level 0)
- `RwBool (void)`: returns m+0x20 (0x7f35cb).
- librw: absent.

## 0x7f35d0 RwTextureSetRaster  (catalogue: specified/REA, port level 0)
- `RwTexture* (RwTexture* tex, RwRaster* r)`: if r == NULL: `tex+0x0 = NULL` (0x7f35f8), return value EAX = tex (loaded at
  0x7f35f4). Else `stdFunc[8](tex, r, 0)` (+0x68, 0x7f35e6); returns tex if non-zero else NULL. The device function stores the
  raster (D3D9 0x4cbd40), so this function itself never writes tex+0 in the non-NULL case.
- librw: absent (librw assigns `tex->raster` directly).

## 0x7f3770 RwTexDictionaryForAllTexDictionaries  (catalogue: specified/REA, port level 0)
- `RwBool (RwTexDictionaryCallBack cb, void* data)`: walk the module list from m+0x00 (`next` +0) until back at the head; the
  next link is read before the call (0x7f3791); `cb(link - 0x10, data)` (dict pointer, 0x7f3793); a NULL return stops. Returns 1.
- librw: absent.

## 0x7f3aa0 RwTextureGenerateMipmapName  (catalogue: specified/REA, port level 0)
- `RwBool (RwChar* name, RwChar* maskName, RwUInt8 mipLevel, RwInt32 format)`: if m+0x30 is NULL -> 0; else tail-jump to it
  with the same four arguments (the `mov [esp+0xc], ecx` at 0x7f3ab7 rewrites the third argument with itself).
- librw: absent.

## 0x7f3c70 RwTextureSetMipmapGenerationCallBack  (catalogue: specified/REA, port level 0)
- `RwBool (RwTextureCallBackMipmapGeneration cb)`: m+0x2c = cb; returns 1. NULL allowed (no check).
- librw: absent.

## 0x7f3c90 RwTextureGetMipmapGenerationCallBack  (catalogue: specified/REA, port level 0)
- returns m+0x2c.
- librw: absent.

## 0x7f3ca0 RwTextureSetMipmapNameCallBack  (catalogue: specified/REA, port level 0)
- `RwBool (RwTextureCallBackMipmapName cb)`: m+0x30 = cb; returns 1.
- librw: absent.

## 0x7f3cc0 RwTextureGetMipmapNameCallBack  (catalogue: specified/REA, port level 0)
- returns m+0x30.
- librw: absent.

## 0x7f3cd0 RwTextureRasterGenerateMipmaps  (catalogue: specified/REA, port level 0)
- `RwBool (RwRaster* r, RwImage* img)`: `(m+0x2c)(r, img)` (no NULL check, 0x7f3ce5); returns `result != 0` (neg/sbb/neg).
- librw: absent.

## 0x7f3d00 cb_TEXTUREMODULE_RwEngine_Close  (catalogue: specified/REA, port level 3)
- `void* (void* instance, RwInt32 offset, RwInt32 size)`; returns instance (0x7f3e4f).
- 0x7f3d0e: if m+0x24 != NULL: `memoryFuncs.free (+0x138)(m+0x24)`; m+0x24 = 0; m+0x28 (u16) = 0. (If m+0x24 is NULL, +0x28 is
  not cleared.)
- 0x7f3d4f-0x7f3d7e: only if the texture freelist m+0x08 AND the dictionary freelist m+0x0c are non-NULL and the dictionary list
  is non-empty: search the list for `[0xc97b78]` (compare `link - 0x10`). Not found -> skip the destroy.
- Found (0x7f3d80-0x7f3de8) = inlined `RwTexDictionaryDestroy` of the default dictionary d:
  if m+0x10 == d -> m+0x10 = NULL. For each texture link in d+0x08 (next read before the call): `RwTextureDestroy(link-8)`
  0x7f3820 (pushed with an extra 0 argument); a 0 return stops the loop (0x7f3da4). `_rwPluginRegistryDeInitObject(&[0x8e23e4],
  d)`; unlink d+0x10 (`prev->next = next; next->prev = prev`, 0x7f3dbe/0x7f3dc6); `memoryFree(+0x148)(m+0x0c, d)`;
  `[0xc97b78] = 0`.
- Then if m+0x08 != NULL: `RwFreeListDestroy` 0x801b80, m+0x08 = 0 (0x7f3e13). If m+0x0c != NULL: destroy, m+0x0c = 0 (0x7f3e3f).
  `[0xc97b50]--`.
- Other dictionaries still alive are not destroyed (their memory goes with the freelist).
- librw: absent (librw engine close differs).

## 0x7f4080 sub_7f4080 (proposed: _rwTextureDefaultMipmapNameCallBack)  (catalogue: unknown/REA, port level 0)
- Evidence: installed into m+0x30 by `cb_TEXTUREMODULE_RwEngine_Open` (pass-1, 0x7f404a); signature of RwTextureCallBackMipmapName.
- `RwBool (RwChar* name, RwChar* maskName, RwUInt8 mipLevel, RwInt32 format)`. Local suffix `{'m', c, 0}` (0x7f4085, 0x7f40a3,
  0x7f40a7) with `c = "0123456789abcdef"[mipLevel]` (table 0x8846d8) if `1 <= mipLevel <= 15` (byte compares `jbe`/`jae 0x10`,
  0x7f408c-0x7f4090), else c = 0.
- If c != 0: `rwstrcat(name, suffix)` (+0x100, 0x7f40be); then if maskName != NULL and `maskName[0] != 0`: `rwstrcat(maskName,
  suffix)` (0x7f40e0). Returns 1 always. Level 0 or >= 16: names untouched. No length check (names are 32-byte buffers).
- librw: absent.

## 0x7f53d0 _rwTextureDefaultFindCallBack  (catalogue: specified/REA, port level 0)
- `RwTexture* (const RwChar* name)`.
- Case 1, current dictionary d = m+0x10 != NULL (0x7f53e7): for each texture in d+0x08 (list order): compare `tex+0x10` with
  name (inline compare below); first match -> return tex. Not found -> NULL; the other dictionaries are NOT searched.
- Case 2, no current dictionary (0x7f5469): for each dictionary in the module list (head m+0x00), for each of its textures, same
  compare; first match returns. None -> NULL.
- Inline compare (0x7f5413-0x7f5449, repeated at 0x7f5491): while both current chars are non-zero: uppercase each if it is in
  'a'..'z' (signed byte compares with 0x61/0x7a, `add 0xe0`); mismatch -> no match. After the loop, equal raw bytes (both 0) ->
  match. Bytes >= 0x80 are compared as-is.
- librw: texture.cpp `defaultFindCB` differs: librw searches only the current dictionary (`TexDictionary::getCurrent()->find`),
  with `strncmp_ci` limited to 32 chars; the exe falls back to all dictionaries when there is no current one.

## 0x8045a0 RwTextureStreamGetSize  (catalogue: specified/REA, port level 1)
- `RwUInt32 (const RwTexture* tex)`: `a = _rwStringStreamGetSize(tex+0x10) + 0x1c` (0x8045b5); `a = a + _rwStringStreamGetSize(
  tex+0x30) + 0xc` (0x8045c3); returns `a + _rwPluginRegistryGetSize(&[0x8e23cc], tex) + 0xc` (0x8045cf). (0x1c = struct header
  0xc + 4-byte struct + string header 0xc.)
- librw: texture.cpp `Texture::streamGetSize` matches.

## 0x8045e0 RwTextureStreamWrite  (catalogue: specified/REA, port level 4)
- `const RwTexture* (const RwTexture* tex, RwStream* s)`. Size as in 0x8045a0 (recomputed inline, 0x8045eb-0x80460e).
- `_rwStreamWriteVersionedChunkHeader(s, 6, size, 0x36003, 0xffff)` 0x804624; 0 -> NULL. `(s, 1, 4, 0x36003, 0xffff)` 0x804644;
  0 -> NULL.
- Flags dword (0x804655-0x804685): `mipbit = (tex+0 raster != NULL && !(raster+0x23 & 0x10))` (raster cFormat lacks
  rwRASTERFORMATAUTOMIPMAP>>8); `flags = (mipbit << 16) | (tex+0x50 & 0xffff)`. `RwStreamWrite(s, &flags, 4)` (raw, no swap); 0 ->
  NULL.
- `_rwStringStreamWrite(name tex+0x10, s)` 0x80a4a0, then mask tex+0x30; either 0 -> NULL. `_rwPluginRegistryWriteDataChunks(
  &[0x8e23cc], s, tex)`; returns tex if non-zero else NULL.
- librw: texture.cpp `Texture::streamWrite` differs: librw writes filterAddressing only (no bit 16 auto-mipmap flag).

## 0x804930 RwTexDictionaryStreamGetSize  (catalogue: specified/REA, port level 1)
- `RwUInt32 (const RwTexDictionary* d)`: local `{size = 0x10, ok = 1}` (0x804943, 0x80494b); `RwTexDictionaryForAllTextures(d,
  _rwTexDictionaryTextureGetSizeCB 0x804990, &local)`; `ok == 0` -> 0 (0x804961). Else `size + _rwPluginRegistryGetSize(&[0x8e23e4],
  d) + 0xc` (0x80497c). (0x10 = struct header + 4-byte struct.)
- librw: texture.cpp `TexDictionary::streamGetSize` matches (librw has no ok flag).

## 0x804990 _rwTexDictionaryTextureGetSizeCB  (catalogue: specified/REA, port level 1)
- `RwTexture* (RwTexture* tex, struct {RwUInt32 size; RwBool ok;}* d)`: `stdFunc[25](&n, tex, 0)` (+0xac; n = first-argument
  slot). 0 -> `d->ok = 0` (0x8049b5), return NULL. Else `d->size += n + 0xc`; `d->size += _rwPluginRegistryGetSize(&[0x8e23cc],
  tex) + 0xc` (0x8049cb-0x8049dc); return tex.
- librw: absent (inlined in librw streamGetSize).

## 0x8049f0 RwTexDictionaryStreamWrite  (catalogue: specified/REA, port level 4)
- `const RwTexDictionary* (const RwTexDictionary* d, RwStream* s)`.
- Size: local `{0x10, ok = 1}`, `RwTexDictionaryForAllTextures(d, 0x804990, &local)`; `ok == 0` -> NULL (0x804a2b);
  `size = local.size + _rwPluginRegistryGetSize(&[0x8e23e4], d) + 0xc` (0x804a3f); size 0 -> NULL (dead check).
- `_rwStreamWriteVersionedChunkHeader(s, 0x16 rwID_TEXDICTIONARY, size, 0x36003, 0xffff)` 0x804a62; `(s, 1, 4, 0x36003, 0xffff)`
  0x804a84; either 0 -> NULL.
- Count textures with `_rwTexDictionaryCountTexturesCB` 0x804b60 (from 0); `count >= 0x10000` -> NULL (unsigned, 0x804abd), no error
  set and the chunk headers already written stay.
- Struct `{u16 numTextures = count, u16 deviceId}` (0x804ad9): deviceId from `_rwDeviceSystemRequest(RwEngineInstance+0x10, 0x16
  GETID, &struct+2, 0, 0)` 0x7f2ab0, then the high byte is cleared (`and byte [..+3], 0`, 0x804aea) -> deviceId = low byte (2 for
  D3D9). `RwStreamWrite(s, &struct, 4)`; 0 -> NULL.
- `RwTexDictionaryForAllTextures(d, _rwTexDictionaryTextureWriteCB 0x804b70, &{s, ok = 1})` (0x804b21), then ALWAYS
  `_rwPluginRegistryWriteDataChunks(&[0x8e23e4], s, d)` (0x804b2d). Returns d only if that returned non-zero AND ok != 0
  (0x804b35-0x804b3f), else NULL.
- librw: texture.cpp `TexDictionary::streamWrite` differs: no 0x10000 limit, deviceId from the platform, aborts on the first
  failed texture without writing the extension.

## 0x804b60 _rwTexDictionaryCountTexturesCB  (catalogue: specified/REA, port level 0)
- `RwTexture* (RwTexture* tex, RwInt32* count)`: `++*count`; return tex.
- librw: absent.

## 0x804b70 _rwTexDictionaryTextureWriteCB  (catalogue: specified/REA, port level 4)
- `RwTexture* (RwTexture* tex, struct {RwStream* s; RwBool ok;}* d)`. Every failure sets `d->ok = 0` and returns NULL.
- `stdFunc[25](&n, tex, 0)` (+0xac, 0x804b86); 0 -> fail. `total = n + _rwPluginRegistryGetSize(&[0x8e23cc], tex) + 0xc`
  (0x804bbd; stored back into the argument slot). `_rwStreamWriteVersionedChunkHeader(d->s, 0x15, total, 0x36003, 0xffff)`; 0 -> fail.
  `stdFunc[27](d->s, tex, total)` (+0xb4, 0x804beb; n argument = total chunk size); 0 -> fail.
  `_rwPluginRegistryWriteDataChunks(&[0x8e23cc], d->s, tex)`; 0 -> fail. Returns tex.
- librw: absent (librw writes native textures inside `TexDictionary::streamWrite`).

## 0x804c30 RwTexDictionaryStreamRead  (catalogue: specified/REA, port level 5)
- `RwTexDictionary* (RwStream* s)`.
- `RwStreamFindChunk(s, 1, &size, &version)`; 0 -> NULL. `version < 0x34000 || > 0x36003` -> error `{1, 0x80000004}`, NULL
  (0x804e07).
- `RwStreamRead(s, &hdr, size)` into the 4-byte stack slot of the first argument (0x804c7a; hdr zeroed first 0x804c84): a struct
  chunk larger than 4 bytes overwrites the caller's stack (original bug; real files have 4). Read != size -> NULL.
- `dev = 0; _rwDeviceSystemRequest(RwEngineInstance+0x10, 0x16, &dev, 0, 0)`; `dev &= 0xff` (0x804cc0). If `dev != 0` and
  `hdr.deviceId (u16 at +2) != 0` and they differ (16-bit compare 0x804cd8) -> return NULL (no error set).
- `d = RwTexDictionaryCreate()` 0x7f3600; NULL -> NULL.
- Loop while `hdr.numTextures (u16) != 0` (checked before the first pass 0x804cf0 and at the end 0x804d7f), decrement first
  (0x804cfc):
  `RwStreamFindChunk(s, 0x15, &size, &version)` 0 -> fail A; version out of [0x34000, 0x36003] -> fail B (0x804dc1);
  `stdFunc[26](s, &tex, size)` (+0xb0 NATIVETEXTUREREAD, 0x804d49) 0 or tex == NULL -> fail A;
  `_rwPluginRegistryReadDataChunks(&[0x8e23cc], s, tex)` 0 -> fail A (tex is not in d yet: leaked);
  `RwTexDictionaryAddTexture(d, tex)` 0x7f3980.
- `_rwPluginRegistryReadDataChunks(&[0x8e23e4], s, d)` (0x804d97); 0 -> fail A; else return d.
- Fail A (0x804da3): `RwTexDictionaryForAllTextures(d, _rwTexDictionaryDestroyTextureCB 0x804e40, NULL)`,
  `RwTexDictionaryDestroy(d)` 0x7f36a0, return NULL. Fail B: the same, then `RwErrorSet({1, _rwerror(0x80000004)})`, NULL.
- librw: texture.cpp `TexDictionary::streamRead` differs: no version checks, reads the 4-byte struct with fixed size, no device-ID
  check, textures read via `Texture::streamReadNative`; on failure it destroys the dictionary (which in librw destroys textures).

## 0x804e40 _rwTexDictionaryDestroyTextureCB  (catalogue: specified/REA, port level 3)
- `RwTexture* (RwTexture* tex, void*)`: `RwTextureDestroy(tex)` 0x7f3820; returns tex (always non-NULL, so iteration continues).
- librw: absent.

## 0x804e60 _rwTextureChunkInfoRead  (catalogue: specified/REA, port level 4)
- `RwTextureChunkInfo* (RwStream* s, RwTextureChunkInfo* info /*filtering +0, addressingU +4, addressingV +8*/, RwInt32* bytesRead)`.
- `RwStreamFindChunk(s, 1, &size, NULL)`; 0 -> NULL. `flags = 0; RwStreamRead(s, &flags, 4)`; != 4 -> NULL (0x804e97).
  `*bytesRead = size + 0xc` (0x804eb1); `RwStreamSkip(s, size - 4)` (result ignored). `info[0] = flags & 0xff`, `info[1] =
  (flags >> 8) & 0xf`, `info[2] = (flags >> 12) & 0xf` (0x804ec5-0x804ede). Returns info. Bit 16 is ignored.
- librw: absent.
