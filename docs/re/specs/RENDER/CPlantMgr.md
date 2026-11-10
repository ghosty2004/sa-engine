# CPlantMgr (RENDER) - full specs

## 0x5DBE40 CPlantMgr::_CalcDistanceSqrToEntity  (catalogue: specified/REA, port level 2)
- Convention: NON-STANDARD register call. Inputs: EBX = `CEntity* entity`, ESI = `const CVector* posn`; no stack args, plain `ret`
  (0x5DBEA7). Returns float in ST0. Preserves EBX, ESI, EDI (EDI pushed/popped 0x5DBE43/0x5DBE71); clobbers EAX, ECX, EDX.
  gr notes "bad call conv." and keeps the hook commented out (PlantMgr.cpp:101).
- Callers: CPlantMgr::_ColEntityCache_Update 0x5DC510 (2 sites, 0x5DC564 and 0x5DC827; both load ESI = [esp+0x34] = camera pos pointer and
  EBX = entity, then `fcomp [0x86BF20]`).
- Callees: CEntity::GetColModel 0x535300 (thiscall ECX = entity, returns CColModel* in EAX), CEntity::TransformFromObjectSpace 0x5334F0
  (thiscall, args (CVector* out, const CVector* offset), returns out; `ret 8`).
- Fields: CColModel +0x18 boundSphere.center (CVector), +0x24 boundSphere.radius (f32).
- Behaviour (locals: r at [esp+4], dst at [esp+8..0x10] before the `pop edi`):
  1. `cm = entity->GetColModel()` (0x5DBE46).
  2. `entity->TransformFromObjectSpace(&dst, &cm->boundSphere.center (cm+0x18))` (0x5DBE58).
  3. x87: `dx = posn.x - dst.x` (0x5DBE5D), `dy = posn.y - dst.y` (0x5DBE66), `r = cm->radius` copied as raw dword (0x5DBE63/0x5DBE69),
     `dz = posn.z - dst.z` (0x5DBE72). `d = sqrt((dz*dz + dy*dy) + dx*dx)` (0x5DBE79-0x5DBE89; that summation order), d kept in ST0 (not stored).
  4. `fcom r; test ah,0x41; jne` (0x5DBE91-0x5DBE9A): only if `d > r` (strict; equal or NaN skip) `d = d - r` (0x5DBE9C).
  5. return `d * d` (0x5DBEA0-0x5DBEA2).
- gta-reversed: present (PlantMgr.cpp:704) and equivalent except summation order: gr's `DistanceBetweenPoints(dst, posn)` sums
  `x*x + y*y + z*z`, the original `(z*z + y*y) + x*x` (also the subtraction is posn - dst; irrelevant for squares). Port keeps the original
  order. Open questions: none.
