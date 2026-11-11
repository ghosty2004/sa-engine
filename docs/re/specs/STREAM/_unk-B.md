# STREAM: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x731600 TxdParentConstructor  (catalogue: specified/REA, port level 0)
- gr TxdStore_Plugin.cpp:21. RwTexDictionary plugin ctor registered by CTxdStore::PluginAttach 0x731650. `__cdecl void* (void* txd [esp+4], int32, int32)`.
- Behaviour: `*(u32*)(txd + ms_txdPluginOffset [0xC88018]) = 0` (no offset check); return value: EAX = txd (loaded first at 0x731600). Open questions: none.

## 0x731620 TxdParentDestructor  (catalogue: specified/REA, port level 0)
- gr TxdStore_Plugin.cpp:27. `__cdecl void* (void* txd, int32, int32)`: returns txd, no effect. Open questions: none.

## 0x7316D0 CTxdStore::GetDefaultTxd (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: reads `CTxdStore::defaultTxds` 0xC88004 (int16 table); only caller CClothesBuilder::RequestTexture 0x5A4220. Name is descriptive (no symbol).
- Convention: `__cdecl int32 (int32 index [esp+4])`, plain `ret`. Behaviour: `return (int32)(int16)defaultTxds[index]` (movsx, no bounds check). Open questions: original name.
