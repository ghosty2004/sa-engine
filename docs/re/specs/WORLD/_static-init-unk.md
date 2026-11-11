# WORLD: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x849860 | 0x8a4130 | g[0x9690d0] <- 0.005f [0x858cf4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849880 | 0x8a4134 | g[0x9690d4] <- 1.0f [0x858624] / 50.0f [0x858cfc] | 0.02 | none (dead store) |
| 0x8498a0 | 0x8a4138 | g[0x9690d8] <- 100.0f [0x858628] / 3.6f [0x858cf8] | 27.777779 | none (dead store) |
| 0x8498c0 | 0x8a413c | g[0x9690dc] <- 1.0f [0x858624] / (50.0f [0x858cfc] * 50.0f [0x858cfc]) | 0.0004 | none (dead store) |
| 0x8498e0 | 0x8a4140 | g[0x9690e0] <- g[0x9690dc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849900 | 0x8a4144 | g[0x9690e4] <- 0.277778f [0x858630] / 50.0f [0x858cfc] | 0.00555556 | none (dead store) |
| 0x849aa0 | 0x8a4178 | g[0x96a708] <- 0.005f [0x8594f4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849ac0 | 0x8a417c | g[0x96a70c] <- 1.0f [0x858624] / 50.0f [0x8594fc] | 0.02 | none (dead store) |
| 0x849ae0 | 0x8a4180 | g[0x96a710] <- 100.0f [0x858628] / 3.6f [0x8594f8] | 27.777779 | none (dead store) |
| 0x849b00 | 0x8a4184 | g[0x96a714] <- 1.0f [0x858624] / (50.0f [0x8594fc] * 50.0f [0x8594fc]) | 0.0004 | 0x849b22 in 0x849b20 __init_g_96a718 |
| 0x849b60 | 0x8a4190 | g[0x96a7e4] <- 0.005f [0x859530] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849b80 | 0x8a4194 | g[0x96a7e8] <- 1.0f [0x858624] / 50.0f [0x859538] | 0.02 | none (dead store) |
| 0x849ba0 | 0x8a4198 | g[0x96a7ec] <- 100.0f [0x858628] / 3.6f [0x859534] | 27.777779 | none (dead store) |
| 0x849bc0 | 0x8a419c | g[0x96a7f0] <- 1.0f [0x858624] / (50.0f [0x859538] * 50.0f [0x859538]) | 0.0004 | none (dead store) |
| 0x849be0 | 0x8a41a0 | g[0x96a7f4] <- g[0x96a7f0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849c00 | 0x8a41a4 | g[0x96a7f8] <- 0.277778f [0x858630] / 50.0f [0x859538] | 0.00555556 | none (dead store) |
| 0x849d10 | 0x8a41c4 | g[0x96aba4] <- 0.005f [0x8598a0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849d30 | 0x8a41c8 | g[0x96aba8] <- 1.0f [0x858624] / 50.0f [0x8598a8] | 0.02 | none (dead store) |
| 0x849d50 | 0x8a41cc | g[0x96abac] <- 100.0f [0x858628] / 3.6f [0x8598a4] | 27.777779 | none (dead store) |
| 0x849d70 | 0x8a41d0 | g[0x96abb0] <- 1.0f [0x858624] / (50.0f [0x8598a8] * 50.0f [0x8598a8]) | 0.0004 | none (dead store) |
| 0x849d90 | 0x8a41d4 | g[0x96abb4] <- g[0x96abb0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849db0 | 0x8a41d8 | g[0x96abb8] <- 0.277778f [0x858630] / 50.0f [0x8598a8] | 0.00555556 | none (dead store) |
| 0x849dd0 | 0x8a41dc | g[0x96c02c] <- 0.005f [0x85997c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x849df0 | 0x8a41e0 | g[0x96c030] <- 1.0f [0x858624] / 50.0f [0x859984] | 0.02 | none (dead store) |
| 0x849e10 | 0x8a41e4 | g[0x96c034] <- 100.0f [0x858628] / 3.6f [0x859980] | 27.777779 | none (dead store) |
| 0x849e30 | 0x8a41e8 | g[0x96c038] <- 1.0f [0x858624] / (50.0f [0x859984] * 50.0f [0x859984]) | 0.0004 | none (dead store) |
| 0x849e50 | 0x8a41ec | g[0x96c03c] <- g[0x96c038] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x849e70 | 0x8a41f0 | g[0x96c040] <- 0.277778f [0x858630] / 50.0f [0x859984] | 0.00555556 | none (dead store) |
| 0x84a0a0 | 0x8a4234 | g[0x977ba0] <- 0.005f [0x859acc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a0c0 | 0x8a4238 | g[0x977ba4] <- 1.0f [0x858624] / 50.0f [0x859ad4] | 0.02 | none (dead store) |
| 0x84a0e0 | 0x8a423c | g[0x977ba8] <- 100.0f [0x858628] / 3.6f [0x859ad0] | 27.777779 | none (dead store) |
| 0x84a100 | 0x8a4240 | g[0x977bac] <- 1.0f [0x858624] / (50.0f [0x859ad4] * 50.0f [0x859ad4]) | 0.0004 | none (dead store) |
| 0x84a120 | 0x8a4244 | g[0x977bb0] <- g[0x977bac] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a140 | 0x8a4248 | g[0x977bb4] <- 0.277778f [0x858630] / 50.0f [0x859ad4] | 0.00555556 | none (dead store) |
| 0x84a160 | 0x8a424c | g[0x977bcc] <- 0.005f [0x859ae0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a180 | 0x8a4250 | g[0x977bd0] <- 1.0f [0x858624] / 50.0f [0x859ae8] | 0.02 | none (dead store) |
| 0x84a1a0 | 0x8a4254 | g[0x977bd4] <- 100.0f [0x858628] / 3.6f [0x859ae4] | 27.777779 | none (dead store) |
| 0x84a1c0 | 0x8a4258 | g[0x977bd8] <- 1.0f [0x858624] / (50.0f [0x859ae8] * 50.0f [0x859ae8]) | 0.0004 | none (dead store) |
| 0x84a1e0 | 0x8a425c | g[0x977bdc] <- g[0x977bd8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a200 | 0x8a4260 | g[0x977be0] <- 0.277778f [0x858630] / 50.0f [0x859ae8] | 0.00555556 | none (dead store) |
| 0x84a3c0 | 0x8a4298 | g[0x97f64c] <- 0.005f [0x859c04] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a3e0 | 0x8a429c | g[0x97f650] <- 1.0f [0x858624] / 50.0f [0x859c0c] | 0.02 | none (dead store) |
| 0x84a400 | 0x8a42a0 | g[0x97f654] <- 100.0f [0x858628] / 3.6f [0x859c08] | 27.777779 | none (dead store) |
| 0x84a420 | 0x8a42a4 | g[0x97f658] <- 1.0f [0x858624] / (50.0f [0x859c0c] * 50.0f [0x859c0c]) | 0.0004 | none (dead store) |
| 0x84a440 | 0x8a42a8 | g[0x97f65c] <- g[0x97f658] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a460 | 0x8a42ac | g[0x97f660] <- 0.277778f [0x858630] / 50.0f [0x859c0c] | 0.00555556 | none (dead store) |
| 0x84a480 | 0x8a42b0 | g[0xa43098] <- 0.005f [0x859c10] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a4a0 | 0x8a42b4 | g[0xa4309c] <- 1.0f [0x858624] / 50.0f [0x859c18] | 0.02 | none (dead store) |
| 0x84a4c0 | 0x8a42b8 | g[0xa430a0] <- 100.0f [0x858628] / 3.6f [0x859c14] | 27.777779 | none (dead store) |
| 0x84a4e0 | 0x8a42bc | g[0xa430a4] <- 1.0f [0x858624] / (50.0f [0x859c18] * 50.0f [0x859c18]) | 0.0004 | none (dead store) |
| 0x84a500 | 0x8a42c0 | g[0xa430a8] <- g[0xa430a4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a520 | 0x8a42c4 | g[0xa430ac] <- 0.277778f [0x858630] / 50.0f [0x859c18] | 0.00555556 | none (dead store) |
| 0x84a560 | 0x8a42cc | g[0xa43310] <- 0.005f [0x859c70] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a8a0 | 0x8a4330 | g[0xa95160] <- 0.005f [0x859e74] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84a8c0 | 0x8a4334 | g[0xa95164] <- 1.0f [0x858624] / 50.0f [0x859e7c] | 0.02 | none (dead store) |
| 0x84a8e0 | 0x8a4338 | g[0xa95168] <- 100.0f [0x858628] / 3.6f [0x859e78] | 27.777779 | none (dead store) |
| 0x84a900 | 0x8a433c | g[0xa9516c] <- 1.0f [0x858624] / (50.0f [0x859e7c] * 50.0f [0x859e7c]) | 0.0004 | none (dead store) |
| 0x84a920 | 0x8a4340 | g[0xa95170] <- g[0xa9516c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84a940 | 0x8a4344 | g[0xa95174] <- 0.277778f [0x858630] / 50.0f [0x859e7c] | 0.00555556 | none (dead store) |
| 0x84aa20 | 0x8a4360 | g[0xa95194] <- 0.005f [0x859ee8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84aa40 | 0x8a4364 | g[0xa95198] <- 1.0f [0x858624] / 50.0f [0x859ef0] | 0.02 | none (dead store) |
| 0x84aa60 | 0x8a4368 | g[0xa9519c] <- 100.0f [0x858628] / 3.6f [0x859eec] | 27.777779 | none (dead store) |
| 0x84aa80 | 0x8a436c | g[0xa951a0] <- 1.0f [0x858624] / (50.0f [0x859ef0] * 50.0f [0x859ef0]) | 0.0004 | none (dead store) |
| 0x84aaa0 | 0x8a4370 | g[0xa951a4] <- g[0xa951a0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84aac0 | 0x8a4374 | g[0xa951a8] <- 0.277778f [0x858630] / 50.0f [0x859ef0] | 0.00555556 | none (dead store) |
| 0x84aae0 | 0x8a4378 | g[0xa951ac] <- 0.005f [0x859f04] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ab00 | 0x8a437c | g[0xa951b0] <- 1.0f [0x858624] / 50.0f [0x859f0c] | 0.02 | none (dead store) |
| 0x84ab20 | 0x8a4380 | g[0xa951b4] <- 100.0f [0x858628] / 3.6f [0x859f08] | 27.777779 | none (dead store) |
| 0x84ab40 | 0x8a4384 | g[0xa951b8] <- 1.0f [0x858624] / (50.0f [0x859f0c] * 50.0f [0x859f0c]) | 0.0004 | none (dead store) |
| 0x84ab60 | 0x8a4388 | g[0xa951bc] <- g[0xa951b8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ab80 | 0x8a438c | g[0xa951c0] <- 0.277778f [0x858630] / 50.0f [0x859f0c] | 0.00555556 | none (dead store) |
| 0x84aba0 | 0x8a4390 | g[0xa951c4] <- 0.005f [0x859f10] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84abc0 | 0x8a4394 | g[0xa951c8] <- 1.0f [0x858624] / 50.0f [0x859f18] | 0.02 | none (dead store) |
| 0x84abe0 | 0x8a4398 | g[0xa951cc] <- 100.0f [0x858628] / 3.6f [0x859f14] | 27.777779 | none (dead store) |
| 0x84ac00 | 0x8a439c | g[0xa951d0] <- 1.0f [0x858624] / (50.0f [0x859f18] * 50.0f [0x859f18]) | 0.0004 | none (dead store) |
| 0x84ac20 | 0x8a43a0 | g[0xa951d4] <- g[0xa951d0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ac40 | 0x8a43a4 | g[0xa951d8] <- 0.277778f [0x858630] / 50.0f [0x859f18] | 0.00555556 | none (dead store) |
| 0x84ad20 | 0x8a43c0 | g[0xa9577c] <- 0.005f [0x859f44] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ad40 | 0x8a43c4 | g[0xa95780] <- 1.0f [0x858624] / 50.0f [0x859f4c] | 0.02 | none (dead store) |
| 0x84ad60 | 0x8a43c8 | g[0xa95784] <- 100.0f [0x858628] / 3.6f [0x859f48] | 27.777779 | none (dead store) |
| 0x84ad80 | 0x8a43cc | g[0xa95788] <- 1.0f [0x858624] / (50.0f [0x859f4c] * 50.0f [0x859f4c]) | 0.0004 | none (dead store) |
| 0x84ada0 | 0x8a43d0 | g[0xa9578c] <- g[0xa95788] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84adc0 | 0x8a43d4 | g[0xa95790] <- 0.277778f [0x858630] / 50.0f [0x859f4c] | 0.00555556 | none (dead store) |
| 0x84ade0 | 0x8a43d8 | g[0xa95794] <- 0.005f [0x859f54] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ae00 | 0x8a43dc | g[0xa95798] <- 1.0f [0x858624] / 50.0f [0x859f5c] | 0.02 | none (dead store) |
| 0x84ae20 | 0x8a43e0 | g[0xa9579c] <- 100.0f [0x858628] / 3.6f [0x859f58] | 27.777779 | none (dead store) |
| 0x84ae40 | 0x8a43e4 | g[0xa957a0] <- 1.0f [0x858624] / (50.0f [0x859f5c] * 50.0f [0x859f5c]) | 0.0004 | none (dead store) |
| 0x84ae60 | 0x8a43e8 | g[0xa957a4] <- g[0xa957a0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ae80 | 0x8a43ec | g[0xa957a8] <- 0.277778f [0x858630] / 50.0f [0x859f5c] | 0.00555556 | none (dead store) |
| 0x84aea0 | 0x8a43f0 | g[0xa957ac] <- 0.005f [0x859f84] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84aec0 | 0x8a43f4 | g[0xa957b0] <- 1.0f [0x858624] / 50.0f [0x859f8c] | 0.02 | none (dead store) |
| 0x84aee0 | 0x8a43f8 | g[0xa957b4] <- 100.0f [0x858628] / 3.6f [0x859f88] | 27.777779 | none (dead store) |
| 0x84af00 | 0x8a43fc | g[0xa957b8] <- 1.0f [0x858624] / (50.0f [0x859f8c] * 50.0f [0x859f8c]) | 0.0004 | none (dead store) |
| 0x84af20 | 0x8a4400 | g[0xa957bc] <- g[0xa957b8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84af40 | 0x8a4404 | g[0xa957c0] <- 0.277778f [0x858630] / 50.0f [0x859f8c] | 0.00555556 | none (dead store) |
| 0x84af60 | 0x8a4408 | g[0xa957c4] <- 0.005f [0x85a0c4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84af80 | 0x8a440c | g[0xa957c8] <- 1.0f [0x858624] / 50.0f [0x85a0cc] | 0.02 | none (dead store) |
| 0x84afa0 | 0x8a4410 | g[0xa957cc] <- 100.0f [0x858628] / 3.6f [0x85a0c8] | 27.777779 | none (dead store) |
| 0x84afc0 | 0x8a4414 | g[0xa957d0] <- 1.0f [0x858624] / (50.0f [0x85a0cc] * 50.0f [0x85a0cc]) | 0.0004 | none (dead store) |
| 0x84afe0 | 0x8a4418 | g[0xa957d4] <- g[0xa957d0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b000 | 0x8a441c | g[0xa957d8] <- 0.277778f [0x858630] / 50.0f [0x85a0cc] | 0.00555556 | none (dead store) |
| 0x84b020 | 0x8a4420 | g[0xa957dc] <- 0.005f [0x85a320] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b040 | 0x8a4424 | g[0xa957e0] <- 1.0f [0x858624] / 50.0f [0x85a328] | 0.02 | none (dead store) |
| 0x84b060 | 0x8a4428 | g[0xa957e4] <- 100.0f [0x858628] / 3.6f [0x85a324] | 27.777779 | none (dead store) |
| 0x84b080 | 0x8a442c | g[0xa957e8] <- 1.0f [0x858624] / (50.0f [0x85a328] * 50.0f [0x85a328]) | 0.0004 | none (dead store) |
| 0x84b0a0 | 0x8a4430 | g[0xa957ec] <- g[0xa957e8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b0c0 | 0x8a4434 | g[0xa957f0] <- 0.277778f [0x858630] / 50.0f [0x85a328] | 0.00555556 | none (dead store) |
| 0x84b0e0 | 0x8a4438 | g[0xa957f4] <- 0.005f [0x85a418] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b100 | 0x8a443c | g[0xa95800] <- 0.005f [0x85a41c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b120 | 0x8a4440 | g[0xa95804] <- 1.0f [0x858624] / 50.0f [0x85a424] | 0.02 | none (dead store) |
| 0x84b140 | 0x8a4444 | g[0xa95808] <- 100.0f [0x858628] / 3.6f [0x85a420] | 27.777779 | none (dead store) |
| 0x84b160 | 0x8a4448 | g[0xa9580c] <- 1.0f [0x858624] / (50.0f [0x85a424] * 50.0f [0x85a424]) | 0.0004 | none (dead store) |
| 0x84b180 | 0x8a444c | g[0xa95810] <- g[0xa9580c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b1a0 | 0x8a4450 | g[0xa95814] <- 0.277778f [0x858630] / 50.0f [0x85a424] | 0.00555556 | none (dead store) |
| 0x84b1e0 | 0x8a4458 | g[0xa9a7f4] <- 0.005f [0x85a4d0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84b200 | 0x8a445c | g[0xa9a7f8] <- 1.0f [0x858624] / 50.0f [0x85a4d8] | 0.02 | none (dead store) |
| 0x84b220 | 0x8a4460 | g[0xa9a7fc] <- 100.0f [0x858628] / 3.6f [0x85a4d4] | 27.777779 | none (dead store) |
| 0x84b240 | 0x8a4464 | g[0xa9a800] <- 1.0f [0x858624] / (50.0f [0x85a4d8] * 50.0f [0x85a4d8]) | 0.0004 | none (dead store) |
| 0x84b260 | 0x8a4468 | g[0xa9a804] <- g[0xa9a800] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84b280 | 0x8a446c | g[0xa9a808] <- 0.277778f [0x858630] / 50.0f [0x85a4d8] | 0.00555556 | none (dead store) |
| 0x84dab0 | 0x8a4978 | g[0xb717ec] <- 0.005f [0x8638b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84dad0 | 0x8a497c | g[0xb717f0] <- 1.0f [0x858624] / 50.0f [0x8638b8] | 0.02 | none (dead store) |
| 0x84daf0 | 0x8a4980 | g[0xb717f4] <- 100.0f [0x858628] / 3.6f [0x8638b4] | 27.777779 | none (dead store) |
| 0x84db10 | 0x8a4984 | g[0xb717f8] <- 1.0f [0x858624] / (50.0f [0x8638b8] * 50.0f [0x8638b8]) | 0.0004 | none (dead store) |
| 0x84db30 | 0x8a4988 | g[0xb717fc] <- g[0xb717f8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84db50 | 0x8a498c | g[0xb71800] <- 0.277778f [0x858630] / 50.0f [0x8638b8] | 0.00555556 | none (dead store) |
| 0x84e560 | 0x8a4ad0 | g[0xb7689c] <- 0.005f [0x863e1c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e580 | 0x8a4ad4 | g[0xb768a0] <- 1.0f [0x858624] / 50.0f [0x863e24] | 0.02 | none (dead store) |
| 0x84e5a0 | 0x8a4ad8 | g[0xb768a4] <- 100.0f [0x858628] / 3.6f [0x863e20] | 27.777779 | none (dead store) |
| 0x84e5c0 | 0x8a4adc | g[0xb768a8] <- 1.0f [0x858624] / (50.0f [0x863e24] * 50.0f [0x863e24]) | 0.0004 | none (dead store) |
| 0x84e5e0 | 0x8a4ae0 | g[0xb768ac] <- g[0xb768a8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e600 | 0x8a4ae4 | g[0xb768b0] <- 0.277778f [0x858630] / 50.0f [0x863e24] | 0.00555556 | none (dead store) |
| 0x84e800 | 0x8a4b28 | g[0xb7cb90] <- 0.005f [0x864e24] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e820 | 0x8a4b2c | g[0xb7cb94] <- 1.0f [0x858624] / 50.0f [0x864e2c] | 0.02 | none (dead store) |
| 0x84e840 | 0x8a4b30 | g[0xb7cb98] <- 100.0f [0x858628] / 3.6f [0x864e28] | 27.777779 | none (dead store) |
| 0x84e860 | 0x8a4b34 | g[0xb7cb9c] <- 1.0f [0x858624] / (50.0f [0x864e2c] * 50.0f [0x864e2c]) | 0.0004 | none (dead store) |
| 0x84e880 | 0x8a4b38 | g[0xb7cba0] <- g[0xb7cb9c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e8a0 | 0x8a4b3c | g[0xb7cba4] <- 0.277778f [0x858630] / 50.0f [0x864e2c] | 0.00555556 | none (dead store) |
| 0x84e8c0 | 0x8a4b40 | g[0xb7cd80] <- 0.005f [0x864e8c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e8e0 | 0x8a4b44 | g[0xb7cd84] <- 1.0f [0x858624] / 50.0f [0x864e94] | 0.02 | none (dead store) |
| 0x84e900 | 0x8a4b48 | g[0xb7cd88] <- 100.0f [0x858628] / 3.6f [0x864e90] | 27.777779 | none (dead store) |
| 0x84e920 | 0x8a4b4c | g[0xb7cd8c] <- 1.0f [0x858624] / (50.0f [0x864e94] * 50.0f [0x864e94]) | 0.0004 | none (dead store) |
| 0x84e940 | 0x8a4b50 | g[0xb7cd90] <- g[0xb7cd8c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e960 | 0x8a4b54 | g[0xb7cd94] <- 0.277778f [0x858630] / 50.0f [0x864e94] | 0.00555556 | none (dead store) |
| 0x84eca0 | 0x8a4bb8 | g[0xba18dc] <- 0.005f [0x865050] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ecc0 | 0x8a4bbc | g[0xba18e0] <- 1.0f [0x858624] / 50.0f [0x865058] | 0.02 | none (dead store) |
| 0x84ece0 | 0x8a4bc0 | g[0xba18e4] <- 100.0f [0x858628] / 3.6f [0x865054] | 27.777779 | none (dead store) |
| 0x84ed00 | 0x8a4bc4 | g[0xba18e8] <- 1.0f [0x858624] / (50.0f [0x865058] * 50.0f [0x865058]) | 0.0004 | none (dead store) |
| 0x84ed20 | 0x8a4bc8 | g[0xba18ec] <- g[0xba18e8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ed40 | 0x8a4bcc | g[0xba18f0] <- 0.277778f [0x858630] / 50.0f [0x865058] | 0.00555556 | none (dead store) |
| 0x84f210 | 0x8a4c60 | g[0xbab344] <- 0.005f [0x866cb8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f230 | 0x8a4c64 | g[0xbab348] <- 1.0f [0x858624] / 50.0f [0x866cc0] | 0.02 | none (dead store) |
| 0x84f250 | 0x8a4c68 | g[0xbab34c] <- 100.0f [0x858628] / 3.6f [0x866cbc] | 27.777779 | none (dead store) |
| 0x84f270 | 0x8a4c6c | g[0xbab350] <- 1.0f [0x858624] / (50.0f [0x866cc0] * 50.0f [0x866cc0]) | 0.0004 | none (dead store) |
| 0x84f290 | 0x8a4c70 | g[0xbab354] <- g[0xbab350] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f380 | 0x8a4c88 | g[0xbaf5f8] <- 0.005f [0x866d08] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f3a0 | 0x8a4c8c | g[0xbaf5fc] <- 1.0f [0x858624] / 50.0f [0x866d10] | 0.02 | none (dead store) |
| 0x84f3c0 | 0x8a4c90 | g[0xbaf600] <- 100.0f [0x858628] / 3.6f [0x866d0c] | 27.777779 | none (dead store) |
| 0x84f3e0 | 0x8a4c94 | g[0xbaf604] <- 1.0f [0x858624] / (50.0f [0x866d10] * 50.0f [0x866d10]) | 0.0004 | none (dead store) |
| 0x84f400 | 0x8a4c98 | g[0xbaf608] <- g[0xbaf604] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f420 | 0x8a4c9c | g[0xbaf60c] <- 0.277778f [0x858630] / 50.0f [0x866d10] | 0.00555556 | none (dead store) |
| 0x84f440 | 0x8a4ca0 | g[0xbaf610] <- 0.005f [0x866d18] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f460 | 0x8a4ca4 | g[0xbaf614] <- 1.0f [0x858624] / 50.0f [0x866d20] | 0.02 | none (dead store) |
| 0x84f480 | 0x8a4ca8 | g[0xbaf618] <- 100.0f [0x858628] / 3.6f [0x866d1c] | 27.777779 | none (dead store) |
| 0x84f4a0 | 0x8a4cac | g[0xbaf61c] <- 1.0f [0x858624] / (50.0f [0x866d20] * 50.0f [0x866d20]) | 0.0004 | none (dead store) |
| 0x84f4c0 | 0x8a4cb0 | g[0xbaf620] <- g[0xbaf61c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f4e0 | 0x8a4cb4 | g[0xbaf624] <- 0.277778f [0x858630] / 50.0f [0x866d20] | 0.00555556 | none (dead store) |
| 0x84f500 | 0x8a4cb8 | g[0xbaf628] <- 0.005f [0x866d54] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f520 | 0x8a4cbc | g[0xbaf62c] <- 1.0f [0x858624] / 50.0f [0x866d5c] | 0.02 | none (dead store) |
| 0x84f540 | 0x8a4cc0 | g[0xbaf630] <- 100.0f [0x858628] / 3.6f [0x866d58] | 27.777779 | none (dead store) |
| 0x84f560 | 0x8a4cc4 | g[0xbaf634] <- 1.0f [0x858624] / (50.0f [0x866d5c] * 50.0f [0x866d5c]) | 0.0004 | none (dead store) |
| 0x84f580 | 0x8a4cc8 | g[0xbaf638] <- g[0xbaf634] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f5a0 | 0x8a4ccc | g[0xbaf63c] <- 0.277778f [0x858630] / 50.0f [0x866d5c] | 0.00555556 | none (dead store) |
| 0x84f5c0 | 0x8a4cd0 | g[0xbaf640] <- 0.005f [0x866d60] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f5e0 | 0x8a4cd4 | g[0xbaf644] <- 1.0f [0x858624] / 50.0f [0x866d68] | 0.02 | none (dead store) |
| 0x84f600 | 0x8a4cd8 | g[0xbaf648] <- 100.0f [0x858628] / 3.6f [0x866d64] | 27.777779 | none (dead store) |
| 0x84f620 | 0x8a4cdc | g[0xbaf64c] <- 1.0f [0x858624] / (50.0f [0x866d68] * 50.0f [0x866d68]) | 0.0004 | none (dead store) |
| 0x84f640 | 0x8a4ce0 | g[0xbaf650] <- g[0xbaf64c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f660 | 0x8a4ce4 | g[0xbaf654] <- 0.277778f [0x858630] / 50.0f [0x866d68] | 0.00555556 | none (dead store) |
| 0x84f680 | 0x8a4ce8 | g[0xbaf658] <- 0.005f [0x866d6c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f6a0 | 0x8a4cec | g[0xbaf65c] <- 1.0f [0x858624] / 50.0f [0x866d74] | 0.02 | none (dead store) |
| 0x84f6c0 | 0x8a4cf0 | g[0xbaf660] <- 100.0f [0x858628] / 3.6f [0x866d70] | 27.777779 | none (dead store) |
| 0x84f6e0 | 0x8a4cf4 | g[0xbaf664] <- 1.0f [0x858624] / (50.0f [0x866d74] * 50.0f [0x866d74]) | 0.0004 | none (dead store) |
| 0x84f700 | 0x8a4cf8 | g[0xbaf668] <- g[0xbaf664] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f720 | 0x8a4cfc | g[0xbaf66c] <- 0.277778f [0x858630] / 50.0f [0x866d74] | 0.00555556 | none (dead store) |
| 0x84f760 | 0x8a4d04 | g[0xbb3dcc] <- 0.005f [0x866d78] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f780 | 0x8a4d08 | g[0xbb3dd0] <- 1.0f [0x858624] / 50.0f [0x866d80] | 0.02 | none (dead store) |
| 0x84f7a0 | 0x8a4d0c | g[0xbb3dd4] <- 100.0f [0x858628] / 3.6f [0x866d7c] | 27.777779 | none (dead store) |
| 0x84f7c0 | 0x8a4d10 | g[0xbb3dd8] <- 1.0f [0x858624] / (50.0f [0x866d80] * 50.0f [0x866d80]) | 0.0004 | none (dead store) |
| 0x84f7e0 | 0x8a4d14 | g[0xbb3ddc] <- g[0xbb3dd8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f800 | 0x8a4d18 | g[0xbb3de0] <- 0.277778f [0x858630] / 50.0f [0x866d80] | 0.00555556 | none (dead store) |
| 0x84f8e0 | 0x8a4d34 | g[0xbb4200] <- 0.005f [0x866da4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f900 | 0x8a4d38 | g[0xbb4204] <- 1.0f [0x858624] / 50.0f [0x866dac] | 0.02 | none (dead store) |
| 0x84f920 | 0x8a4d3c | g[0xbb4208] <- 100.0f [0x858628] / 3.6f [0x866da8] | 27.777779 | none (dead store) |
| 0x84f940 | 0x8a4d40 | g[0xbb420c] <- 1.0f [0x858624] / (50.0f [0x866dac] * 50.0f [0x866dac]) | 0.0004 | none (dead store) |
| 0x84f960 | 0x8a4d44 | g[0xbb4210] <- g[0xbb420c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f980 | 0x8a4d48 | g[0xbb4214] <- 0.277778f [0x858630] / 50.0f [0x866dac] | 0.00555556 | none (dead store) |
| 0x84fc20 | 0x8a4d9c | g[0xbb4a74] <- 0.005f [0x866f3c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84fc40 | 0x8a4da0 | g[0xbb4a78] <- 1.0f [0x858624] / 50.0f [0x866f44] | 0.02 | none (dead store) |
| 0x84fc60 | 0x8a4da4 | g[0xbb4a7c] <- 100.0f [0x858628] / 3.6f [0x866f40] | 27.777779 | none (dead store) |
| 0x84fc80 | 0x8a4da8 | g[0xbb4a80] <- 1.0f [0x858624] / (50.0f [0x866f44] * 50.0f [0x866f44]) | 0.0004 | none (dead store) |
| 0x84fca0 | 0x8a4dac | g[0xbb4a84] <- g[0xbb4a80] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84fcc0 | 0x8a4db0 | g[0xbb4a88] <- 0.277778f [0x858630] / 50.0f [0x866f44] | 0.00555556 | none (dead store) |
| 0x854a00 | 0x8a5754 | g[0xc40424] <- 1.0f [0x858624] / (50.0f [0x87267c] * 50.0f [0x87267c]) | 0.0004 | 0x439f82 in 0x439f60 CCheat::CountrysideInvasionCheat |

## Other initialisers and destructors

## 0x84e2e0 __init_gDummyMatrix  (catalogue: unknown/REA, port level 1)
- push 1.0f (imm 0x3F800000); ECX = 0xB74240 (gDummyMatrix, gr StaticRef<CMatrix>); call 0x59AED0 CMatrix::SetScale(float) (the 1-arg CMatrix(float scale) form); _atexit 0x821D1E(0x856480).

## 0x84fdc0 __init_g_procObjMan  (catalogue: unknown/REA, port level 2)
- ECX = 0xBB7CB0 (g_procObjMan); call 0x5A3E20 ProcObjectMan_c::ProcObjectMan_c; _atexit 0x821D1E(0x856670).

## 0x854c70 __init_854c70  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856b00) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (WORLD).

## 0x854c80 __init_854c80  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856b10) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (WORLD).

## 0x856480 __dtor_gDummyMatrix  (catalogue: unknown/REA, port level 2)
- ECX = 0xB74240; tail jump to 0x59ACD0 CMatrix::~CMatrix.

## 0x856670 __dtor_g_procObjMan  (catalogue: unknown/REA, port level 2)
- ECX = 0xBB7CB0; tail jump to 0x5A3090 ProcObjectMan_c::~ProcObjectMan_c.

## 0x856b00 __dtor_854c70  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x854c70).

## 0x856b10 __dtor_854c80  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x854c80).
