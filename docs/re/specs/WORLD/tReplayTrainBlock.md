# tReplayTrainBlock (WORLD) - RE-SPEC-FINAL group A

## 0x45CDB0 tReplayTrainBlock::ExtractTrainUpdateData  (catalogue: specified/REA, port level 19)
- Convention: `__thiscall void (tReplayTrainBlock* this, CTrain* train [esp+4], float interpolation [esp+8])`, `ret 8`.
- Callees: tReplayVehicleBlock::ExtractVehicleUpdateData 0x45BA90; CEntity::CleanUpOldReference 0x571A00; CEntity::RegisterReference 0x571B70.
- Globals: CReplay::m_VehiclePoolConversion 0x97F680 (int32[110]) addressed as `[0x97F67C + 4*ref]` (= conversion[ref - 1], the "-1" folded
  into the base); CPools::ms_pVehiclePool [0xB74494].
- Behaviour: `ExtractVehicleUpdateData(this, train, interpolation)`; dword copies `train->+0x5A4 = this->+0x34` (trainSpeed), `+0x5A8 = +0x38`
  (currentRailDistance), `+0x5AC = +0x3C` (length); byte `train->+0x5C0 = this->+0x48` (trackId).
  If `this->+0x40` (prevCarriageRef) != 0: if `train->+0x5D0` != 0 CleanUpOldReference(&train->+0x5D0); `idx = m_VehiclePoolConversion[ref-1]`;
  `v = (flags[idx] & 0x80) ? 0 : objects + idx*0xA18` (inlined GetAt, flag test `test byte,0x80`); `train->+0x5D0 = v`; if v
  `v->RegisterReference(&train->+0x5D0)`. Same for `this->+0x44` (nextCarriageRef) into `train->+0x5D4` (flag test via `cmp byte,0; jns`).
- gta-reversed: ReplayBlockType.cpp:210 - same logic (FindPoolIndexForVehicle(ref - 1) inlined). Open questions: none.
