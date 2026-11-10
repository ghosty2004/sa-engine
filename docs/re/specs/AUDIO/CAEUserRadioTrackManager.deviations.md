# CAEUserRadioTrackManager (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Managers/AEUserRadioTrackManager.cpp` (pin 20d8d2a). Fields: +0 tUserTracksInfo*
m_pUserTracksInfo (12-byte records {u32 offset, u32 length, i32 type}), +6 u16 m_nUserTracksCount, +0x18 i32 m_nUserTracksScanState
(2 = COMPLETE, 3 = ERROR). Strings: 0x860A88 "sa-utrax.dat", 0x860A98 "sa-ufiles.dat", 0x860ADC "wb", 0x859C6C "rb",
0x860ACC "\\User Tracks\\", 0x860AC4 "\\*.*", 0x859F7C "\\", 0x860AA8 ".lnk", 0x858B54 "".

## 0x4F2FD0 CAEUserRadioTrackManager::ReadUserTracks  (gr_deviations = 1)
Original: `__thiscall bool (this)`. `SetDirMyDocuments()`; `f = OpenFile("sa-utrax.dat", "rb")`; `SetDir("")`; if `f <= 0` (signed)
return 0; `size = GetTotalSize(f)`; if 0: close, return 0; `+6 = (u16)(size / 12)` (`mul 0xAAAAAAAB; shr 3`); `+0 =
CMemoryMgr::Malloc(size, 0)`; `Read(f, +0, size)`; close; return 1.
Row :166 (NOTSA: free the previous buffer): CONFIRMED - the original overwrites +0 without freeing (leak on repeated calls).
Class: keep-original (leak is invisible; port may free but must not change observable state - record in ledger).

## 0x4F30F0 CAEUserRadioTrackManager::ResolveShortcut  (gr_deviations = 3)
Original: `char* (const char* path [esp+4])`, `ret 4` (ECX unused), ANSI. `CoCreateInstance(CLSID 0x860A68, NULL, 1, IID 0x860838,
&link)` fail -> return 0. `link->QueryInterface(IID_IPersistFile 0x887A1C, &pf)` fail -> `link->Release()`, return 0.
`MultiByteToWideChar(0, 0, path, -1, wbuf, 0x104)`; `pf->Load(wbuf, 0)` (vtable +0x14) fail -> release both, return 0.
`buf = new(0x104)`; `hr = link->GetPath(buf, 0x104, &findData, 1)` (vtable +0xC, flags = 1 = SLGP_SHORTPATH); release pf and link;
return `hr < 0 ? 0 : buf` (buf leaks on failure).
Row :381 (NOTSA wide-string function): harmless replacement. REFUTED.
Row :387 (`NOTSA_UNREACHABLE` on CoCreateInstance failure): CONFIRMED (original returns null). Class: keep-original.
Row :401 (`NOTSA_UNREACHABLE` on Load/GetPath failure): CONFIRMED (original returns null). Class: keep-original.
Found in passing: gr calls GetPath with flags 0 (long path), the original with 1 (8.3 short path) - CONFIRMED, keep-original.

## 0x4F4690 CAEUserRadioTrackManager::WriteUserTracksFile  (gr_deviations = 2)
Original: `__thiscall i32 (this; char* dir, u32* curLen, FILE f, vector<tUserTracksInfo*>* recs, i32 depth)`, `ret 0x14`.
If `depth >= 15` return 0 (0x4F4696). Walks `dir + "\\*.*"` with FindFirstFileA/FindNextFileA/FindClose; skips names starting
with '.'; sub-directories recurse with `depth + 1`; names ending in ".lnk" (strrchr + _stricmp) are resolved with
ResolveShortcut and checked with GetFileAttributesA (directory -> recurse); other files with `GetAudioFileType(path) != 0`
0x4F31F0 get a heap record `{*curLen, (u16)lstrlenA(path), type}`, `CFileMgr::Write(f, path, len)`, push_back via 0x4F4620,
`*curLen += len`; returns the track count.
Row :311 (`todo: FIX_BUGS`): no behaviour. REFUTED, harmless.
Row :354 (NOTSA: UTF-8 generic path): CONFIRMED - the original writes the ANSI Windows path with backslashes ("<MyDocs>\\User
Tracks\\..."); gr writes a UTF-8 path with '/' separators, so sa-ufiles.dat contents and offsets differ. Class: keep-original.

## 0x4F4A20 CAEUserRadioTrackManager::WriteUserTracksThread  (gr_deviations = 1)
Original: `__stdcall u32 (CAEUserRadioTrackManager* self)`, `ret 4`, returns 0.
- Original bytes: `CoInitialize(NULL)`; `SetDirMyDocuments()`; `f = OpenFile("sa-ufiles.dat", "wb")`; if `f == 0`: `self->+0x18 = 3`,
  free the (empty) vector, return 0 (no CoUninitialize, no SetDir). Else `path = new(lstrlenA(ms_dirName 0xB71A60) + 0xF)` =
  ms_dirName + "\\User Tracks\\"; `n = WriteUserTracksFile(path, &len, f, &vec, 0)`; free path; close f; `u = OpenFile(
  "sa-utrax.dat", "wb")` (NOT checked); if `n == 1`: `Write(u, vec[0], 12)` once extra (0x4F4AF5-0x4F4B06); then for every
  record `Write(u, rec, 12)` and free it; close u; `SetDir("")`; `*(i32*)0xBA67F4 = 0` (FrontEndMenuManager+0xAC, gr
  m_UserTrackIndex); `self->+0x18 = 2`; `CoUninitialize()`; free the vector; return 0.
Row :292 ("GTASA doesn't check if sa-utrax.dat fails to open"): CONFIRMED (gr sets ERROR; original writes to handle 0 /
continues). Class: keep-original.
Found in passing (CONFIRMED, keep-original): (1) with exactly one track the original writes its record twice into
sa-utrax.dat (original bug); (2) gr calls `ReadUserTracks()` at the end and stores m_bUserTracksLoaded - the original thread does
not; (3) gr's scan state on an empty scan differs (original always COMPLETE once sa-ufiles.dat opened).
