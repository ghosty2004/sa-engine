# CInformFriendsEventQueue: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x4AC1E0 CInformFriendsEventQueue::Add  (gr_deviations = 1)
Original bytes (`__cdecl bool (CPed* ped [esp+4], CEvent* event [esp+8])`, plain `ret`, AL): table ms_informFriendsEvents 0xA9AFB8,
8 entries of {+0 CPed* ped, +4 CEvent* event, +8 u32 time} (loop end 0xA9B01C). free = -1; for i = 0..7: if `e.ped == ped`: if
`e.event->GetEventType() == event->GetEventType()` (e.event's call first; no null check of e.event) return 0 (0x4AC1F3-0x4AC20C); else if
free == -1 and `e.event == 0` free = i (0x4AC218-0x4AC222). If free == -1 return 0. `r = rand() & 0xFFFF`; `f = (float)r *
3.0517578e-05f [0x858B14]` (f32); `e.ped = ped`, if ped != 0 `ped->RegisterReference(&e.ped)`; `e.event = event`;
`e.time = (m_snTimeInMilliseconds - ftol(f * -500.0f [0x85AB78])) + 300` (= now + 300 + trunc(r*500/32768), range 300..799;
0x4AC235-0x4AC28E). Return 1.
Row InformFriendsEventQueue.cpp:53 (`m_Time = GetTimeInMS() + CGeneral::GetRandomNumberInRange(300u, 800u)`):
- Original: the inlined original helper formula (rand & 0xFFFF, 1/32768, truncation) above; gr's helper uses `lerp(300, 799, rand/32767)`.
- Verdict: CONFIRMED (different value for the same rand()). Class: keep-original.
