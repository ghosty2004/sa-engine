# CFileLoader (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x5B3680 LoadingScreenLoadingFile  (catalogue: specified/REA, port level 16)
Free function from FileLoader.cpp (gta-reversed FileLoader.cpp:2461), placed here with its file.
- Convention: `__cdecl void (const char* path)`; path at [esp+4]; plain `ret`. Preserves ESI.
- Callers: 1 function (CFileLoader::LoadLevel 0x5B9030, 9 call sites).
- Callees: `_strrchr` 0x821490 (CRT), `sprintf` 0x821BB5 (CRT, unbounded), `orig!(0x53DED0 LoadingScreen)`
  `(const char* msg1, const char* msg2, const char* msg3)` (cdecl).
- Globals: W `gString` 0xB71670 (char[352] per gr common.h:63).
- Behaviour:
  1. `p = strrchr(path, '\\'); name = p ? p + 1 : path;` (the inlined GetFilename 0x5B3660)
  2. `sprintf(gString, "Loading %s" /*0x868B40*/, name);`
  3. `LoadingScreen("Loading the Game" /*0x863A6C*/, gString, NULL);`
- Edge cases: plain `sprintf` - a name longer than 343 chars overflows gString (gr uses `sprintf_s`, which
  would instead invoke the invalid-parameter handler; port keeps the unbounded write semantics or at least
  the same result for in-range input).
- gta-reversed: FileLoader.cpp:2462-2466, same apart from sprintf_s.
- Open questions: none.

## 0x5B7670 CFileLoader::Load2dEffect  (catalogue: specified/REA, port level 2)
Parses one line of the `2dfx` section of an IDE file and appends a C2dEffect to the global 2dfx store.
gta-reversed: `RH_ScopedInstall(Load2dEffect, 0x5B7670, { .Reversed = false })`, body is `plugin::Call` -
everything below is from the bytes.

- Convention: `__cdecl void Load2dEffect(const char* line)`; line at [esp+4]; stack frame 0x1CC bytes of
  locals; saves EBX (= line cursor), ESI (= effect), EDI. Every exit is a tail `jmp 0x7316B0
  CTxdStore::PopCurrentTxd` after restoring registers, so EAX on return is whatever PopCurrentTxd returns
  (the function is void; the only caller ignores it).
- Callers: 1 (CFileLoader::LoadObjectTypes 0x5B8400).
- Callees (via `orig!`): `sscanf` 0x8220AD (CRT, 11 call sites); CTxdStore::PushCurrentTxd 0x7316A0;
  CTxdStore::FindTxdSlot(const char*) 0x731850; CTxdStore::SetCurrentTxd(int) 0x7319C0;
  CModelInfo::Get2dEffectStore 0x4C5A60; CBaseModelInfo::Add2dEffect 0x4C4D20 (thiscall, 1 arg, `ret 4`);
  CMemoryMgr::Malloc 0x72F420 `(size, hint)`; RwTextureRead 0x7F3AC0 `(name, maskName)`; `_ftol` 0x821B40
  (x87 ST0 -> EDX:EAX, truncation toward zero; pops ST0); `strncpy` 0x821F40; CTxdStore::PopCurrentTxd 0x7316B0.
