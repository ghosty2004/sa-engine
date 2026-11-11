# CPedAttractor (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x5EB530 CPedAttractor::IsAtHeadOfQueue  (gr_deviations = 1)
- Attractors/PedAttractor.cpp:149 `return !m_ArrivedPeds.empty() && m_ArrivedPeds.front() == ped; // NOTE/BUGFIX: Check if the array is empty`:
  Original (`__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`): `return *(this->[+0x20]) == ped` - loads the queue's data pointer at +0x20
  (m_ArrivedPeds storage) and dereferences its first element WITHOUT any size/empty test (0x5EB530-0x5EB53D, `sete`). An empty queue with a null
  data pointer crashes; an empty queue with retained storage compares a stale element.
- Verdict: CONFIRMED (gr adds the empty check). Class: keep-original (port reads element 0 unconditionally; a null data pointer must not be silently
  treated as false - record the divergence if the port has to guard it).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5E9600 CPedAttractor::ComputeDeltaPos  (gr_deviations = 1)
Original bytes (`__thiscall float ()`, `ret`, ST0): `min = -m_DeltaPos (+0x54)` stored f32 (`fchs; fstp`), `max = m_DeltaPos` (raw dword copy); `r = _rand()` 0x821B1E
(NOT masked); `(float)r * 3.05185094e-05f [0x858C7C]` (f32 of 1/32767), `* (max - min)`, `+ min` (0x5E9613-0x5E9630). This is an inlined random-in-range that differs from
both the original helper 0x41BD90 and gr's.
Row PedAttractor.cpp:194 (`CGeneral::GetRandomNumberInRange(-m_DeltaPos, m_DeltaPos)`):
- Verdict: CONFIRMED (gr's float helper computes `max*t + min*(1-t)` with its own t; the original computes `min + (r * k) * (max - min)` as above). Class: keep-original.

## 0x5E9640 CPedAttractor::ComputeDeltaHeading  (gr_deviations = 1)
Original bytes: identical to 0x5E9600 with m_DeltaHeading (+0x58): `t = (float)rand() * 3.05185094e-05f [0x858C7C]; return t * (max - min) + min`, min = -m_DeltaHeading
(0x5E9643-0x5E9670).
Row PedAttractor.cpp:199: Verdict CONFIRMED. Class: keep-original.
