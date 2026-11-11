# GTA-side RenderWare helpers (anim blend update, GTA clump stream reader, D3D resource buffers, JPEG plugin, visibility plugin) - pass-3 specs, writer A (RE-SPEC-RW-3)

Game code that sits on top of RW 3.6. gta-reversed status per function; where it has C++ the differences to the bytes
are listed. All `__cdecl` unless stated.

## Layouts used (offsets proven by the instruction addresses given)

- `CAnimBlendClumpData*` = `*(RpClump* + [0xb5f878])` (`ClumpOffset`, i32, 0x4d34f3..0x4d34fe): +0x0 association list
  head (`CLink` next; association = link - 4, 0x4d3534), +0x8 numFrameData (0x4d3621), +0x10 `AnimBlendFrameData*`
  frames (root first; byte +0 flags: 0x20 NeedsKeyFrameUpdate, 0x40 compressed, 0x80 IsUpdatingFrame; 0x4d3616..0x4d362d).
  Global `gpAnimBlendClump` `[0xb4ea0c]` (written 0x4d3519).
- `CAnimBlendAssociation` (link at +0x4): +0x14 hierarchy (0x4d3545), +0x18 blendAmount (0x4d357e), +0x24 speed
  (0x4d357b), +0x28 timeStep (written 0x4d3605 as `link+0x24`), +0x2e u16 flags (byte +0x2e: bit 0 playing, bit 5
  sync/movement; 0x4d356b, 0x4d35bc). `CAnimBlendHierarchy`: +0x4 sequences pointer (0x4d3548), +0x10 totalTime (0x4d3578).
- GTA clump stream reader: `rwFrameList` {+0 RwFrame** frames, +4 i32 num} (0x72e2ca..0x72e2d7); geometry list {+0
  RpGeometry** geoms, +4 i32 num} (0x72e0b7, 0x72e0cc); statics: geometry list at 0xc87b00, frame list at 0xc87af4
  (num at `[0xc87af8]`, cleared 0x72e718).
- Plugin offsets: `gPipelinePluginOffset` `[0x8d6080]` (atomic, 4 bytes), `gNodeNamePluginOffset` `[0xc87c5c]` (frame,
  name chars), `CVisibilityPlugins::ms_atomicPluginOffset` `[0x8d608c]` (atomic: +0 i16 modelId, +2 u16 flags).
- JPEG destination manager: `cinfo->dest` at `cinfo + 0x18` (0x5d03e4): +0 next_output_byte, +4 free_in_buffer;
  buffer `g_ScreenshotFileBuf` 0xbd0b78 of 0x32000 bytes; file handle `[0xbd0b74]`. JPEG decode globals:
  `g_JpegDecodingToRaster` `[0xbd0160]` (u32* write cursor), `g_JpegScan` 0xbd0170 (4 bytes per pixel: r, g, b, pad).
- Constants: `[0x858b50] = 0.0f`.
- RwGlobals `G` (`[0xc97b24]`): +0x120 `rwstrlen` string function (0x72fa97).

## 0x4d34f0 RpAnimBlend::RpAnimBlendClumpUpdateAnimations  (catalogue: specified/REA, port level 5)
- Convention: `void (RpClump* clump, f32 dt, bool onScreen)`. Locals: `A` (sum of synced length) [0x10], `B` (sum of
  synced blend) [0x14], update context `ctx` [0x18]: +0 u32 includePartial, +4 `CAnimBlendNode* nodes[12]` (frame after
  `push esi`).
- 1. `bd = *(clump + [0xb5f878])`; `A = B = 0`; `[0xb4ea0c] = bd` (0x4d3519, written before the empty test); if the
  list is empty return (0x4d351f). `n = 0`, `ctx.includePartial = 0`.
