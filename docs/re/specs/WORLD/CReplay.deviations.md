# CReplay: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x45c210 CReplay::Display  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Replay.cpp:252` (notsa): "NOTSA: FPS-independent 'REPLAY' text counter": gr toggles the text every 1000 ms of `CTimer::GetTimeInMS()`.
  - Original bytes: `if (CReplay::Mode (0xa43088) == 0) return` (0x45c211-0x45c218); frame counter at 0xa43240: `c = (c + 1) & 0xFFFF; store` (0x45c21e-0x45c22b); draw only if `c & 0x20` (0x45c229-0x45c230), i.e. 32 frames on, 32 off. Font setup: `SetScale(width * 0.0015625f [0x859520] * 1.5f, height * 0.0022321429f [0x859524] * 1.5f)` (RsGlobal width 0xc17044, height 0xc17048, `fild`), `SetOrientation(1)`, `SetBackground(0, 0)`, `SetCentreSize((float)(width - 20))`, `SetProportional(1)`, `SetColor(CRGBA(255,255,200,200))`, `SetFontStyle(1)`; then only if `Mode == 1`: `PrintString((float)(width / 10), (float)(height / 15), TheText.Get("REPLAY"))` (integer divisions via magic multiplies, 0x45c2e5-0x45c330).
  - Verdict: CONFIRMED. The original blinks on a per-call frame counter (frame-rate dependent); gr uses wall time. Also the original's entry test is `Mode != 0` (gr: `ShouldStandardCameraBeProcessed()`), and the font state is set even when `Mode != 1` (only the print is skipped).
  - Class: keep-original.

## 0x45d390 CReplay::EmptyPedsAndVehiclePools_NoDestructors  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Replay.cpp:298` (pool-iteration): vehicles, `CWorld::Remove(&veh)`.
- gr `Replay.cpp:302` (pool-iteration): peds, `CWorld::Remove(&ped)` then `g_realTimeShadowMan.ReturnRealTimeShadow(ped.m_pShadowData)` if non-null.
  - Original bytes: vehicle pool (0xb74494) descending (0x45d3b0-0x45d3d3), `CWorld::Remove` (0x563280); then ped pool (0xb74490) descending (0x45d3e4-0x45d41d), `CWorld::Remove`, then `shadow = [ped+0x134]`; if non-null `CRealTimeShadowManager::ReturnRealTimeShadow(&g_realTimeShadowMan 0xc40350, shadow)` (0x705b30). Vehicles before peds, as in gr.
  - Verdict (both): CONFIRMED order differs (descending in the original). The removals are per entity, but the order in which sector-list nodes and real-time shadows are returned to their free lists differs, which can change which node/shadow slot is handed out next.
  - Class: keep-original (iterate slots from size-1 down to 0).

## 0x45d430 CReplay::MarkEverythingAsNew  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Replay.cpp:275` (pool-iteration): `for (auto& ped : GetPedPool()->GetAllValid()) ped.bHasAlreadyBeenRecorded = false;`
- gr `Replay.cpp:279` (pool-iteration): same for vehicles, `veh.vehicleFlags.bHasAlreadyBeenRecorded = false`.
  - Original bytes: ped pool pointer from an obfuscated load (stub 0x4058d5, `rol` decode, EDX = pool). Ped loop: `eax = size`, decrement-first, descending to 0 (0x45d447-0x45d46b), skip free (`js`, 0x45d457) and null address (0x45d45d), `and dword [ped+0x470], 0xffffdfff` (0x45d45f). Vehicle pool 0xb74494: descending (0x45d483-0x45d4a2), skip `flags & 0x80` (0x45d48d), `and byte [veh+0x42b], 0xfb` (0x45d499). Peds first, then vehicles (same as gr).
  - Verdict (both rows): CONFIRMED order differs (descending), but each iteration only clears a bit on its own entity. No observable difference.
  - Class: harmless.

