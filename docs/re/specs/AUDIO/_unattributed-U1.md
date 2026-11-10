# AUDIO unattributed / unnamed functions (RE-SPEC-3, group U1)

Each section proposes a Class::Method name with the evidence. Sections ordered by address.

## 0x4D7990 CAEAudioChannel::sub_4D7990 (proposed: CAEAudioChannel::IsBufferPlaying)  (catalogue: unknown/REA, port level 0)
- Evidence: returns `m_nBufferStatus (+0x5C) & 1` (DSBSTATUS_PLAYING), exactly gr's inline `CAEAudioChannel::IsBufferPlaying()`
  (AEAudioChannel.h:65); callers are CAEAudioChannel-derived methods (CAEStreamThread::MainLoop 0x4F15C0,
  CAEStreamingChannel::SetFrequencyScalingFactor 0x4F2060, CAEStreamingChannel::Pause 0x4F2170).
- Convention: `__thiscall u32 (this = ECX)`, plain `ret`. Behaviour: `eax = [ecx+0x5C] & 1` (0x4D7990-0x4D7993); full
  EAX is the result (0 or 1).
- gta-reversed: inline in the header (not hooked). Open questions: none.

## 0x4DC670 sub_4DC670 (proposed: operator==(const CVector&, const CVector&))  (catalogue: unknown/REA, port level 0)
- Evidence: compares three consecutive f32 of two pointers; callers compare sound positions (CAEScriptAudioEntity::
  PlayLoadedMissionAudio 0x4EC270 (2), PlayResidentSoundEvent 0x4EC550, PlayMissionBankSound 0x4EC6D0 (4),
  CAEWeatherAudioEntity::UpdateParameters 0x505A00). gr has an inline `operator==(CVector, CVector)` (not hooked).
- Convention: `__cdecl i32 (const CVector* a [esp+4], const CVector* b [esp+8])`, EAX = 1/0, plain `ret`.
- Behaviour: `a.x == b.x && a.y == b.y && a.z == b.z`, each `fld a; fcomp b; test ah,0x44; jp false` (exact equality; NaN or
  any difference -> 0), evaluated x, y, z in order with early exit (0x4DC674-0x4DC69B). Full EAX is written.
- gta-reversed: inline operator (same semantics). Open questions: none.

## 0x4DEFE0 CAEGlobalWeaponAudioEntity::sub_4DEFE0 (proposed: CAEGlobalWeaponAudioEntity::PickAmbientGunFire)  (catalogue: unknown/REA, port level 1)
- Evidence: only caller ServiceAmbientGunFire 0x4DF210 (2 sites), passing `&s_GunShots 0xB61318, &s_WeaponType 0xB6131C,
  &s_Delay 0xB61320` (0x4DF42A-0x4DF445); body matches gr's (unhooked) `PickAmbientGunFire(int32&, eWeaponType&, uint32&)`.
- Convention: `__thiscall void (this = ECX unused; i32* shots [esp+4], i32* type [+8], u32* delay [+0xC])`, `ret 0xC`.
- Behaviour: `c = CAEAudioUtility::GetRandomNumberInRange<int32>(0, 1)` 0x4D9C10. If `c == 1`: `*shots = Rnd(2, 4)`,
  `*type = 0x16` (WEAPON_PISTOL), `*delay = Rnd(500, 1200)`. If `c == 0`: `*shots = Rnd(4, 14)`, `*type = 0x1E` (WEAPON_AK47),
  `*delay = Rnd(90, 350)`. Any other `c` (impossible): nothing written (0x4DEFF2 `jne` to ret). Order of RNG calls: c, shots, delay.
- gta-reversed: uses `CGeneral::DoCoinFlip()` for `c` (different RNG formula) - CONFIRMED deviation, keep-original
  (CAEAudioUtility range 0..1 via 0x4D9C10). Note: the AK delay here is 90..350, while ServiceAmbientGunFire's per-shot
  re-roll uses 90..450 (0x4DF86C) - both original. Open questions: none.