- 2. For each association `a` (next link read before the calls, 0x4d3537): if `a->UpdateBlend(dt)` (thiscall 0x4d1490)
  returns true and `a->hier->sequences != NULL` (+4, 0x4d3548): `CAnimManager::UncompressAnimation(a->hier)`; if `n < 0xb`
  (signed, 0x4d3558): `ctx.nodes[n++] = a->GetNode(0)` (0x4ceb60); if `a->flags & 0x2000` (byte +0x2f bit 5, read as
  `(b[+0x2e] >> 5) & 1`, 0x4d356b): `A = f32(A + hier->totalTime / a->speed * a->blendAmount)` (x87 `fld total; fdiv speed;
  fmul blend; fadd A`, 0x4d3578..0x4d3585), `B = f32(B + a->blendAmount)`; else `ctx.includePartial = 1` (0x4d3596).
  Associations whose UpdateBlend is false or whose sequences pointer is NULL contribute nothing.
- 3. For each association (0x4d35b0..0x4d360c), only if `flags & 1`: `ts = sync ? hier->totalTime : a->speed` when `A ==
  0.0`; when `A != 0.0` (or NaN): `ts = sync ? hier->totalTime / A * B : a->speed`; `a->timeStep = f32(ts * dt)`
  (0x4d3601/0x4d3605). `ctx.nodes[n] = NULL` (0x4d360e).
- 4. `fd = bd->frames`; `fd->flags |= 0x80` (0x4d3619). If `fd->flags & 0x40` (0x4d362d): `ForAllFrames(skinned ? 0x4d2e40 :
  0x4d32d0, &ctx)` (thiscall on bd). Else if `onScreen`: if `fd->flags & 0x20`: `RpAnimBlendNodeUpdateKeyFrames(fd,
  bd->numFrameData)` with EAX = `&ctx` (register arg, 0x4d3690..0x4d3696); `ForAllFrames(skinned ? 0x4d2b90 : 0x4d30a0,
  &ctx)`; `fd->flags &= 0xdf`. Else `ForAllFrames(0x4d2e10, &ctx)`; `fd->flags |= 0x20` (0x4d3710). `skinned` =
  `RwHelper::GetFirstAtomic(clump) != NULL && RpSkinGeometryGetSkin(atomic->geometry) != NULL` (evaluated each time).
- 5. For each association: `a->UpdateTime(dt, A == 0.0 ? 1.0f : f32(B / A))` (thiscall 0x4d13d0, 0x4d3720..0x4d3751).
- 6. `RwFrameUpdateObjects(clump->frame)` (0x4d3764).
- gta-reversed (RpAnimBlend.cpp:1755, unhooked) differs: tests `sequences.empty()` (count) instead of the pointer, uses
  `UpdateTimeStep(dt, 1/A*B)` and `UpdateTime(dt, 1/A*B)` (exe: `total/A*B` and `B/A`, different rounding). Port follows
  the bytes.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x5d03e0 sub_5d03e0 (proposed: JPegPlugin::InitDestination)  (catalogue: unknown/REA, port level 0)
- `void (j_compress_ptr cinfo)`: `dest->next_output_byte = 0xbd0b78`, `dest->free_in_buffer = 0x32000` (0x5d03e7/0x5d03f0).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x5d0400 sub_5d0400 (proposed: JPegPlugin::EmptyOutputBuffer)  (catalogue: unknown/REA, port level 1)
- `boolean (j_compress_ptr cinfo)`: `CFileMgr::Write([0xbd0b74], 0xbd0b78, 0x32000)` (0x5d0410); reset the destination
  as in 0x5d03e0 (0x5d041c/0x5d0428); return AL = 1.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x5d0440 sub_5d0440 (proposed: JPegPlugin::TermDestination)  (catalogue: unknown/REA, port level 1)
- `void (j_compress_ptr cinfo)`: `n = dest->next_output_byte - 0xbd0b78`; if `n > 0` (signed, 0x5d0450):
  `CFileMgr::Write([0xbd0b74], 0xbd0b78, n)` (0x5d045f).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x5d0570 JPegPlugin::sub_5D0570 (proposed: JPegPlugin::WriteScanlineToRaster)  (catalogue: unknown/REA, port level 1)
