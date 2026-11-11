# RW: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x41b1a0 ClumpCollisionConstructor (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void* (void* object, i32 offset, i32 size)`; returns EAX = object (0x41b1a0). RpClump plugin constructor callback; nothing else.

## 0x41b1b0 ClumpCollisionCopyConstructor (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void* (void* dst, const void* src, i32 offset, i32 size)`; returns EAX = dst (0x41b1b0).

## 0x41b1c0 ClumpCollisionDestructor (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void* (void* object, i32 offset, i32 size)`; returns EAX = object (0x41b1c0).

## 0x41b1d0 ClumpCollisionStreamRead (proposed)  (catalogue: specified/REA, port level 3)
- `__cdecl RwStream* (RwStream* stream [esp+4], i32 len [esp+8], void* object, i32 off, i32 size)`; returns EAX = stream. SEH frame (state var) around the `new CColModel`.
- Behaviour: `0x72f4a0()` (gr CMemoryMgr::LockScratchPad); `RwStreamRead(stream, PC_Scratch 0xc8e0c8, len)` 0x7ec9d0; `m = operator new(0x30)` 0x40fc30, `if (m) CColModel::CColModel(m)` 0x40fb60 else m = 0 (no null check afterwards).
- fourcc = [0xc8e0c8]: `"COL2"` 0x324c4f43 -> 2, `"COL3"` 0x334c4f43 -> 3, `"COLL"` 0x4c4c4f43 -> 1, else 0 (0x41b22c-0x41b249).
- 1: `CFileLoader::LoadCollisionModel(0xc8e0e8 (=PC_Scratch+0x20), m, 0)` 0x537580. 2: `LoadCollisionModelVer2(0xc8e0e8, [0xc8e0cc]-0x18, m, 0)` 0x537ee0. 3: `LoadCollisionModelVer3(0xc8e0e8, [0xc8e0cc]-0x18, m, 0)` 0x537ce0. 0 (unknown fourcc): `LoadCollisionModel(0xc8e0c8 (PC_Scratch itself), m, 0)` (0x41b268 -> 0x41b2a5).
- Then `m->MakeMultipleAlloc()` 0x40f740; `CCollisionPlugin::ms_currentModel [0x9689e0]->SetColModel(m, 1)` 0x4c4bc0; `ms_currentModel->byte[0x13] |= 8` (0x41b2c7, gr bDontWriteZBuffer); `0x72f4b0()` (ReleaseScratchPad).
- gta-reversed differs: its (unused, incomplete) code does nothing for an unknown fourcc; the original parses it as version 1 from offset 0.

## 0x41b2f0 ClumpCollisionStreamWrite (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl RwStream* (RwStream* stream, i32 len, const void* obj, i32 off, i32 size)`: returns EAX = stream; writes nothing.

## 0x41b300 ClumpCollisionGetSize (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl i32 (const void*, i32, i32)`: `or eax,-1` -> returns -1 (plugin data never streamed).

## 0x4cad90 _rwD3D9RasterRenderGeneric (proposed)  (catalogue: unknown/REA, port level 5)
- Convention: custom register args - EAX = source raster, ECX = second argument (forwarded), EDX = third argument (forwarded, e.g. the rectangle), one stack arg [esp+4] (forwarded); returns EAX = 1 / 0. Caller cleans the stack (plain `ret`).
- `if (!src) return 0`. Target raster `t = *[0xb4e9e4]` loaded through a HOODLUM indirection (0x401ee5: `[0x15606f8] - [0x5da61a]` = 0xb4e9e4 statically).
- `switch (t->type [+0x20])` (table 0x4cadf8): 0 and 4 -> table A; 2 and 5 -> table B; 1, 3, > 5 -> return 0.
- Table A (0x4cae10) on `src->type`: 0, 2, 4, 5 -> `r = 0x4caae0(edx, ecx)` (cdecl, EAX = src still set) and return `r != 0`; 3 or > 5 -> return 1; 1 -> return 0.
- Table B (0x4cae28) on `src->type`: 0, 4, 5 -> `0x4ca5a0(edx, [esp+4])` with ESI = src, return 1; 2 -> as table A's call to 0x4caae0; 3 or > 5 -> return 1; 1 -> return 0.
- Catalogue extent note: 0x401ee5-0x401efa belongs to this function.
- Open question: exact meaning of the forwarded ECX/EDX/stack arguments (caller-side, 0x4cadcd/0x4caddf).

## 0x4ccd70 _rwD3D9RasterCreateZBuffer (proposed)  (catalogue: unknown/REA, port level 1)
- Convention: custom, `esi` = D3D9 raster extension, `edi` = RwRaster; returns EAX = 1 success / 0 failure.
- `d = _RwD3D9ZBufferDepth [0xc9befc]`. If d == 16: `ext->format(+0x18) = _rwD3D9CheckValidZBufferFormat(0x49) ? 0x49 (D15S1) : 0x50 (D16)` 0x4cbd80. If d == 32: `0x4b` (D24S8) if valid, else `0x4f` (D24X4S4) if valid, else `0x47` (D32) if valid, else `0x4d` (D24X8). Other depths: format untouched.
- `byte[ext+8] = 0`; `GetClientRect(WindowHandle [0xc97c1c], &rc)` (import 0x858304).
- `if (rc.right == raster->width [+0xc] && rc.bottom == raster->height [+0x10])`: `ext[0] = _RwD3D9DepthStencilSurface [0xc97c2c]`; return 1 (shares the window's depth buffer; no AddRef).
- Else: `_RwD3DDevice->vtbl[5]()` (EvictManagedResources); `hr = _RwD3DDevice->CreateDepthStencilSurface(width, height, ext->format, [0xc9c050] (multisample type), [0xc9c054] (quality), FALSE, &ext[0], NULL)` (vtable slot 0x74); `if (hr == 0x8876017c (D3DERR_OUTOFVIDEOMEMORY) || hr < 0) return 0; return 1`.
- rc.left/top are not subtracted (client rect origin is 0 anyway).
