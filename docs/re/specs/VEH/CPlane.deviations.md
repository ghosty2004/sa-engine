# CPlane: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/Plane.cpp`. Layout: see CPlane.md / CHeli.md.

## 0x6C8E20 CPlane::Constructor  (gr_deviations = 1)
Convention: `__thiscall CPlane* (int32 modelIndex [esp+4], eVehicleCreatedBy createdBy [esp+8])`, `ret 8`, EAX = this; SEH frame.

Row Plane.cpp:114 (original-comment: `m_LastHSMissileStruct = 0u; // originally not initialized`).
- Original bytes: the tail (0x6C90C8-0x6C9124) writes +0x9D0 = 0 (m_planeDamageWave), +0x9D4 = 0 (m_pGunParticles), byte +0x9D8 = 0x10
  (m_nFiringMultiplier = 16), +0x9DC = 0, +0x9E0 = 0, +0x9E8/+0x9EC/+0x9F0/+0x9F4 = 0 (jettrus particles), +0x9F8 = 0 (smoke), byte
  +0xA00 = 0 (m_bSmokeEjectorEnabled), and for model 520 the u16 +0x86C = HARRIER_NOZZLE_ROTATE_LIMIT (0x8D33C4). Offset +0x9E4
  (m_LastHSMissileStruct) is never written: it keeps whatever the vehicle pool slot contained.
- Verdict: CONFIRMED (gr zeroes a field the original leaves uninitialised).
- Class: keep-original (do not initialise +0x9E4; a port that must be deterministic should note that the value is the previous pool
  content).
Rest of the constructor checked against gr (all match): CAutomobile ctor(modelIndex, createdBy, true) 0x6B0A90; vtable 0x871948;
`m_nVehicleSubType = 4`; float fields +0x988..+0x994 = 0, +0x998 = 1.0f, +0x9C4/+0x9C8/+0x9CC/+0x9A0/+0x9A4/+0x9B4/+0x9B8/+0x9BC/+0x9C0/
+0x99C = 0, +0x9A8 = 15.0f, +0x9AC = 25.0f, +0x9B0 = 20.0f; if model != 539 (Vortex) physical flag 0x01000000 (bDontCollideWithFlyers);
`m_nExtendedRemovalRange = 255`; bNeverUseSmallerRemovalRange (+0x42C |= 0x40), bIsBig (+0x429 |= 4). Door 2 set-ups via
`SetDoorStatus(2, 0)` and raw writes to m_doors[2] (+0x5E8 open angle, +0x5EC closed angle = 0, u16 +0x5F0 dirn, u8 +0x5F2 axis):
models 520/476/512 and 513: (1.8849558f, dirn 0x13, axis 1); 519: (-2.3561945f, 0x12, 1) + `GetFirstObject(m_aCarNodes[5])->flags = 0`;
553: (-1.2566371f, 0x14, 2); 513 additionally clears the object flags of nodes 7 and 4 (in that order); 539: if
`m_panels[0].frameId == 0xFFFF` then `m_panels[0].SetPanel(21, 1, -0.25f)`. Wheel loop i = 0..3: `m_wheelPosition[i] =
(m_wheelPosition[i] - GetWheelPosn(i, false).z) + GetWheelPosn(i, true).z`.

## 0x6CACB0 CPlane::OpenDoor  (gr_deviations = 1)
Convention: virtual `__thiscall void (CPed* ped, int32 componentId, eDoors door, float ratio, bool playSound)`, `ret 0x14`; SEH frame.

Row Plane.cpp:267 (original-comment: "Unfinished code R*, which removed in Android", gr writes `if (false)`).
- Original bytes: `CAutomobile::OpenDoor(ped, componentId, door, ratio, playSound)` (0x6CACEB); then if `m_nModelIndex == 513` (STUNT)
  AND byte `[0xC1CAFC] != 0` (0x6CACF8): `CMatrix m(&m_aCarNodes[componentId]->modelling, false)`; `y = (m_doors[door].m_angle (+0x5C4 + door*0x18)
  - m_doors[door].m_prevAngle (+0x5C8 + door*0x18)) + m.pos.y`; `m.SetTranslate(m.pos.x, y, m.pos.z)`; `m.UpdateRW()`; `~m`.
  The catalogue lists no writer of 0xC1CAFC (only this read, catalog/rwsets.csv), so the byte is always 0 and the block never runs.
- Verdict: REFUTED (gr's constant `false` is behaviourally identical to the never-written global; the dead block in gr is a faithful
  transcription).
- Class: harmless (a port may keep the global read for fidelity; it has no effect).

## 0x6CCA50 CPlane::CountPlanesAndHelis  (gr_deviations = 1)
Row Plane.cpp:315 (pool-iteration: `for (auto& vehicle : GetVehiclePool()->GetAllValid())`, ascending order).
- Original bytes: `__cdecl uint32 ()`, EAX = count. Reads `CPools::ms_pVehiclePool` (0xB74494); `n = pool->m_nSize` (+8);
  if 0 return 0 (0x6CCA5E). Loops slot `i = n-1 .. 0` (descending, 0x6CCA70-0x6CCA98): skip if `pool->m_byteMap[i]` (+4)
  has bit 7 set (free, `js` 0x6CCA7D) or if `pool->m_pObjects + i*0xA18` is null (only possible for slot 0 with a null
  base, 0x6CCA83); count when `vehicle->m_nVehicleSubType` (+0x594, dword) == 3 (HELI) or == 4 (PLANE) (0x6CCA85-0x6CCA93).
  Element stride 0xA18 (= sizeof(CHeli), the vehicle pool element size).
- Verdict: REFUTED (iteration order differs but the result is a pure count; gr's IsSubHeli/IsSubPlane test the same
  `m_nVehicleSubType` field). Note for the port: the test is on the SUB-type (+0x594), not `m_nVehicleType` (+0x590).
- Class: harmless.