- Convention: `bool (RwRaster* r, i32 unused, u32 first, u32 end)` (ebp frame: r +8, first +0x10, end +0x14); returns 1.
- For `i = first; i < end` (unsigned, 0x5d057b/0x5d05d6): build `RwRGBA {g_JpegScan[i].r, .g, .b, 0xff}` in the `first`
  stack slot (0x5d0590..0x5d05b6); `px = 0x803740(&rgba, r->cFormat << 8)` (catalogue RwImageGammaCorrectValue†, low;
  converts the colour to the raster pixel format; 0x5d05bc); `*[0xbd0160] = px`; `[0xbd0160] += 4` (0x5d05c1..0x5d05d8).
- gta-reversed: absent.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72e080 sub_72e080 (proposed: GtaGeometryListStreamRead)  (catalogue: unknown/REA, port level 9)
- Convention (register): EBX = `RwStream*`, EDI = geometry list; returns EDI or NULL. Caller 0x72e3a0.
- `RwStreamFindChunk(s, 1 STRUCT, &len, &ver)` fail -> NULL; `RwStreamRead(s, &num, 4) != 4` -> NULL (0x72e0ac);
  `list->num = num` (0x72e0b7). If `num <= 0`: `list->geoms = NULL`, return list. Else `list->geoms =
  CMemoryMgr::Malloc(num*4, 0)` (0x72f420); NULL -> return NULL. For `i < num`: `RwStreamFindChunk(s, 0xf GEOMETRY, NULL,
  &ver)` fail -> NULL; `geoms[i] = RpGeometryStreamRead(s)`; NULL -> NULL (0x72e0f0..0x72e124). Failures leave the
  partly filled list for the caller.
- gta-reversed: absent. librw: geometry list read in clump.cpp differs (librw allocates with its own allocator).

## 0x72e270 sub_72e270 (proposed: GtaAtomicStreamRead)  (catalogue: unknown/REA, port level 9)
- Convention (register + stack): EAX = `RwStream*`, EBX = frame list, `[esp+4]` = geometry list; returns `RpAtomic*` or NULL.
  Callers 0x72e3a0 and `RpClumpGtaStreamRead2` 0x72e620.
- `RwStreamFindChunk(s, 1, &len, &ver)` fail -> NULL; `RwStreamRead(s, buf, len)` into a 16-byte local (`frameIndex` +0,
  `geomIndex` +4, `flags` +8) - `len` is not checked against 16 (a larger chunk overflows the stack); `read != len` ->
  NULL (0x72e2ab). `a = RpAtomicCreate()`; NULL -> NULL. `a->flags (+2) = (u8)flags` (0x72e2c7). If `frames->num != 0`:
  `RpAtomicSetFrame(a, frames->frames[frameIndex])` (0x72e2dc). If `geoms->num != 0`: `RpAtomicSetGeometry(a,
  geoms->geoms[geomIndex], 0)` (0x72e2f9), return a. Else `RwStreamFindChunk(s, 0xf, NULL, &ver)` and `g =
  RpGeometryStreamRead(s)`; either failing -> `RpAtomicDestroy(a)`, NULL; else `RpAtomicSetGeometry(a, g, 0)`,
  `RpGeometryDestroy(g)` (drops the extra reference), return a.
- Note: the atomic extension chunk is NOT read (unlike `_rpClumpAtomicStreamRead`).
- gta-reversed: absent.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72e360 sub_72e360 (proposed: GtaGeometryListDeinitialize)  (catalogue: unknown/REA, port level 6)
- Convention (register): ESI = geometry list; returns ESI in EAX. If `num != 0`: for `i < num` (signed) destroy non-NULL
  `geoms[i]` (0x72e370..0x72e388), `CMemoryMgr::Free(geoms)` (0x72e38d). Then (through a `.HOODLUM` hop 0x72e398 ->
  0x4020f3) `list->num = 0` always, return (0x72e39d).
