# CTaskComplexBeInGroup (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x6330B0 CTaskComplexBeInGroup::MonitorSecondaryGroupTask  (gr_deviations = 1)
- Tasks/TaskTypes/TaskComplexBeInGroup.cpp:56 `#ifdef FIX_BUGS pedGrpSecTask = slot != -1 ? GetTaskSecondary(slot) : nullptr #else GetTaskSecondary(slot)`:
  Original (`__thiscall void (this, CPed* ped [esp+4])`, `ret 4`): `gi = &CPedGroups::ms_groups[m_nGroupId (+0xC)].intelligence` (`imul 0x2D4`, + 0xC09950);
  `grp = gi->GetTaskSecondary(ped)` 0x5F8620; `slot = gi->GetTaskSecondarySlot(ped)` 0x5F8650; `pedTask = ped->m_pIntelligence->m_TaskMgr.GetTaskSecondary(slot)`
  0x681810 called UNCONDITIONALLY (0x6330CA-0x6330EA) - with slot == -1 it returns the default primary task (see CTaskManager.deviations.md).
  Rest (matches gr's #else branch): if `m_SecondaryTask (+0x20) != 0 && grp == m_SecondaryTask && pedTask == 0`: `gi->ReportFinishedTask(ped, m_SecondaryTask)`
  0x5F86F0, `+0x20 = 0`, `+0x24 = -1` (0x633100-0x633117). If `grp != m_SecondaryTask`: if `GetTaskSecondary(+0x24)` != 0 (called twice, 0x633134/0x63314A)
  `->MakeAbortable(ped, 0, 0)`; then if `pedTask == 0 || pedTask->MakeAbortable(ped, 1, 0)`: `+0x20 = grp; +0x24 = slot`; if grp != 0
  `SetTaskSecondary(grp->Clone(), slot)` 0x681B60 (0x633171-0x633192).
- Verdict: CONFIRMED (FIX_BUGS build differs for slot -1; the original is the #else branch). Class: keep-original.