## 0x4DFB60 sub_4DFB60 (proposed: CAEBankLoader::InitialiseRequests)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller CAEMP3BankLoader::Initialise 0x4E08F0; it is the first block of gr's `Initialise` (`rng::fill(m_Requests,
  CAESoundRequest{}); m_NextOneSoundReqIdx = m_RequestCnt = m_NextRequestIdx = 0;`, AEMP3BankLoader.cpp:27-30). Requests at
  +0x24, 50 x 0x20 (gr CAESoundRequest: +0 SlotInfo, +0x14 Status, +0x18 i16 Bank, +0x1A i16 Slot, +0x1C i16 SoundID).
- Convention: `__thiscall void (this = ECX: CAEMP3BankLoader*)`, plain `ret`.
- Behaviour: for each of the 50 requests (0x4DFB70 loop, ESI = 0x32): `Bank = -1`, `Slot = -1`, `SoundID = -1` (i16),
  `SlotInfo = 0`, `Status = 0`. BankOffsetBytes, BankNumBytes, StreamDataPtr, StreamBufPtr and PakFileNo are NOT written.
  Then `+0x664 = +0x666 = +0x668 = 0` (u16: m_NextOneSoundReqIdx, m_RequestCnt, m_NextRequestIdx).
- gta-reversed: inlined into Initialise via a value-initialised CAESoundRequest (also zeroes the five unwritten fields -
  harmless, they are written before use). Open questions: none.

## 0x4E7990 sub_4E7990 (proposed: CAEMFDecoder::OpenQuickTimeMovie)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller CAEMFDecoder::Initialise 0x4E7CB0; calls the statically linked QuickTime runtime (0x818970 ..
  0x81DAA0) and looks for a track of media type 'soun' (0x736F756E). The QuickTime routine identities below are inferred from
  argument shapes and are marked "probably".
- Convention: `__cdecl i16 (const char* path [esp+4], Movie* outMovie [esp+8], Media* outMedia [esp+0xC])`, AX = OSErr, plain
  `ret`. Locals: FSSpec at [esp+0x20], resRefNum [esp+8], movie [esp+8 after reuse] (stack frame 0x128).
- Behaviour:
  1. `0x818970(path, &spec, 0)` (probably NativePathNameToFSSpec); `err = 0x81DAA0(&spec, &tmp)` (2 args; result tested as i16);
     if `err != 0` return err (0x4E79C7).
  2. `err = 0x81B680(&spec, &refNum, 1)` (probably OpenMovieFile, fsRdPerm); if `err` return err.
  3. `e2 = 0x81AB80(&movie, refNum, &resId, NULL, 1, &changed)` (probably NewMovieFromFile, newMovieActive); `0x81A9E0(refNum)`
     (probably CloseMovieFile, always called); if `(i16)e2 != 0` return e2.
  4. `track = 0x81BC60(movie, 1, 'soun', 1)` (probably GetMovieIndTrackType); if null return -2009 (0xFFFFF827; AX 0xF827) -
     *outMovie is not written (movie leaks).
  5. `media = 0x81A0E0(track)` (probably GetTrackMedia); `*outMedia = media`; result = 0, or -2008 (0xF828) if media is null;
     `*outMovie = movie`; return result.
- gta-reversed: none (gr replaced the QuickTime decoder by Media Foundation). Port: platform-dependent (QuickTime is not
  available) - record in the ledger. Open questions: exact QuickTime API identities of 0x818970, 0x81DAA0, 0x81B680,
  0x81AB80, 0x81A9E0, 0x81BC60, 0x81A0E0 (only the call shapes are certain).

## 0x4E7A80 sub_4E7A80 (proposed: CAEMFDecoder::GetQuickTimeSoundDescription)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller CAEMFDecoder::Initialise 0x4E7CB0; queries a QuickTime media's sample description and its 'wave'
  (0x77617665) extension and copies format fields into a caller struct.
- Convention: `__cdecl i16 (Media media [esp+4], void** outExt [esp+8], u8* outFmt [esp+0xC])`, AX = error, plain `ret`.
- Behaviour:
  1. `h1 = 0x818DB0(0)` (probably NewHandle); `0x81A300(media, 1, h1)` (probably GetMediaSampleDescription); `err =
     0x819860()` (probably GetMoviesError); if `(i16)err != 0` go to 4.
  2. `h2 = 0x818DB0(0)`; if `0x81C600(h1, &h2, 'wave') != 0` (probably GetSoundDescriptionExtension): `err = 0`, go to 3.
     Else: `n = 0x8192B0(h2)` (GetHandleSize); `0x818F90(h2)` (HLock); `*outExt = 0x818E70(n)` (NewPtr); `err = 0x818D70()`
     (MemError); `0x819390(*h2, *outExt, n)` (BlockMove); `0x818FB0(h2)` (HUnlock).
  3. From `d = *h1`: `outFmt+0x28 = d+4` (dword), `outFmt+4 = (i32)(i16)(d+0x18)`, `outFmt+0x3E = (u16)(d+0x1A)`, `outFmt+8 =
     d+0x20` (dword, 16.16 fixed sample rate), `outFmt+0x38 = (u16)(d+0x1C)` (0x4E7B1F-0x4E7B4C).
  4. If `h2` (the local, 0 when step 2 was skipped) non-null: `0x819270(h2)` (DisposeHandle); if `h1`: `0x819270(h1)`. Return `err`.
- gta-reversed: none. Open questions: QuickTime API identities as listed (call shapes certain); field meaning of the
  CAEMFDecoder offsets +4/+8/+0x28/+0x38/+0x3E.

## 0x4F0B10 CAEStaticChannel::CAEStaticChannel (proposed)  (catalogue: unknown/REA, port level 1)
- Evidence: calls the CAEAudioChannel ctor 0x4D7890 and installs vtable 0x85F3CC; fields +0x60/+0x62/+0x8A match gr
  `CAEStaticChannel(IDirectSound*, uint16 channelId, bool hardwareMixAvailable, uint32 samplesPerSec, uint16 bitsPerSample)`
  (AEStaticChannel.cpp:25; m_OverwriteIntroWhenWrapped +0x60, m_bShouldPlay +0x62, m_IsHardwareMixAvailable +0x8A).
  Caller: CAEAudioHardware::Initialise 0x4D9930.
- Convention: `__thiscall CAEStaticChannel* (this = ECX; ds [esp+4], u16 channelId [+8], u8 hwMix [+0xC], u32 sps [+0x10],
  u16 bits [+0x14])`, returns this, `ret 0x14`.
- Behaviour: `CAEAudioChannel::CAEAudioChannel(this, ds, channelId, sps, bits)` (0x4F0B29); `+0x60 = 0`, `+0x62 = 0`;
  `+0 = 0x85F3CC` (0x4F0B3A); `+0x8A = hwMix` (0x4F0B40). Nothing else is initialised.
- gta-reversed: same. Open questions: none.

## 0x4F0B50 CAEStaticChannel::~CAEStaticChannel (proposed)  (catalogue: unknown/REA, port level 1)
- Evidence: installs vtable 0x85F3CC (CAEStaticChannel) and chains to ~CAEAudioChannel 0x4D7910; called from the deleting
  dtor 0x4F1020.
- Convention: `__thiscall void (this = ECX)`, plain `ret`. MSVC EH frame (handler 0x83C0D8, state 0 then -1 before the base
  dtor call, 0x4F0B78/0x4F0BB2); no user-visible effect.
- Behaviour: `+0 = 0x85F3CC`; if `+8 != 0` `--g_numSoundChannelsUsed` (0xB5F898); if `+8 != 0` `(+8)->Release()` (vtable +8);
  `+8 = 0`; if `+0xC != 0` `(+0xC)->Release()`; `+0xC = 0` (0x4F0B6D-0x4F0BAB); then `CAEAudioChannel::~CAEAudioChannel(this)`
  0x4D7910 (sets the base vptr, finds both pointers null, does nothing else).
- gta-reversed: implicit dtor in gr (no body hooked). Open questions: none.

## 0x502BF0 sub_502BF0 (proposed: CAEWMADecoder::ReleaseLibrary)  (catalogue: unknown/REA, port level 0)
- Evidence: frees `CAEWMADecoder::wmvCoreModule` (0xB6BAD4) guarded by the flag at 0xB6BAD8 (the WMV core "loaded" flag);
  only caller CAEUserRadioTrackManager::Shutdown 0x4F2FB0.
- Convention: `__cdecl void ()` (static), plain `ret`.
- Behaviour: if `*(u32*)0xB6BAD8 != 0`: `FreeLibrary(*(HMODULE*)0xB6BAD4)` (IAT 0x858110), `*(u32*)0xB6BAD8 = 0` (0x502C05).
  The module handle itself is not cleared.
- gta-reversed: no hooked equivalent found. Open questions: none.

## vt_866db0 - WinSock TCP helper class (proposed: CNetSocket / "WINSOCK" helper)
Vtable 0x866DB0: slot 0 0x59CB70, slots 1..4 `_purecall` 0x82263A, slot 5 0x59CD50, slot 6 0x59CC70, slot 7 0x59CCC0, slot 8
0x59CCE0, slot 9 0x59CD10; the string "WINSOCK." follows the table (0x866DD8). No callers in the catalogue (all reached only
through the vtable; the object is never constructed in the shipped code paths we have). The catalogue slot names
(Destructor, SynchPlayback, Stop, SetFrequencyScalingFactor, vfunc_9, Play) come from an audio-channel vtable guess and are
WRONG: the bodies are WinSock wrappers (WS2_32 ordinals: 3 closesocket, 4 connect, 9 htons, 11 inet_addr, 16 recv, 19 send,
23 socket; thunks at 0x8180B6, 0x81809E, 0x8180AA, 0x8180A4, 0x8180C2, 0x8180BC, 0x8180B0). `this` is never read by slots 6..9.

## 0x59CB70 vt_866db0::Destructor (scalar deleting destructor)  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`. Behaviour: `+0 = 0x866DB0`; if `flags & 1` `_free(this)`
  0x8214BD; return this. Open questions: none.

