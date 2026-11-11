# INPUT: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x541D80 CPad::CPad (proposed)  (catalogue: specified/REA, port level 2)
- Name evidence: only reference is __init_CPad__Pads 0x84E1F0 (element ctor of the CPad::Pads array); gr `CPad::CPad()` (Pad.cpp:117) calls `Clear(true, true)`.
- Convention: `__thiscall CPad* (this = ECX)`, returns `this` in EAX, plain `ret`.
- Callees: CPad::Clear 0x541A70 (`__thiscall (bool, bool)`, pushes 1 then 1 at 0x541D81/0x541D83).
- Behaviour: `Clear(1, 1); return this;`. Open questions: none.

## 0x541E50 CPad::UpdatePads (thunk, proposed)  (catalogue: unknown/REA, port level 6)
- Single `jmp CPad::UpdatePads 0x541DD0` (same convention, `__cdecl void ()`); one call site at 0x5D13FD. Open questions: none.
