# CTaskComplexFollowNodeRoute (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Fields: +0x18 m_MoveState, +0x2C m_NodeRoute (CNodeRoute*: +0 count, +4 CNodeAddress[8]), +0x30 m_PtRoute (CPointRoute*: +0 count), +0x34 m_CurrNode
(CNodeAddress {u16 area +0x34, u16 node +0x36}), +0x38 m_CurrPtIdx.

## 0x669690 CTaskComplexFollowNodeRoute::CreateSubTask  (gr_deviations = 1)
- cpp:212 `NOTSA_UNREACHABLE();`: Original (`ret 8`): `eax = 0` at 0x6696AD; dispatch 0x6696AF-0x669858 (0x387, 0xCB, 0x2C0, 0x384 below; 0x39E, 0x516 above);
  an unknown type reaches 0x66985C with EAX still 0 (0x669858 `jne 0x66985C` skips nothing that writes EAX). Verdict: CONFIRMED (returns nullptr). Class: keep-original.

## 0x66EBE0 CTaskComplexFollowNodeRoute::CalcGoToTaskType  (gr_deviations = 1)
- cpp:121 `assert(m_CurrPtIdx + 1 <= m_PtRoute->GetSize()); // Original code just clamped the value`: Original 0x66EC04-0x66EC2E: if `m_MoveState > 4` (signed):
  `idx = m_CurrPtIdx (+0x38)`; `if (idx >= m_PtRoute->count) idx = count - 1` (signed `jl`, 0x66EC19-0x66EC1D); `if (idx < 0) return 0x384` (TASK_SIMPLE_GO_TO_POINT);
  the CLAMPED local idx is then used for the current point (`lea edi,[eax+eax*2]`, 0x66EC3C). The member is not modified.
  Verdict: CONFIRMED (gr asserts and uses the unclamped member). Class: keep-original. (Slow-down test uses 0.707f [0x863C1C]; clamp to [0.0f, 1.0f] and
  `* 3.0f [0x858B3C]` for move state 7, else `* 1.5f [0x858CE8]` - consistent with gr.)

## 0x66EFA0 CTaskComplexFollowNodeRoute::ComputePathNodes  (gr_deviations = 1, plus 2 classified rows)
- cpp:318 `assert(node); // Original code checks for this too`: Original 0x66F239-0x66F24F: `if (node.area == 0xFFFF || ThePaths.m_pPathNodes[area] (0x96F854 + 4*area) == 0)` the
  position fetch 0x667310 is skipped (it does NOT skip the node). REFUTED as a separate difference (folded into the next row). harmless.
- cpp:323 `else if (notsa::IsFixBugs()) continue;`: Original never skips: with no loaded area it keeps the PREVIOUS iteration's nodePos in the same stack slot
  (uninitialised on the first iteration), still does `nodePos.z += 1.0f [0x858624]` (accumulating on the stale z, 0x66F267-0x66F277) and calls
  CanGoStraightThere 0x66EF20 (0x66F294). On success `if (i + 1 <= 8) m_NodeRoute->count = i + 1` (0x66F2AA-0x66F2B6) and stop.
  Verdict: CONFIRMED (fix-bugs path differs; also gr's per-iteration fresh nodePos differs from the original's reused slot). Class: keep-original.
- cpp:336 `notsa::indexof(m_NodeRoute->GetAll(), m_CurrNode, 0)`: Original 0x66F3ED-0x66F438: `m_CurrPtIdx = 0`; if `m_CurrNode.area != 0xFFFF`, linear search i = 0..count-1
  comparing area then node (u16 each); found -> `m_CurrPtIdx = i`, not found -> stays 0; then `if (count > 0) m_CurrNode = route[m_CurrPtIdx]`.
  Verdict: REFUTED (same result). Class: harmless.
