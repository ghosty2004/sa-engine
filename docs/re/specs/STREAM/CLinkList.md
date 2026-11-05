# CLinkList<T> (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x408230 CLinkList<CEntity*>::Insert(CEntity* const&)  (catalogue: unknown/REA, port level 0; proposed name)
Name hint was `sub_408230_from_CStreaming__AddEntity`. Evidence for the name: `this` layout is exactly
gta-reversed `CLinkList<T>` (Core/LinkList.h, size 0x34: usedListHead +0x0, usedListTail +0xC, freeListHead
+0x18, freeListTail +0x24, links +0x30; `CLink<T>` = {T data +0x0, CLink* prev +0x4, CLink* next +0x8}),
and the body is gr's `CLink<T>* Insert(T const& data)` line for line. The only caller, CStreaming::AddEntity
(0x409650, 3 call sites), inserts into `CStreaming::ms_rwObjectInstances` (`CLinkList<CEntity*>` at 0x9654F0).
- Convention: `__thiscall CLink<CEntity*>* Insert(const CEntity** data)`; ECX = this, data pointer at [esp+4];
  `ret 4`; returns EAX. Body in `.HOODLUM` at 0x1563580 (0x408230 is a jmp stub). Preserves ESI.
- Callees: none. Globals: none (only through `this`).
- Behaviour (exact store order):
  1. `link = this->freeListHead.next` ([ecx+0x20]). If `link == &this->freeListTail` (ecx+0x24): return NULL.
  2. `link->data = *data;`
  3. Unlink: `link->next->prev = link->prev; link->prev->next = link->next;` (reads `next` at +8 first, then
     `prev` at +4; then re-reads both for the second store).
  4. Link after usedListHead: `link->next = this->usedListHead.next; this->usedListHead.next->prev = link;
     link->prev = this (== &usedListHead); this->usedListHead.next = link;`
  5. Return `link`.
- Edge cases: `data` is written before unlinking (no observable difference). No null check on `data`.
- gta-reversed: Core/LinkList.h `Insert(T const&)` (inline template, not hooked) - same semantics.
- Open questions: none.
