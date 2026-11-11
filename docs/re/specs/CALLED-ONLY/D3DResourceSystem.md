# D3DResourceSystem (CALLED-ONLY) — RE-SPEC-REST

Recycling pools for Direct3D 9 textures and index buffers (all statics; gta-reversed only has `plugin::Call` wrappers in
`RenderWare/D3DResourceSystem.cpp`, so there is no gr C++ to compare).

Globals (layouts from gr `D3DTextureBuffer.h` / `D3DIndexDataBuffer.h`, confirmed by the accesses below):
- `UseD3DResourceBuffering` u8 [0x8D6084] (initial .data value 1).
- `FreeTextureBufferIndex` i32 [0xC87C60] = number of texture buffers in use (1..17).
- `TextureBuffer[17]` at 0xC87C68, stride 0x1C (D3DTextureBuffer: +0x0 format, +0x4 width, +0x8 levels (-1 undefined, 1 =
  single level, 0 = mip chain), +0xC capacity, +0x10 counter (pushes minus pops), +0x14 size (entries stored), +0x18 IDirect3DTexture9**
  array). Element 0 (fields 0xC87C68/6C/70/74/78/7C/80) is the catch-all buffer; elements 1..16 are per (format, size, levels).
- `IndexDataBuffer[16]` at 0xC87E48, stride 0x18 (D3DIndexDataBuffer: +0x0 format, +0x4 field_4 (max index count of the bucket),
  +0x8 capacity, +0xC counter, +0x10 size, +0x14 IDirect3DIndexBuffer9** array), followed by the overflow index buffer at 0xC87FC8
  (format 0xC87FC8, field_4 0xC87FCC, capacity 0xC87FD0, counter 0xC87FD4, size 0xC87FD8, array 0xC87FDC).
- Round-robin cursors: texture tidy cursor i32 [0xC87FE0], index tidy cursor i32 [0xC87FE4].
- COM calls are through the object's vtable (stdcall, object pushed): +0x8 Release, texture +0x34 GetLevelCount,
  texture +0x44 GetLevelDesc(level, D3DSURFACE_DESC*), index buffer +0x34 GetDesc(D3DINDEXBUFFER_DESC*).
- CRT: operator new 0x82119A, _free 0x8214BD.

## 0x7305E0 D3DResourceSystem::TidyUpD3DTextures(uint32 count)  (catalogue: specified/REA, port level 0)
- `__cdecl`, count at [esp+4], plain `ret`. Callers: CGame::TidyUpMemory 0x53C500, CGame::DrasticTidyUpMemory 0x53C810,
  CMenuManager::Process 0x57B440.
- Protection: 0x7305ED jumps to a stub at 0x40A16B that computes `edx = *(u32*)([0x1560904] + [0x685002])` with CF cleared
  (0xF0B844 + 0xFFD7C79C = 0xC87FE0 mod 2^32), i.e. `edx = cursor [0xC87FE0]`, flags preserved, then returns to 0x7305F2. Port as a
  plain read of 0xC87FE0.
- Behaviour: `left = count; empty = 0`; if count == 0 return (0x7305E8). `n = FreeTextureBufferIndex`. Loop (0x730600):
  1. if `empty >= n` (signed) → return.
  2. `b = &TextureBuffer[cursor]`. If `b.size != 0` (0x730611): `b.counter--; b.size--; t = b.array[b.size]`; if t != NULL:
     `t->Release()`, reload `n` and `cursor` from memory, `empty = 0`, `left--` (0x73062A-0x73063E). If `b.size == 0` or t == NULL:
     `empty++` (0x730641) (a NULL slot still loses its counter/size decrement).
  3. `cursor = (cursor + 1) % n` (signed `idiv`), stored to [0xC87FE0] (0x730642-0x73064A); loop while `left != 0`.
- gta-reversed: `plugin::Call<0x7305E0>` only.

## 0x730740 D3DResourceSystem::TidyUpD3DIndexBuffers(uint32 count)  (catalogue: specified/REA, port level 0)
- `__cdecl`, count [esp+4], `ret`. Callers: CGame::TidyUpMemory 0x53C500, CGame::DrasticTidyUpMemory 0x53C810 (tail call),
  CMenuManager::Process 0x57B440.
