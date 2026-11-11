# CGenericGameStorage (SAVE) - RE-SPEC-REST group B

## 0x5D0D20 CGenericGameStorage::OpenFileForReading  (catalogue: specified/REA, port level 2)
- `__cdecl bool (const char* fileName [esp+4], int32 slot [esp+8])`, plain `ret`. Globals (CGenericGameStorage statics): ms_WorkBuffer u8* 0xC16EE8, ms_WorkBufferPos u32 0xC16EEC, ms_FileHandle 0xC16EF0, ms_FilePos u32 0xC16EF4, ms_FileSize u32 0xC16EF8, ms_bFailed u8 0xC16EFC, ms_WorkBufferSize u32 0x8D2BE0, ms_CheckSum u32 0xC16134; `PcSaveHelper` = C_PcSave object 0xC17034 whose first dword is the error code.
- 1. If fileName != 0: copy it byte by byte (including the terminator) into ms_LoadFileName 0xC15FC8 (inlined strcpy), then
  `sprintf(ms_LoadFileNameWithPath 0xC15F60, "%s%i%s" (0x86BAD4), C_PcSave::DefaultPCSaveFileName 0xC16F18, slot + 1, ".b" (0x86BADC))`
  (sprintf 0x821BB5). If fileName == 0 both are skipped (the previous ms_LoadFileName is reused).
- 2. `h = CFileMgr::OpenFile(ms_LoadFileName, "rb" (0x859C6C))` 0x538900 (the file opened is ms_LoadFileName, not the ...WithPath
  string built above); `ms_FileHandle = h`. If h == 0: `PcSaveHelper.error = 4`; return 0.
- 3. `ms_FileSize = CFileMgr::GetTotalSize(h)` 0x5389E0; `ms_WorkBufferSize = 0xC800`; `ms_WorkBufferPos = 0xC800`; `ms_FilePos = 0`;
  if ms_WorkBuffer == 0 `ms_WorkBuffer = operator new(0xC801)` 0x821195. Return 1.
- gta-reversed: not reversed (hook only). Open questions: none.

## 0x5D0DD0 CGenericGameStorage::OpenFileForWriting  (catalogue: specified/REA, port level 1)
- `__cdecl bool ()`, plain `ret`. `h = CFileMgr::OpenFile(ms_SaveFileName 0xC16DB8, "wb" (0x860ADC))` 0x538900; `ms_FileHandle (0xC16EF0) = h`;
  if h == 0: `PcSaveHelper (0xC17034).error = 4`; return 0. Else `ms_FilePos (0xC16EF4) = 0`, `ms_WorkBufferPos (0xC16EEC) = 0`; if
  `ms_WorkBuffer (0xC16EE8) == 0` `ms_WorkBuffer = operator new(0xC801)` 0x821195; return 1. ms_WorkBufferSize is NOT reset.

## 0x5D0E30 CGenericGameStorage::CloseFile  (catalogue: specified/REA, port level 2)
- `__cdecl bool ()`, plain `ret`. If `ms_WorkBuffer (0xC16EE8)`: `free(ms_WorkBuffer)` 0x8213AE; `ms_WorkBuffer = 0`. `r = CFileMgr::CloseFile(
  ms_FileHandle 0xC16EF0)` 0x5389D0; returns AL = (r == 0) (`neg; sbb al,al; inc al`).

## 0x5D0E90 CGenericGameStorage::MakeValidSaveName  (catalogue: specified/REA, port level 0)
- `__cdecl void (int32 slot [esp+4])`, plain `ret`, local `char buf[0x104]`.
- `buf[0] = 0`; `sprintf(buf, "%s%i" (0x86BAE0), DefaultPCSaveFileName 0xC16F18, slot + 1)`; if `strlen(buf) >= 0x101` `buf[0x101] = 0`;
  append ".b" (3 bytes copied from 0x86BADC: '.', 'b', 0) at the end of buf (inlined strcat); copy buf into ms_SaveFileName 0xC16DB8
  (inlined strcpy). Then walk ms_SaveFileName from the start until a 0 or '\n' (0x0A) byte: every '?' (0x3F) becomes ' ' (0x20).

