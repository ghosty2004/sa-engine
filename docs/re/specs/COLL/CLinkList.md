# CLinkList<T> template instances (COLL), full specs

Layout (gta-reversed Core/LinkList.h, Link.h; size 0x34): `CLink<T> usedListHead` +0x00, `usedListTail` +0x0c,
`freeListHead` +0x18, `freeListTail` +0x24, `CLink<T>* links` +0x30. `CLink<T>` (T = pointer): `data` +0,
`prev` +4, `next` +8.

## 0x4161f0 CLinkList<CCollisionData*>::Insert (proposed name; catalogue: unknown/REA, port level 0)
Name evidence: only caller is `CCollision::CalculateTrianglePlanes(CCollisionData*)` 0x416330 (2 sites), which
in gta-reversed calls `ms_colModelCache.Insert(colData)` twice (Collision.cpp:70 and :78, the second after
evicting the least recently used entry); `ms_colModelCache` is 0x96592c. The body is exactly
`CLinkList<T>::Insert(T const&)`.
- Convention / signature: `__thiscall CLink<CCollisionData*>* (CLinkList* this = ECX, CCollisionData* const* data
  [esp+4])`, return EAX, `ret 4`. Stub `jmp 0x1570010` (`.HOODLUM` body). Preserves ESI (pushed only on the
  success path); clobbers EDX.
- Callers: 1 function (0x416330, 2 sites). Callees: none. Globals: none directly (the instance passed is
  `CCollision::ms_colModelCache` 0x96592c).
- Behaviour:
  1. `link = this->freeListHead.next` ([this+0x20]). If `link == &this->freeListTail` (this+0x24) return NULL.
  2. `link->data = *data`.
  3. Unlink: `link->next->prev = link->prev`; `link->prev->next = link->next` (in that order; the second reads
     `link->next` again from memory after the first store).
  4. Insert after the used-list head: `link->next = this->usedListHead.next` ([this+8]);
     `this->usedListHead.next->prev = link`; `link->prev = this` (i.e. `&usedListHead`); `this->usedListHead.next
     = link`.
  5. Return `link`.
- Edge cases: none beyond the empty free list.
- gta-reversed: `CLinkList<T>::Insert(T const&)` (header template, not hooked at this address): identical
  operations and order.
- Open questions: none.
