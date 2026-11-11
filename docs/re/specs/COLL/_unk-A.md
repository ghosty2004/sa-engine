# COLL: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x40f030 CCollisionData::CCollisionData (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall CCollisionData* ()`, EAX = this.
- Writes: word +0,+2,+4 = 0 (sphere/box/triangle counts), byte +6 = 0 (line count), byte +7 `&= 0xf8` (three flag bits cleared, others kept, 0x40f037), dwords +8..+0x2c = 0 (10 pointers/counters).
- gta-reversed: `CCollisionData::CCollisionData` (Collision/CollisionData.cpp:43); gr notes the flag bits; bytes clear exactly bits 0-2 of +7.

## 0x40f700 CColModel::~CColModel (proposed)  (catalogue: specified/REA, port level 4)
- `__thiscall void ()` (this = CColModel*).
- `if (!(this[0x29] & 4)) return` (m_bIsActive, 0x40f706); `if (!m_pColData[0x2c]) return`; if `this[0x29] & 2` (m_bIsSingleColDataAlloc, 0x40f711): `CCollision::RemoveTrianglePlanes(colData)` 0x416400 then `CMemoryMgr::Free(colData)` 0x72f430; else `colData->RemoveCollisionVolumes()` 0x40f070 then CRT `free(colData)` 0x8214bd (the `operator delete`).
- m_pColData is not nulled (destructor). gta-reversed: `CColModel::~CColModel` (Collision/ColModel.cpp:36) with `RemoveCollisionVolumes` inlined.

## 0x40faf0 CBoundingBox::CBoundingBox (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall CBoundingBox* ()`, EAX = this; body 0x1566620.
- Stores 1.0f to +0,+4,+8 and -1.0f to +0xc,+0x10,+0x14 (immediates 0x3f800000 / 0xbf800000 via a stack temp). gta-reversed `CBoundingBox()` (Collision/BoundingBox.h): same inverted-empty box.

## 0x40fb60 CColModel::CColModel (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall CColModel* ()`, EAX = this; body 0x156c690.
- `CBoundingBox::CBoundingBox(this)` 0x40faf0; `byte[+0x28] = 0` (m_nColSlot); `[+0x2c] = 0` (m_pColData); `byte[+0x29] = (byte[+0x29] & 0xfc) | 4` (clear bits 0-1, set m_bIsActive bit 2, 0x156c6a0-0x156c6a7). The bounding sphere (+0x18..+0x24) is not initialised.
- gta-reversed: `CColModel::CColModel` (Collision/ColModel.cpp).

## 0x40fc80 CColPoint::operator= (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CColPoint& (const CColPoint& src)`, EAX = this, `ret 4`; body 0x15637e0.
- Copies m_vecPoint (+0..8), m_vecNormal (+0x10..0x18), surfaceA/pieceA/lightingA (+0x20..0x22), surfaceB/pieceB/lightingB (+0x23..0x25), m_fDepth (+0x28). Padding +0xc, +0x1c, +0x26..0x27 not copied.
- gta-reversed: compiler-generated `CColPoint::operator=` (Collision/ColPoint.h:31).

## 0x411bf0 CColPoint::SetSurfaceTypeA (setter of byte +0x20) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (u8 v)`, `ret 4`; body 0x1563610: `this[0x20] = v`.
- Open question: owner class (0x1563614): +0x20 matches CColPoint m_nSurfaceTypeA, used on the wheel col points in ProcessCarWheelPair.

## 0x47e050 CColPoint::GetSurfaceTypeB (byte +0x23) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall u8 ()`: AL = this[0x23] (upper EAX bytes unchanged) (0x47e050). Open question: owner class (0x47e050).

## 0x4c4c00 CColModel::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 5)
- `__thiscall void* (u32 flags)`, `ret 4`: `CColModel 0x40f700(this)` (destructor); `if (flags & 1) 0x40fc40 ; CColModel::operator delete(this)` (operator delete / free); EAX = this (0x4c4c00).
