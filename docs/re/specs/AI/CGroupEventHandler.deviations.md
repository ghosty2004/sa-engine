# CGroupEventHandler (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Source: `re/gta-reversed/source/game_sa/Events/GroupEventHandler.cpp` (pin 20d8d2a). RNG reference: original
`CGeneral::GetRandomNumberInRange(int min, int max)` 0x407180 = `min + _ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * (max - min))`;
gr's helper is `(int)lerp(min, max-1, rand()/32767.0f)` (docs/re/cat14-gr-deviations.md finding 1). Group member array: CPedGroup + 0xC, 8 slots.

## 0x5F9BD0 CGroupEventHandler::ComputeStareResponse  (gr_deviations = 1)
- GroupEventHandler.cpp:145 `timeout + CGeneral::GetRandomNumberInRange(-timeoutBias, timeoutBias)`:
  Original: after the 64.0f [0x859A44] squared-distance gate (0x5F9C46-0x5F9C57), loop over the 8 member slots (counter 8 at 0x5F9C6A);
  for each non-null member (0x5F9C72-0x5F9C76) the range helper is INLINED: `neg = -bias` (0x5F9C7C-0x5F9C7E); `r = rand() & 0xFFFF`;
  `fild r; fmul 3.0517578e-05f [0x858B14]` then `fild (bias - neg)` and `fmul st(1)` -> `_ftol((2*bias) * (r*2^-15))` (operand order: the span is
  ST0, 0x5F9C8E-0x5F9CA6); `time = trunc + neg + timeout` (0x5F9CB5-0x5F9CB7) passed to the CTaskComplexStareAtPed ctor. One rand() per non-null member.
- Verdict: CONFIRMED (gr helper formula differs; same rand() consumption: one per member). Class: keep-original.

## 0x5FA290 CGroupEventHandler::ComputeDoDealResponse  (gr_deviations = 2)
- GroupEventHandler.cpp:675 `const auto partnerType = CGeneral::GetRandomNumberInRange(0, 6)` (+ `partnerType + 1` passed to CEventCreatePartnerTask):
  Original: NO random number is drawn here. CTaskComplexPartnerDeal ctor 0x684190 gets (tag 0x86C828, dealWith, 1, 0, CVector{0,0,0})
  (0x5FA3D9-0x5FA41F); the partner event ctor 0x5F6190 is called with pushes `0, 0, closest, 0` -> (type 0, closest, false, 0.0f)
  (0x5FA473-0x5FA47E). So the original passes 0 as the first argument and consumes no rand() at this point.
  Verdict: CONFIRMED (gr consumes an extra rand() and passes 1..6 instead of 0). Class: keep-original.
- GroupEventHandler.cpp:700 `CGeneral::GetRandomNumberInRange(3000, 6000)` (LookAt duration): Original 0x5FA4BD-0x5FA4C7: `GetRandomNumberInRange(0xBB8, 0x1770)`
  0x407180 per member slot (8, 0x5FA4AA) that is non-null and != closest; then `g_ikChainMan.LookAt(0x86C828 tag, m, closest, time, 5, 0, 1, 0.25f, 500, 3, 0)`
  0x618970 (0x5FA4CF-0x5FA4F0). Verdict: CONFIRMED (helper formula). Class: keep-original.

## 0x5FA550 CGroupEventHandler::ComputeGreetResponse  (gr_deviations = 2)
- GroupEventHandler.cpp:620 `partnerType = GetRandomNumberInRange(0, 6)`: Original 0x5FA69A-0x5FA69E `r = GetRandomNumberInRange(0, 6)` 0x407180 (cdecl args left
  on the stack and reused as the upper two dwords of the by-value CVector{0,0,0} argument, 0x5FA6A3-0x5FA6D7); CTaskComplexPartnerGreet ctor 0x684210
  gets (tag 0x86C838, toGreet, 1, 0, r, CVector{0,0,0}); CEventCreatePartnerTask ctor 0x5F6190 gets (r + 1, closest, 0, 0.0f) (`inc esi` at 0x5FA73C).
  Verdict: CONFIRMED (helper formula only; structure matches). Class: keep-original.
- GroupEventHandler.cpp:646 LookAt `GetRandomNumberInRange(3000, 6000)`: Original 0x5FA78A-0x5FA794, same LookAt argument list as DoDeal with tag 0x86C838
  (0x5FA79C-0x5FA7BD). Verdict: CONFIRMED. Class: keep-original.

## 0x5FB390 CGroupEventHandler::ComputeResponseSexyPed  (gr_deviations = 1)
- GroupEventHandler.cpp:194 `ComputeStareResponse(pg, e.m_SexyPed, originator, GetRandomNumberInRange(3000, 5000), 1000)`: Original 0x5FB43E-0x5FB448
  `GetRandomNumberInRange(0xBB8, 0x1388)` 0x407180, then calls ComputeStareResponse(EDI, ESI, [esp+0x24], r, 1000) (cdecl, gr order pg, sexyPed, originator)
  (0x5FB451-0x5FB459). Verdict: CONFIRMED (helper formula). Class: keep-original.
