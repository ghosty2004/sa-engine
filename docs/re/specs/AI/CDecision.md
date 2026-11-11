# CDecision (AI) - RE-SPEC-3 group I4

Layout (gr Decision.h, size 0x3C): +0x00 m_Tasks[6] (i32 eTaskType), +0x18 m_Probs[6][4] (u8, per relationship
0..3), +0x30 m_Bools[6][2] (bool, per decision type).

## 0x6040D0 CDecision::MakeDecision  (catalogue: specified/REA, port level 1)
- `__thiscall void (CDecision* this, int32 relationship [esp+4], bool type [esp+8] (byte), int32 excl1 [esp+0xC], int32 excl2
  [esp+0x10], int32 excl3 [esp+0x14], int32 forcedTaskType [esp+0x18], int16* outTaskType [esp+0x1C], int16* outFacial
  [esp+0x20])`, `ret 0x20`. (Offsets are on entry; in the body they are +0x70.) gr's commented prototype
  `MakeDecision(int32, bool, int32, int32, int32, int32, int16&, int16&)` matches the argument count.
- Callers: 0x606060 (unnamed), CDecisionMakerTypes::MakeDecision (ped) 0x606E70, (group) 0x606F80. Callees: CDecisionSimple::
  MakeDecision 0x6007A0.
- Locals: `simple` = a CDecisionSimple on the stack (body offset +0x3C: tasks, +0x54: chances, +0x6C: count);
  `candTask[6]` (+0x24), `candProb[6]` u8 (+0x14), `candIdx[6]` u8 (+0x1C, written but never read).
- Behaviour:
  1. Init (0x6040DF-0x60410C): for i in 0..5 `simple.tasks[i] = 200`, `simple.chances[i] = 0` (bit pattern 0); `candProb[0..5] = 0`
     (dword + word store); `candTask[0..5] = 200` (rep stosd). candIdx is not initialised.
  2. Filter (i = 0..5, 0x604119-0x604173), n = 0: `p = m_Probs[i][relationship]` (byte at this + 0x18 + 4*i + relationship; the
     relationship index is not range-checked); skip if p == 0 (0x604134); `t = m_Tasks[i]`; skip if t == -1 (0x604139), t == excl1
     (0x60413E), t == excl2 (0x604144), t == excl3 (0x60414D) (in this order); skip if `m_Bools[i][type != 0]` == 0 (byte at this +
     0x30 + 2*i + (type ? 1 : 0), 0x604156). Otherwise `candTask[n] = t; candProb[n] = p; candIdx[n] = (u8)i; n++`.
  3. `*outTaskType = 200` (0x604181), unconditionally.
  4. If n <= 0 (signed, 0x604187): `*outFacial = -1` (0xFFFF, 0x604212), return.
  5. Build (0x60418D-0x6041C0): `simple.count = 6` (ALWAYS 6, not n); `acc = 0.0f [0x858B50]` (x87 st0); for i = 0..5:
     `simple.tasks[i] = candTask[i]`; `acc += (int32)candProb[i]` (`fiadd` of the zero-extended byte); `simple.chances[i] = acc`
     (`fst`, f32 store; acc stays in st0). Unused slots i >= n keep task 200 and add 0, so their chance equals the total.
  6. Normalise (0x6041C2-0x6041E7): `inv = 1.0f [0x858624] / acc` (`fdivr`: memory / st0); for i = 0..5:
     `simple.chances[i] = inv * simple.chances[i]` (f32 store each). Then pop inv.
  7. `CDecisionSimple::MakeDecision(&simple, forcedTaskType, outTaskType, &localIndex)` 0x6007A0 (0x6041F4); the chosen index is
     discarded (localIndex is never read).
  8. `*outFacial = -1` (0xFFFF, 0x604200) - unconditionally, also after a successful decision.
- Edge cases / oddities: outFacial is always -1 (the original never selects a facial response here); because count is 6 and the
  unused slots have chance 1.0 with task 200, a random value never falls past the last slot (but rounding of inv*acc may leave the
  last used slot's chance slightly below 1.0, in which case the draw falls into slot n with task 200 = "no response").
  With forcedTaskType != -1 that is among the candidates, that candidate is chosen (step 2 of 0x6007A0).
- gta-reversed: body commented out (`//RH_ScopedInstall(MakeDecision, 0x6040D0, { .Reversed = false })`, Decision.cpp:14/54):
  port from these bytes. Open questions: none.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x600600 CDecision::Add  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, int32 taskType [esp+4], const float* probs [esp+8] (4 floats), const int32* flags [esp+0xC] (2 ints))`, `ret 0xC`.
- Layout: +0x00 int32 task[6], +0x18 u8 prob[6][4], +0x30 u8 friendFlag[6][2].
- Behaviour: for slot i = 0..5 (0x600610-0x6006A2): the slot is free iff `task[i] == -1` and all four `prob[i][j] == 0` (unsigned byte, `ja`) and both
  `friendFlag[i][0..1] == 0`; the FIRST free slot is filled (0x600641-0x600691): `task[i] = taskType`; `prob[i][j] = (u8)_ftol(probs[j])` for j = 0..3
  (`_ftol` 0x821B40 truncation, low byte stored); `friendFlag[i][0] = flags[0] != 0`, `[i][1] = flags[1] != 0`. No free slot -> nothing. (`cmp esi,-1` at 0x60063C is dead.)

## 0x600710 CDecision::HasResponse  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`, `ret`: returns AL = 1 if any of the six dwords at this+0..+0x14 is != 200 (0xC8) (0x600714-0x600724), else AL = 0 (upper EAX bits = loop index).
