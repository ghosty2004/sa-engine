# WORLD: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x54F1E0 CPlaceable::CPlaceable (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: called first in CEntity::Constructor 0x532A90 and CCamera::Constructor 0x51A450; stores vtable 0x863C40 (CPlaceable vtable; the same constant is restored in the dtor 0x54F490) and zeroes the CSimpleTransform and matrix pointer (gr Placeable.cpp `CPlaceable::CPlaceable() : m_matrix(nullptr) { m_placement = {}; }`).
- Convention: `__thiscall CPlaceable* (this = ECX)`, returns this, plain `ret`.
- Behaviour: `+0 = 0x863C40; +4,+8,+0xC (m_placement.m_vPosn), +0x10 (m_fHeading), +0x14 (m_matrix) = 0` (dword stores in that order).
- gta-reversed: CPlaceable ctor (Placeable.cpp) equivalent. Open questions: none.

## 0x54F490 CPlaceable::~CPlaceable (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: called at the end of CEntity::Destructor 0x535E90 and CCamera::~CCamera 0x50A870, and as the EH unwind of CCamera::Constructor; restores vtable 0x863C40 (see ctor 0x54F1E0).
- Convention: `__thiscall void (this = ECX)`, plain `ret`.
- Callees: CPlaceable::RemoveMatrix 0x54F3B0 (thiscall on this). Globals: numMatrices 0xB74238 (int32, decremented), gDummyMatrix 0xB74240 (address stored).
- Behaviour: `vtbl = 0x863C40` (stored at 0x54F498, after reading m_matrix); `if (this->m_matrix /* +0x14 */) { RemoveMatrix(); numMatrices--; }` then unconditionally `m_matrix = &gDummyMatrix`.
- gta-reversed: matches gr `CPlaceable::~CPlaceable()` (Placeable.cpp). Open questions: none.

## 0x54F5D0 CPlaceable::`scalar deleting destructor' (proposed)  (catalogue: specified/REA, port level 1)
- Name evidence: slot 0 of the CPlaceable vtable 0x863C40; body = inlined ~CPlaceable (0x54F490) plus the delete-flag test. No direct callers.
- Convention: `__thiscall CPlaceable* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `vtbl = 0x863C40; if (m_matrix) { RemoveMatrix(); numMatrices--; } m_matrix = &gDummyMatrix (0xB74240); if (flags & 1) free(this) (0x8214BD); return this;` (m_matrix store happens after the flag test instruction but before the branch: always executed). Open questions: none.

## 0x563180 CPtrListDoubleLink::CPtrListDoubleLink (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: element ctor handed to `eh_vector_constructor_iterator` by CRepeatSector ctor 0x563190 (element size 4, count 3) together with dtor 0x5630F0 (which tail-jumps to CPtrListDoubleLink::Flush 0x552470). Same bytes as CSprite2d::Constructor 0x727230 (a 4-byte zeroing ctor).
- Convention: `__thiscall CPtrListDoubleLink* (this = ECX)`, returns this. Behaviour: `this->m_node = nullptr` (dword at +0). Open questions: none.

## 0x563190 CRepeatSector::CRepeatSector (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only reference __init_CWorld__ms_aRepeatSectors 0x84E9E0 (element ctor of CWorld::ms_aRepeatSectors); gr RepeatSector.h: `CPtrListDoubleLink<CPhysical*> m_lists[3]`.
- Convention: `__thiscall CRepeatSector* (this = ECX)`, returns this, plain `ret`.
- Behaviour: `eh_vector_constructor_iterator(this, 4, 3, 0x563180 /*list ctor*/, 0x5630F0 /*list dtor*/)` (CRT 0x821D30, cdecl, callee-cleaned by the CRT helper: no `add esp` follows). Open questions: none.

## 0x564040 CSector::CSector (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only reference __init_CWorld__ms_aSectors 0x84E9B0; gr Sector.h: CSector = two CPtrListDoubleLink (m_buildings, m_dummies), 8 bytes. Same bytes as CShadowCamera::Constructor 0x7053F0.
- Convention: `__thiscall CSector* (this = ECX)`, returns this. Behaviour: dword +0 = 0, dword +4 = 0. Open questions: none.

## 0x590DE0 FurnitureGroup_c::FurnitureGroup_c  (catalogue: specified/REA, port level 1)
- gr FurnitureGroup_c.h:14 (`FurnitureGroup_c() = default; // 0x590DE0`). Element ctor (size 0xC, count 9) passed by FurnitureManager_c ctor 0x591260.
- Convention: `__thiscall FurnitureGroup_c* (this = ECX)`, returns this. Behaviour: calls 0x4A8DD0 (List_c::List_c, uncatalogued name: the list-header ctor, on `this+0`), returns this. Open questions: 0x4A8DD0 body not re-specified here (List_c ctor; see its own entry).

