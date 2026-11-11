# CDecisionMaker (AI) - RE-SPEC-FINAL (helper B)

## 0x4634D0 CDecisionMaker::SetDefault  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: CDecisionMaker*)`, plain `ret`. Callers: none recorded (vtable-less; reached only via code not in the corpus).
- Behaviour: `p = this`; 41 iterations (`edi = 0x29`, 0x4634D4, `dec edi; jne` 0x4634EA): `CDecision::SetDefault(p)` 0x600530 (thiscall, 0x4634E2); `p += 0x3C` (0x4634E7). I.e. `for i in 0..41 { this->m_Decisions[i].SetDefault() }` with CDecision stride 0x3C, starting at offset +0 (no header field before the array).
- gta-reversed: CDecisionMaker (Events/DecisionMaker.h) has `CDecision m_Decisions[41]`; gr's inline loop matches.
