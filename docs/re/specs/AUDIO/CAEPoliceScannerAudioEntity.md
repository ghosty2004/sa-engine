# CAEPoliceScannerAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

Statics (gr AEPoliceScannerAudioEntity.h): s_fVolumeOffset f32 0xB61CF8, s_bStoppingScanner u8 0xB61CFC, s_pSound 0xB61D00,
s_nSectionPlaying i16 0xB61D04, s_nAbortPlaybackTime u32 0xB61D08, s_nPlaybackStartTime u32 0xB61D0C, s_pCurrentSlots
tScannerSlot* 0xB61D10, s_SlotState i16[5] 0xB61D14, s_ScannerSlotSecond 0xB61D20, s_ScannerSlotFirst 0xB61D34,
s_pPSControlling 0xB61D48, s_nScannerPlaybackState i16 0xB61D4C, click volume f32 0xB61D54 (.bss, 0.0).
tScannerSlot = {i16 Bank; i16 SoundID} (4 bytes). Bank slots used: 0x21 + i (i = 0..4). Instances are embedded in CWanted
at +0x21C.

## 0x4E6CD0 CAEPoliceScannerAudioEntity::LoadSlots  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl void ()` (static; ECX not used), plain `ret`. Caller: Service 0x4E7630 (tail).
- Callees: CStreaming::IsVeryBusy 0x4076A0, AEAudioHardware.LoadSound 0x4D8ED0 `(u16 bank, u16 sound, i16 slot)`,
  AEAudioHardware.IsSoundLoaded 0x4D8EF0 `(u16 bank, u16 sound, i16 slot)`.
- Behaviour: if `s_pCurrentSlots == 0` return. `ready = 1`. For `i = 0..4` (slot `0x21 + i`, `s = s_pCurrentSlots[i]`):
  - state 0: if `s.Bank < 0 || s.SoundID < 0` -> state = 1. Else: if `!IsVeryBusy()`: `LoadSound(s.Bank, s.SoundID, 0x21+i)`,
    state = 2; in both sub-cases `ready = 0` (0x4E6D74-0x4E6D7A).
  - state 2: if `IsSoundLoaded(s.Bank, s.SoundID, 0x21+i)` state = 3 else `ready = 0`.
  - other states: nothing. (`s_pCurrentSlots` is re-read after every call.)
  If `ready`: `s_nScannerPlaybackState = 4`.
- gta-reversed: unhooked C++ equivalent (`IsActive()` = "bank < 0 || sound < 0"). Open questions: none.

## 0x4E6F60 CAEPoliceScannerAudioEntity::PlayLoadedDialogue  (catalogue: specified/REA, port level 8)
- Convention: `__thiscall void (this = ECX)`, plain `ret`. Local CAESound at [esp+0x1C] (0x74 bytes, EH frame 0x83C078;
  only its +8 is zeroed - inlined default ctor - and it is destroyed with ~CAESound 0x4EF660 at the end). Caller: Service
  (tail, state 5 -> 7), UpdateParameters 0x4E7590.
- Behaviour:
  1. `i` = first index 0..4 with `s_SlotState[i] == 5`; if none, `i = 0`. Then search `j` from `i` to 4 for `s_SlotState[j]
     == 3` (0x4E6FE0).
  2. Found `j`: `vol = (f32)(i8)eventVolumes[0xA4] + s_fVolumeOffset` (fild + fadd, stored f32); `snd.Initialise(0x21 + j,
     s_pCurrentSlots[j].SoundID (u16), this, CVector(0.0f, 1.0f, 0.0f), vol, 1.0f, 1.0f, 1.0f, 0, 0, 0.0f, 0)`;
     `snd.m_ClientVariable = (f32)j`; `snd.m_Flags = 0x107`; `snd.m_Event = 0xA4`; `s_pSound = RequestNewSound(&snd)`; if
     non-null: `s_SlotState[j] = 5`, `s_nScannerPlaybackState = 7`.
  3. Not found (0x4E6FF8): `AudioEngine.ReportFrontendAudioEvent(0x31 SCANNER_CLICK, [0xB61D54] + s_fVolumeOffset, 1.0f)`;
     `ReportFrontendAudioEvent(0x30, 0.0f, 1.0f)`. If `s_nSectionPlaying != 0`: FinishedPlayingScannerDialogue 0x4E6C30.
     Else: `s_pCurrentSlots = &s_ScannerSlotSecond`, `s_nSectionPlaying = 1`; for the 5 slots `state = (bank < 0 || sound < 0)
     ? 1 : 0`; `s_nAbortPlaybackTime = now + [0x8C8148] (5000)`; `s_nPlaybackStartTime = 0`; `s_nScannerPlaybackState = 2`.
- gta-reversed: calls the original via plugin::CallMethod; its dead C++ below omits the abort-time store and the 0xB61D54 term.
  Open questions: none.

## 0x4E71E0 CAEPoliceScannerAudioEntity::AddAudioEvent  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void (this = ECX; i32 event [esp+4], i32 crime [esp+8], const CVector* point [esp+0xC])`, `ret 0xC`.
- Callees: Rnd 0x4D9C10, CTheZones::FindSmallestZoneForPosition 0x572360 `(CVector*, bool)`, CWorld::FindPlayerWanted 0x56E230
  `(i32)`, CWorld::FindPlayerPed 0x56E210 `(i32)`, CVehicle::GetRemapIndex 0x6D0B70, CPedIntelligence::GetTaskSwim 0x601070,
  GetTaskJetPack 0x601110, PlayPoliceScannerDialogue 0x4E6ED0 (thiscall `(tScannerSlot* first, tScannerSlot* second)`).
