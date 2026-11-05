# CStuckCarCheck (VEH) - full specs (writer group E)

Layout (gr `StuckCarCheck.h`): `m_aStuckCars[16]`, stride 0x24: +0x00 CarHandle i32, +0x04 LastPos CVector,
+0x10 LastChecked i32, +0x14 StuckRadius f32, +0x18 CheckTime i32, +0x1C CarStuck, +0x1D WarpCar, +0x1E
WarpIfStuck, +0x1F WarpIfFlipped, +0x20 WarpIfInWater (bools), +0x21 NumberOfNodesToCheck i8.

## 0x463970 CStuckCarCheck::ResetArrayElement  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CStuckCarCheck* this = ECX, uint16 index [esp+4])` (index zero-extended from the
  low word, 0x15660C3), `ret 4`. Stub at 0x463970 -> `.HOODLUM` 0x15660C0.
- Callers: CStuckCarCheck::Process 0x465680.
- Behaviour on `e = &m_aStuckCars[index]` (no bounds check): `e->LastPos = (-5000.0f, -5000.0f, -5000.0f)`
  (0xC59C4000, staged through stack temporaries); `e->CarHandle = -1`; `e->LastChecked = -1` (+0x10 = 0xFFFFFFFF);
  `e->StuckRadius = 0` (+0x14 = 0 bits); `e->CheckTime = 0`; bytes +0x1C..+0x21 = 0.
- gta-reversed: gr's `ResetArrayElement(tStuckCar&)` overload; check that a port sets LastChecked to -1 (not 0).
