# CORE: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84d930 | 0x8a4948 | g[0xb714f0] <- 0.005f [0x863860] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d950 | 0x8a494c | g[0xb714f4] <- 1.0f [0x858624] / 50.0f [0x863868] | 0.02 | none (dead store) |
| 0x84d970 | 0x8a4950 | g[0xb714f8] <- 100.0f [0x858628] / 3.6f [0x863864] | 27.777779 | none (dead store) |
| 0x84d990 | 0x8a4954 | g[0xb714fc] <- 1.0f [0x858624] / (50.0f [0x863868] * 50.0f [0x863868]) | 0.0004 | none (dead store) |
| 0x84d9b0 | 0x8a4958 | g[0xb71500] <- g[0xb714fc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | 0x155e135 in no catalogue function |
| 0x84d9d0 | 0x8a495c | g[0xb71504] <- 0.277778f [0x858630] / 50.0f [0x863868] | 0.00555556 | none (dead store) |
| 0x84dcf0 | 0x8a49c0 | g[0xb71a48] <- 0.005f [0x8639a4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84dd10 | 0x8a49c4 | g[0xb71a4c] <- 1.0f [0x858624] / 50.0f [0x8639ac] | 0.02 | none (dead store) |
| 0x84dd30 | 0x8a49c8 | g[0xb71a50] <- 100.0f [0x858628] / 3.6f [0x8639a8] | 27.777779 | none (dead store) |
| 0x84dd50 | 0x8a49cc | g[0xb71a54] <- 1.0f [0x858624] / (50.0f [0x8639ac] * 50.0f [0x8639ac]) | 0.0004 | none (dead store) |
| 0x84dd70 | 0x8a49d0 | g[0xb71a58] <- g[0xb71a54] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84dd90 | 0x8a49d4 | g[0xb71a5c] <- 0.277778f [0x858630] / 50.0f [0x8639ac] | 0.00555556 | none (dead store) |
| 0x84ddb0 | 0x8a49d8 | g[0xb71f68] <- 0.005f [0x863a30] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ddd0 | 0x8a49dc | g[0xb71f6c] <- 1.0f [0x858624] / 50.0f [0x863a38] | 0.02 | none (dead store) |
| 0x84ddf0 | 0x8a49e0 | g[0xb71f70] <- 100.0f [0x858628] / 3.6f [0x863a34] | 27.777779 | none (dead store) |
| 0x84de10 | 0x8a49e4 | g[0xb71f74] <- 1.0f [0x858624] / (50.0f [0x863a38] * 50.0f [0x863a38]) | 0.0004 | none (dead store) |
| 0x84de30 | 0x8a49e8 | g[0xb71f78] <- g[0xb71f74] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84de50 | 0x8a49ec | g[0xb71f7c] <- 0.277778f [0x858630] / 50.0f [0x863a38] | 0.00555556 | none (dead store) |
| 0x84df50 | 0x8a4a0c | g[0xb72960] <- 0.005f [0x863ac8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84df70 | 0x8a4a10 | g[0xb72964] <- 1.0f [0x858624] / 50.0f [0x863ad0] | 0.02 | none (dead store) |
| 0x84df90 | 0x8a4a14 | g[0xb72968] <- 100.0f [0x858628] / 3.6f [0x863acc] | 27.777779 | none (dead store) |
| 0x84dfb0 | 0x8a4a18 | g[0xb7296c] <- 1.0f [0x858624] / (50.0f [0x863ad0] * 50.0f [0x863ad0]) | 0.0004 | none (dead store) |
| 0x84dfd0 | 0x8a4a1c | g[0xb72970] <- g[0xb7296c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84dff0 | 0x8a4a20 | g[0xb72974] <- 0.277778f [0x858630] / 50.0f [0x863ad0] | 0.00555556 | none (dead store) |
| 0x84e030 | 0x8a4a28 | g[0xb72c88] <- 0.005f [0x863adc] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e050 | 0x8a4a2c | g[0xb72c8c] <- 1.0f [0x858624] / 50.0f [0x863ae4] | 0.02 | none (dead store) |
| 0x84e070 | 0x8a4a30 | g[0xb72c90] <- 100.0f [0x858628] / 3.6f [0x863ae0] | 27.777779 | none (dead store) |
| 0x84e090 | 0x8a4a34 | g[0xb72c94] <- 1.0f [0x858624] / (50.0f [0x863ae4] * 50.0f [0x863ae4]) | 0.0004 | none (dead store) |
| 0x84e0b0 | 0x8a4a38 | g[0xb72c98] <- g[0xb72c94] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e0d0 | 0x8a4a3c | g[0xb72c9c] <- 0.277778f [0x858630] / 50.0f [0x863ae4] | 0.00555556 | none (dead store) |
| 0x84e220 | 0x8a4a68 | g[0xb7421c] <- 0.005f [0x863b4c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e240 | 0x8a4a6c | g[0xb74220] <- 1.0f [0x858624] / 50.0f [0x863b54] | 0.02 | none (dead store) |
| 0x84e260 | 0x8a4a70 | g[0xb74224] <- 100.0f [0x858628] / 3.6f [0x863b50] | 27.777779 | none (dead store) |
| 0x84e280 | 0x8a4a74 | g[0xb74228] <- 1.0f [0x858624] / (50.0f [0x863b54] * 50.0f [0x863b54]) | 0.0004 | none (dead store) |
| 0x84e2a0 | 0x8a4a78 | g[0xb7422c] <- g[0xb74228] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e2c0 | 0x8a4a7c | g[0xb74230] <- 0.277778f [0x858630] / 50.0f [0x863b54] | 0.00555556 | none (dead store) |
| 0x8502c0 | 0x8a4e70 | g[0xbd00fc] <- 0.005f [0x86a410] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8502e0 | 0x8a4e74 | g[0xbd0100] <- 1.0f [0x858624] / 50.0f [0x86a418] | 0.02 | none (dead store) |
| 0x850300 | 0x8a4e78 | g[0xbd0104] <- 100.0f [0x858628] / 3.6f [0x86a414] | 27.777779 | none (dead store) |
| 0x850320 | 0x8a4e7c | g[0xbd0108] <- 1.0f [0x858624] / (50.0f [0x86a418] * 50.0f [0x86a418]) | 0.0004 | none (dead store) |
| 0x850340 | 0x8a4e80 | g[0xbd010c] <- g[0xbd0108] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x850360 | 0x8a4e84 | g[0xbd0110] <- 0.277778f [0x858630] / 50.0f [0x86a418] | 0.00555556 | none (dead store) |
| 0x851c30 | 0x8a5194 | g[0xc1701c] <- 0.005f [0x86d2d8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x851c50 | 0x8a5198 | g[0xc17020] <- 1.0f [0x858624] / 50.0f [0x86d2e0] | 0.02 | none (dead store) |
| 0x851c70 | 0x8a519c | g[0xc17024] <- 100.0f [0x858628] / 3.6f [0x86d2dc] | 27.777779 | none (dead store) |
| 0x851c90 | 0x8a51a0 | g[0xc17028] <- 1.0f [0x858624] / (50.0f [0x86d2e0] * 50.0f [0x86d2e0]) | 0.0004 | none (dead store) |
| 0x851cb0 | 0x8a51a4 | g[0xc1702c] <- g[0xc17028] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x851cd0 | 0x8a51a8 | g[0xc17030] <- 0.277778f [0x858630] / 50.0f [0x86d2e0] | 0.00555556 | none (dead store) |
| 0x855630 | 0x8a58d4 | g[0xc87b0c] <- 0.005f [0x872aa4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855e40 | 0x8a59d8 | g[0xc8cdd0] <- 0.005f [0x872ea4] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x855e60 | 0x8a59dc | g[0xc8cdd4] <- 1.0f [0x858624] / 50.0f [0x872eac] | 0.02 | none (dead store) |
| 0x855e80 | 0x8a59e0 | g[0xc8cdd8] <- 100.0f [0x858628] / 3.6f [0x872ea8] | 27.777779 | none (dead store) |
| 0x855ea0 | 0x8a59e4 | g[0xc8cdcc] <- 1.0f [0x858624] / (50.0f [0x872eac] * 50.0f [0x872eac]) | 0.0004 | none (dead store) |
| 0x855ec0 | 0x8a59e8 | g[0xc8cde0] <- g[0xc8cdcc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x855ee0 | 0x8a59ec | g[0xc8cddc] <- 0.277778f [0x858630] / 50.0f [0x872eac] | 0.00555556 | none (dead store) |

## Other initialisers and destructors

## 0x84e010 __init_g_LoadMonitor  (catalogue: unknown/REA, port level 1)
- ECX = 0xB72978 (g_LoadMonitor); call 0x53CFA0 CLoadMonitor::Constructor; _atexit 0x821D1E(0x856430 CLoadMonitor::Destructor).

## 0x855670 __init_g_Heaps  (catalogue: unknown/REA, port level 0)
- ECX = 0xC87B40 (g_Heaps[0], gr StaticRef<std::array<CMemoryHeap,3>>); tail jump to 0x72E860 CMemoryHeap::CMemoryHeap (`mov eax, ecx; mov dword [eax], 0; ret`). No atexit. Catalogue note: 0x72E860 is not a catalogue entry and is folded into this extent.

## 0x855680 __init_g_c87b88  (catalogue: unknown/REA, port level 0)
- ECX = 0xC87B88 (g_Heaps[1]); tail jump to 0x72E860 CMemoryHeap::CMemoryHeap ([this] = 0).
