# CTaskComplexChat: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x682E20 CTaskComplexChat::CreateFirstSubTask  (gr_deviations = 3)
Original bytes (`__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH; +0xC m_IsChatter u8, +0x10 m_ChatPartner, +0x18 int16
m_GlobalSpeechContext): return 0 if m_ChatPartner == 0 (0x682E38). If !m_IsChatter: `new(0x20) CTaskSimpleStandStill(999999, 0, 0, 8.0f)`
0x62F310 (0x682FFA-0x683024). Else: if `(int16)ctx > -1` (signed `cmp ax,0xFFFF; jle`, 0x682E53) `ped->Say(ctx, 0, 1.0f, 0, 0, 0)`
0x5EFFE0; `g_InterestingEvents.Add(1, ped)` 0x602590; if ctx == 0xA4 (164) or 0xA6 < ctx <= 0xA8 (167, 168) (0x682E7F-0x682E96):
`r = CGeneral::GetRandomNumberInRange(0, 3)` 0x407180 (0x682ED9): r == 0 -> `new(0x20) CTaskSimpleRunAnim(0, 0xA6, 4.0f, 0)` 0x61A8B0,
1 -> anim 0xA7, 2 -> anim 0xA8, anything else -> `new(0x34) CTaskSimpleChat(4000)` (0x682FC1-0x682FE2, dead because 0x407180(0,3)
returns 0..2). Otherwise `new(0x34) CTaskSimpleChat(999999)` (0x682E98-0x682EBD). Every allocation failure returns 0.
Row TaskComplexChat.cpp:71 (fix-bugs: `GetRandomNumberInRange(0, notsa::IsFixBugs() ? 4 : 3)`):
- Original: range (0, 3) only (push 3 / push 0 at 0x682ED5).
- Verdict: CONFIRMED (gr's FixBugs build reaches the Chat(4000) case). Class: keep-original.
Row TaskComplexChat.cpp:71 (rng-helper):
- Original: calls the original helper 0x407180; gr's inline formula differs (cat14 finding 1).
- Verdict: CONFIRMED. Class: keep-original.
Row TaskComplexChat.cpp:75 (`case 3: ... // NOTE: (FIXED-BUG) Originally unreachable`):
- Original: the Chat(4000) branch exists (0x682FC1) and is unreachable as gr says.
- Verdict: CONFIRMED as a deviation only through the FixBugs range above; the comment itself is accurate. Class: keep-original.
Minor (no row): the Say test is `(int16)ctx > -1` in the original vs gr's `!= CTX_GLOBAL_UNK (-1)`; they differ only for ctx < -1. Harmless.
