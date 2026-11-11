# CPedToPlayerConversations (AI) - RE-SPEC-AI-2 (lead)

Static state (gr PedToPlayerConversations.h): m_State 0x969A20 (u32: 0 INACTIVE, 1 PEDHASOPENED, 2 WAITINGFORFINALWORD, 3 WAITINGTOFINISH),
m_pPed 0x9691C0 (registered ref), m_Topic 0x9691BC (i32), m_TimeOfLastPlayerConversation 0x9691B4, m_StartTime 0x9691B8, m_bPositiveOpening
0x9691B1 (u8). Two more statics used only here, no gr name: 0x969A3C i32 pool scan index (proposed `ms_nScanIndex`) and 0x969A40 CVehicle*
registered ref to the last vehicle the player was in (proposed `ms_pLastPlayerVehicle`).

## 0x43B0F0 CPedToPlayerConversations::Update  (catalogue: specified/REA, port level 9) - RE-SPEC-AI-2 (lead)
- Convention: `__cdecl void ()`, SEH frame (only for the stack CEventScriptCommand and task allocations). EBP = player = FindPlayerPed(-1)
  for the whole function, EBX = 0, ESI = 3 until a ped pointer is loaded into it.
- Callers: CGame::Process 0x53BEE0. Callees (addresses): FindPlayerPed 0x56E210, FindPlayerVehicle 0x56E0D0 (args -1, 0),
  CEntity::RegisterReference 0x571B70, CPlayerPed::GetWantedLevel 0x41BE60, CConversations::IsConversationGoingOn 0x43AAC0,
  CPed::PedIsReadyForConversation 0x43ABA0 (arg 1), CPool<CPed>::GetAt 0x4082A0 (thunk into .HOODLUM 0x1566860: `flags[i] < 0 ? 0 :
  objects + i*0x7C4`, `ret 4`), CEntity::GetIsOnScreen 0x534540, VectorSub 0x40FE60, CPlaceable::GetTopDirection 0x41CCB0,
  CVector::DotProduct 0x40FDB0, CWorld::FindPlayerCoors 0x56E010, CVector::Magnitude 0x4082C0, CGeneral::GetRandomNumberInRange(int,int)
  0x407180 (original formula `min + ftol((rand() & 0xFFFF) / 32768 * (max - min))`, i.e. max exclusive; see cat14 finding 1),
  ped speech entity (ped +0x294) virtual slot 5 (+0x14) = CAEPedSpeechAudioEntity::WillPedChatAboutTopic(topic),
  CAEPedSpeechAudioEntity::RequestPlayerConversation 0x4E38C0 (cdecl, ped) / ReleasePlayerConversation 0x4E3960, CStats::GetStatValue
  0x558E40 (ST0), `_ftol` 0x821B40, CPedClothesDesc::HasVisibleNewHairCut 0x5A7970 / HasVisibleTattoo 0x5A79D0 (ECX = player
  m_pPlayerData(+0x480)->m_pPedClothesDesc(+4)), CPed::Say 0x5EFFE0, CPed::DisablePedSpeech 0x5EFF60 / EnablePedSpeech 0x5EFF70 /
  GetPedTalking 0x5EFF50, IKChainManager_c::LookAt 0x618970 / IsLooking 0x6181A0 / AbortLookAt 0x618280 (ECX = g_ikChainMan 0xC15448),
  `rand` 0x821B1E, CTask::operator new 0x61A5A0, CTaskComplexSequence ctor 0x632BD0 / AddTask 0x632D10, CTaskSimpleStandStill ctor
  0x62F310, CTaskComplexKillPedOnFoot ctor 0x620E30, CEventScriptCommand ctor 0x4B0A00 / dtor 0x4B0A50, CEventGroup::Add 0x4AB420,
  0x43A6A0 (thunk into .HOODLUM 0x15697D0: `pedType == 22 || pedType == 5`; proposed name `IsFemalePedType(ePedType)`),
  CPedToPlayerConversations::EndConversation 0x43AB10.
- Globals also read: CPools::ms_pPedPool 0xB74490 (size at +8), CModelInfo::ms_modelInfoPtrs 0xA9B0C8, CWeather::Wind 0xC812F0,
  CClock::ms_nGameClockHours 0xB70153, CTimer::m_snTimeInMilliseconds 0xB7CB84 ("now").
