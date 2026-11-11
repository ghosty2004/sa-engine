# RW: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84b730 | 0x8a4504 | g[0xa9af54] <- 0.005f [0x85aa98] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b750 | 0x8a4508 | g[0xa9af58] <- 1.0f [0x858624] / 50.0f [0x85aaa0] | 0.02 | none (dead store) |
| 0x84b770 | 0x8a450c | g[0xa9af5c] <- 100.0f [0x858628] / 3.6f [0x85aa9c] | 27.777779 | none (dead store) |
| 0x84b790 | 0x8a4510 | g[0xa9af60] <- 1.0f [0x858624] / (50.0f [0x85aaa0] * 50.0f [0x85aaa0]) | 0.0004 | none (dead store) |
| 0x84b7b0 | 0x8a4514 | g[0xa9af64] <- g[0xa9af60] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b7d0 | 0x8a4518 | g[0xa9af68] <- 0.277778f [0x858630] / 50.0f [0x85aaa0] | 0.00555556 | none (dead store) |
| 0x84be10 | 0x8a45dc | g[0xb4e754] <- 0.005f [0x85c660] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84be30 | 0x8a45e0 | g[0xb4e758] <- 1.0f [0x858624] / 50.0f [0x85c668] | 0.02 | none (dead store) |
| 0x84be50 | 0x8a45e4 | g[0xb4e75c] <- 100.0f [0x858628] / 3.6f [0x85c664] | 27.777779 | none (dead store) |
| 0x84be70 | 0x8a45e8 | g[0xb4e760] <- 1.0f [0x858624] / (50.0f [0x85c668] * 50.0f [0x85c668]) | 0.0004 | none (dead store) |
| 0x84be90 | 0x8a45ec | g[0xb4e764] <- g[0xb4e760] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84beb0 | 0x8a45f0 | g[0xb4e768] <- 0.277778f [0x858630] / 50.0f [0x85c668] | 0.00555556 | none (dead store) |
| 0x84bed0 | 0x8a45f4 | g[0xb4ea04] <- 0.005f [0x85c6c8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c020 | 0x8a461c | g[0xb5f860] <- 0.005f [0x85eea4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c040 | 0x8a4620 | g[0xb5f864] <- 1.0f [0x858624] / 50.0f [0x85eeac] | 0.02 | none (dead store) |
| 0x84c060 | 0x8a4624 | g[0xb5f868] <- 100.0f [0x858628] / 3.6f [0x85eea8] | 27.777779 | none (dead store) |
| 0x84c080 | 0x8a4628 | g[0xb5f86c] <- 1.0f [0x858624] / (50.0f [0x85eeac] * 50.0f [0x85eeac]) | 0.0004 | none (dead store) |
| 0x84c0a0 | 0x8a462c | g[0xb5f870] <- g[0xb5f86c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c0c0 | 0x8a4630 | g[0xb5f874] <- 0.277778f [0x858630] / 50.0f [0x85eeac] | 0.00555556 | none (dead store) |
| 0x84f9a0 | 0x8a4d4c | g[0xbb421c] <- 0.005f [0x866e50] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f9c0 | 0x8a4d50 | g[0xbb4220] <- 1.0f [0x858624] / 50.0f [0x866e58] | 0.02 | none (dead store) |
| 0x84f9e0 | 0x8a4d54 | g[0xbb4224] <- 100.0f [0x858628] / 3.6f [0x866e54] | 27.777779 | none (dead store) |
| 0x84fa00 | 0x8a4d58 | g[0xbb4228] <- 1.0f [0x858624] / (50.0f [0x866e58] * 50.0f [0x866e58]) | 0.0004 | none (dead store) |
| 0x84fa20 | 0x8a4d5c | g[0xbb422c] <- g[0xbb4228] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84fa40 | 0x8a4d60 | g[0xbb4230] <- 0.277778f [0x858630] / 50.0f [0x866e58] | 0.00555556 | none (dead store) |
| 0x84faa0 | 0x8a4d6c | g[0xbb4a40] <- 0.005f [0x866e68] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fac0 | 0x8a4d70 | g[0xbb4a44] <- 1.0f [0x858624] / 50.0f [0x866e70] | 0.02 | none (dead store) |
| 0x84fae0 | 0x8a4d74 | g[0xbb4a48] <- 100.0f [0x858628] / 3.6f [0x866e6c] | 27.777779 | none (dead store) |
| 0x84fb00 | 0x8a4d78 | g[0xbb4a4c] <- 1.0f [0x858624] / (50.0f [0x866e70] * 50.0f [0x866e70]) | 0.0004 | none (dead store) |
| 0x84fb20 | 0x8a4d7c | g[0xbb4a50] <- g[0xbb4a4c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84fb40 | 0x8a4d80 | g[0xbb4a54] <- 0.277778f [0x858630] / 50.0f [0x866e70] | 0.00555556 | none (dead store) |
| 0x84fce0 | 0x8a4db4 | g[0xbb7c90] <- 0.005f [0x866fd8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fde0 | 0x8a4dd4 | g[0xbbc8c4] <- 0.005f [0x86702c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fe00 | 0x8a4dd8 | g[0xbc128c] <- 0.005f [0x867138] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850380 | 0x8a4e88 | g[0xbd0118] <- 0.005f [0x86a45c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8503a0 | 0x8a4e8c | g[0xbd011c] <- 1.0f [0x858624] / 50.0f [0x86a464] | 0.02 | none (dead store) |
| 0x8503c0 | 0x8a4e90 | g[0xbd0120] <- 100.0f [0x858628] / 3.6f [0x86a460] | 27.777779 | none (dead store) |
| 0x8503e0 | 0x8a4e94 | g[0xbd0124] <- 1.0f [0x858624] / (50.0f [0x86a464] * 50.0f [0x86a464]) | 0.0004 | none (dead store) |
| 0x850400 | 0x8a4e98 | g[0xbd0128] <- g[0xbd0124] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850420 | 0x8a4e9c | g[0xbd012c] <- 0.277778f [0x858630] / 50.0f [0x86a464] | 0.00555556 | none (dead store) |
| 0x850440 | 0x8a4ea0 | g[0xbd0130] <- 3.35f [0x86ab80] - 1.85f [0x86a468] | 1.4999999 | none (dead store) |
| 0x850460 | 0x8a4ea4 | g[0xbd0134] <- 5.85f [0x86ab84] - 1.85f [0x86a468] | 4.0 | none (dead store) |
| 0x850480 | 0x8a4ea8 | g[0xbd0138] <- 0.005f [0x86ab88] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8504a0 | 0x8a4eac | g[0xbd013c] <- 1.0f [0x858624] / 50.0f [0x86ab90] | 0.02 | none (dead store) |
| 0x8504c0 | 0x8a4eb0 | g[0xbd0140] <- 100.0f [0x858628] / 3.6f [0x86ab8c] | 27.777779 | none (dead store) |
| 0x8504e0 | 0x8a4eb4 | g[0xbd0144] <- 1.0f [0x858624] / (50.0f [0x86ab90] * 50.0f [0x86ab90]) | 0.0004 | none (dead store) |
| 0x850500 | 0x8a4eb8 | g[0xbd0148] <- g[0xbd0144] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850520 | 0x8a4ebc | g[0xbd014c] <- 0.277778f [0x858630] / 50.0f [0x86ab90] | 0.00555556 | none (dead store) |
| 0x850540 | 0x8a4ec0 | g[0xbd0150] <- 0.005f [0x86af94] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850560 | 0x8a4ec4 | g[0xc02b84] <- 0.005f [0x86b958] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850580 | 0x8a4ec8 | g[0xc02b88] <- 1.0f [0x858624] / 50.0f [0x86b960] | 0.02 | none (dead store) |
| 0x8505a0 | 0x8a4ecc | g[0xc02b8c] <- 100.0f [0x858628] / 3.6f [0x86b95c] | 27.777779 | none (dead store) |
| 0x8505c0 | 0x8a4ed0 | g[0xc02b90] <- 1.0f [0x858624] / (50.0f [0x86b960] * 50.0f [0x86b960]) | 0.0004 | none (dead store) |
| 0x8505e0 | 0x8a4ed4 | g[0xc02b94] <- g[0xc02b90] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850600 | 0x8a4ed8 | g[0xc02b98] <- 0.277778f [0x858630] / 50.0f [0x86b960] | 0.00555556 | none (dead store) |
| 0x850620 | 0x8a4edc | g[0xc02b9c] <- 0.005f [0x86b964] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850640 | 0x8a4ee0 | g[0xc02ba0] <- 1.0f [0x858624] / 50.0f [0x86b96c] | 0.02 | none (dead store) |
| 0x850660 | 0x8a4ee4 | g[0xc02ba4] <- 100.0f [0x858628] / 3.6f [0x86b968] | 27.777779 | none (dead store) |
| 0x850680 | 0x8a4ee8 | g[0xc02ba8] <- 1.0f [0x858624] / (50.0f [0x86b96c] * 50.0f [0x86b96c]) | 0.0004 | none (dead store) |
| 0x8506a0 | 0x8a4eec | g[0xc02bac] <- g[0xc02ba8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8506c0 | 0x8a4ef0 | g[0xc02bb0] <- 0.277778f [0x858630] / 50.0f [0x86b96c] | 0.00555556 | none (dead store) |
| 0x8506e0 | 0x8a4ef4 | g[0xc02bb4] <- 0.005f [0x86bb08] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850700 | 0x8a4ef8 | g[0xc02bb8] <- 1.0f [0x858624] / 50.0f [0x86bb10] | 0.02 | none (dead store) |
| 0x850720 | 0x8a4efc | g[0xc02bbc] <- 100.0f [0x858628] / 3.6f [0x86bb0c] | 27.777779 | none (dead store) |
| 0x850740 | 0x8a4f00 | g[0xc02bc0] <- 1.0f [0x858624] / (50.0f [0x86bb10] * 50.0f [0x86bb10]) | 0.0004 | none (dead store) |
| 0x850760 | 0x8a4f04 | g[0xc02bc4] <- g[0xc02bc0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850780 | 0x8a4f08 | g[0xc02bc8] <- 0.277778f [0x858630] / 50.0f [0x86bb10] | 0.00555556 | none (dead store) |
| 0x8507c0 | 0x8a4f10 | g[0xc02c24] <- 0.005f [0x86bb3c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8507e0 | 0x8a4f14 | g[0xc02d34] <- 0.005f [0x86bea0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8556c0 | 0x8a58ec | g[0xc8801c] <- 0.005f [0x872ae8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8556e0 | 0x8a58f0 | g[0xc88058] <- 0.005f [0x872b00] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855700 | 0x8a58f4 | g[0xc8805c] <- 1.0f [0x858624] / 50.0f [0x872b08] | 0.02 | none (dead store) |
| 0x855720 | 0x8a58f8 | g[0xc88060] <- 100.0f [0x858628] / 3.6f [0x872b04] | 27.777779 | none (dead store) |
| 0x855740 | 0x8a58fc | g[0xc88064] <- 1.0f [0x858624] / (50.0f [0x872b08] * 50.0f [0x872b08]) | 0.0004 | none (dead store) |
| 0x855760 | 0x8a5900 | g[0xc88068] <- g[0xc88064] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855780 | 0x8a5904 | g[0xc8806c] <- 0.277778f [0x858630] / 50.0f [0x872b08] | 0.00555556 | none (dead store) |
| 0x855f00 | 0x8a59f0 | g[0xc9211c] <- 0.005f [0x874748] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855f20 | 0x8a59f4 | g[0xc92120] <- 1.0f [0x858624] / 50.0f [0x874750] | 0.02 | none (dead store) |
| 0x855f40 | 0x8a59f8 | g[0xc92124] <- 100.0f [0x858628] / 3.6f [0x87474c] | 27.777779 | none (dead store) |
| 0x855f60 | 0x8a59fc | g[0xc92128] <- 1.0f [0x858624] / (50.0f [0x874750] * 50.0f [0x874750]) | 0.0004 | none (dead store) |
| 0x855f80 | 0x8a5a00 | g[0xc9212c] <- g[0xc92128] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855fa0 | 0x8a5a04 | g[0xc92130] <- 0.277778f [0x858630] / 50.0f [0x874750] | 0.00555556 | none (dead store) |

