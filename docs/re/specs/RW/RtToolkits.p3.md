# RtDict toolkit pass 3 (RE-SPEC-RW-3 lead, taking over part of writer B's unwritten file)

All `__cdecl`. Layouts (RW 3.6 rtdict.h; offsets proven by the instructions cited):
- RtDictSchema: +0x0 name, +0x4 dictChunkType (also used as memory hint `dictChunkType | 0x30000`, 0x7ceda2), +0x8 entryChunkType,
  +0xc compatibilityVersion, +0x10 RwSList* dictionaries, +0x14 RtDict* current, +0x18 addRefCB, +0x1c destroyCB, +0x20 getNameCB,
  +0x24 streamGetSizeCB, +0x28 streamReadCB, +0x2c streamReadCompatibilityCB, +0x30 streamWriteCB.
- RtDict (8 bytes): +0x0 schema, +0x4 RwSList* entries (4-byte entries).
- RwSList helpers (writer B, RwEngine.p3.md): _rwSListCreate 0x809160(entrySize, hint), _rwSListGetNewEntry 0x809240(list, hint),
  _rwSListDestroyEndEntries 0x809400(list, n) (`numElements -= n`), _rwSListDestroy 0x809440, _rwSListGetNumEntries 0x8094b0 (+4),
  _rwSListGetEntry 0x809510(list, i) = `entries + i * entrySize` (address), _rwSListGetBegin 0x809530, _rwSListGetEnd 0x809540.
librw: absent (no RtDict).

## 0x7ced70 RtDictSchemaDestruct  (catalogue: specified/REA, port level 1)
- `RwBool (RtDictSchema* s)`: `_rwSListDestroy(s->dictionaries)`; returns 1. The dictionaries themselves are not destroyed.

## 0x7ced90 RtDictSchemaCreateDict  (catalogue: specified/REA, port level 2)
- `RtDict* (RtDictSchema* s)`. `d = Engine+0x134(8, s->dictChunkType | 0x30000)`; NULL -> `RwErrorSet({0x1bf, _rwerror(0x80000013, 8)})` (plugin id 0x1bf,
  0x7cedc0), NULL. `d->entries = _rwSListCreate(4, hint)`; NULL -> free d, NULL. `d->schema = s`; `slot = _rwSListGetNewEntry(s->dictionaries, hint)`; NULL ->
  destroy the entry list, free d, NULL; else `*slot = d`, return d. Does not change `s->current`.

## 0x7cee50 RtDictSchemaAddDict  (catalogue: specified/REA, port level 1)
- `RtDictSchema* (RtDictSchema* s, RtDict* d)`: `slot = _rwSListGetNewEntry(s->dictionaries, hint)`; NULL -> NULL; `*slot = d`; returns s. `d->schema` is not set.

## 0x7cee80 RtDictSchemaRemoveDict  (catalogue: specified/REA, port level 1)
- `RtDictSchema* (RtDictSchema* s, RtDict* d)`: find d in s->dictionaries; not found -> NULL. Found: the slot gets the LAST entry's value
  (`*GetEntry(list, num-1)`, 0x7ceebe-0x7ceecd), then `DestroyEndEntries(list, 1)`; returns s. `s->current` is not cleared.

## 0x7ceee0 RtDictSchemaGetCurrentDict  (catalogue: specified/REA, port level 0)
- returns `s+0x14`.

## 0x7ceef0 RtDictSchemaSetCurrentDict  (catalogue: specified/REA, port level 0)
- `RtDictSchema* (RtDictSchema* s, RtDict* d)`: `s+0x14 = d`; returns s (EAX = s). Caller CStreaming::ConvertBufferToObject 0x40c6b0.

## 0x7cef00 _rtDictSchemaInitDict  (catalogue: specified/REA, port level 1)
- `RtDict* (RtDictSchema* s, RtDict* d)`: like RtDictSchemaCreateDict on caller-provided storage: `d->entries = _rwSListCreate(4, hint)` (NULL -> NULL);
  `d->schema = s`; append d to s->dictionaries (failure -> destroy the entry list, NULL); returns d.

## 0x7cef60 _rtDictDestruct  (catalogue: specified/REA, port level 1)
- `RwBool (RtDict* d)`: for each entry (begin..end) `d->schema->destroyCB (+0x1c)(entry)`; `_rwSListDestroy(d->entries)`; returns 1. d is not removed from
  the schema list and not freed.

