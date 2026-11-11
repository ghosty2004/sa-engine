# AUDIO: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84c0e0 | 0x8a4634 | g[0xb5f880] <- 0.005f [0x85eec0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c100 | 0x8a4638 | g[0xb5f884] <- 1.0f [0x858624] / 50.0f [0x85eec8] | 0.02 | none (dead store) |
| 0x84c120 | 0x8a463c | g[0xb5f888] <- 100.0f [0x858628] / 3.6f [0x85eec4] | 27.777779 | none (dead store) |
| 0x84c140 | 0x8a4640 | g[0xb5f88c] <- 1.0f [0x858624] / (50.0f [0x85eec8] * 50.0f [0x85eec8]) | 0.0004 | none (dead store) |
| 0x84c160 | 0x8a4644 | g[0xb5f890] <- g[0xb5f88c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c180 | 0x8a4648 | g[0xb5f894] <- 0.277778f [0x858630] / 50.0f [0x85eec8] | 0.00555556 | none (dead store) |
| 0x84c1b0 | 0x8a4650 | g[0xb5f89c] <- 0.005f [0x85f060] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c1d0 | 0x8a4654 | g[0xb5f8a0] <- 1.0f [0x858624] / 50.0f [0x85f068] | 0.02 | none (dead store) |
| 0x84c1f0 | 0x8a4658 | g[0xb5f8a4] <- 100.0f [0x858628] / 3.6f [0x85f064] | 27.777779 | none (dead store) |
| 0x84c210 | 0x8a465c | g[0xb5f8a8] <- 1.0f [0x858624] / (50.0f [0x85f068] * 50.0f [0x85f068]) | 0.0004 | none (dead store) |
| 0x84c230 | 0x8a4660 | g[0xb5f8ac] <- g[0xb5f8a8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c250 | 0x8a4664 | g[0xb5f8b0] <- 0.277778f [0x858630] / 50.0f [0x85f068] | 0.00555556 | none (dead store) |
| 0x84c2b0 | 0x8a4670 | g[0xb61290] <- 0.005f [0x85f084] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c2d0 | 0x8a4674 | g[0xb61294] <- 1.0f [0x858624] / 50.0f [0x85f08c] | 0.02 | none (dead store) |
| 0x84c2f0 | 0x8a4678 | g[0xb61298] <- 100.0f [0x858628] / 3.6f [0x85f088] | 27.777779 | none (dead store) |
| 0x84c310 | 0x8a467c | g[0xb6129c] <- 1.0f [0x858624] / (50.0f [0x85f08c] * 50.0f [0x85f08c]) | 0.0004 | none (dead store) |
| 0x84c330 | 0x8a4680 | g[0xb612a0] <- g[0xb6129c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c350 | 0x8a4684 | g[0xb612a4] <- 0.277778f [0x858630] / 50.0f [0x85f08c] | 0.00555556 | none (dead store) |
| 0x84c370 | 0x8a4688 | g[0xb612a8] <- 0.005f [0x85f090] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c390 | 0x8a468c | g[0xb612ac] <- 1.0f [0x858624] / 50.0f [0x85f098] | 0.02 | none (dead store) |
| 0x84c3b0 | 0x8a4690 | g[0xb612b0] <- 100.0f [0x858628] / 3.6f [0x85f094] | 27.777779 | none (dead store) |
| 0x84c3d0 | 0x8a4694 | g[0xb612b4] <- 1.0f [0x858624] / (50.0f [0x85f098] * 50.0f [0x85f098]) | 0.0004 | none (dead store) |
| 0x84c3f0 | 0x8a4698 | g[0xb612b8] <- g[0xb612b4] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c410 | 0x8a469c | g[0xb612bc] <- 0.277778f [0x858630] / 50.0f [0x85f098] | 0.00555556 | none (dead store) |
| 0x84c430 | 0x8a46a0 | g[0xb612c0] <- 0.005f [0x85f0b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c450 | 0x8a46a4 | g[0xb612c4] <- 1.0f [0x858624] / 50.0f [0x85f0b8] | 0.02 | none (dead store) |
| 0x84c470 | 0x8a46a8 | g[0xb612c8] <- 100.0f [0x858628] / 3.6f [0x85f0b4] | 27.777779 | none (dead store) |
| 0x84c490 | 0x8a46ac | g[0xb612cc] <- 1.0f [0x858624] / (50.0f [0x85f0b8] * 50.0f [0x85f0b8]) | 0.0004 | none (dead store) |
| 0x84c4b0 | 0x8a46b0 | g[0xb612d0] <- g[0xb612cc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c4d0 | 0x8a46b4 | g[0xb612d4] <- 0.277778f [0x858630] / 50.0f [0x85f0b8] | 0.00555556 | none (dead store) |
| 0x84c520 | 0x8a46c0 | g[0xb612e8] <- 0.005f [0x85f0f4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c540 | 0x8a46c4 | g[0xb61300] <- 0.005f [0x85f108] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c560 | 0x8a46c8 | g[0xb61304] <- 1.0f [0x858624] / 50.0f [0x85f110] | 0.02 | none (dead store) |
| 0x84c580 | 0x8a46cc | g[0xb61308] <- 100.0f [0x858628] / 3.6f [0x85f10c] | 27.777779 | none (dead store) |
| 0x84c5a0 | 0x8a46d0 | g[0xb6130c] <- 1.0f [0x858624] / (50.0f [0x85f110] * 50.0f [0x85f110]) | 0.0004 | none (dead store) |
| 0x84c5c0 | 0x8a46d4 | g[0xb61310] <- g[0xb6130c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c5e0 | 0x8a46d8 | g[0xb61314] <- 0.277778f [0x858630] / 50.0f [0x85f110] | 0.00555556 | none (dead store) |
| 0x84c600 | 0x8a46dc | g[0xb6136c] <- 0.005f [0x85f178] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c620 | 0x8a46e0 | g[0xb61370] <- 1.0f [0x858624] / 50.0f [0x85f180] | 0.02 | none (dead store) |
| 0x84c640 | 0x8a46e4 | g[0xb61374] <- 100.0f [0x858628] / 3.6f [0x85f17c] | 27.777779 | none (dead store) |
| 0x84c660 | 0x8a46e8 | g[0xb61378] <- 1.0f [0x858624] / (50.0f [0x85f180] * 50.0f [0x85f180]) | 0.0004 | none (dead store) |
| 0x84c680 | 0x8a46ec | g[0xb6137c] <- g[0xb61378] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c6a0 | 0x8a46f0 | g[0xb61380] <- 0.277778f [0x858630] / 50.0f [0x85f180] | 0.00555556 | none (dead store) |
| 0x84c6c0 | 0x8a46f4 | g[0xb6138c] <- 0.005f [0x85f1cc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c6e0 | 0x8a46f8 | g[0xb61390] <- 1.0f [0x858624] / 50.0f [0x85f1d4] | 0.02 | none (dead store) |
| 0x84c700 | 0x8a46fc | g[0xb61394] <- 100.0f [0x858628] / 3.6f [0x85f1d0] | 27.777779 | none (dead store) |
| 0x84c720 | 0x8a4700 | g[0xb61398] <- 1.0f [0x858624] / (50.0f [0x85f1d4] * 50.0f [0x85f1d4]) | 0.0004 | none (dead store) |
| 0x84c740 | 0x8a4704 | g[0xb6139c] <- g[0xb61398] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c760 | 0x8a4708 | g[0xb613a0] <- 0.277778f [0x858630] / 50.0f [0x85f1d4] | 0.00555556 | none (dead store) |
| 0x84c780 | 0x8a470c | g[0xb613a4] <- g[0x8ae5f4] (initial .data value -17.0f [0x8ae5f4]) - g[0x8ae5f0] (initial .data value -100.0f [0x8ae5f0]) | 83.0 | none (dead store) |
| 0x84c7a0 | 0x8a4710 | g[0xb613a8] <- g[0x8ae5fc] (initial .data value 0.56f [0x8ae5fc]) - g[0x8ae5f8] (initial .data value 0.56f [0x8ae5f8]) | 0.0 | none (dead store) |
| 0x84c7c0 | 0x8a4714 | g[0xb613ac] <- g[0x8ae604] (initial .data value -15.0f [0x8ae604]) - g[0x8ae600] (initial .data value -100.0f [0x8ae600]) | 85.0 | none (dead store) |
| 0x84c800 | 0x8a471c | g[0xb613b4] <- g[0x8ae614] (initial .data value 11.0f [0x8ae614]) - g[0x8ae610] (initial .data value 2.0f [0x8ae610]) | 9.0 | none (dead store) |
| 0x84c820 | 0x8a4720 | g[0xb613b8] <- g[0x8ae61c] (initial .data value 0.71f [0x8ae61c]) - g[0x8ae618] (initial .data value 0.4f [0x8ae618]) | 0.30999997 | none (dead store) |
| 0x84c880 | 0x8a472c | g[0xb613c4] <- g[0x8ae680] (initial .data value 1.2f [0x8ae680]) - g[0x8ae67c] (initial .data value 0.8f [0x8ae67c]) | 0.40000004 | none (dead store) |
| 0x84c8a0 | 0x8a4730 | g[0xb613c8] <- g[0x8ae690] (initial .data value 1.6f [0x8ae690]) - g[0x8ae68c] (initial .data value 1.0f [0x8ae68c]) | 0.6 | none (dead store) |
| 0x84c8e0 | 0x8a4738 | g[0xb61c1c] <- 0.005f [0x85f258] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c900 | 0x8a473c | g[0xb61c20] <- 1.0f [0x858624] / 50.0f [0x85f260] | 0.02 | none (dead store) |
| 0x84c920 | 0x8a4740 | g[0xb61c24] <- 100.0f [0x858628] / 3.6f [0x85f25c] | 27.777779 | none (dead store) |
| 0x84c940 | 0x8a4744 | g[0xb61c28] <- 1.0f [0x858624] / (50.0f [0x85f260] * 50.0f [0x85f260]) | 0.0004 | none (dead store) |
| 0x84c960 | 0x8a4748 | g[0xb61c2c] <- g[0xb61c28] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84c980 | 0x8a474c | g[0xb61c30] <- 0.277778f [0x858630] / 50.0f [0x85f260] | 0.00555556 | none (dead store) |
| 0x84c9d0 | 0x8a4754 | g[0xb61ce0] <- 0.005f [0x85f350] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84c9f0 | 0x8a4758 | g[0xb61ce4] <- 1.0f [0x858624] / 50.0f [0x85f358] | 0.02 | none (dead store) |
| 0x84ca10 | 0x8a475c | g[0xb61ce8] <- 100.0f [0x858628] / 3.6f [0x85f354] | 27.777779 | none (dead store) |
| 0x84ca30 | 0x8a4760 | g[0xb61cec] <- 1.0f [0x858624] / (50.0f [0x85f358] * 50.0f [0x85f358]) | 0.0004 | none (dead store) |
| 0x84ca50 | 0x8a4764 | g[0xb61cf0] <- g[0xb61cec] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ca70 | 0x8a4768 | g[0xb61cf4] <- 0.277778f [0x858630] / 50.0f [0x85f358] | 0.00555556 | none (dead store) |
| 0x84ca90 | 0x8a476c | g[0xb61d58] <- 1.0f [0x858624] / 50.0f [0x85f360] | 0.02 | none (dead store) |
| 0x84cab0 | 0x8a4770 | g[0xb61d5c] <- 100.0f [0x858628] / 3.6f [0x85f35c] | 27.777779 | none (dead store) |
| 0x84cad0 | 0x8a4774 | g[0xb61d60] <- 1.0f [0x858624] / (50.0f [0x85f360] * 50.0f [0x85f360]) | 0.0004 | none (dead store) |
| 0x84caf0 | 0x8a4778 | g[0xb61d64] <- g[0xb61d60] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84cb10 | 0x8a477c | g[0xb61d68] <- 0.277778f [0x858630] / 50.0f [0x85f360] | 0.00555556 | none (dead store) |
| 0x84cb30 | 0x8a4780 | g[0xb61d6c] <- 0.005f [0x85f364] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cb50 | 0x8a4784 | g[0xb61d74] <- 0.005f [0x85f36c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cb70 | 0x8a4788 | g[0xb62c74] <- 1.0f [0x858624] / 50.0f [0x85f398] | 0.02 | none (dead store) |
| 0x84cb90 | 0x8a478c | g[0xb62c78] <- 100.0f [0x858628] / 3.6f [0x85f394] | 27.777779 | none (dead store) |
| 0x84cbb0 | 0x8a4790 | g[0xb62c7c] <- 1.0f [0x858624] / (50.0f [0x85f398] * 50.0f [0x85f398]) | 0.0004 | none (dead store) |
| 0x84cbd0 | 0x8a4794 | g[0xb62c80] <- g[0xb62c7c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84cbf0 | 0x8a4798 | g[0xb62c84] <- 0.277778f [0x858630] / 50.0f [0x85f398] | 0.00555556 | none (dead store) |
| 0x84cc10 | 0x8a479c | g[0xb62c88] <- 0.005f [0x85f39c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cc40 | 0x8a47a4 | g[0xb62c8c] <- 0.005f [0x85f3b8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cc60 | 0x8a47a8 | g[0xb62c90] <- 1.0f [0x858624] / 50.0f [0x85f3c0] | 0.02 | none (dead store) |
| 0x84cc80 | 0x8a47ac | g[0xb62c94] <- 100.0f [0x858628] / 3.6f [0x85f3bc] | 27.777779 | none (dead store) |
| 0x84cca0 | 0x8a47b0 | g[0xb62c98] <- 1.0f [0x858624] / (50.0f [0x85f3c0] * 50.0f [0x85f3c0]) | 0.0004 | none (dead store) |
| 0x84ccc0 | 0x8a47b4 | g[0xb62c9c] <- g[0xb62c98] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84cce0 | 0x8a47b8 | g[0xb62ca0] <- 0.277778f [0x858630] / 50.0f [0x85f3c0] | 0.00555556 | none (dead store) |
| 0x84cd00 | 0x8a47bc | g[0xb62ca8] <- 0.005f [0x85f3c8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cd40 | 0x8a47c4 | g[0xb6b96c] <- 0.005f [0x85f43c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cd70 | 0x8a47cc | g[0xb6ba08] <- 0.005f [0x860ae0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84cd90 | 0x8a47d0 | g[0xb6ba0c] <- 1.0f [0x858624] / 50.0f [0x860ae8] | 0.02 | none (dead store) |
| 0x84cdb0 | 0x8a47d4 | g[0xb6ba10] <- 100.0f [0x858628] / 3.6f [0x860ae4] | 27.777779 | none (dead store) |
| 0x84cdd0 | 0x8a47d8 | g[0xb6ba14] <- 1.0f [0x858624] / (50.0f [0x860ae8] * 50.0f [0x860ae8]) | 0.0004 | none (dead store) |
| 0x84cdf0 | 0x8a47dc | g[0xb6ba18] <- g[0xb6ba14] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ce10 | 0x8a47e0 | g[0xb6ba1c] <- 0.277778f [0x858630] / 50.0f [0x860ae8] | 0.00555556 | none (dead store) |
| 0x84ceb0 | 0x8a47f4 | g[0xb6ba30] <- g[0x8cbc64] (initial .data value 1.25f [0x8cbc64]) - g[0x8cbc60] (initial .data value 1.0f [0x8cbc60]) | 0.25 | none (dead store) |
| 0x84ced0 | 0x8a47f8 | g[0xb6ba34] <- g[0xb6b9d4] (.bss, still 0.0 here: no earlier initialiser writes it) - g[0x8cbc68] (initial .data value -2.0f [0x8cbc68]) | 2.0 | 0x84cef2 in 0x84cef0 __init_g_b6ba38 |
| 0x84cf60 | 0x8a480c | g[0xb6ba48] <- g[0x8cbc84] (initial .data value 1.25f [0x8cbc84]) - g[0x8cbc80] (initial .data value 0.925f [0x8cbc80]) | 0.325 | 0x84cfa2 in 0x84cfa0 __init_g_b6ba50 |
| 0x84cf80 | 0x8a4810 | g[0xb6ba4c] <- g[0x8cbc88] (initial .data value 2.0f [0x8cbc88]) - g[0xb6b9d8] (.bss, still 0.0 here: no earlier initialiser writes it) | 2.0 | 0x84cfd2 in 0x84cfc0 __init_g_b6ba54 |
| 0x84d010 | 0x8a4824 | g[0xb6ba60] <- g[0x8cbca0] (initial .data value -1.0f [0x8cbca0]) - g[0x8cbc9c] (initial .data value -2.0f [0x8cbc9c]) | 1.0 | none (dead store) |
| 0x84d0f0 | 0x8a4840 | g[0xb6ba7c] <- g[0x8cbf0c] (initial .data value 9.0f [0x8cbf0c]) - g[0xb6ba00] (.bss, still 0.0 here: no earlier initialiser writes it) | 9.0 | none (dead store) |
| 0x84d170 | 0x8a4850 | g[0xb6ba8c] <- g[0x8cbf78] (initial .data value -5.0f [0x8cbf78]) - g[0x8cbf74] (initial .data value -8.0f [0x8cbf74]) | 3.0 | none (dead store) |
| 0x84d1f0 | 0x8a4860 | g[0xb6ba98] <- g[0x8cbfcc] (initial .data value -3.0f [0x8cbfcc]) - g[0x8cbfc8] (initial .data value -9.0f [0x8cbfc8]) | 6.0 | none (dead store) |
| 0x84d330 | 0x8a4888 | g[0xb6badc] <- 0.005f [0x862de4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d350 | 0x8a488c | g[0xb6bae0] <- 0.005f [0x862e08] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d370 | 0x8a4890 | g[0xb6bae4] <- 1.0f [0x858624] / 50.0f [0x862e10] | 0.02 | none (dead store) |
| 0x84d390 | 0x8a4894 | g[0xb6bae8] <- 100.0f [0x858628] / 3.6f [0x862e0c] | 27.777779 | none (dead store) |
| 0x84d3b0 | 0x8a4898 | g[0xb6baec] <- 1.0f [0x858624] / (50.0f [0x862e10] * 50.0f [0x862e10]) | 0.0004 | none (dead store) |
| 0x84d3d0 | 0x8a489c | g[0xb6baf0] <- g[0xb6baec] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84d3f0 | 0x8a48a0 | g[0xb6baf4] <- 0.277778f [0x858630] / 50.0f [0x862e10] | 0.00555556 | none (dead store) |
| 0x84d410 | 0x8a48a4 | g[0xb6bb00] <- 0.005f [0x862e20] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d430 | 0x8a48a8 | g[0xb6bb04] <- 1.0f [0x858624] / 50.0f [0x862e28] | 0.02 | none (dead store) |
| 0x84d450 | 0x8a48ac | g[0xb6bb08] <- 100.0f [0x858628] / 3.6f [0x862e24] | 27.777779 | none (dead store) |
| 0x84d470 | 0x8a48b0 | g[0xb6bb0c] <- 1.0f [0x858624] / (50.0f [0x862e28] * 50.0f [0x862e28]) | 0.0004 | none (dead store) |
| 0x84d490 | 0x8a48b4 | g[0xb6bb10] <- g[0xb6bb0c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84d4b0 | 0x8a48b8 | g[0xb6bb14] <- 0.277778f [0x858630] / 50.0f [0x862e28] | 0.00555556 | none (dead store) |
| 0x84d510 | 0x8a48c4 | g[0xb6bc78] <- 0.005f [0x862e48] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d530 | 0x8a48c8 | g[0xb6bc7c] <- 1.0f [0x858624] / 50.0f [0x862e50] | 0.02 | none (dead store) |
| 0x84d550 | 0x8a48cc | g[0xb6bc80] <- 100.0f [0x858628] / 3.6f [0x862e4c] | 27.777779 | none (dead store) |
| 0x84d570 | 0x8a48d0 | g[0xb6bc84] <- 1.0f [0x858624] / (50.0f [0x862e50] * 50.0f [0x862e50]) | 0.0004 | none (dead store) |
| 0x84d590 | 0x8a48d4 | g[0xb6bc88] <- g[0xb6bc84] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84d5b0 | 0x8a48d8 | g[0xb6bc8c] <- 0.277778f [0x858630] / 50.0f [0x862e50] | 0.00555556 | none (dead store) |
| 0x850200 | 0x8a4e58 | g[0xbd00e0] <- 0.005f [0x868b34] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x850220 | 0x8a4e5c | g[0xbd00e4] <- 1.0f [0x858624] / 50.0f [0x868b3c] | 0.02 | none (dead store) |
| 0x850240 | 0x8a4e60 | g[0xbd00e8] <- 100.0f [0x858628] / 3.6f [0x868b38] | 27.777779 | none (dead store) |
| 0x850260 | 0x8a4e64 | g[0xbd00ec] <- 1.0f [0x858624] / (50.0f [0x868b3c] * 50.0f [0x868b3c]) | 0.0004 | none (dead store) |
| 0x850280 | 0x8a4e68 | g[0xbd00f0] <- g[0xbd00ec] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x8502a0 | 0x8a4e6c | g[0xbd00f4] <- 0.277778f [0x858630] / 50.0f [0x868b3c] | 0.00555556 | none (dead store) |
| 0x851600 | 0x8a50d0 | g[0xc0b1d8] <- 100.0f [0x858628] / 3.6f [0x86cf0c] | 27.777779 | 0x4e794d in 0x4e7940 CAEMFDecoder::GetStreamPlayTimeMs |

## Other initialisers and destructors

## 0x84c1a0 __init_84c1a0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x8562e0) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (AUDIO).

## 0x84c4f0 __init_84c4f0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856310) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (AUDIO).

## 0x84cc30 __init_84cc30  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856350) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (AUDIO).

## 0x84cd60 __init_84cd60  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856370) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (AUDIO).

## 0x84d190 __init_g_8cc0b8  (catalogue: unknown/REA, port level 0)
- dword [0x8CC0B8] = dword [0x8CBF74]; dword [0x8CC0C0] = dword [0x8CBF78] (copies two initialised .data dwords; .data, AUDIO initialiser run). Both destinations are in .data, so this overwrites their link-time values at CRT init.

## 0x8562e0 __dtor_84c1a0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84c1a0).

## 0x856310 __dtor_84c4f0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84c4f0).

## 0x856350 __dtor_84cc30  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84cc30).

## 0x856370 __dtor_84cd60  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84cd60).