## 0x590E00 FurnitureItem::FurnitureItem (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: element ctor (size 0x10, count 0x200) with dtor 0x590E10 passed by FurnitureManager_c ctor 0x591260 for `this+0x6C` = gr `std::array<FurnitureItem, 512> m_FurnitureItem` (FurnitureManager_c.h:35). Body is only the ListItem_c base ctor; byte-identical to InteriorGroup_c::Constructor 0x597FE0 and Furniture_c ctor 0x590EC0.
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `ListItem_c::ListItem_c(this)` (0x4A8DB0); return this. Open questions: none.

## 0x590E20 FurnitureSubGroup_c::FurnitureSubGroup_c  (catalogue: specified/REA, port level 1)
- gr FurnitureSubGroup_c.h:13 (`// 0x590E20`). Element ctor referenced by __init_FurnitureManager_c__g_subGroupStore 0x84F320.
- Convention: `__thiscall FurnitureSubGroup_c* (this = ECX)`, returns this. SEH frame (handler 0x83CE88; state 0 after the base ctor, unwind calls 0x4A8DC0 = ListItem_c dtor on this).
- Behaviour: `ListItem_c::ListItem_c(this)` (0x4A8DB0); `List_c::List_c(this+0xC)` (0x4A8DD0); return this. No other fields written. Open questions: none.

## 0x590EC0 Furniture_c::Furniture_c  (catalogue: specified/REA, port level 1)
- gr Furniture_c.h:21 (`// 0x590EC0`). Referenced by __init_g_bae1f8 (static Furniture_c array element ctor).
- Convention: `__thiscall Furniture_c* (this = ECX)`, returns this. Behaviour: `ListItem_c::ListItem_c(this)` (0x4A8DB0); return this. Open questions: none.

## 0x591260 FurnitureManager_c::FurnitureManager_c  (catalogue: specified/REA, port level 1)
- gr FurnitureManager_c.h:20 (`// 0x591260`). Only caller __init_g_furnitureMan 0x84F300.
- Convention: `__thiscall FurnitureManager_c* (this = ECX)`, returns this; SEH frame (handler 0x83CEEC; unwind = eh_vector_destructor_iterator 0x821E02).
- Behaviour: `eh_vector_constructor_iterator(this+0, 0xC, 9, FurnitureGroup_c ctor 0x590DE0, dtor 0x590DF0)` (m_Groups[9]); state 0; `eh_vector_constructor_iterator(this+0x6C, 0x10, 0x200, 0x590E00, 0x590E10)` (m_FurnitureItem[512]); state 1; `List_c::List_c(this+0x206C)` (m_FurnitureList, 0x4A8DD0); return this.
- Open questions: none.

## 0x59E620 BreakManager_c::BreakManager_c (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_g_breakMan 0x84FA80; constructs 64 BreakObject_c (gr BreakManager_c.h `BreakObject_c m_aObjects[64]`).
- Convention: `__thiscall BreakManager_c* (this = ECX)`, returns this. Behaviour: `eh_vector_constructor_iterator(this, 0x20, 0x40, BreakObject_c::BreakObject_c 0x59D170, 0x59D180 /*empty dtor*/)`; return this.
- Open question: element size 0x20 is what the bytes pass (0x59E62F); check against gr's BreakObject_c size when porting.

