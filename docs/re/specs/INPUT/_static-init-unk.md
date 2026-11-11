# INPUT: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84d7a0 | 0x8a4914 | g[0xb70160] <- 0.005f [0x863284] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d7c0 | 0x8a4918 | g[0xb70164] <- 1.0f [0x858624] / 50.0f [0x86328c] | 0.02 | none (dead store) |
| 0x84d7e0 | 0x8a491c | g[0xb70168] <- 100.0f [0x858628] / 3.6f [0x863288] | 27.777779 | none (dead store) |
| 0x84d800 | 0x8a4920 | g[0xb7016c] <- 1.0f [0x858624] / (50.0f [0x86328c] * 50.0f [0x86328c]) | 0.0004 | none (dead store) |
| 0x84d820 | 0x8a4924 | g[0xb70170] <- g[0xb7016c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84d840 | 0x8a4928 | g[0xb70174] <- 0.277778f [0x858630] / 50.0f [0x86328c] | 0.00555556 | none (dead store) |
| 0x84d860 | 0x8a492c | g[0xb7017c] <- 0.005f [0x863290] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84d880 | 0x8a4930 | g[0xb70180] <- 1.0f [0x858624] / 50.0f [0x863298] | 0.02 | none (dead store) |
| 0x84d8a0 | 0x8a4934 | g[0xb70184] <- 100.0f [0x858628] / 3.6f [0x863294] | 27.777779 | none (dead store) |
| 0x84d8c0 | 0x8a4938 | g[0xb70188] <- 1.0f [0x858624] / (50.0f [0x863298] * 50.0f [0x863298]) | 0.0004 | none (dead store) |
| 0x84d8e0 | 0x8a493c | g[0xb7018c] <- g[0xb70188] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84d900 | 0x8a4940 | g[0xb70190] <- 0.277778f [0x858630] / 50.0f [0x863298] | 0.00555556 | none (dead store) |
| 0x84e130 | 0x8a4a4c | g[0xb73440] <- 0.005f [0x863b3c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84e150 | 0x8a4a50 | g[0xb73444] <- 1.0f [0x858624] / 50.0f [0x863b44] | 0.02 | none (dead store) |
| 0x84e170 | 0x8a4a54 | g[0xb73448] <- 100.0f [0x858628] / 3.6f [0x863b40] | 27.777779 | none (dead store) |
| 0x84e190 | 0x8a4a58 | g[0xb7344c] <- 1.0f [0x858624] / (50.0f [0x863b44] * 50.0f [0x863b44]) | 0.0004 | none (dead store) |
| 0x84e1b0 | 0x8a4a5c | g[0xb73450] <- g[0xb7344c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84e1d0 | 0x8a4a60 | g[0xb73454] <- 0.277778f [0x858630] / 50.0f [0x863b44] | 0.00555556 | none (dead store) |

## Other initialisers and destructors

## 0x84e0f0 __init_84e0f0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856440) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (INPUT).

## 0x84e100 __init_84e100  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856450) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (INPUT).

## 0x855fd0 __init_AllValidWinJoys  (catalogue: unknown/REA, port level 0)
- EAX = 0xC92148, ECX = 0; loop (EAX < 0xC92168, signed `jl`, 2 iterations: 0xC92148, 0xC92158): dword [EAX-4] = 0; bytes [EAX], [EAX+1], [EAX+2] = 0; EAX += 0x10. Constructs 2 joystick entries of 0x10 bytes starting at 0xC92144 (first dword and three bytes at +4..+6 zeroed).

## 0x856440 __dtor_84e0f0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84e0f0).

## 0x856450 __dtor_84e100  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x84e100).