- Globals: R `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8; R/W `CModelInfo::ms_2dFXInfoStore` 0xB4C2D8 (through the
  returned pointer: +0x0 i32 count, +0x4 C2dEffect[], 0x40 bytes each).
- C2dEffect layout (gta-reversed Plugins/TwoDEffectPlugin/2dEffect.h, size 0x40): +0x0 CVector m_Pos,
  +0xC u8 m_Type, +0x10 type-specific union. Per-type field names are given in the cases below.
- Locals referred to below as `L+off` = `[esp+off]` measured right after the prologue `push edi`
  (i.e. `L+0x1D8` is the return address). Header locals: `L+0x1C` model id (i32), `L+0x14` x, `L+0x10` y,
  `L+0x18` z (f32), `L+0xC` type (i32).

### Common prologue (always executed, in this order)
1. `sscanf(line, "%d %f %f %f %d" /*0x869A30*/, &L+0x1C, &L+0x14, &L+0x10, &L+0x18, &L+0xC);`
   (return value ignored; fields not matched keep their uninitialised stack contents).
2. `CTxdStore::PushCurrentTxd(); CTxdStore::SetCurrentTxd(CTxdStore::FindTxdSlot("particle" /*0x85C648*/));`
3. `mi = ms_modelInfoPtrs[L+0x1C]` (no bounds/null check).
4. `store = CModelInfo::Get2dEffectStore(); effect = &store->objects[store->count]` (address
   `store + 4 + count*0x40`); `store->count = count + 1` (written before step 5; no capacity check).
5. `mi->Add2dEffect(effect)` - called BEFORE any effect field is written.
6. `effect->m_Pos.x = L+0x14; .y = L+0x10; .z = L+0x18;` (raw 32-bit copies) `effect->m_Type = (u8)L+0xC`.
7. Switch on the full 32-bit `L+0xC` as unsigned: values > 10 (including negatives) and 2, 4 go to the
   default exit (nothing else written). Jump table at 0x5B8004.
In every case, fields of the effect that the case does not write keep whatever the store slot held before
(zero on first use; stale data if the store count was reset by CModelInfo::ReInit2dEffects).
Later per-case `sscanf` calls re-parse the header into the same locals; those re-parsed header values are
NOT copied to the effect again (position/type were stored in step 6 from the first parse).

### type 0 EFFECT_LIGHT (0x5B78C2)
1. `sscanf(line, "%d %f %f %f %d %d %d %d %d" /*0x8699E4*/, &L+0x1C, &L+0x14, &L+0x10, &L+0x18, &L+0xC,
   &r=L+0x20, &g=L+0x28, &b=L+0x24, &a=L+0x30);`
2. Quoted names, scanning `line` with a byte cursor `p` (starts at `line`):
   - advance until a `"` has been consumed (`do c = *p++; while (c != '"')`);
   - copy bytes into local buffer `coronaTex` (L+0x88) until the next `"` (not copied), then NUL-terminate;
     `p` now points at that closing quote; `p++`;
   - advance until another `"` has been consumed; copy into `shadowTex` (L+0xEC) up to the next `"`,
     NUL-terminate; `p` points at the closing quote.
   No length limits (stack overflow on long names) and no end-of-string check (a line without 4 quotes
   reads past its terminator) - original behaviour.
3. `sscanf(p + 1, "%f %f %f %f %d %d %d %d %d %d %d %d %d" /*0x8699BC*/, &effect+0x14, &effect+0x18,
   &effect+0x1C, &effect+0x20, &d5=L+0x2C, &d6=L+0x34, &d7=L+0x48, &d8=L+0x40, &d9=L+0x38, &d10=L+0x4C,
   &d11=L+0x3C, &d12=L+0x50, &d13=L+0x44);` - the four floats go straight into the effect:
   +0x14 m_fCoronaFarClip, +0x18 m_fPointlightRange, +0x1C m_fCoronaSize, +0x20 m_fShadowSize.
4. Stores (each the low byte / low word of the int local):
   +0x10 = r, +0x11 = g, +0x12 = b, +0x13 = a (m_color);
   +0x29 = d5 (m_nShadowColorMultiplier), +0x26 = d6 (m_nCoronaFlashType), +0x27 = d7 (m_bCoronaEnableReflection),
   +0x28 = d8 (m_nCoronaFlareType), u16 +0x24 = d9 (m_nFlags), +0x2A = d10 (m_nShadowZDistance),
   +0x2B = d11 (offsetX), +0x2C = d12 (offsetY), +0x2D = d13 (offsetZ).
5. `effect+0x30 (m_pCoronaTex) = RwTextureRead(coronaTex, NULL);` then
   `effect+0x34 (m_pShadowTex) = RwTextureRead(shadowTex, NULL);` (current TXD = "particle" slot).
6. If `(u8)d9 & 4`: `byte[effect+0x24] &= 0xFD` (clear flag bit 1 when bit 2 is set; tested on the local
   d9, applied to the stored flags).
7. Exit.

### type 1 EFFECT_PARTICLE (0x5B7A40)
`sscanf(line, "%d %f %f %f %d %s" /*0x8699A8*/, &L+0x1C, &L+0x14, &L+0x10, &L+0x18, &L+0xC, effect+0x10);`
- the name is scanned directly into `m_szName` (+0x10, char[24]) without a width limit (a long token runs
  into the following effect slot). Exit.

### type 3 EFFECT_ATTRACTOR (0x5B7A79)
1. `sscanf(line, "%d %f %f %f %d %d %f %f %f %f %f %f %f %f %f %d %d %s" /*0x869970*/, &L+0x1C, &L+0x14,
   &L+0x10, &L+0x18, &L+0xC, &atype=L+0x20, effect+0x10, +0x14, +0x18, +0x1C, +0x20, +0x24, +0x28, +0x2C,
   +0x30, &d16=L+0x28, &d17=L+0x24, scriptName=L+0x158);`
   (floats 7..15 go directly to m_vecQueueDir +0x10..0x18, m_vecUseDir +0x1C..0x24, m_vecForwardDir +0x28..0x30).
2. `byte[+0x34] (m_nAttractorType) = atype;`
3. `strcpy(effect+0x38 /*m_szScriptName, char[8]*/, L+0x158)` (inline byte copy including the NUL, unbounded:
   names of 8+ chars overflow into the next store slot). The local buffer is not initialised, so a missing
   `%s` copies stack garbage up to the first NUL.