## 0x59F660 CObject::~CObject (proposed)  (catalogue: specified/REA, port level 14)
- gr Object.cpp:117 (`// 0x59F660`). Convention: `__thiscall void (CObject* this = ECX)`; SEH frame handler 0x83D228 (state 0 = CPhysical part). Callers: CObject scalar deleting dtor 0x5A2BD0, CProjectile dtor 0x5A40E0 (tail), CHandObject dtor 0x59F000 (tail), CCutsceneObject ctor (unwind), ...
- Behaviour, in order:
  1. `vtbl = 0x866F60`.
  2. If `m_nObjectFlags (dword +0x140) & 0x300000` (script-brain flags): `CStreaming::SetMissionDoesntRequireModel(CTheScripts::ScriptsForBrains[m_wScriptTriggerIndex (int16 +0x168)].m_nIMGindex (int16 at 0xA90CF0 + idx*20) + 0x6676)` (0x409C90); clear those two bits; `CTheScripts::RemoveFromWaitingForScriptBrainArray(this, (u16)+0x168)` (0x46ABC0); `+0x168 = -1`.
  3. If `byte +0x142 & 1`: `CColStore::RemoveRef(CModelInfo::ms_modelInfoPtrs[m_nModelIndex (+0x22)]->m_pColModel (+0x14)->m_nColSlot (byte +0x28))` (0x4107D0).
  4. `CRadar::ClearBlipForEntity(3 /*BLIP_OBJECT*/, handle)` (0x587C60) with `handle = (idx << 8) + flagsByte`, `idx = (this - pool->m_pObjects) / 0x19C` (magic divide on CPools::ms_pObjectPool 0xB7449C).
  5. If `int16 +0x14A != -1`: `ms_modelInfoPtrs[+0x14A]->RemoveRef()` (0x4C4BB0). If `int16 +0x16A != -1`: `CTxdStore::RemoveRef(+0x16A)` (0x731A30).
  6. If `m_nObjectType (byte +0x13C) == 3` and `CObject::nNoTempObjects (u16 0xBB4A70) > 0` (unsigned): `nNoTempObjects--`.
  7. `RemoveFromControlCodeList()` (0x59F450).
  8. If `m_nModelIndex == MI_TRAINCROSSING1 (u16 0x8CD6B8)`: `e = *(CEntity**)(this+0x170)`; `p = e->m_matrix (+0x14) ? &e->m_matrix->pos (+0x30) : &e->m_placement.pos (+4)` (e is dereferenced without a null check); `ThePaths.SetLinksBridgeLights(p.x - 12.0f, p.x + 12.0f, p.y - 12.0f, p.y + 12.0f, false)` (0x44D960, ECX = ThePaths 0x96F050; constant `12.0f [0x858CCC]`).
  9. If `m_pFire (+0x164)`: `m_pFire->Extinguish()` (0x5393F0).
  10. `CPhysical::Destructor(this)` (0x542450).
- gta-reversed: gr uses `GetPosition()` of the object itself for the train-crossing case; the bytes use the entity pointer stored at +0x170 (the field the ctors zero). Open question: identity of +0x170 (gr m_pDummyObject? its use here means bridge-light reset relies on it being set for crossing objects, 0x59F7A2).

