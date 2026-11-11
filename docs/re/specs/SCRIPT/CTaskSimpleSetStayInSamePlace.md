# CTaskSimpleSetStayInSamePlace (SCRIPT) — RE-SPEC-REST

## 0x62F590 CTaskSimpleSetStayInSamePlace::CTaskSimpleSetStayInSamePlace(bool stay)  (catalogue: specified/REA, port level 1)
- `__thiscall`, u8 [esp+4], `ret 4`, returns EAX = this. CTaskSimple base ctor 0x61A390; `this[+8] = stay`; `vtable = 0x86DD50`.
  Caller: ProcessCommands1500To1599 0x490DB0.

## 0x62F5E0 CTaskSimpleSetStayInSamePlace::ProcessPed(CPed* ped)  (catalogue: specified/REA, port level 0)
- `__thiscall`, ped [esp+4], `ret 4`, returns AL = 1 (task finishes at once).
- `ped[+0x46C] = (ped[+0x46C] & ~0x200000) | ((this[+8] << 21) & 0x200000)` (xor-mask idiom; bit 21 of the first ped-flags dword =
  bStayInSamePlace per gr naming, not re-verified).
