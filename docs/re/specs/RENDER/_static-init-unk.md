# RENDER: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84b510 | 0x8a44c0 | g[0xa9ade8] <- 0.005f [0x85a5d0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b530 | 0x8a44c4 | g[0xa9adec] <- 1.0f [0x858624] / 50.0f [0x85a5d8] | 0.02 | none (dead store) |
| 0x84b550 | 0x8a44c8 | g[0xa9adf0] <- 100.0f [0x858628] / 3.6f [0x85a5d4] | 27.777779 | none (dead store) |
| 0x84b570 | 0x8a44cc | g[0xa9adf4] <- 1.0f [0x858624] / (50.0f [0x85a5d8] * 50.0f [0x85a5d8]) | 0.0004 | none (dead store) |
| 0x84b590 | 0x8a44d0 | g[0xa9adf8] <- g[0xa9adf4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b5b0 | 0x8a44d4 | g[0xa9adfc] <- 0.277778f [0x858630] / 50.0f [0x85a5d8] | 0.00555556 | none (dead store) |
| 0x84b630 | 0x8a44e4 | g[0xa9ae78] <- 0.005f [0x85a990] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e320 | 0x8a4a88 | g[0xb744c8] <- 0.005f [0x863c44] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e340 | 0x8a4a8c | g[0xb744cc] <- 1.0f [0x858624] / 50.0f [0x863c4c] | 0.02 | none (dead store) |
| 0x84e360 | 0x8a4a90 | g[0xb744d0] <- 100.0f [0x858628] / 3.6f [0x863c48] | 27.777779 | none (dead store) |
| 0x84e380 | 0x8a4a94 | g[0xb744d4] <- 1.0f [0x858624] / (50.0f [0x863c4c] * 50.0f [0x863c4c]) | 0.0004 | none (dead store) |
| 0x84e3a0 | 0x8a4a98 | g[0xb744d8] <- g[0xb744d4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e3c0 | 0x8a4a9c | g[0xb744dc] <- 0.277778f [0x858630] / 50.0f [0x863c4c] | 0.00555556 | none (dead store) |
| 0x84e3e0 | 0x8a4aa0 | g[0xb745a4] <- 0.005f [0x863de0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e400 | 0x8a4aa4 | g[0xb745a8] <- 1.0f [0x858624] / 50.0f [0x863de8] | 0.02 | none (dead store) |
| 0x84e420 | 0x8a4aa8 | g[0xb745ac] <- 100.0f [0x858628] / 3.6f [0x863de4] | 27.777779 | none (dead store) |
| 0x84e440 | 0x8a4aac | g[0xb745b0] <- 1.0f [0x858624] / (50.0f [0x863de8] * 50.0f [0x863de8]) | 0.0004 | none (dead store) |
| 0x84e460 | 0x8a4ab0 | g[0xb745b4] <- g[0xb745b0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e480 | 0x8a4ab4 | g[0xb745b8] <- 0.277778f [0x858630] / 50.0f [0x863de8] | 0.00555556 | none (dead store) |
| 0x84e4a0 | 0x8a4ab8 | g[0xb76858] <- 0.005f [0x863dfc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e4c0 | 0x8a4abc | g[0xb7685c] <- 1.0f [0x858624] / 50.0f [0x863e04] | 0.02 | none (dead store) |
| 0x84e4e0 | 0x8a4ac0 | g[0xb76860] <- 100.0f [0x858628] / 3.6f [0x863e00] | 27.777779 | none (dead store) |
| 0x84e500 | 0x8a4ac4 | g[0xb76864] <- 1.0f [0x858624] / (50.0f [0x863e04] * 50.0f [0x863e04]) | 0.0004 | none (dead store) |
| 0x84e520 | 0x8a4ac8 | g[0xb76868] <- g[0xb76864] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e540 | 0x8a4acc | g[0xb7686c] <- 0.277778f [0x858630] / 50.0f [0x863e04] | 0.00555556 | none (dead store) |
| 0x84e620 | 0x8a4ae8 | g[0xb79518] <- 0.005f [0x863e48] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e640 | 0x8a4aec | g[0xb7951c] <- 1.0f [0x858624] / 50.0f [0x863e50] | 0.02 | none (dead store) |
| 0x84e660 | 0x8a4af0 | g[0xb79520] <- 100.0f [0x858628] / 3.6f [0x863e4c] | 27.777779 | none (dead store) |
| 0x84e680 | 0x8a4af4 | g[0xb79524] <- 1.0f [0x858624] / (50.0f [0x863e50] * 50.0f [0x863e50]) | 0.0004 | none (dead store) |
| 0x84e6a0 | 0x8a4af8 | g[0xb79528] <- g[0xb79524] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e6c0 | 0x8a4afc | g[0xb7952c] <- 0.277778f [0x858630] / 50.0f [0x863e50] | 0.00555556 | none (dead store) |
| 0x84e6f0 | 0x8a4b04 | g[0xb7c488] <- 0.005f [0x864dfc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e710 | 0x8a4b08 | g[0xb7c48c] <- 1.0f [0x858624] / 50.0f [0x864e04] | 0.02 | none (dead store) |
| 0x84e730 | 0x8a4b0c | g[0xb7c490] <- 100.0f [0x858628] / 3.6f [0x864e00] | 27.777779 | none (dead store) |
| 0x84e750 | 0x8a4b10 | g[0xb7c494] <- 1.0f [0x858624] / (50.0f [0x864e04] * 50.0f [0x864e04]) | 0.0004 | none (dead store) |
| 0x84e770 | 0x8a4b14 | g[0xb7c498] <- g[0xb7c494] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e790 | 0x8a4b18 | g[0xb7c49c] <- 0.277778f [0x858630] / 50.0f [0x864e04] | 0.00555556 | none (dead store) |
| 0x84fd00 | 0x8a4db8 | g[0xbb7c98] <- 0.005f [0x866fdc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fd20 | 0x8a4dbc | g[0xbb7c9c] <- 1.0f [0x858624] / 50.0f [0x866fe4] | 0.02 | none (dead store) |
| 0x84fd40 | 0x8a4dc0 | g[0xbb7ca0] <- 100.0f [0x858628] / 3.6f [0x866fe0] | 27.777779 | none (dead store) |
| 0x84fd60 | 0x8a4dc4 | g[0xbb7ca4] <- 1.0f [0x858624] / (50.0f [0x866fe4] * 50.0f [0x866fe4]) | 0.0004 | none (dead store) |
| 0x84fd80 | 0x8a4dc8 | g[0xbb7ca8] <- g[0xbb7ca4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84fda0 | 0x8a4dcc | g[0xbb7cac] <- 0.277778f [0x858630] / 50.0f [0x866fe4] | 0.00555556 | none (dead store) |
| 0x850800 | 0x8a4f18 | g[0xc02dc4] <- 0.005f [0x86bf04] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850820 | 0x8a4f1c | g[0xc02dc8] <- 1.0f [0x858624] / 50.0f [0x86bf0c] | 0.02 | none (dead store) |
| 0x850840 | 0x8a4f20 | g[0xc02dcc] <- 100.0f [0x858628] / 3.6f [0x86bf08] | 27.777779 | none (dead store) |
| 0x850860 | 0x8a4f24 | g[0xc02dd0] <- 1.0f [0x858624] / (50.0f [0x86bf0c] * 50.0f [0x86bf0c]) | 0.0004 | none (dead store) |
| 0x850880 | 0x8a4f28 | g[0xc02dd4] <- g[0xc02dd0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8508a0 | 0x8a4f2c | g[0xc02dd8] <- 0.277778f [0x858630] / 50.0f [0x86bf0c] | 0.00555556 | none (dead store) |
| 0x8508f0 | 0x8a4f34 | g[0xc03a40] <- 0.005f [0x86bf10] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850ac0 | 0x8a4f68 | g[0xc0919c] <- 100.0f [0x858628] / 3.6f [0x86c114] | 27.777779 | 0x720c3d in 0x720930 CSkidmarks::RegisterOne- |
| 0x853e50 | 0x8a55e0 | g[0xc1df18] <- 0.005f [0x871fe0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853e70 | 0x8a55e4 | g[0xc1df1c] <- 1.0f [0x858624] / 50.0f [0x871fe8] | 0.02 | none (dead store) |
| 0x853e90 | 0x8a55e8 | g[0xc1df20] <- 100.0f [0x858628] / 3.6f [0x871fe4] | 27.777779 | none (dead store) |
| 0x853eb0 | 0x8a55ec | g[0xc1df24] <- 1.0f [0x858624] / (50.0f [0x871fe8] * 50.0f [0x871fe8]) | 0.0004 | none (dead store) |
| 0x853ed0 | 0x8a55f0 | g[0xc1df28] <- g[0xc1df24] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853ef0 | 0x8a55f4 | g[0xc1df2c] <- 0.277778f [0x858630] / 50.0f [0x871fe8] | 0.00555556 | none (dead store) |
| 0x854430 | 0x8a5698 | g[0xc38050] <- 1.0f [0x858624] / 50.0f [0x872360] | 0.02 | none (dead store) |
| 0x854450 | 0x8a569c | g[0xc38054] <- 100.0f [0x858628] / 3.6f [0x87235c] | 27.777779 | none (dead store) |
| 0x854470 | 0x8a56a0 | g[0xc38058] <- 1.0f [0x858624] / (50.0f [0x872360] * 50.0f [0x872360]) | 0.0004 | 0xcfe116 in no catalogue function |
| 0x854490 | 0x8a56a4 | g[0xc3805c] <- g[0xc38058] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8544b0 | 0x8a56a8 | g[0xc38060] <- 0.277778f [0x858630] / 50.0f [0x872360] | 0.00555556 | none (dead store) |
| 0x8544f0 | 0x8a56b0 | g[0xc3a1e4] <- 0.005f [0x872514] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854510 | 0x8a56b4 | g[0xc3a1e8] <- 1.0f [0x858624] / 50.0f [0x87251c] | 0.02 | none (dead store) |
| 0x854530 | 0x8a56b8 | g[0xc3a1ec] <- 100.0f [0x858628] / 3.6f [0x872518] | 27.777779 | none (dead store) |
| 0x854550 | 0x8a56bc | g[0xc3a1f0] <- 1.0f [0x858624] / (50.0f [0x87251c] * 50.0f [0x87251c]) | 0.0004 | none (dead store) |
| 0x854570 | 0x8a56c0 | g[0xc3a1f4] <- g[0xc3a1f0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854590 | 0x8a56c4 | g[0xc3a1f8] <- 0.277778f [0x858630] / 50.0f [0x87251c] | 0.00555556 | none (dead store) |
| 0x854610 | 0x8a56d0 | g[0xc3e03c] <- 0.005f [0x872520] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854630 | 0x8a56d4 | g[0xc3e040] <- 1.0f [0x858624] / 50.0f [0x872528] | 0.02 | none (dead store) |
| 0x854650 | 0x8a56d8 | g[0xc3e044] <- 100.0f [0x858628] / 3.6f [0x872524] | 27.777779 | none (dead store) |
| 0x854670 | 0x8a56dc | g[0xc3e048] <- 1.0f [0x858624] / (50.0f [0x872528] * 50.0f [0x872528]) | 0.0004 | none (dead store) |
| 0x854690 | 0x8a56e0 | g[0xc3e04c] <- g[0xc3e048] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8546b0 | 0x8a56e4 | g[0xc3e050] <- 0.277778f [0x858630] / 50.0f [0x872528] | 0.00555556 | none (dead store) |
| 0x8546d0 | 0x8a56e8 | g[0xc3efb4] <- 0.005f [0x8725b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854710 | 0x8a56f0 | g[0xc3f044] <- 0.005f [0x8725b8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854730 | 0x8a56f4 | g[0xc3f0d8] <- 0.005f [0x8725f8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854750 | 0x8a56f8 | g[0xc402e8] <- 0.005f [0x872620] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854770 | 0x8a56fc | g[0xc402ec] <- 1.0f [0x858624] / 50.0f [0x872628] | 0.02 | none (dead store) |
| 0x854790 | 0x8a5700 | g[0xc402f0] <- 100.0f [0x858628] / 3.6f [0x872624] | 27.777779 | none (dead store) |
| 0x8547b0 | 0x8a5704 | g[0xc402f4] <- 1.0f [0x858624] / (50.0f [0x872628] * 50.0f [0x872628]) | 0.0004 | none (dead store) |
| 0x8547d0 | 0x8a5708 | g[0xc402f8] <- g[0xc402f4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8547f0 | 0x8a570c | g[0xc402fc] <- 0.277778f [0x858630] / 50.0f [0x872628] | 0.00555556 | none (dead store) |
| 0x8548c0 | 0x8a572c | g[0xc40338] <- 0.005f [0x872664] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8548e0 | 0x8a5730 | g[0xc4033c] <- 1.0f [0x858624] / 50.0f [0x87266c] | 0.02 | none (dead store) |
| 0x854900 | 0x8a5734 | g[0xc40340] <- 100.0f [0x858628] / 3.6f [0x872668] | 27.777779 | none (dead store) |
| 0x854920 | 0x8a5738 | g[0xc40344] <- 1.0f [0x858624] / (50.0f [0x87266c] * 50.0f [0x87266c]) | 0.0004 | none (dead store) |
| 0x854940 | 0x8a573c | g[0xc40348] <- g[0xc40344] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854960 | 0x8a5740 | g[0xc4034c] <- 0.277778f [0x858630] / 50.0f [0x87266c] | 0.00555556 | none (dead store) |
| 0x8549a0 | 0x8a5748 | g[0xc40418] <- 0.005f [0x872674] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8549c0 | 0x8a574c | g[0xc4041c] <- 1.0f [0x858624] / 50.0f [0x87267c] | 0.02 | none (dead store) |
| 0x8549e0 | 0x8a5750 | g[0xc40420] <- 100.0f [0x858628] / 3.6f [0x872678] | 27.777779 | none (dead store) |
| 0x854a20 | 0x8a5758 | g[0xc40428] <- g[0xc40424] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854a40 | 0x8a575c | g[0xc4042c] <- 0.277778f [0x858630] / 50.0f [0x87267c] | 0.00555556 | none (dead store) |
| 0x854a80 | 0x8a5764 | g[0xc6a160] <- 0.005f [0x87274c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854ac0 | 0x8a576c | g[0xc6a180] <- 1.0f [0x858624] / 50.0f [0x872758] | 0.02 | none (dead store) |
| 0x854ae0 | 0x8a5770 | g[0xc6a184] <- 100.0f [0x858628] / 3.6f [0x872754] | 27.777779 | none (dead store) |
| 0x854b00 | 0x8a5774 | g[0xc6a188] <- 1.0f [0x858624] / (50.0f [0x872758] * 50.0f [0x872758]) | 0.0004 | none (dead store) |
| 0x854b20 | 0x8a5778 | g[0xc6a18c] <- g[0xc6a188] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854b40 | 0x8a577c | g[0xc6a190] <- 0.277778f [0x858630] / 50.0f [0x872758] | 0.00555556 | none (dead store) |
| 0x854b90 | 0x8a5784 | g[0xc6a8a8] <- 0.005f [0x87281c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854bb0 | 0x8a5788 | g[0xc6aa84] <- 0.005f [0x87282c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854bd0 | 0x8a578c | g[0xc6aa88] <- 1.0f [0x858624] / 50.0f [0x872834] | 0.02 | none (dead store) |
| 0x854bf0 | 0x8a5790 | g[0xc6aa8c] <- 100.0f [0x858628] / 3.6f [0x872830] | 27.777779 | none (dead store) |
| 0x854c10 | 0x8a5794 | g[0xc6aa90] <- 1.0f [0x858624] / (50.0f [0x872834] * 50.0f [0x872834]) | 0.0004 | none (dead store) |
| 0x854c30 | 0x8a5798 | g[0xc6aa94] <- g[0xc6aa90] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854c50 | 0x8a579c | g[0xc6aa98] <- 0.277778f [0x858630] / 50.0f [0x872834] | 0.00555556 | none (dead store) |
| 0x854e20 | 0x8a57d8 | g[0xc71bdc] <- 0.005f [0x872938] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854e40 | 0x8a57dc | g[0xc71be0] <- 1.0f [0x858624] / 50.0f [0x872940] | 0.02 | none (dead store) |
| 0x854e60 | 0x8a57e0 | g[0xc71be4] <- 100.0f [0x858628] / 3.6f [0x87293c] | 27.777779 | none (dead store) |
| 0x854e80 | 0x8a57e4 | g[0xc71be8] <- 1.0f [0x858624] / (50.0f [0x872940] * 50.0f [0x872940]) | 0.0004 | none (dead store) |
| 0x854ea0 | 0x8a57e8 | g[0xc71bec] <- g[0xc71be8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854ec0 | 0x8a57ec | g[0xc71bf0] <- 0.277778f [0x858630] / 50.0f [0x872940] | 0.00555556 | none (dead store) |
| 0x854f10 | 0x8a57f4 | g[0xc73c34] <- 0.005f [0x872948] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854f30 | 0x8a57f8 | g[0xc73c38] <- 1.0f [0x858624] / 50.0f [0x872950] | 0.02 | none (dead store) |
| 0x854f50 | 0x8a57fc | g[0xc73c3c] <- 100.0f [0x858628] / 3.6f [0x87294c] | 27.777779 | none (dead store) |
| 0x854f70 | 0x8a5800 | g[0xc73c40] <- 1.0f [0x858624] / (50.0f [0x872950] * 50.0f [0x872950]) | 0.0004 | none (dead store) |
| 0x854f90 | 0x8a5804 | g[0xc73c44] <- g[0xc73c40] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854fb0 | 0x8a5808 | g[0xc73c48] <- 0.277778f [0x858630] / 50.0f [0x872950] | 0.00555556 | none (dead store) |
| 0x854ff0 | 0x8a5810 | g[0xc73c60] <- 0.005f [0x872974] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855010 | 0x8a5814 | g[0xc785f4] <- 0.005f [0x872978] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855030 | 0x8a5818 | g[0xc785f8] <- 1.0f [0x858624] / 50.0f [0x872980] | 0.02 | none (dead store) |
| 0x855050 | 0x8a581c | g[0xc785fc] <- 100.0f [0x858628] / 3.6f [0x87297c] | 27.777779 | none (dead store) |
| 0x855070 | 0x8a5820 | g[0xc78600] <- 1.0f [0x858624] / (50.0f [0x872980] * 50.0f [0x872980]) | 0.0004 | none (dead store) |
| 0x855090 | 0x8a5824 | g[0xc78604] <- g[0xc78600] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8550b0 | 0x8a5828 | g[0xc78608] <- 0.277778f [0x858630] / 50.0f [0x872980] | 0.00555556 | none (dead store) |
| 0x8550d0 | 0x8a582c | g[0xc799b0] <- 0.005f [0x872984] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8550f0 | 0x8a5830 | g[0xc799b4] <- 1.0f [0x858624] / 50.0f [0x87298c] | 0.02 | none (dead store) |
| 0x855110 | 0x8a5834 | g[0xc799b8] <- 100.0f [0x858628] / 3.6f [0x872988] | 27.777779 | none (dead store) |
| 0x855130 | 0x8a5838 | g[0xc799bc] <- 1.0f [0x858624] / (50.0f [0x87298c] * 50.0f [0x87298c]) | 0.0004 | none (dead store) |
| 0x855150 | 0x8a583c | g[0xc799c0] <- g[0xc799bc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855170 | 0x8a5840 | g[0xc799c4] <- 0.277778f [0x858630] / 50.0f [0x87298c] | 0.00555556 | none (dead store) |
| 0x855190 | 0x8a5844 | g[0xc79a8c] <- 0.005f [0x872990] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8551b0 | 0x8a5848 | g[0xc79a90] <- 1.0f [0x858624] / 50.0f [0x872998] | 0.02 | none (dead store) |
| 0x8551d0 | 0x8a584c | g[0xc79a94] <- 100.0f [0x858628] / 3.6f [0x872994] | 27.777779 | none (dead store) |
| 0x8551f0 | 0x8a5850 | g[0xc79a98] <- 1.0f [0x858624] / (50.0f [0x872998] * 50.0f [0x872998]) | 0.0004 | none (dead store) |
| 0x855210 | 0x8a5854 | g[0xc79a9c] <- g[0xc79a98] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855230 | 0x8a5858 | g[0xc79aa0] <- 0.277778f [0x858630] / 50.0f [0x872998] | 0.00555556 | none (dead store) |
| 0x855250 | 0x8a585c | g[0xc7c72c] <- 0.005f [0x8729b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855270 | 0x8a5860 | g[0xc7c730] <- 1.0f [0x858624] / 50.0f [0x8729b8] | 0.02 | none (dead store) |
| 0x855290 | 0x8a5864 | g[0xc7c734] <- 100.0f [0x858628] / 3.6f [0x8729b4] | 27.777779 | none (dead store) |
| 0x8552b0 | 0x8a5868 | g[0xc7c738] <- 1.0f [0x858624] / (50.0f [0x8729b8] * 50.0f [0x8729b8]) | 0.0004 | none (dead store) |
| 0x8552d0 | 0x8a586c | g[0xc7c73c] <- g[0xc7c738] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8552f0 | 0x8a5870 | g[0xc7c740] <- 0.277778f [0x858630] / 50.0f [0x8729b8] | 0.00555556 | none (dead store) |
| 0x855360 | 0x8a587c | g[0xc80724] <- 0.005f [0x872a1c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855380 | 0x8a5880 | g[0xc80728] <- 1.0f [0x858624] / 50.0f [0x872a24] | 0.02 | none (dead store) |
| 0x8553a0 | 0x8a5884 | g[0xc8072c] <- 100.0f [0x858628] / 3.6f [0x872a20] | 27.777779 | none (dead store) |
| 0x8553c0 | 0x8a5888 | g[0xc80730] <- 1.0f [0x858624] / (50.0f [0x872a24] * 50.0f [0x872a24]) | 0.0004 | none (dead store) |
| 0x8553e0 | 0x8a588c | g[0xc80734] <- g[0xc80730] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855400 | 0x8a5890 | g[0xc80738] <- 0.277778f [0x858630] / 50.0f [0x872a24] | 0.00555556 | none (dead store) |
| 0x855450 | 0x8a5898 | g[0xc81344] <- 0.005f [0x872a68] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855470 | 0x8a589c | g[0xc81348] <- 1.0f [0x858624] / 50.0f [0x872a70] | 0.02 | none (dead store) |
| 0x855490 | 0x8a58a0 | g[0xc8134c] <- 100.0f [0x858628] / 3.6f [0x872a6c] | 27.777779 | none (dead store) |
| 0x8554b0 | 0x8a58a4 | g[0xc81350] <- 1.0f [0x858624] / (50.0f [0x872a70] * 50.0f [0x872a70]) | 0.0004 | none (dead store) |
| 0x8554d0 | 0x8a58a8 | g[0xc81354] <- g[0xc81350] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8554f0 | 0x8a58ac | g[0xc81358] <- 0.277778f [0x858630] / 50.0f [0x872a70] | 0.00555556 | none (dead store) |
| 0x855530 | 0x8a58b4 | g[0xc81454] <- 0.005f [0x872a90] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855550 | 0x8a58b8 | g[0xc815bc] <- 0.005f [0x872a94] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

## Other initialisers and destructors

## 0x84b5d0 __init_g_fx  (catalogue: unknown/REA, port level 2)
- ECX = 0xA9AE00 (g_fx, Fx_c); call 0x49E620 Fx_c::Constructor; _atexit 0x821D1E(0x8561D0).

## 0x84b650 __init_g_fxMan  (catalogue: unknown/REA, port level 2)
- ECX = 0xA9AE80 (g_fxMan, FxManager_c); call 0x4A9470 FxManager_c::Constructor; _atexit 0x821D1E(0x8561E0).

## 0x84e6e0 __init_84e6e0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x8564a0) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (RENDER).

## 0x84fe20 __init_gOctTreeBase  (catalogue: unknown/REA, port level 1)
- ECX = 0xBC1290 (gOctTreeBase, gr StaticRef<COctTreeBase>); call 0x5A7570 COctTreeBase::COctTreeBase (EAX = this; [+4],[+0x1C],[+0x20],[+0x24] = 0; dwords at +0xA,+0xE,+0x12,+0x16 = -1 (unaligned); byte [+8] = 0; vptr [+0] = 0x867178); _atexit 0x821D1E(0x856690). Subsystem: COctTree is the texture-palette quantiser (gr Core/OctTree.h); proposed RENDER, UNCLEAR.

## 0x84fe60 __init_COctTree__ms_octTreePool  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x8566A0) only (pool storage needs no construction).

## 0x854980 __init_g_realTimeShadowMan  (catalogue: unknown/REA, port level 1)
- ECX = 0xC40350 (g_realTimeShadowMan, gr StaticRef<CRealTimeShadowManager>); call 0x706770 (CRealTimeShadowManager ctor); _atexit 0x821D1E(0x856AD0).

## 0x8561d0 __dtor_g_fx  (catalogue: unknown/REA, port level 2)
- ECX = 0xA9AE00; tail jump to 0x49E630 Fx_c::Destructor.

## 0x8561e0 __dtor_g_fxMan  (catalogue: unknown/REA, port level 2)
- ECX = 0xA9AE80; tail jump to 0x4A90A0 FxManager_c::Destructor.

## 0x8564a0 __dtor_84e6e0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84e6e0).

## 0x856690 __dtor_gOctTreeBase  (catalogue: unknown/REA, port level 2)
- ECX = 0xBC1290; jmp 0x5A4140 -> jmp 0x5A7400: [ECX] = vptr 0x867170 (COctTree vtable); tail jump to 0x5A6FC0 COctTree::empty.

## 0x8566a0 __dtor_COctTree__ms_octTreePool  (catalogue: unknown/REA, port level 1)
- Pool teardown of COctTree::ms_octTreePool (0xBC12E4: +0 objects ptr, +4 flags ptr, +8 i32 size, +0xC i32 top, +0x10 u8 ownsAllocations): if [0xBC12EC] (size) > 0 (signed, 0x8566A8 `cmp eax, ebx; jle`): if byte [0xBC12F4] != 0: operator delete([0xBC12E4]) then operator delete([0xBC12E8]) (0x8214BD, cdecl, `add esp, 8` once after both); then (still inside size > 0) [0xBC12E4] = [0xBC12E8] = [0xBC12EC] = [0xBC12F0] = 0. Same as gr CPool::Flush order.

## 0x856ad0 __dtor_g_realTimeShadowMan  (catalogue: unknown/REA, port level 5)
- ECX = 0xC40350; tail jump to 0x7067A0 (CRealTimeShadowManager dtor).

## 0x856bc0 __dtor_local_static_CMatrix_c803e8  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0xC803E8, registered on first use by CSpecialFX::AddWeaponStreak (caller list: 0x7233f0). Body: ECX = 0xC803E8; tail jump to 0x59ACD0 CMatrix::~CMatrix.
