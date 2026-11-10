# CTaskComplexWalkRoundBuildingAttempt (AI) - RE-SPEC-3 group I3 (written by the lead)
Vtable 0x86F334. Layout (gr): +0xC m_route (CPointRoute*), +0x10 m_crapRoute, +0x14 m_targetPos, +0x20 m_pos, +0x2C m_normal, +0x38
m_targetEntity (registered ref), +0x3C m_offset, +0x48 m_moveState i8, +0x49 flags (bit0 headOnCollision, bit1 routeHasPoints, bit2
isWalkingAroundEntity, bit3 crapRouteHasPoints, bit4 justAbort, bit5 isFollowingRouteNow), +0x4A m_nAttempts.

## 0x654840 CTaskComplexWalkRoundBuildingAttempt::CTaskComplexWalkRoundBuildingAttempt (entity overload)  (catalogue: specified/REA, port level 2)
- `__thiscall this (int32 moveState [esp+4], CEntity* target [+8], const CVector* offset [+0xC], const CVector* pos [+0x10], const CVector*
  normal [+0x14], bool headOn [+0x18])`, `ret 0x18`, SEH.
- `CTaskComplex()`; vtbl = 0x86F334; `m_pos = *pos`; `m_normal = *normal`; `m_targetEntity = target`; `m_offset = *offset`; `m_moveState = (i8)moveState`;
  `flags = (flags & 0xC4) | (headOn & 1) | 4` (bits 6/7 keep uninitialised pool contents); `m_nAttempts = 0`; `m_normal.z = 0` (+0x34);
  `m_normal.Normalise()` 0x59C910; `m_route = CPointRoute::operator new(0x64)` 0x41B7C0 with `count = 0` (or 0); `m_crapRoute` likewise;
  `CEntity::RegisterReference(target, &m_targetEntity)` 0x571B70 - called WITHOUT a null check (a null target faults: original bug).
  m_targetPos (+0x14) is not initialised.
- gta-reversed: cpp:44 C++ (not hooked) matches (unconditional `CEntity::RegisterReference(m_targetEntity)`); gr zero-initialises the
  flag byte and m_targetPos (default member initialisers) where the original keeps pool contents in bits 6/7 and leaves m_targetPos.
