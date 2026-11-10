# POP static initialisers / atexit destructors (group P2)

All are `__cdecl void ()` with plain `ret`, run from the CRT `__xc` initialiser table (no direct callers).

## 0x84A640 __init_CRoadBlocks__RoadBlockNodes  (catalogue: specified/REA, port level 0)
Constructs `CRoadBlocks::RoadBlockNodes` 0xA435A0 (CNodeAddress[0x145 = 325], stride 4): loop `ecx = 0x145` down to 0
(0x84A645-0x84A659) storing `word [eax] = 0xFFFF` (m_wAreaId only, 0x84A650) and `eax += 4`. m_wNodeId (+2) is NOT written
(stays 0 from .bss). No atexit registration (trivial destructor). gta-reversed's `CNodeAddress` default constructor sets both
words to -1 (NodeAddress.h:11-12); the original inline constructor only sets the area id - port: write only +0 of each entry.

## 0x84B370 __init_g_a9a8bc  (catalogue: specified/REA, port level 0)
`f32 [0xA9A8BC] = f32 [0x85A58C] (0.005f) * f32 [0x858620] (0.0625f)` (fld/fmul/fstp, 0x84B370-0x84B37C) = 0.0003125f
(bits 0x39A3D70A after 24-bit rounding). No atexit. The global is unnamed in gta-reversed (it lies 4 bytes before
`CTagManager::ms_tagDesc` 0xA9A8C0); the byte reads of `[eax*8 + 0xA9A8BC]` in CTagManager 0x49CDE0/0x49CEC0/0x49CFE0 are
`ms_tagDesc[eax-1].alpha` (+4) with a 1-based index, not reads of this float. No direct reader found.

## 0x850CE0 __init_CGangs__Gang  (catalogue: specified/REA, port level 0)
`eh_vector_constructor_iterator(0xC091F0 CGangs::Gang, size 0x10, count 0xA, ctor 0x5DE520, dtor 0x5DE540)` 0x821D30 (stdcall,
5 args pushed 0x850CE0-0x850CEE), then `atexit(0x856760 __dtor_CGangs__Gang)` 0x821D1E (`pop ecx`). CGangInfo ctor 0x5DE520
(thiscall): `+0 (u8) = 0xFF`, `+4 = +8 = +0xC = 0` (dwords); +1..+3 untouched. CGangInfo dtor 0x5DE540 is an empty `ret`.

## 0x8540D0 __init_g_c2b950  (catalogue: specified/REA, port level 0)
`f32 [0xC2B950] = f32 [0x8722DC] (0.005f) * f32 [0x858620] (0.0625f)` (0x8540D0-0x8540DC) = 0.0003125f. No atexit. 0xC2B950 is
the first address after `CTheCarGenerators::CarGeneratorArray` (the `cmp esi,0xC2B950` in CTheCarGenerators::Process 0x6F3FA4 is
that array's end bound, not a use of this float). No direct reader found.

## 0x856760 __dtor_CGangs__Gang  (catalogue: specified/REA, port level 0)
atexit destructor registered by 0x850CE0: `eh_vector_destructor_iterator(0xC091F0, 0x10, 0xA, 0x5DE540)` 0x821E02 (stdcall, 4 args).
The element destructor 0x5DE540 is an empty `ret`, so this has no observable effect.
