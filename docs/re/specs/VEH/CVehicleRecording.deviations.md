# CVehicleRecording: gta-reversed deviation notes (VEH, writer group E)

Source: `re/gta-reversed/source/game_sa/VehicleRecording.cpp` (pin 20d8d2a).

## 0x459D10 CVehicleRecording::SkipForwardInRecording  (gr_deviations = 1)
Row VehicleRecording.cpp:369 (`// NOTSA: Original code does OOB-access if no index found.`).
- Original bytes: `__cdecl void (CVehicle* vehicle, float distance)`. Index search (unrolled by 8) over i = 0..15:
  `bPlaybackGoing[i] (0x97D6F0) && pVehicleForPlayback[i] (0x97D840) == vehicle`; if none, i = 16 and the arrays
  are indexed out of bounds. `d = distance` (x87). `buf = pPlaybackBuffer[i]` (0x97D800), `idx = PlaybackIndex[i]`
  (0x97D7C0, a BYTE offset; frames are 0x20 bytes, position x/y at frame +0x14/+0x18, time u32 at +0).
  Loop (0x459E10): if `d > 0`: stop if `idx >= PlaybackBufferSize[i] (0x97D780) - 0x20` (unsigned); else
  `idx += 0x20` (stored), `d -= sqrt(dx*dx + dy*dy)` (old frame minus new frame), repeat while `d > 0`.
  If `d <= 0` (also on entry with d == 0): stop if `idx <= 0x20`; else `idx -= 0x20`, `d += dist`, repeat while `d < 0`.
  Then `PlaybackRunningTime[i] (0x97D740) = (float)(uint32)frame.time` (fild + 2^32 fix-up); if `bUseCarAI[i]`
  (0x97D6C0): `RestoreInfoForCar(pVehicleForPlayback[i], buf + idx, false)` and `vehicle->vtable[23]
  (ProcessControlCollisionCheck)(false)`.
- Verdict: CONFIRMED (gr returns when not found; the original reads slot 16). Further (untagged) differences: gr's
  algorithm differs from the bytes - it accumulates `pace` toward `distance` in a forward loop and then a separate
  backward loop (stepping back while `pace > distance`), stops the backward walk at `index > 1` (bytes: offset >
  0x20), and with `distance == 0` does not step back one frame as the original does. gr's frame helpers
  (GetFramesFromPlaybackBuffer / GetCurrentFrameIndex) were not checked against the byte-offset arithmetic.
- Class: keep-original (the function has no callers in the catalogue).