- Locals: `A[5]` (first, at [B+0x24]) and `S[5]` (second, at [B+0x10]) tScannerSlot arrays, every i16 initialised to -1;
  `ped = 0`.
- Behaviour:
  1. If `event != 0xA4` return. If `crime <= 1 || crime >= 0x17` return.
  2. `A[0] = {0x96, T160[Rnd(0, 3)]}` (i16 table 0x8C8160 = {0, 5, 8, 0}; A[0].Bank is written before the Rnd call);
     `A[1] = {0x97, T168[crime]}` (i16 table 0x8C8168, indexed by crime: {-1, -1, 0, 7, 5, 8, 2, 1, 1, 1, 0, 8, 10, 3, 8, 3, 10,
     4, 5, 8, 9, 6, 6}).
  3. `z = FindSmallestZoneForPosition(point, 1)`; null -> return. Find `k` in 0..0xC1 with zone label (z+8, z+0xC as two
     dwords) equal to the 8-byte name at `0x8C8198 + 8k` and `(i16)0x8C87A8[k] >= 0`; none -> return.
  4. If `(u8)0x8C8930[k] != 0` (direction words), with zone rect i16 x1 (+0x10), y1 (+0x12), x2 (+0x16), y2 (+0x18):
     `w = x2 - x1`, `h = y2 - y1` (ints, fild), `cx = w * 0.5f + x1`, `cy = h * 0.5f + y1` (fiadd; stored f32), `qw = w * 0.25f`
     (stored), `qh = h * 0.25f` (kept in ST0).
     - if `point.y > cy + qh`: `A[2] = {0x95, 2}`, `dirY = 1`; else if `point.y < cy - qh`: `A[2] = {0x95, 3}`, `dirY = 1`.
     - if `qw + cx < point.x`: `A[3] = {0x95, 1}`; else if `cx - qw > point.x`: `A[3] = {0x95, 4}`; else if `!dirY`: `A[3] =
       {0x95, 0}`.
  5. `A[4] = {0x93, 0x8C87A8[k]}`.
  6. `ped` = `FindPlayerPed(p)` for the `p` in {0, 1} whose `FindPlayerWanted(p) + 0x21C == this` (last match wins).
  7. If `ped`: if `ped->+0x46C & 0x100` (in vehicle): `veh = ped->+0x58C`; if `veh` and `c = (i8)veh->+0x1D4` in 0..0x2D and
     `(i16)0x8C89F8[c] >= 0`: `S[0] = {0x96, 7}`, `S[1] = {0x96, (c in {0x19, 0x1D, 0x1E, 0x22, 0x25, 0x29}) ? 3 : 1}`; if
     `(u8)0x8C8A54[c] != 0`: if `veh->GetRemapIndex() != -1` -> `S[3] = {0x94, 4}`; else `col = (u8)veh->+0x434`, if `0 < col <
     0x7F` and `(i16)0x8C8A88[col] >= 0` -> `S[3] = {0x94, 0x8C8A88[col]}`. Then `S[4] = {0x98, 0x8C89F8[c]}`.
     On foot: if `GetTaskSwim()` -> `S[4].SoundID = 2`; else if `GetTaskJetPack()` -> nothing; else `S[4].SoundID = 4`; in the
     swim / default cases `S[0] = {0x96, 7}` and `S[4].Bank = 0x96`.
  8. `PlayPoliceScannerDialogue(A, S)` (also reached when ped/vehicle checks fail, with S partially -1).
- gta-reversed: `plugin::CallMethod` only. Open questions: none (data tables at the listed addresses are read verbatim).

## 0x4E7630 CAEPoliceScannerAudioEntity::Service  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl void ()` (static), plain `ret` or tail jumps. Caller: (game loop via CWanted / AudioEngine).
- Behaviour: `stop = (TheCamera.m_bWideScreenOn (0xB6F065) && s_nScannerPlaybackState != 0) ? (s_bStoppingScanner = 1, 1)
  : s_bStoppingScanner`. Switch on `(i16)s_nScannerPlaybackState` (table 0x4E774C, > 7 -> return):
  - 2: if `now > s_nAbortPlaybackTime` (unsigned) or `stop` -> tail FinishedPlayingScannerDialogue 0x4E6C30; else tail LoadSlots.
  - 4: if `stop` -> Finished...; if `now < s_nPlaybackStartTime` (unsigned) return; `s_fVolumeOffset = (*(void**)0xB6B98C != 0)
    ? 0.0f : [0x8C815C] (-8.0f)`; `ReportFrontendAudioEvent(0x31, [0xB61D54] + s_fVolumeOffset, 1.0f)`;
    `ReportFrontendAudioEvent(0x2F NOISE_START, [0x8C8158] (-6.0f) + s_fVolumeOffset, 1.0f)`; `s_nPlaybackStartTime = now +
    [0x8C8154] (300)`; `s_nScannerPlaybackState = 5`.
  - 5: if `stop` -> Finished...; if `now >= s_nPlaybackStartTime` and `s_pPSControlling != 0`: `s_nScannerPlaybackState = 7`,
    tail `s_pPSControlling->PlayLoadedDialogue()`.
  - 0, 1, 3, 6, 7: nothing.
- gta-reversed: unhooked C++ equivalent. Open questions: none.
