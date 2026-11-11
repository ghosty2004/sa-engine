# VEH: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x6A0750 GetCurrentAtomicObjectCB  (catalogue: specified/REA, port level 0)
- gr Automobile.cpp:6460. RwFrameForAllObjects callback used by CAutomobile::SpawnFlyingComponent, BlowUpCar, ... (9 refs).
- Convention: `__cdecl RwObject* (RwObject* object [esp+4], void* data [esp+8])`, returns `object` (EAX unchanged).
- Behaviour: `if (object->flags /* byte +2 */ & 4 /* rpATOMICRENDER */) *(RwObject**)data = object; return object;` (iteration never stops early). Open questions: none.

## 0x6C4190 CHeli::CHeli  (catalogue: specified/REA, port level 16)
- gr Heli.cpp:26. Convention: `__thiscall CHeli* (this = ECX, int32 modelIndex [esp+4], eVehicleCreatedBy createdBy [esp+8])`, `ret 8`, returns this. SEH frame; the handler-address push is relocated through .HOODLUM stub 0x403C98 (plain code, sub of two obfuscated dwords), resume 0x6C4197. 10 callers (CCarCtrl::GetNewVehicleDependingOnCarModel, CCheat::VehicleCheat, CStoredCar, ...).
- Callees: CAutomobile::Constructor 0x6B0A90 `(modelIndex, createdBy, 1 /*setupSuspensionLines*/)`, CDamageManager::SetDoorStatus 0x6C21C0, CRT rand 0x821B1E.
- Behaviour (store order): after the base ctor: `vtbl = 0x871680`; `+0x594 = 3` (m_nVehicleSubType = HELI); dwords `+0x98C..+0x9A4 = 0` (7 dwords); `+0x9B0, +0x9A8, +0x9AC = 10.0f`; `+0x9B4 = 0`; `byte +0x988 &= 0xFC` (clear two heli flags); `+0xA00 = 0`; `+0x40 (m_nPhysicalFlags dword) |= 0x01000000`.
  - If `modelIndex == 0x1A9` (425 = MODEL_HUNTER): `m_damageManager (+0x5A0).SetDoorStatus(2, 0)`; `+0x5E8 = 0.9424779f` (0x3F71463B); `+0x5EC = 0`; `byte +0x5F2 = 1`; `word +0x5F0 = 0x13` (door 2 open-angle setup).
  - `byte +0x9B9 = 4; dword +0x9BA = 0` (unaligned); `+0x9F0 = CTimer::m_snTimeInMilliseconds`; loop 6 times: `dword [+0x9C0 + 4i] = 0; dword [+0x9D8 + 4i] = 0`; `+0xA04 = 0`; `+0xA08 = CTimer::m_snTimeInMilliseconds`; `byte +0x42C |= 0x40`; `byte +0x3DF = 10`; `+0xA0C = 0`; `byte +0xA10 = 0x10`; `byte +0x9B8 = 0`; `byte +0xA11 = 0`.
  - `+0xA14 = (float)rand() * 3.0518509e-05f [0x858C7C] * 6.0f [0x858B44] + 2.0f [0x858CA0]` (x87 in that order; rand result 0..0x7FFF).
- gta-reversed: gr uses CGeneral::GetRandomNumberInRange for +0xA14 (rng-helper deviation, keep original formula above). Open question: gr names for the +0x98C..+0x9B4 block and the +0x9C0/+0x9D8 arrays (offsets certain).

## 0x6D0450 cTransmission::cTransmission  (catalogue: specified/REA, port level 0)
- gr cTransmission.h:41. Empty ctor (`mov eax, ecx; ret`): writes nothing. Callers __init_gHandlingDataMgr 0x8543D0, 0x6F4FA0. Open questions: none.

## 0x6D0B10 CRideAnimData::CRideAnimData  (catalogue: specified/REA, port level 0)
- gr RideAnimData.h:28. Callers CBike::Constructor 0x6BF430, CQuadBike::Constructor 0x6CE370. Convention: `__thiscall CRideAnimData* (this = ECX)`, returns this.
- Behaviour: seven dword stores of 0 at +0..+0x18 (m_nAnimGroup, m_fSteerAngle, m_fAnimLean, m_dwUnknown, m_fHandlebarsAngle, m_fSitAngle (gr names) ...). gta-reversed: gr marks the ctor NOTSA (deviation-maybe); the bytes zero all 0x1C bytes. Open questions: none.

## 0x6D0B30 CRideAnimData::~CRideAnimData (proposed)  (catalogue: unknown/REA, port level 0)
- Empty function (single `ret`), called from CBike::Destructor and CQuadBike::~CQuadBike (member dtor). Open questions: none.

## 0x6E2D50 CVehicle::operator new (proposed)  (catalogue: specified/REA, port level 1)
- `__cdecl void* (size_t)` (size ignored): `ECX = CPools::ms_pVehiclePool [0xB74494]; jmp CVehiclePool::New 0x6E2A50`. 15 callers (CCarCtrl::GetNewVehicleDependingOnCarModel, ...). Open questions: none.

## 0x6E2D90 CVehicle::operator delete (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void (void* p [esp+4])`, plain `ret`; 23 callers (mostly EH unwind). Behaviour (inline CPool::Delete on ms_pVehiclePool 0xB74494, element size 0xA18): `idx = (p - objs) / 0xA18` (signed magic imul 0x06572EC3, sar edx 6, add sign bit); `map[idx] |= 0x80`; `if (idx < firstFree) firstFree = idx`. Open questions: none.

## 0x6F5370 CTrainNode::CTrainNode  (catalogue: specified/REA, port level 0)
- gr TrainNode.cpp:11. Element ctor used by CTrain::ReadAndInterpretTrackFile 0x6F55D0. `__thiscall CTrainNode* (this = ECX)`, returns this; only writes `byte +9 = 0` (lighting/surface byte; gr `m_nSurfaceLighting`/flag). Open questions: none.

## 0x6F5F80 CTrainNode::GetLightingFromCollision  (catalogue: specified/REA, port level 8)
- gr TrainNode.cpp:61. Convention: `__thiscall u8 (CTrainNode* this = ECX)`, result AL. Callers CTrain::FindPositionOnTrackFromCoors (2), CTrain::ProcessControl (2).
- Fields: +0/+2/+4 int16 x,y,z (fixed point, 1/8 units), +8 u8 cached lighting, +9 u8 lighting-valid flag.
- Constants: `0.125f [0x858C48]`, `1.0f [0x858624]`, immediate -1000.0f (0xC47A0000).
- Behaviour: `if (this->+9) return this->+8;` `pos.x = (float)x * 0.125f; pos.y = (float)y * 0.125f; pos.z = (float)z * 0.125f + 1.0f` (fild of the sign-extended int16, fmul, fadd for z); `hit = CWorld::ProcessVerticalLine(pos, -1000.0f, colPoint, outEntity, 1 /*buildings*/, 0, 0, 0, 0, 0, 0)` (0x5674E0, 11 args cdecl, popped 0x2C); `if (hit) { this->+8 = colPoint.m_nLightingB (+0x25); this->+9 = 1; return it; } return 0x48;` (miss is not cached).
- gta-reversed: matches. Open questions: none.
