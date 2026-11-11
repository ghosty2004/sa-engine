# WORLD: functions without gta-reversed source (RE-SPEC-UNK lead)

## 0x403e00 CBuilding::CBuilding (proposed)  (catalogue: unknown/REA, port level 1)
- Convention: `__thiscall` (ECX = CBuilding*), returns EAX = this, plain `ret`. The game-.text entry is a
  5-byte `jmp` stub (0x403E00) into the `.HOODLUM` body 0x156CFD0-0x156CFF0 (CAT-9 stub/body pair).
- Callers (4): 0x537A10, 0x538090, 0x592AA0, 0x5A32D0 (CAnimatedBuilding / CFileLoader::LoadObjectInstance /
  Interior_c::PlaceFurniture / procedural objects per the catalogue). Callees: 0x532A90 CEntity::Constructor.
- Behaviour: 0x156CFD1 ESI = this; call CEntity::Constructor (thiscall, ECX = this); 0x156CFD8 AL = byte
  [this+0x36]; AL &= 0xF9; AL |= 1; byte [this+0x36] = AL (entity type bits 0-2 = 1 = ENTITY_TYPE_BUILDING,
  status bits 3-7 kept; gr CEntity m_nType/m_nStatus bitfield at +0x36); 0x156CFE2 dword [this+0x1C] |= 1
  (first entity flag bit, gr `m_bUsesCollision`); 0x156CFE6 vptr [this+0] = 0x8585C8 (CBuilding vtable,
  data-tables `vtable-gr:CBuilding`); EAX = this.
- gta-reversed: CBuilding::CBuilding exists in source (Entity/Building.cpp) but this address is not a hook;
  the field writes above are what the port must reproduce (type set before the flag, vptr last).
- Open questions: none.
