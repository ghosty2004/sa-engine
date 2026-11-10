# CAEVorbisDecoder (AUDIO) - full specs (RE-SPEC-3, group U1)

## 0x502700 CAEVorbisDecoder::~CAEVorbisDecoder (scalar deleting destructor)  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall CAEVorbisDecoder* (this = ECX, u32 flags [esp+4])`, returns this in EAX, `ret 4`. No direct callers
  (vtable slot 0 of CAEVorbisDecoder).
- Behaviour: call CAEVorbisDecoder::Destructor 0x5023F0 (thiscall, this); if `flags & 1`: `_free(this)` 0x8214BD (cdecl, caller pops 4).
  Return this.
- gta-reversed: compiler-generated (virtual dtor). Open questions: none.
