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

## 0x43C190 CConversationForPed::Update  (catalogue: specified/REA, port level 8) - RE-SPEC-AI-2 (lead)
- Convention: `__thiscall void ()` (this = ECX: CConversationForPed*, kept in ESI), plain `ret`. 0x43C190 is a 5-byte stub; body in
  `.HOODLUM` 0x15668B0-0x1566D41 including the range 0x1566AF2-0x1566C4A that the bundle missed (disassembled for this spec).
- Callers: CConversations::Update 0x43C590 (1). Callees: IsPlayerInPositionForConversation 0x43AC40, CPad::GetPad 0x53FB70,
  CPad::ConversationNoJustDown 0x541200, CPad::ConversationYesJustDown 0x5411D0 (reached only through the disguised call below),
  CMessages::ClearSmallMessagesOnly 0x69DD30, CText::Get 0x6A0050 (ECX = TheText 0xC1B340), CMessages::AddMessageJump 0x69F1E0,
  CMessages::AddMessageQ 0x69F0B0, CWorld::FindPlayerPed 0x56E210 (arg -1), CPed::Say 0x5EFFE0, CConversations::AwkwardSay 0x43A810.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84 (R, u32), CConversations::m_Nodes 0x969570 (R; CConversationNode stride 0x18:
  +0 char m_Name[8], +8 i16 m_NodeYes (0x969578), +0xA i16 m_NodeNo (0x96957A), +0xC i32 m_Speech (0x96957C), +0x10 i32 m_SpeechY
  (0x969580), +0x14 i32 m_SpeechN (0x969584)), gString 0xB71670 (W, char buffer), TheText 0xC1B340, two 2-byte .rdata strings:
  0x8594E0 = "Y\0" (u16 0x0059), 0x8594E4 = "N\0" (u16 0x004E).
- Fields of this (gr ConversationForPed.h, size 0x1C): +4 m_CurrentNode i32, +8 m_pPed CPed*, +0xC m_LastChange u32,
  +0x10 m_LastTimeWeWereCloseEnough u32, +0x14 m_Status u32 (0 INACTIVE, 1, 2, 3; see behaviour), +0x18 m_Enabled u8,
  +0x19 m_SuppressSubtitles u8. Node index i = m_CurrentNode, node = m_Nodes[i] (address 0x969570 + 0x18*i, computed as `lea (i*3)*8`).
- Obfuscation (no behavioural effect in an unmodified exe; the port uses the resolved values):
  - "push K" sequences `push r; pushfd; mov r,[A]; xor/add/adc/sbb r,[B]; popfd; xchg [esp],r` compute a constant from a .text dword and
    a .HOODLUM dword: 0x15668C9-0x15668D8 pushes 0 (the arg of IsPlayerInPositionForConversation = false); 0x1566973-0x1566982 pushes 0
    (GetPad(0)); 0x1566A24-0x1566A34 pushes 0 (last Say arg); 0x1566B71-0x1566B80 pushes 3 (AddMessageJump flag); 0x1566CC1-0x1566CD1
    pushes 0x3F800000 (= 1.0f, Say probability); 0x1566C1F-0x1566C2D loads EDX = [0xB7CB84] (address computed as 0xF01D146D - 0xEF6548E9).
  - Disguised call 0x1566AD3: `call 0x1566AD8` pushes 0x1566AD8, the target computes 0x5411D0 into the stack slot and adds 0x1A to the
    pushed return address, then `ret`: equivalent to `call CPad::ConversationYesJustDown` (ECX = GetPad(0) result) returning to 0x1566AF2.
  - 0x1566B2D-0x1566B4F: `call eax` into 0xCB73E0 (no-CD .text) which calls an expression-tree evaluator through [0x1422EA0] with a
    descriptor at 0x155FF8C and adds 0x28 if the trap flag is set; its result is used as the strcpy destination (EDX). Not evaluated
    statically; the destination is taken to be gString 0xB71670 because the following strlen (0x1566B5A) and the suffix store (0x1566B88)
    operate on 0xB71670, exactly as the mirror path at 0x15669CA. UNCLEAR only in the sense that the evaluator was not traced.