- gta-reversed: absent.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72e3a0 sub_72e3a0 (proposed: RpClumpGtaStreamReadFull)  (catalogue: unknown/REA, port level 10)
- Convention: `RpClump* (RwStream* s)` (ebp frame, stack aligned to 8); no static caller (reached through a pointer or dead).
  Locals: counts struct (numAtomics, numLights, numCameras) [0x2c], frame list [0x1c], geometry list [0x24].
- `RwStreamFindChunk(s, 1, &len, &ver)` fail -> NULL; `RwStreamRead(s, &counts, 0xc) != 0xc` -> NULL (fixed 12 bytes,
  0x72e3cc..0x72e3df). `c = RpClumpCreate()`; NULL -> NULL. `FindChunk(s, 0xe FRAMELIST)` fail -> NULL (clump leaked,
  0x72e40c); `_rwFrameListStreamRead(s, &fl)` fail -> `RpClumpDestroy(c)`, NULL. `FindChunk(s, 0x1a GEOMETRYLIST)` fail
  -> frame list deinit, destroy, NULL. `0x72e080(s, &gl)` fail -> frame list deinit, destroy, NULL (geometry list not
  freed). `c->frame (+4) = fl.frames[0]` (0x72e4a1). For `i < numAtomics` (signed): `FindChunk(s, 0x14 ATOMIC)` and
  `a = 0x72e270(s, &fl, &gl)`; any failure -> `0x72e360(&gl)`, deinit frames, destroy clump, NULL; else
  `RpClumpAddAtomic(c, a)`. End: `0x72e360(&gl)`, `_rwFrameListDeinitialize(&fl)`, return c. Lights, cameras and the clump
  extension are not read.
- gta-reversed: absent.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72e700 RpClumpGtaCancelStream  (catalogue: specified/REA, port level 0)
- `void (void)`: `0x72e360` with ESI = 0xc87b00 (static geometry list, 0x72e701); `_rwFrameListDeinitialize(0xc87af4)`
  (0x72e710); `[0xc87af8] <- 0` (0x72e718).
- gta-reversed: thunk only.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72fa20 cb_RSNODENAME_RwFrame_StreamWrite  (catalogue: specified/REA, port level 0)
- Plugin stream-write CB `RwStream* (RwStream* s, i32 len, const void* frame, i32 off, i32 size)`:
  `RwStreamWrite(s, frame + [0xc87c5c], len)` (0x72fa38); returns `s` (not the write result).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72fa80 cb_RSNODENAME_RwFrame_StreamGetSize  (catalogue: specified/REA, port level 0)
- `i32 (const void* frame, i32 off, i32 size)`: `p = frame + [0xc87c5c]`; `p == NULL` -> 0 (0x72fa8c); else
  `G->rwstrlen(p)` (`G+0x120`, 0x72fa97).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72fb90 cb_RSPIPELINE_RpAtomic_StreamRead  (catalogue: specified/REA, port level 2)
- `RwStream* (RwStream* s, i32 len, void* atomic, i32 off, i32 size)`: `RwStreamRead(s, atomic + [0x8d6080], len)`
  (0x72fba8; result ignored, `len` not checked against 4); returns `s`.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72fbc0 cb_RSPIPELINE_RpAtomic_StreamGetSize  (catalogue: specified/REA, port level 0)
- Returns 4 (0x72fbc0).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72fec0 D3DTextureBuffer::Clear  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: for `i < this+0x14` (count, signed, re-read): `items(+0x18)[i]->Release()` (IUnknown vtbl+8,
  0x72fed9); `this+0x14 = 0`, `this+0x10 = 0` (0x72fee5/0x72fee8).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x72fef0 D3DTextureBuffer::Destroy  (catalogue: specified/REA, port level 0)