## 0x59CC70 vt_866db0::SynchPlayback (proposed: Connect)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall SOCKET (this unused; u32 port [esp+4], const char* ip [esp+8])`, `ret 8`.
- Behaviour: `s = socket(2 AF_INET, 1 SOCK_STREAM, 0)`; `sockaddr_in sa` on the stack: `family = 2` (u16), `port = htons(port)`
  (u16), `addr = inet_addr(ip)` (zero padding is NOT cleared - the 8 bytes of sin_zero are stack garbage); `r = connect(s, &sa,
  16)`; return `r == -1 ? 0 : s` (`inc; neg; sbb; and`, 0x59CCB2-0x59CCB7). The socket is not closed on failure. Open questions: none.

## 0x59CCC0 vt_866db0::Stop (proposed: Close)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall i32 (this unused; SOCKET s [esp+4])`, `ret 4`. Behaviour: `closesocket(s)`; return 1. Open questions: none.

## 0x59CCE0 vt_866db0::SetFrequencyScalingFactor (proposed: Send)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall i32 (this unused; SOCKET s [esp+4], const char* buf [esp+8], i32 len [esp+0xC])`, `ret 0xC`.
- Behaviour: `r = send(s, buf, len, 0)`; return `r == -1 ? 0 : r` (`sete; dec; and`). Open questions: none.

