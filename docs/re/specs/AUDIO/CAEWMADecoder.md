# CAEWMADecoder (AUDIO) - full specs (RE-SPEC-3, group U1)

## 0x502C40 CAEWMADecoder::~CAEWMADecoder (scalar deleting destructor)  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall CAEWMADecoder* (this = ECX, u32 flags [esp+4])`, returns this in EAX, `ret 4`. No direct callers
  (vtable slot 0 of CAEWMADecoder).
- Behaviour: call CAEWMADecoder::Destructor 0x502760 (thiscall, this); if `flags & 1`: `_free(this)` 0x8214BD (cdecl, caller pops 4).
  Return this.
- gta-reversed: compiler-generated (virtual dtor). Open questions: none.
