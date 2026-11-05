# CClothes (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5A7A20 GetClothesModelFromName  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl int32(const char* name [esp+4])`, plain `ret`, return EAX. Callers: CClothes::LoadClothesFile 0x5A7B30 (2 sites).
- Behaviour: sequence of exact compares implemented as `repe cmpsb` over `strlen(lit)+1` bytes (so the terminating NUL is
  compared: equivalent to `strcmp(name, lit) == 0`; reads of `name` stop at the first mismatch). Order and results:
  "torso" 0x8590CC -> 0, "head" 0x8590E8 -> 1, "hands" 0x867130 -> 2, "legs" 0x859124 -> 3, "feet" 0x867128 -> 4,
  "necklace" 0x8670A0 -> 5, "watch" 0x867098 -> 6, "glasses" 0x867090 -> 7, "hat" 0x86708C -> 8,
  "extra1" 0x85C3B0 -> 9 (0x5A7B0D-0x5A7B23: `setne al; dec eax; and eax, 9` gives 9 on match, 0 otherwise).
  Any other string -> 0 (CLOTHES_MODEL_TORSO). Case-sensitive.
- gta-reversed: `GetClothesModelFromName` (`Clothes.cpp:47`), same (unknown -> CLOTHES_MODEL_TORSO).

## 0x5A7EA0 CClothes::GetTextureDependency  (catalogue: specified/REA, gr hook `.Reversed = false`, port level 0)
- Convention: `__cdecl eClothesModelPart(eClothesTexturePart part [esp+4])`, plain `ret`, return EAX.
- Callers: CRunningScript::ProcessCommands2200To2299 0x474900, CShopping::Buy 0x49BF70, CPedClothesDesc::SetTextureAndModel
  (hash 0x5A8050 / name 0x5A8080), CClothes::Init 0x5A80D0.
- Behaviour: `if ((uint32)part > 17) return 10` (`ja`, unsigned: negatives also give 10). Otherwise byte table 0x5A7F18
  `00 01 02 03 04x9 05 06 07 08 09` selects jump table 0x5A7EF0. Result per part:
  0->0, 1->1, 2->3, 3->4, 4..12->10, 13->5, 14->6, 15->7, 16->8, 17->9.
- gta-reversed: `Clothes.cpp:222` switch gives the same mapping (TORSO, HEAD, LEGS->3, SHOES->4, NECKLACE..SPECIAL -> 5..9,
  default 10). The `.Reversed = false` comment ("crashes when enabled") is not explained by a behaviour difference.
