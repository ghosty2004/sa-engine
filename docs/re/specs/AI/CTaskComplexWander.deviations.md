# CTaskComplexWander (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x673D00 CTaskComplexWander::GetWanderTaskByPedType  (gr_deviations = 1)
- TaskComplexWander.cpp:330 `uint8 randomDir = CGeneral::GetRandomNumberInRange(0, 8);` (drawn once before the switch):
  Original (cdecl, SEH): the direction is drawn INSIDE each case, after `CTask::operator new` succeeded: cop 0x673D5B-0x673D6C
  `CTaskComplexWanderCop(p, 4, GetRandomNumberInRange(0, 8))` 0x460C80 (2 args); medic 0x673DA1-0x673DB4 `CTaskComplexWanderMedic(p, 4, r, 1)` 0x658770;
  criminal 0x673DE9-0x673DFC `CTaskComplexWanderCriminal(p, 4, r, 1)` 0x48E610; prostitute 0x673E31-0x673E44 0x672690; default 0x673E5B-0x673E9F:
  `r = _ftol((rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 8.0f [0x859000])` (inlined) and `CTaskComplexWanderStandard(p, 4, r, 1)` 0x48E4F0.
  When the pool is full no rand() is consumed and 0 is returned (0x673EB4).
- Verdict: CONFIRMED (helper formula and rand consumption order/count). Class: keep-original.