- Behaviour: if count == 0 return. Phase 1 (overflow buffer first, 0x730750): while `size [0xC87FD8] != 0`: `counter [0xC87FD4]--;
  size--; ib = array [0xC87FDC][size]`; if ib == NULL → phase 2; else `ib->Release(); left--`; if left == 0 return (0x730780).
  Phase 2 (0x730785): if left == 0 return; `cursor = [0xC87FE4]`, `empty = 0`; loop: if `empty >= 0x10` → return; `b =
  &IndexDataBuffer[cursor]`; if `b.size != 0`: `b.counter--; b.size--; ib = b.array[b.size]`; if ib: Release, reload cursor,
  `empty = 0`, `left--`; else (size 0 or NULL) `empty++`. `cursor = (cursor + 1) % 16` (signed remainder idiom, 0x7307D1-0x7307DE),
  stored to [0xC87FE4]; loop while left != 0.
- gta-reversed: `plugin::Call<0x730740>` only.

## 0x730830 D3DResourceSystem::Init  (catalogue: specified/REA, port level 0)
- `__cdecl`, no args, `ret`. Caller: CGame::InitialiseRenderWare 0x5BD600.
- Behaviour in store order: `FreeTextureBufferIndex [0xC87C60] = 1`; `UseD3DResourceBuffering [0x8D6084] = 1`;
  TextureBuffer[0]: format [0xC87C68] = 0, width [0xC87C6C] = 0, levels [0xC87C70] = -1; `array [0xC87C80] = operator new(0x40)`;
  capacity [0xC87C74] = 16; size [0xC87C7C] = 0; counter [0xC87C78] = 0.
  IndexDataBuffer[i], i = 0..15 (0x730890 loop, ESI from 0xC87E5C step 0x18 while < 0xC87FDC): format = 0x65 (D3DFMT_INDEX16),
  field_4 = 100*(i+1) (EDI starts 0x64, += 0x64), array = operator new(0x40), capacity = 16, size = 0, counter = 0.
  Overflow buffer: format [0xC87FC8] = 0x65, field_4 [0xC87FCC] = 0, array [0xC87FDC] = operator new(0x40), capacity [0xC87FD0] = 16,
  size [0xC87FD8] = 0, counter [0xC87FD4] = 0.
- TextureBuffer[1..16] are not touched (Setup fills them on demand). No NULL check on operator new.
- gta-reversed: `plugin::Call<0x730830>` only.

## 0x730900 D3DResourceSystem::CancelBuffering  (catalogue: specified/REA, port level 1)
- `__cdecl`, no args, `ret`. Caller: CGame::ShutdownRenderWare 0x53BB80.
- Behaviour: for i = 0 while i < FreeTextureBufferIndex (re-read each iteration, 0x73094B): for j = 0 while j < TextureBuffer[i].size
  (re-read): `TextureBuffer[i].array[j]->Release()` (no NULL check); then `size = 0; counter = 0`; if array != NULL: `_free(array);
  array = NULL`. Then `FreeTextureBufferIndex = 0` (0x730958). The same for IndexDataBuffer[0..15] (ESI 0xC87E58 step 0x18 while
  < 0xC87FD8) and for the overflow buffer (0x7309A6-0x7309E8). Finally `UseD3DResourceBuffering = 0` (0x7309F1).
- gta-reversed: `plugin::Call<0x730900>` only.

## 0x730A00 D3DResourceSystem::Shutdown  (catalogue: specified/REA, port level 0)
- `__cdecl`, no args, `ret`. Caller: CGame::Shutdown 0x53C900.
- Behaviour: the release loops of CancelBuffering 0x730900 (all texture buffers i < FreeTextureBufferIndex, 16 index buckets,
  overflow), each followed by `size = 0; counter = 0`, and `FreeTextureBufferIndex = 0` (0x730A45) — but the arrays are NOT freed
  and UseD3DResourceBuffering is NOT changed (original leak of the 0x40-byte arrays; keep).
- gta-reversed: `plugin::Call<0x730A00>` only.

## 0x730AC0 D3DResourceSystem::SetUseD3DResourceBuffering(bool)  (catalogue: specified/REA, port level 0)
- `__cdecl`, u8 at [esp+4], `ret`. `[0x8D6084] = (u8)arg`. Callers: CGame::ShutDownForRestart 0x53C550, InitialiseWhenRestarting
  0x53C680, Shutdown 0x53C900, Init3 0x5BA400, Init1 0x5BF840.

