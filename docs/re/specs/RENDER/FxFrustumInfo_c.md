# FxFrustumInfo_c (RENDER) - full specs

Layout (gr `Fx/FxFrustumInfo.h`, size 0x54): +0 m_Sphere (FxSphere_c: +0 center x,y,z, +0xC radius, +0x10 m_nNumPlanesPassed
u32), +0x14 m_Planes[4] (FxPlane_c 0x10 each: +0 normal x,y,z, +0xC distance).

## 0x4AA030 FxFrustumInfo_c::IsCollision  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = FxFrustumInfo_c*), arg `FxSphere_c* sphere` [esp+4], `ret 4`; returns EAX = 0 or 1
  (full register, 0x4AA06F / 0x4AA0B9 / 0x4AA0C7).
- Callers: FxManager_c::ShouldCreate 0x4A9500, FxSystem_c::IsVisible 0x4AAF30. Callees: none.
- Behaviour:
  1. Sphere test (x87, 0x4AA034-0x4AA06D): `dx = s.x - f.x`, `dy = s.y - f.y`, `dz = s.z - f.z`, `r = f.r + s.r`
     (f = this->m_Sphere, s = *sphere); `d2 = (dz*dz + dy*dy) + dx*dx`; `fcompp` of `r*r` against d2: continue only if
     `r*r > d2` (strict; equal or NaN -> return 0 at 0x4AA06F).
  2. Plane loop (0x4AA077-0x4AA0B5): `start = sphere->m_nNumPlanesPassed` (+0x10). For i = 0..3:
     `k = (start + i) & 3`; plane P = this + 0x14 + 0x10*k;
     `dist = ((P.n.y * s.y + P.n.x * s.x) + P.n.z * s.z) - P.d` (order: y term loaded first 0x4AA08A, then x, then z, then fsub);
     if `dist > s.r` (fcomp, `test ah,0x41; je` 0x4AA0AE; NaN -> not greater): `sphere->m_nNumPlanesPassed = k` (0x4AA0C3),
     return 0.
  3. All four planes passed: return 1 (m_nNumPlanesPassed unchanged).
- Edge cases: the plane order starts at the plane that rejected the sphere last time (rotating cache, `& 3`); a value >= 4 in
  +0x10 works because of the mask, but the stored value is always the masked k.
- gta-reversed: `plugin::CallMethodAndReturn` followed by dead code (marked "todo: maybe wrong", FxFrustumInfo.cpp:9) that
  differs: it always starts at plane 0, writes `m_nNumPlanesPassed = i` on every passing plane (so a full pass leaves 3),
  and uses `GetDistToPlane(...) > radius` / `m_Sphere.IsCollision` inlines whose exact float order is not the above. Port the
  bytes, not the dead gr code. Open questions: none.
