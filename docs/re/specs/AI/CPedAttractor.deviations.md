# CPedAttractor (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x5EB530 CPedAttractor::IsAtHeadOfQueue  (gr_deviations = 1)
- Attractors/PedAttractor.cpp:149 `return !m_ArrivedPeds.empty() && m_ArrivedPeds.front() == ped; // NOTE/BUGFIX: Check if the array is empty`:
  Original (`__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`): `return *(this->[+0x20]) == ped` - loads the queue's data pointer at +0x20
  (m_ArrivedPeds storage) and dereferences its first element WITHOUT any size/empty test (0x5EB530-0x5EB53D, `sete`). An empty queue with a null
  data pointer crashes; an empty queue with retained storage compares a stale element.
- Verdict: CONFIRMED (gr adds the empty check). Class: keep-original (port reads element 0 unconditionally; a null data pointer must not be silently
  treated as false - record the divergence if the port has to guard it).