## 0x59CD10 vt_866db0::vfunc_9 (proposed: RecvAll)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall i32 (this unused; SOCKET s [esp+4], char* buf [esp+8], i32 len [esp+0xC])`, `ret 0xC`.
- Behaviour: `r = 0`; while `len > 0`: `r = recv(s, buf, len, 0)`; if `r <= 0` break; `len -= r; buf += r`. Returns the LAST
  `recv` result (not the total): positive byte count of the final chunk on success, 0 / -1 on close/error, 0 if `len <= 0`
  initially. Open questions: none.

## 0x59CD50 vt_866db0::Play (proposed: scalar deleting destructor, slot 5)  (catalogue: specified/REA, port level 1)
- Name correction: not "Play" - it calls 0x59CBA0 (`mov [ecx], 0x866DB0; ret`, the base destructor body) and frees.
- Convention: `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`. Behaviour: `0x59CBA0(this)`; if `flags & 1` `_free(this)`;
  return this. Open questions: why a second deleting destructor sits in slot 5 (likely a derived/aliased dtor).

## 0x747300 getDvdGamePath  (catalogue: specified/REA, port level 2)
- Name: already named (gr WinMain.cpp `char* getDvdGamePath()`, comment 0x747300); a free function, kept here because it has no
  class. Callers: CAEMP3TrackLoader::IsCurrentAudioStreamAvailable 0x4E0BD0, CAEMP3TrackLoader::Initialise 0x4E0C50.