## 0x7cefb0 RtDictAddEntry  (catalogue: specified/REA, port level 1)
- `RtDict* (RtDict* d, void* entry)`: `slot = _rwSListGetNewEntry(d->entries, schema->dictChunkType | 0x30000)`; `*slot = entry` WITHOUT a NULL check
  (0x7cefce); `schema->addRefCB (+0x18)(entry)`; returns d.

## 0x7cefe0 RtDictFindNamedEntry  (catalogue: specified/REA, port level 1)
- `void* (RtDict* d, const RwChar* name)`: for each entry e in order: `Engine+0x114 (vecStrcmp)(schema->getNameCB (+0x20)(e), name) == 0` -> return e
  (case-sensitive). Not found -> NULL. The callback pointer is read once, the strcmp slot each iteration.

## 0x7cf060 RtDictForAllEntries  (catalogue: specified/REA, port level 1)
- `RtDict* (RtDict* d, RtDictEntryCallBack cb, void* data)`: `cb(entry, data)` for each entry; NULL stops. Returns d in both cases.

## 0x7cf0c0 RtDictRemoveEntry  (catalogue: specified/REA, port level 1)
- `RtDict* (RtDict* d, void* entry)`: find entry; not found -> NULL. Found: `schema->destroyCB (+0x1c)(entry)`; then the slot is overwritten with
  `_rwSListGetEntry(d->entries, numEntries)` - the ADDRESS one past the last entry (no `dec`, no dereference: 0x7cf105-0x7cf114) - and
  `DestroyEndEntries(list, 1)`. ORIGINAL BUG: the removed slot ends up holding a pointer into the list buffer instead of the last entry, and the real
  last entry is dropped (compare RtDictSchemaRemoveDict 0x7cee80, which does it correctly). Returns d.

## 0x7cf130 RtDictDestroy  (catalogue: specified/REA, port level 1)
- `RwBool (RtDict* d)`: if `schema->current == d` set it to 0; remove d from `schema->dictionaries` (last-entry move, as 0x7cee80; nothing when not found);
  destroyCB on every entry; `_rwSListDestroy(d->entries)`; free d (Engine+0x138); returns 1. Caller CStreaming::ConvertBufferToObject.

## 0x7cf1f0 RtDictStreamGetSize  (catalogue: specified/REA, port level 1)
- `RwUInt32 (RtDict* d)`: `0x10 + sum over entries (schema->streamGetSizeCB (+0x24)(e) + 0xc)`.

## 0x7cf5b0 RtDictSchemaForAllDictionaries  (catalogue: specified/REA, port level 1)
- `RwBool (RtDictSchema* s, RtDictCallBack cb, void* data)`: `cb(dict, data)` for each dictionary; NULL stops. Returns 1 always.

## libpng callbacks used by RtPNGImageRead 0x7cf9b0 / RtPNGImageWrite 0x7cf600 (libpng is statically linked; png_struct+0x54 = io_ptr)

## 0x7cf940 _rtPNGMalloc  (catalogue: specified/REA, port level 1)
- `void* (png_structp png, png_size_t size)`: returns `Engine+0x134(size, 0x3010d)` (png is ignored).

## 0x7cf960 _rtPNGFree  (catalogue: specified/REA, port level 1)
- `void (png_structp png, void* p)`: `Engine+0x138(p)`.

## 0x7cf980 _rtPNGWriteData  (catalogue: specified/REA, port level 1)
- `void (png_structp png [esp+4], png_bytep data, png_size_t len)`: `s = png+0x54` (io_ptr, the RwStream); if s: replace the first stack argument with s and
  tail-jump to `RwStreamWrite 0x7ecb30(s, data, len)` (its RwStream* return lands in EAX, ignored by libpng); else `0x7d34f0(png, NULL)` (libpng error
  routine; UNCLEAR which png_* function 0x7d34f0 is - not specified in this pass).

## 0x7cfde0 _rtPNGReadData  (catalogue: specified/REA, port level 1)
- Same shape as 0x7cf980 with `RwStreamRead 0x7ec9d0(s, data, len)`; a short read is NOT reported to libpng (the byte count is discarded).
