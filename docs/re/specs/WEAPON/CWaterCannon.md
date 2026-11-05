# CWaterCannon (WEAPON) - full specs

CWaterCannon is 0x3CC bytes (`CWaterCannons::aCannons` 0xC80740, 3 entries); m_Audio (CAEWaterCannonAudioEntity) at +0x32C.

## 0x728B10 CWaterCannon::CWaterCannon  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = CWaterCannon*, size 0x3CC), returns `this` in EAX, plain `ret`.
- Callers: only via `eh_vector_constructor_iterator` from `__init_CWaterCannons__aCannons` 0x855420.
- Behaviour (the whole ctor; the inlined audio-entity ctor reduced to three stores): `+0x33C = 0` (m_Audio
  (+0x32C).m_tempSound (+8).m_PhysicalEntity (+8)); `+0x32C = 0x872A60` (CAEWaterCannonAudioEntity vtable);
  `+0x3A8 (u8) = 0` (m_Audio.m_bInitialized). Nothing else is written (all other fields keep their .bss zero).
- gta-reversed: `= default` (C++ ctor of the member), no difference in observable state. Open questions: none.

## 0x728B30 CWaterCannon::~CWaterCannon  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall`, plain tail jump. `ecx += 0x32C; jmp CAEWaterCannonAudioEntity::~CAEWaterCannonAudioEntity`
  0x728AB0 (which sets the vtable 0x872A60, calls Terminate 0x5030A0, sets the base vtable 0x859988 and destroys the
  CAESound at +8 via 0x4EF660).
- Callers: `eh_vector_destructor_iterator` from `__dtor_CWaterCannons__aCannons` 0x856BF0 (atexit).
- gta-reversed: `= default`. Open questions: none.