## 0x5D0F40 CGenericGameStorage::GetNameOfSavedGame  (catalogue: specified/REA, port level 0)
- `__cdecl char* (int32 slot [esp+4])`: returns `ms_SlotFileName (0xC16368) + 0x104*slot` (no bounds check).

## 0x5D0F50 CGenericGameStorage::GetCurrentVersionNumber  (catalogue: specified/REA, port level 1)
- `__cdecl uint32 ()`, plain `ret`, local `char buf[0x28]`: `sprintf(buf, "%s%s" (0x86BAE8), "Apr 28 2005" (0x86BAF0), "10:28:55"
  (0x86BAFC))` 0x821BB5; returns `CKeyGen::GetKey(buf)` 0x53CF00 (EAX). The build date/time strings are constants of this executable.

## 0x5D0F80 CGenericGameStorage::SaveWorkBuffer  (catalogue: specified/REA, port level 2)
- `__cdecl bool (bool writeChecksum [esp+4])`, plain `ret`. Globals (CGenericGameStorage statics): ms_WorkBuffer u8* 0xC16EE8, ms_WorkBufferPos u32 0xC16EEC, ms_FileHandle 0xC16EF0, ms_FilePos u32 0xC16EF4, ms_FileSize u32 0xC16EF8, ms_bFailed u8 0xC16EFC, ms_WorkBufferSize u32 0x8D2BE0, ms_CheckSum u32 0xC16134; `PcSaveHelper` = C_PcSave object 0xC17034 whose first dword is the error code.
- 1. If ms_bFailed return 0. `n = ms_WorkBufferPos`; if n == 0 return 1.
- 2. For i = 0 .. n-1: `ms_CheckSum += (u8)ms_WorkBuffer[i]` (u32 wrap).
- 3. If writeChecksum: if `n > 0xC7FC` (unsigned): `SaveWorkBuffer(false)` (recursive flush; ms_WorkBufferPos / ms_WorkBuffer reloaded).
  `*(u32*)(ms_WorkBuffer + ms_WorkBufferPos) = ms_CheckSum`; `ms_WorkBufferPos += 4`.
- 4. `w = CFileMgr::Write(ms_FileHandle, ms_WorkBuffer, ms_WorkBufferPos)` 0x538970; if `CFileMgr::GetErrorReadWrite(ms_FileHandle)`
  0x538A50 != 0 or `w != ms_WorkBufferPos`: FAIL. Else `ms_FilePos += ms_WorkBufferPos`; `ms_WorkBufferPos = 0`; return 1.
- FAIL (0x5D104B): `PcSaveHelper.error = 0xB`; if ms_WorkBuffer: free 0x8213AE and set 0; if `CFileMgr::CloseFile(ms_FileHandle)` != 0
  `PcSaveHelper.error = 6`; `strncpy(ms_SaveFileNameJustSaved 0xC16030, ms_SaveFileName 0xC16DB8, 0x103)` 0x821F40; `ms_bFailed = 1`;
  return 0.
- Original bugs (keep): in the recursive case the bytes of the flushed buffer are added to ms_CheckSum twice (step 2 runs in both calls);
  if the recursive flush fails, ms_WorkBuffer is null and step 3 writes the checksum through a null pointer.

## 0x5D10B0 CGenericGameStorage::LoadWorkBuffer  (catalogue: specified/REA, port level 3)
- `__cdecl bool ()`, plain `ret`. Globals (CGenericGameStorage statics): ms_WorkBuffer u8* 0xC16EE8, ms_WorkBufferPos u32 0xC16EEC, ms_FileHandle 0xC16EF0, ms_FilePos u32 0xC16EF4, ms_FileSize u32 0xC16EF8, ms_bFailed u8 0xC16EFC, ms_WorkBufferSize u32 0x8D2BE0, ms_CheckSum u32 0xC16134; `PcSaveHelper` = C_PcSave object 0xC17034 whose first dword is the error code.
- If ms_bFailed return 0. `sz = (ms_FilePos + 0xC800 > ms_FileSize) ? ms_FileSize - ms_FilePos : 0xC800` (unsigned compares). If sz == 0
  or `sz & 3` != 0: return 0 WITHOUT setting ms_bFailed or an error code.
