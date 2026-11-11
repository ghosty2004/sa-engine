# _unattributed-FA (RENDER) - RE-SPEC-FINAL group A

## 0x49EA80 ListItem_c-derived ctor (calls ListItem_c ctor)  (catalogue: unknown/REA, port level 1)
Catalogue name: <none>; proposed: inline default ctor of a ListItem_c-derived class with no vtable and no extra initialised members (no code
callers; owner UNCLEAR).
- Convention: `__thiscall T* (this)`, plain `ret`, returns this. Body: `ListItem_c::ListItem_c(this)` 0x4A8DB0; return this.
- gta-reversed: n/a. Open questions: owner class (no xrefs).

## 0x4A9010 sub_4a9010 (Fx list helper)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>; gr name: `List_c::GetItemOffset(bool fromHead, int32 offset)` (Core/List_c.h:251; used by FxManager).
- Convention: `__thiscall ListItem_c* (List_c* this, bool fromHead [esp+4], int32 offset [esp+8])`, `ret 8`.
- Behaviour: from head: `p = this->head (+0)`; from tail: `p = this->tail (+4)`. If `offset <= 0` return p. Else repeat `offset` times:
  if `p == 0` return 0; `p = fromHead ? p->m_pNext (+4) : p->m_pPrev (+0)`; return p.
- gta-reversed: List_c.h:251 - same result. Open questions: none.

## 0x4A9090 FxSphere_c-derived ctor (calls FxSphere_c ctor)  (catalogue: unknown/REA, port level 1)
Catalogue name: <none>; proposed: inline default ctor of an FxSphere_c-holding object (no code callers, owner UNCLEAR).
- Convention: `__thiscall T* (this)`, plain `ret`, returns this. Body: `FxSphere_c::FxSphere_c(this)` 0x4A9F70; return this.
- gta-reversed: n/a. Open questions: owner class (no xrefs).

## 0x4A9110 ListItem_c-derived ctor (calls ListItem_c ctor)  (catalogue: unknown/REA, port level 1)
Catalogue name: <none>; proposed: inline default ctor of a ListItem_c-derived class (byte-identical to 0x49EA80; no code callers).
- Convention: `__thiscall T* (this)`, plain `ret`, returns this. Body: `ListItem_c::ListItem_c(this)` 0x4A8DB0; return this.
- gta-reversed: n/a. Open questions: owner class (no xrefs).
