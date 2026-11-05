# CCutsceneMgr (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5AFA50 FindCutsceneAudioTrackId  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl int16(const char* name [esp+4])`, return AX (callers use the 16-bit value; upper EAX bits are whatever the
  last instruction left: on the found path EAX = high bits from the `_stricmp` result (0) + AX; on the not-found path `or ax, 0xFFFF`
  keeps the upper half of the previous EAX = table dword). Plain `ret`. Caller: CCutsceneMgr::LoadCutsceneData_preload 0x5B05A0.
- Data: table at 0x8D0AA8 of {const char* name; int32 trackId} (8 bytes), terminated by a null name; 151 entries in the shipped
  .data ("BCESAR2" -> 626 ... "DATE6b" -> 669; several DATE* names map to 669/670).
- Behaviour: for each entry from the first while `entry.name != null`: if `_stricmp(name, entry.name) == 0` (0x8229B6, case-insensitive)
  return `(int16)entry.trackId` (0x5AFA85); not found -> return -1 (0x5AFA7F). The first entry is tested before the loop's null check
  only if its name pointer is non-null (0x5AFA50-0x5AFA5E).
