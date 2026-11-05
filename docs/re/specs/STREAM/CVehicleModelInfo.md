# CVehicleModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable 0x85C5C8 (gr VehicleModelInfo.cpp:24). Size 0x308. Store `CModelInfo::ms_vehicleModelInfoStore`
(0xB1F650, element 0x308, count 0xD4). Fields used: +0x40 f32 m_fWheelSizeFront, +0x44 f32 m_fWheelSizeRear.

## 0x4C5920 CVehicleModelInfo::~CVehicleModelInfo  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this)`; plain `ret`.
- Behaviour: `this->vptr = 0x85BB9C`. Nothing else (the chain ~CVehicleModelInfo -> ~CClumpModelInfo ->
  ~CBaseModelInfo is trivial and reduced to the last vptr store). In particular m_pVehicleStruct is not freed
  here.
- Callers: 0x4C8E10 (scalar deleting dtor), store ctor 0x4C5F50 (EH element dtor), atexit 0x8562A0.
- gta-reversed: implicit/trivial. Open questions: none.

## 0x4C8D60 CVehicleModelInfo::CVehicleStructure::CVehicleStructure  (catalogue: specified/REA, port level 0)
Layout (gr VehicleModelInfo.h:153-170, size 0x314): +0x0 CVector m_avDummyPos[15]; +0xB4 UpgradePosnDesc
m_aUpgrades[18] (0x20 each: CVector pos +0, CQuaternion rot +0xC, i32 m_nParentComponentId +0x1C);
+0x2F4 RpAtomic* m_apExtras[6]; +0x30C u8 m_nNumExtras; +0x310 u32 m_nMaskComponentsDamagable.
- Convention: `__thiscall CVehicleStructure* (this)`; plain `ret`; EAX = this. Preserves EBX, EBP, ESI, EDI.
- Callees: `orig!(0x821D30 eh vector constructor iterator)(this+0xB4, 0x20, 18, 0x4C73F0, 0x4C7400)`
  (0x4C73F0 UpgradePosnDesc ctor = `mov eax,ecx; ret`, 0x4C7400 dtor = `ret`; both no-ops).
- Callers: 1 (CVehicleModelInfo::SetClump 0x4C95C0, after allocating from the CVehicleStructure pool).
- Behaviour (order):
  1. Vector-construct the 18 upgrade descriptors (no field writes).
  2. Three stack temporaries are set to 0 and copied: for i in 0..15: `m_avDummyPos[i] = {0,0,0}` (three u32
     zero stores x, y, z per element, ascending).
  3. for i in 0..18: `m_aUpgrades[i].m_nParentComponentId = -1` (u32 0xFFFFFFFF at this+0xD0+i*0x20).
  4. `memset(this+0x2F4, 0, 24)` (m_apExtras, `rep stosd` x6).
  5. `m_nNumExtras = 0` (byte), `m_nMaskComponentsDamagable = 0`.
  6. Return this.
- Edge cases: upgrade positions/rotations are left uninitialised (pool memory).
- gta-reversed: VehicleModelInfo.cpp:1580, same.
- Open questions: none.

## 0x4C8E10 CVehicleModelInfo::`scalar deleting destructor' (catalogue: CVehicleModelInfo::vfunc_0, specified/REA, port level 1)
- Convention: `__thiscall CVehicleModelInfo* (this, uint32 flags)`; `ret 4`; EAX = this.
- Behaviour: `~CVehicleModelInfo(this)` (0x4C5920); if `(u8)flags & 1`: `operator delete(this)` (0x8214BD);
  return this. Name evidence: slot-0 pattern (CBaseModelInfo.md). Open questions: none.

## 0x6A06F0 CVehicleModelInfo::GetWheelSize  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall float (CVehicleModelInfo* this, bool front)`; front = low byte of [esp+4];
  `ret 4`; returns ST0.
- Callers: 1 (CAutomobile::UpdateWheelMatrix 0x6AA290).
- Behaviour: `if ((u8)front != 0) fld dword [this+0x40] (front size) else fld dword [this+0x44] (rear size)`.
  The f32 is loaded unchanged into ST0 (exact).
- gta-reversed: VehicleModelInfo.h:458 `GetWheelSize(bool front)` inline - same.
- Open questions: none.
