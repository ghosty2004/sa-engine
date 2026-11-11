# AI: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

Generated from the bytes by the RE-SPEC-UNK lead (decoder checked by hand on samples). All `__init_*`
functions are entries of the CRT `__xc` initialiser table 0x8A4004 (called once, in table-slot order, by the
CRT before `WinMain`; `__cdecl void(void)`, plain `ret`, no arguments, no return value). `__dtor_*` functions
are registered with `_atexit` (0x821D1E) and run at process exit in reverse registration order. CRT helpers:
`eh vector constructor iterator` 0x821D30 (stdcall: arr, elemSize, count, ctor, dtor; ascending); `eh vector destructor iterator` 0x821E02 (stdcall: arr, elemSize, count, dtor; descending); 0x8214BD operator delete (cdecl).

## Header-constant initialisers (table)

About 225 translation units include a header that defines six float constants which the compiler could not
fold, so every such TU gets its own copies, computed by six `__xc` initialisers (one per constant, in this
order): `a * 0.0625` (a = 0.005), `1 / b` (b = 50.0), `100 / c` (c = 3.6), `1 / (b * b)`, `previous * 10`,
`0.277778 / b`. The operands a, b, c are TU-local .rdata copies (addresses differ per TU), 0.0625, 1.0,
100.0, 10.0 and 0.277778 are shared .rdata constants. A few other initialisers of the same shape subtract
two globals. Instruction shapes: `fld a; fmul b; fstp g` / `fld a; fdiv b; fstp g` / `fld a; fsub b; fstp g`
/ `fld a; fmul b; fdivr c; fstp g` (= c / (a * b)), then `ret`.

Port rules: compute in x87 order (left operand loaded first; `fdivr c` = c / ST0) and store as f32. The x87
precision control at CRT-init time is the process default (53-bit) because D3D has not yet lowered it to
24-bit; for every row below the f32 result was computed with both a 53-bit and a 24-bit model and they agree
unless the row says DIFFERS. Most target globals are never read by any code (the "readers" column comes from
a scan of every code byte for the absolute address outside the initialisers; "dead store" = no occurrence):
a port only needs them for memory-image parity, but must keep the initialisation order because some rows
read a global set by an earlier row (shown as `g[...] (= value, set by an earlier initialiser)`).
The subsystem of a row is the TU guess described in the classification evidence (low/medium confidence);
the behaviour does not depend on it.

