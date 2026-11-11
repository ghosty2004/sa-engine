# CPedAttractorManager (AI) - RE-SPEC-3 group I2

## 0x5EDF10 CPedAttractorManager::CPedAttractorManager  (catalogue: specified/REA, port level 2)
- `__thiscall this ()`, plain `ret` (SEH frame only). Zeroes first/last/end (+4, +8, +0xC) of each of the 10 SArrays (stride 0x10:
  +0x04..+0x9C); the allocator dwords (+0x00, +0x10, ... +0x90) are left untouched. Returns this. gr `= default` - equivalent.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5EE400 CPedAttractorManager::sub_5EE400 (proposed: std::vector<T*>::push_back)  (catalogue: unknown/REA, port level 7)
- Name proposal: `std::vector<T>::push_back(const T&)` for a 4-byte T (the SArray<CPedXxxAttractor*> lists of CPedAttractorManager). Evidence: capacity test then uninitialized_fill_n(last, 1, &val) / _Insert_n(end, 1, val); callers CPedAttractorManager::RestoreStuffFromMem 0x5EF710 and RegisterPed<CPedAttractor> 0x5EF980.
- `__thiscall void (this = ECX: vector*, const T* val [esp+4])`, `ret 4`.
- Behaviour: `first = +4`; `size = first ? (last-first)>>2 : 0` (0x5EE403-0x5EE414). `if first != NULL && size < (end-first)>>2` (unsigned `jae`, 0x5EE425): `uninitialized_fill_n(last, 1, val)` 0x5EB390 (cdecl, with two extra dwords this/val pushed; `add esp, 0x14`) (0x5EE438); `last += 4` (0x5EE440/0x5EE443). Else: `_Insert_n(this, where = last, count = 1, val)` 0x5EDA50 (thiscall, 0x5EE45C).
