# CRope (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Rope.cpp`. CRope (0x328, Rope.h): +0x000 m_aSegments[32] (CVector), +0x180
m_aSpeed[32], +0x300 m_nId, +0x304 m_fGroundZ, +0x308 m_fMass, +0x30C m_fTotalLength, +0x310 m_pRopeHolder,
+0x314 m_pAttachedEntity, +0x318 m_pRopeAttachObject, +0x31C m_fSegmentLength, +0x320 m_nTime, +0x324 m_nSegments,
+0x325 m_nType, +0x326 m_nFlags1, +0x327 m_nFlags2.

## 0x556070 CRope::CreateHookObjectForRope  (gr_deviations = 1)
Row Rope.cpp:66 (notsa-unreachable, `default: NOTSA_UNREACHABLE()` in the model switch):
- Original bytes 0x55608A-0x5560D2: returns at once if `m_pAttachedEntity (+0x314) != 0`. Then `t = m_nType (+0x325) - 1`;
  if `t > 6` (unsigned, i.e. type 0 NONE or >= 8 incl. SWAT) it returns without creating anything (0x5560A9 ->
  0x556183). Jump table 0x556194: types 1, 4, 6, 7 -> `MI_CRANE_MAGNET (0x8CD700)`, 2 -> `MI_CRANE_HARNESS (0x8CD6FC)`,
  3 -> `MI_MINI_MAGNET (0x8CD740)`, 5 -> `MI_WRECKING_BALL (0x8CD71C)` (each read as u16, zero-extended).
- gr: types 0 and > 8 hit NOTSA_UNREACHABLE (abort); SWAT returns via MODEL_INVALID.
- Verdict: CONFIRMED (the original silently returns for every unlisted type). Class: harmless (keep-original: return).

Rest of the function, checked against gr (matches): `new(0x17C)` 0x5A1EE0 (pool operator new), `CObject(model, true)`
0x5A1D70 (null result skips the ctor, eax = 0); `m_pAttachedEntity = obj`; `obj->RegisterReference(&m_pAttachedEntity)`
0x571B70; position = `m_aSegments[31]` (+0x174/+0x178/+0x17C) written to `matrix->pos (+0x30..)` if `obj->m_matrix (+0x14)`
else to placement pos (+4..); `obj->+0x13C (m_nObjectType) = 5`; virtual `SetIsStatic(false)` (vtable +0x10);
`obj->+0x40 |= 0x2000000` (bAttachedToEntity); `CWorld::Add(obj)` 0x563220; `m_pRopeAttachObject (+0x318) = 0`;
`m_nFlags1 (+0x326) = 0`. `__thiscall`, plain `ret`, SEH frame.
