# CEventSource (AI) - RE-SPEC-3 group I4

## 0x4ABAC0 CEventSource::ComputeEventSourceType  (catalogue: specified/REA, port level 2)
- `__cdecl int32 (const CEvent* event [esp+4], const CPed* ped [esp+8])` (static), plain `ret`, result in EAX (0..3).
  Body split by the no-CD build: 0x4ABAC0-0x4ABAE4, the load at 0x403C75-0x403C7B (a .text island), back at 0x4ABAE9.
- Callers: CEventEditableResponse::ComputeResponseTaskType (ped) 0x4B56C0, ComputeResponseTaskOfType 0x4B5730,
  ComputeResponseTaskType (group) 0x4B57A0. Callees: CPedIntelligence::IsThreatenedBy 0x601C30, CPedIntelligence::IsFriendlyWith
  0x601BC0, CPed::IsPlayer 0x5DF8F0.
- Behaviour:
  1. `src = event->GetSourceEntity()` (vtbl+0x28, slot 10, 0x4ABACA). If src == 0 return 0 (0x4ABAD1 -> 0x4ABB27).
  2. If `(src->m_nType (byte +0x36) & 7) != 3` (not ENTITY_TYPE_PED) return 0 (0x4ABAD3-0x4ABADC).
  3. `intel = ped->m_pIntelligence` (+0x47C, 0x403C75). If `intel->IsThreatenedBy(*src)` return 3 (0x4ABAEC-0x4ABAF7).
  4. Else if `intel->IsFriendlyWith(*src)` return 2 (0x4ABB01-0x4ABB0C).
  5. Else if `src->IsPlayer()` return 1 (0x4ABB15-0x4ABB20). Else return 0 (EBX = 0, 0x4ABB2C).
- Return values match gr eDecisionRelationship (3 THREAT, 2 FRIEND, 1 PLAYER, 0 NEUTRAL).
- gta-reversed: EventSource.cpp:6 is `plugin::CallAndReturn` (not reversed) - port from these bytes. Open questions: none.
