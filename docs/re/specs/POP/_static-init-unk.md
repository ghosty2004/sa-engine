# POP: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x8497a0 | 0x8a4118 | g[0x969058] <- 0.005f [0x858c68] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8497c0 | 0x8a411c | g[0x96905c] <- 1.0f [0x858624] / 50.0f [0x858c70] | 0.02 | none (dead store) |
| 0x8497e0 | 0x8a4120 | g[0x969060] <- 100.0f [0x858628] / 3.6f [0x858c6c] | 27.777779 | none (dead store) |
| 0x849800 | 0x8a4124 | g[0x969064] <- 1.0f [0x858624] / (50.0f [0x858c70] * 50.0f [0x858c70]) | 0.0004 | none (dead store) |
| 0x849820 | 0x8a4128 | g[0x969068] <- g[0x969064] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849840 | 0x8a412c | g[0x96906c] <- 0.277778f [0x858630] / 50.0f [0x858c70] | 0.00555556 | none (dead store) |
| 0x8499e0 | 0x8a4160 | g[0x969a24] <- 0.005f [0x8594a0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849a20 | 0x8a4168 | g[0x969a2c] <- 100.0f [0x858628] / 3.6f [0x8594a4] | 27.777779 | none (dead store) |
| 0x849a40 | 0x8a416c | g[0x969a30] <- 1.0f [0x858624] / (50.0f [0x8594a8] * 50.0f [0x8594a8]) | 0.0004 | none (dead store) |
| 0x849a60 | 0x8a4170 | g[0x969a34] <- g[0x969a30] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849a80 | 0x8a4174 | g[0x969a38] <- 0.277778f [0x858630] / 50.0f [0x8594a8] | 0.00555556 | none (dead store) |
| 0x849c20 | 0x8a41a8 | g[0x96a8bc] <- 0.005f [0x859814] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849c40 | 0x8a41ac | g[0x96a8c0] <- 1.0f [0x858624] / 50.0f [0x85981c] | 0.02 | none (dead store) |
| 0x849c60 | 0x8a41b0 | g[0x96a8c4] <- 100.0f [0x858628] / 3.6f [0x859818] | 27.777779 | none (dead store) |
| 0x849c80 | 0x8a41b4 | g[0x96a8c8] <- 1.0f [0x858624] / (50.0f [0x85981c] * 50.0f [0x85981c]) | 0.0004 | none (dead store) |
| 0x849ca0 | 0x8a41b8 | g[0x96a8cc] <- g[0x96a8c8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849cc0 | 0x8a41bc | g[0x96a8d0] <- 0.277778f [0x858630] / 50.0f [0x85981c] | 0.00555556 | none (dead store) |
| 0x849ec0 | 0x8a41f8 | g[0x96ea7c] <- 0.005f [0x859a60] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849ee0 | 0x8a41fc | g[0x96ea80] <- 1.0f [0x858624] / 50.0f [0x859a68] | 0.02 | none (dead store) |
| 0x849f00 | 0x8a4200 | g[0x96ea84] <- 100.0f [0x858628] / 3.6f [0x859a64] | 27.777779 | none (dead store) |
| 0x849f20 | 0x8a4204 | g[0x96ea88] <- 1.0f [0x858624] / (50.0f [0x859a68] * 50.0f [0x859a68]) | 0.0004 | none (dead store) |
| 0x849f40 | 0x8a4208 | g[0x96ea8c] <- g[0x96ea88] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849f60 | 0x8a420c | g[0x96ea90] <- 0.277778f [0x858630] / 50.0f [0x859a68] | 0.00555556 | none (dead store) |
| 0x849f80 | 0x8a4210 | g[0x96f034] <- 0.005f [0x859a80] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849fa0 | 0x8a4214 | g[0x96f038] <- 1.0f [0x858624] / 50.0f [0x859a88] | 0.02 | none (dead store) |
| 0x849fc0 | 0x8a4218 | g[0x96f03c] <- 100.0f [0x858628] / 3.6f [0x859a84] | 27.777779 | none (dead store) |
| 0x849fe0 | 0x8a421c | g[0x96f040] <- 1.0f [0x858624] / (50.0f [0x859a88] * 50.0f [0x859a88]) | 0.0004 | none (dead store) |
| 0x84a000 | 0x8a4220 | g[0x96f044] <- g[0x96f040] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a020 | 0x8a4224 | g[0x96f048] <- 0.277778f [0x858630] / 50.0f [0x859a88] | 0.00555556 | none (dead store) |
| 0x84b2b0 | 0x8a4474 | g[0xa9a8a4] <- 0.005f [0x85a560] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b2d0 | 0x8a4478 | g[0xa9a8a8] <- 1.0f [0x858624] / 50.0f [0x85a568] | 0.02 | none (dead store) |
| 0x84b2f0 | 0x8a447c | g[0xa9a8ac] <- 100.0f [0x858628] / 3.6f [0x85a564] | 27.777779 | none (dead store) |
| 0x84b310 | 0x8a4480 | g[0xa9a8b0] <- 1.0f [0x858624] / (50.0f [0x85a568] * 50.0f [0x85a568]) | 0.0004 | none (dead store) |
| 0x84b330 | 0x8a4484 | g[0xa9a8b4] <- g[0xa9a8b0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b390 | 0x8a4490 | g[0xa9ad7c] <- 0.005f [0x85a590] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b3b0 | 0x8a4494 | g[0xa9ad80] <- 1.0f [0x858624] / 50.0f [0x85a598] | 0.02 | none (dead store) |
| 0x84b3d0 | 0x8a4498 | g[0xa9ad84] <- 100.0f [0x858628] / 3.6f [0x85a594] | 27.777779 | none (dead store) |
| 0x84b3f0 | 0x8a449c | g[0xa9ad88] <- 1.0f [0x858624] / (50.0f [0x85a598] * 50.0f [0x85a598]) | 0.0004 | none (dead store) |
| 0x84b410 | 0x8a44a0 | g[0xa9ad8c] <- g[0xa9ad88] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b430 | 0x8a44a4 | g[0xa9ad90] <- 0.277778f [0x858630] / 50.0f [0x85a598] | 0.00555556 | none (dead store) |
| 0x84b450 | 0x8a44a8 | g[0xa9ad98] <- 0.005f [0x85a5ac] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b470 | 0x8a44ac | g[0xa9ad9c] <- 1.0f [0x858624] / 50.0f [0x85a5b4] | 0.02 | none (dead store) |
| 0x84b490 | 0x8a44b0 | g[0xa9ada0] <- 100.0f [0x858628] / 3.6f [0x85a5b0] | 27.777779 | none (dead store) |
| 0x84b4b0 | 0x8a44b4 | g[0xa9ada4] <- 1.0f [0x858624] / (50.0f [0x85a5b4] * 50.0f [0x85a5b4]) | 0.0004 | none (dead store) |
| 0x84b4d0 | 0x8a44b8 | g[0xa9ada8] <- g[0xa9ada4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b4f0 | 0x8a44bc | g[0xa9adac] <- 0.277778f [0x858630] / 50.0f [0x85a5b4] | 0.00555556 | none (dead store) |
| 0x84de90 | 0x8a49f4 | g[0xb72928] <- 0.005f [0x863a60] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84deb0 | 0x8a49f8 | g[0xb7292c] <- 1.0f [0x858624] / 50.0f [0x863a68] | 0.02 | none (dead store) |
| 0x84ded0 | 0x8a49fc | g[0xb72930] <- 100.0f [0x858628] / 3.6f [0x863a64] | 27.777779 | none (dead store) |
| 0x84def0 | 0x8a4a00 | g[0xb72934] <- 1.0f [0x858624] / (50.0f [0x863a68] * 50.0f [0x863a68]) | 0.0004 | none (dead store) |
| 0x84df10 | 0x8a4a04 | g[0xb72938] <- g[0xb72934] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84df30 | 0x8a4a08 | g[0xb7293c] <- 0.277778f [0x858630] / 50.0f [0x863a68] | 0.00555556 | none (dead store) |
| 0x8509a0 | 0x8a4f44 | g[0xc09178] <- 0.005f [0x86bf24] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8509c0 | 0x8a4f48 | g[0xc0917c] <- 0.005f [0x86c098] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8509e0 | 0x8a4f4c | g[0xc09180] <- 1.0f [0x858624] / 50.0f [0x86c0a0] | 0.02 | none (dead store) |
| 0x850a00 | 0x8a4f50 | g[0xc09184] <- 100.0f [0x858628] / 3.6f [0x86c09c] | 27.777779 | none (dead store) |
| 0x850a20 | 0x8a4f54 | g[0xc09188] <- 1.0f [0x858624] / (50.0f [0x86c0a0] * 50.0f [0x86c0a0]) | 0.0004 | none (dead store) |
| 0x850a40 | 0x8a4f58 | g[0xc0918c] <- g[0xc09188] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850a60 | 0x8a4f5c | g[0xc09190] <- 0.277778f [0x858630] / 50.0f [0x86c0a0] | 0.00555556 | none (dead store) |
| 0x850a80 | 0x8a4f60 | g[0xc09194] <- 0.005f [0x86c110] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850aa0 | 0x8a4f64 | g[0xc09198] <- 1.0f [0x858624] / 50.0f [0x86c118] | 0.02 | none (dead store) |
| 0x850ae0 | 0x8a4f6c | g[0xc091a0] <- 1.0f [0x858624] / (50.0f [0x86c118] * 50.0f [0x86c118]) | 0.0004 | none (dead store) |
| 0x850b00 | 0x8a4f70 | g[0xc091a4] <- g[0xc091a0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850b20 | 0x8a4f74 | g[0xc091a8] <- 0.277778f [0x858630] / 50.0f [0x86c118] | 0.00555556 | none (dead store) |
| 0x850b40 | 0x8a4f78 | g[0xc091ac] <- 0.005f [0x86c188] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850b60 | 0x8a4f7c | g[0xc091b0] <- 1.0f [0x858624] / 50.0f [0x86c190] | 0.02 | none (dead store) |
| 0x850b80 | 0x8a4f80 | g[0xc091b4] <- 100.0f [0x858628] / 3.6f [0x86c18c] | 27.777779 | none (dead store) |
| 0x850ba0 | 0x8a4f84 | g[0xc091b8] <- 1.0f [0x858624] / (50.0f [0x86c190] * 50.0f [0x86c190]) | 0.0004 | none (dead store) |
| 0x850bc0 | 0x8a4f88 | g[0xc091bc] <- g[0xc091b8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850be0 | 0x8a4f8c | g[0xc091c0] <- 0.277778f [0x858630] / 50.0f [0x86c190] | 0.00555556 | none (dead store) |
| 0x850c00 | 0x8a4f90 | g[0xc091c4] <- 0.005f [0x86c1f0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850c20 | 0x8a4f94 | g[0xc091c8] <- 1.0f [0x858624] / 50.0f [0x86c1f8] | 0.02 | none (dead store) |
| 0x850c40 | 0x8a4f98 | g[0xc091cc] <- 100.0f [0x858628] / 3.6f [0x86c1f4] | 27.777779 | none (dead store) |
| 0x850c60 | 0x8a4f9c | g[0xc091d0] <- 1.0f [0x858624] / (50.0f [0x86c1f8] * 50.0f [0x86c1f8]) | 0.0004 | none (dead store) |
| 0x850c80 | 0x8a4fa0 | g[0xc091d4] <- g[0xc091d0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850ca0 | 0x8a4fa4 | g[0xc091d8] <- 0.277778f [0x858630] / 50.0f [0x86c1f8] | 0.00555556 | none (dead store) |
| 0x850cc0 | 0x8a4fa8 | g[0xc091ec] <- 0.005f [0x86c268] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851770 | 0x8a50fc | g[0xc0bc18] <- 0.005f [0x86d084] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851790 | 0x8a5100 | g[0xc0bc1c] <- 1.0f [0x858624] / 50.0f [0x86d08c] | 0.02 | none (dead store) |
| 0x8517b0 | 0x8a5104 | g[0xc0bc20] <- 100.0f [0x858628] / 3.6f [0x86d088] | 27.777779 | none (dead store) |
| 0x8517d0 | 0x8a5108 | g[0xc0bc24] <- 1.0f [0x858624] / (50.0f [0x86d08c] * 50.0f [0x86d08c]) | 0.0004 | none (dead store) |
| 0x8517f0 | 0x8a510c | g[0xc0bc28] <- g[0xc0bc24] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851810 | 0x8a5110 | g[0xc0bc2c] <- 0.277778f [0x858630] / 50.0f [0x86d08c] | 0.00555556 | none (dead store) |
| 0x851830 | 0x8a5114 | g[0xc0e97c] <- 0.005f [0x86d204] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851850 | 0x8a5118 | g[0xc0e980] <- 1.0f [0x858624] / 50.0f [0x86d20c] | 0.02 | none (dead store) |
| 0x851870 | 0x8a511c | g[0xc0e984] <- 100.0f [0x858628] / 3.6f [0x86d208] | 27.777779 | none (dead store) |
| 0x851890 | 0x8a5120 | g[0xc0e988] <- 1.0f [0x858624] / (50.0f [0x86d20c] * 50.0f [0x86d20c]) | 0.0004 | none (dead store) |
| 0x8518b0 | 0x8a5124 | g[0xc0e98c] <- g[0xc0e988] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8518d0 | 0x8a5128 | g[0xc0e990] <- 0.277778f [0x858630] / 50.0f [0x86d20c] | 0.00555556 | none (dead store) |
| 0x8518f0 | 0x8a512c | g[0xc0fcc0] <- 0.005f [0x86d210] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851910 | 0x8a5130 | g[0xc0fcc4] <- 1.0f [0x858624] / 50.0f [0x86d218] | 0.02 | none (dead store) |
| 0x851930 | 0x8a5134 | g[0xc0fcc8] <- 100.0f [0x858628] / 3.6f [0x86d214] | 27.777779 | none (dead store) |
| 0x851950 | 0x8a5138 | g[0xc0fccc] <- 1.0f [0x858624] / (50.0f [0x86d218] * 50.0f [0x86d218]) | 0.0004 | none (dead store) |
| 0x851970 | 0x8a513c | g[0xc0fcd0] <- g[0xc0fccc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851990 | 0x8a5140 | g[0xc0fcd4] <- 0.277778f [0x858630] / 50.0f [0x86d218] | 0.00555556 | none (dead store) |
| 0x853ff0 | 0x8a5614 | g[0xc279b4] <- 0.005f [0x872178] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854010 | 0x8a5618 | g[0xc279b8] <- 1.0f [0x858624] / 50.0f [0x872180] | 0.02 | none (dead store) |
| 0x854030 | 0x8a561c | g[0xc279bc] <- 100.0f [0x858628] / 3.6f [0x87217c] | 27.777779 | none (dead store) |
| 0x854050 | 0x8a5620 | g[0xc279c0] <- 1.0f [0x858624] / (50.0f [0x872180] * 50.0f [0x872180]) | 0.0004 | none (dead store) |
| 0x854070 | 0x8a5624 | g[0xc279c4] <- g[0xc279c0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854090 | 0x8a5628 | g[0xc279c8] <- 0.277778f [0x858630] / 50.0f [0x872180] | 0.00555556 | none (dead store) |
| 0x8540f0 | 0x8a5634 | g[0xc2b954] <- 1.0f [0x858624] / 50.0f [0x8722e4] | 0.02 | none (dead store) |
| 0x854110 | 0x8a5638 | g[0xc2b958] <- 100.0f [0x858628] / 3.6f [0x8722e0] | 27.777779 | none (dead store) |
| 0x854130 | 0x8a563c | g[0xc2b95c] <- 1.0f [0x858624] / (50.0f [0x8722e4] * 50.0f [0x8722e4]) | 0.0004 | none (dead store) |
| 0x854150 | 0x8a5640 | g[0xc2b960] <- g[0xc2b95c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854170 | 0x8a5644 | g[0xc2b964] <- 0.277778f [0x858630] / 50.0f [0x8722e4] | 0.00555556 | none (dead store) |

## Other initialisers and destructors

## 0x84a060 __init_g_977af0  (catalogue: unknown/REA, port level 0)
- EAX = 0x977AF0, ECX = 0x20; loop: word [EAX] = 0xFFFF; EAX += 4; --ECX != 0 (32 iterations, ascending): sets the first i16 of 32 4-byte elements (array 0x977AF0..0x977B70) to -1, the other 2 bytes untouched (a CNodeAddress-like {i16 area, i16 node} array with an inline ctor area = -1). Neighbours: __init_aNodesToBeCleared / __init_aExteriorNodeLinkedTo (POP).

## 0x84a220 __init_g_977be8  (catalogue: unknown/REA, port level 1)
- ECX = 0x977BE8; call 0x4541E0 (stub to .HOODLUM 0x1568BF0: `eh vector constructor iterator` 0x821D30 (stdcall: arr, elemSize, count, ctor, dtor; ascending)(this+8, elemSize=0x34, count=0x32 (50), ctor=0x453FA0 `mov eax,ecx; ret`, dtor=0x453FB0 `ret`); returns EAX = this; +0 and +4 untouched); _atexit 0x821D1E(0x856150). Object type unknown (gr has no StaticRef at 0x977BE8); .bss neighbourhood is the CGangWars / POP TU (UNCLEAR).

## 0x856150 __dtor_g_977be8  (catalogue: unknown/REA, port level 0)
- ECX = 0x977BE8; tail jump to 0x454200: `eh vector destructor iterator` 0x821E02 (stdcall: arr, elemSize, count, dtor; descending)(ECX+8, elemSize=0x34, count=0x32, dtor=0x453FB0); `ret`.
