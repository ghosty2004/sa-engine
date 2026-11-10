# FxInfoManager_c (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Fx/FxInfoManager.cpp`. FxInfoManager_c: +0 m_nNumInfos (int32), +4 m_pInfos
(FxInfo_c**), +8 m_MovementOffset (u8), +9 m_RenderOffset (u8). FxInfo_c: +4 type (u16), +6 m_bTimeModeParticle.
Fx memory pool `g_fxMan.m_Pool` = 0xA9AF2C (`FxMemoryPool_c::GetMem(size, align)` 0x4A9CA0, thiscall).

## 0x4A7B00 FxInfoManager_c::AddFxInfo  (gr_deviations = 3)
Rows FxInfoManager.cpp:93, 94, 95 (NOTSA cases FX_INFO_ROTATE_DATA / TEXCOORDS_DATA / ROTATEOFFSET_DATA):
- Original bytes 0x4A7B0E-0x4A8386: a compare tree (`cmp eax, 0x2080` 0x4A7B12, `0x1200` 0x4A7B2A, `0x1020` 0x4A7B3B,
  byte-indexed jump table 0x4A83A0 -> 0x4A838C for 0x1001..0x1010, `0x4010` 0x4A7F79, ...) that handles exactly the 32
  SA types (0x1001..0x1200, 0x2001..0x2400, 0x4001..0x4400, 0x8001) and calls 32 constructors (callee list: EmRate
  0x4A6A00 ... Smoke 0x4A68E0; 0x2001 calls FUN_004A5270 = FxInfoNoise_c ctor). Each case is
  `p = g_fxMan.m_Pool.GetMem(size, 4)`; `if (p) ctor(p)`; returns p (null when GetMem fails, default 0x4A8376 returns 0).
  `__stdcall`-like thiscall: `this` unused, arg at [esp+4], `ret 4`; SEH frame only for the ctor-throws unwind (`_free`).
- gr: the three Manhunt cases are commented out (`// NOTSA case ...`); their values equal FX_INFO_HEATHAZE_DATA 0x4008,
  FX_INFO_TRAIL_DATA 0x4010 and FX_INFO_SELFLIT_DATA 0x4200, which both gr and the original map to the SA classes.
- Verdict: REFUTED (no behavioural difference). Class: harmless.
- Note for the port: the original allocates from the Fx memory pool (GetMem), not from the C++ heap; gr's `new`
  must be checked against the class operator new before it is reused.

## 0x5C0B70 FxInfoManager_c::Load  (gr_deviations = 1)
Rows :150 and :174 (asserts) are deviates=no (debug checks only). Row FxInfoManager.cpp:164 (`NOTSA_UNREACHABLE("Unknown FX
Info: %s")`):
- Original bytes 0x5C0C35-0x5C15EA: 32 inline `repe cmpsb` compares of the scanned field (`sscanf(line, "%s" [0x85C714],
  field)`, 0x5C0C2D) against the names (first 0x86B2A8 "FX_INFO_EMRATE_DATA:", last 0x86AFC4 "FX_INFO_SMOKE_DATA:"),
  each followed by `push type; jmp 0x5C163B` (AddFxInfo + `info->Load(file, version)` vtable +4 at 0x5C165D). When no
  name matches, `jne 0x5C1676` (0x5C15EA) skips AddFxInfo, the info Load and the trailing ReadLine, and then still does
  `m_pInfos[i]->+6 = timeModePrt` (0x5C1676-0x5C1684) and reads `m_pInfos[i]->+4` (0x5C168D) through the slot that was
  never written (pool memory from GetMem 0x5C0BD8, not cleared): undefined behaviour, no message.
- gr: aborts with a message.
- Verdict: CONFIRMED (the original does not stop; it writes through an uninitialised pointer). Class: harmless
  (only reachable with a malformed effects.fxp); a port may trap here, but must not silently skip.

Found in passing (matches gr, recorded for the port): the TIMEMODEPRT field is read only for non-emission types and
only when `(double)version < 0.7` (f64 0x86B1D0, `fild version; fcomp; test ah,1; jne skip`) is false, i.e. version >= 1
for integer versions (gr: `version >= 1`, equivalent). After the loop: `if (m_RenderOffset == 0xFF) m_RenderOffset =
(u8)m_nNumInfos; if (m_MovementOffset == 0xFF) m_MovementOffset = m_RenderOffset` (0x5C16C6-0x5C16DB). Per info
(0x5C168D-0x5C16B1): `type & 0x1000` -> nothing; else `type & 0x2000` -> movement offset = i if still 0xFF; else
`type & 0xC000` -> render offset = i if still 0xFF. Convention: thiscall, `ret 8` (file, version).