## 0x45d540 CReplay::CreatePlayerPed  (gr_deviations = 1)
Rows checked: 2 (CONFIRMED 0, REFUTED 1, UNCLEAR 1).
- gr `source/game_sa/Replay.cpp:1483` (assert, deviates=no): `assert(next.type == REPLAY_PACKET_CLOTHES)`. REFUTED (debug only; the original passes `&packet + 8` to DealWithNewPedPacket without a check, 0x45d5ef). Class: harmless.
- gr `Replay.cpp:1490` (notsa): `CAddressInReplayBuffer address(buffer, i, offset); // m_bSlot definition (i) is NOTSA`.
  - Original bytes: the address struct is built on the stack with only two fields written: `[esp+0x14] = offset (ESI)` and `[esp+0x18] = buffer pointer (EDI)` (0x45d5dd-0x45d5e1); the third field (m_bSlot, `[esp+0x1c]`) is never written in this function, so `CReplay::ProcessPedUpdate(ped, 1.0f, &address)` (0x45ca70) receives whatever is on the stack there.
  - Verdict: CONFIRMED that gr invents a value (and its `i` is the loop counter, not the buffer slot). UNCLEAR whether it matters: open question whether ProcessPedUpdate (0x45ca70) reads m_bSlot (address 0x45d646).
  - Class: keep-original (do not depend on the slot field here).
- Found in passing (keep-original, CONFIRMED): buffer loop. The original runs `i = 0..7` (`cmp ecx,8; jl`, 0x45d624-0x45d62c), slot = `(Playback.m_bSlot (byte 0x97fb6c) + i) % 8` (signed-remainder idiom), and when `BufferStatus[slot] == 0` (0x45d57d) or the buffer's first packet type is 0 (END, 0x45d596) it SKIPS to the next i. gr (Replay.cpp:1470-1473) stops the whole search at the first unavailable buffer or END-first buffer. Packet walk: type 3 (ped header) with no player yet -> `player = DealWithNewPedPacket(packet, 1, packet + 8)` (0x45cea0); type 4 (ped update) with a player: `ped = (pool flags[conv[ref]] & 0x80) ? null : pool + conv[ref]*0x7c4` (`m_PedPoolConversion` 0x97f838, ref = byte +1); if `ped == player` -> ProcessPedUpdate and return player; advance by `FindSizeOfPacket(type)` (0x45c850) until a type-0 byte. Returns null if not found.

## 0x45e300 CReplay::RecordThisFrame  (gr_deviations = 4)
Rows checked: 4 (CONFIRMED 0, REFUTED 4, UNCLEAR 0).
- gr `source/game_sa/Replay.cpp:834` (pool-iteration): vehicle loop that sums packet sizes.
- gr `Replay.cpp:861` (pool-iteration): ped loop that sums packet sizes.
- gr `Replay.cpp:905` (pool-iteration): vehicle loop that writes vehicle packets.
- gr `Replay.cpp:951` (pool-iteration): ped loop that writes ped header/update packets.
  - Original bytes: all four loops are ASCENDING slot loops with a signed `jle` guard on the size: size pass vehicles `ecx = 0..size-1` (`inc ecx; cmp ecx,edx; jl`, 0x45e36a-0x45e373), size pass peds (`inc edi`, 0x45e3b6-0x45e3bf), write pass vehicles (`jl 0x45e5a0`, 0x45e6d1-0x45e6de), write pass peds (`inc ebx; jl 0x45e700`, 0x45e7c9-0x45e7d2). Free slots are skipped by the flag byte sign (`js` / `test 0x80`) and null addresses, and entities with no RwObject (`[e+0x18] == 0`) are skipped, as in gr. The size pass starts at 0x74 (116) (0x45e322).
  - Verdict (all four): REFUTED. The original also iterates in ascending slot order.
  - Class: harmless.

