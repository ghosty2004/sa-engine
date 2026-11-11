# RwIm3D: pipeline getters and module close (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwIm3D.md` (read-only) covers transform/end/render-indexed-primitive.

## Layout (IMMEDIATEMODULE data `m = RwEngineInstance + [0xc9c078]`)
- +0x0 current transform pipeline (0x7ef9db); current render pipelines per primitive type: +0x4 TRILIST, +0x8 TRIFAN, +0xc TRISTRIP,
  +0x10 LINELIST, +0x14 POLYLINE, +0x18 POINTLIST (jump table .text 0x7efaa0, cases 0x7ef9fd-0x7efa6f); +0x1c default transform
  pipeline (0x7efdeb..0x7efe00), +0x20..+0x30 default render pipeline block (0x7efdeb, zeroed 0x80a187-0x80a192); +0x38 Im3D pool
  (0x7efddb). `[0xc9c07c]` module open count (0x7efe13).
- Error: `RwErrorSet({1, _rwerror(0x25, primType)})` for an invalid primitive type (0x7efa78).

## 0x7ef9d0 RwIm3DGetTransformPipeline  (catalogue: specified/REA, port level 0)
- returns m+0x0.
- librw: absent.

## 0x7ef9e0 RwIm3DGetRenderPipeline  (catalogue: specified/REA, port level 1)
- `RxPipeline* (RwPrimitiveType t)`: `t-1 > 5` (unsigned) -> error 0x25 with t as argument, NULL. Else 1 LINELIST -> m+0x10,
  2 POLYLINE -> m+0x14, 3 TRILIST -> m+0x4, 4 TRISTRIP -> m+0xc, 5 TRIFAN -> m+0x8, 6 POINTLIST -> m+0x18 (table 0x7efaa0 =
  {0x7efa3a, 0x7efa4e, 0x7ef9fd, 0x7efa25, 0x7efa11, 0x7efa62}).
- librw: absent.

## 0x7efdd0 _rwIm3DGetPool  (catalogue: specified/REA, port level 0)
- returns the address m+0x38.
- librw: absent.

## 0x7efde0 cb_IMMEDIATEMODULE_RwEngine_Close  (catalogue: specified/REA, port level 3)
- `void* (void* instance, RwInt32, RwInt32)`: `_rwIm3DDestroyRenderPipelines(&m+0x20)` 0x80a140, then
  `_rwIm3DDestroyTransformPipeline(&m+0x1c)` 0x80a110, `[0xc9c07c]--`. Returns instance.
- librw: absent.

## 0x80a110 _rwIm3DDestroyTransformPipeline  (catalogue: specified/REA, port level 2)
- `void (RxPipeline** p)`: `RwIm3DSetTransformPipeline(NULL)` 0x7efac0 (restores the default); if `*p` != NULL: `_rxPipelineDestroy(*p)`
  0x805820, `*p = NULL`.
- librw: absent.

## 0x80a140 _rwIm3DDestroyRenderPipelines  (catalogue: specified/REA, port level 2)
- `void (RxPipeline** p /*5 slots*/)`: `_rwD3D9Im3DRenderClose()` 0x80dfb0; `RwIm3DSetRenderPipeline(NULL, t)` 0x7efb10 for t = 3, 5, 4, 1, 2
  in this order (POINTLIST 6 is not reset); if `p[0]` != NULL `_rxPipelineDestroy(p[0])`; then `p[0..4] = NULL` (only p[0] is destroyed:
  all primitive types share one pipeline object).
- librw: absent.
