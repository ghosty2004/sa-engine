# tReplay*Block::Make*UpdateData (WORLD)

All three are `__thiscall` members that fill the block at `this` (ECX, the replay buffer slot) — not
functions returning a block by value as gta-reversed writes them. Signature:
`void (this = ECX: block*, CVehicle* veh [esp+4], int32 poolIdx [esp+8])`, `ret 8`, return value
unused (EAX garbage). Only caller of each: `CReplay::RecordThisFrame` 0x45e300.

Shared first steps: `byte[this+0] = type`, **then** `tReplayVehicleBlock::MakeVehicleUpdateData(this, veh,
poolIdx)` (0x45b730, `__thiscall`, ECX = this, `ret 8`). 0x45b730 never writes `this+0` (checked: its
stores start at +1), so the type byte written first survives. The block is not zeroed by anyone here
(gr's `ret{}` zero-init does not exist in the original; padding bytes keep old buffer contents).

`_ftol` = CRT 0x821b40 (ST0 → truncation toward zero, 64-bit, EAX = low dword); only AL is stored.

## 0x45bf90 tReplayBmxBlock::MakeBmxUpdateData  (catalogue: specified/REA, port level 2)
- Type byte `0x0f` (REPLAY_PACKET_BMX). Then MakeVehicleUpdateData.
- `rad = veh->vtable[0xbc/4]()` (`__thiscall`, virtual `CVehicle::GetRideAnimData`, returns
  `CRideAnimData*`); `byte[this+0x34] = (u8) _ftol(rad->LeanAngle (+8) * 50.0f [0x858b40])`.
- The virtual is called **again**: `rad2 = veh->GetRideAnimData()`; `byte[this+0x35] =
  (u8) _ftol(rad2->BarSteerAngle (+4) * 50.0f)`.
- Product is x87 with 24-bit PC; conversion is truncation; negative values wrap to u8 (low byte of the
  int). No null check on the returned pointer.
- gta-reversed: provided (ReplayBlockType.cpp:96) as `MakeBikeUpdateData` + overwrite of type; same
  field values (gr's `(uint8)float` cast is UB for negatives; original = low byte of truncated int).
- Open questions: none.

## 0x45c070 tReplayBikeBlock::MakeBikeUpdateData  (catalogue: specified/REA, port level 2)
- Identical to 0x45bf90 except the type byte is `0x02` (REPLAY_PACKET_BIKE). Same two virtual calls,
  same stores (+0x34 from `LeanAngle*50`, +0x35 from `BarSteerAngle*50`).
- gta-reversed: provided (ReplayBlockType.cpp:85); equivalent apart from the notes above.
- Open questions: none.

## 0x45cd10 tReplayTrainBlock::MakeTrainUpdateData  (catalogue: specified/REA, port level 2)
- Stub `jmp 0x156a610`. Type byte `0x12` (REPLAY_PACKET_TRAIN), then MakeVehicleUpdateData.
- Copies (raw dwords/bytes, in this order): `this+0x34 = veh+0x5a4` (`m_fTrainSpeed`),
  `this+0x38 = veh+0x5a8` (`m_fCurrentRailDistance`), `this+0x3c = veh+0x5ac` (`m_fLength`),
  `byte this+0x48 = byte veh+0x5c0` (`m_nTrackId`), `this+0x40 = 0`, `this+0x44 = 0`.
- `if (p = veh+0x5d0 (m_pPrevCarriage)) this+0x40 = idx(p) + 1`;
  `if (q = veh+0x5d4 (m_pNextCarriage)) this+0x44 = idx(q) + 1`, where
  `idx(p) = (int32)(p - CPools::ms_pVehiclePool->m_pObjects) / 0xa18` with signed truncating division
  (magic 0x06572ec3, `sar 6`, add sign bit). `CPools::ms_pVehiclePool` 0xb74494 is re-read for each.
- gta-reversed: provided (ReplayBlockType.cpp:63), equivalent (`GetIndex + 1`).
- Open questions: none.
