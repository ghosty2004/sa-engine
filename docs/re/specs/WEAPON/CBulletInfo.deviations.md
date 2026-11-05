# CBulletInfo (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/BulletInfo.cpp` (pin 20d8d2a).

## 0x7360D0 CBulletInfo::Update  (gr_deviations = 3)
Local CColPoint `cp` at [esp+0x24] in the hit-handling code (point +0, normal +0x10, surfaceB +0x23, pieceB +0x24, lightingB +0x25).
Rows BulletInfo.cpp:133 ("Originally: if (pieceB < 13u || pieceB > 16u)"), :135 ("originally `if` body"), :148 ("originally
`else` body").
- Original bytes (vehicle hit, 0x7363F2-0x7364FC): skip if the creator is a ped whose `m_pAttachedTo (+0xFC) == vehicle`.
  `piece = cp.pieceB` (byte, `cmp ch, 0xD / 0x10`, unsigned): `13 <= piece <= 16` -> `veh->vtbl[0xB0/4](piece, true)` (BurstTyre)
  and `g_fx.AddTyreBurst(cp.point, cp.normal)` (0x49F300); otherwise `veh->InflictDamage(creator, weaponType, (float)(i16)damage,
  cp.point)` (0x6D7C90), then if `weaponType == 0x25` (flamethrower) `gFireManager.StartFire(veh, creator, 0.8f, 1, 7000, 100)`,
  else if `TheCamera.IsSphereVisible(cp.point, 1.0f)`: `g_fx.AddBulletImpact(cp.point, cp.normal, cp.surfaceB, 8,
  cp.lightingB.GetCurrentLighting(0.5f))`.
- Verdict (all three): REFUTED - gr's switch over pieces 13..16 is the same partition as the original if/else.
  Class: harmless.
- Found in passing (CONFIRMED, gr LOGIC ERROR, keep-original): both `IsSphereVisible` calls in this part (0x7364B9 vehicle branch,
  0x73650B default branch) pass the collision POINT (`lea [esp+0x28]` after one push = cp+0), gr passes `colPoint.m_vecNormal`.