- As Clear (0x72ff00..0x72ff20), then if `items` -> `free(items)` (0x8214bd), `items = NULL` (0x72ff32).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x7301d0 D3DIndexDataBuffer::Clear  (catalogue: specified/REA, port level 0)
- `__thiscall`: count at +0x10, items at +0x14 (Release each, vtbl+8, 0x7301e9); `+0x10 = 0`, `+0xc = 0` (0x7301f5/0x7301f8).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x730200 D3DIndexDataBuffer::Destroy  (catalogue: specified/REA, port level 0)
- As 0x7301d0, then `free(+0x14)` and NULL (0x73023a/0x730242).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x730450 sub_730450 (proposed: CGrowArray<T*>::Clear)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()` on {+0 capacity, +4 count, +8 data}: if `data` -> `free(data)`, `data = NULL` (0x73045b/0x730463);
  `count = 0` (0x73046a).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x7304b0 sub_7304b0 (proposed: CGrowArray<T*>::SetCapacity, catalogue D3DIndexDataBuffer::Setup)  (catalogue: unknown/REA, port level 1)
- `__thiscall void (i32 cap)`, `ret 4`: if `cap == this->capacity` return (0x7304b8). `p = operator new(cap*4)` (0x7304c5,
  no NULL check); copy `data[i]` for `i < count && i < cap` (0x7304d8..0x7304eb); `free(data)` (0x7304f1, also when NULL);
  `data = p`, `capacity = cap`. `count` is not reduced.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x730ad0 D3DTextureBuffer::PushWithoutIncreasingCounter  (catalogue: specified/REA, port level 0)
- `__thiscall bool (void* item)`, `ret 4`: `this+0x10 = this+0x10` (self-copy, no effect, 0x730ad0..0x730ad6); if
  `count(+0x14) < capacity(+0xc)` (signed): `items(+0x18)[count] = item`, `count++`, return 1; else return 0.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x730b30 D3DResourceSystem::sub_730B30 (proposed: CGrowArray<T*>::Push)  (catalogue: unknown/REA, port level 2)
- `__thiscall void (void* item)`, `ret 4`: while `count >= capacity` (signed): `SetCapacity(capacity * 2)` (0x730b47);
  `data[count++] = item`. With `capacity == 0` this loops forever (SetCapacity(0) returns at once). Kept.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x730e50 sub_730e50 (proposed: DestroyTextureCB, catalogue RwTexDictionaryGtaStreamRead-helper)  (catalogue: unknown/REA, port level 3)
- `RwTexture* (RwTexture* t, void* data)`: `RwTextureDestroy(t)`; returns NULL (0x730e5d), so a ForAllTextures walk stops
  after the first texture.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x732150 CVisibilityPlugins::AtomicConstructor  (catalogue: specified/REA, port level 0)
- Plugin ctor `void* (void* atomic, i32 off, i32 size)`: `*(i16*)(atomic + [0x8d608c]) = -1` (0x73215a), `*(u16*)(atomic +
  [0x8d608c] + 2) = 0` (0x732166); returns atomic.
- gta-reversed: reversed (matches).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x732b30 CVisibilityPlugins::RenderAtomic  (catalogue: specified/REA, port level 6)
- `RpAtomic* (RpAtomic* a)`: tail-like call `AtomicDefaultRenderCallBack(a)` (0x7491c0), returns its result.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x7322f0 CVisibilityPlugins::SetClumpForAllAtomicsFlag  (catalogue: specified/REA, port level 0)
- `void (RpClump* c, i32 flag)`: `RpClumpForAllAtomics(c, 0x7322d0 setClumpForAllAtomicsFlagCB, (void*)flag)` (0x7322ff).
- librw: absent (GTA-side code; librw has no equivalent).

## 0x732d60 CVisibilityPlugins::sub_732D60 (proposed: owner-unknown FreeMember50)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `free(this+0x50)` (0x8213ae), `this+0x50 = 0` (0x732d6f). UNCLEAR (0x732d61): owning class.
- librw: absent (GTA-side code; librw has no equivalent).

## 0x732d80 CVisibilityPlugins::sub_732D80 (proposed: owner-unknown FreeMember30)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `free(this+0x30)` (0x8213ae), `this+0x30 = 0` (0x732d8f). UNCLEAR (0x732d81): owning class.
- librw: absent (GTA-side code; librw has no equivalent).

