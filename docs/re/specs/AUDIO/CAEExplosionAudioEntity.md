# CAEExplosionAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

CAEAudioEntity base (gr AEAudioEntity.h, size 0x7C): +0 vptr, +4 CEntity* m_Entity, +8 CAESound m_tempSound (0x74; its
+8 m_PhysicalEntity is at entity+0x10). CAEExplosionAudioEntity size 0x80: +0x7C gr `uint32 m_Speed`.

## 0x5074B0 CAEExplosionAudioEntity::CAEExplosionAudioEntity  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAEExplosionAudioEntity* (this = ECX)`, returns this, plain `ret`. Caller:
  `__init_CExplosion__m_ExplosionAudioEntity` 0x8559E0.
- Behaviour: `+0x10 = 0` (the inlined CAESound default ctor: m_tempSound.m_PhysicalEntity = 0, cf. 0x446FE0); `+0 = 0x862E60`
  (vtable); `+0x7C = 0` as a BYTE (0x5074BD). Nothing else is written (m_Entity +4 and the rest of m_tempSound stay as
  the static storage holds them - zero in .bss).
- gta-reversed: default member initialisers (`m_Entity{}`, full `CAESound{}`, `uint32 m_Speed{}`) write more fields; for a
  static object in .bss the result is identical. Only the low byte of +0x7C is written by the original. Open questions: none.
