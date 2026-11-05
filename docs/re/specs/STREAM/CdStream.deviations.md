# CdStream: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/CdStreamInfo.cpp` (pin 20d8d2a). CdStream stride 0x30:
+0 sector offset, +4 sectors to read, +8 buffer, +0xD bLocked, +0xE bInUse, +0x10 status, +0x14 sync
semaphore, +0x18 file handle, +0x1C OVERLAPPED. Status values: 0 success, 0xFA waiting, 0xFE failure,
0xFF reading.

## 0x406460 CdStream::CdStreamSync  (gr_deviations = 3)
Rows CdStreamInfo.cpp:109, :115, :119 (`#ifdef APPLY_CD_STREAM_DEADLOCK_FIX`).
- `APPLY_CD_STREAM_DEADLOCK_FIX` is not defined anywhere in the gr tree (grep of the whole repo finds only
  the uses in CdStreamInfo.cpp), so gr's default build compiles the original code path.
- Original bytes (.HOODLUM 0x156CD80): no lock. If `gStreamingInitialized`: if `nSectorsToRead != 0` then
  `bLocked = 1` (0x156CDA4) and `WaitForSingleObject(sync, INFINITE)`; then `bInUse = 0`, return status.
  Else, if `gOverlappedIO && hFile`: GetOverlappedResult through an obfuscated thunk (0x156CDCD onward);
  else return 0.
- Verdict: REFUTED (gr default build = original).
- Class: keep-original (do not add the critical section; the original race between 0x156CDA4 and the
  thread's `bLocked` test is part of the original behaviour).
- Open question: the GetOverlappedResult tail at 0x156CDD1-0x156CE02 is obfuscated (computed return
  address); its mapping of the result to the return value (gr: false -> 0xFE, else 0) was not decoded here.

## 0x406560 CdStream::CdStreamThread  (gr_deviations = 1)
Row CdStreamInfo.cpp:235 (same define).
- Original bytes: after RemoveFirstInQueue (inlined, 0x40664E-0x406663) it does, without any lock,
  `nSectorsToRead = 0` (0x406669), `if bLocked: ReleaseSemaphore(sync, 1, 0)` (0x40666C-0x406678),
  `bInUse = 0` (0x40667E).
- Verdict: REFUTED (define not set in gr).
- Class: keep-original.

## 0x406A20 CdStream::CdStreamRead  (gr_deviations = 1)
Row CdStreamInfo.cpp:164 (`notsa::IsFixBugs() ? pos.Offset : pos.ToInt()`, CD_STREAM_READ_POS_FIX).
- Original bytes (.HOODLUM 0x156C2C0): `gLastCdStreamPosn (0x8E4898) = sectorCount + posn` with the full
  32-bit posn argument (`lea ecx, [ebx+esi]` at 0x156C2EC, before `and esi, 0xFFFFFF` at 0x156C2FB), i.e.
  the FileID byte (bits 24..31) is included. The masked offset is used only for the stream's sector offset.
- Verdict: CONFIRMED (gr's IsFixBugs() is constexpr true, so gr stores the offset without the FileID).
- Class: keep-original: `gLastCdStreamPosn = sectorCount + posn_raw` (wrapping u32 add).

## 0x406B70 CdStream::CdStreamInit  (gr_deviations = 1)
Row CdStreamInfo.cpp:286 (`#ifndef FIX_BUGS` around FILE_FLAG_NO_BUFFERING).
- Original bytes: `GetDiskFreeSpaceA(NULL, &sectorsPerCluster, &bytesPerSector, &freeClusters,
  &totalClusters)` (0x406BB4); `flags = (bytesPerSector <= 0x800 ? 0x20000000 : 0) | 0x40000000`
  (0x406BBA-0x406BD1, unsigned compare `ja`), stored to gStreamFileCreateFlags (0x8E3FE0).
- Verdict: CONFIRMED (FIX_BUGS is defined in gr, so gr never adds FILE_FLAG_NO_BUFFERING).
- Class: keep-original (add 0x20000000 when bytesPerSector <= 2048; bytesPerSector is whatever
  GetDiskFreeSpaceA wrote, the local is not initialised before the call).
