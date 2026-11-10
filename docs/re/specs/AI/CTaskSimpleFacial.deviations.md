# CTaskSimpleFacial (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x692E80 CTaskSimpleFacial::ProcessPed  (gr_deviations = 2)
- TaskSimpleFacial.cpp:58 `if (GetRandomNumberInRange(0, 100) < 40)`: Original (anim id == 0xA0 talk, 0x692EB6) 0x692EBE-0x692EE4:
  `_ftol((rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 100.0f [0x858628]) < 40` (signed `jge` skips). Verdict: CONFIRMED (formula). Class: keep-original.
- TaskSimpleFacial.cpp:59 `anim->SetSpeed(GetRandomNumberInRange(0.5f, 3.0f))`: Original 0x692EE6-0x692EFF:
  `fild rand() (NOT masked); fmul 3.0518509e-05f [0x858C7C]; fmul 2.5f [0x858FA0]; fadd 0.5f [0x858B8C]` -> speed. Verdict: CONFIRMED (gr lerp
  `3*t + 0.5*(1-t)` rounds differently). Class: keep-original.
