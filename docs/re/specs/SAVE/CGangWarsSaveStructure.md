# CGangWarsSaveStructure (SAVE) - RE-SPEC-REST group B

## 0x5D2620 CGangWarsSaveStructure::Construct  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. Copies CGangWars statics into the structure:
  +0 u8 = bGangWarsActive 0x96AB90; +4 = State 0x96AB64; +8 = TimeStarted 0x96AB60;
  +0xC = index of pZoneInfoToFightOver (0x96AB8C) in CTheZones::ZoneInfoArray (0xBA1DF0, stride 0x11) searched linearly over
  `i < (u16)TotalNumberOfZoneInfos [0xBA1DE8]`; stays -1 if the pointer is null or not found;
  +0x10 = index of pZoneToFightOver (0x96AB88) in NavigationZoneArray (0xBA3798, stride 0x20) over `(u16)TotalNumberOfNavigationZones
  [0xBA3794]`, -1 if null / not found;
  +0x14..+0x1C = CoorsOfPlayerAtStartOfWar 0x96ABBC (3 dwords); +0x20 = Gang1 0x96AB84; +0x24 = Gang2 0x96AB50; +0x28 = WarFerocity
  0x96AB58; +0x2C = LastTimeInArea 0x96AB54; +0x30 = State2 0x96AB4C; +0x34 = TimeTillNextAttack 0x96AB48; +0x38..+0x40 = PointOfAttack
  0x96ABC8 (3 dwords); +0x44 = FightTimer 0x96AB44; +0x48 = RadarBlip 0x96AB98; +0x4C u8 = bPlayerIsCloseby 0x96AB92; +0x50 =
  TerritoryUnderControlPercentage 0x96AB9C; +0x54 = Difficulty 0x96AB80 (all dword copies unless noted).

## 0x5D2740 CGangWarsSaveStructure::Extract  (catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. Exact inverse of Construct 0x5D2620 (same field <-> global map, same order): `pZoneInfoToFightOver
  = (+0xC == -1) ? 0 : 0xBA1DF0 + 0x11*(+0xC)`; `pZoneToFightOver = (+0x10 == -1) ? 0 : 0xBA3798 + (+0x10 << 5)`; no range checks.
