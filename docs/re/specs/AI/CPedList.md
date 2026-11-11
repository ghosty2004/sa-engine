# CPedList (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x699E20 CPedList::FillUpHoles  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`, `ret`: compacts the 30 slots at this+4 in order (3 unrolled blocks of 10, 0x699E30-0x699EB5): every non-null entry is copied to the next write
  position (starting at this+4); n = number of non-null entries; then slots n..29 are zeroed (`rep stosd`, 0x699EBB-0x699ECD). m_count (+0) is NOT updated.

## 0x69A340 CPedList::BuildListFromGroup_NotInCar_NoLeader  (catalogue: specified/REA, port level 2)
- `__thiscall void (this, CPedGroupMembership* m [esp+4])`, `ret 4`: m_count = 0; for i = 0..6 (leader slot 7 excluded, `cmp edi,7`): `p = m->GetMember(i)` 0x5F69B0; if p and
  `p->m_pIntelligence->IsInACarOrEnteringOne()` 0x601CC0 returns 0 (full EAX test): `GetMember(i)` is called AGAIN (0x69A36F) and stored at m_peds[m_count++] if m_count < 30.
  Then zero the unused tail (0x69A387-0x69A39B).

## 0x69A450 CPedList::RemovePedsAttackingPedType  (catalogue: specified/REA, port level 2)
- `__thiscall void (this, int32 pedType [esp+4])`, `ret 4`: for each of the first m_count entries (count read once into EBX; signed `jle` skip when <= 0): `t =
  ped->m_pIntelligence->FindTaskByType(1000)` 0x600EE0; the entry is KEPT only if t != 0 and `t+0x10` (target ped) != 0 and `target->m_nPedType == pedType`; otherwise
  entry = 0 and m_count-- (0x69A495-0x69A49B). Then `FillUpHoles()` 0x699E20. (Despite the name, peds that are NOT attacking that ped type are removed.)
