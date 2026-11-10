# CPlayerRelationshipRecorder (AI) - RE-SPEC-3 group I3

Layout (gr PlayerRelationshipRecorder.h): 16 entries of 8 bytes: +0 Ped (CPed*), +4 Relationship (u8 used here).

## 0x61A150 CPlayerRelationshipRecorder::ClearRelationshipWithPlayer  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, const CPed* ped [esp+4])`, `ret 4`. Caller: CPedIntelligence dtor 0x607300.
- Behaviour: for i = 0..15 upward (`cmp eax,0x10; jl`, 0x61A15B-0x61A15F): if `this[i].Ped == ped` (0x61A156): `this[i].Ped = 0`,
  byte `this[i].+4 = 0` (only the low byte is cleared, 0x61A164-0x61A16B) and return. Only the first match is cleared.
- gta-reversed: PlayerRelationshipRecorder.cpp:72 `relationship.Flush()` on the first match - matches if Flush clears Ped and the u8.
