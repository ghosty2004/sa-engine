# CTaskSimpleHoldEntity (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x6930F0 CTaskSimpleHoldEntity::DropEntity  (gr_deviations = 2)
- TaskSimpleHoldEntity.cpp:451 `m_vecMoveSpeed += GetRandomNumberInRange(+0.03f, 0.06f) * ped->GetForward()`: Original 0x6931EA-0x69322A:
  `m = ped->m_matrix` (+0x14, re-read, no null check); `s = ((float)rand() * 3.0518509e-05f [0x858C7C]) * 0.03f [0x858B10] + 0.03f [0x858B10]`;
  products in x87 order `s*fwd.z, s*fwd.y, s*fwd.x` (matrix +0x18, +0x14, +0x10), then `speed.x += s*fwd.x`, `.y`, `.z` stored f32 in that order.
  Verdict: CONFIRMED (helper rounding). Class: keep-original.
- TaskSimpleHoldEntity.cpp:452 `+= GetRandomNumberInRange(-0.03f, 0.03f) * ped->GetRight()`: Original 0x69322D-0x693270: `s = (rand() * 3.0518509e-05f)
  * 0.06f [0x859934] - 0.03f [0x858B10]`, times right (+8, +4, +0 order), added x, y, z. Then the up term `0.01f [0x858C58] * up` (0x693273-0x69328B).
  Verdict: CONFIRMED. Class: keep-original.