4. `byte[+0x35] (m_nPedExistingProbability) = d16; byte[+0x36] (field_36) = d17;` +0x37 (m_nFlags) not written.

### type 5 EFFECT_INTERIOR (0x5B7B1B)
1. Pre-set to -1 (i32): L+0x20, 0x24, 0x28, 0x2C, 0x30, 0x34, 0x38, 0x3C, 0x40, 0x44, 0x48, 0x4C, 0x50, 0x60,
   0x64, 0x68, 0x6C, 0x70, 0x74, 0x7C, 0x80, 0x84, 0x88, 0x8C, 0x90; L+0x78 = 50 (0x32). Not pre-set:
   L+0x54, L+0x58, L+0x5C, L+0xC0, L+0xE8.
2. `sscanf(line, "%d %f %f %f %d %d %f %f %f %f %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d  %d %d %d %d  %d %d %d %d  %d %d %d %d"
   /*0x8698F8*/, ...)` with 38 targets; numbering the conversions #1..#38:
   #1 L+0x1C, #2 L+0x14, #3 L+0x10, #4 L+0x18, #5 L+0xC, #6 `(int*)(effect+0x10)` (a full int32 store at
   +0x10..0x13), #7 L+0xE8 (f32), #8 L+0xC0 (f32), #9 L+0x5C (f32), #10 `(float*)(effect+0x30)` (m_rot),
   #11 L+0x54, #12 L+0x58, #13 L+0x38, #14 L+0x78, #15 L+0x44, #16 L+0x50, #17 L+0x3C, #18 L+0x4C,
   #19 L+0x40, #20 L+0x48, #21 L+0x34, #22 L+0x2C, #23 L+0x30, #24 L+0x24, #25 L+0x28, #26 L+0x20,
   #27 L+0x6C, #28 L+0x7C, #29 L+0x60, #30 L+0x88, #31 L+0x70, #32 L+0x80, #33 L+0x64, #34 L+0x8C,
   #35 L+0x74, #36 L+0x84, #37 L+0x68, #38 L+0x90.
3. Width/depth/height: `fld f32 #7; _ftol -> byte[+0x12] (m_width)`; `fld f32 #8; _ftol -> byte[+0x13]
   (m_depth)`; `fld f32 #9; _ftol -> byte[+0x14] (m_height)` (in this order; truncation toward zero; only
   the low byte of the 64-bit result is stored). These overwrite bytes 2..3 of #6's int store; byte +0x10
   (m_type) keeps the low byte of #6, byte +0x11 is overwritten below.
4. Byte stores (low byte of each local): +0x11 = #13 (m_groupId), +0x15 = #11 (m_door), +0x16 = #15
   (lDoorStart), +0x17 = #16 (lDoorEnd), +0x18 = #19 (rDoorStart), +0x19 = #20 (rDoorEnd), +0x1A = #17
   (tDoorStart), +0x1B = #18 (tDoorEnd), +0x1C = #21 (lWindowStart), +0x1D = #22 (lWindowEnd), +0x1E = #25
   (rWindowStart), +0x1F = #26 (rWindowEnd), +0x20 = #23 (tWindowStart), +0x21 = #24 (tWindowEnd),
   +0x22/+0x23/+0x24 = #27/#31/#35 (m_noGoLeft[0..2]), +0x25/+0x26/+0x27 = #28/#32/#36 (m_noGoBottom[0..2]),
   +0x28/+0x29/+0x2A = #29/#33/#37 (m_noGoWidth[0..2]), +0x2B/+0x2C/+0x2D = #30/#34/#38 (m_noGoDepth[0..2]),
   +0x2E = #12 (m_seed), +0x2F = #14 (m_status, default 50).
   (The interleaving is exactly as listed: conversions 27..38 are four groups of three, stored by group.)
5. Exit.
Note: missing conversions #7-#9, #11, #12 leave uninitialised stack values (floats then go through `_ftol`).

### type 6 EFFECT_ENEX (0x5B7DC8)
1. `sscanf(line, "%d %f %f %f %d %f %f %f %f %f %f %f %d %d %s %d" /*0x8698C4*/, &L+0x1C, &L+0x14, &L+0x10,
   &L+0x18, &L+0xC, effect+0x10, +0x14, +0x18, +0x1C, +0x20, +0x24, +0x28, &d13=L+0x58, &d14=L+0x54,
   name=L+0x88, &d16=L+0x5C);`
   (floats 6..12 -> m_fEnterAngle +0x10, m_vecRadius +0x14/+0x18, m_vecExitPosn +0x1C..0x24, m_fExitAngle +0x28).
