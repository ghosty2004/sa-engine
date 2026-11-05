# CIdleCam (CAM) - full specs (group A part)

## 0x517760 CIdleCam::CIdleCam  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall CIdleCam* (CIdleCam* this = ECX)`, plain `ret`; returns `this` in EAX (0x517768).
  ESI is saved/restored (0x517760/0x51776A).
- Callers: none in the catalogue (the global `gIdleCam` is constructed by a static initialiser that may inline or call
  this; not my scope). Callees: CIdleCam::Init 0x50E6D0 (thiscall, ECX = this unchanged at the call 0x517763).
- Behaviour: `Init()`; `return this`. No field is written directly by the constructor.
- Note for the port: CIdleCam::Init 0x50E6D0 (not in this file) writes fields +0x14..+0x94 with constants and then
  calls CIdleCam::Reset 0x50A160 with a pushed argument 0 (`push eax` at 0x50E6E2, eax = 0); the constructor only
  forwards to it.
- gta-reversed: IdleCam.h declares the constructor; no hook at 0x517760. No difference in behaviour.
