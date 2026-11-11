# ProcSurfaceInfo_c (RENDER) - full specs

Layout: see ProcSurfaceInfo_c.deviations.md.

## 0x5A2F80 ProcSurfaceInfo_c::IsPtInTriangle2D  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall`-shaped but `this` (ECX) is never read; all args on the stack by value, callee pops: `ret 0x3C`.
  Returns AL (bool). Stack: [esp+4] x, [+8] y, [+0x0C] v1 (x,y,z), [+0x18] v2, [+0x24] v3, [+0x30] normal, [+0x3C] float* zOut.
  The only caller (AddObjects 0x5A3B76) sets ECX = this anyway. gr notes "can't hook" (signature as a member with
  by-value CVectors).
- Callers: ProcSurfaceInfo_c::AddObjects 0x5A3850 (grid path). Callees: none. Constant: 0.0f [0x858B50].
- Behaviour (each term `A + B` with A, B products computed in this order, x87):
  1. `t1 = (v1.x - v2.x)*(v1.y - y) + (v2.y - v1.y)*(v1.x - x)` (0x5A2F80-0x5A2FA4). If `!(t1 <= 0)` return 0
     (`fcomp 0; test ah,0x41; jp` at 0x5A2FB1: t1 > 0 or NaN -> false).
  2. `t2 = (v2.x - v3.x)*(v2.y - y) + (v3.y - v2.y)*(v2.x - x)` (0x5A2FB7-0x5A2FDB), same test (0x5A2FE8).
  3. `t3 = (v1.y - v3.y)*(v3.x - x) + (v3.x - v1.x)*(v3.y - y)` (0x5A2FEA-0x5A300E), same test (0x5A301B).
  4. `*zOut = ((-(n.x*x) - n.y*y) - (-(((v1.z*n.z) + n.y*v1.y) + n.x*v1.x))) / n.z` (0x5A301D-0x5A3059, stored f32);
     return 1. n.z == 0 gives inf/NaN (no check).
- Edge cases: points on an edge (term == 0) are inside. Works only for one winding (all terms <= 0).
- gta-reversed: ProcSurfaceInfo.cpp:246, not hooked; matches the bytes (including the term order). Open questions: none.

## 0x5A2EA0 ProcSurfaceInfo_c::~ProcSurfaceInfo_c  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`: `ecx += 0x3C; jmp 0x4A8DE0` (List_c::~List_c of the member at +0x3C; empty). gta-reversed: `= default`. Open questions: none.