## 0x45ecd0 CReplay::RestoreStuffFromMem  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 1, REFUTED 2, UNCLEAR 0).
- gr `source/game_sa/Replay.cpp:737` (pool-iteration): ped loop (delete replay peds, re-add the others).
- gr `Replay.cpp:749` (pool-iteration): vehicle loop (same).
  - Original bytes: both loops are ASCENDING (`inc edi; ... jl 0x45ece6`, 0x45ed34-0x45ed3d, ped pool 0xb74490; `inc edi; ... jl 0x45ed51`, 0x45ed8b-0x45ed94, vehicle pool 0xb74494), with the pool pointer reloaded every iteration (0x45ed2c, 0x45ed83).
  - Verdict (both): REFUTED (ascending in the original too). Class: harmless.
- gr `Replay.cpp:791` (fix-bugs-comment): "FIX_BUGS: ms_nNumGang not restored"; gr copies `ms_nNumGang_Stored` back into `CPopulation::ms_nNumGang`.
  - Original bytes: the tail restores only ms_nNumCivMale (0xc0ec70), ms_nNumCivFemale (0xc0ec6c), ms_nNumCop (0xc0ec68), ms_nNumEmergency (0xc0ec64), ms_nTotalCarPassengerPeds (0xc0ec34), ms_nTotalCivPeds (0xc0ec30), ms_nTotalGangPeds (0xc0ec2c), ms_nTotalPeds (0xc0ec28), ms_nTotalMissionPeds (0xc0ec24) from their `_Stored` copies (0x45eea5-0x45ef14), then returns (0x45ef1a). No ms_nNumGang copy.
  - Verdict: CONFIRMED. The original leaves the per-gang counters as they were after playback.
  - Class: keep-original.

## 0x45f380 CReplay::PlayBackThisFrameInterpolation  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Replay.cpp:1111` (notsa-unreachable): `default: NOTSA_UNREACHABLE("Unknown vehicle subtype ...")` in the vehicle-creation switch of the REPLAY_PACKET_VEHICLE case.
  - Original bytes: `switch (packet.vehicleSubType (byte +0x30))` through the table at 0x4600bc (0x45f414-0x45f421): 0 -> CAutomobile (0x45f4a0), 1 -> CMonsterTruck (0x45f548), 2 -> CQuadBike (0x45f513), 5 -> CBoat (0x45f464), 6 -> CTrain (0x45f428), 11 -> CTrailer (0x45f4de); every case ends with `mov ebp, eax` (0x45f583, EBP = new vehicle) and falls into the shared setup code at 0x45f585. Subtypes 3, 4, 7, 8, 9, 10 and any value > 11 jump directly to 0x45f585, skipping `mov ebp, eax`, so the setup code (status nibble `(b & 7) | 8` on +0x36, matrix decompression, flags at +0x1c/+0x42f, ...) runs on whatever EBP holds: the vehicle created by an earlier VEHICLE packet in this call, or the value loaded at 0x45f3a5 (`[esp+0x6c]`, the function argument).
  - Verdict: CONFIRMED (no unreachable path; the original continues with a stale pointer). UNCLEAR what that pointer is in practice: open question at 0x45f585 (depends on earlier packets of the same call). Class: keep-original (only reachable with corrupt replay data).
- gr `Replay.cpp:1302` (notsa-unreachable): `default: NOTSA_UNREACHABLE("Invalid packet read ...")` in the packet-type switch.
  - Original bytes: `t = byte [packet]`; `t == 8` (end of frame) handled first (0x45f3ba); `if (t > 0x13) goto 0x45f3b0` (`ja`, 0x45f3c8), and the table at 0x46006c also sends type 11 (REPLAY_PACKET_PARTICLE) to 0x45f3b0. 0x45f3b0 is the loop head that re-reads the SAME packet (`[ebx]` base + `[ebx+4]` offset, unchanged), so an unknown type (or type 11) makes the original loop forever.
  - Verdict: CONFIRMED (the original hangs instead of aborting). Class: keep-original (only with corrupt data; type 11 is never written).
