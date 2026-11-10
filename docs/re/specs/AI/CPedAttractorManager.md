# CPedAttractorManager (AI) - RE-SPEC-3 group I2

## 0x5EDF10 CPedAttractorManager::CPedAttractorManager  (catalogue: specified/REA, port level 2)
- `__thiscall this ()`, plain `ret` (SEH frame only). Zeroes first/last/end (+4, +8, +0xC) of each of the 10 SArrays (stride 0x10:
  +0x04..+0x9C); the allocator dwords (+0x00, +0x10, ... +0x90) are left untouched. Returns this. gr `= default` - equivalent.
