# WEAPON: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84d9f0 | 0x8a4960 | g[0xb717d4] <- 0.005f [0x863874] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84da10 | 0x8a4964 | g[0xb717d8] <- 1.0f [0x858624] / 50.0f [0x86387c] | 0.02 | none (dead store) |
| 0x84da30 | 0x8a4968 | g[0xb717dc] <- 100.0f [0x858628] / 3.6f [0x863878] | 27.777779 | none (dead store) |
| 0x84da50 | 0x8a496c | g[0xb717e0] <- 1.0f [0x858624] / (50.0f [0x86387c] * 50.0f [0x86387c]) | 0.0004 | none (dead store) |
| 0x84da70 | 0x8a4970 | g[0xb717e4] <- g[0xb717e0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84da90 | 0x8a4974 | g[0xb717e8] <- 0.277778f [0x858630] / 50.0f [0x86387c] | 0.00555556 | none (dead store) |
| 0x84db70 | 0x8a4990 | g[0xb7180c] <- 0.005f [0x863918] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84db90 | 0x8a4994 | g[0xb71810] <- 1.0f [0x858624] / 50.0f [0x863920] | 0.02 | none (dead store) |
| 0x84dbb0 | 0x8a4998 | g[0xb71814] <- 100.0f [0x858628] / 3.6f [0x86391c] | 27.777779 | none (dead store) |
| 0x84dbd0 | 0x8a499c | g[0xb71818] <- 1.0f [0x858624] / (50.0f [0x863920] * 50.0f [0x863920]) | 0.0004 | none (dead store) |
| 0x84dbf0 | 0x8a49a0 | g[0xb7181c] <- g[0xb71818] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84dc10 | 0x8a49a4 | g[0xb71820] <- 0.277778f [0x858630] / 50.0f [0x863920] | 0.00555556 | none (dead store) |
| 0x84ea60 | 0x8a4b70 | g[0xb9b7d4] <- 0.005f [0x864ff0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ea80 | 0x8a4b74 | g[0xb9b7d8] <- 1.0f [0x858624] / 50.0f [0x864ff8] | 0.02 | none (dead store) |
| 0x84eaa0 | 0x8a4b78 | g[0xb9b7dc] <- 100.0f [0x858628] / 3.6f [0x864ff4] | 27.777779 | none (dead store) |
| 0x84eac0 | 0x8a4b7c | g[0xb9b7e0] <- 1.0f [0x858624] / (50.0f [0x864ff8] * 50.0f [0x864ff8]) | 0.0004 | none (dead store) |
| 0x84eae0 | 0x8a4b80 | g[0xb9b7e4] <- g[0xb9b7e0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84eb00 | 0x8a4b84 | g[0xb9b7e8] <- 0.277778f [0x858630] / 50.0f [0x864ff8] | 0.00555556 | none (dead store) |
| 0x855340 | 0x8a5878 | g[0xc80548] <- 0.005f [0x872a10] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8557a0 | 0x8a5908 | g[0xc8870c] <- 0.005f [0x872b0c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8557c0 | 0x8a590c | g[0xc88710] <- 1.0f [0x858624] / 50.0f [0x872b14] | 0.02 | none (dead store) |
| 0x8557e0 | 0x8a5910 | g[0xc88714] <- 100.0f [0x858628] / 3.6f [0x872b10] | 27.777779 | none (dead store) |
| 0x855800 | 0x8a5914 | g[0xc88718] <- 1.0f [0x858624] / (50.0f [0x872b14] * 50.0f [0x872b14]) | 0.0004 | none (dead store) |
| 0x855820 | 0x8a5918 | g[0xc8871c] <- g[0xc88718] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855840 | 0x8a591c | g[0xc88720] <- 0.277778f [0x858630] / 50.0f [0x872b14] | 0.00555556 | none (dead store) |
| 0x855860 | 0x8a5920 | g[0xc88728] <- 0.005f [0x872b4c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855880 | 0x8a5924 | g[0xc8872c] <- 1.0f [0x858624] / 50.0f [0x872b54] | 0.02 | none (dead store) |
| 0x8558a0 | 0x8a5928 | g[0xc88730] <- 100.0f [0x858628] / 3.6f [0x872b50] | 27.777779 | none (dead store) |
| 0x8558c0 | 0x8a592c | g[0xc88734] <- 1.0f [0x858624] / (50.0f [0x872b54] * 50.0f [0x872b54]) | 0.0004 | none (dead store) |
| 0x8558e0 | 0x8a5930 | g[0xc88738] <- g[0xc88734] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855900 | 0x8a5934 | g[0xc8873c] <- 0.277778f [0x858630] / 50.0f [0x872b54] | 0.00555556 | none (dead store) |
| 0x855920 | 0x8a5938 | g[0xc888b8] <- 0.005f [0x872b5c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855940 | 0x8a593c | g[0xc888bc] <- 1.0f [0x858624] / 50.0f [0x872b64] | 0.02 | none (dead store) |
| 0x855960 | 0x8a5940 | g[0xc888c0] <- 100.0f [0x858628] / 3.6f [0x872b60] | 27.777779 | none (dead store) |
| 0x855980 | 0x8a5944 | g[0xc888c4] <- 1.0f [0x858624] / (50.0f [0x872b64] * 50.0f [0x872b64]) | 0.0004 | none (dead store) |
| 0x8559a0 | 0x8a5948 | g[0xc888c8] <- g[0xc888c4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8559c0 | 0x8a594c | g[0xc888cc] <- 0.277778f [0x858630] / 50.0f [0x872b64] | 0.00555556 | none (dead store) |
| 0x855a20 | 0x8a5958 | g[0xc89194] <- 1.0f [0x858624] / 50.0f [0x872be4] | 0.02 | none (dead store) |
| 0x855a40 | 0x8a595c | g[0xc89198] <- 100.0f [0x858628] / 3.6f [0x872be0] | 27.777779 | none (dead store) |
| 0x855a60 | 0x8a5960 | g[0xc8919c] <- 1.0f [0x858624] / (50.0f [0x872be4] * 50.0f [0x872be4]) | 0.0004 | none (dead store) |
| 0x855a80 | 0x8a5964 | g[0xc891a0] <- g[0xc8919c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855aa0 | 0x8a5968 | g[0xc891a4] <- 0.277778f [0x858630] / 50.0f [0x872be4] | 0.00555556 | none (dead store) |
| 0x855ae0 | 0x8a5970 | g[0xc8967c] <- 1.0f [0x858624] / 50.0f [0x872c0c] | 0.02 | none (dead store) |
| 0x855b00 | 0x8a5974 | g[0xc89680] <- 100.0f [0x858628] / 3.6f [0x872c08] | 27.777779 | none (dead store) |
| 0x855b20 | 0x8a5978 | g[0xc89684] <- 1.0f [0x858624] / (50.0f [0x872c0c] * 50.0f [0x872c0c]) | 0.0004 | none (dead store) |
| 0x855b40 | 0x8a597c | g[0xc89688] <- g[0xc89684] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855b60 | 0x8a5980 | g[0xc8968c] <- 0.277778f [0x858630] / 50.0f [0x872c0c] | 0.00555556 | none (dead store) |
| 0x855b80 | 0x8a5984 | g[0xc8a7c4] <- 0.005f [0x872c10] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855ba0 | 0x8a5988 | g[0xc8a7c8] <- 1.0f [0x858624] / 50.0f [0x872c18] | 0.02 | 0xd0b4e4 in no catalogue function |
| 0x855bc0 | 0x8a598c | g[0xc8a7cc] <- 100.0f [0x858628] / 3.6f [0x872c14] | 27.777779 | none (dead store) |
| 0x855be0 | 0x8a5990 | g[0xc8a7d0] <- 1.0f [0x858624] / (50.0f [0x872c18] * 50.0f [0x872c18]) | 0.0004 | none (dead store) |
| 0x855c00 | 0x8a5994 | g[0xc8a7d4] <- g[0xc8a7d0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855c60 | 0x8a59a0 | g[0xc8a81c] <- 0.005f [0x872c90] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855c80 | 0x8a59a4 | g[0xc8a820] <- 1.0f [0x858624] / 50.0f [0x872c98] | 0.02 | none (dead store) |
| 0x855ca0 | 0x8a59a8 | g[0xc8a824] <- 100.0f [0x858628] / 3.6f [0x872c94] | 27.777779 | none (dead store) |
| 0x855cc0 | 0x8a59ac | g[0xc8a828] <- 1.0f [0x858624] / (50.0f [0x872c98] * 50.0f [0x872c98]) | 0.0004 | none (dead store) |
| 0x855ce0 | 0x8a59b0 | g[0xc8a82c] <- g[0xc8a828] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855d00 | 0x8a59b4 | g[0xc8a830] <- 0.277778f [0x858630] / 50.0f [0x872c98] | 0.00555556 | none (dead store) |
| 0x855d50 | 0x8a59bc | g[0xc8aaa0] <- 0.005f [0x872cdc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855d90 | 0x8a59c4 | g[0xc8aaa8] <- 100.0f [0x858628] / 3.6f [0x872ce0] | 27.777779 | none (dead store) |
| 0x855db0 | 0x8a59c8 | g[0xc8aaac] <- 1.0f [0x858624] / (50.0f [0x872ce4] * 50.0f [0x872ce4]) | 0.0004 | none (dead store) |
| 0x855dd0 | 0x8a59cc | g[0xc8aab0] <- g[0xc8aaac] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855df0 | 0x8a59d0 | g[0xc8aab4] <- 0.277778f [0x858630] / 50.0f [0x872ce4] | 0.00555556 | none (dead store) |

## Other initialisers and destructors

## 0x855fc0 __init_855fc0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856c80) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (WEAPON).

## 0x856c80 __dtor_855fc0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x855fc0).
