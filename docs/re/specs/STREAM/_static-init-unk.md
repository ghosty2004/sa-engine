# STREAM: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x848f10 | 0x8a4008 | g[0x8e3ee0] <- 0.005f [0x8585b8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x848f30 | 0x8a400c | g[0x8e3ee4] <- 1.0f [0x858624] / 50.0f [0x8585c0] | 0.02 | none (dead store) |
| 0x848f50 | 0x8a4010 | g[0x8e3ee8] <- 100.0f [0x858628] / 3.6f [0x8585bc] | 27.777779 | none (dead store) |
| 0x848f70 | 0x8a4014 | g[0x8e3eec] <- 1.0f [0x858624] / (50.0f [0x8585c0] * 50.0f [0x8585c0]) | 0.0004 | none (dead store) |
| 0x848f90 | 0x8a4018 | g[0x8e3ef0] <- g[0x8e3eec] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x848fb0 | 0x8a401c | g[0x8e3ef4] <- 0.277778f [0x858630] / 50.0f [0x8585c0] | 0.00555556 | none (dead store) |
| 0x848fd0 | 0x8a4020 | g[0x8e3fb8] <- 0.005f [0x858964] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x848ff0 | 0x8a4024 | g[0x8e3fbc] <- 1.0f [0x858624] / 50.0f [0x85896c] | 0.02 | none (dead store) |
| 0x849010 | 0x8a4028 | g[0x8e3fc0] <- 100.0f [0x858628] / 3.6f [0x858968] | 27.777779 | none (dead store) |
| 0x849030 | 0x8a402c | g[0x8e3fc4] <- 1.0f [0x858624] / (50.0f [0x85896c] * 50.0f [0x85896c]) | 0.0004 | none (dead store) |
| 0x849050 | 0x8a4030 | g[0x8e3fc8] <- g[0x8e3fc4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849070 | 0x8a4034 | g[0x8e3fcc] <- 0.277778f [0x858630] / 50.0f [0x85896c] | 0.00555556 | none (dead store) |
| 0x849090 | 0x8a4038 | g[0x9654d8] <- 0.005f [0x858b04] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8490b0 | 0x8a403c | g[0x9654dc] <- 1.0f [0x858624] / 50.0f [0x858b0c] | 0.02 | none (dead store) |
| 0x8490d0 | 0x8a4040 | g[0x9654e0] <- 100.0f [0x858628] / 3.6f [0x858b08] | 27.777779 | none (dead store) |
| 0x8490f0 | 0x8a4044 | g[0x9654e4] <- 1.0f [0x858624] / (50.0f [0x858b0c] * 50.0f [0x858b0c]) | 0.0004 | none (dead store) |
| 0x849110 | 0x8a4048 | g[0x9654e8] <- g[0x9654e4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849130 | 0x8a404c | g[0x9654ec] <- 0.277778f [0x858630] / 50.0f [0x858b0c] | 0.00555556 | none (dead store) |
| 0x849150 | 0x8a4050 | g[0x96553c] <- 0.005f [0x858b7c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849170 | 0x8a4054 | g[0x965540] <- 1.0f [0x858624] / 50.0f [0x858b84] | 0.02 | none (dead store) |
| 0x849190 | 0x8a4058 | g[0x965544] <- 100.0f [0x858628] / 3.6f [0x858b80] | 27.777779 | none (dead store) |
| 0x8491b0 | 0x8a405c | g[0x965548] <- 1.0f [0x858624] / (50.0f [0x858b84] * 50.0f [0x858b84]) | 0.0004 | none (dead store) |
| 0x8491d0 | 0x8a4060 | g[0x96554c] <- g[0x965548] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8491f0 | 0x8a4064 | g[0x965550] <- 0.277778f [0x858630] / 50.0f [0x858b84] | 0.00555556 | none (dead store) |
| 0x84bb50 | 0x8a4584 | g[0xa9b0b4] <- 0.005f [0x85bae4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bb70 | 0x8a4588 | g[0xa9b0b8] <- 0.005f [0x85bb98] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bb90 | 0x8a458c | g[0xa9b0bc] <- 0.005f [0x85bbd8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bbb0 | 0x8a4590 | g[0xa9b0c0] <- 0.005f [0x85bbe4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bbd0 | 0x8a4594 | g[0xaae94c] <- 0.005f [0x85bbe8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bd30 | 0x8a45c0 | g[0xb4dbe0] <- 0.005f [0x85be80] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bd50 | 0x8a45c4 | g[0xb4e6c0] <- 0.005f [0x85c5b4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84bd70 | 0x8a45c8 | g[0xb4e6c4] <- 1.0f [0x858624] / 50.0f [0x85c5bc] | 0.02 | none (dead store) |
| 0x84bd90 | 0x8a45cc | g[0xb4e6c8] <- 100.0f [0x858628] / 3.6f [0x85c5b8] | 27.777779 | none (dead store) |
| 0x84bdb0 | 0x8a45d0 | g[0xb4e6cc] <- 1.0f [0x858624] / (50.0f [0x85c5bc] * 50.0f [0x85c5bc]) | 0.0004 | none (dead store) |
| 0x84bdd0 | 0x8a45d4 | g[0xb4e6d0] <- g[0xb4e6cc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84bdf0 | 0x8a45d8 | g[0xb4e6d4] <- 0.277778f [0x858630] / 50.0f [0x85c5bc] | 0.00555556 | none (dead store) |
| 0x84dc30 | 0x8a49a8 | g[0xb71824] <- 0.005f [0x863998] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84dc50 | 0x8a49ac | g[0xb71828] <- 1.0f [0x858624] / 50.0f [0x8639a0] | 0.02 | none (dead store) |
| 0x84dc70 | 0x8a49b0 | g[0xb7182c] <- 100.0f [0x858628] / 3.6f [0x86399c] | 27.777779 | none (dead store) |
| 0x84dc90 | 0x8a49b4 | g[0xb71830] <- 1.0f [0x858624] / (50.0f [0x8639a0] * 50.0f [0x8639a0]) | 0.0004 | none (dead store) |
| 0x84dcb0 | 0x8a49b8 | g[0xb71834] <- g[0xb71830] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84dcd0 | 0x8a49bc | g[0xb71838] <- 0.277778f [0x858630] / 50.0f [0x8639a0] | 0.00555556 | none (dead store) |
| 0x850140 | 0x8a4e40 | g[0xbc40bc] <- 0.005f [0x868b28] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850160 | 0x8a4e44 | g[0xbc40c0] <- 1.0f [0x858624] / 50.0f [0x868b30] | 0.02 | none (dead store) |
| 0x850180 | 0x8a4e48 | g[0xbc40c4] <- 100.0f [0x858628] / 3.6f [0x868b2c] | 27.777779 | none (dead store) |
| 0x8501a0 | 0x8a4e4c | g[0xbc40c8] <- 1.0f [0x858624] / (50.0f [0x868b30] * 50.0f [0x868b30]) | 0.0004 | none (dead store) |
| 0x8501c0 | 0x8a4e50 | g[0xbc40cc] <- g[0xbc40c8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8501e0 | 0x8a4e54 | g[0xbc40d0] <- 0.277778f [0x858630] / 50.0f [0x868b30] | 0.00555556 | none (dead store) |
| 0x855570 | 0x8a58bc | g[0xc87ad0] <- 0.005f [0x872a98] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855590 | 0x8a58c0 | g[0xc87ad4] <- 1.0f [0x858624] / 50.0f [0x872aa0] | 0.02 | none (dead store) |
| 0x8555b0 | 0x8a58c4 | g[0xc87ad8] <- 100.0f [0x858628] / 3.6f [0x872a9c] | 27.777779 | none (dead store) |
| 0x8555d0 | 0x8a58c8 | g[0xc87adc] <- 1.0f [0x858624] / (50.0f [0x872aa0] * 50.0f [0x872aa0]) | 0.0004 | none (dead store) |
| 0x8555f0 | 0x8a58cc | g[0xc87ae0] <- g[0xc87adc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855610 | 0x8a58d0 | g[0xc87ae4] <- 0.277778f [0x858630] / 50.0f [0x872aa0] | 0.00555556 | none (dead store) |
| 0x855650 | 0x8a58d8 | g[0xc87b38] <- 0.005f [0x872ac0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8556a0 | 0x8a58e8 | g[0xc88000] <- 0.005f [0x872ac4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

