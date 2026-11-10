# CTaskComplexGoToCarDoorAndStandStill (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x64A5F0 CTaskComplexGoToCarDoorAndStandStill::CreateSubTask  (gr_deviations = 1)
- cpp:99 `NOTSA_UNREACHABLE();`: Original (stub `nop; jmp 0x40A1DB` obfuscated SEH prologue, body from 0x64A5F6, `ret 8`): dispatch on the type at [esp+4]:
  0x33C (0x64A6F2), 0xCA PAUSE (0x64A6B9), 0xCB STAND_STILL (0x64A677), 0x32A (0x64A636), 0x384 GO_TO_POINT (0x64A7BE), 0x389 FOLLOW_POINT_ROUTE (0x64A76D);
  0x516 TASK_FINISHED and every other value reach 0x64A759 `xor eax,eax` -> return 0 (also used when an allocation fails).
  Verdict: CONFIRMED (returns nullptr). Class: keep-original.

## 0x64A820 CTaskComplexGoToCarDoorAndStandStill::ControlSubTask  (gr_deviations = 1)
- cpp:231 `NOTSA_UNREACHABLE("SubTaskType: {}", (int)tt);`: Original: `edi = 0` (0x64A82A); sub-task type dispatch 0x64A88B-0x64A8AB (0x33C, 0xCA..0xCB, 0x32A, ...);
  unknown types jump to 0x64ABD5 which returns EDI (no write to EDI on the dispatch path; the first writes are inside cases, 0x64A8DD).
  Verdict: CONFIRMED (returns nullptr). Class: keep-original.
