# _unattributed-FA (STREAM) - RE-SPEC-FINAL group A

## 0x4C3740 model info member dtor (deletes +8 via ...)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>; proposed: a STREAM helper that deletes three owned polymorphic objects (no code callers; owner class UNCLEAR - a
byte-identical twin exists at 0x4C3E30).
- Convention: `__thiscall void (this)`, plain `ret`.
- Behaviour: if `this->+8` != 0 call its virtual deleting dtor (vtable slot 0) with 1; `+8 = 0`; same for `+4` (then `+4 = 0`); then if
  `+0xC` != 0 delete it and `+0xC = 0` (+0xC is zeroed only when it was non-null).
- gta-reversed: n/a. Open questions: owner class (no xrefs).

## 0x4C3E30 model info member dtor (deletes +8 via ...)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>; proposed: a STREAM helper that deletes three owned polymorphic objects (no code callers; owner class UNCLEAR - a
byte-identical twin exists at 0x4C3740).
- Convention: `__thiscall void (this)`, plain `ret`.
- Behaviour: if `this->+8` != 0 call its virtual deleting dtor (vtable slot 0) with 1; `+8 = 0`; same for `+4` (then `+4 = 0`); then if
  `+0xC` != 0 delete it and `+0xC = 0` (+0xC is zeroed only when it was non-null).
- gta-reversed: n/a. Open questions: owner class (no xrefs).

## 0x5B2AB0 sub_5B2AB0  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`: `CBoundingBox::CBoundingBox(this)` 0x40FAF0; return this. No code callers (one of four byte-identical
  compiler instances; owner class UNCLEAR - proposed an inline ctor of a struct whose first member is a CBoundingBox, e.g. CColModel/CBox).
- gta-reversed: CBoundingBox default ctor. Open questions: owner class (no xrefs).

## 0x5B38F0 AddTextureCB  (catalogue: specified/REA, port level 1)
- `__cdecl RwTexture* (RwTexture* t [esp+4], RwTexDictionary* d [esp+8])`, plain `ret`: `RwTexDictionaryAddTexture(d, t)` 0x7F3980; return t.
  Callers CFileLoader::AddTexDictionaries 0x5B3910, LoadLevel 0x5B9030. gta-reversed: same. Open questions: none.

## 0x731630 cb_RSTXDPARENT_RwTexDictionary_Copy  (catalogue: specified/REA, port level 0)
- RW plugin copy callback `__cdecl void* (void* dst [esp+4], const void* src [esp+8], int32 offset, int32 size)`, plain `ret`, EAX = dst:
  `*(u32*)(dst + ms_txdPluginOffset) = *(u32*)(src + ms_txdPluginOffset)` ([0xC88018]). Registered by CTxdStore::PluginAttach 0x731650.
- gta-reversed: TxdStore copy callback - same. Open questions: none.
