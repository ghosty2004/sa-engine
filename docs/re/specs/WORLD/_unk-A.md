# WORLD: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x404090 CBuilding::operator new (proposed)  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void* (u32 size)` (size ignored); `mov ecx,[0xb74498]` (CPools::ms_pBuildingPool) then `jmp 0x403fa0` CBuildingPool::New (thiscall, returns EAX).
- gta-reversed: CBuilding uses the pool-backed `operator new` (gr Entity/Building.cpp); out-of-line copy here.

## 0x4040a0 CBuilding::operator delete (proposed)  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void* p)`; body relocated to 0x156c060 (`jmp` at 0x4040a0).
- Behaviour: pool = [0xb74498] CPools::ms_pBuildingPool; `idx = (p - pool->m_pObjects) / 0x38` (signed exact division via `imul 0x92492493; add edx,ecx; sar edx,5; +sign bit`, 0x156c06d-0x156c081); `pool->m_byteMap[idx] |= 0x80` (mark free, 0x156c086); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (signed `jge` at 0x156c08c).
- Equals inline `CPool<CBuilding>::Delete` (gr Core/Pool.h:266) out-of-line.

## 0x404350 CEntity::AsXxx (identity cast) (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall`, returns EAX = this (0x404350 `mov eax,ecx`). No side effects.
- Out-of-line copy of an identity inline (a gr `AsVehicle()`/`AsPed()`-style cast in Entity/Entity.h); which one cannot be told from bytes.
- Open question: exact inline name (0x404350); behaviour is fully determined.

## 0x4043e0 CEntity::GetStatus (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall` (this = CEntity*), returns EAX = `((u8)this[0x36]) >> 3` (0x15700f0/0x15700f4), i.e. `m_nStatus` (5-bit field above `m_nType`).
- gta-reversed: `CEntity::GetStatus()` inline (Entity/Entity.h). Body relocated to 0x15700f0.

## 0x404420 CEntity::GetModelIndex (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall`, returns EAX = sign-extended i16 at this+0x22 (`m_nModelIndex`), 0x1563a40.
- gta-reversed: inline `GetModelIndex()/GetModelId()` (Entity/Entity.h). Note the sign extension (model -1 stays -1).

## 0x404590 CQuadTreeNode::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 3)
- `__thiscall void* (u32 flags)`, `ret 4`: `CQuadTreeNode::~CQuadTreeNode(this)` 0x552520; `if (flags & 1) CQuadTreeNode::operator delete(this)` 0x552c90 (pool-backed, cdecl); EAX = this.

## 0x4072e0 CWorld::IncrementCurrentScanCode (proposed)  (catalogue: specified/REA, port level 1)
- `__cdecl void ()`; body at 0x1570050.
- `if ((u16)[0xb7cd78] >= 0xffff)` (`cmp word,-1; jae`, 0x1570050) { `CWorld::ClearScanCodes()` 0x563470; `[0xb7cd78] = 1` } else `++[0xb7cd78]` (0x157005a).
- gta-reversed: `CWorld::IncrementCurrentScanCode()` (World.h inline). Callers CPhysical::ProcessShift, CPlayerInfo::Process.

## 0x41bd80 CPlaceable::GetMatrix (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = [this+0x14] (0x41bd80). In CPlaceable +0x14 is `m_matrix` (CMatrixLink*); gr `GetMatrix()` returns `*m_matrix` (a reference = the pointer). No null check.
- Open question: owner class from caller context only (0x41bd80).

## 0x41cc70 CPlaceable::GetRightDirection (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall CVector* (CVector* out)` (hidden result), EAX = out, `ret 4`.
- `if (m_matrix [+0x14])` copy m_matrix->right (+0..+8) to out (0x41cc7c-0x41cc8b); else `h = m_placement.heading [+0x10]`: `fsin h`, `fcos h`, `out.z = 0` (integer store), `out.x = cos(h)`, `out.y = sin(h)` (0x41cc92-0x41cca5).
- gta-reversed: `CPlaceable::GetRightVector()` (Entity/Placeable.cpp).

## 0x41ccb0 CPlaceable::GetTopDirection (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall CVector* (CVector* out)`, EAX = out, `ret 4`.
- `if (m_matrix)` copy m_matrix->forward (+0x10..+0x18); else `fcos h; fsin h; out.z = 0; fchs` -> `out.x = -sin(h)`, `out.y = cos(h)` (0x41ccd7-0x41ccf0).
- gta-reversed: `CPlaceable::GetForwardVector()` (Entity/Placeable.cpp).

## 0x43e1a0 CPtrListSingleLink::~CPtrListSingleLink (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`: `jmp CPtrListSingleLink::Flush 0x552400`.

## 0x532560 CDummy::~CDummy (proposed)  (catalogue: unknown/REA, port level 13)
- `__thiscall void ()`: `[this] = 0x8638c0` (CDummy vtable); `jmp CEntity::~CEntity 0x535e90`.

## 0x5335e0 CPtrListSingleLink::AddItem (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall CPtrNodeSingleLink* (void* item)`, `ret 4`.
- `node = CPtrNodeSingleLink::operator new(8)` 0x552380; if node: `node->item [+0] = item; node->next [+4] = m_node; m_node = node`; return node.
- Edge (original bug): if the pool returns 0, the code still writes `[0+4] = m_node` and sets `m_node = 0` (0x533602-0x533609) - access violation in the original. gr adds no check either. Port: keep the write semantics only if reproducing the crash is wanted; otherwise document.

## 0x533610 CPtrListSingleLink::DeleteItem (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void (void* item)`, `ret 4`.
- Walk from `m_node` keeping `prev`; first node with `node->item == item` (0x533620): if `node == m_node` -> `m_node = node->next`, `operator delete(node)` 0x552390; else `if (prev) prev->next = node->next` then delete. Not found -> nothing.
