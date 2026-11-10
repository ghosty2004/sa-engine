# CTaskManager (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

CTaskManager layout: +0 m_aPrimaryTasks[5], +0x14 m_aSecondaryTasks[6], +0x2C m_pPed.

## 0x681740 CTaskManager::FindActiveTaskByType  (gr_deviations = 1)
- Tasks/TaskManager.cpp:94 `// NOTE: Original code doesn't break first match, but that's by a bug, not intentional.`:
  Original (`__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`, result EBX -> EAX):
  1. Active primary = first non-null of m_aPrimaryTasks[0..4] (0x681750-0x68175A). Walk t = active, t = t->GetSubTask() (vtbl+8): at the loop top
     `if (found) return found` (0x681765-0x681767); `if (t->GetTaskType() (vtbl+0x10) == taskType) found = t`; then `t = t->GetSubTask()` (called once more
     even on the matching task). So the OUTERMOST match in the active chain wins.
  2. If nothing found: for each of the 6 secondary slots in order (+0x14.., counter 6, 0x68178A-0x6817BC): walk the chain while `t != 0 && found == 0`
     with the same test; after a match the remaining slots are still iterated but nothing is tested (`test ebx,ebx; jne` at 0x68179A).
- Verdict: REFUTED as a behavioural difference: the original also returns the first (outermost, lowest-slot) match; it only keeps looping without effect
  and makes one extra GetSubTask call on the matched task (no side effects). Class: harmless.

## 0x681810 CTaskManager::GetTaskSecondary  (gr_deviations = 1)
- Tasks/TaskManager.cpp:110 `if (notsa::IsFixBugs() && taskIndex == TASK_SECONDARY_INVALID) return nullptr;`:
  Original (`__thiscall CTask* (this, int idx [esp+4])`, `ret 4`): `return *(this + 0x14 + 4*idx)` with no range check (0x681810-0x681814).
  idx = -1 therefore returns m_aPrimaryTasks[4] (+0x10, the default primary task).
- Verdict: CONFIRMED (only when gr's runtime fix-bugs flag is on). Class: keep-original.
