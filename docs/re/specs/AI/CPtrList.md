# CPtrList (AI catalogue rows) - RE-SPEC-3 group I4

## 0x5521D0 CPtrList::IsMemberOfList  (catalogue: specified/REA, port level 0)
- `__thiscall bool (CPtrList* this, void* item [esp+4])`, `ret 4`, result in AL only (upper EAX = node pointer / garbage).
- Callers: CCover::Update 0x6997E0.
- Behaviour: `node = this->pNode (+0)`; while node != 0: `if (node->pItem (+0) == item) return 1` (0x5521E0); `node = node->pNext
  (+4)` (0x5521E4). Return 0 (0x5521EB). Walks forward from the head; no other side effects.
- gta-reversed: CPtrList::IsMemberOfList inline loop - matches.
