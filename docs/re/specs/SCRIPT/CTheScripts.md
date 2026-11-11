# CTheScripts (SCRIPT, RE-SPEC-REST search-light helpers)

CTheScripts::ScriptSearchLightArray at 0xA94D68: 8 entries, stride 0x7C (end 0xA95148). Fields used: +0x0 u8 used, +0x3 u8 movement
state, +0x4 u16 reference id (handle high word), +0x14/+0x18 current x/y, +0x28..+0x30 target point 1, +0x34..+0x3C point 2, +0x40 f32
speed, +0x48 CEntity* followed entity, +0x58..+0x60 origin (0xA94DC0..), +0x64..+0x6C axis A (0xA94DCC), +0x70..+0x78 axis B (0xA94DD8).
Callees: CTheScripts::GetActualScriptThingIndex 0x4839A0 (cdecl (handle, type 2 = search light) → index or < 0),
CVector::NormaliseAndMag 0x59C970 (thiscall, returns length in ST0), CEntity::CleanUpOldReference 0x571A00 / RegisterReference 0x571B70
(thiscall on the entity, arg = CEntity** slot).

## 0x493280 CTheScripts::IsPointWithinSearchLight(const CVector* p, int32 index)  (catalogue: specified/REA, port level 1)
- `__cdecl`, p [esp+4], index [esp+8] (the slot is reused for a temporary), `ret`; returns AL. Callers: IsEntityWithinSearchLight 0x493900,
  IsEntityWithinAnySearchLight 0x493960.
- Behaviour: copy axis A (+0x64) and axis B (+0x70) to locals; `la = A.NormaliseAndMag()` (stored f32); `lb = B.NormaliseAndMag()` (kept in
  ST0). `d = *p - origin(+0x58)` (dx, dy, dz from f32 memory). `u = (((A.z*dz) + (A.y*dy)) + (A.x*dx)) / la` stored f32;
  `v = (((B.z*dz) + (B.y*dy)) + (B.x*dx)) / lb` (fdivrp, kept extended). Return `(v*v + u*u) <= [0x858624]=1.0f` (`test ah, 0x41; jnp`:
  less or equal → 1; greater or NaN → 0).

## 0x493360 CTheScripts::MoveSearchLightBetweenTwoPoints(int32 handle, f32 x1, y1, z1, f32 x2, y2, z2, f32 speed)  (catalogue: specified/REA, port level 5)
- `__cdecl`, 8 args [esp+4..+0x20], `ret`. Caller: ProcessCommands1700To1799 0x496E00.
- `i = GetActualScriptThingIndex(handle, 2)`; < 0 → return. `d1 = (dy1*dy1) + (dx1*dx1)` with `dx1 = SL.x(+0x14) - x1`, `dy1 = SL.y(+0x18) - y1`;
  `d2` likewise for (x2, y2) (z ignored). Store `+0x28 = x1, +0x2C = y1, +0x30 = z1, +0x34 = x2, +0x38 = y2, +0x3C = z2` (raw).
  `fcompp` d1 vs d2 (0x4933E3): if `d1 <= d2` → `SL+3 = 0x81` (head to point 1 first) else (greater or NaN) `SL+3 = 0x82`.
  `+0x40 = speed`; if `SL.entity (+0x48)` → `entity->CleanUpOldReference(&SL.entity)`; `SL.entity = NULL`.

## 0x493420 CTheScripts::MoveSearchLightToEntity(int32 handle, CEntity* entity, f32 speed)  (catalogue: specified/REA, port level 5)
- `__cdecl`, `ret`. Stub → .HOODLUM 0x156F000. Caller: ProcessCommands1700To1799 0x496E00.
- `i` as above (< 0 → return). `SL+3 = 0x83`; `+0x28..+0x3C = 0` (six dwords); `+0x40 = speed`. If `SL.entity` → CleanUpOldReference
  0x571A00 (reached through a disguised call at 0x156F045: [0x15605BC] + [0x4D3940] + CF 1 = 0x571A00, returning to 0x156F064).
  Then `SL.entity = entity; entity->RegisterReference(&SL.entity)` (no NULL check on entity).

## 0x493480 CTheScripts::MoveSearchLightToPointAndStop(int32 handle, f32 x, f32 y, f32 z, f32 speed)  (catalogue: specified/REA, port level 5)
- `__cdecl`, `ret`. Stub → .HOODLUM 0x1562220. Caller: ProcessCommands1700To1799 0x496E00.
- `i` (< 0 → return). `+0x28 = x, +0x2C = y, +0x30 = z, +0x34/+0x38/+0x3C = 0; SL+3 = 0x84; +0x40 = speed`. If `SL.entity` →
  CleanUpOldReference (disguised call at 0x156227C: [0x156048C] - [0x4BFEC4] = 0x571A00, returns to 0x156229B); `SL.entity = NULL`.

## 0x493900 CTheScripts::IsEntityWithinSearchLight(int32 handle, CEntity* entity)  (catalogue: specified/REA, port level 5)
- `__cdecl`, `ret`, returns AL. Stub → .HOODLUM 0x156C000. Caller: ProcessCommands1700To1799 0x496E00.
- `i` (< 0 → 0); `pos = entity->m_matrix ? matrix.pos : placement.pos` (copied); return `IsPointWithinSearchLight(&pos, i)` normalised to 0/1.

## 0x493960 CTheScripts::IsEntityWithinAnySearchLight(CEntity* entity, int32* outHandle)  (catalogue: specified/REA, port level 2)
- `__cdecl`, `ret`, returns AL. Caller: ProcessCommands1900To1999 0x46B460.
- `*outHandle = -1`; pos as above; for i = 0..7: if `SL[i].used` and `IsPointWithinSearchLight(&pos, i)` → `*outHandle = (SL[i].id (u16 +4) << 16) | i`;
  return 1. None → 0.
