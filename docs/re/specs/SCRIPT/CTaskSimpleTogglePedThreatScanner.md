# CTaskSimpleTogglePedThreatScanner (SCRIPT) — RE-SPEC-REST

## 0x492D10 CTaskSimpleTogglePedThreatScanner::CTaskSimpleTogglePedThreatScanner(bool a, bool b, bool c)  (catalogue: specified/REA, port level 1)
- `__thiscall`, three u8 args [esp+4..0xC], `ret 0xC`, returns EAX = this. Stub → .HOODLUM 0x15622B0. Caller: ProcessCommands1600To1699 0x493FE0.
- CTaskSimple base ctor 0x61A390; `this[+8] = a; vtable = 0x85A334; this[+9] = b; this[+0xA] = c`.

## 0x6337A0 CTaskSimpleTogglePedThreatScanner::ProcessPed(CPed* ped)  (catalogue: specified/REA, port level 0)
- `__thiscall`, ped [esp+4], `ret 4`, returns AL = 1.
- `s = ped->m_pIntelligence (+0x47C) + 0x238` (a scanner/flags block in CPedIntelligence); `s[+0xD] = this[+9]; s[+0xC] = this[+8];
  s[+0xE] = this[+0xA]` (store order as listed).
