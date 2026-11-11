# RwImage module and OS file interface (RW core) - RE-SPEC-RW writer C

All `__cdecl`. IMAGEMODULE globals: base = `RwEngineInstance [0xc97b24] + [0xc9a65c]` (derived offset 0x19c, size 0x220):
+0x000 RwFreeList* image freelist; +0x004 char* image path buffer; +0x008 i32 path buffer size; +0x00c u8 gammaTable[256];
+0x10c u8 invGammaTable[256]; +0x20c f32 gammaValue; +0x210 image-format list buffer, +0x214 its capacity, +0x218
RwFreeList* (0x34-byte entries, image formats), +0x21c count (all from 0x802280 / 0x803fe0). Open count [0xc9a660].

## 0x802280 cb_IMAGEMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Open callback `(instance, offset, size)`; returns instance or NULL.
- Steps: `[0xc9a65c] = offset`;
  1. module+0 = `_rwFreeListCreateInternal([0x8e2670] (image registry size, 0x1c), [0x8e2688] (0x80), 4, [0x8e268c] (1),
     0xc9a664, 0x40018)` (via thunk to 0x8019b0); NULL -> return NULL.
  2. module+0x218 = `_rwFreeListCreateInternal(0x34, [0x8e2688], 4, [0x8e2694] (1), 0xc9a638, 0x40406)`; NULL -> destroy
     the first freelist, clear, return NULL.
  3. module+8 = 0x100; module+4 = `RwGlobals.malloc(0x100, 0x1040406)` (+0x134); NULL -> destroy both freelists (second
     first), clear, return NULL. `path[0] = 0`. `[0xc9a660]++`. `RwImageSetGamma(1.0f)`.
  4. module+0x21c = 0; module+0x214 = 0x100; module+0x210 = `malloc(0x100, 0x1040018)`; NULL -> `free(path)` (+0x138),
     path = NULL, size = 0, destroy both freelists, return NULL (the open count stays incremented - original quirk).

## 0x803fe0 RwImageSetGamma  (catalogue: specified/REA, port level 0)
- `RwBool (RwReal gamma)`; returns 1. ebp frame, stack aligned to 8.
- `module.gammaValue = gamma`; `inv = 1.0f [0x858624] / gamma` (stored f32); `gammaTable[0] = invGammaTable[0] = 0`.
- For i = 1..255 (signed loop, 0x80410b): `x = (f32)((float)i * 0.003921569f [0x859a3c])`;
  `gammaTable[i] = (u8)trunc(pow((double)x, (double)inv) * 255.0f [0x859aac] + 0.5f [0x858b8c])`;
  `invGammaTable[i] = (u8)trunc(pow((double)x, (double)gamma) * 255.0f + 0.5f)`. `pow` is the CRT routine at 0x8220f0
  (double args on the stack, result in ST0); the products are rounded to f32 before the conversion (`fstp dword`), and
  the conversion uses a temporary control word with RC = truncate (`or ah, 0xc`, `fistp`), restoring the old word.
  NOTE: the forward table uses 1/gamma and the inverse table uses gamma.
- Parity: pow runs under the game's 24-bit x87 precision control; the Rust port must use `sa_math::x87`-exact pow
  or a table captured from the oracle.

## 0x804130 RwOsGetFileInterface  (catalogue: specified/REA, port level 0)
- `RwFileFunctions* ()`: returns `RwEngineInstance + 0xc4`.

## 0x804140 _rwFileSystemOpen  (catalogue: specified/REA, port level 0)
- `RwBool ()`: fills `RwGlobals.fileFuncs` (+0xc4..+0xec, RW 3.6 `RwFileFunctions` order): +0xc4 rwfexist = 0x804200
  (`_rwFileExists`), +0xc8 rwfopen = 0x8232d8, +0xcc rwfclose = 0x82318b, +0xd0 rwfread = 0x823521, +0xd4 rwfwrite =
  0x823674, +0xd8 rwfgets = 0x823798, +0xdc rwfputs = 0x8262b8, +0xe0 rwfeof = 0x8262a2, +0xe4 rwfseek = 0x82374f,
  +0xe8 rwfflush = 0x823e86, +0xec rwftell = 0x826261 (CRT functions in game `.text`; slot roles from the RW header order,
  the CRT identities themselves are not re-verified here). Returns 1.
- This confirms the 0x2c-byte `fileFuncs` block that gta-reversed's RwGlobals lacks (README §6).
