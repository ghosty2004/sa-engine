# CCopPed (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5DE2C0 CCopPed::vfunc_0 = scalar deleting destructor  (catalogue: specified/REA, port level 17)
- Convention: virtual slot 0, `__thiscall CCopPed*(uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `CCopPed::Destructor` 0x5DE0D0; if `flags & 1`: `CPed::operator delete(this)` 0x5E4760 (cdecl). Callers (virtual):
  CSetPiece::Update 0x499A80, CPopulation::AddDeadPedInFrontOfCar 0x612CD0.
