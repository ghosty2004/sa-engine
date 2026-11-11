# PED: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84eb20 | 0x8a4b88 | g[0xb9b97c] <- 0.005f [0x86500c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84eb40 | 0x8a4b8c | g[0xb9b980] <- 1.0f [0x858624] / 50.0f [0x865014] | 0.02 | none (dead store) |
| 0x84eb60 | 0x8a4b90 | g[0xb9b984] <- 100.0f [0x858628] / 3.6f [0x865010] | 27.777779 | none (dead store) |
| 0x84eb80 | 0x8a4b94 | g[0xb9b988] <- 1.0f [0x858624] / (50.0f [0x865014] * 50.0f [0x865014]) | 0.0004 | none (dead store) |
| 0x84eba0 | 0x8a4b98 | g[0xb9b98c] <- g[0xb9b988] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ebc0 | 0x8a4b9c | g[0xb9b990] <- 0.277778f [0x858630] / 50.0f [0x865014] | 0.00555556 | none (dead store) |
| 0x84fe70 | 0x8a4de8 | g[0xbc1c60] <- 0.005f [0x867180] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fe90 | 0x8a4dec | g[0xbc1c64] <- 1.0f [0x858624] / 50.0f [0x867188] | 0.02 | none (dead store) |
| 0x84feb0 | 0x8a4df0 | g[0xbc1c68] <- 100.0f [0x858628] / 3.6f [0x867184] | 27.777779 | none (dead store) |
| 0x84fed0 | 0x8a4df4 | g[0xbc1c6c] <- 1.0f [0x858624] / (50.0f [0x867188] * 50.0f [0x867188]) | 0.0004 | none (dead store) |
| 0x84fef0 | 0x8a4df8 | g[0xbc1c70] <- g[0xbc1c6c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ff10 | 0x8a4dfc | g[0xbc1c74] <- 0.277778f [0x858630] / 50.0f [0x867188] | 0.00555556 | none (dead store) |
| 0x850d10 | 0x8a4fb0 | g[0xc09290] <- 0.005f [0x86c26c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850d30 | 0x8a4fb4 | g[0xc09294] <- 1.0f [0x858624] / 50.0f [0x86c274] | 0.02 | none (dead store) |
| 0x850d50 | 0x8a4fb8 | g[0xc09298] <- 100.0f [0x858628] / 3.6f [0x86c270] | 27.777779 | none (dead store) |
| 0x850d70 | 0x8a4fbc | g[0xc0929c] <- 1.0f [0x858624] / (50.0f [0x86c274] * 50.0f [0x86c274]) | 0.0004 | none (dead store) |
| 0x850d90 | 0x8a4fc0 | g[0xc092a0] <- g[0xc0929c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850db0 | 0x8a4fc4 | g[0xc092a4] <- 0.277778f [0x858630] / 50.0f [0x86c274] | 0.00555556 | none (dead store) |
| 0x8516f0 | 0x8a50ec | g[0xc0bbf8] <- 100.0f [0x858628] / 3.6f [0x86d04c] | 27.777779 | none (dead store) |
| 0x851710 | 0x8a50f0 | g[0xc0bbfc] <- 1.0f [0x858624] / (50.0f [0x86d050] * 50.0f [0x86d050]) | 0.0004 | none (dead store) |
| 0x851730 | 0x8a50f4 | g[0xc0bc00] <- g[0xc0bbfc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851750 | 0x8a50f8 | g[0xc0bc04] <- 0.277778f [0x858630] / 50.0f [0x86d050] | 0.00555556 | none (dead store) |
| 0x8546f0 | 0x8a56ec | g[0xc3f038] <- 0.005f [0x8725b4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

