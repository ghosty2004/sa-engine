# CData (HUD) - RE-SPEC-FINAL group A

## 0x69FFD0 CData::Unload (copy)  (catalogue: unknown/REA, port level 2)
- `__thiscall void (this)`, plain `ret`: if `this->data (+0)` { `free(data)` (CRT _free 0x8213AE); `+0 = 0` }; `+4 (size) = 0`. No code callers
  (duplicate instance). gta-reversed: Text Unload - same. Open questions: none.
