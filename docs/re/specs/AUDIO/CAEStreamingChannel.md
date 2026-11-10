# CAEStreamingChannel (AUDIO) - full specs (RE-SPEC-3, group U1)

## 0x4F27F0 CAEStreamingChannel::~CAEStreamingChannel (scalar deleting destructor)  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CAEStreamingChannel* (this = ECX, u32 flags [esp+4])`, returns this, `ret 4`. No direct callers
  (vtable slot 0).
- Behaviour: CAEStreamingChannel::Destructor 0x4F2200 (this); if `flags & 1`: `_free(this)` 0x8214BD. Return this.
- gta-reversed: compiler-generated. Open questions: none.
