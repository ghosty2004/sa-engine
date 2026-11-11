# HUD: static initialisers and atexit destructors without gta-reversed source (RE-SPEC-UNK, lead)

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
| 0x84e110 | 0x8a4a48 | g[0xb72cac] <- 0.005f [0x863b38] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ebe0 | 0x8a4ba0 | g[0xba176c] <- 0.005f [0x865044] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ec00 | 0x8a4ba4 | g[0xba1770] <- 1.0f [0x858624] / 50.0f [0x86504c] | 0.02 | none (dead store) |
| 0x84ec20 | 0x8a4ba8 | g[0xba1774] <- 100.0f [0x858628] / 3.6f [0x865048] | 27.777779 | none (dead store) |
| 0x84ec40 | 0x8a4bac | g[0xba1778] <- 1.0f [0x858624] / (50.0f [0x86504c] * 50.0f [0x86504c]) | 0.0004 | none (dead store) |
| 0x84ec60 | 0x8a4bb0 | g[0xba177c] <- g[0xba1778] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ec80 | 0x8a4bb4 | g[0xba1780] <- 0.277778f [0x858630] / 50.0f [0x86504c] | 0.00555556 | none (dead store) |
| 0x84ed60 | 0x8a4bd0 | g[0xba671c] <- 0.005f [0x86505c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84eda0 | 0x8a4bd8 | g[0xba6730] <- 1.0f [0x858624] / 50.0f [0x8651b0] | 0.02 | none (dead store) |
| 0x84edc0 | 0x8a4bdc | g[0xba6734] <- 100.0f [0x858628] / 3.6f [0x8651ac] | 27.777779 | none (dead store) |
| 0x84ede0 | 0x8a4be0 | g[0xba6738] <- 1.0f [0x858624] / (50.0f [0x8651b0] * 50.0f [0x8651b0]) | 0.0004 | none (dead store) |
| 0x84ee00 | 0x8a4be4 | g[0xba673c] <- g[0xba6738] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ee20 | 0x8a4be8 | g[0xba6740] <- 0.277778f [0x858630] / 50.0f [0x8651b0] | 0.00555556 | none (dead store) |
| 0x84ee60 | 0x8a4bf0 | g[0xba82c0] <- 0.005f [0x86558c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ee80 | 0x8a4bf4 | g[0xba82c4] <- 1.0f [0x858624] / 50.0f [0x865594] | 0.02 | none (dead store) |
| 0x84eea0 | 0x8a4bf8 | g[0xba82c8] <- 100.0f [0x858628] / 3.6f [0x865590] | 27.777779 | none (dead store) |
| 0x84eec0 | 0x8a4bfc | g[0xba82cc] <- 1.0f [0x858624] / (50.0f [0x865594] * 50.0f [0x865594]) | 0.0004 | none (dead store) |
| 0x84eee0 | 0x8a4c00 | g[0xba82d0] <- g[0xba82cc] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84ef00 | 0x8a4c04 | g[0xba82d4] <- 0.277778f [0x858630] / 50.0f [0x865594] | 0.00555556 | none (dead store) |
| 0x84ef20 | 0x8a4c08 | g[0xba82e4] <- 0.005f [0x8665b0] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84ef40 | 0x8a4c0c | g[0xba82e8] <- 1.0f [0x858624] / 50.0f [0x8665b8] | 0.02 | none (dead store) |
| 0x84ef60 | 0x8a4c10 | g[0xba82ec] <- 100.0f [0x858628] / 3.6f [0x8665b4] | 27.777779 | none (dead store) |
| 0x84ef80 | 0x8a4c14 | g[0xba82f0] <- 1.0f [0x858624] / (50.0f [0x8665b8] * 50.0f [0x8665b8]) | 0.0004 | none (dead store) |
| 0x84efa0 | 0x8a4c18 | g[0xba82f4] <- g[0xba82f0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84efc0 | 0x8a4c1c | g[0xba82f8] <- 0.277778f [0x858630] / 50.0f [0x8665b8] | 0.00555556 | none (dead store) |
| 0x84efe0 | 0x8a4c20 | g[0xba86bc] <- 0.005f [0x86695c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f000 | 0x8a4c24 | g[0xba86c0] <- 1.0f [0x858624] / 50.0f [0x866964] | 0.02 | none (dead store) |
| 0x84f020 | 0x8a4c28 | g[0xba86c4] <- 100.0f [0x858628] / 3.6f [0x866960] | 27.777779 | none (dead store) |
| 0x84f040 | 0x8a4c2c | g[0xba86c8] <- 1.0f [0x858624] / (50.0f [0x866964] * 50.0f [0x866964]) | 0.0004 | none (dead store) |
| 0x84f060 | 0x8a4c30 | g[0xba86cc] <- g[0xba86c8] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f080 | 0x8a4c34 | g[0xba86d0] <- 0.277778f [0x858630] / 50.0f [0x866964] | 0.00555556 | none (dead store) |
| 0x84f100 | 0x8a4c40 | g[0xbab1e4] <- 0.005f [0x866ba8] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x84f120 | 0x8a4c44 | g[0xbab1e8] <- 1.0f [0x858624] / 50.0f [0x866bb0] | 0.02 | none (dead store) |
| 0x84f140 | 0x8a4c48 | g[0xbab1ec] <- 100.0f [0x858628] / 3.6f [0x866bac] | 27.777779 | none (dead store) |
| 0x84f160 | 0x8a4c4c | g[0xbab1f0] <- 1.0f [0x858624] / (50.0f [0x866bb0] * 50.0f [0x866bb0]) | 0.0004 | none (dead store) |
| 0x84f180 | 0x8a4c50 | g[0xbab1f4] <- g[0xbab1f0] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x84f1a0 | 0x8a4c54 | g[0xbab1f8] <- 0.277778f [0x858630] / 50.0f [0x866bb0] | 0.00555556 | none (dead store) |
| 0x8532a0 | 0x8a5470 | g[0xc1a558] <- 0.005f [0x870e24] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x8532c0 | 0x8a5474 | g[0xc1a55c] <- 1.0f [0x858624] / 50.0f [0x870e2c] | 0.02 | none (dead store) |
| 0x8532e0 | 0x8a5478 | g[0xc1a560] <- 100.0f [0x858628] / 3.6f [0x870e28] | 27.777779 | none (dead store) |
| 0x853300 | 0x8a547c | g[0xc1a564] <- 1.0f [0x858624] / (50.0f [0x870e2c] * 50.0f [0x870e2c]) | 0.0004 | none (dead store) |
| 0x853320 | 0x8a5480 | g[0xc1a568] <- g[0xc1a564] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x853340 | 0x8a5484 | g[0xc1a56c] <- 0.277778f [0x858630] / 50.0f [0x870e2c] | 0.00555556 | none (dead store) |
| 0x853380 | 0x8a548c | g[0xc1aeb4] <- 0.005f [0x870f48] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854c90 | 0x8a57a8 | g[0xc6e990] <- 0.005f [0x872884] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |
| 0x854cb0 | 0x8a57ac | g[0xc6e994] <- 1.0f [0x858624] / 50.0f [0x87288c] | 0.02 | none (dead store) |
| 0x854cd0 | 0x8a57b0 | g[0xc6e998] <- 100.0f [0x858628] / 3.6f [0x872888] | 27.777779 | none (dead store) |
| 0x854cf0 | 0x8a57b4 | g[0xc6e99c] <- 1.0f [0x858624] / (50.0f [0x87288c] * 50.0f [0x87288c]) | 0.0004 | none (dead store) |
| 0x854d10 | 0x8a57b8 | g[0xc6e9a0] <- g[0xc6e99c] (= 0.0004, set by an earlier initialiser) * 10.0f [0x85862c] | 0.0039999997 | none (dead store) |
| 0x854d30 | 0x8a57bc | g[0xc6e9a4] <- 0.277778f [0x858630] / 50.0f [0x87288c] | 0.00555556 | none (dead store) |
| 0x854d80 | 0x8a57c4 | g[0xc71a58] <- 0.005f [0x87289c] * 0.0625f [0x858620] | 0.0003125 | none (dead store) |

## Other initialisers and destructors

## 0x853460 __init_TheText  (catalogue: unknown/REA, port level 1)
- ECX = 0xC1B340 (TheText); call 0x6A00F0 CText::Constructor; _atexit 0x821D1E(0x856A00).

## 0x854da0 __init_854da0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856b80) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (HUD).

## 0x854db0 __init_854db0  (catalogue: unknown/REA, port level 0)
- _atexit 0x821D1E(0x856b90) only: the object has a trivial (inlined) constructor but a user-declared destructor whose body is empty. Subsystem from the next named initialiser in the __xc table (HUD).

## 0x856a00 __dtor_TheText  (catalogue: unknown/REA, port level 3)
- ECX = 0xC1B340; tail jump to 0x6A0140 CText::Destructor.

## 0x856b80 __dtor_854da0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x854da0).

## 0x856b90 __dtor_854db0  (catalogue: unknown/REA, port level 0)
- Empty destructor: single `ret` (registered by 0x854db0).
