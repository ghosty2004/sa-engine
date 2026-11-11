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

## 0x64D2B0 CTaskComplexGoToCarDoorAndStandStill::CreateNextSubTask  (gr_deviations = 1)
Row TaskComplexGoToCarDoorAndStandStill.cpp:127 (`default: NOTSA_UNREACHABLE("SubTaskType: {}", tt)`):
- Original bytes: the switch on `m_pSubTask->GetTaskType()` (vfunc +0x10, 0x64D2BE) handles 0x33C, 0xCA, 0xCB, 0x32A, 0x384, 0x389; any
  other type reaches 0x64D42D and returns EDI = 0 (set at 0x64D2BC), i.e. nullptr.
- Verdict: CONFIRMED (gr aborts, the original returns nullptr = task finished).
- Class: keep-original.
Extra observation (not a catalogued row): case 0xCA (PAUSE) with `m_bTryingToEnterInWater (+0x48)` computes
`fsqrt(dx*dx + dy*dy)` of (ped pos - m_TargetPt +0x38) via VectorSub 0x40FE60 and sets `+0x49 = 1` when `dist <= 0.5f [0x858B8C]`
(`test ah,5; jp` skips for > and NaN) (0x64D33D-0x64D362); gr compares the squared 2D distance with 0.25. Equivalent except for rounding
at the boundary: port the sqrt form. Case 0x384 (GO_TO_POINT) sets +0x49 = 1 when `subtask->+0x20 & 4` and
`fabs(subtask->+0x14 - pedpos.z) <= 3.0f [0x858B3C]` (0x64D3F6-0x64D414); it never clears +0x49.
