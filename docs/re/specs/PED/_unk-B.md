# PED: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x60D5B0 CPlayerPed::CPlayerPed  (catalogue: specified/REA, port level 36)
- gr PlayerPed.cpp:138. Convention: `__thiscall CPlayerPed* (this = ECX, int32 playerId [esp+4], bool bGroupCreated [esp+8])`, `ret 8`, returns this; SEH frame handler 0x83ED58. Callers: CReplay::DealWithNewPedPacket, CPools::LoadPedPool, CPlayerPed::SetupPlayerPed 0x60D790.
- Behaviour, in order:
  1. `CPed::Constructor(this, 0 /*PED_TYPE_PLAYER1*/)` (0x5E8030); `vtbl = 0x86D168`; `m_pPlayerData (+0x480) = &CWorld::Players[playerId].m_PlayerData` (0xB7CD9C + playerId*0x190); `m_pPlayerData->AllocateData()` (0x571880).
  2. `CPed::SetModelIndex(0)` (0x5E4880); `SetInitialState(bGroupCreated)` (0x60CD20).
  3. `if (+0x71C) CleanUpOldReference(+0x71C, &+0x71C)`; `+0x71C = 0` (unconditional).
  4. `SetPedState(1)` (0x5E4500).
  5. `gPlayIdlesAnimBlockIndex (0xC0BC10) = CAnimManager::GetAnimationBlockIndex("playidles" at 0x86D1EC)` (0x4D3990).
  6. If `!bGroupCreated`: `pd = m_pPlayerData; pd->m_nPlayerGroup (+0x38) = CPedGroups::AddGroup()` (0x5FB800); `CPedGroups::ms_groups[g].m_groupIntelligence.SetDefaultTaskAllocatorType(5)` (0x5FBB70; ECX = 0xC09950 + g*0x2D4); `byte [0xC09BF0 + g*0x2D4] = 1` (group's player flag); `ms_groups[g].m_groupMembership.SetLeader(this)` (0x5FB9C0; ECX = 0xC09928 + g*0x2D4); `ms_groups[g].Process()` (0x5FC7E0; ECX = 0xC09920 + g*0x2D4); then clear bits 8, 9, 10 of `pd->+0x34` (three separate and-masks, re-reading pd each time).
  7. `f = CStats::GetFatAndMuscleModifier(9)` (0x559AF0, ST0); `+0x544 = f; +0x540 = f` (fst then fstp).
  8. `byte +0x72D = 15; byte +0x72E = 15`; `+0x79C = 0; +0x7A0 = 0`.
  9. `m_pedSpeech (+0x294).Initialise(this)` (CAEPedSpeechAudioEntity::Initialise 0x4E68D0).
  10. `m_pIntelligence (+0x47C)->+0xC8 = 30.0f; ->+0xC4 = 2`.
  11. `dword +0x478 = (old & ~0x200000) | ((bGroupCreated & 1) << 21)` (xor-merge, ped flags bit 21).
- gta-reversed: matches gr's sequence. Open questions: none.
