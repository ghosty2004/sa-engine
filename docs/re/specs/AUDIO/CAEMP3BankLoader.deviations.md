# CAEMP3BankLoader (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Loaders/AEMP3BankLoader.h/.cpp` (pin 20d8d2a). Fields: +0x14 u8 m_IsInitialised,
+0x1C u8* m_Buffer, +0x34.. CAESoundRequest m_Requests[50] (stride 0x20 as walked from +0x3E-6 = status), +0x66A i16
m_StreamingChannel.

## 0x4DFB10 CAEMP3BankLoader::Constructor  (gr_deviations = 1)
Original: `__thiscall CAEMP3BankLoader* (this = ECX)`, returns this, plain `ret`.
Row AEMP3BankLoader.h:41 (NOTSA `Constructor()` wrapper calling the C++ ctor).
- Original bytes: only `+0x14 = 0` (u8) and `+0x66A = 4` (u16) (0x4DFB12/0x4DFB16). No other field is initialised.
- Verdict: REFUTED as a behavioural deviation for the two fields; note that gr's ctor (default member initialisers of
  CAEBankLoader) also zeroes every other member, while the original leaves them as allocated (the object comes from
  operator new in CAEAudioHardware::Initialise 0x4D9930). Class: harmless (nothing reads them before Initialise).

## 0x4DFE30 CAEMP3BankLoader::Service  (gr_deviations = 6)
Original: `__thiscall void (this = ECX)`, plain `ret`. Loop over the 50 requests (`cmp eax,0x32`, 0x4E0199).
Rows :221, :258, :259, :317, :318 (`assert(...)`): no checks in the original (memcpy at 0x4DFE92 etc. is unconditional).
Verdict: REFUTED (debug-only). Class: harmless.
Row :345 (`NOTSA_UNREACHABLE("Invalid: {}")` for an unknown request status):
- Original bytes: `s = req.Status`; `s == 1` -> REQUESTED block 0x4E010F; `s == 2` -> PENDING_READ 0x4DFF50; `s == 3` ->
  PENDING_LOAD_ONE_SOUND 0x4DFE5A; anything else (0 or invalid) -> next request (0x4DFE54 `jne 0x4E0191`).
- Verdict: CONFIRMED (gr aborts; the original skips). Class: keep-original.
