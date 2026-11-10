# RenderBuffer (RENDER) - full specs

Globals (gr `common.h`): `uiTempBufferVerticesStored` 0xC4B950 (i32), `uiTempBufferIndicesStored` 0xC4B954 (i32),
`aTempBufferIndices` 0xC4B958 (RxVertexIndex[]), `TempBufferVertices` 0xC4D958 (RwIm3DVertex[]), `g_nNumIm3dDrawCalls` 0xB73708.

## 0x6E7680 RenderBuffer::RenderIfDoesntFit  (catalogue: specified/REA, port level 6)
- NAME/SIGNATURE CORRECTION: despite the gr name and its `(int32 nIdx, int32 nVtx)` prototype (gr RenderBuffer.hpp:58), the original
  takes NO arguments and performs NO fit check; it is a copy of `RenderStuffInBuffer` (0x707800) with a `LittleTest()` call added.
  Better name: `RenderBuffer::RenderStuffInBufferCounted` (proposed). The "does it fit" test is inlined in every caller (e.g.
  CWaterLevel::RenderFlatWaterTriangle_OneLayer 0x6E8ED0-0x6E8EF1: `if (uiTempBufferIndicesStored >= 0xFFD || uiTempBufferVerticesStored >= 0x7FD) call 0x6E7680`).
- Convention: `__cdecl void ()`, no stack arguments read, plain `ret` (0x6E76D3); ESI preserved (push/pop).
- Callers (4): 0x6E8780 (unnamed), CWaterLevel::RenderFlatWaterTriangle_OneLayer 0x6E8ED0, 0x6E91D0 (unnamed),
  CWaterLevel::RenderFlatWaterRectangle_OneLayer 0x6E9940.
- Callees: Common::LittleTest 0x541330 (`++g_nNumIm3dDrawCalls` [0xB73708], via a .text detour 0x409B56), RwIm3DTransform 0x7EF450,
  RwIm3DRenderIndexedPrimitive 0x7EF550, RwIm3DEnd 0x7EF520 (all cdecl, caller pops).
- Behaviour:
  1. `n = uiTempBufferVerticesStored` (0x6E7680); if `n == 0` go to 4 (0x6E768A).
  2. `LittleTest()` (0x6E768C). Reload `n = uiTempBufferVerticesStored` (0x6E7691).
     `if (!RwIm3DTransform(TempBufferVertices 0xC4D958, n, NULL /*ltm*/, 1 /*rwIM3D_VERTEXUV*/))` go to 4 (0x6E769F-0x6E76A9).
  3. `RwIm3DRenderIndexedPrimitive(3 /*rwPRIMTYPETRILIST*/, aTempBufferIndices 0xC4B958, uiTempBufferIndicesStored)` (0x6E76B9);
     `RwIm3DEnd()` (0x6E76C1).
  4. `uiTempBufferVerticesStored = 0` (0x6E76C6), then `uiTempBufferIndicesStored = 0` (0x6E76CC) - always, also when nothing was drawn.
- Edge cases: the index count is not checked (0 indices with >0 vertices still calls RenderIndexedPrimitive). Differs from 0x707800 only by
  the LittleTest call and the order of the two clearing stores (0x707800 clears indices first; not observable).
- gta-reversed: present but unhooked and WRONG: `if (indicesStored + nIdx > IdxBufferSize || verticesStored + nVtx > VtxBufferSize)
  RenderStuffInBuffer();` (RenderBuffer.cpp:89) - the original has no arguments, no condition, and increments g_nNumIm3dDrawCalls. Ports
  must follow the bytes (and callers must not pass arguments). Open questions: none.
