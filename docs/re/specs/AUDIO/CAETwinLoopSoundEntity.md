# CAETwinLoopSoundEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

Layout (gr AETwinLoopSoundEntity.h): CAEAudioEntity base (+0 vptr, +4 m_Entity, +8 CAESound m_tempSound), +0x88 u16
m_IsInUse, +0xA0 CAESound* m_Sounds[0], +0xA4 m_Sounds[1]. Vtables: CAETwinLoopSoundEntity 0x85F438, CAEAudioEntity 0x859988.

## 0x4F2930 CAETwinLoopSoundEntity::StopSound  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void (this = ECX)`, plain `ret` / tail jump. No direct callers in the catalogue (reached via the
  owners' inlined/indirect paths).
- Behaviour: if `+0xA0 != 0`: `CAESound::StopSound(+0xA0)` 0x4EF1C0; if `+0xA4 != 0`: tail `CAESound::StopSound(+0xA4)`.
  Pointers are NOT cleared and m_IsInUse is not touched.
- gta-reversed: same. Open questions: none.

## 0x4F2B80 CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this = ECX)`, plain `ret` (non-deleting). MSVC EH frame (handler 0x83C158; state 0 during the
  body, -1 before the base part). Callers: 0x506B20 (2), ~CAEPedAudioEntity 0x5DE920, CAEVehicleAudioEntity::Destructor
  0x6D0A10, `__dtor_m_sRainSoundL/R` 0x856380/0x856390 (tail).
- Behaviour: `+0 = 0x85F438`; if `(u16)+0x88 != 0` (0x4F2BA6): for `+0xA0` then `+0xA4`: if non-null
  `CAESound::StopSoundAndForget()` 0x4EF850 and store 0; then `+0x88 = 0` (u16). Then (base CAEAudioEntity dtor, inlined):
  `+0 = 0x859988`, `CAESound::~CAESound(this + 8)` 0x4EF660.
- gta-reversed: `if (m_IsInUse) StopSoundAndForget(); m_IsInUse = false` - same. Open questions: none.
