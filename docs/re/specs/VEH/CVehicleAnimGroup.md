# CVehicleAnimGroup (VEH) - full specs (writer group E)

Layout (gr `VehicleAnimGroupData.h`, size 0x94): +0x00 m_ucFirstGroup u8, +0x01 m_ucSecondGroup u8, +0x02 pad[2],
+0x04 m_animFlags, +0x08 m_specialFlags, +0x0C m_generalTiming (0x14 bytes), +0x20 m_aInOutTiming[2] (0x20 bytes),
+0x40 m_aVecDoorOffsets[7] (CVector, stride 0xC).

## 0x5BD420 CVehicleAnimGroup::CVehicleAnimGroup  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CVehicleAnimGroup* (this = ECX)`, returns `this` in EAX, plain `ret`.
- Callers: cHandlingDataMgr::LoadHandlingData 0x5BD830, `__init_CVehicleAnimGroupData__m_vehicleAnimGroups` 0x853E20.
- Behaviour: bytes +0x00, +0x01 = 0; dwords +0x04, +0x08 = 0; loop 5 times (0x5BD451-0x5BD470): dword +0x0C+4i = 0
  and door offset `[i]` (+0x40 + 0xC*i) = (0, 0, 0) (values from zeroed stack temporaries); loop 4 times
  (0x5BD480-0x5BD489): dwords +0x20+4i and +0x30+4i = 0. Net: all of +0x04..+0x3F and door offsets 0..4 are zero.
  Untouched: +0x02, +0x03 and door offsets 5 and 6 (+0x7C..+0x93).
- gta-reversed: VehicleAnimGroupData.cpp:16; same effect (its "BUG?" note about the last two offsets matches the
  bytes, row deviates=no).
