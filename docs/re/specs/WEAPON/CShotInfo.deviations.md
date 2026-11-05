# CShotInfo (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/ShotInfo.cpp` (pin 20d8d2a). aShotInfos at 0xC89690, 100 x 0x2C.

## 0x739E60 CShotInfo::Update  (gr_deviations = 1)
Original: `__cdecl void ()`, aligned frame (`and esp,-8`). Loop over `aShotInfos` (esi = &shot+0x18 starting 0xC896A8; fields
relative to esi: type -0x18, origin -0x14, offset -8, range +4, creator +8, destroyTime +0xC, exist +0x10, executed +0x11).
Row ShotInfo.cpp:111 ("originally OOBs ..., which ends up with the closest ped member").
- Original bytes (0x739F8C-0x73A0D3): `ebx = creator->m_pIntelligence (+0x47C) + 0x130`; `for (n = 16; n > 0; ) { p = ebx[n]; n--; ... }`
  i.e. reads the dwords at +0x130 + 4*16 down to +0x130 + 4*1 (entry 0 is never read). With gr's layout (CPedScanner
  m_apEntities[16] at +0x130, m_pClosestEntityInRange right after) n = 16 is the closest-ped field.
- Verdict: REFUTED - gr deliberately reproduces the same sequence (closest ped first, then entries 15..1, entry 0 skipped).
  Class: harmless (keep the original index sequence 16..1).
Found in passing (CONFIRMED, keep-original):
- Range growth (`flags & 0x80000`): `range += ms_fTimeStep * 0.075f` (f32 0x8714A8 = 0.07500000298, 0x739F19). gr uses `0.0075f`
  (ten times smaller) - gr constant error.
- Slow-down (`flags & 0x10000`): `k = pow(0.96 (f64 0x86F0E0, = (double)0.96f), (double)ms_fTimeStep)` via CRT pow 0x822130,
  then each offset component `*= k` with k kept in x87 (not rounded to f32 first); gr uses `pow(0.96f, ts)` in float.
- Per candidate ped: skip null, `!IsPointerValid()`, `+0x46C & 0x100` (in vehicle), `+0x40 & 0x80000` (bFireProof),
  `!IsPedInControl()`; then `((dz*dz + dy*dy) + dx*dx) < max(range, 1.0f)` (strict; d = ped pos - origin). Non spray/extinguisher
  (type != 0x29/0x2A): `gFireManager.StartFire(ped, creator, 0.8f, 1, 7000, 100)` (0x53A050) and continue; else if
  `!executed`: `GenerateDamageEvent(ped, creator, type, (i16)wi->+0x22, 3, ped->GetLocalDirection(creatorPos - pedPos (x,y)))`,
  `executed = 1`. These match gr.
