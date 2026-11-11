# CMatrix (MATH) — RE-SPEC-REST

## 0x45AF40 CMatrix::ScaleAll  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = CMatrix*), f32 `scale` at [esp+4], `ret 4`, void. Callers: tReplayVehicleBlock::ExtractVehicleUpdateData
  0x45BA90, CReplay::ProcessPedUpdate 0x45CA70 (2 sites).
- Extent is split: 0x45AF40 `lea eax, [ecx+8]; jmp 0x401BA5`; 0x401BA5 `mov ecx, 4; jmp 0x45AF48` (a linker-shared tail
  stub in the low .text), then the loop 0x45AF48-0x45AF6A.
- Behaviour: EAX starts at this+8, ECX = 4 iterations; each iteration does `eax += 0x10` first and then scales the three f32 at
  [eax-0x18], [eax-0x14], [eax-0x10] — i.e. this+0x00/+0x04/+0x08, then +0x10.., +0x20.., +0x30.. (right, up, at, pos rows of
  CMatrix; the pad dwords +0x0C/+0x1C/+0x2C/+0x3C are not touched). Each element: `fld scale; fmul [elem]; fstp [elem]`
  (scale * elem, f32). So the translation (pos, +0x30..+0x38) is scaled too ("All").
- gta-reversed: not provided (REA source).
