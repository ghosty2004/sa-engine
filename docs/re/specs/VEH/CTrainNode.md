# CTrainNode (VEH) - full specs (writer group E)

Layout (gr `TrainNode.h`, size 0xA): +0 x i16, +2 y i16, +4 z i16 (world * 8), +6 m_nDistanceFromStart u16
(world * 3), +8 m_nSurfaceLighting, +9 m_bSurfLightingFound.

## 0x6F5440 CTrainNode::GetPosn  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CVector* (const CTrainNode* this = ECX, CVector* ret [esp+4])` (RVO), returns `ret` in
  EAX, `ret 4`.
- Callers: CTrain::ProcessControl 0x6F86A0 (4 sites).
- Behaviour: `ret->x = (float)x * 0.125f; ret->y = (float)y * 0.125f; ret->z = (float)z * 0.125f` (0x858C48 = 0.125f;
  sign-extended int16 -> `fild` dword; computed z first, then y, then x on the x87 stack, stored x, y, z).
