# CPointList (AI) - RE-SPEC-FINAL (helper B)

## 0x699F00 CPointList::Empty  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. Callers: 0x6986E0, CCover::FindCoordinatesCoverPoint 0x699570, CTaskAllocatorKillOnFoot::AllocateTasks 0x69BCA0.
- Behaviour: `this->+0 (count) = 0`; zero 6 dwords at `+0x124..+0x13B` (`rep stosd`, ECX = 6; the per-point flag bytes). The point coordinates (+4..+0x123) are not cleared.