- Convention: `__cdecl char* ()`, plain `ret`. Frame `sub esp,0x110`: drive buffer at [esp+0x10] (8 bytes), result at
  [esp+0x18] (init 0), count at [esp+0x1C], volume-name buffer at [esp+0x20] (0xFF bytes passed).
- Callees (IAT): SetErrorMode 0x85813C, GetLogicalDriveStringsA 0x858138, GetDriveTypeA 0x858134, GetVolumeInformationA
  0x858130, lstrlenA 0x8580DC, lstrcpyA 0x8580D4; operator new 0x821195 (jmp 0x82119A), _free 0x8213AE (jmp 0x8214BD).
- Behaviour:
  1. `SetErrorMode(1)` (SEM_FAILCRITICALERRORS; never restored). `n = GetLogicalDriveStringsA(0, NULL)`; `drives = new(n)`;
     `len = GetLogicalDriveStringsA(n, drives)`.
  2. Byte loop `i = 0 .. len-1` (unsigned, 0x74733D/0x74739F) with `j` (EDI) = write index into the 8-byte drive buffer:
     `drive[j++] = drives[i]`; when the byte is 0: if `GetDriveTypeA(drive) == 5` (DRIVE_CDROM) and
     `GetVolumeInformationA(drive, vol, 0xFF, NULL, NULL, NULL, NULL, 0) != 0` and the first 16 bytes of `vol` equal
     "GTA_SAN_ANDREAS\0" (0x874B08, `repe cmpsb` with ECX = 0x10, 0x747392): `res = new(lstrlenA(drive) + 0x21)`,
     `lstrcpyA(res, drive)` and go to 3. Otherwise `j = 0` and continue.
  3. `_free(drives)`; return `res` (null if not found).
- Edge cases / original bugs: the drive buffer is 8 bytes and the copy is unbounded (a logical drive string longer than 7
  characters would overwrite [esp+0x18] = result, then more stack); in practice drive roots are "X:\". The returned
  buffer is never freed by the callers' success paths (+0x18 of the track loader keeps it). Volume label compare is 16 bytes
  exact (label must be exactly "GTA_SAN_ANDREAS").
- gta-reversed: reversed (WinMain.cpp:37) with strcpy_s / MAX_PATH+1 buffers - same result for normal drive strings (gr allocates strlen+1, the original strlen+0x21); its
  `assert(bufferSize > 0)` row is debug-only (harmless). Open questions: none.
