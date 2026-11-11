# CORE: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x4048e0 CPool<CVehicle>::GetAtRef (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall` (this = CPool*), arg `i32 ref` [esp+4], returns EAX, `ret 4`.
- Behaviour: `idx = ref >> 8` (arithmetic `sar`, 0x4048eb); `if (m_byteMap[idx] != (u8)ref) return 0` (byte compare 0x4048f2, full 8 bits incl. the free flag); else `return m_pObjects + idx*0xa18` (the `imul eax,eax,0xa18` lives at 0x40210a, reached by `jmp` from 0x4048fa and back to 0x4048ff).
- No bounds check on idx. gta-reversed: `CPool::GetAtRef` (Core/Pool.h:285); gr adds a range check (`idx < m_nSize`) that the original lacks.
- Catalogue extent note: the piece 0x40210a-0x402115 belongs to this function (protection-relocated instruction).

## 0x4049c0 CPool<T>::Flush (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `if (m_nSize [+8] > 0)` (signed) { `if (m_bOwnsAllocations [+0x10])` { `operator delete[](m_pObjects [+0])` 0x8213ae; `operator delete[](m_byteMap [+4])` }; `[+0] = [+4] = [+8] = [+0xc] = 0` }. With m_nSize <= 0 nothing is cleared. gr `CPool::Flush` (Core/Pool.h).

## 0x405960 CPool<T>::Flush (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x4049c0.

## 0x4059b0 CPool<IplDef>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall` (this = CPool*: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xc m_nFirstFree), returns EAX = slot or 0. Body at 0x15654e0.
- Behaviour: `wrapped=false`; loop: `++m_nFirstFree` (stored, 0x15654ef); `if (m_nFirstFree == m_nSize)` (equality, 0x15654ed/0x15654f2) { `m_nFirstFree = 0` (stored before the test, 0x15654f6); `if (wrapped) return 0` (0x15654fd); `wrapped = true` }; repeat while `!(m_byteMap[m_nFirstFree] & 0x80)` (0x1565507). Then `byte &= 0x7f` (0x1565517); `byte = (byte & 0x80) | ((byte+1) & 0x7f)` (id increment, 0x1565529-0x1565532); return `m_pObjects + m_nFirstFree*0x34`.
- Edge: on exhaustion m_nFirstFree is left 0 (gr leaves -1); compare is `==` not `>=`. gta-reversed `CPool::New` (Core/Pool.h:201).

## 0x405a60 CPool<T>::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: inlined 0x4049c0 body; then `if (flags & 1) free(this)` 0x8214bd; EAX = this.

## 0x4082a0 CPool<CPed>::GetAt (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* (i32 idx)`, `ret 4`; body 0x1566860.
- `if ((i8)m_byteMap[idx] < 0) return 0` (`cmp byte,0; jns`, 0x1566867); else `return m_pObjects + idx*0x7c4`.
- gta-reversed: `CPool::GetAt(idx)` (Core/Pool.h:169). Callers CPedToPlayerConversations::Update, script opcodes 18xx.

## 0x410610 CPool<T>::Flush (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x4049c0.

## 0x410fe0 CPool<T>::Flush (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x4049c0.

## 0x411030 CPool<ColDef>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Identical code to 0x4059b0 (CPool<IplDef>::New) except the element stride `imul 0x2c` (0x156fc09). Body 0x156fbb0. Same edge case: on exhaustion m_nFirstFree = 0 and returns 0; wrap test is `==`.

## 0x4110e0 CPool<T>::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x405a60.

## 0x41bd50 CTimer::GetTimeStep (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float ()`: `fld [0xb7cb5c]` (CTimer::ms_fTimeStep), ST0 return. gr `CTimer::GetTimeStep()` inline (Core/Timer.h).

## 0x424160 CPool<CVehicle>::GetRef (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall i32 (const void* obj)`, `ret 4`; body 0x15667f0.
- `idx = (obj - m_pObjects) / 0xa18` (signed exact division via `imul 0x6572ec3; sar edx,6; +sign`, 0x15667fb-0x156680a); `return (idx << 8) + (u8)m_byteMap[idx]` (0x156680c-0x1566813).
- gta-reversed: `CPool::GetRef` (Core/Pool.h). Element size 0xa18 = vehicle pool.

## 0x43e3f0 CPtrList::CPtrList (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CPtrList* ()`: `[this] = 0` (m_node), EAX = this (0x43e3f0). gr `CPtrList()` (Core/PtrList.h).

## 0x43eeb0 CPool<T>::Flush (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x4049c0.

## 0x43f720 CPool<CEntryExit>::CPool (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall CPool* (i32 size [esp+4], const char* name [esp+8] (unused))`, EAX = this, `ret 8`; body 0x1561900.
- `m_pObjects [+0] = operator new(size*0x3c)` 0x821195; `m_byteMap [+4] = operator new(size)`; `m_bOwnsAllocations [+0x10] = 1`; `m_nSize [+8] = size`; `m_nFirstFree [+0xc] = -1`.
- `for (i = 0; i < size; ++i)` (signed, skipped if size <= 0): `byte |= 0x80` then `byte &= 0x80` (free, id 0) (0x1561935-0x1561952). No null checks on the allocations.
- gta-reversed: `CPool(size, name)` ctor (Core/Pool.h).

