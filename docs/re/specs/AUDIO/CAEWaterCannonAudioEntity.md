# CAEWaterCannonAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

Layout (gr AEWaterCannonAudioEntity.h, size 0xA0): CAEAudioEntity base (+0 vptr, +4 m_Entity, +8 CAESound m_tempSound),
+0x90 CAESound* m_Sounds[4]. CAESound fields: +0x14 m_Volume, +0x18 m_RollOffFactor, +0x1C m_Speed, +0x56 u16 m_Flags.
Vtables: CAEWaterCannonAudioEntity 0x872A60, CAEAudioEntity 0x859988.

## 0x502F80 CAEWaterCannonAudioEntity::UpdateGenericWaterCannonSound  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX; bool active [esp+4], i16 id [+8], i16 bankSlot [+0xC], i16 sfxId [+0x10],
  f32 speed [+0x14], f32 volume [+0x18], CVector pos [+0x1C..+0x24] by value, f32 rollOff [+0x28])`, `ret 0x28`.
  Caller: Service 0x5030D0 (4 sites). The body continues through the .text stub 0x407E77 (`movsx eax, id; jmp 0x502F8D`) -
  same behaviour inline; flags of `test al,al` (0x502F86) survive the stub.
- Callees: CAESound::Initialise 0x4EFE50, CAESoundManager::RequestNewSound 0x4EFB10 (ECX = 0xB62CB0), CAESound::SetPosition
  0x4EF880 (thiscall, CVector by value), CAESound::StopSound 0x4EF1C0.
- Behaviour: `slot = &m_Sounds[(i32)(i16)id]` (+0x90 + 4*id, no bounds check).
  - `active` and `*slot == 0`: `m_tempSound(+8).Initialise(bankSlot, sfxId, this, pos, volume, 1.0f, 1.0f, 1.0f, 0, 0, 0.0f, 0)`
    (0x502FA7-0x502FE4); then `m_tempSound.m_Volume (+0x1C) = volume`, `.m_RollOffFactor (+0x20) = rollOff`, `.m_Speed (+0x24) =
    speed`, `.m_Flags (+0x5E) = 4` (0x502FF5-0x503004); `*slot = RequestNewSound(&m_tempSound)`.
  - `active` and `*slot != 0`: `(*slot)->m_Volume = volume` (0x503017); `(*slot)->m_Speed = speed`; `(*slot)->SetPosition(pos)`.
  - `!active`: if `*slot != 0` `(*slot)->StopSound()` (pointer kept).
- gta-reversed: same results (gr passes everything through PlaySound's parameter struct). Open questions: none.

## 0x728AB0 CAEWaterCannonAudioEntity::~CAEWaterCannonAudioEntity  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this = ECX)`, plain `ret`; MSVC EH frame (handler 0x848D38). Caller: CWaterCannon::~CWaterCannon
  0x728B30 (tail).
- Behaviour: `+0 = 0x872A60`; `Terminate()` 0x5030A0 (StopSound on each non-null m_Sounds[i]); base part: `+0 = 0x859988`;
  `CAESound::~CAESound(this + 8)` 0x4EF660.
- gta-reversed: `~CAEWaterCannonAudioEntity() { Terminate(); }` - same. Open questions: none.
