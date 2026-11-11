# RW math pass 3: bounding boxes, x87 control word, vector module (RE-SPEC-RW-3 lead, taking over part of writer B's unwritten file)

All `__cdecl`. RwBBox: +0x0 sup (RwV3d), +0xc inf (RwV3d). x87 compare idiom: `fcomp` + `fnstsw ax`; `and eax, 0x4100; jne` jumps when C0 or C3 is
set (ST0 < src, equal, or unordered); `test ah, 5; jp` jumps when C0 and C2 are both clear or both set (ST0 > src, equal, or unordered).

## 0x809020 RwBBoxInitialize  (catalogue: specified/REA, port level 0)
- `RwBBox* (RwBBox* b, const RwV3d* v)`: `b->inf (+0xc) = *v` then `b->sup (+0) = *v` (dword copies); returns b.

## 0x809060 RwBBoxAddPoint  (catalogue: specified/REA, port level 0)
- `RwBBox* (RwBBox* b, const RwV3d* p)`. For x, y, z in that order: `if (!(inf.c <= p.c) and ordered) inf.c = p.c`, i.e. inf takes p when `p.c < inf.c`
  (0x809068-0x8090a2); then for x, y, z: sup takes p when `sup.c < p.c` (0x8090a5-0x8090d9). NaN components never update. Dword copies; returns b.
- librw: base.cpp BBox::addPoint matches (it uses plain float compares; NaN behaviour same).

## 0x8090e0 RwBBoxContainsPoint  (catalogue: specified/REA, port level 0)
- `RwBool (const RwBBox* b, const RwV3d* p)`: returns 0 at the first failing test in this order: `inf.x <= p.x`, `inf.y <= p.y`, `inf.z <= p.z`,
  `p.x <= sup.x`, `p.y <= sup.y`, `p.z <= sup.z` (unordered compares count as passing); else 1.
- librw: base.cpp BBox::containsPoint matches for ordinary values.

## 0x857404 _rwProcessorInitialize  (catalogue: specified/REA, port level 0)
- `void (void)`, EAX preserved (push/pop). `fnstcw [0xcaf000]`; copy the dword at [0xcaf000] to [0xcaf004] (saved control word); `cw = (cw & 0xfffffcff) | 0x3f`
  (precision control 00 = 24-bit single precision, all exceptions masked); `fldcw`. Called by D3D9System FinalizeStart.
- This is the reason in-game x87 code runs at 24-bit precision (see BRIEF.md).

## 0x85742b _rwProcessorRelease  (catalogue: specified/REA, port level 0)
- `void (void)`: `fldcw [0xcaf004]` (restores the control word saved by 0x857404). Caller 0x7f7100.

## 0x857432 _rwProcessorForceSinglePrecision  (catalogue: specified/REA, port level 0)
- `void (void)`: `fnstcw [0xcaf000]`; `[0xcaf000] = ([0xcaf000] & 0xfffffcff) | 0x3f` (dword ops); `fldcw [0xcaf000]`. Caller: _rwD3D9CameraBeginUpdate
  0x7f8f20 (D3D9 can reset the FPU state, so every camera begin re-forces single precision).

## 0x7ede20 cb_VECTORMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Engine close callback: `m = Engine + [0xc97934]`: free `m+4` (Engine+0x138) if set and zero it, then free `m+0` if set and zero it (the sqrt / inverse-sqrt
  tables, see pass 1 RwMatrix.md); `[0xc97938] -= 1`; returns instance.
