# CPatrolRoute (AI) - RE-SPEC-FINAL (helper B)

## 0x6677A0 CPatrolRoute::~CPatrolRoute  (catalogue: unknown/REA, port level 0)
- `__thiscall void (this: CPatrolRoute*)`, plain `ret`. Callers: 0x66BA70 and the atexit destructor `__dtor_g_c18db8` 0x856950 (static instance at 0xC18DB8).
- Behaviour: `eh_vector_destructor_iterator(this + 4, 0x28, 8, 0x46A380)` 0x821E02 (stdcall-style helper; destroys 8 elements of size 0x28 starting at +4 with the element dtor 0x46A380; 0x6677AD).
