# CPedType: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x608790 CPedType::FindPedType  (gr_deviations = 3)
Original bytes (`__cdecl ePedType(const char* name [esp+4])`):
  1. for i = 0..31: inline `strcmp(name, table[i])` with table = 0x8D23B8 (32 char* : PLAYER1, PLAYER2, PLAYER_NETWORK,
     PLAYER_UNUSED, CIVMALE, CIVFEMALE, COP, GANG1..GANG10, DEALER, MEDIC, FIREMAN, CRIMINAL, BUM, PROSTITUTE, SPECIAL,
     MISSION1..MISSION8); first match returns i (0x6087A0-0x6087E5, 0x608803).
  2. Not found: `"PLAYER_NETWORK"` (0x86D028, 15-byte `repe cmpsb`) -> 2; else `"PLAYER_UNUSED"` (0x86D018, 14 bytes) -> 3;
     else -> 32 (`setne; dec; and 0xFFFFFFE3; add 0x20`, 0x60880A-0x608825). Both extra compares are dead (the loop already
     covers those names), so the observable result for an unknown name is 32 (PED_TYPE_COUNT). Case-sensitive.
Row PedType.cpp:106 (notsa-helper mapping, deviates=no): Verdict REFUTED (same 32 names, same indices). Class: harmless.
Row PedType.cpp:140 (NOTE: original returned PED_TYPE_COUNT as invalid sentinel): Verdict CONFIRMED - gr `find_value` hits
  `NOTSA_UNREACHABLE` for an unknown name; the original returns 32. Class: keep-original.
Row PedType.cpp:141 (`notsa::find_value`, deviates=no): Verdict REFUTED for known names (covered by row 140). Class: harmless.

## 0x608B00 CPedType::Shutdown  (gr_deviations = 1)
Row PedType.cpp:35 (NOTSA `delete[] ms_apPedTypesOld`).
- Original bytes: `p = CPedType::ms_apPedTypes` (0xC0BBE8); if p != null: `eh_vector_destructor_iterator(p, 0x14, count = [p-4],
  CAcquaintance::~CAcquaintance 0x608780)` (0x821E02, stdcall) then `operator delete[](p - 4)` (0x8213AE) (0x608B09-0x608B24).
  The pointer is not cleared. Nothing else is freed (there is no "old" array in the original).
- Verdict: CONFIRMED (gr frees an extra gr-only array).
- Class: harmless (gr-only storage; a port has no ms_apPedTypesOld and must not null ms_apPedTypes).

## 0x608E40 CPedType::Initialise  (gr_deviations = 1)
Row PedType.cpp:28 (NOTSA `ms_apPedTypesOld = new CAcquaintance[PED_TYPE_COUNT]()`).
- Original bytes (SEH frame, `__cdecl void()`): `p = operator new(0x284)` (0x821195; 4-byte count header + 32 * 0x14);
  if p: `*(int32*)p = 32`, `eh_vector_constructor_iterator(p+4, 0x14, 32, CAcquaintance::CAcquaintance 0x608AE0,
  ~CAcquaintance 0x608780)`, `ms_apPedTypes (0xC0BBE8) = p+4`; else `ms_apPedTypes = 0` (0x608E57-0x608E95).
  Then for each of the 32 entries (offset 0..0x26C step 0x14) zero 5 dwords, re-reading `ms_apPedTypes` each time
  (0x608EA3-0x608EC6; with a null allocation this writes through null - original behaviour). Then `CPedType::LoadPedData()`
  0x608B30. No second array.
- Verdict: CONFIRMED (gr allocates an extra gr-only array).
- Class: harmless (do not port the extra array).
