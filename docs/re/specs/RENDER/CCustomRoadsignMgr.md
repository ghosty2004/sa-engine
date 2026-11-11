# CCustomRoadsignMgr (RENDER) - RE-SPEC-FINAL group A

## 0x6FE200 CCustomRoadsignMgr::SetTextureCB  (catalogue: unknown/REA, port level 4)
- `__cdecl RpMaterial* (RpMaterial* m [esp+4], RwTexture* tex [esp+8])`, plain `ret`: `RpMaterialSetTexture(m, tex)` 0x74DBC0; return m.
  Caller CCustomRoadsignMgr::SetupRoadsignAtomic 0x6FED60. gta-reversed: same. Open questions: none.
