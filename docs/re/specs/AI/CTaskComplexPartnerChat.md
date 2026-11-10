# CTaskComplexPartnerChat (AI) - RE-SPEC-3 group I2
Vtable 0x8707C4, type 0x4B4. Extra fields: +0x74 u8 m_playConversation (speech), +0x75 u8 (keep task without speech), +0x76 u8
m_conversationStarted. Conversation tables (CAEPedSpeechAudioEntity): words at 0xB613F8 - 4n / 0xB613FA - 4n,
s_ConversationLength i16 [0xB613E8].

## 0x681E10 CTaskComplexPartnerChat::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x4B4.

## 0x682C60 CTaskComplexPartnerChat::MakeAbortable  (catalogue: specified/REA, port level 4)
- `r = m_pSubTask->MakeAbortable(ped, priority, event)`; if r: if `+0x74 && +0x76` { `CAEPedSpeechAudioEntity::ReleasePedConversation()`
  0x4E52A0; `+0x76 = 0` }; return 1. Else 0. `ret 0xC`.

## 0x684320 CTaskComplexPartnerChat::~CTaskComplexPartnerChat  (catalogue: specified/REA, port level 1)
- Complete dtor (SEH): vtbl = 0x8707C4; if `+0x74 && +0x76` ReleasePedConversation() (+0x76 not cleared); then 0x683A40.

## 0x684380 CTaskComplexPartnerChat::GetPartnerSequence  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* ()`, plain ret, SEH. Returns a CTaskComplexSequence.
- `seq = new(0x40) CTaskComplexSequence()` (may be 0; every AddTask below then runs with ECX = 0 - ORIGINAL BUG on pool exhaustion);
  `seq->AddTask(new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_partner, 0.5f [0x86FC7C], 0.001f))`.
- `k = _ftol((((float)((i8)m_updateDirectionCount << 4) + m_point.z) + m_point.y) + m_point.x) & 0x7F`; `f = (float)k * 0.0078125f
  [0x858B88]`; `L = 1.0f - f` (stored f32); `tA = _ftol(f * 3000.0f [0x859A94] + 3500.0f [0x8707FC])`; `tB = _ftol(L * 3000.0f + 3500.0f)`.
- If !m_leadSpeaker: `+0x74 = m_partner->intel->FindTaskByType(0x4B4)->+0x74` (no null check).
- If +0x74: `n = (i8)m_updateDirectionCount`; low 16 bits of tB := word [0xB613F8 - 4n], low 16 bits of tA := word [0xB613FA - 4n]
  (only the low halves are replaced - the upper halves of the 3500..6500 values are 0); if `3 - n == s_ConversationLength` set
  `m_updateDirectionCount = 0`. Lead: AddTask(new(0x20) CTaskComplexChat(true, m_partner, (i8)m_updateDirectionCount, tB)) 0x682CB0 then
  AddTask(CTaskComplexChat(false, partner, idx, tB)); non-lead: Chat(false, ..., tA) then Chat(true, ..., tA).
- Else: lead: AddTask(new(0x34) CTaskSimpleChat(tA)) 0x5F65A0, AddTask(new(0x20) CTaskSimpleStandStill(tB, false, false, 8.0f));
  non-lead: StandStill(tA, ...) then SimpleChat(tB).
- Return seq.

## 0x684FC0 CTaskComplexPartnerChat::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- call 0x684320; `if (flags & 1) CTask::operator delete`; return this; `ret 4`.
