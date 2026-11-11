# CAM: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84d5f0 | 0x8a48e0 | g[0xb6eba0] <- 0.005f [0x862e6c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d610 | 0x8a48e4 | g[0xb6ec88] <- 0.005f [0x862f04] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d630 | 0x8a48e8 | g[0xb6ec8c] <- 1.0f [0x858624] / 50.0f [0x862f0c] | 0.02 | none (dead store) |
| 0x84d650 | 0x8a48ec | g[0xb6ec90] <- 100.0f [0x858628] / 3.6f [0x862f08] | 27.777779 | none (dead store) |
| 0x84d670 | 0x8a48f0 | g[0xb6ec94] <- 1.0f [0x858624] / (50.0f [0x862f0c] * 50.0f [0x862f0c]) | 0.0004 | none (dead store) |
| 0x84d690 | 0x8a48f4 | g[0xb6ec98] <- g[0xb6ec94] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84d6b0 | 0x8a48f8 | g[0xb6ec9c] <- 0.277778f [0x858630] / 50.0f [0x862f0c] | 0.00555556 | none (dead store) |
| 0x850060 | 0x8a4e24 | g[0xbc405c] <- 0.005f [0x868a50] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850080 | 0x8a4e28 | g[0xbc4060] <- 1.0f [0x858624] / 50.0f [0x868a58] | 0.02 | none (dead store) |
| 0x8500a0 | 0x8a4e2c | g[0xbc4064] <- 100.0f [0x858628] / 3.6f [0x868a54] | 27.777779 | none (dead store) |
| 0x8500c0 | 0x8a4e30 | g[0xbc4068] <- 1.0f [0x858624] / (50.0f [0x868a58] * 50.0f [0x868a58]) | 0.0004 | none (dead store) |
| 0x8500e0 | 0x8a4e34 | g[0xbc406c] <- g[0xbc4068] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850100 | 0x8a4e38 | g[0xbc4070] <- 0.277778f [0x858630] / 50.0f [0x868a58] | 0.00555556 | none (dead store) |
| 0x854fd0 | 0x8a580c | g[0xc73c4c] <- 0.005f [0x872970] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

