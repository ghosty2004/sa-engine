# POP static initialisers (group P1)

Each is a `__cdecl void ()` C++ dynamic initialiser called from the CRT `__xc` table at start-up (no callers in code, plain `ret`); none
registers an atexit destructor.

## 0x84A040 __init_aNodesToBeCleared  (catalogue: specified/REA, port level 0)
- Constructs `aNodesToBeCleared` 0x972CD0 (gr PathFind.cpp:27, `std::array<CNodeAddress, 5000>`): loop `ecx = 0x1388` (5000) down to 0, writes
  `u16 0xFFFF` to `[eax]` (the area word) and steps `eax += 4` (0x84A045-0x84A059). Only the area half of each CNodeAddress is set; the node
  word (+2) keeps its .bss zero. This is the CNodeAddress default constructor (`m_wAreaId = 0xFFFF`) inlined.
- Port: initialise all 5000 entries to `{area: 0xFFFF, node: 0}`. Open questions: none.

## 0x84A080 __init_aExteriorNodeLinkedTo  (catalogue: specified/REA, port level 0)
- Same pattern for `aExteriorNodeLinkedTo` 0x977B7C (gr PathFind.cpp:25, `std::array<CNodeAddress, NUM_PATH_INTERIOR_AREAS>`), count 8
  (0x84A085): 8 entries `{area: 0xFFFF, node: 0}` (stride 4, 0x84A090-0x84A099). Open questions: none.

## 0x84B350 __init_g_a9a8b8  (catalogue: specified/REA, port level 0)
- Stores `0x858630 = 0.2777779996395111 (f32) / 0x85A568 = 50.0 (f32)` into the f32 global 0xA9A8B8 (x87 `fld; fdiv; fstp`, 0x84B350-0x84B35C).
  Value: the f32 rounding of 0.277778f / 50.0f (about 0.00555556). This is a translation-unit copy of gr's `VELOCITY_CONST = 0.277778f / 50.f`
  (cHandlingDataMgr.cpp:11; km/h to units-per-frame); gr folds it into a constexpr, the original computes it at start-up.
- Proposed name: `g_fVelocityConst` (unnamed in gr; 6 gr users per catalog/names-extra.csv). Open questions: which translation unit owns 0xA9A8B8
  (no behavioural effect).