- "Say(p)" below = `Say(p, 0, 1.0f, 1, 0, 0)` (overrideSilence = 1) unless stated; "Say0(p)" = `Say(p, 0, 1.0f, 0, 0, 0)`.
  "OPEN(pos, p)" = if RequestPlayerConversation(ped) { m_bPositiveOpening = pos; ped->Say(p); START } else SKIP.
  "AGGRO(target ped)" = seq = new(0x40) CTaskComplexSequence(); seq->AddTask(new(0x20) CTaskSimpleStandStill(2000, 0, 0, 8.0f));
  seq->AddTask(new(0x38) CTaskComplexKillPedOnFoot(player, -1, 0, 0, 0, 0)); stack CEventScriptCommand ev(3, seq, 0);
  m_pPed->m_pIntelligence(+0x47C)->eventGroup(+0x68).Add(&ev, false); ~ev (a failed allocation passes a null task to AddTask).
- Behaviour:
  0. (0x43B10E-0x43B13F) If FindPlayerVehicle(-1, 0): [0x969A40] = it; it->RegisterReference(0x969A40) (the old ref is not cleared).
     Switch m_State (jump table 0x43C10C; > 3 -> return):
  1. State 0 (0x43B15E): return unless now > m_TimeOfLastPlayerConversation + 30000 (unsigned), FindPlayerVehicle(-1,0) == 0,
     player GetWantedLevel() == 0, !IsConversationGoingOn(), player->PedIsReadyForConversation(1). Then up to 8 iterations
     (counter at a local, 0x43B967-0x43B973): idx = ++[0x969A3C]; if idx >= pool size: idx = 0 (0x43B1D4-0x43B1E8); ped = GetAt(idx).
     A ped is a candidate when (each test in this order, any failure = next iteration): ped != 0; ped->+0x484 m_nCreatedBy == 1;
     ped->PedIsReadyForConversation(1); !(ped->+0x474 & 0x800000); ped->GetIsOnScreen(); DotProduct(ped->GetTopDirection(), playerPos
     - pedPos) > 0.0f [0x858B50]; DotProduct(player->GetTopDirection(), pedPos - playerPos) > 0.0f [0x858B50]; |pedPos -
     FindPlayerCoors(-1)| < 7.0f [0x858F48] (Magnitude, `test ah,5; jp` -> NaN fails) (0x43B202-0x43B326).
     Topic: pedType (+0x598) == 7 or 9..16 -> m_Topic = GetRandomNumberInRange(8, 10) (8 or 9); else GetRandomNumberInRange(0, 7) (0..6;
     topic 7 is therefore unreachable) (0x43B32C-0x43B358). If !ped->speech.WillPedChatAboutTopic(m_Topic) -> next iteration.
     Topic switch (table 0x43C11C; topic > 9 -> SKIP):
     - 0 (0x43B380): v = [0x969A40]; SKIP if v == 0 or 2D |vPos - pedPos| (x*x + y*y, fsqrt) >= 20.0f [0x858BA4] (NaN skips). c =
       (i8)ms_modelInfoPtrs[v->m_nModelIndex]->+0x4D; class table: c-1 in 0..8 via byte table 0x43C150 = {0,1,1,0,0,0,2,2,1}, else SKIP:
       0 -> OPEN(0, 0x30); 1 -> if v->m_fHealth (+0x4C0) > 800.0f [0x8594DC] OPEN(1, 0x39) else SKIP; 2 -> SKIP.
     - 1 (0x43B47C): s = ftol(GetStatValue(0x42) + (ped->+0x20 u16 & 0x1FF) - 256.0f [0x858FB4]) (x87: fiadd int then fsub);
       s > 500 -> OPEN(1, 0x3A); s < 150 -> OPEN(0, 0x31); else SKIP.
     - 2 (0x43B516): s as topic 1; SKIP unless player clothes HasVisibleNewHairCut(0); s > 500 -> OPEN(1, 0x3B); else if s < 150 or
       GetStatValue(0x1F) < 5.0f [0x858C80] (`test ah,5; jp`) -> OPEN(0, 0x32); else SKIP.
     - 3 (0x43B5E9): a = ftol(GetStatValue(0x17) + (u8)ped->+0x20 - 128.0f [0x858BF4]); b = ftol(GetStatValue(0x15) + (u8)ped->+0x20 -
       128.0f [0x858BF4]); if a > 400 and b < 250 -> OPEN(1, 0x3C); else if (a - b) < 200 and b > 500 -> OPEN(0, 0x33); else SKIP.
     - 4 (0x43B6BE): s = ftol(GetStatValue(0x42) + (ped->+0x20 & 0x3FF) - 512.0f [0x8594D8]); > 700 -> OPEN(1, 0x3D); < 300 ->
       OPEN(0, 0x34); else SKIP.
     - 5 (0x43B758): s = ftol(GetStatValue(0x19) + (ped->+0x20 & 0x1FF) - 256.0f [0x858FB4]); > 700 -> OPEN(1, 0x3E); < 300 ->
       OPEN(0, 0x35); else SKIP.
     - 6 (0x43B7F1): SKIP unless player HasVisibleTattoo(); r = GetRandomNumberInRange(0, 1000); > 700 -> OPEN(1, 0x3F); < 300 ->
       OPEN(0, 0x36); else SKIP.
     - 7 (0x43B890, unreachable): if Wind < 0.1f [0x858B1C]: if 6 < hours < 20 OPEN(1, 0x40) else SKIP; else if Wind > 0.5f [0x858B8C]
       OPEN(0, 0x37) else SKIP.
     - 8 (0x43B923): if RequestPlayerConversation(ped): ped->Say(0xFB); m_bPositiveOpening = 1; START. 9 (0x43B941): same with 0xFC.
     SKIP (0x43B94E): if (rand() & 0xFFF) == 3: ped->Say0(0x58) and return (0x43BA21); else next iteration.
     START (0x43B99B -> disguised `push 0x9691C0` at 0x401A1D (0x265809D6 + 0xDA3E87E9 + CF 1) -> 0x43B9A0): m_State = 1; m_pPed = ped;
     ped->RegisterReference(&m_pPed); m_TimeOfLastPlayerConversation = m_StartTime = now; ped->DisablePedSpeech(0);
     LookAt("Ped2Pl_Conversation" [0x8594C4], ped, player, 100000, 5, nullptr, 0, 0.25f (imm), 500, 8, 0);
     LookAt("Ped2Pl_ConversationP" [0x8594AC], player, ped, 100000, 5, nullptr, 0, 0.25f, 500, 8, 0); return.
  2. State 1 (0x43BA38): if m_pPed == 0: NULLPED = {m_State = 0; ReleasePlayerConversation(); if IsLooking(player) AbortLookAt(player, 250)};
     return. If now > m_StartTime + 4000: m_State = 0; ReleasePlayerConversation(); m_pPed->EnablePedSpeech(); abort the player's look
     (if looking); if IsLooking(m_pPed): AbortLookAt(m_pPed, 250) and, if m_bPositiveOpening, m_pPed->Say0(0x38) (the Say is inside the
     IsLooking branch, 0x43BAD4); return.
     YES = GetPad(0)->ConversationYesJustDown():
     - positive opening: topic table 0x43C15C: 0..7 -> player->Say(0x81, 0x82, 0x86, 0x89, 0x8A, 0x8B, 0x8C, 0xEA resp.), then m_State = 3,
       m_StartTime = now. 8 -> player->Say(0xEC); if GetRandomNumberInRange(0, 100) < 75: AGGRO; EndConversation(). 9 -> player->Say(0x6B);
       same 75% AGGRO; EndConversation(). Topic > 9 -> m_State = 3, m_StartTime = now (no Say).
     - negative opening: player->Say(IsFemalePedType(m_pPed->pedType) ? 0x83 : 0x84); m_State = 3; m_StartTime = now.
     else NO = GetPad(0)->ConversationNoJustDown():
     - positive opening: topic 8 -> player->Say(0xEB), EndConversation(); topic 9 -> player->Say(0x6A), EndConversation(); other ->
       player->Say(female ? 0x83 : 0x84), m_State = 2, m_StartTime = now.
     - negative opening: player->Say(female ? 0x87 : 0x88); if m_pPed->pedType != 8 and GetRandomNumberInRange(0, 100) < 40: AGGRO;
       m_State = 3; m_StartTime = now.
  3. State 2 (0x43C086): m_pPed == 0 -> NULLPED, return. If now > m_StartTime + 3000 and !m_pPed->GetPedTalking(): m_pPed->Say(0x85);
     m_State = 3; m_StartTime = now.
  4. State 3 (0x43C0CE): if now > m_StartTime + 2500 or m_pPed == 0: EndConversation().
- Edge cases: the aggressive sequence targets the player and is pushed as a script-command event on m_pPed (primary task slot 3).
  In state 1 the AGGRO paths end the conversation immediately; the 25%/60% non-aggro paths of topics 8/9 also end it.
- gta-reversed: plugin::Call (PedToPlayerConversations.cpp:27), not reversed.
- Open questions: meaning of ped +0x474 bit 0x800000 (0x43B220) and of vehicle model-info byte +0x4D (0x43B3EA, a vehicle class used
  only here); stat ids 0x15/0x17/0x19/0x1F/0x42 named only by number.