- `r = CFileMgr::Read(ms_FileHandle, ms_WorkBuffer, sz)` 0x538950; if `GetErrorReadWrite(ms_FileHandle)` 0x538A50 != 0 or r != sz:
  `PcSaveHelper.error = 5`; if `!CloseFile()` 0x5D0E30 `PcSaveHelper.error = 6`; `ms_bFailed = 1`; return 0.
- Else `ms_FilePos += sz`; `ms_WorkBufferSize = sz`; `ms_WorkBufferPos = 0`; return 1.

## 0x5D1170 CGenericGameStorage::CheckDataNotCorrupt  (catalogue: specified/REA, port level 4)
- `__cdecl bool (int32 slot [esp+4], const char* fileName [esp+8])`, plain `ret`. Globals (CGenericGameStorage statics): ms_WorkBuffer u8* 0xC16EE8, ms_WorkBufferPos u32 0xC16EEC, ms_FileHandle 0xC16EF0, ms_FilePos u32 0xC16EF4, ms_FileSize u32 0xC16EF8, ms_bFailed u8 0xC16EFC, ms_WorkBufferSize u32 0x8D2BE0, ms_CheckSum u32 0xC16134; `PcSaveHelper` = C_PcSave object 0xC17034 whose first dword is the error code.
- 1. `ms_bFailed = 0`; local flag `f = 0` (byte at esp+0x13; never written again); `sum = 0` (u32). If `!OpenFileForReading(fileName,
  slot)` 0x5D0D20 return 0.
- 2. While `LoadWorkBuffer()` 0x5D10B0: `sum += (u8)ms_WorkBuffer[i]` for i = 0 .. ms_WorkBufferSize-1.
- 3. `bad = (PcSaveHelper.error != 0) ? 1 : f`. If `ms_WorkBufferSize < 4` or bad: free ms_WorkBuffer (if non-null; set 0),
  `CFileMgr::CloseFile(ms_FileHandle)`, return 0.
- 4. `stored = *(u32*)(ms_WorkBuffer + ms_WorkBufferSize - 4)`; `sum -= b[-4] + b[-3] + b[-2] + b[-1]` (the 4 checksum bytes, u8 each);
  free ms_WorkBuffer (if non-null; set 0); CloseFile(ms_FileHandle) (result ignored); return `sum == stored` (sete).
- Note: ms_WorkBufferSize after the loop is the size of the last chunk that loaded (LoadWorkBuffer returns 0 at EOF without changing it).

## 0x5D1380 CGenericGameStorage::CheckSlotDataValid  (catalogue: specified/REA, port level 5)
- `__cdecl bool (int32 slot [esp+4])`, plain `ret`, local `char name[0x104]`. `C_PcSave::GenerateGameFilename(&PcSaveHelper 0xC17034,
  slot, name)` 0x6190A0 (thiscall). If `CheckDataNotCorrupt(slot, name)` 0x5D1170: `CStreaming::DeleteAllRwObjects()` 0x4090A0; return 1.
  Else `PcSaveHelper.error (0xC17034) = 7`; return 0. Matches gr GenericGameStorage.cpp (CheckSlotDataValid, unhooked).

## 0x618D00 CGenericGameStorage::GetSavedGameDateAndTime  (catalogue: specified/REA, port level 0)
- `__cdecl GxtChar* (int32 slot [esp+4])`: returns `ms_SlotSaveDate (0xC16138) + 0x46*slot` (no bounds check); plain `ret`.

