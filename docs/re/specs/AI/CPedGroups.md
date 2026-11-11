# CPedGroups (AI) - RE-SPEC-3 group I2
Statics: `ms_groups[8]` CPedGroup (0x2D4 bytes) at 0xC09920, `ms_activeGroups[8]` u8 at 0xC098E0, `ScriptReferenceIndex[8]` u16 at
0xC098D0. CPedGroup: +8 CPedGroupMembership (+4 = 8 member slots CPed*, index 7 = leader, i.e. group+0xC..+0x28), +0x30
CPedGroupIntelligence, +0x2D0 m_bIsMissionGroup. Several entries are obfuscated with a `jmp` to a .text stub that loads the
first group pointer (0x401881) - no behavioural effect.

## 0x5F7E80 CPedGroups::GetPedsGroup  (catalogue: specified/REA, port level 0)
- `__cdecl CPedGroup* (const CPed* ped)`. For g = 0..7: if `ms_activeGroups[g]` and ped != 0: if ped equals any of member slots 0..6
  (group+0xC..+0x24, ascending) or the leader slot (group+0x28) -> return `&ms_groups[g]`. Return 0. (ped null -> 0 after the loop.)

## 0x5F7EE0 CPedGroups::GetGroupId  (catalogue: specified/REA, port level 0)
- `__cdecl int32 (const CPedGroup* g)`: index i (0..7) with `&ms_groups[i] == g`, else -1 (`or eax,-1`). (EDI is used but not saved -
  the compiler relies on it being untouched: `mov edi,edi` no-op only.)

## 0x5F7F10 CPedGroups::IsInPlayersGroup  (catalogue: specified/REA, port level 0)
- `__cdecl bool (const CPed* ped)`: if `ped->+0x480` (m_pPlayerData) != 0 return 0; return 1 if ped equals any of group 0's member
  slots 0..6 (0xC0992C..0xC09944) or its leader slot [0xC09948]; else 0. ms_activeGroups[0] is NOT checked.

## 0x5F7F40 CPedGroups::AreInSameGroup  (catalogue: specified/REA, port level 1)
- `__cdecl bool (const CPed* a, const CPed* b)`: 0 if either is null. For each active group g: if a is in member slots 0..6 or the
  leader slot and `ms_groups[g].m_groupMembership.IsMember(b)` 0x5F6A10 -> return 1. Return 0.

## 0x5FB800 CPedGroups::AddGroup  (catalogue: specified/REA, port level 5)
- `__cdecl int32 ()`: first g in 0..7 with `!ms_activeGroups[g]`; none -> -1. Set `ms_activeGroups[g] = 1`; for slot 0..7 with a
  non-null member: `membership.RemoveMember(slot)` 0x5F80D0; `m_groupIntelligence.Flush()` 0x5F7350; `+0x2D0 = 0`; return g.

## 0x5FB8C0 CPedGroups::Init  (catalogue: specified/REA, port level 5)
- `__cdecl void ()`: for g = 0..7: if active: `ms_activeGroups[g] = 0`, remove every non-null member slot (0..7) via RemoveMember(slot),
  intelligence Flush, `+0x2D0 = 0`; then (for every g) `ScriptReferenceIndex[g] = 1`.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5F7E30 CPedGroups::RegisterKillByPlayer  (catalogue: specified/REA, port level 0)
- `__cdecl void ()`: if `ms_bIsPlayerOnAMission` [0xC098E8] (u8) -> `ms_iNoOfPlayerKills` [0xC098EC]++ (0x5F7E39).

## 0x5F7E40 CPedGroups::IsGroupLeader  (catalogue: specified/REA, port level 0)
- `__cdecl bool (CPed* ped [esp+4])`, `ret`: for group i = 0..7 (pointer 0xC09948 = ms_groups 0xC09920 + 0x28 = leader slot, stride 0x2D4, until 0xC0AFE8): if
  `ms_activeGroups[i]` [0xC098E0 + i] and ped != 0 and leader == ped -> AL = 1 (0x5F7E73). Else AL = 0. (ped == 0 always gives 0.)

## 0x5FB8A0 CPedGroups::RemoveAllFollowersFromGroup  (catalogue: specified/REA, port level 3)
- `__cdecl void (int32 groupId [esp+4])`: if `ms_activeGroups[groupId]` [0xC098E0] -> tail-jump `CPedGroup::RemoveAllFollowers(&ms_groups[groupId])` 0x5FB7D0 (ECX =
  0xC09920 + id*0x2D4, 0x5FB8AE-0x5FB8BA). No bounds check.

## 0x5FB930 CPedGroups::CleanUpForShutDown  (catalogue: specified/REA, port level 5)
- `__cdecl void ()`: for each of the 8 groups (membership pointer starts at 0xC09928 = ms_groups+8, stride 0x2D4): for i = 0..7, if `membership.m_members[i] (+4+4i) != 0`
  -> `membership.RemoveMember(i)` 0x5F80D0 (0x5FB946-0x5FB95B); then `(membership+0x28 = group+0x30).Flush()` 0x5F7350 (CPedGroupIntelligence); then byte group+0x2D0 = 0
  (membership+0x2C8, 0x5FB969). ms_activeGroups is not touched.