## 0x730B70 D3DResourceSystem::DestroyTexture(IDirect3DTexture9* tex)  (catalogue: specified/REA, port level 2)
- `__cdecl`, tex at [esp+4] ([esp+0x38] in the body), `ret`. Caller: _rwD3D9RasterDestroy 0x4CBB00. Callees: D3DTextureBuffer::Setup
  0x72FE80 (thiscall, args format, width, levels, capacity), Push 0x72FFF0, Resize 0x730020 (thiscall, newCapacity),
  PushWithoutIncreasingCounter 0x730AD0.
- Behaviour:
  1. If `UseD3DResourceBuffering == 0` → `tex->Release()`; return (0x730D19).
  2. `oneLevel = 1`; if `tex->GetLevelCount() > 1` (unsigned, 0x730B93) → `oneLevel = 0`.
  3. `tex->GetLevelDesc(0, &desc)` (desc local at [esp+0x14]; Format +0, Width +0x18, Height +0x1C). `w = Width`, `h = Height`,
     `n = FreeTextureBufferIndex`.
  4. If `w == h` and `n > 1` (0x730BAE-0x730BBF): `wf = (f32)(u32)w` (fild + `[0x858C54]=4294967296.0f` fix-up when the sign bit is
     set, stored f32). For i = 1 while i < n: compare `(float)(i32)TextureBuffer[i].width` with wf (`fcomp; test ah, 0x44; jp` → equal
     only), then `desc.Format == TextureBuffer[i].format` and `oneLevel == TextureBuffer[i].levels` → found (0x730C6A). (Widths are
     compared through floats, so values above 2^24 could alias; irrelevant for the <= 256 sizes stored.)
  5. Found buffer b (0x730C6A): `b.counter++`; if `b.size < b.capacity` (signed): `b.array[b.size] = tex; b.size++`; return. Else
     `b.Resize(2 * b.capacity)` then `b.PushWithoutIncreasingCounter(tex)` (which stores only if size < capacity); return.
  6. Not found (0x730C03): if `n < 17` (`cmp ecx, 0x11; jge`) and `w == h` and `w >= 0x20` (unsigned): if `w > 0x100` → Release and
     return; else `TextureBuffer[n].Setup(desc.Format, w, oneLevel, 16)`, then `TextureBuffer[FreeTextureBufferIndex].Push(tex)` (index
     re-read after Setup) and `FreeTextureBufferIndex++` (0x730C2D-0x730C5D); return.
  7. Otherwise (0x730CAB): if `w > 0x100` or `h > 0x100` (unsigned) → Release and return. Else catch-all buffer 0: `counter
     [0xC87C78]++`; if `size [0xC87C7C] < capacity [0xC87C74]` → `array[size] = tex; size++`; else `TextureBuffer[0].Resize(2*capacity)`
     and, if now `size < capacity`, store and `size++`; if still full the texture is dropped WITHOUT Release (0x730CFB → 0x730D1F;
     original leak, unreachable unless Resize fails to grow).
- gta-reversed: `plugin::Call<0x730B70>` only.

## 0x730D30 D3DResourceSystem::DestroyIndexBuffer(IDirect3DIndexBuffer9* ib)  (catalogue: specified/REA, port level 2)
- `__cdecl`, ib at [esp+4] ([esp+0x20]), `ret`. Caller: _rxD3D9DefaultDestroyInstanceCallback 0x4C9990. Callee: D3DIndexDataBuffer::Resize
  0x730330 (thiscall, newCapacity).
- Behaviour:
  1. If `UseD3DResourceBuffering == 0` → `ib->Release()`; return (0x730E27).
  2. `ib->GetDesc(&desc)` (local at [esp+8]; Size at +0x10). `k = ((Size >> 1) - 1) / 100` (u32 arithmetic: `shr 1; dec;` then the
     multiply-by-0x51EB851F / `shr 5` divide by 100, 0x730D51-0x730D5F). Size < 2 wraps to a huge k.
  3. If `k < 16` (signed compare of the u32 quotient; values >= 2^31 are impossible after /100): bucket b = IndexDataBuffer[k];
     `b.counter++`; if `b.size < b.capacity` (signed) → `b.array[b.size] = ib; b.size++`; else `b.Resize(2*b.capacity)` and store if now
     `size < capacity`, otherwise drop without Release (0x730DA3 → return).
  4. Else overflow buffer: `counter [0xC87FD4]++`; if `size [0xC87FD8] < capacity [0xC87FD0]` → `array [0xC87FDC][size] = ib; size++`;
     else Resize(2*capacity) on the buffer at 0xC87FC8, then store if room, else drop.
- gta-reversed: `plugin::Call<0x730D30>` only.
