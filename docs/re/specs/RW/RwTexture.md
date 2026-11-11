# RwTexture / RwTexDictionary (RW core, TEXTUREMODULE) - RE-SPEC-RW writer C

RenderWare 3.6 texture and texture-dictionary API as linked into gta_sa.exe 1.0 US. All functions are `__cdecl`
(caller pops). gta-reversed only has thunks/`plugin::Call` wrappers for them (RenderWare/rw/*.cpp); everything here
comes from the bytes. librw (aap/librw `src/texture.cpp`) was used as a reading aid; differences are listed per function.

## Layouts (offsets proven by the instruction addresses given)

`RwGlobals` (instance pointer `RwEngineInstance` [0xc97b24]): +0x48 `stdFunc[29]` (`stdFunc[i]` at +0x48+4*i);
+0xfc `rwstrncpy(dst, src, n)` (0x7f38ba), +0x120 `rwstrlen(s)` (0x7f38c7); +0x144 freelist alloc
`void* (*)(RwFreeList*, RwUInt32 hint)` (0x7f37d6), +0x148 freelist free `(RwFreeList*, void*)` (0x7f3889).
(String table: +0xf0 sprintf, +0xf4 vsprintf, +0xf8 strcpy, +0xfc strncpy, ... +0x120 strlen, consistent with the
RW 3.6 `RwStringFunctions` order starting at +0xf0 and memoryFuncs at +0x134.)

TEXTUREMODULE globals: base = `RwEngineInstance + [0xc97b4c]` (offset stored by the module Open 0x7f3ebb; derived
offset 0x420, plugins.md). Fields (0x34 bytes):

| off | type | meaning | evidence |
|---|---|---|---|
| +0x00 | RwLLLink | head of the list of all texture dictionaries (`next` +0, `prev` +4) | 0x7f3f62/0x7f3f73, 0x7f3681 |
| +0x08 | RwFreeList* | texture freelist | 0x7f37d1, 0x7f3ee0 |
| +0x0c | RwFreeList* | texture-dictionary freelist | 0x7f3611, 0x7f3f2d |
| +0x10 | RwTexDictionary* | current dictionary | 0x7f3a80, 0x7f3f97 |
| +0x14 | RwTextureCallBackRead | read callback | 0x7f3510, 0x7f4023 |
| +0x18 | RwTextureCallBackFind | find callback | 0x7f34e0, 0x7f4010 |
| +0x1c | RwBool | mipmapping | 0x7f3540, 0x7f3ff1 |
| +0x20 | RwBool | auto mipmapping | 0x7f3570, 0x7f4000 |
| +0x24 | u32 | zeroed at open (unused in this pass) | 0x7f405d |
| +0x28 | u16 | zeroed at open (unused in this pass) | 0x7f406c |
| +0x2c | fn | mipmap generation callback, default `_rwTextureDefaultMipmapGenerate` 0x7f5140 | 0x7f4036 |
| +0x30 | fn | mipmap name callback, default `_rwTextureMipmapNameCB` 0x7f4080 | 0x7f404a |

Other module globals: 0xc97b50 module open count (0x7f3f7c); 0xc97b78 default dictionary created at open
(0x7f3f92); static freelist structs 0xc97b54 (textures, 0x7f3eb6) and 0xc97b28 (dictionaries, 0x7f3f0f).
Registries: RwTexture 0x8e23cc (base size 0x58), RwTexDictionary 0x8e23e4 (base size 0x18).

`RwTexDictionary` (0x18 + plugins): +0x00 RwObject (type byte 6 = rwTEXDICTIONARY, 0x7f3629; subType/flags/privateFlags
0, parent 0), +0x08 RwLinkList `texturesInDict` (link.next +8, link.prev +0xc; 0x7f3684-0x7f368a), +0x10 RwLLLink
`lInInstance` (next +0x10, prev +0x14; 0x7f3650/0x7f3660).

`RwTexture` (0x58 + plugins): +0x00 RwRaster* raster (0x7f37ee), +0x04 RwTexDictionary* dict (0x7f37ff, 0x7f3984),
+0x08 RwLLLink `lInDictionary` (next +8, prev +0xc; 0x7f398b-0x7f3999), +0x10 char name[32] (0x7f38b6), +0x30 char
mask[32] (0x7f3926), +0x50 u32 filterAddressing (byte +0x50 filter mode, nibble bits 8-11 addressU, bits 12-15
addressV; 0x7f37eb-0x7f380b), +0x54 i32 refCount (0x7f3808, 0x7f3825).

Error helper: `RwError e = {pluginID = 1, errorCode = _rwerror(code, ...)}; RwErrorSet(&e)` (`_rwerror` 0x8088d0 is
variadic and returns the code, RwErrorSet 0x808820).

## 0x7f34d0 RwTextureSetFindCallBack  (catalogue: specified/REA, port level 0)
- `RwBool (RwTextureCallBackFind cb)`: module+0x18 = cb (0x7f34e0); returns 1.

## 0x7f34f0 RwTextureGetFindCallBack  (catalogue: specified/REA, port level 0)
- `RwTextureCallBackFind ()`: returns module+0x18 (0x7f34fb).

## 0x7f3500 RwTextureSetReadCallBack  (catalogue: specified/REA, port level 0)
- `RwBool (RwTextureCallBackRead cb)`: module+0x14 = cb (0x7f3510); returns 1.

## 0x7f3530 RwTextureSetMipmapping  (catalogue: specified/REA, port level 0)
- `RwBool (RwBool enable)`: module+0x1c = enable (stored as the full dword, not normalised; 0x7f3540); returns 1.

## 0x7f3550 RwTextureGetMipmapping  (catalogue: specified/REA, port level 0)
- `RwBool ()`: returns module+0x1c (0x7f355b).

## 0x7f3560 RwTextureSetAutoMipmapping  (catalogue: specified/REA, port level 0)
- `RwBool (RwBool enable)`: module+0x20 = enable (0x7f3570); returns 1.

## 0x7f3580 RwTextureGetAutoMipmapping  (catalogue: specified/REA, port level 0)
- `RwBool ()`: returns module+0x20 (0x7f358b).

## 0x7f3600 RwTexDictionaryCreate  (catalogue: specified/REA, port level 1)
- `RwTexDictionary* ()`. Callers 7 (CTxdStore, RwGtaStream readers, module Open 0x7f3f81).
- Behaviour:
  1. `d = memoryAlloc(module+0x0c, 0x30016)` (RwGlobals+0x144, 0x7f3616); NULL -> return NULL (no error set).
  2. RwObject: type = 6, subType = flags = privateFlags = 0, parent = NULL (0x7f3629-0x7f3635).
  3. Insert at the HEAD of the module dictionary list: `d->lInInstance.next = module.next` (0x7f3650);
     `d->lInInstance.prev = &module+0` (0x7f3660); `module.next->prev = &d->lInInstance` (0x7f3672);
     `module.next = &d->lInInstance` (0x7f3681).
  4. Empty texture list: `d+0xc = d+8; d+8 = d+8` (0x7f3687/0x7f368a).
  5. `_rwPluginRegistryInitObject(0x8e23e4, d)` 0x8086e0 (return value ignored); return d.
- librw: `TexDictionary::create` matches in substance (librw appends to a global list via `link`, order may differ).

## 0x7f36a0 RwTexDictionaryDestroy  (catalogue: specified/REA, port level 3)
- `RwBool (RwTexDictionary* d)`; returns 1 always.
- Behaviour:
  1. If module current dictionary (+0x10) == d: current = NULL (0x7f36ba).
  2. Walk `d->texturesInDict` from the head (link `l` -> texture `l - 8`): read `next = l->next` FIRST, then
     `RwTextureDestroy(tex)` 0x7f3820; if it returns 0 the walk stops (0x7f36de; it always returns 1, so in practice
     every texture is visited); continue while `next != &d->texturesInDict`. RwTextureDestroy only decrements the
     reference count of textures still referenced elsewhere, so such textures stay alive but REMAIN LINKED in the
     list of the freed dictionary (their `dict` field still points at it) - original behaviour.
  3. `_rwPluginRegistryDeInitObject(0x8e23e4, d)` 0x808740.
  4. Unlink `lInInstance` (`prev->next = next; next->prev = prev`, 0x7f36f8-0x7f3700).
  5. `memoryFree(module+0x0c, d)` (RwGlobals+0x148, 0x7f3713).
- librw: same order of steps (librw destroys textures via `forAllTextures`).

## 0x7f3730 RwTexDictionaryForAllTextures  (catalogue: specified/REA, port level 0)
- `RwTexDictionary* (RwTexDictionary* d, RwTexture* (*cb)(RwTexture*, void*), void* data)`.
- Walk from head; `next` is read before the callback (so the callback may unlink/destroy the texture);
  `cb(l - 8, data)`; stop when it returns NULL (0x7f3756). Returns d (both exits, 0x7f3760/0x7f376c).

## 0x7f37c0 RwTextureCreate  (catalogue: specified/REA, port level 1)
- `RwTexture* (RwRaster* raster)`; callers 6.
- Behaviour: `t = memoryAlloc(module+0x08, 0x30006)` (0x7f37d6); NULL -> return NULL. Then (order 0x7f37eb-0x7f380b):
  filterAddressing = 0; raster = arg; byte +0x51 = 0x11; dict = NULL; name[0] = 0; mask[0] = 0; refCount = 1;
  byte +0x50 = 1. Net filterAddressing = 0x1101 (rwFILTERNEAREST, addressU = addressV = rwTEXTUREADDRESSWRAP).
  `_rwPluginRegistryInitObject(0x8e23cc, t)` (result ignored); return t. The lInDictionary link is not initialised.
- librw: `Texture::create` sets the same 0x1101 default; librw also links into a global texture list (absent in the exe).

## 0x7f3820 RwTextureDestroy  (catalogue: specified/REA, port level 2)
- `RwBool (RwTexture* t)`; returns 1 always (0x7f3892).
- Behaviour:
  1. `t->refCount -= 1` (stored, 0x7f382b); if the new value > 0 (signed, `jg` 0x7f3830): return 1.
  2. Otherwise refCount is written back as `new + 1` (= the value on entry, normally 1; 0x7f3839) so plugin
     destructors see the old count; `_rwPluginRegistryDeInitObject(0x8e23cc, t)`.
  3. If `t->dict != NULL`: unlink lInDictionary (0x7f384b-0x7f3859); `dict` itself is not cleared.
  4. If `t->raster != NULL`: `RwRasterDestroy(raster)` 0x7fb020, then raster = NULL.
  5. `t->refCount -= 1` again (0x7f3876), `memoryFree(module+0x08, t)`.
- librw: differs - librw does not restore refCount around the plugin destructors and removes from a global list.

## 0x7f38a0 RwTextureSetName  (catalogue: specified/REA, port level 1)
- `RwTexture* (RwTexture* t, const char* name)`; returns t.
- Behaviour: `rwstrncpy(t->name, name, 0x20)` (RwGlobals+0xfc, 0x7f38ba); `len = rwstrlen(name)` (+0x120); if
  `len >= 0x20` (unsigned, 0x7f38d3): error `_rwerror(0x8000001e, name, 0x20, 0x1f, (int)(signed char)name[0x1f])`
  (0x7f38ec) + RwErrorSet, and `t->name[0x1f] = 0` (0x7f3902). (strncpy does not terminate a 32-char name; the
  explicit store does.)
- librw: `setName` uses strncpy without the error report.

## 0x7f3910 RwTextureSetMaskName  (catalogue: specified/REA, port level 1)
- Same as 0x7f38a0 on `t->mask` (+0x30; copy 0x7f392a, error 0x7f395c, terminator `t->mask[0x1f]` = byte +0x4f, 0x7f3972).

## 0x7f3980 RwTexDictionaryAddTexture  (catalogue: specified/REA, port level 0)
- `RwTexture* (RwTexDictionary* d, RwTexture* t)`; returns t (EAX = t throughout).
- If `t->dict != NULL`: unlink t->lInDictionary (0x7f398b). Then `t->dict = d` (0x7f39a1) and insert at the HEAD of
  `d->texturesInDict`: `t.next = d.first; t.prev = &d->list; d.first->prev = &t.link; d.first = &t.link`
  (0x7f39a7-0x7f39b6). No duplicate-name check.

## 0x7f39c0 RwTexDictionaryRemoveTexture  (catalogue: specified/REA, port level 0)
- `RwTexture* (RwTexture* t)`: if `t->dict != NULL`: `t->dict = NULL` (0x7f39d1), unlink. Returns t.

## 0x7f39f0 RwTexDictionaryFindNamedTexture  (catalogue: specified/REA, port level 0)
- `RwTexture* (RwTexDictionary* d, const char* name)`.
- Walk from the head (most recently added first). For each texture compare `t->name` with `name` case-insensitively
  (0x7f3a17-0x7f3a50): bytes are compared as SIGNED chars; only 'a'..'z' (0x61..0x7a) are folded by adding 0xe0
  (= -0x20); loop while both current chars are non-zero and equal after folding; a fold-mismatch skips to the next
  texture; the match is accepted when the final raw bytes are equal (i.e. both 0). Return the first match, else NULL.
  (The `t+0x10 == NULL` test at 0x7f3a0f is dead.)
- librw: `TexDictionary::find` uses `strncmp_ci(name, 32)`; the exe compares up to the terminator, no length limit.

## 0x7f3a70 RwTexDictionarySetCurrent  (catalogue: specified/REA, port level 0)
- `RwTexDictionary* (RwTexDictionary* d)`: module+0x10 = d; returns d (EAX = arg, 0x7f3a7c).

## 0x7f3a90 RwTexDictionaryGetCurrent  (catalogue: specified/REA, port level 0)
- `RwTexDictionary* ()`: returns module+0x10.

## 0x7f3ac0 RwTextureRead  (catalogue: specified/REA, port level 1)
- `RwTexture* (const char* name, const char* maskName)`; callers: RwTextureStreamRead 0x804841 and game code.
  (plugin-sdk mislabels 0x4c7510 as RwTextureRead; README §6.)
- Behaviour:
  1. `t = findCB(name)` (module+0x18, one argument, 0x7f3ad5). If non-NULL: `t->refCount++` (0x7f3ae6), return t
     (the texture is NOT moved into the current dictionary).
  2. `t = readCB(name, maskName)` (module+0x14, 0x7f3afe). If NULL: error code 0x16 with
     `_rwerror(0x16, name, maskName)` if maskName != NULL (0x7f3b19), else `_rwerror(0x16, name, "(null)")` with the
     string at [0x8e240c] (0x7f3b3f); RwErrorSet; return NULL.
  3. If the current dictionary (module+0x10) is non-NULL: same as RwTexDictionaryAddTexture(current, t) inlined
     (unlink if t->dict, set dict, insert at head; 0x7f3b71-0x7f3b9f). Return t.
- librw: `Texture::read` matches the find/refCount/read/add order; librw has no error report.

## 0x7f3bb0 RwTextureRegisterPlugin  (catalogue: specified/REA, port level 2)
- `RwInt32 (RwInt32 size, RwUInt32 pluginID, ctor, dtor, copy)`: returns
  `_rwPluginRegistryAddPlugin(0x8e23cc, size, pluginID, ctor, dtor, copy)` 0x8084a0.

## 0x7f3c10 RwTexDictionaryRegisterPlugin  (catalogue: specified/REA, port level 2)
- Same as 0x7f3bb0 with registry 0x8e23e4.

## 0x7f3ea0 cb_TEXTUREMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Engine-module Open callback `void* (void* instance, RwInt32 offset, RwInt32 size)`; returns instance on success,
  NULL on failure.
- Behaviour (0x7f3ea0-0x7f4076):
  1. `[0xc97b4c] = offset`.
  2. Texture freelist: `RwFreeListCreateAndPreallocateSpace(entrySize = [0x8e23cc] (registry sizeOfStruct),
     entriesPerBlock = [0x8e23fc] (0x80), alignment 4, numBlocksToPreallocate = [0x8e2400] (1), inFreeList = 0xc97b54,
     hint 0x40006)` 0x801b70 -> module+0x08; NULL -> return NULL.
  3. Dictionary freelist: same with `[0x8e23e4]`, `[0x8e2404]` (5), 4, `[0x8e2408]` (1), 0xc97b28, hint 0x40408
     -> module+0x0c; NULL -> `RwFreeListDestroy(module+0x08)`, module+0x08 = NULL, return NULL.
  4. module list head: next = prev = &module (0x7f3f62/0x7f3f73). `[0xc97b50]++`.
  5. `d = RwTexDictionaryCreate()`; `[0xc97b78] = d`; module+0x10 = d. NULL -> destroy both freelists (dictionary
     list first, then texture list), clear both pointers, return NULL.
  6. module+0x1c (mipmapping) = 0; +0x20 (auto) = 0; +0x18 find = 0x7f53d0; +0x14 read = 0x7f40f0;
     +0x2c = 0x7f5140; +0x30 = 0x7f4080; +0x24 = 0 (dword); +0x28 = 0 (word). Return instance.
- Note: the freelist sizes come from `.data` (0x8e23fc.. are part of the registry block after the two registries).

## 0x804550 RwTextureRegisterPluginStream  (catalogue: specified/REA, port level 1)
- `RwInt32 (RwUInt32 pluginID, readCB, writeCB, getSizeCB)`: returns
  `_rwPluginRegistryAddPluginStream(0x8e23cc, pluginID, read, write, getSize)` 0x8088e0.

## 0x8046e0 RwTextureStreamRead  (catalogue: specified/REA, port level 5)
- `RwTexture* (RwStream* stream)`; frame 0x110 bytes of locals: `flags` (4 bytes), `length`, `version`,
  `mask[128]`, `name[128]`.
- Behaviour:
  1. `RwStreamFindChunk(stream, 1 /*rwID_STRUCT*/, &length, &version)` 0x7ed2d0; 0 -> return NULL.
  2. Version must satisfy `0x34000 <= version <= 0x36003` (unsigned, 0x804712/0x80471d); else
     `_rwerror(0x80000004)` + RwErrorSet, return NULL.
  3. `flags = 0`; `RwStreamRead(stream, &flags, length)` 0x7ec9d0; result != length -> return NULL (no error).
     BUG (original): `length` is not checked against 4; a struct chunk longer than 4 bytes overwrites the stack
     (in practice length is 4).
  4. If `(flags & 0xf000) == 0` (0x80475b): `flags |= (flags & 0xf00) << 4` (addressV := addressU).
  5. `filter = flags & 0xff`; save `oldMip = RwTextureGetMipmapping()`, `oldAuto = RwTextureGetAutoMipmapping()`.
     If filter is 3, 4, 5 or 6 (the mip filter modes): `RwTextureSetMipmapping(1)`;
     `RwTextureSetAutoMipmapping((flags & 0x10000) ? 0 : 1)` (bit 0 of byte 2, 0x8047b6). Otherwise
     `RwTextureSetMipmapping(0)`, `RwTextureSetAutoMipmapping(0)`.
  6. `_rwStringStreamFindAndRead(name, stream)` 0x80a510 then `(mask, stream)`; either 0 -> restore mip/auto
     (SetMipmapping(oldMip), SetAutoMipmapping(oldAuto)), return NULL.
  7. `t = RwTextureRead(name, mask)` (with the mip state of step 5 in force). NULL ->
     `_rwPluginRegistrySkipDataChunks(0x8e23cc, stream)` 0x808c10 (result ignored), restore, return NULL.
  8. If `t->refCount == 1` (freshly read, 0x804871): `t->filterAddressing = flags & 0xffff` (whole dword written).
  9. Restore mip/auto, then `_rwPluginRegistryReadDataChunks(0x8e23cc, stream, t)` 0x808980; return t if that
     returned non-zero, else NULL (the texture is leaked, not destroyed).
- librw: `Texture::streamRead` matches steps 4-9 (same 0x10000 meaning and refCount==1 rule); librw reads exactly
  4 bytes (no overflow) and has no version-range check.
- Open questions: none.