2. `u16[+0x2C] (m_nInteriorId) = d13; byte[+0x2E] (m_nFlags1) = d14; byte[+0x2F] (m_nSkyColor) = d16;`
3. `strncpy(effect+0x30 /*m_szInteriorName*/, name, 8)` (pads with NULs; not terminated when 8+ chars).
   +0x38 m_nTimeOn, +0x39 m_nTimeOff, +0x3A m_nFlags2 are not written here.

### type 7 EFFECT_ROADSIGN (0x5B7711)
1. Four 33-byte local text buffers are each set to `' '` followed by 32 NUL bytes: t1 = L+0xEC, t2 = L+0x88,
   t3 = L+0x134, t4 = L+0x110.
2. `sscanf(line, "%d %f %f %f %d %f %f %f %f %f %d %s %s %s %s" /*0x869A00*/, &L+0x1C, &L+0x14, &L+0x10,
   &L+0x18, &L+0xC, &f6=L+0x20, &f7=L+0x28, &f8=L+0x24, &f9=L+0x30, &f10=L+0x2C, &d11=L+0x34, t1, t2, t3, t4);`
3. Raw 32-bit copies: +0x10 = f6, +0x14 = f7 (m_vecSize), +0x18 = f8, +0x1C = f9, +0x20 = f10 (m_vecRotation);
   `u16[+0x24] (m_nFlags) = d11`.
4. `text = CMemoryMgr::Malloc(0x40, 0); effect+0x28 (m_pText) = text;` (no null check)
   `memcpy(text+0x00, t1, 16); memcpy(text+0x10, t2, 16); memcpy(text+0x20, t3, 16); memcpy(text+0x30, t4, 16);`
   (dword copies; the 16 bytes are taken as they are - no terminator is added; a missing token leaves
   `" "` + NULs from step 1).
5. `effect+0x2C (m_pAtomic) = NULL`. Exit.
Edge: a `%s` token longer than 32 chars overflows the local buffer (unbounded `%s`).

### type 8 EFFECT_TRIGGER_POINT (slot machine wheel) (0x5B7E63)
`sscanf(line, "%d %f %f %f %d %d" /*0x8698B0*/, &L+0x1C, &L+0x14, &L+0x10, &L+0x18, &L+0xC, &id=L+0xC8);
i32[+0x10] (m_nId) = id;` Exit. (`id` uninitialised if absent.)

### type 9 EFFECT_COVER_POINT (0x5B7EAA)
`sscanf(line, "%d %f %f %f %d %f %f %d" /*0x869898*/, &L+0x1C, &L+0x14, &L+0x10, &L+0x18, &L+0xC,
&dx=L+0xD0, &dy=L+0xD8, &usage=L+0xB4);` then `+0x10 = dx, +0x14 = dy` (raw f32 copies, m_DirOfCover),
`byte[+0x18] (m_Usage) = usage`. Exit.

### type 10 EFFECT_ESCALATOR (0x5B7F15)
1. `sscanf(line, "%d %f %f %f %d %f %f %f %f %f %f %f %f %f %d" /*0x869868*/, &L+0x1C, &L+0x14, &L+0x10,
   &L+0x18, &L+0xC, &a=L+0xE0, &b=L+0xBC, &c=L+0xDC, &d=L+0xC4, &e=L+0xAC, &f=L+0xCC, &g=L+0xE4, &h=L+0xD4,
   &i=L+0xB0, &dir=L+0xB8);`
2. Raw copies: +0x10 = a, +0x14 = b, +0x18 = c (m_vecBottom); +0x1C = d, +0x20 = e, +0x24 = f (m_vecTop);
   +0x28 = g, +0x2C = h, +0x30 = i (m_vecEnd); `byte[+0x34] (m_nDirection) = (dir != 0) ? 1 : 0`
   (full 32-bit test). Exit.

### Edge cases (all cases)
- No validation of model id, store capacity, sscanf return values, or string lengths (see per-case notes);
  uninitialised locals are copied when a line has fewer fields. A port must reproduce the results for
  well-formed lines exactly; malformed-line behaviour depends on stack garbage and is not portable bit-exact
  (see open question).
- The TXD stack is pushed once and popped exactly once on every path (tail call).

### gta-reversed
Not reversed (`plugin::Call`). No deviation rows. re3/reVC versions differ in fields and formats; do not use.

### Open questions
- Uninitialised-local behaviour for malformed lines (e.g. 0x5B76B9 model id, 0x5B7CB7 interior floats,
  0x5B7AE2 attractor script name): values come from whatever earlier calls left on the stack. Not resolvable
  statically; the port should treat these as "unspecified" and the parity corpus should only contain
  well-formed IDE lines (the shipped data are well-formed). Does not affect behaviour on valid data.