| address | __xc slot | effect | f32 result | readers |
|---|---|---|---|---|
| 0x849390 | 0x8a4098 | g[0x9689e4] <- 0.005f [0x858c2c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8493b0 | 0x8a409c | g[0x9689e8] <- 1.0f [0x858624] / 50.0f [0x858c34] | 0.02 | none (dead store) |
| 0x8493d0 | 0x8a40a0 | g[0x9689ec] <- 100.0f [0x858628] / 3.6f [0x858c30] | 27.777779 | none (dead store) |
| 0x8493f0 | 0x8a40a4 | g[0x9689f0] <- 1.0f [0x858624] / (50.0f [0x858c34] * 50.0f [0x858c34]) | 0.0004 | none (dead store) |
| 0x849410 | 0x8a40a8 | g[0x9689f4] <- g[0x9689f0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849430 | 0x8a40ac | g[0x9689f8] <- 0.277778f [0x858630] / 50.0f [0x858c34] | 0.00555556 | none (dead store) |
| 0x849620 | 0x8a40e8 | g[0x969000] <- 0.005f [0x858c3c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849640 | 0x8a40ec | g[0x969004] <- 1.0f [0x858624] / 50.0f [0x858c44] | 0.02 | none (dead store) |
| 0x849660 | 0x8a40f0 | g[0x969008] <- 100.0f [0x858628] / 3.6f [0x858c40] | 27.777779 | none (dead store) |
| 0x849680 | 0x8a40f4 | g[0x96900c] <- 1.0f [0x858624] / (50.0f [0x858c44] * 50.0f [0x858c44]) | 0.0004 | none (dead store) |
| 0x8496a0 | 0x8a40f8 | g[0x969010] <- g[0x96900c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8496c0 | 0x8a40fc | g[0x969014] <- 0.277778f [0x858630] / 50.0f [0x858c44] | 0.00555556 | none (dead store) |
| 0x8496e0 | 0x8a4100 | g[0x969040] <- 0.005f [0x858c5c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849700 | 0x8a4104 | g[0x969044] <- 1.0f [0x858624] / 50.0f [0x858c64] | 0.02 | none (dead store) |
| 0x849720 | 0x8a4108 | g[0x969048] <- 100.0f [0x858628] / 3.6f [0x858c60] | 27.777779 | none (dead store) |
| 0x849740 | 0x8a410c | g[0x96904c] <- 1.0f [0x858624] / (50.0f [0x858c64] * 50.0f [0x858c64]) | 0.0004 | none (dead store) |
| 0x849760 | 0x8a4110 | g[0x969050] <- g[0x96904c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849780 | 0x8a4114 | g[0x969054] <- 0.277778f [0x858630] / 50.0f [0x858c64] | 0.00555556 | none (dead store) |
| 0x849920 | 0x8a4148 | g[0x969194] <- 0.005f [0x859074] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849940 | 0x8a414c | g[0x969198] <- 1.0f [0x858624] / 50.0f [0x85907c] | 0.02 | none (dead store) |
| 0x849960 | 0x8a4150 | g[0x96919c] <- 100.0f [0x858628] / 3.6f [0x859078] | 27.777779 | none (dead store) |
| 0x849980 | 0x8a4154 | g[0x9691a0] <- 1.0f [0x858624] / (50.0f [0x85907c] * 50.0f [0x85907c]) | 0.0004 | none (dead store) |
| 0x8499a0 | 0x8a4158 | g[0x9691a4] <- g[0x9691a0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8499c0 | 0x8a415c | g[0x9691a8] <- 0.277778f [0x858630] / 50.0f [0x85907c] | 0.00555556 | none (dead store) |
| 0x84b5f0 | 0x8a44dc | g[0xa9ae70] <- 0.005f [0x85a774] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b610 | 0x8a44e0 | g[0xa9ae74] <- 0.005f [0x85a7d8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b670 | 0x8a44ec | g[0xa9af3c] <- 0.005f [0x85a9f8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b690 | 0x8a44f0 | g[0xa9af40] <- 1.0f [0x858624] / 50.0f [0x85aa00] | 0.02 | none (dead store) |
| 0x84b6b0 | 0x8a44f4 | g[0xa9af44] <- 100.0f [0x858628] / 3.6f [0x85a9fc] | 27.777779 | none (dead store) |
| 0x84b6d0 | 0x8a44f8 | g[0xa9af48] <- 1.0f [0x858624] / (50.0f [0x85aa00] * 50.0f [0x85aa00]) | 0.0004 | none (dead store) |
| 0x84b6f0 | 0x8a44fc | g[0xa9af4c] <- g[0xa9af48] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b710 | 0x8a4500 | g[0xa9af50] <- 0.277778f [0x858630] / 50.0f [0x85aa00] | 0.00555556 | none (dead store) |
| 0x84b7f0 | 0x8a451c | g[0xa9af70] <- 0.005f [0x85aaa4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b810 | 0x8a4520 | g[0xa9af74] <- 1.0f [0x858624] / 50.0f [0x85aaac] | 0.02 | none (dead store) |
| 0x84b830 | 0x8a4524 | g[0xa9af78] <- 100.0f [0x858628] / 3.6f [0x85aaa8] | 27.777779 | none (dead store) |
| 0x84b850 | 0x8a4528 | g[0xa9af7c] <- 1.0f [0x858624] / (50.0f [0x85aaac] * 50.0f [0x85aaac]) | 0.0004 | none (dead store) |
| 0x84b870 | 0x8a452c | g[0xa9af80] <- g[0xa9af7c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b890 | 0x8a4530 | g[0xa9af84] <- 0.277778f [0x858630] / 50.0f [0x85aaac] | 0.00555556 | none (dead store) |
| 0x84b8b0 | 0x8a4534 | g[0xa9af88] <- 0.005f [0x85aab8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b8d0 | 0x8a4538 | g[0xa9af8c] <- 1.0f [0x858624] / 50.0f [0x85aac0] | 0.02 | none (dead store) |
| 0x84b8f0 | 0x8a453c | g[0xa9af90] <- 100.0f [0x858628] / 3.6f [0x85aabc] | 27.777779 | none (dead store) |
| 0x84b910 | 0x8a4540 | g[0xa9af94] <- 1.0f [0x858624] / (50.0f [0x85aac0] * 50.0f [0x85aac0]) | 0.0004 | none (dead store) |
| 0x84b930 | 0x8a4544 | g[0xa9af98] <- g[0xa9af94] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b950 | 0x8a4548 | g[0xa9af9c] <- 0.277778f [0x858630] / 50.0f [0x85aac0] | 0.00555556 | none (dead store) |
| 0x84b970 | 0x8a454c | g[0xa9afa0] <- 0.005f [0x85aac4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b990 | 0x8a4550 | g[0xa9afa4] <- 1.0f [0x858624] / 50.0f [0x85aacc] | 0.02 | none (dead store) |
| 0x84b9b0 | 0x8a4554 | g[0xa9afa8] <- 100.0f [0x858628] / 3.6f [0x85aac8] | 27.777779 | none (dead store) |
| 0x84b9d0 | 0x8a4558 | g[0xa9afac] <- 1.0f [0x858624] / (50.0f [0x85aacc] * 50.0f [0x85aacc]) | 0.0004 | none (dead store) |
| 0x84b9f0 | 0x8a455c | g[0xa9afb0] <- g[0xa9afac] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ba10 | 0x8a4560 | g[0xa9afb4] <- 0.277778f [0x858630] / 50.0f [0x85aacc] | 0.00555556 | none (dead store) |
| 0x84bab0 | 0x8a4570 | g[0xa9b09c] <- 1.0f [0x858624] / 50.0f [0x85b894] | 0.02 | none (dead store) |
| 0x84baf0 | 0x8a4578 | g[0xa9b0a4] <- 1.0f [0x858624] / (50.0f [0x85b894] * 50.0f [0x85b894]) | 0.0004 | none (dead store) |
| 0x84bb10 | 0x8a457c | g[0xa9b0a8] <- g[0xa9b0a4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84bb30 | 0x8a4580 | g[0xa9b0ac] <- 0.277778f [0x858630] / 50.0f [0x85b894] | 0.00555556 | none (dead store) |
| 0x850dd0 | 0x8a4fc8 | g[0xc09828] <- 0.005f [0x86c3d0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850df0 | 0x8a4fcc | g[0xc0982c] <- 1.0f [0x858624] / 50.0f [0x86c3d8] | 0.02 | none (dead store) |
| 0x850e10 | 0x8a4fd0 | g[0xc09830] <- 100.0f [0x858628] / 3.6f [0x86c3d4] | 27.777779 | none (dead store) |
| 0x850e30 | 0x8a4fd4 | g[0xc09834] <- 1.0f [0x858624] / (50.0f [0x86c3d8] * 50.0f [0x86c3d8]) | 0.0004 | none (dead store) |
| 0x850e50 | 0x8a4fd8 | g[0xc09838] <- g[0xc09834] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850e70 | 0x8a4fdc | g[0xc0983c] <- 0.277778f [0x858630] / 50.0f [0x86c3d8] | 0.00555556 | none (dead store) |
| 0x850e90 | 0x8a4fe0 | g[0xc09844] <- 0.005f [0x86c3dc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850eb0 | 0x8a4fe4 | g[0xc09848] <- 1.0f [0x858624] / 50.0f [0x86c3e4] | 0.02 | none (dead store) |
| 0x850ed0 | 0x8a4fe8 | g[0xc0984c] <- 100.0f [0x858628] / 3.6f [0x86c3e0] | 27.777779 | none (dead store) |
| 0x850ef0 | 0x8a4fec | g[0xc09850] <- 1.0f [0x858624] / (50.0f [0x86c3e4] * 50.0f [0x86c3e4]) | 0.0004 | none (dead store) |
| 0x850f10 | 0x8a4ff0 | g[0xc09854] <- g[0xc09850] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850f30 | 0x8a4ff4 | g[0xc09858] <- 0.277778f [0x858630] / 50.0f [0x86c3e4] | 0.00555556 | none (dead store) |
| 0x850f70 | 0x8a5000 | g[0xc0987c] <- 0.005f [0x86c644] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850f90 | 0x8a5004 | g[0xc09880] <- 1.0f [0x858624] / 50.0f [0x86c64c] | 0.02 | none (dead store) |
| 0x850fb0 | 0x8a5008 | g[0xc09884] <- 100.0f [0x858628] / 3.6f [0x86c648] | 27.777779 | none (dead store) |
| 0x850fd0 | 0x8a500c | g[0xc09888] <- 1.0f [0x858624] / (50.0f [0x86c64c] * 50.0f [0x86c64c]) | 0.0004 | none (dead store) |
| 0x850ff0 | 0x8a5010 | g[0xc0988c] <- g[0xc09888] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851010 | 0x8a5014 | g[0xc09890] <- 0.277778f [0x858630] / 50.0f [0x86c64c] | 0.00555556 | none (dead store) |
| 0x851030 | 0x8a5018 | g[0xc0989c] <- 0.005f [0x86c650] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851050 | 0x8a501c | g[0xc098a0] <- 1.0f [0x858624] / 50.0f [0x86c658] | 0.02 | none (dead store) |
| 0x851070 | 0x8a5020 | g[0xc098a4] <- 100.0f [0x858628] / 3.6f [0x86c654] | 27.777779 | none (dead store) |
| 0x851090 | 0x8a5024 | g[0xc098a8] <- 1.0f [0x858624] / (50.0f [0x86c658] * 50.0f [0x86c658]) | 0.0004 | none (dead store) |
| 0x8510b0 | 0x8a5028 | g[0xc098ac] <- g[0xc098a8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8510d0 | 0x8a502c | g[0xc098b0] <- 0.277778f [0x858630] / 50.0f [0x86c658] | 0.00555556 | none (dead store) |
| 0x8510f0 | 0x8a5030 | g[0xc098b8] <- 0.005f [0x86c68c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851110 | 0x8a5034 | g[0xc098bc] <- 1.0f [0x858624] / 50.0f [0x86c694] | 0.02 | none (dead store) |
| 0x851130 | 0x8a5038 | g[0xc098c0] <- 100.0f [0x858628] / 3.6f [0x86c690] | 27.777779 | none (dead store) |
| 0x851150 | 0x8a503c | g[0xc098c4] <- 1.0f [0x858624] / (50.0f [0x86c694] * 50.0f [0x86c694]) | 0.0004 | none (dead store) |
| 0x851170 | 0x8a5040 | g[0xc098c8] <- g[0xc098c4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851190 | 0x8a5044 | g[0xc098cc] <- 0.277778f [0x858630] / 50.0f [0x86c694] | 0.00555556 | none (dead store) |
| 0x8511b0 | 0x8a5048 | g[0xc098f0] <- 0.005f [0x86c6b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8511d0 | 0x8a504c | g[0xc098f4] <- 1.0f [0x858624] / 50.0f [0x86c6b8] | 0.02 | none (dead store) |
| 0x8511f0 | 0x8a5050 | g[0xc098f8] <- 100.0f [0x858628] / 3.6f [0x86c6b4] | 27.777779 | none (dead store) |
| 0x851210 | 0x8a5054 | g[0xc098fc] <- 1.0f [0x858624] / (50.0f [0x86c6b8] * 50.0f [0x86c6b8]) | 0.0004 | none (dead store) |
| 0x851230 | 0x8a5058 | g[0xc09900] <- g[0xc098fc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851250 | 0x8a505c | g[0xc09904] <- 0.277778f [0x858630] / 50.0f [0x86c6b8] | 0.00555556 | none (dead store) |
| 0x851360 | 0x8a507c | g[0xc0afc4] <- 0.005f [0x86c854] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8513a0 | 0x8a5084 | g[0xc0afcc] <- 100.0f [0x858628] / 3.6f [0x86c858] | 27.777779 | none (dead store) |
| 0x8513c0 | 0x8a5088 | g[0xc0afd0] <- 1.0f [0x858624] / (50.0f [0x86c85c] * 50.0f [0x86c85c]) | 0.0004 | none (dead store) |
| 0x8513e0 | 0x8a508c | g[0xc0afd4] <- g[0xc0afd0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851400 | 0x8a5090 | g[0xc0afd8] <- 0.277778f [0x858630] / 50.0f [0x86c85c] | 0.00555556 | none (dead store) |
| 0x851420 | 0x8a5094 | g[0xc0b03c] <- 0.005f [0x86c8a8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851440 | 0x8a5098 | g[0xc0b040] <- 1.0f [0x858624] / 50.0f [0x86c8b0] | 0.02 | none (dead store) |
| 0x851460 | 0x8a509c | g[0xc0b044] <- 100.0f [0x858628] / 3.6f [0x86c8ac] | 27.777779 | none (dead store) |
| 0x851480 | 0x8a50a0 | g[0xc0b048] <- 1.0f [0x858624] / (50.0f [0x86c8b0] * 50.0f [0x86c8b0]) | 0.0004 | none (dead store) |
| 0x8514a0 | 0x8a50a4 | g[0xc0b04c] <- g[0xc0b048] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8514c0 | 0x8a50a8 | g[0xc0b050] <- 0.277778f [0x858630] / 50.0f [0x86c8b0] | 0.00555556 | none (dead store) |
| 0x851500 | 0x8a50b0 | g[0xc0b1b8] <- 0.005f [0x86cefc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851520 | 0x8a50b4 | g[0xc0b1bc] <- 1.0f [0x858624] / 50.0f [0x86cf04] | 0.02 | none (dead store) |
| 0x851540 | 0x8a50b8 | g[0xc0b1c0] <- 100.0f [0x858628] / 3.6f [0x86cf00] | 27.777779 | none (dead store) |
| 0x851560 | 0x8a50bc | g[0xc0b1c4] <- 1.0f [0x858624] / (50.0f [0x86cf04] * 50.0f [0x86cf04]) | 0.0004 | none (dead store) |
| 0x851580 | 0x8a50c0 | g[0xc0b1c8] <- g[0xc0b1c4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8515a0 | 0x8a50c4 | g[0xc0b1cc] <- 0.277778f [0x858630] / 50.0f [0x86cf04] | 0.00555556 | none (dead store) |
| 0x8515c0 | 0x8a50c8 | g[0xc0b1d0] <- 0.005f [0x86cf08] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8515e0 | 0x8a50cc | g[0xc0b1d4] <- 1.0f [0x858624] / 50.0f [0x86cf10] | 0.02 | none (dead store) |
| 0x851620 | 0x8a50d4 | g[0xc0b1dc] <- 1.0f [0x858624] / (50.0f [0x86cf10] * 50.0f [0x86cf10]) | 0.0004 | none (dead store) |
| 0x851640 | 0x8a50d8 | g[0xc0b1e0] <- g[0xc0b1dc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851660 | 0x8a50dc | g[0xc0b1e4] <- 0.277778f [0x858630] / 50.0f [0x86cf10] | 0.00555556 | none (dead store) |
| 0x851b70 | 0x8a517c | g[0xc16f00] <- 0.005f [0x86d2cc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851b90 | 0x8a5180 | g[0xc16f04] <- 1.0f [0x858624] / 50.0f [0x86d2d4] | 0.02 | none (dead store) |
| 0x851bb0 | 0x8a5184 | g[0xc16f08] <- 100.0f [0x858628] / 3.6f [0x86d2d0] | 27.777779 | none (dead store) |
| 0x851bd0 | 0x8a5188 | g[0xc16f0c] <- 1.0f [0x858624] / (50.0f [0x86d2d4] * 50.0f [0x86d2d4]) | 0.0004 | 0x61d3b6 in 0x61d0b0, 0x61d5c0 in 0x61d400 |
| 0x851bf0 | 0x8a518c | g[0xc16f10] <- g[0xc16f0c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851c10 | 0x8a5190 | g[0xc16f14] <- 0.277778f [0x858630] / 50.0f [0x86d2d4] | 0.00555556 | none (dead store) |
| 0x851cf0 | 0x8a51ac | g[0xc17088] <- 0.005f [0x86d474] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851d10 | 0x8a51b0 | g[0xc1708c] <- 1.0f [0x858624] / 50.0f [0x86d47c] | 0.02 | none (dead store) |
| 0x851d30 | 0x8a51b4 | g[0xc17090] <- 100.0f [0x858628] / 3.6f [0x86d478] | 27.777779 | none (dead store) |
| 0x851d50 | 0x8a51b8 | g[0xc17094] <- 1.0f [0x858624] / (50.0f [0x86d47c] * 50.0f [0x86d47c]) | 0.0004 | none (dead store) |
| 0x851d70 | 0x8a51bc | g[0xc17098] <- g[0xc17094] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851d90 | 0x8a51c0 | g[0xc1709c] <- 0.277778f [0x858630] / 50.0f [0x86d47c] | 0.00555556 | none (dead store) |
| 0x851db0 | 0x8a51c4 | g[0xc170a0] <- 0.005f [0x86d480] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851dd0 | 0x8a51c8 | g[0xc170a4] <- 1.0f [0x858624] / 50.0f [0x86d488] | 0.02 | none (dead store) |
| 0x851df0 | 0x8a51cc | g[0xc170a8] <- 100.0f [0x858628] / 3.6f [0x86d484] | 27.777779 | none (dead store) |
| 0x851e10 | 0x8a51d0 | g[0xc170ac] <- 1.0f [0x858624] / (50.0f [0x86d488] * 50.0f [0x86d488]) | 0.0004 | none (dead store) |
| 0x851e30 | 0x8a51d4 | g[0xc170b0] <- g[0xc170ac] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851e50 | 0x8a51d8 | g[0xc170b4] <- 0.277778f [0x858630] / 50.0f [0x86d488] | 0.00555556 | none (dead store) |
| 0x851e70 | 0x8a51dc | g[0xc170b8] <- 0.005f [0x86d4f8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851e90 | 0x8a51e0 | g[0xc170bc] <- 1.0f [0x858624] / 50.0f [0x86d500] | 0.02 | none (dead store) |
| 0x851eb0 | 0x8a51e4 | g[0xc170c0] <- 100.0f [0x858628] / 3.6f [0x86d4fc] | 27.777779 | none (dead store) |
| 0x851ed0 | 0x8a51e8 | g[0xc170c4] <- 1.0f [0x858624] / (50.0f [0x86d500] * 50.0f [0x86d500]) | 0.0004 | none (dead store) |
| 0x851ef0 | 0x8a51ec | g[0xc170c8] <- g[0xc170c4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851f10 | 0x8a51f0 | g[0xc170cc] <- 0.277778f [0x858630] / 50.0f [0x86d500] | 0.00555556 | none (dead store) |
| 0x851f30 | 0x8a51f4 | g[0xc177b8] <- 0.005f [0x86d610] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851f70 | 0x8a51fc | g[0xc177c0] <- 100.0f [0x858628] / 3.6f [0x86d614] | 27.777779 | none (dead store) |
| 0x851f90 | 0x8a5200 | g[0xc177c4] <- 1.0f [0x858624] / (50.0f [0x86d618] * 50.0f [0x86d618]) | 0.0004 | none (dead store) |
| 0x851fb0 | 0x8a5204 | g[0xc177c8] <- g[0xc177c4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851fd0 | 0x8a5208 | g[0xc177cc] <- 0.277778f [0x858630] / 50.0f [0x86d618] | 0.00555556 | none (dead store) |
| 0x852020 | 0x8a5214 | g[0xc178d8] <- 0.005f [0x86db18] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852040 | 0x8a5218 | g[0xc178dc] <- 1.0f [0x858624] / 50.0f [0x86db20] | 0.02 | none (dead store) |
| 0x852060 | 0x8a521c | g[0xc178e0] <- 100.0f [0x858628] / 3.6f [0x86db1c] | 27.777779 | none (dead store) |
| 0x852080 | 0x8a5220 | g[0xc178e4] <- 1.0f [0x858624] / (50.0f [0x86db20] * 50.0f [0x86db20]) | 0.0004 | none (dead store) |
| 0x8520a0 | 0x8a5224 | g[0xc178e8] <- g[0xc178e4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8520c0 | 0x8a5228 | g[0xc178ec] <- 0.277778f [0x858630] / 50.0f [0x86db20] | 0.00555556 | none (dead store) |
| 0x852160 | 0x8a5238 | g[0xc18bf4] <- 1.0f [0x858624] / 50.0f [0x86e674] | 0.02 | none (dead store) |
| 0x852180 | 0x8a523c | g[0xc18bf8] <- 100.0f [0x858628] / 3.6f [0x86e670] | 27.777779 | none (dead store) |
| 0x8521c0 | 0x8a5244 | g[0xc18c00] <- g[0xc18bfc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8521e0 | 0x8a5248 | g[0xc18c04] <- 0.277778f [0x858630] / 50.0f [0x86e674] | 0.00555556 | none (dead store) |
| 0x852380 | 0x8a527c | g[0xc18c84] <- 0.005f [0x86f10c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8523a0 | 0x8a5280 | g[0xc18c88] <- 1.0f [0x858624] / 50.0f [0x86f114] | 0.02 | none (dead store) |
| 0x8523c0 | 0x8a5284 | g[0xc18c8c] <- 100.0f [0x858628] / 3.6f [0x86f110] | 27.777779 | none (dead store) |
| 0x8523e0 | 0x8a5288 | g[0xc18c90] <- 1.0f [0x858624] / (50.0f [0x86f114] * 50.0f [0x86f114]) | 0.0004 | none (dead store) |
| 0x852400 | 0x8a528c | g[0xc18c94] <- g[0xc18c90] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852420 | 0x8a5290 | g[0xc18c98] <- 0.277778f [0x858630] / 50.0f [0x86f114] | 0.00555556 | none (dead store) |
| 0x852440 | 0x8a5294 | g[0xc18c9c] <- 0.005f [0x86f428] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852460 | 0x8a5298 | g[0xc18ca0] <- 1.0f [0x858624] / 50.0f [0x86f430] | 0.02 | none (dead store) |
| 0x852480 | 0x8a529c | g[0xc18ca4] <- 100.0f [0x858628] / 3.6f [0x86f42c] | 27.777779 | none (dead store) |
| 0x8524a0 | 0x8a52a0 | g[0xc18ca8] <- 1.0f [0x858624] / (50.0f [0x86f430] * 50.0f [0x86f430]) | 0.0004 | none (dead store) |
| 0x8524c0 | 0x8a52a4 | g[0xc18cac] <- g[0xc18ca8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8524e0 | 0x8a52a8 | g[0xc18cb0] <- 0.277778f [0x858630] / 50.0f [0x86f430] | 0.00555556 | none (dead store) |
| 0x852500 | 0x8a52ac | g[0xc18cb4] <- 0.005f [0x86f434] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852520 | 0x8a52b0 | g[0xc18cb8] <- 1.0f [0x858624] / 50.0f [0x86f43c] | 0.02 | none (dead store) |
| 0x852540 | 0x8a52b4 | g[0xc18cbc] <- 100.0f [0x858628] / 3.6f [0x86f438] | 27.777779 | none (dead store) |
| 0x852560 | 0x8a52b8 | g[0xc18cc0] <- 1.0f [0x858624] / (50.0f [0x86f43c] * 50.0f [0x86f43c]) | 0.0004 | none (dead store) |
| 0x852580 | 0x8a52bc | g[0xc18cc4] <- g[0xc18cc0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8525a0 | 0x8a52c0 | g[0xc18cc8] <- 0.277778f [0x858630] / 50.0f [0x86f43c] | 0.00555556 | none (dead store) |
| 0x8525c0 | 0x8a52c4 | g[0xc18ccc] <- 0.005f [0x86f63c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8525e0 | 0x8a52c8 | g[0xc18cd0] <- 1.0f [0x858624] / 50.0f [0x86f644] | 0.02 | none (dead store) |
| 0x852600 | 0x8a52cc | g[0xc18cd4] <- 100.0f [0x858628] / 3.6f [0x86f640] | 27.777779 | none (dead store) |
| 0x852620 | 0x8a52d0 | g[0xc18cd8] <- 1.0f [0x858624] / (50.0f [0x86f644] * 50.0f [0x86f644]) | 0.0004 | none (dead store) |
| 0x852640 | 0x8a52d4 | g[0xc18cdc] <- g[0xc18cd8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852660 | 0x8a52d8 | g[0xc18ce0] <- 0.277778f [0x858630] / 50.0f [0x86f644] | 0.00555556 | none (dead store) |
| 0x8526c0 | 0x8a52ec | g[0xc18d0c] <- 0.005f [0x86f81c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8526e0 | 0x8a52f0 | g[0xc18d10] <- 1.0f [0x858624] / 50.0f [0x86f824] | 0.02 | none (dead store) |
| 0x852700 | 0x8a52f4 | g[0xc18d14] <- 100.0f [0x858628] / 3.6f [0x86f820] | 27.777779 | none (dead store) |
| 0x852720 | 0x8a52f8 | g[0xc18d18] <- 1.0f [0x858624] / (50.0f [0x86f824] * 50.0f [0x86f824]) | 0.0004 | none (dead store) |
| 0x852740 | 0x8a52fc | g[0xc18d1c] <- g[0xc18d18] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852760 | 0x8a5300 | g[0xc18d20] <- 0.277778f [0x858630] / 50.0f [0x86f824] | 0.00555556 | none (dead store) |
| 0x852890 | 0x8a532c | g[0xc18f5c] <- 0.005f [0x870198] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8528b0 | 0x8a5330 | g[0xc18f60] <- 1.0f [0x858624] / 50.0f [0x8701a0] | 0.02 | none (dead store) |
| 0x8528d0 | 0x8a5334 | g[0xc18f64] <- 100.0f [0x858628] / 3.6f [0x87019c] | 27.777779 | none (dead store) |
| 0x8528f0 | 0x8a5338 | g[0xc18f68] <- 1.0f [0x858624] / (50.0f [0x8701a0] * 50.0f [0x8701a0]) | 0.0004 | none (dead store) |
| 0x852910 | 0x8a533c | g[0xc18f6c] <- g[0xc18f68] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852930 | 0x8a5340 | g[0xc18f70] <- 0.277778f [0x858630] / 50.0f [0x8701a0] | 0.00555556 | none (dead store) |
| 0x852950 | 0x8a5344 | g[0xc18f80] <- 0.005f [0x870390] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852970 | 0x8a5348 | g[0xc18f84] <- 1.0f [0x858624] / 50.0f [0x870398] | 0.02 | none (dead store) |
| 0x852990 | 0x8a534c | g[0xc18f88] <- 100.0f [0x858628] / 3.6f [0x870394] | 27.777779 | none (dead store) |
| 0x8529b0 | 0x8a5350 | g[0xc18f8c] <- 1.0f [0x858624] / (50.0f [0x870398] * 50.0f [0x870398]) | 0.0004 | none (dead store) |
| 0x8529d0 | 0x8a5354 | g[0xc18f90] <- g[0xc18f8c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8529f0 | 0x8a5358 | g[0xc18f94] <- 0.277778f [0x858630] / 50.0f [0x870398] | 0.00555556 | none (dead store) |
| 0x852a90 | 0x8a536c | g[0xc19634] <- 0.005f [0x870648] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852ab0 | 0x8a5370 | g[0xc19638] <- 1.0f [0x858624] / 50.0f [0x870650] | 0.02 | none (dead store) |
| 0x852ad0 | 0x8a5374 | g[0xc1963c] <- 100.0f [0x858628] / 3.6f [0x87064c] | 27.777779 | none (dead store) |
| 0x852af0 | 0x8a5378 | g[0xc19640] <- 1.0f [0x858624] / (50.0f [0x870650] * 50.0f [0x870650]) | 0.0004 | none (dead store) |
| 0x852b10 | 0x8a537c | g[0xc19644] <- g[0xc19640] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852b30 | 0x8a5380 | g[0xc19648] <- 0.277778f [0x858630] / 50.0f [0x870650] | 0.00555556 | none (dead store) |
| 0x852b50 | 0x8a5384 | g[0xc1964c] <- 0.005f [0x870654] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852b70 | 0x8a5388 | g[0xc19650] <- 1.0f [0x858624] / 50.0f [0x87065c] | 0.02 | none (dead store) |
| 0x852b90 | 0x8a538c | g[0xc19654] <- 100.0f [0x858628] / 3.6f [0x870658] | 27.777779 | none (dead store) |
| 0x852bb0 | 0x8a5390 | g[0xc19658] <- 1.0f [0x858624] / (50.0f [0x87065c] * 50.0f [0x87065c]) | 0.0004 | none (dead store) |
| 0x852bd0 | 0x8a5394 | g[0xc1965c] <- g[0xc19658] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852bf0 | 0x8a5398 | g[0xc19660] <- 0.277778f [0x858630] / 50.0f [0x87065c] | 0.00555556 | none (dead store) |
| 0x852c10 | 0x8a539c | g[0xc19688] <- 0.005f [0x870860] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852c30 | 0x8a53a0 | g[0xc1968c] <- 1.0f [0x858624] / 50.0f [0x870868] | 0.02 | none (dead store) |
| 0x852c50 | 0x8a53a4 | g[0xc19690] <- 100.0f [0x858628] / 3.6f [0x870864] | 27.777779 | none (dead store) |
| 0x852c70 | 0x8a53a8 | g[0xc19694] <- 1.0f [0x858624] / (50.0f [0x870868] * 50.0f [0x870868]) | 0.0004 | none (dead store) |
| 0x852c90 | 0x8a53ac | g[0xc19698] <- g[0xc19694] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852cb0 | 0x8a53b0 | g[0xc1969c] <- 0.277778f [0x858630] / 50.0f [0x870868] | 0.00555556 | none (dead store) |
| 0x852cd0 | 0x8a53b4 | g[0xc196a0] <- 0.005f [0x870970] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852cf0 | 0x8a53b8 | g[0xc196a4] <- 1.0f [0x858624] / 50.0f [0x870978] | 0.02 | none (dead store) |
| 0x852d10 | 0x8a53bc | g[0xc196a8] <- 100.0f [0x858628] / 3.6f [0x870974] | 27.777779 | none (dead store) |
| 0x852d30 | 0x8a53c0 | g[0xc196ac] <- 1.0f [0x858624] / (50.0f [0x870978] * 50.0f [0x870978]) | 0.0004 | none (dead store) |
| 0x852d50 | 0x8a53c4 | g[0xc196b0] <- g[0xc196ac] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852d70 | 0x8a53c8 | g[0xc196b4] <- 0.277778f [0x858630] / 50.0f [0x870978] | 0.00555556 | none (dead store) |
| 0x852d90 | 0x8a53cc | g[0xc196b8] <- 0.005f [0x870a58] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852db0 | 0x8a53d0 | g[0xc196bc] <- 1.0f [0x858624] / 50.0f [0x870a60] | 0.02 | none (dead store) |
| 0x852dd0 | 0x8a53d4 | g[0xc196c0] <- 100.0f [0x858628] / 3.6f [0x870a5c] | 27.777779 | none (dead store) |
| 0x852df0 | 0x8a53d8 | g[0xc196c4] <- 1.0f [0x858624] / (50.0f [0x870a60] * 50.0f [0x870a60]) | 0.0004 | none (dead store) |
| 0x852e10 | 0x8a53dc | g[0xc196c8] <- g[0xc196c4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852e30 | 0x8a53e0 | g[0xc196cc] <- 0.277778f [0x858630] / 50.0f [0x870a60] | 0.00555556 | none (dead store) |
| 0x852e50 | 0x8a53e4 | g[0xc196d0] <- 0.005f [0x870c10] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852e70 | 0x8a53e8 | g[0xc196d4] <- 1.0f [0x858624] / 50.0f [0x870c18] | 0.02 | none (dead store) |
| 0x852e90 | 0x8a53ec | g[0xc196d8] <- 100.0f [0x858628] / 3.6f [0x870c14] | 27.777779 | none (dead store) |
| 0x852eb0 | 0x8a53f0 | g[0xc196dc] <- 1.0f [0x858624] / (50.0f [0x870c18] * 50.0f [0x870c18]) | 0.0004 | none (dead store) |
| 0x852ed0 | 0x8a53f4 | g[0xc196e0] <- g[0xc196dc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852ef0 | 0x8a53f8 | g[0xc196e4] <- 0.277778f [0x858630] / 50.0f [0x870c18] | 0.00555556 | none (dead store) |
| 0x852f30 | 0x8a5400 | g[0xc1976c] <- 0.005f [0x870d20] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852f50 | 0x8a5404 | g[0xc19770] <- 1.0f [0x858624] / 50.0f [0x870d28] | 0.02 | none (dead store) |
| 0x852f70 | 0x8a5408 | g[0xc19774] <- 100.0f [0x858628] / 3.6f [0x870d24] | 27.777779 | none (dead store) |
| 0x852f90 | 0x8a540c | g[0xc19778] <- 1.0f [0x858624] / (50.0f [0x870d28] * 50.0f [0x870d28]) | 0.0004 | none (dead store) |
| 0x852fb0 | 0x8a5410 | g[0xc1977c] <- g[0xc19778] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852fd0 | 0x8a5414 | g[0xc19780] <- 0.277778f [0x858630] / 50.0f [0x870d28] | 0.00555556 | none (dead store) |
| 0x852ff0 | 0x8a5418 | g[0xc19784] <- 0.005f [0x870dfc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853010 | 0x8a541c | g[0xc19788] <- 1.0f [0x858624] / 50.0f [0x870e04] | 0.02 | none (dead store) |
| 0x853030 | 0x8a5420 | g[0xc1978c] <- 100.0f [0x858628] / 3.6f [0x870e00] | 27.777779 | none (dead store) |
| 0x853050 | 0x8a5424 | g[0xc19790] <- 1.0f [0x858624] / (50.0f [0x870e04] * 50.0f [0x870e04]) | 0.0004 | none (dead store) |
| 0x853070 | 0x8a5428 | g[0xc19794] <- g[0xc19790] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853090 | 0x8a542c | g[0xc19798] <- 0.277778f [0x858630] / 50.0f [0x870e04] | 0.00555556 | none (dead store) |
| 0x8530b0 | 0x8a5430 | g[0xc197b0] <- 0.005f [0x870e08] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8530d0 | 0x8a5434 | g[0xc197b4] <- 1.0f [0x858624] / 50.0f [0x870e10] | 0.02 | none (dead store) |
| 0x8530f0 | 0x8a5438 | g[0xc197b8] <- 100.0f [0x858628] / 3.6f [0x870e0c] | 27.777779 | none (dead store) |
| 0x853110 | 0x8a543c | g[0xc197bc] <- 1.0f [0x858624] / (50.0f [0x870e10] * 50.0f [0x870e10]) | 0.0004 | none (dead store) |
| 0x853130 | 0x8a5440 | g[0xc197c0] <- g[0xc197bc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853150 | 0x8a5444 | g[0xc197c4] <- 0.277778f [0x858630] / 50.0f [0x870e10] | 0.00555556 | none (dead store) |
| 0x853180 | 0x8a544c | g[0xc1a300] <- 0.005f [0x870e14] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8531a0 | 0x8a5450 | g[0xc1a304] <- 1.0f [0x858624] / 50.0f [0x870e1c] | 0.02 | none (dead store) |
| 0x8531c0 | 0x8a5454 | g[0xc1a308] <- 100.0f [0x858628] / 3.6f [0x870e18] | 27.777779 | none (dead store) |
| 0x8531e0 | 0x8a5458 | g[0xc1a30c] <- 1.0f [0x858624] / (50.0f [0x870e1c] * 50.0f [0x870e1c]) | 0.0004 | none (dead store) |
| 0x853200 | 0x8a545c | g[0xc1a310] <- g[0xc1a30c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853220 | 0x8a5460 | g[0xc1a314] <- 0.277778f [0x858630] / 50.0f [0x870e1c] | 0.00555556 | none (dead store) |

## Other initialisers and destructors

## 0x850f50 __init_g_c09860  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856780) only.

## 0x8514e0 __init_g_InterestingEvents  (catalogue: unknown/REA, port level 1)
- ECX = 0xC0B058; call 0x6023A0 CInterestingEvents::Constructor; _atexit 0x821D1E(0x856880 CInterestingEvents::Destructor).

## 0x8526b0 __init_g_c18cf0  (catalogue: unknown/REA, port level 0)
- dword [0xC18CF0] = dword [0x86F668] (.rdata, f32 1.0f [0x86f668], u32 0x3F800000): a float global initialised from a constant. Readers: 0x46e8c0 in 0x46d050 CRunningScript::ProcessCommands1800To1899, 0x490eb4 in 0x490db0 CRunningScript::ProcessCommands1500To1599, 0x4b9684 in 0x4b9470 CEventHandler::ComputeDeadPedResponse, 0x4b9ba6 in 0x4b9b50 CEventHandler::ComputePedToFleeResponse, 0x4bab7b in 0x4baad0 CEventHandler::ComputeLowAngerAtPlayerResponse, 0x4bacbb in 0x4bac10 CEventHandler::ComputeHighAngerAtPlayerResponse, 0x4bb46e in 0x4bb2e0 CEventHandler::ComputeVehicleOnFireResponse, 0x4bb53f in 0x4bb2e0 CEventHandler::ComputeVehicleOnFireResponse, 0x4bb60b in 0x4bb2e0 CEventHandler::ComputeVehicleOnFireResponse, 0x4bba2c in 0x4bb800 CEventHandler::ComputeSpecialResponse, 0x4bc10f in 0x4bc050 CEventHandler::ComputeSeenCopResponse, 0x4bc28e in 0x4bc230 CEventHandler::ComputeDangerResponse, 0x4bc334 in 0x4bc230 CEventHandler::ComputeDangerResponse, 0x4bc85f in 0x4bc710 CEventHandler::ComputeShotFiredResponse, 0x4bc8af in 0x4bc710 CEventHandler::ComputeShotFiredResponse, 0x4bc9ee in 0x4bc710 CEventHandler::ComputeShotFiredResponse, 0x4bcaa5 in 0x4bc710 CEventHandler::ComputeShotFiredResponse, 0x4bccab in 0x4bcc30 CEventHandler::ComputeDraggedOutCarResponse, 0x4bd06a in 0x4bcc30 CEventHandler::ComputeDraggedOutCarResponse, 0x4bd1eb in 0x4bcc30 CEventHandler::ComputeDraggedOutCarResponse, 0x4bfbff in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4bfc94 in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4bfcf2 in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4bfd48 in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4bfe7b in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4bff82 in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4c005a in 0x4bf9b0 CEventHandler::ComputePersonalityResponseToDamage, 0x4c161d in 0x4c1590 CEventHandler::ComputePedEnteredVehicleResponse, 0x4c1c99 in 0x4c19a0 CEventHandler::ComputePedThreatResponse, 0x4c20d2 in 0x4c19a0 CEventHandler::ComputePedThreatResponse, 0x4c22ec in 0x4c19a0 CEventHandler::ComputePedThreatResponse, 0x4c28d6 in 0x4c2840 CEventHandler::ComputeGunAimedAtResponse, 0x4c2c7a in 0x4c2840 CEventHandler::ComputeGunAimedAtResponse, 0x4c2cd8 in 0x4c2840 CEventHandler::ComputeGunAimedAtResponse, 0x4c2d21 in 0x4c2840 CEventHandler::ComputeGunAimedAtResponse, 0x4c3380 in 0x4c2fc0 CEventHandler::ComputeVehicleDamageResponse, 0x4c34b7 in 0x4c3430 CEventHandler::ComputeGotKnockedOverByCarResponse, 0x4c3566 in 0x4c3430 CEventHandler::ComputeGotKnockedOverByCarResponse, 0x4c368d in 0x4c35f0 CEventHandler::ComputeSeenPanickedPedResponse, 0x5f9211 in 0x5f90a0 CGroupEventHandler::ComputeResponseLeaderExitedCar, 0x5f93e3 in 0x5f90a0 CGroupEventHandler::ComputeResponseLeaderExitedCar, 0x5fa1e6 in 0x5fa130 CGroupEventHandler::ComputeFleePedResponse, 0x60c2e3 in 0x60c1e0 CPlayerPed::KeepAreaAroundPlayerClear, 0x6334d5 in 0x633470 CTaskComplexOnFire::CreateSubTask, 0x65fcf2 in 0x65fc80 CTaskGangHassleVehicle::CreateNextSubTask, 0x697eb3 in 0x697b70 CTaskComplexReactToGunAimedAt::CreateSubTask.

## 0x852850 __init_852850  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856940) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (AI).

## 0x852860 __init_g_c18db4  (catalogue: unknown/REA, port level 0)
- dword [0xC18DB4] = dword [0x86FC78] (.rdata, f32 0.5f [0x86fc78], u32 0x3F000000): a float global initialised from a constant. Readers: 0x491362 in 0x490db0 CRunningScript::ProcessCommands1500To1599, 0x63ceb8 in 0x63ce80 CTaskComplexDriveToPoint::CreateSubTaskCannotGetInCar.

## 0x852870 __init_g_c18db8  (catalogue: unknown/REA, port level 1)
- ECX = 0xC18DB8; call 0x66D440 (ctor: [this+0] = 0; `eh vector constructor iterator` 0x821D30 (stdcall: arr, elemSize, count, ctor, dtor; ascending)(this+4, elemSize=0x28, count=8, ctor=0x46AE30, dtor=0x46A380); then bytes this+0x4, +0x1C, +0x2C, +0x44, +0x54, +0x6C, +0x7C, +0x94, +0xA4, +0xBC, +0xCC, +0xE4, +0xF4, +0x10C, +0x11C, +0x134 = 0; EAX = this); _atexit 0x821D1E(0x856950). Object type unknown (AI task-region TU).

## 0x856780 __dtor_g_c09860  (catalogue: unknown/REA, port level 1)
- if dword [0xC09860] != 0: operator delete([0xC09860]) (0x8214BD, cdecl); then [0xC09860] = [0xC09864] = [0xC09868] = 0 (std::vector-like begin/end/capacity triple at 0xC09860; the element type is unknown; neighbouring AI initialisers). 

## 0x856940 __dtor_852850  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x852850).

## 0x856950 __dtor_g_c18db8  (catalogue: unknown/REA, port level 1)
- ECX = 0xC18DB8; tail jump to 0x6677A0 (destructor of the same unknown type).

## 0x8569a0 __dtor_local_static_CMatrix_c195e8  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0xC195E8, registered on first use by CTaskSimpleClimb::ScanToGrabSectorList<CPtrListSingleLink<CPhysical*>> (caller list: 0x67de10). Body: ECX = 0xC195E8; tail jump to 0x59ACD0 CMatrix::~CMatrix.
