# CPedGroupIntelligence (AI) - RE-SPEC-3 group I4

Layout (gr PedGroupIntelligence.h, size 0x2A0): +0x00 m_pPedGroup, +0x04 m_CurrentEvent, +0x08 m_HighestPriorityEvent,
+0x0C m_PedTaskPairs[8], +0xAC m_SecondaryPedTaskPairs[8], +0x14C m_ScriptCommandPedTaskPairs[8], +0x1EC m_DefaultPedTaskPairs[8].
CPedTaskPair (0x14): +0 Ped, +4 Task, +8 Slot (i32, invalid = -1), +0xC MatchID, +0x10 UsedTask (u8).
CPedGroup +0x8 = CPedGroupMembership (+0 m_group, +4 m_members[8]), so group+0xC = members[0].

## 0x5F7800 CPedGroupIntelligence::ComputeScriptCommandTasks  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`, frame 0xA8 bytes. Callers: CPedGroupIntelligence::Process 0x5FC4A0. Callees: virtual
  scalar deleting dtors only.
- Purpose: re-index the script-command task pairs so that pair i belongs to group member i.
- Behaviour:
  1. Local `tmp[8]` CPedTaskPair initialised to {Ped 0, Task 0, Slot -1, MatchID 0, UsedTask 0} (0x5F7820-0x5F7839).
  2. For i = 0..7 (0x5F7850-0x5F78AB), `m = group->members[i]` (`[m_pPedGroup + 0xC + 4*i]`), `s = &m_ScriptCommandPedTaskPairs[i]`:
     `tmp[i].Ped = s->Ped; tmp[i].Task = s->Task` (0x5F7859-0x5F785C); if `m != s->Ped`: `tmp[i].Ped = m` (0x5F7868);
     search j = 0..7 for the first `m_ScriptCommandPedTaskPairs[j].Ped == m` (0x5F7871-0x5F787C): `tmp[i].Task =` found ?
     `pairs[j].Task` : 0 (0x5F7880-0x5F7893). Note: when m == 0 the search matches the first pair whose Ped is 0 (its Task is
     taken). Slot/MatchID/UsedTask of tmp are never copied.
  3. For k = 0..7 (0x5F78AD-0x5F78F8), `s = &m_ScriptCommandPedTaskPairs[k]`: if `s->Task != 0`: search j = 0..7 for the first
     `tmp[j].Ped == s->Ped` (0x5F78C7-0x5F78D2); if found and `tmp[j].Task != 0` keep it (0x5F78DD); otherwise (not found, or
     found with Task 0) `s->Task->vtbl[0](1)` (delete), `s->Ped = 0; s->Task = 0` (0x5F78E1-0x5F78EE). The check is by Ped, not by
     task pointer: a task kept here may still be referenced by a different tmp entry (no double-ownership check).
  4. Copy back (0x5F78FA-0x5F79A5): for i = 0..7 `m_ScriptCommandPedTaskPairs[i].Ped = tmp[i].Ped`, `.Task = tmp[i].Task`
     (Slot, MatchID and UsedTask of the member pairs are left unchanged).
- gta-reversed: PedGroupIntelligence.cpp:442 `plugin::CallMethod<0x5F7800>` (`.Reversed = false`): port from these bytes.
  Open questions: none.
