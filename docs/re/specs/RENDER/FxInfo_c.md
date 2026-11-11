# FxInfo_c (RENDER) - RE-SPEC-FINAL group A

## 0x4A4940 FxInfo_c::~FxInfo_c  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this)`, plain `ret`. Path: 0x4A4940 `nop; jmp 0x4038A0` (HOODLUM chunk: `vtable = 0x85A7DC` (FxInfo_c), `jmp
  0x4A4946`) -> `ret`. Used as SEH unwind action by every FxInfo*_c ctor (64 sites).
- gta-reversed: FxInfo.h virtual dtor `= default` - matches. Open questions: none.
