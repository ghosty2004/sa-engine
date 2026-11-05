# CColDisk (VEH) - full specs (writer group E)

Layout (gr `Collision/ColDisk.h`, size 0x24): CColSphere base (+0x00 centre CVector, +0x0C radius f32, +0x10
CColSurface: +0x10 material u8, +0x11 piece u8, +0x12 lighting u8, +0x13 unused), +0x14 `m_vThickness` CVector
(used as the disk end point), +0x20 `m_fThickness` f32.

## 0x40FD50 CColDisk::Set  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CColDisk* this = ECX, float radius [esp+4], const CVector* start [esp+8],
  const CVector* end [esp+0xC], float thickness [esp+0x10], uint8 material [esp+0x14], uint8 piece [esp+0x18],
  uint8 lighting [esp+0x1C])`, `ret 0x1C`. 5-byte stub at 0x40FD50 -> body in `.HOODLUM` 0x156CE20.
- Callers: CMonsterTruck::SetupSuspensionLines 0x6C7FB0. Callees: none.
- Behaviour (store order as in the bytes): `+0x0C = radius`; `+0x00..+0x08 = *start` (3 dwords);
  `+0x14..+0x1C = *end`; `+0x20 = thickness`; `+0x10 = material`; `+0x11 = piece`; `+0x12 = lighting`
  (byte stores of the low byte of each dword argument). Returns nothing meaningful (EAX = garbage).
- gta-reversed: only a commented-out `Set` in ColDisk.h with the same parameter order (names startRadius /
  endRadius for what the layout calls radius / thickness).
