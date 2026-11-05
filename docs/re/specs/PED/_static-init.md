# _static-init (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x84FF30 __init_PlayerClothes  (catalogue: specified/REA, port level 0)
- CRT static initialiser, `__cdecl void()`, no atexit. Inlined `CPedClothesDesc::CPedClothesDesc()` (= Initialise 0x5A78F0) on
  `PlayerClothes` 0xBC1C78 (CPedClothesDesc, 0x78 bytes): `rep stosd` 10 dwords of 0 at 0xBC1C78 (m_anModelKeys[10]),
  18 dwords of 0 at 0xBC1CA0 (m_anTextureKeys[18]), then dword 0 at 0xBC1CE8 (m_fFatStat) and 0xBC1CEC (m_fMuscleStat)
  (0x84FF31-0x84FF50).
- gta-reversed: `CPedClothesDesc::Initialise` (`PedClothesDesc.cpp:33`), same.