## 0x5A1D10 CObject::CObject()  (catalogue: specified/REA, port level 14)
- gr Object.cpp:77 (`// 0x5A1D10`). Convention: `__thiscall CObject* (this = ECX)`, returns this; SEH frame (handler 0x83D2A8, unwind CPhysical::Destructor 0x542450).
- Callers: 0x59EEB0, CProjectile ctor 0x5A4030, CCutsceneObject ctor 0x5B19D0, CAutomobile::SpawnFlyingComponent 0x6A8580, ... (5).
- Behaviour: `CPhysical::Constructor(this)` (0x542260); `vtbl = 0x866F60` (CObject vtable); `+0x170 = 0` (dword; gr m_pControlCodeList/first field after CPhysical in gr's ctor list - not verified by name here); `CObject::Init(this)` (0x59F840); then `byte +0x13C = 0` (after Init); return this.
- gta-reversed: gr ctor `CObject::CObject() : CPhysical() { m_pDummyObject = nullptr; Init(); m_nObjectType = OBJECT_UNKNOWN; }` - order Init then +0x13C matches. Open question: gr field names for +0x170 and +0x13C (byte, set after Init) to confirm.

## 0x5A1D70 CObject::CObject(int32, bool)  (catalogue: specified/REA, port level 14)
- gr Object.cpp:84 (`// 0x5A1D70`). Convention: `__thiscall CObject* (this = ECX, int32 modelId [esp+4], bool create [esp+8])`, `ret 8`, returns this; SEH frame handler 0x83D2C8.
- Callers: 13 (CPickup::GiveUsAPickUpObject, CRoadBlocks::CreateRoadBlockBetween2Points, CPools::MakeSureSlotInObjectPoolIsEmpty, ...).
- Behaviour: `CPhysical::Constructor(this)`; `vtbl = 0x866F60`; `+0x170 = 0`; `if (create /* byte compare with 0 */) CEntity::SetModelIndex(modelId) (0x532AE0) else CEntity::SetModelIndexNoCreate(modelId) (0x533700)`; `CObject::Init()` (0x59F840); return this. Unlike 0x5A1D10 it does not write byte +0x13C.
- gta-reversed: matches gr. Open questions: none.

## 0x5A1EE0 CObject::operator new  (catalogue: specified/REA, port level 1)
- gr Object.cpp:153. `__cdecl void* (size_t)` (size ignored): `ECX = CPools::ms_pObjectPool [0xB7449C]; jmp CObjectPool::New 0x5A1C20`. 16 callers. Open questions: none.

## 0x5A1F20 CObject::operator delete  (catalogue: specified/REA, port level 0)
- gr Object.cpp:163. Convention `__cdecl void (void* p [esp+4])`, plain `ret`. Globals: CPools::ms_pObjectPool 0xB7449C.
- Behaviour (inline CPool::Delete, element size 0x19C): `idx = (p - pool->m_pObjects) / 0x19C` via signed magic multiply (imul 0x13E22CBD, sar edx 5, add sign; 0x5A1F2D..0x5A1F3F); `m_byteMap[idx] |= 0x80`; `if (idx < m_nFirstFree) m_nFirstFree = idx`. Open questions: none.

## 0x6E3E30 WaterCreature_c::WaterCreature_c (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: element ctor (size 0x34, count 128) passed by WaterCreatureManager_c ctor 0x6E3EB0.
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `ListItem_c::ListItem_c(this)` (0x4A8DB0); `dword +8 = 0` (m_pObject). Open questions: none.

## 0x6E3EB0 WaterCreatureManager_c::WaterCreatureManager_c (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: only caller __init_g_waterCreatureMan 0x853F10; gr WaterCreatureManager_c.h: `WaterCreature_c m_aCreatures[128]; TList_c<WaterCreature_c> m_freeList, m_createdList`.
- Convention: `__thiscall T* (this = ECX)`, returns this; SEH frame (unwind 0x4A8DE0 List_c dtor, eh_vector_destructor).
- Behaviour: `eh_vector_constructor_iterator(this, 0x34, 0x80, WaterCreature_c ctor 0x6E3E30, dtor 0x6E3E50)`; `List_c::List_c(this+0x1A00)`; `List_c::List_c(this+0x1A0C)`; return this. Fields after +0x1A18 untouched.
- Open questions: none.
