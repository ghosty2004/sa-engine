# CConversations (AI) - RE-SPEC-3 group I4

Globals: m_SettingUpConversationNumNodes 0x9691C8 (i32), m_SettingUpConversationPed 0x9691CC, m_SettingUpConversation 0x9691D0 (u8),
m_Conversations 0x9691D8 (14 x CConversationForPed, stride 0x1C, m_pPed at +8), m_aTempNodes 0x969360 (stride 0x2C: +0x00 name[8],
+0x08 nameYes[8], +0x10 nameNo[8], +0x18 i32 finalNodeIndex, +0x1C i16 tempYes, +0x1E i16 tempNo, +0x20/+0x24/+0x28 three dwords
(speech ids)), m_Nodes 0x969570 (50 x CConversationNode, stride 0x18: +0 name[8], +8 i16 nodeYes, +0xA i16 nodeNo, +0xC/+0x10/+0x14 dwords).

## 0x43ADB0 CConversations::DoneSettingUpConversation  (catalogue: specified/REA, port level 2)
- `__cdecl void (bool suppressSubtitles [esp+4])`, plain `ret`. Callers: CRunningScript::ProcessCommands1800To1899 0x46D050,
  ProcessCommands2600To2699 0x479DA0. Callees: CEntity::RegisterReference 0x571B70 only (string compares/copies are inlined).
- Behaviour (n = m_SettingUpConversationNumNodes, re-read from the global at the start of every inner pass):
  1. Link resolution (0x43ADBD-0x43AE6B), for i = 0..n-1: `temp[i].tempNo = -1; temp[i].tempYes = -1`; for j = 0..n-1 (ascending):
     if `strcmp(temp[i].nameYes, temp[j].name) == 0` -> `tempYes = j`; if `strcmp(temp[i].nameNo, temp[j].name) == 0` -> `tempNo = j`
     (inline byte compare; the LAST matching j wins).
  2. Free conversation (0x43AE71-0x43AE92, 0x43AF28): first k in 0..13 with `m_Conversations[k].m_pPed == 0`; conv = &m_Conversations[k].
     If none: conv = 0 (NOT checked: step 5 then writes through a null pointer - original bug).
  3. Node allocation (0x43AEA0-0x43AED0), for i = 0..n-1: first node m in 0..49 with `m_Nodes[m].name[0] == 0`; reserve it with
     name = "X" (0x58, 0); `temp[i].final = m`. If no node is free: `temp[i].final = 0` and nothing is reserved.
  4. Copy (0x43AED2-0x43AFA4), for i = 0..n-1 (node = &m_Nodes[temp[i].final]): `strcpy(node.name, temp[i].name)` (byte loop incl. the
     terminator, no length limit); `node.nodeYes = tempYes >= 0 ? (int16)temp[tempYes].final : -1`; `node.nodeNo = tempNo >= 0 ?
     (int16)temp[tempNo].final : -1` (signed int16 tests, 0x43AF0D / 0x43AF61); node +0xC/+0x10/+0x14 = temp +0x20/+0x24/+0x28.
  5. Conversation (0x43AFAE-0x43AFE5): `conv.m_FirstNode = conv.m_CurrentNode = temp[0].final` (read even when n == 0);
     `conv.m_pPed = m_SettingUpConversationPed`; `m_pPed->RegisterReference(&conv.m_pPed)` (no null check); `m_LastChange =
     m_snTimeInMilliseconds`; `m_LastTimeWeWereCloseEnough = 0`; `m_Enabled = 1`; `m_SuppressSubtitles = suppressSubtitles`; `m_Status = 0`.
  6. `m_SettingUpConversationNumNodes = 0; m_SettingUpConversation = 0` (0x43AFEB-0x43AFF0).
- gta-reversed: not reversed (plugin call / absent): port from these bytes. Open questions: none.
