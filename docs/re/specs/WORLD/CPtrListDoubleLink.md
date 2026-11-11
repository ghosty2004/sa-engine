# CPtrListDoubleLink / CPtrNodeDoubleLink (WORLD)

Node (0xc bytes, pool `CPools::ms_pPtrNodeDoubleLinkPool` 0xb74488): `+0 item`, `+4 next`, `+8 prev`.
List: `+0 head` (node*). Node allocation: `0x5523c0` (`__cdecl void* (u32 size)` → pool New via
`jmp 0x5522e0` with ECX = pool), free: `0x5523d0` (`__cdecl void (void*)`, pool delete with element size
0xc, magic 0x2aaaaaab `sar 1`).

## 0x5329a0 CPtrNodeDoubleLink::sub_5329a0 (proposed: `CPtrNodeDoubleLink::AddToList(CPtrListDoubleLink*)`)  (catalogue: unknown/REA, port level 0)
- Evidence: links `this` (ECX, a node) at the head of the list passed as argument; callers
  `CEntity::CreateRwObject` 0x533d30 and `CEntity::AttachToRwObject` 0x533ed0 use it for the
  `CWorld::ms_listMovingEntityPtrs.AddItem` path that gta-reversed inlines (Entity.cpp:309/1197).
- Convention / signature: `__thiscall void (this = ECX: node*, CPtrListDoubleLink* list [esp+4])`, `ret 4`.
  EAX = list on return (unused).
- Behaviour: `node->prev = 0; node->next = list->head; if (list->head) list->head->prev = node;
  list->head = node` (list->head re-read for the null test).
- Open questions: none.

## 0x533670 CPtrListDoubleLink::AddItem  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall CPtrNodeDoubleLink* (this = ECX: list, void* item [esp+4])`,
  returns the new node in EAX, `ret 4`.
- Callers: 1 (`CEntity::Add(const CRect&)` 0x5347d0). Callees: node `new` 0x5523c0 (`push 0xc`, caller pops).
- Behaviour: `n = new(0xc)`; `if (n) n->item = item; else n = NULL;` then unconditionally
  `n->prev = 0; n->next = head; if (head) head->prev = n; head = n`. Return n.
- Edge cases: if the pool is exhausted and `new` returns NULL the code writes to address 8 (crash);
  original behaviour.
- gta-reversed: template `CPtrList::AddItem` (equivalent); not a separate hook.
- Open questions: none.

## 0x5336b0 CPtrListDoubleLink::DeleteItem  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall void (this = ECX: list, void* item [esp+4])`, `ret 4`.
- Callers: 2 (`CEntity::DetachFromRwObject` 0x533fb0, `CEntity::DeleteRwObject` 0x534030). Callees:
  node `delete` 0x5523d0.
- Behaviour: `h = head; for (n = h; n; n = n->next) if (n->item == item) break;` if not found return.
  Found (first match from the head): `if (n == h) head = n->next; if (n->prev) n->prev->next = n->next;
  if (n->next) n->next->prev = n->prev; delete(n)`. Only the first matching node is removed.
- gta-reversed: template code, equivalent.
- Open questions: none.

## 0x5630F0 CPtrListDoubleLink::~CPtrListDoubleLink  (catalogue: unknown/REA, port level 1)
- Thunk: `jmp CPtrListDoubleLink::Flush` 0x552470 (this unchanged). Address used as the element dtor by CRepeatSector's vector-dtor 0x5631B0 and by 0x563190.
- gta-reversed: `~CPtrListDoubleLink() { Flush(); }` - matches. Open questions: none.
