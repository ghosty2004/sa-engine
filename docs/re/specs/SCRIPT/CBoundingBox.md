# CBoundingBox (SCRIPT) — RE-SPEC-REST

## 0x470100 CBoundingBox::SetMinMax(CVector min, CVector max)  (catalogue: specified/REA, port level 0)
- `__thiscall`, six f32 by value at [esp+4..+0x18], `ret 0x18`; returns EAX = this. Stub → .HOODLUM 0x15649B0. Caller:
  ProcessCommands2000To2099 0x472310.
- `this[+0..+8] = min (x, y, z)`; `this[+0xC..+0x14] = max` (raw dword copies).
