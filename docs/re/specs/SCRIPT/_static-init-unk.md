# SCRIPT: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84a580 | 0x8a42d0 | g[0xa43588] <- 0.005f [0x859c74] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a5a0 | 0x8a42d4 | g[0xa4358c] <- 1.0f [0x858624] / 50.0f [0x859c7c] | 0.02 | none (dead store) |
| 0x84a5c0 | 0x8a42d8 | g[0xa43590] <- 100.0f [0x858628] / 3.6f [0x859c78] | 27.777779 | none (dead store) |
| 0x84a5e0 | 0x8a42dc | g[0xa43594] <- 1.0f [0x858624] / (50.0f [0x859c7c] * 50.0f [0x859c7c]) | 0.0004 | none (dead store) |
| 0x84a600 | 0x8a42e0 | g[0xa43598] <- g[0xa43594] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a620 | 0x8a42e4 | g[0xa4359c] <- 0.277778f [0x858630] / 50.0f [0x859c7c] | 0.00555556 | none (dead store) |
| 0x84a660 | 0x8a42ec | g[0xa90834] <- 0.005f [0x859cec] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a680 | 0x8a42f0 | g[0xa90838] <- 1.0f [0x858624] / 50.0f [0x859cf4] | 0.02 | none (dead store) |
| 0x84a6a0 | 0x8a42f4 | g[0xa9083c] <- 100.0f [0x858628] / 3.6f [0x859cf0] | 27.777779 | none (dead store) |
| 0x84a6c0 | 0x8a42f8 | g[0xa90840] <- 1.0f [0x858624] / (50.0f [0x859cf4] * 50.0f [0x859cf4]) | 0.0004 | none (dead store) |
| 0x84a6e0 | 0x8a42fc | g[0xa90844] <- g[0xa90840] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a700 | 0x8a4300 | g[0xa90848] <- 0.277778f [0x858630] / 50.0f [0x859cf4] | 0.00555556 | none (dead store) |
| 0x84a7e0 | 0x8a4318 | g[0xa95148] <- 0.005f [0x859d90] * 0.0625f [0x858620] | 0.0003125 | 0x4939bd in 0x493960 CTheScripts::IsEntityWithinAnySearchLight |
| 0x84a800 | 0x8a431c | g[0xa9514c] <- 1.0f [0x858624] / 50.0f [0x859d98] | 0.02 | none (dead store) |
| 0x84a820 | 0x8a4320 | g[0xa95150] <- 100.0f [0x858628] / 3.6f [0x859d94] | 27.777779 | none (dead store) |
| 0x84a840 | 0x8a4324 | g[0xa95154] <- 1.0f [0x858624] / (50.0f [0x859d98] * 50.0f [0x859d98]) | 0.0004 | 0x84a862 in 0x84a860 __init_g_a95158 |
| 0x84a880 | 0x8a432c | g[0xa9515c] <- 0.277778f [0x858630] / 50.0f [0x859d98] | 0.00555556 | none (dead store) |
| 0x84a960 | 0x8a4348 | g[0xa95178] <- 0.005f [0x859eb0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a980 | 0x8a434c | g[0xa9517c] <- 1.0f [0x858624] / 50.0f [0x859eb8] | 0.02 | none (dead store) |
| 0x84a9a0 | 0x8a4350 | g[0xa95180] <- 100.0f [0x858628] / 3.6f [0x859eb4] | 27.777779 | none (dead store) |
| 0x84a9c0 | 0x8a4354 | g[0xa95184] <- 1.0f [0x858624] / (50.0f [0x859eb8] * 50.0f [0x859eb8]) | 0.0004 | none (dead store) |
| 0x84a9e0 | 0x8a4358 | g[0xa95188] <- g[0xa95184] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84aa00 | 0x8a435c | g[0xa9518c] <- 0.277778f [0x858630] / 50.0f [0x859eb8] | 0.00555556 | none (dead store) |
| 0x84ac60 | 0x8a43a8 | g[0xa951dc] <- 0.005f [0x859f20] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ac80 | 0x8a43ac | g[0xa951e0] <- 1.0f [0x858624] / 50.0f [0x859f28] | 0.02 | none (dead store) |
| 0x84aca0 | 0x8a43b0 | g[0xa951e4] <- 100.0f [0x858628] / 3.6f [0x859f24] | 27.777779 | none (dead store) |
| 0x84acc0 | 0x8a43b4 | g[0xa951e8] <- 1.0f [0x858624] / (50.0f [0x859f28] * 50.0f [0x859f28]) | 0.0004 | none (dead store) |
| 0x84ace0 | 0x8a43b8 | g[0xa951ec] <- g[0xa951e8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ad00 | 0x8a43bc | g[0xa951f0] <- 0.277778f [0x858630] / 50.0f [0x859f28] | 0.00555556 | none (dead store) |
| 0x84fa60 | 0x8a4d64 | g[0xbb423c] <- 0.005f [0x866e5c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fb60 | 0x8a4d84 | g[0xbb4a58] <- 0.005f [0x866ed0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fb80 | 0x8a4d88 | g[0xbb4a5c] <- 1.0f [0x858624] / 50.0f [0x866ed8] | 0.02 | none (dead store) |
| 0x84fba0 | 0x8a4d8c | g[0xbb4a60] <- 100.0f [0x858628] / 3.6f [0x866ed4] | 27.777779 | none (dead store) |
| 0x84fbc0 | 0x8a4d90 | g[0xbb4a64] <- 1.0f [0x858624] / (50.0f [0x866ed8] * 50.0f [0x866ed8]) | 0.0004 | none (dead store) |
| 0x84fbe0 | 0x8a4d94 | g[0xbb4a68] <- g[0xbb4a64] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84fc00 | 0x8a4d98 | g[0xbb4a6c] <- 0.277778f [0x858630] / 50.0f [0x866ed8] | 0.00555556 | none (dead store) |
| 0x852780 | 0x8a5304 | g[0xc18d30] <- 0.005f [0x86fc68] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8527a0 | 0x8a5308 | g[0xc18d34] <- 1.0f [0x858624] / 50.0f [0x86fc70] | 0.02 | none (dead store) |
| 0x8527c0 | 0x8a530c | g[0xc18d38] <- 100.0f [0x858628] / 3.6f [0x86fc6c] | 27.777779 | none (dead store) |
| 0x8527e0 | 0x8a5310 | g[0xc18d3c] <- 1.0f [0x858624] / (50.0f [0x86fc70] * 50.0f [0x86fc70]) | 0.0004 | none (dead store) |
| 0x852800 | 0x8a5314 | g[0xc18d40] <- g[0xc18d3c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852820 | 0x8a5318 | g[0xc18d44] <- 0.277778f [0x858630] / 50.0f [0x86fc70] | 0.00555556 | none (dead store) |
| 0x8544d0 | 0x8a56ac | g[0xc39ed8] <- 0.005f [0x872510] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

