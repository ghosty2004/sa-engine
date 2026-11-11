# _unattributed-FA (COLL) - RE-SPEC-FINAL group A

## 0x4161D0 member buffer free (sub_4161d0)  (catalogue: unknown/REA, port level 2)
Catalogue name: <none>, proposed: `<COLL owner>::FreeBuffer30()` - no static callers (0 xrefs; reached only through a table or dead), so the
owning class is UNCLEAR; it frees a heap block at +0x30.
- Convention: `__thiscall void (this)`, plain `ret`.
- Behaviour: `free(this->+0x30)` (CRT _free 0x8213AE, cdecl; null is allowed by free); then `this->+0x30 = 0`.
- gta-reversed: absent. Open questions: owner class / caller (no code xrefs found in the bundle).

## 0x537550 CColModel-like ctor (calls CBoundingBox ctor)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`: `CBoundingBox::CBoundingBox(this)` 0x40FAF0; return this. No code callers (one of four byte-identical
  compiler instances; owner class UNCLEAR - proposed an inline ctor of a struct whose first member is a CBoundingBox, e.g. CColModel/CBox).
- gta-reversed: CBoundingBox default ctor. Open questions: owner class (no xrefs).

## 0x537560 CColModel-like ctor (calls CBoundingBox ctor)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`: `CBoundingBox::CBoundingBox(this)` 0x40FAF0; return this. No code callers (one of four byte-identical
  compiler instances; owner class UNCLEAR - proposed an inline ctor of a struct whose first member is a CBoundingBox, e.g. CColModel/CBox).
- gta-reversed: CBoundingBox default ctor. Open questions: owner class (no xrefs).

## 0x537570 CColModel-like ctor (calls CBoundingBox ctor)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`: `CBoundingBox::CBoundingBox(this)` 0x40FAF0; return this. No code callers (one of four byte-identical
  compiler instances; owner class UNCLEAR - proposed an inline ctor of a struct whose first member is a CBoundingBox, e.g. CColModel/CBox).
- gta-reversed: CBoundingBox default ctor. Open questions: owner class (no xrefs).