- Behaviour (t = CTimer::m_snTimeInMilliseconds, re-read at each use; all time compares unsigned):
  1. If m_pPed == 0 or m_Enabled == 0 -> m_Status = 0, return (0x15668B3-0x15668C3, 0x1566D39).
  2. If !IsPlayerInPositionForConversation(false) -> m_Status = 0, return (0x15668DB-0x15668E2).
  3. If t > m_LastTimeWeWereCloseEnough + 10000 (0x15668E8-0x15668F9, `ja`) -> START.
  4. If m_Status == 1 (0x15668FF): if t > m_LastChange + 4000 -> START (0x1566905-0x1566910); else if GetPad(0)->ConversationNoJustDown()
     -> START (0x1566922); else if GetPad(0)->ConversationYesJustDown() -> START (0x156693B). (Pressing either button while status 1
     skips the wait.)
  5. If t <= m_LastChange + 400 (0x1566948-0x1566958, `jbe`) -> COMMON.
  6. If m_Status is not 0, 2 or 3 -> COMMON (0x156695E-0x156696D).
  7. NO (0x1566985): if GetPad(0)->ConversationNoJustDown() and node.m_NodeNo >= 0 (signed i16 compare 0x15669A2):
     a. if !m_SuppressSubtitles: ClearSmallMessagesOnly(); strcpy(gString, node.m_Name) (byte loop 0x15669D0); append "N" (u16 store of
        [0x8594E4] at the terminator, 0x1566A02); AddMessageJump(TheText.Get(gString), 4000, 3, 1) (0x15669EF-0x1566A0B).
     b. if node.m_SpeechN > 0 (signed): FindPlayerPed(-1)->Say((u16)m_SpeechN, 0, 1.0f, 0, 0, 0) (0x1566A19-0x1566A52).
     c. if node.m_SpeechN < 0: AwkwardSay(-m_SpeechN, FindPlayerPed(-1)) (0x1566A5D-0x1566A80) (node re-read from m_CurrentNode).
     d. m_CurrentNode = (i32)(i16)node.m_NodeNo (0x1566A8E); m_LastChange = t (0x1566A9E).
     e. if !m_SuppressSubtitles: AddMessageQ(TheText.Get(m_Nodes[new].m_Name), 4000, 1, 1) (0x1566AAC-0x1566AC2, 0x1566C4B-0x1566C56).
     f. m_Status = 1 (0x1566C5E) -> COMMON.
     If No was pressed but m_NodeNo < 0, control falls to YES (0x15669AB `jl 0x1566AC7`).
  8. YES (0x1566AC7): if !GetPad(0)->ConversationYesJustDown() -> COMMON (0x1566AF4); if node.m_NodeYes < 0 -> COMMON (0x1566B00).
     Same as NO with: suffix "Y" [0x8594E0] (0x1566B68/0x1566B88), AddMessageJump(.., 4000, 3, 1) (0x1566B6F-0x1566B9B), Say/AwkwardSay
     with m_SpeechY (+0x10) by FindPlayerPed(-1) (0x1566BA9-0x1566C01), m_CurrentNode = (i16)m_NodeYes (0x1566C0F/0x1566C1C),
     m_LastChange = t (0x1566C30), AddMessageQ(new node name, 4000, 1, 1) if !suppress (0x1566C35-0x1566C56), m_Status = 1.
  9. START (0x1566C6A): if !m_SuppressSubtitles: ClearSmallMessagesOnly(); AddMessageJump(TheText.Get(node.m_Name), 4000, 1, 1)
     (0x1566C71-0x1566C98). If node.m_Speech > 0: m_pPed->Say((u16)m_Speech, 0, 1.0f, 0, 0, 0) (0x1566CA6-0x1566CDA); if < 0:
     AwkwardSay(-m_Speech, m_pPed) (0x1566CE5-0x1566CF7). m_Status = 2; m_LastChange = t (0x1566CFF-0x1566D0C).
  10. COMMON (0x1566D0F): m_LastTimeWeWereCloseEnough = t; if m_Status == 2 and t > m_LastChange + 4000 -> m_Status = 3 (0x1566D17-0x1566D30).
- Edge cases: the suffix store writes 2 bytes at strlen(gString) (copies the NUL too). A node name of 8 chars without NUL would run into
  m_NodeYes - original behaviour. Status 0 reached with a valid ped enters the input step (6) as well, so a button press while INACTIVE
  (but in position) can advance the tree; START is only reached via the 10 s / 4 s timers or step 4.
- gta-reversed: plugin::CallMethod (ConversationForPed.cpp:23), not reversed; enum names PLAYER_SPEAKING/PED_SPEAKING/WAITINGFORINPUT
  = 1/2/3 match the use above (1 after the player answered, 2 ped speaking, 3 waiting for input).
- Open questions: the expression-tree result at 0x1566B4A (see obfuscation) was not evaluated.
