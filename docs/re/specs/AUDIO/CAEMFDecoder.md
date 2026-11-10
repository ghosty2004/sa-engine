# CAEMFDecoder (AUDIO) - full specs (RE-SPEC-3, group U1)

## 0x4E7C50 CAEMFDecoder::~CAEMFDecoder (scalar deleting destructor)  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall CAEMFDecoder* (this = ECX, u32 flags [esp+4])`, returns this in EAX, `ret 4`. No direct callers
  (vtable slot 0 of CAEMFDecoder).
- Behaviour: call CAEMFDecoder::Destructor 0x4E77C0 (thiscall, this); if `flags & 1`: `_free(this)` 0x8214BD (cdecl, caller pops 4).
  Return this.
- gta-reversed: compiler-generated (virtual dtor). Open questions: none.
