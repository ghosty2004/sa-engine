# VEH: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84a240 | 0x8a4268 | g[0x97d648] <- 0.005f [0x859af8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a2a0 | 0x8a4274 | g[0x97d654] <- 1.0f [0x858624] / (50.0f [0x859b00] * 50.0f [0x859b00]) | 0.0004 | none (dead store) |
| 0x84a2c0 | 0x8a4278 | g[0x97d658] <- g[0x97d654] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a2e0 | 0x8a427c | g[0x97d65c] <- 0.277778f [0x858630] / 50.0f [0x859b00] | 0.00555556 | none (dead store) |
| 0x84a320 | 0x8a4284 | g[0x97f638] <- 1.0f [0x858624] / 50.0f [0x859bc8] | 0.02 | none (dead store) |
| 0x84a360 | 0x8a428c | g[0x97f640] <- 1.0f [0x858624] / (50.0f [0x859bc8] * 50.0f [0x859bc8]) | 0.0004 | none (dead store) |
| 0x84a380 | 0x8a4290 | g[0x97f644] <- g[0x97f640] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a3a0 | 0x8a4294 | g[0x97f648] <- 0.277778f [0x858630] / 50.0f [0x859bc8] | 0.00555556 | none (dead store) |
| 0x852200 | 0x8a524c | g[0xc18c08] <- 0.005f [0x86ec24] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x852220 | 0x8a5250 | g[0xc18c0c] <- 1.0f [0x858624] / 50.0f [0x86ec2c] | 0.02 | none (dead store) |
| 0x852240 | 0x8a5254 | g[0xc18c10] <- 100.0f [0x858628] / 3.6f [0x86ec28] | 27.777779 | none (dead store) |
| 0x852260 | 0x8a5258 | g[0xc18c14] <- 1.0f [0x858624] / (50.0f [0x86ec2c] * 50.0f [0x86ec2c]) | 0.0004 | none (dead store) |
| 0x852280 | 0x8a525c | g[0xc18c18] <- g[0xc18c14] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8522a0 | 0x8a5260 | g[0xc18c1c] <- 0.277778f [0x858630] / 50.0f [0x86ec2c] | 0.00555556 | none (dead store) |
| 0x8522c0 | 0x8a5264 | g[0xc18c24] <- 0.005f [0x86f0ec] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8522e0 | 0x8a5268 | g[0xc18c28] <- 1.0f [0x858624] / 50.0f [0x86f0f4] | 0.02 | none (dead store) |
| 0x852300 | 0x8a526c | g[0xc18c2c] <- 100.0f [0x858628] / 3.6f [0x86f0f0] | 27.777779 | none (dead store) |
| 0x852320 | 0x8a5270 | g[0xc18c30] <- 1.0f [0x858624] / (50.0f [0x86f0f4] * 50.0f [0x86f0f4]) | 0.0004 | none (dead store) |
| 0x852340 | 0x8a5274 | g[0xc18c34] <- g[0xc18c30] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x852360 | 0x8a5278 | g[0xc18c38] <- 0.277778f [0x858630] / 50.0f [0x86f0f4] | 0.00555556 | none (dead store) |
| 0x853360 | 0x8a5488 | g[0xc1aeb0] <- 0.005f [0x870f44] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8533a0 | 0x8a5490 | g[0xc1b328] <- 0.005f [0x870f4c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8533c0 | 0x8a5494 | g[0xc1b32c] <- 1.0f [0x858624] / 50.0f [0x870f54] | 0.02 | none (dead store) |
| 0x8533e0 | 0x8a5498 | g[0xc1b330] <- 100.0f [0x858628] / 3.6f [0x870f50] | 27.777779 | none (dead store) |
| 0x853400 | 0x8a549c | g[0xc1b334] <- 1.0f [0x858624] / (50.0f [0x870f54] * 50.0f [0x870f54]) | 0.0004 | none (dead store) |
| 0x853420 | 0x8a54a0 | g[0xc1b338] <- g[0xc1b334] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853440 | 0x8a54a4 | g[0xc1b33c] <- 0.277778f [0x858630] / 50.0f [0x870f54] | 0.00555556 | none (dead store) |
| 0x853480 | 0x8a54ac | g[0xc1bfdc] <- 0.005f [0x870fc0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8534a0 | 0x8a54b0 | g[0xc1bfe0] <- 1.0f [0x858624] / 50.0f [0x870fc8] | 0.02 | none (dead store) |
| 0x8534c0 | 0x8a54b4 | g[0xc1bfe4] <- 100.0f [0x858628] / 3.6f [0x870fc4] | 27.777779 | none (dead store) |
| 0x8534e0 | 0x8a54b8 | g[0xc1bfe8] <- 1.0f [0x858624] / (50.0f [0x870fc8] * 50.0f [0x870fc8]) | 0.0004 | 0x853502 in 0x853500 __init_g_c1bfec |
| 0x853520 | 0x8a54c0 | g[0xc1bff0] <- 0.277778f [0x858630] / 50.0f [0x870fc8] | 0.00555556 | 0x6aadf2 in 0x6aab50 CAutomobile::PreRender |
| 0x853580 | 0x8a54c8 | g[0xc1c808] <- 0.005f [0x871274] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8535a0 | 0x8a54cc | g[0xc1c80c] <- 1.0f [0x858624] / 50.0f [0x87127c] | 0.02 | none (dead store) |
| 0x8535c0 | 0x8a54d0 | g[0xc1c810] <- 100.0f [0x858628] / 3.6f [0x871278] | 27.777779 | none (dead store) |
| 0x8535e0 | 0x8a54d4 | g[0xc1c814] <- 1.0f [0x858624] / (50.0f [0x87127c] * 50.0f [0x87127c]) | 0.0004 | 0x853602 in 0x853600 __init_g_c1c818 |
| 0x853640 | 0x8a54e0 | g[0xc1c824] <- 0.005f [0x8714b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853660 | 0x8a54e4 | g[0xc1c828] <- 1.0f [0x858624] / 50.0f [0x8714b8] | 0.02 | none (dead store) |
| 0x853680 | 0x8a54e8 | g[0xc1c82c] <- 100.0f [0x858628] / 3.6f [0x8714b4] | 27.777779 | none (dead store) |
| 0x8536a0 | 0x8a54ec | g[0xc1c830] <- 1.0f [0x858624] / (50.0f [0x8714b8] * 50.0f [0x8714b8]) | 0.0004 | none (dead store) |
| 0x8536c0 | 0x8a54f0 | g[0xc1c834] <- g[0xc1c830] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8536e0 | 0x8a54f4 | g[0xc1c838] <- 0.277778f [0x858630] / 50.0f [0x8714b8] | 0.00555556 | none (dead store) |
| 0x8537c0 | 0x8a5510 | g[0xc1c878] <- 0.005f [0x871648] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8537e0 | 0x8a5514 | g[0xc1c87c] <- 1.0f [0x858624] / 50.0f [0x871650] | 0.02 | none (dead store) |
| 0x853800 | 0x8a5518 | g[0xc1c880] <- 100.0f [0x858628] / 3.6f [0x87164c] | 27.777779 | none (dead store) |
| 0x853820 | 0x8a551c | g[0xc1c884] <- 1.0f [0x858624] / (50.0f [0x871650] * 50.0f [0x871650]) | 0.0004 | none (dead store) |
| 0x853840 | 0x8a5520 | g[0xc1c888] <- g[0xc1c884] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853860 | 0x8a5524 | g[0xc1c88c] <- 0.277778f [0x858630] / 50.0f [0x871650] | 0.00555556 | none (dead store) |
| 0x8538a0 | 0x8a552c | g[0xc1c974] <- 0.005f [0x871668] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8538c0 | 0x8a5530 | g[0xc1c978] <- 1.0f [0x858624] / 50.0f [0x871670] | 0.02 | none (dead store) |
| 0x8538e0 | 0x8a5534 | g[0xc1c97c] <- 100.0f [0x858628] / 3.6f [0x87166c] | 27.777779 | none (dead store) |
| 0x853900 | 0x8a5538 | g[0xc1c980] <- 1.0f [0x858624] / (50.0f [0x871670] * 50.0f [0x871670]) | 0.0004 | none (dead store) |
| 0x853920 | 0x8a553c | g[0xc1c984] <- g[0xc1c980] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853940 | 0x8a5540 | g[0xc1c988] <- 0.277778f [0x858630] / 50.0f [0x871670] | 0.00555556 | none (dead store) |
| 0x853960 | 0x8a5544 | g[0xc1cac0] <- 0.005f [0x8717c8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853980 | 0x8a5548 | g[0xc1cac4] <- 1.0f [0x858624] / 50.0f [0x8717d0] | 0.02 | none (dead store) |
| 0x8539a0 | 0x8a554c | g[0xc1cac8] <- 100.0f [0x858628] / 3.6f [0x8717cc] | 27.777779 | none (dead store) |
| 0x8539c0 | 0x8a5550 | g[0xc1cacc] <- 1.0f [0x858624] / (50.0f [0x8717d0] * 50.0f [0x8717d0]) | 0.0004 | none (dead store) |
| 0x8539e0 | 0x8a5554 | g[0xc1cad0] <- g[0xc1cacc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853a00 | 0x8a5558 | g[0xc1cad4] <- 0.277778f [0x858630] / 50.0f [0x8717d0] | 0.00555556 | none (dead store) |
| 0x853a20 | 0x8a555c | g[0xc1cae4] <- 0.005f [0x8718f4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853a40 | 0x8a5560 | g[0xc1cae8] <- 1.0f [0x858624] / 50.0f [0x8718fc] | 0.02 | none (dead store) |
| 0x853a60 | 0x8a5564 | g[0xc1caec] <- 100.0f [0x858628] / 3.6f [0x8718f8] | 27.777779 | none (dead store) |
| 0x853a80 | 0x8a5568 | g[0xc1caf0] <- 1.0f [0x858624] / (50.0f [0x8718fc] * 50.0f [0x8718fc]) | 0.0004 | none (dead store) |
| 0x853aa0 | 0x8a556c | g[0xc1caf4] <- g[0xc1caf0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853ac0 | 0x8a5570 | g[0xc1caf8] <- 0.277778f [0x858630] / 50.0f [0x8718fc] | 0.00555556 | none (dead store) |
| 0x853ae0 | 0x8a5574 | g[0xc1cb00] <- 0.005f [0x871ab0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853b00 | 0x8a5578 | g[0xc1cb04] <- 1.0f [0x858624] / 50.0f [0x871ab8] | 0.02 | none (dead store) |
| 0x853b20 | 0x8a557c | g[0xc1cb08] <- 100.0f [0x858628] / 3.6f [0x871ab4] | 27.777779 | none (dead store) |
| 0x853b40 | 0x8a5580 | g[0xc1cb0c] <- 1.0f [0x858624] / (50.0f [0x871ab8] * 50.0f [0x871ab8]) | 0.0004 | none (dead store) |
| 0x853b60 | 0x8a5584 | g[0xc1cb10] <- g[0xc1cb0c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853b80 | 0x8a5588 | g[0xc1cb14] <- 0.277778f [0x858630] / 50.0f [0x871ab8] | 0.00555556 | none (dead store) |
| 0x853ba0 | 0x8a558c | g[0xc1cb18] <- 0.005f [0x871c04] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853bc0 | 0x8a5590 | g[0xc1cb1c] <- 1.0f [0x858624] / 50.0f [0x871c0c] | 0.02 | none (dead store) |
| 0x853be0 | 0x8a5594 | g[0xc1cb20] <- 100.0f [0x858628] / 3.6f [0x871c08] | 27.777779 | none (dead store) |
| 0x853c00 | 0x8a5598 | g[0xc1cb24] <- 1.0f [0x858624] / (50.0f [0x871c0c] * 50.0f [0x871c0c]) | 0.0004 | 0x853c22 in 0x853c20 __init_g_c1cb28 |
| 0x853c40 | 0x8a55a0 | g[0xc1cb2c] <- 0.277778f [0x858630] / 50.0f [0x871c0c] | 0.00555556 | none (dead store) |
| 0x853c60 | 0x8a55a4 | g[0xc1cb44] <- 1.0f [0x858624] / 50.0f [0x871d48] | 0.02 | none (dead store) |
| 0x853c80 | 0x8a55a8 | g[0xc1cb48] <- 100.0f [0x858628] / 3.6f [0x871d44] | 27.777779 | none (dead store) |
| 0x853ca0 | 0x8a55ac | g[0xc1cb4c] <- 1.0f [0x858624] / (50.0f [0x871d48] * 50.0f [0x871d48]) | 0.0004 | none (dead store) |
| 0x853cc0 | 0x8a55b0 | g[0xc1cb50] <- g[0xc1cb4c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853d00 | 0x8a55b8 | g[0xc1cc5c] <- 0.005f [0x871d88] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853d20 | 0x8a55bc | g[0xc1cc60] <- 1.0f [0x858624] / 50.0f [0x871d90] | 0.02 | none (dead store) |
| 0x853d40 | 0x8a55c0 | g[0xc1cc64] <- 100.0f [0x858628] / 3.6f [0x871d8c] | 27.777779 | none (dead store) |
| 0x853d60 | 0x8a55c4 | g[0xc1cc68] <- 1.0f [0x858624] / (50.0f [0x871d90] * 50.0f [0x871d90]) | 0.0004 | none (dead store) |
| 0x853d80 | 0x8a55c8 | g[0xc1cc6c] <- g[0xc1cc68] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853da0 | 0x8a55cc | g[0xc1cc70] <- 0.277778f [0x858630] / 50.0f [0x871d90] | 0.00555556 | none (dead store) |
| 0x853f30 | 0x8a55fc | g[0xc228f4] <- 0.005f [0x871fec] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x853f50 | 0x8a5600 | g[0xc228f8] <- 1.0f [0x858624] / 50.0f [0x871ff4] | 0.02 | 0xcd947b in no catalogue function |
| 0x853f70 | 0x8a5604 | g[0xc228fc] <- 100.0f [0x858628] / 3.6f [0x871ff0] | 27.777779 | none (dead store) |
| 0x853f90 | 0x8a5608 | g[0xc22900] <- 1.0f [0x858624] / (50.0f [0x871ff4] * 50.0f [0x871ff4]) | 0.0004 | none (dead store) |
| 0x853fb0 | 0x8a560c | g[0xc22904] <- g[0xc22900] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853fd0 | 0x8a5610 | g[0xc22908] <- 0.277778f [0x858630] / 50.0f [0x871ff4] | 0.00555556 | none (dead store) |
| 0x854190 | 0x8a5648 | g[0xc2b978] <- 0.005f [0x8722ec] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8541b0 | 0x8a564c | g[0xc2b97c] <- 1.0f [0x858624] / 50.0f [0x8722f4] | 0.02 | none (dead store) |
| 0x8541d0 | 0x8a5650 | g[0xc2b980] <- 100.0f [0x858628] / 3.6f [0x8722f0] | 27.777779 | none (dead store) |
| 0x8541f0 | 0x8a5654 | g[0xc2b984] <- 1.0f [0x858624] / (50.0f [0x8722f4] * 50.0f [0x8722f4]) | 0.0004 | none (dead store) |
| 0x854210 | 0x8a5658 | g[0xc2b988] <- g[0xc2b984] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854230 | 0x8a565c | g[0xc2b98c] <- 0.277778f [0x858630] / 50.0f [0x8722f4] | 0.00555556 | none (dead store) |
| 0x854250 | 0x8a5660 | g[0xc2b990] <- 0.005f [0x8722f8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854270 | 0x8a5664 | g[0xc2b994] <- 1.0f [0x858624] / 50.0f [0x872300] | 0.02 | none (dead store) |
| 0x854290 | 0x8a5668 | g[0xc2b998] <- 100.0f [0x858628] / 3.6f [0x8722fc] | 27.777779 | none (dead store) |
| 0x8542b0 | 0x8a566c | g[0xc2b99c] <- 1.0f [0x858624] / (50.0f [0x872300] * 50.0f [0x872300]) | 0.0004 | none (dead store) |
| 0x8542d0 | 0x8a5670 | g[0xc2b9a0] <- g[0xc2b99c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8542f0 | 0x8a5674 | g[0xc2b9a4] <- 0.277778f [0x858630] / 50.0f [0x872300] | 0.00555556 | none (dead store) |
| 0x854310 | 0x8a5678 | g[0xc2b9ac] <- 1.0f [0x858624] / 50.0f [0x872344] | 0.02 | none (dead store) |
| 0x854330 | 0x8a567c | g[0xc2b9b0] <- 100.0f [0x858628] / 3.6f [0x872340] | 27.777779 | none (dead store) |
| 0x854370 | 0x8a5684 | g[0xc2b9b8] <- g[0xc2b9b4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8543b0 | 0x8a568c | g[0xc2b9c0] <- 0.005f [0x87234c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

## Other initialisers and destructors

## 0x856a10 __dtor_local_static_CMatrix_c1c220  (catalogue: unknown/REA, port level 2)
- atexit destructor of a function-local `static CMatrix` at 0xC1C220, registered on first use by CAutomobile::RcbanditCheck1CarWheels (caller list: 0x6b3f70). Body: ECX = 0xC1C220; tail jump to 0x59ACD0 CMatrix::~CMatrix.
