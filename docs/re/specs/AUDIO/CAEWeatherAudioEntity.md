# CAEWeatherAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

CAEAudioEntity base: +0 vptr, +4 m_Entity, +8 CAESound m_tempSound (m_PhysicalEntity at +0x10). Size 0x80; +0x7C u8
m_nThunderFrequencyVariationCounter (gr AEWeatherAudioEntity.h).

## 0x72A620 CAEWeatherAudioEntity::CAEWeatherAudioEntity  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAEWeatherAudioEntity* (this = ECX)`, returns this, plain `ret`. Caller:
  `__init_CWeather__m_WeatherAudioEntity` 0x855510.
- Behaviour: `+0x10 = 0` (inlined CAESound default ctor), `+0 = 0x872A74` (vtable), `+0x7C = 0` (u8) (0x72A624-0x72A62D).
- gta-reversed: same effect for the static instance. Open questions: none.