## 0x618E70 CGenericGameStorage::InitRadioStationPositionList  (catalogue: specified/REA, port level 0)
- Empty body: `ret` (0x618E70).

## 0x618E90 CGenericGameStorage::DoGameSpecificStuffAfterSucessLoad  (catalogue: specified/REA, port level 21)
- `__cdecl void ()`, plain `ret`. Only caller: GenericLoad 0x5D17B0. Calls in this exact order:
  1. `TheText.Load(false)` 0x6A01A0 (this = 0xC1B340).
  2. `CCollision::SortOutCollisionAfterLoad()` 0x411E30.
  3. `CStreaming::LoadSceneCollision(p)` 0x40ED80 and then `CStreaming::LoadScene(p)` 0x40EB70, each with `p = TheCamera.m_matrix
     ([0xB6F03C]) ? matrix + 0x30 : &TheCamera.placement.pos (0xB6F02C)` (re-evaluated for each call).
  4. `CStreaming::LoadAllRequestedModels(false)` 0x40EA10; `CGame::TidyUpMemory(true, false)` 0x53C500.
  5. `JustLoadedDontFadeInYet (0xC16EDC) = 1`; `StillToFadeOut (0xC16EDD) = 1`; `TheCamera.Fade(0.0f, 0)` 0x50AC20 (pushes 0, 0).
  6. `CTheScripts::Process()` 0x46A000; `CTagManager::UpdateNumTagged()` 0x49CDE0; `CClothes::RebuildPlayer(CWorld::Players[0].m_pPed
     ([0xB7CD98]), false)` 0x5A82C0; `CPopCycle::Update()` 0x610BF0; `CStreaming::RemoveInappropriatePedModels()` 0x40B3A0;
     `CGangWars::ClearSpecificZonesToTriggerGangWar()` 0x443FF0.
  7. `CGangWars::bTrainingMission (0x96AB91) = 0`; `CTheZones::FillZonesWithGangColours((u8)CGangWars::bGangWarsActive [0x96AB90])` 0x572440
     (the bGangWarsActive byte is read before the bTrainingMission store).
- gta-reversed: not reversed. Open questions: none.

## 0x618F50 CGenericGameStorage::DoGameSpecificStuffBeforeSave  (catalogue: specified/REA, port level 13)
- `__cdecl void ()`, plain `ret`. Only caller: C_PcSave::SaveSlot 0x619060. In order:
  1. `CRopes::Shutdown()` 0x556B10; `CPickups::RemovePickupObjects()` 0x455470.
  2. `h = (u8)[0xB7CEE7]` (CWorld::Players[0] byte at +0x14F: the player's max health); `h = min(h, 200)` (unsigned compare `jae`);
     `CWorld::Players[0].m_pPed ([0xB7CD98])->m_fHealth (+0x540) = (f32)(i32)h` (fild/fstp).
  3. `CGangWars::EndGangWar(false)` 0x4464C0; `CStats::IncrementStat(0x88, 1.0f)` 0x55C180; `CGameLogic::PassTime(360)` 0x4414C0.
  4. `*(u16*)(0xB7CEDC + 0x190*(u8)CWorld::PlayerInFocus [0xB7CD74]) = 0` (CPlayerInfo +0x144 of the focused player).
  5. `CPlayerPed::ResetSprintEnergy(FindPlayerPed(-1))` 0x60A530; `CPlayerPed::SetWantedLevel(FindPlayerPed(-1), 0)` 0x609F10.
  6. `CGame::TidyUpMemory(true, false)` 0x53C500.
- Note: step 2 writes Players[0]'s ped but step 4 uses PlayerInFocus. gta-reversed: not reversed. Open questions: the gr names of the
  CPlayerInfo fields at +0x14F / +0x144 (behaviour fully given by the offsets).

## 0x619000 CGenericGameStorage::RestoreForStartLoad  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x619000; upper EAX bits undefined), `ret` (0 bytes of stack args, ignored).
