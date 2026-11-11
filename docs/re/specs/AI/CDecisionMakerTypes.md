# CDecisionMakerTypes (AI) - RE-SPEC-3 group I2

Layout (gr DecisionMakerTypes.h, confirmed by the offsets in the bytes): +0 m_NoOfDecisionMakers i32, +4 m_DecisionMakers[20]
(CDecisionMaker = 41 CDecision of 0x3C = 0x99C bytes each), +0xC034 m_EventIndices[96] i32, +0xC1B4 m_DefaultRandomPedDecisionMaker,
+0xCB50 m_DefaultMissionPedDecisionMaker, +0xD4EC m_DefaultPlayerPedDecisionMaker, +0xDE88 m_DefaultRandomPedGroupDecisionMaker,
+0xE824 m_DefaultMissionPedGroupDecisionMaker; size 0xF1C0. Statics: `m_IsActive[20]` u8 0xC0B01C, a parallel u8[20] at 0xC0AFE0
(stored from AddDecisionMaker's `type` arg; 0 = ped decision maker, non-zero = group decision maker - see RemoveDecisionMaker),
instance pointer 0xC0B030. Ped: +0x47C intelligence (+0xB4 ped decision-maker type, +0xB8 group decision-maker type), +0x484 created-by
(2 = mission), +0x59C m_pStats (+0x32 i8 default decision maker). Group: +0x2C8 group decision-maker type, +0x2D0 m_bIsMissionGroup.
`0x606060` = CDecisionMaker::MakeDecision(eventIdx, ...) (see _unattributed-I1.md).

## 0x4684F0 CDecisionMakerTypes::GetInstance  (catalogue: specified/REA, port level 3)
- `__cdecl CDecisionMakerTypes* ()`, SEH prologue obfuscated (0x4021ED, no effect).
- If `[0xC0B030]` != 0 return it. Else `p = operator new(0xF1C0)` 0x82119A; if p: `[0xC0B030] = 0x4650F0(p)` (ctor, thiscall) and
  return it; else store 0 and return 0. gr: `.Reversed = false`.

## 0x600840 CDecisionMakerTypes::LoadEventIndices  (catalogue: specified/REA, port level 2)
- `__thiscall void ()`: `0x5BB9F0(this + 0xC034, "PedEvent.txt" 0x86CD44)` (stdcall helper ReadEventIndexFile, see _unattributed-I1.md).

## 0x6043A0 CDecisionMakerTypes::RemoveDecisionMaker  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, int32 dm [esp+4])`, `ret 4`.
- If `!m_IsActive[dm]` return. If `[0xC0AFE0 + dm] == 0` (ped decision maker): walk `CPools::ms_pPedPool` [0xB74490] from the last slot
  down to slot 0 (stride 0x7C4; a slot is skipped when its byteMap byte has bit 7 set): if `ped->m_pIntelligence->+0xB4 == dm` call
  `CPedIntelligence::SetPedDecisionMakerType((i8)ped->m_pStats->+0x32)` 0x600B50 (0x6043C5-0x604419). Else (group decision maker):
  for the 8 groups `CPedGroups::ms_groups` (group intelligence at 0xC09950 + i*0x2D4): if `CPedGroups::ms_activeGroups[i]` (u8 0xC098E0)
  and `intel->+0x298 == dm`: `CPedGroupIntelligence::SetGroupDecisionMakerType(-1)` 0x5F7340.
  Then (both paths) `m_IsActive[dm] = 0`, `[0xC0AFE0 + dm] = 0`, `m_NoOfDecisionMakers--`. gr: `.Reversed = false`.

## 0x606E70 CDecisionMakerTypes::MakeDecision (ped)  (catalogue: specified/REA, port level 3)
- `__thiscall void (this, CPed* ped, int32 eventType, int32 srcType, bool inVeh, int32 t1, int32 t2, int32 t3, int32 t4,
  bool useGroupDM, int16* outTask, int16* outFacial)` (11 stack args), `ret 0x2C`.
- `dm = useGroupDM ? intel->+0xB8 : intel->+0xB4`; `idx = m_EventIndices[eventType]` (+0xC034, no bounds check); `*outTask = 200`
  (0xC8, TASK_NONE); `*outFacial = -1`.
- dm == -2: `CDecision::MakeDecision(this + 0xD4EC + idx*0x3C, srcType, inVeh, t1, t2, t3, t4, outTask, outFacial)` 0x6040D0 (player default).
- dm == -1: `ped->+0x484 == 2` ? `CDecisionMaker::MakeDecision(this+0xCB50, idx, srcType, ..., outFacial)` 0x606060 (mission default) :
  same on `this+0xC1B4` (random default).
- else: `CDecision::MakeDecision(this + 4 + (dm*0x29 + idx)*0x3C, srcType, inVeh, t1..t4, outTask, outFacial)`.
- gr: `.Reversed = false` (plugin::Call).

## 0x606F80 CDecisionMakerTypes::MakeDecision (group)  (catalogue: specified/REA, port level 2)
- `__thiscall int32 (this, CPedGroup* g, int32 eventType, int32 srcType, bool inVeh, int32 t1, t2, t3, t4)`, `ret 0x20`.
- `idx = m_EventIndices[eventType]`; a local int16 task = 200 is kept in the group arg slot; the facial output pointer passed is the
  address of the t4 arg slot (its value is overwritten, ignored). `dm = g->+0x2C8`:
  - dm == -1: `g->m_bIsMissionGroup == 1` ? decision at `this + 0xE824 + idx*0x3C` : `this + 0xDE88 + idx*0x3C`;
  - else (including -2): `this + 4 + (dm*0x29 + idx)*0x3C`.
  `CDecision::MakeDecision(decision, srcType, inVeh, t1, t2, t3, t4, &task, &t4slot)`; return `(i32)(i16)task` (movsx).
- gr: plugin::Call.

## 0x607050 CDecisionMakerTypes::AddDecisionMaker  (catalogue: specified/REA, port level 2)
- `__thiscall int32 (this, CDecisionMaker* src [esp+4], int32 type [esp+8], bool forMission [esp+0xC])`, `ret 0xC`.
- Slot range: forMission ? [15, 20) : [0, 15) (the mission start value 15 is loaded through an obfuscated jump to 0x406A7C).
  First i in range with `!m_IsActive[i]`; if none return -1. Then `m_IsActive[i] = 1`; `[0xC0AFE0 + i] = (u8)type`; for k = 0..40:
  `CDecision::From(&m_DecisionMakers[i].decisions[k], &src->decisions[k])` 0x6006B0 (thiscall on the destination);
  `m_NoOfDecisionMakers++`; return i.
- gr: `.Reversed = false`.

## 0x604490 CDecisionMakerTypes::FlushDecisionMakerEventResponse  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, int32 dm [esp+4], int32 eventType [esp+8])`, `ret 8`. `k = ((int32*)(this + 0xC034))[eventType]` (event -> decision
  index table); `CDecision::SetDefault(this + 4 + 0x3C*(k + 0x29*dm))` 0x600530 (decision makers of 0x29 decisions x 0x3C bytes, starting
  at +4). No bounds checks. Caller: CRunningScript::ProcessCommands1800To1899.

## 0x6044C0 CDecisionMakerTypes::AddEventResponse  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, int32 dm [esp+4], int32 eventType [esp+8], int32 taskId [esp+0xC], float* probs [esp+0x10], uint8* flags
  [esp+0x14])`, `ret 0x14`. `k = ((int32*)(this + 0xC034))[eventType]`; `CDecision::Add(this + 4 + 0x3C*(k + 0x29*dm), taskId, probs,
  flags)` 0x600600 (thiscall, the three trailing arguments forwarded unchanged). No bounds checks.
