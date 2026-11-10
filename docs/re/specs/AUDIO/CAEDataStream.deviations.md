# CAEDataStream (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Loaders/AEDataStream.cpp` (pin 20d8d2a). Layout: +0 vptr (0x85F0BC), +4 HANDLE
m_pFileHandle, +8 char* m_pszFilename, +0xC u8 m_bIsOpen, +0x10 u32 m_nCurrentPosition, +0x14 u32 m_nStartPosition,
+0x18 u32 m_nLength, +0x1C i32 m_nTrackId, +0x20 u8 m_bIsEncrypted, +0x24 i32 m_lRefCount.

## 0x4DC2B0 CAEDataStream::Initialise  (gr_deviations = 1)
Original: `__thiscall bool (this = ECX)`, plain `ret`.
Row AEDataStream.cpp:197 (NOTSA: rewritten with CFileMgr).
- Original bytes: if `+0xC` return 1. `h = CreateFileA(+8, 0x80000000 GENERIC_READ, 1 FILE_SHARE_READ, NULL, 3 OPEN_EXISTING,
  0, NULL)` (IAT 0x8580A0); `+4 = h` (stored even when invalid). If `h != -1`: `+0x10 = SetFilePointer(h, +0x14, NULL, 0)`
  (IAT 0x858090), `+0xC = 1`, result = 1. Then, independently of success, if `+0x18 == 0`: `+0x18 = GetFileSize(+4, NULL)`
  (IAT 0x858074) - with an invalid handle this stores 0xFFFFFFFF. Returns the success byte.
- Verdict: CONFIRMED - gr uses CFileMgr (and calls `CFileMgr::SetDir("")`, which changes the file manager's current
  directory as a side effect), stores nullptr instead of INVALID_HANDLE_VALUE, and only fills the length on success.
  Class: keep-original (Win32 semantics: failure leaves `+4 = -1` and may set `+0x18 = 0xFFFFFFFF`).

## 0x4DC620 CAEDataStream::Constructor  (gr_deviations = 1)
Original: `__thiscall CAEDataStream* (this; i32 trackId [esp+4], char* filename [+8], u32 start [+0xC], u32 length [+0x10],
bool encrypted [+0x14])`, `ret 0x14`, returns this.
Row AEDataStream.cpp:9 ("originally startPosition and length is int32").
- Original bytes: `+8 = filename; +0xC = 0; +0x10 = 0; +0x14 = start; +0x18 = length; +0 = 0x85F0BC; +0x1C = trackId;
  +0x20 = encrypted; +0x24 = 1` (0x4DC62A-0x4DC653). Plain dword moves: signedness is irrelevant here.
- Verdict: REFUTED (no behavioural difference in this function). Class: harmless. Note: the original does NOT write `+4`
  (file handle); gr sets it to nullptr - harmless because Initialise overwrites it before use.
