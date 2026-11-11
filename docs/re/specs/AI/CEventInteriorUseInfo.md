# CEventInteriorUseInfo (AI) - RE-SPEC-FINAL (helper B)

## 0x674ED0 CEventInteriorUseInfo::~CEventInteriorUseInfo  (catalogue: unknown/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `vtbl = 0x8701A8` (in the .HOODLUM detour 0x401AC8; entry `nop; jmp`), then tail-jump to the CEvent base dtor 0x4AC010 (0x674ED6). Callers: scalar deleting dtor 0x674F90 and one more.
