# CConversationForPed (AI) - RE-SPEC-3 group I4

Layout (gr Conversations/ConversationForPed.h, size 0x1C): +0x0 m_FirstNode, +0x4 m_CurrentNode, +0x8 m_pPed, +0xC m_LastChange,
+0x10 m_LastTimeWeWereCloseEnough, +0x14 m_Status, +0x18 m_Enabled (u8), +0x19 m_SuppressSubtitles (u8).

## 0x43AC40 CConversationForPed::IsPlayerInPositionForConversation  (catalogue: specified/REA, port level 3)
- `__thiscall bool (this, bool randomConversation [esp+4])`, `ret 4`, AL. Callers: CConversations::IsPlayerInPositionForConversation
  0x43B0B0, CConversationForPed::Update 0x43C190. Callees: CWorld::FindPlayerCoors 0x56E010 (out, -1), CWorld::FindPlayerPed 0x56E210 (-1,
  called 4-5 times), CPedGroupMembership::CountMembersExcludingLeader 0x5F6AA0, CPed::PedIsReadyForConversation 0x43ABA0.
- Behaviour (P = player, p = m_pPed):
  1. `d = FindPlayerCoors() - p.pos` (p position from matrix +0x30 or placement +4; components x, y, z); `dist = sqrt((dz*dz + dy*dy) + dx*dx)`
     (fsqrt; 0x43AC63-0x43AC86). Return 0 if `dist > 4.0f [0x858B90]` (`test ah,0x41; jne` continues on <=, == and NaN).
  2. `dot = (Pf.z*pf.z + Pf.y*pf.y) + Pf.x*pf.x` with Pf = player matrix forward (+0x10), pf = p->m_matrix forward (no null checks)
     (0x43ACA5-0x43ACCE). Return 0 if `dot > 0.0f [0x858B50]` (they must face each other; NaN continues).
  3. `s = sqrt(P.moveSpeed.y^2 + P.moveSpeed.x^2)` (+0x48 then +0x44: computed y*y + x*x... order: `fld y; fld x; x*x; y*y; add` ->
     `x*x + y*y` with x*x first) (0x43ACDD-0x43ACFA). Return 0 if `s > 0.01f [0x858C58]`.
  4. If `p[+0x764] == FindPlayerPed(-1)` and `m_snTimeInMilliseconds < p[+0x768] + 6000` (unsigned) return 0 (0x43AD0D-0x43AD37).
  5. If randomConversation (byte): `grp = CPedGroups::ms_groups[P->m_pPlayerData (+0x480)->m_nPlayerGroup (+0x38)]` (stride 0x2D4; membership
     at 0xC09928 = ms_groups + 8); if `grp.membership.CountMembersExcludingLeader() >= 1` return 0 (0x43AD3D-0x43AD6D).
  6. Return `P->PedIsReadyForConversation(randomConversation) && p->PedIsReadyForConversation(randomConversation)` (player first; second
     call skipped if the first fails) (0x43AD7A-0x43AD9D).
- Edge cases: step 3 order detail: the stack holds y then x; `fld st(0)` squares x first; result `x*x + y*y`.
- gta-reversed: ConversationForPed.cpp:28 `plugin::CallMethodAndReturn` (`.Reversed = false`). Open questions: names of ped +0x764/+0x768
  (last ped talked to / time).
