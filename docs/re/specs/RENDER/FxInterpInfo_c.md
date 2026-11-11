# FxInterpInfo_c (RENDER) - RE-SPEC-FINAL group A

## 0x4A8D10 FxInterpInfo_c::scalar deleting destructor  (catalogue: unknown/REA, port level 1)
- Convention: `__thiscall FxInterpInfo_c* (this, uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `vtable = 0x85A968` (FxInterpInfo_c); if `flags & 1` CRT `operator delete(this)` 0x8214BD. No other members are freed here.
- gta-reversed: compiler-generated - matches. Open questions: none.
