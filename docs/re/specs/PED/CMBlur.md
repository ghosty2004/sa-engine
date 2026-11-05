# CMBlur (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x71D730 CMBlur::SetDrunkBlur  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void(float drunkness [esp+4])`, plain `ret`. Caller: CPlayerPed::ProcessControl 0x60EA90.
- Globals written: `CMBlur::Drunkness` 0xC73C58 (f32).
- Behaviour:
  1. `fld v; fcomp 1.0f [0x858624]; test ah,5; jnp` (0x71D730-0x71D73F): if `v < 1.0f` go to 2; otherwise (v >= 1.0 **or NaN**)
     store 1.0f (0x3F800000) and return.
  2. `fld v; fcomp 0.0f [0x858B50]; test ah,0x41; je` (0x71D74C-0x71D75B): if `v > 0.0f` store v (raw dword copy, 0x71D768);
     else store 0 (+0.0f).
  So `Drunkness = clamp(v, 0, 1)` with NaN -> 1.0f and -0.0 -> +0.0.