## 0x43f780 CPool<CEntryExit>::Flush (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x4049c0.

## 0x43f950 CPool<T>::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x405a60.

## 0x4442d0 CPool<CPed>::GetRef (proposed)  (catalogue: unknown/REA, port level 0)
- Same as 0x424160 with element size 0x7c4 (`imul 0x41ee7ca7; sar edx,9`, 0x156cd4b). Body 0x156cd40. Callers CGangWars::CreateAttackWave, script opcodes.

## 0x44d2b0 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x44d2b0). Port: no-op. Open question: which (debug) routine CGame::Init1 calls here.

## 0x463360 CRGBA::CRGBA() (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CRGBA* ()`, EAX = this, no stores (0x463360). gr `CRGBA() = default` (Core/RGBA.h).

## 0x463370 CRGBA::operator= (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CRGBA& (const CRGBA& src)`, EAX = this, `ret 4`; copies bytes +0,+1,+2,+3 in order (0x156e2d6-0x156e2e9).

## 0x465040 CPool<CObject>::GetAtRef (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* (i32 ref)`, `ret 4`: `idx = ref >> 8` (sar); `if (m_byteMap[idx] != (u8)ref) return 0` (0x465052); else `m_pObjects + idx*0x19c` (0x465059). No bounds check.

## 0x465070 CPool<CObject>::GetRef (proposed)  (catalogue: unknown/REA, port level 0)
- Same as 0x424160 with element size 0x19c (`imul 0x13e22cbd; sar edx,5`). Body 0x1561010.

## 0x49c8f0 CPool<CStuntJump>::CPool (proposed)  (catalogue: unknown/REA, port level 1)
- Same body as 0x43f720 with `imul eax,eax,0x44` (0x156fa88). Body 0x156fa80, `ret 8`.

## 0x4a8db0 ListItem_c::ListItem_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall ListItem_c* ()`, EAX = this: `[this+0] = 0` (m_pPrev; the store sits at 0x401c62, reached by jmp), `[this+4] = 0` (m_pNext, 0x4a8db8).
- Catalogue extent note: 0x401c62-0x401c6d belongs to this function.

## 0x4a8dc0 ListItem_c::~ListItem_c (empty body) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x4a8dc0): the out-of-line empty ListItem_c destructor. Port: no-op.

## 0x4a8dd0 List_c::List_c (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall List_c* ()`, EAX = this; `[+0] = [+4] = [+8] = 0`. gr `TList_c` (Core/List_c.h).

## 0x4a8df0 List_c::AddItem (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (ListItem_c* item)`, `ret 4`.
- `old = m_head; m_head = item; item->m_pPrev = 0; item->m_pNext = old` (0x4a8df8-0x4a8e00); `if (old) old->m_pPrev = item; else m_tail = item`; `++m_count` (EAX ends as the old count). gr `TList_c::AddItem` (Core/List_c.h:86).

## 0x4a8e30 List_c::RemoveItem (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (ListItem_c* item)`, `ret 4`.
- `prev = item->m_pPrev; next = item->m_pNext`; `if (next) next->m_pPrev = prev; else m_tail = prev` (0x4a8e3c-0x4a8e42); `if (prev) prev->m_pNext = next; else m_head = item->m_pNext`; `--m_count`. Item's own links are not cleared. EAX = old count (gr returns the item; callers in this build do not use EAX - open question per caller).

## 0x4a8e70 List_c::RemoveHead (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall ListItem_c* ()`.
- `h = m_head; if (!h) return 0`; `if (h == m_tail) { m_tail = 0; m_head = 0; --m_count; return h }`; else `m_head = h->m_pNext; if (m_head) m_head->m_pPrev = 0; --m_count; return h` (0x4a8e90-0x4a8e9f). Removed item's links are not cleared.

## 0x4a8eb0 List_c::RemoveAll (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[+0] = [+4] = [+8] = 0` (items are not unlinked). EAX = 0.

## 0x4a8ec0 List_c::GetNumItems (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall u32 ()`: EAX = m_count [+8].

## 0x4c94c0 CPool<CVehicleModelInfo::CVehicleStructure>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Same as 0x4059b0 (CPool::New) with element size 0x314 (`imul 0x314`, 0x4c9519), not relocated. Exhaustion leaves m_nFirstFree = 0 and returns 0.

## 0x508e90 remote debug/screenshot command handler (sub_508e90) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x508e90). Port: no-op.

## 0x508ea0 remote debug/screenshot command handler (sub_508ea0) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x508ea0). Port: no-op.

## 0x53e8d0 CLinkList<T>::Clear (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`.
- `while ((l = usedHead.next [+8]) != &usedTail [+0xc])`: unlink `l->next->prev = l->prev; l->prev->next = l->next`; insert after freeHead (+0x18): `l->next = freeHead.next; freeHead.next->prev = l; l->prev = &freeHead; freeHead.next = l` (0x53e8e0-0x53e901).
- gta-reversed: `CLinkList::RemoveAll` (Core/LinkList.h) equivalent.
